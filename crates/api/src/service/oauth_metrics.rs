//! Prometheus counters for OAuth endpoints (R025).
//!
//! Required metric names (from the contract spec):
//!   - oauth_token_total{client_id, grant_type}
//!   - oauth_invalid_grant_total{client_id, error}
//!   - oauth_network_error_total{client_id}
//!   - oauth_revoke_total{client_id}
//!   - oauth_authorize_total{client_id}  (completeness; authorize is on web)

use once_cell::sync::Lazy;
use prometheus::{opts, register_counter_vec, CounterVec};

/// oauth_token_total — incremented on every successful /oauth/token grant
pub static OAUTH_TOKEN_TOTAL: Lazy<CounterVec> = Lazy::new(|| {
    register_counter_vec!(
        opts!("oauth_token_total", "Total successful OAuth token grants"),
        &["client_id", "grant_type"]
    )
    .expect("Failed to register oauth_token_total")
});

/// oauth_invalid_grant_total — incremented on every invalid_grant, invalid_client etc.
pub static OAUTH_INVALID_GRANT_TOTAL: Lazy<CounterVec> = Lazy::new(|| {
    register_counter_vec!(
        opts!(
            "oauth_invalid_grant_total",
            "Total OAuth invalid grant responses"
        ),
        &["client_id", "error"]
    )
    .expect("Failed to register oauth_invalid_grant_total")
});

/// oauth_network_error_total — incremented on 5xx / DB failures
pub static OAUTH_NETWORK_ERROR_TOTAL: Lazy<CounterVec> = Lazy::new(|| {
    register_counter_vec!(
        opts!(
            "oauth_network_error_total",
            "Total OAuth server/network errors"
        ),
        &["client_id"]
    )
    .expect("Failed to register oauth_network_error_total")
});

/// oauth_revoke_total — incremented on every /oauth/revoke call
pub static OAUTH_REVOKE_TOTAL: Lazy<CounterVec> = Lazy::new(|| {
    register_counter_vec!(
        opts!("oauth_revoke_total", "Total OAuth revoke calls"),
        &["client_id"]
    )
    .expect("Failed to register oauth_revoke_total")
});

/// oauth_authorize_total — incremented on every /oauth/authorize (completeness; lives on web)
pub static OAUTH_AUTHORIZE_TOTAL: Lazy<CounterVec> = Lazy::new(|| {
    register_counter_vec!(
        opts!(
            "oauth_authorize_total",
            "Total OAuth authorize requests (web surface)"
        ),
        &["client_id"]
    )
    .expect("Failed to register oauth_authorize_total")
});

/// Convenience: increment token grant counter
pub fn inc_token(client_id: &str, grant_type: &str) {
    OAUTH_TOKEN_TOTAL
        .with_label_values(&[client_id, grant_type])
        .inc();
}

/// Convenience: increment invalid grant counter
pub fn inc_invalid_grant(client_id: &str, error: &str) {
    OAUTH_INVALID_GRANT_TOTAL
        .with_label_values(&[client_id, error])
        .inc();
}

/// Convenience: increment network error counter
pub fn inc_network_error(client_id: &str) {
    OAUTH_NETWORK_ERROR_TOTAL
        .with_label_values(&[client_id])
        .inc();
}

/// Convenience: increment revoke counter
pub fn inc_revoke(client_id: &str) {
    OAUTH_REVOKE_TOTAL.with_label_values(&[client_id]).inc();
}
