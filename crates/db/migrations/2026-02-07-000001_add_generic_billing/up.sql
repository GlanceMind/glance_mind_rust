-- ============================================================================
-- Generic Billing System for AIPub Plans
-- ============================================================================
-- Adds incremental billing model:
--   fn_freeze_budget     - Freeze estimated cost at plan creation
--   fn_consume_from_frozen - Deduct per sub-task completion (incremental)
--   fn_finalize_plan     - Refund remaining frozen at plan terminal state
-- ============================================================================

-- ============================================================================
-- PART 1: Schema Changes
-- ============================================================================

-- 1.1 Add reference_type to wallet transactions for business traceability
ALTER TABLE gm_wallet_transactions
    ADD COLUMN IF NOT EXISTS reference_type VARCHAR(50);

COMMENT ON COLUMN gm_wallet_transactions.reference_type
    IS 'Business type: campaign, aipub_plan, etc.';

-- 1.2 Add billing fields to aipub_plans (all plan_types)
ALTER TABLE gm_aipub_plans
    ADD COLUMN IF NOT EXISTS billing_status VARCHAR(20) DEFAULT 'none' NOT NULL,
    ADD COLUMN IF NOT EXISTS frozen_cost NUMERIC(10,2) DEFAULT 0.00 NOT NULL,
    ADD COLUMN IF NOT EXISTS consumed_cost NUMERIC(10,2) DEFAULT 0.00 NOT NULL,
    ADD COLUMN IF NOT EXISTS frozen_at TIMESTAMPTZ;

ALTER TABLE gm_aipub_plans
    ADD CONSTRAINT aipub_plans_valid_billing_status
    CHECK (billing_status IN ('none', 'frozen', 'settled'));

COMMENT ON COLUMN gm_aipub_plans.billing_status IS 'Billing state: none=no billing, frozen=budget frozen, settled=finalized';
COMMENT ON COLUMN gm_aipub_plans.frozen_cost IS 'Total frozen amount (set at creation, immutable)';
COMMENT ON COLUMN gm_aipub_plans.consumed_cost IS 'Consumed amount (incremented per sub-task completion)';
COMMENT ON COLUMN gm_aipub_plans.frozen_at IS 'Freeze timestamp for reconciliation timeout detection';

