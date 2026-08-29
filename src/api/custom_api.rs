use std::future::Future;

use chrono::{Local, Utc};
use ncm_api_rs::error::Result;
use ncm_api_rs::{ApiClient, ApiResponse, CryptoType, Query, RequestOption};
use serde_json::json;

pub trait ApiClientExt {
    fn home_recommend_resource(
        &self,
        query: &Query,
    ) -> impl Future<Output = Result<ApiResponse>> + Send;
    fn home_category_daily_song_list(
        &self,
        query: &Query,
    ) -> impl Future<Output = Result<ApiResponse>> + Send;

    /// 新版移动端首页推荐资源流（EAPI）。
    fn home_recommend_resource_show(
        &self,
        query: &Query,
    ) -> impl Future<Output = Result<ApiResponse>> + Send;

    fn pc_recent_listen_list(
        &self,
        query: &Query,
    ) -> impl Future<Output = Result<ApiResponse>> + Send;
}

impl ApiClientExt for ApiClient {
    fn home_recommend_resource(
        &self,
        query: &Query,
    ) -> impl Future<Output = Result<ApiResponse>> + Send {
        let client_time = query.get_or(
            "clientTime",
            &Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
        );

        let data = json!({
            "clientTime": client_time
        });

        let option = RequestOption {
            crypto: CryptoType::Weapi,
            cookie: query.cookie.clone(),
            ua: query.ua.clone(),
            proxy: query.proxy.clone(),
            real_ip: query.real_ip.clone(),
            random_cn_ip: query.random_cn_ip,
            e_r: query.e_r,
            domain: query.domain.clone(),
            check_token: false,
        };
        self.request("/api/pc/daily/rcmd/block", data, option)
    }

    fn home_category_daily_song_list(
        &self,
        query: &Query,
    ) -> impl Future<Output = Result<ApiResponse>> + Send {
        let category_id = query.get_or("category_id", "1000");
        let tag_id = query.get_or("tag_id", "10015");
        let song_ids = query.get_or("song_ids", "");

        let data = json!({
            "source": "homepage",
            "categoryId": category_id,
            "tagId": tag_id,
            "songId": song_ids
        });

        let option = RequestOption {
            crypto: CryptoType::Weapi,
            cookie: query.cookie.clone(),
            ua: query.ua.clone(),
            proxy: query.proxy.clone(),
            real_ip: query.real_ip.clone(),
            random_cn_ip: query.random_cn_ip,
            e_r: query.e_r,
            domain: query.domain.clone(),
            check_token: true,
        };

        self.request("/api/homepage/category/daily/song/list", data, option)
    }

