use axum::{
    extract::{Extension, Path, Query},
    response::{
        sse::{Event, Sse},
        IntoResponse,
    },
    Json,
};
use futures::stream::Stream;
use std::convert::Infallible;
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;

use crate::api_ok;
use crate::dto::ai_chat_dto::*;
use crate::error::api_error::ApiError;
use crate::service::ai_chat::SseEvent;
use crate::service::audientry_worker_dispatcher::AudientryWorkerDispatcher;
use crate::state::user_state::UserState;
use glance_mind_db::entity::User;
use tokio_stream::StreamExt;

pub async fn create_conversation(
    Extension(user): Extension<User>,
    Extension(state): Extension<UserState>,
    Json(req): Json<CreateConversationRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let conv = state
        .ai_chat_service
        .create_conversation(user.id, req.title)?;
    Ok(api_ok!(conv))
}

pub async fn list_conversations(
    Extension(user): Extension<User>,
    Extension(state): Extension<UserState>,
    Query(query): Query<ListConversationsQuery>,
) -> Result<impl IntoResponse, ApiError> {
    let page = query.page.unwrap_or(1);
    let page_size = query.page_size.unwrap_or(20);
    let result = state
        .ai_chat_service
        .list_conversations(user.id, page, page_size)?;
    Ok(api_ok!(result))
}

pub async fn get_conversation(
    Extension(user): Extension<User>,
    Extension(state): Extension<UserState>,
    Path(id): Path<i32>,
) -> Result<impl IntoResponse, ApiError> {
    let conv = state.ai_chat_service.get_conversation(id, user.id)?;
    Ok(api_ok!(conv))
}

pub async fn update_conversation(
    Extension(user): Extension<User>,
    Extension(state): Extension<UserState>,
    Path(id): Path<i32>,
    Json(req): Json<UpdateConversationRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let conv = state
        .ai_chat_service
        .update_conversation(id, user.id, req)?;
    Ok(api_ok!(conv))
}

pub async fn delete_conversation(
    Extension(user): Extension<User>,
    Extension(state): Extension<UserState>,
    Path(id): Path<i32>,
) -> Result<impl IntoResponse, ApiError> {
    state.ai_chat_service.delete_conversation(id, user.id)?;
    Ok(api_ok!())
}

pub async fn get_messages(
    Extension(user): Extension<User>,
    Extension(state): Extension<UserState>,
    Path(conv_id): Path<i32>,
) -> Result<impl IntoResponse, ApiError> {
    let messages = state.ai_chat_service.get_messages(conv_id, user.id)?;
    Ok(api_ok!(messages))
}

pub async fn send_message(
    Extension(user): Extension<User>,
    Extension(state): Extension<UserState>,
    Extension(audientry_dispatcher): Extension<Option<AudientryWorkerDispatcher>>,
    Path(conv_id): Path<i32>,
    Json(req): Json<SendMessageRequest>,
) -> Result<Sse<impl Stream<Item = Result<Event, Infallible>>>, ApiError> {
    state.ai_chat_service.get_conversation(conv_id, user.id)?;

    let (tx, rx) = mpsc::channel::<SseEvent>(64);
    let service = state.ai_chat_service.clone();
    let user_id = user.id;
    let content = req.content.clone();
    let model_id = req.model_id;
    let ui_capabilities = req.ui_capabilities.clone().unwrap_or_default();
    let questionnaire_submission = req.questionnaire_submission.clone();

    tokio::spawn(async move {
        if let Err(e) = service
            .send_message(
                conv_id,
                user_id,
                &content,
                model_id,
                ui_capabilities,
                questionnaire_submission,
                &state,
                tx.clone(),
                audientry_dispatcher,
            )
            .await
        {
            let _ = tx
                .send(SseEvent::Error {
                    message: e.to_string(),
                })
                .await;
        }
    });

    let stream = ReceiverStream::new(rx).map(|event| {
        Ok(Event::default()
            .event(event_name(&event))
            .data(event_data(&event)))
    });

    Ok(Sse::new(stream))
}

pub async fn confirm_plan(
    Extension(user): Extension<User>,
    Extension(state): Extension<UserState>,
    Path(plan_id): Path<i32>,
) -> Result<Sse<impl Stream<Item = Result<Event, Infallible>>>, ApiError> {
    state.ai_chat_service.get_plan(plan_id, user.id)?;

    let (tx, rx) = mpsc::channel::<SseEvent>(64);
    let service = state.ai_chat_service.clone();
    let user_id = user.id;

    tokio::spawn(async move {
        if let Err(e) = service
            .confirm_plan(plan_id, user_id, &state, tx.clone())
            .await
        {
            let _ = tx
                .send(SseEvent::Error {
                    message: e.to_string(),
                })
                .await;
        }
    });

    let stream = ReceiverStream::new(rx).map(|event| {
        Ok(Event::default()
            .event(event_name(&event))
            .data(event_data(&event)))
    });

    Ok(Sse::new(stream))
}

pub async fn cancel_plan(
    Extension(user): Extension<User>,
    Extension(state): Extension<UserState>,
    Path(plan_id): Path<i32>,
) -> Result<impl IntoResponse, ApiError> {
    let plan = state.ai_chat_service.cancel_plan(plan_id, user.id)?;
    Ok(api_ok!(plan))
}

