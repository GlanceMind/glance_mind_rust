-- Remove bitmask column
ALTER TABLE gm_users DROP COLUMN IF EXISTS permissions;

-- Recreate the old permissions table
CREATE TABLE gm_user_permissions (
    id SERIAL PRIMARY KEY,
    user_id INTEGER NOT NULL REFERENCES gm_users(id) ON DELETE CASCADE,
    feature_key VARCHAR(50) NOT NULL,
    is_enabled BOOLEAN NOT NULL DEFAULT false,
    granted_by VARCHAR(20) NOT NULL DEFAULT 'system',
    expires_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ,
    CONSTRAINT uq_user_feature UNIQUE (user_id, feature_key)
);
CREATE INDEX idx_user_permissions_user_id ON gm_user_permissions(user_id);
CREATE INDEX idx_user_permissions_feature ON gm_user_permissions(feature_key);
