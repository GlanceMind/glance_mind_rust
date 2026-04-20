//! Validation for plan-level `PublishSchedule` JSON.
//!
//! Source of truth: `glance_mind_protocol/proto/aipub.proto` §996
//! (PublishSchedule message). Fields:
//!   * `scheduled_at` — REQUIRED. UTC ISO-8601 instant (RFC 3339).
//!   * `timezone`     — optional IANA tz name. Default "UTC".
//!   * `save_as_draft` — required bool (absent → false).
//!
//! Like `behavior_validation`, this module validates the JSON wire
//! shape rather than the prost Rust type so the public API contract
//! stays the source of truth and proto evolution (adding optional
//! fields) doesn't require a DTO change.

use crate::error::api_error::ApiError;
use crate::error::business_error::BusinessError;
use chrono::{DateTime, Utc};
use serde_json::Value as JsonValue;

/// Validate a plan's schedule payload.
///
/// `Ok(())` when the shape is acceptable (or `schedule` is `None`).
/// `Err(ApiError::BadRequest)` with a precise message otherwise.
///
/// Accepted:
///   * `null` / absent → use `None` caller-side; not this module's job.
///   * object with a non-empty `scheduled_at` that parses as RFC 3339.
///   * optional `timezone` string (we only check it's a string; we
///     don't dial IANA here — chronotz is heavier than this validator
///     should pull in. Bad tz will surface as a worker-side error).
///   * optional `save_as_draft` bool.
///   * unknown top-level keys pass through for forward-compat.
pub fn validate(schedule: Option<&JsonValue>) -> Result<(), ApiError> {
    let Some(value) = schedule else {
        return Ok(());
    };

    let map = value
        .as_object()
        .ok_or_else(|| invalid("schedule must be a JSON object".to_string()))?;

    // scheduled_at — required, non-empty string, RFC 3339.
    let scheduled_at = map
        .get("scheduled_at")
        .ok_or_else(|| invalid("schedule.scheduled_at is required (UTC ISO-8601)".to_string()))?;
    let scheduled_at = scheduled_at
        .as_str()
        .ok_or_else(|| invalid("schedule.scheduled_at must be a string".to_string()))?;
    if scheduled_at.is_empty() {
        return Err(invalid(
            "schedule.scheduled_at must not be empty".to_string(),
        ));
    }
    if DateTime::parse_from_rfc3339(scheduled_at).is_err() {
        return Err(invalid(format!(
            "schedule.scheduled_at must be RFC 3339 / ISO-8601 with timezone offset, got {:?}",
            scheduled_at
        )));
    }

    if let Some(tz) = map.get("timezone") {
        if !tz.is_null() && !tz.is_string() {
            return Err(invalid(
                "schedule.timezone must be a string (IANA tz name)".to_string(),
            ));
        }
    }

    if let Some(draft) = map.get("save_as_draft") {
        if !draft.is_boolean() {
            return Err(invalid(
                "schedule.save_as_draft must be a boolean".to_string(),
            ));
        }
    }

    Ok(())
}

/// Parse the `scheduled_at` out of a validated schedule payload.
/// Returns `None` when the schedule is absent; returns the parsed
/// `DateTime<Utc>` otherwise. Callers that want to compare against
/// `Utc::now()` (e.g. API-side filtering of ready tasks in a future
/// follow-up) can use this without re-validating.
pub fn parsed_scheduled_at(schedule: Option<&JsonValue>) -> Option<DateTime<Utc>> {
    let s = schedule?.as_object()?.get("scheduled_at")?.as_str()?;
    DateTime::parse_from_rfc3339(s)
        .ok()
        .map(|dt| dt.with_timezone(&Utc))
}

