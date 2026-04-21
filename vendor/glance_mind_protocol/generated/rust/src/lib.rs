//! GlanceMind Protocol - Auto-generated from Protocol Buffers
//!
//! This crate provides protocol definitions for:
//! - Queue messages (Scheduler <-> Agent)
//! - REST API (API <-> Executor)

#![allow(clippy::derive_partial_eq_without_eq)]

use serde::{Deserialize, Deserializer, Serialize, Serializer};

// Include the generated protobuf code
pub mod glance_mind {
    include!(concat!(env!("OUT_DIR"), "/glance_mind.rs"));
}

// Re-export commonly used types at crate root
pub use glance_mind::*;

// ============================================================
// JSON Serialization for Enums
// ============================================================

impl Serialize for Platform {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.to_json_str())
    }
}

impl<'de> Deserialize<'de> for Platform {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Platform::from_json_str(&s).ok_or_else(|| {
            serde::de::Error::unknown_variant(
                &s,
                &[
                    "reddit",
                    "tiktok",
                    "facebook",
                    "instagram",
                    "twitter",
                    "youtube",
                ],
            )
        })
    }
}

impl Platform {
    /// Convert to lowercase string for JSON serialization
    pub fn to_json_str(&self) -> &'static str {
        match self {
            Platform::Unspecified => "unspecified",
            Platform::Reddit => "reddit",
            Platform::Tiktok => "tiktok",
            Platform::Facebook => "facebook",
            Platform::Instagram => "instagram",
            Platform::Twitter => "twitter",
            Platform::Youtube => "youtube",
        }
    }

    /// Parse from string (case-insensitive)
    pub fn from_json_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "reddit" => Some(Platform::Reddit),
            "tiktok" => Some(Platform::Tiktok),
            "facebook" => Some(Platform::Facebook),
            "instagram" => Some(Platform::Instagram),
            "twitter" => Some(Platform::Twitter),
            "youtube" => Some(Platform::Youtube),
            "unspecified" => Some(Platform::Unspecified),
            _ => None,
        }
    }

    /// Convert from database platform_id
    pub fn from_platform_id(id: i32) -> Self {
        match id {
            1 => Platform::Reddit,
            2 => Platform::Tiktok,
            3 => Platform::Facebook,
            4 => Platform::Instagram,
            5 => Platform::Twitter,
            6 => Platform::Youtube,
            _ => Platform::Unspecified,
        }
    }
}

impl Serialize for DataType {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.to_json_str())
    }
}

impl<'de> Deserialize<'de> for DataType {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        DataType::from_json_str(&s).ok_or_else(|| {
            serde::de::Error::unknown_variant(
                &s,
                &[
                    "video_content",
                    "video_metadata",
                    "video_comments",
                    "keyword_search",
                ],
            )
        })
    }
}

impl DataType {
    pub fn to_json_str(&self) -> &'static str {
        match self {
            DataType::Unspecified => "unspecified",
            DataType::VideoContent => "video_content",
            DataType::VideoMetadata => "video_metadata",
            DataType::VideoComments => "video_comments",
            DataType::KeywordSearch => "keyword_search",
        }
    }

    pub fn from_json_str(s: &str) -> Option<Self> {
        match s {
            "video_content" => Some(DataType::VideoContent),
            "video_metadata" => Some(DataType::VideoMetadata),
            "video_comments" => Some(DataType::VideoComments),
            "keyword_search" => Some(DataType::KeywordSearch),
            "unspecified" => Some(DataType::Unspecified),
            _ => None,
        }
    }
}

impl Serialize for TimeRange {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.to_json_str())
    }
}

impl<'de> Deserialize<'de> for TimeRange {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        TimeRange::from_json_str(&s).ok_or_else(|| {
            serde::de::Error::unknown_variant(
                &s,
                &["all_time", "last_24h", "last_7d", "last_30d", "last_180d"],
            )
        })
    }
}

