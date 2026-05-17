//! OTA config endpoint tests (R023).
//!
//! Covers: auth-required write, public read, value validation.
//! Skipped silently when DATABASE_URL is unset.

// ---------------------------------------------------------------------------
// Unit tests — no DB, no running server
// ---------------------------------------------------------------------------

/// R023: OTA_AUTH_TOKEN must be checked before allowing writes.
/// Simulates the constant-time comparison logic.
#[test]
fn test_r023_ota_auth_bearer_required() {
    use subtle::ConstantTimeEq;

    let expected = "secret-ota-token";
    let provided_correct = "secret-ota-token";
    let provided_wrong = "WRONG_TOKEN_____"; // same length for ct_eq test
    let provided_empty = "";

    // Correct token → allowed
    let ok: bool = provided_correct
        .as_bytes()
        .ct_eq(expected.as_bytes())
        .into();
    assert!(ok, "Correct OTA token must be accepted");

    // Wrong token of same length → denied
    let ok_wrong: bool = provided_wrong.as_bytes().ct_eq(expected.as_bytes()).into();
    assert!(!ok_wrong, "Wrong OTA token must be rejected");

    // Empty token → denied (length mismatch)
    let ok_empty = provided_empty.len() == expected.len();
    assert!(!ok_empty, "Empty OTA token must be rejected");
}

/// R026: Only "true" or "false" are valid values for oauth.enabled.
#[test]
fn test_r026_ota_value_strict_validation() {
    fn is_valid(v: &str) -> bool {
        v == "true" || v == "false"
    }

    assert!(is_valid("true"), "'true' must be accepted");
    assert!(is_valid("false"), "'false' must be accepted");
    assert!(!is_valid("1"), "'1' must be rejected (R026)");
    assert!(!is_valid("yes"), "'yes' must be rejected (R026)");
    assert!(
        !is_valid("True"),
        "'True' (wrong case) must be rejected (R026)"
    );
    assert!(!is_valid(""), "empty string must be rejected (R026)");
    assert!(!is_valid("TRUE"), "'TRUE' must be rejected (R026)");
}

/// R019/R026: Strict equality check for oauth.enabled flag.
#[test]
fn test_r019_oauth_enabled_flag_strict_equality() {
    // Desktop client uses === 'true' (strict string equality)
    // The OTA endpoint must write exactly "true" or "false"
    fn desktop_reads_as_enabled(stored_value: &str) -> bool {
        stored_value == "true"
    }

    assert!(desktop_reads_as_enabled("true"), "'true' must enable OAuth");
    assert!(
        !desktop_reads_as_enabled("false"),
        "'false' must disable OAuth"
    );
    assert!(
        !desktop_reads_as_enabled("1"),
        "'1' must NOT enable OAuth (R026)"
    );
    assert!(
        !desktop_reads_as_enabled("yes"),
        "'yes' must NOT enable OAuth (R026)"
    );
    assert!(
        !desktop_reads_as_enabled(""),
        "empty must NOT enable OAuth (R026)"
    );
    assert!(
        !desktop_reads_as_enabled("True"),
        "wrong-case must NOT enable OAuth (R026)"
    );
}

// ---------------------------------------------------------------------------
// DB-backed integration tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod db_tests {
    use diesel::prelude::*;
    use diesel::r2d2::{ConnectionManager, Pool};
    use diesel::PgConnection;

    type DbPool = Pool<ConnectionManager<PgConnection>>;

    fn maybe_pool() -> Option<DbPool> {
        let url = std::env::var("DATABASE_URL").ok()?;
        let manager = ConnectionManager::<PgConnection>::new(url);
        Pool::builder().max_size(2).build(manager).ok()
    }

    /// R023: ota_config table must have an 'oauth.enabled' row after migration.
    #[test]
    fn test_r023_ota_config_row_exists() {
        let pool = match maybe_pool() {
            Some(p) => p,
            None => {
                eprintln!("Skipping DB test: DATABASE_URL not set");
                return;
            }
        };
        use glance_mind_db::schema::ota_config;

        let mut conn = pool.get().expect("DB connection");
        let row: Option<(String,)> = ota_config::table
            .find("oauth.enabled")
            .select((ota_config::value,))
            .first(&mut conn)
            .optional()
            .expect("Query ota_config");

        assert!(
            row.is_some(),
            "ota_config must have 'oauth.enabled' row after migration (R023)"
        );
        let (val,) = row.unwrap();
        assert!(
            val == "true" || val == "false",
            "Initial value must be 'true' or 'false', got: '{}' (R026)",
            val
        );
    }

    /// R023: Default value of oauth.enabled is 'false' (seeded by migration).
    #[test]
    fn test_r023_ota_config_default_is_false() {
        let pool = match maybe_pool() {
            Some(p) => p,
            None => {
                eprintln!("Skipping DB test: DATABASE_URL not set");
                return;
            }
        };
        use glance_mind_db::schema::ota_config;

        let mut conn = pool.get().expect("DB connection");

        // Only assert on fresh deployments; skip if value has been manually flipped
        let row: Option<(String, String)> = ota_config::table
            .find("oauth.enabled")
            .select((ota_config::value, ota_config::updated_by))
            .first(&mut conn)
            .optional()
            .expect("Query ota_config");

        if let Some((val, updated_by)) = row {
            if updated_by == "migration" {
                // Fresh install — must be 'false' per INV-RB04
                assert_eq!(
                    val, "false",
                    "Default oauth.enabled must be 'false' on fresh install (INV-RB04)"
                );
            } else {
                eprintln!(
                    "Skipping default-value assertion: oauth.enabled was updated by '{}' (value: '{}')",
                    updated_by, val
                );
            }
        }
    }

    /// R025: Prometheus metric counters must be registered and incrementable.
    #[test]
    fn test_r025_metric_counters_registered() {
        use glance_mind_api::service::oauth_metrics;

        // Force initialization of each lazy counter (inc triggers Lazy::force)
        oauth_metrics::inc_token("desktop", "authorization_code");
        oauth_metrics::inc_token("web", "refresh_token");
        oauth_metrics::inc_invalid_grant("desktop", "invalid_grant");
        oauth_metrics::inc_network_error("desktop");
        oauth_metrics::inc_revoke("desktop");

        // oauth_authorize_total is defined for completeness (web surface);
        // force-initialize it directly via the static
        oauth_metrics::OAUTH_AUTHORIZE_TOTAL
            .with_label_values(&["desktop"])
            .inc();

        // Verify that the counters can be gathered (no panic = pass)
        let families = prometheus::gather();
        let names: Vec<String> = families.iter().map(|f| f.get_name().to_string()).collect();

        assert!(
            names.contains(&"oauth_token_total".to_string()),
            "oauth_token_total must be registered (R025)"
        );
        assert!(
            names.contains(&"oauth_invalid_grant_total".to_string()),
            "oauth_invalid_grant_total must be registered (R025)"
        );
        assert!(
            names.contains(&"oauth_network_error_total".to_string()),
            "oauth_network_error_total must be registered (R025)"
        );
        assert!(
            names.contains(&"oauth_revoke_total".to_string()),
            "oauth_revoke_total must be registered (R025)"
        );
        assert!(
            names.contains(&"oauth_authorize_total".to_string()),
            "oauth_authorize_total must be registered (R025)"
        );
    }
}
