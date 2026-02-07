use crate::handler::material_handler;
use crate::state::user_state::UserState;
use axum::{
    routing::{get, post},
    Router,
};

pub fn material_routes() -> Router<UserState> {
    Router::new()
        // User materials management
        .route("/materials", get(material_handler::list_materials).post(material_handler::create_material))
        .route(
            "/materials/:id",
            get(material_handler::get_material)
                .put(material_handler::update_material)
                .delete(material_handler::delete_material),
        )
        // Collect tags from video_cases
        .route("/material-tags", get(material_handler::list_tags))
        // Favorite from video_case
        .route("/video-cases/:task_no/favorite", post(material_handler::favorite_from_video_case))
}