impl TimeRange {
    pub fn to_json_str(&self) -> &'static str {
        match self {
            TimeRange::Unspecified => "unspecified",
            TimeRange::AllTime => "all_time",
            TimeRange::Last24h => "last_24h",
            TimeRange::Last7d => "last_7d",
            TimeRange::Last30d => "last_30d",
            TimeRange::Last180d => "last_180d",
        }
    }

    pub fn from_json_str(s: &str) -> Option<Self> {
        match s {
            "all_time" => Some(TimeRange::AllTime),
            "last_24h" => Some(TimeRange::Last24h),
            "last_7d" => Some(TimeRange::Last7d),
            "last_30d" => Some(TimeRange::Last30d),
            "last_180d" => Some(TimeRange::Last180d),
            "unspecified" => Some(TimeRange::Unspecified),
            _ => None,
        }
    }
}

impl Serialize for CommentStatus {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.to_json_str())
    }
}

impl<'de> Deserialize<'de> for CommentStatus {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        CommentStatus::from_json_str(&s).ok_or_else(|| {
            serde::de::Error::unknown_variant(&s, &["pending", "processing", "completed", "failed"])
        })
    }
}

impl CommentStatus {
    pub fn to_json_str(&self) -> &'static str {
        match self {
            CommentStatus::Unspecified => "unspecified",
            CommentStatus::Pending => "pending",
            CommentStatus::Processing => "processing",
            CommentStatus::Completed => "completed",
            CommentStatus::Failed => "failed",
        }
    }

    pub fn from_json_str(s: &str) -> Option<Self> {
        match s {
            "pending" => Some(CommentStatus::Pending),
            "processing" => Some(CommentStatus::Processing),
            "completed" => Some(CommentStatus::Completed),
            "failed" => Some(CommentStatus::Failed),
            "unspecified" => Some(CommentStatus::Unspecified),
            _ => None,
        }
    }

    /// Case-insensitive parse used by API DTOs that read DB string columns.
    /// Falls back to `Unspecified` instead of None so call sites stay simple.
    pub fn from_str_status(s: &str) -> Self {
        Self::from_json_str(&s.to_lowercase()).unwrap_or(CommentStatus::Unspecified)
    }

    /// Convert from i16 status code (database format)
    pub fn from_i16(status: i16) -> Self {
        match status {
            0 => CommentStatus::Pending,
            1 => CommentStatus::Processing,
            2 => CommentStatus::Completed,
            _ => CommentStatus::Unspecified,
        }
    }

    /// Convert to i16 status code (database format)
    pub fn to_i16(&self) -> i16 {
        match self {
            CommentStatus::Pending => 0,
            CommentStatus::Processing => 1,
            CommentStatus::Completed => 2,
            CommentStatus::Failed => 3,
            CommentStatus::Unspecified => -1,
        }
    }
}

// =====================================================================
// serde adapters for prost-generated enum int fields.
//
// prost generates proto enum fields as `i32`. Without help, serde defaults
// to integer JSON output ("platform": 3). Production consumers (executor,
// agent, frontend) expect the lowercase string form ("platform": "facebook").
// The adapters below restore the string wire format on a per-field basis,
// applied via build.rs `field_attribute`.
// =====================================================================

pub mod serde_helpers {
    use serde::{Deserialize, Deserializer, Serializer};

    use super::glance_mind::{CommentStatus, DataType, Platform, TimeRange};

    macro_rules! impl_enum_string_adapter {
        ($mod_name:ident, $enum_ty:path) => {
            pub mod $mod_name {
                use super::*;

                pub fn serialize<S: Serializer>(value: &i32, s: S) -> Result<S::Ok, S::Error> {
                    let v: $enum_ty = <$enum_ty as ::core::convert::TryFrom<i32>>::try_from(*value)
                        .unwrap_or_default();
                    s.serialize_str(v.to_json_str())
                }

                pub fn deserialize<'de, D>(d: D) -> Result<i32, D::Error>
                where
                    D: Deserializer<'de>,
                {
                    let s = String::deserialize(d)?;
                    let parsed = <$enum_ty>::from_json_str(&s).ok_or_else(|| {
                        serde::de::Error::custom(format!("unknown variant: {}", s))
                    })?;
                    Ok(parsed as i32)
                }
            }
        };
    }

