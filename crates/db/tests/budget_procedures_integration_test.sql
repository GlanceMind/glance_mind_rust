-- ============================================================================
-- Integration Tests for Campaign Budget Management Stored Procedures
-- ============================================================================
-- Run these tests against a test database after migration
-- Usage: psql -d test_db -f budget_procedures_integration_test.sql
-- ============================================================================

-- Test setup: Create a test user and wallet
DO $$
DECLARE
    v_test_user_id INT := 99999;
    v_test_wallet_balance NUMERIC := 1000;
BEGIN
    RAISE NOTICE '=== Starting Budget Procedures Integration Tests ===';
    
    -- Clean up any existing test data
    DELETE FROM gm_wallet_transactions WHERE user_id = v_test_user_id;
    DELETE FROM gm_crawler_tasks WHERE campaign_id IN (SELECT id FROM gm_campaigns WHERE user_id = v_test_user_id);
    DELETE FROM gm_campaigns WHERE user_id = v_test_user_id;
    DELETE FROM gm_user_wallets WHERE user_id = v_test_user_id;
    DELETE FROM gm_users WHERE id = v_test_user_id;
    
    -- Create test user
    INSERT INTO gm_users (id, email, password_hash, status, full_name, role, is_active, created_at)
    VALUES (v_test_user_id, 'test@test.com', 'hash', 'active', 'Test User', 'user', true, NOW());
    
    -- Create test wallet with initial balance
    INSERT INTO gm_user_wallets (user_id, balance_points, frozen_points, created_at)
    VALUES (v_test_user_id, v_test_wallet_balance, 0, NOW());
    
    -- Ensure test platform and region exist (use existing or create)
    INSERT INTO gm_platforms (id, name, display_name, is_active, base_url, page_size, created_at)
    VALUES (99, 'test_platform', 'Test Platform', true, 'https://test.com', 20, NOW())
    ON CONFLICT (id) DO NOTHING;
    
    INSERT INTO gm_regions (id, platform_id, code, display_name, name, is_active, created_at)
    VALUES (99, 99, 'TEST', 'Test Region', 'test', true, NOW())
    ON CONFLICT (id) DO NOTHING;
    
    INSERT INTO gm_ai_models (id, name, provider, model_key, cost_multiplier, is_active, created_at, model_type)
    VALUES (99, 'Test Model', 'test', 'test-model', 1.0, true, NOW(), 'chat')
    ON CONFLICT (id) DO NOTHING;
    
    -- Create pricing rules for test platform
    INSERT INTO gm_pricing_rules (action_type, platform_id, cost_points, created_at)
    VALUES ('SCAN_POST', 99, 0.5, NOW())
    ON CONFLICT DO NOTHING;
    INSERT INTO gm_pricing_rules (action_type, platform_id, cost_points, created_at)
    VALUES ('AI_ANALYZE', 99, 0.5, NOW())
    ON CONFLICT DO NOTHING;
    
    RAISE NOTICE 'Test setup complete: user_id=%, initial_balance=%', v_test_user_id, v_test_wallet_balance;
END;
$$;

-- ============================================================================
-- TEST 1: fn_activate_campaign - Success (Create campaign, then activate)
-- ============================================================================
DO $$
DECLARE
    v_test_user_id INT := 99999;
    v_campaign_id INT;
    v_result RECORD;
    v_wallet RECORD;
    v_campaign RECORD;
    v_tx_count INT;
