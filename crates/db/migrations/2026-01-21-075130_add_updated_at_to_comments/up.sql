-- Add updated_at column to TikTok tables for consistency with other platforms
-- These columns track when records were last updated (e.g., after AI analysis)

-- 1. Add to gm_agent_comments (TikTok comments)
ALTER TABLE gm_agent_comments 
ADD COLUMN IF NOT EXISTS updated_at TIMESTAMPTZ;

UPDATE gm_agent_comments 
SET updated_at = created_at 
WHERE updated_at IS NULL;

COMMENT ON COLUMN gm_agent_comments.updated_at IS 'Timestamp when the record was last updated';

-- 2. Add to gm_agent_videos (TikTok videos)
ALTER TABLE gm_agent_videos 
ADD COLUMN IF NOT EXISTS updated_at TIMESTAMPTZ;

UPDATE gm_agent_videos 
SET updated_at = created_at 
WHERE updated_at IS NULL;

COMMENT ON COLUMN gm_agent_videos.updated_at IS 'Timestamp when the record was last updated';
