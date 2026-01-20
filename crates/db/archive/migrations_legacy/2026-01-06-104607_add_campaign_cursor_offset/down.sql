-- Remove page_size from platforms
ALTER TABLE gm_platforms
DROP COLUMN IF EXISTS page_size;

-- Remove total_scanned from campaigns
ALTER TABLE gm_campaigns
DROP COLUMN IF EXISTS total_scanned;

-- Remove offset/limit from tasks
ALTER TABLE gm_crawler_tasks
DROP COLUMN IF EXISTS search_offset,
DROP COLUMN IF EXISTS search_limit;

DROP INDEX IF EXISTS idx_campaigns_total_scanned;
