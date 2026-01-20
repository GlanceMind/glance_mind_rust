use axum::{
    routing::{get, post},
    Router,
};
use std::sync::Arc;

use crate::{config::database::Database, handler::notification_handler};

pub fn notification_routes(db_conn: Arc<Database>) -> Router {
    Router::new()
        .route("/", get(notification_handler::get_notifications))
        .route("/unread-count", get(notification_handler::get_unread_count))
        .route("/:id/read", post(notification_handler::mark_as_read))
        .route("/read-all", post(notification_handler::mark_all_as_read))
        .with_state(db_conn)
}
