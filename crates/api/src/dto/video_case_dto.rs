use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

// ===== Video Case List Item (Page Response) =====

/// Video case item for list response
/// Matches the page data structure from external API
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VideoCaseListItem {
    /// Video ID (bigint in database)
    pub video_id: Option<i64>,
    /// Case ID reference
    pub case_id: Option<i32>,
    /// User ID
    pub user_id: Option<String>,
    /// TikTok category ID
    pub tt_category_id: Option<String>,
    /// Category name in English
    pub category_name_en: Option<String>,
    /// Category name in Chinese
    pub category_name_cn: Option<String>,
    /// AI model used (video_model from database)
    pub model: Option<String>,
    /// Favorite status (0: not favorite, 1: favorite)
    pub favorite_status: i32,
    /// Script content
    pub script: Option<String>,
    /// Task number (primary key)
    pub task_no: String,
    /// Task type
    pub task_type: Option<String>,
    /// Generated video URL
    pub video_url: Option<String>,
    /// AI generated image URL
    pub ai_image_url: Option<String>,
    /// AI prompt used
    pub ai_prompt: Option<String>,
    /// Generation progress (0-100)
    pub progress: i32,
    /// Reference image URL
    pub refer_image_url: Option<String>,
    /// Video generation status
    pub video_status: String,
    /// Error message if failed
    pub error_message: Option<String>,
    /// Video size (e.g., "720x1280")
    pub size: Option<String>,
    /// Case status
    pub case_status: i32,
}

// ===== Video Case Detail Response =====

/// Video item within a case
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default, rename_all = "camelCase")]
pub struct VideoCaseVideo {
    /// Video ID
    pub video_id: i32,
    /// Task number for this video
    pub task_no: Option<String>,
    /// Video URL
    pub video_url: Option<String>,
    /// AI generated image URL
    pub ai_image_url: Option<String>,
    /// AI prompt
    pub ai_prompt: Option<String>,
    /// Progress percentage
    pub progress: i32,
    /// Reference image URL
    pub refer_image_url: Option<String>,
    /// Video status
    pub video_status: String,
    /// Error message
    pub error_message: Option<String>,
    /// Model used
    pub model: Option<String>,
    /// Video size
    pub size: Option<String>,
    /// Video duration in seconds
    pub seconds: Option<String>,
    /// Storyboard frames
    pub storyboards: Vec<JsonValue>,
}

/// Video case detail response
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VideoCaseDetail {
    /// Detail ID (detail_id from database)
    pub id: Option<i32>,
    /// Task number (primary key)
    pub task_no: String,
    /// Number of videos to generate
    pub num: i32,
    /// Status code (detail_status from database)
    pub status: i32,
    /// TikTok category ID
    pub tt_category_id: Option<String>,
    /// Category name in Chinese
    pub category_name_cn: Option<String>,
    /// Reference image URLs
    pub image_urls: Vec<String>,
    /// Script content
    pub script: Option<String>,
    /// Character configurations
    pub characters: Vec<JsonValue>,
    /// Reference video URL
    pub refer_video_url: Option<String>,
    /// Product selling point
    pub selling_point: Option<String>,
    /// Product name
    pub product_name: Option<String>,
    /// Brand name
    pub brand_name: Option<String>,
    /// Video language
    pub video_language: Option<String>,
    /// Video model used
    pub video_model: Option<String>,
    /// Creation time
    pub create_time: String,
    /// Number of completed videos
    pub completed_num: Option<i32>,
    /// Generated videos
    pub videos: Vec<VideoCaseVideo>,
}

// ===== List Response with Pagination =====

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VideoCaseListResponse {
    pub items: Vec<VideoCaseListItem>,
    pub total: i64,
    pub page: i32,
    pub page_size: i32,
}

// ===== Query Parameters =====

#[derive(Debug, Deserialize)]
pub struct VideoCaseListQuery {
    pub page: Option<i32>,
    pub page_size: Option<i32>,
    pub status: Option<i32>,
    pub video_status: Option<String>,
    pub category_id: Option<String>,
    pub user_id: Option<String>,
}

// ===== Conversion from Entity =====

impl From<glance_mind_db::entity::video_case::VideoCase> for VideoCaseListItem {
    fn from(case: glance_mind_db::entity::video_case::VideoCase) -> Self {
        Self {
            video_id: Some(i64::from(case.id)),
            case_id: case.case_id,
            user_id: case.user_id,
            tt_category_id: case.tt_category_id,
            category_name_en: case.category_name_en,
            category_name_cn: case.category_name_cn,
            model: case.video_model.clone().or(case.model.clone()),
            favorite_status: case.favorite_status.unwrap_or(0),
            script: case.script,
            task_no: case.task_no,
            task_type: case.task_type,
            video_url: case.video_url,
            ai_image_url: case.ai_image_url,
            ai_prompt: case.ai_prompt,
            progress: case.progress.unwrap_or(0),
            refer_image_url: case.refer_image_url,
            video_status: case.video_status.unwrap_or_else(|| "pending".to_string()),
            error_message: case.error_message,
            size: case.size,
            case_status: case.case_status.unwrap_or(0),
        }
    }
}

impl From<glance_mind_db::entity::video_case::VideoCase> for VideoCaseDetail {
    fn from(case: glance_mind_db::entity::video_case::VideoCase) -> Self {
        // Parse image_urls from JSONB
        let image_urls: Vec<String> = case
            .image_urls
            .and_then(|v| serde_json::from_value(v).ok())
            .unwrap_or_default();

        // Parse characters from JSONB
        let characters: Vec<JsonValue> = case
            .characters
            .and_then(|v| serde_json::from_value(v).ok())
            .unwrap_or_default();

        // Parse videos from JSONB
        let videos: Vec<VideoCaseVideo> = case
            .videos
            .and_then(|v| serde_json::from_value(v).ok())
            .unwrap_or_default();

        let create_time = case.created_at.format("%Y-%m-%d %H:%M:%S").to_string();

        Self {
            id: Some(case.id),
            task_no: case.task_no,
            num: case.num.unwrap_or(1),
            status: case.status.unwrap_or(0),
            tt_category_id: case.tt_category_id,
            category_name_cn: case.category_name_cn,
            image_urls,
            script: case.script,
            characters,
            refer_video_url: case.refer_video_url,
            selling_point: case.selling_point,
            product_name: case.product_name,
            brand_name: case.brand_name,
            video_language: case.video_language,
            video_model: case.video_model.or(case.model),
            create_time,
            completed_num: case.completed_num,
            videos,
        }
    }
}
