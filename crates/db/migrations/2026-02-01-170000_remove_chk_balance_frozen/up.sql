-- Remove incorrect constraint chk_balance_frozen
-- 
-- The constraint `balance_points >= frozen_points` conflicts with the wallet freeze logic.
-- 
-- Wallet design semantics:
--   - balance_points = available balance
--   - frozen_points = frozen amount
--   - total_assets = balance_points + frozen_points
--
-- The constraint assumed balance is total and frozen is a subset, which is incorrect.
-- Correct constraints are: balance_points >= 0 AND frozen_points >= 0 (already exist)

ALTER TABLE gm_user_wallets DROP CONSTRAINT IF EXISTS chk_balance_frozen;
