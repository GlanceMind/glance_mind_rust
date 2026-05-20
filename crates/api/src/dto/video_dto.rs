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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct VideoModelOptionDto {
    pub value: String,
    pub label: String,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct VideoModelCapabilitiesDto {
    pub default_orientation: String,
    pub orientation_options: Vec<VideoModelOptionDto>,
    pub default_seconds: String,
    pub duration_options: Vec<VideoModelOptionDto>,
    pub image_input_mode: String,
    pub requires_image: bool,
    pub min_images: Option<i32>,
    pub max_images: i32,
    pub supports_keyframe_prompts: bool,
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
    /// Explicit Vidu mode token (text2video|image2video|start_end_frame|reference_video|multi_frame|ad_film). oneclick is AIPub-only.
    #[serde(default)]
    pub vidu_mode: Option<String>,
    /// Vidu quality tier (standard|fast). fast ⇒ viduq1/1080p.
    #[serde(default)]
    pub vidu_quality: Option<String>,
}

impl CreateVideoRequest {
    pub fn validate_params(&self) -> Result<(), String> {
        let seconds_val: i32 = self
            .seconds
            .parse()
            .map_err(|_| "seconds must be a valid number".to_string())?;
        // Extended for Vidu (4, 5, 8) + Sora (10, 15) + Jimeng (5, 10)
        if ![4, 5, 8, 10, 15].contains(&seconds_val) {
            return Err("seconds can only be 4, 5, 8, 10, or 15".to_string());
        }

        let valid_sizes = [
            "1280x720",
            "720x1280",
            "720x720",
            "960x720",
            "720x960",
            "1260x540",
            "1920x1080",
            "1080x1920",
            "1080x1080",
        ];
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

#[cfg(test)]
mod tests {
    use super::*;

    fn make_request(seconds: &str, size: &str) -> CreateVideoRequest {
        CreateVideoRequest {
            title: Some("Test".to_string()),
            prompt: Some("A test video".to_string()),
            ai_model_id: Some(1),
            orientation: VideoOrientation::Portrait,
            seconds: seconds.to_string(),
            size: size.to_string(),
            vidu_mode: None,
            vidu_quality: None,
        }
    }

    #[test]
    fn test_validate_vidu_duration_4s() {
        let req = make_request("4", "1280x720");
        assert!(req.validate_params().is_ok());
    }

    #[test]
    fn test_validate_vidu_duration_5s() {
        let req = make_request("5", "1280x720");
        assert!(req.validate_params().is_ok());
    }

    #[test]
    fn test_validate_vidu_duration_8s() {
        let req = make_request("8", "1280x720");
        assert!(req.validate_params().is_ok());
    }

    #[test]
    fn test_validate_sora_duration_10s() {
        let req = make_request("10", "1280x720");
        assert!(req.validate_params().is_ok());
    }

    #[test]
    fn test_validate_sora_duration_15s() {
        let req = make_request("15", "720x1280");
        assert!(req.validate_params().is_ok());
    }

    #[test]
    fn test_validate_invalid_duration_3s() {
        let req = make_request("3", "1280x720");
        let result = req.validate_params();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("seconds"));
    }

    #[test]
    fn test_validate_invalid_duration_6s() {
        let req = make_request("6", "1280x720");
        assert!(req.validate_params().is_err());
    }

    #[test]
    fn test_validate_invalid_duration_text() {
        let req = make_request("abc", "1280x720");
        let result = req.validate_params();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("valid number"));
    }

    #[test]
    fn test_validate_vidu_size_1920x1080() {
        let req = make_request("4", "1920x1080");
        assert!(req.validate_params().is_ok());
    }

    #[test]
    fn test_validate_vidu_size_1080x1920() {
        let req = make_request("4", "1080x1920");
        assert!(req.validate_params().is_ok());
    }

    #[test]
    fn test_validate_vidu_size_1080x1080() {
        let req = make_request("4", "1080x1080");
        assert!(req.validate_params().is_ok());
    }

    #[test]
    fn test_validate_standard_sizes() {
        let valid = vec![
            "1280x720", "720x1280", "720x720", "960x720", "720x960", "1260x540",
        ];
        for size in valid {
            let req = make_request("10", size);
            assert!(
                req.validate_params().is_ok(),
                "size '{}' should be valid",
                size
            );
        }
    }

    #[test]
    fn test_validate_invalid_size() {
        let req = make_request("10", "640x480");
        let result = req.validate_params();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("invalid size"));
    }

    #[test]
    fn test_validate_invalid_size_empty() {
        let req = make_request("10", "");
        assert!(req.validate_params().is_err());
    }
}
