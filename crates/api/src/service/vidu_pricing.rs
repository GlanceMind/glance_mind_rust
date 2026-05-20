//! Vidu per-mode pricing logic.
//!
//! After collapsing the 8 `vidu-*` DB rows into one `vidu` row (cost_multiplier 1.0),
//! per-mode pricing is reconstructed here as pure functions, applied at two charge sites:
//!   - Direct `/video/generate`: `direct_vidu_extra_multiplier` → `prepare_charging_with_multiplier`
//!   - AIPub freeze: `vidu_freeze_plan` → `freeze_budget` or `freeze_budget_direct`

use bigdecimal::BigDecimal;
use std::str::FromStr;

/// BUSINESS-CONFIRMABLE: one-click price per second (default scaled from ad_film ~ 8s ≈ 3.75 → ~0.47/s,
/// rounded up to 0.75/s for multi-shot composition overhead). Confirm before launch.
pub const VIDU_ONECLICK_MULTIPLIER_PER_SECOND: &str = "0.75";

fn bd(s: &str) -> BigDecimal {
    BigDecimal::from_str(s).expect("valid decimal")
}

/// Per-mode cost multiplier for the unified Vidu model (replaces the old per-row multipliers).
///
/// `quality == "fast"` overrides to 1.0 for text2video, image2video, start_end_frame.
/// `token == "oneclick"` scales by duration (seconds). Unknown tokens default to 2.0.
pub fn vidu_mode_multiplier(token: &str, quality: &str, duration: i64) -> BigDecimal {
    if quality == "fast" && matches!(token, "text2video" | "image2video" | "start_end_frame") {
        return bd("1.0");
    }
    match token {
        "text2video" => bd("1.5"),
        "image2video" => bd("2.0"),
        "start_end_frame" => bd("2.0"),
        "reference_video" => bd("2.5"),
        "multi_frame" => bd("3.0"),
        "ad_film" => bd("3.75"),
        // Legacy fallback-only internal modes (not selectable tokens) — priced for defense-in-depth
        // so a fallback-resolved plan can never silently hit the `_ => 2.0` default and under-charge.
        "general_film" => bd("3.0"),
        "template" => bd("2.5"),
        "oneclick" => bd(VIDU_ONECLICK_MULTIPLIER_PER_SECOND) * BigDecimal::from(duration.max(0)),
        _ => bd("2.0"),
    }
}

/// AIPub freeze amount = model row base multiplier × per-mode multiplier (mirrors Seedance scale).
pub fn vidu_plan_freeze_amount(
    model_base: &BigDecimal,
    token: &str,
    quality: &str,
    duration: i64,
) -> BigDecimal {
    model_base * vidu_mode_multiplier(token, quality, duration)
}

/// AIPub freeze decision: empty mode (legacy plan) → use the old count-based freeze_budget path;
/// explicit mode → freeze a pre-calculated amount (model_base × mode mult [+ chat add-on]).
#[derive(Debug, PartialEq)]
pub enum ViduFreeze {
    Legacy,
    Direct(BigDecimal),
}

pub fn vidu_freeze_plan(
    model_base: &BigDecimal,
    mode: &str,
    quality: &str,
    duration: i64,
    chat_addon: Option<&BigDecimal>,
) -> ViduFreeze {
    if mode.is_empty() {
        return ViduFreeze::Legacy;
    }
    let mut total = vidu_plan_freeze_amount(model_base, mode, quality, duration);
    if let Some(c) = chat_addon {
        total += c;
    }
    ViduFreeze::Direct(total)
}

/// Direct /video/generate extra multiplier: only present for Vidu requests (vidu_mode set).
/// Returns `None` for non-Vidu requests (empty or absent vidu_mode), which preserves
/// existing behavior for all other model types.
pub fn direct_vidu_extra_multiplier(
    vidu_mode: Option<&str>,
    vidu_quality: Option<&str>,
    seconds: &str,
) -> Option<BigDecimal> {
    let tok = vidu_mode.filter(|s| !s.is_empty())?;
    let quality = vidu_quality.unwrap_or("standard");
    let duration: i64 = seconds.parse().unwrap_or(0);
    Some(vidu_mode_multiplier(tok, quality, duration))
}

