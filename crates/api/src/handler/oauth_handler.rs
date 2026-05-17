//! OAuth 2.0 endpoints:
//!   POST /oauth/token            — authorization_code + refresh_token grants
//!   POST /oauth/revoke           — RFC 7009 token revocation (idempotent + family-aware)
//!   POST /oauth/authorize/grant  — consent-page: issue authorization code (JWT-authenticated)
//!   POST /ota/config             — set oauth.enabled flag (bearer-token auth, R023)
//!   GET  /ota/config/oauth.enabled — public read of the OTA flag

use crate::error::api_error::ApiError;
use crate::repository::user_repository::UserRepositoryTrait;
use crate::service::oauth_metrics;
use crate::service::oauth_service::AuthorizeGrantParams;
use crate::state::oauth_state::OauthState;
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::response::IntoResponse;
use axum::{Form, Json};
use serde::Deserialize;
use serde_json::{json, Value};
use subtle::ConstantTimeEq;

// ---------------------------------------------------------------------------
// Form payloads
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
pub struct TokenRequestForm {
    pub grant_type: Option<String>,
    // authorization_code fields
    pub code: Option<String>,
    pub code_verifier: Option<String>,
    pub redirect_uri: Option<String>,
    // shared
    pub client_id: Option<String>,
    // refresh_token fields
    pub refresh_token: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct RevokeRequestForm {
    pub token: Option<String>,
    pub client_id: Option<String>,
}

// ---------------------------------------------------------------------------
// POST /oauth/token
// ---------------------------------------------------------------------------

pub async fn token(
    State(state): State<OauthState>,
    headers: HeaderMap,
    Form(form): Form<TokenRequestForm>,
) -> Result<impl IntoResponse, ApiError> {
    // Extract IP and UA for audit log
    let ip = extract_ip(&headers);
    let ua = extract_ua(&headers);

    // Rate limit check (R015: 10/min/IP)
    let rate_ip = ip.clone().unwrap_or_else(|| "unknown".to_string());
    if !state.oauth_service.rate_limiter.check_and_record(&rate_ip) {
        return Ok(oauth_json(
            StatusCode::TOO_MANY_REQUESTS,
            json!({"error": "slow_down"}),
        ));
    }

    let grant_type = form.grant_type.as_deref().unwrap_or("");

    match grant_type {
        "authorization_code" => {
            let code = require_field(&form.code, "code")?;
            let code_verifier = require_field(&form.code_verifier, "code_verifier")?;
            let client_id = require_field(&form.client_id, "client_id")?;
            let redirect_uri = require_field(&form.redirect_uri, "redirect_uri")?;

            match state
                .oauth_service
                .exchange_code(code, code_verifier, client_id.clone(), redirect_uri, ip, ua)
                .await
            {
                Ok(resp) => {
                    oauth_metrics::inc_token(&client_id, "authorization_code");
                    let body = serde_json::to_value(resp).map_err(|e| {
                        ApiError::InternalServerError(format!("Serialization error: {}", e))
                    })?;
                    Ok(oauth_json(StatusCode::OK, body))
                }
                Err(ApiError::BadRequest(msg))
                    if msg.contains("invalid_grant") || msg.contains("unauthorized_client") =>
                {
                    let err_code = if msg.contains("unauthorized_client") {
                        "unauthorized_client"
                    } else {
                        "invalid_grant"
                    };
                    oauth_metrics::inc_invalid_grant(&client_id, err_code);
                    Ok(oauth_json(
                        StatusCode::BAD_REQUEST,
                        json!({"error": err_code}),
                    ))
                }
                Err(e) => {
                    oauth_metrics::inc_network_error(&client_id);
                    Err(e)
                }
            }
        }
        "refresh_token" => {
            let raw_token = require_field(&form.refresh_token, "refresh_token")?;
            let client_id = require_field(&form.client_id, "client_id")?;

            match state
                .oauth_service
                .exchange_refresh_token(raw_token, client_id.clone(), ip, ua)
                .await
            {
                Ok(resp) => {
                    oauth_metrics::inc_token(&client_id, "refresh_token");
                    let body = serde_json::to_value(resp).map_err(|e| {
                        ApiError::InternalServerError(format!("Serialization error: {}", e))
                    })?;
                    Ok(oauth_json(StatusCode::OK, body))
                }
                Err(ApiError::BadRequest(msg))
                    if msg.contains("invalid_grant") || msg.contains("unauthorized_client") =>
                {
                    let err_code = if msg.contains("unauthorized_client") {
                        "unauthorized_client"
                    } else {
                        "invalid_grant"
                    };
                    oauth_metrics::inc_invalid_grant(&client_id, err_code);
                    Ok(oauth_json(
                        StatusCode::BAD_REQUEST,
                        json!({"error": err_code}),
                    ))
                }
                Err(e) => {
                    oauth_metrics::inc_network_error(&client_id);
                    Err(e)
                }
            }
        }
        _ => {
            // Unknown / missing grant type
            Ok(oauth_json(
                StatusCode::BAD_REQUEST,
                json!({"error": "unsupported_grant_type"}),
            ))
        }
    }
}

// ---------------------------------------------------------------------------
// POST /oauth/revoke  (RFC 7009)
// ---------------------------------------------------------------------------

pub async fn revoke(
    State(state): State<OauthState>,
    headers: HeaderMap,
    Form(form): Form<RevokeRequestForm>,
) -> Result<impl IntoResponse, ApiError> {
    let ip = extract_ip(&headers);
    let ua = extract_ua(&headers);

    // RFC 7009 §2.1 — client authentication is REQUIRED; reject missing/wrong client_id with 401
    let client_id = match form.client_id.as_deref() {
        Some(c) if !c.is_empty() => c.to_string(),
        _ => {
            return Ok(oauth_json(
                StatusCode::UNAUTHORIZED,
                json!({"error": "invalid_client"}),
            ));
        }
    };

    if !state.oauth_service.validate_client_id(&client_id) {
        return Ok(oauth_json(
            StatusCode::UNAUTHORIZED,
            json!({"error": "invalid_client"}),
        ));
    }

    let token = match form.token.as_deref() {
        Some(t) if !t.is_empty() => t.to_string(),
        _ => {
            // RFC 7009 §2.1 — the "token" parameter is REQUIRED.
            // A missing or empty token field is invalid_request (400), not a silent 200.
            return Ok(oauth_json(
                StatusCode::BAD_REQUEST,
                json!({"error": "invalid_request", "error_description": "missing parameter: token"}),
            ));
        }
    };

    state
        .oauth_service
        .revoke_token(token, client_id.clone(), ip, ua)
        .await?;

    oauth_metrics::inc_revoke(&client_id);

    // RFC 7009 §2.2 — empty 200 body
    Ok(oauth_json(StatusCode::OK, json!({})))
}

// ---------------------------------------------------------------------------
// POST /ota/config  (R023 — service-account bearer token auth)
// ---------------------------------------------------------------------------
//
// Body: application/x-www-form-urlencoded
// Accepted key=value: oauth.enabled=true  OR  oauth.enabled=false  (strict)
// Auth: Authorization: Bearer <OTA_AUTH_TOKEN env var>

#[derive(Debug, Deserialize)]
pub struct OtaConfigForm {
    #[serde(rename = "oauth.enabled")]
    pub oauth_enabled: Option<String>,
}

pub async fn set_ota_config(
    State(state): State<OauthState>,
    headers: HeaderMap,
    Form(form): Form<OtaConfigForm>,
) -> Result<impl IntoResponse, ApiError> {
    // Authenticate: compare Bearer token to OTA_AUTH_TOKEN (constant-time).
    // Token is read once at service startup and stored in OauthState (P1 #1 fix).
    let expected = &state.ota_auth_token;
    if expected.is_empty() {
        return Ok(oauth_json(
            StatusCode::INTERNAL_SERVER_ERROR,
            json!({"error": "ota_auth_not_configured"}),
        ));
    }

    let provided = extract_bearer_token(&headers).unwrap_or_default();
    if !constant_time_eq(provided.as_bytes(), expected.as_bytes()) {
        return Ok(oauth_json(
            StatusCode::UNAUTHORIZED,
            json!({"error": "unauthorized"}),
        ));
    }

    // Validate value: only "true" or "false" allowed (R026)
    let value = match form.oauth_enabled.as_deref() {
        Some("true") => "true",
        Some("false") => "false",
        _ => {
            return Ok(oauth_json(
                StatusCode::BAD_REQUEST,
                json!({"error": "invalid_value", "message": "oauth.enabled must be 'true' or 'false'"}),
            ));
        }
    };

    let updated = state
        .oauth_service
        .oauth_repo
        .upsert_ota_config(
            "oauth.enabled".to_string(),
            value.to_string(),
            "ota_api".to_string(),
        )
        .await
        .map_err(|e| ApiError::InternalServerError(format!("DB error: {}", e)))?;

    Ok(oauth_json(
        StatusCode::OK,
        json!({"oauth.enabled": updated.value}),
    ))
}

// ---------------------------------------------------------------------------
// GET /ota/config/oauth.enabled  (public read — R023)
// ---------------------------------------------------------------------------

pub async fn get_ota_config(
    State(state): State<OauthState>,
) -> Result<impl IntoResponse, ApiError> {
    let row = state
        .oauth_service
        .oauth_repo
        .get_ota_config("oauth.enabled".to_string())
        .await
        .map_err(|e| ApiError::InternalServerError(format!("DB error: {}", e)))?;

    let value = row.map(|r| r.value).unwrap_or_else(|| "false".to_string());

    Ok(oauth_json(StatusCode::OK, json!({"oauth.enabled": value})))
}

// ---------------------------------------------------------------------------
// POST /oauth/authorize/grant  (JWT-authenticated user consent)
// ---------------------------------------------------------------------------

/// JSON body for the consent-page grant endpoint.
#[derive(Debug, Deserialize)]
pub struct GrantRequest {
    pub client_id: Option<String>,
    pub redirect_uri: Option<String>,
    pub scope: Option<String>,
    pub state: Option<String>,
    pub code_challenge: Option<String>,
    pub code_challenge_method: Option<String>,
}

/// Issue an authorization code after the user has consented on the web consent page.
///
/// Authentication: JWT Bearer token in `Authorization` header (same JWT as /auth/login).
/// The user is resolved from the token inline — no separate UserState is needed because
/// `OauthState` embeds an `OauthService` that already holds a `UserService`.
pub async fn authorize_grant(
    State(state): State<OauthState>,
    headers: HeaderMap,
    Json(body): Json<GrantRequest>,
) -> Result<impl IntoResponse, ApiError> {
    // --- JWT auth (mirrors middleware/auth.rs logic) ---
    let token = extract_bearer_token(&headers).ok_or_else(|| {
        ApiError::Unauthorized("missing or invalid Authorization header".to_string())
    })?;

    let token_data = state
        .oauth_service
        .user_service
        .retrieve_token_claims(&token)
        .map_err(|e| {
            use jsonwebtoken::errors::ErrorKind;
            match e.kind() {
                ErrorKind::ExpiredSignature => ApiError::Unauthorized("token expired".to_string()),
                _ => ApiError::Unauthorized("invalid token".to_string()),
            }
        })?;

    let user = state
        .oauth_service
        .user_service
        .user_repo
        .find_by_identifier(token_data.claims.identifier)
        .await
        .ok_or_else(|| ApiError::Unauthorized("user not found".to_string()))?;

    // --- Extract IP / UA for audit log ---
    let ip = extract_ip(&headers);
    let ua = extract_ua(&headers);

    // --- Validate required fields ---
    let client_id = require_field(&body.client_id, "client_id")?;
    let redirect_uri = require_field(&body.redirect_uri, "redirect_uri")?;
    let scope = require_field(&body.scope, "scope")?;
    let state_param = require_field(&body.state, "state")?;
    let code_challenge = require_field(&body.code_challenge, "code_challenge")?;
    let code_challenge_method =
        require_field(&body.code_challenge_method, "code_challenge_method")?;

    // --- Delegate to service ---
    let code = state
        .oauth_service
        .issue_authorization_code(AuthorizeGrantParams {
            user_id: user.id as i64,
            client_id,
            redirect_uri,
            scope,
            state: state_param,
            code_challenge,
            code_challenge_method,
            ip,
            user_agent: ua,
        })
        .await?;

    Ok(oauth_json(StatusCode::OK, json!({ "code": code })))
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn extract_ip(headers: &HeaderMap) -> Option<String> {
    headers
        .get("x-forwarded-for")
        .or_else(|| headers.get("x-real-ip"))
        .and_then(|v| v.to_str().ok())
        .map(|s| s.split(',').next().unwrap_or(s).trim().to_string())
}

fn extract_ua(headers: &HeaderMap) -> Option<String> {
    headers
        .get("user-agent")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
}

fn extract_bearer_token(headers: &HeaderMap) -> Option<String> {
    headers
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer "))
        .map(|s| s.trim().to_string())
}

fn require_field(opt: &Option<String>, name: &str) -> Result<String, ApiError> {
    opt.clone().filter(|s| !s.is_empty()).ok_or_else(|| {
        ApiError::BadRequest(format!(
            r#"{{"error":"invalid_request","error_description":"missing parameter: {}"}}"#,
            name
        ))
    })
}

/// Build a plain JSON response (no ApiResponse wrapper — OAuth error format differs from app format)
fn oauth_json(status: StatusCode, body: Value) -> impl IntoResponse {
    (
        status,
        [(
            axum::http::header::CONTENT_TYPE,
            "application/json; charset=utf-8",
        )],
        body.to_string(),
    )
}

/// Constant-time byte comparison (wraps `subtle::ConstantTimeEq`)
fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        // Pad the comparison to avoid length-timing leaks
        let dummy = vec![0u8; b.len()];
        let _ = dummy.ct_eq(b);
        return false;
    }
    a.ct_eq(b).into()
}
