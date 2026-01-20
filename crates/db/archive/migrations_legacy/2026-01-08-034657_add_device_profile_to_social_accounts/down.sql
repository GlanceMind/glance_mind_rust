-- Rollback: Remove device_id and profile_name from social accounts

ALTER TABLE gm_social_accounts
DROP COLUMN IF EXISTS device_id,
DROP COLUMN IF EXISTS profile_name;
