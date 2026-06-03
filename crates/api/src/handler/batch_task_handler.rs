//! Module C: Batch Create API handler.
//!
//! Thin skeleton: validates the body (via `ValidatedRequest`, which enforces a
//! non-empty `idempotency_key`) and delegates to [`BatchTaskService`]. Prod
//! wiring is intentionally minimal here — the Diesel dedupe store and the
//! single-task creator used in production are wrong stubs during the RED phase.

use crate::api_ok;
use crate::dto::batch_task_dto::BatchCreateTasksDto;
use crate::error::api_error::ApiError;
use crate::error::request_error::ValidatedRequest;
use crate::service::batch_task_service::{
    BatchTaskService, DieselBatchDedupeStore, DispatchSingleTaskCreator,
};
use crate::state::user_state::UserState;
use axum::{extract::State, response::IntoResponse, Extension};
use glance_mind_db::entity::user::User;

/// POST /ai-tasks/batch — create a single-item batch.
pub async fn create_batch(
    State(state): State<UserState>,
    Extension(user): Extension<User>,
    ValidatedRequest(payload): ValidatedRequest<BatchCreateTasksDto>,
) -> Result<impl IntoResponse, ApiError> {
    let service = BatchTaskService::new(
        DispatchSingleTaskCreator::new(state.clone()),
        DieselBatchDedupeStore::new(state.db.pool.clone()),
    );
    let result = service.create_batch(user.id, payload).await?;
    Ok(api_ok!(result))
}