BEGIN
    RAISE NOTICE '';
    RAISE NOTICE '=== TEST 1: fn_activate_campaign - Success ===';
    
    -- First create a campaign manually (simulating API create)
    INSERT INTO gm_campaigns (
        user_id, name, platform_id, region_id, ai_model_id,
        budget_cap, product_prompt, schedule_type,
        status, pending_consumption, actual_consumption,
        is_frozen, total_scanned, created_at, auto_reply_comments
    ) VALUES (
        v_test_user_id, 'Test Campaign 1', 99, 99, 99,
        500, 'Test product prompt', 'CONTINUOUS',
        'DRAFT', 0, 0, FALSE, 0, NOW(), TRUE
    ) RETURNING id INTO v_campaign_id;
    
    -- Activate the campaign (freezes budget)
    SELECT * INTO v_result FROM fn_activate_campaign(v_campaign_id);
    
    -- Assertions
    IF NOT v_result.success THEN
        RAISE EXCEPTION 'TEST 1 FAILED: Expected success=true, got false. Message: %', v_result.message;
    END IF;
    
    -- Check wallet state
    SELECT * INTO v_wallet FROM gm_user_wallets WHERE user_id = v_test_user_id;
    IF v_wallet.balance_points != 500 THEN
        RAISE EXCEPTION 'TEST 1 FAILED: Expected balance=500, got %', v_wallet.balance_points;
    END IF;
    IF v_wallet.frozen_points != 500 THEN
        RAISE EXCEPTION 'TEST 1 FAILED: Expected frozen=500, got %', v_wallet.frozen_points;
    END IF;
    
    -- Check campaign state
    SELECT * INTO v_campaign FROM gm_campaigns WHERE id = v_campaign_id;
    IF v_campaign.status != 'ACTIVE' THEN
        RAISE EXCEPTION 'TEST 1 FAILED: Expected status=ACTIVE, got %', v_campaign.status;
    END IF;
    IF NOT v_campaign.is_frozen THEN
        RAISE EXCEPTION 'TEST 1 FAILED: Expected is_frozen=true';
    END IF;
    
    -- Check transaction log
    SELECT COUNT(*) INTO v_tx_count FROM gm_wallet_transactions 
    WHERE user_id = v_test_user_id AND type = 'FREEZE' AND reference_id = v_campaign_id;
    IF v_tx_count != 1 THEN
        RAISE EXCEPTION 'TEST 1 FAILED: Expected 1 FREEZE transaction, got %', v_tx_count;
    END IF;
    
    RAISE NOTICE 'TEST 1 PASSED: Campaign activated with ID %, balance=%, frozen=%', 
        v_campaign_id, v_wallet.balance_points, v_wallet.frozen_points;
END;
$$;

-- ============================================================================
-- TEST 2: fn_activate_campaign - Insufficient Balance
-- ============================================================================
DO $$
DECLARE
    v_test_user_id INT := 99999;
    v_campaign_id INT;
    v_result RECORD;
BEGIN
    RAISE NOTICE '';
    RAISE NOTICE '=== TEST 2: fn_activate_campaign - Insufficient Balance ===';
    
    -- Create a campaign with more budget than available
    INSERT INTO gm_campaigns (
        user_id, name, platform_id, region_id, ai_model_id,
        budget_cap, product_prompt, schedule_type,
        status, pending_consumption, actual_consumption,
        is_frozen, total_scanned, created_at, auto_reply_comments
    ) VALUES (
        v_test_user_id, 'Test Campaign Fail', 99, 99, 99,
        1000, 'Test', 'CONTINUOUS',  -- More than remaining 500
        'DRAFT', 0, 0, FALSE, 0, NOW(), TRUE
    ) RETURNING id INTO v_campaign_id;
    
    -- Try to activate
    SELECT * INTO v_result FROM fn_activate_campaign(v_campaign_id);
    
    IF v_result.success THEN
        RAISE EXCEPTION 'TEST 2 FAILED: Expected success=false for insufficient balance';
    END IF;
    
    IF NOT v_result.message LIKE '%Insufficient balance%' THEN
        RAISE EXCEPTION 'TEST 2 FAILED: Expected insufficient balance message, got: %', v_result.message;
    END IF;
    
    -- Clean up the failed campaign
    DELETE FROM gm_campaigns WHERE id = v_campaign_id;
    
    RAISE NOTICE 'TEST 2 PASSED: Correctly rejected campaign activation with insufficient balance';
END;
$$;

-- ============================================================================
-- TEST 3: fn_reserve_task_budget - Success
-- ============================================================================
DO $$
DECLARE
    v_test_user_id INT := 99999;
    v_campaign_id INT;
    v_result RECORD;
    v_campaign RECORD;
BEGIN
    RAISE NOTICE '';
    RAISE NOTICE '=== TEST 3: fn_reserve_task_budget - Success ===';
    
    -- Get the campaign ID from Test 1
    SELECT id INTO v_campaign_id FROM gm_campaigns 
    WHERE user_id = v_test_user_id AND name = 'Test Campaign 1';
    
    -- Reserve budget for task
    SELECT * INTO v_result FROM fn_reserve_task_budget(
        p_campaign_id := v_campaign_id,
        p_task_cost := 100,
        p_search_limit := 20
    );
    
    IF NOT v_result.success THEN
        RAISE EXCEPTION 'TEST 3 FAILED: Expected success=true, got false. Message: %', v_result.message;
    END IF;
    
    IF v_result.reserved_amount != 100 THEN
        RAISE EXCEPTION 'TEST 3 FAILED: Expected reserved_amount=100, got %', v_result.reserved_amount;
    END IF;
    
    -- Check campaign pending_consumption
    SELECT * INTO v_campaign FROM gm_campaigns WHERE id = v_campaign_id;
    IF v_campaign.pending_consumption != 100 THEN
        RAISE EXCEPTION 'TEST 3 FAILED: Expected pending_consumption=100, got %', v_campaign.pending_consumption;
    END IF;
    
    RAISE NOTICE 'TEST 3 PASSED: Task budget reserved, pending_consumption=%', v_campaign.pending_consumption;
