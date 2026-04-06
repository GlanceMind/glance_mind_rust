-- Rollback: Generic Billing System for AIPub Plans

DROP FUNCTION IF EXISTS fn_finalize_plan(INT, VARCHAR);
DROP FUNCTION IF EXISTS fn_consume_from_frozen(INT, VARCHAR, INT, INT, VARCHAR, INT);
DROP FUNCTION IF EXISTS fn_freeze_budget(INT, INT, INT, INT, INT, INT, INT, VARCHAR, INT);

ALTER TABLE gm_aipub_plans DROP CONSTRAINT IF EXISTS aipub_plans_valid_billing_status;
ALTER TABLE gm_aipub_plans DROP COLUMN IF EXISTS billing_status;
ALTER TABLE gm_aipub_plans DROP COLUMN IF EXISTS frozen_cost;
ALTER TABLE gm_aipub_plans DROP COLUMN IF EXISTS consumed_cost;
ALTER TABLE gm_aipub_plans DROP COLUMN IF EXISTS frozen_at;

ALTER TABLE gm_wallet_transactions DROP COLUMN IF EXISTS reference_type;

DELETE FROM gm_pricing_rules WHERE action_type = 'IMAGE' AND platform_id IS NULL;
DROP INDEX IF EXISTS idx_pricing_rules_action_null_platform;
