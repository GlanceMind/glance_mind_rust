mod config;
mod extractor;
mod manager;
mod types;

pub use manager::{ChargingContext, ChargingManager};
pub use types::ActionType;

use axum::{
    body::Body,
    extract::{OriginalUri, State},
    http::Request,
    middleware::Next,
    response::Response,
    Extension,
};

use crate::error::api_error::ApiError;
use crate::state::user_state::UserState;
use glance_mind_db::entity::user::User;

#[derive(Clone, Debug)]
pub struct ChargingUser {
    pub user_id: i32,
}

pub async fn charging_middleware(
    OriginalUri(original_uri): OriginalUri,
    State(state): State<UserState>,
    Extension(user): Extension<User>,
    req: Request<Body>,
    next: Next<Body>,
) -> Result<Response, ApiError> {
    // Use original full path instead of nested relative path
    let route = original_uri.path().to_string();

    tracing::info!(
        "[CHARGING] Middleware entry - route: {}, user: {}",
        route,
        user.id
    );
    tracing::info!("[CHARGING] Original full URI: {}", original_uri);
    tracing::info!("[CHARGING] Relative URI: {}", req.uri());

    // 1. Find route configuration
    let extractor = match config::find_extractor(&route) {
        Some(ext) => {
            tracing::info!("[CHARGING] Found charging config - route: {}", route);
            ext
        }
        None => {
            // Not in charging route list, pass through
            tracing::warn!(
                "[CHARGING] Route {} not in charging list, passing through",
                route
            );
            tracing::info!(
                "[CHARGING] Currently registered charging routes: {:?}",
                config::get_registered_routes()
            );
            return Ok(next.run(req).await);
        }
    };

    tracing::info!(
        "[CHARGING] Route {} requires charging, starting processing",
        route
    );

    // 2. Extract charging parameters (if required header is missing, extract returns None)
    let params = extractor.extract(&req).await.ok_or_else(|| {
        let error_msg = match route.as_str() {
            r if r.contains("/scan/post") => "Missing required X-PLATFORM-ID header".to_string(),
            _ => "Cannot extract charging parameters, please check required headers".to_string(),
        };
        tracing::warn!(
            "Charging parameter extraction failed - route: {}, reason: {}",
            route,
            error_msg
        );
        ApiError::BadRequest(error_msg)
    })?;

    tracing::debug!(
        "Extracted charging parameters: action_type={:?}, ai_model_id={:?}, platform_id={:?}",
        params.action_type,
        params.ai_model_id,
        params.platform_id
    );

    // 3. Prepare charging (calculate cost + check balance)
    tracing::info!(
        "[CHARGING] Preparing charge - user: {}, action_type: {:?}",
        user.id,
        params.action_type
    );

    let charging_context = state
        .charging_manager
        .prepare_charging(
            user.id,
            params.action_type,
            params.ai_model_id,
            params.platform_id,
        )
        .await?;

    tracing::info!(
        "[CHARGING] Charging preparation complete - user: {}, cost: {} points (base: {} x multiplier: {})",
        user.id,
        charging_context.final_cost,
        charging_context.base_cost,
        charging_context.cost_multiplier
    );

    // 4. Inject context into request extensions
    let mut req = req;
    req.extensions_mut().insert(charging_context.clone());
    req.extensions_mut()
        .insert(ChargingUser { user_id: user.id });

    // 5. Execute business logic
    tracing::info!("[CHARGING] Executing business logic - route: {}", route);
    let response = next.run(req).await;

    // 6. Check response status code
    let status = response.status();
    tracing::info!(
        "[CHARGING] Business logic execution complete - status: {}",
        status
    );

    if status.is_success() {
        // 7. Execute charge deduction
        tracing::info!(
            "[CHARGING] API execution successful, starting charge - user: {}, amount: {}",
            user.id,
            charging_context.final_cost
        );

        match state
            .charging_manager
            .execute_charging(user.id, &charging_context, None)
            .await
        {
            Ok(transaction) => {
                tracing::info!(
                    "[CHARGING] Charge successful - transaction_id: {}, amount: {}, user: {}",
                    transaction.id,
                    transaction.amount,
                    user.id
                );
            }
            Err(e) => {
                tracing::error!(
                    "[CHARGING] Charge failed - user: {}, error: {:?}",
                    user.id,
                    e
                );
                // Charge failed but API executed successfully, log error but don't affect response
                // Production environment may need more complex handling (retry, alert, etc.)
                return Err(e);
            }
        }
    } else {
        tracing::warn!(
            "[CHARGING] API execution failed (status: {}), not charging",
            status
        );
    }

    tracing::info!("[CHARGING] Middleware finished - route: {}", route);
    Ok(response)
}
