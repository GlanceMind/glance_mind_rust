use super::types::ActionType;
use async_trait::async_trait;
use axum::body::Body;
use axum::http::Request;

#[derive(Debug, Clone)]
pub struct ChargingParams {
    pub action_type: ActionType,
    pub ai_model_id: Option<i32>,
    pub platform_id: Option<i32>,
}

#[async_trait]
pub trait ChargingParamExtractor: Send + Sync {
    async fn extract(&self, req: &Request<Body>) -> Option<ChargingParams>;
}

// AI analysis extractor
// Query pricing rules by action_type (platform_id = NULL)
pub struct AiAnalyzeExtractor;

#[async_trait]
impl ChargingParamExtractor for AiAnalyzeExtractor {
    async fn extract(&self, _req: &Request<Body>) -> Option<ChargingParams> {
        // AI_ANALYZE queries first record by action_type (platform_id = NULL)
        Some(ChargingParams {
            action_type: ActionType::AiAnalyze,
            ai_model_id: None,
            platform_id: None,
        })
    }
}

// Video generate extractor
// Query pricing rules by action_type, then calculate multiplier with ai_model_id.
// X-AI-Model-ID is optional for backward compatibility because the video handler
// still supports requests that omit ai_model_id and fall back to the default model.
pub struct VideoGenerateExtractor;

#[async_trait]
impl ChargingParamExtractor for VideoGenerateExtractor {
    async fn extract(&self, req: &Request<Body>) -> Option<ChargingParams> {
        let ai_model_id = match req.headers().get("X-AI-Model-ID") {
            Some(value) => {
                let value = value.to_str().ok()?;
                Some(value.parse().ok()?)
            }
            None => None,
        };

        // video_generate queries first record by action_type, then calculates multiplier with ai_model_id
        Some(ChargingParams {
            action_type: ActionType::VideoGenerate,
            ai_model_id,
            platform_id: None,
        })
    }
}

// Scan post extractor
// Query pricing rules by action_type + platform_id
// Required parameter: X-PLATFORM-ID
pub struct ScanPostExtractor;

#[async_trait]
impl ChargingParamExtractor for ScanPostExtractor {
    async fn extract(&self, req: &Request<Body>) -> Option<ChargingParams> {
        // scan_post must provide platform_id, query by action_type + platform_id
        let platform_id = req
            .headers()
            .get("X-PLATFORM-ID")
            .and_then(|v| v.to_str().ok())
            .and_then(|s| s.parse().ok())?; // Return None if not provided or parse fails

        Some(ChargingParams {
            action_type: ActionType::ScanPost,
            ai_model_id: None,
            platform_id: Some(platform_id),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::{ChargingParamExtractor, VideoGenerateExtractor};
    use axum::body::Body;
    use axum::http::Request;

    #[tokio::test]
    async fn video_generate_allows_missing_model_header() {
        let req = Request::builder()
            .uri("/api/v1/video/generate")
            .body(Body::empty())
            .expect("request");

        let params = VideoGenerateExtractor
            .extract(&req)
            .await
            .expect("extractor should allow missing header");

        assert_eq!(params.ai_model_id, None);
    }

    #[tokio::test]
    async fn video_generate_extracts_valid_model_header() {
        let req = Request::builder()
            .uri("/api/v1/video/generate")
            .header("X-AI-Model-ID", "9")
            .body(Body::empty())
            .expect("request");

        let params = VideoGenerateExtractor
            .extract(&req)
            .await
            .expect("extractor should parse valid header");

        assert_eq!(params.ai_model_id, Some(9));
    }

    #[tokio::test]
    async fn video_generate_rejects_invalid_model_header() {
        let req = Request::builder()
            .uri("/api/v1/video/generate")
            .header("X-AI-Model-ID", "not-a-number")
            .body(Body::empty())
            .expect("request");

        let params = VideoGenerateExtractor.extract(&req).await;

        assert!(params.is_none(), "invalid header should fail extraction");
    }
}
