DROP INDEX IF EXISTS idx_wallet_txn_external_txn_id_unique;

ALTER TABLE gm_wallet_transactions
    DROP COLUMN IF EXISTS payment_status,
    DROP COLUMN IF EXISTS platform_txn_id,
    DROP COLUMN IF EXISTS open_order_id,
    DROP COLUMN IF EXISTS paid_at;
