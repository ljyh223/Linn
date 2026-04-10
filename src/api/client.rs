use once_cell::sync::Lazy;
use std::sync::RwLock;
use ncm_api_rs::{ApiClient, ApiResponse, Query, create_client};

static CLIENT: Lazy<RwLock<Option<ApiClient>>> = Lazy::new(|| {
    RwLock::new(None)
});

pub fn init_client(cookie: String) {
    let client = create_client(Some(cookie));

    let mut guard = CLIENT.write().unwrap();
    *guard = Some(client);
}

fn client() -> ApiClient {
    CLIENT
        .read()
        .unwrap()
        .as_ref()
        .expect("NCM client not initialized")
        .clone()
}

// pub async fn recommend_songs() -> anyhow::Result<Vec<ncm_api_rs::Song>> {
//     let query = Query::new();
//     let res = client().recommend_songs(&query).await?;
//     Ok(res)
// }

// pub async fn personal_fm() -> anyhow::Result<Vec<ApiResponse>> {
//     let query = Query::new();
//     let res = client().personal_fm(&query).await?;

//     Ok(res)
// }