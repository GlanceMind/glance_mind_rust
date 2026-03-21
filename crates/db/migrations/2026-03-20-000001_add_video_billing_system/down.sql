-- Rollback: Video Billing System

DROP FUNCTION IF EXISTS fn_claim_timed_out_video_tasks(INT, INT);
DROP FUNCTION IF EXISTS fn_fail_video_task(INT, INT, TEXT);
DROP FUNCTION IF EXISTS fn_complete_video_task_success(INT, INT, TEXT);
DROP FUNCTION IF EXISTS fn_allocate_aipub_task_billing(INT, INT, INT, INT);

DROP INDEX IF EXISTS idx_wallet_txn_biz_key;
ALTER TABLE gm_wallet_transactions DROP COLUMN IF EXISTS biz_key;
ALTER TABLE gm_wallet_transactions DROP COLUMN IF EXISTS reference_sub_id;
ALTER TABLE gm_wallet_transactions DROP COLUMN IF EXISTS reference_sub_type;

ALTER TABLE gm_aipub_tasks DROP COLUMN IF EXISTS video_ai_task_id;
ALTER TABLE gm_aipub_tasks DROP COLUMN IF EXISTS video_stage_started_at;

DROP INDEX IF EXISTS idx_task_billings_unique_task;
DROP INDEX IF EXISTS idx_task_billings_task;
DROP INDEX IF EXISTS idx_task_billings_plan;
DROP TABLE IF EXISTS gm_aipub_task_billings;
