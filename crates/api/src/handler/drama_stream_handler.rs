use axum::{
    extract::Path,
    response::sse::{Event, Sse},
    Extension,
};
use futures::stream::Stream;
use glance_mind_db::entity::user::User;
use std::convert::Infallible;
use tokio_stream::wrappers::ReceiverStream;
use tokio_stream::StreamExt;

use crate::error::api_error::ApiError;
use crate::service::drama_projection::DramaProjectionService;
use crate::service::drama_stream_hub::{DramaSseEvent, DramaStreamHub};

pub async fn stream_project(
    Extension(user): Extension<User>,
    Extension(projection): Extension<DramaProjectionService>,
    Extension(hub): Extension<DramaStreamHub>,
    Path(project_id): Path<String>,
) -> Result<Sse<impl Stream<Item = Result<Event, Infallible>>>, ApiError> {
    let snapshot = projection
        .get_projection(&project_id)
        .map_err(|e| ApiError::InternalServerError(format!("projection read failed: {}", e)))?;

    if let Some(ref row) = snapshot {
        if row.user_id != user.id {
            return Err(ApiError::Forbidden("no access to this project".to_string()));
        }
    }

    let rx = hub.subscribe(&project_id).await;
    let (tx_out, rx_out) = tokio::sync::mpsc::channel::<DramaSseEvent>(64);

    if let Some(row) = snapshot {
        let snap_event = DramaSseEvent {
            event_type: "project_snapshot".to_string(),
            project_id: row.project_id.clone(),
            run_id: row.run_id.unwrap_or_default(),
            sequence: row.last_event_sequence,
            interaction_version: row.interaction_version,
            status: row.status,
            current_stage: row.current_stage,
            payload: serde_json::json!({
                "title": row.title,
                "pending_stage": row.pending_stage,
                "progress_percent": row.progress_percent,
                "cost_reserve_cents": row.cost_reserve_cents,
                "cost_consumed_cents": row.cost_consumed_cents,
                "error_message": row.error_message,
            }),
        };
        let _ = tx_out.send(snap_event).await;
    }

    let heartbeat_tx = tx_out.clone();

    tokio::spawn(async move {
        let mut stream = ReceiverStream::new(rx);
        while let Some(event) = stream.next().await {
            if tx_out.send(event).await.is_err() {
                break;
            }
        }
    });
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(15));
        loop {
            interval.tick().await;
            let hb = DramaSseEvent {
                event_type: "heartbeat".to_string(),
                project_id: String::new(),
                run_id: String::new(),
                sequence: 0,
                interaction_version: 0,
                status: String::new(),
                current_stage: None,
                payload: serde_json::json!({}),
            };
            if heartbeat_tx.send(hb).await.is_err() {
                break;
            }
        }
    });

    let sse_stream = ReceiverStream::new(rx_out).map(|event| {
        let data = serde_json::to_string(&event).unwrap_or_default();
        Ok(Event::default().event(&event.event_type).data(data))
    });

    Ok(Sse::new(sse_stream))
}
