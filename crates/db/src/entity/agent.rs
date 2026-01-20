use crate::schema::gm_agent_comments;
use diesel::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Queryable, Selectable, Serialize, Deserialize, Clone)]
#[diesel(table_name = gm_agent_comments)]
pub struct AgentComment {
    pub id: i32,
    pub video_db_id: i32,
    pub comment_id: String,
    pub user_nickname: Option<String>,
    pub user_unique_id: Option<String>,
    pub content: Option<String>,
    pub reason: Option<String>,
    pub suggested_reply: Option<String>,
    pub create_time: Option<chrono::NaiveDateTime>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub campaign_id: Option<i32>,
    pub status: i16,
    pub suggested_dm: Option<String>,
    pub suggested_reply_post: Option<String>,
}

use crate::schema::gm_agent_videos;

#[derive(Debug, Queryable, Selectable, Serialize, Deserialize, Clone)]
#[diesel(table_name = gm_agent_videos)]
pub struct AgentVideo {
    pub id: i32,
    pub video_id: Option<String>,
    pub author: Option<String>,
    pub description: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub task_id: i32,
    pub campaign_id: Option<i32>,
    pub like_count: Option<i32>,
    pub comment_count: Option<i32>,
    pub share_count: Option<i32>,
    pub play_count: Option<i32>,
    pub publish_time: Option<i64>,
    pub author_unique_id: Option<String>,
    pub url: Option<String>,
}
