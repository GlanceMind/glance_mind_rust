-- Rollback: recreate table structure (data will be lost)
CREATE TABLE IF NOT EXISTS gm_ai_video_models (
    id SERIAL PRIMARY KEY,
    model_key VARCHAR(100) NOT NULL,
    model_name VARCHAR(255) NOT NULL,
    provider VARCHAR(100) NOT NULL,
    description TEXT,
    features JSONB,
    cost_per_generation DECIMAL(10, 2) NOT NULL,
    cost_per_upload DECIMAL(10, 2),
    api_endpoint VARCHAR(500),
    model_version VARCHAR(50),
    max_prompt_length INTEGER,
    supported_formats JSONB,
    max_image_size_mb INTEGER,
    estimated_time_minutes INTEGER,
    daily_limit INTEGER,
    is_active BOOLEAN NOT NULL DEFAULT true,
    is_default BOOLEAN NOT NULL DEFAULT false,
    sort_order INTEGER NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ
);
