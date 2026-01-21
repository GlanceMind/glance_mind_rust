-- ============================================================================
-- Campaign Budget Management Stored Procedures
-- ============================================================================
-- This migration adds:
-- 1. New columns to gm_crawler_tasks table
-- 2. New column to gm_campaigns table
-- 3. 9 stored procedures for budget management
-- ============================================================================

-- ============================================================================
-- PART 1: Table Alterations
-- ============================================================================

-- Add new columns to gm_crawler_tasks
ALTER TABLE gm_crawler_tasks
ADD COLUMN IF NOT EXISTS reserved_amount NUMERIC(18,4) DEFAULT 0,
ADD COLUMN IF NOT EXISTS actual_consumption NUMERIC(18,4) DEFAULT 0,
ADD COLUMN IF NOT EXISTS settled_at TIMESTAMPTZ DEFAULT NULL;

-- Add completed_reason to gm_campaigns
ALTER TABLE gm_campaigns
ADD COLUMN IF NOT EXISTS completed_reason TEXT DEFAULT NULL;

-- ============================================================================
-- PART 2: Helper Function - Get Unit Price
-- ============================================================================

CREATE OR REPLACE FUNCTION fn_get_unit_price(p_platform_id INT)
RETURNS NUMERIC AS $$
DECLARE
    v_unit_price NUMERIC;
BEGIN
    -- Sum SCAN_POST and AI_ANALYZE costs for the platform
    SELECT COALESCE(SUM(cost_points), 0)
    INTO v_unit_price
    FROM gm_pricing_rules
    WHERE platform_id = p_platform_id
      AND action_type IN ('SCAN_POST', 'AI_ANALYZE');
    
    -- Return default if no pricing found
    IF v_unit_price = 0 THEN
        v_unit_price := 1.0; -- Default unit price
    END IF;
    
    RETURN v_unit_price;
END;
$$ LANGUAGE plpgsql;

-- ============================================================================
-- PART 3: fn_activate_campaign - Activate Campaign and Freeze Budget
-- ============================================================================
-- Called when campaign status changes from DRAFT to ACTIVE (first activation)
-- Freezes the budget_cap amount and generates FREEZE transaction

CREATE OR REPLACE FUNCTION fn_activate_campaign(p_campaign_id INT)
RETURNS TABLE(
    success BOOLEAN,
    message TEXT
) AS $$
DECLARE
    v_campaign RECORD;
    v_wallet RECORD;
    v_budget_cap NUMERIC;
BEGIN
    -- 1. Lock and get campaign
    SELECT * INTO v_campaign
    FROM gm_campaigns
    WHERE id = p_campaign_id
    FOR UPDATE;
    
    IF NOT FOUND THEN
        RETURN QUERY SELECT FALSE, 'Campaign not found'::TEXT;
        RETURN;
    END IF;
    
    -- 2. Check if already frozen (idempotency)
    IF v_campaign.is_frozen THEN
        RETURN QUERY SELECT TRUE, 'Campaign already activated'::TEXT;
        RETURN;
    END IF;
    
    -- 3. Get budget cap
    v_budget_cap := COALESCE(v_campaign.budget_cap, 0);
    IF v_budget_cap <= 0 THEN
        -- No budget to freeze, just activate
        UPDATE gm_campaigns
        SET status = 'ACTIVE',
            is_frozen = TRUE,
            updated_at = NOW()
        WHERE id = p_campaign_id;
        
        RETURN QUERY SELECT TRUE, 'Campaign activated (no budget)'::TEXT;
        RETURN;
    END IF;
    
    -- 4. Lock wallet row
    SELECT * INTO v_wallet
    FROM gm_user_wallets
    WHERE user_id = v_campaign.user_id
    FOR UPDATE;
    
    IF NOT FOUND THEN
        RETURN QUERY SELECT FALSE, 'Wallet not found'::TEXT;
        RETURN;
    END IF;
    
    -- 5. Validate balance is sufficient
    IF v_wallet.balance_points < v_budget_cap THEN
        RETURN QUERY SELECT FALSE, 
            format('Insufficient balance. Required: %s, Available: %s', v_budget_cap, v_wallet.balance_points)::TEXT;
        RETURN;
    END IF;
    
    -- 6. Freeze budget: balance -> frozen
    UPDATE gm_user_wallets
    SET balance_points = balance_points - v_budget_cap,
        frozen_points = frozen_points + v_budget_cap,
        updated_at = NOW()
    WHERE user_id = v_campaign.user_id;
    
    -- 7. Update campaign status
    UPDATE gm_campaigns
    SET status = 'ACTIVE',
        is_frozen = TRUE,
        updated_at = NOW()
    WHERE id = p_campaign_id;
    
    -- 8. Generate FREEZE transaction log
    INSERT INTO gm_wallet_transactions (
        user_id, amount, type, reference_id, description, created_at
    ) VALUES (
        v_campaign.user_id,
        -v_budget_cap,
        'FREEZE',
        p_campaign_id,
        format('Campaign activated: %s (budget frozen)', v_campaign.name),
        NOW()
    );
    
    RETURN QUERY SELECT TRUE, 'Campaign activated successfully'::TEXT;
