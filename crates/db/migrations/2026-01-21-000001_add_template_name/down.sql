-- Remove name column from gm_campaign_templates table
ALTER TABLE gm_campaign_templates DROP COLUMN IF EXISTS name;
