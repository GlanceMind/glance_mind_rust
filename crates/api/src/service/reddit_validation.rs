//! Validation for Reddit plan `ai_input` shape.
//!
//! Reddit plans (plan_type in {reddit_text, reddit_image, reddit_link})
//! require `ai_input.reddit_config` with a well-formed
//! `RedditPostConfig` plus a non-empty `content_prompt`.
//!
//! Previously this validation was inline in `aipub_service::create_plan`
//! using dynamic JSON indexing (`ai_input["reddit_config"]["subreddit"]`
//! etc.). This module switches to typed access via
//! `glance_mind_protocol::glance_mind::RedditPostConfig`, which gives us:
//!   - Compile-time field checking (typos on `subreddit` / `link_url`
//!     caught at build time)
//!   - Single source of truth (proto schema owns the field list)
//!   - No ambiguity about optional vs required fields

use crate::error::api_error::ApiError;
use crate::error::business_error::BusinessError;
use glance_mind_db::entity::aipub::PlanType;
use glance_mind_protocol::glance_mind::RedditPostConfig;
use serde_json::Value as JsonValue;

/// Validates that `ai_input` carries a valid Reddit config for the given
/// plan type. Returns `Ok(())` on success; `BusinessError::InvalidInput`
/// otherwise.
pub fn validate(plan_type: PlanType, ai_input: Option<&JsonValue>) -> Result<(), ApiError> {
    // All Reddit plans require ai_input + content_prompt + reddit_config.
    let ai_input = ai_input.ok_or_else(|| {
        invalid(format!(
            "{} plan requires ai_input with reddit_config",
            plan_type.as_str()
        ))
    })?;

    // content_prompt is top-level, required for all Reddit types.
    let content_prompt = ai_input
        .get("content_prompt")
        .and_then(JsonValue::as_str)
        .unwrap_or_default();
    if content_prompt.is_empty() {
        return Err(invalid(
            "Reddit plan requires ai_input.content_prompt".to_string(),
        ));
    }

    // reddit_config typed parse. Missing field = null in serde_json,
    // which fails deserialization.
    let reddit_config_json = ai_input.get("reddit_config").ok_or_else(|| {
        invalid(format!(
            "{} plan requires ai_input.reddit_config",
            plan_type.as_str()
        ))
    })?;
    if reddit_config_json.is_null() {
        return Err(invalid(format!(
            "{} plan requires ai_input.reddit_config",
            plan_type.as_str()
        )));
    }
    let reddit_config: RedditPostConfig = serde_json::from_value(reddit_config_json.clone())
        .map_err(|e| invalid(format!("ai_input.reddit_config is malformed: {}", e)))?;

    // subreddit required for all Reddit types.
    if reddit_config.subreddit.is_empty() {
        return Err(invalid(
            "Reddit plan requires reddit_config.subreddit".to_string(),
        ));
    }

    // reddit_link requires link_url.
    if plan_type == PlanType::RedditLink {
        let has_link_url = reddit_config
            .link_url
            .as_deref()
            .is_some_and(|s| !s.is_empty());
        if !has_link_url {
            return Err(invalid(
                "reddit_link plan requires reddit_config.link_url".to_string(),
            ));
        }
    }

    // reddit_image requires either image_prompt (AI gen) or
    // uploaded_image_urls (user-provided).
    if plan_type == PlanType::RedditImage {
        let has_image_prompt = reddit_config
            .image_prompt
            .as_deref()
            .is_some_and(|s| !s.is_empty());
        let has_uploaded = !reddit_config.uploaded_image_urls.is_empty();
        if !has_image_prompt && !has_uploaded {
            return Err(invalid(
                "reddit_image plan requires reddit_config.image_prompt or \
                 reddit_config.uploaded_image_urls"
                    .to_string(),
            ));
        }
    }

    Ok(())
}

fn invalid(msg: String) -> ApiError {
    ApiError::BusinessError(BusinessError::InvalidInput(msg))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn base_input() -> JsonValue {
        json!({
            "content_prompt": "generate reddit post about tech",
            "reddit_config": {
                "subreddit": "programming",
                "reddit_post_type": "TEXT",
            }
        })
    }

    #[test]
    fn text_plan_happy_path() {
        let input = base_input();
        assert!(validate(PlanType::RedditText, Some(&input)).is_ok());
    }

    #[test]
    fn missing_ai_input_rejected() {
        let err = validate(PlanType::RedditText, None).unwrap_err();
        assert!(err.to_string().contains("requires ai_input"));
    }

    #[test]
    fn missing_content_prompt_rejected() {
        let mut input = base_input();
        input["content_prompt"] = json!("");
        let err = validate(PlanType::RedditText, Some(&input)).unwrap_err();
        assert!(err.to_string().contains("content_prompt"));
    }

    #[test]
    fn missing_reddit_config_rejected() {
        let input = json!({"content_prompt": "abc"});
        let err = validate(PlanType::RedditText, Some(&input)).unwrap_err();
        assert!(err.to_string().contains("reddit_config"));
    }

    #[test]
    fn null_reddit_config_rejected() {
        let input = json!({"content_prompt": "abc", "reddit_config": null});
        let err = validate(PlanType::RedditText, Some(&input)).unwrap_err();
        assert!(err.to_string().contains("reddit_config"));
    }

    #[test]
    fn missing_subreddit_rejected() {
        let input = json!({
            "content_prompt": "abc",
            "reddit_config": {"subreddit": ""}
        });
        let err = validate(PlanType::RedditText, Some(&input)).unwrap_err();
        assert!(err.to_string().contains("subreddit"));
    }

    #[test]
    fn reddit_link_requires_link_url() {
        let mut input = base_input();
        input["reddit_config"]["reddit_post_type"] = json!("LINK");
        let err = validate(PlanType::RedditLink, Some(&input)).unwrap_err();
        assert!(err.to_string().contains("link_url"));
    }

    #[test]
    fn reddit_link_with_url_ok() {
        let mut input = base_input();
        input["reddit_config"]["reddit_post_type"] = json!("LINK");
        input["reddit_config"]["link_url"] = json!("https://example.com");
        assert!(validate(PlanType::RedditLink, Some(&input)).is_ok());
    }

    #[test]
    fn reddit_image_requires_image_prompt_or_uploads() {
        let mut input = base_input();
        input["reddit_config"]["reddit_post_type"] = json!("IMAGE");
        let err = validate(PlanType::RedditImage, Some(&input)).unwrap_err();
        assert!(err.to_string().contains("image_prompt"));
    }

    #[test]
    fn reddit_image_with_prompt_ok() {
        let mut input = base_input();
        input["reddit_config"]["reddit_post_type"] = json!("IMAGE");
        input["reddit_config"]["image_prompt"] = json!("a cat in space");
        assert!(validate(PlanType::RedditImage, Some(&input)).is_ok());
    }

    #[test]
    fn reddit_image_with_uploads_ok() {
        let mut input = base_input();
        input["reddit_config"]["reddit_post_type"] = json!("IMAGE");
        input["reddit_config"]["uploaded_image_urls"] = json!(["https://cdn.example/a.jpg"]);
        assert!(validate(PlanType::RedditImage, Some(&input)).is_ok());
    }

    #[test]
    fn malformed_reddit_config_rejected() {
        let input = json!({
            "content_prompt": "abc",
            "reddit_config": "not an object"
        });
        let err = validate(PlanType::RedditText, Some(&input)).unwrap_err();
        assert!(err.to_string().contains("malformed"));
    }
}
