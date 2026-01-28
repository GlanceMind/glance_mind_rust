use crate::dto::laozhang_dto::{
    CreateVideoFromImageRequest, CreateVideoFromTextRequest, VideoTaskDetailResponse,
    VideoTaskResponse,
};
use crate::error::{
    api_error::ApiError, business_error::BusinessError, infrastructure_error::InfrastructureError,
};
use reqwest::{multipart, Client};
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// LaoZhang.ai API Client
///
/// For calling video generation service at https://api.laozhang.ai
#[derive(Clone)]
pub struct LaoZhangClient {
    client: Client,
    api_key: String,
    base_url: String,
}

impl LaoZhangClient {
    /// Create new LaoZhang Client
    ///
    /// # Arguments
    /// * `api_key` - API key
    /// * `base_url` - API base URL, defaults to "https://api.laozhang.ai"
    pub fn new(api_key: String, base_url: Option<String>) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(300)) // 5 minute timeout
            .build()
            .expect("Failed to build HTTP client");

        Self {
            client,
            api_key,
            base_url: base_url.unwrap_or_else(|| "https://api.laozhang.ai".to_string()),
        }
    }

    /// Text-to-video - Generate video from text prompt
    ///
    /// # Arguments
    /// * `request` - Text-to-video request parameters
    ///
    /// # Returns
    /// Returns video task info including task ID
    pub async fn create_video_from_text(
        &self,
        request: CreateVideoFromTextRequest,
    ) -> Result<VideoTaskResponse, ApiError> {
        let url = format!("{}/v1/videos", self.base_url);

        tracing::info!(
            "Starting text-to-video task: model={}, prompt='{}', size={}, seconds={}, prompt_length={} chars",
            request.model,
            request.prompt,
            request.size,
            request.seconds,
            request.prompt.len()
        );

        // Validate prompt length (per LaoZhang docs, prompt should have sufficient description)
        if request.prompt.trim().len() < 10 {
            tracing::warn!(
                "Prompt too short ({} chars), may be rejected by LaoZhang API. Recommend at least 20 chars with detailed description.",
                request.prompt.trim().len()
            );
        }

        tracing::debug!(
            "LaoZhang text-to-video request body: {}",
            serde_json::to_string_pretty(&request).unwrap_or_default()
        );

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
            .map_err(|e| {
                tracing::error!("LaoZhang text-to-video request failed: {:?}", e);
                ApiError::InfrastructureError(InfrastructureError::ExternalApiRequestFailed(
                    e.to_string(),
                ))
            })?;

        let status = response.status();
        let body_text = response.text().await.map_err(|e| {
            tracing::error!("Failed to read LaoZhang response: {:?}", e);
            ApiError::InfrastructureError(InfrastructureError::ExternalApiResponseParsingFailed(
                e.to_string(),
            ))
        })?;

        tracing::info!(
            "LaoZhang API response: status={}, body_length={} bytes",
            status,
            body_text.len()
        );
        tracing::debug!("LaoZhang response content: {}", body_text);

        if !status.is_success() {
            tracing::error!(
                "LaoZhang text-to-video failed: status={}, body={}",
                status,
                body_text
            );
            return Err(ApiError::InfrastructureError(
                InfrastructureError::ExternalApiRequestFailed(format!(
                    "LaoZhang API error: {} - {}",
                    status, body_text
                )),
            ));
        }

        let task_response: VideoTaskResponse = serde_json::from_str(&body_text).map_err(|e| {
            tracing::error!(
                "Failed to parse LaoZhang response: {:?}, body: {}",
                e,
                body_text
            );
            ApiError::InfrastructureError(InfrastructureError::ExternalApiResponseParsingFailed(
                e.to_string(),
            ))
        })?;

        tracing::info!(
            "LaoZhang text-to-video task created successfully: task_id={}, status={}, prompt='{}'",
            task_response.id,
            task_response.status,
            request.prompt
        );
        Ok(task_response)
    }

    /// Get MIME type from filename
    fn get_mime_type_from_filename(filename: &str) -> Result<&'static str, ApiError> {
        let extension = filename
            .rsplit('.')
            .next()
            .ok_or(ApiError::BusinessError(
                BusinessError::FilenameMissingExtension,
            ))?
            .to_lowercase();

        match extension.as_str() {
            "jpg" | "jpeg" => Ok("image/jpeg"),
            "png" => Ok("image/png"),
            "webp" => Ok("image/webp"),
            _ => Err(ApiError::BusinessError(
                BusinessError::UnsupportedImageFormat(extension),
            )),
        }
    }

    /// Image-to-video - Generate video from image and text prompt
    ///
    /// # Arguments
    /// * `request` - Image-to-video request parameters
    ///
    /// # Returns
    /// Returns video task info including task ID
    pub async fn create_video_from_image(
        &self,
        request: CreateVideoFromImageRequest,
    ) -> Result<VideoTaskResponse, ApiError> {
        let url = format!("{}/v1/videos", self.base_url);

        // Validate image size
        const MAX_IMAGE_SIZE: usize = 5 * 1024 * 1024; // 5MB
        if request.image_data.len() > MAX_IMAGE_SIZE {
            let size_mb = request.image_data.len() as f64 / 1024.0 / 1024.0;
            tracing::warn!(
                "Image too large: {:.2}MB (max 5MB), filename={}",
                size_mb,
                request.image_filename
            );
            return Err(ApiError::BusinessError(BusinessError::ImageTooLarge(
                size_mb,
            )));
        }

        // Dynamically get MIME type
        let mime_type = Self::get_mime_type_from_filename(&request.image_filename)?;

        let size_mb = request.image_data.len() as f64 / 1024.0 / 1024.0;
        tracing::info!(
            "Starting image-to-video task: model={}, prompt={}, size={}, seconds={}, image_size={:.2}MB, filename={}, mime_type={}",
            request.model,
            request.prompt,
            request.size,
            request.seconds,
            size_mb,
            request.image_filename,
            mime_type
        );

        // Build multipart form
        let part = multipart::Part::bytes(request.image_data.clone())
            .file_name(request.image_filename.clone())
            .mime_str(mime_type) // Use dynamic MIME type
            .map_err(|e| {
                tracing::error!("Failed to create multipart: {:?}", e);
                ApiError::InfrastructureError(InfrastructureError::ExternalApiRequestFailed(
                    e.to_string(),
                ))
            })?;

        let form = multipart::Form::new()
            .text("model", request.model.clone())
            .text("prompt", request.prompt.clone())
            .part("input_reference", part)
            .text("size", request.size.clone())
            .text("seconds", request.seconds.clone());

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .multipart(form)
            .send()
            .await
            .map_err(|e| {
                tracing::error!("LaoZhang image-to-video request failed: {:?}", e);
                ApiError::InfrastructureError(InfrastructureError::ExternalApiRequestFailed(
                    e.to_string(),
                ))
            })?;

        let status = response.status();
        let body_text = response.text().await.map_err(|e| {
            tracing::error!("Failed to read LaoZhang response: {:?}", e);
            ApiError::InfrastructureError(InfrastructureError::ExternalApiResponseParsingFailed(
                e.to_string(),
            ))
        })?;

        tracing::info!(
            "LaoZhang API response: status={}, body_length={} bytes",
            status,
            body_text.len()
        );

        if !status.is_success() {
            tracing::error!(
                "LaoZhang image-to-video failed: status={}, body={}",
                status,
                body_text
            );
            return Err(ApiError::InfrastructureError(
                InfrastructureError::ExternalApiRequestFailed(format!(
                    "LaoZhang API error: {} - {}",
                    status, body_text
                )),
            ));
        }

        tracing::debug!("LaoZhang response content: {}", body_text);

        let task_response: VideoTaskResponse = serde_json::from_str(&body_text).map_err(|e| {
            tracing::error!(
                "Failed to parse LaoZhang response: {:?}, body: {}",
                e,
                body_text
            );
            ApiError::InfrastructureError(InfrastructureError::ExternalApiResponseParsingFailed(
                e.to_string(),
            ))
        })?;

        tracing::info!(
            "LaoZhang image-to-video task created successfully: task_id={}, status={}",
            task_response.id,
            task_response.status
        );
        Ok(task_response)
    }

    /// Query task status
    ///
    /// # Arguments
    /// * `task_id` - Task ID
    ///
    /// # Returns
    /// Returns task details including status, progress, video URL, etc.
    pub async fn get_task_status(
        &self,
        task_id: &str,
    ) -> Result<VideoTaskDetailResponse, ApiError> {
        let url = format!("{}/v1/videos/{}", self.base_url, task_id);

        let response = self
            .client
            .get(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .send()
            .await
            .map_err(|e| {
                tracing::error!("LaoZhang query task status request failed: {:?}", e);
                ApiError::InfrastructureError(InfrastructureError::ExternalApiRequestFailed(
                    e.to_string(),
                ))
            })?;

        let status = response.status();
        let body_text = response.text().await.map_err(|e| {
            tracing::error!("Failed to read LaoZhang response: {:?}", e);
            ApiError::InfrastructureError(InfrastructureError::ExternalApiResponseParsingFailed(
                e.to_string(),
            ))
        })?;

        if !status.is_success() {
            tracing::error!(
                "LaoZhang query task status failed: status={}, body={}",
                status,
                body_text
            );
            return Err(ApiError::InfrastructureError(
                InfrastructureError::ExternalApiRequestFailed(format!(
                    "LaoZhang API error: {} - {}",
                    status, body_text
                )),
            ));
        }

        let task_detail: VideoTaskDetailResponse =
            serde_json::from_str(&body_text).map_err(|e| {
                tracing::error!(
                    "Failed to parse LaoZhang response: {:?}, body: {}",
                    e,
                    body_text
                );
                ApiError::InfrastructureError(
                    InfrastructureError::ExternalApiResponseParsingFailed(e.to_string()),
                )
            })?;

        tracing::info!(
            "LaoZhang query task status successful: task_id={}, status={}, progress={:?}",
            task_id,
            task_detail.status,
            task_detail.progress
        );

        Ok(task_detail)
    }

    /// Download video content
    ///
    /// # Arguments
    /// * `task_id` - Task ID
    ///
    /// # Returns
    /// Returns video file byte data
    pub async fn download_video(&self, task_id: &str) -> Result<Vec<u8>, ApiError> {
        let url = format!("{}/v1/videos/{}/content", self.base_url, task_id);

        let response = self
            .client
            .get(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .send()
            .await
            .map_err(|e| {
                tracing::error!("LaoZhang download video request failed: {:?}", e);
                ApiError::InfrastructureError(InfrastructureError::ExternalApiRequestFailed(
                    e.to_string(),
                ))
            })?;

        let status = response.status();
        if !status.is_success() {
            let body_text = response.text().await.unwrap_or_default();
            tracing::error!(
                "LaoZhang download video failed: status={}, body={}",
                status,
                body_text
            );
            return Err(ApiError::InfrastructureError(
                InfrastructureError::ExternalApiRequestFailed(format!(
                    "LaoZhang API error: {} - {}",
                    status, body_text
                )),
            ));
        }

        let video_bytes = response.bytes().await.map_err(|e| {
            tracing::error!("Failed to read video data: {:?}", e);
            ApiError::InternalServerError(format!("Failed to read video: {}", e))
        })?;

        tracing::info!(
            "LaoZhang download video successful: task_id={}, size={} bytes",
            task_id,
            video_bytes.len()
        );

        Ok(video_bytes.to_vec())
    }

    /// Get video download URL (relative path)
    ///
    /// # Arguments
    /// * `task_id` - Task ID
    ///
    /// # Returns
    /// Returns relative path for video download
    pub fn get_video_url(&self, task_id: &str) -> String {
        format!("/v1/videos/{}/content", task_id)
    }

    /// Get full video download URL (absolute path)
    ///
    /// # Arguments
    /// * `task_id` - Task ID
    ///
    /// # Returns
    /// Returns full URL for video download
    pub fn get_full_video_url(&self, task_id: &str) -> String {
        format!("{}/v1/videos/{}/content", self.base_url, task_id)
    }

    // ========== Veo-3.1 Model Convenience Functions ==========

    /// Veo-3.1 text-to-video - Use veo-3.1 model to generate video from text prompt
    ///
    /// # Arguments
    /// * `prompt` - Text prompt
    ///
    /// # Returns
    /// Returns video task info including task ID
    ///
    /// # Example
    /// ```rust,no_run
    /// # use glance_mind_api::service::laozhang_client::LaoZhangClient;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = LaoZhangClient::new("your_api_key".to_string(), None);
    /// let task = client.create_video_from_text_veo(
    ///     "A cute cat playing with a ball in a sunny garden".to_string()
    /// ).await?;
    /// println!("Task ID: {}", task.id);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn create_video_from_text_veo(
        &self,
        prompt: String,
    ) -> Result<VideoTaskResponse, ApiError> {
        let request = CreateVideoFromTextRequest {
            model: "veo-3.1".to_string(),
            prompt,
            size: "720x1280".to_string(), // veo-3.1 default portrait
            seconds: "8".to_string(),     // veo-3.1 default 8 seconds
        };

        tracing::info!(
            "Creating text-to-video with veo-3.1 model: prompt='{}'",
            request.prompt
        );

        self.create_video_from_text(request).await
    }

    /// Veo-3.1-FL image-to-video - Use single image as reference to generate video
    ///
    /// # Arguments
    /// * `prompt` - Text prompt describing how to animate the image
    /// * `image_data` - Image file byte data
    /// * `image_filename` - Image filename (used to determine MIME type)
    ///
    /// # Returns
    /// Returns video task info including task ID
    ///
    /// # Example
    /// ```rust,no_run
    /// # use glance_mind_api::service::laozhang_client::LaoZhangClient;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = LaoZhangClient::new("your_api_key".to_string(), None);
    /// let image_data = std::fs::read("cat.jpg")?;
    /// let task = client.create_video_from_image_veo(
    ///     "Make the cat slowly blink its eyes".to_string(),
    ///     image_data,
    ///     "cat.jpg".to_string()
    /// ).await?;
    /// println!("Task ID: {}", task.id);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn create_video_from_image_veo(
        &self,
        prompt: String,
        image_data: Vec<u8>,
        image_filename: String,
    ) -> Result<VideoTaskResponse, ApiError> {
        let request = CreateVideoFromImageRequest {
            model: "veo-3.1-fl".to_string(),
            prompt,
            image_data,
            image_filename,
            size: "720x1280".to_string(), // veo-3.1-fl default portrait
            seconds: "8".to_string(),     // veo-3.1-fl default 8 seconds
        };

        tracing::info!(
            "Creating image-to-video with veo-3.1-fl model: prompt='{}', image={}",
            request.prompt,
            request.image_filename
        );

        self.create_video_from_image(request).await
    }

    /// Veo-3.1 dual-image-to-video - Use two images as start and end frames to generate transition video
    ///
    /// # Arguments
    /// * `prompt` - Text prompt describing video transition effect
    /// * `start_frame_data` - Start frame image byte data
    /// * `start_frame_filename` - Start frame image filename
    /// * `end_frame_data` - End frame image byte data
    /// * `end_frame_filename` - End frame image filename
    /// * `landscape` - Whether to use landscape mode (true: veo-3.1-landscape-fl, false: veo-3.1-fl)
    ///
    /// # Returns
    /// Returns video task info including task ID
    ///
    /// # Example
    /// ```rust,no_run
    /// # use glance_mind_api::service::laozhang_client::LaoZhangClient;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = LaoZhangClient::new("your_api_key".to_string(), None);
    /// let start_frame = std::fs::read("start.jpg")?;
    /// let end_frame = std::fs::read("end.jpg")?;
    /// let task = client.create_video_from_two_images_veo(
    ///     "Animate this scene with futuristic tech effects".to_string(),
    ///     start_frame,
    ///     "start.jpg".to_string(),
    ///     end_frame,
    ///     "end.jpg".to_string(),
    ///     true // Use landscape mode
    /// ).await?;
    /// println!("Task ID: {}", task.id);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn create_video_from_two_images_veo(
        &self,
        prompt: String,
        start_frame_data: Vec<u8>,
        start_frame_filename: String,
        end_frame_data: Vec<u8>,
        end_frame_filename: String,
        landscape: bool,
    ) -> Result<VideoTaskResponse, ApiError> {
        let url = format!("{}/v1/videos", self.base_url);

        // Select model based on landscape mode
        let model = if landscape {
            "veo-3.1-landscape-fl"
        } else {
            "veo-3.1-fl"
        };

        let size = if landscape {
            "1280x720" // Landscape
        } else {
            "720x1280" // Portrait
        };

        // Validate image sizes
        const MAX_IMAGE_SIZE: usize = 5 * 1024 * 1024; // 5MB
        if start_frame_data.len() > MAX_IMAGE_SIZE {
            return Err(ApiError::BusinessError(BusinessError::StartFrameTooLarge(
                start_frame_data.len() as f64 / 1024.0 / 1024.0,
            )));
        }
        if end_frame_data.len() > MAX_IMAGE_SIZE {
            return Err(ApiError::BusinessError(BusinessError::EndFrameTooLarge(
                end_frame_data.len() as f64 / 1024.0 / 1024.0,
            )));
        }

        tracing::info!(
            "Creating dual-image-to-video with {} model: prompt='{}', start={}, end={}, size={}",
            model,
            prompt,
            start_frame_filename,
            end_frame_filename,
            size
        );

        // Get MIME types
        let start_mime = Self::get_mime_type_from_filename(&start_frame_filename)?;
        let end_mime = Self::get_mime_type_from_filename(&end_frame_filename)?;

        // Build multipart form, add two input_reference fields
        let start_part = multipart::Part::bytes(start_frame_data)
            .file_name(start_frame_filename.clone())
            .mime_str(start_mime)
            .map_err(|e| {
                tracing::error!("Failed to create start frame multipart: {:?}", e);
                ApiError::InfrastructureError(InfrastructureError::ExternalApiRequestFailed(
                    e.to_string(),
                ))
            })?;

        let end_part = multipart::Part::bytes(end_frame_data)
            .file_name(end_frame_filename.clone())
            .mime_str(end_mime)
            .map_err(|e| {
                tracing::error!("Failed to create end frame multipart: {:?}", e);
                ApiError::InfrastructureError(InfrastructureError::ExternalApiRequestFailed(
                    e.to_string(),
                ))
            })?;

        let form = multipart::Form::new()
            .text("model", model.to_string())
            .text("prompt", prompt.clone())
            .part("input_reference", start_part)
            .part("input_reference", end_part);

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .multipart(form)
            .send()
            .await
            .map_err(|e| {
                tracing::error!("LaoZhang dual-image-to-video request failed: {:?}", e);
                ApiError::InfrastructureError(InfrastructureError::ExternalApiRequestFailed(
                    e.to_string(),
                ))
            })?;

        let status = response.status();
        let body_text = response.text().await.map_err(|e| {
            tracing::error!("Failed to read LaoZhang response: {:?}", e);
            ApiError::InfrastructureError(InfrastructureError::ExternalApiResponseParsingFailed(
                e.to_string(),
            ))
        })?;

        tracing::info!(
            "LaoZhang API response: status={}, body_length={} bytes",
            status,
            body_text.len()
        );

        if !status.is_success() {
            tracing::error!(
                "LaoZhang dual-image-to-video failed: status={}, body={}",
                status,
                body_text
            );
            return Err(ApiError::InfrastructureError(
                InfrastructureError::ExternalApiRequestFailed(format!(
                    "LaoZhang API error: {} - {}",
                    status, body_text
                )),
            ));
        }

        tracing::debug!("LaoZhang response content: {}", body_text);

        let task_response: VideoTaskResponse = serde_json::from_str(&body_text).map_err(|e| {
            tracing::error!(
                "Failed to parse LaoZhang response: {:?}, body: {}",
                e,
                body_text
            );
            ApiError::InfrastructureError(InfrastructureError::ExternalApiResponseParsingFailed(
                e.to_string(),
            ))
        })?;

        tracing::info!(
            "LaoZhang dual-image-to-video task created successfully: task_id={}, status={}",
            task_response.id,
            task_response.status
        );
        Ok(task_response)
    }

    /// Analyze video using Gemini model (for material prompt generation)
    ///
    /// Supported models:
    /// - gemini-2.5-pro: Detailed and accurate, recommended for complex video analysis
    /// - gemini-2.5-flash: Fast and cost-effective, suitable for batch processing
    ///
    /// # Arguments
    /// * `model` - Model name (e.g., "gemini-2.5-pro", "gemini-2.5-flash")
    /// * `video_url` - URL of the video to analyze
    /// * `prompt` - Analysis instruction prompt
    /// * `max_tokens` - Maximum tokens in response (optional)
    ///
    /// # Returns
    /// Returns the analysis result as a string
    pub async fn analyze_video(
        &self,
        model: &str,
        video_url: &str,
        prompt: &str,
        max_tokens: Option<u32>,
    ) -> Result<String, ApiError> {
        let url = format!("{}/v1/chat/completions", self.base_url);

        let request = VideoAnalysisRequest::new(model, prompt, video_url, max_tokens);

        tracing::info!(
            "Starting video analysis: model={}, video_url='{}', max_tokens={:?}",
            model,
            video_url.chars().take(50).collect::<String>(),
            max_tokens
        );

        tracing::debug!(
            "Video analysis request body: {}",
            serde_json::to_string_pretty(&request).unwrap_or_default()
        );

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
            .map_err(|e| {
                tracing::error!("Video analysis request failed: {:?}", e);
                ApiError::InfrastructureError(InfrastructureError::ExternalApiRequestFailed(
                    e.to_string(),
                ))
            })?;

        let status = response.status();
        let body_text = response.text().await.map_err(|e| {
            tracing::error!("Failed to read video analysis response: {:?}", e);
            ApiError::InfrastructureError(InfrastructureError::ExternalApiResponseParsingFailed(
                e.to_string(),
            ))
        })?;

        tracing::info!(
            "Video analysis response: status={}, body_len={}",
            status,
            body_text.len()
        );

        if !status.is_success() {
            tracing::error!(
                "Video analysis failed: status={}, body={}",
                status, body_text
            );

            if let Ok(error_response) = serde_json::from_str::<LaoZhangErrorResponse>(&body_text) {
                return Err(ApiError::InfrastructureError(
                    InfrastructureError::ExternalApiRequestFailed(format!(
                        "Video analysis error: {}",
                        error_response.error.message
                    )),
                ));
            }

            return Err(ApiError::InfrastructureError(
                InfrastructureError::ExternalApiRequestFailed(format!(
                    "Video analysis error: {} - {}",
                    status, body_text
                )),
            ));
        }

        tracing::debug!("Video analysis response content: {}", body_text);

        let analysis_response: VideoAnalysisResponse =
            serde_json::from_str(&body_text).map_err(|e| {
                tracing::error!(
                    "Failed to parse video analysis response: {:?}, body: {}",
                    e, body_text
                );
                ApiError::InfrastructureError(InfrastructureError::ExternalApiResponseParsingFailed(
                    e.to_string(),
                ))
            })?;

        let content = analysis_response
            .get_content()
            .ok_or_else(|| {
                ApiError::InfrastructureError(InfrastructureError::ExternalApiResponseParsingFailed(
                    "No content in video analysis response".to_string(),
                ))
            })?;

        tracing::info!(
            "Video analysis completed: id={}, model={}, content_len={}",
            analysis_response.id,
            analysis_response.model,
            content.len()
        );

        Ok(content.to_string())
    }
}

