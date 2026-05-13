DROP FUNCTION IF EXISTS fn_complete_task(INT, TEXT, TEXT);
DROP FUNCTION IF EXISTS fn_complete_task(INT, TEXT);

CREATE OR REPLACE FUNCTION fn_complete_task(
    p_task_id INT,
    p_final_status TEXT DEFAULT 'completed'
)
RETURNS TABLE(
    success BOOLEAN,
    campaign_status TEXT
) AS $$
DECLARE
    v_task RECORD;
    v_campaign_id INT;
    v_campaign_status TEXT;
    v_active_tasks INT;
    v_settle_result RECORD;
BEGIN
    SELECT t.*, c.status as current_campaign_status
    INTO v_task
    FROM gm_crawler_tasks t
    JOIN gm_campaigns c ON t.campaign_id = c.id
    WHERE t.id = p_task_id
    FOR UPDATE OF t;

    IF NOT FOUND THEN
        RETURN QUERY SELECT FALSE, 'NOT_FOUND'::TEXT;
        RETURN;
    END IF;

    v_campaign_id := v_task.campaign_id;
    v_campaign_status := v_task.current_campaign_status;

    UPDATE gm_crawler_tasks
    SET status = p_final_status,
        updated_at = NOW()
    WHERE id = p_task_id;

    SELECT * INTO v_settle_result
    FROM fn_settle_task_consumption(p_task_id);

    IF v_campaign_status = 'STOPPING' THEN
        SELECT COUNT(*) INTO v_active_tasks
        FROM gm_crawler_tasks
        WHERE campaign_id = v_campaign_id
          AND status NOT IN ('completed', 'failed', 'cancelled');

        IF v_active_tasks = 0 THEN
            PERFORM fn_finalize_campaign(v_campaign_id);
            v_campaign_status := 'STOPPED';
        END IF;
    END IF;

    RETURN QUERY SELECT TRUE, v_campaign_status;
END;
$$ LANGUAGE plpgsql;

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

ALTER TABLE gm_crawler_tasks
DROP COLUMN IF EXISTS terminal_reason;
