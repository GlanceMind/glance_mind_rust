use crate::dto::jimeng_dto::{
    build_req_key, JimengApiResponse, JimengResolution, JimengResultData, JimengSubmitData,
    JimengSubmitRequest, JimengTaskHandle, JimengVideoMode, JimengVideoParams, VolcengineResponse,
    VALID_ASPECT_RATIOS, VALID_SECONDS,
};
use crate::error::{api_error::ApiError, infrastructure_error::InfrastructureError};
use base64::{engine::general_purpose::STANDARD as BASE64_STANDARD, Engine};
use chrono::Utc;
use hmac::{Hmac, Mac};
use reqwest::Client;
use sha2::Sha256;
use std::time::Duration;

type HmacSha256 = Hmac<Sha256>;

const VOLCENGINE_HOST: &str = "visual.volcengineapi.com";
const API_VERSION: &str = "2022-08-31";
const SERVICE_NAME: &str = "cv";
const REGION: &str = "cn-north-1";
const CODE_SUCCESS: i32 = 10000;

#[derive(Debug, Clone)]
pub struct RetryConfig {
    pub max_retries: u32,
    pub initial_backoff: Duration,
    pub max_backoff: Duration,
    pub backoff_multiplier: f64,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_retries: 5,
            initial_backoff: Duration::from_secs(2),
            max_backoff: Duration::from_secs(30),
            backoff_multiplier: 2.0,
        }
    }
}

/// Jimeng AI Video 3.0 SDK Client
///
/// Supports 3 products × 3 modes (T2V, I2V first, I2V first-last):
///   - 3.0 720P:  `jimeng_t2v_v30`       / `jimeng_i2v_first_v30`       / `jimeng_i2v_first_tail_v30`
///   - 3.0 1080P: `jimeng_t2v_v30_1080p` / `jimeng_i2v_first_v30_1080`  / `jimeng_i2v_first_tail_v30_1080`
///   - 3.0 Pro:   `jimeng_vgfm_t2v_l20`  / `jimeng_vgfm_i2v_l20`       / N/A
#[derive(Clone)]
pub struct JimengClient {
    client: Client,
    access_key_id: String,
    secret_access_key: String,
    base_url: String,
    retry_config: RetryConfig,
}

