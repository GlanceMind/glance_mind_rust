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

-- Historical user migration: insert default permissions for all existing users
-- dm_control is disabled by default, others are enabled
INSERT INTO gm_user_permissions (user_id, feature_key, is_enabled, granted_by)
SELECT id, 'dm_control', false, 'system' FROM gm_users
ON CONFLICT DO NOTHING;

INSERT INTO gm_user_permissions (user_id, feature_key, is_enabled, granted_by)
SELECT id, 'ai_publish', true, 'system' FROM gm_users
ON CONFLICT DO NOTHING;

INSERT INTO gm_user_permissions (user_id, feature_key, is_enabled, granted_by)
SELECT id, 'ai_content_gen', true, 'system' FROM gm_users
ON CONFLICT DO NOTHING;

INSERT INTO gm_user_permissions (user_id, feature_key, is_enabled, granted_by)
SELECT id, 'ai_lead_gen', true, 'system' FROM gm_users
ON CONFLICT DO NOTHING;
