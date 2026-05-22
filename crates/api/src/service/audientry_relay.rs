//! Audientry SSE relay: maps Redis pub/sub channel payloads into ai_chat
//! `SseEvent`s and (Task 6) drives a bounded subscription loop.

use crate::dto::audientry_dto::AudientryChannelEvent;
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
}
