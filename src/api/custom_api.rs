use chrono::Local;
use ncm_api_rs::error::Result;
use ncm_api_rs::{ApiClient, ApiResponse, CryptoType, Query, RequestOption};
use serde_json::json;

use crate::api::Song;

pub trait ApiClientExt {
    async fn home_recommend_resource(&self, query: &Query) -> Result<ApiResponse>;
    async fn home_category_daily_song_list(&self, query: &Query) -> Result<ApiResponse>;
}

impl ApiClientExt for ApiClient {
    async fn home_recommend_resource(&self, query: &Query) -> Result<ApiResponse> {
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
        self.request("/api/pc/daily/rcmd/block", data, option).await
    }

    // homepage/category/daily/song/list

    // {
    //     "source": "homepage",
    //     "categoryId": "1000",
    //     "tagId": "10015",
    //     "songId": "2106445921,1840647840,2693587200",
    //     "csrf_token": "4c7079ba8be54af5b6559e49701f7353"
    // }

    async fn home_category_daily_song_list(&self, query: &Query) -> Result<ApiResponse> {
        let category_id = query.get_or("category_id", "1000");
        let tag_id = query.get_or("tag_id", "10015");
        let song_ids = query.get_or("song_ids", "");
        let data = json!({
            "source": "homepage",
            "categoryId": category_id,
            "tagId": tag_id,
            "songId": song_ids
        });
        eprintln!("{}", data);

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
            .await
    }
}
