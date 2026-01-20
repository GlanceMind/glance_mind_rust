use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct CrawlerTaskDto {
    pub id: i32,
    pub campaign_id: i32,
    pub keywords: Option<Vec<String>>, // Simplify Option<Vec<Option<String>>> to Vec<String> for API if possible, but let's match entity for now or clean up.
    pub max_count: i32,
    pub process_count: i32,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CrawlerResultDto {
    pub id: i32,
    pub task_id: i32,
    pub video_id: String,
    pub video_title: Option<String>,
    pub comment_count: Option<i32>,
    pub view_count: Option<i32>,
    pub like_count: Option<i32>,
    pub share_count: Option<i32>,
    pub play_count: Option<i32>,
    pub author_name: Option<String>,
    pub processed: bool,
    pub replied: bool,
    pub created_at: DateTime<Utc>,
}
