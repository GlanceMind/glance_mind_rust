use bigdecimal::BigDecimal;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use validator::Validate;

// ===== Video Orientation Enum =====

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum VideoOrientation {
    #[serde(rename = "portrait")]
    #[default]
    Portrait, // Portrait 9:16
    #[serde(rename = "landscape")]
    Landscape, // Landscape 16:9
}

impl VideoOrientation {
    pub fn as_str(&self) -> &'static str {
        match self {
            VideoOrientation::Portrait => "portrait",
            VideoOrientation::Landscape => "landscape",
        }
    }
}

// ===== Video Generation Related =====

#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct CreateVideoRequest {
    pub title: Option<String>,
    #[validate(length(max = 2000, message = "Prompt cannot exceed 2000 characters"))]
    pub prompt: Option<String>,
    pub ai_model_id: Option<i32>,
    #[serde(default)]
    pub orientation: VideoOrientation,
    pub seconds: String, // "10" or "15"
    pub size: String,    // "1280x720" (landscape) or "720x1280" (portrait)
}

impl CreateVideoRequest {
    pub fn validate_params(&self) -> Result<(), String> {
        // prompt can be empty when there's an image (image-to-video)
        // but in multipart processing, at least prompt or image is required

        // Validate seconds
        let seconds_val: i32 = self
            .seconds
            .parse()
            .map_err(|_| "seconds must be a valid number".to_string())?;
        if ![5, 10, 15].contains(&seconds_val) {
            return Err("seconds can only be 5, 10, or 15".to_string());
        }

        // Validate size (extended for Jimeng aspect ratios)
        let valid_sizes = ["1280x720", "720x1280", "720x720", "960x720", "720x960", "1260x540"];
        if !valid_sizes.contains(&self.size.as_str()) {
            return Err(format!("invalid size: {}", self.size));
        }

        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateVideoResponse {
    pub task_id: String,
    pub status: String,
    pub cost_points: BigDecimal,
    pub estimated_time: String,
    pub ai_model_name: Option<String>,
    pub expires_at: Option<i64>, // Task expiration time (Unix timestamp)
}

// ===== Video Task List Response =====

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoTaskResponse {
    pub id: i32,
    pub task_id: String,
    pub title: Option<String>,
    pub prompt: Option<String>,
    pub status: String,
    pub progress_pct: Option<BigDecimal>,
    pub video_url: Option<String>,
    pub thumbnail_url: Option<String>,
    pub cost_points: BigDecimal,
    pub error_message: Option<String>,
    pub created_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub ai_model_name: Option<String>,
    pub orientation: Option<String>,
    pub video_seconds: Option<String>,
    pub video_size: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoTaskListResponse {
    pub tasks: Vec<VideoTaskResponse>,
    pub total: i64,
    pub page: i32,
    pub page_size: i32,
}
