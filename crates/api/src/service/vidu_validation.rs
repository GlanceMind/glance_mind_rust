//! Vidu video configuration validation for AIPub plans.
//! Validates ai_input.vidu_config against the Vidu mode contract.
//! Mirrors seedance_validation.rs in structure.

use crate::error::api_error::ApiError;
use crate::error::business_error::BusinessError;
use serde_json::Value as JsonValue;

const VALID_TOKENS: [&str; 7] = [
    "text2video",
    "image2video",
    "start_end_frame",
    "reference_video",
    "multi_frame",
    "ad_film",
    "oneclick",
];

pub struct ViduValidated {
    pub mode: String,
}

pub fn is_vidu_plan(ai_input: &Option<JsonValue>) -> bool {
    ai_input
        .as_ref()
        .and_then(|v| v.get("vidu_config"))
        .is_some_and(|v| !v.is_null())
}

fn invalid(msg: &str) -> ApiError {
    ApiError::BusinessError(BusinessError::InvalidInput(msg.to_string()))
}

pub fn validate_vidu_config(ai_input: &JsonValue) -> Result<ViduValidated, ApiError> {
    let cfg = ai_input
        .get("vidu_config")
        .filter(|v| !v.is_null())
        .ok_or_else(|| invalid("vidu_config missing"))?;
    let mode = cfg.get("mode").and_then(|v| v.as_str()).unwrap_or("");
    if mode.is_empty() {
        // legacy plan (pre-migration): scheduler falls back to model_key derivation
        return Ok(ViduValidated {
            mode: String::new(),
        });
    }
    if !VALID_TOKENS.contains(&mode) {
        return Err(invalid(&format!("unknown vidu mode: {mode}")));
    }
    if mode == "oneclick" {
        let imgs = ai_input
            .get("reference_image_urls")
            .and_then(|v| v.as_array())
            .map(|a| a.len())
            .unwrap_or(0);
        if !(1..=7).contains(&imgs) {
            return Err(invalid(
                "oneclick requires 1-7 images in reference_image_urls",
            ));
        }
        let dur = ai_input
            .get("video_config")
            .and_then(|c| c.get("duration"))
            .and_then(|v| v.as_i64())
            .unwrap_or(0);
        // v1 cap: Vidu supports 1-180s, but until one-click pricing is confirmed we bound worst-case
        // spend with VIDU_ONECLICK_MAX_DURATION_SECONDS = 60 (easy to raise later).
        const VIDU_ONECLICK_MAX_DURATION_SECONDS: i64 = 60;
        if !(1..=VIDU_ONECLICK_MAX_DURATION_SECONDS).contains(&dur) {
            return Err(invalid("oneclick duration must be 1-60 seconds (v1 cap)"));
        }
    }
    Ok(ViduValidated {
        mode: mode.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn accepts_valid_oneclick_plan() {
        let ai = json!({"vidu_config": {"mode": "oneclick"},
                        "video_config": {"duration": 30},
                        "reference_image_urls": ["https://x/1.png","https://x/2.png"]});
        let v = validate_vidu_config(&ai).expect("valid");
        assert_eq!(v.mode, "oneclick");
    }
    #[test]
    fn rejects_unknown_mode() {
        let ai = json!({"vidu_config": {"mode": "bogus"}});
        assert!(validate_vidu_config(&ai).is_err());
    }
    #[test]
    fn rejects_oneclick_duration_out_of_range() {
        let ai = json!({"vidu_config": {"mode":"oneclick"}, "video_config": {"duration": 999},
                        "reference_image_urls":["https://x/1.png"]});
        assert!(validate_vidu_config(&ai).is_err());
    }
    #[test]
    fn rejects_oneclick_too_many_images() {
        let urls: Vec<String> = (0..8).map(|i| format!("https://x/{i}.png")).collect();
        let ai = json!({"vidu_config": {"mode":"oneclick"}, "video_config": {"duration": 10},
                        "reference_image_urls": urls});
        assert!(validate_vidu_config(&ai).is_err());
    }
    #[test]
    fn legacy_plan_without_mode_is_allowed() {
        let ai = json!({"vidu_config": {"style":"general"}}); // pre-migration plan
        assert!(validate_vidu_config(&ai).is_ok());
    }
    #[test]
    fn is_vidu_plan_detects_config() {
        assert!(is_vidu_plan(&Some(
            json!({"vidu_config": {"mode":"image2video"}})
        )));
        assert!(!is_vidu_plan(&Some(json!({"seedance_config": {}}))));
    }
}
