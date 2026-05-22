//! Audientry SSE relay: maps Redis pub/sub channel payloads into ai_chat
//! `SseEvent`s and drives a bounded subscription loop.

use std::time::{Duration, Instant};

use tokio::sync::mpsc::Sender;

use crate::dto::audientry_dto::{audientry_events_channel, AudientryChannelEvent};
use crate::service::ai_chat::types::SseEvent;

/// Parse a channel payload and map it to an SSE event.
///
/// Unknown / garbage payloads return `None` (the caller ignores + logs them).
/// The `kind` discriminator is stripped so the desktop receives the payload
/// shape declared in the frozen contract (`audientry_phase` / `audientry_report`
/// data == the event/report minus `kind`).
pub fn map_channel_payload(payload: &str) -> Option<SseEvent> {
    let value: serde_json::Value = serde_json::from_str(payload).ok()?;
    let routed: AudientryChannelEvent = serde_json::from_value(value.clone()).ok()?;
    match routed {
        AudientryChannelEvent::Phase(_) => Some(SseEvent::AudientryPhase {
            data: strip_kind(value),
        }),
        AudientryChannelEvent::Report(_) => Some(SseEvent::AudientryReport {
            data: strip_kind(value),
        }),
    }
}

fn strip_kind(mut v: serde_json::Value) -> serde_json::Value {
    if let Some(obj) = v.as_object_mut() {
        obj.remove("kind");
    }
    v
}

/// Returns true once the payload is the terminal report (caller stops the loop).
pub fn is_terminal(payload: &str) -> bool {
    serde_json::from_str::<serde_json::Value>(payload)
        .ok()
        .and_then(|v| {
            v.get("kind")
                .and_then(|k| k.as_str())
                .map(|s| s == "report")
        })
        .unwrap_or(false)
}

/// Wall-clock deadline check. `RedisError::is_timeout()` is platform-unreliable,
/// so the relay also tracks an `Instant` deadline as the authoritative bound.
pub fn deadline_exceeded(started: Instant, max: Duration) -> bool {
    started.elapsed() >= max
}

/// Max times the relay will reconnect (across connect / subscribe / mid-stream
/// failures) before giving up with a specific error.
const MAX_RECONNECTS: u8 = 5;

/// Subscribe to the per-job channel and forward mapped SSE events until the
/// terminal report, the wall-clock deadline, a reconnect cap, or the receiver
/// is dropped (client gone). Returns `Ok(true)` iff a terminal report was relayed.
///
/// The synchronous redis pub/sub runs inside `spawn_blocking`. Every failure
/// path (connect, subscribe, mid-stream stream error) is bounded by
/// `MAX_RECONNECTS` with linear backoff and logs the underlying error — the loop
/// never busy-spins to the deadline. Idle ticks (per-message read timeout) are
/// expected and only bounded by the wall-clock deadline.
pub async fn relay_job_events(
    client: redis::Client,
    job_id: String,
    tx: Sender<SseEvent>,
    max: Duration,
) -> Result<bool, String> {
    tokio::task::spawn_blocking(move || {
        let channel = audientry_events_channel(&job_id);
        let started = Instant::now();
        let mut reconnects = 0u8;
        loop {
            if deadline_exceeded(started, max) {
                let _ = tx.blocking_send(SseEvent::Error {
                    message: format!("audientry job {job_id} timed out"),
                });
                return Ok(false);
            }
            if reconnects >= MAX_RECONNECTS {
                let _ = tx.blocking_send(SseEvent::Error {
                    message: format!(
                        "audientry relay for {job_id} lost the subscription after {reconnects} reconnects"
                    ),
                });
                return Ok(false);
            }

            let mut conn = match client.get_connection() {
                Ok(c) => c,
                Err(e) => {
                    reconnects += 1;
                    tracing::warn!("audientry relay connect failed: {e}");
                    std::thread::sleep(Duration::from_millis(500 * reconnects as u64));
                    continue;
                }
            };
            let mut pubsub = conn.as_pubsub();
            if let Err(e) = pubsub.subscribe(&channel) {
                reconnects += 1;
                tracing::warn!("audientry relay subscribe failed: {e}");
                std::thread::sleep(Duration::from_millis(500 * reconnects as u64));
                continue;
            }
            let _ = pubsub.set_read_timeout(Some(Duration::from_secs(1)));

            loop {
                if deadline_exceeded(started, max) {
                    let _ = tx.blocking_send(SseEvent::Error {
                        message: format!("audientry job {job_id} timed out"),
                    });
                    return Ok(false);
                }
                match pubsub.get_message() {
                    Ok(msg) => {
                        let payload: String = msg.get_payload().unwrap_or_default();
                        if let Some(ev) = map_channel_payload(&payload) {
                            // receiver dropped => client gone, stop.
                            if tx.blocking_send(ev).is_err() {
                                return Ok(false);
                            }
                        }
                        if is_terminal(&payload) {
                            return Ok(true);
                        }
                    }
                    // expected idle tick (bounded by the wall-clock deadline above)
                    Err(e) if e.is_timeout() => continue,
                    // real stream error: count, log, back off, reconnect
                    Err(e) => {
                        reconnects += 1;
                        tracing::warn!("audientry relay stream error: {e}");
                        std::thread::sleep(Duration::from_millis(500 * reconnects as u64));
                        break;
                    }
                }
            }
        }
    })
    .await
    .map_err(|e| format!("relay join error: {e}"))?
}

