use redis::Client;

/// Lightweight Redis service providing per-account daily quota reservation.
///
/// Uses atomic INCR to guarantee that concurrent calls never exceed the limit.
/// Keys follow the pattern `reply_quota:{account_id}:{YYYY-MM-DD}` with a 26-hour TTL.
#[derive(Clone)]
pub struct RedisService {
    pub(crate) client: Client,
}

impl RedisService {
    pub fn new(client: Client) -> Self {
        Self { client }
    }

    /// Atomically try to reserve one reply slot for `account_id` on `date`.
    ///
    /// Returns `Ok(true)` if the reservation succeeded (counter < limit),
    /// `Ok(false)` if the account has reached its daily limit.
    pub fn try_reserve(
        &self,
        account_id: i32,
        date: &str,
        daily_limit: i32,
    ) -> Result<bool, String> {
        let key = format!("reply_quota:{}:{}", account_id, date);

        let mut conn = self
            .client
            .get_connection()
            .map_err(|e| format!("Redis connection error: {e}"))?;

        // INCR is atomic: returns the value *after* increment.
        let new_val: i32 = redis::cmd("INCR")
            .arg(&key)
            .query(&mut conn)
            .map_err(|e| format!("Redis INCR error: {e}"))?;

        if new_val == 1 {
            // First reservation today — set TTL (26 hours to cover timezone edge cases).
            let _: () = redis::cmd("EXPIRE")
                .arg(&key)
                .arg(93600) // 26 * 3600
                .query(&mut conn)
                .map_err(|e| format!("Redis EXPIRE error: {e}"))?;
        }

        if new_val <= daily_limit {
            Ok(true)
        } else {
            // Over limit — roll back the increment so counter stays accurate.
            let _: () = redis::cmd("DECR")
                .arg(&key)
                .query(&mut conn)
                .map_err(|e| format!("Redis DECR error: {e}"))?;
            Ok(false)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn get_test_redis() -> Option<RedisService> {
        let url = std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379".into());
        let client = Client::open(url.as_str()).ok()?;
        // Verify connectivity
        let mut conn = client.get_connection().ok()?;
        let _: Result<String, _> = redis::cmd("PING").query(&mut conn);
        Some(RedisService::new(client))
    }

    fn flush_key(svc: &RedisService, key: &str) {
        if let Ok(mut conn) = svc.client.get_connection() {
            let _: Result<(), _> = redis::cmd("DEL").arg(key).query(&mut conn);
        }
    }

    #[test]
    fn test_reserve_under_limit() {
        let svc = match get_test_redis() {
            Some(s) => s,
            None => {
                eprintln!("Skipping: Redis not available");
                return;
            }
        };
        let date = "test-reserve-under-limit";
        let key = format!("reply_quota:9001:{}", date);
        flush_key(&svc, &key);

        let result = svc.try_reserve(9001, date, 50).unwrap();
        assert!(result, "First reserve should succeed");

        flush_key(&svc, &key);
    }

    #[test]
    fn test_reserve_reaches_limit() {
        let svc = match get_test_redis() {
            Some(s) => s,
            None => {
                eprintln!("Skipping: Redis not available");
                return;
            }
        };
        let date = "test-reserve-reaches-limit";
        let key = format!("reply_quota:9002:{}", date);
        flush_key(&svc, &key);

        let limit = 3;
        for i in 0..limit {
            let ok = svc.try_reserve(9002, date, limit).unwrap();
            assert!(ok, "Reserve {} should succeed", i + 1);
        }

        let over = svc.try_reserve(9002, date, limit).unwrap();
        assert!(!over, "Reserve beyond limit should fail");

        flush_key(&svc, &key);
    }

    #[test]
    fn test_reserve_different_days() {
        let svc = match get_test_redis() {
            Some(s) => s,
            None => {
                eprintln!("Skipping: Redis not available");
                return;
            }
        };
        let day_a = "test-day-a";
        let day_b = "test-day-b";
        let key_a = format!("reply_quota:9003:{}", day_a);
        let key_b = format!("reply_quota:9003:{}", day_b);
        flush_key(&svc, &key_a);
        flush_key(&svc, &key_b);

        assert!(svc.try_reserve(9003, day_a, 1).unwrap());
        assert!(!svc.try_reserve(9003, day_a, 1).unwrap());
        // Different day should still succeed
        assert!(svc.try_reserve(9003, day_b, 1).unwrap());

        flush_key(&svc, &key_a);
        flush_key(&svc, &key_b);
    }

    #[test]
    fn test_reserve_different_accounts() {
        let svc = match get_test_redis() {
            Some(s) => s,
            None => {
                eprintln!("Skipping: Redis not available");
                return;
            }
        };
        let date = "test-diff-accounts";
        let key_a = format!("reply_quota:9004:{}", date);
        let key_b = format!("reply_quota:9005:{}", date);
        flush_key(&svc, &key_a);
        flush_key(&svc, &key_b);

        assert!(svc.try_reserve(9004, date, 1).unwrap());
        assert!(!svc.try_reserve(9004, date, 1).unwrap());
        // Different account should still succeed
        assert!(svc.try_reserve(9005, date, 1).unwrap());

        flush_key(&svc, &key_a);
        flush_key(&svc, &key_b);
    }

    #[test]
    fn test_key_expiry() {
        let svc = match get_test_redis() {
            Some(s) => s,
            None => {
                eprintln!("Skipping: Redis not available");
                return;
            }
        };
        let date = "test-key-expiry";
        let key = format!("reply_quota:9006:{}", date);
        flush_key(&svc, &key);

        svc.try_reserve(9006, date, 50).unwrap();

        let mut conn = svc.client.get_connection().unwrap();
        let ttl: i64 = redis::cmd("TTL").arg(&key).query(&mut conn).unwrap();
        assert!(
            (86400..=93600).contains(&ttl),
            "TTL should be between 86400 and 93600, got {}",
            ttl
        );

        flush_key(&svc, &key);
    }
}
