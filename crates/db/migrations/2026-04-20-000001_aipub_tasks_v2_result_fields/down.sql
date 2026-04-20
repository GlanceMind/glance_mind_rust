-- Phase 4 Round 3 Task 3 — rollback v2 result columns.

ALTER TABLE gm_aipub_tasks
    DROP COLUMN IF EXISTS media_results,
    DROP COLUMN IF EXISTS post_publish_results,
    DROP COLUMN IF EXISTS failed_error_code,
    DROP COLUMN IF EXISTS execution_log,
    DROP COLUMN IF EXISTS platform_post_id;
