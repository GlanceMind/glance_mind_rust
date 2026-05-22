//! Deterministic `/audientry` input parsing.
//!
//! The slash command is the deterministic entry point (the LLM `audientry` tool
//! is only a natural-language fallback). This module turns a chat message — or a
//! resumed questionnaire submission — into either a ready [`ProductBrief`] or a
//! follow-up [`QuestionnairePayload`] asking for the missing fields.

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
}