END;
$$;

-- ============================================================================
-- TEST 4: fn_update_task_progress - Success
-- ============================================================================
DO $$
DECLARE
    v_test_user_id INT := 99999;
    v_campaign_id INT;
    v_task_id INT;
    v_result RECORD;
    v_task RECORD;
    v_campaign RECORD;
BEGIN
    RAISE NOTICE '';
    RAISE NOTICE '=== TEST 4: fn_update_task_progress - Success ===';
    
    -- Get the campaign ID
    SELECT id INTO v_campaign_id FROM gm_campaigns 
    WHERE user_id = v_test_user_id AND name = 'Test Campaign 1';
    
    -- Create a test task
    INSERT INTO gm_crawler_tasks (
        campaign_id, max_count, process_count, status, search_offset, search_limit, 
        reserved_amount, actual_consumption, created_at
    ) VALUES (
        v_campaign_id, 10, 0, 'pending', 0, 20, 100, 0, NOW()
    ) RETURNING id INTO v_task_id;
    
    -- Update progress (simulate processing 1 video)
    SELECT * INTO v_result FROM fn_update_task_progress(v_task_id, 1);
    
    IF NOT v_result.success THEN
        RAISE EXCEPTION 'TEST 4 FAILED: Expected success=true';
    END IF;
    
    IF v_result.new_process_count != 1 THEN
        RAISE EXCEPTION 'TEST 4 FAILED: Expected new_process_count=1, got %', v_result.new_process_count;
    END IF;
    
    -- Check task actual_consumption updated
    SELECT * INTO v_task FROM gm_crawler_tasks WHERE id = v_task_id;
    IF v_task.actual_consumption <= 0 THEN
        RAISE EXCEPTION 'TEST 4 FAILED: Expected task actual_consumption > 0, got %', v_task.actual_consumption;
    END IF;
    
    -- Check campaign actual_consumption updated
    SELECT * INTO v_campaign FROM gm_campaigns WHERE id = v_campaign_id;
    IF v_campaign.actual_consumption <= 0 THEN
        RAISE EXCEPTION 'TEST 4 FAILED: Expected campaign actual_consumption > 0, got %', v_campaign.actual_consumption;
    END IF;
    
    RAISE NOTICE 'TEST 4 PASSED: Progress updated, task_consumption=%, campaign_consumption=%', 
        v_task.actual_consumption, v_campaign.actual_consumption;
END;
$$;

-- ============================================================================
-- TEST 5: fn_complete_task - Success
-- ============================================================================
DO $$
DECLARE
    v_test_user_id INT := 99999;
    v_campaign_id INT;
    v_task_id INT;
    v_result RECORD;
    v_task RECORD;
    v_campaign RECORD;
    v_tx_count INT;
BEGIN
    RAISE NOTICE '';
    RAISE NOTICE '=== TEST 5: fn_complete_task - Success ===';
    
    -- Get the campaign and task IDs
    SELECT id INTO v_campaign_id FROM gm_campaigns 
    WHERE user_id = v_test_user_id AND name = 'Test Campaign 1';
    
    SELECT id INTO v_task_id FROM gm_crawler_tasks 
    WHERE campaign_id = v_campaign_id AND settled_at IS NULL
    LIMIT 1;
    
    -- Complete the task
    SELECT * INTO v_result FROM fn_complete_task(v_task_id, 'completed');
    
    IF NOT v_result.success THEN
        RAISE EXCEPTION 'TEST 5 FAILED: Expected success=true';
    END IF;
    
    -- Check task status and settled_at
    SELECT * INTO v_task FROM gm_crawler_tasks WHERE id = v_task_id;
    IF v_task.status != 'completed' THEN
        RAISE EXCEPTION 'TEST 5 FAILED: Expected status=completed, got %', v_task.status;
    END IF;
    IF v_task.settled_at IS NULL THEN
        RAISE EXCEPTION 'TEST 5 FAILED: Expected settled_at not null';
    END IF;
    
    -- Check SETTLE transaction created
    SELECT COUNT(*) INTO v_tx_count FROM gm_wallet_transactions 
    WHERE type = 'SETTLE' AND reference_id = v_task_id;
    IF v_tx_count != 1 THEN
        RAISE EXCEPTION 'TEST 5 FAILED: Expected 1 SETTLE transaction, got %', v_tx_count;
    END IF;
    
    -- Check campaign pending_consumption reduced
    SELECT * INTO v_campaign FROM gm_campaigns WHERE id = v_campaign_id;
    IF v_campaign.pending_consumption != 0 THEN
        RAISE EXCEPTION 'TEST 5 FAILED: Expected pending_consumption=0 after settlement, got %', v_campaign.pending_consumption;
    END IF;
    
    RAISE NOTICE 'TEST 5 PASSED: Task completed and settled, campaign_status=%', v_result.campaign_status;
