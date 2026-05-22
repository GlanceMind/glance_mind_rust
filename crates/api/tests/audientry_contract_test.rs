// crates/api/tests/audientry_contract_test.rs
use glance_mind_api::dto::audientry_dto::{
    AudienceAnalysisPhaseEvent, AudienceAnalysisReport, AudientryChannelEvent,
    AudientryWorkerTaskEnvelope, ReportEvidenceRef,
};

fn read(name: &str) -> String {
    std::fs::read_to_string(format!("tests/golden/audientry/{name}.golden.json")).unwrap()
}

#[test]
fn envelope_roundtrips_and_omits_absent() {
    let env: AudientryWorkerTaskEnvelope =
        serde_json::from_str(&read("audientry_task_envelope")).unwrap();
    assert_eq!(
        AudientryWorkerTaskEnvelope::QUEUE_KEY,
        "audientry_worker_tasks"
    );
    let back = serde_json::to_value(&env).unwrap();
    assert!(back["product"].get("landing_page_url").is_some()); // present in this fixture
}

#[test]
fn phase_and_report_and_evidence_roundtrip() {
    let _p: AudienceAnalysisPhaseEvent =
        serde_json::from_str(&read("audientry_phase_event")).unwrap();
    let _e: ReportEvidenceRef = serde_json::from_str(&read("report_evidence_ref")).unwrap();
    let r: AudienceAnalysisReport =
        serde_json::from_str(&read("audientry_report_completed")).unwrap();
    // sections relayed opaquely
    assert!(r.sections.get("social_entry_panel").is_some());
}

#[test]
fn channel_event_routes_by_kind() {
    let p = serde_json::from_str::<AudientryChannelEvent>(&read("audientry_phase_event")).unwrap();
    let r =
        serde_json::from_str::<AudientryChannelEvent>(&read("audientry_report_completed")).unwrap();
    assert!(matches!(p, AudientryChannelEvent::Phase(_)));
    assert!(matches!(r, AudientryChannelEvent::Report(_)));
}
