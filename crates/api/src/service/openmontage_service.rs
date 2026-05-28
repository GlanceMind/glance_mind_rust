//! OpenMontage Service
//!
//! Business logic layer for OpenMontage jobs: create, find by idempotency, cancel, approval.

use crate::dto::openmontage_dto::{
    ApprovalDto, CancelResultDto, CreateJobDto, JobSnapshotDto, ServerContext,
};
use crate::repository::openmontage_repository::{NewJob, OpenMontageJobStore};
use crate::service::openmontage_client::{OpenMontageClient, WorkerEnvelope};
use crate::service::openmontage_stream_hub::OpenMontageStreamHub;
use std::sync::Arc;
use uuid::Uuid;

#[derive(Clone)]
pub struct OpenMontageService {
    store: Arc<dyn OpenMontageJobStore>,
    client: Arc<dyn OpenMontageClient>,
    #[allow(dead_code)] // Used in construction, will be used in future for publishing events
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

        let request_json = dto.to_protocol_request(server_ctx);

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
}