END;
$$;

-- ============================================================================
-- TEST 6: fn_stop_campaign_gracefully - Immediate Stop (no active tasks)
-- ============================================================================
DO $$
DECLARE
    v_test_user_id INT := 99999;
    v_campaign_id INT;
    v_result RECORD;
    v_wallet RECORD;
    v_campaign RECORD;
    v_tx_count INT;
BEGIN
    RAISE NOTICE '';
    RAISE NOTICE '=== TEST 6: fn_stop_campaign_gracefully - Immediate Stop ===';
    
    -- Get the campaign ID
    SELECT id INTO v_campaign_id FROM gm_campaigns 
    WHERE user_id = v_test_user_id AND name = 'Test Campaign 1';
    
    -- Get wallet state before
    SELECT * INTO v_wallet FROM gm_user_wallets WHERE user_id = v_test_user_id;
    RAISE NOTICE 'Before stop: balance=%, frozen=%', v_wallet.balance_points, v_wallet.frozen_points;
    
    -- Stop campaign gracefully (all tasks already completed)
    SELECT * INTO v_result FROM fn_stop_campaign_gracefully(v_campaign_id);
    
    IF NOT v_result.success THEN
        RAISE EXCEPTION 'TEST 6 FAILED: Expected success=true';
    END IF;
    
    IF NOT v_result.immediate_stopped THEN
        RAISE EXCEPTION 'TEST 6 FAILED: Expected immediate_stopped=true';
    END IF;
    
    -- Check wallet state after (should have refund)
    SELECT * INTO v_wallet FROM gm_user_wallets WHERE user_id = v_test_user_id;
    RAISE NOTICE 'After stop: balance=%, frozen=%', v_wallet.balance_points, v_wallet.frozen_points;
    
    IF v_wallet.frozen_points != 0 THEN
        RAISE EXCEPTION 'TEST 6 FAILED: Expected frozen_points=0 after stop, got %', v_wallet.frozen_points;
    END IF;
    
    -- Check campaign status
    SELECT * INTO v_campaign FROM gm_campaigns WHERE id = v_campaign_id;
    IF v_campaign.status NOT IN ('STOPPED', 'COMPLETED') THEN
        RAISE EXCEPTION 'TEST 6 FAILED: Expected status STOPPED or COMPLETED, got %', v_campaign.status;
    END IF;
    
    -- Check UNFREEZE and REFUND transactions
    SELECT COUNT(*) INTO v_tx_count FROM gm_wallet_transactions 
    WHERE type = 'UNFREEZE' AND reference_id = v_campaign_id;
    IF v_tx_count < 1 THEN
        RAISE EXCEPTION 'TEST 6 FAILED: Expected UNFREEZE transaction';
    END IF;
    
    RAISE NOTICE 'TEST 6 PASSED: Campaign stopped, refunded_amount=%, final balance=%', 
        v_result.refunded_amount, v_wallet.balance_points;
END;
$$;

-- ============================================================================
-- TEST 7: fn_complete_task - Idempotency
-- ============================================================================
DO $$
DECLARE
    v_test_user_id INT := 99999;
    v_campaign_id INT;
    v_task_id INT;
    v_result1 RECORD;
    v_result2 RECORD;
    v_tx_count_before INT;
    v_tx_count_after INT;
