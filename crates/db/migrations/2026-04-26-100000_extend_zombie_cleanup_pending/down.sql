-- Restore the v1 procedure shape (processing-only, single timeout arg).
--
-- Drops both the v1 single-arg form (in case it lingers) and the v2
-- two-arg form, then recreates v1 verbatim. Idempotent.

DROP FUNCTION IF EXISTS fn_cleanup_zombie_tasks(INT, INT);
DROP FUNCTION IF EXISTS fn_cleanup_zombie_tasks(INT);
DROP FUNCTION IF EXISTS fn_cleanup_zombie_tasks();

CREATE OR REPLACE FUNCTION fn_cleanup_zombie_tasks(p_timeout_hours INT DEFAULT 24)
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

    RETURN QUERY SELECT v_cleaned, v_total_refund;
END;
$$ LANGUAGE plpgsql;