END;
$$ LANGUAGE plpgsql;

-- ============================================================================
-- PART 4: fn_reserve_task_budget - Scheduler creates Task
-- ============================================================================

CREATE OR REPLACE FUNCTION fn_reserve_task_budget(
    p_campaign_id INT,
    p_task_cost NUMERIC,
    p_search_limit INT
)
RETURNS TABLE(
    success BOOLEAN,
    message TEXT,
    new_pending NUMERIC,
    reserved_amount NUMERIC
) AS $$
DECLARE
    v_campaign RECORD;
BEGIN
    -- 1. Lock campaign row
    SELECT * INTO v_campaign
    FROM gm_campaigns
    WHERE id = p_campaign_id
    FOR UPDATE;
    
    IF NOT FOUND THEN
        RETURN QUERY SELECT FALSE, 'Campaign not found'::TEXT, 0::NUMERIC, 0::NUMERIC;
        RETURN;
    END IF;
    
    -- 2. Check campaign status
    IF v_campaign.status NOT IN ('ACTIVE') THEN
        RETURN QUERY SELECT FALSE, 
            format('Campaign is not active. Current status: %s', v_campaign.status)::TEXT,
            v_campaign.pending_consumption,
            0::NUMERIC;
        RETURN;
    END IF;
    
    -- 3. Check budget availability
    IF v_campaign.pending_consumption + v_campaign.actual_consumption + p_task_cost > v_campaign.budget_cap THEN
        -- Budget exhausted, mark campaign as COMPLETED
        UPDATE gm_campaigns
        SET status = 'COMPLETED',
            completed_reason = 'BUDGET_EXHAUSTED',
            updated_at = NOW()
        WHERE id = p_campaign_id;
        
        RETURN QUERY SELECT FALSE, 'BUDGET_EXHAUSTED'::TEXT, v_campaign.pending_consumption, 0::NUMERIC;
        RETURN;
    END IF;
    
    -- 4. Update campaign metrics (no wallet operation - funds already frozen at campaign creation)
    UPDATE gm_campaigns
    SET pending_consumption = pending_consumption + p_task_cost,
        total_scanned = total_scanned + p_search_limit,
        updated_at = NOW()
    WHERE id = p_campaign_id;
    
    RETURN QUERY SELECT TRUE, 
        'Task budget reserved'::TEXT,
        v_campaign.pending_consumption + p_task_cost,
        p_task_cost;
END;
$$ LANGUAGE plpgsql;

-- ============================================================================
-- PART 5: fn_update_task_progress - Agent updates progress
-- ============================================================================

CREATE OR REPLACE FUNCTION fn_update_task_progress(
    p_task_id INT,
    p_increment INT DEFAULT 1
)
RETURNS TABLE(
    success BOOLEAN,
    should_stop BOOLEAN,
    new_process_count INT,
    new_actual_consumption NUMERIC
) AS $$
DECLARE
    v_task RECORD;
    v_campaign RECORD;
    v_unit_price NUMERIC;
    v_consumption_increment NUMERIC;
