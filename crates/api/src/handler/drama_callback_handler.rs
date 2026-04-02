use axum::{
    body::Bytes,
    body::BoxBody,
    http::StatusCode,
    response::{IntoResponse, Response},
    Extension, Json,
};
use crate::dto::drama_dto::DramaCallbackEvent;
use crate::service::drama_projection::DramaProjectionService;
use crate::service::drama_stream_hub::{DramaSseEvent, DramaStreamHub};
use hmac::{Hmac, Mac};
use sha2::Sha256;

type HmacSha256 = Hmac<Sha256>;

fn compute_signature(secret: &str, timestamp: &str, body: &[u8]) -> Option<String> {
    let mut mac = HmacSha256::new_from_slice(secret.as_bytes()).ok()?;
    mac.update(timestamp.as_bytes());
    mac.update(b".");
    mac.update(body);
    let digest = mac.finalize().into_bytes();
    Some(digest.iter().map(|b| format!("{:02x}", b)).collect())
}

fn verify_signature(signature: Option<&str>, timestamp: Option<&str>, body: &[u8]) -> bool {
    let secret = std::env::var("DRAMA_CALLBACK_SECRET").unwrap_or_default();
    if secret.is_empty() {
        // Keep current dev behavior when the secret is not configured.
        return true;
    }
    let sig = match signature {
        Some(v) if !v.is_empty() => v,
        _ => return false,
    };
    let ts = match timestamp {
        Some(v) if !v.is_empty() => v,
        _ => return false,
    };

    let expected = match compute_signature(&secret, ts, body) {
        Some(v) => v,
        None => return false,
    };
    expected.eq_ignore_ascii_case(sig)
}

fn parse_event(body: &[u8]) -> Result<DramaCallbackEvent, String> {
    serde_json::from_slice(body).map_err(|e| format!("invalid callback body: {e}"))
}

pub async fn ingest_callback(
    Extension(projection): Extension<DramaProjectionService>,
    Extension(stream_hub): Extension<DramaStreamHub>,
    headers: axum::http::HeaderMap,
    body: Bytes,
) -> Response<BoxBody> {
    let sig = headers
        .get("x-drama-signature")
        .and_then(|v| v.to_str().ok());
    let ts = headers
        .get("x-drama-timestamp")
        .and_then(|v| v.to_str().ok());

    if !verify_signature(sig, ts, &body) {
        return (
            StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({
                "error": "invalid callback signature"
            })),
        )
            .into_response();
    }

    let event = match parse_event(&body) {
        Ok(event) => event,
        Err(detail) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({
                    "error": "invalid callback payload",
                    "detail": detail,
                })),
            )
                .into_response();
        }
    };

    tracing::info!(
        project_id = %event.project_id,
        run_id = %event.run_id,
        event_type = %event.event_type,
        sequence = event.sequence,
        "Drama callback received"
    );

    match projection.ingest_event(&event) {
        Ok(true) => {
            let (proj_status, proj_interaction_version) = projection
                .get_projection(&event.project_id)
                .ok()
                .flatten()
                .map(|row| (row.status, row.interaction_version))
                .unwrap_or_else(|| (String::new(), 0));

            let sse_event = DramaSseEvent {
                event_type: event.event_type.clone(),
                project_id: event.project_id.clone(),
                run_id: event.run_id.clone(),
                sequence: event.sequence,
                interaction_version: proj_interaction_version,
                status: proj_status,
                current_stage: event.stage_code.clone(),
                payload: event.payload.clone(),
            };
            stream_hub.publish(sse_event).await;

            (
                StatusCode::OK,
                Json(serde_json::json!({
                    "accepted": true,
                    "event_id": event.event_id,
                })),
            )
                .into_response()
        }
        Ok(false) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "accepted": false,
                "reason": "duplicate event_id",
                "event_id": event.event_id,
            })),
        )
            .into_response(),
        Err(e) => {
            tracing::error!("Callback ingestion failed: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "error": "ingestion failed",
                    "detail": e,
                })),
            )
                .into_response()
        }
    }
}

#[derive(serde::Deserialize)]
pub struct ReplayRequest {
    pub run_id: String,
    pub from_sequence: i64,
}