#[cfg(test)]
mod tests {
    use super::*;

    fn read(n: &str) -> String {
        std::fs::read_to_string(format!("tests/golden/audientry/{n}.golden.json")).unwrap()
    }

    #[test]
    fn maps_phase_payload_to_audientry_phase_event() {
        let ev = map_channel_payload(&read("audientry_phase_event")).expect("mapped");
        assert!(matches!(
            ev,
            crate::service::ai_chat::types::SseEvent::AudientryPhase { .. }
        ));
    }

    #[test]
    fn maps_report_payload_to_audientry_report_event() {
        let ev = map_channel_payload(&read("audientry_report_completed")).expect("mapped");
        assert!(matches!(
            ev,
            crate::service::ai_chat::types::SseEvent::AudientryReport { .. }
        ));
    }

    #[test]
    fn ignores_garbage_payload() {
        assert!(map_channel_payload("{not json").is_none());
    }

    #[test]
    fn deadline_elapsed_is_detected_without_redis() {
        use std::time::{Duration, Instant};
        let started = Instant::now() - Duration::from_secs(31);
        assert!(deadline_exceeded(started, Duration::from_secs(30)));
        assert!(!deadline_exceeded(Instant::now(), Duration::from_secs(30)));
    }

    #[tokio::test]
    #[ignore] // opt-in: RUN_REDIS_INTEGRATION_TESTS=1 ... -- --include-ignored
    async fn relay_forwards_published_events_to_channel() {
        if std::env::var("RUN_REDIS_INTEGRATION_TESTS").as_deref() != Ok("1") {
            return;
        }
        use crate::dto::audientry_dto::audientry_events_channel;
        let url = std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379".into());
        let client = redis::Client::open(url.as_str()).unwrap();
        let (tx, mut rx) = tokio::sync::mpsc::channel(16);
        let job = format!("aud_relay_{}", std::process::id());
        let chan = audientry_events_channel(&job);
        let handle = tokio::spawn(relay_job_events(
            client.clone(),
            job.clone(),
            tx,
            std::time::Duration::from_secs(5),
        ));
        // publish a phase then a report
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        let mut con = client.get_connection().unwrap();
        let _: i64 = redis::cmd("PUBLISH")
            .arg(&chan)
            .arg(r#"{"kind":"phase","job_id":"x","phase":"m01","module_name":"m01_product_understanding","status":"completed","partial_summary":"ok"}"#)
            .query(&mut con)
            .unwrap();
        let _: i64 = redis::cmd("PUBLISH")
            .arg(&chan)
            .arg(r#"{"kind":"report","job_id":"x","status":"completed","summary":{"recommended_persona":"p","headline_strategy":"s","key_evidence":[]},"sections":{},"evidence_index":[],"audit":{"artifact_bundle_id":"b","module_runs":[]}}"#)
            .query(&mut con)
            .unwrap();
        let first = rx.recv().await.unwrap();
        assert!(matches!(
            first,
            crate::service::ai_chat::types::SseEvent::AudientryPhase { .. }
        ));
        handle.await.unwrap().unwrap();
    }
}
