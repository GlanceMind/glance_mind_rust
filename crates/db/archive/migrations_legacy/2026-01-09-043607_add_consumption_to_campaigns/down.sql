-- Remove consumption tracking fields from campaigns
DROP INDEX IF EXISTS idx_campaigns_consumption;

ALTER TABLE gm_campaigns
DROP COLUMN pending_consumption,
DROP COLUMN actual_consumption;