BEGIN
    -- 1. Lock and get task
    SELECT t.*, c.platform_id, c.status as campaign_status
    INTO v_task
    FROM gm_crawler_tasks t
    JOIN gm_campaigns c ON t.campaign_id = c.id
    WHERE t.id = p_task_id
    FOR UPDATE OF t;
    
    IF NOT FOUND THEN
        RETURN QUERY SELECT FALSE, TRUE, 0, 0::NUMERIC;
        RETURN;
    END IF;
    
    -- 2. Check task status - reject if already completed
    IF v_task.status NOT IN ('pending', 'processing') THEN
        RETURN QUERY SELECT FALSE, TRUE, v_task.process_count, v_task.actual_consumption;
        RETURN;
    END IF;
    
    -- 3. Get unit price and calculate consumption increment
    v_unit_price := fn_get_unit_price(v_task.platform_id);
    v_consumption_increment := p_increment * v_unit_price;
    
    -- 4. Update task progress and actual_consumption
    UPDATE gm_crawler_tasks
    SET process_count = process_count + p_increment,
        actual_consumption = actual_consumption + v_consumption_increment,
        status = 'processing',
        updated_at = NOW()
    WHERE id = p_task_id;
    
    -- 5. Update campaign actual_consumption
    UPDATE gm_campaigns
    SET actual_consumption = actual_consumption + v_consumption_increment,
        updated_at = NOW()
    WHERE id = v_task.campaign_id;
    
    -- 6. Check if should stop (campaign is STOPPING)
    RETURN QUERY SELECT TRUE,
        v_task.campaign_status = 'STOPPING',
        v_task.process_count + p_increment,
        v_task.actual_consumption + v_consumption_increment;
END;
$$ LANGUAGE plpgsql;

-- ============================================================================
-- PART 6: fn_settle_task_consumption - Internal settlement
-- ============================================================================

CREATE OR REPLACE FUNCTION fn_settle_task_consumption(p_task_id INT)
RETURNS TABLE(
    success BOOLEAN,
    actual_cost NUMERIC
) AS $$
DECLARE
    v_task RECORD;
    v_campaign_id INT;
BEGIN
    -- 1. Get and lock task
    SELECT * INTO v_task
    FROM gm_crawler_tasks
    WHERE id = p_task_id
    FOR UPDATE;
    
    IF NOT FOUND THEN
        RETURN QUERY SELECT FALSE, 0::NUMERIC;
        RETURN;
    END IF;
    
    -- 2. Idempotency check
    IF v_task.settled_at IS NOT NULL THEN
        RETURN QUERY SELECT TRUE, v_task.actual_consumption;
        RETURN;
    END IF;
    
    v_campaign_id := v_task.campaign_id;
    
    -- 3. Update campaign pending_consumption
    UPDATE gm_campaigns
    SET pending_consumption = pending_consumption - v_task.reserved_amount,
        updated_at = NOW()
    WHERE id = v_campaign_id;
    
    -- 4. Mark task as settled
    UPDATE gm_crawler_tasks
    SET settled_at = NOW(),
        updated_at = NOW()
    WHERE id = p_task_id;
    
    -- 5. Generate SETTLE transaction log (for audit trail)
    INSERT INTO gm_wallet_transactions (
        user_id, amount, type, reference_id, description, created_at
    )
    SELECT 
        c.user_id,
        -v_task.actual_consumption,
        'SETTLE',
        p_task_id,
        format('Task %s settled: consumed %s points', p_task_id, v_task.actual_consumption),
        NOW()
    FROM gm_campaigns c
    WHERE c.id = v_campaign_id;
    
    RETURN QUERY SELECT TRUE, v_task.actual_consumption;
END;
$$ LANGUAGE plpgsql;

-- ============================================================================
-- PART 7: fn_finalize_campaign - Campaign final settlement
-- ============================================================================

