use crate::dto::laozhang_dto::{
    CreateImageRequest, CreateVideoFromImageRequest, CreateVideoFromTextRequest, ImageResponse,
    VideoTaskDetailResponse, VideoTaskResponse,
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

    // ========== Image Generation Functions ==========

    /// Generate image from text prompt
    ///
    /// Supports multiple models including:
    /// - gpt-4o-image: Best value, high quality ($0.01/image)
    /// - dall-e-3: OpenAI official, rich details ($0.04/image)
    /// - dall-e-2: Classic model, stable ($0.02/image)
    /// - black-forest-labs/flux-pro-v1.1: Professional quality ($0.035/image)
    /// - claude-3.5-sonnet-img: Claude image generation
    ///
    /// # Arguments
    /// * `request` - Image generation request parameters
    ///
    /// # Returns
    /// Returns image generation response containing image URLs
    ///
    /// # Example
    /// ```rust,no_run
    /// # use glance_mind_api::service::laozhang_client::LaoZhangClient;
    /// # use glance_mind_api::dto::laozhang_dto::CreateImageRequest;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = LaoZhangClient::new("your_api_key".to_string(), None);
    /// let request = CreateImageRequest {
    ///     model: "gpt-4o-image".to_string(),
    ///     prompt: "A serene Japanese garden with cherry blossoms".to_string(),
    ///     ..Default::default()
    /// };
    /// let response = client.create_image(request).await?;
    /// if let Some(url) = response.get_first_url() {
    ///     println!("Image URL: {}", url);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn create_image(
        &self,
        request: CreateImageRequest,
    ) -> Result<ImageResponse, ApiError> {
        let url = format!("{}/v1/images/generations", self.base_url);

        tracing::info!(
            "Starting image generation: model={}, prompt='{}', n={:?}, size={:?}, quality={:?}, style={:?}",
            request.model,
            request.prompt,
            request.n,
            request.size,
            request.quality,
            request.style
        );

        tracing::debug!(
            "LaoZhang image generation request body: {}",
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
                tracing::error!("LaoZhang image generation request failed: {:?}", e);
                ApiError::InfrastructureError(InfrastructureError::ExternalApiRequestFailed(
                    e.to_string(),
                ))
            })?;

        let status = response.status();
        let body_text = response.text().await.map_err(|e| {
            tracing::error!("Failed to read LaoZhang image response: {:?}", e);
            ApiError::InfrastructureError(InfrastructureError::ExternalApiResponseParsingFailed(
                e.to_string(),
            ))
        })?;

        tracing::info!(
            "LaoZhang image API response: status={}, body_length={} bytes",
            status,
            body_text.len()
        );
        tracing::debug!("LaoZhang image response content: {}", body_text);

        if !status.is_success() {
            tracing::error!(
                "LaoZhang image generation failed: status={}, body={}",
                status,
                body_text
            );

            // Try to parse structured error response
            if let Ok(error_response) = serde_json::from_str::<LaoZhangErrorResponse>(&body_text) {
                return Err(ApiError::InfrastructureError(
                    InfrastructureError::ExternalApiRequestFailed(format!(
                        "LaoZhang image generation error: {}",
                        error_response.error.message
                    )),
                ));
            }

            return Err(ApiError::InfrastructureError(
                InfrastructureError::ExternalApiRequestFailed(format!(
                    "LaoZhang API error: {} - {}",
                    status, body_text
                )),
            ));
        }

        let image_response: ImageResponse = serde_json::from_str(&body_text).map_err(|e| {
            tracing::error!(
                "Failed to parse LaoZhang image response: {:?}, body: {}",
                e,
                body_text
            );
            ApiError::InfrastructureError(InfrastructureError::ExternalApiResponseParsingFailed(
                e.to_string(),
            ))
        })?;

        tracing::info!(
            "LaoZhang image generation successful: model={}, images_count={}, prompt='{}'",
            request.model,
            image_response.data.len(),
            request.prompt
        );

        Ok(image_response)
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
    use crate::dto::laozhang_dto::ImageData;

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

    // ========== Image Generation Tests ==========

    #[test]
    fn test_create_image_request_default() {
        let request = CreateImageRequest::default();
        assert_eq!(request.model, "gpt-4o-image");
        assert_eq!(request.prompt, "");
        assert_eq!(request.n, Some(1));
        assert_eq!(request.size, Some("1024x1024".to_string()));
        assert!(request.quality.is_none());
        assert!(request.style.is_none());
    }

    #[test]
    fn test_create_image_request_gpt4o_image() {
        let request = CreateImageRequest {
            model: "gpt-4o-image".to_string(),
            prompt: "A serene Japanese garden with cherry blossoms".to_string(),
            n: Some(1),
            size: Some("1024x1024".to_string()),
            ..Default::default()
        };

        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("gpt-4o-image"));
        assert!(json.contains("Japanese garden"));
        // quality and style should be omitted from JSON (skip_serializing_if)
        assert!(!json.contains("quality"));
        assert!(!json.contains("style"));
    }

    #[test]
    fn test_create_image_request_dalle3_with_quality_and_style() {
        let request = CreateImageRequest {
            model: "dall-e-3".to_string(),
            prompt: "A detailed oil painting of a robot playing chess".to_string(),
            n: Some(1),
            size: Some("1024x1024".to_string()),
            quality: Some("hd".to_string()),
            style: Some("vivid".to_string()),
            ..Default::default()
        };

        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("dall-e-3"));
        assert!(json.contains("\"quality\":\"hd\""));
        assert!(json.contains("\"style\":\"vivid\""));
    }

    #[test]
    fn test_create_image_request_with_response_format() {
        let request = CreateImageRequest {
            model: "gpt-4o-image".to_string(),
            prompt: "test".to_string(),
            response_format: Some("url".to_string()),
            ..Default::default()
        };

        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("\"response_format\":\"url\""));
    }

    #[test]
    fn test_create_image_request_serialization_all_fields() {
        let request = CreateImageRequest {
            model: "dall-e-3".to_string(),
            prompt: "test prompt".to_string(),
            n: Some(2),
            size: Some("1024x1792".to_string()),
            quality: Some("hd".to_string()),
            style: Some("natural".to_string()),
            response_format: Some("url".to_string()),
        };

        let json_value: serde_json::Value = serde_json::to_value(&request).unwrap();
        assert_eq!(json_value["model"], "dall-e-3");
        assert_eq!(json_value["prompt"], "test prompt");
        assert_eq!(json_value["n"], 2);
        assert_eq!(json_value["size"], "1024x1792");
        assert_eq!(json_value["quality"], "hd");
        assert_eq!(json_value["style"], "natural");
        assert_eq!(json_value["response_format"], "url");
    }

    #[test]
    fn test_create_image_request_serialization_optional_fields_omitted() {
        let request = CreateImageRequest {
            model: "gpt-4o-image".to_string(),
            prompt: "test".to_string(),
            n: None,
            size: None,
            quality: None,
            style: None,
            response_format: None,
        };

        let json = serde_json::to_string(&request).unwrap();
        // Only model and prompt should be present
        assert!(json.contains("model"));
        assert!(json.contains("prompt"));
        assert!(!json.contains("\"n\""));
        assert!(!json.contains("\"size\""));
        assert!(!json.contains("\"quality\""));
        assert!(!json.contains("\"style\""));
        assert!(!json.contains("\"response_format\""));
    }

    #[test]
    fn test_image_response_deserialization_with_url() {
        let json = r#"{
            "created": 1706000000,
            "data": [
                {
                    "url": "https://example.com/image1.png",
                    "revised_prompt": "A beautiful garden..."
                }
            ]
        }"#;

        let response: ImageResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.created, 1706000000);
        assert_eq!(response.data.len(), 1);
        assert_eq!(
            response.data[0].url.as_deref(),
            Some("https://example.com/image1.png")
        );
        assert_eq!(
            response.data[0].revised_prompt.as_deref(),
            Some("A beautiful garden...")
        );
        assert!(response.data[0].b64_json.is_none());
        assert!(response.data[0].has_image());
    }

    #[test]
    fn test_image_response_deserialization_with_b64_json() {
        let json = r#"{
            "created": 1706000000,
            "data": [
                {
                    "b64_json": "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNk+M9QDwADhgGAWjR9awAAAABJRU5ErkJggg==",
                    "revised_prompt": "A tiny image"
                }
            ]
        }"#;

        let response: ImageResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.created, 1706000000);
        assert_eq!(response.data.len(), 1);
        assert!(response.data[0].url.is_none());
        assert!(response.data[0].b64_json.is_some());
        assert!(response.data[0].has_image());
        assert!(response.has_images());
        assert_eq!(response.image_count(), 1);
        // get_first_url returns None since there's no URL
        assert!(response.get_first_url().is_none());
        // get_first_b64 returns the base64 data
        assert!(response.get_first_b64().is_some());
    }

    #[test]
    fn test_image_response_get_first_url() {
        let response = ImageResponse {
            created: 1706000000,
            data: vec![
                ImageData {
                    url: Some("https://example.com/image1.png".to_string()),
                    b64_json: None,
                    revised_prompt: None,
                },
                ImageData {
                    url: Some("https://example.com/image2.png".to_string()),
                    b64_json: None,
                    revised_prompt: None,
                },
            ],
        };

        assert_eq!(
            response.get_first_url(),
            Some("https://example.com/image1.png")
        );
    }

    #[test]
    fn test_image_response_get_all_urls() {
        let response = ImageResponse {
            created: 1706000000,
            data: vec![
                ImageData {
                    url: Some("https://example.com/image1.png".to_string()),
                    b64_json: None,
                    revised_prompt: None,
                },
                ImageData {
                    url: Some("https://example.com/image2.png".to_string()),
                    b64_json: None,
                    revised_prompt: None,
                },
            ],
        };

        let urls = response.get_all_urls();
        assert_eq!(urls.len(), 2);
        assert_eq!(urls[0], "https://example.com/image1.png");
        assert_eq!(urls[1], "https://example.com/image2.png");
    }

    #[test]
    fn test_image_response_mixed_url_and_b64() {
        let response = ImageResponse {
            created: 1706000000,
            data: vec![
                ImageData {
                    url: Some("https://example.com/image1.png".to_string()),
                    b64_json: None,
                    revised_prompt: None,
                },
                ImageData {
                    url: None,
                    b64_json: Some("base64data".to_string()),
                    revised_prompt: None,
                },
            ],
        };

        // get_all_urls only returns URL-based images
        assert_eq!(response.get_all_urls().len(), 1);
        // But image_count counts all images
        assert_eq!(response.image_count(), 2);
        assert!(response.has_images());
    }

    #[test]
    fn test_image_response_empty_data() {
        let response = ImageResponse {
            created: 1706000000,
            data: vec![],
        };

        assert!(response.get_first_url().is_none());
        assert!(response.get_all_urls().is_empty());
        assert!(response.get_first_b64().is_none());
        assert!(!response.has_images());
        assert_eq!(response.image_count(), 0);
    }

    #[test]
    fn test_image_response_without_revised_prompt() {
        let json = r#"{
            "created": 1706000000,
            "data": [
                {
                    "url": "https://example.com/image1.png"
                }
            ]
        }"#;

        let response: ImageResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.data.len(), 1);
        assert!(response.data[0].revised_prompt.is_none());
    }

    #[test]
    fn test_image_response_multiple_images() {
        let json = r#"{
            "created": 1706000000,
            "data": [
                {"url": "https://example.com/img1.png"},
                {"url": "https://example.com/img2.png"},
                {"url": "https://example.com/img3.png"}
            ]
        }"#;

        let response: ImageResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.data.len(), 3);
        assert_eq!(response.get_all_urls().len(), 3);
    }

    #[test]
    fn test_image_data_empty() {
        let json = r#"{
            "created": 1706000000,
            "data": [{}]
        }"#;

        let response: ImageResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.data.len(), 1);
        assert!(!response.data[0].has_image());
        assert!(!response.has_images());
    }

    /// Verify all supported image models can be serialized correctly
    #[test]
    fn test_all_image_models_serialization() {
        let models = vec![
            ("gpt-4o-image", "1024x1024", None, None),
            ("sora-image", "1024x1024", None, None),
            ("dall-e-3", "1024x1024", Some("hd"), Some("vivid")),
        ];

        for (model, size, quality, style) in models {
            let request = CreateImageRequest {
                model: model.to_string(),
                prompt: "Test prompt for image generation".to_string(),
                n: Some(1),
                size: Some(size.to_string()),
                quality: quality.map(|q| q.to_string()),
                style: style.map(|s| s.to_string()),
                ..Default::default()
            };

            // Verify serialization works
            let json_str = serde_json::to_string(&request).unwrap();
            assert!(
                json_str.contains(model),
                "JSON should contain model name: {}",
                model
            );

            // Verify deserialization round-trip
            let deserialized: CreateImageRequest = serde_json::from_str(&json_str).unwrap();
            assert_eq!(deserialized.model, model);
            assert_eq!(deserialized.prompt, "Test prompt for image generation");
            assert_eq!(deserialized.n, Some(1));
            assert_eq!(deserialized.size, Some(size.to_string()));
            assert_eq!(deserialized.quality, quality.map(|q| q.to_string()));
            assert_eq!(deserialized.style, style.map(|s| s.to_string()));
        }
    }

    // ========== Image Generation Manual Tests (Requires API Key) ==========

    /// Manual test: Generate image with all supported models
    ///
    /// Run with:
    /// ```bash
    /// export LAOZHANG_API_KEY="your_api_key"
    /// cargo test test_manual_create_image_all_models -- --ignored --nocapture
    /// ```
    #[tokio::test]
    #[ignore]
    async fn test_manual_create_image_all_models() {
        let api_key = std::env::var("LAOZHANG_API_KEY")
            .expect("Please set LAOZHANG_API_KEY environment variable");

        let client = LaoZhangClient::new(api_key, None);

        let test_prompt = "A serene Japanese garden with cherry blossoms in spring";

        // (model, size, quality, style, response_format)
        let models: Vec<(&str, &str, Option<&str>, Option<&str>, Option<&str>)> = vec![
            ("gpt-4o-image", "1024x1024", None, None, Some("url")),
            ("sora-image", "1024x1024", None, None, Some("url")),
            ("dall-e-3", "1024x1024", Some("hd"), Some("vivid"), None),
        ];

        println!("\nTest: Image Generation - All Models");
        println!("====================================");
        println!("Prompt: {}", test_prompt);
        println!();

        let mut success_count = 0;
        let total = models.len();

        for (model, size, quality, style, response_format) in &models {
            println!("--- Testing model: {} ---", model);

            let request = CreateImageRequest {
                model: model.to_string(),
                prompt: test_prompt.to_string(),
                n: Some(1),
                size: Some(size.to_string()),
                quality: quality.map(|q| q.to_string()),
                style: style.map(|s| s.to_string()),
                response_format: response_format.map(|f| f.to_string()),
            };

            match client.create_image(request).await {
                Ok(response) => {
                    success_count += 1;
                    println!(
                        "  OK: {} image(s) generated (has_images={})",
                        response.data.len(),
                        response.has_images()
                    );
                    if let Some(url) = response.get_first_url() {
                        println!("  URL: {}...", &url[..url.len().min(80)]);
                    }
                    if let Some(b64) = response.get_first_b64() {
                        println!("  B64: ({}... {} bytes)", &b64[..b64.len().min(30)], b64.len());
                    }
                    if let Some(revised) = response
                        .data
                        .first()
                        .and_then(|d| d.revised_prompt.as_deref())
                    {
                        println!(
                            "  Revised prompt: {}...",
                            &revised[..revised.len().min(60)]
                        );
                    }
                }
                Err(e) => {
                    println!("  FAIL: {:?}", e);
                }
            }
            println!();
        }

        println!("====================================");
        println!(
            "Results: {}/{} models succeeded ({:.0}%)",
            success_count,
            total,
            success_count as f64 / total as f64 * 100.0
        );

        assert_eq!(
            success_count, total,
            "All active image models should generate successfully"
        );
    }

    /// Manual test: Generate image with gpt-4o-image model
    ///
    /// Run with:
    /// ```bash
    /// export LAOZHANG_API_KEY="your_api_key"
    /// cargo test test_manual_create_image_gpt4o -- --ignored --nocapture
    /// ```
    #[tokio::test]
    #[ignore]
    async fn test_manual_create_image_gpt4o() {
        let api_key = std::env::var("LAOZHANG_API_KEY")
            .expect("Please set LAOZHANG_API_KEY environment variable");

        let client = LaoZhangClient::new(api_key, None);

        let request = CreateImageRequest {
            model: "gpt-4o-image".to_string(),
            prompt: "A cute cat wearing a tiny top hat, digital art style".to_string(),
            n: Some(1),
            size: Some("1024x1024".to_string()),
            response_format: Some("url".to_string()),
            ..Default::default()
        };

        println!("\nTest: Image Generation - GPT-4o Image");
        println!("====================================");
        println!("Model: {}", request.model);
        println!("Prompt: {}", request.prompt);
        println!("Size: {:?}", request.size);
        println!("Response format: {:?}", request.response_format);
        println!();

        match client.create_image(request).await {
            Ok(response) => {
                println!("Request successful!");
                println!("  created: {}", response.created);
                println!("  images: {}", response.data.len());
                for (i, img) in response.data.iter().enumerate() {
                    if let Some(ref url) = img.url {
                        println!("  image[{}] url: {}", i, url);
                    }
                    if let Some(ref b64) = img.b64_json {
                        println!("  image[{}] b64_json: {} bytes", i, b64.len());
                    }
                    if let Some(ref revised) = img.revised_prompt {
                        println!("  image[{}] revised_prompt: {}", i, revised);
                    }
                }
                assert!(
                    response.has_images(),
                    "Response should contain at least one image"
                );
            }
            Err(e) => {
                println!("Request failed: {:?}", e);
                panic!("Image generation failed");
            }
        }
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
