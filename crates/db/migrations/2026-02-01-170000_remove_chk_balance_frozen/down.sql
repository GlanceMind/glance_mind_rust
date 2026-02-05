-- Restore constraint (not recommended - this constraint is incompatible with freeze logic)
-- Only use this if you need to rollback for testing purposes

ALTER TABLE gm_user_wallets 
ADD CONSTRAINT chk_balance_frozen CHECK (balance_points >= frozen_points);
