use crate::schema::{gm_crawler_results, gm_crawler_tasks};
use chrono::{DateTime, Utc};
use diesel::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Queryable, Selectable, Identifiable, Serialize, Deserialize, Debug, Clone)]
#[diesel(table_name = gm_crawler_tasks)]
pub struct CrawlerTask {
    pub id: i32,
    pub campaign_id: i32,
    pub keywords: Option<Vec<Option<String>>>,
    pub max_count: i32,
    pub process_count: i32,
    pub search_offset: i32,
    pub search_limit: i32,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: Option<DateTime<Utc>>,
}

#[derive(Queryable, Selectable, Identifiable, Serialize, Deserialize, Debug, Clone)]
#[diesel(table_name = gm_crawler_results)]
pub struct CrawlerResult {
    pub id: i32,
    pub task_id: i32,
    pub video_id: String,
    pub video_title: Option<String>,
    pub comment_count: Option<i32>,
    pub view_count: Option<i32>,
    pub author_name: Option<String>,
    pub processed: bool,
    pub replied: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: Option<DateTime<Utc>>,
    pub like_count: Option<i32>,
}