    impl_enum_string_adapter!(platform, Platform);
    impl_enum_string_adapter!(data_type, DataType);
    impl_enum_string_adapter!(time_range, TimeRange);
    impl_enum_string_adapter!(comment_status, CommentStatus);
}

// =====================================================================
// Convenience constructors on prost-generated message types.
// Mirror helpers that previously lived in the hand-maintained
// lib_inline.rs (now deleted). Kept as ergonomic shims so consumers
// don't need to write field-init syntax for common shapes.
// =====================================================================

impl glance_mind::Pagination {
    pub fn new(total: i64, page: i32, per_page: i32) -> Self {
        let total_pages = if per_page > 0 {
            ((total as f64) / (per_page as f64)).ceil() as i32
        } else {
            0
        };
        Self {
            total,
            page,
            per_page,
            total_pages,
        }
    }
}

impl glance_mind::DeviceCommentsResponse {
    pub fn new(
        campaign: glance_mind::CampaignConfig,
        comments: Vec<glance_mind::CommentData>,
        total: i64,
        page: i32,
        per_page: i32,
    ) -> Self {
        Self {
            campaign: Some(campaign),
            comments,
            pagination: Some(glance_mind::Pagination::new(total, page, per_page)),
        }
    }
}

// =====================================================================
// AIPub helper constructors / accessors.
// Mirror methods that lived on the deleted lib_inline.rs / scheduler
// protocol_gen so consumers don't need to rewrite every call site.
// =====================================================================

/// Current AIPub protocol version. Bumped to 2 when the v2 unified
/// schema (UnifiedPublishContent / UnifiedAiPubInput) becomes the
/// default writer in scheduler.
pub const AIPUB_PROTOCOL_VERSION: i32 = 1;

impl glance_mind::AiTaskInput {
    /// Create input for content generation task.
    pub fn for_content_gen(video_prompt: Option<String>, content_prompt: Option<String>) -> Self {
        Self {
            version: AIPUB_PROTOCOL_VERSION,
            video_prompt,
            content_prompt,
            ..Default::default()
        }
    }

    /// Create input for video generation task.
    pub fn for_video_gen(model: String, prompt: String, aipub_task_id: i32) -> Self {
        Self {
            version: AIPUB_PROTOCOL_VERSION,
            model: Some(model),
            prompt: Some(prompt),
            aipub_task_id: Some(aipub_task_id),
            ..Default::default()
        }
    }

    /// Create input for video generation with start/end frame images
    /// (for FL / Vidu image-to-video models).
    pub fn for_video_gen_with_images(
        model: String,
        prompt: String,
        aipub_task_id: i32,
        start_image_url: Option<String>,
        end_image_url: Option<String>,
    ) -> Self {
        Self {
            version: AIPUB_PROTOCOL_VERSION,
            model: Some(model),
            prompt: Some(prompt),
            aipub_task_id: Some(aipub_task_id),
            start_image_url,
            end_image_url,
            ..Default::default()
        }
    }
}

impl glance_mind::AiTaskResult {
    /// Create result for content generation task (count + pending video count).
    pub fn for_content_gen(content_count: i32, video_pending_count: i32) -> Self {
        Self {
            version: AIPUB_PROTOCOL_VERSION,
            content_count: Some(content_count),
            video_pending_count: Some(video_pending_count),
            generated_at: Some(chrono::Utc::now().to_rfc3339()),
            ..Default::default()
        }
    }

    /// Create result for content generation with content variations populated.
    pub fn for_content_gen_with_variations(
        content_variations: Vec<glance_mind::ContentVariation>,
        video_pending_count: i32,
    ) -> Self {
        let content_count = content_variations.len() as i32;
        Self {
            version: AIPUB_PROTOCOL_VERSION,
            content_count: Some(content_count),
            video_pending_count: Some(video_pending_count),
            content_variations,
            generated_at: Some(chrono::Utc::now().to_rfc3339()),
            ..Default::default()
        }
    }

    /// Create result for video generation task (just url).
    pub fn for_video_gen(video_url: String) -> Self {
        Self {
            version: AIPUB_PROTOCOL_VERSION,
            video_url: Some(video_url),
            generated_at: Some(chrono::Utc::now().to_rfc3339()),
            ..Default::default()
        }
    }

