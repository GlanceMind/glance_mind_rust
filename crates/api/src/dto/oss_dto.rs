//! OSS Upload DTOs

use serde::Serialize;

/// Response for image upload
#[derive(Debug, Clone, Serialize)]
pub struct UploadImageResponse {
    /// Public URL of the uploaded image
    pub image_url: String,
    /// Generated filename
    pub filename: String,
    /// File size in bytes
    pub size: usize,
}
