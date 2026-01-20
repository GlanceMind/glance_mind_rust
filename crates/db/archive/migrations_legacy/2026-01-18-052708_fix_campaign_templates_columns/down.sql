-- 回滚迁移：恢复旧的列结构
ALTER TABLE gm_campaign_templates 
  RENAME COLUMN dm_prompt TO dm_template;

-- 删除新添加的列
ALTER TABLE gm_campaign_templates 
  DROP COLUMN IF EXISTS reply_post_prompt;

-- 恢复旧列（如果需要回滚）
ALTER TABLE gm_campaign_templates 
  ADD COLUMN IF NOT EXISTS template_content TEXT NOT NULL DEFAULT '',
  ADD COLUMN IF NOT EXISTS tone_instruction TEXT,
  ADD COLUMN IF NOT EXISTS forbidden_words_prompt TEXT;
