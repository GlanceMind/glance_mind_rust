use crate::handler::template_handler;
use crate::state::user_state::UserState;
use axum::{
    routing::{get, post},
    Router,
};

pub fn routes() -> Router<UserState> {
    Router::new()
        // Nested under campaigns for create/list
        .route(
            "/campaigns/:id/templates",
            get(template_handler::list_templates).post(template_handler::create_template),
        )
        // Auto-generate route - specific path must come before param route
        .route(
            "/templates/auto-generate",
            post(template_handler::auto_generate),
        )
        // All templates for user
        .route("/templates", get(template_handler::list_all_templates))
        // Top-level for get/update/delete specific template
        .route(
            "/templates/:id",
            get(template_handler::get_template)
                .put(template_handler::update_template)
                .delete(template_handler::delete_template),
        )
        .route(
            "/reply-template-library",
            get(template_handler::list_reusable_templates)
                .post(template_handler::create_reusable_template),
        )
        .route(
            "/reply-template-library/:id",
            get(template_handler::get_reusable_template)
                .put(template_handler::update_reusable_template)
                .delete(template_handler::delete_reusable_template),
        )
        .route(
            "/reply-template-library/:id/assign",
            post(template_handler::assign_reusable_template),
        )
}
