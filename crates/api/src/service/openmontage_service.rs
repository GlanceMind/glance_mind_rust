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

        // M0-T7: Validate budget is finite and non-negative
        dto.validate_budget()?;

        // M0-T4: Validate pipeline allowlist + availability BEFORE enqueuing
        let pipeline = dto.pipeline.as_deref().unwrap_or("animated-explainer");

        // 1. Check allowlist (6 production pipelines)
        const PRODUCTION_PIPELINES: &[&str] = &[
            "animated-explainer",
            "animation",
            "avatar-spokesperson",
            "cinematic",
            "screen-demo",
            "hybrid",
        ];

        if !PRODUCTION_PIPELINES.contains(&pipeline) {
            return Err(format!(
                "Pipeline '{}' not found. Available pipelines: {}",
                pipeline,
                PRODUCTION_PIPELINES.join(", ")
            ));
        }

        // 2. Check availability from live preflight/pipelines snapshot
        let pipelines_dto = self.client.read_pipelines()?;
        if let Some(ref pipelines) = pipelines_dto {
            if let Some(info) = pipelines.pipelines.iter().find(|p| p.name == pipeline) {
                // Pipeline exists in snapshot - check stability
                if info.stability != "production" {
                    let preflight_dto = self.client.read_preflight()?;
                    let warnings = preflight_dto
                        .as_ref()
                        .map(|p| p.warnings.join("; "))
                        .unwrap_or_default();
                    return Err(format!(
                        "Pipeline '{}' is currently unavailable (stability: {}). {}",
                        pipeline,
                        info.stability,
                        if warnings.is_empty() {
                            "Check preflight for details."
                        } else {
                            &warnings
                        }
                    ));
                }
            } else {
                // Pipeline not in snapshot at all
                return Err(format!(
                    "Pipeline '{}' is currently unavailable. Available pipelines: {}",
                    pipeline,
                    pipelines
                        .pipelines
                        .iter()
                        .filter(|p| p.stability == "production")
                        .map(|p| p.name.as_str())
                        .collect::<Vec<_>>()
                        .join(", ")
                ));
            }
        }
        // If no pipelines snapshot, proceed (fallback to warming_up behavior)
        if pipelines_dto.is_none() {
            tracing::warn!(
                pipeline = %pipeline,
                "OpenMontage pipeline snapshot unavailable; proceeding (warming-up fallback)"
            );
        }

        // M0-T5: Compute idempotency key and request hash
        let (effective_idempotency_key, request_hash) = derive_idempotency_key(&dto)?;

        // Generate IDs for new job
        let job_id = Uuid::new_v4().to_string();
        let project_id = format!("omx-{}", job_id);
        let request_id = Uuid::new_v4().to_string();

        // Server context
        let server_ctx = ServerContext {
            job_id: job_id.clone(),
            user_id,
            tenant_id: tenant_id.to_string(),
            callback_secret_ref: Some("internal-callback-secret".to_string()),
        };

        let mut request_json = dto.to_protocol_request(server_ctx);

        // M2: Resolve asset_ids and build assets/tool_invocations based on input_mode
        let mut resolved_asset_roles = Vec::new();
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

                    // M4-T3: Collect resolved asset roles for per-pipeline validation
                    resolved_asset_roles.push(asset.role.clone());

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

                // Build tool_invocations based on input_mode using the DTO's mapping helper
                if let Some(ref input_mode) = dto.input_mode {
                    tool_invocations = CreateJobDto::build_tool_invocations_for_input_mode(
                        input_mode,
                        &dto.prompt,
                        dto.duration_seconds.unwrap_or(60),
                        &assets_array,
                    );
                }

                request_json["assets"] = serde_json::Value::Array(assets_array.clone());
                if !tool_invocations.is_empty() {
                    request_json["tool_invocations"] = serde_json::Value::Array(tool_invocations);
                }

                // M0b-T5: Scan resolved assets for secret material
                // After assets are resolved and merged into request_json, scan them recursively
                use crate::dto::openmontage_dto::scan_json_for_secrets;
                let assets_value = serde_json::Value::Array(assets_array);
                scan_json_for_secrets(&assets_value, "resolved_assets")
                    .map_err(|e| format!("secret_material_rejected: {}", e))?;
            }
        }

        // M4-T3: Validate per-pipeline required asset roles (hybrid requires source_video)
        if let Some(ref input_mode_str) = dto.input_mode {
            use crate::dto::openmontage_dto::{validate_input_mode_for_pipeline, InputMode};

            // Parse input_mode string to InputMode enum
            let input_mode: InputMode = serde_json::from_str(&format!("\"{}\"", input_mode_str))
                .map_err(|_| format!("Invalid input_mode: {}", input_mode_str))?;

            // Validate input_mode + required asset roles for this pipeline
            validate_input_mode_for_pipeline(pipeline, input_mode, &resolved_asset_roles)
                .map_err(|e| format!("validation_error: {}", e))?;
        }

        // M5-T3: Reject screen-demo real_capture at intake (cannot run server-side)
        if pipeline == "screen-demo" {
            match dto.production_mode.as_deref() {
                Some("synthetic_terminal") | Some("uploaded_recording") => {
                    // Valid screen-demo production modes — pass through
                }
                Some("real_capture") => {
                    return Err(
                        "screen-demo real_capture cannot run server-side — choose synthetic_terminal or upload a recording (uploaded_recording)."
                            .to_string(),
                    );
                }
                None | Some(_) => {
                    // production_mode absent or invalid value → reject with actionable message
                    return Err(
                        "screen-demo requires production_mode (synthetic_terminal or uploaded_recording)."
                            .to_string(),
                    );
                }
            }
        }

        let new_job = NewJob {
            job_id: job_id.clone(),
            project_id: project_id.clone(),
            user_id,
            tenant_id: tenant_id.to_string(),
            request_id: request_id.clone(),
            idempotency_key: effective_idempotency_key.clone(),
            request_hash: request_hash.clone(),
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
            render_runtime: dto.render_runtime.clone(),
            approval_policy: dto.approval_policy.clone(),
            budget_limit_usd: dto.budget_limit_usd,
        };

        // M0b-T1: Atomic upsert at the store level — returns (job, created: bool)
        let result = self.store.create_job(new_job)?;

        // M0-T5 + M0b-T1: Idempotency conflict check (if existing job found, verify request_hash)
        if !result.created {
            // Found existing job — check request hash
            if result.job.request_hash == request_hash {
                // Same key + same body → return existing job (NO enqueue)
                return Ok(JobSnapshotDto {
                    job_id: result.job.job_id.clone(),
                    project_id: result.job.project_id.clone(),
                    status: result.job.status.clone(),
                    pipeline: result.job.pipeline.clone(),
                    current_stage: result.job.current_stage.clone(),
                    progress_pct: result.job.progress_pct,
                    error_json: result.job.error_json.clone(),
                    last_event_sequence: result.job.last_event_sequence,
                    next_event_sequence: result.job.next_event_sequence,
                    sync_required: result.job.sync_required,
                    snapshot_json: result.job.snapshot_json.clone(),
                    created_at: result.job.created_at.to_rfc3339(),
                    updated_at: result.job.updated_at.map(|t| t.to_rfc3339()),
                    artifacts: Self::extract_artifacts(&result.job.snapshot_json),
                });
            } else {
                // Same key + different body → HTTP 409 conflict (NO enqueue)
                return Err(
                    "Idempotency conflict: same key with different request body".to_string()
                );
            }
        }

        // NEW JOB CREATED — enqueue the run
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

        self.client.enqueue_run(envelope)?;

        Ok(JobSnapshotDto {
            job_id: result.job.job_id.clone(),
            project_id: result.job.project_id.clone(),
            status: result.job.status.clone(),
            pipeline: result.job.pipeline.clone(),
            current_stage: result.job.current_stage.clone(),
            progress_pct: result.job.progress_pct,
            error_json: result.job.error_json.clone(),
            last_event_sequence: result.job.last_event_sequence,
            next_event_sequence: result.job.next_event_sequence,
            sync_required: result.job.sync_required,
            snapshot_json: result.job.snapshot_json.clone(),
            created_at: result.job.created_at.to_rfc3339(),
            updated_at: result.job.updated_at.map(|t| t.to_rfc3339()),
            artifacts: Self::extract_artifacts(&result.job.snapshot_json),
        })
    }

    /// M0-T6: Get raw job record for authorization checks.
    /// Use this before calling other methods to verify ownership.
    pub fn get_job_raw(
        &self,
        job_id: &str,
    ) -> Result<Option<crate::repository::openmontage_repository::Job>, String> {
        self.store.get_job(job_id)
    }

    /// M4-T5b: Extract artifacts from snapshot_json (defensive - returns [] if missing/malformed).
    fn extract_artifacts(
        snapshot_json: &serde_json::Value,
    ) -> Vec<crate::handler::openmontage_handler::ArtifactDto> {
        snapshot_json
            .get("artifacts")
            .and_then(|v| v.as_array())
            .and_then(|arr| serde_json::from_value(serde_json::Value::Array(arr.clone())).ok())
            .unwrap_or_default()
    }

    /// Convert a Job entity to JobSnapshotDto.
    /// Used internally to avoid duplicate DB reads on read paths.
    pub fn snapshot_from_job(
        job: &crate::repository::openmontage_repository::Job,
    ) -> JobSnapshotDto {
        JobSnapshotDto {
            job_id: job.job_id.clone(),
            project_id: job.project_id.clone(),
            status: job.status.clone(),
            pipeline: job.pipeline.clone(),
            current_stage: job.current_stage.clone(),
            progress_pct: job.progress_pct,
            error_json: job.error_json.clone(),
            last_event_sequence: job.last_event_sequence,
            next_event_sequence: job.next_event_sequence,
            sync_required: job.sync_required,
            snapshot_json: job.snapshot_json.clone(),
            created_at: job.created_at.to_rfc3339(),
            updated_at: job.updated_at.map(|t| t.to_rfc3339()),
            artifacts: Self::extract_artifacts(&job.snapshot_json),
        }
    }

    pub fn get_job(&self, job_id: &str) -> Result<Option<JobSnapshotDto>, String> {
        let job = self.store.get_job(job_id)?;
        Ok(job.as_ref().map(Self::snapshot_from_job))
    }

    pub fn cancel_job(&self, job_id: &str) -> Result<CancelResultDto, String> {
        // Re-fetch job for fresh state (mutations need latest status; read paths reuse already-fetched job)
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
        // Re-fetch job for fresh state (mutations need latest status; read paths reuse already-fetched job)
        let job = self
            .store
            .get_job(job_id)?
            .ok_or_else(|| "Job not found".to_string())?;

        // Enqueue resume with approval decision
        let envelope = WorkerEnvelope {
            task_id: Uuid::new_v4().to_string(),
            job_id: job_id.to_string(),
            project_id: job.project_id.clone(),
            attempt: 1,
            max_attempts: 3,
            kind: "resume".to_string(),
            request_json: serde_json::json!({}), // empty for resume
            resume_from_stage: None,             // worker will infer from checkpoint
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

        // M0b-T5: Redact secrets in event payload BEFORE persistence + SSE publish
        use crate::dto::openmontage_dto::redact_secrets_in_json;
        let raw_event_json = serde_json::to_value(&event).unwrap_or_default();
        let redacted_event_json = redact_secrets_in_json(&raw_event_json);

        let new_event = NewJobEvent {
            job_id: job_id.to_string(),
            sequence: event.sequence,
            event_id: event.event_id.clone(),
            event_type: event.event_type.clone(),
            status: Some(event.status.clone()),
            event_json: redacted_event_json.clone(),
        };

        let append_result = self.store.append_event(new_event.clone())?;

        if append_result.inserted {
            self.store.update_from_event(&new_event)?;

            // Publish to SSE hub (with redacted payload)
            let sse_event = OpenMontageSseEvent {
                event_type: event.event_type.clone(),
                job_id: job_id.to_string(),
                project_id: event.job.project_id.clone(),
                sequence: event.sequence,
                status: event.status.clone(),
                stage: Some(event.stage.clone()),
                progress_pct: event.progress_pct,
                payload: redacted_event_json,
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
            "reference_image" | "start_frame" | "end_frame" | "brand_asset" | "avatar" // M3-T2: avatar is an image
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

/// Pure idempotency key derivation — extracts the canonical-body hashing logic
/// and effective-key selection from the async `create_job` method.
///
/// Returns `(effective_key, request_hash)` where:
/// - `request_hash` is the SHA-256 hex digest of the canonical request body
/// - `effective_key` is either the client-supplied `idempotency_key` (if present)
///   or the `request_hash` itself (if absent)
///
/// # Canonical Body Contract
///
/// IDEMPOTENCY CONTRACT: when adding a field to CreateJobDto, decide if it affects idempotency.
/// If yes, add it here; if no, leave it out deliberately. Omitting a meaningful field silently
/// breaks dedup. (idempotency_key is excluded by design — no self-reference.)
fn derive_idempotency_key(dto: &CreateJobDto) -> Result<(String, String), String> {
    use sha2::{Digest, Sha256};

    // Canonical request body for hashing (exclude idempotency_key itself)
    // M0b-T6: Include new cross-tier fields in idempotency hash
    let canonical_body = serde_json::json!({
        "title": dto.title,
        "prompt": dto.prompt,
        "target_platform": dto.target_platform,
        "language": dto.language,
        "duration_seconds": dto.duration_seconds,
        "aspect_ratio": dto.aspect_ratio,
        "input_mode": dto.input_mode,
        "pipeline": dto.pipeline,
        "style_playbook": dto.style_playbook,
        "render_runtime": dto.render_runtime,
        "quality_tier": dto.quality_tier,
        "approval_policy": dto.approval_policy,
        "budget_limit_usd": dto.budget_limit_usd,
        "provider_slots": dto.provider_slots,
        "asset_ids": dto.asset_ids,
        "tool_invocations": dto.tool_invocations,
        "metadata": dto.metadata,
        "source_script": dto.source_script,
        "voice_selection": dto.voice_selection,
        "production_mode": dto.production_mode,
        "audience": dto.audience,
        "objective": dto.objective,
        "brand_json": dto.brand_json,
    });

    let canonical_str = serde_json::to_string(&canonical_body)
        .map_err(|e| format!("Failed to serialize canonical body: {}", e))?;

    // Compute request hash (SHA-256)
    let mut hasher = Sha256::new();
    hasher.update(canonical_str.as_bytes());
    let request_hash = format!("{:x}", hasher.finalize());

    // Effective idempotency key: client-supplied or derived from body hash
    let effective_idempotency_key = dto
        .idempotency_key
        .clone()
        .unwrap_or_else(|| request_hash.clone());

    Ok((effective_idempotency_key, request_hash))
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        /// Property: derive_idempotency_key is deterministic — calling it twice
        /// with the same DTO should return the same (effective_key, request_hash).
        #[test]
        fn prop_derive_idempotency_key_is_deterministic(
            title in ".{1,100}",
            prompt in ".{1,200}",
            platform in "(youtube|tiktok|instagram)",
            key in proptest::option::of(".{1,50}"),
        ) {
            let dto = CreateJobDto {
                title: title.clone(),
                prompt: prompt.clone(),
                target_platform: platform.clone(),
                idempotency_key: key.clone(),
                ..Default::default()
            };

            let (key1, hash1) = derive_idempotency_key(&dto)
                .expect("derive_idempotency_key should succeed");
            let (key2, hash2) = derive_idempotency_key(&dto)
                .expect("derive_idempotency_key should succeed");

            prop_assert_eq!(key1, key2, "effective_key must be deterministic");
            prop_assert_eq!(hash1, hash2, "request_hash must be deterministic");
        }

        /// Property: client-supplied idempotency_key takes precedence —
        /// if dto.idempotency_key is Some(k), then effective_key == k.
        #[test]
        fn prop_client_key_takes_precedence(
            title in ".{1,100}",
            prompt in ".{1,200}",
            client_key in ".{1,50}",
        ) {
            let dto = CreateJobDto {
                title,
                prompt,
                target_platform: "youtube".to_string(),
                idempotency_key: Some(client_key.clone()),
                ..Default::default()
            };

            let (effective_key, _hash) = derive_idempotency_key(&dto)
                .expect("derive_idempotency_key should succeed");

            prop_assert_eq!(effective_key, client_key,
                "When idempotency_key is Some(k), effective_key must be k");
        }

        /// Property: idempotency_key is excluded from hash —
        /// two DTOs identical except for different idempotency_key values
        /// should produce the SAME request_hash.
        #[test]
        fn prop_idempotency_key_excluded_from_hash(
            title in ".{1,100}",
            prompt in ".{1,200}",
            key1 in ".{1,50}",
            key2 in ".{1,50}",
        ) {
            // Filter out case where keys are identical (would be trivial)
            prop_assume!(key1 != key2);

            let dto1 = CreateJobDto {
                title: title.clone(),
                prompt: prompt.clone(),
                target_platform: "youtube".to_string(),
                idempotency_key: Some(key1),
                ..Default::default()
            };

            let dto2 = CreateJobDto {
                title,
                prompt,
                target_platform: "youtube".to_string(),
                idempotency_key: Some(key2),
                ..Default::default()
            };

            let (_eff1, hash1) = derive_idempotency_key(&dto1)
                .expect("derive_idempotency_key should succeed");
            let (_eff2, hash2) = derive_idempotency_key(&dto2)
                .expect("derive_idempotency_key should succeed");

            prop_assert_eq!(hash1, hash2,
                "request_hash must be identical when only idempotency_key differs");
        }

        /// Property: screen-demo create accepted ⟺ production_mode ∈ {synthetic_terminal, uploaded_recording}
        /// M5-T3: Property test for screen-demo production_mode validation
        #[test]
        fn prop_screen_demo_acceptance_iff_valid_production_mode(
            title in ".{1,100}",
            prompt in ".{1,200}",
            production_mode in proptest::option::of("(synthetic_terminal|uploaded_recording|real_capture|invalid_mode)"),
        ) {
            use crate::repository::openmontage_repository::InMemoryJobStore;
            use crate::service::openmontage_client::MockOpenMontageClient;
            use crate::service::openmontage_stream_hub::OpenMontageStreamHub;
            use std::sync::Arc;

            let store = Arc::new(InMemoryJobStore::new());
            let client = Arc::new(MockOpenMontageClient::new());
            let hub = OpenMontageStreamHub::new();
            let service = OpenMontageService::new(store.clone(), client.clone(), hub);

            let dto = CreateJobDto {
                title,
                prompt,
                target_platform: "youtube".to_string(),
                pipeline: Some("screen-demo".to_string()),
                input_mode: Some("text_to_video".to_string()),
                production_mode: production_mode.clone(),
                ..Default::default()
            };

            let result = service.create_job(1, "test-tenant", dto);

            // Define acceptance condition: production_mode is Some and in {synthetic_terminal, uploaded_recording}
            let is_valid_mode = production_mode
                .as_ref()
                .map(|m| m == "synthetic_terminal" || m == "uploaded_recording")
                .unwrap_or(false);

            if is_valid_mode {
                // Should succeed
                prop_assert!(result.is_ok(), "Valid production_mode should succeed, got error: {:?}", result.err());
                // Verify job was enqueued
                prop_assert_eq!(client.get_enqueued().len(), 1, "Valid mode should enqueue job");
            } else {
                // Should reject with error mentioning screen-demo or production_mode
                prop_assert!(result.is_err(), "Invalid or absent production_mode should fail");
                let err = result.unwrap_err();
                prop_assert!(
                    err.contains("screen-demo") || err.contains("production_mode"),
                    "Error should mention screen-demo or production_mode, got: {}",
                    err
                );
                // Verify no enqueue
                prop_assert_eq!(client.get_enqueued().len(), 0, "Invalid mode should not enqueue");
            }
        }
    }
}
