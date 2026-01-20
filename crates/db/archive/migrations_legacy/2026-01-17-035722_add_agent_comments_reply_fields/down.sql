-- Remove suggested_reply_post field from gm_agent_comments table
ALTER TABLE gm_agent_comments 
DROP COLUMN suggested_reply_post;
