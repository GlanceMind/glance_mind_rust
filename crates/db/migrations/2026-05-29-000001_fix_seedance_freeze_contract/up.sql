-- =============================================================================
-- Fix Seedance freeze billing contract (deploy via normal CD migration path)
-- =============================================================================
-- fn_freeze_budget_direct previously recorded freeze transactions as
--   type='freeze' (lowercase) + reference_type=NULL
-- which billing reconciliation/conservation (which filter
--   type='FREEZE' AND reference_type='aipub_plan')
-- cannot see — breaking the FREEZE = SETTLE + REFUND invariant for Seedance.
--
-- The fixed definition already exists in the flat file
-- crates/db/migrations/seedance_v1.sql, but `diesel migration run` only applies
-- VERSIONED migrations and skips flat files — so the fix never reached prod via
-- CD. This versioned migration re-defines the function with the canonical
-- contract so it deploys through the normal pipeline.
--
-- Idempotent: CREATE OR REPLACE (safe to run whether or not the function exists).
-- =============================================================================

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
    'Freeze a pre-calculated amount for Seedance or other non-standard billing (canonical FREEZE + reference_type)';
