-- Rollback: Remove tables and columns added in up.sql

-- Drop tables in reverse order (due to foreign key dependencies)
DROP TABLE IF EXISTS gm_agent_facebook_comments;
DROP TABLE IF EXISTS gm_agent_facebook_posts;
DROP TABLE IF EXISTS gm_ai_video_models;
DROP TABLE IF EXISTS gm_admin_users;

-- Remove like_count column from gm_crawler_results
ALTER TABLE gm_crawler_results DROP COLUMN IF EXISTS like_count;
