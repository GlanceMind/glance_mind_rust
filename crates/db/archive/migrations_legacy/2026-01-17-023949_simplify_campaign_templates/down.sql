-- Revert simplification of gm_campaign_templates table

-- Remove reply_post_prompt column
ALTER TABLE gm_campaign_templates 
DROP COLUMN IF EXISTS reply_post_prompt;

-- Rename dm_prompt back to dm_template
ALTER TABLE gm_campaign_templates 
RENAME COLUMN dm_prompt TO dm_template;

-- Add back removed columns
ALTER TABLE gm_campaign_templates 
ADD COLUMN template_content TEXT NOT NULL DEFAULT '',
ADD COLUMN tone_instruction TEXT,
ADD COLUMN forbidden_words_prompt TEXT;
