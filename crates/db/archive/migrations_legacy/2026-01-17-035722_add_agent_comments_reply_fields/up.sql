-- Add suggested_reply_post field to gm_agent_comments table
-- This field stores AI-generated viral comment suggestions (hot comments)
ALTER TABLE gm_agent_comments 
ADD COLUMN suggested_reply_post TEXT;

-- Add comment for documentation
COMMENT ON COLUMN gm_agent_comments.suggested_reply_post IS 'AI-generated viral comment suggestion for hot reply (max 3 per task)';
