use crate::schema::{oauth_audit_log, oauth_codes, oauth_refresh_tokens, ota_config};
use chrono::{DateTime, Utc};
use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use uuid::Uuid;

// ---------------------------------------------------------------------------
// oauth_codes
// ---------------------------------------------------------------------------

/// A short-lived authorization code (TTL 60s) issued by /oauth/authorize.
/// Single-use: `redeemed_at` is set on first redemption, and reuse is rejected.
#[derive(Debug, Clone, Queryable, Selectable, Serialize, Deserialize)]
#[diesel(table_name = oauth_codes)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct OauthCode {
    pub code: String,
    pub user_id: i64,
    pub client_id: String,
    pub redirect_uri: String,
    pub code_challenge: String,
    pub code_challenge_method: String,
    pub scope: String,
    pub state: String,
    pub expires_at: DateTime<Utc>,
    pub redeemed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Insertable, Serialize, Deserialize)]
#[diesel(table_name = oauth_codes)]
pub struct NewOauthCode {
    pub code: String,
    pub user_id: i64,
    pub client_id: String,
    pub redirect_uri: String,
    pub code_challenge: String,
    pub code_challenge_method: String,
    pub scope: String,
    pub state: String,
    pub expires_at: DateTime<Utc>,
}

// ---------------------------------------------------------------------------
// oauth_refresh_tokens
// ---------------------------------------------------------------------------

/// A long-lived refresh token (TTL 30d, default).
/// Token rotation: each successful refresh replaces this row with a new one in the same family.
/// Family revocation: if a revoked token in the family is replayed, all family members are revoked.
#[derive(Debug, Clone, Queryable, Selectable, Serialize, Deserialize)]
#[diesel(table_name = oauth_refresh_tokens)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct OauthRefreshToken {
    pub id: i64,
    pub token_hash: String,
    pub family_id: Uuid,
    pub user_id: i64,
    pub client_id: String,
    pub scope: String,
    pub issued_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub revoked_at: Option<DateTime<Utc>>,
    pub replaced_by_id: Option<i64>,
}

#[derive(Debug, Clone, Insertable, Serialize, Deserialize)]
#[diesel(table_name = oauth_refresh_tokens)]
pub struct NewOauthRefreshToken {
    pub token_hash: String,
    pub family_id: Uuid,
    pub user_id: i64,
    pub client_id: String,
    pub scope: String,
    pub expires_at: DateTime<Utc>,
}

// ---------------------------------------------------------------------------
// oauth_audit_log
// ---------------------------------------------------------------------------

/// Immutable audit record for every security-relevant OAuth event.
/// event_type values: code_issued | code_redeemed | code_reused | refresh_rotated |
///                    family_revoked | revoke_called
#[derive(Debug, Clone, Queryable, Selectable, Serialize, Deserialize)]
#[diesel(table_name = oauth_audit_log)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct OauthAuditLog {
    pub id: i64,
    pub event_type: String,
    pub client_id: String,
    pub user_id: Option<i64>,
    pub ip: Option<String>,
    pub user_agent: Option<String>,
    pub metadata: Option<JsonValue>,
    pub occurred_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Insertable, Serialize, Deserialize)]
#[diesel(table_name = oauth_audit_log)]
pub struct NewOauthAuditLog {
    pub event_type: String,
    pub client_id: String,
    pub user_id: Option<i64>,
    pub ip: Option<String>,
    pub user_agent: Option<String>,
    pub metadata: Option<JsonValue>,
}

// ---------------------------------------------------------------------------
// ota_config
// ---------------------------------------------------------------------------

/// Key-value OTA config store (initially just 'oauth.enabled').
#[derive(Debug, Clone, Queryable, Selectable, Serialize, Deserialize)]
#[diesel(table_name = ota_config)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct OtaConfig {
    pub key: String,
    pub value: String,
    pub updated_at: DateTime<Utc>,
    pub updated_by: String,
}

#[derive(Debug, Clone, AsChangeset, Serialize, Deserialize)]
#[diesel(table_name = ota_config)]
pub struct OtaConfigUpdate {
    pub value: String,
    pub updated_at: DateTime<Utc>,
    pub updated_by: String,
}

#[derive(Debug, Clone, Insertable, Serialize, Deserialize)]
#[diesel(table_name = ota_config)]
pub struct NewOtaConfig {
    pub key: String,
    pub value: String,
    pub updated_by: String,
}
