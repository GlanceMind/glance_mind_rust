-- ============================================================================
-- Video Billing System: Task-Level Settlement + Exact-Once Procedures
-- ============================================================================
-- Adds:
--   1. gm_aipub_task_billings — per-task billing snapshot
--   2. gm_aipub_tasks columns — video_stage_started_at, video_ai_task_id
--   3. gm_wallet_transactions columns — reference_sub_type/id, biz_key
--   4. fn_allocate_aipub_task_billing — snapshot pricing at task creation
--   5. fn_complete_video_task_success — exact-once success settlement
--   6. fn_fail_video_task — exact-once failure + plan finalization
--   7. fn_claim_timed_out_video_tasks — timeout candidate leasing
-- ============================================================================


-- ============================================================================
-- PART 1: Schema Changes
-- ============================================================================

-- 1.1 Task-level billing snapshot table
CREATE TABLE IF NOT EXISTS gm_aipub_task_billings (
    id                    SERIAL PRIMARY KEY,
    plan_id               INT NOT NULL REFERENCES gm_aipub_plans(id),
    aipub_task_id         INT NOT NULL REFERENCES gm_aipub_tasks(id),
    video_ai_task_id      INT,
    chat_model_id         INT,
    video_model_id        INT,
    reserved_chat_amount  NUMERIC(10,2) NOT NULL DEFAULT 0,
    reserved_video_amount NUMERIC(10,2) NOT NULL DEFAULT 0,
    reserved_total_amount NUMERIC(10,2) NOT NULL DEFAULT 0,
    settled_chat_amount   NUMERIC(10,2) NOT NULL DEFAULT 0,
    settled_video_amount  NUMERIC(10,2) NOT NULL DEFAULT 0,
    settled_total_amount  NUMERIC(10,2) NOT NULL DEFAULT 0,
    billing_status        VARCHAR(20) NOT NULL DEFAULT 'allocated'
                          CHECK (billing_status IN ('allocated','settled','failed_refundable','finalized')),
    pricing_snapshot      JSONB,
    created_at            TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at            TIMESTAMPTZ,
    finalized_at          TIMESTAMPTZ
);

CREATE INDEX IF NOT EXISTS idx_task_billings_plan ON gm_aipub_task_billings(plan_id);
CREATE INDEX IF NOT EXISTS idx_task_billings_task ON gm_aipub_task_billings(aipub_task_id);
CREATE UNIQUE INDEX IF NOT EXISTS idx_task_billings_unique_task
    ON gm_aipub_task_billings(aipub_task_id) WHERE billing_status != 'finalized';

COMMENT ON TABLE gm_aipub_task_billings IS 'Per-task billing snapshot: reserved vs settled amounts';

-- 1.2 Add video-stage tracking columns to aipub_tasks
ALTER TABLE gm_aipub_tasks
    ADD COLUMN IF NOT EXISTS video_stage_started_at TIMESTAMPTZ,
    ADD COLUMN IF NOT EXISTS video_ai_task_id INT;

COMMENT ON COLUMN gm_aipub_tasks.video_stage_started_at IS 'Timestamp when task entered video_pending (timeout start)';
COMMENT ON COLUMN gm_aipub_tasks.video_ai_task_id IS 'FK to the video-gen ai_task (explicit, avoids JSON parsing)';

-- 1.3 Add task-level tracing and idempotency to wallet transactions
ALTER TABLE gm_wallet_transactions
    ADD COLUMN IF NOT EXISTS reference_sub_type VARCHAR(50),
    ADD COLUMN IF NOT EXISTS reference_sub_id INT,
    ADD COLUMN IF NOT EXISTS biz_key VARCHAR(128);

CREATE UNIQUE INDEX IF NOT EXISTS idx_wallet_txn_biz_key
    ON gm_wallet_transactions(biz_key) WHERE biz_key IS NOT NULL;

COMMENT ON COLUMN gm_wallet_transactions.reference_sub_type IS 'Sub-entity type: aipub_task_billing, etc.';
COMMENT ON COLUMN gm_wallet_transactions.reference_sub_id IS 'Sub-entity ID for fine-grained tracing';
COMMENT ON COLUMN gm_wallet_transactions.biz_key IS 'Unique business key for idempotent ledger entries';


