// crates/api/tests/audientry_flow_test.rs
//! Deterministic coverage for the /audientry interception routing decision and
//! the audit-row builder. The DB/LLM/redis-coupled `handle_audientry` itself is
//! exercised by the opt-in relay integration test + the cross-repo e2e gate.

use glance_mind_api::service::ai_chat::audientry_input::is_audientry_command;
use glance_mind_api::service::ai_chat_service::audientry_audit_row;

#[test]
fn slash_command_is_recognized_before_llm() {
    assert!(is_audientry_command("/audientry Sleep Tea\nherbal tea"));
    assert!(is_audientry_command("  /audientry"));
    assert!(!is_audientry_command("tell me about audientry"));
}

#[test]
fn audit_row_marks_failure_on_unsuccessful_relay() {
    let row = audientry_audit_row(7, 42, false);
    assert_eq!(row.tool_name, "audientry");
    assert!(!row.success);
    assert_eq!(row.conversation_id, Some(42));
    assert_eq!(row.user_id, 7);
}

#[test]
fn audit_row_marks_success_on_completed_relay() {
    let row = audientry_audit_row(7, 42, true);
    assert_eq!(row.tool_name, "audientry");
    assert!(row.success);
    assert!(row.error_message.is_none());
}