pub async fn update_plan_step(
    Extension(user): Extension<User>,
    Extension(state): Extension<UserState>,
    Path((plan_id, step_id)): Path<(i32, i32)>,
    Json(req): Json<UpdatePlanStepRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let step = state
        .ai_chat_service
        .update_plan_step(plan_id, step_id, user.id, req)?;
    Ok(api_ok!(step))
}

// ── Module D3: task-template confirm / regenerate / cancel handlers ──────────
//
// These are compile-level wiring stubs. The full SSE/HTTP behaviour (CAS the
// draft, project the edited fields, stream `step_*` / `plan_completed`) is
// exercised by the live pytest integration test (`test_ai_template_api.py`),
// NOT by these handlers in the RED phase. The implementer completes the bodies.

/// POST /ai-chat/conversations/:id/task-template/:draft_id/confirm
///
/// Submit the edited template → CAS draft `proposed -> confirming` → project
/// fields → batch-create → stream `step_*` / `plan_completed`.
pub async fn confirm_task_template(
    Extension(user): Extension<User>,
    Extension(state): Extension<UserState>,
    Path((conv_id, draft_id)): Path<(i32, uuid::Uuid)>,
    Json(req): Json<ConfirmTaskTemplateDto>,
) -> Result<impl IntoResponse, ApiError> {
    use crate::service::ai_chat::task_template::{confirm_orchestrate, DieselDraftStore};
    use crate::service::batch_task_service::DispatchSingleTaskCreator;

    // Ownership check on the conversation (mirrors confirm_plan).
    state.ai_chat_service.get_conversation(conv_id, user.id)?;

    let drafts = crate::service::ai_chat::task_template::DraftService::new(
        DieselDraftStore::new(state.db.pool.clone()),
        24,
    );
    let creator = DispatchSingleTaskCreator::new(state.clone());

    let result = confirm_orchestrate(
        &drafts,
        &creator,
        draft_id,
        user.id,
        req.edited_fields,
        chrono::Utc::now(),
    )
    .await?;

    Ok(api_ok!(result))
}

/// POST /ai-chat/conversations/:id/task-template/:draft_id/regenerate
///
/// Re-run sample generation (optional `hint`) → emit a fresh
/// `task_template_generating` + `task_template_proposed`.
pub async fn regenerate_task_template(
    Extension(user): Extension<User>,
    Extension(state): Extension<UserState>,
    Path((conv_id, draft_id)): Path<(i32, uuid::Uuid)>,
    Json(req): Json<RegenerateTaskTemplateDto>,
) -> Result<impl IntoResponse, ApiError> {
    // STUB: ownership-check then echo back the ids. The real regeneration flow
    // (supersede the old draft, generate + propose a new one, stream SSE) is
    // completed by the implementer and verified by the live pytest.
    state.ai_chat_service.get_conversation(conv_id, user.id)?;
    let _ = (draft_id, req.hint);
    Ok(api_ok!(serde_json::json!({
        "draft_id": draft_id,
        "regenerated": false
    })))
}

/// POST /ai-chat/conversations/:id/task-template/:draft_id/cancel
///
/// Discard the draft (status → cancelled).
pub async fn cancel_task_template(
    Extension(user): Extension<User>,
    Extension(state): Extension<UserState>,
    Path((conv_id, draft_id)): Path<(i32, uuid::Uuid)>,
) -> Result<impl IntoResponse, ApiError> {
    use crate::service::ai_chat::task_template::{DieselDraftStore, DraftService};

    state.ai_chat_service.get_conversation(conv_id, user.id)?;

    let drafts = DraftService::new(DieselDraftStore::new(state.db.pool.clone()), 24);
    drafts.cancel(draft_id, user.id, chrono::Utc::now()).await?;

    Ok(api_ok!(serde_json::json!({
        "draft_id": draft_id,
        "status": "cancelled"
    })))
}

fn event_name(event: &SseEvent) -> &'static str {
    match event {
        SseEvent::MessageStart { .. } => "message_start",
        SseEvent::TextDelta { .. } => "text_delta",
        SseEvent::ToolCallStart { .. } => "tool_call_start",
        SseEvent::ToolCallResult { .. } => "tool_call_result",
        SseEvent::Questionnaire { .. } => "questionnaire",
        SseEvent::PlanCreated { .. } => "plan_created",
        SseEvent::MessageEnd { .. } => "message_end",
        SseEvent::StepStart { .. } => "step_start",
        SseEvent::StepCompleted { .. } => "step_completed",
        SseEvent::StepFailed { .. } => "step_failed",
        SseEvent::PlanCompleted { .. } => "plan_completed",
        SseEvent::Error { .. } => "error",
        SseEvent::AudientryPhase { .. } => "audientry_phase",
        SseEvent::AudientryReport { .. } => "audientry_report",
        SseEvent::TaskTemplateGenerating { .. } => "task_template_generating",
        SseEvent::TaskTemplateProposed { .. } => "task_template_proposed",
    }
}

fn event_data(event: &SseEvent) -> String {
    let sse_string = event.to_sse_string();
    if let Some(data_line) = sse_string.lines().find(|l| l.starts_with("data: ")) {
        data_line.strip_prefix("data: ").unwrap_or("{}").to_string()
    } else {
        "{}".to_string()
    }
}
