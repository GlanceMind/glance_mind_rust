-- =============================================================================
-- Seedance 2.0 Migration
-- 1. Extend ai_task_type CHECK to include 'seedance_video'
-- 2. Insert Seedance model records into gm_ai_models
-- 3. Create fn_freeze_budget_direct for Seedance-specific billing
-- =============================================================================

-- 1. Extend ai_task_type CHECK constraint
DO $$
BEGIN
    IF EXISTS (
        SELECT 1 FROM pg_constraint WHERE conname = 'aipub_ai_tasks_valid_task_type'
    ) THEN
        ALTER TABLE gm_aipub_ai_tasks DROP CONSTRAINT aipub_ai_tasks_valid_task_type;
    END IF;

    ALTER TABLE gm_aipub_ai_tasks ADD CONSTRAINT aipub_ai_tasks_valid_task_type
    CHECK (task_type IN (
        'video_gen', 'content_gen', 'image_gen', 'combined',
        'account_grooming', 'seedance_video'
    ));

    RAISE NOTICE 'Updated task_type constraint to include seedance_video';
END $$;

-- 2. Insert Seedance model records
-- cost_multiplier represents base unit price (per 4s 480p no-audio generation)
INSERT INTO gm_ai_models (name, provider, model_key, cost_multiplier, is_active, model_type)
VALUES ('Seedance 2.0', 'volcengine', 'doubao-seedance-2-0-260128', 2.00, true, 'video')
ON CONFLICT DO NOTHING;

INSERT INTO gm_ai_models (name, provider, model_key, cost_multiplier, is_active, model_type)
VALUES ('Seedance 2.0 fast', 'volcengine', 'doubao-seedance-2-0-fast-260128', 1.00, true, 'video')
ON CONFLICT DO NOTHING;

-- 3. fn_freeze_budget_direct: freeze a pre-calculated fixed amount
-- For Seedance and other non-standard billing where the API layer computes the total
-- Mirrors fn_freeze_budget logic but accepts a direct amount instead of count-based calculation
CREATE OR REPLACE FUNCTION fn_freeze_budget_direct(
    p_user_id   INT,
    p_amount    NUMERIC,
    p_ref_type  VARCHAR,
    p_ref_id    INT
) RETURNS NUMERIC AS $$
DECLARE
    v_available      NUMERIC;
    v_billing_status VARCHAR;
BEGIN
    -- Idempotency: if plan already frozen/settled, return existing frozen_cost
    IF p_ref_type = 'aipub_plan' AND p_ref_id IS NOT NULL THEN
        SELECT billing_status, frozen_cost INTO v_billing_status, v_available
        FROM gm_aipub_plans WHERE id = p_ref_id;
        IF v_billing_status IN ('frozen', 'settled') THEN
            RETURN v_available;
        END IF;
    END IF;

    IF p_amount <= 0 THEN
        RAISE EXCEPTION 'Freeze amount must be positive, got: %', p_amount;
    END IF;

    -- Row-level lock + balance validation (same pattern as fn_freeze_budget)
    SELECT balance_points - frozen_points INTO v_available
    FROM gm_user_wallets WHERE user_id = p_user_id FOR UPDATE;

    IF v_available IS NULL THEN
        RAISE EXCEPTION 'Wallet not found for user_id=%', p_user_id;
    END IF;

    IF v_available < p_amount THEN
        RAISE EXCEPTION 'Insufficient balance: available=%, required=%', v_available, p_amount;
    END IF;

    -- Atomic wallet update: deduct from balance, add to frozen
    UPDATE gm_user_wallets
    SET balance_points = balance_points - p_amount,
        frozen_points  = frozen_points + p_amount,
        updated_at     = NOW()
    WHERE user_id = p_user_id;

    -- Record freeze transaction
    -- Mirror fn_freeze_budget: canonical uppercase type + reference_type so
    -- billing reconciliation/conservation (which filter type='FREEZE' AND
    -- reference_type='aipub_plan') see Seedance freezes like every other provider.
    INSERT INTO gm_wallet_transactions (user_id, amount, type, reference_id, reference_type, description)
    VALUES (p_user_id, -p_amount, 'FREEZE', p_ref_id, p_ref_type, 'Seedance video generation budget freeze');

    -- Atomic plan update
    IF p_ref_type = 'aipub_plan' AND p_ref_id IS NOT NULL THEN
        UPDATE gm_aipub_plans
        SET billing_status = 'frozen',
            frozen_cost    = p_amount,
            consumed_cost  = 0,
            frozen_at      = NOW(),
            updated_at     = NOW()
        WHERE id = p_ref_id;
    END IF;

    RETURN p_amount;
END;
$$ LANGUAGE plpgsql;

COMMENT ON FUNCTION fn_freeze_budget_direct IS
    'Freeze a pre-calculated amount for Seedance or other non-standard billing';

DO $$ BEGIN RAISE NOTICE 'Seedance V1 migration completed'; END $$;
