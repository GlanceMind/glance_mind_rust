-- Facebook Posts Table
-- Stores Facebook post metadata fetched via Apify API
CREATE TABLE IF NOT EXISTS public.gm_agent_facebook_posts (
    id SERIAL PRIMARY KEY,
    task_id INTEGER NOT NULL,
    campaign_id INTEGER,
    
    -- Facebook Post Identifiers
    facebook_post_id VARCHAR(255) NOT NULL,
    post_type VARCHAR(50) DEFAULT 'post',
    url TEXT,
    
    -- Post Content
    message TEXT DEFAULT '',
    message_rich TEXT,
    
    -- Timestamps
    timestamp BIGINT,
    posted_at TIMESTAMP WITH TIME ZONE,
    
    -- Engagement Metrics
    reactions_count INTEGER DEFAULT 0,
    comments_count INTEGER DEFAULT 0,
    reshare_count INTEGER DEFAULT 0,
    
    -- Reactions Breakdown
    reactions_like INTEGER DEFAULT 0,
    reactions_love INTEGER DEFAULT 0,
    reactions_haha INTEGER DEFAULT 0,
    reactions_wow INTEGER DEFAULT 0,
    reactions_sad INTEGER DEFAULT 0,
    reactions_angry INTEGER DEFAULT 0,
    reactions_care INTEGER DEFAULT 0,
    
    -- Author Info
    author_id VARCHAR(255),
    author_name VARCHAR(255),
    author_url TEXT,
    author_profile_picture_url TEXT,
    author_title VARCHAR(255),
    
    -- Media Info
    has_image BOOLEAN DEFAULT false,
    image_url TEXT,
    image_width INTEGER,
    image_height INTEGER,
    image_id VARCHAR(255),
    
    has_video BOOLEAN DEFAULT false,
    video_thumbnail TEXT,
    
    -- External/Attached Content
    external_url TEXT,
    attached_post_url TEXT,
    
    -- IDs for fetching comments/shares
    comments_id VARCHAR(255),
    shares_id VARCHAR(255),
    
    -- System Timestamps
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW() NOT NULL,
    updated_at TIMESTAMP WITH TIME ZONE,
    
    -- Constraints
    CONSTRAINT gm_agent_facebook_posts_task_id_fkey 
        FOREIGN KEY (task_id) REFERENCES public.gm_crawler_tasks(id) ON DELETE CASCADE,
    CONSTRAINT gm_agent_facebook_posts_campaign_id_fkey 
        FOREIGN KEY (campaign_id) REFERENCES public.gm_campaigns(id) ON DELETE CASCADE,
    CONSTRAINT gm_agent_facebook_posts_task_id_post_id_key 
        UNIQUE (task_id, facebook_post_id)
);

-- Facebook Comments Table
-- Stores Facebook comments with AI analysis results
CREATE TABLE IF NOT EXISTS public.gm_agent_facebook_comments (
    id SERIAL PRIMARY KEY,
    post_db_id INTEGER NOT NULL,
    campaign_id INTEGER,
    
    -- Facebook Comment Identifiers
    facebook_comment_id VARCHAR(255) NOT NULL,
    parent_comment_id VARCHAR(255),
    comment_url TEXT,
    
    -- Comment Content
    comment_text TEXT NOT NULL,
    
    -- AI Analysis Results
    reason TEXT,
    suggested_reply TEXT,
    suggested_dm TEXT,
    suggested_reply_post TEXT,
    status VARCHAR(50) DEFAULT 'PENDING',
    
    -- Comment User Info
    comment_user_id VARCHAR(255),
    comment_username VARCHAR(255),
    comment_user_url TEXT,
    comment_user_profile_picture TEXT,
    
    -- Engagement
    like_count INTEGER DEFAULT 0,
    reply_count INTEGER DEFAULT 0,
    threading_depth INTEGER DEFAULT 0,
    
    -- Timestamps
    created_at_ts BIGINT,
    comment_created_at TIMESTAMP WITH TIME ZONE,
    
    -- Post Context
    facebook_post_id VARCHAR(255),
    post_url TEXT,
    
    -- System Timestamps
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW() NOT NULL,
    updated_at TIMESTAMP WITH TIME ZONE,
    
    -- Constraints
    CONSTRAINT gm_agent_facebook_comments_post_db_id_fkey 
        FOREIGN KEY (post_db_id) REFERENCES public.gm_agent_facebook_posts(id) ON DELETE CASCADE,
    CONSTRAINT gm_agent_facebook_comments_campaign_id_fkey 
        FOREIGN KEY (campaign_id) REFERENCES public.gm_campaigns(id) ON DELETE CASCADE,
    CONSTRAINT gm_agent_facebook_comments_comment_id_post_db_id_key 
        UNIQUE (facebook_comment_id, post_db_id)
);

-- Indexes for Facebook Posts
CREATE INDEX IF NOT EXISTS idx_facebook_posts_task_id ON public.gm_agent_facebook_posts(task_id);
CREATE INDEX IF NOT EXISTS idx_facebook_posts_campaign_id ON public.gm_agent_facebook_posts(campaign_id);
CREATE INDEX IF NOT EXISTS idx_facebook_posts_facebook_post_id ON public.gm_agent_facebook_posts(facebook_post_id);
CREATE INDEX IF NOT EXISTS idx_facebook_posts_author_id ON public.gm_agent_facebook_posts(author_id);
CREATE INDEX IF NOT EXISTS idx_facebook_posts_posted_at ON public.gm_agent_facebook_posts(posted_at);
CREATE INDEX IF NOT EXISTS idx_facebook_posts_created_at ON public.gm_agent_facebook_posts(created_at DESC);

-- Indexes for Facebook Comments
CREATE INDEX IF NOT EXISTS idx_facebook_comments_post_db_id ON public.gm_agent_facebook_comments(post_db_id);
CREATE INDEX IF NOT EXISTS idx_facebook_comments_campaign_id ON public.gm_agent_facebook_comments(campaign_id);
CREATE INDEX IF NOT EXISTS idx_facebook_comments_facebook_comment_id ON public.gm_agent_facebook_comments(facebook_comment_id);
CREATE INDEX IF NOT EXISTS idx_facebook_comments_status ON public.gm_agent_facebook_comments(status);
CREATE INDEX IF NOT EXISTS idx_facebook_comments_comment_username ON public.gm_agent_facebook_comments(comment_username);
CREATE INDEX IF NOT EXISTS idx_facebook_comments_created_at ON public.gm_agent_facebook_comments(created_at DESC);
CREATE INDEX IF NOT EXISTS idx_facebook_comments_parent_id ON public.gm_agent_facebook_comments(parent_comment_id);

-- Trigger for auto-updating updated_at on Facebook Posts
CREATE TRIGGER set_updated_at_facebook_posts 
    BEFORE UPDATE ON public.gm_agent_facebook_posts 
    FOR EACH ROW EXECUTE FUNCTION public.diesel_set_updated_at();

-- Trigger for auto-updating updated_at on Facebook Comments
CREATE TRIGGER set_updated_at_facebook_comments 
    BEFORE UPDATE ON public.gm_agent_facebook_comments 
    FOR EACH ROW EXECUTE FUNCTION public.diesel_set_updated_at();
