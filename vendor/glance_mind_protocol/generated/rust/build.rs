use std::io::Result;

fn main() -> Result<()> {
    let mut config = prost_build::Config::new();

    // Apply serde derives to MESSAGE types only.
    // Enums are excluded because lib.rs has manual `impl Serialize/Deserialize`
    // for Platform / DataType / TimeRange / CommentStatus that emit
    // lowercase string variants ("tiktok") instead of prost's default
    // SCREAMING_SNAKE_CASE int repr.
    let messages = [
        // crawler_task.proto
        "CrawlerTask",
        "CrawlerTaskMeta",
        "CrawlerTaskSpec",
        "TaskConfig",
        "TaskFilters",
        // device_comments.proto
        "DeviceCommentsQuery",
        "DeviceCommentsResponse",
        "CampaignConfig",
        "CommentData",
        "Pagination",
        "UpdateCommentStatusRequest",
        "UpdateCommentStatusResponse",
        // dm.proto
        "DmMessage",
        "ConversationMeta",
        "DmEvent",
        "ReplyCommand",
        "DeviceHeartbeat",
        // patrol.proto
        "AccountStats",
        "AccountPatrolStats",
        "PatrolReport",
        "PatrolConfig",
        // aipub.proto v1
        "AiPubImageConfig",
        "ReferenceVideoConfig",
        "VideoGenerationConfig",
        "ViduVideoConfig",
        "SeedanceVideoConfig",
        "AiTaskInput",
        "AiTaskResult",
        "ContentVariation",
        "AiPubTaskContent",
        "RedditPostConfig",
        "AiPubInput",
        "UploadTaskMessage",
        "UploadTaskMeta",
        "RedditPublishContent",
        "ExecutorPublishTask",
        "AccountGroomingTaskContent",
        "ExecutorTaskStatusUpdate",
        // aipub.proto v2
        "MediaItem",
        "TextBlock",
        "LinkItem",
        "EntityTag",
        "UserMention",
        "PublishBehavior",
        "PublishSchedule",
        "PostPublishAction",
        "UnifiedPublishContent",
        "MediaPublishResult",
        "PostPublishActionResult",
        "PublishMetrics",
        "UnifiedPublishResult",
        "TextGenerationSpec",
        "ImageGenerationSpec",
        "VideoGenerationSpec",
        "UnifiedAiPubInput",
        "AccountMediaOverride",
    ];
    for m in messages {
        config.type_attribute(
            format!(".glance_mind.{}", m),
            "#[derive(serde::Serialize, serde::Deserialize)]",
        );
    }

    // Lenient deserialization (#[serde(default)] at struct level) lets
    // missing JSON fields fall back to Default::default() — needed for
    // backward compat with upstream JSON that omits default-valued fields.
    //
    // CAREFUL: prost-build type_attribute uses suffix matching — so a path
    // `.glance_mind.AiPubInput` also matches `.glance_mind.AiPubInput.PlatformConfig`
    // (the nested oneof enum). Applying #[serde(default)] to a prost Oneof
    // enum fails to compile (no Default impl). Messages containing oneof
    // fields are EXCLUDED here; their fields get per-field #[serde(default)]
    // via lenient_fields below instead.
    let lenient_messages = [
        "AiPubImageConfig",
        "ReferenceVideoConfig",
        "VideoGenerationConfig",
        "ViduVideoConfig",
        "SeedanceVideoConfig",
        "AiTaskInput",
        "AiTaskResult",
        "ContentVariation",
        "AiPubTaskContent",
        "RedditPostConfig",
        "RedditPublishContent",
        "AccountGroomingTaskContent",
        "ExecutorTaskStatusUpdate",
        // NOTE: AiPubInput / UnifiedAiPubInput excluded — they have oneofs
    ];
    for m in lenient_messages {
        config.type_attribute(format!(".glance_mind.{}", m), "#[serde(default)]");
    }

    // Per-field #[serde(default)] for fields on oneof-containing messages
    // (where struct-level default is unsafe). All non-optional AiPubInput
    // fields need this since upstream JSON often omits empty ones.
    let lenient_fields = [
        // AiPubInput: non-optional required-at-proto-level fields.
        ".glance_mind.AiPubInput.video_prompt",
        ".glance_mind.AiPubInput.content_prompt",
        ".glance_mind.AiPubInput.prompt",
        ".glance_mind.AiPubInput.account_images",
        ".glance_mind.AiPubInput.reference_images",
        // UnifiedAiPubInput: same treatment for v2 input (also has oneof).
        ".glance_mind.UnifiedAiPubInput.version",
        ".glance_mind.UnifiedAiPubInput.text_generations",
        ".glance_mind.UnifiedAiPubInput.image_generations",
        ".glance_mind.UnifiedAiPubInput.video_generations",
        ".glance_mind.UnifiedAiPubInput.initial_media",
        ".glance_mind.UnifiedAiPubInput.account_media",
        ".glance_mind.UnifiedAiPubInput.generation_extras",
    ];
    for field in lenient_fields {
        config.field_attribute(field, "#[serde(default)]");
    }

    // Backward-compat: JSON wire key "description" aliases to field "text_content".
    // Preserved from the deleted lib_inline.rs hand-mirrored struct.
    // Note: for ContentVariation / AiPubTaskContent, struct-level serde(default)
    // is already applied via lenient_messages, so only alias is needed here.
    config.field_attribute(
        ".glance_mind.ContentVariation.text_content",
        "#[serde(alias = \"description\")]",
    );
    config.field_attribute(
        ".glance_mind.AiPubTaskContent.text_content",
        "#[serde(alias = \"description\")]",
    );

    // Enum int fields that must serialize as lowercase string on the wire
    // (production consumers rely on "platform": "facebook" not 3).
    // Adapters live in src/lib.rs::serde_helpers.
    // TaskFilters.time_range is Option<i32> and needs an Option-aware
    // adapter — TODO follow-up; for now it serializes as int-or-null.
    let string_enum_fields = [
        (".glance_mind.CrawlerTaskSpec.platform", "platform"),
        (".glance_mind.CrawlerTaskSpec.data_type", "data_type"),
        (".glance_mind.CommentData.platform", "platform"),
        (".glance_mind.CommentData.status", "comment_status"),
        (
            ".glance_mind.UpdateCommentStatusRequest.status",
            "comment_status",
        ),
    ];
    for (path, module) in string_enum_fields {
        config.field_attribute(
            path,
            format!(
                "#[serde(serialize_with = \"crate::serde_helpers::{}::serialize\", \
                  deserialize_with = \"crate::serde_helpers::{}::deserialize\")]",
                module, module
            ),
        );
    }

    // Generate code from proto files. v2 cutover adds aipub.proto +
    // patrol.proto to the prost pipeline (previously hand-mirrored in
    // the now-deleted lib_inline.rs).
    config.compile_protos(
        &[
            "../../proto/common.proto",
            "../../proto/crawler_task.proto",
            "../../proto/device_comments.proto",
            "../../proto/dm.proto",
            "../../proto/aipub.proto",
            "../../proto/patrol.proto",
        ],
        &["../../proto/"],
    )?;

    Ok(())
}
