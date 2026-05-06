DROP VIEW IF EXISTS gm_resolved_campaign_templates;

UPDATE gm_campaign_templates AS campaign
SET
    name = library.name,
    dm_prompt = library.dm_prompt,
    reply_prompt = library.reply_prompt,
    reply_post_prompt = library.reply_post_prompt,
    updated_at = NOW()
FROM gm_reply_template_library AS library
WHERE campaign.library_template_id = library.id;

DROP INDEX IF EXISTS idx_campaign_templates_campaign_library_unique;
DROP INDEX IF EXISTS idx_campaign_templates_library_template_id;

ALTER TABLE gm_campaign_templates
    DROP COLUMN IF EXISTS library_template_id;

DROP INDEX IF EXISTS idx_reply_template_library_user_id;
DROP TABLE IF EXISTS gm_reply_template_library;
