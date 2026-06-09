//! Validation for a campaign's account group (social group).
//!
//! Incident grounding: prod campaign 271 had all automation flags on but
//! `social_group_id = NULL` and 0 linked accounts, yet it activated and ran —
//! generating 50 AI reply/DM suggestions that could never be sent. Root cause:
//! there was NO validation requiring an account group.
//!
//! Product decision (FIXED): an account group is UNCONDITIONALLY REQUIRED at
//! campaign create (not gated on automation flags). The group must be VALID:
//!   * non-null `social_group_id`
//!   * the group exists
//!   * the group belongs to the same user (enforced by the caller fetching it
//!     scoped via `SocialGroupRepository::find_by_id(id, user_id)`)
//!   * the group's `platform_id` matches the campaign's `platform_id`
//!
//! This module owns the *pure* decision: given the fetched group (or `None`
//! when missing / not owned) and the campaign's platform, decide whether the
//! pairing is acceptable. Ownership is enforced upstream: a non-owned or
//! non-existent group id resolves to `None` here, which is rejected.

use crate::error::api_error::ApiError;
use glance_mind_db::entity::social_group::SocialGroup;

/// Validate that a campaign has a valid account group.
///
/// `group` is the group fetched scoped to the campaign owner
/// (`find_by_id(social_group_id, user_id)`), so `None` covers both
/// "no social_group_id was set" and "the id does not resolve to a group the
/// user owns". Either way it is rejected (group is required).
///
/// `Ok(())` when the group exists and its `platform_id` matches
/// `campaign_platform_id`. `Err(ApiError::BadRequest)` with a precise bilingual
/// message otherwise.
pub fn validate_campaign_social_group(
    group: Option<&SocialGroup>,
    campaign_platform_id: i32,
) -> Result<(), ApiError> {
    let Some(group) = group else {
        return Err(ApiError::BadRequest(
            "A valid account group is required for this campaign / 该广告活动必须绑定一个有效的账号分组"
                .to_string(),
        ));
    };

    if group.platform_id != campaign_platform_id {
        return Err(ApiError::BadRequest(format!(
            "Account group platform ({}) does not match campaign platform ({}) / 账号分组所属平台（{}）与广告活动平台（{}）不一致",
            group.platform_id, campaign_platform_id, group.platform_id, campaign_platform_id
        )));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDateTime;

    fn group_with_platform(platform_id: i32) -> SocialGroup {
        SocialGroup {
            id: 1,
            user_id: 42,
            platform_id,
            group_name: "test group".to_string(),
            created_at: NaiveDateTime::default(),
            updated_at: None,
        }
    }

    #[test]
    fn none_group_is_rejected_required_missing() {
        // Required-missing: no social_group_id, or id not owned by the user.
        let err = validate_campaign_social_group(None, 1).unwrap_err();
        match err {
            ApiError::BadRequest(msg) => {
                assert!(
                    msg.contains("account group") || msg.contains("账号分组"),
                    "expected required-group message, got {msg:?}"
                );
            }
            other => panic!("expected BadRequest, got {other:?}"),
        }
    }

    #[test]
    fn platform_mismatch_is_rejected() {
        let group = group_with_platform(2);
        let err = validate_campaign_social_group(Some(&group), 1).unwrap_err();
        match err {
            ApiError::BadRequest(msg) => {
                assert!(
                    msg.contains("platform") || msg.contains("平台"),
                    "expected platform-mismatch message, got {msg:?}"
                );
            }
            other => panic!("expected BadRequest, got {other:?}"),
        }
    }

    #[test]
    fn matching_platform_is_ok() {
        let group = group_with_platform(1);
        assert!(validate_campaign_social_group(Some(&group), 1).is_ok());
    }
}
