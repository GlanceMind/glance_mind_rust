use crate::error::{api_error::ApiError, token_error::TokenError, user_error::UserError};
use crate::repository::user_repository::UserRepositoryTrait;
// TokenServiceTrait removed
use crate::state::user_state::UserState;
use axum::extract::State;
use axum::headers::authorization::{Authorization, Bearer};
use axum::headers::Header;
use axum::{http, http::Request, middleware::Next, response::IntoResponse};
use jsonwebtoken::errors::ErrorKind;

pub async fn auth<B>(
    State(state): State<UserState>,
    mut req: Request<B>,
    next: Next<B>,
) -> Result<impl IntoResponse, ApiError> {
    let mut headers = req
        .headers_mut()
        .iter()
        .filter_map(|(header_name, header_value)| {
            if header_name == http::header::AUTHORIZATION {
                return Some(header_value);
            }
            None
        });

    let header: Authorization<Bearer> =
        Authorization::decode(&mut headers).map_err(|_| TokenError::MissingToken)?;
    let token = header.token();
    match state.user_service.retrieve_token_claims(token) {
        Ok(token_data) => {
            let user = state
                .user_service
                .user_repo
                .find_by_identifier(token_data.claims.identifier)
                .await;
            match user {
                Some(user) => {
                    req.extensions_mut().insert(user);
                    Ok(next.run(req).await)
                }
                None => Err(UserError::UserNotFound)?,
            }
        }
        Err(err) => match err.kind() {
            ErrorKind::ExpiredSignature => Err(TokenError::TokenExpired)?,
            _ => {
                tracing::warn!("JWT validation failed: {:?}", err.kind());
                Err(TokenError::InvalidToken)?
            }
        },
    }
}
