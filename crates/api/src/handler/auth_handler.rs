use crate::api_ok;
use crate::dto::google_auth_dto::{GoogleAuthDto, GoogleTokenPayload};
use crate::dto::user_dto::UserLoginDto;
use crate::error::{api_error::ApiError, request_error::ValidatedRequest, user_error::UserError};
use crate::repository::login_log_repository::LoginLogRepositoryTrait;
use crate::repository::user_repository::UserRepositoryTrait;
use crate::state::auth_state::AuthState;
use axum::http::HeaderMap;
use axum::{extract::State, response::IntoResponse};
use glance_mind_db::entity::login_log::NewLoginLog;
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

/// Verify Google ID token with proper signature verification
///
/// This function fetches Google's public keys and verifies the JWT signature
async fn verify_google_token(id_token: &str) -> Result<GoogleTokenPayload, ApiError> {
    // 1. Decode token header to get key ID (kid)
    let header = decode_header(id_token)
        .map_err(|e| ApiError::Unauthorized(format!("Invalid token format: {}", e)))?;

    let kid = header
        .kid
        .ok_or_else(|| ApiError::Unauthorized("Missing key ID in token header".to_string()))?;

    // 2. Fetch Google's public keys (JWKS)
    let jwks_url = "https://www.googleapis.com/oauth2/v3/certs";
    let jwks_response = reqwest::get(jwks_url)
        .await
        .map_err(|e| ApiError::InternalServerError(format!("Failed to fetch JWKS: {}", e)))?
        .json::<serde_json::Value>()
        .await
        .map_err(|e| ApiError::InternalServerError(format!("Failed to parse JWKS: {}", e)))?;

    // 3. Find the matching key from JWKS
    let jwks_keys = jwks_response["keys"]
        .as_array()
        .ok_or_else(|| ApiError::Unauthorized("Invalid JWKS format".to_string()))?;

    let matching_key = jwks_keys
        .iter()
        .find(|key| key["kid"].as_str() == Some(&kid))
        .ok_or_else(|| ApiError::Unauthorized("Key ID not found in JWKS".to_string()))?;

    // 4. Extract RSA components (n and e)
    let n = matching_key["n"]
        .as_str()
        .ok_or_else(|| ApiError::Unauthorized("Missing 'n' in JWK".to_string()))?;
    let e = matching_key["e"]
        .as_str()
        .ok_or_else(|| ApiError::Unauthorized("Missing 'e' in JWK".to_string()))?;

    // 5. Create decoding key from RSA components
    let decoding_key = DecodingKey::from_rsa_components(n, e)
        .map_err(|e| ApiError::Unauthorized(format!("Invalid RSA key: {}", e)))?;

    // 6. Configure validation
    let expected_client_id = std::env::var("GOOGLE_CLIENT_ID").unwrap_or_else(|_| {
        "681668668825-ponbo5o1acr4mokia310aumd8ch70jdn.apps.googleusercontent.com".to_string()
    });

    let mut validation = Validation::new(Algorithm::RS256);
    validation.set_audience(&[&expected_client_id]);
    validation.set_issuer(&["accounts.google.com", "https://accounts.google.com"]);

    // 7. Verify signature and decode payload
    let token_data =
        decode::<GoogleTokenPayload>(id_token, &decoding_key, &validation).map_err(|e| {
            tracing::error!("Google token verification failed: {:?}", e);
            ApiError::Unauthorized(format!("Invalid Google token: {}", e))
        })?;

    // 8. Additional validation
    let payload = token_data.claims;

    // Verify expiration (should already be checked by jsonwebtoken, but double-check)
    let now = chrono::Utc::now().timestamp();
    if payload.exp < now {
        return Err(ApiError::Unauthorized("Token expired".to_string()));
    }

    tracing::info!(
        "Google OAuth token verified successfully for email: {}",
        payload.email
    );
    Ok(payload)
}
