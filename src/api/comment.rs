use ncm_api_rs::Query;

use crate::api::{Comment, MusicComment, UserInfo, client::client};

pub async fn get_song_comments(id: u64) -> anyhow::Result<MusicComment> {
    let query = Query::new().param("id", &id.to_string());
    match client().comment_music(&query).await {
        Ok(resp) => {
            let mut hot_comment = resp.body["hotComments"]
                .as_array()
                .cloned()
                .unwrap_or_default()
                .iter()
                .map(|c| {
                    let user = c["user"].as_object().unwrap();
                    Comment {
                        id: c["commentId"].as_u64().unwrap_or_default(),
                        content: c["content"].as_str().unwrap_or_default().to_string(),
                        liked_count: c["likedCount"].as_u64().unwrap_or_default(),
                        user: UserInfo {
                            id: user["userId"].as_u64().unwrap_or_default(),
                            name: user["nickname"].as_str().unwrap_or_default().to_string(),
                            avatar_url: user["avatarUrl"].as_str().unwrap_or_default().to_string(),
                        },
                    }
                })
                .collect();
            let comment = resp.body["comments"]
                .as_array()
                .cloned()
                .unwrap_or_default()
                .iter()
                .map(|c| {
                    let user = c["user"].as_object().unwrap();
                    Comment {
                        id: c["commentId"].as_u64().unwrap_or_default(),
                        content: c["content"].as_str().unwrap_or_default().to_string(),
                        liked_count: c["likedCount"].as_u64().unwrap_or_default(),
                        user: UserInfo {
                            id: user["userId"].as_u64().unwrap_or_default(),
                            name: user["nickname"].as_str().unwrap_or_default().to_string(),
                            avatar_url: user["avatarUrl"].as_str().unwrap_or_default().to_string(),
                        },
                    }
                })
                .collect();

            Ok(MusicComment {
                song_id: id,
                hot_comments: hot_comment,
                comments: comment,
            })
        }
        Err(e) => {
            eprintln!("获取评论失败， song id: {}, {}", id, e);
            Err(e.into())
        }
    }
}
