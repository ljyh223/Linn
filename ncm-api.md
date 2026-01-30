# 网易云音乐 Rust API 完整文档

基于 `netease-cloud-music-api` v1.5.2

## 目录

- [1. 初始化和配置](#1-初始化和配置)
- [2. 登录相关 API](#2-登录相关-api)
- [3. 用户相关 API](#3-用户相关-api)
- [4. 歌单和歌曲相关 API](#4-歌单和歌曲相关-api)
- [5. 推荐相关 API](#5-推荐相关-api)
- [6. 收藏/操作相关 API](#6-收藏操作相关-api)
- [7. 搜索相关 API](#7-搜索相关-api)
- [8. 歌手相关 API](#8-歌手相关-api)
- [9. 专辑相关 API](#9-专辑相关-api)
- [10. 排行榜和热门歌单 API](#10-排行榜和热门歌单-api)
- [11. 首页相关 API](#11-首页相关-api)
- [12. 电台相关 API](#12-电台相关-api)
- [13. 下载相关 API](#13-下载相关-api)
- [完整使用示例](#完整使用示例)

---

## 1. 初始化和配置

### 1.1 默认初始化

```rust
use netease_cloud_music_api::MusicApi;

let api = MusicApi::default();
```

### 1.2 指定最大连接数

```rust
let api = MusicApi::new(10);
```

### 1.3 从 Cookie Jar 初始化

```rust
use isahc::cookies::CookieJar;

let cookie_jar = CookieJar::new();
let api = MusicApi::from_cookie_jar(cookie_jar, 10);
```

### 1.4 获取 Cookie Jar

```rust
let jar = api.cookie_jar();
// 返回: Option<&CookieJar>
```

### 1.5 设置代理

```rust
api.set_proxy("socks5://127.0.0.1:1080")?;
// 返回: Result<()>
// 支持协议: http, https, socks4, socks4a, socks5, socks5h
```

---

## 2. 登录相关 API

### 2.1 账号密码登录

支持邮箱或手机号（11位数字）

```rust
let login_info = api.login(
    "your_email@example.com".to_string(),
    "your_password".to_string()
).await?;
// 返回: Result<LoginInfo>
// LoginInfo 包含: code, uid, nickname, avatar_url, vip_type, msg
```

### 2.2 获取验证码

用于手机号登录

```rust
api.captcha(
    "86".to_string(),              // 国家码
    "13800138000".to_string()      // 手机号
).await?;
// 返回: Result<()>
```

### 2.3 手机号验证码登录

```rust
let login_info = api.login_cellphone(
    "86".to_string(),               // 国家码
    "13800138000".to_string(),      // 手机号
    "123456".to_string()            // 验证码
).await?;
// 返回: Result<LoginInfo>
```

### 2.4 创建登录二维码

```rust
let (qr_url, unikey) = api.login_qr_create().await?;
// 返回: Result<(String, String)>
// - qr_url: 二维码链接
// - unikey: 用于检查二维码状态的key
```

### 2.5 检查登录二维码状态

```rust
let msg = api.login_qr_check(unikey).await?;
// 返回: Result<Msg>
// Msg.code: 801=等待扫码, 802=授权中, 803=成功
```

### 2.6 检查登录状态

```rust
let login_info = api.login_status().await?;
// 返回: Result<LoginInfo>
```

### 2.7 退出登录

```rust
api.logout().await;
// 返回: ()
```

---

## 3. 用户相关 API

### 3.1 每日签到

```rust
let msg = api.daily_task().await?;
// 返回: Result<Msg>
```

### 3.2 用户喜欢音乐ID列表

```rust
let song_ids = api.user_song_id_list(12345678).await?;
// 返回: Result<Vec<u64>>
```

### 3.3 用户歌单

```rust
let song_lists = api.user_song_list(
    12345678,    // uid: 用户ID
    0,           // offset: 起始位置
    20           // limit: 数量
).await?;
// 返回: Result<Vec<SongList>>
```

### 3.4 用户收藏专辑列表

```rust
let albums = api.album_sublist(
    0,    // offset: 起始位置
    20    // limit: 数量
).await?;
// 返回: Result<Vec<SongList>>
```

### 3.5 用户云盘

```rust
let songs = api.user_cloud_disk().await?;
// 返回: Result<Vec<SongInfo>>
```

---

## 4. 歌单和歌曲相关 API

### 4.1 歌单详情

```rust
let playlist_detail = api.song_list_detail(12345678).await?;
// 返回: Result<PlayListDetail>
// PlayListDetail 包含: id, name, cover_img_url, description,
//                      create_time, track_update_time, songs
```

### 4.2 歌曲详情

```rust
let songs = api.songs_detail(&[12345, 67890]).await?;
// 返回: Result<Vec<SongInfo>>
```

### 4.3 歌曲URL

获取歌曲播放链接

```rust
let song_urls = api.songs_url(
    &[12345, 67890],    // ids: 歌曲ID列表
    "320000"             // br: 码率
).await?;
// 返回: Result<Vec<SongUrl>>
// SongUrl 包含: id, url, rate
// 码率选项:
// - "128000"  - 标音质
// - "192000"  - 较高音质
// - "320000"  - 高音质
// - "999000"  - 无损音质
// - "1900000" - 高无损音质
```

### 4.4 歌词

```rust
let lyrics = api.song_lyric(12345).await?;
// 返回: Result<Lyrics>
// Lyrics 包含:
// - lyric: Vec<String> - 歌词
// - tlyric: Vec<String> - 翻译
```

### 4.5 歌单动态信息

```rust
let dynamic = api.songlist_detail_dynamic(12345678).await?;
// 返回: Result<PlayListDetailDynamic>
// PlayListDetailDynamic 包含:
// - subscribed: bool - 是否已收藏
// - booked_count: u64 - 收藏数
// - play_count: u64 - 播放数
// - comment_count: u64 - 评论数
```

---

## 5. 推荐相关 API

### 5.1 每日推荐歌单

```rust
let playlists = api.recommend_resource().await?;
// 返回: Result<Vec<SongList>>
```

### 5.2 每日推荐歌曲

```rust
let songs = api.recommend_songs().await?;
// 返回: Result<Vec<SongInfo>>
```

### 5.3 私人FM

```rust
let songs = api.personal_fm().await?;
// 返回: Result<Vec<SongInfo>>
```

### 5.4 心动模式/智能播放

```rust
let songs = api.playmode_intelligence_list(
    12345,    // sid: 歌曲ID
    67890     // pid: 歌单ID
).await?;
// 返回: Result<Vec<SongInfo>>
```

---

## 6. 收藏/操作相关 API

### 6.1 收藏/取消收藏歌曲

```rust
let success = api.like(
    true,     // like: true收藏, false取消
    12345     // songid: 歌曲ID
).await;
// 返回: bool
```

### 6.2 FM不喜欢

```rust
let success = api.fm_trash(12345).await;
// 返回: bool
```

### 6.3 收藏/取消收藏歌单

```rust
let success = api.song_list_like(
    true,     // like: true收藏, false取消
    12345     // id: 歌单ID
).await;
// 返回: bool
```

### 6.4 收藏/取消收藏专辑

```rust
let success = api.album_like(
    true,     // like: true收藏, false取消
    12345     // id: 专辑ID
).await;
// 返回: bool
```

---

## 7. 搜索相关 API

### 7.1 通用搜索

```rust
let result = api.search(
    "周杰伦".to_string(),    // keywords: 关键词
    1,                       // types: 类型
    0,                       // offset: 起始位置
    20                       // limit: 数量
).await?;
// 返回: Result<String>
// types 可选值:
// - 1: 单曲
// - 10: 专辑
// - 100: 歌手
// - 1000: 歌单
// - 1002: 用户
// - 1004: MV
// - 1006: 歌词
// - 1009: 电台
// - 1014: 视频
```

### 7.2 搜索单曲

```rust
let songs = api.search_song(
    "周杰伦".to_string(),    // keywords: 关键词
    0,                       // offset: 起始位置
    20                       // limit: 数量
).await?;
// 返回: Result<Vec<SongInfo>>
```

### 7.3 搜索歌手

```rust
let singers = api.search_singer(
    "周杰伦".to_string(),    // keywords: 关键词
    0,                       // offset: 起始位置
    20                       // limit: 数量
).await?;
// 返回: Result<Vec<SingerInfo>>
// SingerInfo 包含: id, name, pic_url
```

### 7.4 搜索专辑

```rust
let albums = api.search_album(
    "周杰伦".to_string(),    // keywords: 关键词
    0,                       // offset: 起始位置
    20                       // limit: 数量
).await?;
// 返回: Result<Vec<SongList>>
```

### 7.5 搜索歌单

```rust
let playlists = api.search_songlist(
    "周杰伦".to_string(),    // keywords: 关键词
    0,                       // offset: 起始位置
    20                       // limit: 数量
).await?;
// 返回: Result<Vec<SongList>>
```

### 7.6 搜索歌词

```rust
let songs = api.search_lyrics(
    "晴天".to_string(),      // keywords: 关键词
    0,                       // offset: 起始位置
    20                       // limit: 数量
).await?;
// 返回: Result<Vec<SongInfo>>
```

---

## 8. 歌手相关 API

### 8.1 获取歌手热门单曲

```rust
let songs = api.singer_songs(6452).await?;
// 返回: Result<Vec<SongInfo>>
// 参数: 歌手ID (例如周杰伦=6452)
```

### 8.2 获取歌手全部单曲

```rust
let songs = api.singer_all_songs(
    6452,           // id: 歌手ID
    "hot",          // order: "hot"热门 或 "time"时间
    0,              // offset: 起始位置
    20              // limit: 数量
).await?;
// 返回: Result<Vec<SongInfo>>
```

---

## 9. 专辑相关 API

### 9.1 全部新碟

```rust
let albums = api.new_albums(
    "ALL",    // area: 地区
    0,        // offset: 起始位置
    20        // limit: 数量
).await?;
// 返回: Result<Vec<SongList>>
// area 可选值:
// - "ALL" - 全部
// - "ZH"  - 华语
// - "EA"  - 欧美
// - "KR"  - 韩国
// - "JP"  - 日本
```

### 9.2 专辑详情

```rust
let album_detail = api.album(12345).await?;
// 返回: Result<AlbumDetail>
// AlbumDetail 包含: id, name, pic_url, description, publish_time,
//                   artist_id, artist_name, artist_pic_url, songs
```

### 9.3 专辑动态信息

```rust
let dynamic = api.album_detail_dynamic(12345).await?;
// 返回: Result<AlbumDetailDynamic>
// AlbumDetailDynamic 包含:
// - is_sub: bool - 是否已收藏
// - sub_count: u64 - 收藏数
// - comment_count: u64 - 评论数
```

---

## 10. 排行榜和热门歌单 API

### 10.1 获取排行榜列表

```rust
let toplist = api.toplist().await?;
// 返回: Result<Vec<TopList>>
// TopList 包含: id, name, update, description, cover
```

### 10.2 热门歌曲/排行榜

```rust
let songs = api.top_songs(3778678).await?;
// 返回: Result<PlayListDetail>

// 常用榜单ID:
// 云音乐飙升榜:     19723756
// 云音乐新歌榜:     3779629
// 网易原创歌曲榜:   2884035
// 云音乐热歌榜:     3778678
// 云音乐古典音乐榜: 71384707
// 云音乐ACG音乐榜:  71385702
// 云音乐韩语榜:     745956260
// 云音乐嘻哈榜:     991319590
// 抖音排行榜:       2250011882
// UK排行榜周榜:     180106
// 美国Billboard周榜: 60198
// KTV嗨榜:          21845217
// iTunes榜:         11641012
// Hit FM Top榜:     120001
// 日本Oricon周榜:   60131
// 台湾Hito排行榜:   112463
// 香港电台中文歌曲龙虎榜: 10169002
// 华语金曲榜:       4395559
```

### 10.3 热门推荐歌单

```rust
let playlists = api.top_song_list(
    "华语",    // cat: 分类
    "hot",     // order: "hot"热门 或 "new"最新
    0,         // offset: 起始位置
    20         // limit: 数量
).await?;
// 返回: Result<Vec<SongList>>

// cat 可选值:
// - "全部", "华语", "欧美", "日语", "韩语", "粤语", "小语种"
// - "流行", "摇滚", "民谣", "电子", "舞曲", "说唱", "轻音乐"
// - "爵士", "乡村", "R&B/Soul", "古典", "民族", "英伦"
// - "金属", "朋克", "蓝调", "雷鬼", "世界音乐", "拉丁"
// - "另类/独立", "New Age", "古风", "后摇", "Bossa Nova"
// - "清晨", "夜晚", "学习", "工作", "午休", "下午茶"
// - "地铁", "驾车", "运动", "旅行", "散步", "酒吧"
// - "怀旧", "清新", "浪漫", "性感", "伤感", "治愈"
// - "放松", "孤独", "感动", "兴奋", "快乐", "安静"
// - "思念", "影视原声", "ACG", "儿童", "校园", "游戏"
// - "70后", "80后", "90后", "网络歌曲", "KTV", "经典"
// - "翻唱", "吉他", "钢琴", "器乐", "榜单", "00后"
```

### 10.4 精品歌单

```rust
let playlists = api.top_song_list_highquality(
    "华语",    // cat: 分类
    0,         // lasttime: 分页参数，首次传0
    20         // limit: 数量
).await?;
// 返回: Result<Vec<SongList>>

// cat 可选值:
// "全部", "华语", "欧美", "韩语", "日语", "粤语", "小语种",
// "运动", "ACG", "影视原声", "流行", "摇滚", "后摇", "古风",
// "民谣", "轻音乐", "电子", "器乐", "说唱", "古典", "爵士"
```

---

## 11. 首页相关 API

### 11.1 获取APP首页信息

```rust
let result = api.homepage(
    ClientType::Pc    // client_type: 客户端类型
).await?;
// 返回: Result<String>

// ClientType 可选值:
// - ClientType::Pc
// - ClientType::Android
// - ClientType::Iphone
// - ClientType::Ipad
```

### 11.2 获取首页轮播图

```rust
let banners = api.banners().await?;
// 返回: Result<Vec<BannersInfo>>
// BannersInfo 包含: pic, target_id, target_type
// target_type: Song(1), Album(10), Unknown
```

---

## 12. 电台相关 API

### 12.1 用户电台订阅列表

```rust
let radios = api.user_radio_sublist(
    0,    // offset: 起始位置
    20    // limit: 数量
).await?;
// 返回: Result<Vec<SongList>>
```

### 12.2 电台节目列表

```rust
let programs = api.radio_program(
    12345,    // rid: 电台ID
    0,        // offset: 起始位置
    20        // limit: 数量
).await?;
// 返回: Result<Vec<SongInfo>>
```

---

## 13. 下载相关 API

### 13.1 下载图片

```rust
use std::path::PathBuf;

api.download_img(
    "https://example.com/pic.jpg",                // url: 图片URL
    PathBuf::from("/path/to/save.jpg"),           // path: 保存路径
    300,                                           // width: 宽度
    300                                            // high: 高度
).await?;
// 返回: Result<()>
```

### 13.2 下载音乐

```rust
api.download_song(
    "https://example.com/song.mp3",               // url: 音乐URL
    PathBuf::from("/path/to/save.mp3")            // path: 保存路径
).await?;
// 返回: Result<()>
```

---

## 完整使用示例

```rust
use netease_cloud_music_api::{MusicApi, ClientType};
use async_std::main;

#[main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. 创建API实例
    let api = MusicApi::default();

    // 可选: 设置代理
    // api.set_proxy("socks5://127.0.0.1:1080")?;

    // 2. 登录（可选，某些功能需要登录）
    let login_info = api.login(
        "your_email@example.com".to_string(),
        "your_password".to_string()
    ).await?;
    println!("登录成功: {}", login_info.nickname);

    // 3. 搜索歌曲
    let songs = api.search_song("周杰伦".to_string(), 0, 10).await?;
    println!("\n=== 搜索结果 ===");
    for song in songs {
        println!("{} - {}", song.name, song.singer);
    }

    // 4. 获取每日推荐
    let recommended = api.recommend_songs().await?;
    println!("\n=== 每日推荐 ({} 首) ===", recommended.len());
    for song in recommended.iter().take(5) {
        println!("{} - {}", song.name, song.singer);
    }

    // 5. 获取排行榜
    let top_songs = api.top_songs(3778678).await?; // 云音乐热歌榜
    println!("\n=== 云音乐热歌榜 ({} 首) ===", top_songs.songs.len());
    for song in top_songs.songs.iter().take(5) {
        println!("{} - {}", song.name, song.singer);
    }

    // 6. 获取歌单详情
    if let Some(playlist) = api.user_song_list(login_info.uid, 0, 1).await?.first() {
        let playlist_detail = api.song_list_detail(playlist.id).await?;
        println!("\n=== 歌单: {} ===", playlist_detail.name);
        println!("描述: {}", playlist_detail.description);
        println!("歌曲数: {}", playlist_detail.songs.len());

        // 7. 获取第一首歌的URL和歌词
        if let Some(song) = playlist_detail.songs.first() {
            println!("\n第一首歌: {}", song.name);

            // 获取播放URL
            let urls = api.songs_url(&[song.id], "320000").await?;
            if let Some(url) = urls.first() {
                println!("播放链接: {}", url.url);
            }

            // 获取歌词
            let lyrics = api.song_lyric(song.id).await?;
            println!("歌词行数: {}", lyrics.lyric.len());
            if !lyrics.lyric.is_empty() {
                println!("第一句歌词: {}", lyrics.lyric.first().unwrap());
            }
        }
    }

    // 8. 获取歌手信息
    let singer_songs = api.singer_songs(6452).await?; // 周杰伦
    println!("\n=== 周杰伦热门歌曲 ({} 首) ===", singer_songs.len());
    for song in singer_songs.iter().take(5) {
        println!("{} - {}", song.name, song.album);
    }

    // 9. 获取新碟
    let new_albums = api.new_albums("ZH", 0, 10).await?;
    println!("\n=== 华语新碟 ({} 张) ===", new_albums.len());
    for album in new_albums.iter().take(5) {
        println!("{} - {}", album.name, album.author);
    }

    // 10. 热门歌单
    let hot_playlists = api.top_song_list("流行", "hot", 0, 5).await?;
    println!("\n=== 流行热门歌单 ({} 个) ===", hot_playlists.len());
    for playlist in hot_playlists {
        println!("{} - {}", playlist.name, playlist.author);
    }

    // 11. 私人FM
    let fm_songs = api.personal_fm().await?;
    println!("\n=== 私人FM ({} 首) ===", fm_songs.len());
    for song in fm_songs.iter().take(5) {
        println!("{} - {}", song.name, song.singer);
    }

    // 12. 每日签到
    let msg = api.daily_task().await?;
    println!("\n=== 每日签到 ===");
    if msg.code == 200 {
        println!("签到成功!");
    } else {
        println!("签到信息: {}", msg.msg);
    }

    Ok(())
}
```

---

## 项目依赖配置

在 `Cargo.toml` 中添加：

```toml
[dependencies]
netease-cloud-music-api = "1.5.2"
async-std = { version = "1", features = ["attributes"] }
anyhow = "1"
```

---

## 数据结构说明

### LoginInfo
```rust
pub struct LoginInfo {
    pub code: i32,           // 登录状态码
    pub uid: u64,            // 用户ID
    pub nickname: String,    // 用户昵称
    pub avatar_url: String,  // 用户头像
    pub vip_type: i32,       // VIP等级 (0=普通, 11=黑胶)
    pub msg: String,         // 状态消息
}
```

### SongInfo
```rust
pub struct SongInfo {
    pub id: u64,                  // 歌曲ID
    pub name: String,             // 歌名
    pub singer: String,           // 歌手
    pub album: String,            // 专辑
    pub album_id: u64,            // 专辑ID
    pub pic_url: String,          // 封面图
    pub duration: u64,            // 歌曲时长
    pub song_url: String,         // 歌曲链接
    pub copyright: SongCopyright, // 版权信息
}
```

### SongList
```rust
pub struct SongList {
    pub id: u64,              // 歌单/专辑ID
    pub name: String,         // 名称
    pub cover_img_url: String, // 封面
    pub author: String,       // 作者
}
```

### SongUrl
```rust
pub struct SongUrl {
    pub id: u64,       // 歌曲ID
    pub url: String,   // 播放URL
    pub rate: u32,     // 码率
}
```

### PlayListDetail
```rust
pub struct PlayListDetail {
    pub id: u64,                  // 歌单ID
    pub name: String,             // 歌单名
    pub cover_img_url: String,    // 封面
    pub description: String,      // 描述
    pub create_time: u64,         // 创建时间
    pub track_update_time: u64,   // 更新时间
    pub songs: Vec<SongInfo>,     // 歌曲列表
}
```

### Lyrics
```rust
pub struct Lyrics {
    pub lyric: Vec<String>,    // 歌词
    pub tlyric: Vec<String>,   // 翻译
}
```

---

## 注意事项

1. **异步运行时**: 本库使用异步函数，需要配合 `async-std` 或 `tokio` 使用
2. **登录要求**: 部分功能需要登录后才能使用（如每日推荐、收藏等）
3. **请求频率**: 建议控制请求频率，避免被限制
4. **代理支持**: 可以通过 `set_proxy` 方法设置代理
5. **Cookie管理**: 库会自动管理Cookie，登录后可以保持会话
6. **错误处理**: 所有API返回 `Result` 类型，需要正确处理错误