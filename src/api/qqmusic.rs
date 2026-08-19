//! QQ Music lyric source.
//!
//! QQ Music returns QRC/TTML lyric content encrypted as hex. The payload uses
//! the same character-timing format as the existing YRC parser, so the
//! decoded content is stored in `LyricDetail::yrc` and parsed by the common
//! lyric pipeline.

use anyhow::{Context, bail};
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use flate2::read::{DeflateDecoder, GzDecoder, ZlibDecoder};
use reqwest::header::{ACCEPT_ENCODING, CONTENT_TYPE, COOKIE, HeaderMap, HeaderValue, USER_AGENT};
use serde_json::{Value, json};
use std::io::Read;

use super::{LyricDetail, Song};

const BASE_URL: &str = "https://u.y.qq.com/cgi-bin/musicu.fcg";
const KEY1: &[u8; 16] = b"!@#)(NHLiuy*$%^&";
const KEY2: &[u8; 16] = b"123ZXC!@#)(*$%^&";
const KEY3: &[u8; 16] = b"!@#)(*$%^&abcDEF";

fn response_preview(body: &[u8]) -> String {
    let text = String::from_utf8_lossy(body).replace('\n', " ");
    text.chars().take(180).collect()
}

fn parse_json_response(body: &[u8], endpoint: &str) -> anyhow::Result<Value> {
    serde_json::from_slice(body).map_err(|error| {
        anyhow::anyhow!(
            "QQ {endpoint} returned non-JSON: {error}; body_prefix={:?}",
            response_preview(body)
        )
    })
}

fn decode_http_body(body: &[u8]) -> anyhow::Result<Vec<u8>> {
    if body.starts_with(&[0x1f, 0x8b]) {
        let mut decoded = Vec::new();
        GzDecoder::new(body)
            .read_to_end(&mut decoded)
            .context("failed to decompress QQ HTTP response")?;
        log::debug!(
            "[lyrics][qq] decompressed gzip response {} -> {} bytes",
            body.len(),
            decoded.len()
        );
        Ok(decoded)
    } else {
        Ok(body.to_vec())
    }
}

#[derive(Debug, Clone)]
pub struct QqMusicSong {
    pub song_id: u64,
    pub mid: String,
    pub title: String,
    pub singer_name: String,
    pub album_name: String,
    pub duration: u64,
}

fn headers() -> HeaderMap {
    let mut headers = HeaderMap::new();
    headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
    headers.insert(ACCEPT_ENCODING, HeaderValue::from_static("gzip"));
    headers.insert(USER_AGENT, HeaderValue::from_static("okhttp/3.14.9"));
    headers.insert(COOKIE, HeaderValue::from_static("tmeLoginType=-1;"));
    headers
}

/// Search QQ Music's public lite endpoint.
pub async fn search_qqmusic(
    keyword: &str,
    page: u64,
    page_size: u64,
) -> anyhow::Result<Vec<QqMusicSong>> {
    log::debug!("[lyrics][qq][search] requesting page={page} page_size={page_size}");
    let body = json!({
        "comm": {
            "ct": 11, "cv": "1003006", "v": "1003006", "os_ver": "15",
            "phonetype": "24122RKC7C", "tmeAppID": "qqmusiclight",
            "nettype": "NETWORK_WIFI", "udid": "0"
        },
        "request": {
            "method": "DoSearchForQQMusicLite",
            "module": "music.search.SearchCgiService",
            "param": {
                "query": keyword, "search_type": 0, "page_num": page,
                "num_per_page": page_size, "highlight": 0, "nqc_flag": 0,
                "page_id": 1, "grp": 1
            }
        }
    });

    let response = reqwest::Client::new()
        .post(BASE_URL)
        .headers(headers())
        .body(serde_json::to_vec(&body)?)
        .send()
        .await?;
    let status = response.status();
    let body = decode_http_body(&response.bytes().await?)?;
    if !status.is_success() {
        bail!(
            "QQ search HTTP {status}; body_prefix={:?}",
            response_preview(&body)
        );
    }
    let data = parse_json_response(&body, "search")?;

    let songs = data
        .get("request")
        .and_then(|v| v.get("data"))
        .and_then(|v| v.get("body"))
        .and_then(|v| v.get("item_song"))
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();

    let results: Vec<_> = songs
        .into_iter()
        .filter_map(|s| {
            let song_id = s.get("id")?.as_u64()?;
            let title = s
                .get("title")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_owned();
            let singer_name = s
                .get("singer")
                .and_then(Value::as_array)
                .map(|artists| {
                    artists
                        .iter()
                        .filter_map(|a| a.get("name").and_then(Value::as_str))
                        .filter(|name| !name.is_empty())
                        .collect::<Vec<_>>()
                        .join(" / ")
                })
                .unwrap_or_default();
            Some(QqMusicSong {
                song_id,
                mid: s
                    .get("mid")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_owned(),
                title,
                singer_name,
                album_name: s
                    .get("album")
                    .and_then(|v| v.get("name"))
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_owned(),
                duration: s
                    .get("interval")
                    .and_then(Value::as_u64)
                    .unwrap_or_default()
                    * 1000,
            })
        })
        .collect();
    log::info!("[lyrics][qq][search] results={}", results.len());
    eprintln!("[lyrics] QQ search results={}", results.len());
    Ok(results)
}

