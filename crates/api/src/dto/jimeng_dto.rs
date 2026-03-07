use serde::{Deserialize, Serialize};

// =============================================================================
// Video Mode & Resolution
// =============================================================================

/// Jimeng video generation mode
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum JimengVideoMode {
    /// Text-to-Video
    TextToVideo,
    /// Image-to-Video (single image as first frame)
    ImageFirstFrame,
    /// Image-to-Video (two images: first frame + last frame)
    ImageFirstLastFrame,
}

/// Jimeng video resolution tier — maps to the 3 opened products
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum JimengResolution {
    /// 3.0 720P (standard)
    V30_720p,
    /// 3.0 1080P (high-definition)
    V30_1080p,
    /// 3.0 Pro 1080P (best quality)
    V30Pro,
}

impl JimengResolution {
    pub fn label(&self) -> &'static str {
        match self {
            Self::V30_720p => "3.0 720P",
            Self::V30_1080p => "3.0 1080P",
            Self::V30Pro => "3.0 Pro",
        }
    }
}

/// Build the Volcengine `req_key` from mode + resolution.
///
/// Per Volcengine docs: T2V uses `_1080p` suffix, I2V uses `_1080` (no 'p')
///
/// | Product     | T2V                     | I2V First Frame              | I2V First-Last Frame              |
/// |-------------|-------------------------|------------------------------|-----------------------------------|
/// | 3.0 720P    | jimeng_t2v_v30          | jimeng_i2v_first_v30         | jimeng_i2v_first_tail_v30         |
/// | 3.0 1080P   | jimeng_t2v_v30_1080p    | jimeng_i2v_first_v30_1080    | jimeng_i2v_first_tail_v30_1080    |
/// | 3.0 Pro     | jimeng_vgfm_t2v_l20     | jimeng_vgfm_i2v_l20          | N/A (not supported)               |
pub fn build_req_key(mode: JimengVideoMode, resolution: JimengResolution) -> Option<String> {
    let key = match (mode, resolution) {
        (JimengVideoMode::TextToVideo, JimengResolution::V30Pro) => "jimeng_vgfm_t2v_l20",
        (JimengVideoMode::ImageFirstFrame, JimengResolution::V30Pro) => "jimeng_vgfm_i2v_l20",
        (JimengVideoMode::ImageFirstLastFrame, JimengResolution::V30Pro) => return None,
        (JimengVideoMode::TextToVideo, JimengResolution::V30_720p) => "jimeng_t2v_v30",
        (JimengVideoMode::TextToVideo, JimengResolution::V30_1080p) => "jimeng_t2v_v30_1080p",
        (JimengVideoMode::ImageFirstFrame, JimengResolution::V30_720p) => "jimeng_i2v_first_v30",
        (JimengVideoMode::ImageFirstFrame, JimengResolution::V30_1080p) => "jimeng_i2v_first_v30_1080",
        (JimengVideoMode::ImageFirstLastFrame, JimengResolution::V30_720p) => "jimeng_i2v_first_tail_v30",
        (JimengVideoMode::ImageFirstLastFrame, JimengResolution::V30_1080p) => "jimeng_i2v_first_tail_v30_1080",
    };
    Some(key.into())
}

// =============================================================================
// Unified Video Generation Parameters (upper-layer convenience)
// =============================================================================

/// High-level parameters for creating a Jimeng video, used by video_service.
#[derive(Debug, Clone)]
pub struct JimengVideoParams {
    pub prompt: String,
    pub resolution: JimengResolution,
    /// Duration in seconds: 5 or 10
    pub seconds: i32,
    /// Aspect ratio (only for T2V): "16:9", "9:16", "1:1", etc.
    pub aspect_ratio: Option<String>,
    /// Base64-encoded first frame image (for I2V first-frame and first-last-frame)
    pub image_base64: Option<String>,
    /// Base64-encoded last frame image (only for I2V first-last-frame)
    pub end_image_base64: Option<String>,
}

// =============================================================================
// API Request / Response Types
// =============================================================================

/// Volcengine API submit request body
///
/// Images MUST be provided via `binary_data_base64` (base64-encoded image data).
/// Note: `image_urls` is NOT supported by the direct Volcengine API — it is silently
/// ignored, causing I2V requests to degrade to T2V. Only use `binary_data_base64`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JimengSubmitRequest {
    pub req_key: String,
    pub prompt: String,
    pub frames: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub aspect_ratio: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub binary_data_base64: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seed: Option<i64>,
}

impl JimengSubmitRequest {
    pub fn seconds_to_frames(seconds: i32) -> i32 {
        24 * seconds + 1
    }
}

