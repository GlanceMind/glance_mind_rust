//! OSS Service Module
//!
//! Provides functionality to upload images to Aliyun OSS using reqwest directly.
//! This implementation uses HMAC-SHA1 signing for authentication.
//!
//! ## Environment Variables
//!
//! - `OSS_ACCESS_KEY_ID`: Aliyun Access Key ID
//! - `OSS_ACCESS_KEY_SECRET`: Aliyun Access Key Secret
//! - `OSS_ENDPOINT`: OSS endpoint (e.g., oss-ap-southeast-1.aliyuncs.com)
//! - `OSS_BUCKET`: OSS bucket name

use crate::error::api_error::ApiError;
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use chrono::Utc;
use hmac::{Hmac, Mac};
use sha1::Sha1;
use tracing::{error, info};
use uuid::Uuid;

type HmacSha1 = Hmac<Sha1>;

/// OSS Service configuration
#[derive(Debug, Clone)]
pub struct OssConfig {
    pub access_key_id: String,
    pub access_key_secret: String,
    pub endpoint: String,
    pub bucket: String,
}

impl OssConfig {
    /// Create config from environment variables
    pub fn from_env() -> Result<Self, ApiError> {
        let access_key_id = std::env::var("OSS_ACCESS_KEY_ID")
            .map_err(|_| ApiError::InternalServerError("OSS_ACCESS_KEY_ID not set".to_string()))?;
        let access_key_secret = std::env::var("OSS_ACCESS_KEY_SECRET").map_err(|_| {
            ApiError::InternalServerError("OSS_ACCESS_KEY_SECRET not set".to_string())
        })?;
        let endpoint = std::env::var("OSS_ENDPOINT")
            .map_err(|_| ApiError::InternalServerError("OSS_ENDPOINT not set".to_string()))?;
        let bucket = std::env::var("OSS_BUCKET")
            .map_err(|_| ApiError::InternalServerError("OSS_BUCKET not set".to_string()))?;

        Ok(Self {
            access_key_id,
            access_key_secret,
            endpoint,
            bucket,
        })
    }

    /// Check if OSS is configured
    pub fn is_configured() -> bool {
        std::env::var("OSS_ACCESS_KEY_ID").is_ok()
            && std::env::var("OSS_ACCESS_KEY_SECRET").is_ok()
            && std::env::var("OSS_ENDPOINT").is_ok()
            && std::env::var("OSS_BUCKET").is_ok()
    }
}

/// OSS Service for uploading images
pub struct OssService {
    config: OssConfig,
    client: reqwest::Client,
}

impl OssService {
    /// Create a new OSS service
    pub fn new(config: OssConfig) -> Self {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(120))
            .build()
            .unwrap_or_default();

