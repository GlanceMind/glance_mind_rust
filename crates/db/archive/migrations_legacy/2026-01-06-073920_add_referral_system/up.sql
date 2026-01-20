-- Add invite_code and invited_by to users table
ALTER TABLE gm_users 
ADD COLUMN invite_code VARCHAR(36) UNIQUE,
ADD COLUMN invited_by VARCHAR(36);

CREATE INDEX idx_users_invite_code ON gm_users(invite_code);
CREATE INDEX idx_users_invited_by ON gm_users(invited_by);

-- Create referrals table for tracking invitation relationships
CREATE TABLE gm_referrals (
    id SERIAL PRIMARY KEY,
    referrer_id INTEGER NOT NULL REFERENCES gm_users(id) ON DELETE CASCADE,
    referee_id INTEGER NOT NULL REFERENCES gm_users(id) ON DELETE CASCADE,
    commission_rate DECIMAL(5,2) NOT NULL DEFAULT 10.00,
    total_earned DECIMAL(10,2) NOT NULL DEFAULT 0.00,
    status VARCHAR(20) NOT NULL DEFAULT 'ACTIVE',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ,
    CONSTRAINT uq_referee UNIQUE(referee_id)
);

CREATE INDEX idx_referrals_referrer ON gm_referrals(referrer_id);
CREATE INDEX idx_referrals_status ON gm_referrals(status);

-- Trigger for updated_at
CREATE OR REPLACE FUNCTION update_referrals_updated_at()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER referrals_updated_at_trigger
BEFORE UPDATE ON gm_referrals
FOR EACH ROW
EXECUTE FUNCTION update_referrals_updated_at();

-- Create referral_earnings table for tracking commission payouts
CREATE TABLE gm_referral_earnings (
    id SERIAL PRIMARY KEY,
    referral_id INTEGER NOT NULL REFERENCES gm_referrals(id) ON DELETE CASCADE,
    transaction_id INTEGER,  -- Reference to transaction, no FK for now
    amount DECIMAL(10,2) NOT NULL,
    description TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_referral_earnings_referral ON gm_referral_earnings(referral_id);
CREATE INDEX idx_referral_earnings_transaction ON gm_referral_earnings(transaction_id);