    fn home_recommend_resource_show(
        &self,
        query: &Query,
    ) -> impl Future<Output = Result<ApiResponse>> + Send {
        let now = Utc::now();
        let is_first_screen = query.get_or("is_first_screen", "true") == "true";
        let ext_json = json!({
            "refreshAction": if is_first_screen { "pull" } else { "loadMore" },
            "firstRequestPerLaunch": false,
            "carrier": "",
            "forceFreshForNewUser": false,
            "homeReqSource": "home",
            "currentExploreHomeType": "main",
            "currentNewUserExploreHomeType": "main",
            "homeFrameworkType": "fastPlay",
            "adSceneExt": "",
            "clientMobileSize": "{\"width\":1272,\"length\":2603}",
            "clientMobileSizeDp": "{\"width\":363.42856,\"length\":743.7143}",
            "noteHomeType": "note",
            "showSearchStartDragonBall": false,
            "fmName": { "fmTitle": "漫游", "fmLongTitle": "私人漫游" }
        })
        .to_string();
        let loaded_positions = query.get_or("loaded_position_codes", "");
        let client_cache_blocks = json!([
            "PAGE_RECOMMEND_RANK",
            "PAGE_RECOMMEND_RADAR",
            "PAGE_RECOMMEND_SPECIAL_CLOUD_VILLAGE_PLAYLIST",
            "PAGE_RECOMMEND_NEW_SONG_AND_ALBUM",
            "PAGE_RECOMMEND_RED_SIMILAR_SONG",
            "PAGE_RECOMMEND_PODCAST_RADIO_PROGRAM",
            "PAGE_RECOMMEND_PRIVATE_RCMD_SONG",
            "PAGE_RECOMMEND_SCENE_PLAYLIST_LOCATION",
            "PAGE_RECOMMEND_FEELING_PLAYLIST_LOCATION",
            "PAGE_RECOMMEND_COMBINATION",
            "PAGE_RECOMMEND_DAILY_RECOMMEND"
        ])
        .to_string();
        let alg_demote_blocks = json!([
            "PAGE_RECOMMEND_DAILY_RECOMMEND",
            "PAGE_RECOMMEND_VIP_SMALL_CARD",
            "PAGE_RECOMMEND_VIP_MODULE",
            "PAGE_RECOMMEND_BANNER_6",
            "PAGE_RECOMMEND_RADAR",
            "PAGE_RECOMMEND_BANNER_1",
            "PAGE_RECOMMEND_FEELING_PLAYLIST_LOCATION",
            "PAGE_RECOMMEND_SPECIAL_CLOUD_VILLAGE_PLAYLIST",
            "PAGE_RECOMMEND_SHORTCUT",
            "PAGE_RECOMMEND_SCENE_PLAYLIST_LOCATION",
            "PAGE_RECOMMEND_RED_SIMILAR_SONG",
            "PAGE_RECOMMEND_PRIVATE_RCMD_SONG",
            "PAGE_RECOMMEND_SURVEY",
            "PAGE_RECOMMEND_NEW_SONG_AND_ALBUM",
            "PAGE_RECOMMEND_RANK",
            "PAGE_RECOMMEND_MONTH_YEAR_PLAYLIST",
            "PAGE_RECOMMEND_ARTIST_TREND",
            "PAGE_RECOMMEND_PODCAST_RADIO_PROGRAM"
        ])
        .to_string();
        let next_page_block_order = json!([
            "PAGE_RECOMMEND_COMBINATION",
            "PAGE_RECOMMEND_SPECIAL_CLOUD_VILLAGE_PLAYLIST",
            "PAGE_RECOMMEND_SURVEY",
            "PAGE_RECOMMEND_RANK",
            "PAGE_RECOMMEND_MONTH_YEAR_PLAYLIST",
            "PAGE_RECOMMEND_RED_SIMILAR_SONG",
            "PAGE_RECOMMEND_NEW_SONG_AND_ALBUM",
            "PAGE_RECOMMEND_PRIVATE_RCMD_SONG",
            "PAGE_RECOMMEND_ARTIST_TREND",
            "PAGE_RECOMMEND_PODCAST_RADIO_PROGRAM"
        ])
        .to_string();

        // 只发送首页渲染必需的通用上下文。广告、设备标识、曝光记录等
        // 抓包字段既不稳定，也不应被持久化或复用。
        let mut data = json!({
            "pageCode": "HOME_RECOMMEND_PAGE",
            "pageStyleType": "cutBlock",
            "cursor": query.get_or("cursor", "0"),
            "refresh": "true",
            "isFirstScreen": if is_first_screen { "true" } else { "false" },
            "loadedPositionCodes": loaded_positions,
            "callbackParameters": "",
            "clientCacheBlockCode": client_cache_blocks,
            "clientTime": Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
            "reqTimeStamp": now.timestamp_millis().to_string(),
            "widthDp": query.get_or("width_dp", "363.42856"),
            "heightDp": query.get_or("height_dp", "743.7143"),
            "extJson": ext_json,
            "adExtJson": "{}",
            "ruleJson": "{}",
            "header": "{}",
            "e_r": true,
        });
        if is_first_screen {
            data["algDemoteBlockCodeOrderList"] = alg_demote_blocks.into();
        } else {
            data["blockCodeOrderList"] = next_page_block_order.into();
        }

        let option = RequestOption {
            crypto: CryptoType::Eapi,
            cookie: query.cookie.clone(),
            ua: query.ua.clone(),
            proxy: query.proxy.clone(),
            real_ip: query.real_ip.clone(),
            random_cn_ip: query.random_cn_ip,
            e_r: Some(true),
            // 此接口抓包域名为 interface3；ncm-api-rs 的默认 EAPI 域名不同。
            domain: query
                .domain
                .clone()
                .or_else(|| Some("https://interface3.music.163.com".to_string())),
            check_token: true,
        };

        self.request("/api/link/page/rcmd/resource/show", data, option)
    }

    fn pc_recent_listen_list(
        &self,
        query: &Query,
    ) -> impl Future<Output = Result<ApiResponse>> + Send {
        let data = json!({});

        let option = RequestOption {
            crypto: CryptoType::Weapi,
            cookie: query.cookie.clone(),
            ua: query.ua.clone(),
            proxy: query.proxy.clone(),
            real_ip: query.real_ip.clone(),
            random_cn_ip: query.random_cn_ip,
            e_r: query.e_r,
            domain: query.domain.clone(),
            check_token: true,
        };

        self.request("/api/pc/recent/listen/list", data, option)
    }
}