fn b64(value: &str) -> String {
    BASE64.encode(value.as_bytes())
}

fn normalize(value: &str) -> String {
    value
        .chars()
        .filter(|ch| ch.is_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}

fn match_score(song: &Song, candidate: &QqMusicSong) -> i32 {
    let title = normalize(&song.name);
    let candidate_title = normalize(&candidate.title);
    let mut score = if !title.is_empty() && title == candidate_title {
        60
    } else if !title.is_empty()
        && (!candidate_title.is_empty()
            && (candidate_title.contains(&title) || title.contains(&candidate_title)))
    {
        35
    } else {
        0
    };

    let wanted_artists: Vec<_> = song
        .artists
        .iter()
        .map(|artist| normalize(&artist.name))
        .filter(|artist| !artist.is_empty())
        .collect();
    let candidate_artists = normalize(&candidate.singer_name);
    if !wanted_artists.is_empty() {
        let matched = wanted_artists
            .iter()
            .filter(|artist| candidate_artists.contains(artist.as_str()))
            .count();
        if matched == wanted_artists.len() {
            score += 25;
        } else if matched > 0 {
            score += 12;
        }
    }

    let album = normalize(&song.album.name);
    let candidate_album = normalize(&candidate.album_name);
    if !album.is_empty() && album == candidate_album {
        score += 10;
    }

    if song.duration > 0 && candidate.duration > 0 {
        let difference = song.duration.abs_diff(candidate.duration);
        if difference <= 3_000 {
            score += 5;
        } else if difference <= 10_000 {
            score += 2;
        }
    }
    score
}

fn select_best_song(song: &Song, candidates: Vec<QqMusicSong>) -> anyhow::Result<QqMusicSong> {
    let mut ranked: Vec<_> = candidates
        .into_iter()
        .map(|candidate| (match_score(song, &candidate), candidate))
        .collect();
    ranked.sort_by(|left, right| right.0.cmp(&left.0));
    let (score, candidate) = ranked
        .into_iter()
        .next()
        .context("QQ Music song not found")?;
    log::info!(
        "[lyrics][qq][match] ncm_song_id={} qq_song_id={} score={} title={:?} singer={:?}",
        song.id,
        candidate.song_id,
        score,
        candidate.title,
        candidate.singer_name
    );
    eprintln!(
        "[lyrics] QQ match ncm_song_id={} qq_song_id={} score={} title={:?}",
        song.id, candidate.song_id, score, candidate.title
    );
    if score < 35 {
        bail!("QQ Music search match too weak: score={score}");
    }
    Ok(candidate)
}

fn decode_hex(value: &str) -> anyhow::Result<Vec<u8>> {
    let value = value.trim();
    if value.len() % 2 != 0 {
        bail!("QQ lyric has an odd-length hex payload");
    }
    (0..value.len())
        .step_by(2)
        .map(|i| {
            u8::from_str_radix(&value[i..i + 2], 16).with_context(|| "invalid QQ lyric hex payload")
        })
        .collect()
}

/*
 * QQ's lyric cipher is not a call to a platform DES API.  It is the small
 * DES implementation shipped by lx-music (the Python below is a direct
 * translation of deps/pyqdes/main.cpp).  In particular, bit_num() treats
 * every 32-bit word as little-endian while the Feistel state is big-endian.
 * Keeping that representation here is important: wrapping a normal DES-ECB
 * implementation and reversing bytes does not produce the same cipher.
 */
#[derive(Clone, Copy, PartialEq, Eq)]
enum DesMode {
    Encrypt,
    Decrypt,
}

const S_BOXES: [[u8; 64]; 8] = [
    [
        14, 4, 13, 1, 2, 15, 11, 8, 3, 10, 6, 12, 5, 9, 0, 7, 0, 15, 7, 4, 14, 2, 13, 1, 10, 6, 12,
        11, 9, 5, 3, 8, 4, 1, 14, 8, 13, 6, 2, 11, 15, 12, 9, 7, 3, 10, 5, 0, 15, 12, 8, 2, 4, 9,
        1, 7, 5, 11, 3, 14, 10, 0, 6, 13,
    ],
    [
        15, 1, 8, 14, 6, 11, 3, 4, 9, 7, 2, 13, 12, 0, 5, 10, 3, 13, 4, 7, 15, 2, 8, 15, 12, 0, 1,
        10, 6, 9, 11, 5, 0, 14, 7, 11, 10, 4, 13, 1, 5, 8, 12, 6, 9, 3, 2, 15, 13, 8, 10, 1, 3, 15,
        4, 2, 11, 6, 7, 12, 0, 5, 14, 9,
    ],
    [
        10, 0, 9, 14, 6, 3, 15, 5, 1, 13, 12, 7, 11, 4, 2, 8, 13, 7, 0, 9, 3, 4, 6, 10, 2, 8, 5,
        14, 12, 11, 15, 1, 13, 6, 4, 9, 8, 15, 3, 0, 11, 1, 2, 12, 5, 10, 14, 7, 1, 10, 13, 0, 6,
        9, 8, 7, 4, 15, 14, 3, 11, 5, 2, 12,
    ],
    [
        7, 13, 14, 3, 0, 6, 9, 10, 1, 2, 8, 5, 11, 12, 4, 15, 13, 8, 11, 5, 6, 15, 0, 3, 4, 7, 2,
        12, 1, 10, 14, 9, 10, 6, 9, 0, 12, 11, 7, 13, 15, 1, 3, 14, 5, 2, 8, 4, 3, 15, 0, 6, 10,
        10, 13, 8, 9, 4, 5, 11, 12, 7, 2, 14,
    ],
    [
        2, 12, 4, 1, 7, 10, 11, 6, 8, 5, 3, 15, 13, 0, 14, 9, 14, 11, 2, 12, 4, 7, 13, 1, 5, 0, 15,
        10, 3, 9, 8, 6, 4, 2, 1, 11, 10, 13, 7, 8, 15, 9, 12, 5, 6, 3, 0, 14, 11, 8, 12, 7, 1, 14,
        2, 13, 6, 15, 0, 9, 10, 4, 5, 3,
    ],
    [
        12, 1, 10, 15, 9, 2, 6, 8, 0, 13, 3, 4, 14, 7, 5, 11, 10, 15, 4, 2, 7, 12, 9, 5, 6, 1, 13,
        14, 0, 11, 3, 8, 9, 14, 15, 5, 2, 8, 12, 3, 7, 0, 4, 10, 1, 13, 11, 6, 4, 3, 2, 12, 9, 5,
        15, 10, 11, 14, 1, 7, 6, 0, 8, 13,
    ],
    [
        4, 11, 2, 14, 15, 0, 8, 13, 3, 12, 9, 7, 5, 10, 6, 1, 13, 0, 11, 7, 4, 9, 1, 10, 14, 3, 5,
        12, 2, 15, 8, 6, 1, 4, 11, 13, 12, 3, 7, 14, 10, 15, 6, 8, 0, 5, 9, 2, 6, 11, 13, 8, 1, 4,
        10, 7, 9, 5, 0, 15, 14, 2, 3, 12,
    ],
    [
        13, 2, 8, 4, 6, 15, 11, 1, 10, 9, 3, 14, 5, 0, 12, 7, 1, 15, 13, 8, 10, 3, 7, 4, 12, 5, 6,
        11, 0, 14, 9, 2, 7, 11, 4, 1, 9, 12, 14, 2, 0, 6, 10, 13, 15, 3, 5, 8, 2, 1, 14, 7, 4, 10,
        8, 13, 15, 12, 9, 0, 3, 5, 6, 11,
    ],
];

#[inline]
fn bit_num(a: &[u8], b: usize, c: usize) -> u32 {
    let byte_index = (b / 32) * 4 + 3 - (b % 32) / 8;
    let bit_position = 7 - (b % 8);
    (((a[byte_index] >> bit_position) & 1) as u32) << c
}

#[inline]
fn bit_num_int_r(a: u32, b: usize, c: usize) -> u32 {
    ((a >> (31 - b)) & 1) << c
}

#[inline]
fn bit_num_int_l(a: u32, b: usize, c: usize) -> u32 {
    (a.wrapping_shl(b as u32) & 0x8000_0000) >> c
}

#[inline]
fn s_box_bit(a: u8) -> usize {
    (((a & 0x20) | ((a & 0x1f) >> 1) | ((a & 1) << 4)) & 0x3f) as usize
}

fn initial_permutation(input: &[u8; 8]) -> [u32; 2] {
    let mut state = [0u32; 2];
    state[0] = bit_num(input, 57, 31)
        | bit_num(input, 49, 30)
        | bit_num(input, 41, 29)
        | bit_num(input, 33, 28)
        | bit_num(input, 25, 27)
        | bit_num(input, 17, 26)
        | bit_num(input, 9, 25)
        | bit_num(input, 1, 24)
        | bit_num(input, 59, 23)
        | bit_num(input, 51, 22)
        | bit_num(input, 43, 21)
        | bit_num(input, 35, 20)
        | bit_num(input, 27, 19)
        | bit_num(input, 19, 18)
        | bit_num(input, 11, 17)
        | bit_num(input, 3, 16)
        | bit_num(input, 61, 15)
        | bit_num(input, 53, 14)
        | bit_num(input, 45, 13)
        | bit_num(input, 37, 12)
        | bit_num(input, 29, 11)
        | bit_num(input, 21, 10)
        | bit_num(input, 13, 9)
        | bit_num(input, 5, 8)
        | bit_num(input, 63, 7)
        | bit_num(input, 55, 6)
        | bit_num(input, 47, 5)
        | bit_num(input, 39, 4)
        | bit_num(input, 31, 3)
        | bit_num(input, 23, 2)
        | bit_num(input, 15, 1)
        | bit_num(input, 7, 0);
    state[1] = bit_num(input, 56, 31)
        | bit_num(input, 48, 30)
        | bit_num(input, 40, 29)
        | bit_num(input, 32, 28)
        | bit_num(input, 24, 27)
        | bit_num(input, 16, 26)
        | bit_num(input, 8, 25)
        | bit_num(input, 0, 24)
        | bit_num(input, 58, 23)
        | bit_num(input, 50, 22)
        | bit_num(input, 42, 21)
        | bit_num(input, 34, 20)
        | bit_num(input, 26, 19)
        | bit_num(input, 18, 18)
        | bit_num(input, 10, 17)
        | bit_num(input, 2, 16)
        | bit_num(input, 60, 15)
        | bit_num(input, 52, 14)
        | bit_num(input, 44, 13)
        | bit_num(input, 36, 12)
        | bit_num(input, 28, 11)
        | bit_num(input, 20, 10)
        | bit_num(input, 12, 9)
        | bit_num(input, 4, 8)
        | bit_num(input, 62, 7)
        | bit_num(input, 54, 6)
        | bit_num(input, 46, 5)
        | bit_num(input, 38, 4)
        | bit_num(input, 30, 3)
        | bit_num(input, 22, 2)
        | bit_num(input, 14, 1)
        | bit_num(input, 6, 0);
    state
}

fn inverse_permutation(state: [u32; 2]) -> [u8; 8] {
    let (a, b) = (state[0], state[1]);
    let mut out = [0u8; 8];
    out[3] = (bit_num_int_r(b, 7, 7)
        | bit_num_int_r(a, 7, 6)
        | bit_num_int_r(b, 15, 5)
        | bit_num_int_r(a, 15, 4)
        | bit_num_int_r(b, 23, 3)
        | bit_num_int_r(a, 23, 2)
        | bit_num_int_r(b, 31, 1)
        | bit_num_int_r(a, 31, 0)) as u8;
    out[2] = (bit_num_int_r(b, 6, 7)
        | bit_num_int_r(a, 6, 6)
        | bit_num_int_r(b, 14, 5)
        | bit_num_int_r(a, 14, 4)
        | bit_num_int_r(b, 22, 3)
        | bit_num_int_r(a, 22, 2)
        | bit_num_int_r(b, 30, 1)
        | bit_num_int_r(a, 30, 0)) as u8;
    out[1] = (bit_num_int_r(b, 5, 7)
        | bit_num_int_r(a, 5, 6)
        | bit_num_int_r(b, 13, 5)
        | bit_num_int_r(a, 13, 4)
        | bit_num_int_r(b, 21, 3)
        | bit_num_int_r(a, 21, 2)
        | bit_num_int_r(b, 29, 1)
        | bit_num_int_r(a, 29, 0)) as u8;
    out[0] = (bit_num_int_r(b, 4, 7)
        | bit_num_int_r(a, 4, 6)
        | bit_num_int_r(b, 12, 5)
        | bit_num_int_r(a, 12, 4)
        | bit_num_int_r(b, 20, 3)
        | bit_num_int_r(a, 20, 2)
        | bit_num_int_r(b, 28, 1)
        | bit_num_int_r(a, 28, 0)) as u8;
    out[7] = (bit_num_int_r(b, 3, 7)
        | bit_num_int_r(a, 3, 6)
        | bit_num_int_r(b, 11, 5)
        | bit_num_int_r(a, 11, 4)
        | bit_num_int_r(b, 19, 3)
        | bit_num_int_r(a, 19, 2)
        | bit_num_int_r(b, 27, 1)
        | bit_num_int_r(a, 27, 0)) as u8;
    out[6] = (bit_num_int_r(b, 2, 7)
        | bit_num_int_r(a, 2, 6)
        | bit_num_int_r(b, 10, 5)
        | bit_num_int_r(a, 10, 4)
        | bit_num_int_r(b, 18, 3)
        | bit_num_int_r(a, 18, 2)
        | bit_num_int_r(b, 26, 1)
        | bit_num_int_r(a, 26, 0)) as u8;
    out[5] = (bit_num_int_r(b, 1, 7)
        | bit_num_int_r(a, 1, 6)
        | bit_num_int_r(b, 9, 5)
        | bit_num_int_r(a, 9, 4)
        | bit_num_int_r(b, 17, 3)
        | bit_num_int_r(a, 17, 2)
        | bit_num_int_r(b, 25, 1)
        | bit_num_int_r(a, 25, 0)) as u8;
    out[4] = (bit_num_int_r(b, 0, 7)
        | bit_num_int_r(a, 0, 6)
        | bit_num_int_r(b, 8, 5)
        | bit_num_int_r(a, 8, 4)
        | bit_num_int_r(b, 16, 3)
        | bit_num_int_r(a, 16, 2)
        | bit_num_int_r(b, 24, 1)
        | bit_num_int_r(a, 24, 0)) as u8;
    out
}

fn feistel(value: u32, key: &[u8; 6]) -> u32 {
    let t1 = bit_num_int_l(value, 31, 0)
        | ((value & 0xf000_0000) >> 1)
        | bit_num_int_l(value, 4, 5)
        | bit_num_int_l(value, 3, 6)
        | ((value & 0x0f00_0000) >> 3)
        | bit_num_int_l(value, 8, 11)
        | bit_num_int_l(value, 7, 12)
        | ((value & 0x00f0_0000) >> 5)
        | bit_num_int_l(value, 12, 17)
        | bit_num_int_l(value, 11, 18)
        | ((value & 0x000f_0000) >> 7)
        | bit_num_int_l(value, 16, 23);
    let t2 = bit_num_int_l(value, 15, 0)
        | ((value & 0x0000_f000) << 15)
        | bit_num_int_l(value, 20, 5)
        | bit_num_int_l(value, 19, 6)
        | ((value & 0x0000_0f00) << 13)
        | bit_num_int_l(value, 24, 11)
        | bit_num_int_l(value, 23, 12)
        | ((value & 0x0000_00f0) << 11)
        | bit_num_int_l(value, 28, 17)
        | bit_num_int_l(value, 27, 18)
        | ((value & 0x0000_000f) << 9)
        | bit_num_int_l(value, 0, 23);
    let six = [
        ((t1 >> 24) as u8),
        ((t1 >> 16) as u8),
        ((t1 >> 8) as u8),
        ((t2 >> 24) as u8),
        ((t2 >> 16) as u8),
        ((t2 >> 8) as u8),
    ];
    let mut x = [0u8; 6];
    for i in 0..6 {
        x[i] = six[i] ^ key[i];
    }
    let s = |n: usize, v: u8| -> u32 { (S_BOXES[n][s_box_bit(v)] as u32) << (28 - n * 4) };
    let state = s(0, x[0] >> 2)
        | s(1, (x[0] << 4) | (x[1] >> 4))
        | s(2, (x[1] << 2) | (x[2] >> 6))
        | s(3, x[2])
        | s(4, x[3] >> 2)
        | s(5, (x[3] << 4) | (x[4] >> 4))
        | s(6, (x[4] << 2) | (x[5] >> 6))
        | s(7, x[5]);
    bit_num_int_l(state, 15, 0)
        | bit_num_int_l(state, 6, 1)
        | bit_num_int_l(state, 19, 2)
        | bit_num_int_l(state, 20, 3)
        | bit_num_int_l(state, 28, 4)
        | bit_num_int_l(state, 11, 5)
        | bit_num_int_l(state, 27, 6)
        | bit_num_int_l(state, 16, 7)
        | bit_num_int_l(state, 0, 8)
        | bit_num_int_l(state, 14, 9)
        | bit_num_int_l(state, 22, 10)
        | bit_num_int_l(state, 25, 11)
        | bit_num_int_l(state, 4, 12)
        | bit_num_int_l(state, 17, 13)
        | bit_num_int_l(state, 30, 14)
        | bit_num_int_l(state, 9, 15)
        | bit_num_int_l(state, 1, 16)
        | bit_num_int_l(state, 7, 17)
        | bit_num_int_l(state, 23, 18)
        | bit_num_int_l(state, 13, 19)
        | bit_num_int_l(state, 31, 20)
        | bit_num_int_l(state, 26, 21)
        | bit_num_int_l(state, 2, 22)
        | bit_num_int_l(state, 8, 23)
        | bit_num_int_l(state, 18, 24)
        | bit_num_int_l(state, 12, 25)
        | bit_num_int_l(state, 29, 26)
        | bit_num_int_l(state, 5, 27)
        | bit_num_int_l(state, 21, 28)
        | bit_num_int_l(state, 10, 29)
        | bit_num_int_l(state, 3, 30)
        | bit_num_int_l(state, 24, 31)
}

fn key_schedule(key: &[u8; 16], mode: DesMode) -> [[u8; 6]; 16] {
    const SHIFTS: [usize; 16] = [1, 1, 2, 2, 2, 2, 2, 2, 1, 2, 2, 2, 2, 2, 2, 1];
    const PC1C: [usize; 28] = [
        56, 48, 40, 32, 24, 16, 8, 0, 57, 49, 41, 33, 25, 17, 9, 1, 58, 50, 42, 34, 26, 18, 10, 2,
        59, 51, 43, 35,
    ];
    const PC1D: [usize; 28] = [
        62, 54, 46, 38, 30, 22, 14, 6, 61, 53, 45, 37, 29, 21, 13, 5, 60, 52, 44, 36, 28, 20, 12,
        4, 27, 19, 11, 3,
    ];
    const PC2: [usize; 48] = [
        13, 16, 10, 23, 0, 4, 2, 27, 14, 5, 20, 9, 22, 18, 11, 3, 25, 7, 15, 6, 26, 19, 12, 1, 40,
        51, 30, 36, 46, 54, 29, 39, 50, 44, 32, 47, 43, 48, 38, 55, 33, 52, 45, 41, 49, 35, 28, 31,
    ];
    let mut c = 0u32;
    let mut d = 0u32;
    for i in 0..28 {
        c |= bit_num(key, PC1C[i], 31 - i);
        d |= bit_num(key, PC1D[i], 31 - i);
    }
    let mut schedule = [[0u8; 6]; 16];
    for i in 0..16 {
        c = ((c << SHIFTS[i]) | (c >> (28 - SHIFTS[i]))) & 0xffff_fff0;
        d = ((d << SHIFTS[i]) | (d >> (28 - SHIFTS[i]))) & 0xffff_fff0;
        let slot = if mode == DesMode::Decrypt { 15 - i } else { i };
        for j in 0..24 {
            schedule[slot][j / 8] |= bit_num_int_r(c, PC2[j], 7 - (j % 8)) as u8;
        }
        for j in 24..48 {
            schedule[slot][j / 8] |= bit_num_int_r(d, PC2[j] - 27, 7 - (j % 8)) as u8;
        }
    }
    schedule
}

fn des_block(input: &[u8; 8], schedule: &[[u8; 6]; 16]) -> [u8; 8] {
    let mut state = initial_permutation(input);
    for key in schedule.iter().take(15) {
        let t = state[1];
        state[1] = feistel(state[1], key) ^ state[0];
        state[0] = t;
    }
    state[0] = feistel(state[1], &schedule[15]) ^ state[0];
    inverse_permutation(state)
}

fn des_ecb(input: &[u8], key: &[u8; 16], decrypt: bool) -> anyhow::Result<Vec<u8>> {
    if input.len() % 8 != 0 {
        bail!("QQ lyric encrypted payload is not DES block aligned");
    }
    let schedule = key_schedule(
        key,
        if decrypt {
            DesMode::Decrypt
        } else {
            DesMode::Encrypt
        },
    );
    let mut output = Vec::with_capacity(input.len());
    for chunk in input.chunks_exact(8) {
        output.extend_from_slice(&des_block(chunk.try_into().unwrap(), &schedule));
    }
    Ok(output)
}

fn decompress_qq_payload(decoded: &[u8]) -> anyhow::Result<Vec<u8>> {
    let mut plain = Vec::new();
    if ZlibDecoder::new(decoded).read_to_end(&mut plain).is_ok() {
        return Ok(plain);
    }

    plain.clear();
    if GzDecoder::new(decoded).read_to_end(&mut plain).is_ok() {
        return Ok(plain);
    }

    plain.clear();
    if DeflateDecoder::new(decoded).read_to_end(&mut plain).is_ok() {
        return Ok(plain);
    }

    // A few QQ responses have returned an already-decoded XML/QRC payload.
    if decoded.starts_with(b"<") || decoded.starts_with(b"[") || decoded.starts_with(b"{") {
        return Ok(decoded.to_vec());
    }

    let prefix: String = decoded
        .iter()
        .take(16)
        .map(|byte| format!("{byte:02x}"))
        .collect();
    bail!("QQ lyric payload is not zlib/gzip/deflate; decrypted_prefix={prefix}")
}

fn decode_lyric(encrypted_hex: &str) -> anyhow::Result<String> {
    if encrypted_hex.trim().is_empty() {
        return Ok(String::new());
    }
    let mut decoded = decode_hex(encrypted_hex)?;
    let encrypted_len = decoded.len();
    decoded = des_ecb(&decoded, KEY1, true)?;
    decoded = des_ecb(&decoded, KEY2, false)?;
    decoded = des_ecb(&decoded, KEY3, true)?;

    let plain = decompress_qq_payload(&decoded).context("failed to decompress QQ lyric")?;
    let xml = String::from_utf8_lossy(&plain);
    if !xml.trim_start().starts_with('<') {
        log::debug!(
            "[lyrics][qq][decode] encrypted_bytes={encrypted_len} decoded_bytes={} format=plain",
            plain.len()
        );
        return Ok(xml.into_owned());
    }

    // The service wraps QRC in a LyricContent XML attribute.
    if let Some(start) = xml.find("LyricContent=\"") {
        let content_start = start + "LyricContent=\"".len();
        if let Some(end) = xml[content_start..].find('"') {
            let content = xml[content_start..content_start + end]
                .replace("&apos;", "'")
                .replace("&quot;", "\"")
                .replace("&lt;", "<")
                .replace("&gt;", ">")
                .replace("&amp;", "&")
                // XML attributes commonly encode QRC line breaks as numeric
                // entities. Without decoding these, the parser sees the
                // first entity before `[start,duration]` and rejects the
                // whole line.
                .replace("&#10;", "\n")
                .replace("&#xA;", "\n")
                .replace("&#x0A;", "\n")
                .replace("&#13;", "\r")
                .replace("&#xD;", "\r")
                .replace("&#x0D;", "\r");
            log::debug!(
                "[lyrics][qq][decode] encrypted_bytes={encrypted_len} decoded_bytes={} format=xml_attribute",
                content.len()
            );
            return Ok(content);
        }
    }
    log::debug!(
        "[lyrics][qq][decode] encrypted_bytes={encrypted_len} decoded_bytes={} format=xml",
        plain.len()
    );
    Ok(xml.into_owned())
}

/// Fetch and decode QQ lyric data for a searched song.
pub async fn fetch_qq_lyric(song: &QqMusicSong) -> anyhow::Result<LyricDetail> {
    log::debug!("[lyrics][qq] requesting song_id={}", song.song_id);
    let body = json!({
        "comm": {
            "_channelid": "", "_os_version": "6.2.9200-2", "authst": "", "ct": 11,
            "cv": "1003006", "patch": "118", "psrf_access_token_expiresAt": 0,
            "psrf_qqaccess_token": "", "psrf_qqopenid": "", "psrf_qqunionid": "",
            "tmeAppID": "qqmusiclight", "tmeLoginType": 0, "uin": "", "wid": ""
        },
        "music.musichallSong.PlayLyricInfo.GetPlayLyricInfo": {
            "method": "GetPlayLyricInfo", "module": "music.musichallSong.PlayLyricInfo",
            "param": {
                "albumName": b64(&song.album_name), "crypt": 1, "ct": 19, "cv": 2111,
                "interval": song.duration / 1000, "lrc_t": 0, "qrc": 1, "qrc_t": 0,
                "roma": 0, "roma_t": 0, "singerName": b64(&song.singer_name),
                "songID": song.song_id, "songName": b64(&song.title), "trans": 1,
                "trans_t": 0, "type": 0
            }
        }
    });
    let response = reqwest::Client::new()
        .post(BASE_URL)
        .headers(headers())
        .body(serde_json::to_vec(&body)?)
        .send()
        .await?;
    let status = response.status();
    let body = decode_http_body(&response.bytes().await?)?;
    if !status.is_success() {
        bail!(
            "QQ lyric HTTP {status}; body_prefix={:?}",
            response_preview(&body)
        );
    }
    let data = parse_json_response(&body, "lyric")?;
    let data = data
        .get("music.musichallSong.PlayLyricInfo.GetPlayLyricInfo")
        .and_then(|v| v.get("data"))
        .context("QQ lyric response has no data")?;
    let lyric = decode_lyric(
        data.get("lyric")
            .and_then(Value::as_str)
            .unwrap_or_default(),
    )?;
    let trans = decode_lyric(
        data.get("trans")
            .and_then(Value::as_str)
            .unwrap_or_default(),
    )?;
    let pure = lyric.trim().is_empty();
    log::info!(
        "[lyrics][qq] song_id={} lyric_bytes={} translation_bytes={} word_timed={}",
        song.song_id,
        lyric.len(),
        trans.len(),
        lyric
            .lines()
            .any(|line| line.contains(',') && line.contains('('))
    );
    Ok(LyricDetail {
        // Keep the decoded payload in both slots: QRC uses `yrc`, while some
        // QQ responses fall back to ordinary LRC and need `lyric` parsing.
        lyric: (!lyric.is_empty()).then_some(lyric.clone()),
        tlyric: None,
        yrc: (!lyric.is_empty()).then_some(lyric),
        ytlrc: (!trans.is_empty()).then_some(trans),
        is_pure_music: pure,
    })
}

/// Search by the NCM song metadata and fetch the closest QQ result.
pub async fn fetch_qq_lyric_for_song(song: &Song) -> anyhow::Result<LyricDetail> {
    log::info!("[lyrics][qq] searching for ncm_song_id={}", song.id);
    let singers = song
        .artists
        .iter()
        .map(|a| a.name.as_str())
        .collect::<Vec<_>>()
        .join(" ");
    let keyword = if singers.is_empty() {
        song.name.clone()
    } else {
        format!("{} {}", song.name, singers)
    };
    let results = search_qqmusic(&keyword, 1, 10).await?;
    let result = select_best_song(song, results)?;
    fetch_qq_lyric(&result).await
}

#[cfg(test)]
mod tests {
    use super::{decode_hex, des_ecb, fetch_qq_lyric_for_song};
    use crate::api::{Artist, Song};
    use crate::utils::lyric_parse::parse_lyric;

    #[test]
    fn hex_decoder_rejects_invalid_input() {
        assert!(decode_hex("0").is_err());
        assert!(decode_hex("gg").is_err());
        assert_eq!(decode_hex("00ff").unwrap(), vec![0, 255]);
    }

    #[test]
    fn reference_des_roundtrip() {
        let key16 = *b"!@#)(NHLiuy*$%^&";
        let plain = decode_hex("0123456789abcdef0011223344556677").unwrap();
        let encrypted = des_ecb(&plain, &key16, false).unwrap();
        assert_ne!(encrypted, plain);
        assert_eq!(des_ecb(&encrypted, &key16, true).unwrap(), plain);
    }

    /// Live regression test for the song from the reported zero-line log.
    /// Run explicitly with `cargo test -- --ignored --nocapture` when network
    /// access to QQ Music is available.
    #[tokio::test]
    #[ignore = "requires live QQ Music network access"]
    async fn live_qrc_song_28240362_parses_lines() {
        let song = Song {
            id: 28_240_362,
            name: "ebb and flow (潮起潮落)".into(),
            artists: vec![Artist {
                name: "Ray".into(),
                ..Default::default()
            }],
            ..Default::default()
        };
        let detail = fetch_qq_lyric_for_song(&song).await.unwrap();
        let lines = parse_lyric(&detail).expect("QQ QRC should produce parsed lines");
        assert!(!lines.is_empty(), "QQ QRC parser returned zero lines");
        assert!(detail.yrc.as_deref().is_some_and(|raw| raw.contains('(')));
    }
}
