use ncm_api_rs::Query;

use crate::api::{Comment, CommentFloor, MusicComment, UserInfo, client::client};

fn parse_comment(c: &serde_json::Value) -> Comment {
    let fallback = serde_json::Map::new();
    let user = c["user"].as_object().unwrap_or(&fallback);
    Comment {
        id: c["commentId"].as_u64().unwrap_or_default(),
        content: c["content"].as_str().unwrap_or_default().to_string(),
        liked_count: c["likedCount"].as_u64().unwrap_or_default(),
        reply_count: c["replyCount"].as_u64().unwrap_or_default(),
        be_replied: c["beReplied"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .map(|r| {
                        let ru = r["user"].as_object().unwrap_or(&fallback);
                        Comment {
                            id: r["beRepliedCommentId"].as_u64().unwrap_or_default(),
                            content: r["content"].as_str().unwrap_or_default().to_string(),
                            liked_count: 0,
                            reply_count: 0,
                            be_replied: Vec::new(),
                            time: 0,
                            time_str: String::new(),
                            user: UserInfo {
                                id: ru["userId"].as_u64().unwrap_or_default(),
                                name: ru["nickname"].as_str().unwrap_or_default().to_string(),
                                avatar_url: ru["avatarUrl"]
                                    .as_str()
                                    .unwrap_or_default()
                                    .to_string(),
                            },
                        }
                    })
                    .collect()
            })
            .unwrap_or_default(),
        time: c["time"].as_i64().unwrap_or_default(),
        time_str: c["timeStr"].as_str().unwrap_or_default().to_string(),
        user: UserInfo {
            id: user["userId"].as_u64().unwrap_or_default(),
            name: user["nickname"].as_str().unwrap_or_default().to_string(),
            avatar_url: user["avatarUrl"].as_str().unwrap_or_default().to_string(),
        },
    }
}

/// 新版评论（/api/v2/resource/comments）
/// sort_type: 99 = 推荐, 2 = 热度, 3 = 时间
/// cursor: 从上一页响应中回传（推荐/热度由服务端通过 pageNo 计算，时间排序需回传 cursor）
/// 返回 (评论列表, 是否还有更多, 下一页 cursor)
pub async fn get_song_comments_new(
    id: u64,
    page_no: i64,
    sort_type: i64,
    cursor: &str,
) -> anyhow::Result<(Vec<Comment>, bool, String)> {
    let query = Query::new()
        .param("id", &id.to_string())
        .param("pageNo", &page_no.to_string())
        .param("sortType", &sort_type.to_string());
    let query = if cursor.is_empty() {
        query
    } else {
        query.param("cursor", cursor)
    };
    match client().comment_new(&query).await {
        Ok(resp) => {
            let data = &resp.body["data"];
            let comments = data["comments"]
                .as_array()
                .cloned()
                .unwrap_or_default()
                .iter()
                .map(parse_comment)
                .collect();
            let has_more = data["hasMore"].as_bool().unwrap_or_default();
            let next_cursor = data["cursor"].as_str().unwrap_or("").to_string();
            Ok((comments, has_more, next_cursor))
        }
        Err(e) => {
            eprintln!(
                "获取新版评论失败， song id: {}, sort_type: {}, {}",
                id, sort_type, e
            );
            Err(e.into())
        }
    }
}

/// 楼中楼回复
pub async fn get_comment_floor(
    id: u64,
    parent_comment_id: u64,
    time: i64,
) -> anyhow::Result<CommentFloor> {
    let query = Query::new()
        .param("id", &id.to_string())
        .param("parentCommentId", &parent_comment_id.to_string())
        .param("time", &time.to_string())
        .param("limit", "20");
    match client().comment_floor(&query).await {
        Ok(resp) => {
            let data = &resp.body["data"];
            let replies = data["comments"]
                .as_array()
                .cloned()
                .unwrap_or_default()
                .iter()
                .map(parse_comment)
                .collect();
            let has_more = data["hasMore"].as_bool().unwrap_or_default();
            let cursor = data["comments"]
                .as_array()
                .and_then(|arr| arr.last())
                .and_then(|c| c["time"].as_i64())
                .unwrap_or_default();
            Ok(CommentFloor {
                parent_comment_id,
                replies,
                has_more,
                cursor,
            })
        }
        Err(e) => {
            eprintln!(
                "获取楼层评论失败，song id: {}, parent: {}, {}",
                id, parent_comment_id, e
            );
            Err(e.into())
        }
    }
}

/// MV 评论（/comment/mv）
pub async fn get_mv_comments(id: u64) -> anyhow::Result<Vec<Comment>> {
    let query = Query::new()
        .param("id", &id.to_string())
        .param("limit", "30");
    match client().comment_mv(&query).await {
        Ok(resp) => {
            let comments = resp.body["comments"]
                .as_array()
                .cloned()
                .unwrap_or_default()
                .iter()
                .map(parse_comment)
                .collect();
            Ok(comments)
        }
        Err(e) => {
            eprintln!("获取 MV 评论失败， mv id: {}, {}", id, e);
            Err(e.into())
        }
    }
}

pub async fn get_song_comments(id: u64) -> anyhow::Result<MusicComment> {
    let query = Query::new().param("id", &id.to_string());
    match client().comment_music(&query).await {
        Ok(resp) => {
            let hot_comment = resp.body["hotComments"]
                .as_array()
                .cloned()
                .unwrap_or_default()
                .iter()
                .map(parse_comment)
                .collect();
            let comment = resp.body["comments"]
                .as_array()
                .cloned()
                .unwrap_or_default()
                .iter()
                .map(parse_comment)
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
