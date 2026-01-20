-- 重命名列以匹配新的命名约定
ALTER TABLE gm_campaign_templates 
  RENAME COLUMN dm_template TO dm_prompt;

-- 添加新列
ALTER TABLE gm_campaign_templates 
  ADD COLUMN IF NOT EXISTS reply_post_prompt TEXT;

-- 删除旧的不再使用的列
ALTER TABLE gm_campaign_templates 
  DROP COLUMN IF EXISTS template_content,
  DROP COLUMN IF EXISTS tone_instruction,
  DROP COLUMN IF EXISTS forbidden_words_prompt;
