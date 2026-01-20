-- Remove username column and related indexes
DROP INDEX IF EXISTS idx_users_username;
DROP INDEX IF EXISTS idx_users_username_unique;
ALTER TABLE gm_users DROP COLUMN IF EXISTS username;

-- Clean up users with NULL email before adding constraint back
DELETE FROM gm_users WHERE email IS NULL;

-- Restore email to NOT NULL
ALTER TABLE gm_users 
ALTER COLUMN email SET NOT NULL;
