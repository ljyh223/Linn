use std::future::Future;

use chrono::Local;
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
}
