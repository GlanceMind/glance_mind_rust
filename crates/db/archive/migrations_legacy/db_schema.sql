-- Database Schema Summary
-- Generated to consolidate all migrations into a single source of truth.

-- 1. Functions & Triggers
CREATE OR REPLACE FUNCTION diesel_manage_updated_at(_tbl regclass) RETURNS VOID AS $$
BEGIN
    EXECUTE format('CREATE TRIGGER set_updated_at BEFORE UPDATE ON %s
                    FOR EACH ROW EXECUTE PROCEDURE diesel_set_updated_at()', _tbl);
END;
$$ LANGUAGE plpgsql;

CREATE OR REPLACE FUNCTION diesel_set_updated_at() RETURNS trigger AS $$
BEGIN
    IF (
        NEW IS DISTINCT FROM OLD AND
        NEW.updated_at IS NOT DISTINCT FROM OLD.updated_at
    ) THEN
        NEW.updated_at := current_timestamp;
    END IF;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- 2. Users Table
CREATE TABLE "users" (
    "id" SERIAL PRIMARY KEY,
    email VARCHAR(255) NOT NULL UNIQUE,
    password_hash VARCHAR(255) NOT NULL,
    invitation_code VARCHAR(50) UNIQUE,
    referred_by VARCHAR(50),
    company_name VARCHAR(255),
    api_key VARCHAR(255),
    status VARCHAR(50) NOT NULL DEFAULT 'PENDING_VERIFICATION',
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ
);

CREATE INDEX idx_users_email ON users(email);
CREATE INDEX idx_users_invitation_code ON users(invitation_code);
SELECT diesel_manage_updated_at('users');

-- 3. Config Tables (Platforms, Regions, AI Models, Pricing Rules)
CREATE TABLE platforms (
    id SERIAL PRIMARY KEY,
    name VARCHAR NOT NULL UNIQUE,
    display_name VARCHAR NOT NULL,
    is_active BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ
);
SELECT diesel_manage_updated_at('platforms');

CREATE TABLE regions (
    id SERIAL PRIMARY KEY,
    platform_id INTEGER NOT NULL REFERENCES platforms(id) ON DELETE CASCADE,
    code VARCHAR NOT NULL,
    display_name VARCHAR NOT NULL,
    is_active BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ,
    UNIQUE(platform_id, code)
);
CREATE INDEX idx_regions_platform_id ON regions(platform_id);
SELECT diesel_manage_updated_at('regions');

CREATE TABLE ai_models (
    id SERIAL PRIMARY KEY,
    name VARCHAR NOT NULL,
    provider VARCHAR NOT NULL,
    model_key VARCHAR NOT NULL,
    cost_multiplier DECIMAL(10,2) NOT NULL DEFAULT 1.0,
    is_active BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ
);
SELECT diesel_manage_updated_at('ai_models');

CREATE TABLE pricing_rules (
    id SERIAL PRIMARY KEY,
    action_type VARCHAR NOT NULL,
    platform_id INTEGER REFERENCES platforms(id) ON DELETE CASCADE,
    cost_points DECIMAL(10,2) NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ,
    UNIQUE(action_type, platform_id)
);
CREATE INDEX idx_pricing_rules_action_platform ON pricing_rules(action_type, platform_id);
SELECT diesel_manage_updated_at('pricing_rules');

-- 4. Social Accounts
CREATE TABLE social_accounts (
    id SERIAL PRIMARY KEY,
    user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    platform_id INTEGER NOT NULL REFERENCES platforms(id),
    username VARCHAR NOT NULL,
    credentials JSONB NOT NULL,
    proxy_url VARCHAR,
    status VARCHAR NOT NULL DEFAULT 'ACTIVE',
    health_score INT DEFAULT 100 CHECK (health_score >= 0 AND health_score <= 100),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ
);
CREATE INDEX idx_social_accounts_user_id ON social_accounts(user_id);
CREATE INDEX idx_social_accounts_platform_id ON social_accounts(platform_id);
SELECT diesel_manage_updated_at('social_accounts');

-- 5. Campaigns Table (Consolidated)
CREATE TABLE campaigns (
    id SERIAL PRIMARY KEY,
    user_id INTEGER NOT NULL REFERENCES users(id),
    name VARCHAR NOT NULL,
    status VARCHAR NOT NULL DEFAULT 'DRAFT',
    
    -- Config/Targeting
    platform_id INTEGER NOT NULL REFERENCES platforms(id),
    region_id INTEGER NOT NULL REFERENCES regions(id),
    ai_model_id INTEGER NOT NULL REFERENCES ai_models(id),
    
    target_audience TEXT,
    product_prompt TEXT NOT NULL DEFAULT '', -- Added from 20251231020000
    keyword TEXT, -- Added for crawler search keywords
    -- user_persona_prompt has been REMOVED
    
    schedule_config JSONB,
    enable_ai_refactor BOOLEAN DEFAULT false,
    persona_id INTEGER,
    max_scan_count INT DEFAULT 1000,
    budget_cap DECIMAL(10,2),
    end_date TIMESTAMPTZ,
    schedule_type VARCHAR NOT NULL,
    
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ
);
CREATE INDEX idx_campaigns_platform_id ON campaigns(platform_id);
CREATE INDEX idx_campaigns_region_id ON campaigns(region_id);
CREATE INDEX idx_campaigns_ai_model_id ON campaigns(ai_model_id);
CREATE INDEX idx_campaigns_status ON campaigns(status);
SELECT diesel_manage_updated_at('campaigns');

-- 6. Campaign Templates
CREATE TABLE campaign_templates (
    id SERIAL PRIMARY KEY,
    campaign_id INTEGER NOT NULL REFERENCES campaigns(id) ON DELETE CASCADE,
    template_content TEXT NOT NULL,
    tone_instruction TEXT,
    weight INT NOT NULL DEFAULT 10,
    
    reply_prompt TEXT, -- Added from 20251231020000
    forbidden_words_prompt TEXT, -- Added from 20251231020000
    
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ
);
SELECT diesel_manage_updated_at('campaign_templates');

-- 7. Campaign Accounts (Many-to-Many)
CREATE TABLE campaign_accounts (
    id SERIAL PRIMARY KEY,
    campaign_id INTEGER NOT NULL REFERENCES campaigns(id) ON DELETE CASCADE,
    account_id INTEGER NOT NULL REFERENCES social_accounts(id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ,
    UNIQUE(campaign_id, account_id)
);
CREATE INDEX idx_campaign_accounts_campaign_id ON campaign_accounts(campaign_id);
CREATE INDEX idx_campaign_accounts_account_id ON campaign_accounts(account_id);
SELECT diesel_manage_updated_at('campaign_accounts');

-- 8. Wallet Tables
CREATE TABLE user_wallets (
    user_id INTEGER PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
    balance_points DECIMAL(10,2) NOT NULL DEFAULT 0.00 CHECK (balance_points >= 0),
    frozen_points DECIMAL(10,2) NOT NULL DEFAULT 0.00 CHECK (frozen_points >= 0),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ
);
SELECT diesel_manage_updated_at('user_wallets');

CREATE TABLE wallet_transactions (
    id SERIAL PRIMARY KEY,
    user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    amount DECIMAL(10,2) NOT NULL,
    type VARCHAR NOT NULL,
    payment_method VARCHAR,
    external_txn_id VARCHAR,
    reference_id INTEGER,
    description TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ
);
CREATE INDEX idx_wallet_transactions_user_id ON wallet_transactions(user_id);
CREATE INDEX idx_wallet_transactions_type ON wallet_transactions(type);
CREATE INDEX idx_wallet_transactions_created_at ON wallet_transactions(created_at DESC);
SELECT diesel_manage_updated_at('wallet_transactions');

-- 9. Initial Seed Data
-- Platforms
INSERT INTO platforms (name, display_name) VALUES
    ('REDDIT', 'Reddit'),
    ('TIKTOK', 'TikTok'),
    ('FACEBOOK', 'Facebook');

-- Regions
INSERT INTO regions (platform_id, code, display_name)
SELECT p.id, r.code, r.display_name
FROM platforms p
CROSS JOIN (
    VALUES 
        ('GLOBAL', 'Global'),
        ('US', 'United States'),
        ('UK', 'United Kingdom'),
        ('JP', 'Japan'),
        ('CN', 'China'),
        ('EU', 'European Union')
) AS r(code, display_name);

-- AI Models
INSERT INTO ai_models (name, provider, model_key, cost_multiplier) VALUES
    ('GPT-4o', 'OPENAI', 'gpt-4o-2024-05-13', 2.0),
    ('GPT-3.5 Turbo', 'OPENAI', 'gpt-3.5-turbo', 0.5),
    ('Claude 3.5 Sonnet', 'ANTHROPIC', 'claude-3-5-sonnet-20241022', 1.5),
    ('Gemini 1.5 Pro', 'GEMINI', 'gemini-1.5-pro', 1.0);

-- Pricing Rules
INSERT INTO pricing_rules (action_type, platform_id, cost_points) VALUES
    ('SCAN_POST', NULL, 0.01),
    ('AI_ANALYZE', NULL, 0.05),
    ('GENERATE_REPLY', NULL, 0.10),
    ('POST_REPLY', NULL, 0.20);

-- 10. Diesel Migrations History
-- This table mimics the behavior of diesel CLI, marking all current migrations as executed.
CREATE TABLE IF NOT EXISTS __diesel_schema_migrations (
    version VARCHAR(50) PRIMARY KEY,
    run_on TIMESTAMP NOT NULL DEFAULT NOW()
);

INSERT INTO __diesel_schema_migrations (version) VALUES
    ('20251229120000'),
    ('20251229130000'),
    ('20251229140000'),
    ('20251229140001'),
    ('20251229140002'),
    ('20251230183123'), -- Note format difference in CLI output vs filename sometimes, sticking to what I saw in `migration list` but normalized
    ('20251231020000')
ON CONFLICT (version) DO NOTHING;

-- 11. Crawler Tasks & Results
CREATE TABLE crawler_tasks (
    id BIGINT PRIMARY KEY,
    campaign_id INTEGER NOT NULL REFERENCES campaigns(id) ON DELETE CASCADE,
    keywords TEXT[],
    max_count INTEGER NOT NULL,
    process_count INTEGER NOT NULL DEFAULT 0,
    status VARCHAR(50) NOT NULL DEFAULT 'init',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ
);
CREATE INDEX idx_crawler_tasks_campaign_id ON crawler_tasks(campaign_id);
CREATE INDEX idx_crawler_tasks_status ON crawler_tasks(status);
SELECT diesel_manage_updated_at('crawler_tasks');

CREATE TABLE crawler_results (
    id SERIAL PRIMARY KEY,
    task_id BIGINT NOT NULL REFERENCES crawler_tasks(id) ON DELETE CASCADE,
    video_id VARCHAR(255) NOT NULL,
    video_title TEXT,
    comment_count INTEGER,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ
);
CREATE INDEX idx_crawler_results_task_id ON crawler_results(task_id);
CREATE INDEX idx_crawler_results_video_id ON crawler_results(video_id);
SELECT diesel_manage_updated_at('crawler_results');
