-- Remove search_options column from gm_campaigns table
ALTER TABLE gm_campaigns 
DROP COLUMN IF EXISTS search_options;
