use super::auth;
use crate::config::database::Database;
use crate::middleware::auth as auth_middleware;
use crate::middleware::charging;
use crate::middleware::permission;
use crate::routes::{oauth as oauth_routes, register, user};
#[allow(unused_imports)]
use crate::service::nats_dm_service::NatsDmService;
use crate::service::redis_service::RedisService;
use crate::state::auth_state::AuthState;
use crate::state::oauth_state::OauthState;
// use crate::state::token_state::TokenState;
use crate::state::user_state::UserState;
use axum::http::header::{ACCEPT, AUTHORIZATION, CONTENT_TYPE};
use axum::http::HeaderName;
use axum::routing::get;
use axum::{middleware, Router};
use std::sync::Arc;
use tower::ServiceBuilder;
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;

fn cors_allowed_headers() -> [HeaderName; 5] {
    [
        AUTHORIZATION,
        CONTENT_TYPE,
        ACCEPT,
        HeaderName::from_static("idempotency-key"),
        HeaderName::from_static("x-idempotency-key"),
    ]
}

pub fn routes(
    db_conn: Arc<Database>,
    nats_dm_service: Option<NatsDmService>,
    redis_service: Option<RedisService>,
) -> Router {
    let oauth_state = OauthState::new(&db_conn);
    let merged_router = {
        let auth_state = AuthState::new(&db_conn);
        let mut user_state = UserState::new(&db_conn);

        // Inject NATS DM service if available
        if let Some(svc) = nats_dm_service {
            user_state.set_nats_dm_service(svc);
        }

        // Inject Redis service if available (for daily reply quotas)
        if let Some(svc) = redis_service {
            user_state.set_redis_service(svc);
        }

        // Background self-healing for lost payment callbacks.
        user_state
            .wallet_service
            .spawn_recharge_reconciliation_loop();

        let drama_stream_hub = crate::service::drama_stream_hub::DramaStreamHub::new();
        let drama_worker_dispatcher =
            match crate::service::drama_worker_dispatcher::DramaWorkerDispatcher::from_env() {
                Ok(dispatcher) => Some(dispatcher),
                Err(e) => {
                    tracing::warn!("Drama worker dispatcher unavailable: {e}");
                    None
                }
            };

        let novel_worker_dispatcher =
            match crate::service::novel_worker_dispatcher::NovelWorkerDispatcher::from_env() {
                Ok(dispatcher) => Some(dispatcher),
                Err(e) => {
                    tracing::warn!("Novel worker dispatcher unavailable: {e}");
                    None
                }
            };

        let audientry_worker_dispatcher =
            match crate::service::audientry_worker_dispatcher::AudientryWorkerDispatcher::from_env()
            {
                Ok(dispatcher) => Some(dispatcher),
                Err(e) => {
                    tracing::warn!("Audientry worker dispatcher unavailable: {e}");
                    None
                }
            };

        // OpenMontage
        // B02: select the persistent Pg-backed store when a DATABASE_URL is
        // configured (prod), else the in-memory store (dev/test). An explicit
        // OPENMONTAGE_STORE_MODE=postgres|memory overrides the default. This
        // replaces the previous hardwired InMemoryJobStore, which silently lost
        // every job/event on restart and left the entire Pg store dead code.
        let openmontage_store_cfg =
            crate::repository::openmontage_repository::OpenMontageStoreConfig {
                database_url: std::env::var("DATABASE_URL").ok(),
                store_mode: std::env::var("OPENMONTAGE_STORE_MODE").ok(),
            };
        let openmontage_store =
            crate::repository::openmontage_repository::build_openmontage_job_store(
                &openmontage_store_cfg,
            );
        let openmontage_client =
            match crate::service::openmontage_client::RedisOpenMontageClient::from_env() {
                Ok(client) => std::sync::Arc::new(client)
                    as std::sync::Arc<dyn crate::service::openmontage_client::OpenMontageClient>,
                Err(e) => {
                    tracing::warn!("OpenMontage Redis client unavailable: {e}, using mock");
                    std::sync::Arc::new(
                        crate::service::openmontage_client::MockOpenMontageClient::new(),
                    )
                        as std::sync::Arc<dyn crate::service::openmontage_client::OpenMontageClient>
                }
            };
        let openmontage_hub = crate::service::openmontage_stream_hub::OpenMontageStreamHub::new();
        let openmontage_service = crate::service::openmontage_service::OpenMontageService::new(
            openmontage_store.clone(),
            openmontage_client.clone(),
            openmontage_hub.clone(),
        );

        // /api/v1
        Router::new()
            .nest(
                "/auth",
                auth::routes()
                    .with_state(auth_state)
                    .merge(register::routes().with_state(user_state.clone())),
            )
            .nest(
                "/user",
                user::routes()
                    .layer(
                        ServiceBuilder::new()
                            .layer(middleware::from_fn_with_state(
                                user_state.clone(),
                                auth_middleware::auth,
                            ))
                            .layer(axum::Extension(user_state.clone())),
                    )
                    .with_state(user_state.clone()),
            )
            .nest(
                "/campaigns",
                crate::routes::campaign::routes()
                    .layer(
                        ServiceBuilder::new()
                            .layer(middleware::from_fn_with_state(
                                user_state.clone(),
                                auth_middleware::auth,
                            ))
                            .layer(middleware::from_fn(permission::permission_middleware))
                            .layer(axum::Extension(user_state.campaign_service.clone()))
                            .layer(axum::Extension(user_state.agent_service.clone()))
                            .layer(axum::Extension(user_state.clone())),
                    )
                    .with_state(user_state.clone()),
            )
            .merge(
                crate::routes::template::routes()
                    .layer(
                        ServiceBuilder::new()
                            .layer(middleware::from_fn_with_state(
                                user_state.clone(),
                                auth_middleware::auth,
                            ))
                            .layer(axum::Extension(user_state.clone())),
                    )
                    .with_state(user_state.clone()),
            )
            .nest(
                "/config",
                crate::routes::config::routes()
                    .layer(axum::Extension(user_state.platform_service.clone()))
                    .layer(axum::Extension(user_state.config_service.clone()))
                    .with_state(user_state.clone()),
            )
            .nest(
                "/wallet",
                crate::routes::wallet::routes()
                    .layer(
                        ServiceBuilder::new()
                            .layer(middleware::from_fn_with_state(
                                user_state.clone(),
                                auth_middleware::auth,
                            ))
                            .layer(axum::Extension(user_state.clone())),
                    )
                    .with_state(user_state.clone()),
            )
            .nest(
                "/promo-codes",
                crate::routes::promo_code::promo_code_routes(Arc::new(
                    user_state.promo_code_service.clone(),
                ))
                .layer(
                    ServiceBuilder::new()
                        .layer(middleware::from_fn_with_state(
                            user_state.clone(),
                            auth_middleware::auth,
                        ))
                        .layer(axum::Extension(user_state.clone())),
                ),
            )
            .nest(
                "/accounts",
                crate::routes::social_account::routes()
                    .layer(
                        ServiceBuilder::new()
                            .layer(middleware::from_fn_with_state(
                                user_state.clone(),
                                auth_middleware::auth,
                            ))
                            .layer(axum::Extension(user_state.clone())),
                    )
                    .with_state(user_state.clone()),
            )
            .nest(
                "/social-groups",
                crate::routes::social_group::routes()
                    .layer(
                        ServiceBuilder::new()
                            .layer(middleware::from_fn_with_state(
                                user_state.clone(),
                                auth_middleware::auth,
                            ))
                            .layer(axum::Extension(user_state.clone())),
                    )
                    .with_state(user_state.clone()),
            )
            .nest(
                "/stats",
                crate::routes::dashboard::routes()
                    .layer(
                        ServiceBuilder::new()
                            .layer(middleware::from_fn_with_state(
                                user_state.clone(),
                                auth_middleware::auth,
                            ))
                            .layer(axum::Extension(user_state.clone())),
                    )
                    .with_state(user_state.clone()),
            )
            .nest(
                "/ai",
                crate::routes::ai::routes()
                    .layer(
                        ServiceBuilder::new()
                            .layer(middleware::from_fn_with_state(
                                user_state.clone(),
                                auth_middleware::auth,
                            ))
                            .layer(middleware::from_fn(permission::permission_middleware))
                            .layer(middleware::from_fn_with_state(
                                user_state.clone(),
                                charging::charging_middleware,
                            ))
                            .layer(axum::Extension(user_state.clone())),
                    )
                    .with_state(user_state.clone()),
            )
            .nest(
                "/agent",
                crate::routes::agent_analysis::routes()
                    .layer(
                        ServiceBuilder::new()
                            .layer(middleware::from_fn_with_state(
                                user_state.clone(),
                                auth_middleware::auth,
                            ))
                            .layer(middleware::from_fn(permission::permission_middleware))
                            .layer(middleware::from_fn_with_state(
                                user_state.clone(),
                                charging::charging_middleware,
                            ))
                            .layer(axum::Extension(user_state.clone())),
                    )
                    .with_state(user_state.clone()),
            )
            .nest(
                "/crawler-tasks",
                crate::routes::crawler::routes()
                    .layer(
                        ServiceBuilder::new()
                            .layer(middleware::from_fn_with_state(
                                user_state.clone(),
                                auth_middleware::auth,
                            ))
                            .layer(axum::Extension(user_state.clone())),
                    )
                    .with_state(user_state.clone()),
            )
            .nest(
                "/agent",
                crate::routes::agent::routes(user_state.clone()).layer(
                    ServiceBuilder::new()
                        .layer(middleware::from_fn_with_state(
                            user_state.clone(),
                            auth_middleware::auth,
                        ))
                        .layer(middleware::from_fn_with_state(
                            user_state.clone(),
                            permission::permission_middleware,
                        ))
                        .layer(axum::Extension(user_state.clone())),
                ),
            )
            .nest(
                "/video",
                crate::routes::video::video_routes()
                    .layer(
                        ServiceBuilder::new()
                            .layer(middleware::from_fn_with_state(
                                user_state.clone(),
                                auth_middleware::auth,
                            ))
                            .layer(middleware::from_fn(permission::permission_middleware))
                            .layer(middleware::from_fn_with_state(
                                user_state.clone(),
                                charging::charging_middleware,
                            ))
                            .layer(axum::Extension(user_state.clone())),
                    )
                    .with_state(user_state.clone()),
            )
            .nest(
                "/video-cases",
                crate::routes::video_case::video_case_routes()
                    .layer(
                        ServiceBuilder::new()
                            .layer(middleware::from_fn_with_state(
                                user_state.clone(),
                                auth_middleware::auth,
                            ))
                            .layer(axum::Extension(user_state.clone())),
                    )
                    .with_state(user_state.clone()),
            )
            .merge(
                crate::routes::material::material_routes()
                    .layer(
                        ServiceBuilder::new()
                            .layer(middleware::from_fn_with_state(
                                user_state.clone(),
                                auth_middleware::auth,
                            ))
                            .layer(axum::Extension(user_state.clone())),
                    )
                    .with_state(user_state.clone()),
            )
            .nest(
                "/scan",
                crate::routes::scan::routes()
                    .layer(
                        ServiceBuilder::new()
                            .layer(middleware::from_fn_with_state(
                                user_state.clone(),
                                auth_middleware::auth,
                            ))
                            .layer(middleware::from_fn(permission::permission_middleware))
                            .layer(middleware::from_fn_with_state(
                                user_state.clone(),
                                charging::charging_middleware,
                            ))
                            .layer(axum::Extension(user_state.clone())),
                    )
                    .with_state(user_state.clone()),
            )
            .nest(
                "/public",
                crate::routes::public::public_routes(&user_state).with_state(user_state.clone()),
            )
            .nest(
                "/upload-tasks",
                crate::routes::upload_task::routes()
                    .layer(
                        ServiceBuilder::new()
                            .layer(middleware::from_fn_with_state(
                                user_state.clone(),
                                auth_middleware::auth,
                            ))
                            .layer(middleware::from_fn(permission::permission_middleware))
                            .layer(axum::Extension(user_state.clone())),
                    )
                    .with_state(user_state.clone()),
            )
            .nest(
                "/notifications",
                crate::routes::notification::notification_routes(db_conn.clone()).layer(
                    ServiceBuilder::new()
                        .layer(middleware::from_fn_with_state(
                            user_state.clone(),
                            auth_middleware::auth,
                        ))
                        .layer(axum::Extension(user_state.clone())),
                ),
            )
            // AI Publish User Routes (requires auth + permission)
            .merge(
                crate::routes::aipub::aipub_user_routes()
                    .layer(
                        ServiceBuilder::new()
                            .layer(middleware::from_fn_with_state(
                                user_state.clone(),
                                auth_middleware::auth,
                            ))
                            .layer(middleware::from_fn(permission::permission_middleware))
                            .layer(axum::Extension(user_state.clone())),
                    )
                    .with_state(user_state.clone()),
            )
            // AI Batch Create (Module C) — single-item MVP (requires auth)
            .merge(
                axum::Router::new()
                    .route(
                        "/ai-tasks/batch",
                        axum::routing::post(
                            crate::handler::batch_task_handler::create_batch,
                        ),
                    )
                    .layer(
                        ServiceBuilder::new()
                            .layer(middleware::from_fn_with_state(
                                user_state.clone(),
                                auth_middleware::auth,
                            ))
                            .layer(axum::Extension(user_state.clone())),
                    )
                    .with_state(user_state.clone()),
            )
            // AI Publish Internal Routes (for Scheduler - no auth for now)
            .merge(crate::routes::aipub::aipub_internal_routes().with_state(user_state.clone()))
            // AI Publish Public Routes (for Executor - no auth for now)
            .merge(crate::routes::aipub::aipub_public_routes().with_state(user_state.clone()))
            // Public feedback / contact form (no auth — shown on public pages too)
            .nest(
                "/feedback",
                crate::routes::feedback::routes().with_state(user_state.clone()),
            )
            // DM Group Control Routes (requires auth + permission)
            .nest(
                "/dm",
                crate::routes::dm::routes()
                    .layer(
                        ServiceBuilder::new()
                            .layer(middleware::from_fn_with_state(
                                user_state.clone(),
                                auth_middleware::auth,
                            ))
                            .layer(middleware::from_fn(permission::permission_middleware))
                            .layer(axum::Extension(user_state.clone())),
                    )
                    .with_state(user_state.clone()),
            )
            // Patrol Stats Collection Routes (requires auth)
            .nest(
                "/patrol",
                crate::routes::patrol::routes()
                    .layer(
                        ServiceBuilder::new()
                            .layer(middleware::from_fn_with_state(
                                user_state.clone(),
                                auth_middleware::auth,
                            ))
                            .layer(axum::Extension(user_state.clone())),
                    )
                    .with_state(user_state.clone()),
            )
            // AI Chat Mode Routes (requires auth)
            .nest(
                "/ai-chat",
                crate::routes::ai_chat::routes()
                    .layer(
                        ServiceBuilder::new()
                            .layer(middleware::from_fn_with_state(
                                user_state.clone(),
                                auth_middleware::auth,
                            ))
                            .layer(axum::Extension(audientry_worker_dispatcher.clone()))
                            .layer(axum::Extension(user_state.clone())),
                    )
                    .with_state(user_state.clone()),
            )
            // AI Short Drama Routes (requires auth, proxies to gm_agent_hub gateway)
            .nest(
                "/drama",
                crate::routes::drama::routes()
                    .layer(
                        ServiceBuilder::new()
                            .layer(middleware::from_fn_with_state(
                                user_state.clone(),
                                auth_middleware::auth,
                            ))
                            .layer(axum::Extension(
                                crate::service::drama_facade::DramaFacade::new(),
                            ))
                            .layer(axum::Extension(
                                crate::service::drama_billing::DramaBillingGuard::new(
                                    &user_state.db,
                                ),
                            ))
                            .layer(axum::Extension(
                                crate::service::drama_projection::DramaProjectionService::new(
                                    &user_state.db,
                                ),
                            ))
                            .layer(axum::Extension(
                                crate::service::drama_project_meta_service::DramaProjectMetaService::new(
                                    &user_state.db,
                                ),
                            ))
                            .layer(axum::Extension(drama_worker_dispatcher.clone()))
                            .layer(axum::Extension(drama_stream_hub.clone()))
                            .layer(axum::Extension(user_state.clone())),
                    )
                    .with_state(user_state.clone()),
            )
            // AI Short Drama Internal Callback (no user auth, internal network only)
            .nest(
                "/internal/drama",
                crate::routes::drama::internal_routes()
                    .layer(axum::Extension(
                        crate::service::drama_projection::DramaProjectionService::new(
                            &user_state.db,
                        ),
                    ))
                    .layer(axum::Extension(drama_stream_hub.clone()))
                    .with_state(user_state.clone()),
            )
            // Novel Engine Internal Callback (no user auth, internal network only)
            .nest(
                "/internal/novel",
                crate::routes::novel::internal_routes()
                    .layer(axum::Extension(
                        crate::service::novel_service::NovelService::new(&user_state.db),
                    ))
                    .with_state(user_state.clone()),
            )
            // DM Auto-Reply Internal Routes (requires INTERNAL_SERVICE_TOKEN — for gm_customer_service)
            .nest(
                "/internal",
                crate::routes::internal::dm::routes()
                    .layer(axum::Extension(user_state.clone()))
                    .with_state(user_state.clone()),
            )
            // DM Auto-Reply Admin Routes (clear_review — requires user JWT auth)
            .nest(
                "/admin",
                crate::routes::internal::dm::admin_routes()
                    .layer(
                        ServiceBuilder::new()
                            .layer(middleware::from_fn_with_state(
                                user_state.clone(),
                                auth_middleware::auth,
                            ))
                            .layer(axum::Extension(user_state.clone())),
                    )
                    .with_state(user_state.clone()),
            )
            // Novel Engine Routes (requires auth)
            .nest(
                "/novel",
                crate::routes::novel::routes()
                    .layer(
                        ServiceBuilder::new()
                            .layer(middleware::from_fn_with_state(
                                user_state.clone(),
                                auth_middleware::auth,
                            ))
                            .layer(axum::Extension(
                                crate::service::novel_service::NovelService::new(&user_state.db),
                            ))
                            .layer(axum::Extension(novel_worker_dispatcher.clone()))
                            .layer(axum::Extension(user_state.clone())),
                    )
                    .with_state(user_state.clone()),
            )
            // OpenMontage Internal Callback (no JWT; guarded by X-Internal-Token
            // shared-secret middleware inside internal_routes() — see B01)
            .nest(
                "/internal/openmontage",
                crate::routes::openmontage::internal_routes()
                    .layer(axum::Extension(openmontage_service.clone())),
            )
            // OpenMontage Routes (requires auth)
            .nest(
                "/openmontage",
                crate::routes::openmontage::user_routes()
                    .layer(
                        ServiceBuilder::new()
                            .layer(middleware::from_fn_with_state(
                                user_state.clone(),
                                auth_middleware::auth,
                            ))
                            .layer(axum::Extension(openmontage_service.clone()))
                            .layer(axum::Extension(user_state.clone())),
                    ),
            )
    };

    Router::new()
        .route("/health", get(|| async { "Healthy..." }))
        .nest("/api/v1", merged_router)
        // OAuth 2.0 token + revoke endpoints (additive — do NOT touch /auth/*)
        .nest(
            "/oauth",
            oauth_routes::oauth_routes().with_state(oauth_state.clone()),
        )
        // OTA feature-flag config endpoints
        .nest("/ota", oauth_routes::ota_routes().with_state(oauth_state))
        .layer(TraceLayer::new_for_http())
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(cors_allowed_headers()),
        )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cors_headers_allow_desktop_idempotency_key() {
        let headers = cors_allowed_headers();
        assert!(headers
            .iter()
            .any(|header| header.as_str() == "idempotency-key"));
    }
}
