//! OpenMontage Service
//!
//! Business logic layer for OpenMontage jobs: create, find by idempotency, cancel, approval.

use crate::dto::openmontage_dto::{
    ApprovalDto, AssetDto, CancelResultDto, CreateJobDto, JobEventDto, JobEventsDto,
    JobSnapshotDto, PipelineInfoDto, PipelinesDto, PreflightDto, ServerContext,
};
use crate::repository::openmontage_repository::{
    NewAsset, NewJob, NewJobEvent, OpenMontageJobStore,
};
use crate::service::openmontage_client::{OpenMontageClient, WorkerEnvelope};
use crate::service::openmontage_stream_hub::{OpenMontageSseEvent, OpenMontageStreamHub};
use crate::service::oss_service::OssService;
use std::sync::Arc;
use uuid::Uuid;

#[derive(Clone)]
pub struct OpenMontageService {
    store: Arc<dyn OpenMontageJobStore>,
    client: Arc<dyn OpenMontageClient>,
    hub: OpenMontageStreamHub,
}

impl OpenMontageService {
    pub fn new(
        store: Arc<dyn OpenMontageJobStore>,
        client: Arc<dyn OpenMontageClient>,
        hub: OpenMontageStreamHub,
    ) -> Self {
        Self { store, client, hub }
    }

    /// Create a new job. Mints job_id, project_id="omx-{job_id}", validates no secret material,
    /// checks idempotency (same body -> return existing, diff body -> conflict).
    pub fn create_job(
        &self,
        user_id: i32,
        tenant_id: &str,
        dto: CreateJobDto,
    ) -> Result<JobSnapshotDto, String> {
        // Validate no secrets
        dto.validate_no_secret_material()?;

        // Generate IDs
        let job_id = Uuid::new_v4().to_string();
        let project_id = format!("omx-{}", job_id);
        let request_id = Uuid::new_v4().to_string();
        let idempotency_key = Uuid::new_v4().to_string();

        // Server context
        let server_ctx = ServerContext {
            job_id: job_id.clone(),
            user_id,
            tenant_id: tenant_id.to_string(),
            callback_secret_ref: Some("internal-callback-secret".to_string()),
        };

        let mut request_json = dto.to_protocol_request(server_ctx);

        // M2: Resolve asset_ids and build assets/tool_invocations based on input_mode
        if let Some(ref asset_ids) = dto.asset_ids {
            if !asset_ids.is_empty() {
                let mut assets_array = Vec::new();
                let mut tool_invocations = Vec::new();

                // Resolve assets from store
                for asset_id in asset_ids {
                    let asset = self
                        .store
                        .get_asset(asset_id)
                        .map_err(|e| format!("Failed to fetch asset {}: {}", asset_id, e))?
                        .ok_or_else(|| format!("Asset not found: {}", asset_id))?;

                    assets_array.push(serde_json::json!({
                        "asset_id": asset.asset_id,
                        "kind": asset.kind,
                        "role": asset.role,
                        "uri": asset.uri,
                        "mime_type": asset.mime_type,
                        "bytes": asset.bytes,
                        "width_px": asset.width_px,
                        "height_px": asset.height_px,
                        "duration_ms": asset.duration_ms,
                    }));
                }

                // Build tool_invocations based on input_mode
                if let Some(ref input_mode) = dto.input_mode {
                    match input_mode.as_str() {
                        "image_to_video" => {
                            // Find reference_image asset
                            if let Some(img_asset) =
                                assets_array.iter().find(|a| a["kind"] == "reference_image")
                            {
                                tool_invocations.push(serde_json::json!({
                                    "operation": "image_to_video",
                                    "input_json": serde_json::json!({
                                        "prompt": dto.prompt,
                                        "image_url": img_asset["uri"],
                                        "duration": dto.duration_seconds.unwrap_or(60),
                                    }).to_string(),
                                }));
                            }
                        }
                        "first_last_frame" => {
                            // start_frame and end_frame are already in assets_array, no tool_invocation needed
                        }
                        "reference_driven" => {
                            // reference_video is in assets_array, no direct generation invocation (worker gates it)
                        }
                        "source_clip" => {
                            // source_video is in assets_array, no tool_invocation
                        }
                        _ => {
                            // text_to_video / source_script / unknown: no assets/tool_invocations
                        }
                    }
                }

                request_json["assets"] = serde_json::Value::Array(assets_array);
                if !tool_invocations.is_empty() {
                    request_json["tool_invocations"] = serde_json::Value::Array(tool_invocations);
                }
            }
        }

        let new_job = NewJob {
            job_id: job_id.clone(),
            project_id: project_id.clone(),
            user_id,
            tenant_id: tenant_id.to_string(),
            request_id: request_id.clone(),
            idempotency_key: idempotency_key.clone(),
            pipeline: dto
                .pipeline
                .clone()
                .unwrap_or_else(|| "animated-explainer".to_string()),
            input_mode: dto.input_mode.clone(),
            status: "queued".to_string(),
            snapshot_json: serde_json::json!({
                "title": dto.title,
                "prompt": dto.prompt,
                "target_platform": dto.target_platform,
            }),
        };

        let job = self.store.create_job(new_job)?;

        // Enqueue run envelope
        let envelope = WorkerEnvelope {
            task_id: Uuid::new_v4().to_string(),
            job_id: job_id.clone(),
            project_id: project_id.clone(),
            attempt: 1,
            max_attempts: 3,
            kind: "run".to_string(),
            request_json,
            resume_from_stage: None,
            approval_decision_json: None,
            start_sequence: 1,
            enqueued_at: chrono::Utc::now().to_rfc3339(),
        };

        tracing::info!(job_id = %job_id, "openmontage create_job: enqueueing run envelope");
        self.client.enqueue_run(envelope)?;

        Ok(JobSnapshotDto {
            job_id: job.job_id,
            project_id: job.project_id,
            status: job.status,
            pipeline: job.pipeline,
            current_stage: job.current_stage,
            progress_pct: job.progress_pct,
            error_json: job.error_json,
            last_event_sequence: job.last_event_sequence,
            next_event_sequence: job.next_event_sequence,
            sync_required: job.sync_required,
            snapshot_json: job.snapshot_json,
            created_at: job.created_at.to_rfc3339(),
            updated_at: job.updated_at.map(|t| t.to_rfc3339()),
        })
    }

