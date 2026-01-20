-- Drop constraint
ALTER TABLE gm_user_wallets
DROP CONSTRAINT IF EXISTS chk_balance_frozen;

-- Drop freeze tracking from campaigns
ALTER TABLE gm_campaigns
DROP COLUMN IF EXISTS is_frozen;

-- Drop deposit tracking from wallets
ALTER TABLE gm_user_wallets
DROP COLUMN IF EXISTS deposit_cny,
DROP COLUMN IF EXISTS deposit_usd;
