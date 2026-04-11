use crate::handler::{material_folder_handler, material_handler};
use crate::state::user_state::UserState;
use axum::{
    routing::{get, post, put},
    Router,
};

pub fn material_routes() -> Router<UserState> {
    Router::new()
        // User materials management
        .route(
            "/materials",
            get(material_handler::list_materials).post(material_handler::create_material),
        )
        .route("/materials/upload", post(material_handler::upload_material))
        .route(
            "/materials/:id",
            get(material_handler::get_material)
                .put(material_handler::update_material)
                .delete(material_handler::delete_material),
        )
        // Re-analyze material prompt
        .route(
            "/materials/:id/analyze",
            post(material_handler::re_analyze_material),
        )
        // Collect tags from video_cases
        .route("/material-tags", get(material_handler::list_tags))
        // Favorite from video_case
        .route(
            "/video-cases/:task_no/favorite",
            post(material_handler::favorite_from_video_case),
        )
        // Material folders
        .route(
            "/material-folders",
            get(material_folder_handler::list_folders).post(material_folder_handler::create_folder),
        )
        .route(
            "/material-folders/tree",
            get(material_folder_handler::get_folder_tree),
        )
        .route(
            "/material-folders/:id",
            put(material_folder_handler::update_folder)
                .delete(material_folder_handler::delete_folder),
        )
}