BEGIN
    RAISE NOTICE '';
    RAISE NOTICE '=== TEST 7: fn_complete_task - Idempotency ===';
    
    -- Create a new campaign manually
    INSERT INTO gm_campaigns (
        user_id, name, platform_id, region_id, ai_model_id,
        budget_cap, product_prompt, schedule_type,
        status, pending_consumption, actual_consumption,
        is_frozen, total_scanned, created_at, auto_reply_comments
    ) VALUES (
        v_test_user_id, 'Idempotency Test Campaign', 99, 99, 99,
        100, 'Test', 'CONTINUOUS',
        'DRAFT', 0, 0, FALSE, 0, NOW(), TRUE
    ) RETURNING id INTO v_campaign_id;
    
    -- Activate the campaign
    PERFORM fn_activate_campaign(v_campaign_id);
    
    -- Update pending_consumption and actual_consumption for task (to maintain consistency)
    UPDATE gm_campaigns SET pending_consumption = 50, actual_consumption = 25 WHERE id = v_campaign_id;
    
    -- Create task
    INSERT INTO gm_crawler_tasks (
        campaign_id, max_count, process_count, status, search_offset, search_limit,
        reserved_amount, actual_consumption, created_at
    ) VALUES (
        v_campaign_id, 10, 5, 'processing', 0, 20, 50, 25, NOW()
    ) RETURNING id INTO v_task_id;
    
    -- First complete
    SELECT * INTO v_result1 FROM fn_complete_task(v_task_id, 'completed');
    
    -- Count transactions
    SELECT COUNT(*) INTO v_tx_count_before FROM gm_wallet_transactions;
    
    -- Second complete (should be idempotent)
    SELECT * INTO v_result2 FROM fn_complete_task(v_task_id, 'completed');
    
    -- Count transactions again
    SELECT COUNT(*) INTO v_tx_count_after FROM gm_wallet_transactions;
    
    -- Both should succeed
    IF NOT v_result1.success OR NOT v_result2.success THEN
        RAISE EXCEPTION 'TEST 7 FAILED: Expected both calls to succeed';
    END IF;
    
    -- No new transactions should be created on second call
    IF v_tx_count_after != v_tx_count_before THEN
        RAISE EXCEPTION 'TEST 7 FAILED: Idempotency violation - new transactions created on duplicate call';
    END IF;
    
    RAISE NOTICE 'TEST 7 PASSED: fn_complete_task is idempotent';
END;
$$;

-- ============================================================================
-- TEST 8: Data Consistency Check
-- ============================================================================
DO $$
DECLARE
    v_test_user_id INT := 99999;
    v_wallet RECORD;
    v_campaign RECORD;
    v_task_sum NUMERIC;
    v_consistency_ok BOOLEAN := TRUE;
BEGIN
    RAISE NOTICE '';
    RAISE NOTICE '=== TEST 8: Data Consistency Check ===';
    
    -- For each campaign, check:
    -- 1. campaign.actual_consumption = SUM(task.actual_consumption)
    -- 2. campaign.pending_consumption = SUM(unsettled task.reserved_amount)
    
    FOR v_campaign IN 
        SELECT c.*, 
               COALESCE(SUM(t.actual_consumption), 0) as task_actual_sum,
               COALESCE(SUM(CASE WHEN t.settled_at IS NULL THEN t.reserved_amount ELSE 0 END), 0) as unsettled_reserved_sum
        FROM gm_campaigns c
        LEFT JOIN gm_crawler_tasks t ON t.campaign_id = c.id
        WHERE c.user_id = v_test_user_id
        GROUP BY c.id
    LOOP
        IF v_campaign.actual_consumption != v_campaign.task_actual_sum THEN
            RAISE NOTICE 'Consistency issue: Campaign % actual_consumption (%) != task sum (%)', 
                v_campaign.id, v_campaign.actual_consumption, v_campaign.task_actual_sum;
            v_consistency_ok := FALSE;
        END IF;
        
        IF v_campaign.status IN ('ACTIVE', 'STOPPING') AND 
           v_campaign.pending_consumption != v_campaign.unsettled_reserved_sum THEN
            RAISE NOTICE 'Consistency issue: Campaign % pending_consumption (%) != unsettled sum (%)', 
                v_campaign.id, v_campaign.pending_consumption, v_campaign.unsettled_reserved_sum;
            v_consistency_ok := FALSE;
        END IF;
    END LOOP;
    
    -- Check wallet consistency
    SELECT * INTO v_wallet FROM gm_user_wallets WHERE user_id = v_test_user_id;
    
    SELECT COALESCE(SUM(budget_cap), 0) INTO v_task_sum 
    FROM gm_campaigns 
    WHERE user_id = v_test_user_id AND status IN ('ACTIVE', 'STOPPING');
    
    IF v_wallet.frozen_points != v_task_sum THEN
        RAISE NOTICE 'Consistency issue: frozen_points (%) != active campaign budgets (%)', 
            v_wallet.frozen_points, v_task_sum;
        v_consistency_ok := FALSE;
    END IF;
    
    IF v_consistency_ok THEN
        RAISE NOTICE 'TEST 8 PASSED: All data consistency checks passed';
    ELSE
        RAISE EXCEPTION 'TEST 8 FAILED: Data consistency issues found';
    END IF;
