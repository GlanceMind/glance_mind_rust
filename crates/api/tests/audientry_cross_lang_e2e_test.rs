//! M5 Tier 2 — cross-language local e2e: real Rust dispatcher + relay <-> real Python worker.
//!
//! Opt-in (`RUN_REDIS_INTEGRATION_TESTS=1`). Requires:
//!   1. a real redis (e.g. `docker compose -f docker-compose.e2e.yml up -d redis` in gm_audientry,
//!      which exposes it on host port 6381), and
//!   2. a running Python worker against the SAME redis, e.g.:
//!        GM_AUDIENTRY_FAKE_PROVIDERS=1 REDIS_URL=redis://127.0.0.1:6381 python -m gm_audientry.worker
//!
//! Run:
//!   RUN_REDIS_INTEGRATION_TESTS=1 REDIS_URL=redis://127.0.0.1:6381 \
//!     cargo test -p glance_mind_api --test audientry_cross_lang_e2e_test -- --ignored --nocapture

use std::time::Duration;

use glance_mind_api::dto::audientry_dto::{AudientryWorkerTaskEnvelope, ProductBrief};
use glance_mind_api::service::ai_chat::types::SseEvent;
use glance_mind_api::service::audientry_relay::relay_job_events;
use glance_mind_api::service::audientry_worker_dispatcher::AudientryWorkerDispatcher;

#[tokio::test]
#[ignore] // opt-in: needs real redis + a running `python -m gm_audientry.worker`
async fn rust_enqueue_python_worker_relay_round_trip() {
    if std::env::var("RUN_REDIS_INTEGRATION_TESTS").as_deref() != Ok("1") {
        return;
    }
    let url = std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6381".into());
    let client = redis::Client::open(url.as_str()).expect("redis client");
    let dispatcher =
        AudientryWorkerDispatcher::new(client.clone(), AudientryWorkerTaskEnvelope::QUEUE_KEY);
    let job_id = format!("aud_xlang_{}", std::process::id());

    // Subscribe/relay BEFORE enqueue so we don't miss early phase events.
    let (tx, mut rx) = tokio::sync::mpsc::channel::<SseEvent>(64);
    let relay = tokio::spawn(relay_job_events(
        client.clone(),
        job_id.clone(),
        tx,
        Duration::from_secs(180),
    ));
    // give the relay a moment to establish its subscription
    tokio::time::sleep(Duration::from_millis(300)).await;

    // Marker-rich description so the Python deterministic ProductUnderstandingService yields
    // candidates and the worker emits a full m01..m07 phase stream (bare prose would
    // short-circuit to a blocked report before the pipeline runs, with no phase events).
    dispatcher
        .enqueue(&AudientryWorkerTaskEnvelope {
            contract_version: "2026-04-29".into(),
            job_id: job_id.clone(),
            conversation_id: 1,
            user_id: 2,
            product: ProductBrief {
                contract_version: "2026-04-29".into(),
                name: "Sleep Tea".into(),
                description: "Fact: herbal sleep tea sold direct-to-consumer. \
                              Pain: I cannot fall asleep without grogginess the next day."
                    .into(),
                landing_page_url: None,
                locale: "en-US".into(),
            },
            questionnaire_answers: None,
        })
        .await
        .expect("enqueue");

    // Collect relayed SSE events until the terminal report (or the channel closes).
    let mut saw_phase = false;
    let mut saw_report = false;
    while let Some(ev) = rx.recv().await {
        match ev {
            SseEvent::AudientryPhase { .. } => saw_phase = true,
            SseEvent::AudientryReport { .. } => {
                saw_report = true;
                break;
            }
            _ => {}
        }
    }
    assert!(
        saw_phase && saw_report,
        "expected phase + report relayed from the python worker (saw_phase={saw_phase}, saw_report={saw_report})"
    );
    let _ = relay.await;
}
