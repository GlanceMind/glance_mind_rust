-- ============================================================================
-- Novel Engine Phase 1 — Canonical + Worker Runtime Schemas
-- ============================================================================
-- Spec: AI_NovelGenerator/.cursor/plans/20260325_novel_backend_phase1_spec.md
-- Tables 1–20: gm_novel_* (canonical control plane, public schema)
-- Tables 21–24: hb_novel_*  (worker runtime, public schema for phase 1)
-- ============================================================================

-- --------------------------------------------------------------------------
-- 1. gm_novel_projects
-- --------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS gm_novel_projects (
    project_id              TEXT PRIMARY KEY,
    user_id                 INT NOT NULL REFERENCES gm_users(id),
    title                   VARCHAR(255) NOT NULL DEFAULT '',
    topic                   TEXT NOT NULL,
    genre                   VARCHAR(100) NOT NULL DEFAULT '',
    description             TEXT NOT NULL DEFAULT '',
    num_chapters            INT NOT NULL,
    target_words_per_chapter INT NOT NULL,
    default_user_guidance   TEXT,
    status                  VARCHAR(32) NOT NULL DEFAULT 'draft',
    current_stage           VARCHAR(50),
    pending_stage           VARCHAR(50),
    current_chapter_number  INT,
    progress_percent        REAL NOT NULL DEFAULT 0,
    interaction_version     INT NOT NULL DEFAULT 1,
    last_error_message      TEXT,
    metadata                JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at              TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at              TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    completed_at            TIMESTAMPTZ,
    deleted_at              TIMESTAMPTZ
);

CREATE INDEX IF NOT EXISTS idx_gm_novel_projects_user_status
    ON gm_novel_projects(user_id, status);
CREATE INDEX IF NOT EXISTS idx_gm_novel_projects_deleted_at
    ON gm_novel_projects(deleted_at);

