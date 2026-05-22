// crates/api/src/dto/audientry_dto.rs
use serde::{Deserialize, Serialize};
use serde_json::Value;

// finding 1: pydantic FrozenModel injects contract_version on every payload. Declare it with a
// serde default on every typed struct so Rust both parses it and re-emits it (matching the
// Python-generated golden fixtures + what prod payloads actually contain).
fn default_contract_version() -> String {
    "2026-04-29".into()
}
fn default_locale() -> String {
    "en-US".into()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductBrief {
    #[serde(default = "default_contract_version")]
    pub contract_version: String,
    pub name: String,
    pub description: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub landing_page_url: Option<String>,
    #[serde(default = "default_locale")]
    pub locale: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudientryWorkerTaskEnvelope {
    #[serde(default = "default_contract_version")]
    pub contract_version: String,
    pub job_id: String,
    pub conversation_id: i32,
    pub user_id: i32,
    pub product: ProductBrief,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub questionnaire_answers: Option<std::collections::BTreeMap<String, String>>,
}
impl AudientryWorkerTaskEnvelope {
    pub const QUEUE_KEY: &'static str = "audientry_worker_tasks";
}
pub fn audientry_events_channel(job_id: &str) -> String {
    format!("audientry:job:{job_id}:events")
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportEvidenceRef {
    #[serde(default = "default_contract_version")]
    pub contract_version: String,
    pub ref_id: String,
    pub source: String,
    pub kind: String,
    pub quote: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub oss_ref: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub m04_decision: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudienceAnalysisPhaseEvent {
    #[serde(default = "default_contract_version")]
    pub contract_version: String,
    pub job_id: String,
    pub phase: String,
    pub module_name: String,
    pub status: String,
    pub partial_summary: String,
    #[serde(default)]
    pub evidence_refs: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub artifact_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub warning: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportSummary {
    #[serde(default = "default_contract_version")]
    pub contract_version: String,
    #[serde(default)]
    pub recommended_persona: String,
    pub headline_strategy: String,
    #[serde(default)]
    pub key_evidence: Vec<ReportEvidenceRef>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportAudit {
    #[serde(default = "default_contract_version")]
    pub contract_version: String,
    pub artifact_bundle_id: String,
    #[serde(default)]
    pub module_runs: Vec<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub evidence_snapshot_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudienceAnalysisReport {
    #[serde(default = "default_contract_version")]
    pub contract_version: String,
    pub job_id: String,
    pub status: String,
    pub summary: ReportSummary,
    pub sections: Value, // opaque pass-through (relay only); nested contract_version rides through
    #[serde(default)]
    pub evidence_index: Vec<ReportEvidenceRef>,
    pub audit: ReportAudit,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum AudientryChannelEvent {
    Phase(AudienceAnalysisPhaseEvent),
    Report(AudienceAnalysisReport),
}
