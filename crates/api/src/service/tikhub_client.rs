use reqwest::{multipart, Client};
use serde_json::Value as JsonValue;
use std::time::Duration;

use crate::dto::video_dto::{
    TikHubCreateVideoData, TikHubResponse, TikHubTaskDetailData, TikHubTaskStatusData,
    TikHubUploadImageData,
};
use crate::error::api_error::ApiError;

#[derive(Clone)]
pub struct TikHubClient {
    client: Client,
    api_token: String,
    base_url: String,
}

impl TikHubClient {
    pub fn new(api_token: String, base_url: String) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .expect("Failed to create HTTP client");

        Self {
            client,
            api_token,
            base_url,
        }
    }

    /// Upload image to get media_id
    /// POST /api/v1/sora2/upload_image
    pub async fn upload_image(
        &self,
        image_data: &[u8],
        content_type: &str,
    ) -> Result<TikHubUploadImageData, ApiError> {
        let url = format!("{}/api/v1/sora2/upload_image", self.base_url);

        // Create multipart form
        let filename = format!(
            "image.{}",
            Self::get_extension_from_content_type(content_type)
        );
        let part = multipart::Part::bytes(image_data.to_vec())
            .file_name(filename)
            .mime_str(content_type)
            .map_err(|e| {
                tracing::error!("Failed to create multipart: {:?}", e);
                ApiError::InternalServerError(format!("Failed to create upload request: {}", e))
            })?;

        let form = multipart::Form::new().part("file", part);

        // Send request
        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_token))
            .multipart(form)
            .send()
            .await
            .map_err(|e| {
                tracing::error!("TikHub upload image request failed: {:?}", e);
                ApiError::InternalServerError(format!("Upload image request failed: {}", e))
            })?;

        let status = response.status();
        let body = response.text().await.map_err(|e| {
            tracing::error!("Failed to read TikHub response: {:?}", e);
            ApiError::InternalServerError("Failed to read response".to_string())
        })?;

        if !status.is_success() {
            tracing::error!("TikHub upload image failed: status={}, body={}", status, body);
            return Err(ApiError::InternalServerError(format!(
                "TikHub API error: {}",
                body
            )));
        }

        // Parse response
        let tikhub_response: TikHubResponse<TikHubUploadImageData> = serde_json::from_str(&body)
            .map_err(|e| {
                tracing::error!("Failed to parse TikHub response: {:?}, body: {}", e, body);
                ApiError::InternalServerError("Failed to parse response".to_string())
            })?;

        if tikhub_response.code != 200 {
            return Err(ApiError::InternalServerError(format!(
                "TikHub API error: {}",
                tikhub_response.message
            )));
        }

        tikhub_response
            .data
            .ok_or_else(|| ApiError::InternalServerError("TikHub response missing data field".to_string()))
    }

    /// Create video generation task
    /// POST /api/v1/sora2/create_video
    pub async fn create_video(
        &self,
        prompt: Option<String>,
        media_id: Option<String>,
        orientation: Option<String>, // Video orientation parameter
    ) -> Result<TikHubCreateVideoData, ApiError> {
        let url = format!("{}/api/v1/sora2/create_video", self.base_url);

        let mut body = serde_json::Map::new();
        if let Some(p) = prompt {
            body.insert("prompt".to_string(), JsonValue::String(p));
        }
        if let Some(m) = media_id {
            body.insert("media_id".to_string(), JsonValue::String(m));
        }
        // Add orientation parameter, default to portrait
        let orientation_value = orientation.unwrap_or_else(|| "portrait".to_string());
        body.insert(
            "orientation".to_string(),
            JsonValue::String(orientation_value),
        );

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_token))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| {
                tracing::error!("TikHub create video request failed: {:?}", e);
                ApiError::InternalServerError(format!("Create video request failed: {}", e))
            })?;

        let status = response.status();
        let body_text = response.text().await.map_err(|e| {
            tracing::error!("Failed to read TikHub response: {:?}", e);
            ApiError::InternalServerError("Failed to read response".to_string())
        })?;

        if !status.is_success() {
            tracing::error!("TikHub create video failed: status={}, body={}", status, body_text);
            return Err(ApiError::InternalServerError(format!(
                "TikHub API error: {}",
                body_text
            )));
        }

        let tikhub_response: TikHubResponse<TikHubCreateVideoData> =
            serde_json::from_str(&body_text).map_err(|e| {
                tracing::error!("Failed to parse TikHub response: {:?}, body: {}", e, body_text);
                ApiError::InternalServerError("Failed to parse response".to_string())
            })?;

        if tikhub_response.code != 200 {
            return Err(ApiError::InternalServerError(format!(
                "TikHub API error: {}",
                tikhub_response.message
            )));
        }

        tikhub_response
            .data
            .ok_or_else(|| ApiError::InternalServerError("TikHub response missing data field".to_string()))
    }

    /// Query task status (free API)
    /// GET /api/v1/sora2/get_task_status?task_id=xxx
    pub async fn get_task_status(&self, task_id: &str) -> Result<TikHubTaskStatusData, ApiError> {
        let url = format!(
            "{}/api/v1/sora2/get_task_status?task_id={}",
            self.base_url, task_id
        );

        let response = self
            .client
            .get(&url)
            .header("Authorization", format!("Bearer {}", self.api_token))
            .send()
            .await
            .map_err(|e| {
                tracing::error!("TikHub query task status request failed: {:?}", e);
                ApiError::InternalServerError(format!("Query task status failed: {}", e))
            })?;

        let status = response.status();
        let body = response.text().await.map_err(|e| {
            tracing::error!("Failed to read TikHub response: {:?}", e);
            ApiError::InternalServerError("Failed to read response".to_string())
        })?;

        if !status.is_success() {
            tracing::error!("TikHub query task status failed: status={}, body={}", status, body);
            return Err(ApiError::InternalServerError(format!(
                "TikHub API error: {}",
                body
            )));
        }

        let tikhub_response: TikHubResponse<TikHubTaskStatusData> = serde_json::from_str(&body)
            .map_err(|e| {
                tracing::error!("Failed to parse TikHub response: {:?}, body: {}", e, body);
                ApiError::InternalServerError("Failed to parse response".to_string())
            })?;

        if tikhub_response.code != 200 {
            return Err(ApiError::InternalServerError(format!(
                "TikHub API error: {}",
                tikhub_response.message
            )));
        }

        tikhub_response
            .data
            .ok_or_else(|| ApiError::InternalServerError("TikHub response missing data field".to_string()))
    }

    /// Get task details (watermark-free video) ($0.05/request)
    /// GET /api/v1/sora2/get_task_detail?task_id=xxx
    pub async fn get_task_detail(&self, task_id: &str) -> Result<TikHubTaskDetailData, ApiError> {
        let url = format!(
            "{}/api/v1/sora2/get_task_detail?task_id={}",
            self.base_url, task_id
        );

        let response = self
            .client
            .get(&url)
            .header("Authorization", format!("Bearer {}", self.api_token))
            .send()
            .await
            .map_err(|e| {
                tracing::error!("TikHub get task detail request failed: {:?}", e);
                ApiError::InternalServerError(format!("Get task detail failed: {}", e))
            })?;

        let status = response.status();
        let body = response.text().await.map_err(|e| {
            tracing::error!("Failed to read TikHub response: {:?}", e);
            ApiError::InternalServerError("Failed to read response".to_string())
        })?;

        if !status.is_success() {
            tracing::error!("TikHub get task detail failed: status={}, body={}", status, body);
            return Err(ApiError::InternalServerError(format!(
                "TikHub API error: {}",
                body
            )));
        }

        let tikhub_response: TikHubResponse<TikHubTaskDetailData> = serde_json::from_str(&body)
            .map_err(|e| {
                tracing::error!("Failed to parse TikHub response: {:?}, body: {}", e, body);
                ApiError::InternalServerError("Failed to parse response".to_string())
            })?;

        if tikhub_response.code != 200 {
            return Err(ApiError::InternalServerError(format!(
                "TikHub API error: {}",
                tikhub_response.message
            )));
        }

        tikhub_response
            .data
            .ok_or_else(|| ApiError::InternalServerError("TikHub response missing data field".to_string()))
    }

    /// Get file extension from content-type
    fn get_extension_from_content_type(content_type: &str) -> &'static str {
        match content_type {
            "image/jpeg" => "jpg",
            "image/png" => "png",
            "image/webp" => "webp",
            "image/gif" => "gif",
            _ => "jpg",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_extension_from_content_type() {
        assert_eq!(
            TikHubClient::get_extension_from_content_type("image/jpeg"),
            "jpg"
        );
        assert_eq!(
            TikHubClient::get_extension_from_content_type("image/png"),
            "png"
        );
        assert_eq!(
            TikHubClient::get_extension_from_content_type("image/webp"),
            "webp"
        );
    }
}