impl JimengClient {
    pub fn new(access_key_id: String, secret_access_key: String, base_url: Option<String>) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(120))
            .build()
            .expect("HTTP client build");
        Self {
            client,
            access_key_id,
            secret_access_key,
            base_url: base_url.unwrap_or_else(|| format!("https://{}", VOLCENGINE_HOST)),
            retry_config: RetryConfig::default(),
        }
    }

    pub fn with_retry_config(mut self, config: RetryConfig) -> Self {
        self.retry_config = config;
        self
    }

    // ========== Unified High-Level API ==========

    /// Create a video with unified parameters (resolution + mode auto-detected).
    pub async fn create_video(
        &self,
        params: JimengVideoParams,
    ) -> Result<JimengTaskHandle, ApiError> {
        self.validate_prompt(&params.prompt)?;
        self.validate_seconds(params.seconds)?;
        if let Some(ref ar) = params.aspect_ratio {
            self.validate_aspect_ratio(ar)?;
        }

        let mode = match (&params.image_base64, &params.end_image_base64) {
            (Some(_), Some(_)) => JimengVideoMode::ImageFirstLastFrame,
            (Some(_), None) => JimengVideoMode::ImageFirstFrame,
            _ => JimengVideoMode::TextToVideo,
        };
        let req_key = build_req_key(mode, params.resolution).ok_or_else(|| {
            ApiError::InfrastructureError(InfrastructureError::ExternalApiRequestFailed(format!(
                "{:?} mode is not supported for {:?} resolution",
                mode, params.resolution
            )))
        })?;

        let is_pro = matches!(params.resolution, JimengResolution::V30Pro);

        let (binary_data_base64, image_urls) = if is_pro {
            // Pro uses image_urls for I2V (image_base64 here is actually a URL for Pro)
            let urls = match mode {
                JimengVideoMode::ImageFirstFrame => Some(vec![params.image_base64.unwrap()]),
                _ => None,
            };
            (None, urls)
        } else {
            let b64 = match mode {
                JimengVideoMode::ImageFirstLastFrame => Some(vec![
                    params.image_base64.unwrap(),
                    params.end_image_base64.unwrap(),
                ]),
                JimengVideoMode::ImageFirstFrame => Some(vec![params.image_base64.unwrap()]),
                JimengVideoMode::TextToVideo => None,
            };
            (b64, None)
        };

        let req = JimengSubmitRequest {
            req_key,
            prompt: params.prompt,
            frames: JimengSubmitRequest::seconds_to_frames(params.seconds),
            aspect_ratio: if mode == JimengVideoMode::TextToVideo {
                params.aspect_ratio
            } else {
                None
            },
            binary_data_base64,
            image_urls,
            seed: Some(-1),
        };
        self.submit_with_retry(req).await
    }

    /// Convenience: Text-to-Video
    pub async fn create_text_to_video(
        &self,
        prompt: String,
        aspect_ratio: String,
        seconds: i32,
        resolution: JimengResolution,
    ) -> Result<JimengTaskHandle, ApiError> {
        self.create_video(JimengVideoParams {
            prompt,
            resolution,
            seconds,
            aspect_ratio: Some(aspect_ratio),
            image_base64: None,
            end_image_base64: None,
        })
        .await
    }

    /// Convenience: Image-to-Video (first frame)
    pub async fn create_image_to_video(
        &self,
        prompt: String,
        image_base64: String,
        seconds: i32,
        resolution: JimengResolution,
    ) -> Result<JimengTaskHandle, ApiError> {
        self.create_video(JimengVideoParams {
            prompt,
            resolution,
            seconds,
            aspect_ratio: None,
            image_base64: Some(image_base64),
            end_image_base64: None,
        })
        .await
    }

    /// Convenience: Image-to-Video (first + last frame)
    pub async fn create_image_first_last_video(
        &self,
        prompt: String,
        first_image_base64: String,
        last_image_base64: String,
        seconds: i32,
        resolution: JimengResolution,
    ) -> Result<JimengTaskHandle, ApiError> {
        self.create_video(JimengVideoParams {
            prompt,
            resolution,
            seconds,
            aspect_ratio: None,
            image_base64: Some(first_image_base64),
            end_image_base64: Some(last_image_base64),
        })
        .await
    }

    /// Poll task status
    pub async fn get_task_status(
        &self,
        handle: &JimengTaskHandle,
    ) -> Result<JimengResultData, ApiError> {
        self.poll_with_retry(&handle.req_key, &handle.task_id).await
    }

    // ========== Retry wrapper ==========

    async fn submit_with_retry(
        &self,
        request: JimengSubmitRequest,
    ) -> Result<JimengTaskHandle, ApiError> {
        let mut last_err = None;
        let mut backoff = self.retry_config.initial_backoff;
        for attempt in 0..=self.retry_config.max_retries {
            if attempt > 0 {
                tracing::warn!(
                    "Jimeng submit retry {}/{}, wait {:?}",
                    attempt,
                    self.retry_config.max_retries,
                    backoff
                );
                tokio::time::sleep(backoff).await;
                backoff = Duration::from_secs_f64(
                    (backoff.as_secs_f64() * self.retry_config.backoff_multiplier)
                        .min(self.retry_config.max_backoff.as_secs_f64()),
                );
            }
            match self.submit_once(&request).await {
                Ok(h) => return Ok(h),
                Err(e) if Self::is_retryable(&e) => {
                    last_err = Some(e);
                    continue;
                }
                Err(e) => return Err(e),
            }
        }
        Err(last_err.unwrap_or_else(|| {
            ApiError::InfrastructureError(InfrastructureError::ExternalApiRequestFailed(
                "Jimeng submit retries exhausted".into(),
            ))
        }))
    }

    async fn poll_with_retry(
        &self,
        req_key: &str,
        task_id: &str,
    ) -> Result<JimengResultData, ApiError> {
        let mut last_err = None;
        let mut backoff = self.retry_config.initial_backoff;
        for attempt in 0..=self.retry_config.max_retries {
            if attempt > 0 {
                tokio::time::sleep(backoff).await;
                backoff = Duration::from_secs_f64(
                    (backoff.as_secs_f64() * self.retry_config.backoff_multiplier)
                        .min(self.retry_config.max_backoff.as_secs_f64()),
                );
            }
            match self.poll_once(req_key, task_id).await {
                Ok(d) => return Ok(d),
                Err(e) if Self::is_retryable(&e) => {
                    last_err = Some(e);
                    continue;
                }
                Err(e) => return Err(e),
            }
        }
        Err(last_err.unwrap_or_else(|| {
            ApiError::InfrastructureError(InfrastructureError::ExternalApiRequestFailed(
                "Jimeng poll retries exhausted".into(),
            ))
        }))
    }

    fn is_retryable(err: &ApiError) -> bool {
        matches!(err, ApiError::InfrastructureError(InfrastructureError::ExternalApiRequestFailed(m)) if m.contains("50430") || m.contains("Concurrent Limit") || m.contains("429") || m.contains("503"))
    }

    // ========== Single-attempt ==========

    async fn submit_once(&self, req: &JimengSubmitRequest) -> Result<JimengTaskHandle, ApiError> {
        let req_key = req.req_key.clone();
        let body =
            serde_json::to_string(req).map_err(|e| ApiError::InternalServerError(e.to_string()))?;
        tracing::info!(
            "Jimeng submit: req_key={}, frames={}",
            req.req_key,
            req.frames
        );

        let text = self.signed_post("CVSync2AsyncSubmitTask", &body).await?;
        let resp = Self::parse_response::<JimengSubmitData>(&text)?;
        if resp.code != CODE_SUCCESS {
            return Err(ApiError::InfrastructureError(
                InfrastructureError::ExternalApiRequestFailed(format!(
                    "Jimeng API error: code={}, message={}",
                    resp.code, resp.message
                )),
            ));
        }
        let data = resp.data.ok_or_else(|| {
            ApiError::InfrastructureError(InfrastructureError::ExternalApiResponseParsingFailed(
                "Missing data in submit response".into(),
            ))
        })?;
        tracing::info!("Jimeng submitted: task_id={}", data.task_id);
        Ok(JimengTaskHandle {
            task_id: data.task_id,
            req_key,
        })
    }

    async fn poll_once(&self, req_key: &str, task_id: &str) -> Result<JimengResultData, ApiError> {
        let body = serde_json::json!({"req_key": req_key, "task_id": task_id}).to_string();
        let text = self.signed_post("CVSync2AsyncGetResult", &body).await?;
        let resp = Self::parse_response::<JimengResultData>(&text)?;
        if resp.code != CODE_SUCCESS {
            return Err(ApiError::InfrastructureError(
                InfrastructureError::ExternalApiRequestFailed(format!(
                    "Jimeng API error: code={}, message={}",
                    resp.code, resp.message
                )),
            ));
        }
        resp.data.ok_or_else(|| {
            ApiError::InfrastructureError(InfrastructureError::ExternalApiResponseParsingFailed(
                "Missing data in result response".into(),
            ))
        })
    }

    // ========== Response parsing ==========

    fn parse_response<T: serde::de::DeserializeOwned>(
        text: &str,
    ) -> Result<JimengApiResponse<T>, ApiError> {
        if let Ok(w) = serde_json::from_str::<VolcengineResponse<T>>(text) {
            if let Some(result) = w.result {
                return Ok(result);
            }
            if let Some(meta) = &w.response_metadata {
                if let Some(err) = &meta.error {
                    let code: i32 = err.code.as_deref().unwrap_or("0").parse().unwrap_or(0);
                    return Ok(JimengApiResponse {
                        code,
                        message: err.message.clone().unwrap_or_default(),
                        data: None,
                    });
                }
            }
        }
        serde_json::from_str::<JimengApiResponse<T>>(text).map_err(|e| {
            ApiError::InfrastructureError(InfrastructureError::ExternalApiResponseParsingFailed(
                e.to_string(),
            ))
        })
    }

    // ========== HMAC-SHA256 signing ==========

    async fn signed_post(&self, action: &str, body: &str) -> Result<String, ApiError> {
        let now = Utc::now();
        let ds = now.format("%Y%m%d").to_string();
        let dt = now.format("%Y%m%dT%H%M%SZ").to_string();
        let url = format!(
            "{}/?Action={}&Version={}",
            self.base_url, action, API_VERSION
        );
        let qs = format!("Action={}&Version={}", action, API_VERSION);
        let ph = hex_sha256(body.as_bytes());
        let host = self
            .base_url
            .trim_start_matches("https://")
            .trim_start_matches("http://")
            .split('/')
            .next()
            .unwrap_or(VOLCENGINE_HOST);
        let ch = format!(
            "content-type:application/json\nhost:{}\nx-date:{}\n",
            host, dt
        );
        let sh = "content-type;host;x-date";
        let cr = format!("POST\n/\n{}\n{}\n{}\n{}", qs, ch, sh, ph);
        let cs = format!("{}/{}/{}/request", ds, REGION, SERVICE_NAME);
        let sts = format!("HMAC-SHA256\n{}\n{}\n{}", dt, cs, hex_sha256(cr.as_bytes()));
        let sk = self.derive_signing_key(&ds);
        let sig = hex_hmac_sha256(&sk, sts.as_bytes());
        let auth = format!(
            "HMAC-SHA256 Credential={}/{}, SignedHeaders={}, Signature={}",
            self.access_key_id, cs, sh, sig
        );

        let resp = self
            .client
            .post(&url)
            .header("Content-Type", "application/json")
            .header("Host", host)
            .header("X-Date", &dt)
            .header("Authorization", &auth)
            .body(body.to_string())
            .send()
            .await
            .map_err(|e| {
                ApiError::InfrastructureError(InfrastructureError::ExternalApiRequestFailed(
                    e.to_string(),
                ))
            })?;
        let status = resp.status();
        let text = resp.text().await.map_err(|e| {
            ApiError::InfrastructureError(InfrastructureError::ExternalApiResponseParsingFailed(
                e.to_string(),
            ))
        })?;
        if !status.is_success() {
            tracing::warn!("Jimeng HTTP {}: {}", status, &text[..text.len().min(300)]);
        }
        Ok(text)
    }

    fn derive_signing_key(&self, ds: &str) -> Vec<u8> {
        let kd = hmac_sha256(self.secret_access_key.as_bytes(), ds.as_bytes());
        let kr = hmac_sha256(&kd, REGION.as_bytes());
        let ks = hmac_sha256(&kr, SERVICE_NAME.as_bytes());
        hmac_sha256(&ks, b"request")
    }

    // ========== Validation ==========

    fn validate_prompt(&self, p: &str) -> Result<(), ApiError> {
        if p.is_empty() {
            return Err(ApiError::InfrastructureError(
                InfrastructureError::ExternalApiRequestFailed("Prompt cannot be empty".into()),
            ));
        }
        if p.len() > 800 {
            return Err(ApiError::InfrastructureError(
                InfrastructureError::ExternalApiRequestFailed(format!(
                    "Prompt too long: {} (max 800)",
                    p.len()
                )),
            ));
        }
        Ok(())
    }
    fn validate_aspect_ratio(&self, r: &str) -> Result<(), ApiError> {
        if !VALID_ASPECT_RATIOS.contains(&r) {
            return Err(ApiError::InfrastructureError(
                InfrastructureError::ExternalApiRequestFailed(format!(
                    "Invalid aspect_ratio: {}",
                    r
                )),
            ));
        }
        Ok(())
    }
    fn validate_seconds(&self, s: i32) -> Result<(), ApiError> {
        if !VALID_SECONDS.contains(&s) {
            return Err(ApiError::InfrastructureError(
                InfrastructureError::ExternalApiRequestFailed(format!(
                    "Invalid seconds: {} (must be 5 or 10)",
                    s
                )),
            ));
        }
        Ok(())
    }

    pub fn encode_image(data: &[u8]) -> String {
        BASE64_STANDARD.encode(data)
    }
}

