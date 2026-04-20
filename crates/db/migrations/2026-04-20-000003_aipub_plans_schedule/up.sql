-- Phase 4 Round 3 Task 8 — plan-level PublishSchedule persistence.
--
-- Source of truth: glance_mind_protocol/proto/aipub.proto §996
-- (PublishSchedule message). Stored as JSONB alongside `behavior`
-- (Task 7a) so the proto can evolve independently without requiring
-- a column migration on every field change.
--
-- Plan-level (not task-level): one PublishSchedule config drives all
-- tasks derived from a plan. The aipub_service merges this column
-- into task.content.schedule at task derivation time so workers see
-- it without an extra API hop (same pattern as plan.behavior →
-- task.content.behavior established in Task 7a).
--
-- NULL = "publish immediately when ready" (the default before this
-- column existed). The worker's ensure_unified path keeps falling
-- back to the current immediate-publish behaviour when
-- content.schedule is absent.

ALTER TABLE gm_aipub_plans
    ADD COLUMN IF NOT EXISTS schedule JSONB NULL;

COMMENT ON COLUMN gm_aipub_plans.schedule IS
  'Phase 4 R3 Task 8 — PublishSchedule proto JSON (scheduled_at UTC '
  'ISO-8601, timezone IANA, save_as_draft bool). Merged into '
  'task.content.schedule at task derivation. NULL = publish immediately.';
