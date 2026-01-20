-- Rollback Instagram Posts and Comments Tables

-- Drop triggers
DROP TRIGGER IF EXISTS trigger_update_instagram_comments_updated_at ON gm_agent_instagram_comments;
DROP TRIGGER IF EXISTS trigger_update_instagram_posts_updated_at ON gm_agent_instagram_posts;

-- Drop functions
DROP FUNCTION IF EXISTS update_instagram_comments_updated_at();
DROP FUNCTION IF EXISTS update_instagram_posts_updated_at();

-- Drop tables (comments first due to foreign key)
DROP TABLE IF EXISTS gm_agent_instagram_comments;
DROP TABLE IF EXISTS gm_agent_instagram_posts;

-- Drop sequences
DROP SEQUENCE IF EXISTS gm_agent_instagram_comments_id_seq;
DROP SEQUENCE IF EXISTS gm_agent_instagram_posts_id_seq;