-- ============================================================================
-- PART 2: fn_allocate_aipub_task_billing
-- ============================================================================
-- Called by Scheduler when creating aipub_task for a video plan.
-- Snapshots the per-task reserved amounts from current pricing rules.
-- Idempotent: returns existing billing if already allocated.

CREATE OR REPLACE FUNCTION fn_allocate_aipub_task_billing(
    p_plan_id         INT,
    p_aipub_task_id   INT,
    p_chat_model_id   INT DEFAULT NULL,
    p_video_model_id  INT DEFAULT NULL
) RETURNS INT AS $$
DECLARE
    v_existing_id  INT;
    v_chat_base    NUMERIC;
    v_video_base   NUMERIC;
    v_chat_mult    NUMERIC := 1.0;
    v_video_mult   NUMERIC := 1.0;
    v_chat_amount  NUMERIC;
    v_video_amount NUMERIC;
    v_total        NUMERIC;
    v_new_id       INT;
BEGIN
    -- Idempotent: check if already allocated
    SELECT id INTO v_existing_id
    FROM gm_aipub_task_billings
    WHERE aipub_task_id = p_aipub_task_id
      AND billing_status != 'finalized'
    LIMIT 1;

    IF v_existing_id IS NOT NULL THEN
        RETURN v_existing_id;
    END IF;

    -- Query base prices
    SELECT COALESCE(cost_points, 1.0) INTO v_chat_base
    FROM gm_pricing_rules
    WHERE action_type = 'AI_ANALYZE' AND platform_id IS NULL
    LIMIT 1;
    v_chat_base := COALESCE(v_chat_base, 1.0);

    SELECT COALESCE(cost_points, 0) INTO v_video_base
    FROM gm_pricing_rules
    WHERE action_type = 'VIDEO_GENERATE' AND platform_id IS NULL
    LIMIT 1;
    IF v_video_base IS NULL OR v_video_base = 0 THEN
        SELECT COALESCE(cost_points, 0) INTO v_video_base
        FROM gm_pricing_rules
        WHERE action_type = 'VIDEO' AND platform_id IS NULL
        LIMIT 1;
    END IF;
    v_video_base := COALESCE(v_video_base, 0);

    -- Query model multipliers
    IF p_chat_model_id IS NOT NULL THEN
        SELECT COALESCE(cost_multiplier, 1.0) INTO v_chat_mult
        FROM gm_ai_models WHERE id = p_chat_model_id AND is_active = true;
    END IF;
    v_chat_mult := COALESCE(v_chat_mult, 1.0);

    IF p_video_model_id IS NOT NULL THEN
        SELECT COALESCE(cost_multiplier, 1.0) INTO v_video_mult
        FROM gm_ai_models WHERE id = p_video_model_id AND is_active = true;
    END IF;
    v_video_mult := COALESCE(v_video_mult, 1.0);

    -- Calculate amounts
    v_chat_amount  := 1 * v_chat_base * v_chat_mult;
    v_video_amount := 1 * v_video_base * v_video_mult;
    v_total        := v_chat_amount + v_video_amount;

    -- Insert billing snapshot (ON CONFLICT for concurrent allocations)
    INSERT INTO gm_aipub_task_billings (
        plan_id, aipub_task_id, chat_model_id, video_model_id,
        reserved_chat_amount, reserved_video_amount, reserved_total_amount,
        billing_status, pricing_snapshot, created_at
    ) VALUES (
        p_plan_id, p_aipub_task_id, p_chat_model_id, p_video_model_id,
        v_chat_amount, v_video_amount, v_total,
        'allocated',
        jsonb_build_object(
            'chat_base', v_chat_base, 'chat_mult', v_chat_mult,
            'video_base', v_video_base, 'video_mult', v_video_mult
        ),
        NOW()
    )
    ON CONFLICT (aipub_task_id) WHERE billing_status != 'finalized'
    DO UPDATE SET updated_at = NOW()
    RETURNING id INTO v_new_id;

    RETURN v_new_id;
END;
$$ LANGUAGE plpgsql;


-- ============================================================================
-- PART 3: fn_complete_video_task_success
-- ============================================================================
-- Exact-once video success: lock + idempotent check + settle chat+video +
-- update statuses + write ledger + push plan status.
-- Returns: 'already_done' | 'settled' | 'plan_finalized'

