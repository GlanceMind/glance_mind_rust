use rig::providers::openai;
use std::env;

const DEFAULT_DEEPSEEK_BASE_URL: &str = "https://api.deepseek.com";
const DEFAULT_DEEPSEEK_MODEL: &str = "deepseek-v4-pro";
const DEEPSEEK_OPENAI_PATH: &str = "/v1";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeepSeekConfig {
    pub api_key: String,
    pub base_url: String,
    pub model: String,
}

pub fn from_env() -> DeepSeekConfig {
    from_values(
        env::var("DEEPSEEK_API_KEY").ok().as_deref(),
        env::var("DEEPSEEK_BASE_URL").ok().as_deref(),
        env::var("DEEPSEEK_MODEL").ok().as_deref(),
    )
    .unwrap_or_else(|message| panic!("{message}"))
}

pub fn completions_client(config: &DeepSeekConfig) -> openai::CompletionsClient {
    let client: openai::Client = openai::Client::builder()
        .base_url(&config.base_url)
        .api_key(&config.api_key)
        .build()
        .expect("Failed to build DeepSeek OpenAI-compatible client");

    client.completions_api()
}

pub fn safe_provider_error(context: &str) -> String {
    format!("{context}: DeepSeek provider request failed")
}

pub(crate) fn from_values(
    api_key: Option<&str>,
    base_url: Option<&str>,
    model: Option<&str>,
) -> Result<DeepSeekConfig, String> {
    let api_key = required_trimmed(api_key, "DEEPSEEK_API_KEY")?;
    let base_url =
        normalize_openai_base_url(optional_trimmed(base_url).unwrap_or(DEFAULT_DEEPSEEK_BASE_URL));
    let model = optional_trimmed(model)
        .unwrap_or(DEFAULT_DEEPSEEK_MODEL)
        .to_string();

    Ok(DeepSeekConfig {
        api_key,
        base_url,
        model,
    })
}

fn required_trimmed(value: Option<&str>, key: &str) -> Result<String, String> {
    optional_trimmed(value)
        .map(ToOwned::to_owned)
        .ok_or_else(|| format!("{key} must be set for DeepSeek runtime LLM calls"))
}

fn normalize_openai_base_url(value: &str) -> String {
    let trimmed = value.trim_end_matches('/');
    if trimmed.ends_with(DEEPSEEK_OPENAI_PATH) {
        trimmed.to_string()
    } else {
        format!("{trimmed}{DEEPSEEK_OPENAI_PATH}")
    }
}

fn optional_trimmed(value: Option<&str>) -> Option<&str> {
    value.map(str::trim).filter(|value| !value.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_deepseek_config_from_explicit_env_values() {
        let config = from_values(
            Some("test-deepseek-key"),
            Some("https://deepseek.example/v1/"),
            Some("deepseek-chat-test"),
        )
        .expect("explicit config should resolve");

        assert_eq!(config.api_key, "test-deepseek-key");
        assert_eq!(config.base_url, "https://deepseek.example/v1");
        assert_eq!(config.model, "deepseek-chat-test");
    }

    #[test]
    fn rejects_missing_api_key_without_exposing_secret_values() {
        let error = from_values(
            None,
            Some("https://deepseek.example/v1"),
            Some("deepseek-chat"),
        )
        .expect_err("missing API key should fail");

        assert!(error.contains("DEEPSEEK_API_KEY"));
        assert!(!error.contains("deepseek-chat"));
        assert!(!error.contains("https://deepseek.example"));
    }

    #[test]
    fn defaults_match_backend_deepseek_runtime_contract() {
        let config = from_values(Some("test-deepseek-key"), None, None)
            .expect("defaulted config should resolve");

        assert_eq!(config.base_url, "https://api.deepseek.com/v1");
        // ASSERTION-CHANGE-JUSTIFIED: reverts a prior change that was based on a
        // false premise. "deepseek-v4-pro" IS a valid model on api.deepseek.com
        // (verified via live curl: HTTP 200, response model="deepseek-v4-pro").
        // The real production incident was an expired API key (HTTP 401), not the
        // model name; "deepseek-chat" maps to the weaker deepseek-v4-flash, so
        // defaulting to it silently downgraded the team's chosen model. Restore
        // the intended default.
        assert_eq!(config.model, "deepseek-v4-pro");
    }

    #[test]
    fn provider_error_messages_are_redacted() {
        let message = safe_provider_error("AI Provider Error");

        assert_eq!(
            message,
            "AI Provider Error: DeepSeek provider request failed"
        );
        assert!(!message.contains("No available accounts"));
    }

    #[test]
    fn normalizes_root_deepseek_base_url_to_openai_compatible_v1_path() {
        let config = from_values(
            Some("test-deepseek-key"),
            Some("https://api.deepseek.com"),
            Some("deepseek-v4-pro"),
        )
        .expect("root base URL should normalize");

        assert_eq!(config.base_url, "https://api.deepseek.com/v1");
    }

    #[test]
    fn trims_base_url_and_model_values() {
        let config = from_values(
            Some("  test-deepseek-key  "),
            Some("  https://deepseek.example/v1///  "),
            Some("  deepseek-chat  "),
        )
        .expect("trimmed config should resolve");

        assert_eq!(config.api_key, "test-deepseek-key");
        assert_eq!(config.base_url, "https://deepseek.example/v1");
        assert_eq!(config.model, "deepseek-chat");
    }
}
