use serde::{Deserialize, Serialize};
use validator::Validate;

#[derive(Debug, Deserialize, Validate)]
pub struct AiGenerateRequest {
    pub platform: String,
    pub region: String,
    pub product_description: String,

    // Optional because for "AUDIENCE" generation we might rely mainly on product/platform/region
    // But usually needed for context.
    pub target_audience: Option<String>,

    // "PERSONA" | "AUDIENCE" | "REPLY"
    #[validate(custom = "validate_generation_type")]
    pub generation_type: String,

    // Only used when generation_type is "REPLY"
    pub reply_requirements: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct AiGenerateResponse {
    pub content: String,
}

fn validate_generation_type(generation_type: &str) -> Result<(), validator::ValidationError> {
    match generation_type {
        "PERSONA" | "AUDIENCE" | "REPLY" | "PRODUCT_DESCRIPTION" | "KEYWORD" => Ok(()),
        _ => Err(validator::ValidationError::new("invalid_generation_type")),
    }
}