CREATE OR REPLACE FUNCTION fn_complete_video_task_success(
    p_video_ai_task_id  INT,
    p_aipub_task_id     INT,
    p_video_url         TEXT
) RETURNS VARCHAR AS $$
DECLARE
    v_ai_task     RECORD;
    v_billing     RECORD;
    v_plan        RECORD;
    v_biz_key     VARCHAR;
    v_settle_amt  NUMERIC;
    v_all_terminal BOOLEAN;
    v_all_failed   BOOLEAN;
BEGIN
    -- ① Lock ai_task
    SELECT * INTO v_ai_task
    FROM gm_aipub_ai_tasks WHERE id = p_video_ai_task_id FOR UPDATE;

    IF v_ai_task IS NULL THEN
        RETURN 'not_found';
    END IF;

    -- ② Idempotent: already completed
    IF v_ai_task.status = 'completed' THEN
        RETURN 'already_done';
    END IF;

    -- ③ Lock billing row
    SELECT * INTO v_billing
    FROM gm_aipub_task_billings
    WHERE aipub_task_id = p_aipub_task_id
      AND billing_status = 'allocated'
    FOR UPDATE;

    -- If no billing row or already settled, check biz_key idempotency
    IF v_billing IS NULL THEN
        -- Maybe already settled
        IF EXISTS (
            SELECT 1 FROM gm_aipub_task_billings
            WHERE aipub_task_id = p_aipub_task_id
              AND billing_status IN ('settled', 'finalized')
        ) THEN
            RETURN 'already_done';
        END IF;
        RETURN 'no_billing';
    END IF;

    -- ④ Build biz_key for ledger idempotency
    v_biz_key := format('aipub_plan:%s:task:%s:settle', v_billing.plan_id, p_aipub_task_id);

    -- Check if ledger entry already exists
    IF EXISTS (SELECT 1 FROM gm_wallet_transactions WHERE biz_key = v_biz_key) THEN
        RETURN 'already_done';
    END IF;

    v_settle_amt := v_billing.reserved_total_amount;

    -- ⑤ Update ai_task -> completed
    UPDATE gm_aipub_ai_tasks
    SET status = 'completed',
        progress = 100,
        result = jsonb_build_object('video_url', p_video_url, 'version', '2.0'),
        updated_at = NOW(),
        completed_at = NOW()
    WHERE id = p_video_ai_task_id;

    -- ⑥ Update aipub_task -> ready (with video_url in content)
    UPDATE gm_aipub_tasks
    SET status = 'ready',
        content = content || jsonb_build_object('video_url', p_video_url),
        updated_at = NOW()
    WHERE id = p_aipub_task_id
      AND status NOT IN ('ready', 'completed');

    -- ⑦ Settle billing: chat + video
    UPDATE gm_aipub_task_billings
    SET settled_chat_amount  = reserved_chat_amount,
        settled_video_amount = reserved_video_amount,
        settled_total_amount = reserved_total_amount,
        billing_status       = 'settled',
        updated_at           = NOW()
    WHERE id = v_billing.id;

    -- ⑧ Consume from frozen wallet (reuse existing fn_consume_from_frozen logic inline)
    SELECT * INTO v_plan FROM gm_aipub_plans WHERE id = v_billing.plan_id FOR UPDATE;

    IF FOUND AND v_plan.billing_status = 'frozen' THEN
        -- Safety cap
        IF v_plan.consumed_cost + v_settle_amt > v_plan.frozen_cost THEN
            v_settle_amt := v_plan.frozen_cost - v_plan.consumed_cost;
        END IF;

        IF v_settle_amt > 0 THEN
            -- Deduct from frozen
            PERFORM 1 FROM gm_user_wallets WHERE user_id = v_plan.user_id FOR UPDATE;

            UPDATE gm_user_wallets
            SET frozen_points = frozen_points - v_settle_amt,
                updated_at = NOW()
            WHERE user_id = v_plan.user_id;

            -- Update plan consumed
            UPDATE gm_aipub_plans
            SET consumed_cost = consumed_cost + v_settle_amt,
                updated_at = NOW()
            WHERE id = v_billing.plan_id;

            -- Write SETTLE ledger entry
            INSERT INTO gm_wallet_transactions
                (user_id, amount, type, reference_id, reference_type,
                 reference_sub_type, reference_sub_id, biz_key, description, created_at)
            VALUES (
                v_plan.user_id, -v_settle_amt, 'SETTLE',
                v_billing.plan_id, 'aipub_plan',
                'aipub_task_billing', v_billing.id, v_biz_key,
                format('Video task settle: chat=%s + video=%s = %s pt (task=%s)',
                    v_billing.reserved_chat_amount, v_billing.reserved_video_amount,
                    v_settle_amt, p_aipub_task_id),
                NOW()
            );
        END IF;
    END IF;

    -- ⑨ Check if plan should be finalized
    SELECT NOT EXISTS (
        SELECT 1 FROM gm_aipub_tasks
        WHERE plan_id = v_billing.plan_id
          AND status NOT IN ('ready', 'completed', 'failed')
    ) AND NOT EXISTS (
        SELECT 1 FROM gm_aipub_ai_tasks
        WHERE plan_id = v_billing.plan_id
          AND status = 'processing'
    ) INTO v_all_terminal;

    IF v_all_terminal THEN
        SELECT NOT EXISTS (
            SELECT 1 FROM gm_aipub_tasks
            WHERE plan_id = v_billing.plan_id
              AND status IN ('ready', 'completed')
        ) INTO v_all_failed;

        IF v_all_failed THEN
            PERFORM fn_finalize_plan(v_billing.plan_id, 'failed');
            RETURN 'plan_finalized';
        ELSE
            UPDATE gm_aipub_plans
            SET status = 'ready', updated_at = NOW()
            WHERE id = v_billing.plan_id
              AND status NOT IN ('ready', 'completed', 'failed');
        END IF;
    END IF;

    RETURN 'settled';