/// Volcengine top-level response wrapper
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VolcengineResponse<T> {
    #[serde(alias = "ResponseMetadata")]
    pub response_metadata: Option<ResponseMetadata>,
    #[serde(alias = "Result")]
    pub result: Option<JimengApiResponse<T>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponseMetadata {
    #[serde(alias = "Action")]
    pub action: Option<String>,
    #[serde(alias = "Error")]
    pub error: Option<VolcengineError>,
    #[serde(alias = "RequestId")]
    pub request_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VolcengineError {
    #[serde(alias = "Code")]
    pub code: Option<String>,
    #[serde(alias = "Message")]
    pub message: Option<String>,
}

/// Inner Result object
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JimengApiResponse<T> {
    pub code: i32,
    #[serde(default)]
    pub message: String,
    pub data: Option<T>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JimengSubmitData {
    pub task_id: String,
}

/// Handle returned after submission — carries task_id + req_key for polling
#[derive(Debug, Clone)]
pub struct JimengTaskHandle {
    pub task_id: String,
    pub req_key: String,
}

/// Task result data (from real Volcengine GetResult)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JimengResultData {
    #[serde(default)]
    pub task_id: String,
    pub status: String,
    #[serde(default)]
    pub resp_data: Option<String>,
    #[serde(default)]
    pub video_url: Option<String>,
}

impl JimengResultData {
    pub fn is_done(&self) -> bool { self.status == "done" }
    pub fn is_running(&self) -> bool { matches!(self.status.as_str(), "running" | "submitted" | "in_queue") }
    pub fn is_failed(&self) -> bool { matches!(self.status.as_str(), "failed" | "error") }

    /// Extract video URL from resp_data (may be plain URL or JSON with `urls` array)
    pub fn get_video_url(&self) -> Option<String> {
        if let Some(url) = &self.video_url {
            if !url.is_empty() { return Some(url.clone()); }
        }
        if let Some(data) = &self.resp_data {
            if data.is_empty() { return None; }
            if data.starts_with("http") { return Some(data.clone()); }
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(data) {
                if let Some(url) = json.get("urls").and_then(|a| a.as_array())
                    .and_then(|a| a.first()).and_then(|u| u.as_str()) {
                    return Some(url.to_string());
                }
            }
            return Some(data.clone());
        }
        None
    }
}

// =============================================================================
// Constants
// =============================================================================

pub const VALID_ASPECT_RATIOS: &[&str] = &["16:9", "4:3", "1:1", "3:4", "9:16", "21:9"];
pub const VALID_SECONDS: &[i32] = &[5, 10];

pub const ALL_RESOLUTIONS: &[JimengResolution] = &[
    JimengResolution::V30_720p,
    JimengResolution::V30_1080p,
    JimengResolution::V30Pro,
];

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_req_key_t2v() {
        assert_eq!(build_req_key(JimengVideoMode::TextToVideo, JimengResolution::V30_720p).unwrap(), "jimeng_t2v_v30");
        assert_eq!(build_req_key(JimengVideoMode::TextToVideo, JimengResolution::V30_1080p).unwrap(), "jimeng_t2v_v30_1080p");
        assert_eq!(build_req_key(JimengVideoMode::TextToVideo, JimengResolution::V30Pro).unwrap(), "jimeng_vgfm_t2v_l20");
    }

    #[test]
    fn test_build_req_key_i2v() {
        assert_eq!(build_req_key(JimengVideoMode::ImageFirstFrame, JimengResolution::V30_720p).unwrap(), "jimeng_i2v_first_v30");
        assert_eq!(build_req_key(JimengVideoMode::ImageFirstFrame, JimengResolution::V30_1080p).unwrap(), "jimeng_i2v_first_v30_1080");
        assert_eq!(build_req_key(JimengVideoMode::ImageFirstFrame, JimengResolution::V30Pro).unwrap(), "jimeng_vgfm_i2v_l20");
    }

    #[test]
    fn test_build_req_key_i2v_first_last() {
        assert_eq!(build_req_key(JimengVideoMode::ImageFirstLastFrame, JimengResolution::V30_720p).unwrap(), "jimeng_i2v_first_tail_v30");
        assert_eq!(build_req_key(JimengVideoMode::ImageFirstLastFrame, JimengResolution::V30_1080p).unwrap(), "jimeng_i2v_first_tail_v30_1080");
        assert!(build_req_key(JimengVideoMode::ImageFirstLastFrame, JimengResolution::V30Pro).is_none(), "Pro does not support first-last frame");
    }

    #[test]
    fn test_resolution_label() {
        assert_eq!(JimengResolution::V30_720p.label(), "3.0 720P");
        assert_eq!(JimengResolution::V30_1080p.label(), "3.0 1080P");
        assert_eq!(JimengResolution::V30Pro.label(), "3.0 Pro");
    }

    #[test]
    fn test_seconds_to_frames() {
        assert_eq!(JimengSubmitRequest::seconds_to_frames(5), 121);
        assert_eq!(JimengSubmitRequest::seconds_to_frames(10), 241);
    }

    #[test]
    fn test_result_status() {
        let done = JimengResultData { task_id: "".into(), status: "done".into(), resp_data: None, video_url: None };
        assert!(done.is_done()); assert!(!done.is_running());
        let running = JimengResultData { task_id: "".into(), status: "in_queue".into(), resp_data: None, video_url: None };
        assert!(running.is_running()); assert!(!running.is_done());
    }

    #[test]
    fn test_get_video_url_from_json() {
        let r = JimengResultData {
            task_id: "".into(), status: "done".into(),
            resp_data: Some(r#"{"urls":["https://cdn.example.com/v.mp4"]}"#.into()),
            video_url: None,
        };
        assert_eq!(r.get_video_url().unwrap(), "https://cdn.example.com/v.mp4");
    }
}