CREATE OR REPLACE FUNCTION fn_finalize_campaign(p_campaign_id INT)
RETURNS TABLE(
    success BOOLEAN,
    refunded_amount NUMERIC
) AS $$
DECLARE
    v_campaign RECORD;
    v_user_id INT;
    v_refund NUMERIC;
    v_unsettled_count INT;
BEGIN
    -- 1. Lock campaign
    SELECT * INTO v_campaign
    FROM gm_campaigns
    WHERE id = p_campaign_id
    FOR UPDATE;
    
    IF NOT FOUND THEN
        RETURN QUERY SELECT FALSE, 0::NUMERIC;
        RETURN;
    END IF;
    
    -- 2. Check if already finalized (status is STOPPED or COMPLETED with completed_reason)
    IF v_campaign.status IN ('STOPPED', 'COMPLETED') AND v_campaign.completed_reason IS NOT NULL THEN
        RETURN QUERY SELECT TRUE, 0::NUMERIC;
        RETURN;
    END IF;
    
    v_user_id := v_campaign.user_id;
    
    -- 3. Check for unsettled tasks
    SELECT COUNT(*) INTO v_unsettled_count
    FROM gm_crawler_tasks
    WHERE campaign_id = p_campaign_id
      AND settled_at IS NULL
      AND status NOT IN ('completed', 'failed', 'cancelled');
    
    IF v_unsettled_count > 0 THEN
        RETURN QUERY SELECT FALSE, 0::NUMERIC;
        RETURN;
    END IF;
    
    -- 4. Calculate refund: budget_cap - actual_consumption
    v_refund := v_campaign.budget_cap - v_campaign.actual_consumption;
    
    -- 5. Lock and update wallet
    UPDATE gm_user_wallets
    SET frozen_points = frozen_points - v_campaign.budget_cap,
        balance_points = balance_points + v_refund,
        updated_at = NOW()
    WHERE user_id = v_user_id;
    
    -- 6. Update campaign status
    UPDATE gm_campaigns
    SET status = CASE 
            WHEN status = 'STOPPING' THEN 'STOPPED'
            ELSE 'COMPLETED'
        END,
        completed_reason = COALESCE(completed_reason, 'FINALIZED'),
        pending_consumption = 0,
        updated_at = NOW()
    WHERE id = p_campaign_id;
    
    -- 7. Generate UNFREEZE transaction log
    INSERT INTO gm_wallet_transactions (
        user_id, amount, type, reference_id, description, created_at
    ) VALUES (
        v_user_id,
        v_campaign.budget_cap,
        'UNFREEZE',
        p_campaign_id,
        format('Campaign %s finalized: unfrozen %s points', p_campaign_id, v_campaign.budget_cap),
        NOW()
    );
    
    -- 8. Generate REFUND transaction log (if any refund)
    IF v_refund > 0 THEN
        INSERT INTO gm_wallet_transactions (
            user_id, amount, type, reference_id, description, created_at
        ) VALUES (
            v_user_id,
            v_refund,
            'REFUND',
            p_campaign_id,
            format('Campaign %s refund: %s points returned', p_campaign_id, v_refund),
            NOW()
        );
    END IF;
    
    RETURN QUERY SELECT TRUE, v_refund;
END;
$$ LANGUAGE plpgsql;

-- ============================================================================
-- PART 8: fn_complete_task - Agent completes task
-- ============================================================================

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
    -- 1. Get and lock task
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
    
    -- 2. Update task status
    UPDATE gm_crawler_tasks
    SET status = p_final_status,
        updated_at = NOW()
    WHERE id = p_task_id;
    
    -- 3. Call settlement
    SELECT * INTO v_settle_result
    FROM fn_settle_task_consumption(p_task_id);
    
    -- 4. Check if all tasks are done and campaign is STOPPING
    IF v_campaign_status = 'STOPPING' THEN
        SELECT COUNT(*) INTO v_active_tasks
        FROM gm_crawler_tasks
        WHERE campaign_id = v_campaign_id
          AND status NOT IN ('completed', 'failed', 'cancelled');
        
        IF v_active_tasks = 0 THEN
            -- All tasks done, finalize campaign
            PERFORM fn_finalize_campaign(v_campaign_id);
            v_campaign_status := 'STOPPED';
        END IF;
    END IF;
    
    RETURN QUERY SELECT TRUE, v_campaign_status;
