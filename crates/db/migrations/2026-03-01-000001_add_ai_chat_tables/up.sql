-- AI Chat Mode tables

CREATE TABLE gm_ai_conversations (
    id SERIAL PRIMARY KEY,
    user_id INTEGER NOT NULL REFERENCES gm_users(id) ON DELETE CASCADE,
    title VARCHAR(255) NOT NULL DEFAULT '',
    status VARCHAR(20) NOT NULL DEFAULT 'active',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ
);

CREATE INDEX idx_ai_conversations_user_id ON gm_ai_conversations(user_id);
CREATE INDEX idx_ai_conversations_status ON gm_ai_conversations(user_id, status);

CREATE TABLE gm_ai_messages (
    id SERIAL PRIMARY KEY,
    conversation_id INTEGER NOT NULL REFERENCES gm_ai_conversations(id) ON DELETE CASCADE,
    role VARCHAR(20) NOT NULL,
    content TEXT NOT NULL DEFAULT '',
    tool_calls JSONB,
    tool_call_id VARCHAR(100),
    plan_id INTEGER,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_ai_messages_conversation_id ON gm_ai_messages(conversation_id);
CREATE INDEX idx_ai_messages_created_at ON gm_ai_messages(conversation_id, created_at);

CREATE TABLE gm_ai_plans (
    id SERIAL PRIMARY KEY,
    conversation_id INTEGER NOT NULL REFERENCES gm_ai_conversations(id) ON DELETE CASCADE,
    message_id INTEGER REFERENCES gm_ai_messages(id) ON DELETE SET NULL,
    user_id INTEGER NOT NULL REFERENCES gm_users(id) ON DELETE CASCADE,
    title VARCHAR(255) NOT NULL DEFAULT '',
    description TEXT NOT NULL DEFAULT '',
    status VARCHAR(20) NOT NULL DEFAULT 'draft',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ
);

CREATE INDEX idx_ai_plans_user_id ON gm_ai_plans(user_id);
CREATE INDEX idx_ai_plans_conversation_id ON gm_ai_plans(conversation_id);
CREATE INDEX idx_ai_plans_status ON gm_ai_plans(user_id, status);

CREATE TABLE gm_ai_plan_steps (
    id SERIAL PRIMARY KEY,
    plan_id INTEGER NOT NULL REFERENCES gm_ai_plans(id) ON DELETE CASCADE,
    step_order INTEGER NOT NULL DEFAULT 0,
    tool_name VARCHAR(100) NOT NULL,
    tool_params JSONB NOT NULL DEFAULT '{}',
    description TEXT NOT NULL DEFAULT '',
    status VARCHAR(20) NOT NULL DEFAULT 'pending',
    result JSONB,
    error_message TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ
);

CREATE INDEX idx_ai_plan_steps_plan_id ON gm_ai_plan_steps(plan_id);

CREATE TABLE gm_ai_tool_audit_logs (
    id SERIAL PRIMARY KEY,
    user_id INTEGER NOT NULL REFERENCES gm_users(id) ON DELETE CASCADE,
    conversation_id INTEGER REFERENCES gm_ai_conversations(id) ON DELETE SET NULL,
    tool_name VARCHAR(100) NOT NULL,
    safety_level VARCHAR(20) NOT NULL DEFAULT 'read_only',
    success BOOLEAN NOT NULL DEFAULT true,
    error_message TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_ai_tool_audit_user_id ON gm_ai_tool_audit_logs(user_id);
CREATE INDEX idx_ai_tool_audit_created_at ON gm_ai_tool_audit_logs(created_at);

SELECT diesel_manage_updated_at('gm_ai_conversations');
SELECT diesel_manage_updated_at('gm_ai_plans');
SELECT diesel_manage_updated_at('gm_ai_plan_steps');
