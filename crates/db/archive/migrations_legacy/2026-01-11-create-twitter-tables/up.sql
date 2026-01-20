-- Twitter Tables Migration (Up)
-- Created: 2026-01-11
-- 
-- Tables:
-- 1. gm_agent_twitter_tweets - Stores tweets fetched by the agent
-- 2. gm_agent_twitter_comments - Stores tweet replies/comments with AI analysis

-- ============================================================================
-- Table: gm_agent_twitter_tweets
-- ============================================================================
CREATE TABLE gm_agent_twitter_tweets (
    id SERIAL PRIMARY KEY,
    task_id INTEGER NOT NULL REFERENCES gm_crawler_tasks(id) ON DELETE CASCADE,
    campaign_id INTEGER REFERENCES gm_campaigns(id) ON DELETE CASCADE,

    -- Twitter tweet identifiers
    twitter_tweet_id VARCHAR(255) NOT NULL,      -- Tweet ID from Twitter
    conversation_id VARCHAR(255),                 -- Thread/conversation ID

    -- Tweet content
    full_text TEXT NOT NULL DEFAULT '',           -- Tweet text content
    lang VARCHAR(10),                             -- Language code (e.g., 'en', 'zh')

    -- Author info
    screen_name VARCHAR(255),                     -- Twitter handle (e.g., @username)
    user_name VARCHAR(255),                       -- Display name
    user_id VARCHAR(255),                         -- Twitter user ID
    user_description TEXT,                        -- User bio
    user_followers_count INTEGER DEFAULT 0,       -- Follower count
    user_avatar TEXT,                             -- Avatar URL
    user_verified BOOLEAN DEFAULT FALSE,          -- Verified status

    -- Media
    media_urls TEXT[],                            -- Array of media URLs (images/videos)
    has_media BOOLEAN DEFAULT FALSE,              -- Whether tweet has media

    -- Engagement metrics
    favorite_count INTEGER DEFAULT 0,             -- Likes
    retweet_count INTEGER DEFAULT 0,              -- Retweets
    reply_count INTEGER DEFAULT 0,                -- Replies
    quote_count INTEGER DEFAULT 0,                -- Quote tweets
    bookmark_count INTEGER DEFAULT 0,             -- Bookmarks
    view_count INTEGER DEFAULT 0,                 -- Views

    -- Reply info (if this tweet is a reply)
    is_reply BOOLEAN DEFAULT FALSE,
    in_reply_to_status_id VARCHAR(255),           -- Parent tweet ID
    in_reply_to_user_id VARCHAR(255),             -- Parent user ID

    -- Timestamps
    created_at_str VARCHAR(255),                  -- Raw created_at string
    created_at_ts BIGINT,                         -- Parsed timestamp
    tweet_created_at TIMESTAMP WITH TIME ZONE,    -- Converted datetime
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW() NOT NULL,
    updated_at TIMESTAMP WITH TIME ZONE,

    -- Constraints
    UNIQUE(twitter_tweet_id, task_id)
);

CREATE INDEX idx_twitter_tweets_task_id ON gm_agent_twitter_tweets(task_id);
CREATE INDEX idx_twitter_tweets_campaign_id ON gm_agent_twitter_tweets(campaign_id);
CREATE INDEX idx_twitter_tweets_twitter_id ON gm_agent_twitter_tweets(twitter_tweet_id);
CREATE INDEX idx_twitter_tweets_conversation_id ON gm_agent_twitter_tweets(conversation_id);
CREATE INDEX idx_twitter_tweets_screen_name ON gm_agent_twitter_tweets(screen_name);


-- ============================================================================
-- Table: gm_agent_twitter_comments
-- ============================================================================
CREATE TABLE gm_agent_twitter_comments (
    id SERIAL PRIMARY KEY,
    tweet_db_id INTEGER NOT NULL REFERENCES gm_agent_twitter_tweets(id) ON DELETE CASCADE,
    campaign_id INTEGER REFERENCES gm_campaigns(id) ON DELETE CASCADE,

    -- Comment identifiers (comments are also tweets in Twitter)
    twitter_comment_id VARCHAR(255) NOT NULL,     -- Comment tweet ID
    conversation_id VARCHAR(255),                 -- Thread/conversation ID

    -- Author info
    comment_screen_name VARCHAR(255),             -- Commenter's handle
    comment_user_name VARCHAR(255),               -- Commenter's display name
    comment_user_id VARCHAR(255),                 -- Commenter's Twitter user ID
    comment_user_followers INTEGER DEFAULT 0,     -- Commenter's followers

    -- Comment content
    comment_text TEXT NOT NULL,                   -- Comment text

    -- AI analysis results
    reason TEXT,                                  -- Why this comment needs reply
    suggested_reply TEXT,                         -- AI-generated reply suggestion
    status VARCHAR(50) DEFAULT 'PENDING',         -- PENDING, APPROVED, REJECTED, SENT

    -- Engagement metrics
    favorite_count INTEGER DEFAULT 0,
    retweet_count INTEGER DEFAULT 0,
    reply_count INTEGER DEFAULT 0,

    -- Comment structure (reply chain)
    in_reply_to_status_id VARCHAR(255),           -- Parent tweet/comment ID
    is_reply BOOLEAN DEFAULT TRUE,                -- Always true for comments

    -- Media
    media_urls TEXT[],
    has_media BOOLEAN DEFAULT FALSE,

    -- Timestamps
    created_at_str VARCHAR(255),                  -- Raw created_at string
    created_at_ts BIGINT,                         -- Parsed timestamp
    comment_created_at TIMESTAMP WITH TIME ZONE,  -- Converted datetime
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW() NOT NULL,
    updated_at TIMESTAMP WITH TIME ZONE,

    -- Constraints
    UNIQUE(twitter_comment_id, tweet_db_id)
);

CREATE INDEX idx_twitter_comments_tweet_db_id ON gm_agent_twitter_comments(tweet_db_id);
CREATE INDEX idx_twitter_comments_campaign_id ON gm_agent_twitter_comments(campaign_id);
CREATE INDEX idx_twitter_comments_twitter_id ON gm_agent_twitter_comments(twitter_comment_id);
CREATE INDEX idx_twitter_comments_status ON gm_agent_twitter_comments(status);
CREATE INDEX idx_twitter_comments_screen_name ON gm_agent_twitter_comments(comment_screen_name);