END;
$$ LANGUAGE plpgsql;


-- ============================================================================
-- PART 4: fn_fail_video_task
-- ============================================================================
-- Exact-once video failure: lock + idempotent check + mark failed +
-- plan terminal check + finalize if all done.
-- Returns: 'already_done' | 'failed' | 'plan_finalized' | 'plan_ready'

CREATE OR REPLACE FUNCTION fn_fail_video_task(
    p_ai_task_id    INT,
    p_aipub_task_id INT,
    p_error_msg     TEXT
) RETURNS VARCHAR AS $$
DECLARE
    v_ai_task       RECORD;
    v_plan_id       INT;
    v_all_ai_done   BOOLEAN;
    v_all_task_done BOOLEAN;
    v_all_failed    BOOLEAN;
BEGIN
    -- ① Lock ai_task
    SELECT * INTO v_ai_task
    FROM gm_aipub_ai_tasks WHERE id = p_ai_task_id FOR UPDATE;

    IF v_ai_task IS NULL THEN
        RETURN 'not_found';
    END IF;

    -- ② Idempotent: never overwrite a completed task with failed
    IF v_ai_task.status = 'completed' THEN
        RETURN 'already_done';
    END IF;

    -- Already failed + not retryable → skip
    IF v_ai_task.status = 'failed' AND v_ai_task.retry_count >= 5 THEN
        RETURN 'already_done';
    END IF;

    v_plan_id := v_ai_task.plan_id;

    -- ③ Mark ai_task failed (retry_count=5 prevents re-pickup)
    UPDATE gm_aipub_ai_tasks
    SET status = 'failed',
        retry_count = 5,
        error_message = p_error_msg,
        updated_at = NOW()
    WHERE id = p_ai_task_id;

    -- ④ Mark aipub_task failed (only if not already terminal)
    IF p_aipub_task_id > 0 THEN
        UPDATE gm_aipub_tasks
        SET status = 'failed',
            error_message = p_error_msg,
            updated_at = NOW()
        WHERE id = p_aipub_task_id
          AND status NOT IN ('ready', 'completed');
    END IF;

    -- ⑤ Mark billing as failed_refundable
    UPDATE gm_aipub_task_billings
    SET billing_status = 'failed_refundable',
        updated_at = NOW()
    WHERE aipub_task_id = p_aipub_task_id
      AND billing_status = 'allocated';

    -- ⑥ Check plan-level terminal state
    SELECT NOT EXISTS (
        SELECT 1 FROM gm_aipub_ai_tasks
        WHERE plan_id = v_plan_id AND status = 'processing'
    ) INTO v_all_ai_done;

    IF NOT v_all_ai_done THEN
        RETURN 'failed';
    END IF;

    SELECT NOT EXISTS (
        SELECT 1 FROM gm_aipub_tasks
        WHERE plan_id = v_plan_id
          AND status NOT IN ('ready', 'completed', 'failed')
    ) INTO v_all_task_done;

    IF NOT v_all_task_done THEN
        RETURN 'failed';
    END IF;

    -- ⑦ All terminal: determine plan outcome
    SELECT NOT EXISTS (
        SELECT 1 FROM gm_aipub_tasks
        WHERE plan_id = v_plan_id AND status IN ('ready', 'completed')
    ) INTO v_all_failed;

    IF v_all_failed THEN
        PERFORM fn_finalize_plan(v_plan_id, 'failed');

        -- Mark all failed_refundable billings as finalized
        UPDATE gm_aipub_task_billings
        SET billing_status = 'finalized',
            finalized_at = NOW(),
            updated_at = NOW()
        WHERE plan_id = v_plan_id
          AND billing_status = 'failed_refundable';

        RETURN 'plan_finalized';
    ELSE
        UPDATE gm_aipub_plans
        SET status = 'ready', updated_at = NOW()
        WHERE id = v_plan_id
          AND status NOT IN ('ready', 'completed', 'failed');
        RETURN 'plan_ready';
    END IF;
