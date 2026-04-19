-- Phase 4 Round 3 Task 7a — plan-level PublishBehavior persistence.
--
-- Source of truth: glance_mind_protocol/proto/aipub.proto §960
-- (PublishBehavior message). Stored as a JSONB blob so the proto
-- can evolve (graduate platform_extras keys → typed fields) without
-- requiring a column migration on every change.
--
-- Plan-level (not task-level): one PublishBehavior config drives all
-- tasks derived from a plan. The aipub_service merges this column
-- into task.content.behavior at task derivation time so workers see
-- it without an extra API hop.
--
-- NULL = "use platform defaults" (the worker treats absence of
-- content.behavior as platform-default, which is the safe choice
-- for backward compat with plans created before this column existed).

ALTER TABLE gm_aipub_plans
    ADD COLUMN IF NOT EXISTS behavior JSONB NULL;

COMMENT ON COLUMN gm_aipub_plans.behavior IS
  'Phase 4 R3 Task 7a — PublishBehavior proto JSON (visibility, '
  'allow_comments, is_nsfw, allow_duet, share_to_facebook, …). '
  'Merged into task.content.behavior at task derivation. '
  'NULL = use platform defaults.';
