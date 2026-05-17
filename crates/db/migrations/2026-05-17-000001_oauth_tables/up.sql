-- OAuth tables migration
-- Adds: oauth_codes, oauth_refresh_tokens, oauth_audit_log, ota_config

CREATE TABLE oauth_codes (
    code TEXT PRIMARY KEY,
    user_id BIGINT NOT NULL REFERENCES gm_users(id) ON DELETE CASCADE,
    client_id TEXT NOT NULL,
    redirect_uri TEXT NOT NULL,
    code_challenge TEXT NOT NULL,
    code_challenge_method TEXT NOT NULL CHECK (code_challenge_method = 'S256'),
    scope TEXT NOT NULL,
    state TEXT NOT NULL,
    expires_at TIMESTAMPTZ NOT NULL,
    redeemed_at TIMESTAMPTZ
);
CREATE INDEX idx_oauth_codes_expires ON oauth_codes(expires_at);

CREATE TABLE oauth_refresh_tokens (
    id BIGSERIAL PRIMARY KEY,
    token_hash TEXT NOT NULL UNIQUE,
    family_id UUID NOT NULL,
    user_id BIGINT NOT NULL REFERENCES gm_users(id) ON DELETE CASCADE,
    client_id TEXT NOT NULL,
    scope TEXT NOT NULL,
    issued_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    expires_at TIMESTAMPTZ NOT NULL,
    revoked_at TIMESTAMPTZ,
    replaced_by_id BIGINT REFERENCES oauth_refresh_tokens(id) ON DELETE SET NULL
);
CREATE INDEX idx_oauth_refresh_family ON oauth_refresh_tokens(family_id);
CREATE INDEX idx_oauth_refresh_user ON oauth_refresh_tokens(user_id);
CREATE INDEX idx_oauth_refresh_expires ON oauth_refresh_tokens(expires_at);

CREATE TABLE oauth_audit_log (
    id BIGSERIAL PRIMARY KEY,
    event_type TEXT NOT NULL,
    client_id TEXT NOT NULL,
    user_id BIGINT,
    ip TEXT,
    user_agent TEXT,
    metadata JSONB,
    occurred_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX idx_oauth_audit_event ON oauth_audit_log(event_type, occurred_at DESC);

CREATE TABLE ota_config (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_by TEXT NOT NULL
);
INSERT INTO ota_config (key, value, updated_by) VALUES ('oauth.enabled', 'false', 'migration');
