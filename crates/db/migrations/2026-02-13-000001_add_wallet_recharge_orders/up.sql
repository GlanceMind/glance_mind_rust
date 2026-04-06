-- Add payment-specific columns to gm_wallet_transactions for XunhuPay recharge flow.
-- Separates payment lifecycle status from the existing business reference_type.

ALTER TABLE gm_wallet_transactions
    ADD COLUMN IF NOT EXISTS payment_status VARCHAR(20),
    ADD COLUMN IF NOT EXISTS platform_txn_id VARCHAR(64),
    ADD COLUMN IF NOT EXISTS open_order_id VARCHAR(64),
    ADD COLUMN IF NOT EXISTS paid_at TIMESTAMPTZ;

COMMENT ON COLUMN gm_wallet_transactions.payment_status
    IS 'Payment lifecycle: PENDING, PAID, FAILED, REFUNDING, REFUNDED (only for RECHARGE type)';
COMMENT ON COLUMN gm_wallet_transactions.platform_txn_id
    IS 'Third-party platform transaction id (e.g. XunhuPay transaction_id)';
COMMENT ON COLUMN gm_wallet_transactions.open_order_id
    IS 'XunhuPay internal order id (open_order_id)';
COMMENT ON COLUMN gm_wallet_transactions.paid_at
    IS 'Actual payment completion timestamp';

-- Unique index on merchant order number to guarantee idempotent lookups
CREATE UNIQUE INDEX IF NOT EXISTS idx_wallet_txn_external_txn_id_unique
    ON gm_wallet_transactions (external_txn_id)
    WHERE external_txn_id IS NOT NULL;
