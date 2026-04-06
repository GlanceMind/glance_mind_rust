-- Add unique constraint on (campaign_id, comment_id) to gm_agent_comments table
-- This is required for the ON CONFLICT upsert logic in save_comment_with_analysis

-- First, remove any duplicate records (keep the one with highest id)
DELETE FROM gm_agent_comments a
USING gm_agent_comments b
WHERE a.campaign_id = b.campaign_id 
  AND a.comment_id = b.comment_id 
  AND a.id < b.id;

-- Add unique constraint (IF NOT EXISTS pattern for idempotency)
DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM pg_constraint 
        WHERE conname = 'gm_agent_comments_campaign_id_comment_id_key'
    ) THEN
        ALTER TABLE gm_agent_comments 
        ADD CONSTRAINT gm_agent_comments_campaign_id_comment_id_key 
        UNIQUE (campaign_id, comment_id);
    END IF;
END $$;
