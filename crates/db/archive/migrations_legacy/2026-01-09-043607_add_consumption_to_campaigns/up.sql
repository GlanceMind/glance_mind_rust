-- Add consumption tracking fields to campaigns
ALTER TABLE gm_campaigns
ADD COLUMN pending_consumption NUMERIC NOT NULL DEFAULT 0,
ADD COLUMN actual_consumption NUMERIC NOT NULL DEFAULT 0;

COMMENT ON COLUMN gm_campaigns.pending_consumption IS 'Points reserved for created but uncompleted tasks';
COMMENT ON COLUMN gm_campaigns.actual_consumption IS 'Points consumed by completed tasks';

-- Add index for budget queries
CREATE INDEX idx_campaigns_consumption ON gm_campaigns(pending_consumption, actual_consumption);
