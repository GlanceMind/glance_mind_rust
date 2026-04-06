-- Reverse of 2026-03-25-000001_add_novel_engine_schemas/up.sql

DROP FUNCTION IF EXISTS fn_gm_novel_next_event_sequence(TEXT);

DROP TABLE IF EXISTS hb_novel_vector_operations;
DROP TABLE IF EXISTS hb_novel_provider_request_logs;
DROP TABLE IF EXISTS hb_novel_worker_attempts;
DROP TABLE IF EXISTS hb_novel_worker_tasks;

DROP TABLE IF EXISTS gm_novel_memory_chunks;
DROP TABLE IF EXISTS gm_novel_knowledge_chunks;
DROP TABLE IF EXISTS gm_novel_knowledge_imports;
DROP TABLE IF EXISTS gm_novel_consistency_checks;
DROP TABLE IF EXISTS gm_novel_plot_arc_snapshots;
DROP TABLE IF EXISTS gm_novel_global_summary_snapshots;
DROP TABLE IF EXISTS gm_novel_character_state_snapshots;
DROP TABLE IF EXISTS gm_novel_chapters;
DROP TABLE IF EXISTS gm_novel_chapter_prompts;
DROP TABLE IF EXISTS gm_novel_blueprint_chapters;
DROP TABLE IF EXISTS gm_novel_blueprints;
DROP TABLE IF EXISTS gm_novel_architectures;
DROP TABLE IF EXISTS gm_novel_architecture_checkpoints;
DROP TABLE IF EXISTS gm_novel_stage_events;
DROP TABLE IF EXISTS gm_novel_stage_runs;
DROP TABLE IF EXISTS gm_novel_jobs;
DROP TABLE IF EXISTS gm_novel_project_config_snapshots;
DROP TABLE IF EXISTS gm_novel_embedding_profiles;
DROP TABLE IF EXISTS gm_novel_llm_profiles;
DROP TABLE IF EXISTS gm_novel_projects;
