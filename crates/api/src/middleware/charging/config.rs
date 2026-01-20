use super::extractor::{
    AiAnalyzeExtractor, ChargingParamExtractor, ScanPostExtractor, VideoGenerateExtractor,
};
use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::sync::Arc;

pub static ROUTE_CHARGING_MAP: Lazy<HashMap<&'static str, Arc<dyn ChargingParamExtractor>>> =
    Lazy::new(|| {
        let mut map: HashMap<&'static str, Arc<dyn ChargingParamExtractor>> = HashMap::new();

        // Use full path (obtained via OriginalUri)

        // AI_ANALYZE charging mode (fixed fee, no extra headers needed)
        map.insert("/api/v1/ai/generate", Arc::new(AiAnalyzeExtractor));

        // VIDEO_GENERATE charging mode (requires X-AI-Model-ID)
        map.insert("/api/v1/video/generate", Arc::new(VideoGenerateExtractor));

        // SCAN_POST charging mode (requires X-PLATFORM-ID)
        map.insert("/api/v1/scan/post", Arc::new(ScanPostExtractor));

        // Agent analysis uses same charging mode as AI Generate (fixed fee)
        map.insert("/api/v1/agent/analyze", Arc::new(AiAnalyzeExtractor));

        map
    });

pub fn find_extractor(route: &str) -> Option<Arc<dyn ChargingParamExtractor>> {
    ROUTE_CHARGING_MAP.get(route).cloned()
}

pub fn get_registered_routes() -> Vec<&'static str> {
    ROUTE_CHARGING_MAP.keys().copied().collect()
}