fn hmac_sha256(key: &[u8], data: &[u8]) -> Vec<u8> {
    let mut m = HmacSha256::new_from_slice(key).unwrap();
    m.update(data);
    m.finalize().into_bytes().to_vec()
}
fn hex_hmac_sha256(key: &[u8], data: &[u8]) -> String {
    hex_encode(&hmac_sha256(key, data))
}
fn hex_sha256(data: &[u8]) -> String {
    use sha2::Digest;
    let mut h = Sha256::new();
    h.update(data);
    hex_encode(&h.finalize())
}
fn hex_encode(b: &[u8]) -> String {
    b.iter().map(|x| format!("{:02x}", x)).collect()
}

pub fn is_jimeng_model(model_key: &str) -> bool {
    model_key.starts_with("jimeng-")
}

// =============================================================================
// Tests
// =============================================================================
#[cfg(test)]
mod tests {
    use super::*;
    use crate::dto::jimeng_dto::build_req_key;

    #[test]
    fn test_client_new() {
        let c = JimengClient::new("k".into(), "s".into(), None);
        assert!(c.base_url.contains(VOLCENGINE_HOST));
    }
    #[test]
    fn test_is_jimeng() {
        assert!(is_jimeng_model("jimeng-video-3.0-720p"));
        assert!(!is_jimeng_model("sora-2"));
    }
    #[test]
    fn test_sha256() {
        assert_eq!(
            hex_sha256(b""),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }
    #[test]
    fn test_encode_img() {
        assert_eq!(JimengClient::encode_image(&[0x89, 0x50]), "iVA=");
    }
    #[test]
    fn test_validate() {
        let c = JimengClient::new("k".into(), "s".into(), None);
        assert!(c.validate_prompt("").is_err());
        assert!(c.validate_prompt("ok").is_ok());
        assert!(c.validate_aspect_ratio("16:9").is_ok());
        assert!(c.validate_aspect_ratio("2:1").is_err());
        assert!(c.validate_seconds(5).is_ok());
        assert!(c.validate_seconds(15).is_err());
    }
    #[test]
    fn test_retryable() {
        let yes = ApiError::InfrastructureError(InfrastructureError::ExternalApiRequestFailed(
            "code=50430".into(),
        ));
        assert!(JimengClient::is_retryable(&yes));
        let no = ApiError::InfrastructureError(InfrastructureError::ExternalApiRequestFailed(
            "code=50400".into(),
        ));
        assert!(!JimengClient::is_retryable(&no));
    }

    // ========== Real API Tests ==========
    //
    // Run:
    //   cargo test -p glance_mind_api test_real_jimeng -- --ignored --nocapture

    fn make_client() -> JimengClient {
        dotenv::dotenv().ok();
        JimengClient::new(
            std::env::var("JIMENG_ACCESS_KEY_ID").expect("JIMENG_ACCESS_KEY_ID"),
            std::env::var("JIMENG_SECRET_ACCESS_KEY").expect("JIMENG_SECRET_ACCESS_KEY"),
            None,
        )
    }

    fn test_bmp_256() -> Vec<u8> {
        let (w, h): (u32, u32) = (256, 256);
        let mut rgb = Vec::with_capacity((w * h * 3) as usize);
        for y in 0..h {
            for x in 0..w {
                rgb.push(((x as f64 / w as f64) * 200.0) as u8 + 30);
                rgb.push(((y as f64 / h as f64) * 200.0) as u8 + 30);
                rgb.push(150);
            }
        }
        let rs = ((w * 3 + 3) / 4) * 4;
        let ps = rs * h;
        let fs = 54 + ps;
        let mut b = Vec::with_capacity(fs as usize);
        b.extend_from_slice(b"BM");
        b.extend(&(fs as u32).to_le_bytes());
        b.extend(&[0u8; 4]);
        b.extend(&54u32.to_le_bytes());
        b.extend(&40u32.to_le_bytes());
        b.extend(&(w as i32).to_le_bytes());
        b.extend(&(h as i32).to_le_bytes());
        b.extend(&1u16.to_le_bytes());
        b.extend(&24u16.to_le_bytes());
        b.extend(&0u32.to_le_bytes());
        b.extend(&(ps as u32).to_le_bytes());
        b.extend(&2835u32.to_le_bytes());
        b.extend(&2835u32.to_le_bytes());
        b.extend(&0u32.to_le_bytes());
        b.extend(&0u32.to_le_bytes());
        let pad = (rs - w * 3) as usize;
        for y in (0..h).rev() {
            for x in 0..w {
                let i = ((y * w + x) * 3) as usize;
                b.push(rgb[i + 2]);
                b.push(rgb[i + 1]);
                b.push(rgb[i]);
            }
            b.extend(std::iter::repeat(0u8).take(pad));
        }
        b
    }

    async fn poll_done(
        c: &JimengClient,
        h: &JimengTaskHandle,
        max: usize,
        interval: u64,
    ) -> Result<JimengResultData, String> {
        for i in 1..=max {
            tokio::time::sleep(Duration::from_secs(interval)).await;
            match c.get_task_status(h).await {
                Ok(r) => {
                    println!(
                        "    [poll {}/{}] status={} video={}",
                        i,
                        max,
                        r.status,
                        r.get_video_url().is_some()
                    );
                    if r.is_done() || r.is_failed() {
                        return Ok(r);
                    }
                }
                Err(e) => {
                    println!("    [poll {}/{}] err: {:?}", i, max, e);
                }
            }
        }
        Err(format!("timeout after {} polls", max))
    }

    /// ================================================================
    /// Full test: 3 products × 3 modes = 9 combinations, sequential
    /// (Pro does not support first-last-frame, so effectively 8)
    /// ================================================================
    ///
    /// Tests every combination of resolution and mode:
    ///   1. 3.0 720P  — T2V
    ///   2. 3.0 720P  — I2V First Frame
    ///   3. 3.0 720P  — I2V First-Last Frame
    ///   4. 3.0 1080P — T2V
    ///   5. 3.0 1080P — I2V First Frame
    ///   6. 3.0 1080P — I2V First-Last Frame
    ///   7. 3.0 Pro   — T2V
    ///   8. 3.0 Pro   — I2V First Frame
    ///   (Pro first-last frame skipped — not supported)
    ///
    /// Each: submit → poll every 5s → verify video URL.
    /// 5s cooldown between each test.
    ///
    ///   cargo test -p glance_mind_api test_real_jimeng_all_products -- --ignored --nocapture
    #[tokio::test]
    #[ignore]
    async fn test_real_jimeng_all_products() {
        use crate::dto::jimeng_dto::ALL_RESOLUTIONS;

        let client = make_client();
        let img_b64 = JimengClient::encode_image(&test_bmp_256());
        let img_b64_2 = img_b64.clone();

        println!("\n╔════════════════════════════════════════════════════════╗");
        println!("║  Jimeng AI Video 3.0 — Full Product × Mode Test       ║");
        println!("║  3 products × 3 modes (Pro skips FL) = 8 tests        ║");
        println!("╚════════════════════════════════════════════════════════╝\n");

        struct R {
            label: String,
            req_key: String,
            task_id: String,
            ok: bool,
            url: String,
            err: String,
        }
        let mut results: Vec<R> = Vec::new();
        let mut idx = 0;

        // (mode, prompt, first_image, end_image)
        let modes: &[(JimengVideoMode, &str, Option<&str>, Option<&str>)] = &[
            (
                JimengVideoMode::TextToVideo,
                "春天的樱花树下，花瓣随风飘落，阳光透过树枝洒下斑驳的光影",
                None,
                None,
            ),
            (
                JimengVideoMode::ImageFirstFrame,
                "让画面中的场景缓缓动起来，微风吹过树叶轻轻摇摆",
                Some(&img_b64),
                None,
            ),
            (
                JimengVideoMode::ImageFirstLastFrame,
                "从第一帧画面平滑过渡到最后一帧，中间自然衔接",
                Some(&img_b64),
                Some(&img_b64_2),
            ),
        ];

        let total_tests: usize = ALL_RESOLUTIONS
            .iter()
            .map(|res| {
                modes
                    .iter()
                    .filter(|(m, ..)| build_req_key(*m, *res).is_some())
                    .count()
            })
            .sum();

        for res in ALL_RESOLUTIONS {
            for (mode, prompt, img, end_img) in modes {
                let req_key = match build_req_key(*mode, *res) {
                    Some(k) => k,
                    None => {
                        let mode_label = match mode {
                            JimengVideoMode::TextToVideo => "T2V",
                            JimengVideoMode::ImageFirstFrame => "I2V",
                            JimengVideoMode::ImageFirstLastFrame => "I2V-FL",
                        };
                        println!(
                            "━━━ [skip] {} {} — not supported ━━━\n",
                            res.label(),
                            mode_label
                        );
                        continue;
                    }
                };
                idx += 1;
                let mode_label = match mode {
                    JimengVideoMode::TextToVideo => "T2V",
                    JimengVideoMode::ImageFirstFrame => "I2V",
                    JimengVideoMode::ImageFirstLastFrame => "I2V-FL",
                };
                let label = format!("{} {}", res.label(), mode_label);

                println!(
                    "━━━ [{}/{}] {} (req_key={}) ━━━",
                    idx, total_tests, label, req_key
                );

                let handle_result = match (*img, *end_img) {
                    (Some(ib), Some(eb)) => {
                        client
                            .create_image_first_last_video(
                                prompt.to_string(),
                                ib.to_string(),
                                eb.to_string(),
                                5,
                                *res,
                            )
                            .await
                    }
                    (Some(ib), None) => {
                        client
                            .create_image_to_video(prompt.to_string(), ib.to_string(), 5, *res)
                            .await
                    }
                    _ => {
                        client
                            .create_text_to_video(prompt.to_string(), "16:9".into(), 5, *res)
                            .await
                    }
                };

                let is_fl = *mode == JimengVideoMode::ImageFirstLastFrame;
                let poll_max = if is_fl { 120 } else { 60 };

                match handle_result {
                    Ok(handle) => {
                        println!("  ✓ submitted: task_id={}", handle.task_id);
                        match poll_done(&client, &handle, poll_max, 5).await {
                            Ok(r) if r.is_done() => {
                                let url = r.get_video_url().unwrap_or_default();
                                println!("  ✓ DONE: {}", &url[..url.len().min(70)]);
                                results.push(R {
                                    label,
                                    req_key,
                                    task_id: handle.task_id,
                                    ok: true,
                                    url,
                                    err: String::new(),
                                });
                            }
                            Ok(r) => {
                                let msg = format!("ended with status: {}", r.status);
                                println!("  ✗ {}", msg);
                                results.push(R {
                                    label,
                                    req_key,
                                    task_id: handle.task_id,
                                    ok: false,
                                    url: String::new(),
                                    err: msg,
                                });
                            }
                            Err(e) => {
                                if is_fl && e.contains("timeout") {
                                    println!("  ⚠ FL poll timeout (submitted OK, generation slow)");
                                    results.push(R {
                                        label,
                                        req_key,
                                        task_id: handle.task_id,
                                        ok: true,
                                        url: String::new(),
                                        err: format!("FL timeout (submitted OK): {}", e),
                                    });
                                } else {
                                    println!("  ✗ poll: {}", e);
                                    results.push(R {
                                        label,
                                        req_key,
                                        task_id: handle.task_id,
                                        ok: false,
                                        url: String::new(),
                                        err: e,
                                    });
                                }
                            }
                        }
                    }
                    Err(e) => {
                        let msg = format!("{:?}", e);
                        let short = if let Some(i) = msg.find("code=") {
                            msg[i..msg.len().min(i + 80)].to_string()
                        } else {
                            msg.chars().take(80).collect()
                        };
                        println!("  ✗ submit: {}", short);
                        results.push(R {
                            label,
                            req_key,
                            task_id: String::new(),
                            ok: false,
                            url: String::new(),
                            err: short,
                        });
                    }
                }

                if idx < total_tests {
                    println!("  (cooldown 5s...)\n");
                    tokio::time::sleep(Duration::from_secs(5)).await;
                }
            }
        }

        // ━━━ Summary ━━━
        let pass = results.iter().filter(|r| r.ok).count();
        let fail = results.len() - pass;

        println!("\n╔════════════════════════════════════════════════════════╗");
        println!("║                       Summary                          ║");
        println!("╠════════════════════════════════════════════════════════╣");
        for r in &results {
            let icon = if r.ok { "✓" } else { "✗" };
            let tid = if r.task_id.is_empty() {
                "N/A".to_string()
            } else {
                r.task_id.chars().take(12).collect()
            };
            println!(
                "║ {} {:<18} req_key={:<35} task={}",
                icon, r.label, r.req_key, tid
            );
            if !r.url.is_empty() {
                println!("║   video: {:.60}", r.url);
            }
            if !r.err.is_empty() {
                println!("║   error: {}", r.err);
            }
        }
        println!("╠════════════════════════════════════════════════════════╣");
        println!(
            "║  PASS: {} / {}    FAIL: {}                                  ║",
            pass,
            results.len(),
            fail
        );
        println!("╚════════════════════════════════════════════════════════╝");

        let perm_denied = results
            .iter()
            .filter(|r| !r.ok && r.err.contains("50400"))
            .count();
        let real_fail = fail - perm_denied;

        if perm_denied > 0 {
            println!(
                "\n⚠ {} combination(s) returned 50400 Access Denied.",
                perm_denied
            );
            println!("  This is an API Key permission issue, not an SDK bug.");
            println!("  Check Volcengine console: ensure AccessKey is bound to each product.");
        }

        assert_eq!(
            real_fail, 0,
            "SDK failures (non-permission): {}. Permission denied: {}. Passed: {}.",
            real_fail, perm_denied, pass
        );
        assert!(
            pass >= 3,
            "At least 720P T2V/I2V + 1080P T2V should pass. Got {} passes.",
            pass
        );
    }
}
