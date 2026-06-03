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
//!
//! NOTE (Module A skeleton): the method bodies below are deliberately-WRONG
//! compiling stubs. The TEST-AUTHOR owns the assertions; a different engineer
//! implements the real logic to make the RED tests GREEN.

use super::completeness::DraftConfig;

/// Which kind of task config we are validating.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskKind {
    /// Social-monitor campaign (`CampaignCreateDto`).
    Campaign,
    /// AI publish plan (`CreatePlanDto`).
    PublishPlan,
}

/// How important a field is to a complete config.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
    pub fn for_kind(kind: TaskKind, _draft: &DraftConfig) -> Self {
        // WRONG STUB: no fields. The implementer fills these per the contract.
        TaskConfigSpec {
            kind,
            fields: Vec::new(),
        }
    }

    /// Keys whose [`Importance`] is `Required` or `Recommended` (the "expected" set).
    pub fn expected_keys(&self) -> Vec<&'static str> {
        // WRONG STUB.
        Vec::new()
    }

    /// Keys whose [`Importance`] is `Required`.
    pub fn required_keys(&self) -> Vec<&'static str> {
        // WRONG STUB.
        Vec::new()
    }
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
}
