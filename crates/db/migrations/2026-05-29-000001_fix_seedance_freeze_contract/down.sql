-- Revert fn_freeze_budget_direct to its prior (pre-fix) definition.
--
-- WARNING: the prior version recorded freeze transactions as
--   type='freeze' (lowercase) + reference_type=NULL
-- which breaks billing reconciliation/conservation. This down migration exists
-- only for diesel reversibility — do NOT apply it in production.

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

    SELECT balance_points - frozen_points INTO v_available
    FROM gm_user_wallets WHERE user_id = p_user_id FOR UPDATE;

    IF v_available IS NULL THEN
        RAISE EXCEPTION 'Wallet not found for user_id=%', p_user_id;
    END IF;

    IF v_available < p_amount THEN
        RAISE EXCEPTION 'Insufficient balance: available=%, required=%', v_available, p_amount;
    END IF;

    UPDATE gm_user_wallets
    SET balance_points = balance_points - p_amount,
        frozen_points  = frozen_points + p_amount,
        updated_at     = NOW()
    WHERE user_id = p_user_id;

    INSERT INTO gm_wallet_transactions (user_id, amount, type, description, reference_id)
    VALUES (p_user_id, -p_amount, 'freeze', 'Seedance video generation budget freeze', p_ref_id);

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