    /// Create error result. Renamed from `error()` (which collided with the
    /// generated `error` field on AiTaskResult message).
    pub fn for_error(error_msg: String) -> Self {
        Self {
            version: AIPUB_PROTOCOL_VERSION,
            error: Some(error_msg),
            generated_at: Some(chrono::Utc::now().to_rfc3339()),
            ..Default::default()
        }
    }
}

impl glance_mind::AiPubInput {
    pub fn has_reference_video(&self) -> bool {
        self.reference_video.is_some()
    }

    pub fn with_reference_video(
        mut self,
        reference_video: glance_mind::ReferenceVideoConfig,
    ) -> Self {
        self.reference_video = Some(reference_video);
        self
    }

    /// Get the effective video prompt (falls back to legacy `prompt` if empty).
    pub fn get_video_prompt(&self) -> &str {
        if !self.video_prompt.is_empty() {
            &self.video_prompt
        } else {
            &self.prompt
        }
    }

    /// Get the effective content prompt (falls back to legacy `prompt` if empty).
    pub fn get_content_prompt(&self) -> &str {
        if !self.content_prompt.is_empty() {
            &self.content_prompt
        } else {
            &self.prompt
        }
    }

    /// Per-account image override; falls back to default_images when absent.
    /// Note: prost generates account_images as HashMap (proto map), not Option<HashMap>.
    pub fn get_images_for_account(
        &self,
        account_id: &str,
    ) -> Option<&glance_mind::AiPubImageConfig> {
        if let Some(config) = self.account_images.get(account_id) {
            return Some(config);
        }
        self.default_images.as_ref()
    }
}

impl glance_mind::ReferenceVideoConfig {
    pub fn new(video_url: impl Into<String>, model_name: impl Into<String>) -> Self {
        Self {
            video_url: video_url.into(),
            model_name: model_name.into(),
            target_duration: None,
        }
    }
}

impl glance_mind::PatrolCollectionType {
    /// Lowercase wire-format string ("profile" / "notification").
    /// DB column gm_patrol_reports.report_type stores this string form.
    pub fn to_json_str(&self) -> &'static str {
        match self {
            glance_mind::PatrolCollectionType::Unspecified => "unspecified",
            glance_mind::PatrolCollectionType::Profile => "profile",
            glance_mind::PatrolCollectionType::Notification => "notification",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_platform_json_serialization() {
        let platform = Platform::Tiktok;
        let json = serde_json::to_string(&platform).unwrap();
        assert_eq!(json, r#""tiktok""#);

        let parsed: Platform = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, Platform::Tiktok);
    }

    #[test]
    fn test_crawler_task_json_serialization() {
        let task = CrawlerTask {
            meta: Some(CrawlerTaskMeta {
                task_id: 123,
                source: "campaign-456".to_string(),
                timestamp: 1234567890.0,
                campaign_id: 456,
            }),
            spec: Some(CrawlerTaskSpec {
                platform: Platform::Tiktok.into(),
                data_type: DataType::VideoComments.into(),
            }),
            config: Some(TaskConfig {
                keywords: vec!["test".to_string()],
                max_count: 50,
                search_offset: 0,
                search_limit: 20,
                filters: Some(TaskFilters {
                    time_range: Some(TimeRange::Last180d.into()),
                    region: Some("US".to_string()),
                }),
                search_options: None,
            }),
        };

        let json = serde_json::to_string_pretty(&task).unwrap();
        println!("{}", json);

        let parsed: CrawlerTask = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.meta.unwrap().task_id, 123);
    }

    #[test]
    fn test_comment_status_i16_conversion() {
        assert_eq!(CommentStatus::from_i16(0), CommentStatus::Pending);
        assert_eq!(CommentStatus::Completed.to_i16(), 2);
    }

    // =====================================================
    // image_provider_routes.json fixture validation
    // (TDD step 1: matrix CI gate single source of truth)
    // =====================================================

    #[derive(serde::Deserialize, Debug)]
    struct RoutesFile {
        version: u32,
        routes: Vec<RouteEntry>,
    }

