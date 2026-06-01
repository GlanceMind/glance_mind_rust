//! Deterministic `/audientry` input parsing.
//!
//! The slash command is the deterministic entry point (the LLM `audientry` tool
//! is only a natural-language fallback). This module turns a chat message — or a
//! resumed questionnaire submission — into either a ready [`ProductBrief`] or a
//! follow-up [`QuestionnairePayload`] asking for the missing fields.

use std::collections::BTreeMap;

use serde_json::Value;

use crate::dto::audientry_dto::ProductBrief;
use crate::service::ai_chat::types::{
    QuestionnaireControl, QuestionnaireField, QuestionnairePayload, QuestionnaireSubmission,
};

/// Returns true when a chat message is the deterministic `/audientry` command
/// (leading whitespace tolerated). A message merely *mentioning* audientry
/// (e.g. "tell me about audientry") is not intercepted.
pub fn is_audientry_command(content: &str) -> bool {
    content.trim_start().starts_with("/audientry")
}

/// Field keys recognised on the `audientry` LLM tool call. Mirrors the tool's
/// JSON-schema `properties` in `tool_registry.rs`.
const AUDIENTRY_TOOL_ARG_KEYS: [&str; 4] =
    ["product_name", "description", "landing_page_url", "locale"];

/// Adapt an `audientry` LLM tool call's JSON arguments into a synthetic
/// [`QuestionnaireSubmission`] with `intent == "audientry"`.
///
/// The natural-language tool is only advertised to the LLM as a fallback; its
/// execution is *not* wired into the generic tool dispatcher (that path returns
/// "Unknown tool: audientry"). Instead we funnel the call through the same
/// deterministic entry point as the `/audientry` slash command by reusing
/// [`parse_audientry_input`]'s resolution-order #1 (questionnaire answers). Any
/// string-valued arg key the tool recognises is copied verbatim into `answers`;
/// missing required fields naturally fall through to the follow-up questionnaire.
pub fn audientry_submission_from_tool_args(params: &Value) -> QuestionnaireSubmission {
    let mut answers: BTreeMap<String, Value> = BTreeMap::new();
    for key in AUDIENTRY_TOOL_ARG_KEYS {
        if let Some(v) = params.get(key) {
            if let Some(s) = v.as_str() {
                if !s.trim().is_empty() {
                    answers.insert(key.to_string(), Value::String(s.to_string()));
                }
            }
        }
    }
    QuestionnaireSubmission {
        questionnaire_id: Some("audientry_brief".into()),
        intent: "audientry".into(),
        answers,
        auto_filled: BTreeMap::new(),
        display_message: None,
    }
}

/// Outcome of parsing audientry input.
pub enum AudientryParse {
    /// Enough info to enqueue the analysis.
    Ready(ProductBrief),
    /// Missing required fields — ask the user via a structured questionnaire.
    NeedInput(QuestionnairePayload),
}

/// Parse `/audientry` input into a [`ProductBrief`] or a follow-up questionnaire.
///
/// Resolution order:
/// 1. A questionnaire submission with `intent == "audientry"` takes precedence
///    (the resume path after the user filled the follow-up form).
/// 2. Otherwise the free-text body after the command is parsed: first line is the
///    product name, the remainder is the description.
/// 3. If neither yields a name + description, a questionnaire is requested.
pub fn parse_audientry_input(
    content: &str,
    sub: Option<&QuestionnaireSubmission>,
) -> AudientryParse {
    // 1) questionnaire answers take precedence (resume path)
    if let Some(s) = sub {
        if s.intent == "audientry" {
            let name = s
                .answers
                .get("product_name")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .trim()
                .to_string();
            let desc = s
                .answers
                .get("description")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .trim()
                .to_string();
            if !name.is_empty() && !desc.is_empty() {
                let locale = s
                    .answers
                    .get("locale")
                    .and_then(|v| v.as_str())
                    .filter(|s| !s.trim().is_empty())
                    .unwrap_or("en-US")
                    .to_string();
                let lp = s
                    .answers
                    .get("landing_page_url")
                    .and_then(|v| v.as_str())
                    .map(|s| s.trim())
                    .filter(|s| !s.is_empty())
                    .map(|s| s.to_string());
                return AudientryParse::Ready(ProductBrief {
                    contract_version: default_contract_version(),
                    name,
                    description: desc,
                    landing_page_url: lp,
                    locale,
                });
            }
        }
    }

    // 2) parse the free-text body after the command
    let body = content
        .trim_start()
        .strip_prefix("/audientry")
        .unwrap_or(content)
        .trim();
    let mut lines = body.splitn(2, '\n');
    let name = lines.next().unwrap_or("").trim().to_string();
    let desc = lines.next().unwrap_or("").trim().to_string();
    if !name.is_empty() && !desc.is_empty() {
        return AudientryParse::Ready(ProductBrief {
            contract_version: default_contract_version(),
            name,
            description: desc,
            landing_page_url: None,
            locale: "en-US".into(),
        });
    }

    // 3) not enough info — ask for it
    AudientryParse::NeedInput(need_input_questionnaire())
}

