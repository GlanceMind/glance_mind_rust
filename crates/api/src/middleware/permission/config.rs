use once_cell::sync::Lazy;
use std::collections::HashMap;

/// Feature permission bits stored in `gm_users.permissions` (BIGINT / i64).
///
/// Each variant equals `1 << bit_position`. Up to 63 features can be defined.
#[repr(i64)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Permission {
    DmControl = 1 << 0,    // 1
    AiPublish = 1 << 1,    // 2
    AiContentGen = 1 << 2, // 4
    AiLeadGen = 1 << 3,    // 8
}

/// Default bitmask for new users: everything ON except DmControl.
pub const DEFAULT_PERMISSIONS: i64 =
    Permission::AiPublish as i64 | Permission::AiContentGen as i64 | Permission::AiLeadGen as i64; // 14

impl Permission {
    /// Check whether `perm_bits` has this permission enabled.
    #[inline]
    pub fn check(self, perm_bits: i64) -> bool {
        (perm_bits & self as i64) != 0
    }

    pub fn name(self) -> &'static str {
        match self {
            Permission::DmControl => "dm_control",
            Permission::AiPublish => "ai_publish",
            Permission::AiContentGen => "ai_content_gen",
            Permission::AiLeadGen => "ai_lead_gen",
        }
    }

    /// All defined permissions (for iteration / admin UI).
    pub const ALL: &'static [Permission] = &[
        Permission::DmControl,
        Permission::AiPublish,
        Permission::AiContentGen,
        Permission::AiLeadGen,
    ];
}

/// Maps route-path prefixes to the required [`Permission`].
pub static ROUTE_PERMISSION_MAP: Lazy<HashMap<&'static str, Permission>> = Lazy::new(|| {
    let mut m = HashMap::new();
    m.insert("/api/v1/dm", Permission::DmControl);
    m.insert("/api/v1/campaigns", Permission::AiLeadGen);
    m.insert("/api/v1/agent", Permission::AiLeadGen);
    m.insert("/api/v1/scan", Permission::AiLeadGen);
    m.insert("/api/v1/publish_plans", Permission::AiPublish);
    m.insert("/api/v1/upload-tasks", Permission::AiPublish);
    m.insert("/api/v1/ai/", Permission::AiContentGen);
    m.insert("/api/v1/video", Permission::AiContentGen);
    m
});

pub fn find_required_permission(route: &str) -> Option<Permission> {
    ROUTE_PERMISSION_MAP
        .iter()
        .filter(|(prefix, _)| route.starts_with(**prefix))
        .max_by_key(|(prefix, _)| prefix.len())
        .map(|(_, perm)| *perm)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_permissions_value() {
        assert_eq!(DEFAULT_PERMISSIONS, 14);
        assert!(!Permission::DmControl.check(DEFAULT_PERMISSIONS));
        assert!(Permission::AiPublish.check(DEFAULT_PERMISSIONS));
        assert!(Permission::AiContentGen.check(DEFAULT_PERMISSIONS));
        assert!(Permission::AiLeadGen.check(DEFAULT_PERMISSIONS));
    }

    #[test]
    fn test_check_single_bit() {
        assert!(Permission::DmControl.check(1));
        assert!(!Permission::DmControl.check(0));
        assert!(Permission::AiPublish.check(2));
        assert!(!Permission::AiPublish.check(1));
    }

    #[test]
    fn test_check_combined_bits() {
        let bits: i64 = Permission::DmControl as i64 | Permission::AiLeadGen as i64; // 1 + 8 = 9
        assert!(Permission::DmControl.check(bits));
        assert!(Permission::AiLeadGen.check(bits));
        assert!(!Permission::AiPublish.check(bits));
        assert!(!Permission::AiContentGen.check(bits));
    }

    #[test]
    fn test_all_permissions_distinct() {
        let mut seen = std::collections::HashSet::new();
        for p in Permission::ALL {
            assert!(seen.insert(*p as i64), "Duplicate bit value: {}", *p as i64);
        }
        assert_eq!(seen.len(), 4);
    }

    #[test]
    fn test_find_dm_routes() {
        assert_eq!(
            find_required_permission("/api/v1/dm/conversations"),
            Some(Permission::DmControl)
        );
        assert_eq!(
            find_required_permission("/api/v1/dm/send"),
            Some(Permission::DmControl)
        );
    }

    #[test]
    fn test_find_campaign_routes() {
        assert_eq!(
            find_required_permission("/api/v1/campaigns"),
            Some(Permission::AiLeadGen)
        );
        assert_eq!(
            find_required_permission("/api/v1/campaigns/123"),
            Some(Permission::AiLeadGen)
        );
    }

    #[test]
    fn test_find_content_gen_routes() {
        assert_eq!(
            find_required_permission("/api/v1/ai/generate"),
            Some(Permission::AiContentGen)
        );
        assert_eq!(
            find_required_permission("/api/v1/ai/models"),
            Some(Permission::AiContentGen)
        );
        assert_eq!(
            find_required_permission("/api/v1/video/generate"),
            Some(Permission::AiContentGen)
        );
    }

    #[test]
    fn test_aipub_route_not_confused_with_ai() {
        assert_ne!(
            find_required_permission("/api/v1/aipub/upload-image"),
            Some(Permission::AiContentGen),
            "/api/v1/aipub should NOT match /api/v1/ai/ prefix"
        );
    }

    #[test]
    fn test_find_publish_routes() {
        assert_eq!(
            find_required_permission("/api/v1/publish_plans"),
            Some(Permission::AiPublish)
        );
        assert_eq!(
            find_required_permission("/api/v1/upload-tasks"),
            Some(Permission::AiPublish)
        );
    }

    #[test]
    fn test_unregistered_routes_return_none() {
        assert_eq!(find_required_permission("/api/v1/user/profile"), None);
        assert_eq!(find_required_permission("/api/v1/wallet/balance"), None);
        assert_eq!(find_required_permission("/health"), None);
    }

    #[test]
    fn test_permission_names() {
        assert_eq!(Permission::DmControl.name(), "dm_control");
        assert_eq!(Permission::AiPublish.name(), "ai_publish");
        assert_eq!(Permission::AiContentGen.name(), "ai_content_gen");
        assert_eq!(Permission::AiLeadGen.name(), "ai_lead_gen");
    }

    #[test]
    fn test_find_agent_routes() {
        assert_eq!(
            find_required_permission("/api/v1/agent/analyze"),
            Some(Permission::AiLeadGen)
        );
        assert_eq!(
            find_required_permission("/api/v1/agent"),
            Some(Permission::AiLeadGen)
        );
    }

    #[test]
    fn test_find_scan_routes() {
        assert_eq!(
            find_required_permission("/api/v1/scan/videos"),
            Some(Permission::AiLeadGen)
        );
        assert_eq!(
            find_required_permission("/api/v1/scan"),
            Some(Permission::AiLeadGen)
        );
    }

    #[test]
    fn test_check_all_off() {
        let bits: i64 = 0;
        for p in Permission::ALL {
            assert!(!p.check(bits), "{} should be off when bits=0", p.name());
        }
    }

    #[test]
    fn test_check_all_on() {
        let bits: i64 = Permission::DmControl as i64
            | Permission::AiPublish as i64
            | Permission::AiContentGen as i64
            | Permission::AiLeadGen as i64; // 15
        for p in Permission::ALL {
            assert!(p.check(bits), "{} should be on when bits=15", p.name());
        }
    }
}
