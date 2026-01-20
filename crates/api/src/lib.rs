use axum::Router;
use std::sync::Arc;

pub mod config;
pub mod dto;
pub mod error;
pub mod handler;
pub mod middleware;
pub mod repository;
pub mod response;
pub mod routes;
pub mod service;
pub mod state;

// Re-export schema and entity from glance_mind_db for convenience
pub use glance_mind_db::entity;
pub use glance_mind_db::schema;

use config::database::Database;
use config::parameter;

pub fn app(db: Arc<Database>) -> Router {
    routes::root::routes(db)
}

// NOTE: Database migrations are now managed centrally in glance_mind_db
// Run migrations using: cd ../glance_mind_db && diesel migration run
// Or use: cargo run --bin db-migrate

pub async fn run() {
    // Initialize tracing first
    tracing_subscriber::fmt::init();

    parameter::init();
    let db = Arc::new(Database::new());

    // Migrations are now managed by glance_mind_db repository
    // To run migrations, use one of these methods:
    // 1. cd ../glance_mind_db && diesel migration run
    // 2. cd ../glance_mind_db && cargo run --bin db-migrate
    // 3. CI/CD pipeline will automatically run migrations on deployment
    tracing::info!("📦 Database migrations managed by glance_mind_db");

    let host = format!("0.0.0.0:{}", parameter::get("PORT"));
    tracing::info!("🚀 Starting server on {}", host);
    axum::Server::bind(&host.parse().unwrap())
        .serve(app(db).into_make_service())
        .await
        .unwrap_or_else(|e| panic!("Server error: {}", e));
}
