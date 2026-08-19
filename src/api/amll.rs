//! AMLL (Apple Music style) TTML lyric source for Netease song IDs.

const BASE_URL: &str = "https://amlldb.bikonoo.com/ncm-lyrics";

pub async fn fetch_amll_ttml(id: u64) -> anyhow::Result<Option<String>> {
    let url = format!("{BASE_URL}/{id}.ttml");
    log::debug!("[lyrics][amll] requesting song_id={id}");
    let response = reqwest::get(url).await?;
    let status = response.status();
    if status.as_u16() == 404 || !status.is_success() {
        log::debug!("[lyrics][amll] unavailable song_id={id} status={status}");
        return Ok(None);
    }
    let body = response.bytes().await?;
    let text = String::from_utf8_lossy(&body).trim().to_string();
    if text.is_empty() || text == "歌词不存在" {
        log::debug!("[lyrics][amll] empty song_id={id} bytes={}", body.len());
        Ok(None)
    } else {
        log::info!("[lyrics][amll] found song_id={id} bytes={}", body.len());
        Ok(Some(text))
    }
}