// ============================================================================
// Video Analysis Types (Gemini Vision API)
// ============================================================================

/// Content item for multimodal messages
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ContentItem {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "image_url")]
    ImageUrl { image_url: ImageUrlContent },
}

/// Image/Video URL content
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageUrlContent {
    pub url: String,
}

/// Message for chat completions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: Vec<ContentItem>,
}

/// Video analysis request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoAnalysisRequest {
    pub model: String,
    pub messages: Vec<ChatMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
}

impl VideoAnalysisRequest {
    /// Create a new video analysis request
    pub fn new(model: &str, prompt: &str, video_url: &str, max_tokens: Option<u32>) -> Self {
        Self {
            model: model.to_string(),
            messages: vec![ChatMessage {
                role: "user".to_string(),
                content: vec![
                    ContentItem::Text {
                        text: prompt.to_string(),
                    },
                    ContentItem::ImageUrl {
                        image_url: ImageUrlContent {
                            url: video_url.to_string(),
                        },
                    },
                ],
            }],
            max_tokens,
        }
    }
}

/// Choice in chat completion response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatChoice {
    pub index: u32,
    pub message: ChatResponseMessage,
    pub finish_reason: Option<String>,
}

/// Message in chat completion response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatResponseMessage {
    pub role: String,
    pub content: String,
}

