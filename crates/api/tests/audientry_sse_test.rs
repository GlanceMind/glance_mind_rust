// crates/api/tests/audientry_sse_test.rs
use glance_mind_api::service::ai_chat::types::SseEvent;
use serde_json::json;

#[test]
fn audientry_phase_serializes_to_sse_frame() {
    let ev = SseEvent::AudientryPhase {
        data: json!({"job_id":"j","phase":"m03","status":"completed"}),
    };
    let s = ev.to_sse_string();
    assert!(s.starts_with("event: audientry_phase\n"));
    assert!(s.contains("\"phase\":\"m03\""));
}

#[test]
fn audientry_report_serializes_to_sse_frame() {
    let ev = SseEvent::AudientryReport {
        data: json!({"job_id":"j","status":"completed"}),
    };
    assert!(ev.to_sse_string().starts_with("event: audientry_report\n"));
}
