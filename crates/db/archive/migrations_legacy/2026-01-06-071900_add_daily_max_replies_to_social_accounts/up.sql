-- Add daily_max_replies column to gm_social_accounts table
ALTER TABLE gm_social_accounts 
ADD COLUMN IF NOT EXISTS daily_max_replies INTEGER NOT NULL DEFAULT 50;