    #[derive(serde::Deserialize, Debug, PartialEq)]
    struct RouteEntry {
        model_key: String,
        provider: String,
        modes: Vec<String>,
    }

    fn load_routes() -> RoutesFile {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../proto/image_provider_routes.json");
        let raw = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("cannot read {}: {}", path.display(), e));
        serde_json::from_str(&raw).expect("routes JSON must parse")
    }

    #[test]
    fn test_image_provider_routes_fixture_exists_and_parses() {
        let f = load_routes();
        assert_eq!(f.version, 1, "fixture schema version must be 1");
        assert!(!f.routes.is_empty(), "must have at least one route");
    }

    #[test]
    fn test_image_provider_routes_provider_whitelist() {
        let f = load_routes();
        for r in &f.routes {
            assert!(
                matches!(r.provider.as_str(), "flux" | "seedream" | "openai"),
                "unknown provider {} in route {}",
                r.provider,
                r.model_key
            );
        }
    }

    #[test]
    fn test_image_provider_routes_mode_whitelist() {
        let f = load_routes();
        for r in &f.routes {
            assert!(!r.modes.is_empty(), "route {} has empty modes", r.model_key);
            for m in &r.modes {
                assert!(
                    matches!(m.as_str(), "text_to_image" | "image_edit"),
                    "unknown mode {} in route {}",
                    m,
                    r.model_key
                );
            }
        }
    }

    #[test]
    fn test_image_provider_routes_covers_matrix_table_b() {
        // Must contain every route in image-provider-param-matrix.md Table B.
        let f = load_routes();
        let must_have: &[(&str, &str, &[&str])] = &[
            ("flux-kontext-pro", "flux", &["text_to_image", "image_edit"]),
            ("flux-kontext-max", "flux", &["text_to_image", "image_edit"]),
            (
                "seedream-4-0-250828",
                "seedream",
                &["text_to_image", "image_edit"],
            ),
            (
                "seedream-4-5-251128",
                "seedream",
                &["text_to_image", "image_edit"],
            ),
        ];
        for (model, provider, modes) in must_have {
            let entry = f.routes.iter().find(|r| r.model_key == *model);
            let entry = entry
                .unwrap_or_else(|| panic!("matrix Table B model_key {} missing in fixture", model));
            assert_eq!(entry.provider, *provider, "provider mismatch for {}", model);
            for m in *modes {
                assert!(
                    entry.modes.iter().any(|s| s == m),
                    "mode {} missing for model {}",
                    m,
                    model
                );
            }
        }
    }

    #[test]
    fn test_image_provider_routes_openai_legacy_no_edit() {
        // OpenAI providers (gpt-4o-image, dall-e-3) currently only support text_to_image.
        // legacy.rs hard-codes this; fixture must not list image_edit for them.
        let f = load_routes();
        for r in f.routes.iter().filter(|r| r.provider == "openai") {
            assert!(
                !r.modes.iter().any(|m| m == "image_edit"),
                "OpenAI provider {} cannot list image_edit (legacy client only supports T2I)",
                r.model_key
            );
        }
    }

    // =====================================================
    // v2 message roundtrip tests
    // (TDD step 2: build.rs must compile aipub.proto + patrol.proto)
    // =====================================================

    #[test]
    fn test_v2_unified_pub_content_roundtrip() {
        // Build a minimal valid v2 UnifiedPublishContent and roundtrip it
        // through serde_json. This will only compile after build.rs is
        // refactored to feed aipub.proto into prost-build.
        let content = UnifiedPublishContent {
            version: 2,
            platform: "tiktok".to_string(),
            platform_id: 2,
            content_type: "video".to_string(),
            plan_type: "single_video".to_string(),
            media: vec![MediaItem {
                kind: MediaKind::Video as i32,
                source: MediaSource::AiGenerated as i32,
                status: MediaStatus::Ready as i32,
                role: MediaRole::Primary as i32,
                order: 0,
                url: "https://oss.example.com/video.mp4".to_string(),
                ai_task_id: Some(123),
                mime: None,
                width_px: None,
                height_px: None,
                duration_ms: None,
                bytes: None,
                provider_asset_uri: None,
                language: None,
                parent_media_index: None,
            }],
            texts: vec![],
            links: vec![],
            tags: vec![],
            mentions: vec![],
            platform_extras: Default::default(),
            behavior: None,
            schedule: None,
            post_publish: vec![],
        };

        let json = serde_json::to_string(&content).expect("serialize");
        let parsed: UnifiedPublishContent = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(parsed.version, 2);
        assert_eq!(parsed.media.len(), 1);
        assert_eq!(parsed.media[0].url, "https://oss.example.com/video.mp4");
    }

    #[test]
    fn test_v2_image_generation_spec_roundtrip() {
        // ImageGenerationSpec must include all 15 fields, including the
        // 8 recently added (aspect_ratio, output_format, seed, watermark,
        // provider_hint, mode, safety_tolerance).
        let spec = ImageGenerationSpec {
            prompts: vec!["a cat".to_string()],
            count: 3,
            model: Some("flux-kontext-pro".to_string()),
            width_px: 0,
            height_px: 0,
            role_hint: MediaRole::CarouselItem as i32,
            reference_image_urls: vec![],
            aspect_ratio: Some("16:9".to_string()),
            output_format: Some("png".to_string()),
            extras: Default::default(),
            seed: Some(42),
            watermark: Some(false),
            provider_hint: Some("flux".to_string()),
            mode: Some("text_to_image".to_string()),
            safety_tolerance: Some(2),
        };

        let json = serde_json::to_string(&spec).expect("serialize");
        let parsed: ImageGenerationSpec = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(parsed.aspect_ratio.as_deref(), Some("16:9"));
        assert_eq!(parsed.provider_hint.as_deref(), Some("flux"));
        assert_eq!(parsed.safety_tolerance, Some(2));
    }

    #[test]
    fn test_v2_unified_publish_result_roundtrip() {
        // UnifiedPublishResult replaces v1 ExecutorTaskStatusUpdate with
        // typed status enum + media/post_publish results + initial metrics.
        let result = UnifiedPublishResult {
            version: 2,
            task_id: 999,
            status: PublishResultStatus::Succeeded as i32,
            platform_post_id: Some("t3_xxx".to_string()),
            platform_post_url: Some("https://reddit.com/r/foo/comments/xxx".to_string()),
            published_at: Some("2026-04-19T12:00:00Z".to_string()),
            failed_reason: None,
            failed_error_code: None,
            retry_count: 0,
            next_retry_at: None,
            media_results: vec![],
            post_publish_results: vec![],
            initial_metrics: None,
            raw_response_json: None,
        };

        let json = serde_json::to_string(&result).expect("serialize");
        let parsed: UnifiedPublishResult = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(parsed.status, PublishResultStatus::Succeeded as i32);
        assert_eq!(parsed.platform_post_id.as_deref(), Some("t3_xxx"));
    }

    #[test]
    fn test_v2_unified_ai_pub_input_with_three_specs() {
        // The asymmetric input model: three repeated *GenerationSpec lists
        // (text/image/video) can coexist in one plan.
        let input = UnifiedAiPubInput {
            version: 2,
            text_generations: vec![TextGenerationSpec {
                prompts: vec!["caption".to_string()],
                count: 1,
                target_roles: vec!["CAPTION".to_string()],
                model: Some("gpt-4o".to_string()),
                extras: Default::default(),
            }],
            image_generations: vec![ImageGenerationSpec {
                prompts: vec!["a thumbnail".to_string()],
                count: 1,
                model: Some("flux-kontext-pro".to_string()),
                role_hint: MediaRole::Cover as i32,
                ..Default::default()
            }],
            video_generations: vec![],
            initial_media: vec![],
            account_media: Default::default(),
            platform_config: None,
            generation_extras: Default::default(),
        };

        let json = serde_json::to_string(&input).expect("serialize");
        let parsed: UnifiedAiPubInput = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(parsed.text_generations.len(), 1);
        assert_eq!(parsed.image_generations.len(), 1);
        assert_eq!(parsed.video_generations.len(), 0);
    }
}
