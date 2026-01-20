-- Rollback previous incomplete migration
ALTER TABLE gm_campaigns
DROP COLUMN IF EXISTS current_offset;

-- Add page_size to platforms (platform-level configuration)
ALTER TABLE gm_platforms
ADD COLUMN page_size INTEGER DEFAULT 20 NOT NULL;

-- Add total_scanned to campaigns (track progress)
ALTER TABLE gm_campaigns
ADD COLUMN total_scanned INTEGER DEFAULT 0 NOT NULL;

-- Add offset/limit to tasks (execution parameters)
ALTER TABLE gm_crawler_tasks
ADD COLUMN search_offset INTEGER DEFAULT 0 NOT NULL,
ADD COLUMN search_limit INTEGER DEFAULT 10 NOT NULL;

-- Indexes
CREATE INDEX idx_campaigns_total_scanned ON gm_campaigns(total_scanned);

-- Comments
COMMENT ON COLUMN gm_platforms.page_size IS 'Default number of items to fetch per request for this platform';
COMMENT ON COLUMN gm_campaigns.total_scanned IS 'Total number of videos scanned so far';
COMMENT ON COLUMN gm_crawler_tasks.search_offset IS 'Starting offset for this task';
COMMENT ON COLUMN gm_crawler_tasks.search_limit IS 'Number of items to fetch in this task';