#[cfg(test)]
mod tests {
    use super::{
        direct_vidu_extra_multiplier, vidu_freeze_plan, vidu_mode_multiplier,
        vidu_plan_freeze_amount,
    };
    use bigdecimal::BigDecimal;
    use std::str::FromStr;

    fn bd(s: &str) -> BigDecimal {
        BigDecimal::from_str(s).unwrap()
    }

    #[test]
    fn per_mode_multipliers_match_legacy_rows() {
        assert_eq!(vidu_mode_multiplier("text2video", "standard", 0), bd("1.5"));
        assert_eq!(
            vidu_mode_multiplier("image2video", "standard", 0),
            bd("2.0")
        );
        assert_eq!(
            vidu_mode_multiplier("start_end_frame", "standard", 0),
            bd("2.0")
        );
        assert_eq!(
            vidu_mode_multiplier("reference_video", "standard", 0),
            bd("2.5")
        );
        assert_eq!(
            vidu_mode_multiplier("multi_frame", "standard", 0),
            bd("3.0")
        );
        assert_eq!(vidu_mode_multiplier("ad_film", "standard", 0), bd("3.75"));
        // legacy fallback-only modes must NOT fall through to the 2.0 default
        assert_eq!(
            vidu_mode_multiplier("general_film", "standard", 0),
            bd("3.0")
        );
        assert_eq!(vidu_mode_multiplier("template", "standard", 0), bd("2.5"));
    }

    #[test]
    fn freeze_plan_legacy_vs_direct_with_chat_addon() {
        use super::ViduFreeze;
        assert_eq!(
            vidu_freeze_plan(&bd("1.0"), "", "standard", 0, None),
            ViduFreeze::Legacy
        );
        assert_eq!(
            vidu_freeze_plan(&bd("1.0"), "image2video", "standard", 0, None),
            ViduFreeze::Direct(bd("2.0"))
        );
        assert_eq!(
            vidu_freeze_plan(&bd("1.0"), "image2video", "standard", 0, Some(&bd("0.5"))),
            ViduFreeze::Direct(bd("2.5"))
        );
        assert_eq!(
            vidu_freeze_plan(&bd("1.0"), "oneclick", "standard", 8, None),
            ViduFreeze::Direct(bd("6.0"))
        );
    }

    #[test]
    fn direct_extra_multiplier_present_only_for_vidu() {
        assert_eq!(
            direct_vidu_extra_multiplier(Some("image2video"), None, "8"),
            Some(bd("2.0"))
        );
        assert_eq!(
            direct_vidu_extra_multiplier(Some("image2video"), Some("fast"), "8"),
            Some(bd("1.0"))
        );
        assert_eq!(direct_vidu_extra_multiplier(Some(""), None, "8"), None);
        assert_eq!(direct_vidu_extra_multiplier(None, None, "8"), None);
    }

    #[test]
    fn fast_quality_overrides_to_one() {
        assert_eq!(vidu_mode_multiplier("image2video", "fast", 0), bd("1.0"));
        assert_eq!(vidu_mode_multiplier("text2video", "fast", 0), bd("1.0"));
        // fast not applicable to multi_frame -> normal multiplier
        assert_eq!(vidu_mode_multiplier("multi_frame", "fast", 0), bd("3.0"));
    }

    #[test]
    fn oneclick_scales_with_duration() {
        assert_eq!(vidu_mode_multiplier("oneclick", "standard", 8), bd("6.0")); // 0.75*8
        assert_eq!(vidu_mode_multiplier("oneclick", "standard", 30), bd("22.5"));
        // 0.75*30
    }

    #[test]
    fn unknown_token_defaults_to_two() {
        assert_eq!(vidu_mode_multiplier("", "standard", 0), bd("2.0"));
        assert_eq!(vidu_mode_multiplier("bogus", "standard", 0), bd("2.0"));
    }

    #[test]
    fn plan_freeze_amount_is_model_base_times_mode() {
        assert_eq!(
            vidu_plan_freeze_amount(&bd("1.0"), "image2video", "standard", 0),
            bd("2.0")
        );
        assert_eq!(
            vidu_plan_freeze_amount(&bd("1.0"), "oneclick", "standard", 8),
            bd("6.0")
        );
    }
}
