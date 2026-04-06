use axum::{
    routing::{get, post, put},
    Router,
};

use crate::handler::{drama_callback_handler, drama_handler, drama_stream_handler};
use crate::state::user_state::UserState;

pub fn routes() -> Router<UserState> {
    Router::new()
        .route("/upload-image", post(drama_handler::upload_image))
        .route("/preflight", post(drama_handler::preflight))
        .route(
            "/characters",
            get(drama_handler::list_characters).post(drama_handler::create_character),
        )
        .route(
            "/characters/:id",
            put(drama_handler::update_character).delete(drama_handler::delete_character),
        )
        .route(
            "/scene-assets",
            get(drama_handler::list_scene_assets).post(drama_handler::create_scene_asset),
        )
        .route(
            "/scene-assets/:id",
            put(drama_handler::update_scene_asset).delete(drama_handler::delete_scene_asset),
        )
        .route(
            "/style-assets",
            get(drama_handler::list_style_assets).post(drama_handler::create_style_asset),
        )
        .route(
            "/style-assets/:id",
            put(drama_handler::update_style_asset).delete(drama_handler::delete_style_asset),
        )
        .route(
            "/projects",
            post(drama_handler::create_project).get(drama_handler::list_projects),
        )
        .route(
            "/projects/:id",
            get(drama_handler::get_project).delete(drama_handler::cancel_project),
        )
        .route(
            "/projects/:project_id/meta",
            get(drama_handler::get_project_meta).put(drama_handler::upsert_project_meta),
        )
        .route(
            "/projects/:id/resources",
            get(drama_handler::get_project_resources).put(drama_handler::put_project_resources),
        )
        .route(
            "/projects/:project_id/chapters/:chapter_id/scene-assets",
            get(drama_handler::get_chapter_scene_assets).put(drama_handler::put_chapter_scene_assets),
        )
        .route("/projects/:id/clarify", post(drama_handler::clarify))
        .route(
            "/projects/:id/stages/strategy/select",
            post(drama_handler::strategy_select),
        )
        .route("/projects/:id/approve", post(drama_handler::approve))
        .route("/projects/:id/script", get(drama_handler::get_script))
        .route("/projects/:id/shots", get(drama_handler::get_shots))
        .route("/projects/:id/render", get(drama_handler::get_render))
        .route(
            "/projects/:id/artifacts",
            get(drama_handler::get_artifacts),
        )
        .route("/projects/:id/cost", get(drama_handler::get_cost))
        .route("/projects/:id/fallbacks", get(drama_handler::get_fallbacks))
        .route(
            "/projects/:id/stream",
            get(drama_stream_handler::stream_project),
        )
        // H4: script/feedback as separate revision surface
        .route(
            "/projects/:id/script/feedback",
            post(drama_handler::script_feedback),
        )
        // H5: retry and clone
        .route("/projects/:id/retry", post(drama_handler::retry_project))
        .route("/projects/:id/clone", post(drama_handler::clone_project))
        // H7: scene-level rerun
        .route(
            "/projects/:id/scenes/:scene_id/rerun",
            post(drama_handler::scene_rerun),
        )
}

pub fn internal_routes() -> Router<UserState> {
    Router::new()
        .route("/callback", post(drama_callback_handler::ingest_callback))
        .route(
            "/replay/:project_id",
            post(drama_callback_handler::request_replay),
        )
}
