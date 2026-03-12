use redis::Client;

/// Per-account daily quota reservation backed by Redis.
///
/// Uses an atomic Lua script to guarantee INCR + EXPIRE + limit-check happen
/// in a single round-trip with no crash window between commands.
///
/// Keys follow the pattern `reply_quota:{account_id}:{YYYY-MM-DD}` with a 26-hour TTL
/// (covers UTC timezone edge cases).
#[derive(Clone)]
pub struct RedisService {
    pub(crate) client: Client,
}

/// Lua script executed atomically inside Redis.
///
/// 1. INCR the counter.
/// 2. If this is the first increment (val==1), set a 26h TTL.
/// 3. If the counter is within the limit, return 1 (reserved).
/// 4. Otherwise DECR to roll back and return 0 (exhausted).
///
/// Because the whole script runs in a single EVAL, there is no window
/// where the key exists without a TTL, and no window where the counter
/// is temporarily over the limit visible to other callers.
const RESERVE_SCRIPT: &str = r#"
local val = redis.call('INCR', KEYS[1])
if val == 1 then
    redis.call('EXPIRE', KEYS[1], tonumber(ARGV[2]))
end
if val <= tonumber(ARGV[1]) then
    return 1
else
    redis.call('DECR', KEYS[1])
    return 0
end
"#;

const TTL_SECONDS: i64 = 93600; // 26 * 3600

impl RedisService {
    pub fn new(client: Client) -> Self {
        Self { client }
    }

    /// Atomically try to reserve one reply slot for `account_id` on `date`.
    ///
    /// Returns `Ok(true)` if the reservation succeeded (counter <= limit),
    /// `Ok(false)` if the account has reached its daily limit.
    pub fn try_reserve(
        &self,
        account_id: i32,
        date: &str,
        daily_limit: i32,
    ) -> Result<bool, String> {
        if daily_limit <= 0 {
            return Ok(false);
        }

        let key = format!("reply_quota:{}:{}", account_id, date);

        let mut conn = self
            .client
            .get_connection()
            .map_err(|e| format!("Redis connection error: {e}"))?;

        let result: i32 = redis::cmd("EVAL")
            .arg(RESERVE_SCRIPT)
            .arg(1) // number of KEYS
            .arg(&key)
            .arg(daily_limit)
            .arg(TTL_SECONDS)
            .query(&mut conn)
            .map_err(|e| format!("Redis EVAL error: {e}"))?;

        Ok(result == 1)
    }

