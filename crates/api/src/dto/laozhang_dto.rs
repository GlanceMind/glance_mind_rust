use serde::{Deserialize, Serialize};

// ===== Create Video Request =====

/// Text-to-video request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateVideoFromTextRequest {
    pub model: String,
    pub prompt: String,
    pub size: String,
    pub seconds: String,
}

impl Default for CreateVideoFromTextRequest {
    fn default() -> Self {
        Self {
            model: "sora-2".to_string(),
            prompt: String::new(),
            size: "1280x720".to_string(),
            seconds: "15".to_string(),
        }
    }
}

/// Image-to-video request (for passing parameters)
#[derive(Debug, Clone)]
pub struct CreateVideoFromImageRequest {
    pub model: String,
    pub prompt: String,
    pub image_data: Vec<u8>,
    pub image_filename: String,
    pub size: String,
    pub seconds: String,
}

impl Default for CreateVideoFromImageRequest {
    fn default() -> Self {
        Self {
            model: "sora-2".to_string(),
            prompt: String::new(),
            image_data: Vec::new(),
            image_filename: "image.png".to_string(),
            size: "1280x720".to_string(),
            seconds: "10".to_string(),
        }
    }
}

// ===== Task Response =====

/// Create video task response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoTaskResponse {
    pub id: String,
    pub object: String,
    pub model: String,
    pub status: String,
    #[serde(alias = "created_at")] // Compatible with old version
    pub created: i64,
    #[serde(skip_serializing_if = "Option::is_none", alias = "expires_at")]
    pub expires: Option<i64>,
    // LaoZhang API also returns these fields
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seconds: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size: Option<String>,
}

// ===== Query Task Status Response =====

/// Task in progress response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoTaskInProgress {
    pub id: String,
    pub object: String,
    pub model: String,
    pub status: String,
    #[serde(default)]
    pub progress: i32,
    pub created_at: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seconds: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size: Option<String>,
}

/// Task completed response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoTaskCompleted {
    pub id: String,
    pub object: String,
    pub model: String,
    pub status: String,
    #[serde(default)]
    pub progress: i32,
    pub url: String,
    pub created_at: i64,
    pub completed_at: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seconds: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size: Option<String>,
}

/// Task status (unified response)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "status")]
pub enum VideoTaskStatus {
    #[serde(rename = "submitted")]
    Submitted(VideoTaskResponse),
    #[serde(rename = "queued")]
    Queued(VideoTaskInProgress),
    #[serde(rename = "in_progress")]
    InProgress(VideoTaskInProgress),
    #[serde(rename = "completed")]
    Completed(VideoTaskCompleted),
    #[serde(rename = "failed")]
    Failed { id: String, error: String },
}

/// Query task detail response (generic)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoTaskDetailResponse {
    pub id: String,
    pub object: String,
    pub model: String,
    pub status: String,
    pub progress: Option<i32>,
    pub url: Option<String>,
    pub created_at: i64,
    pub completed_at: Option<i64>,
    pub error: Option<String>,
}

impl VideoTaskDetailResponse {
    /// Check if task is completed
    pub fn is_completed(&self) -> bool {
        self.status == "completed"
    }

    /// Check if task is failed
    pub fn is_failed(&self) -> bool {
        self.status == "failed"
    }

    /// Check if task is still processing
    pub fn is_processing(&self) -> bool {
        self.status == "in_progress" || self.status == "submitted" || self.status == "queued"
    }

    /// Get video download URL
    pub fn get_download_url(&self) -> Option<&str> {
        self.url.as_deref()
    }
}

// ===== Model Detection Helpers =====

/// Check if a model key is a Veo model
pub fn is_veo_model(model_key: &str) -> bool {
    model_key.starts_with("veo-")
}

/// Check if a model key is a Sora model
pub fn is_sora_model(model_key: &str) -> bool {
    model_key.starts_with("sora-")
}

/// Check if a model supports image-to-video (FL models)
pub fn supports_image_to_video(model_key: &str) -> bool {
    model_key.contains("-fl")
}

// ===== Error Response =====

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LaoZhangErrorResponse {
    pub error: ErrorDetail,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorDetail {
    pub message: String,
    pub r#type: String,
    pub code: Option<String>,
}

// ===== Image Generation DTOs =====

/// Image generation request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateImageRequest {
    pub model: String,
    pub prompt: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub n: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quality: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub style: Option<String>,
    /// Response format: "url" or "b64_json". Defaults to model-specific behavior.
    /// Some models (e.g. gpt-4o-image) may return b64_json by default.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_format: Option<String>,
}

impl Default for CreateImageRequest {
    fn default() -> Self {
        Self {
            model: "gpt-4o-image".to_string(),
            prompt: String::new(),
            n: Some(1),
            size: Some("1024x1024".to_string()),
            quality: None,
            style: None,
            response_format: None,
        }
    }
}

/// Image generation response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageResponse {
    pub created: i64,
    pub data: Vec<ImageData>,
}

/// Single image data in response
///
/// The response can contain either a URL or base64-encoded image data,
/// depending on the model and request parameters.
/// - `url`: Direct URL to the generated image (e.g., dall-e-3, flux)
/// - `b64_json`: Base64-encoded image data (e.g., gpt-4o-image default)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageData {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub b64_json: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub revised_prompt: Option<String>,
}

impl ImageData {
    /// Get the image URL if available
    pub fn get_url(&self) -> Option<&str> {
        self.url.as_deref()
    }

    /// Get the base64-encoded image data if available
    pub fn get_b64_json(&self) -> Option<&str> {
        self.b64_json.as_deref()
    }

    /// Check if image data is available (either URL or base64)
    pub fn has_image(&self) -> bool {
        self.url.is_some() || self.b64_json.is_some()
    }
}

impl ImageResponse {
    /// Get the first image URL (only from data items that have a URL)
    pub fn get_first_url(&self) -> Option<&str> {
        self.data.iter().find_map(|img| img.url.as_deref())
    }

    /// Get all image URLs (only from data items that have URLs)
    pub fn get_all_urls(&self) -> Vec<&str> {
        self.data
            .iter()
            .filter_map(|img| img.url.as_deref())
            .collect()
    }

    /// Get the first base64-encoded image data
    pub fn get_first_b64(&self) -> Option<&str> {
        self.data.iter().find_map(|img| img.b64_json.as_deref())
    }

    /// Check if any image was generated successfully
    pub fn has_images(&self) -> bool {
        self.data.iter().any(|img| img.has_image())
    }

    /// Get total number of generated images
    pub fn image_count(&self) -> usize {
        self.data.iter().filter(|img| img.has_image()).count()
    }
}
