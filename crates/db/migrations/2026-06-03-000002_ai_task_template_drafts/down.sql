-- Reverse Module D2.
DROP INDEX IF EXISTS uq_aipub_plans_source_draft;
DROP INDEX IF EXISTS uq_campaigns_source_draft;
ALTER TABLE gm_aipub_plans DROP COLUMN IF EXISTS source_draft_id;
ALTER TABLE gm_campaigns DROP COLUMN IF EXISTS source_draft_id;
DROP INDEX IF EXISTS idx_tt_drafts_conv;
DROP TABLE IF EXISTS gm_ai_task_template_drafts;
