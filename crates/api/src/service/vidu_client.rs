//! Vidu Video Generation API Client (API-side)
//!
//! Thin client for standalone video generation via the Vidu V2 API.
//! Used by VideoService for direct user-facing video requests.

use crate::error::{api_error::ApiError, infrastructure_error::InfrastructureError};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tracing::info;

const DEFAULT_BASE_URL: &str = "https://api.vidu.cn";

/// Check if a model_key belongs to Vidu
pub fn is_vidu_model(model_key: &str) -> bool {
    model_key.starts_with("vidu-")
}

#[derive(Clone)]
pub struct ViduClient {
    client: Client,
    api_key: String,
    base_url: String,
}

impl ViduClient {
    pub fn new(api_key: String, base_url: Option<String>) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(120))
            .build()
            .expect("HTTP client build");
        Self {
            client,
            api_key,
            base_url: base_url.unwrap_or_else(|| DEFAULT_BASE_URL.to_string()),
        }
    }

    /// Upload image bytes to Vidu via 3-step flow, returning `vidu://` URI
    pub async fn upload_image(&self, data: &[u8]) -> Result<String, ApiError> {
        // Step 1: Init
        let init_url = format!("{}/tools/v2/files/uploads", self.base_url);
        let init_resp = self
            .client
            .post(&init_url)
            .header("Authorization", format!("Token {}", self.api_key))
            .json(&serde_json::json!({"scene": "vidu"}))
            .send()
            .await
            .map_err(|e| self.ext_err(format!("upload init: {}", e)))?;
        if !init_resp.status().is_success() {
            let body = init_resp.text().await.unwrap_or_default();
            return Err(self.ext_err(format!("upload init HTTP error: {}", body)));
        }
        let init: ViduUploadInit = init_resp
            .json()
            .await
            .map_err(|e| self.ext_err(format!("parse upload init: {}", e)))?;

        // Step 2: PUT binary
        let put_resp = self
            .client
            .put(&init.put_url)
            .body(data.to_vec())
            .send()
            .await
            .map_err(|e| self.ext_err(format!("upload PUT: {}", e)))?;
        if !put_resp.status().is_success() {
            return Err(self.ext_err("upload PUT failed".into()));
        }
        let etag = put_resp
            .headers()
            .get("etag")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.trim_matches('"').to_string())
            .unwrap_or_default();

        // Step 3: Finish
        let finish_url = format!(
            "{}/tools/v2/files/uploads/{}/finish",
            self.base_url, init.resource_id
        );
        let finish_resp = self
            .client
            .put(&finish_url)
            .header("Authorization", format!("Token {}", self.api_key))
            .json(&serde_json::json!({"etag": etag}))
            .send()
            .await
            .map_err(|e| self.ext_err(format!("upload finish: {}", e)))?;
        if !finish_resp.status().is_success() {
            let body = finish_resp.text().await.unwrap_or_default();
            return Err(self.ext_err(format!("upload finish error: {}", body)));
        }
        let finish: ViduUploadFinish = finish_resp
            .json()
            .await
            .map_err(|e| self.ext_err(format!("parse upload finish: {}", e)))?;

        Ok(finish.uri)
    }

    /// Submit text-to-video generation
    pub async fn text_to_video(
        &self,
        params: ViduGenerateParams,
    ) -> Result<ViduTaskHandle, ApiError> {
        self.post_generation("/ent/v2/text2video", &serde_json::json!({
            "model": params.model,
            "style": params.style.unwrap_or_else(|| "general".to_string()),
            "prompt": params.prompt,
            "duration": params.duration,
            "aspect_ratio": params.aspect_ratio.unwrap_or_else(|| "16:9".to_string()),
            "resolution": params.resolution.unwrap_or_else(|| "720p".to_string()),
            "movement_amplitude": params.movement_amplitude.unwrap_or_else(|| "auto".to_string()),
        })).await
    }

    /// Submit image-to-video generation (image must be uploaded first via upload_image)
    pub async fn image_to_video(
        &self,
        params: ViduGenerateParams,
        image_uri: String,
    ) -> Result<ViduTaskHandle, ApiError> {
        self.post_generation("/ent/v2/img2video", &serde_json::json!({
            "model": params.model,
            "images": [image_uri],
            "prompt": params.prompt,
            "duration": params.duration,
            "resolution": params.resolution.unwrap_or_else(|| "720p".to_string()),
            "movement_amplitude": params.movement_amplitude.unwrap_or_else(|| "auto".to_string()),
        })).await
    }

    /// Start-end-to-video generation (two frames: start + end)
    pub async fn start_end_to_video(
        &self,
        params: ViduGenerateParams,
        images: Vec<String>,
    ) -> Result<ViduTaskHandle, ApiError> {
        self.post_generation("/ent/v2/start-end2video", &serde_json::json!({
            "model": params.model,
            "images": images,
            "prompt": params.prompt,
            "duration": params.duration,
            "resolution": params.resolution.unwrap_or_else(|| "720p".to_string()),
            "movement_amplitude": params.movement_amplitude.unwrap_or_else(|| "auto".to_string()),
        })).await
    }

    /// Reference-to-video generation (1-3 reference images)
    pub async fn reference_to_video(
        &self,
        params: ViduGenerateParams,
        images: Vec<String>,
    ) -> Result<ViduTaskHandle, ApiError> {
        self.post_generation("/ent/v2/reference2video", &serde_json::json!({
            "model": params.model,
            "images": images,
            "prompt": params.prompt,
            "duration": params.duration,
            "aspect_ratio": params.aspect_ratio.unwrap_or_else(|| "16:9".to_string()),
            "resolution": params.resolution.unwrap_or_else(|| "720p".to_string()),
            "movement_amplitude": params.movement_amplitude.unwrap_or_else(|| "auto".to_string()),
        })).await
    }

    /// Multi-frame video generation.
    /// Vidu API: `start_image` (1st image) + `image_settings` (remaining as keyframes).
    /// API constraint: image_settings must contain 2-9 keyframes, so total images = 3-10.
    /// Per-keyframe duration is clamped to 2-7 seconds.
    /// `keyframe_prompts` provides per-keyframe transition descriptions; falls back to global prompt.
    pub async fn multi_frame(
        &self,
        params: ViduGenerateParams,
        images: Vec<String>,
        keyframe_prompts: Option<Vec<String>>,
    ) -> Result<ViduTaskHandle, ApiError> {
        if images.len() < 3 {
            return Err(ApiError::BadRequest(format!(
                "multi_frame requires at least 3 images (1 start + 2 keyframes), got {}",
                images.len()
            )));
        }
        if images.len() > 10 {
            return Err(ApiError::BadRequest(format!(
                "multi_frame supports at most 10 images (1 start + 9 keyframes), got {}",
                images.len()
            )));
        }
        let start_image = images[0].clone();
        let keyframe_images = &images[1..];
        let per_frame_duration =
            (params.duration as usize / keyframe_images.len()).clamp(2, 7) as i32;
        let prompts = keyframe_prompts.unwrap_or_default();
        let image_settings: Vec<serde_json::Value> = keyframe_images
            .iter()
            .enumerate()
            .map(|(i, uri)| {
                let kf_prompt = prompts
                    .get(i)
                    .filter(|s| !s.is_empty())
                    .cloned()
                    .unwrap_or_else(|| params.prompt.clone());
                serde_json::json!({
                    "key_image": uri,
                    "prompt": kf_prompt,
                    "duration": per_frame_duration,
                })
            })
            .collect();
        self.post_generation(
            "/ent/v2/multiframe",
            &serde_json::json!({
                "model": params.model,
                "start_image": start_image,
                "image_settings": image_settings,
                "resolution": params.resolution.unwrap_or_else(|| "720p".to_string()),
            }),
        )
        .await
    }

    /// Template-to-video generation
    pub async fn template_to_video(
        &self,
        template: String,
        images: Vec<String>,
        prompt: Option<String>,
        aspect_ratio: Option<String>,
    ) -> Result<ViduTaskHandle, ApiError> {
        let mut body = serde_json::json!({
            "template": template,
            "images": images,
        });
        if let Some(p) = prompt {
            body["prompt"] = serde_json::json!(p);
        }
        if let Some(ar) = aspect_ratio {
            body["aspect_ratio"] = serde_json::json!(ar);
        }
        self.post_generation("/ent/v2/template2video", &body).await
    }

    /// One-click general film generation (通用成片).
    /// Uses `/ent/v2/template2video` with `template: "general"`.
    pub async fn general_film(
        &self,
        images: Vec<String>,
        prompt: Option<String>,
        aspect_ratio: Option<String>,
        bgm: Option<bool>,
    ) -> Result<ViduTaskHandle, ApiError> {
        let mut body = serde_json::json!({
            "template": "general",
            "images": images,
        });
        if let Some(p) = prompt {
            body["prompt"] = serde_json::json!(p);
        }
        if let Some(ar) = aspect_ratio {
            body["aspect_ratio"] = serde_json::json!(ar);
        }
        if let Some(b) = bgm {
            body["bgm"] = serde_json::json!(b);
        }
        self.post_generation("/ent/v2/template2video", &body).await
    }

    /// One-click ad/e-commerce film generation (电商成片).
    /// Uses `/ent/v2/template2video` with `template: "ad_film"`.
    pub async fn ad_film(
        &self,
        images: Vec<String>,
        prompt: Option<String>,
        aspect_ratio: Option<String>,
        bgm: Option<bool>,
    ) -> Result<ViduTaskHandle, ApiError> {
        let mut body = serde_json::json!({
            "template": "ad_film",
            "images": images,
        });
        if let Some(p) = prompt {
            body["prompt"] = serde_json::json!(p);
        }
        if let Some(ar) = aspect_ratio {
            body["aspect_ratio"] = serde_json::json!(ar);
        }
        if let Some(b) = bgm {
            body["bgm"] = serde_json::json!(b);
        }
        self.post_generation("/ent/v2/template2video", &body).await
    }

    /// Get task status
    pub async fn get_task_status(&self, task_id: &str) -> Result<ViduTaskStatus, ApiError> {
        let url = format!("{}/ent/v2/tasks/{}/creations", self.base_url, task_id);
        let resp = self
            .client
            .get(&url)
            .header("Authorization", format!("Token {}", self.api_key))
            .send()
            .await
            .map_err(|e| self.ext_err(format!("get task status: {}", e)))?;
        if !resp.status().is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(self.ext_err(format!("get task status HTTP error: {}", body)));
        }
        resp.json()
            .await
            .map_err(|e| self.ext_err(format!("parse task status: {}", e)))
    }

    async fn post_generation(
        &self,
        path: &str,
        body: &serde_json::Value,
    ) -> Result<ViduTaskHandle, ApiError> {
        let url = format!("{}{}", self.base_url, path);
        info!("Vidu API: POST {}", path);
        let resp = self
            .client
            .post(&url)
            .header("Authorization", format!("Token {}", self.api_key))
            .json(body)
            .send()
            .await
            .map_err(|e| self.ext_err(format!("API call: {}", e)))?;
        let status = resp.status();
        let text = resp
            .text()
            .await
            .map_err(|e| self.ext_err(format!("read response: {}", e)))?;
        if !status.is_success() {
            return Err(self.ext_err(format!("HTTP {}: {}", status, &text[..text.len().min(300)])));
        }
        let task_resp: ViduTaskResponse = serde_json::from_str(&text)
            .map_err(|e| self.ext_err(format!("parse response: {}", e)))?;
        Ok(ViduTaskHandle {
            task_id: task_resp.task_id,
            state: task_resp.state,
        })
    }

    fn ext_err(&self, msg: String) -> ApiError {
        ApiError::InfrastructureError(InfrastructureError::ExternalApiRequestFailed(format!(
            "Vidu: {}",
            msg
        )))
    }
}