END;
$$ LANGUAGE plpgsql;


-- ============================================================================
-- PART 5: fn_claim_timed_out_video_tasks
-- ============================================================================
-- Claims timeout candidate tasks for a scheduler instance to do final poll.
-- Uses FOR UPDATE SKIP LOCKED to avoid contention between schedulers.
-- Returns a table of candidates with provider info for final polling.

CREATE OR REPLACE FUNCTION fn_claim_timed_out_video_tasks(
    p_timeout_minutes  INT DEFAULT 30,
    p_batch_size       INT DEFAULT 20
) RETURNS TABLE (
    aipub_task_id      INT,
    video_ai_task_id   INT,
    plan_id            INT,
    external_service   VARCHAR,
    external_job_id    VARCHAR,
    jimeng_req_key     VARCHAR
) AS $$
DECLARE
    v_timeout_at TIMESTAMPTZ;
    v_task       RECORD;
    v_ai_task    RECORD;
BEGIN
    v_timeout_at := NOW() - (p_timeout_minutes || ' minutes')::INTERVAL;

    -- Find timed-out aipub_tasks
    FOR v_task IN
        SELECT t.id AS task_id,
               t.video_ai_task_id AS vai_task_id,
               t.plan_id AS t_plan_id
        FROM gm_aipub_tasks t
        WHERE t.status IN ('video_pending', 'video_processing')
          AND t.video_stage_started_at IS NOT NULL
          AND t.video_stage_started_at < v_timeout_at
        ORDER BY t.video_stage_started_at ASC
        LIMIT p_batch_size
        FOR UPDATE OF t SKIP LOCKED
    LOOP
        -- Get video ai_task info for provider final poll
        IF v_task.vai_task_id IS NOT NULL THEN
            SELECT * INTO v_ai_task
            FROM gm_aipub_ai_tasks
            WHERE id = v_task.vai_task_id;

            aipub_task_id    := v_task.task_id;
            video_ai_task_id := v_task.vai_task_id;
            plan_id          := v_task.t_plan_id;
            external_service := v_ai_task.external_service;
            external_job_id  := v_ai_task.external_job_id;
            jimeng_req_key   := v_ai_task.input->>'jimeng_req_key';
            RETURN NEXT;
        ELSE
            -- No video ai_task yet (stuck in video_pending before submission)
            aipub_task_id    := v_task.task_id;
            video_ai_task_id := NULL;
            plan_id          := v_task.t_plan_id;
            external_service := NULL;
            external_job_id  := NULL;
            jimeng_req_key   := NULL;
            RETURN NEXT;
        END IF;
    END LOOP;
END;
$$ LANGUAGE plpgsql;