pub async fn request_replay(
    Extension(_projection): Extension<DramaProjectionService>,
    axum::extract::Path(project_id): axum::extract::Path<String>,
    Json(req): Json<ReplayRequest>,
) -> Response<BoxBody> {
    tracing::info!(
        project_id = %project_id,
        run_id = %req.run_id,
        from_sequence = req.from_sequence,
        "Replay requested"
    );

    // TODO(H6): forward replay request to gm_agent_hub so it resends
    // missed events from the specified sequence onwards.
    (
        StatusCode::ACCEPTED,
        Json(serde_json::json!({
            "project_id": project_id,
            "run_id": req.run_id,
            "from_sequence": req.from_sequence,
            "status": "replay_requested",
        })),
    )
        .into_response()
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use serde_json::json;

    #[test]
    fn compute_signature_matches_verify_logic() {
        let body = br#"{"event_id":"evt_1","project_id":"proj_1"}"#;
        let ts = "1711274400";
        let secret = "worker-secret";
        let sig = compute_signature(secret, ts, body).expect("compute signature");
        let expected = compute_signature(secret, ts, body).expect("recompute signature");
        assert_eq!(sig, expected);
    }

    #[test]
    fn compute_signature_different_body_yields_different_sig() {
        let ts = "1711274400";
        let secret = "worker-secret";
        let sig_a = compute_signature(secret, ts, b"body_a").unwrap();
        let sig_b = compute_signature(secret, ts, b"body_b").unwrap();
        assert_ne!(sig_a, sig_b);
    }

    #[test]
    fn compute_signature_produces_hex_string() {
        let sig = compute_signature("secret", "ts", b"body").unwrap();
        assert!(sig.chars().all(|c| c.is_ascii_hexdigit()));
        assert!(!sig.is_empty());
    }

    #[test]
    fn compute_signature_is_deterministic() {
        let a = compute_signature("s", "t", b"b").unwrap();
        let b = compute_signature("s", "t", b"b").unwrap();
        assert_eq!(a, b);
    }

    #[test]
    fn compute_signature_different_secret_yields_different_sig() {
        let a = compute_signature("secret_a", "ts", b"body").unwrap();
        let b = compute_signature("secret_b", "ts", b"body").unwrap();
        assert_ne!(a, b);
    }

    #[test]
    fn parse_event_decodes_payload() {
        let raw = serde_json::to_vec(&DramaCallbackEvent {
            event_id: "evt_1".to_string(),
            project_id: "proj_1".to_string(),
            run_id: "run_1".to_string(),
            sequence: 1,
            stage_code: Some("s01_strategy".to_string()),
            event_type: "run_started".to_string(),
            occurred_at: Utc::now(),
            payload: json!({"job_id": 99}),
        })
        .expect("encode callback event");

        let parsed = parse_event(&raw).expect("parse callback event");
        assert_eq!(parsed.project_id, "proj_1");
        assert_eq!(parsed.event_type, "run_started");
        assert_eq!(parsed.sequence, 1);
        assert_eq!(parsed.stage_code, Some("s01_strategy".to_string()));
        assert_eq!(parsed.payload.get("job_id"), Some(&json!(99)));
    }

    #[test]
    fn parse_event_rejects_invalid_json() {
        let result = parse_event(b"not json");
        assert!(result.is_err());
    }

    #[test]
    fn parse_event_all_event_types() {
        let event_types = [
            "run_started", "run_completed", "run_completed_with_fallback",
            "run_failed", "run_cancelled",
            "stage_entered", "stage_completed",
            "clarification_required", "clarification_resolved",
            "strategy_package_ready", "script_package_ready",
            "render_progress_recorded",
            "cost_recorded", "fallback_recorded", "artifact_uploaded",
        ];
        for et in &event_types {
            let raw = serde_json::to_vec(&DramaCallbackEvent {
                event_id: format!("evt_{}", et),
                project_id: "proj_1".to_string(),
                run_id: "run_1".to_string(),
                sequence: 1,
                stage_code: None,
                event_type: et.to_string(),
                occurred_at: Utc::now(),
                payload: json!({}),
            })
            .unwrap();
            let parsed = parse_event(&raw).unwrap();
            assert_eq!(parsed.event_type, *et);
        }
    }
}
