//! OAuth 2.0 service: PKCE validation, token issuance, refresh rotation, family revocation.
//!
//! Design principles:
//! - Access tokens are issued via `UserService::generate_token` — same signer/issuer as /auth/login (R013 parity).
//! - Refresh tokens are random 32-byte hex stored as SHA-256 hash (never raw in DB).
//! - Family revocation: replayed revoked refresh token → revoke all tokens sharing family_id.
//! - PKCE: constant-time compare via manual byte equality on SHA-256 hashes.

use crate::config::database::Database;
use crate::error::api_error::ApiError;
use crate::repository::oauth_repository::OauthRepository;
use crate::repository::user_repository::{UserRepository, UserRepositoryTrait};
use crate::service::user_service::UserService;
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use chrono::{Duration, Utc};
use glance_mind_db::entity::oauth::{NewOauthAuditLog, NewOauthRefreshToken};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration as StdDuration, Instant};
use uuid::Uuid;

// ---------------------------------------------------------------------------
// Allowed OAuth clients
// ---------------------------------------------------------------------------
const ALLOWED_CLIENTS: &[&str] = &["desktop", "web"];

// ---------------------------------------------------------------------------
// Rate limiter: sliding-window per IP, 10 grants / 60 seconds
// ---------------------------------------------------------------------------
const RATE_LIMIT_WINDOW_SECS: u64 = 60;
const RATE_LIMIT_MAX_REQUESTS: usize = 10;

#[derive(Clone, Default)]
pub struct IpRateLimiter {
    // IP string -> (window_start, count_in_window)
    state: Arc<Mutex<HashMap<String, (Instant, usize)>>>,
}

impl IpRateLimiter {
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns `true` if the request is allowed, `false` if rate-limited.
    pub fn check_and_record(&self, ip: &str) -> bool {
        let mut map = self.state.lock().unwrap_or_else(|e| e.into_inner());
        let window = StdDuration::from_secs(RATE_LIMIT_WINDOW_SECS);
        let now = Instant::now();

        match map.get_mut(ip) {
            Some((start, count)) => {
                if now.duration_since(*start) >= window {
                    // Window expired — reset
                    *start = now;
                    *count = 1;
                    true
                } else if *count < RATE_LIMIT_MAX_REQUESTS {
                    *count += 1;
                    true
                } else {
                    false
                }
            }
            None => {
                map.insert(ip.to_string(), (now, 1));
                true
            }
        }
    }
}

// ---------------------------------------------------------------------------
// OAuth token response DTO
// ---------------------------------------------------------------------------
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct OauthTokenResponse {
    pub access_token: String,
    pub token_type: String,
    pub expires_in: i64,               // seconds until access token expires
    pub refresh_token: String,         // raw (not hashed)
    pub refresh_token_expires_in: i64, // seconds until refresh token expires (R024)
    pub scope: String,
}

// ---------------------------------------------------------------------------
// OauthService
// ---------------------------------------------------------------------------
#[derive(Clone)]
pub struct OauthService {
    pub oauth_repo: OauthRepository,
    pub user_service: UserService<UserRepository>,
    pub rate_limiter: IpRateLimiter,
}

impl OauthService {
    pub fn new(db: &Arc<Database>) -> Self {
        let pool = db.pool.clone();
        let user_repo = UserRepository::new(pool.clone());
        Self {
            oauth_repo: OauthRepository::new(pool),
            user_service: UserService::new(db, user_repo),
            rate_limiter: IpRateLimiter::new(),
        }
    }

    // -----------------------------------------------------------------------
    // Validate client_id
    // -----------------------------------------------------------------------
    pub fn validate_client_id(&self, client_id: &str) -> bool {
        ALLOWED_CLIENTS.contains(&client_id)
    }

