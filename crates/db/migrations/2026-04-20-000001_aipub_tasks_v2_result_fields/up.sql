-- Phase 4 Round 3 Task 3 — accept UnifiedPublishResult (v2) payload.
--
-- Round 2 switched the worker to emit a richer v2 publish result over the
-- executor callback. Before this migration the API side silently dropped
-- every v2 field except status/result_url/error_message. These columns
-- give each v2 field its own persistent home so we can query and debug
-- real publish outcomes after the fact.
--
-- All columns are nullable because:
--   * v1 legacy payloads still land here with none of these fields set.
--   * A task that hasn't been picked up yet has nothing to report.

ALTER TABLE gm_aipub_tasks
    ADD COLUMN IF NOT EXISTS media_results JSONB NULL,
    ADD COLUMN IF NOT EXISTS post_publish_results JSONB NULL,
    ADD COLUMN IF NOT EXISTS failed_error_code VARCHAR(64) NULL,
    ADD COLUMN IF NOT EXISTS execution_log TEXT NULL,
    ADD COLUMN IF NOT EXISTS platform_post_id VARCHAR(128) NULL;

COMMENT ON COLUMN gm_aipub_tasks.media_results IS
    'Phase 4 Round 3: per-media outcomes from UnifiedPublishResult.media_results[]';
COMMENT ON COLUMN gm_aipub_tasks.post_publish_results IS
    'Phase 4 Round 3: post-publish action outcomes from UnifiedPublishResult.post_publish_results[]';
COMMENT ON COLUMN gm_aipub_tasks.failed_error_code IS
    'Phase 4 Round 3: machine-readable failure class (e.g. UPLOADER_EXCEPTION)';
COMMENT ON COLUMN gm_aipub_tasks.execution_log IS
    'Phase 4 Round 3: optional execution trace attached by the executor';
COMMENT ON COLUMN gm_aipub_tasks.platform_post_id IS
    'Phase 4 Round 3: platform-assigned post id (e.g. TikTok video_id, Reddit t3_xxx)';
