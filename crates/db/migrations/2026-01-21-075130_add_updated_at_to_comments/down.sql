-- Remove updated_at columns from TikTok tables
ALTER TABLE gm_agent_comments DROP COLUMN IF EXISTS updated_at;
ALTER TABLE gm_agent_videos DROP COLUMN IF EXISTS updated_at;
