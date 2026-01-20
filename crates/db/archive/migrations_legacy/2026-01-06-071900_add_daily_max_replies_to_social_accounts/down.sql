-- Remove daily_max_replies column from gm_social_accounts table
ALTER TABLE gm_social_accounts 
DROP COLUMN IF EXISTS daily_max_replies;
