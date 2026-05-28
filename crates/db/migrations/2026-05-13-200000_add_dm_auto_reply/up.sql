-- DM AI customer service: auto-reply config, reply log, product FAQ,
-- and per-conversation human-review state.
--
-- NOTE: dm_conversations lives in NATS JetStream KV (not PostgreSQL);
-- review flags are persisted in a new `gm_dm_conversation_review` table
-- keyed by the NATS conv_id string.

CREATE TABLE gm_auto_reply_config (
    id SERIAL PRIMARY KEY,
    social_account_id INTEGER NOT NULL UNIQUE
        REFERENCES gm_social_accounts(id) ON DELETE CASCADE,
    enabled BOOLEAN NOT NULL DEFAULT FALSE,
    blacklist_keywords TEXT[] NOT NULL DEFAULT '{}',
    rate_limit_per_day INTEGER NOT NULL DEFAULT 5,
    confidence_threshold REAL NOT NULL DEFAULT 0.7,
    rag_threshold REAL NOT NULL DEFAULT 0.6,
    fallback_strategy VARCHAR(32) NOT NULL DEFAULT 'escalate',
    brand_name VARCHAR(128) NOT NULL DEFAULT '',
    last_modified_by INTEGER REFERENCES gm_users(id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_gm_auto_reply_config_enabled
    ON gm_auto_reply_config(social_account_id) WHERE enabled = TRUE;

CREATE TABLE gm_dm_reply_log (
    id BIGSERIAL PRIMARY KEY,
    inbound_msg_id VARCHAR(128) NOT NULL UNIQUE,
    conv_id VARCHAR(128) NOT NULL,
    social_account_id INTEGER NOT NULL REFERENCES gm_social_accounts(id),
    campaign_id INTEGER REFERENCES gm_campaigns(id) ON DELETE SET NULL,
    user_ref VARCHAR(128) NOT NULL,
    user_handle VARCHAR(128),
    platform VARCHAR(32) NOT NULL,
    inbound_text TEXT NOT NULL,
    inbound_received_at TIMESTAMPTZ NOT NULL,
    status VARCHAR(16) NOT NULL CHECK (status IN
        ('skipped','pending','sent','escalated','failed')),
    skip_reason VARCHAR(64),
    rag_score REAL,
    llm_confidence REAL,
    llm_model VARCHAR(64),
    latency_ms INTEGER NOT NULL DEFAULT 0,
    reply_text TEXT,
    escalate_reason VARCHAR(128),
    resolved_at TIMESTAMPTZ,
    resolved_by INTEGER REFERENCES gm_users(id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_gm_dm_reply_log_conv ON gm_dm_reply_log(conv_id);
CREATE INDEX idx_gm_dm_reply_log_campaign_status
    ON gm_dm_reply_log(campaign_id, status, created_at DESC);
CREATE INDEX idx_gm_dm_reply_log_status_created
    ON gm_dm_reply_log(status, created_at DESC);

CREATE TABLE gm_product_faq (
    id SERIAL PRIMARY KEY,
    campaign_id INTEGER NOT NULL REFERENCES gm_campaigns(id) ON DELETE CASCADE,
    question TEXT NOT NULL,
    answer TEXT NOT NULL,
    chunk_type VARCHAR(32) NOT NULL DEFAULT 'faq',
    dify_kb_id VARCHAR(128),
    dify_doc_id VARCHAR(128),
    sync_status VARCHAR(16) NOT NULL DEFAULT 'pending'
        CHECK (sync_status IN ('pending','synced','failed')),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_gm_product_faq_campaign ON gm_product_faq(campaign_id);

CREATE TABLE gm_dm_conversation_review (
    conv_id VARCHAR(128) PRIMARY KEY,
    needs_human_review BOOLEAN NOT NULL DEFAULT FALSE,
    human_review_reason VARCHAR(128),
    resolved_at TIMESTAMPTZ,
    resolved_by INTEGER REFERENCES gm_users(id),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_gm_dm_conversation_review_pending
    ON gm_dm_conversation_review(updated_at DESC)
    WHERE needs_human_review = TRUE;
