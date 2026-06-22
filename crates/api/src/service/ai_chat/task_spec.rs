//! Declarative spec of the fields that make a task config "complete".
//!
//! A [`TaskConfigSpec`] enumerates the fields the assistant expects for a given
//! [`TaskKind`], each tagged with an [`Importance`] (Required / Recommended /
//! Optional) and a [`FieldKind`]. The completeness gate (see
//! [`super::completeness`]) consumes this spec to decide what is missing.
//!
//! Conditional-required fields (e.g. publish-plan fields gated by `plan_type`)
//! are resolved at spec-construction time via [`TaskConfigSpec::for_kind`], which
//! inspects the draft.

use super::completeness::DraftConfig;
use serde::{Deserialize, Serialize};

/// Which kind of task config we are validating.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskKind {
    /// Social-monitor campaign (`CampaignCreateDto`).
    Campaign,
    /// AI publish plan (`CreatePlanDto`).
    PublishPlan,
}

/// How important a field is to a complete config.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Importance {
    /// Must be present; absence blocks submission.
    Required,
    /// Strongly suggested; absence counts toward the template-offer threshold.
    Recommended,
    /// Not counted toward completeness at all.
    Optional,
}

/// The wire/value shape of a field. Serialized later as the wire `type`; kept
/// simple here.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FieldKind {
    String,
    Int,
    Enum,
    Bool,
    Json,
}

/// Spec for a single expected field.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FieldSpec {
    pub key: &'static str,
    pub label_cn: &'static str,
    pub group: &'static str,
    pub importance: Importance,
    pub kind: FieldKind,
}

/// The full set of expected fields for a task kind (after conditional resolution).
#[derive(Debug, Clone)]
pub struct TaskConfigSpec {
    pub kind: TaskKind,
    pub fields: Vec<FieldSpec>,
}

impl TaskConfigSpec {
    /// Build the spec for `kind`, resolving conditional-required fields from the
    /// draft (e.g. publish-plan `plan_type`).
    pub fn for_kind(kind: TaskKind, draft: &DraftConfig) -> Self {
        let fields = match kind {
            TaskKind::Campaign => campaign_fields(),
            TaskKind::PublishPlan => publish_plan_fields(draft),
        };
        TaskConfigSpec { kind, fields }
    }

    /// Keys whose [`Importance`] is `Required` or `Recommended` (the "expected" set).
    pub fn expected_keys(&self) -> Vec<&'static str> {
        self.fields
            .iter()
            .filter(|f| matches!(f.importance, Importance::Required | Importance::Recommended))
            .map(|f| f.key)
            .collect()
    }

    /// Keys whose [`Importance`] is `Required`.
    pub fn required_keys(&self) -> Vec<&'static str> {
        self.fields
            .iter()
            .filter(|f| matches!(f.importance, Importance::Required))
            .map(|f| f.key)
            .collect()
    }
}

/// Convenience constructor for a `FieldSpec`.
const fn field(
    key: &'static str,
    label_cn: &'static str,
    group: &'static str,
    importance: Importance,
    kind: FieldKind,
) -> FieldSpec {
    FieldSpec {
        key,
        label_cn,
        group,
        importance,
        kind,
    }
}

/// The campaign (`CampaignCreateDto`) field spec.
///
/// 7 Required, 6 Recommended, plus the documented Optional fields (which are
/// enumerated so the spec is a faithful description of the config, but are
/// excluded from the "expected" set by [`TaskConfigSpec::expected_keys`]).
fn campaign_fields() -> Vec<FieldSpec> {
    use FieldKind::*;
    use Importance::*;
    vec![
        // Required (7)
        field("name", "活动名称", "basic", Required, String),
        field("platform_id", "平台", "basic", Required, Int),
        field("region_id", "地区", "basic", Required, Int),
        field("ai_model_id", "AI 模型", "ai", Required, Int),
        field("schedule_type", "调度类型", "schedule", Required, Enum),
        field("product_prompt", "产品提示词", "ai", Required, String),
        field("max_scan_count", "最大扫描数", "schedule", Required, Int),
        // Recommended (6)
        field("target_audience", "目标受众", "ai", Recommended, String),
        field("keyword", "关键词", "targeting", Recommended, String),
        field("call_to_action", "行动号召", "ai", Recommended, String),
        field("tone_of_voice", "语气风格", "ai", Recommended, String),
        field("budget_cap", "预算上限", "budget", Recommended, Int),
        field("reply_template_ids", "回复模板", "reply", Recommended, Json),
        // Optional (uncounted)
        field("enable_ai_refactor", "启用 AI 重构", "ai", Optional, Bool),
        field("persona_id", "人设", "ai", Optional, Int),
        field("end_date", "结束日期", "schedule", Optional, String),
        field("schedule_config", "调度配置", "schedule", Optional, Json),
        field("additional_info", "补充信息", "ai", Optional, String),
        field("social_group_id", "社交群组", "targeting", Optional, Int),
        field("auto_like", "自动点赞", "behavior", Optional, Bool),
        field("auto_follow", "自动关注", "behavior", Optional, Bool),
        field("auto_dm", "自动私信", "behavior", Optional, Bool),
        field(
            "auto_reply_comments",
            "自动回复评论",
            "behavior",
            Optional,
            Bool,
        ),
        field(
            "auto_reply_post",
            "自动回复帖子",
            "behavior",
            Optional,
            Bool,
        ),
        field("search_options", "搜索选项", "targeting", Optional, Json),
    ]
}

