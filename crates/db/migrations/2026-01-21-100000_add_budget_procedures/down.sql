-- ============================================================================
-- Rollback: Campaign Budget Management Stored Procedures
-- ============================================================================

-- Drop indexes
DROP INDEX IF EXISTS idx_crawler_tasks_campaign_settled;
DROP INDEX IF EXISTS idx_crawler_tasks_status_updated;
DROP INDEX IF EXISTS idx_campaigns_user_status;

-- Drop stored procedures
DROP FUNCTION IF EXISTS fn_cleanup_zombie_tasks(INT);
DROP FUNCTION IF EXISTS fn_stop_campaign_gracefully(INT);
DROP FUNCTION IF EXISTS fn_complete_task(INT, TEXT);
DROP FUNCTION IF EXISTS fn_finalize_campaign(INT);
DROP FUNCTION IF EXISTS fn_settle_task_consumption(INT);
DROP FUNCTION IF EXISTS fn_update_task_progress(INT, INT);
DROP FUNCTION IF EXISTS fn_reserve_task_budget(INT, NUMERIC, INT);
DROP FUNCTION IF EXISTS fn_activate_campaign(INT);
DROP FUNCTION IF EXISTS fn_get_unit_price(INT);

-- Remove added columns from gm_campaigns
ALTER TABLE gm_campaigns DROP COLUMN IF EXISTS completed_reason;

-- Remove added columns from gm_crawler_tasks
ALTER TABLE gm_crawler_tasks DROP COLUMN IF EXISTS reserved_amount;
ALTER TABLE gm_crawler_tasks DROP COLUMN IF EXISTS actual_consumption;
ALTER TABLE gm_crawler_tasks DROP COLUMN IF EXISTS settled_at;