fn invalid(msg: String) -> ApiError {
    ApiError::BusinessError(BusinessError::InvalidInput(msg))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn none_is_valid() {
        assert!(validate(None).is_ok());
    }

    #[test]
    fn full_payload_passes() {
        let v = json!({
            "scheduled_at": "2027-06-01T12:30:00Z",
            "timezone": "America/Los_Angeles",
            "save_as_draft": true,
        });
        assert!(validate(Some(&v)).is_ok());
    }

    #[test]
    fn minimal_payload_passes() {
        let v = json!({"scheduled_at": "2027-06-01T12:30:00Z"});
        assert!(validate(Some(&v)).is_ok());
    }

    #[test]
    fn missing_scheduled_at_rejected() {
        let v = json!({"save_as_draft": true});
        let err = validate(Some(&v)).unwrap_err();
        assert!(err.to_string().contains("scheduled_at"), "{err}");
    }

    #[test]
    fn empty_scheduled_at_rejected() {
        let v = json!({"scheduled_at": ""});
        let err = validate(Some(&v)).unwrap_err();
        assert!(err.to_string().contains("scheduled_at"), "{err}");
    }

    #[test]
    fn non_iso_scheduled_at_rejected() {
        let v = json!({"scheduled_at": "not-a-timestamp"});
        assert!(validate(Some(&v)).is_err());
    }

    #[test]
    fn scheduled_at_without_timezone_offset_rejected() {
        // RFC 3339 requires a timezone designator. Bare "2027-06-01T12:30:00"
        // is ambiguous (UTC? local? unknown?) — reject so the worker
        // never has to guess.
        let v = json!({"scheduled_at": "2027-06-01T12:30:00"});
        assert!(validate(Some(&v)).is_err());
    }

    #[test]
    fn non_string_scheduled_at_rejected() {
        let v = json!({"scheduled_at": 1700000000});
        assert!(validate(Some(&v)).is_err());
    }

    #[test]
    fn non_bool_save_as_draft_rejected() {
        let v = json!({
            "scheduled_at": "2027-06-01T12:30:00Z",
            "save_as_draft": "yes",
        });
        let err = validate(Some(&v)).unwrap_err();
        assert!(err.to_string().contains("save_as_draft"), "{err}");
    }

    #[test]
    fn non_string_timezone_rejected() {
        let v = json!({
            "scheduled_at": "2027-06-01T12:30:00Z",
            "timezone": 42,
        });
        assert!(validate(Some(&v)).is_err());
    }

    #[test]
    fn null_timezone_is_ok() {
        // Explicit null → fall back to default "UTC" on worker side.
        let v = json!({
            "scheduled_at": "2027-06-01T12:30:00Z",
            "timezone": null,
        });
        assert!(validate(Some(&v)).is_ok());
    }

    #[test]
    fn unknown_top_level_keys_pass_through_for_forward_compat() {
        let v = json!({
            "scheduled_at": "2027-06-01T12:30:00Z",
            "future_proto_field": "anything",
        });
        assert!(validate(Some(&v)).is_ok());
    }

    #[test]
    fn non_object_top_level_rejected() {
        let v = json!("not an object");
        assert!(validate(Some(&v)).is_err());
    }

    #[test]
    fn parsed_scheduled_at_returns_utc_dt() {
        let v = json!({"scheduled_at": "2027-06-01T12:30:00Z"});
        let dt = parsed_scheduled_at(Some(&v)).unwrap();
        assert_eq!(dt.to_rfc3339(), "2027-06-01T12:30:00+00:00");
    }

    #[test]
    fn parsed_scheduled_at_returns_none_for_absent() {
        assert!(parsed_scheduled_at(None).is_none());
    }

    #[test]
    fn parsed_scheduled_at_normalises_non_utc_offset_to_utc() {
        let v = json!({"scheduled_at": "2027-06-01T05:30:00-07:00"});
        let dt = parsed_scheduled_at(Some(&v)).unwrap();
        // -07:00 → +00:00 means +7h of wall clock.
        assert_eq!(dt.to_rfc3339(), "2027-06-01T12:30:00+00:00");
    }
}
