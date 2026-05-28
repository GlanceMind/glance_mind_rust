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

impl OpenMontageProtocolVersion {
    pub fn to_json_str(&self) -> &'static str {
        match self {
            OpenMontageProtocolVersion::Unspecified => "unspecified",
            OpenMontageProtocolVersion::V1 => "v1",
        }
    }

    pub fn from_json_str(s: &str) -> Option<Self> {
        match s {
            "v1" => Some(OpenMontageProtocolVersion::V1),
            "unspecified" => Some(OpenMontageProtocolVersion::Unspecified),
            _ => None,
        }
    }
}

impl OpenMontageJobStatus {
    pub fn to_json_str(&self) -> &'static str {
        match self {
            OpenMontageJobStatus::Unspecified => "unspecified",
            OpenMontageJobStatus::Queued => "queued",
            OpenMontageJobStatus::Preflight => "preflight",
            OpenMontageJobStatus::AwaitingApproval => "awaiting_approval",
            OpenMontageJobStatus::Running => "running",
            OpenMontageJobStatus::Degraded => "degraded",
            OpenMontageJobStatus::Completed => "completed",
            OpenMontageJobStatus::Failed => "failed",
            OpenMontageJobStatus::Cancelled => "cancelled",
            OpenMontageJobStatus::InProgress => "in_progress",
            OpenMontageJobStatus::AwaitingHuman => "awaiting_human",
        }
    }

    pub fn from_json_str(s: &str) -> Option<Self> {
        match s {
            "queued" => Some(OpenMontageJobStatus::Queued),
            "preflight" => Some(OpenMontageJobStatus::Preflight),
            "awaiting_approval" => Some(OpenMontageJobStatus::AwaitingApproval),
            "running" => Some(OpenMontageJobStatus::Running),
            "degraded" => Some(OpenMontageJobStatus::Degraded),
            "completed" => Some(OpenMontageJobStatus::Completed),
            "failed" => Some(OpenMontageJobStatus::Failed),
            "cancelled" => Some(OpenMontageJobStatus::Cancelled),
            "in_progress" => Some(OpenMontageJobStatus::InProgress),
            "awaiting_human" => Some(OpenMontageJobStatus::AwaitingHuman),
            "unspecified" => Some(OpenMontageJobStatus::Unspecified),
            _ => None,
        }
    }
}

impl OpenMontageEventType {
    pub fn to_json_str(&self) -> &'static str {
        match self {
            OpenMontageEventType::Unspecified => "unspecified",
            OpenMontageEventType::JobAccepted => "job_accepted",
            OpenMontageEventType::JobStatusChanged => "job_status_changed",
            OpenMontageEventType::StageStarted => "stage_started",
            OpenMontageEventType::StageCheckpointed => "stage_checkpointed",
            OpenMontageEventType::ApprovalRequired => "approval_required",
            OpenMontageEventType::ApprovalRecorded => "approval_recorded",
            OpenMontageEventType::ArtifactReady => "artifact_ready",
            OpenMontageEventType::JobCompleted => "job_completed",
            OpenMontageEventType::JobFailed => "job_failed",
            OpenMontageEventType::PreflightCompleted => "preflight_completed",
            OpenMontageEventType::PipelineManifestReady => "pipeline_manifest_ready",
            OpenMontageEventType::ToolStarted => "tool_started",
            OpenMontageEventType::ToolCompleted => "tool_completed",
            OpenMontageEventType::ToolFailed => "tool_failed",
            OpenMontageEventType::CheckpointValidated => "checkpoint_validated",
            OpenMontageEventType::ArtifactValidated => "artifact_validated",
            OpenMontageEventType::ProviderBlocked => "provider_blocked",
        }
    }

    pub fn from_json_str(s: &str) -> Option<Self> {
        match s {
            "job_accepted" => Some(OpenMontageEventType::JobAccepted),
            "job_status_changed" => Some(OpenMontageEventType::JobStatusChanged),
            "stage_started" => Some(OpenMontageEventType::StageStarted),
            "stage_checkpointed" => Some(OpenMontageEventType::StageCheckpointed),
            "approval_required" => Some(OpenMontageEventType::ApprovalRequired),
            "approval_recorded" => Some(OpenMontageEventType::ApprovalRecorded),
            "artifact_ready" => Some(OpenMontageEventType::ArtifactReady),
            "job_completed" => Some(OpenMontageEventType::JobCompleted),
            "job_failed" => Some(OpenMontageEventType::JobFailed),
            "preflight_completed" => Some(OpenMontageEventType::PreflightCompleted),
            "pipeline_manifest_ready" => Some(OpenMontageEventType::PipelineManifestReady),
            "tool_started" => Some(OpenMontageEventType::ToolStarted),
            "tool_completed" => Some(OpenMontageEventType::ToolCompleted),
            "tool_failed" => Some(OpenMontageEventType::ToolFailed),
            "checkpoint_validated" => Some(OpenMontageEventType::CheckpointValidated),
            "artifact_validated" => Some(OpenMontageEventType::ArtifactValidated),
            "provider_blocked" => Some(OpenMontageEventType::ProviderBlocked),
            "unspecified" => Some(OpenMontageEventType::Unspecified),
            _ => None,
        }
    }
}