fn default_contract_version() -> String {
    "2026-04-29".into()
}

/// A single-line text field. `QuestionnaireControl` has no dedicated `Text`
/// variant; `Input` is the single-line text control (see `types.rs`).
fn text_field(key: &str, label: &str, required: bool) -> QuestionnaireField {
    QuestionnaireField {
        key: key.into(),
        label: label.into(),
        control: QuestionnaireControl::Input,
        required,
        options: vec![],
        placeholder: None,
        helper_text: None,
        default_value: None,
        max_length: None,
    }
}

fn need_input_questionnaire() -> QuestionnairePayload {
    QuestionnairePayload {
        questionnaire_id: "audientry_brief".into(),
        intent: "audientry".into(),
        title: "受众分析 — 补充产品信息".into(),
        description: Some("请提供产品名称与简介，以便运行受众入场分析。".into()),
        submit_label: "开始分析".into(),
        fields: vec![
            text_field("product_name", "产品名称", true),
            text_field("description", "产品简介", true),
            text_field("landing_page_url", "落地页(可选)", false),
            text_field("locale", "市场/语言 (默认 en-US)", false),
        ],
        auto_filled: vec![],
    }
}

#[cfg(test)]
fn make_submission(
    intent: &str,
    answers: std::collections::BTreeMap<String, serde_json::Value>,
) -> QuestionnaireSubmission {
    QuestionnaireSubmission {
        questionnaire_id: Some("audientry_brief".into()),
        intent: intent.into(),
        answers,
        auto_filled: std::collections::BTreeMap::new(),
        display_message: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn complete_body_yields_ready() {
        let p = parse_audientry_input(
            "/audientry Sleep Tea\nHerbal tea for faster sleep onset",
            None,
        );
        match p {
            AudientryParse::Ready(b) => {
                assert_eq!(b.name, "Sleep Tea");
                assert!(b.description.to_lowercase().contains("herbal"));
            }
            _ => panic!("expected Ready"),
        }
    }

    #[test]
    fn empty_body_requests_questionnaire() {
        let p = parse_audientry_input("/audientry", None);
        assert!(matches!(p, AudientryParse::NeedInput(_)));
    }

    #[test]
    fn questionnaire_answers_complete_the_brief() {
        use std::collections::BTreeMap;
        let mut answers = BTreeMap::new();
        answers.insert("product_name".to_string(), serde_json::json!("Sleep Tea"));
        answers.insert(
            "description".to_string(),
            serde_json::json!("herbal sleep tea"),
        );
        let sub = make_submission("audientry", answers);
        let p = parse_audientry_input("/audientry", Some(&sub));
        assert!(matches!(p, AudientryParse::Ready(_)));
    }

    #[test]
    fn slash_command_is_recognized_before_llm() {
        assert!(is_audientry_command("/audientry Sleep Tea\nherbal tea"));
        assert!(is_audientry_command("  /audientry"));
        assert!(!is_audientry_command("tell me about audientry"));
    }

    #[test]
    fn tool_args_complete_brief_via_submission() {
        // The NL `audientry` tool call (advertised as a fallback) must route
        // through the same deterministic path as `/audientry`. Its args become a
        // synthetic submission that parse resolution-order #1 turns into a brief,
        // preserving the optional landing_page_url and locale fields.
        let params = serde_json::json!({
            "product_name": "Sleep Tea",
            "description": "herbal tea for faster sleep onset",
            "landing_page_url": "https://example.com/sleep-tea",
            "locale": "zh-CN"
        });
        let sub = audientry_submission_from_tool_args(&params);
        assert_eq!(sub.intent, "audientry");
        match parse_audientry_input("", Some(&sub)) {
            AudientryParse::Ready(b) => {
                assert_eq!(b.name, "Sleep Tea");
                assert!(b.description.to_lowercase().contains("herbal"));
                assert_eq!(
                    b.landing_page_url.as_deref(),
                    Some("https://example.com/sleep-tea")
                );
                assert_eq!(b.locale, "zh-CN");
            }
            _ => panic!("expected Ready brief from complete tool args"),
        }
    }

    #[test]
    fn tool_args_missing_description_requests_questionnaire() {
        // Required fields absent → graceful follow-up questionnaire, never an
        // "Unknown tool" error.
        let params = serde_json::json!({ "product_name": "Sleep Tea" });
        let sub = audientry_submission_from_tool_args(&params);
        assert!(matches!(
            parse_audientry_input("", Some(&sub)),
            AudientryParse::NeedInput(_)
        ));
    }

    #[test]
    fn tool_args_ignore_blank_optional_fields() {
        // A blank landing_page_url must not become Some("") on the brief.
        let params = serde_json::json!({
            "product_name": "Sleep Tea",
            "description": "herbal sleep tea",
            "landing_page_url": "   "
        });
        let sub = audientry_submission_from_tool_args(&params);
        match parse_audientry_input("", Some(&sub)) {
            AudientryParse::Ready(b) => {
                assert_eq!(b.landing_page_url, None);
                assert_eq!(b.locale, "en-US");
            }
            _ => panic!("expected Ready brief"),
        }
    }
}
