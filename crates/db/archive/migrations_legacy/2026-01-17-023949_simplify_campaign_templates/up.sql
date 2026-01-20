-- Simplify gm_campaign_templates table structure
-- Keep only: dm_prompt, reply_prompt, reply_post_prompt, weight

-- Drop unused columns
ALTER TABLE gm_campaign_templates 
DROP COLUMN IF EXISTS template_content,
DROP COLUMN IF EXISTS tone_instruction,
DROP COLUMN IF EXISTS forbidden_words_prompt;

-- Rename dm_template to dm_prompt for clarity
ALTER TABLE gm_campaign_templates 
RENAME COLUMN dm_template TO dm_prompt;

-- Add reply_post_prompt column
ALTER TABLE gm_campaign_templates 
ADD COLUMN reply_post_prompt TEXT;

-- Update column comments
COMMENT ON COLUMN gm_campaign_templates.dm_prompt IS 'Direct message prompt template';
COMMENT ON COLUMN gm_campaign_templates.reply_prompt IS 'Reply to comment prompt template';
COMMENT ON COLUMN gm_campaign_templates.reply_post_prompt IS 'Reply to post prompt template';
COMMENT ON COLUMN gm_campaign_templates.weight IS 'Template selection weight (higher = more likely to be selected)';
