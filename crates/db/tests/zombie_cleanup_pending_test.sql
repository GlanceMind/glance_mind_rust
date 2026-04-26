-- SQL integration test for fn_cleanup_zombie_tasks v2 (covers `pending`).
--
-- Run with:
--   psql "$DATABASE_URL" -v ON_ERROR_STOP=1 \
--        -f crates/db/tests/zombie_cleanup_pending_test.sql
--
-- Wrapped in a transaction with ROLLBACK at the end so the test leaves
-- no rows behind. Uses real seed FKs (platforms, regions, ai_models)
-- and the first existing user (id=1) instead of inserting a new one,
-- to avoid colliding with id sequences.
--
-- Asserts:
--   * task in `pending` for > p_pending_timeout_minutes is flagged failed
--   * the campaign's pending_consumption is decremented by reserved_amount
--   * a fresh `pending` task (1 minute old) is left untouched

BEGIN;

-- ============================================================================
-- Setup
-- ============================================================================

DO $$
DECLARE
    v_user_id INT;
BEGIN
    SELECT id INTO v_user_id FROM gm_users ORDER BY id LIMIT 1;
    IF v_user_id IS NULL THEN
        RAISE EXCEPTION 'no gm_users row available for test setup';
    END IF;
    PERFORM set_config('test.user_id', v_user_id::TEXT, true);
END $$;

INSERT INTO gm_campaigns (
    id, user_id, name, status,
    platform_id, region_id, ai_model_id,
    product_prompt, schedule_type,
    max_scan_count, budget_cap,
    pending_consumption, actual_consumption, total_scanned,
    auto_like, auto_follow, auto_dm,
    auto_reply_post, auto_reply_comments,
    is_frozen, created_at
)
VALUES (
    99000001, current_setting('test.user_id')::INT, 'zombie-test', 'ACTIVE',
    2, 1, 1,
    'p', 'ONCE',
    50, 1000,
    250, 0, 0,
    FALSE, FALSE, FALSE,
    FALSE, TRUE,
    FALSE, NOW()
)
ON CONFLICT (id) DO UPDATE SET
    pending_consumption = EXCLUDED.pending_consumption,
    actual_consumption = 0,
    status = 'ACTIVE',
    completed_reason = NULL;

-- Stale pending task: created 1 hour ago, reserved 200 of the 250.
INSERT INTO gm_crawler_tasks (
    id, campaign_id, max_count, process_count, status,
    created_at, search_offset, search_limit, reserved_amount, actual_consumption
)
VALUES (
    99000099, 99000001, 50, 0, 'pending',
    NOW() - INTERVAL '1 hour', 0, 10, 200, 0
)
ON CONFLICT (id) DO UPDATE SET
    status = 'pending',
    created_at = NOW() - INTERVAL '1 hour',
    reserved_amount = 200,
    actual_consumption = 0,
    settled_at = NULL;

-- Fresh pending task: 1 minute old, must be untouched by default 30-min
-- threshold; reserved 50 of the 250.
INSERT INTO gm_crawler_tasks (
    id, campaign_id, max_count, process_count, status,
    created_at, search_offset, search_limit, reserved_amount, actual_consumption
)
VALUES (
    99000100, 99000001, 50, 0, 'pending',
    NOW() - INTERVAL '1 minute', 0, 10, 50, 0
)
ON CONFLICT (id) DO UPDATE SET
    status = 'pending',
    created_at = NOW() - INTERVAL '1 minute',
    reserved_amount = 50,
    actual_consumption = 0,
    settled_at = NULL;

-- ============================================================================
-- Act
-- ============================================================================

SELECT * FROM fn_cleanup_zombie_tasks();

-- ============================================================================
-- Assert
-- ============================================================================

DO $$
DECLARE
    v_stale_status TEXT;
    v_stale_settled_at TIMESTAMPTZ;
    v_fresh_status TEXT;
    v_pending NUMERIC;
BEGIN
    SELECT status, settled_at INTO v_stale_status, v_stale_settled_at
    FROM gm_crawler_tasks WHERE id = 99000099;
    IF v_stale_status <> 'failed' THEN
        RAISE EXCEPTION 'expected stale task 99000099 status=failed, got %', v_stale_status;
    END IF;
    IF v_stale_settled_at IS NULL THEN
        RAISE EXCEPTION 'expected stale task 99000099 to have settled_at set after cleanup';
    END IF;

    SELECT status INTO v_fresh_status
    FROM gm_crawler_tasks WHERE id = 99000100;
    IF v_fresh_status <> 'pending' THEN
        RAISE EXCEPTION 'expected fresh task 99000100 status=pending (untouched), got %', v_fresh_status;
    END IF;

    -- Campaign started with pending_consumption=250, stale task reserved=200.
    -- After cleanup: 250 - 200 = 50 remaining (still owned by fresh task).
    SELECT pending_consumption INTO v_pending
    FROM gm_campaigns WHERE id = 99000001;
    IF v_pending <> 50 THEN
        RAISE EXCEPTION 'expected campaign 99000001 pending_consumption=50 after stale cleanup, got %', v_pending;
    END IF;

    RAISE NOTICE 'zombie_cleanup_pending_test: PASS (stale task settled, fresh task untouched, refund=200)';
END $$;

ROLLBACK;