/// The AI publish-plan (`CreatePlanDto`) field spec, resolving conditional
/// Required fields from the draft's `plan_type` (default `batch_text`).
fn publish_plan_fields(draft: &DraftConfig) -> Vec<FieldSpec> {
    use FieldKind::*;
    use Importance::*;

    let plan_type = draft
        .get("plan_type")
        .and_then(|v| v.as_str())
        .unwrap_or("batch_text");

    let mut fields = vec![
        // Always Required (2)
        field("platform_id", "平台", "basic", Required, Int),
        field("content_type", "内容类型", "basic", Required, Enum),
    ];

    // Conditional Required, gated by plan_type.
    let needs_group_id = matches!(
        plan_type,
        "batch_text" | "account_grooming" | "reddit_text" | "reddit_image" | "reddit_link"
    );
    if needs_group_id {
        fields.push(field("group_id", "群组", "targeting", Required, Int));
    }

    let needs_social_account =
        matches!(plan_type, "single_video" | "direct_publish" | "page_manage");
    if needs_social_account {
        fields.push(field(
            "social_account_id",
            "社交账号",
            "basic",
            Required,
            Int,
        ));
    }

    if plan_type == "single_video" {
        fields.push(field(
            "video_ai_model_id",
            "视频 AI 模型",
            "ai",
            Required,
            Int,
        ));
    }

    if plan_type == "direct_publish" {
        fields.push(field("content", "内容", "content", Required, String));
    }

    // Recommended (5)
    fields.push(field("name", "计划名称", "basic", Recommended, String));
    fields.push(field("plan_type", "计划类型", "basic", Recommended, Enum));
    fields.push(field(
        "ai_input.content_prompt",
        "内容提示词",
        "ai",
        Recommended,
        String,
    ));
    fields.push(field("schedule", "调度", "schedule", Recommended, Json));
    fields.push(field("behavior", "行为", "behavior", Recommended, Json));

    fields
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn draft(v: serde_json::Value) -> DraftConfig {
        DraftConfig::from_value(v)
    }

    fn sorted(mut v: Vec<&'static str>) -> Vec<&'static str> {
        v.sort_unstable();
        v
    }

    /// The campaign required set is exactly the 7 documented keys (order-insensitive).
    #[test]
    fn campaign_required_set_is_exact() {
        // plan_type is irrelevant for campaign; pass an empty draft.
        let spec = TaskConfigSpec::for_kind(TaskKind::Campaign, &draft(json!({})));

        let expected = sorted(vec![
            "name",
            "platform_id",
            "region_id",
            "ai_model_id",
            "schedule_type",
            "product_prompt",
            "max_scan_count",
        ]);

        assert_eq!(
            sorted(spec.required_keys()),
            expected,
            "campaign required set must be exactly the 7 documented keys"
        );
    }

    /// For aipub with `plan_type == single_video`, the required set resolves to
    /// {platform_id, content_type, social_account_id, video_ai_model_id}.
    #[test]
    fn aipub_required_resolves_single_video() {
        let spec = TaskConfigSpec::for_kind(
            TaskKind::PublishPlan,
            &draft(json!({ "plan_type": "single_video" })),
        );

        let expected = sorted(vec![
            "platform_id",
            "content_type",
            "social_account_id",
            "video_ai_model_id",
        ]);

        assert_eq!(
            sorted(spec.required_keys()),
            expected,
            "single_video plan required set must resolve to the 4 documented keys"
        );
    }

    /// For aipub with `plan_type == page_manage`, the required set resolves to
    /// {platform_id, content_type, social_account_id}. page_manage operates a
    /// single page (single account), so it needs social_account_id — NOT
    /// group_id — and carries no video model.
    #[test]
    fn aipub_required_resolves_page_manage() {
        let spec = TaskConfigSpec::for_kind(
            TaskKind::PublishPlan,
            &draft(json!({ "plan_type": "page_manage" })),
        );

        let expected = sorted(vec!["platform_id", "content_type", "social_account_id"]);

        assert_eq!(
            sorted(spec.required_keys()),
            expected,
            "page_manage plan required set must resolve to the 3 documented keys"
        );
    }
}
