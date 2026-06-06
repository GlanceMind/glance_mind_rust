use axum::{
    routing::{delete, get, patch, post},
    Router,
};

use crate::handler::ai_chat_handler;
use crate::state::user_state::UserState;

pub fn routes() -> Router<UserState> {
    Router::new()
        .route("/conversations", post(ai_chat_handler::create_conversation))
        .route("/conversations", get(ai_chat_handler::list_conversations))
        .route("/conversations/:id", get(ai_chat_handler::get_conversation))
        .route(
            "/conversations/:id",
            patch(ai_chat_handler::update_conversation),
        )
        .route(
            "/conversations/:id",
            delete(ai_chat_handler::delete_conversation),
        )
        .route(
            "/conversations/:id/messages",
            get(ai_chat_handler::get_messages),
        )
        .route(
            "/conversations/:id/messages",
            post(ai_chat_handler::send_message),
        )
        .route("/plans/:id/confirm", post(ai_chat_handler::confirm_plan))
        .route("/plans/:id/cancel", post(ai_chat_handler::cancel_plan))
        .route(
            "/plans/:plan_id/steps/:step_id",
            patch(ai_chat_handler::update_plan_step),
        )
        // Module D3: task-template confirm / regenerate / cancel. Routed with
        // `Path<(i32, Uuid)>` = (conversation_id, draft_id).
        .route(
            "/conversations/:id/task-template/:draft_id/confirm",
            post(ai_chat_handler::confirm_task_template),
        )
        .route(
            "/conversations/:id/task-template/:draft_id/regenerate",
            post(ai_chat_handler::regenerate_task_template),
        )
        .route(
            "/conversations/:id/task-template/:draft_id/cancel",
            post(ai_chat_handler::cancel_task_template),
        )
}
