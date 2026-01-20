DROP INDEX IF EXISTS idx_agent_comments_status;
ALTER TABLE gm_agent_comments DROP COLUMN IF EXISTS status;
