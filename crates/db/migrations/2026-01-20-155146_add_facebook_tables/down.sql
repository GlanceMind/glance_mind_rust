-- Drop triggers first
DROP TRIGGER IF EXISTS set_updated_at_facebook_comments ON public.gm_agent_facebook_comments;
DROP TRIGGER IF EXISTS set_updated_at_facebook_posts ON public.gm_agent_facebook_posts;

-- Drop indexes
DROP INDEX IF EXISTS idx_facebook_comments_parent_id;
DROP INDEX IF EXISTS idx_facebook_comments_created_at;
DROP INDEX IF EXISTS idx_facebook_comments_comment_username;
DROP INDEX IF EXISTS idx_facebook_comments_status;
DROP INDEX IF EXISTS idx_facebook_comments_facebook_comment_id;
DROP INDEX IF EXISTS idx_facebook_comments_campaign_id;
DROP INDEX IF EXISTS idx_facebook_comments_post_db_id;

DROP INDEX IF EXISTS idx_facebook_posts_created_at;
DROP INDEX IF EXISTS idx_facebook_posts_posted_at;
DROP INDEX IF EXISTS idx_facebook_posts_author_id;
DROP INDEX IF EXISTS idx_facebook_posts_facebook_post_id;
DROP INDEX IF EXISTS idx_facebook_posts_campaign_id;
DROP INDEX IF EXISTS idx_facebook_posts_task_id;

-- Drop tables (comments first due to foreign key)
DROP TABLE IF EXISTS public.gm_agent_facebook_comments;
DROP TABLE IF EXISTS public.gm_agent_facebook_posts;
