//! Validation for a campaign's account group (social group).
//!
//! Incident grounding: prod campaign 271 had all automation flags on but
//! `social_group_id = NULL` and 0 linked accounts, yet it activated and ran —
//! generating 50 AI reply/DM suggestions that could never be sent. Root cause:
//! there was NO validation requiring an account group.
//!
//! Product decision (E1, revised): an account group is UNCONDITIONALLY REQUIRED
//! at campaign create. The group must be VALID:
//!   * non-null `social_group_id`
//!   * the group exists
//!   * the group belongs to the same user (enforced by the caller fetching it
//!     scoped via `SocialGroupRepository::find_by_id(id, user_id)`)
//!
//! There is intentionally NO platform check. Prod-data finding: 68 of 69
//! `gm_social_groups` rows store `platform_id = 1` (reddit) regardless of their
//! actual accounts — the column is unreliable. The real platform lives on the
//! accounts (`gm_social_accounts.platform_id`). A `group.platform_id ==
//! campaign.platform_id` rule would reject nearly every legitimate
//! TikTok/Facebook campaign, so the platform dimension is removed.
//!
//! This module owns the *pure* decision: given the fetched group (or `None`
//! when missing / not owned), decide whether it is acceptable. Ownership is
//! enforced upstream: a non-owned or non-existent group id resolves to `None`
//! here, which is rejected.

use crate::error::api_error::ApiError;
use glance_mind_db::entity::social_group::SocialGroup;

/// Validate that a campaign has a valid account group.
///
/// `group` is the group fetched scoped to the campaign owner
/// (`find_by_id(social_group_id, user_id)`), so `None` covers both
/// "no social_group_id was set" and "the id does not resolve to a group the
/// user owns". Either way it is rejected (group is required).
///
/// `Ok(())` when the group exists (it is owned by definition of the upstream
/// fetch). `Err(ApiError::BadRequest)` with a bilingual message when missing.
/// There is no platform check (see module docs).
pub fn validate_campaign_social_group(group: Option<&SocialGroup>) -> Result<(), ApiError> {
    if group.is_none() {
        return Err(ApiError::BadRequest(
            "A valid account group is required for this campaign / 该广告活动必须绑定一个有效的账号分组"
                .to_string(),
        ));
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

    // ASSERTION-CHANGE-JUSTIFIED: platform-match removed — gm_social_groups.platform_id
    // is unreliable (defaulted to reddit); validation now requires only a non-null
    // owned group. The `platform_mismatch_is_rejected` test encoded the removed
    // behavior and is deleted wholesale. Per product decision E1.

    #[test]
    fn none_group_is_rejected_required_missing() {
        // Required-missing: no social_group_id, or id not owned by the user.
        let err = validate_campaign_social_group(None).unwrap_err();
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
    fn present_group_is_ok_regardless_of_platform() {
        // Platform is intentionally not checked: gm_social_groups.platform_id is
        // unreliable (defaulted to reddit). Any present, owned group is accepted,
        // even when its stored platform_id differs from the campaign's.
        let group = group_with_platform(2);
        assert!(validate_campaign_social_group(Some(&group)).is_ok());
    }
}