    // -----------------------------------------------------------------------
    // PKCE: verify code_verifier against stored code_challenge (S256)
    // Constant-time compare to prevent timing attacks.
    // -----------------------------------------------------------------------
    pub fn verify_pkce(&self, code_verifier: &str, code_challenge: &str) -> bool {
        let mut hasher = Sha256::new();
        hasher.update(code_verifier.as_bytes());
        let digest = hasher.finalize();
        let computed = URL_SAFE_NO_PAD.encode(digest);

        // Constant-time comparison
        if computed.len() != code_challenge.len() {
            return false;
        }
        computed
            .bytes()
            .zip(code_challenge.bytes())
            .fold(0u8, |acc, (a, b)| acc | (a ^ b))
            == 0
    }

    // -----------------------------------------------------------------------
    // Hash a refresh token for storage
    // -----------------------------------------------------------------------
    pub fn hash_refresh_token(&self, raw: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(raw.as_bytes());
        format!("{:x}", hasher.finalize())
    }

    // -----------------------------------------------------------------------
    // Generate a random raw refresh token (32 bytes = 64 hex chars)
    // -----------------------------------------------------------------------
    pub fn generate_raw_refresh_token(&self) -> String {
        use rand::Rng;
        let bytes: [u8; 32] = rand::rng().random();
        bytes.iter().fold(String::new(), |mut s, b| {
            s.push_str(&format!("{:02x}", b));
            s
        })
    }

