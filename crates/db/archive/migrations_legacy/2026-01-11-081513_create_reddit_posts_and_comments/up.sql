-- Create gm_agent_reddit_posts table
-- Stores Reddit post metadata (similar to gm_agent_videos for TikTok)
CREATE TABLE gm_agent_reddit_posts (
    id SERIAL PRIMARY KEY,
    task_id INTEGER NOT NULL REFERENCES gm_crawler_tasks(id) ON DELETE CASCADE,
    campaign_id INTEGER REFERENCES gm_campaigns(id) ON DELETE CASCADE,
    
    -- Reddit post identifiers
    post_id VARCHAR(255) NOT NULL,           -- Short ID (e.g., "1pqjxcv")
    post_name VARCHAR(255) NOT NULL,         -- Full ID (e.g., "t3_1pqjxcv")
    
    -- Post content
    title TEXT NOT NULL,
    selftext TEXT DEFAULT '',
    
    -- Author info
    author VARCHAR(255),
    
    -- Subreddit info
    subreddit VARCHAR(255) NOT NULL,
    
    -- Post metadata
    url TEXT,
    permalink TEXT,
    domain VARCHAR(255),
    thumbnail TEXT,
    
    -- Engagement metrics
    score INTEGER DEFAULT 0,
    upvote_ratio NUMERIC(5,4) DEFAULT 0.0,
    num_comments INTEGER DEFAULT 0,
    
    -- Flags
    is_video BOOLEAN DEFAULT FALSE,
    
    -- Timestamps
    post_created_at TIMESTAMP WITH TIME ZONE,  -- Reddit post creation time
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW() NOT NULL,
    
    -- Ensure uniqueness per task
    UNIQUE(task_id, post_id)
);

-- Create gm_agent_reddit_comments table
-- Stores AI-analyzed Reddit comments (similar to gm_agent_comments for TikTok)
CREATE TABLE gm_agent_reddit_comments (
    id SERIAL PRIMARY KEY,
    post_db_id INTEGER NOT NULL REFERENCES gm_agent_reddit_posts(id) ON DELETE CASCADE,
    campaign_id INTEGER REFERENCES gm_campaigns(id) ON DELETE CASCADE,
    
    -- Reddit comment identifiers
    comment_id VARCHAR(255) NOT NULL,        -- Short ID (e.g., "nuupeu5")
    comment_name VARCHAR(255) NOT NULL,      -- Full ID (e.g., "t1_nuupeu5")
    
    -- Author info
    author VARCHAR(255),
    
    -- Comment content
    body TEXT,
    
    -- AI analysis results
    reason TEXT,                              -- Why this comment was identified
    suggested_reply TEXT,                     -- AI-generated reply
    status VARCHAR(50) DEFAULT 'PENDING',    -- PENDING, APPROVED, REJECTED, POSTED
    
    -- Comment metadata
    score INTEGER DEFAULT 0,
    parent_id VARCHAR(255),                  -- Parent comment/post ID
    is_reply BOOLEAN DEFAULT FALSE,          -- Is this a reply to another comment
    depth INTEGER DEFAULT 0,                 -- Nesting depth (0 = top-level)
    
    -- Timestamps
    comment_created_at TIMESTAMP WITH TIME ZONE,  -- Reddit comment creation time
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW() NOT NULL
);

-- Create indexes for better query performance
CREATE INDEX idx_reddit_posts_task_id ON gm_agent_reddit_posts(task_id);
CREATE INDEX idx_reddit_posts_campaign_id ON gm_agent_reddit_posts(campaign_id);
CREATE INDEX idx_reddit_posts_post_id ON gm_agent_reddit_posts(post_id);
CREATE INDEX idx_reddit_posts_subreddit ON gm_agent_reddit_posts(subreddit);
CREATE INDEX idx_reddit_posts_created_at ON gm_agent_reddit_posts(created_at DESC);

CREATE INDEX idx_reddit_comments_post_db_id ON gm_agent_reddit_comments(post_db_id);
CREATE INDEX idx_reddit_comments_campaign_id ON gm_agent_reddit_comments(campaign_id);
CREATE INDEX idx_reddit_comments_comment_id ON gm_agent_reddit_comments(comment_id);
CREATE INDEX idx_reddit_comments_status ON gm_agent_reddit_comments(status);
CREATE INDEX idx_reddit_comments_created_at ON gm_agent_reddit_comments(created_at DESC);

-- Add triggers for auto-updating timestamps
CREATE TRIGGER set_updated_at_reddit_posts
    BEFORE UPDATE ON gm_agent_reddit_posts
    FOR EACH ROW
    EXECUTE FUNCTION diesel_set_updated_at();

CREATE TRIGGER set_updated_at_reddit_comments
    BEFORE UPDATE ON gm_agent_reddit_comments
    FOR EACH ROW
    EXECUTE FUNCTION diesel_set_updated_at();

-- Add updated_at columns
ALTER TABLE gm_agent_reddit_posts ADD COLUMN updated_at TIMESTAMP WITH TIME ZONE;
ALTER TABLE gm_agent_reddit_comments ADD COLUMN updated_at TIMESTAMP WITH TIME ZONE;
