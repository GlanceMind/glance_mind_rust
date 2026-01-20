-- Add missing tables and columns to sync with local development database

-- 1. Add like_count column to gm_crawler_results
ALTER TABLE gm_crawler_results ADD COLUMN IF NOT EXISTS like_count INTEGER;
COMMENT ON COLUMN gm_crawler_results.like_count IS 'Video like count';

-- 2. Create gm_admin_users table
CREATE TABLE IF NOT EXISTS gm_admin_users (
    id SERIAL PRIMARY KEY,
    username VARCHAR(50) NOT NULL UNIQUE,
    email VARCHAR(255) NOT NULL UNIQUE,
    password_hash VARCHAR(255) NOT NULL,
    full_name VARCHAR(255) NOT NULL,
    role VARCHAR(20) NOT NULL DEFAULT 'admin',
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    last_login_at TIMESTAMP WITH TIME ZONE,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE
);

-- 3. Create gm_ai_video_models table
CREATE TABLE IF NOT EXISTS gm_ai_video_models (
    id SERIAL PRIMARY KEY,
    model_key VARCHAR(100) NOT NULL UNIQUE,
    model_name VARCHAR(255) NOT NULL,
    provider VARCHAR(100) NOT NULL,
    description TEXT,
    features JSONB,
    cost_per_generation DECIMAL(10,4) NOT NULL DEFAULT 0,
    cost_per_upload DECIMAL(10,4),
    api_endpoint VARCHAR(500),
    model_version VARCHAR(50),
    max_prompt_length INTEGER,
    supported_formats JSONB,
    max_image_size_mb INTEGER,
    estimated_time_minutes INTEGER,
    daily_limit INTEGER,
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    is_default BOOLEAN NOT NULL DEFAULT FALSE,
    sort_order INTEGER NOT NULL DEFAULT 0,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE
);

-- 4. Create gm_agent_facebook_posts table
CREATE TABLE IF NOT EXISTS gm_agent_facebook_posts (
    id SERIAL PRIMARY KEY,
    task_id INTEGER NOT NULL REFERENCES gm_crawler_tasks(id) ON DELETE CASCADE,
    campaign_id INTEGER REFERENCES gm_campaigns(id) ON DELETE CASCADE,
    facebook_post_id VARCHAR(255) NOT NULL,
    post_type VARCHAR(50) DEFAULT 'post',
    url TEXT,
    message TEXT DEFAULT '',
    message_rich TEXT,
    "timestamp" BIGINT,
    posted_at TIMESTAMP WITH TIME ZONE,
    reactions_count INTEGER DEFAULT 0,
    comments_count INTEGER DEFAULT 0,
    reshare_count INTEGER DEFAULT 0,
    reactions_like INTEGER DEFAULT 0,
    reactions_love INTEGER DEFAULT 0,
    reactions_haha INTEGER DEFAULT 0,
    reactions_wow INTEGER DEFAULT 0,
    reactions_sad INTEGER DEFAULT 0,
    reactions_angry INTEGER DEFAULT 0,
    author_name VARCHAR(255),
    author_id VARCHAR(255),
    author_url TEXT,
    author_vanity VARCHAR(255),
    author_profile_picture TEXT,
    is_video BOOLEAN DEFAULT FALSE,
    video_url TEXT,
    video_thumbnail TEXT,
    video_duration INTEGER,
    video_views_count INTEGER,
    images JSONB,
    link_url TEXT,
    link_title VARCHAR(500),
    link_description TEXT,
    location_name VARCHAR(255),
    location_city VARCHAR(255),
    location_country VARCHAR(255),
    is_sponsored BOOLEAN DEFAULT FALSE,
    processed BOOLEAN DEFAULT FALSE,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE
);

CREATE INDEX IF NOT EXISTS idx_facebook_posts_task_id ON gm_agent_facebook_posts(task_id);
CREATE INDEX IF NOT EXISTS idx_facebook_posts_facebook_post_id ON gm_agent_facebook_posts(facebook_post_id);

-- 5. Create gm_agent_facebook_comments table
CREATE TABLE IF NOT EXISTS gm_agent_facebook_comments (
    id SERIAL PRIMARY KEY,
    post_db_id INTEGER NOT NULL REFERENCES gm_agent_facebook_posts(id) ON DELETE CASCADE,
    campaign_id INTEGER REFERENCES gm_campaigns(id) ON DELETE CASCADE,
    facebook_comment_id VARCHAR(255) NOT NULL,
    parent_comment_id VARCHAR(255),
    comment_url TEXT,
    comment_text TEXT NOT NULL,
    reason TEXT,
    suggested_reply TEXT,
    suggested_dm TEXT,
    suggested_reply_post TEXT,
    status VARCHAR(50) DEFAULT 'PENDING',
    comment_user_id VARCHAR(255),
    comment_username VARCHAR(255),
    comment_user_url TEXT,
    comment_user_profile_picture TEXT,
    like_count INTEGER DEFAULT 0,
    reply_count INTEGER DEFAULT 0,
    threading_depth INTEGER DEFAULT 0,
    created_at_ts BIGINT,
    comment_created_at TIMESTAMP WITH TIME ZONE,
    facebook_post_id VARCHAR(255),
    post_url TEXT,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE
);

CREATE INDEX IF NOT EXISTS idx_facebook_comments_post_db_id ON gm_agent_facebook_comments(post_db_id);
CREATE INDEX IF NOT EXISTS idx_facebook_comments_facebook_comment_id ON gm_agent_facebook_comments(facebook_comment_id);
