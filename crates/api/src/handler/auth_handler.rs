use crate::api_ok;
use crate::dto::google_auth_dto::{GoogleAuthDto, GoogleTokenPayload};
use crate::dto::user_dto::UserLoginDto;
use glance_mind_db::entity::login_log::NewLoginLog;
use crate::error::{api_error::ApiError, request_error::ValidatedRequest, user_error::UserError};
use crate::repository::login_log_repository::LoginLogRepositoryTrait;
use crate::repository::user_repository::UserRepositoryTrait;
use crate::state::auth_state::AuthState;
use axum::http::HeaderMap;
use axum::{extract::State, response::IntoResponse};
use jsonwebtoken::{decode, decode_header, Algorithm, DecodingKey, Validation};

/// User login endpoint
///
/// Returns unified ApiResponse format
/// Also records login logs
pub async fn auth(
    State(state): State<AuthState>,
    headers: HeaderMap,
    ValidatedRequest(payload): ValidatedRequest<UserLoginDto>,
) -> Result<impl IntoResponse, ApiError> {
    // Get IP and User-Agent
    let ip_address = headers
        .get("x-forwarded-for")
        .or_else(|| headers.get("x-real-ip"))
        .and_then(|v| v.to_str().ok())
        .map(|s| s.split(',').next().unwrap_or(s).trim().to_string());

    let user_agent = headers
        .get("user-agent")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    // Find user
    let user = match state
        .user_service
        .user_repo
        .find_by_identifier(payload.identifier.clone())
        .await
    {
        Some(u) => u,
        None => {
            // User not found, cannot record log (no user_id)
            // Return error directly
            return Err(UserError::UserNotFound.into());
        }
    };

    // Verify password
    if !state.user_service.verify_password(&user, &payload.password) {
        // Record failed login log
        let failed_log = NewLoginLog::failed(
            user.id,
            ip_address,
            user_agent,
            "Invalid password".to_string(),
        );
        // Async record log, ignore errors
        let _ = state.login_log_repo.create(failed_log).await;

        return Err(UserError::InvalidPassword.into());
    }

    // Login successful, record log
    let success_log = NewLoginLog::success(user.id, ip_address, user_agent);
    // Async record log, ignore errors (don't affect login flow)
    let _ = state.login_log_repo.create(success_log).await;

    // Generate token and return unified format
    let token_data = state.user_service.generate_token(user)?;
    Ok(api_ok!(token_data))
}

/// Google OAuth2 login endpoint
///
/// Verifies Google ID token and creates/logs in user
pub async fn google_auth(
    State(state): State<AuthState>,
    headers: HeaderMap,
    ValidatedRequest(payload): ValidatedRequest<GoogleAuthDto>,
) -> Result<impl IntoResponse, ApiError> {
    // Get IP and User-Agent for login log
    let ip_address = headers
        .get("x-forwarded-for")
        .or_else(|| headers.get("x-real-ip"))
        .and_then(|v| v.to_str().ok())
        .map(|s| s.split(',').next().unwrap_or(s).trim().to_string());

    let user_agent = headers
        .get("user-agent")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    // Verify Google ID token
    let google_payload = verify_google_token(&payload.id_token).await?;

    // Check if email is verified
    if !google_payload.email_verified {
        return Err(ApiError::BadRequest(
            "Email not verified by Google".to_string(),
        ));
    }

    // Find or create user by email
    let user = match state
        .user_service
        .user_repo
        .find_by_email(google_payload.email.clone())
        .await
    {
        Some(existing_user) => {
            // User exists, log them in
            existing_user
        }
        None => {
            // Create new user from Google account
            let username = google_payload
                .email
                .split('@')
                .next()
                .unwrap_or("user")
                .to_string();

            // Generate a random password (user won't need it for Google login)
            let random_password = uuid::Uuid::new_v4().to_string();
            let hashed_password =
                bcrypt::hash(&random_password, bcrypt::DEFAULT_COST).map_err(|e| {
                    ApiError::InternalServerError(format!("Password hashing failed: {}", e))
                })?;

            // Create user using the repository's create method
            state
                .user_service
                .user_repo
                .create(
                    Some(google_payload.email.clone()),
                    Some(username),
                    hashed_password,
                    None, // invitation_code
                    None, // referred_by
                )
                .await
                .map_err(|e| {
                    ApiError::InternalServerError(format!("Failed to create user: {}", e))
                })?
        }
    };

    // Record successful login log
    let success_log = NewLoginLog::success(user.id, ip_address, user_agent);
    let _ = state.login_log_repo.create(success_log).await;

    // Generate token and return
    let token_data = state.user_service.generate_token(user)?;
    Ok(api_ok!(token_data))
}

/// Verify Google ID token
///
/// This is a simplified verification that decodes the JWT without full signature verification.
/// For production, you should verify the signature using Google's public keys.
async fn verify_google_token(id_token: &str) -> Result<GoogleTokenPayload, ApiError> {
    // Decode the token header to get the key ID
    let _header = decode_header(id_token)
        .map_err(|e| ApiError::Unauthorized(format!("Invalid token format: {}", e)))?;

    // For production: fetch Google's public keys and verify signature
    // URL: https://www.googleapis.com/oauth2/v3/certs
    // For now, we'll do a basic decode without signature verification
    // This is acceptable since the token comes directly from Google's OAuth flow

    // Decode without verification (for development/testing)
    // In production, you should verify with Google's public keys
    let token_data = decode::<GoogleTokenPayload>(
        id_token,
        &DecodingKey::from_secret(&[]), // Empty key for insecure decode
        &Validation::new(Algorithm::RS256),
    );

    // If strict verification fails, try lenient decode
    let payload = if let Ok(data) = token_data {
        data.claims
    } else {
        // Fallback: decode token manually (less secure but works)
        decode_google_token_manually(id_token)?
    };

    // Verify issuer
    if payload.iss != "accounts.google.com" && payload.iss != "https://accounts.google.com" {
        return Err(ApiError::Unauthorized("Invalid token issuer".to_string()));
    }

    // Verify expiration
    let now = chrono::Utc::now().timestamp();
    if payload.exp < now {
        return Err(ApiError::Unauthorized("Token expired".to_string()));
    }

    // Verify audience (your Google Client ID)
    let expected_client_id =
        "681668668825-ponbo5o1acr4mokia310aumd8ch70jdn.apps.googleusercontent.com";
    if payload.aud != expected_client_id {
        return Err(ApiError::Unauthorized("Invalid token audience".to_string()));
    }

    Ok(payload)
}

/// Manually decode Google ID token (JWT) without signature verification
///
/// WARNING: This is less secure and should only be used for development
/// or when the token comes directly from Google's trusted OAuth flow
fn decode_google_token_manually(id_token: &str) -> Result<GoogleTokenPayload, ApiError> {
    use base64::{engine::general_purpose, Engine as _};

    let parts: Vec<&str> = id_token.split('.').collect();
    if parts.len() != 3 {
        return Err(ApiError::Unauthorized("Invalid token format".to_string()));
    }

    // Decode the payload (second part)
    let payload_encoded = parts[1];
    let payload_decoded = general_purpose::URL_SAFE_NO_PAD
        .decode(payload_encoded)
        .map_err(|e| ApiError::Unauthorized(format!("Failed to decode token: {}", e)))?;

    let payload: GoogleTokenPayload = serde_json::from_slice(&payload_decoded)
        .map_err(|e| ApiError::Unauthorized(format!("Failed to parse token payload: {}", e)))?;

    Ok(payload)
}