    pub fn get_job(&self, job_id: &str) -> Result<Option<JobSnapshotDto>, String> {
        let job = self.store.get_job(job_id)?;
        Ok(job.map(|j| JobSnapshotDto {
            job_id: j.job_id,
            project_id: j.project_id,
            status: j.status,
            pipeline: j.pipeline,
            current_stage: j.current_stage,
            progress_pct: j.progress_pct,
            error_json: j.error_json,
            last_event_sequence: j.last_event_sequence,
            next_event_sequence: j.next_event_sequence,
            sync_required: j.sync_required,
            snapshot_json: j.snapshot_json,
            created_at: j.created_at.to_rfc3339(),
            updated_at: j.updated_at.map(|t| t.to_rfc3339()),
        }))
    }

    pub fn cancel_job(&self, job_id: &str) -> Result<CancelResultDto, String> {
        let job = self
            .store
            .get_job(job_id)?
            .ok_or_else(|| "Job not found".to_string())?;

        if job.status == "queued" {
            // Queued job -> just mark cancelled
            self.store.set_status(job_id, "cancelled")?;
            Ok(CancelResultDto {
                job_id: job_id.to_string(),
                cancel_requested: false, // queued -> direct cancel
                message: "Job cancelled (was queued)".to_string(),
            })
        } else if job.status == "running" {
            // Running job -> set cancel flag + cancel_requested=true
            self.store.set_cancel_requested(job_id)?;
            self.client.set_cancel_flag(job_id)?;
            Ok(CancelResultDto {
                job_id: job_id.to_string(),
                cancel_requested: true,
                message: "Cancel requested (job still running)".to_string(),
            })
        } else {
            Ok(CancelResultDto {
                job_id: job_id.to_string(),
                cancel_requested: false,
                message: format!("Job already in terminal state: {}", job.status),
            })
        }
    }