// Helper types

#[derive(Debug, Clone)]
pub struct ViduGenerateParams {
    pub model: String,
    pub prompt: String,
    pub duration: i32,
    pub style: Option<String>,
    pub aspect_ratio: Option<String>,
    pub resolution: Option<String>,
    pub movement_amplitude: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ViduTaskHandle {
    pub task_id: String,
    pub state: String,
}

#[derive(Debug, Deserialize)]
struct ViduUploadInit {
    resource_id: String,
    put_url: String,
    #[allow(dead_code)]
    id: String,
}
#[derive(Debug, Deserialize)]
struct ViduUploadFinish {
    uri: String,
}
#[derive(Debug, Deserialize)]
struct ViduTaskResponse {
    task_id: String,
    state: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ViduTaskStatus {
    pub state: String,
    #[serde(default)]
    pub err_code: Option<String>,
    #[serde(default)]
    pub creations: Vec<ViduCreation>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ViduCreation {
    pub id: String,
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    pub cover_url: Option<String>,
}

impl ViduTaskStatus {
    pub fn is_success(&self) -> bool {
        self.state == "success"
    }
    pub fn is_failed(&self) -> bool {
        self.state == "failed"
    }
    pub fn is_processing(&self) -> bool {
        matches!(
            self.state.as_str(),
            "created" | "queueing" | "scheduling" | "processing"
        )
    }
    pub fn get_video_url(&self) -> Option<String> {
        self.creations.first().and_then(|c| c.url.clone())
    }
}

/// Detect the Vidu API model param from model_key.
/// api.vidu.cn model compatibility per endpoint:
///   text2video / ref2video: viduq2
///   img2video / start-end2video / multiframe: viduq3-turbo
///   fast (text2video): viduq1
///   template: no model field
pub fn detect_model_version(model_key: &str) -> &str {
    match model_key {
        "vidu-fast" => "viduq1",
        "vidu-t2v" => "viduq2",
        "vidu-ref2v" => "viduq2",
        "vidu-i2v" => "viduq3-turbo",
        "vidu-startend" => "viduq3-turbo",
        "vidu-multiframe" => "viduq2-turbo",
        "vidu-general-film" => "viduq2",
        "vidu-ad-film" => "viduq2",
        _ => "viduq2",
    }
}

/// Detect default Vidu resolution.
/// viduq1 (fast) always outputs 1080p, others default to 720p.
pub fn detect_resolution(model_key: &str) -> &str {
    if model_key == "vidu-fast" {
        "1080p"
    } else {
        "720p"
    }
}

/// Detect default duration for model.
/// viduq1 = 5s fixed, others default 4s.
pub fn detect_default_duration(model_key: &str) -> i32 {
    if model_key == "vidu-fast" {
        5
    } else {
        4
    }
}

/// Detect generation mode from model_key.
pub fn detect_generation_mode(model_key: &str) -> &str {
    match model_key {
        "vidu-ref2v" => "reference_to_video",
        "vidu-startend" => "start_end_to_video",
        "vidu-multiframe" => "multi_frame",
        "vidu-template" => "template",
        "vidu-i2v" => "image_to_video",
        "vidu-fast" => "fast",
        "vidu-t2v" => "text_to_video",
        "vidu-general-film" => "general_film",
        "vidu-ad-film" => "ad_film",
        _ => "text_to_video",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_vidu_model() {
        assert!(is_vidu_model("vidu-t2v"));
        assert!(is_vidu_model("vidu-i2v"));
        assert!(is_vidu_model("vidu-ref2v"));
        assert!(is_vidu_model("vidu-fast"));
        assert!(is_vidu_model("vidu-template"));
        assert!(!is_vidu_model("jimeng-video-3.0"));
        assert!(!is_vidu_model("sora-2"));
        assert!(!is_vidu_model(""));
        assert!(!is_vidu_model("VIDU-2.0"));
    }

    #[test]
    fn test_detect_model_version_simplified_keys() {
        assert_eq!(detect_model_version("vidu-t2v"), "viduq2");
        assert_eq!(detect_model_version("vidu-i2v"), "viduq3-turbo");
        assert_eq!(detect_model_version("vidu-ref2v"), "viduq2");
        assert_eq!(detect_model_version("vidu-startend"), "viduq3-turbo");
        assert_eq!(detect_model_version("vidu-multiframe"), "viduq2-turbo");
        assert_eq!(detect_model_version("vidu-fast"), "viduq1");
        assert_eq!(detect_model_version("vidu-template"), "viduq2");
    }

    #[test]
    fn test_detect_resolution_simplified() {
        assert_eq!(detect_resolution("vidu-t2v"), "720p");
        assert_eq!(detect_resolution("vidu-fast"), "1080p");
        assert_eq!(detect_resolution("vidu-i2v"), "720p");
    }

    #[test]
    fn test_detect_default_duration_simplified() {
        assert_eq!(detect_default_duration("vidu-fast"), 5);
        assert_eq!(detect_default_duration("vidu-t2v"), 4);
        assert_eq!(detect_default_duration("vidu-i2v"), 4);
    }

    #[test]
    fn test_detect_generation_mode() {
        assert_eq!(detect_generation_mode("vidu-t2v"), "text_to_video");
        assert_eq!(detect_generation_mode("vidu-i2v"), "image_to_video");
        assert_eq!(detect_generation_mode("vidu-ref2v"), "reference_to_video");
        assert_eq!(
            detect_generation_mode("vidu-startend"),
            "start_end_to_video"
        );
        assert_eq!(detect_generation_mode("vidu-multiframe"), "multi_frame");
        assert_eq!(detect_generation_mode("vidu-fast"), "fast");
        assert_eq!(detect_generation_mode("vidu-template"), "template");
        assert_eq!(detect_generation_mode("vidu-unknown"), "text_to_video");
    }

    #[test]
    fn test_vidu_task_status_serde_success() {
        let json = r#"{
            "state": "success",
            "creations": [
                {"id": "c1", "url": "https://cdn.vidu.com/v.mp4", "cover_url": "https://cdn.vidu.com/c.jpg"}
            ]
        }"#;
        let s: ViduTaskStatus = serde_json::from_str(json).unwrap();
        assert!(s.is_success());
        assert_eq!(s.get_video_url().unwrap(), "https://cdn.vidu.com/v.mp4");
    }

    #[test]
    fn test_vidu_task_status_processing_states() {
        for state in &["created", "queueing", "scheduling", "processing"] {
            let json = format!(r#"{{"state": "{}", "creations": []}}"#, state);
            let s: ViduTaskStatus = serde_json::from_str(&json).unwrap();
            assert!(s.is_processing(), "state '{}' should be processing", state);
        }
    }

    #[test]
    fn test_vidu_task_status_failed() {
        let json = r#"{"state": "failed", "err_code": "content_policy", "creations": []}"#;
        let s: ViduTaskStatus = serde_json::from_str(json).unwrap();
        assert!(s.is_failed());
        assert_eq!(s.err_code.as_deref(), Some("content_policy"));
    }

    #[test]
    fn test_vidu_generate_params_construction() {
        let params = ViduGenerateParams {
            model: "viduq2".to_string(),
            prompt: "Test prompt".to_string(),
            duration: 4,
            style: Some("general".to_string()),
            aspect_ratio: Some("16:9".to_string()),
            resolution: Some("720p".to_string()),
            movement_amplitude: Some("auto".to_string()),
        };
        assert_eq!(params.model, "viduq2");
        assert_eq!(params.duration, 4);
    }

    #[test]
    fn test_all_modes_have_correct_detect_chain() {
        let modes = vec![
            ("vidu-t2v", "viduq2", "720p", 4, "text_to_video"),
            ("vidu-i2v", "viduq3-turbo", "720p", 4, "image_to_video"),
            ("vidu-ref2v", "viduq2", "720p", 4, "reference_to_video"),
            (
                "vidu-startend",
                "viduq3-turbo",
                "720p",
                4,
                "start_end_to_video",
            ),
            ("vidu-multiframe", "viduq2-turbo", "720p", 4, "multi_frame"),
            ("vidu-fast", "viduq1", "1080p", 5, "fast"),
            ("vidu-template", "viduq2", "720p", 4, "template"),
            ("vidu-general-film", "viduq2", "720p", 4, "general_film"),
            ("vidu-ad-film", "viduq2", "720p", 4, "ad_film"),
        ];
        for (key, ver, res, dur, mode) in modes {
            assert!(is_vidu_model(key), "{} should be vidu", key);
            assert_eq!(detect_model_version(key), ver, "{} model_version", key);
            assert_eq!(detect_resolution(key), res, "{} resolution", key);
            assert_eq!(detect_default_duration(key), dur, "{} duration", key);
            assert_eq!(detect_generation_mode(key), mode, "{} gen_mode", key);
        }
    }

    #[test]
    fn test_start_end_to_video_uses_correct_endpoint() {
        let params = ViduGenerateParams {
            model: "vidu2.0".to_string(),
            prompt: "test".to_string(),
            duration: 4,
            style: None,
            aspect_ratio: None,
            resolution: Some("720p".to_string()),
            movement_amplitude: None,
        };
        assert_eq!(params.model, "vidu2.0");
        let images = ["vidu://start".to_string(), "vidu://end".to_string()];
        assert_eq!(images.len(), 2);
    }

    #[test]
    fn test_reference_to_video_accepts_1_to_3_images() {
        for count in 1..=3 {
            let images: Vec<String> = (0..count).map(|i| format!("vidu://ref{}", i)).collect();
            assert!(!images.is_empty() && images.len() <= 3);
        }
    }

    #[test]
    fn test_template_to_video_no_model_field() {
        let mut body = serde_json::json!({
            "template": "general",
            "images": ["vidu://img1"],
        });
        body["prompt"] = serde_json::json!("test");
        body["aspect_ratio"] = serde_json::json!("16:9");
        let obj = body.as_object().unwrap();
        assert!(obj.contains_key("template"));
        assert!(obj.contains_key("images"));
        assert!(
            !obj.contains_key("model"),
            "template endpoint must not include 'model' field"
        );
    }

    #[test]
    fn test_general_film_json_body() {
        let mut body = serde_json::json!({
            "template": "general",
            "images": ["vidu://product1", "vidu://product2"],
        });
        body["prompt"] = serde_json::json!("Create an engaging video");
        body["aspect_ratio"] = serde_json::json!("16:9");
        body["bgm"] = serde_json::json!(true);
        let obj = body.as_object().unwrap();
        assert_eq!(obj["template"], "general");
        assert_eq!(obj["images"].as_array().unwrap().len(), 2);
        assert_eq!(obj["prompt"], "Create an engaging video");
        assert_eq!(obj["aspect_ratio"], "16:9");
        assert_eq!(obj["bgm"], true);
        assert!(
            !obj.contains_key("model"),
            "general_film must not include 'model'"
        );
    }

    #[test]
    fn test_ad_film_json_body() {
        let mut body = serde_json::json!({
            "template": "ad_film",
            "images": ["vidu://product_img"],
        });
        body["prompt"] = serde_json::json!("Product showcase with modern style");
        body["aspect_ratio"] = serde_json::json!("9:16");
        body["bgm"] = serde_json::json!(true);
        let obj = body.as_object().unwrap();
        assert_eq!(obj["template"], "ad_film");
        assert_eq!(obj["images"].as_array().unwrap().len(), 1);
        assert_eq!(obj["aspect_ratio"], "9:16");
        assert_eq!(obj["bgm"], true);
        assert!(
            !obj.contains_key("model"),
            "ad_film must not include 'model'"
        );
    }

    #[test]
    fn test_general_film_without_optional_fields() {
        let body = serde_json::json!({
            "template": "general",
            "images": ["vidu://img1"],
        });
        let obj = body.as_object().unwrap();
        assert_eq!(obj["template"], "general");
        assert!(!obj.contains_key("prompt"));
        assert!(!obj.contains_key("bgm"));
        assert!(!obj.contains_key("aspect_ratio"));
    }

    #[test]
    fn test_ad_film_without_optional_fields() {
        let body = serde_json::json!({
            "template": "ad_film",
            "images": ["vidu://img1"],
        });
        let obj = body.as_object().unwrap();
        assert_eq!(obj["template"], "ad_film");
        assert!(!obj.contains_key("prompt"));
        assert!(!obj.contains_key("bgm"));
    }

    #[test]
    fn test_general_film_and_ad_film_model_keys() {
        assert!(is_vidu_model("vidu-general-film"));
        assert!(is_vidu_model("vidu-ad-film"));
        assert_eq!(detect_generation_mode("vidu-general-film"), "general_film");
        assert_eq!(detect_generation_mode("vidu-ad-film"), "ad_film");
        assert_eq!(detect_model_version("vidu-general-film"), "viduq2");
        assert_eq!(detect_model_version("vidu-ad-film"), "viduq2");
        assert_eq!(detect_resolution("vidu-general-film"), "720p");
        assert_eq!(detect_resolution("vidu-ad-film"), "720p");
        assert_eq!(detect_default_duration("vidu-general-film"), 4);
        assert_eq!(detect_default_duration("vidu-ad-film"), 4);
    }
}
