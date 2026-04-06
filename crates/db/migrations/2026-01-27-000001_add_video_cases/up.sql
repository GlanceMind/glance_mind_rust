-- Video cases table for storing AI-generated video case data
-- Used by: API (list/detail endpoints)

CREATE TABLE IF NOT EXISTS gm_data_video_cases (
    -- Primary key
    id SERIAL PRIMARY KEY,
    
    -- Task identification
    task_no VARCHAR(64) NOT NULL UNIQUE,
    case_id INTEGER,
    user_id VARCHAR(64),
    
    -- Category information
    tt_category_id VARCHAR(32),
    category_name_en VARCHAR(255),
    category_name_cn VARCHAR(255),
    
    -- Task configuration
    task_type VARCHAR(16),
    num INTEGER DEFAULT 1,
    status INTEGER DEFAULT 0,
    
    -- Content prompts
    script TEXT,
    selling_point TEXT,
    product_name VARCHAR(255),
    brand_name VARCHAR(255),
    video_language VARCHAR(32),
    
    -- Model configuration
    model VARCHAR(64),
    video_model VARCHAR(64),
    
    -- Media URLs
    video_url TEXT,
    ai_image_url TEXT,
    ai_prompt TEXT,
    refer_image_url TEXT,
    refer_video_url TEXT,
    
    -- JSONB fields for complex data
    image_urls JSONB DEFAULT '[]'::jsonb,
    characters JSONB DEFAULT '[]'::jsonb,
    videos JSONB DEFAULT '[]'::jsonb,
    
    -- Progress and status
    progress INTEGER DEFAULT 0,
    video_status VARCHAR(32) DEFAULT 'pending',
    error_message TEXT,
    size VARCHAR(32),
    case_status INTEGER DEFAULT 0,
    favorite_status INTEGER DEFAULT 0,
    completed_num INTEGER,
    
    -- Timestamps
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ
);

-- Indexes for common queries
CREATE INDEX idx_video_cases_task_no ON gm_data_video_cases(task_no);
CREATE INDEX idx_video_cases_user_id ON gm_data_video_cases(user_id);
CREATE INDEX idx_video_cases_case_id ON gm_data_video_cases(case_id);
CREATE INDEX idx_video_cases_status ON gm_data_video_cases(status);
CREATE INDEX idx_video_cases_video_status ON gm_data_video_cases(video_status);
CREATE INDEX idx_video_cases_created_at ON gm_data_video_cases(created_at DESC);

COMMENT ON TABLE gm_data_video_cases IS 'AI-generated video cases with scripts and references';
COMMENT ON COLUMN gm_data_video_cases.task_no IS 'Unique task number identifier';
COMMENT ON COLUMN gm_data_video_cases.videos IS 'Array of generated video objects with urls, status, etc';
COMMENT ON COLUMN gm_data_video_cases.image_urls IS 'Array of reference image URLs';
COMMENT ON COLUMN gm_data_video_cases.characters IS 'Array of character configurations';