-- 1.3 Fix NULL platform_id uniqueness (PostgreSQL UNIQUE constraint doesn't treat NULL=NULL as conflict)
CREATE UNIQUE INDEX IF NOT EXISTS idx_pricing_rules_action_null_platform
    ON gm_pricing_rules (action_type) WHERE platform_id IS NULL;

-- Clean up any duplicate NULL platform_id rows (keep newest)
DELETE FROM gm_pricing_rules a USING gm_pricing_rules b
WHERE a.platform_id IS NULL AND b.platform_id IS NULL
  AND a.action_type = b.action_type
  AND a.id < b.id;

-- 1.4 Add IMAGE pricing rule (CHAT reuses existing AI_ANALYZE = 1.00pt)
INSERT INTO gm_pricing_rules (action_type, platform_id, cost_points)
VALUES ('IMAGE', NULL, 5.00)
ON CONFLICT (action_type) WHERE platform_id IS NULL DO NOTHING;


-- ============================================================================
-- PART 2: fn_freeze_budget - Freeze estimated cost at plan creation
-- ============================================================================
-- Called by API create_plan. Calculates cost from pricing rules + model
-- multipliers, validates balance, freezes wallet, updates plan.
-- Idempotent: returns frozen_cost if already frozen/settled.

CREATE OR REPLACE FUNCTION fn_freeze_budget(
    p_user_id        INT,
    p_chat_count     INT DEFAULT 0,
    p_image_count    INT DEFAULT 0,
    p_video_count    INT DEFAULT 0,
    p_chat_model_id  INT DEFAULT NULL,
    p_image_model_id INT DEFAULT NULL,
    p_video_model_id INT DEFAULT NULL,
    p_ref_type       VARCHAR DEFAULT NULL,
    p_ref_id         INT DEFAULT NULL
) RETURNS NUMERIC AS $$
DECLARE
    v_chat_base    NUMERIC := 0;
    v_image_base   NUMERIC := 0;
    v_video_base   NUMERIC := 0;
    v_chat_mult    NUMERIC := 1.0;
    v_image_mult   NUMERIC := 1.0;
    v_video_mult   NUMERIC := 1.0;
    v_total        NUMERIC;
    v_available    NUMERIC;
    v_billing_status VARCHAR;
BEGIN
    -- ① Idempotent check: if aipub_plan, check billing_status
    IF p_ref_type = 'aipub_plan' AND p_ref_id IS NOT NULL THEN
        SELECT billing_status INTO v_billing_status
        FROM gm_aipub_plans WHERE id = p_ref_id FOR UPDATE;

        IF v_billing_status IS NULL THEN
            RAISE EXCEPTION 'Plan not found: %', p_ref_id;
        END IF;

        IF v_billing_status = 'frozen' OR v_billing_status = 'settled' THEN
            -- Already frozen/settled, idempotent return
            RETURN (SELECT frozen_cost FROM gm_aipub_plans WHERE id = p_ref_id);
        END IF;

        IF v_billing_status != 'none' THEN
            RAISE EXCEPTION 'Invalid billing_status: %', v_billing_status;
        END IF;
    END IF;

    -- ② Query base prices from pricing rules
    IF p_chat_count > 0 THEN
        -- Chat reuses AI_ANALYZE pricing
        SELECT COALESCE(cost_points, 0) INTO v_chat_base
        FROM gm_pricing_rules
        WHERE action_type = 'AI_ANALYZE' AND platform_id IS NULL
        LIMIT 1;

        IF v_chat_base IS NULL OR v_chat_base = 0 THEN
            -- Fallback: try any AI_ANALYZE rule
            SELECT COALESCE(MIN(cost_points), 1.0) INTO v_chat_base
            FROM gm_pricing_rules
            WHERE action_type = 'AI_ANALYZE';
        END IF;
        v_chat_base := COALESCE(v_chat_base, 1.0);

        IF p_chat_model_id IS NOT NULL THEN
            SELECT COALESCE(cost_multiplier, 1.0) INTO v_chat_mult
            FROM gm_ai_models WHERE id = p_chat_model_id AND is_active = true;
        END IF;
        v_chat_mult := COALESCE(v_chat_mult, 1.0);
    END IF;

    IF p_image_count > 0 THEN
        SELECT COALESCE(cost_points, 0) INTO v_image_base
        FROM gm_pricing_rules
        WHERE action_type = 'IMAGE' AND platform_id IS NULL
        LIMIT 1;

        IF p_image_model_id IS NOT NULL THEN
            SELECT COALESCE(cost_multiplier, 1.0) INTO v_image_mult
            FROM gm_ai_models WHERE id = p_image_model_id AND is_active = true;
        END IF;
    END IF;

    IF p_video_count > 0 THEN
        SELECT COALESCE(cost_points, 0) INTO v_video_base
        FROM gm_pricing_rules
        WHERE action_type = 'VIDEO' AND platform_id IS NULL
        LIMIT 1;

        -- Fallback to VIDEO_GENERATE (legacy)
        IF v_video_base IS NULL OR v_video_base = 0 THEN
            SELECT COALESCE(cost_points, 0) INTO v_video_base
            FROM gm_pricing_rules
            WHERE action_type = 'VIDEO_GENERATE' AND platform_id IS NULL
            LIMIT 1;
        END IF;

        -- Final fallback: default to 0
        v_video_base := COALESCE(v_video_base, 0);

        IF p_video_model_id IS NOT NULL THEN
            SELECT COALESCE(cost_multiplier, 1.0) INTO v_video_mult
            FROM gm_ai_models WHERE id = p_video_model_id AND is_active = true;
        END IF;
        v_video_mult := COALESCE(v_video_mult, 1.0);
    END IF;

    -- ③ Calculate total cost
    v_total := (p_chat_count * v_chat_base * v_chat_mult)
             + (p_image_count * v_image_base * v_image_mult)
             + (p_video_count * v_video_base * v_video_mult);

    IF v_total <= 0 THEN
        RAISE EXCEPTION 'Freeze amount must be positive, got: %', v_total;
    END IF;

    -- ④ Row-level lock + balance validation
    SELECT balance_points - frozen_points INTO v_available
    FROM gm_user_wallets WHERE user_id = p_user_id FOR UPDATE;

    IF v_available IS NULL THEN
        RAISE EXCEPTION 'Wallet not found for user_id=%', p_user_id;
    END IF;

    IF v_available < v_total THEN
        RAISE EXCEPTION 'Insufficient balance: available=%, required=%', v_available, v_total;
    END IF;

    -- ⑤ Atomic wallet update
    UPDATE gm_user_wallets
    SET balance_points = balance_points - v_total,
        frozen_points  = frozen_points + v_total,
        updated_at     = NOW()
    WHERE user_id = p_user_id;

    -- ⑥ Atomic plan update (if aipub_plan)
    IF p_ref_type = 'aipub_plan' AND p_ref_id IS NOT NULL THEN
        UPDATE gm_aipub_plans
        SET billing_status = 'frozen',
            frozen_cost    = v_total,
            consumed_cost  = 0,
            frozen_at      = NOW(),
            updated_at     = NOW()
        WHERE id = p_ref_id;
    END IF;

    -- ⑦ Transaction record
    INSERT INTO gm_wallet_transactions
        (user_id, amount, type, reference_id, reference_type, description, created_at)
    VALUES (
        p_user_id, -v_total, 'FREEZE', p_ref_id, p_ref_type,
        format('Budget frozen: %s pt (chat: %s*%s*%s=%s, image: %s*%s*%s=%s, video: %s*%s*%s=%s)',
            v_total,
            p_chat_count, v_chat_base, v_chat_mult, p_chat_count * v_chat_base * v_chat_mult,
            p_image_count, v_image_base, v_image_mult, p_image_count * v_image_base * v_image_mult,
            p_video_count, v_video_base, v_video_mult, p_video_count * v_video_base * v_video_mult),
        NOW()
    );

    RETURN v_total;
END;
$$ LANGUAGE plpgsql;


-- ============================================================================
-- PART 3: fn_consume_from_frozen - Incremental deduction per sub-task
-- ============================================================================
-- Called by Scheduler after each successful sub-task (chat/image/video).
-- Deducts from frozen (money disappears, not returned to balance).
-- Idempotent: skips if plan not in 'frozen' state.

CREATE OR REPLACE FUNCTION fn_consume_from_frozen(
    p_user_id     INT,
    p_action_type VARCHAR,      -- 'AI_ANALYZE', 'IMAGE', 'VIDEO'/'VIDEO_GENERATE'
    p_model_id    INT DEFAULT NULL,
    p_count       INT DEFAULT 1,
    p_ref_type    VARCHAR DEFAULT NULL,
    p_ref_id      INT DEFAULT NULL
) RETURNS NUMERIC AS $$
DECLARE
    v_base_cost   NUMERIC;
    v_multiplier  NUMERIC := 1.0;
    v_amount      NUMERIC;
    v_plan        RECORD;
BEGIN
    -- ① Safety check: plan must be in frozen state
    IF p_ref_type = 'aipub_plan' AND p_ref_id IS NOT NULL THEN
        SELECT * INTO v_plan FROM gm_aipub_plans WHERE id = p_ref_id FOR UPDATE;
        IF v_plan IS NULL THEN
            RAISE EXCEPTION 'Plan not found: %', p_ref_id;
        END IF;
        IF v_plan.billing_status != 'frozen' THEN
            RETURN v_plan.consumed_cost;  -- Not frozen, idempotent skip
        END IF;
    ELSE
        RAISE EXCEPTION 'consume_from_frozen requires ref_type and ref_id';
    END IF;

    -- ② Query price + multiplier
    SELECT cost_points INTO v_base_cost
    FROM gm_pricing_rules
    WHERE action_type = p_action_type AND platform_id IS NULL
    LIMIT 1;

    -- Fallback: VIDEO → VIDEO_GENERATE (legacy compatibility)
    IF v_base_cost IS NULL AND p_action_type = 'VIDEO' THEN
        SELECT cost_points INTO v_base_cost
        FROM gm_pricing_rules
        WHERE action_type = 'VIDEO_GENERATE' AND platform_id IS NULL
        LIMIT 1;
    END IF;

    IF p_model_id IS NOT NULL THEN
        SELECT cost_multiplier INTO v_multiplier
        FROM gm_ai_models WHERE id = p_model_id AND is_active = true;
    END IF;

    v_amount := p_count * COALESCE(v_base_cost, 0) * COALESCE(v_multiplier, 1.0);

    IF v_amount <= 0 THEN
        RETURN v_plan.consumed_cost;
    END IF;

    -- ③ Safety cap: cannot consume more than remaining frozen
    IF v_plan.consumed_cost + v_amount > v_plan.frozen_cost THEN
        v_amount := v_plan.frozen_cost - v_plan.consumed_cost;
        IF v_amount <= 0 THEN
            RETURN v_plan.consumed_cost;  -- Already fully consumed
        END IF;
    END IF;

    -- ④ Deduct from frozen (money disappears, not returned to balance)
    PERFORM 1 FROM gm_user_wallets WHERE user_id = p_user_id FOR UPDATE;

    UPDATE gm_user_wallets
    SET frozen_points = frozen_points - v_amount,
        updated_at    = NOW()
    WHERE user_id = p_user_id;

    -- ⑤ Update plan.consumed_cost
    UPDATE gm_aipub_plans
    SET consumed_cost = consumed_cost + v_amount,
        updated_at    = NOW()
    WHERE id = p_ref_id;

    -- ⑥ Transaction record
    INSERT INTO gm_wallet_transactions
        (user_id, amount, type, reference_id, reference_type, description, created_at)
    VALUES (
        p_user_id, -v_amount, 'SETTLE', p_ref_id, p_ref_type,
        format('Sub-task consume: %s * %s * %s = %s pt (%s)',
            p_count, COALESCE(v_base_cost, 0), COALESCE(v_multiplier, 1.0), v_amount, p_action_type),
        NOW()
    );

    RETURN v_plan.consumed_cost + v_amount;
END;
$$ LANGUAGE plpgsql;


-- ============================================================================
-- PART 4: fn_finalize_plan - Refund remaining frozen at terminal state
-- ============================================================================
-- Called by ANY component (API or Scheduler) when plan reaches terminal state.
-- Simple: refund = frozen_cost - consumed_cost. No plan_type logic needed.
-- Idempotent: skips if already settled.

CREATE OR REPLACE FUNCTION fn_finalize_plan(
    p_plan_id    INT,
    p_new_status VARCHAR  -- 'completed' or 'failed'
) RETURNS VOID AS $$
DECLARE
    v_plan      RECORD;
    v_remaining NUMERIC;
BEGIN
    -- ① Lock plan row
    SELECT * INTO v_plan FROM gm_aipub_plans WHERE id = p_plan_id FOR UPDATE;
    IF v_plan IS NULL THEN
        RAISE EXCEPTION 'Plan not found: %', p_plan_id;
    END IF;

    -- ② Idempotent: already terminal + settled
    IF v_plan.status IN ('completed', 'failed') AND v_plan.billing_status = 'settled' THEN
        RETURN;
    END IF;

    -- ③ Update plan status (only if not already terminal)
    IF v_plan.status NOT IN ('completed', 'failed') THEN
        UPDATE gm_aipub_plans
        SET status = p_new_status, updated_at = NOW()
        WHERE id = p_plan_id;
    END IF;

    -- ④ billing_status=none → no billing to process (legacy plan or free plan)
    IF v_plan.billing_status != 'frozen' THEN
        RETURN;
    END IF;

    -- ⑤ Refund remaining frozen amount
    v_remaining := v_plan.frozen_cost - v_plan.consumed_cost;

    IF v_remaining > 0 THEN
        PERFORM 1 FROM gm_user_wallets WHERE user_id = v_plan.user_id FOR UPDATE;

        UPDATE gm_user_wallets
        SET frozen_points  = frozen_points - v_remaining,
            balance_points = balance_points + v_remaining,
            updated_at     = NOW()
        WHERE user_id = v_plan.user_id;

        -- REFUND transaction
        INSERT INTO gm_wallet_transactions
            (user_id, amount, type, reference_id, reference_type, description, created_at)
        VALUES (
            v_plan.user_id, v_remaining, 'REFUND', p_plan_id, 'aipub_plan',
            format('Plan finalized refund: %s pt (frozen %s - consumed %s)',
                v_remaining, v_plan.frozen_cost, v_plan.consumed_cost),
            NOW()
        );
    END IF;

    -- ⑥ Mark as settled
    UPDATE gm_aipub_plans
    SET billing_status = 'settled', updated_at = NOW()
    WHERE id = p_plan_id;
END;
$$ LANGUAGE plpgsql;
