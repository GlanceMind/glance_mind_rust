-- Add deposit tracking to wallets
ALTER TABLE gm_user_wallets
ADD COLUMN deposit_cny NUMERIC NOT NULL DEFAULT 0,
ADD COLUMN deposit_usd NUMERIC NOT NULL DEFAULT 0;

COMMENT ON COLUMN gm_user_wallets.deposit_cny IS 'Total deposits in CNY';
COMMENT ON COLUMN gm_user_wallets.deposit_usd IS 'Total deposits in USD';

-- Add freeze tracking to campaigns
ALTER TABLE gm_campaigns
ADD COLUMN is_frozen BOOLEAN NOT NULL DEFAULT FALSE;

COMMENT ON COLUMN gm_campaigns.is_frozen IS 'Whether budget is currently frozen in user wallet';

-- Add constraint to ensure balance >= frozen at all times
ALTER TABLE gm_user_wallets
ADD CONSTRAINT chk_balance_frozen CHECK (balance_points >= frozen_points);
