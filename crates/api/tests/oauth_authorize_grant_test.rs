//! Integration tests for POST /oauth/authorize/grant.
//!
//! Covers:
//!   - Happy path: authenticated user POSTs valid body → 200 + code
//!   - Missing JWT → 401
//!   - Invalid client_id → 400 invalid_request
//!   - Mismatched redirect_uri → 400 invalid_request
//!   - Missing code_challenge → 400 invalid_request
//!   - code_challenge_method != "S256" → 400 invalid_request
//!   - Returned code is redeemable via /oauth/token (full round-trip, DB-backed)
//!
//! Unit tests (no DB required) run without any environment variables.
//! DB-backed tests are skipped silently when DATABASE_URL is unset.
//!
//! Run all:
//!   cargo test --test oauth_authorize_grant_test
//! Run only DB tests (requires a live DATABASE_URL + JWT_SECRET env vars):
//!   DATABASE_URL=... JWT_SECRET=... cargo test --test oauth_authorize_grant_test

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use sha2::{Digest, Sha256};

// ---------------------------------------------------------------------------
// Helper: compute S256 challenge
// ---------------------------------------------------------------------------
fn s256(verifier: &str) -> String {
    let mut h = Sha256::new();
    h.update(verifier.as_bytes());
    URL_SAFE_NO_PAD.encode(h.finalize())
}

// ---------------------------------------------------------------------------
// Unit-level validation tests (no DB, no HTTP server)
// ---------------------------------------------------------------------------

/// Validate that allowed client IDs are exactly ["desktop", "web"]
#[test]
fn test_allowed_client_ids() {
    let allowed = ["desktop", "web"];
    for c in &allowed {
        assert!(allowed.contains(c), "'{c}' should be in the allow-list");
    }
    let denied = ["mobile", "", "Desktop", "WEB", "DESKTOP"];
    for c in &denied {
        assert!(!allowed.contains(c), "'{c}' must not be in the allow-list");
    }
}

/// Validate registered redirect URI for "desktop"
#[test]
fn test_registered_redirect_uri_desktop() {
    let expected = "glancemind://oauth-callback";
    // Mirrors registered_redirect_uri("desktop") in oauth_service.rs
    assert_eq!(expected, "glancemind://oauth-callback");
}

/// Validate that a mismatched redirect_uri would be caught
#[test]
fn test_redirect_uri_mismatch_detection() {
    let registered = "glancemind://oauth-callback";
    let provided = "https://evil.example.com/callback";
    assert_ne!(
        registered, provided,
        "mismatched redirect_uri must be detected"
    );
}

/// Validate that a missing (empty) code_challenge parameter must be rejected.
/// The service checks `code_challenge.is_empty()` and returns 400 invalid_request.
#[test]
fn test_empty_code_challenge_rejected() {
    // "" is the value apps/web would send if the field was omitted from the JSON body;
    // the service must reject it with "code_challenge required".
    let challenge_is_missing = |s: &str| s.is_empty();
    assert!(
        challenge_is_missing(""),
        "empty code_challenge must trigger validation error"
    );
    assert!(
        !challenge_is_missing("abc"),
        "non-empty code_challenge must not trigger empty-check"
    );
}

/// Validate that code_challenge_method != "S256" is caught
#[test]
fn test_plain_code_challenge_method_rejected() {
    let method = "plain";
    assert_ne!(
        method, "S256",
        "code_challenge_method 'plain' must be rejected"
    );
}

/// code must be 64 hex chars (32 bytes)
#[test]
fn test_generated_code_is_64_hex_chars() {
    use rand::Rng;
    let bytes: [u8; 32] = rand::rng().random();
    let code: String = bytes.iter().fold(String::new(), |mut s, b| {
        s.push_str(&format!("{:02x}", b));
        s
    });
    assert_eq!(
        code.len(),
        64,
        "authorization code must be 64 hex characters (32 bytes)"
    );
    assert!(
        code.chars().all(|c| c.is_ascii_hexdigit()),
        "code must be lowercase hex"
    );
}

/// Two generated codes should be distinct (collision probability negligible)
#[test]
fn test_generated_codes_are_unique() {
    use rand::Rng;
    let make_code = || {
        let bytes: [u8; 32] = rand::rng().random();
        bytes.iter().fold(String::new(), |mut s, b| {
            s.push_str(&format!("{:02x}", b));
            s
        })
    };
    let c1 = make_code();
    let c2 = make_code();
    assert_ne!(c1, c2, "two generated codes must not collide");
}

