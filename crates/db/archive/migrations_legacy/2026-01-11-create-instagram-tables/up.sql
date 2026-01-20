-- Instagram Posts and Comments Tables
-- Migration: Create Instagram posts and comments tables

-- Create Instagram posts table (includes both posts and reels)
CREATE TABLE gm_agent_instagram_posts (
    id SERIAL PRIMARY KEY,
    task_id INTEGER NOT NULL REFERENCES gm_crawler_tasks(id) ON DELETE CASCADE,
    campaign_id INTEGER REFERENCES gm_campaigns(id) ON DELETE CASCADE,
    
    -- Instagram post identifiers
    code VARCHAR(255) NOT NULL UNIQUE,           -- Post shortcode (e.g., "CxYz123")
    instagram_id VARCHAR(255),                   -- Instagram internal ID
    
    -- Post type
    media_type INTEGER DEFAULT 1,                -- 1=photo, 2=video (from API)
    product_type VARCHAR(50),                    -- feed, igtv, clips (reels), etc.
    
    -- Content
    caption_text TEXT DEFAULT '',                -- Post caption
    
    -- Author info
    owner_username VARCHAR(255),
    owner_id VARCHAR(255),
    owner_full_name VARCHAR(255),
    
    -- Media URLs
    media_url TEXT,                              -- Primary media URL
    thumbnail_url TEXT,                          -- Thumbnail URL
    
    -- Engagement metrics
    like_count INTEGER DEFAULT 0,
    comment_count INTEGER DEFAULT 0,
    play_count INTEGER DEFAULT 0,                -- For videos/reels
    
    -- Timestamps
    taken_at_ts BIGINT,                          -- Unix timestamp from API
    posted_at TIMESTAMP WITH TIME ZONE,          -- Converted datetime
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW() NOT NULL,
    updated_at TIMESTAMP WITH TIME ZONE
);

-- Create Instagram comments table
CREATE TABLE gm_agent_instagram_comments (
    id SERIAL PRIMARY KEY,
    post_db_id INTEGER NOT NULL REFERENCES gm_agent_instagram_posts(id) ON DELETE CASCADE,
    campaign_id INTEGER REFERENCES gm_campaigns(id) ON DELETE CASCADE,
    
    -- Comment identifiers
    instagram_comment_id VARCHAR(255) NOT NULL,  -- Instagram comment ID
    parent_comment_id VARCHAR(255),              -- For threaded replies
    
    -- Comment content
    comment_text TEXT NOT NULL,
    
    -- AI analysis results
    reason TEXT,                                 -- AI reason for selecting this comment
    suggested_reply TEXT,                        -- AI suggested reply
    status VARCHAR(50) DEFAULT 'PENDING',        -- PENDING, APPROVED, REJECTED, POSTED
    
    -- Author info
    comment_user_id VARCHAR(255),
    comment_username VARCHAR(255),
    comment_user_full_name VARCHAR(255),
    
    -- Engagement metrics
    like_count INTEGER DEFAULT 0,
    comment_like_count INTEGER DEFAULT 0,
    child_comment_count INTEGER DEFAULT 0,       -- Number of replies
    
    -- Timestamps
    created_at_ts BIGINT,                        -- Unix timestamp from API
    comment_created_at TIMESTAMP WITH TIME ZONE,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW() NOT NULL,
    updated_at TIMESTAMP WITH TIME ZONE,
    
    -- Constraints
    UNIQUE(instagram_comment_id, post_db_id)
);

-- Create indexes
CREATE INDEX idx_instagram_posts_task_id ON gm_agent_instagram_posts(task_id);
CREATE INDEX idx_instagram_posts_campaign_id ON gm_agent_instagram_posts(campaign_id);
CREATE INDEX idx_instagram_posts_code ON gm_agent_instagram_posts(code);
CREATE INDEX idx_instagram_posts_owner_username ON gm_agent_instagram_posts(owner_username);
CREATE INDEX idx_instagram_posts_media_type ON gm_agent_instagram_posts(media_type);
CREATE INDEX idx_instagram_posts_posted_at ON gm_agent_instagram_posts(posted_at);

CREATE INDEX idx_instagram_comments_post_db_id ON gm_agent_instagram_comments(post_db_id);
CREATE INDEX idx_instagram_comments_campaign_id ON gm_agent_instagram_comments(campaign_id);
CREATE INDEX idx_instagram_comments_instagram_comment_id ON gm_agent_instagram_comments(instagram_comment_id);
CREATE INDEX idx_instagram_comments_comment_username ON gm_agent_instagram_comments(comment_username);
CREATE INDEX idx_instagram_comments_status ON gm_agent_instagram_comments(status);
CREATE INDEX idx_instagram_comments_parent_id ON gm_agent_instagram_comments(parent_comment_id);

-- Create triggers for updated_at
CREATE OR REPLACE FUNCTION update_instagram_posts_updated_at()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trigger_update_instagram_posts_updated_at
    BEFORE UPDATE ON gm_agent_instagram_posts
    FOR EACH ROW
    EXECUTE FUNCTION update_instagram_posts_updated_at();

CREATE OR REPLACE FUNCTION update_instagram_comments_updated_at()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trigger_update_instagram_comments_updated_at
    BEFORE UPDATE ON gm_agent_instagram_comments
    FOR EACH ROW
    EXECUTE FUNCTION update_instagram_comments_updated_at();