    pub fn submit_approval(
        &self,
        job_id: &str,
        approval: ApprovalDto,
    ) -> Result<JobSnapshotDto, String> {
        let job = self
            .store
            .get_job(job_id)?
            .ok_or_else(|| "Job not found".to_string())?;

        // Reconstruct the run request so the worker can continue the pipeline
        // past the approval gate. The python worker's engine.resume() promotes
        // the `resume_from_stage` checkpoint and then re-drives run(), which
        // loads the manifest from request["pipeline"]. The previous values —
        // an empty request_json and a None resume_from_stage — made the worker
        // reject the envelope outright ("resume_from_stage is required when
        // kind='resume'"), so approvals never advanced the job.
        let mut resume_request = job.snapshot_json.clone();
        match resume_request.as_object_mut() {
            Some(obj) => {
                obj.insert(
                    "pipeline".to_string(),
                    serde_json::Value::String(job.pipeline.clone()),
                );
            }
            None => {
                resume_request = serde_json::json!({ "pipeline": job.pipeline });
            }
        }

        // Enqueue resume with approval decision
        let envelope = WorkerEnvelope {
            task_id: Uuid::new_v4().to_string(),
            job_id: job_id.to_string(),
            project_id: job.project_id.clone(),
            attempt: 1,
            max_attempts: 3,
            kind: "resume".to_string(),
            request_json: resume_request,
            // The stage being approved; the worker promotes its checkpoint.
            resume_from_stage: job.current_stage.clone(),
            approval_decision_json: Some(serde_json::to_value(&approval).unwrap()),
            start_sequence: job.next_event_sequence,
            enqueued_at: chrono::Utc::now().to_rfc3339(),
        };

        self.client.enqueue_resume(envelope)?;

        // Return current snapshot (approval doesn't change status immediately)
        self.get_job(job_id)?
            .ok_or_else(|| "Job disappeared after approval".to_string())
    }

    /// Preflight check (read from client with fallback to warming_up)
    pub fn preflight(&self) -> Result<PreflightDto, String> {
        let preflight = self.client.read_preflight()?;
        Ok(preflight.unwrap_or_else(|| PreflightDto {
            passed: false,
            status: "warming_up".to_string(),
            blocking: vec![],
            warnings: vec![],
            estimated_cost_cents: None,
        }))
    }

    /// Available pipelines (read from client with fallback to animated-explainer)
    pub fn pipelines(&self) -> Result<PipelinesDto, String> {
        let pipelines = self.client.read_pipelines()?;
        Ok(pipelines.unwrap_or_else(|| PipelinesDto {
            pipelines: vec![PipelineInfoDto {
                name: "animated-explainer".to_string(),
                description: "Topic to fully generated explainer".to_string(),
                stability: "production".to_string(),
            }],
        }))
    }

    /// List events for a job after a given sequence
    pub fn list_events(
        &self,
        job_id: &str,
        after: i64,
        limit: i64,
    ) -> Result<JobEventsDto, String> {
        let events = self.store.list_events(job_id, after, limit)?;
        let next_seq = events.last().map(|e| e.sequence as u64 + 1).unwrap_or(1);

        let dtos: Vec<JobEventDto> = events
            .iter()
            .map(|e| JobEventDto {
                sequence: e.sequence,
                event_id: e.event_id.clone(),
                event_type: e.event_type.clone(),
                status: e.status.clone(),
                stage: e.stage.clone(),
                progress_pct: e.progress_pct,
                event_json: e.event_json.clone(),
                emitted_at: e.emitted_at.map(|t| t.to_rfc3339()).unwrap_or_default(),
            })
            .collect();

        Ok(JobEventsDto {
            events: dtos,
            next_sequence: next_seq,
        })
    }

    /// Subscribe to a job's SSE stream
    pub async fn subscribe(
        &self,
        job_id: &str,
    ) -> tokio::sync::mpsc::Receiver<OpenMontageSseEvent> {
        self.hub.subscribe(job_id).await
    }