    // -----------------------------------------------------------------------
    // authorization_code grant
    // -----------------------------------------------------------------------
    pub async fn exchange_code(
        &self,
        code: String,
        code_verifier: String,
        client_id: String,
        redirect_uri: String,
        ip: Option<String>,
        user_agent: Option<String>,
    ) -> Result<OauthTokenResponse, ApiError> {
        // 1. Validate client_id
        if !self.validate_client_id(&client_id) {
            return Err(ApiError::BadRequest(
                r#"{"error":"unauthorized_client"}"#.to_string(),
            ));
        }

        // 2. Atomically claim the code (marks redeemed if currently unredeemed AND not expired).
        //    A None result means the code is unknown, already redeemed, or expired — all map to
        //    invalid_grant (RFC 6749 §5.2).  This single UPDATE-RETURNING eliminates the
        //    TOCTOU race between a SELECT + separate UPDATE (P0 #1).
        let now = Utc::now();
        let oauth_code = match self
            .oauth_repo
            .mark_code_redeemed(code.clone(), now)
            .await
            .map_err(|e| ApiError::InternalServerError(format!("DB error: {}", e)))?
        {
            Some(row) => row,
            None => {
                // Could be unknown, already redeemed (reuse), or expired.
                // Audit as code_reused (covers reuse detection — R012).
                let _ = self
                    .oauth_repo
                    .write_audit(NewOauthAuditLog {
                        event_type: "code_reused".to_string(),
                        client_id: client_id.clone(),
                        user_id: None,
                        ip: ip.clone(),
                        user_agent: user_agent.clone(),
                        metadata: Some(json!({ "code_prefix": &code[..8.min(code.len())] })),
                    })
                    .await;
                return Err(ApiError::BadRequest(
                    r#"{"error":"invalid_grant"}"#.to_string(),
                ));
            }
        };

        // 3. redirect_uri must match exactly (checked after atomic claim — code is consumed either way)
        if oauth_code.redirect_uri != redirect_uri {
            return Err(ApiError::BadRequest(
                r#"{"error":"invalid_grant"}"#.to_string(),
            ));
        }

        // 4. client_id must match what was used to issue the code
        if oauth_code.client_id != client_id {
            return Err(ApiError::BadRequest(
                r#"{"error":"invalid_grant"}"#.to_string(),
            ));
        }

        // 5. PKCE verification (R011 — constant time).
        //    PKCE failure after atomic claim: code is consumed; user must restart flow.
        //    This is RFC-compliant — codes are one-shot regardless of reason for rejection.
        if !self.verify_pkce(&code_verifier, &oauth_code.code_challenge) {
            return Err(ApiError::BadRequest(
                r#"{"error":"invalid_grant"}"#.to_string(),
            ));
        }

        // 6. Look up user
        let user = self
            .user_service
            .user_repo
            .find(oauth_code.user_id as i32)
            .await
            .map_err(|_| ApiError::BadRequest(r#"{"error":"invalid_grant"}"#.to_string()))?;

        // 7. Issue access_token via same signer as /auth/login (R013 parity)
        let token_data = self
            .user_service
            .generate_token(user)
            .map_err(|e| ApiError::InternalServerError(format!("Token error: {}", e)))?;

        // 8. Issue refresh token
        let (raw_refresh, refresh_row) = self
            .create_refresh_token(
                oauth_code.user_id,
                client_id.clone(),
                oauth_code.scope.clone(),
                Uuid::new_v4(), // new family
                None,
            )
            .await?;

        // 9. Audit log
        let _ = self
            .oauth_repo
            .write_audit(NewOauthAuditLog {
                event_type: "code_redeemed".to_string(),
                client_id: client_id.clone(),
                user_id: Some(oauth_code.user_id),
                ip,
                user_agent,
                metadata: Some(json!({ "scope": &oauth_code.scope })),
            })
            .await;

        let access_exp_secs = token_data.exp - token_data.iat;
        let refresh_secs = (refresh_row.expires_at - Utc::now()).num_seconds().max(0);

        Ok(OauthTokenResponse {
            access_token: token_data.token,
            token_type: "Bearer".to_string(),
            expires_in: access_exp_secs,
            refresh_token: raw_refresh,
            refresh_token_expires_in: refresh_secs,
            scope: oauth_code.scope,
        })
    }

    // -----------------------------------------------------------------------
    // refresh_token grant (R014 — rotation + family revocation)
    // -----------------------------------------------------------------------
    pub async fn exchange_refresh_token(
        &self,
        raw_refresh_token: String,
        client_id: String,
        ip: Option<String>,
        user_agent: Option<String>,
    ) -> Result<OauthTokenResponse, ApiError> {
        // 1. Validate client_id
        if !self.validate_client_id(&client_id) {
            return Err(ApiError::BadRequest(
                r#"{"error":"unauthorized_client"}"#.to_string(),
            ));
        }

        // 2. Hash and look up the token (read-only; needed to get family_id for family revocation
        //    if the token is already revoked, and to do validity checks before issuing a new token).
        let hash = self.hash_refresh_token(&raw_refresh_token);
        let row = self
            .oauth_repo
            .find_refresh_token_by_hash(hash)
            .await
            .map_err(|e| ApiError::InternalServerError(format!("DB error: {}", e)))?
            .ok_or_else(|| ApiError::BadRequest(r#"{"error":"invalid_grant"}"#.to_string()))?;

        // 3. Expiry check (before atomic revoke — expired tokens are always invalid_grant).
        if row.expires_at < Utc::now() {
            return Err(ApiError::BadRequest(
                r#"{"error":"invalid_grant"}"#.to_string(),
            ));
        }

        // 4. client_id must match
        if row.client_id != client_id {
            return Err(ApiError::BadRequest(
                r#"{"error":"invalid_grant"}"#.to_string(),
            ));
        }

        // 5. Look up user (before issuing — fail fast if user is gone)
        let user = self
            .user_service
            .user_repo
            .find(row.user_id as i32)
            .await
            .map_err(|_| ApiError::BadRequest(r#"{"error":"invalid_grant"}"#.to_string()))?;

        // 6. Issue new access_token (same signer — R013 parity)
        let token_data = self
            .user_service
            .generate_token(user)
            .map_err(|e| ApiError::InternalServerError(format!("Token error: {}", e)))?;

        // 7. Issue new refresh token (same family).  Inserted BEFORE the old token is revoked
        //    so that we have the new id available for the replaced_by_id link.
        let scope = row.scope.clone();
        let (raw_refresh, new_row) = self
            .create_refresh_token(
                row.user_id,
                client_id.clone(),
                scope.clone(),
                row.family_id,
                Some(row.id),
            )
            .await?;

        // 8. Atomically revoke the old refresh token (WHERE revoked_at IS NULL RETURNING *).
        //    If 0 rows are updated, a concurrent request already revoked this token — trigger
        //    family revocation and return invalid_grant (P0 #2 race fix).
        match self
            .oauth_repo
            .revoke_refresh_token_atomic(row.id, Some(new_row.id))
            .await
            .map_err(|e| ApiError::InternalServerError(format!("DB error: {}", e)))?
        {
            Some(_) => {
                // We won the race; the old token is now revoked.
            }
            None => {
                // Token was already revoked by a concurrent request — family revocation.
                let _ = self.oauth_repo.revoke_family(row.family_id).await;
                let _ = self
                    .oauth_repo
                    .write_audit(NewOauthAuditLog {
                        event_type: "family_revoked".to_string(),
                        client_id: client_id.clone(),
                        user_id: Some(row.user_id),
                        ip: ip.clone(),
                        user_agent: user_agent.clone(),
                        metadata: Some(json!({ "family_id": row.family_id.to_string() })),
                    })
                    .await;
                return Err(ApiError::BadRequest(
                    r#"{"error":"invalid_grant"}"#.to_string(),
                ));
            }
        }

        // 9. Audit log
        let _ = self
            .oauth_repo
            .write_audit(NewOauthAuditLog {
                event_type: "refresh_rotated".to_string(),
                client_id: client_id.clone(),
                user_id: Some(row.user_id),
                ip,
                user_agent,
                metadata: Some(json!({ "family_id": row.family_id.to_string(), "old_id": row.id, "new_id": new_row.id })),
            })
            .await;

        let access_exp_secs = token_data.exp - token_data.iat;
        let refresh_secs = (new_row.expires_at - Utc::now()).num_seconds().max(0);

        Ok(OauthTokenResponse {
            access_token: token_data.token,
            token_type: "Bearer".to_string(),
            expires_in: access_exp_secs,
            refresh_token: raw_refresh,
            refresh_token_expires_in: refresh_secs,
            scope,
        })
    }

    // -----------------------------------------------------------------------
    // Revoke a refresh token (RFC 7009 — idempotent, family-aware)
    // -----------------------------------------------------------------------
    pub async fn revoke_token(
        &self,
        raw_token: String,
        client_id: String,
        ip: Option<String>,
        user_agent: Option<String>,
    ) -> Result<(), ApiError> {
        let hash = self.hash_refresh_token(&raw_token);

        match self
            .oauth_repo
            .find_refresh_token_by_hash(hash)
            .await
            .map_err(|e| ApiError::InternalServerError(format!("DB error: {}", e)))?
        {
            None => {
                // RFC 7009 §2.2: unknown token → still 200 (idempotent)
                Ok(())
            }
            Some(row) => {
                if row.revoked_at.is_some() {
                    // Already revoked — idempotent
                    return Ok(());
                }
                // Revoke entire family
                let _ = self.oauth_repo.revoke_family(row.family_id).await;
                let _ = self
                    .oauth_repo
                    .write_audit(NewOauthAuditLog {
                        event_type: "revoke_called".to_string(),
                        client_id: client_id.clone(),
                        user_id: Some(row.user_id),
                        ip,
                        user_agent,
                        metadata: Some(
                            json!({ "family_id": row.family_id.to_string(), "token_id": row.id }),
                        ),
                    })
                    .await;
                Ok(())
            }
        }
    }

    // -----------------------------------------------------------------------
    // Internal: insert a new refresh token row, returns (raw_token, row)
    // -----------------------------------------------------------------------
    async fn create_refresh_token(
        &self,
        user_id: i64,
        client_id: String,
        scope: String,
        family_id: Uuid,
        _replaced_old_id: Option<i64>, // informational only; actual update done by caller
    ) -> Result<(String, glance_mind_db::entity::oauth::OauthRefreshToken), ApiError> {
        let raw = self.generate_raw_refresh_token();
        let hash = self.hash_refresh_token(&raw);
        let expires_at = Utc::now() + Duration::days(30); // 30-day refresh token (INV-RB06 ≥24h)

        let new_token = NewOauthRefreshToken {
            token_hash: hash,
            family_id,
            user_id,
            client_id,
            scope,
            expires_at,
        };
        let row = self
            .oauth_repo
            .create_refresh_token(new_token)
            .await
            .map_err(|e| ApiError::InternalServerError(format!("DB error: {}", e)))?;
        Ok((raw, row))
    }
}