-- --------------------------------------------------------------------------
-- 2. gm_novel_llm_profiles
-- --------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS gm_novel_llm_profiles (
    id                      BIGSERIAL PRIMARY KEY,
    user_id                 INT NOT NULL REFERENCES gm_users(id),
    name                    VARCHAR(100) NOT NULL,
    interface_format        VARCHAR(50) NOT NULL,
    base_url                TEXT NOT NULL,
    api_key                 TEXT NOT NULL,
    model_name              VARCHAR(200) NOT NULL,
    temperature             DOUBLE PRECISION NOT NULL DEFAULT 0.7,
    max_tokens              INT NOT NULL DEFAULT 4096,
    timeout_seconds         INT NOT NULL DEFAULT 600,
    is_default              BOOLEAN NOT NULL DEFAULT FALSE,
    is_active               BOOLEAN NOT NULL DEFAULT TRUE,
    metadata                JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at              TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at              TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    deleted_at              TIMESTAMPTZ
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_gm_novel_llm_profiles_user_name
    ON gm_novel_llm_profiles(user_id, name) WHERE deleted_at IS NULL;

-- --------------------------------------------------------------------------
-- 3. gm_novel_embedding_profiles
-- --------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS gm_novel_embedding_profiles (
    id                      BIGSERIAL PRIMARY KEY,
    user_id                 INT NOT NULL REFERENCES gm_users(id),
    name                    VARCHAR(100) NOT NULL,
    interface_format        VARCHAR(50) NOT NULL,
    base_url                TEXT NOT NULL,
    api_key                 TEXT NOT NULL,
    model_name              VARCHAR(200) NOT NULL,
    retrieval_k             INT NOT NULL DEFAULT 4,
    is_default              BOOLEAN NOT NULL DEFAULT FALSE,
    is_active               BOOLEAN NOT NULL DEFAULT TRUE,
    metadata                JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at              TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at              TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    deleted_at              TIMESTAMPTZ
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_gm_novel_embedding_profiles_user_name
    ON gm_novel_embedding_profiles(user_id, name) WHERE deleted_at IS NULL;

-- --------------------------------------------------------------------------
-- 4. gm_novel_project_config_snapshots
-- --------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS gm_novel_project_config_snapshots (
    id                              BIGSERIAL PRIMARY KEY,
    project_id                      TEXT NOT NULL REFERENCES gm_novel_projects(project_id) ON DELETE CASCADE,
    architecture_llm_profile_id     BIGINT REFERENCES gm_novel_llm_profiles(id),
    chapter_outline_llm_profile_id  BIGINT REFERENCES gm_novel_llm_profiles(id),
    prompt_draft_llm_profile_id     BIGINT REFERENCES gm_novel_llm_profiles(id),
    final_chapter_llm_profile_id    BIGINT REFERENCES gm_novel_llm_profiles(id),
    consistency_review_llm_profile_id BIGINT REFERENCES gm_novel_llm_profiles(id),
    embedding_profile_id            BIGINT REFERENCES gm_novel_embedding_profiles(id),
    proxy_setting                   JSONB NOT NULL DEFAULT '{}'::jsonb,
    webdav_config                   JSONB NOT NULL DEFAULT '{}'::jsonb,
    other_params                    JSONB NOT NULL DEFAULT '{}'::jsonb,
    is_current                      BOOLEAN NOT NULL DEFAULT TRUE,
    created_at                      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_gm_novel_config_snapshots_project_current
    ON gm_novel_project_config_snapshots(project_id, is_current);

-- --------------------------------------------------------------------------
-- 5. gm_novel_jobs
-- --------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS gm_novel_jobs (
    id                      BIGSERIAL PRIMARY KEY,
    project_id              TEXT NOT NULL REFERENCES gm_novel_projects(project_id) ON DELETE CASCADE,
    chapter_number          INT,
    stage_code              VARCHAR(50) NOT NULL,
    task_type               VARCHAR(50) NOT NULL,
    status                  VARCHAR(32) NOT NULL DEFAULT 'pending',
    idempotency_key         VARCHAR(255),
    request_payload         JSONB NOT NULL DEFAULT '{}'::jsonb,
    result_payload          JSONB NOT NULL DEFAULT '{}'::jsonb,
    error_payload           JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_by              INT REFERENCES gm_users(id),
    created_at              TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at              TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    started_at              TIMESTAMPTZ,
    completed_at            TIMESTAMPTZ
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_gm_novel_jobs_idempotency
    ON gm_novel_jobs(project_id, idempotency_key) WHERE idempotency_key IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_gm_novel_jobs_project_status
    ON gm_novel_jobs(project_id, status, created_at);

-- --------------------------------------------------------------------------
-- 6. gm_novel_stage_runs
-- --------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS gm_novel_stage_runs (
    id                      BIGSERIAL PRIMARY KEY,
    project_id              TEXT NOT NULL REFERENCES gm_novel_projects(project_id) ON DELETE CASCADE,
    job_id                  BIGINT NOT NULL REFERENCES gm_novel_jobs(id) ON DELETE CASCADE,
    chapter_number          INT NOT NULL DEFAULT 0,
    stage_code              VARCHAR(50) NOT NULL,
    status                  VARCHAR(32) NOT NULL DEFAULT 'pending',
    input_hash              VARCHAR(128) NOT NULL DEFAULT '',
    input_payload           JSONB NOT NULL DEFAULT '{}'::jsonb,
    output_payload          JSONB NOT NULL DEFAULT '{}'::jsonb,
    error_message           TEXT,
    attempt_no              INT NOT NULL DEFAULT 1,
    started_at              TIMESTAMPTZ,
    completed_at            TIMESTAMPTZ,
    created_at              TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at              TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_gm_novel_stage_runs_project_stage_chapter
    ON gm_novel_stage_runs(project_id, stage_code, chapter_number);
CREATE UNIQUE INDEX IF NOT EXISTS idx_gm_novel_stage_runs_dedup
    ON gm_novel_stage_runs(project_id, stage_code, chapter_number, input_hash, attempt_no);

-- --------------------------------------------------------------------------
-- 7. gm_novel_stage_events
-- --------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS gm_novel_stage_events (
    id                      BIGSERIAL PRIMARY KEY,
    project_id              TEXT NOT NULL REFERENCES gm_novel_projects(project_id) ON DELETE CASCADE,
    job_id                  BIGINT REFERENCES gm_novel_jobs(id) ON DELETE SET NULL,
    stage_run_id            BIGINT REFERENCES gm_novel_stage_runs(id) ON DELETE SET NULL,
    chapter_number          INT,
    sequence                BIGINT NOT NULL,
    event_type              VARCHAR(100) NOT NULL,
    stage_code              VARCHAR(50),
    payload                 JSONB NOT NULL DEFAULT '{}'::jsonb,
    occurred_at             TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    created_at              TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_gm_novel_stage_events_project_seq
    ON gm_novel_stage_events(project_id, sequence);
CREATE INDEX IF NOT EXISTS idx_gm_novel_stage_events_project_job
    ON gm_novel_stage_events(project_id, job_id);

-- Helper: atomic per-project sequence allocation
CREATE OR REPLACE FUNCTION fn_gm_novel_next_event_sequence(p_project_id TEXT)
RETURNS BIGINT
LANGUAGE plpgsql AS $$
DECLARE
    next_seq BIGINT;
BEGIN
    PERFORM pg_advisory_xact_lock(hashtext(p_project_id));
    SELECT COALESCE(MAX(sequence), 0) + 1
      INTO next_seq
      FROM gm_novel_stage_events
     WHERE project_id = p_project_id;
    RETURN next_seq;
END;
$$;

-- --------------------------------------------------------------------------
-- 8. gm_novel_architecture_checkpoints
-- --------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS gm_novel_architecture_checkpoints (
    id                      BIGSERIAL PRIMARY KEY,
    project_id              TEXT NOT NULL REFERENCES gm_novel_projects(project_id) ON DELETE CASCADE,
    core_seed_result        TEXT,
    character_dynamics_result TEXT,
    character_state_result  TEXT,
    world_building_result   TEXT,
    plot_arch_result        TEXT,
    status                  VARCHAR(32) NOT NULL DEFAULT 'partial',
    source_stage_run_id     BIGINT REFERENCES gm_novel_stage_runs(id) ON DELETE SET NULL,
    created_at              TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at              TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_gm_novel_arch_checkpoints_project
    ON gm_novel_architecture_checkpoints(project_id);

-- --------------------------------------------------------------------------
-- 9. gm_novel_architectures
-- --------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS gm_novel_architectures (
    id                      BIGSERIAL PRIMARY KEY,
    project_id              TEXT NOT NULL REFERENCES gm_novel_projects(project_id) ON DELETE CASCADE,
    core_seed_text          TEXT NOT NULL DEFAULT '',
    character_dynamics_text TEXT NOT NULL DEFAULT '',
    world_building_text     TEXT NOT NULL DEFAULT '',
    plot_architecture_text  TEXT NOT NULL DEFAULT '',
    full_text               TEXT NOT NULL DEFAULT '',
    version_no              INT NOT NULL DEFAULT 1,
    is_current              BOOLEAN NOT NULL DEFAULT TRUE,
    source_stage_run_id     BIGINT REFERENCES gm_novel_stage_runs(id) ON DELETE SET NULL,
    created_at              TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at              TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_gm_novel_architectures_project_current
    ON gm_novel_architectures(project_id, is_current);

-- --------------------------------------------------------------------------
-- 10. gm_novel_blueprints
-- --------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS gm_novel_blueprints (
    id                      BIGSERIAL PRIMARY KEY,
    project_id              TEXT NOT NULL REFERENCES gm_novel_projects(project_id) ON DELETE CASCADE,
    raw_text                TEXT NOT NULL DEFAULT '',
    chunk_size              INT,
    generated_chapter_count INT NOT NULL DEFAULT 0,
    version_no              INT NOT NULL DEFAULT 1,
    is_current              BOOLEAN NOT NULL DEFAULT TRUE,
    source_stage_run_id     BIGINT REFERENCES gm_novel_stage_runs(id) ON DELETE SET NULL,
    created_at              TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at              TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_gm_novel_blueprints_project_current
    ON gm_novel_blueprints(project_id, is_current);

-- --------------------------------------------------------------------------
-- 11. gm_novel_blueprint_chapters
-- --------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS gm_novel_blueprint_chapters (
    id                      BIGSERIAL PRIMARY KEY,
    blueprint_id            BIGINT NOT NULL REFERENCES gm_novel_blueprints(id) ON DELETE CASCADE,
    project_id              TEXT NOT NULL REFERENCES gm_novel_projects(project_id) ON DELETE CASCADE,
    chapter_number          INT NOT NULL,
    chapter_title           VARCHAR(255) NOT NULL DEFAULT '',
    chapter_role            VARCHAR(255) NOT NULL DEFAULT '',
    chapter_purpose         TEXT NOT NULL DEFAULT '',
    suspense_level          VARCHAR(100) NOT NULL DEFAULT '',
    foreshadowing           TEXT NOT NULL DEFAULT '',
    plot_twist_level        VARCHAR(100) NOT NULL DEFAULT '',
    chapter_summary         TEXT NOT NULL DEFAULT '',
    created_at              TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at              TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_gm_novel_blueprint_chapters_dedup
    ON gm_novel_blueprint_chapters(project_id, blueprint_id, chapter_number);

-- --------------------------------------------------------------------------
-- 12. gm_novel_chapter_prompts
-- --------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS gm_novel_chapter_prompts (
    id                      BIGSERIAL PRIMARY KEY,
    project_id              TEXT NOT NULL REFERENCES gm_novel_projects(project_id) ON DELETE CASCADE,
    chapter_number          INT NOT NULL,
    blueprint_chapter_id    BIGINT REFERENCES gm_novel_blueprint_chapters(id) ON DELETE SET NULL,
    user_guidance           TEXT,
    characters_involved     TEXT,
    key_items               TEXT,
    scene_location          TEXT,
    time_constraint         TEXT,
    short_summary           TEXT,
    previous_excerpt        TEXT,
    filtered_context        TEXT,
    prompt_text             TEXT NOT NULL,
    edited_prompt_text      TEXT,
    is_current              BOOLEAN NOT NULL DEFAULT TRUE,
    source_stage_run_id     BIGINT REFERENCES gm_novel_stage_runs(id) ON DELETE SET NULL,
    created_at              TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at              TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_gm_novel_chapter_prompts_project_chapter_current
    ON gm_novel_chapter_prompts(project_id, chapter_number, is_current);

-- --------------------------------------------------------------------------
-- 13. gm_novel_chapters
-- --------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS gm_novel_chapters (
    id                      BIGSERIAL PRIMARY KEY,
    project_id              TEXT NOT NULL REFERENCES gm_novel_projects(project_id) ON DELETE CASCADE,
    chapter_number          INT NOT NULL,
    blueprint_chapter_id    BIGINT REFERENCES gm_novel_blueprint_chapters(id) ON DELETE SET NULL,
    prompt_id               BIGINT REFERENCES gm_novel_chapter_prompts(id) ON DELETE SET NULL,
    title                   VARCHAR(255) NOT NULL DEFAULT '',
    user_guidance           TEXT,
    characters_involved     TEXT,
    key_items               TEXT,
    scene_location          TEXT,
    time_constraint         TEXT,
    draft_text              TEXT,
    final_text              TEXT,
    status                  VARCHAR(32) NOT NULL DEFAULT 'pending',
    is_enriched             BOOLEAN NOT NULL DEFAULT FALSE,
    target_words            INT,
    draft_word_count        INT NOT NULL DEFAULT 0,
    final_word_count        INT NOT NULL DEFAULT 0,
    consistency_status      VARCHAR(32),
    source_stage_run_id     BIGINT REFERENCES gm_novel_stage_runs(id) ON DELETE SET NULL,
    created_at              TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at              TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    finalized_at            TIMESTAMPTZ
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_gm_novel_chapters_project_number
    ON gm_novel_chapters(project_id, chapter_number);
CREATE INDEX IF NOT EXISTS idx_gm_novel_chapters_project_status
    ON gm_novel_chapters(project_id, status);

-- --------------------------------------------------------------------------
-- 14. gm_novel_character_state_snapshots
-- --------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS gm_novel_character_state_snapshots (
    id                      BIGSERIAL PRIMARY KEY,
    project_id              TEXT NOT NULL REFERENCES gm_novel_projects(project_id) ON DELETE CASCADE,
    chapter_number          INT,
    state_text              TEXT NOT NULL DEFAULT '',
    version_no              INT NOT NULL DEFAULT 1,
    is_current              BOOLEAN NOT NULL DEFAULT TRUE,
    source_stage_run_id     BIGINT REFERENCES gm_novel_stage_runs(id) ON DELETE SET NULL,
    created_at              TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_gm_novel_char_state_project_current
    ON gm_novel_character_state_snapshots(project_id, is_current);

-- --------------------------------------------------------------------------
-- 15. gm_novel_global_summary_snapshots
-- --------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS gm_novel_global_summary_snapshots (
    id                      BIGSERIAL PRIMARY KEY,
    project_id              TEXT NOT NULL REFERENCES gm_novel_projects(project_id) ON DELETE CASCADE,
    chapter_number          INT,
    summary_text            TEXT NOT NULL DEFAULT '',
    version_no              INT NOT NULL DEFAULT 1,
    is_current              BOOLEAN NOT NULL DEFAULT TRUE,
    source_stage_run_id     BIGINT REFERENCES gm_novel_stage_runs(id) ON DELETE SET NULL,
    created_at              TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_gm_novel_global_summary_project_current
    ON gm_novel_global_summary_snapshots(project_id, is_current);

-- --------------------------------------------------------------------------
-- 16. gm_novel_plot_arc_snapshots
-- --------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS gm_novel_plot_arc_snapshots (
    id                      BIGSERIAL PRIMARY KEY,
    project_id              TEXT NOT NULL REFERENCES gm_novel_projects(project_id) ON DELETE CASCADE,
    chapter_number          INT,
    plot_arcs_text          TEXT NOT NULL DEFAULT '',
    version_no              INT NOT NULL DEFAULT 1,
    is_current              BOOLEAN NOT NULL DEFAULT TRUE,
    source_stage_run_id     BIGINT REFERENCES gm_novel_stage_runs(id) ON DELETE SET NULL,
    created_at              TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_gm_novel_plot_arcs_project_current
    ON gm_novel_plot_arc_snapshots(project_id, is_current);

-- --------------------------------------------------------------------------
-- 17. gm_novel_consistency_checks
-- --------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS gm_novel_consistency_checks (
    id                      BIGSERIAL PRIMARY KEY,
    project_id              TEXT NOT NULL REFERENCES gm_novel_projects(project_id) ON DELETE CASCADE,
    chapter_number          INT NOT NULL,
    novel_setting_text      TEXT,
    character_state_text    TEXT,
    global_summary_text     TEXT,
    plot_arcs_text          TEXT,
    chapter_text            TEXT NOT NULL,
    result_text             TEXT NOT NULL DEFAULT '',
    status                  VARCHAR(32) NOT NULL DEFAULT 'completed',
    source_stage_run_id     BIGINT REFERENCES gm_novel_stage_runs(id) ON DELETE SET NULL,
    created_at              TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_gm_novel_consistency_checks_project_chapter
    ON gm_novel_consistency_checks(project_id, chapter_number, created_at DESC);

-- --------------------------------------------------------------------------
-- 18. gm_novel_knowledge_imports
-- --------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS gm_novel_knowledge_imports (
    id                      BIGSERIAL PRIMARY KEY,
    project_id              TEXT NOT NULL REFERENCES gm_novel_projects(project_id) ON DELETE CASCADE,
    source_name             VARCHAR(255) NOT NULL,
    source_type             VARCHAR(50) NOT NULL DEFAULT 'text_file',
    original_text           TEXT NOT NULL DEFAULT '',
    segment_count           INT NOT NULL DEFAULT 0,
    status                  VARCHAR(32) NOT NULL DEFAULT 'completed',
    source_stage_run_id     BIGINT REFERENCES gm_novel_stage_runs(id) ON DELETE SET NULL,
    created_at              TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_gm_novel_knowledge_imports_project
    ON gm_novel_knowledge_imports(project_id, created_at DESC);

-- --------------------------------------------------------------------------
-- 19. gm_novel_knowledge_chunks
-- --------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS gm_novel_knowledge_chunks (
    id                      BIGSERIAL PRIMARY KEY,
    project_id              TEXT NOT NULL REFERENCES gm_novel_projects(project_id) ON DELETE CASCADE,
    knowledge_import_id     BIGINT NOT NULL REFERENCES gm_novel_knowledge_imports(id) ON DELETE CASCADE,
    chunk_index             INT NOT NULL,
    content                 TEXT NOT NULL,
    metadata                JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at              TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_gm_novel_knowledge_chunks_import
    ON gm_novel_knowledge_chunks(knowledge_import_id, chunk_index);
CREATE INDEX IF NOT EXISTS idx_gm_novel_knowledge_chunks_project
    ON gm_novel_knowledge_chunks(project_id);

-- --------------------------------------------------------------------------
-- 20. gm_novel_memory_chunks
-- --------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS gm_novel_memory_chunks (
    id                      BIGSERIAL PRIMARY KEY,
    project_id              TEXT NOT NULL REFERENCES gm_novel_projects(project_id) ON DELETE CASCADE,
    source_type             VARCHAR(50) NOT NULL,
    source_ref_id           BIGINT,
    chapter_number          INT,
    chunk_index             INT NOT NULL DEFAULT 0,
    content                 TEXT NOT NULL,
    embedding_profile_id    BIGINT REFERENCES gm_novel_embedding_profiles(id) ON DELETE SET NULL,
    embedding_json          JSONB,
    embedding_dim           INT,
    embedding_status        VARCHAR(32) NOT NULL DEFAULT 'pending',
    metadata                JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at              TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at              TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_gm_novel_memory_chunks_project_source
    ON gm_novel_memory_chunks(project_id, source_type, chapter_number);

-- ============================================================================
-- Worker Runtime Tables (21–24)
-- ============================================================================

-- --------------------------------------------------------------------------
-- 21. hb_novel_worker_tasks
-- --------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS hb_novel_worker_tasks (
    id                      TEXT PRIMARY KEY,
    project_id              TEXT NOT NULL,
    job_id                  BIGINT REFERENCES gm_novel_jobs(id) ON DELETE SET NULL,
    stage_code              VARCHAR(50) NOT NULL,
    task_type               VARCHAR(50) NOT NULL,
    chapter_number          INT,
    status                  VARCHAR(32) NOT NULL DEFAULT 'queued',
    payload                 JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at              TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at              TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    started_at              TIMESTAMPTZ,
    completed_at            TIMESTAMPTZ
);

CREATE INDEX IF NOT EXISTS idx_hb_novel_worker_tasks_project_status
    ON hb_novel_worker_tasks(project_id, status);
CREATE INDEX IF NOT EXISTS idx_hb_novel_worker_tasks_job
    ON hb_novel_worker_tasks(job_id);

-- --------------------------------------------------------------------------
-- 22. hb_novel_worker_attempts
-- --------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS hb_novel_worker_attempts (
    id                      BIGSERIAL PRIMARY KEY,
    worker_task_id          TEXT NOT NULL REFERENCES hb_novel_worker_tasks(id) ON DELETE CASCADE,
    attempt_no              INT NOT NULL,
    worker_name             VARCHAR(100),
    host_name               VARCHAR(100),
    status                  VARCHAR(32) NOT NULL DEFAULT 'running',
    error_message           TEXT,
    started_at              TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    completed_at            TIMESTAMPTZ
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_hb_novel_worker_attempts_dedup
    ON hb_novel_worker_attempts(worker_task_id, attempt_no);

-- --------------------------------------------------------------------------
-- 23. hb_novel_provider_request_logs
-- --------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS hb_novel_provider_request_logs (
    id                      BIGSERIAL PRIMARY KEY,
    project_id              TEXT NOT NULL,
    job_id                  BIGINT REFERENCES gm_novel_jobs(id) ON DELETE SET NULL,
    stage_code              VARCHAR(50),
    provider_kind           VARCHAR(20) NOT NULL,
    interface_format        VARCHAR(50) NOT NULL,
    model_name              VARCHAR(200),
    request_summary         JSONB NOT NULL DEFAULT '{}'::jsonb,
    response_summary        JSONB NOT NULL DEFAULT '{}'::jsonb,
    status                  VARCHAR(32) NOT NULL DEFAULT 'completed',
    latency_ms              INT,
    error_message           TEXT,
    created_at              TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_hb_novel_provider_logs_project
    ON hb_novel_provider_request_logs(project_id, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_hb_novel_provider_logs_job
    ON hb_novel_provider_request_logs(job_id);

-- --------------------------------------------------------------------------
-- 24. hb_novel_vector_operations
-- --------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS hb_novel_vector_operations (
    id                      BIGSERIAL PRIMARY KEY,
    project_id              TEXT NOT NULL,
    operation_type          VARCHAR(50) NOT NULL,
    knowledge_import_id     BIGINT REFERENCES gm_novel_knowledge_imports(id) ON DELETE SET NULL,
    chapter_number          INT,
    status                  VARCHAR(32) NOT NULL DEFAULT 'completed',
    detail                  JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at              TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_hb_novel_vector_ops_project
    ON hb_novel_vector_operations(project_id, operation_type, created_at DESC);