END;
$$ LANGUAGE plpgsql;

-- ============================================================================
-- PART 9: fn_stop_campaign_gracefully - User stops campaign
-- ============================================================================

CREATE OR REPLACE FUNCTION fn_stop_campaign_gracefully(p_campaign_id INT)
RETURNS TABLE(
    success BOOLEAN,
    immediate_stopped BOOLEAN,
    refunded_amount NUMERIC
) AS $$
DECLARE
    v_campaign RECORD;
    v_active_tasks INT;
    v_finalize_result RECORD;
BEGIN
    -- 1. Lock campaign
    SELECT * INTO v_campaign
    FROM gm_campaigns
    WHERE id = p_campaign_id
    FOR UPDATE;
    
    IF NOT FOUND THEN
        RETURN QUERY SELECT FALSE, FALSE, 0::NUMERIC;
        RETURN;
    END IF;
    
    -- 2. Check if already stopped
    IF v_campaign.status IN ('STOPPED', 'COMPLETED') THEN
        RETURN QUERY SELECT TRUE, TRUE, 0::NUMERIC;
        RETURN;
    END IF;
    
    -- 3. Set status to STOPPING
    UPDATE gm_campaigns
    SET status = 'STOPPING',
        updated_at = NOW()
    WHERE id = p_campaign_id;
    
    -- 4. Check for active tasks
    SELECT COUNT(*) INTO v_active_tasks
    FROM gm_crawler_tasks
    WHERE campaign_id = p_campaign_id
      AND status NOT IN ('completed', 'failed', 'cancelled');
    
    IF v_active_tasks = 0 THEN
        -- No active tasks, finalize immediately
        SELECT * INTO v_finalize_result
        FROM fn_finalize_campaign(p_campaign_id);
        
        RETURN QUERY SELECT TRUE, TRUE, v_finalize_result.refunded_amount;
    ELSE
        -- Has active tasks, wait for them to complete
        RETURN QUERY SELECT TRUE, FALSE, 0::NUMERIC;
    END IF;
END;
$$ LANGUAGE plpgsql;

-- ============================================================================
-- PART 10: fn_cleanup_zombie_tasks - Cron job cleanup
-- ============================================================================

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
    -- Find and process zombie tasks (processing status but not updated for too long)
    FOR v_zombie_task IN
        SELECT id
        FROM gm_crawler_tasks
        WHERE status = 'processing'
          AND updated_at < NOW() - (p_timeout_hours || ' hours')::INTERVAL
        FOR UPDATE SKIP LOCKED
    LOOP
        -- Complete the task as failed
        SELECT * INTO v_complete_result
        FROM fn_complete_task(v_zombie_task.id, 'failed');
        
        IF v_complete_result.success THEN
            v_cleaned := v_cleaned + 1;
        END IF;
    END LOOP;
    
    RETURN QUERY SELECT v_cleaned, v_total_refund;
END;
$$ LANGUAGE plpgsql;

-- ============================================================================
-- PART 11: Create index for better performance
-- ============================================================================

CREATE INDEX IF NOT EXISTS idx_crawler_tasks_campaign_settled 
ON gm_crawler_tasks(campaign_id, settled_at);

CREATE INDEX IF NOT EXISTS idx_crawler_tasks_status_updated 
ON gm_crawler_tasks(status, updated_at);

CREATE INDEX IF NOT EXISTS idx_campaigns_user_status 
ON gm_campaigns(user_id, status);

-- ============================================================================
-- PART 12: Zombie task cleanup (handled by Scheduler, not pg_cron)
-- ============================================================================
-- NOTE: Zombie task cleanup is now handled by the Scheduler service
-- which calls fn_cleanup_zombie_tasks periodically (every hour by default).
-- This approach is preferred over pg_cron because:
-- 1. No dependency on pg_cron extension (not available on all PostgreSQL hosts)
-- 2. Easier to manage and configure within the application
-- 3. Better observability through application logs