        Self { config, client }
    }

    /// Create from environment variables
    pub fn from_env() -> Result<Self, ApiError> {
        let config = OssConfig::from_env()?;
        Ok(Self::new(config))
    }

    /// Upload image to OSS (async version using reqwest)
    ///
    /// # Arguments
    /// * `data` - Image binary data
    /// * `user_id` - User ID for organizing files
    /// * `original_filename` - Original filename to preserve extension
    /// * `content_type` - MIME type of the image
    ///
    /// # Returns
    /// * `Ok(UploadResult)` - The upload result with URL and metadata
    /// * `Err(ApiError)` - Upload failed
    pub async fn upload_image(
        &self,
        data: Vec<u8>,
        user_id: i32,
        original_filename: String,
        content_type: String,
    ) -> Result<UploadResult, ApiError> {
        // Generate unique filename
        let extension = Self::get_extension_static(&original_filename, &content_type);
        let timestamp = Utc::now().format("%Y%m%d%H%M%S");
        let uuid_short = Uuid::new_v4().to_string()[..8].to_string();
        let new_filename = format!("{}_{}.{}", timestamp, uuid_short, extension);

        // Build object path: aipub/user_{user_id}/{filename}
        let object_path = format!("aipub/user_{}/{}", user_id, new_filename);
        let data_len = data.len();

        // Build the URL
        let url = format!(
            "https://{}.{}/{}",
            self.config.bucket, self.config.endpoint, object_path
        );

        // Generate date header
        let date = Utc::now().format("%a, %d %b %Y %H:%M:%S GMT").to_string();

        // Build the string to sign
        // Format: VERB + "\n" + Content-MD5 + "\n" + Content-Type + "\n" + Date + "\n" + CanonicalizedOSSHeaders + CanonicalizedResource
        let string_to_sign = format!(
            "PUT\n\n{}\n{}\n/{}/{}",
            content_type, date, self.config.bucket, object_path
        );

        // Calculate signature
        let signature = self.calculate_signature(&string_to_sign)?;

        // Build authorization header
        let authorization = format!("OSS {}:{}", self.config.access_key_id, signature);

        info!(
            "Uploading to OSS: {} ({} bytes)",
            object_path, data_len
        );

        // Send the request
        let response = self
            .client
            .put(&url)
            .header("Date", &date)
            .header("Content-Type", &content_type)
            .header("Authorization", &authorization)
            .body(data)
            .send()
            .await
            .map_err(|e| {
                error!("Failed to send request to OSS: {:?}", e);
                ApiError::InternalServerError(format!("OSS request failed: {}", e))
            })?;

        // Check response status
        let status = response.status();
        if !status.is_success() {
            let error_body = response.text().await.unwrap_or_default();
            error!("OSS upload failed with status {}: {}", status, error_body);
            return Err(ApiError::InternalServerError(format!(
                "OSS upload failed with status {}: {}",
                status, error_body
            )));
        }

        // Build public URL
        let image_url = self.build_url(&object_path);

        info!("Image uploaded to OSS: {} ({} bytes)", image_url, data_len);

        Ok(UploadResult {
            image_url,
            filename: new_filename,
            object_path,
            size: data_len,
        })
    }

    /// Calculate HMAC-SHA1 signature
    fn calculate_signature(&self, string_to_sign: &str) -> Result<String, ApiError> {
        let mut mac = HmacSha1::new_from_slice(self.config.access_key_secret.as_bytes())
            .map_err(|e| ApiError::InternalServerError(format!("HMAC key error: {}", e)))?;

        mac.update(string_to_sign.as_bytes());
        let result = mac.finalize();
        let signature = BASE64.encode(result.into_bytes());

        Ok(signature)
    }

    /// Build public URL for an object
    fn build_url(&self, object_path: &str) -> String {
        format!(
            "https://{}.{}/{}",
            self.config.bucket, self.config.endpoint, object_path
        )
    }

    /// Upload video to OSS (async version using reqwest)
    ///
    /// # Arguments
    /// * `data` - Video binary data
    /// * `user_id` - User ID for organizing files
    /// * `original_filename` - Original filename to preserve extension
    /// * `content_type` - MIME type of the video
    ///
    /// # Returns
    /// * `Ok(UploadResult)` - The upload result with URL and metadata
    /// * `Err(ApiError)` - Upload failed
    pub async fn upload_video(
        &self,
        data: Vec<u8>,
        user_id: i32,
        original_filename: String,
        content_type: String,
    ) -> Result<UploadResult, ApiError> {
        // Generate unique filename
        let extension = Self::get_video_extension_static(&original_filename, &content_type);
        let timestamp = Utc::now().format("%Y%m%d%H%M%S");
        let uuid_short = Uuid::new_v4().to_string()[..8].to_string();
        let new_filename = format!("{}_{}.{}", timestamp, uuid_short, extension);

        // Build object path: materials/user_{user_id}/{filename}
        let object_path = format!("materials/user_{}/{}", user_id, new_filename);
        let data_len = data.len();

        // Build the URL
        let url = format!(
            "https://{}.{}/{}",
            self.config.bucket, self.config.endpoint, object_path
        );

        // Generate date header
        let date = Utc::now().format("%a, %d %b %Y %H:%M:%S GMT").to_string();

        // Build the string to sign
        let string_to_sign = format!(
            "PUT\n\n{}\n{}\n/{}/{}",
            content_type, date, self.config.bucket, object_path
        );

        // Calculate signature
        let signature = self.calculate_signature(&string_to_sign)?;

        // Build authorization header
        let authorization = format!("OSS {}:{}", self.config.access_key_id, signature);

        info!(
            "Uploading video to OSS: {} ({} bytes)",
            object_path, data_len
        );

        // Send the request
        let response = self
            .client
            .put(&url)
            .header("Date", &date)
            .header("Content-Type", &content_type)
            .header("Authorization", &authorization)
            .body(data)
            .send()
            .await
            .map_err(|e| {
                error!("Failed to send video request to OSS: {:?}", e);
                ApiError::InternalServerError(format!("OSS request failed: {}", e))
            })?;

        // Check response status
        let status = response.status();
        if !status.is_success() {
            let error_body = response.text().await.unwrap_or_default();
            error!("OSS video upload failed with status {}: {}", status, error_body);
            return Err(ApiError::InternalServerError(format!(
                "OSS video upload failed with status {}: {}",
                status, error_body
            )));
        }

        // Build public URL
        let video_url = self.build_url(&object_path);

        info!("Video uploaded to OSS: {} ({} bytes)", video_url, data_len);

        Ok(UploadResult {
            image_url: video_url, // Reuse image_url field for video_url
            filename: new_filename,
            object_path,
            size: data_len,
        })
    }

    /// Get file extension from filename or content type (static version)
    fn get_extension_static(filename: &str, content_type: &str) -> String {
        // Try to get from filename first
        if let Some(ext) = filename.rsplit('.').next() {
            let ext_lower = ext.to_lowercase();
            if matches!(
                ext_lower.as_str(),
                "jpg" | "jpeg" | "png" | "gif" | "webp" | "bmp"
            ) {
                return match ext_lower.as_str() {
                    "jpg" | "jpeg" => "jpg",
                    "png" => "png",
                    "gif" => "gif",
                    "webp" => "webp",
                    "bmp" => "bmp",
                    _ => "jpg",
                }
                .to_string();
            }
        }

        // Fall back to content type
        match content_type {
            "image/jpeg" => "jpg",
            "image/png" => "png",
            "image/gif" => "gif",
            "image/webp" => "webp",
            "image/bmp" => "bmp",
            _ => "jpg", // Default to jpg
        }
        .to_string()
    }

    /// Get video file extension from filename or content type (static version)
    fn get_video_extension_static(filename: &str, content_type: &str) -> String {
        // Try to get from filename first
        if let Some(ext) = filename.rsplit('.').next() {
            let ext_lower = ext.to_lowercase();
            if matches!(
                ext_lower.as_str(),
                "mp4" | "mov" | "avi" | "webm" | "mkv" | "flv" | "wmv" | "m4v"
            ) {
                return ext_lower;
            }
        }

        // Fall back to content type
        match content_type {
            "video/mp4" | "video/x-m4v" => "mp4",
            "video/quicktime" => "mov",
            "video/x-msvideo" => "avi",
            "video/webm" => "webm",
            "video/x-matroska" => "mkv",
            "video/x-flv" => "flv",
            "video/x-ms-wmv" => "wmv",
            _ => "mp4", // Default to mp4
        }
        .to_string()
    }
}

/// Result of an upload operation
#[derive(Debug, Clone)]
pub struct UploadResult {
    /// Public URL of the uploaded image
    pub image_url: String,
    /// Generated filename
    pub filename: String,
    /// Full object path in OSS
    pub object_path: String,
    /// File size in bytes
    pub size: usize,
}
