-- =============================================================================
-- AI Publish Module - Rollback Migration
-- =============================================================================

-- Drop tables in reverse order (respecting foreign key constraints)
DROP TABLE IF EXISTS public.gm_aipub_tasks;
DROP TABLE IF EXISTS public.gm_aipub_ai_tasks;
DROP TABLE IF EXISTS public.gm_aipub_plans;