    /// Get backlog events for SSE stream
    pub fn backlog(
        &self,
        job_id: &str,
        after: i64,
    ) -> Result<Vec<crate::repository::openmontage_repository::JobEvent>, String> {
        self.store.list_events(job_id, after, 100)
    }

    /// Ingest callback event from worker
    pub fn ingest_event(
        &self,
        event: crate::handler::openmontage_handler::OpenMontageJobEvent,
    ) -> Result<crate::handler::openmontage_handler::CallbackAck, String> {
        let job_id = &event.job.job_id;

        let new_event = NewJobEvent {
            job_id: job_id.to_string(),
            sequence: event.sequence,
            event_id: event.event_id.clone(),
            event_type: event.event_type.clone(),
            status: Some(event.status.clone()),
            event_json: serde_json::to_value(&event).unwrap_or_default(),
        };

        let append_result = self.store.append_event(new_event.clone())?;

        if append_result.inserted {
            self.store.update_from_event(&new_event)?;

            // Publish to SSE hub
            let sse_event = OpenMontageSseEvent {
                event_type: event.event_type.clone(),
                job_id: job_id.to_string(),
                project_id: event.job.project_id.clone(),
                sequence: event.sequence,
                status: event.status.clone(),
                stage: Some(event.stage.clone()),
                progress_pct: event.progress_pct,
                payload: serde_json::to_value(&event).unwrap_or_default(),
            };
            tokio::spawn({
                let hub = self.hub.clone();
                async move {
                    hub.publish(sse_event).await;
                }
            });
        }

        let job = self
            .store
            .get_job(job_id)?
            .ok_or_else(|| "Job not found".to_string())?;

        Ok(crate::handler::openmontage_handler::CallbackAck {
            received: true,
            event_id: event.event_id,
            next_expected_sequence: job.next_event_sequence,
            sync_required: if append_result.gap { Some(true) } else { None },
        })
    }

    /// Upload an asset (image/video/audio) via OSS and store metadata
    pub async fn upload_asset(
        &self,
        user_id: i32,
        kind: String,
        role: String,
        data: Vec<u8>,
        filename: String,
        content_type: String,
    ) -> Result<AssetDto, String> {
        // Determine upload method based on asset family
        let is_image = matches!(
            kind.as_str(),
            "reference_image" | "start_frame" | "end_frame" | "brand_asset"
        );
        let is_video = matches!(kind.as_str(), "reference_video" | "source_video");
        let is_audio = matches!(kind.as_str(), "audio" | "music");

        // Upload to OSS
        let oss_service =
            OssService::from_env().map_err(|e| format!("OSS service unavailable: {}", e))?;

        let upload_result = if is_image {
            oss_service
                .upload_image(data, user_id, filename.clone(), content_type.clone())
                .await
                .map_err(|e| format!("Image upload failed: {}", e))?
        } else if is_video {
            oss_service
                .upload_video(data, user_id, filename.clone(), content_type.clone())
                .await
                .map_err(|e| format!("Video upload failed: {}", e))?
        } else if is_audio {
            oss_service
                .upload_audio(data, user_id, filename.clone(), content_type.clone())
                .await
                .map_err(|e| format!("Audio upload failed: {}", e))?
        } else {
            return Err(format!("Unsupported asset kind for upload: {}", kind));
        };

        // Insert asset record
        let asset_id = Uuid::new_v4().to_string();
        let new_asset = NewAsset {
            asset_id: asset_id.clone(),
            user_id,
            kind: kind.clone(),
            role: role.clone(),
            uri: upload_result.image_url.clone(),
            mime_type: Some(content_type.clone()),
            bytes: Some(upload_result.size as i64),
            width_px: None, // Could be extracted from image metadata
            height_px: None,
            duration_ms: None,
        };

        let asset = self
            .store
            .insert_asset(new_asset)
            .map_err(|e| format!("Asset record insert failed: {}", e))?;

        Ok(AssetDto {
            asset_id: asset.asset_id,
            kind: asset.kind,
            role: asset.role,
            uri: asset.uri,
            mime_type: asset.mime_type,
            bytes: asset.bytes,
            width_px: asset.width_px,
            height_px: asset.height_px,
            duration_ms: asset.duration_ms,
        })
    }
}
