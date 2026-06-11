use glance_mind_db::entity::social_group::SocialGroup;

use crate::error::api_error::ApiError;
use crate::error::business_error::BusinessError;
use crate::repository::social_group_repository::SocialGroupRepository;

/// Check that `group` belongs to `user_id` and that its platform matches
/// `expected_platform_id`.  Returns `Err(GroupNotFound)` if the group does
/// not belong to the caller, `Err(GroupPlatformMismatch{…})` if the platform
/// differs, and `Ok(())` when both checks pass.
#[allow(unused_variables)]
pub fn check_group_platform(
    group: &SocialGroup,
    user_id: i32,
    expected_platform_id: i32,
) -> Result<(), BusinessError> {
    todo!()
}

/// Load a social group by id + user_id from the repo, then call
/// `check_group_platform`.  Returns the loaded `SocialGroup` on success.
#[allow(unused_variables)]
pub async fn load_and_check_group(
    repo: &SocialGroupRepository,
    group_id: i32,
    user_id: i32,
    expected_platform_id: i32,
) -> Result<SocialGroup, ApiError> {
    todo!()
}

// ---------------------------------------------------------------------------
// Unit + property tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;
    use proptest::prelude::*;

    fn make_group(id: i32, user_id: i32, platform_id: i32) -> SocialGroup {
        SocialGroup {
            id,
            user_id,
            platform_id,
            group_name: "test_group".to_string(),
            created_at: NaiveDate::from_ymd_opt(2024, 1, 1)
                .unwrap()
                .and_hms_opt(0, 0, 0)
                .unwrap(),
            updated_at: None,
        }
    }

    // -----------------------------------------------------------------------
    // Plain unit test: Display string of GroupPlatformMismatch
    // -----------------------------------------------------------------------

    #[test]
    fn test_group_platform_mismatch_display() {
        let err = BusinessError::GroupPlatformMismatch {
            group_platform_id: 1,
            expected_platform_id: 3,
        };
        assert_eq!(
            err.to_string(),
            "Group platform 1 does not match required platform 3"
        );
    }

    // -----------------------------------------------------------------------
    // P1: wrong owner ⇒ GroupNotFound regardless of platforms
    // -----------------------------------------------------------------------

    proptest! {
        #[test]
        fn p1_wrong_owner_returns_group_not_found(
            group_id in 1..i32::MAX,
            group_user_id in 1..i32::MAX,
            caller_user_id in 1..i32::MAX,
            group_platform in 1..100i32,
            expected_platform in 1..100i32,
        ) {
            // We only care about the case where caller != owner
            prop_assume!(group_user_id != caller_user_id);

            let group = make_group(group_id, group_user_id, group_platform);
            let result = check_group_platform(&group, caller_user_id, expected_platform);

            prop_assert!(
                matches!(result, Err(BusinessError::GroupNotFound)),
                "expected Err(GroupNotFound), got {:?}",
                result
            );
        }
    }

    // -----------------------------------------------------------------------
    // P2: correct owner + platform mismatch ⇒ GroupPlatformMismatch with
    //     correct ids populated
    // -----------------------------------------------------------------------

    proptest! {
        #[test]
        fn p2_owner_platform_mismatch_returns_group_platform_mismatch(
            group_id in 1..i32::MAX,
            caller_user_id in 1..i32::MAX,
            group_platform in 1..100i32,
            expected_platform in 1..100i32,
        ) {
            prop_assume!(group_platform != expected_platform);

            let group = make_group(group_id, caller_user_id, group_platform);
            let result = check_group_platform(&group, caller_user_id, expected_platform);

            match result {
                Err(BusinessError::GroupPlatformMismatch {
                    group_platform_id,
                    expected_platform_id,
                }) => {
                    prop_assert_eq!(group_platform_id, group_platform);
                    prop_assert_eq!(expected_platform_id, expected_platform);
                }
                other => prop_assert!(
                    false,
                    "expected Err(GroupPlatformMismatch), got {:?}",
                    other
                ),
            }
        }
    }

    // -----------------------------------------------------------------------
    // P3: correct owner + matching platform ⇒ Ok(())
    // -----------------------------------------------------------------------

    proptest! {
        #[test]
        fn p3_owner_platform_match_returns_ok(
            group_id in 1..i32::MAX,
            caller_user_id in 1..i32::MAX,
            platform in 1..100i32,
        ) {
            let group = make_group(group_id, caller_user_id, platform);
            let result = check_group_platform(&group, caller_user_id, platform);

            prop_assert!(
                matches!(result, Ok(())),
                "expected Ok(()), got {:?}",
                result
            );
        }
    }
}
