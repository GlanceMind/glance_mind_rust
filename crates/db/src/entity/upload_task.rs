use crate::schema::gm_upload_tasks;
use chrono::{DateTime, Utc};
use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

#[derive(Debug, Clone, Serialize, Deserialize, Queryable, Identifiable, Selectable)]
#[diesel(table_name = gm_upload_tasks)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct UploadTask {
    pub id: i32,
    pub user_id: i32,
    pub social_account_id: i32,
    pub task_type: String,
    pub metadata: JsonValue,
    pub status: String,
    pub platform_id: Option<i32>,
    pub created_at: DateTime<Utc>,
    pub updated_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = gm_upload_tasks)]
pub struct NewUploadTask {
    pub user_id: i32,
    pub social_account_id: i32,
    pub task_type: String,
    pub metadata: JsonValue,
    pub status: String,
    pub platform_id: Option<i32>,
}

#[derive(Debug, Clone, AsChangeset)]
#[diesel(table_name = gm_upload_tasks)]
pub struct UpdateUploadTask {
    pub status: Option<String>,
    pub metadata: Option<JsonValue>,
    pub updated_at: Option<DateTime<Utc>>,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum UploadTaskStatus {
    Init,
    Processing,
    Done,
    Failed,
}

#[allow(dead_code)]
impl UploadTaskStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            UploadTaskStatus::Init => "init",
            UploadTaskStatus::Processing => "processing",
            UploadTaskStatus::Done => "done",
            UploadTaskStatus::Failed => "failed",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "init" => Some(UploadTaskStatus::Init),
            "processing" => Some(UploadTaskStatus::Processing),
            "done" => Some(UploadTaskStatus::Done),
            "failed" => Some(UploadTaskStatus::Failed),
            _ => None,
        }
    }
}

impl std::fmt::Display for UploadTaskStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}