impl OpenMontageInputAssetKind {
    pub fn to_json_str(&self) -> &'static str {
        match self {
            OpenMontageInputAssetKind::Unspecified => "unspecified",
            OpenMontageInputAssetKind::ReferenceVideo => "reference_video",
            OpenMontageInputAssetKind::SourceVideo => "source_video",
            OpenMontageInputAssetKind::StartFrame => "start_frame",
            OpenMontageInputAssetKind::EndFrame => "end_frame",
            OpenMontageInputAssetKind::ReferenceImage => "reference_image",
            OpenMontageInputAssetKind::BrandAsset => "brand_asset",
            OpenMontageInputAssetKind::Audio => "audio",
            OpenMontageInputAssetKind::Subtitle => "subtitle",
            OpenMontageInputAssetKind::Narration => "narration",
            OpenMontageInputAssetKind::Music => "music",
            OpenMontageInputAssetKind::Sfx => "sfx",
            OpenMontageInputAssetKind::Diagram => "diagram",
            OpenMontageInputAssetKind::Animation => "animation",
            OpenMontageInputAssetKind::CodeSnippet => "code_snippet",
            OpenMontageInputAssetKind::Font => "font",
            OpenMontageInputAssetKind::Lut => "lut",
        }
    }

    pub fn from_json_str(s: &str) -> Option<Self> {
        match s {
            "reference_video" => Some(OpenMontageInputAssetKind::ReferenceVideo),
            "source_video" => Some(OpenMontageInputAssetKind::SourceVideo),
            "start_frame" => Some(OpenMontageInputAssetKind::StartFrame),
            "end_frame" => Some(OpenMontageInputAssetKind::EndFrame),
            "reference_image" => Some(OpenMontageInputAssetKind::ReferenceImage),
            "brand_asset" => Some(OpenMontageInputAssetKind::BrandAsset),
            "audio" => Some(OpenMontageInputAssetKind::Audio),
            "subtitle" => Some(OpenMontageInputAssetKind::Subtitle),
            "narration" => Some(OpenMontageInputAssetKind::Narration),
            "music" => Some(OpenMontageInputAssetKind::Music),
            "sfx" => Some(OpenMontageInputAssetKind::Sfx),
            "diagram" => Some(OpenMontageInputAssetKind::Diagram),
            "animation" => Some(OpenMontageInputAssetKind::Animation),
            "code_snippet" => Some(OpenMontageInputAssetKind::CodeSnippet),
            "font" => Some(OpenMontageInputAssetKind::Font),
            "lut" => Some(OpenMontageInputAssetKind::Lut),
            "unspecified" => Some(OpenMontageInputAssetKind::Unspecified),
            _ => None,
        }
    }
}

