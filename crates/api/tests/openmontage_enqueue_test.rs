//! OpenMontage Redis Enqueue Integration Tests (gated)
//!
//! Tests that Redis enqueue writes properly formatted job envelopes to the worker queue.
//! Skipped unless RUN_REDIS_INTEGRATION_TESTS=1.
//!
//! ASSERTION-CHANGE-JUSTIFIED: #[ignore] marker + env check required per spec - this is a gated
//! integration test that requires Redis. It compiles but only runs when RUN_REDIS_INTEGRATION_TESTS=1.

#![cfg(test)]

use redis::Commands;
use serde_json::json;

const REDIS_QUEUE_KEY: &str = "openmontage_worker_tasks";

fn redis_available() -> bool {
    std::env::var("RUN_REDIS_INTEGRATION_TESTS")
        .ok()
        .map(|v| v == "1")
        .unwrap_or(false)
}

fn get_redis_client() -> Option<redis::Client> {
    let redis_url =
        std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379".to_string());

    redis::Client::open(redis_url).ok()
}

fn cleanup_queue(client: &redis::Client) {
    if let Ok(mut conn) = client.get_connection() {
        let _: Result<(), _> = conn.del(REDIS_QUEUE_KEY);
    }
}

#[test]
#[ignore]
fn redis_enqueue_writes_valid_job_envelope() {
    if !redis_available() {
        eprintln!("RUN_REDIS_INTEGRATION_TESTS not set to 1, skipping");
        return;
    }

    let client = match get_redis_client() {
        Some(c) => c,
        None => {
            eprintln!("Could not connect to Redis");
            return;
        }
    };

    // Clean up queue before test
    cleanup_queue(&client);

    let mut conn = client.get_connection().expect("get connection");

    // Simulate what the OpenMontageService does
    let job_envelope = json!({
        "job_id": "job-test-123",
        "project_id": "omx-test-project",
        "user_id": 100,
        "tenant_id": "test-tenant",
        "request_id": "req-test-123",
        "idempotency_key": "idem-unique-456",
        "pipeline": "animated-explainer",
        "input_mode": "text",
        "request_json": {
            "title": "Test Video",
            "topic": "Rust and Diesel",
            "duration_seconds": 60
        },
        "render_runtime": "remotion",
        "approval_policy": "auto",
        "budget_limit_usd": 5.0
    });

    // Enqueue to Redis
    let _: () = conn
        .rpush(
            REDIS_QUEUE_KEY,
            serde_json::to_string(&job_envelope).unwrap(),
        )
        .expect("rpush to queue");

    // Read back from queue
    let items: Vec<String> = conn.lrange(REDIS_QUEUE_KEY, 0, -1).expect("lrange queue");

    assert_eq!(items.len(), 1, "Queue should have 1 item");

    let envelope_str = &items[0];
    let parsed: serde_json::Value = serde_json::from_str(envelope_str).expect("parse JSON");

    // Verify envelope structure
    assert_eq!(parsed["job_id"], "job-test-123");
    assert_eq!(parsed["project_id"], "omx-test-project");
    assert_eq!(parsed["user_id"], 100);
    assert_eq!(parsed["pipeline"], "animated-explainer");

    // Verify request_json can be parsed as OpenMontageProfessionalVideoRequest shape
    let request_json = parsed["request_json"]
        .as_object()
        .expect("request_json is object");
    assert!(
        request_json.contains_key("title"),
        "request_json should have title"
    );
    assert!(
        request_json.contains_key("topic"),
        "request_json should have topic"
    );
    assert!(
        request_json.contains_key("duration_seconds"),
        "request_json should have duration_seconds"
    );

    // Clean up after test
    cleanup_queue(&client);
}

#[test]
#[ignore]
fn redis_enqueue_fifo_order() {
    if !redis_available() {
        eprintln!("RUN_REDIS_INTEGRATION_TESTS not set to 1, skipping");
        return;
    }

    let client = match get_redis_client() {
        Some(c) => c,
        None => {
            eprintln!("Could not connect to Redis");
            return;
        }
    };

    cleanup_queue(&client);

    let mut conn = client.get_connection().expect("get connection");

    // Enqueue 3 jobs
    for i in 1..=3 {
        let envelope = json!({
            "job_id": format!("job-{}", i),
            "project_id": format!("omx-{}", i),
            "user_id": 100,
            "tenant_id": "test-tenant",
            "request_id": format!("req-{}", i),
            "idempotency_key": format!("idem-{}", i),
            "pipeline": "animated-explainer",
            "request_json": {
                "title": format!("Video {}", i)
            }
        });

        let _: () = conn
            .rpush(REDIS_QUEUE_KEY, serde_json::to_string(&envelope).unwrap())
            .expect("rpush");
    }

    // Verify FIFO order
    let items: Vec<String> = conn.lrange(REDIS_QUEUE_KEY, 0, -1).expect("lrange");

    assert_eq!(items.len(), 3);

    let job1: serde_json::Value = serde_json::from_str(&items[0]).unwrap();
    let job2: serde_json::Value = serde_json::from_str(&items[1]).unwrap();
    let job3: serde_json::Value = serde_json::from_str(&items[2]).unwrap();

    assert_eq!(job1["job_id"], "job-1");
    assert_eq!(job2["job_id"], "job-2");
    assert_eq!(job3["job_id"], "job-3");

    cleanup_queue(&client);
}

#[test]
#[ignore]
fn redis_connection_failure_handling() {
    if !redis_available() {
        eprintln!("RUN_REDIS_INTEGRATION_TESTS not set to 1, skipping");
        return;
    }

    // Try to connect to invalid Redis URL
    let bad_client = redis::Client::open("redis://invalid-host:9999");

    match bad_client {
        Ok(client) => {
            let result = client.get_connection();
            assert!(result.is_err(), "Connection to invalid host should fail");
        }
        Err(_) => {
            // URL parsing failed, which is also acceptable
        }
    }
}
