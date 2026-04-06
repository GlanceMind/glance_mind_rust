-- Remove unique constraint from gm_agent_comments table
ALTER TABLE gm_agent_comments 
DROP CONSTRAINT IF EXISTS gm_agent_comments_campaign_id_comment_id_key;
