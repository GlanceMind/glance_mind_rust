-- Add device_id and profile_name to social accounts
-- Created: 2026-01-08

ALTER TABLE gm_social_accounts
ADD COLUMN device_id VARCHAR(255),
ADD COLUMN profile_name VARCHAR(255);