    /// Release a previously reserved quota slot (e.g. when a reply fails).
    ///
    /// Decrements the counter but will not let it go below zero.
    pub fn release_quota(
        &self,
        account_id: i32,
        date: &str,
    ) -> Result<(), String> {
        let key = format!("reply_quota:{}:{}", account_id, date);

        let mut conn = self
            .client
            .get_connection()
            .map_err(|e| format!("Redis connection error: {e}"))?;

        let val: i32 = redis::cmd("DECR")
            .arg(&key)
            .query(&mut conn)
            .map_err(|e| format!("Redis DECR error: {e}"))?;

        // If we went below zero (key didn't exist or was already 0), correct it
        if val < 0 {
            let _: () = redis::cmd("INCR")
                .arg(&key)
                .query(&mut conn)
                .map_err(|e| format!("Redis INCR correction error: {e}"))?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn get_test_redis() -> Option<RedisService> {
        let url = std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379".into());
        let client = Client::open(url.as_str()).ok()?;
        let mut conn = client.get_connection().ok()?;
        let _: Result<String, _> = redis::cmd("PING").query(&mut conn);
        Some(RedisService::new(client))
    }

    fn flush_key(svc: &RedisService, key: &str) {
        if let Ok(mut conn) = svc.client.get_connection() {
            let _: Result<(), _> = redis::cmd("DEL").arg(key).query(&mut conn);
        }
    }

    fn get_counter(svc: &RedisService, key: &str) -> i32 {
        let mut conn = svc.client.get_connection().unwrap();
        redis::cmd("GET").arg(key).query::<Option<i32>>(&mut conn).unwrap().unwrap_or(0)
    }

    #[test]
    fn test_reserve_under_limit() {
        let svc = match get_test_redis() {
            Some(s) => s,
            None => { eprintln!("Skipping: Redis not available"); return; }
        };
        let date = "test-reserve-under-limit";
        let key = format!("reply_quota:9001:{}", date);
        flush_key(&svc, &key);

        assert!(svc.try_reserve(9001, date, 50).unwrap(), "First reserve should succeed");
        assert_eq!(get_counter(&svc, &key), 1, "Counter should be 1 after one reserve");

        flush_key(&svc, &key);
    }

    #[test]
    fn test_reserve_reaches_limit() {
        let svc = match get_test_redis() {
            Some(s) => s,
            None => { eprintln!("Skipping: Redis not available"); return; }
        };
        let date = "test-reserve-reaches-limit";
        let key = format!("reply_quota:9002:{}", date);
        flush_key(&svc, &key);

        let limit = 3;
        for i in 0..limit {
            assert!(svc.try_reserve(9002, date, limit).unwrap(), "Reserve {} should succeed", i + 1);
        }
        assert!(!svc.try_reserve(9002, date, limit).unwrap(), "Reserve beyond limit should fail");
        assert_eq!(get_counter(&svc, &key), limit, "Counter should stay at limit after rejection");

        flush_key(&svc, &key);
    }

    #[test]
    fn test_reserve_different_days() {
        let svc = match get_test_redis() {
            Some(s) => s,
            None => { eprintln!("Skipping: Redis not available"); return; }
        };
        let day_a = "test-day-a";
        let day_b = "test-day-b";
        let key_a = format!("reply_quota:9003:{}", day_a);
        let key_b = format!("reply_quota:9003:{}", day_b);
        flush_key(&svc, &key_a);
        flush_key(&svc, &key_b);

        assert!(svc.try_reserve(9003, day_a, 1).unwrap());
        assert!(!svc.try_reserve(9003, day_a, 1).unwrap());
        assert!(svc.try_reserve(9003, day_b, 1).unwrap());

        flush_key(&svc, &key_a);
        flush_key(&svc, &key_b);
    }

    #[test]
    fn test_reserve_different_accounts() {
        let svc = match get_test_redis() {
            Some(s) => s,
            None => { eprintln!("Skipping: Redis not available"); return; }
        };
        let date = "test-diff-accounts";
        let key_a = format!("reply_quota:9004:{}", date);
        let key_b = format!("reply_quota:9005:{}", date);
        flush_key(&svc, &key_a);
        flush_key(&svc, &key_b);

        assert!(svc.try_reserve(9004, date, 1).unwrap());
        assert!(!svc.try_reserve(9004, date, 1).unwrap());
        assert!(svc.try_reserve(9005, date, 1).unwrap());

        flush_key(&svc, &key_a);
        flush_key(&svc, &key_b);
    }

    #[test]
    fn test_key_expiry() {
        let svc = match get_test_redis() {
            Some(s) => s,
            None => { eprintln!("Skipping: Redis not available"); return; }
        };
        let date = "test-key-expiry";
        let key = format!("reply_quota:9006:{}", date);
        flush_key(&svc, &key);

        svc.try_reserve(9006, date, 50).unwrap();

        let mut conn = svc.client.get_connection().unwrap();
        let ttl: i64 = redis::cmd("TTL").arg(&key).query(&mut conn).unwrap();
        assert!(
            (86400..=93600).contains(&ttl),
            "TTL should be between 86400 and 93600, got {}", ttl
        );

        flush_key(&svc, &key);
    }

    #[test]
    fn test_lua_script_counter_stays_at_limit_after_rejection() {
        let svc = match get_test_redis() {
            Some(s) => s,
            None => { eprintln!("Skipping: Redis not available"); return; }
        };
        let date = "test-lua-counter-stable";
        let key = format!("reply_quota:9010:{}", date);
        flush_key(&svc, &key);

        assert!(svc.try_reserve(9010, date, 2).unwrap());
        assert!(svc.try_reserve(9010, date, 2).unwrap());
        assert!(!svc.try_reserve(9010, date, 2).unwrap());
        assert!(!svc.try_reserve(9010, date, 2).unwrap());
        assert!(!svc.try_reserve(9010, date, 2).unwrap());
        // Counter should stay at exactly 2, not drift upward
        assert_eq!(get_counter(&svc, &key), 2, "Counter must stay at limit after repeated rejections");

        flush_key(&svc, &key);
    }

    #[test]
    fn test_ttl_always_set_even_at_limit() {
        let svc = match get_test_redis() {
            Some(s) => s,
            None => { eprintln!("Skipping: Redis not available"); return; }
        };
        let date = "test-ttl-always-set";
        let key = format!("reply_quota:9011:{}", date);
        flush_key(&svc, &key);

        svc.try_reserve(9011, date, 1).unwrap();
        svc.try_reserve(9011, date, 1).unwrap(); // rejected

        let mut conn = svc.client.get_connection().unwrap();
        let ttl: i64 = redis::cmd("TTL").arg(&key).query(&mut conn).unwrap();
        assert!(ttl > 0, "Key must always have a TTL, got {}", ttl);

        flush_key(&svc, &key);
    }

    #[test]
    fn test_reserve_zero_limit_rejected() {
        let svc = match get_test_redis() {
            Some(s) => s,
            None => { eprintln!("Skipping: Redis not available"); return; }
        };
        let date = "test-zero-limit";
        let key = format!("reply_quota:9012:{}", date);
        flush_key(&svc, &key);

        assert!(!svc.try_reserve(9012, date, 0).unwrap(), "Zero limit should reject immediately");
        assert_eq!(get_counter(&svc, &key), 0, "Counter should not increment for zero limit");

        flush_key(&svc, &key);
    }

    #[test]
    fn test_reserve_negative_limit_rejected() {
        let svc = match get_test_redis() {
            Some(s) => s,
            None => { eprintln!("Skipping: Redis not available"); return; }
        };
        assert!(!svc.try_reserve(9013, "test-neg-limit", -1).unwrap());
        assert!(!svc.try_reserve(9013, "test-neg-limit", -100).unwrap());
    }

    #[test]
    fn test_release_quota_basic() {
        let svc = match get_test_redis() {
            Some(s) => s,
            None => { eprintln!("Skipping: Redis not available"); return; }
        };
        let date = "test-release-basic";
        let key = format!("reply_quota:9014:{}", date);
        flush_key(&svc, &key);

        assert!(svc.try_reserve(9014, date, 2).unwrap());
        assert!(svc.try_reserve(9014, date, 2).unwrap());
        assert_eq!(get_counter(&svc, &key), 2);

        svc.release_quota(9014, date).unwrap();
        assert_eq!(get_counter(&svc, &key), 1, "Counter should decrease after release");

        // Can now reserve again
        assert!(svc.try_reserve(9014, date, 2).unwrap());

        flush_key(&svc, &key);
    }

    #[test]
    fn test_release_quota_wont_go_negative() {
        let svc = match get_test_redis() {
            Some(s) => s,
            None => { eprintln!("Skipping: Redis not available"); return; }
        };
        let date = "test-release-no-negative";
        let key = format!("reply_quota:9015:{}", date);
        flush_key(&svc, &key);

        // Release without any reserves — should not go negative
        svc.release_quota(9015, date).unwrap();
        assert_eq!(get_counter(&svc, &key), 0, "Counter should not go below 0");

        // Double release after one reserve
        svc.try_reserve(9015, date, 5).unwrap();
        svc.release_quota(9015, date).unwrap();
        svc.release_quota(9015, date).unwrap();
        assert_eq!(get_counter(&svc, &key), 0, "Counter should not go below 0 after double release");

        flush_key(&svc, &key);
    }
}
