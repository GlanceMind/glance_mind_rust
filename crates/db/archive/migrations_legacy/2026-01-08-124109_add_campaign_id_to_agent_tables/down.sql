ALTER TABLE gm_agent_comments DROP CONSTRAINT IF EXISTS fk_agent_comments_campaign;
ALTER TABLE gm_agent_comments DROP COLUMN IF EXISTS campaign_id;

ALTER TABLE gm_agent_videos DROP CONSTRAINT IF EXISTS fk_agent_videos_campaign;
ALTER TABLE gm_agent_videos DROP COLUMN IF EXISTS campaign_id;
