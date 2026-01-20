use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use validator::Validate;

#[derive(Debug, Deserialize, Validate)]
pub struct CreateUploadTaskDto {
    #[validate(range(min = 1))]
    pub social_account_id: i32,

    pub platform_id: Option<i32>,

    pub metadata: JsonValue,
}

#[derive(Debug, Serialize)]
pub struct UploadTaskResponseDto {
    pub id: i32,
    pub social_account_id: i32,
    pub task_type: String,
    pub metadata: JsonValue,
    pub status: String,
    pub platform_id: Option<i32>,
    pub created_at: DateTime<Utc>,
    pub updated_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize)]
pub struct DeviceTaskQueryDto {
    pub device_id: String,
    pub status: Option<String>,
    #[serde(default = "default_page")]
    pub page: i64,
    #[serde(default = "default_page_size")]
    pub page_size: i64,
}

fn default_page() -> i64 {
    1
}

fn default_page_size() -> i64 {
    10
}

#[derive(Debug, Serialize)]
pub struct DeviceTaskResponseDto {
    pub id: i32,
    pub task_type: String,
    #[serde(rename = "meta")]
    pub metadata: JsonValue,
}

impl From<glance_mind_db::entity::upload_task::UploadTask> for UploadTaskResponseDto {
    fn from(task: glance_mind_db::entity::upload_task::UploadTask) -> Self {
        Self {
            id: task.id,
            social_account_id: task.social_account_id,
            task_type: task.task_type,
            metadata: task.metadata,
            status: task.status,
            platform_id: task.platform_id,
            created_at: task.created_at,
            updated_at: task.updated_at,
        }
    }
}

impl From<glance_mind_db::entity::upload_task::UploadTask> for DeviceTaskResponseDto {
    fn from(task: glance_mind_db::entity::upload_task::UploadTask) -> Self {
        Self {
            id: task.id,
            task_type: task.task_type,
            metadata: task.metadata,
        }
    }
}

// Update upload task status request
#[derive(Debug, Deserialize, Validate)]
pub struct UpdateTaskStatusDto {
    #[validate(range(min = 1))]
    pub task_id: i32,
    pub status: String,
}

// Update response
#[derive(Debug, Serialize)]
pub struct UpdateTaskStatusResponse {
    pub success: bool,
    pub message: String,
}
