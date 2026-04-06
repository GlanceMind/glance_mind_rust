-- User materials table for storing user-uploaded or favorited materials
-- Used by: Materials management feature

CREATE TABLE IF NOT EXISTS gm_user_materials (
    id SERIAL PRIMARY KEY,
    user_id INTEGER NOT NULL REFERENCES gm_users(id) ON DELETE CASCADE,
    
    -- Material content
    video_url TEXT NOT NULL,
    prompt TEXT, -- AI-generated prompt from video analysis
    thumbnail_url TEXT,
    
    -- Tag system (for categorizing materials, not distinguishing source)
    tag VARCHAR(100), -- Single tag, selected from video_cases categories (frontend hardcode)
    
    -- Metadata
    title VARCHAR(255),
    description TEXT,
    duration INTEGER, -- Video duration in seconds
    file_size BIGINT, -- File size in bytes
    
    -- Status
    is_active BOOLEAN DEFAULT true,
    
    -- Timestamps
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ,
    
    -- Foreign key constraint
    CONSTRAINT fk_user FOREIGN KEY (user_id) REFERENCES gm_users(id)
);

-- Indexes for common queries
CREATE INDEX idx_user_materials_user_id ON gm_user_materials(user_id);
CREATE INDEX idx_user_materials_tag ON gm_user_materials(tag);
CREATE INDEX idx_user_materials_created_at ON gm_user_materials(created_at DESC);

COMMENT ON TABLE gm_user_materials IS 'User materials (uploaded or favorited from video_cases)';
COMMENT ON COLUMN gm_user_materials.video_url IS 'Video URL (from OSS upload or video_cases)';
COMMENT ON COLUMN gm_user_materials.prompt IS 'AI-generated prompt from video analysis';
COMMENT ON COLUMN gm_user_materials.tag IS 'Single tag for categorization, collected from video_cases categories';
