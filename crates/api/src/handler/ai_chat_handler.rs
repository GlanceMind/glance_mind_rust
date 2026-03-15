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
use crate::state::user_state::UserState;
use glance_mind_db::entity::User;
use tokio_stream::StreamExt;

pub async fn create_conversation(
    Extension(user): Extension<User>,
    Extension(state): Extension<UserState>,
    Json(req): Json<CreateConversationRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let conv = state.ai_chat_service.create_conversation(user.id, req.title)?;
    Ok(api_ok!(conv))
}

pub async fn list_conversations(
    Extension(user): Extension<User>,
    Extension(state): Extension<UserState>,
    Query(query): Query<ListConversationsQuery>,
) -> Result<impl IntoResponse, ApiError> {
    let page = query.page.unwrap_or(1);
    let page_size = query.page_size.unwrap_or(20);
    let result = state.ai_chat_service.list_conversations(user.id, page, page_size)?;
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
    let conv = state.ai_chat_service.update_conversation(id, user.id, req)?;
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
            )
            .await
        {
            let _ = tx.send(SseEvent::Error { message: e.to_string() }).await;
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
        if let Err(e) = service.confirm_plan(plan_id, user_id, &state, tx.clone()).await {
            let _ = tx.send(SseEvent::Error { message: e.to_string() }).await;
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
    let step = state.ai_chat_service.update_plan_step(plan_id, step_id, user.id, req)?;
    Ok(api_ok!(step))
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