END;
$$;

-- ============================================================================
-- TEST 9: fn_cleanup_zombie_tasks
-- ============================================================================
DO $$
DECLARE
    v_test_user_id INT := 99999;
    v_campaign_id INT;
    v_zombie_task_id INT;
    v_result RECORD;
    v_task RECORD;
BEGIN
    RAISE NOTICE '';
    RAISE NOTICE '=== TEST 9: fn_cleanup_zombie_tasks ===';
    
    -- Create a new campaign manually
    INSERT INTO gm_campaigns (
        user_id, name, platform_id, region_id, ai_model_id,
        budget_cap, product_prompt, schedule_type,
        status, pending_consumption, actual_consumption,
        is_frozen, total_scanned, created_at, auto_reply_comments
    ) VALUES (
        v_test_user_id, 'Zombie Test Campaign', 99, 99, 99,
        100, 'Test', 'CONTINUOUS',
        'DRAFT', 0, 0, FALSE, 0, NOW(), TRUE
    ) RETURNING id INTO v_campaign_id;
    
    -- Activate the campaign
    PERFORM fn_activate_campaign(v_campaign_id);
    
    -- Create a "zombie" task (processing status, old updated_at)
    INSERT INTO gm_crawler_tasks (
        campaign_id, max_count, process_count, status, search_offset, search_limit,
        reserved_amount, actual_consumption, created_at, updated_at
    ) VALUES (
        v_campaign_id, 10, 5, 'processing', 0, 20, 50, 25, 
        NOW() - INTERVAL '48 hours',  -- created 48 hours ago
        NOW() - INTERVAL '48 hours'   -- last updated 48 hours ago
    ) RETURNING id INTO v_zombie_task_id;
    
    -- Update campaign pending_consumption
    UPDATE gm_campaigns SET pending_consumption = 50 WHERE id = v_campaign_id;
    
    -- Run cleanup (24 hour timeout)
    SELECT * INTO v_result FROM fn_cleanup_zombie_tasks(24);
    
    -- Check zombie task was cleaned
    SELECT * INTO v_task FROM gm_crawler_tasks WHERE id = v_zombie_task_id;
    
    IF v_task.status != 'failed' THEN
        RAISE EXCEPTION 'TEST 9 FAILED: Expected zombie task status=failed, got %', v_task.status;
    END IF;
    
    IF v_task.settled_at IS NULL THEN
        RAISE EXCEPTION 'TEST 9 FAILED: Expected zombie task to be settled';
    END IF;
    
    RAISE NOTICE 'TEST 9 PASSED: Zombie tasks cleaned up, count=%', v_result.cleaned_count;
END;
$$;

-- ============================================================================
-- Cleanup
-- ============================================================================
DO $$
DECLARE
    v_test_user_id INT := 99999;
BEGIN
    RAISE NOTICE '';
    RAISE NOTICE '=== Cleaning up test data ===';
    
    -- Clean up test data
    DELETE FROM gm_wallet_transactions WHERE user_id = v_test_user_id;
    DELETE FROM gm_crawler_tasks WHERE campaign_id IN (SELECT id FROM gm_campaigns WHERE user_id = v_test_user_id);
    DELETE FROM gm_campaigns WHERE user_id = v_test_user_id;
    DELETE FROM gm_user_wallets WHERE user_id = v_test_user_id;
    DELETE FROM gm_users WHERE id = v_test_user_id;
    
    -- Clean up test pricing rules
    DELETE FROM gm_pricing_rules WHERE platform_id = 99;
    DELETE FROM gm_regions WHERE id = 99;
    DELETE FROM gm_platforms WHERE id = 99;
    DELETE FROM gm_ai_models WHERE id = 99;
    
    RAISE NOTICE 'Test cleanup complete';
    RAISE NOTICE '';
    RAISE NOTICE '=== All Integration Tests Completed Successfully! ===';
END;
$$;
