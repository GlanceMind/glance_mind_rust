-- Add campaign_id to agent_videos
ALTER TABLE gm_agent_videos
ADD COLUMN campaign_id INTEGER;

-- Add foreign key constraint
ALTER TABLE gm_agent_videos
ADD CONSTRAINT fk_agent_videos_campaign
FOREIGN KEY (campaign_id) REFERENCES gm_campaigns(id) ON DELETE CASCADE;

-- Add campaign_id to agent_comments
ALTER TABLE gm_agent_comments  
ADD COLUMN campaign_id INTEGER;

-- Add foreign key constraint
ALTER TABLE gm_agent_comments
ADD CONSTRAINT fk_agent_comments_campaign
FOREIGN KEY (campaign_id) REFERENCES gm_campaigns(id) ON DELETE CASCADE;