impl OpenMontageArtifactKind {
    pub fn to_json_str(&self) -> &'static str {
        match self {
            OpenMontageArtifactKind::Unspecified => "unspecified",
            OpenMontageArtifactKind::Video => "video",
            OpenMontageArtifactKind::Image => "image",
            OpenMontageArtifactKind::Audio => "audio",
            OpenMontageArtifactKind::Subtitle => "subtitle",
            OpenMontageArtifactKind::Json => "json",
            OpenMontageArtifactKind::Report => "report",
            OpenMontageArtifactKind::Directory => "directory",
            OpenMontageArtifactKind::Narration => "narration",
            OpenMontageArtifactKind::Music => "music",
            OpenMontageArtifactKind::Sfx => "sfx",
            OpenMontageArtifactKind::Diagram => "diagram",
            OpenMontageArtifactKind::Animation => "animation",
            OpenMontageArtifactKind::CodeSnippet => "code_snippet",
            OpenMontageArtifactKind::Font => "font",
            OpenMontageArtifactKind::Lut => "lut",
            OpenMontageArtifactKind::Review => "review",
            OpenMontageArtifactKind::Checkpoint => "checkpoint",
            OpenMontageArtifactKind::Manifest => "manifest",
        }
    }

    pub fn from_json_str(s: &str) -> Option<Self> {
        match s {
            "video" => Some(OpenMontageArtifactKind::Video),
            "image" => Some(OpenMontageArtifactKind::Image),
            "audio" => Some(OpenMontageArtifactKind::Audio),
            "subtitle" => Some(OpenMontageArtifactKind::Subtitle),
            "json" => Some(OpenMontageArtifactKind::Json),
            "report" => Some(OpenMontageArtifactKind::Report),
            "directory" => Some(OpenMontageArtifactKind::Directory),
            "narration" => Some(OpenMontageArtifactKind::Narration),
            "music" => Some(OpenMontageArtifactKind::Music),
            "sfx" => Some(OpenMontageArtifactKind::Sfx),
            "diagram" => Some(OpenMontageArtifactKind::Diagram),
            "animation" => Some(OpenMontageArtifactKind::Animation),
            "code_snippet" => Some(OpenMontageArtifactKind::CodeSnippet),
            "font" => Some(OpenMontageArtifactKind::Font),
            "lut" => Some(OpenMontageArtifactKind::Lut),
            "review" => Some(OpenMontageArtifactKind::Review),
            "checkpoint" => Some(OpenMontageArtifactKind::Checkpoint),
            "manifest" => Some(OpenMontageArtifactKind::Manifest),
            "unspecified" => Some(OpenMontageArtifactKind::Unspecified),
            _ => None,
        }
    }
}

