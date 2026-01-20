-- Create upload tasks table
CREATE TABLE gm_upload_tasks (
    id SERIAL PRIMARY KEY,
    user_id INTEGER NOT NULL REFERENCES gm_users(id) ON DELETE CASCADE,
    social_account_id INTEGER NOT NULL REFERENCES gm_social_accounts(id) ON DELETE CASCADE,
    task_type VARCHAR(50) NOT NULL DEFAULT 'upload',
    metadata JSONB NOT NULL,
    status VARCHAR(20) NOT NULL DEFAULT 'init',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ,
    
    CONSTRAINT valid_status CHECK (status IN ('init', 'processing', 'done', 'failed'))
);

-- Create indexes for common queries
CREATE INDEX idx_upload_tasks_user_id ON gm_upload_tasks(user_id);
CREATE INDEX idx_upload_tasks_social_account_id ON gm_upload_tasks(social_account_id);
CREATE INDEX idx_upload_tasks_status ON gm_upload_tasks(status);
CREATE INDEX idx_upload_tasks_created_at ON gm_upload_tasks(created_at DESC);
CREATE INDEX idx_upload_tasks_status_created ON gm_upload_tasks(status, created_at DESC);
