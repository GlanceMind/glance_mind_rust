use chrono::{DateTime, Utc};
use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

use crate::schema::gm_data_video_cases;

/// Video case entity - mirrors `gm_data_video_cases`.
#[derive(Debug, Clone, Serialize, Deserialize, Queryable, Selectable, Identifiable)]
#[diesel(table_name = gm_data_video_cases)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct VideoCase {
    pub id: i32,
    pub task_no: String,
    pub case_id: Option<i32>,
    pub user_id: Option<String>,
    pub tt_category_id: Option<String>,
    pub category_name_en: Option<String>,
    pub category_name_cn: Option<String>,
    pub task_type: Option<String>,
    pub num: Option<i32>,
    pub status: Option<i32>,
    pub script: Option<String>,
    pub selling_point: Option<String>,
    pub product_name: Option<String>,
    pub brand_name: Option<String>,
    pub video_language: Option<String>,
    pub model: Option<String>,
    pub video_model: Option<String>,
    pub video_url: Option<String>,
    pub ai_image_url: Option<String>,
    pub ai_prompt: Option<String>,
    pub refer_image_url: Option<String>,
    pub refer_video_url: Option<String>,
    pub image_urls: Option<JsonValue>,
    pub characters: Option<JsonValue>,
    pub videos: Option<JsonValue>,
    pub progress: Option<i32>,
    pub video_status: Option<String>,
    pub error_message: Option<String>,
    pub size: Option<String>,
    pub case_status: Option<i32>,
    pub favorite_status: Option<i32>,
    pub completed_num: Option<i32>,
    pub created_at: DateTime<Utc>,
    pub updated_at: Option<DateTime<Utc>>,
}

/// New video case for insertion
#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = gm_data_video_cases)]
pub struct NewVideoCase {
    pub task_no: String,
    pub case_id: Option<i32>,
    pub user_id: Option<String>,
    pub tt_category_id: Option<String>,
    pub category_name_en: Option<String>,
    pub category_name_cn: Option<String>,
    pub task_type: Option<String>,
    pub num: Option<i32>,
    pub status: Option<i32>,
    pub script: Option<String>,
    pub selling_point: Option<String>,
    pub product_name: Option<String>,
    pub brand_name: Option<String>,
    pub video_language: Option<String>,
    pub model: Option<String>,
    pub video_model: Option<String>,
    pub video_url: Option<String>,
    pub ai_image_url: Option<String>,
    pub ai_prompt: Option<String>,
    pub refer_image_url: Option<String>,
    pub refer_video_url: Option<String>,
    pub image_urls: Option<JsonValue>,
    pub characters: Option<JsonValue>,
    pub videos: Option<JsonValue>,
    pub progress: Option<i32>,
    pub video_status: Option<String>,
    pub error_message: Option<String>,
    pub size: Option<String>,
    pub case_status: Option<i32>,
    pub favorite_status: Option<i32>,
    pub completed_num: Option<i32>,
    pub created_at: DateTime<Utc>,
    pub updated_at: Option<DateTime<Utc>>,
}

/// Update video case changeset
#[derive(Debug, Clone, AsChangeset, Default)]
#[diesel(table_name = gm_data_video_cases)]
pub struct UpdateVideoCase {
    pub video_url: Option<String>,
    pub progress: Option<i32>,
    pub video_status: Option<String>,
    pub error_message: Option<String>,
    pub videos: Option<JsonValue>,
    pub completed_num: Option<i32>,
    pub favorite_status: Option<i32>,
    pub updated_at: Option<DateTime<Utc>>,
}