impl OpenMontageErrorCode {
    pub fn to_json_str(&self) -> &'static str {
        match self {
            OpenMontageErrorCode::Unspecified => "unspecified",
            OpenMontageErrorCode::UnsupportedProtocolVersion => "unsupported_protocol_version",
            OpenMontageErrorCode::ValidationError => "validation_error",
            OpenMontageErrorCode::SecretMaterialRejected => "secret_material_rejected",
            OpenMontageErrorCode::IdempotencyConflict => "idempotency_conflict",
            OpenMontageErrorCode::PipelineNotFound => "pipeline_not_found",
            OpenMontageErrorCode::ApprovalRequired => "approval_required",
            OpenMontageErrorCode::ApprovalRejected => "approval_rejected",
            OpenMontageErrorCode::ProviderUnavailable => "provider_unavailable",
            OpenMontageErrorCode::RenderFailed => "render_failed",
            OpenMontageErrorCode::InternalError => "internal_error",
            OpenMontageErrorCode::ToolNotFound => "tool_not_found",
            OpenMontageErrorCode::ToolValidationError => "tool_validation_error",
            OpenMontageErrorCode::CheckpointValidationError => "checkpoint_validation_error",
            OpenMontageErrorCode::ArtifactValidationError => "artifact_validation_error",
            OpenMontageErrorCode::RuntimeUnavailable => "runtime_unavailable",
            OpenMontageErrorCode::BudgetExceeded => "budget_exceeded",
            OpenMontageErrorCode::LiveProviderNotApproved => "live_provider_not_approved",
        }
    }

    pub fn from_json_str(s: &str) -> Option<Self> {
        match s {
            "unsupported_protocol_version" => {
                Some(OpenMontageErrorCode::UnsupportedProtocolVersion)
            }
            "validation_error" => Some(OpenMontageErrorCode::ValidationError),
            "secret_material_rejected" => Some(OpenMontageErrorCode::SecretMaterialRejected),
            "idempotency_conflict" => Some(OpenMontageErrorCode::IdempotencyConflict),
            "pipeline_not_found" => Some(OpenMontageErrorCode::PipelineNotFound),
            "approval_required" => Some(OpenMontageErrorCode::ApprovalRequired),
            "approval_rejected" => Some(OpenMontageErrorCode::ApprovalRejected),
            "provider_unavailable" => Some(OpenMontageErrorCode::ProviderUnavailable),
            "render_failed" => Some(OpenMontageErrorCode::RenderFailed),
            "internal_error" => Some(OpenMontageErrorCode::InternalError),
            "tool_not_found" => Some(OpenMontageErrorCode::ToolNotFound),
            "tool_validation_error" => Some(OpenMontageErrorCode::ToolValidationError),
            "checkpoint_validation_error" => Some(OpenMontageErrorCode::CheckpointValidationError),
            "artifact_validation_error" => Some(OpenMontageErrorCode::ArtifactValidationError),
            "runtime_unavailable" => Some(OpenMontageErrorCode::RuntimeUnavailable),
            "budget_exceeded" => Some(OpenMontageErrorCode::BudgetExceeded),
            "live_provider_not_approved" => Some(OpenMontageErrorCode::LiveProviderNotApproved),
            "unspecified" => Some(OpenMontageErrorCode::Unspecified),
            _ => None,
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

    use super::glance_mind::{
        CommentStatus, DataType, OpenMontageArtifactKind, OpenMontageErrorCode,
        OpenMontageEventType, OpenMontageInputAssetKind, OpenMontageJobStatus,
        OpenMontageProtocolVersion, Platform, TimeRange,
    };

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
    impl_enum_string_adapter!(openmontage_protocol_version, OpenMontageProtocolVersion);
    impl_enum_string_adapter!(openmontage_job_status, OpenMontageJobStatus);
    impl_enum_string_adapter!(openmontage_event_type, OpenMontageEventType);
    impl_enum_string_adapter!(openmontage_input_asset_kind, OpenMontageInputAssetKind);
    impl_enum_string_adapter!(openmontage_artifact_kind, OpenMontageArtifactKind);
    impl_enum_string_adapter!(openmontage_error_code, OpenMontageErrorCode);
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

    #[test]
    fn test_openmontage_protocol_professional_video_request_json_uses_strings() {
        let req = OpenMontageProfessionalVideoRequest {
            version: OpenMontageProtocolVersion::V1 as i32,
            request_id: "gm-plan-123-task-456".to_string(),
            idempotency_key: "gm-openmontage-123-456".to_string(),
            tenant_id: "tenant-1".to_string(),
            user_id: "42".to_string(),
            title: "Launch video".to_string(),
            prompt: "Create a professional TikTok marketing video.".to_string(),
            target_platform: "tiktok".to_string(),
            language: "zh-CN".to_string(),
            duration_seconds: 30,
            aspect_ratio: "9:16".to_string(),
            audience: None,
            objective: None,
            brand_json: None,
            pipeline: "glancemind-marketing-video".to_string(),
            style_playbook: Some("product-growth".to_string()),
            render_runtime: Some("remotion".to_string()),
            quality_tier: "professional".to_string(),
            approval_policy: "auto_except_paid_provider_switch".to_string(),
            budget_limit_usd: 3.0,
            provider_preferences: Default::default(),
            assets: vec![OpenMontageInputAsset {
                kind: OpenMontageInputAssetKind::ReferenceImage as i32,
                role: "brand_reference".to_string(),
                uri: "https://cdn.example.com/ref.png".to_string(),
                mime_type: Some("image/png".to_string()),
                width_px: None,
                height_px: None,
                duration_ms: None,
                metadata_json: None,
            }],
            callback: Some(OpenMontageCallbackConfig {
                callback_url: "https://api.example.com/internal/openmontage/callback".to_string(),
                callback_secret_ref: "vault://openmontage/callback".to_string(),
                event_types: vec!["job.completed".to_string()],
            }),
            metadata_json: Some(r#"{"glancemind_plan_id":123}"#.to_string()),
            source_script: Some("30-second launch script".to_string()),
            source_script_uri: None,
            input_mode: Some("marketing_script".to_string()),
            output_profile: Some("tiktok".to_string()),
            renderer_family: Some("product-reveal".to_string()),
            delivery_promise_json: Some(r#"{"promise_type":"motion_led"}"#.to_string()),
            music_plan_json: None,
            voice_selection_json: None,
            tool_invocations: vec![OpenMontageToolInvocation {
                invocation_id: "assets-zhichuang-1".to_string(),
                stage: "assets".to_string(),
                tool_name: "zhichuang_veo_video".to_string(),
                role: "motion_background".to_string(),
                operation: "text_to_video".to_string(),
                provider: "zhichuang".to_string(),
                capability: "video_generation".to_string(),
                input_json: r#"{"prompt":"Create a clip","duration":8}"#.to_string(),
                idempotency_key: Some("gm-openmontage-123-456-video-1".to_string()),
                max_cost_usd: Some(0.5),
                dry_run: false,
                expected_artifact_roles: vec!["scene_video".to_string()],
                contract_version: Some("0.1.0".to_string()),
                metadata_json: None,
            }],
            artifact_inputs: vec![],
            pipeline_manifest: None,
            preflight_policy: Some("provider_menu_summary".to_string()),
            openmontage_request_json: None,
            provider_slots: Default::default(),
        };

        let json = serde_json::to_string(&req).expect("serialize");
        assert!(json.contains(r#""version":"v1""#));
        assert!(json.contains(r#""kind":"reference_image""#));
        assert!(!json.contains(r#""version":1"#));
        assert!(json.contains(r#""input_mode":"marketing_script""#));

        let parsed: OpenMontageProfessionalVideoRequest =
            serde_json::from_str(&json).expect("deserialize");
        assert_eq!(parsed.version, OpenMontageProtocolVersion::V1 as i32);
        assert_eq!(
            parsed.assets[0].kind,
            OpenMontageInputAssetKind::ReferenceImage as i32
        );
    }

    #[test]
    fn test_openmontage_protocol_job_event_roundtrip() {
        let event = OpenMontageJobEvent {
            version: OpenMontageProtocolVersion::V1 as i32,
            event_id: "evt-1".to_string(),
            sequence: 7,
            job: Some(OpenMontageJobRef {
                job_id: "omx_job_1".to_string(),
                request_id: "req-1".to_string(),
                project_id: "project-1".to_string(),
                correlation_id: "corr-1".to_string(),
                idempotency_key: "idem-1".to_string(),
            }),
            event_type: OpenMontageEventType::JobCompleted as i32,
            status: OpenMontageJobStatus::Completed as i32,
            stage: "publish".to_string(),
            progress_pct: 100,
            checkpoint: None,
            approval: None,
            artifacts: vec![OpenMontageArtifact {
                artifact_id: "artifact-final-video".to_string(),
                kind: OpenMontageArtifactKind::Video as i32,
                role: "primary_video".to_string(),
                uri: "https://oss.example.com/final.mp4".to_string(),
                mime_type: Some("video/mp4".to_string()),
                width_px: Some(1080),
                height_px: Some(1920),
                duration_ms: Some(30000),
                bytes: None,
                metadata_json: None,
                artifact_name: None,
                path: None,
                source_tool: None,
                scene_id: None,
                payload_json: None,
                schema_id: None,
                validated: None,
            }],
            error: None,
            event_json: None,
            emitted_at: "2026-05-27T10:02:00Z".to_string(),
            tool_invocation: None,
            tool_result: None,
            artifact_payloads: vec![],
            checkpoint_full: None,
            preflight: None,
        };

        let json = serde_json::to_string(&event).expect("serialize");
        assert!(json.contains(r#""event_type":"job_completed""#));
        assert!(json.contains(r#""status":"completed""#));
        assert!(!json.contains(r#""event_type":4"#));
        assert!(!json.contains(r#""status":2"#));

        let parsed: OpenMontageJobEvent = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(parsed.sequence, 7);
        assert_eq!(parsed.event_type, OpenMontageEventType::JobCompleted as i32);
        assert_eq!(parsed.status, OpenMontageJobStatus::Completed as i32);
    }

    #[test]
    fn test_openmontage_protocol_error_code_roundtrip() {
        let response = OpenMontageSubmitResponse {
            version: OpenMontageProtocolVersion::V1 as i32,
            job: Some(OpenMontageJobRef {
                job_id: "omx_job_1".to_string(),
                request_id: "req-1".to_string(),
                project_id: "project-1".to_string(),
                correlation_id: "corr-1".to_string(),
                idempotency_key: "idem-1".to_string(),
            }),
            status: OpenMontageJobStatus::Failed as i32,
            accepted_at: "2026-05-27T10:04:00Z".to_string(),
            status_url: None,
            next_event_sequence: 3,
            error: Some(OpenMontageError {
                code: OpenMontageErrorCode::IdempotencyConflict as i32,
                message: "idempotency key was reused with a different request body".to_string(),
                retryable: false,
                detail_json: Some(r#"{"field":"idempotency_key"}"#.to_string()),
            }),
        };

        let json = serde_json::to_string(&response).expect("serialize");
        assert!(json.contains(r#""status":"failed""#));
        assert!(json.contains(r#""code":"idempotency_conflict""#));

        let parsed: OpenMontageSubmitResponse =
            serde_json::from_str(&json).expect("deserialize");
        assert_eq!(parsed.status, OpenMontageJobStatus::Failed as i32);
        assert_eq!(
            parsed.error.expect("error").code,
            OpenMontageErrorCode::IdempotencyConflict as i32
        );
    }

    #[test]
    fn test_openmontage_protocol_internal_api_contract_roundtrip() {
        let contract = OpenMontageToolContract {
            name: "video_compose".to_string(),
            version: "0.1.0".to_string(),
            tier: "core".to_string(),
            capability: "video_post".to_string(),
            provider: "ffmpeg".to_string(),
            stability: "experimental".to_string(),
            status: "available".to_string(),
            execution_mode: "sync".to_string(),
            determinism: "deterministic".to_string(),
            runtime: "local".to_string(),
            module_path: "tools.video.video_compose".to_string(),
            usage_location: "tools/video/video_compose.py".to_string(),
            dependencies: vec!["cmd:ffmpeg".to_string()],
            install_instructions: "Install FFmpeg".to_string(),
            capabilities: vec!["compose_cuts".to_string()],
            input_fields: vec![OpenMontageSchemaField {
                path: "operation".to_string(),
                required: true,
                json_type: "string".to_string(),
                enum_values: vec!["render".to_string(), "compose".to_string()],
                default_json: None,
                description: Some("OpenMontage tool operation".to_string()),
            }],
            output_fields: vec![OpenMontageSchemaField {
                path: "output".to_string(),
                required: false,
                json_type: "string".to_string(),
                enum_values: vec![],
                default_json: None,
                description: None,
            }],
            input_schema_json: Some(r#"{"properties":{"operation":{}}}"#.to_string()),
            output_schema_json: Some(r#"{"properties":{"output":{}}}"#.to_string()),
            artifact_schema_json: Some(r#"{"type":"array"}"#.to_string()),
            progress_schema_json: None,
            supports_json: Some(r#"{}"#.to_string()),
            best_for: vec!["final render".to_string()],
            not_good_for: vec![],
            provider_matrix_json: Some(r#"{}"#.to_string()),
            resource_profile: Some(OpenMontageResourceProfile {
                cpu_cores: 4,
                ram_mb: 2048,
                vram_mb: 0,
                disk_mb: 5000,
                network_required: false,
            }),
            retry_policy: Some(OpenMontageRetryPolicy {
                max_retries: 1,
                backoff_seconds: 1.0,
                retryable_errors: vec!["Conversion failed".to_string()],
            }),
            resume_support: "from_start".to_string(),
            side_effects: vec!["writes video file to output_path".to_string()],
            fallback: None,
            fallback_tools: vec![],
            agent_skills: vec!["remotion".to_string()],
            user_visible_verification: vec!["play the output".to_string()],
            quality_score: None,
            historical_success_rate: None,
            latency_p50_seconds: None,
            render_engines_json: Some(r#"{"ffmpeg":true,"remotion":true}"#.to_string()),
            render_runtimes_json: None,
            remotion_note: Some("Remotion available".to_string()),
            hyperframes_note: None,
            runtime_governance: Some("No silent runtime swaps".to_string()),
            raw_info_json: Some(r#"{"name":"video_compose"}"#.to_string()),
            related_skills: vec!["remotion".to_string()],
        };
        let manifest = OpenMontagePipelineManifest {
            name: "glancemind-marketing-video".to_string(),
            version: "0.1".to_string(),
            description: Some("GlanceMind marketing-video workflow".to_string()),
            category: Some("generated".to_string()),
            stability: Some("beta".to_string()),
            compatible_playbooks: vec![],
            compatible_playbooks_json: Some(r#"{"recommended":["clean-professional"]}"#.to_string()),
            required_skills: vec!["pipelines/glancemind-marketing-video/executive-producer".to_string()],
            stages: vec![OpenMontagePipelineStage {
                name: "proposal".to_string(),
                agent: None,
                skill: Some("pipelines/glancemind-marketing-video/proposal-director".to_string()),
                required_artifacts_in: vec![],
                optional_artifacts_in: vec![],
                produces: vec!["proposal_packet".to_string()],
                preferred_tools: vec![],
                fallback_tools: vec![],
                required_tools: vec![],
                optional_tools: vec![],
                tools_available: vec!["video_compose".to_string()],
                review_focus: vec!["runtime selection".to_string()],
                checkpoint_required: Some(true),
                human_approval_default: Some(true),
                success_criteria: vec!["schema-valid proposal".to_string()],
                sub_stages: vec![],
                metadata_json: None,
            }],
            default_checkpoint_policy: Some("guided".to_string()),
            reference_input: None,
            orchestration: Some(OpenMontagePipelineOrchestration {
                mode: Some("executive-producer".to_string()),
                skill: Some("pipelines/glancemind-marketing-video/executive-producer".to_string()),
                budget_default_usd: Some(3.0),
                max_revisions_per_stage: Some(3),
                max_send_backs: Some(3),
                max_wall_time_minutes: Some(25),
            }),
            extensions: Some(OpenMontageExtensionPermissions {
                custom_scripts: Some(true),
                custom_playbooks: Some(true),
                custom_skills: Some(true),
                custom_tools: Some(false),
            }),
            metadata_json: None,
            raw_manifest_json: Some(r#"{"name":"glancemind-marketing-video"}"#.to_string()),
        };
        let snapshot = OpenMontageJobSnapshot {
            version: OpenMontageProtocolVersion::V1 as i32,
            job: Some(OpenMontageJobRef {
                job_id: "omx_job_1".to_string(),
                request_id: "req-1".to_string(),
                project_id: "project-1".to_string(),
                correlation_id: "corr-1".to_string(),
                idempotency_key: "idem-1".to_string(),
            }),
            status: OpenMontageJobStatus::AwaitingHuman as i32,
            pipeline: "glancemind-marketing-video".to_string(),
            current_stage: "proposal".to_string(),
            progress_pct: 20,
            checkpoints: vec![],
            decisions: vec![],
            approvals: vec![],
            artifacts: vec![],
            error: None,
            metrics_json: None,
            updated_at: "2026-05-27T09:01:00Z".to_string(),
            preflight: Some(OpenMontagePreflightSnapshot {
                composition_runtimes: vec![OpenMontageRuntimeAvailability {
                    name: "ffmpeg".to_string(),
                    available: true,
                    note: None,
                    warnings: vec![],
                }],
                capabilities: vec![OpenMontageCapabilitySummary {
                    capability: "video_generation".to_string(),
                    configured: 1,
                    total: 3,
                    available_providers: vec!["zhichuang".to_string()],
                    unavailable_providers: vec!["seedance".to_string()],
                }],
                setup_offers: vec![OpenMontageSetupOffer {
                    capability: "video_generation".to_string(),
                    tool: "seedance_video".to_string(),
                    provider: "seedance".to_string(),
                    install_instructions: "Set SEEDANCE_API_KEY".to_string(),
                }],
                runtime_warnings: vec![],
                tools: vec![contract],
                pipelines: vec![manifest],
                captured_at: "2026-05-27T09:00:00Z".to_string(),
                provider_menu_summary_json: None,
                provider_menu_json: None,
                support_envelope_json: None,
            }),
            pipeline_manifest: None,
            artifact_payloads: vec![OpenMontageArtifactPayload {
                artifact_name: "proposal_packet".to_string(),
                schema_id: Some("openmontage/artifacts/proposal_packet".to_string()),
                schema_version: Some("1.0".to_string()),
                payload_json: r#"{"version":"1.0"}"#.to_string(),
                validated: true,
                schema_fields: vec![OpenMontageSchemaField {
                    path: "production_plan.render_runtime".to_string(),
                    required: true,
                    json_type: "string".to_string(),
                    enum_values: vec!["remotion".to_string(), "hyperframes".to_string()],
                    default_json: None,
                    description: None,
                }],
                validation_error: None,
                uri: None,
                role: None,
                metadata_json: None,
            }],
            tool_results: vec![],
            full_checkpoints: vec![],
        };

        let json = serde_json::to_string(&snapshot).expect("serialize");
        assert!(json.contains(r#""status":"awaiting_human""#));
        assert!(json.contains(r#""input_schema_json""#));

        let parsed: OpenMontageJobSnapshot = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(parsed.status, OpenMontageJobStatus::AwaitingHuman as i32);
        assert_eq!(
            parsed
                .preflight
                .expect("preflight")
                .tools
                .first()
                .expect("tool")
                .name,
            "video_compose"
        );
    }
}
