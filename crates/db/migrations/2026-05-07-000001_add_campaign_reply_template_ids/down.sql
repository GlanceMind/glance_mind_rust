DROP INDEX IF EXISTS idx_campaigns_reply_template_ids_gin;

ALTER TABLE gm_campaigns
    DROP CONSTRAINT IF EXISTS chk_campaign_reply_template_ids_sane;

ALTER TABLE gm_campaigns
    DROP COLUMN IF EXISTS reply_template_ids;
