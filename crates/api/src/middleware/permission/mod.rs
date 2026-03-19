pub mod config;

use axum::{
    body::Body, extract::OriginalUri, http::Request, middleware::Next, response::Response,
    Extension,
};

use crate::error::api_error::ApiError;
use glance_mind_db::entity::user::User;

/// Permission middleware – zero DB queries.
///
/// Reads `user.permissions` (already loaded by auth middleware) and checks
/// the required bit for the matched route prefix.
pub async fn permission_middleware(
    OriginalUri(original_uri): OriginalUri,
    Extension(user): Extension<User>,
    req: Request<Body>,
    next: Next<Body>,
) -> Result<Response, ApiError> {
    let route = original_uri.path();

    let required = match config::find_required_permission(route) {
        Some(perm) => perm,
        None => return Ok(next.run(req).await),
    };

    if !required.check(user.permissions) {
        tracing::warn!(
            "[PERMISSION] Denied: user {} lacks '{}' (bits={}) for {}",
            user.id,
            required.name(),
            user.permissions,
            route
        );
        return Err(ApiError::PermissionDenied(required.name().to_string()));
    }

    Ok(next.run(req).await)
}