/// Chat completion usage info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatUsage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

/// Video analysis response (chat completions format)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoAnalysisResponse {
    pub id: String,
    pub object: String,
    pub created: i64,
    pub model: String,
    pub choices: Vec<ChatChoice>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage: Option<ChatUsage>,
}

impl VideoAnalysisResponse {
    /// Get the first response content
    pub fn get_content(&self) -> Option<&str> {
        self.choices.first().map(|c| c.message.content.as_str())
    }
}

/// Error response from LaoZhang API
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LaoZhangErrorResponse {
    pub error: LaoZhangError,
}

/// Error details
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LaoZhangError {
    pub message: String,
    #[serde(rename = "type")]
    pub error_type: Option<String>,
    pub code: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========== Basic Tests ==========

    #[test]
    fn test_laozhang_client_new() {
        let client = LaoZhangClient::new(
            "test_api_key".to_string(),
            Some("https://api.laozhang.ai".to_string()),
        );
        assert_eq!(client.api_key, "test_api_key");
        assert_eq!(client.base_url, "https://api.laozhang.ai");
    }

    #[test]
    fn test_laozhang_client_default_base_url() {
        let client = LaoZhangClient::new("test_api_key".to_string(), None);
        assert_eq!(client.base_url, "https://api.laozhang.ai");
    }

    /// Test using example task ID from ai_call.md doc: video_abc123
    #[test]
    fn test_get_video_url_with_doc_example() {
        let client = LaoZhangClient::new("test_api_key".to_string(), None);
        let url = client.get_video_url("video_abc123");
        assert_eq!(url, "/v1/videos/video_abc123/content");
    }

    /// Test using example task ID from ai_call.md doc: video_def456
    #[test]
    fn test_get_video_url_with_image_example() {
        let client = LaoZhangClient::new("test_api_key".to_string(), None);
        let url = client.get_video_url("video_def456");
        assert_eq!(url, "/v1/videos/video_def456/content");
    }

    #[test]
    fn test_get_full_video_url() {
        let client = LaoZhangClient::new("test_api_key".to_string(), None);
        let url = client.get_full_video_url("video_abc123");
        assert_eq!(
            url,
            "https://api.laozhang.ai/v1/videos/video_abc123/content"
        );
    }

    // ========== Text-to-Video Request Tests ==========

    #[test]
    fn test_create_video_from_text_request_default() {
        let request = CreateVideoFromTextRequest::default();
        assert_eq!(request.model, "sora-2");
        assert_eq!(request.size, "1280x720");
        assert_eq!(request.seconds, "15");
        assert!(request.prompt.is_empty());
    }

    /// Test using text-to-video example parameters from ai_call.md doc
    #[test]
    fn test_create_video_from_text_request_with_doc_params() {
        let request = CreateVideoFromTextRequest {
            model: "sora-2".to_string(),
            prompt: "A cute cat playing with a ball in a sunny garden".to_string(),
            size: "1280x720".to_string(),
            seconds: "15".to_string(),
        };

        assert_eq!(request.model, "sora-2");
        assert_eq!(
            request.prompt,
            "A cute cat playing with a ball in a sunny garden"
        );
        assert_eq!(request.size, "1280x720");
        assert_eq!(request.seconds, "15");

        // Validate JSON serialization
        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("sora-2"));
        assert!(json.contains("cute cat"));
    }

    // ========== Image-to-Video Request Tests ==========

    #[test]
    fn test_create_video_from_image_request_default() {
        let request = CreateVideoFromImageRequest::default();
        assert_eq!(request.model, "sora-2");
        assert_eq!(request.size, "1280x720");
        assert_eq!(request.seconds, "10");
        assert_eq!(request.image_filename, "image.png");
        assert!(request.prompt.is_empty());
        assert!(request.image_data.is_empty());
    }

    /// Test using image-to-video example parameters from ai_call.md doc
    #[test]
    fn test_create_video_from_image_request_with_doc_params() {
        // Create test image data (1x1 PNG)
        let test_image = vec![0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];

        let request = CreateVideoFromImageRequest {
            model: "sora-2".to_string(),
            prompt: "Animate the scene in this image with natural dynamic effects".to_string(),
            image_data: test_image.clone(),
            image_filename: "image.png".to_string(),
            size: "1280x720".to_string(),
            seconds: "10".to_string(),
        };

        assert_eq!(request.model, "sora-2");
        assert_eq!(
            request.prompt,
            "Animate the scene in this image with natural dynamic effects"
        );
        assert_eq!(request.size, "1280x720");
        assert_eq!(request.seconds, "10");
        assert_eq!(request.image_filename, "image.png");
        assert_eq!(request.image_data, test_image);
    }

    // ========== Manual Tests (Requires API Key) ==========

    /// Manual test: Text-to-video - Using example parameters from ai_call.md
    ///
    /// Run with:
    /// ```bash
    /// export LAOZHANG_API_KEY="your_api_key"
    /// cargo test test_manual_create_video_from_text -- --ignored --nocapture
    /// ```
    #[tokio::test]
    #[ignore]
    async fn test_manual_create_video_from_text() {
        let api_key = std::env::var("LAOZHANG_API_KEY")
            .expect("Please set LAOZHANG_API_KEY environment variable");

        let client = LaoZhangClient::new(api_key, None);

        // Use doc example parameters
        let request = CreateVideoFromTextRequest {
            model: "sora-2".to_string(),
            prompt: "A cute cat playing with a ball in a sunny garden".to_string(),
            size: "1280x720".to_string(),
            seconds: "15".to_string(),
        };

        println!("\nTest: Text-to-Video");
        println!("====================================");
        println!("Request parameters:");
        println!("  model: {}", request.model);
        println!("  prompt: {}", request.prompt);
        println!("  size: {}", request.size);
        println!("  seconds: {}", request.seconds);
        println!();

        match client.create_video_from_text(request).await {
            Ok(response) => {
                println!("Request successful!");
                println!("  task_id: {}", response.id);
                println!("  object: {}", response.object);
                println!("  model: {}", response.model);
                println!("  status: {}", response.status);
                println!("  created: {}", response.created);
                println!("  expires: {:?}", response.expires);
                println!();
                println!("Use this task_id to query status:");
                println!(
                    "  cargo test test_manual_get_task_status -- --ignored --nocapture TASK_ID={}",
                    response.id
                );
            }
            Err(e) => {
                println!("Request failed: {:?}", e);
                panic!("Text-to-video request failed");
            }
        }
    }

    /// Manual test: Image-to-video - Using example parameters from ai_call.md
    ///
    /// Run with:
    /// ```bash
    /// export LAOZHANG_API_KEY="your_api_key"
    /// cargo test test_manual_create_video_from_image -- --ignored --nocapture
    /// ```
    #[tokio::test]
    #[ignore]
    async fn test_manual_create_video_from_image() {
        let api_key = std::env::var("LAOZHANG_API_KEY")
            .expect("Please set LAOZHANG_API_KEY environment variable");

        let client = LaoZhangClient::new(api_key, None);

        // Create small test image (1x1 transparent PNG)
        let test_image = vec![
            0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x48,
            0x44, 0x52, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x06, 0x00, 0x00,
            0x00, 0x1F, 0x15, 0xC4, 0x89, 0x00, 0x00, 0x00, 0x0A, 0x49, 0x44, 0x41, 0x54, 0x78,
            0x9C, 0x63, 0x00, 0x01, 0x00, 0x00, 0x05, 0x00, 0x01, 0x0D, 0x0A, 0x2D, 0xB4, 0x00,
            0x00, 0x00, 0x00, 0x49, 0x45, 0x4E, 0x44, 0xAE, 0x42, 0x60, 0x82,
        ];

        // Use doc example parameters
        let request = CreateVideoFromImageRequest {
            model: "sora-2".to_string(),
            prompt: "Animate the scene in this image with natural dynamic effects".to_string(),
            image_data: test_image,
            image_filename: "image.png".to_string(),
            size: "1280x720".to_string(),
            seconds: "10".to_string(),
        };

        println!("\nTest: Image-to-Video");
        println!("====================================");
        println!("Request parameters:");
        println!("  model: {}", request.model);
        println!("  prompt: {}", request.prompt);
        println!("  size: {}", request.size);
        println!("  seconds: {}", request.seconds);
        println!("  image_filename: {}", request.image_filename);
        println!("  image_size: {} bytes", request.image_data.len());
        println!();

        match client.create_video_from_image(request).await {
            Ok(response) => {
                println!("Request successful!");
                println!("  task_id: {}", response.id);
                println!("  object: {}", response.object);
                println!("  model: {}", response.model);
                println!("  status: {}", response.status);
                println!("  created: {}", response.created);
                println!("  expires: {:?}", response.expires);
                println!();
                println!("Use this task_id to query status:");
                println!(
                    "  cargo test test_manual_get_task_status -- --ignored --nocapture TASK_ID={}",
                    response.id
                );
            }
            Err(e) => {
                println!("Request failed: {:?}", e);
                panic!("Image-to-video request failed");
            }
        }
    }

    /// Manual test: Query task status
    ///
    /// Run with:
    /// ```bash
    /// export LAOZHANG_API_KEY="your_api_key"
    /// cargo test test_manual_get_task_status -- --ignored --nocapture
    /// ```
    ///
    /// Or specify task ID:
    /// ```bash
    /// TASK_ID=video_abc123 cargo test test_manual_get_task_status -- --ignored --nocapture
    /// ```
    #[tokio::test]
    #[ignore]
    async fn test_manual_get_task_status() {
        let api_key = std::env::var("LAOZHANG_API_KEY")
            .expect("Please set LAOZHANG_API_KEY environment variable");

        // Can specify task ID via environment variable, or use doc example
        let task_id = std::env::var("TASK_ID").unwrap_or_else(|_| "video_abc123".to_string());

        let client = LaoZhangClient::new(api_key, None);

        println!("\nTest: Query Task Status");
        println!("====================================");
        println!("Task ID: {}", task_id);
        println!();

        match client.get_task_status(&task_id).await {
            Ok(status) => {
                println!("Query successful!");
                println!("  task_id: {}", status.id);
                println!("  object: {}", status.object);
                println!("  model: {}", status.model);
                println!("  status: {}", status.status);
                println!("  progress: {:?}", status.progress);
                println!("  url: {:?}", status.url);
                println!("  created_at: {}", status.created_at);
                println!("  completed_at: {:?}", status.completed_at);
                println!();

                if status.is_completed() {
                    println!("Task completed! Can download video");
                    println!("Download video:");
                    println!("  TASK_ID={} cargo test test_manual_download_video -- --ignored --nocapture", task_id);
                } else if status.is_failed() {
                    println!("Task failed: {:?}", status.error);
                } else if status.is_processing() {
                    println!("Task processing... progress: {:?}%", status.progress);
                }
            }
            Err(e) => {
                println!("Query failed: {:?}", e);
                panic!("Query task status failed");
            }
        }
    }

    /// Manual test: Download video
    ///
    /// Run with:
    /// ```bash
    /// export LAOZHANG_API_KEY="your_api_key"
    /// TASK_ID=video_abc123 cargo test test_manual_download_video -- --ignored --nocapture
    /// ```
    #[tokio::test]
    #[ignore]
    async fn test_manual_download_video() {
        let api_key = std::env::var("LAOZHANG_API_KEY")
            .expect("Please set LAOZHANG_API_KEY environment variable");

        let task_id = std::env::var("TASK_ID")
            .expect("Please set TASK_ID environment variable, e.g.: TASK_ID=video_abc123");

        let client = LaoZhangClient::new(api_key, None);

        println!("\nTest: Download Video");
        println!("====================================");
        println!("Task ID: {}", task_id);
        println!();

        match client.download_video(&task_id).await {
            Ok(video_data) => {
                println!("Download successful!");
                println!(
                    "  Video size: {} bytes ({:.2} MB)",
                    video_data.len(),
                    video_data.len() as f64 / 1024.0 / 1024.0
                );

                // Save to file
                let filename = format!("{}.mp4", task_id);
                if let Err(e) = std::fs::write(&filename, &video_data) {
                    println!("Failed to save file: {}", e);
                } else {
                    println!("Saved to: {}", filename);
                }
            }
            Err(e) => {
                println!("Download failed: {:?}", e);
                panic!("Download video failed");
            }
        }
    }
}
