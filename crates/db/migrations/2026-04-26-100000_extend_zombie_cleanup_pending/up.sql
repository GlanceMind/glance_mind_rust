-- Extend fn_cleanup_zombie_tasks to also reap stale `pending` tasks.
--
-- Background (campaign 121, task 3972 / 2026-04-26):
--   When agent-rs fails to deserialize a task pulled from
--   crawler:task_queue (e.g. wire-format mismatch), the task stays in
--   `pending` forever and pending_consumption is never released. The
--   original v1 procedure only handled `processing`, so the budget was
--   frozen indefinitely.
--
-- v2 changes:
--   1. Adds optional p_pending_timeout_minutes (default 30 minutes) so
--      tasks stuck in `pending` past that threshold are flagged failed
--      and their reserved_amount is returned via fn_settle_task_consumption
--      (called transitively by fn_complete_task).
--   2. Drops the v1 single-arg overload first to avoid PostgreSQL function
--      overload ambiguity when callers invoke `fn_cleanup_zombie_tasks()`
--      with no arguments.

DROP FUNCTION IF EXISTS fn_cleanup_zombie_tasks(INT);
DROP FUNCTION IF EXISTS fn_cleanup_zombie_tasks();

CREATE OR REPLACE FUNCTION fn_cleanup_zombie_tasks(
    p_timeout_hours INT DEFAULT 24,
    p_pending_timeout_minutes INT DEFAULT 30
)
RETURNS TABLE(
    cleaned_count INT,
    total_refunded NUMERIC
) AS $$
DECLARE
    v_zombie_task RECORD;
    v_cleaned INT := 0;
    v_total_refund NUMERIC := 0;
    v_complete_result RECORD;
BEGIN
    -- A. Stale `processing` tasks (original v1 behavior).
    FOR v_zombie_task IN
        SELECT id
        FROM gm_crawler_tasks
        WHERE status = 'processing'
          AND updated_at < NOW() - (p_timeout_hours || ' hours')::INTERVAL
        FOR UPDATE SKIP LOCKED
    LOOP
        SELECT * INTO v_complete_result
        FROM fn_complete_task(v_zombie_task.id, 'failed');

        IF v_complete_result.success THEN
            v_cleaned := v_cleaned + 1;
        END IF;
    END LOOP;

    -- B. Stale `pending` tasks (new in v2). These are typically tasks the
    -- agent BRPOP'd but never transitioned to `processing` (e.g. dropped
    -- on deserialization failure, agent crashed mid-flight, etc).
    FOR v_zombie_task IN
        SELECT id
        FROM gm_crawler_tasks
        WHERE status = 'pending'
          AND created_at < NOW() - (p_pending_timeout_minutes || ' minutes')::INTERVAL
        FOR UPDATE SKIP LOCKED
    LOOP
        SELECT * INTO v_complete_result
        FROM fn_complete_task(v_zombie_task.id, 'failed');

        IF v_complete_result.success THEN
            v_cleaned := v_cleaned + 1;
        END IF;
    END LOOP;

    RETURN QUERY SELECT v_cleaned, v_total_refund;
END;
$$ LANGUAGE plpgsql;
