use axum::Router;
use std::sync::Arc;

pub mod config;
pub mod dto;
pub mod error;
pub mod handler;
pub mod middleware;
pub mod platform_routing;
pub mod protocol_gen;
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
use service::nats_dm_service::NatsDmService;
use service::redis_service::RedisService;

pub fn app(
    db: Arc<Database>,
    nats_dm_service: Option<NatsDmService>,
    redis_service: Option<RedisService>,
) -> Router {
    routes::root::routes(db, nats_dm_service, redis_service)
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
    tracing::info!("Database migrations managed by glance_mind_db");

    // Initialize NATS connection for DM group control (optional)
    let nats_dm_service = match init_nats_dm().await {
        Ok(svc) => {
            tracing::info!("NATS DM service initialized successfully");
            Some(svc)
        }
        Err(e) => {
            tracing::warn!("NATS DM service not available (DM features disabled): {e}");
            None
        }
    };

    // Initialize Redis for daily reply quota (optional)
    let redis_service = match init_redis() {
        Ok(svc) => {
            tracing::info!("Redis service initialized successfully (daily limits enabled)");
            Some(svc)
        }
        Err(e) => {
            tracing::warn!("Redis service not available (daily limits disabled): {e}");
            None
        }
    };

    let host = format!("0.0.0.0:{}", parameter::get("PORT"));
    tracing::info!("Starting server on {}", host);
    axum::Server::bind(&host.parse().unwrap())
        .serve(app(db, nats_dm_service, redis_service).into_make_service())
        .await
        .unwrap_or_else(|e| panic!("Server error: {}", e));
}

/// Initialize NATS connection and DM infrastructure.
/// Returns NatsDmService or error if NATS is not configured/reachable.
async fn init_nats_dm() -> Result<NatsDmService, String> {
    let nats_url = std::env::var("NATS_URL").unwrap_or_default();
    if nats_url.is_empty() {
        return Err("NATS_URL not set".into());
    }

    let nats_token = std::env::var("NATS_TOKEN").ok();

    let mut opts = async_nats::ConnectOptions::new();
    if let Some(token) = nats_token {
        opts = opts.token(token);
    }

    let client = opts
        .connect(&nats_url)
        .await
        .map_err(|e| format!("NATS connect to {nats_url}: {e}"))?;

    let svc = NatsDmService::new(client);
    svc.init_infrastructure()
        .await
        .map_err(|e| format!("NATS DM init: {e}"))?;

    Ok(svc)
}

/// Initialize Redis connection for daily reply quota.
/// Reads REDIS_URL from env; falls back to host.docker.internal:6379
/// (same Redis instance used by glance_mind_scheduler).
fn init_redis() -> Result<RedisService, String> {
    let redis_url =
        std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://host.docker.internal:6379".into());

    tracing::info!("Connecting to Redis at {redis_url}");

    let client =
        redis::Client::open(redis_url.as_str()).map_err(|e| format!("Redis client create: {e}"))?;

    let mut conn = client
        .get_connection()
        .map_err(|e| format!("Redis connect to {redis_url}: {e}"))?;

    let _: String = redis::cmd("PING")
        .query(&mut conn)
        .map_err(|e| format!("Redis PING: {e}"))?;

    Ok(RedisService::new(client))
}
