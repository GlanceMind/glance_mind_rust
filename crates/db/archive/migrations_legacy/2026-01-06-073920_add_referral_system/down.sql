-- Drop referral_earnings table
DROP TABLE IF EXISTS gm_referral_earnings CASCADE;

-- Drop referrals table  
DROP TABLE IF EXISTS gm_referrals CASCADE;

-- Remove indexes and columns from users
DROP INDEX IF EXISTS idx_users_invited_by;
DROP INDEX IF EXISTS idx_users_invite_code;

ALTER TABLE gm_users 
DROP COLUMN IF EXISTS invited_by,
DROP COLUMN IF EXISTS invite_code;
