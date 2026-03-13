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
    use super::{AiAnalyzeExtractor, ChargingParamExtractor, ScanPostExtractor};
    use axum::body::Body;
    use axum::http::Request;

    #[tokio::test]
    async fn ai_analyze_extracts_without_headers() {
        let req = Request::builder()
            .uri("/api/v1/ai/generate")
            .body(Body::empty())
            .expect("request");

        let params = AiAnalyzeExtractor
            .extract(&req)
            .await
            .expect("AI analyze should always succeed");

        assert_eq!(params.ai_model_id, None);
        assert_eq!(params.platform_id, None);
        assert_eq!(params.action_type.as_str(), "AI_ANALYZE");
    }

    #[tokio::test]
    async fn scan_post_extracts_valid_platform_header() {
        let req = Request::builder()
            .uri("/api/v1/scan/post")
            .header("X-PLATFORM-ID", "2")
            .body(Body::empty())
            .expect("request");

        let params = ScanPostExtractor
            .extract(&req)
            .await
            .expect("scan post should parse valid header");

        assert_eq!(params.platform_id, Some(2));
        assert_eq!(params.action_type.as_str(), "SCAN_POST");
    }

    #[tokio::test]
    async fn scan_post_fails_without_platform_header() {
        let req = Request::builder()
            .uri("/api/v1/scan/post")
            .body(Body::empty())
            .expect("request");

        let params = ScanPostExtractor.extract(&req).await;
        assert!(params.is_none(), "missing header should fail extraction");
    }

    #[tokio::test]
    async fn scan_post_fails_with_invalid_platform_header() {
        let req = Request::builder()
            .uri("/api/v1/scan/post")
            .header("X-PLATFORM-ID", "not-a-number")
            .body(Body::empty())
            .expect("request");

        let params = ScanPostExtractor.extract(&req).await;
        assert!(params.is_none(), "invalid header should fail extraction");
    }
}
