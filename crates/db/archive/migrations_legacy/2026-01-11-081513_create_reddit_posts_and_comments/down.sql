-- Drop triggers
DROP TRIGGER IF EXISTS set_updated_at_reddit_comments ON gm_agent_reddit_comments;
DROP TRIGGER IF EXISTS set_updated_at_reddit_posts ON gm_agent_reddit_posts;

-- Drop indexes
DROP INDEX IF EXISTS idx_reddit_comments_created_at;
DROP INDEX IF EXISTS idx_reddit_comments_status;
DROP INDEX IF EXISTS idx_reddit_comments_comment_id;
DROP INDEX IF EXISTS idx_reddit_comments_campaign_id;
DROP INDEX IF EXISTS idx_reddit_comments_post_db_id;

DROP INDEX IF EXISTS idx_reddit_posts_created_at;
DROP INDEX IF EXISTS idx_reddit_posts_subreddit;
DROP INDEX IF EXISTS idx_reddit_posts_post_id;
DROP INDEX IF EXISTS idx_reddit_posts_campaign_id;
DROP INDEX IF EXISTS idx_reddit_posts_task_id;

-- Drop tables (comments first due to foreign key)
DROP TABLE IF EXISTS gm_agent_reddit_comments;
DROP TABLE IF EXISTS gm_agent_reddit_posts;