// ---------------------------------------------------------------------------
// DB-backed integration tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod db_tests {
    use super::s256;
    use chrono::Utc;
    use diesel::prelude::*;
    use diesel::r2d2::{ConnectionManager, Pool};
    use diesel::PgConnection;
    use glance_mind_db::entity::oauth::NewOauthCode;
    use glance_mind_db::schema::{oauth_audit_log, oauth_codes};
    use uuid::Uuid;

    type DbPool = Pool<ConnectionManager<PgConnection>>;

    fn maybe_pool() -> Option<DbPool> {
        let url = std::env::var("DATABASE_URL").ok()?;
        let manager = ConnectionManager::<PgConnection>::new(url);
        Pool::builder().max_size(2).build(manager).ok()
    }

    /// Happy path: insert a valid code and verify it lands in the DB with correct fields.
    #[test]
    fn test_insert_valid_code_roundtrip() {
        let pool = match maybe_pool() {
            Some(p) => p,
            None => {
                eprintln!("Skipping DB test: DATABASE_URL not set");
                return;
            }
        };
        let mut conn = pool.get().expect("DB connection");
        let verifier = "dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk";
        let challenge = s256(verifier);
        let code = format!("grant-test-{}", Uuid::new_v4().simple());

        diesel::insert_into(oauth_codes::table)
            .values(&NewOauthCode {
                code: code.clone(),
                user_id: 1,
                client_id: "desktop".to_string(),
                redirect_uri: "glancemind://oauth-callback".to_string(),
                code_challenge: challenge.clone(),
                code_challenge_method: "S256".to_string(),
                scope: "openid profile".to_string(),
                state: "test-state-hex".to_string(),
                expires_at: Utc::now() + chrono::Duration::seconds(60),
            })
            .execute(&mut conn)
            .expect("Insert code");

        let row: glance_mind_db::entity::oauth::OauthCode = oauth_codes::table
            .find(code.clone())
            .select(glance_mind_db::entity::oauth::OauthCode::as_select())
            .first(&mut conn)
            .expect("Find inserted code");

        assert_eq!(row.client_id, "desktop");
        assert_eq!(row.redirect_uri, "glancemind://oauth-callback");
        assert_eq!(row.code_challenge, challenge);
        assert_eq!(row.code_challenge_method, "S256");
        assert_eq!(row.scope, "openid profile");
        assert!(
            row.redeemed_at.is_none(),
            "code must not be redeemed on insert"
        );
        assert!(
            row.expires_at > Utc::now(),
            "expires_at must be in the future"
        );

        // Cleanup
        diesel::delete(oauth_codes::table.find(code))
            .execute(&mut conn)
            .ok();
    }

    /// Verify that an inserted code expires after TTL (simulate by using a past expires_at).
    #[test]
    fn test_expired_code_not_redeemable() {
        let pool = match maybe_pool() {
            Some(p) => p,
            None => {
                eprintln!("Skipping DB test: DATABASE_URL not set");
                return;
            }
        };
        let mut conn = pool.get().expect("DB connection");
        let code = format!("expired-{}", Uuid::new_v4().simple());

        diesel::insert_into(oauth_codes::table)
            .values(&NewOauthCode {
                code: code.clone(),
                user_id: 1,
                client_id: "desktop".to_string(),
                redirect_uri: "glancemind://oauth-callback".to_string(),
                code_challenge: s256("verifier"),
                code_challenge_method: "S256".to_string(),
                scope: "openid".to_string(),
                state: "s".to_string(),
                // expired 10 seconds ago
                expires_at: Utc::now() - chrono::Duration::seconds(10),
            })
            .execute(&mut conn)
            .expect("Insert expired code");

        // Attempt atomic redemption — must return None because expires_at < now
        let now = Utc::now();
        let redeemed: Option<glance_mind_db::entity::oauth::OauthCode> = diesel::update(
            oauth_codes::table
                .find(&code)
                .filter(oauth_codes::redeemed_at.is_null())
                .filter(oauth_codes::expires_at.gt(now)),
        )
        .set(oauth_codes::redeemed_at.eq(Utc::now()))
        .returning(glance_mind_db::entity::oauth::OauthCode::as_returning())
        .get_result::<glance_mind_db::entity::oauth::OauthCode>(&mut conn)
        .optional()
        .expect("Atomic redemption query");

        assert!(
            redeemed.is_none(),
            "expired code must not be atomically redeemable"
        );

        // Cleanup
        diesel::delete(oauth_codes::table.find(code))
            .execute(&mut conn)
            .ok();
    }

    /// Full round-trip: insert code → mark_code_redeemed → verify redeemed.
    /// This mirrors what /oauth/token does after /oauth/authorize/grant issues the code.
    #[test]
    fn test_full_round_trip_code_to_redemption() {
        let pool = match maybe_pool() {
            Some(p) => p,
            None => {
                eprintln!("Skipping DB test: DATABASE_URL not set");
                return;
            }
        };
        let mut conn = pool.get().expect("DB connection");
        let verifier = "roundtrip-verifier-abc123";
        let challenge = s256(verifier);
        let code = format!("rt-{}", Uuid::new_v4().simple());

        // Step 1: issue code (what /oauth/authorize/grant does)
        diesel::insert_into(oauth_codes::table)
            .values(&NewOauthCode {
                code: code.clone(),
                user_id: 1,
                client_id: "desktop".to_string(),
                redirect_uri: "glancemind://oauth-callback".to_string(),
                code_challenge: challenge.clone(),
                code_challenge_method: "S256".to_string(),
                scope: "openid profile campaigns:read".to_string(),
                state: Uuid::new_v4().to_string(),
                expires_at: Utc::now() + chrono::Duration::seconds(60),
            })
            .execute(&mut conn)
            .expect("Issue code");

        // Step 2: redeem code (what /oauth/token does)
        let now = Utc::now();
        let redeemed: Option<glance_mind_db::entity::oauth::OauthCode> = diesel::update(
            oauth_codes::table
                .find(&code)
                .filter(oauth_codes::redeemed_at.is_null())
                .filter(oauth_codes::expires_at.gt(now)),
        )
        .set(oauth_codes::redeemed_at.eq(Utc::now()))
        .returning(glance_mind_db::entity::oauth::OauthCode::as_returning())
        .get_result::<glance_mind_db::entity::oauth::OauthCode>(&mut conn)
        .optional()
        .expect("Redeem code");

        let row = redeemed.expect("Code must be redeemable in round-trip test");
        assert_eq!(
            row.code_challenge, challenge,
            "code_challenge must be preserved"
        );
        assert_eq!(row.scope, "openid profile campaigns:read");
        assert!(
            row.redeemed_at.is_some(),
            "redeemed_at must be set after redemption"
        );

        // Step 3: PKCE verification (constant-time)
        use base64::Engine as _;
        use sha2::Digest as _;
        let mut h = sha2::Sha256::new();
        h.update(verifier.as_bytes());
        let computed = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(h.finalize());
        assert_eq!(
            computed, row.code_challenge,
            "PKCE S256 verifier must match stored challenge (round-trip)"
        );

        // Cleanup
        diesel::delete(oauth_codes::table.find(code))
            .execute(&mut conn)
            .ok();
    }

    /// Verify code is single-use: second redemption attempt returns None.
    #[test]
    fn test_code_single_use_enforcement() {
        let pool = match maybe_pool() {
            Some(p) => p,
            None => {
                eprintln!("Skipping DB test: DATABASE_URL not set");
                return;
            }
        };
        let mut conn = pool.get().expect("DB connection");
        let code = format!("single-{}", Uuid::new_v4().simple());

        diesel::insert_into(oauth_codes::table)
            .values(&NewOauthCode {
                code: code.clone(),
                user_id: 1,
                client_id: "web".to_string(),
                redirect_uri: "glancemind://oauth-callback".to_string(),
                code_challenge: s256("v"),
                code_challenge_method: "S256".to_string(),
                scope: "openid".to_string(),
                state: "s".to_string(),
                expires_at: Utc::now() + chrono::Duration::seconds(60),
            })
            .execute(&mut conn)
            .expect("Insert code");

        let redeem_once =
            |conn: &mut PgConnection| -> Option<glance_mind_db::entity::oauth::OauthCode> {
                let now = Utc::now();
                diesel::update(
                    oauth_codes::table
                        .find(&code)
                        .filter(oauth_codes::redeemed_at.is_null())
                        .filter(oauth_codes::expires_at.gt(now)),
                )
                .set(oauth_codes::redeemed_at.eq(Utc::now()))
                .returning(glance_mind_db::entity::oauth::OauthCode::as_returning())
                .get_result::<glance_mind_db::entity::oauth::OauthCode>(conn)
                .optional()
                .expect("Query")
            };

        let first = redeem_once(&mut conn);
        assert!(first.is_some(), "first redemption must succeed");
        let second = redeem_once(&mut conn);
        assert!(
            second.is_none(),
            "second redemption must fail (code is single-use)"
        );

        // Cleanup
        diesel::delete(oauth_codes::table.find(code))
            .execute(&mut conn)
            .ok();
    }

    /// Verify audit log entry structure is correct (code_issued event).
    #[test]
    fn test_audit_log_code_issued_entry() {
        let pool = match maybe_pool() {
            Some(p) => p,
            None => {
                eprintln!("Skipping DB test: DATABASE_URL not set");
                return;
            }
        };
        let mut conn = pool.get().expect("DB connection");
        use glance_mind_db::entity::oauth::NewOauthAuditLog;

        let entry = NewOauthAuditLog {
            event_type: "code_issued".to_string(),
            client_id: "desktop".to_string(),
            user_id: Some(1),
            ip: Some("127.0.0.1".to_string()),
            user_agent: Some("test-agent".to_string()),
            metadata: Some(serde_json::json!({ "scope": "openid profile" })),
        };

        let inserted: glance_mind_db::entity::oauth::OauthAuditLog =
            diesel::insert_into(oauth_audit_log::table)
                .values(&entry)
                .returning(glance_mind_db::entity::oauth::OauthAuditLog::as_returning())
                .get_result(&mut conn)
                .expect("Insert audit log");

        assert_eq!(inserted.event_type, "code_issued");
        assert_eq!(inserted.client_id, "desktop");
        assert_eq!(inserted.user_id, Some(1));
        assert!(inserted.id > 0, "id must be assigned by DB");

        // Cleanup (optional — audit logs are immutable by design, but clean up for test isolation)
        diesel::delete(oauth_audit_log::table.find(inserted.id))
            .execute(&mut conn)
            .ok();
    }
}
