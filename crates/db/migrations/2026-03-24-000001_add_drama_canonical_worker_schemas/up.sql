-- ============================================================================
-- Drama Canonical + Worker Runtime Schemas
-- ============================================================================
-- This migration establishes the PostgreSQL-only baseline for the future
-- short-drama architecture:
--   - `gm_drama` owns canonical, user-visible truth
--   - `hb_worker_runtime` owns worker execution/runtime recovery state
--
-- Notes:
-- - Existing public projection tables (`gm_drama_*` in public schema) remain
--   unchanged in this step to avoid breaking current handlers.
-- - This migration creates the long-term table baseline so route and worker
--   cutovers no longer depend on sqlite/mysql/local-static assumptions.
-- ============================================================================

CREATE SCHEMA IF NOT EXISTS gm_drama;
CREATE SCHEMA IF NOT EXISTS hb_worker_runtime;

-- ============================================================================
-- gm_drama: canonical control plane tables
-- ============================================================================

CREATE TABLE IF NOT EXISTS gm_drama.projects (
    project_id              TEXT PRIMARY KEY,
    user_id                 INT NOT NULL REFERENCES public.gm_users(id),
    title                   VARCHAR(500) NOT NULL DEFAULT '',
    description             TEXT NOT NULL DEFAULT '',
    content_type            VARCHAR(50),
    genre                   VARCHAR(100),
    style                   VARCHAR(100) NOT NULL DEFAULT 'realistic',
    total_episodes          INT NOT NULL DEFAULT 1,
    total_duration_seconds  INT NOT NULL DEFAULT 0,
    status                  VARCHAR(50) NOT NULL DEFAULT 'draft',
    thumbnail_asset_id      BIGINT,
    tags                    JSONB NOT NULL DEFAULT '[]'::jsonb,
    metadata                JSONB NOT NULL DEFAULT '{}'::jsonb,
    interaction_version     INT NOT NULL DEFAULT 1,
    current_stage           VARCHAR(50),
    pending_stage           VARCHAR(50),
    progress_percent        REAL NOT NULL DEFAULT 0,
    cost_reserve_cents      BIGINT NOT NULL DEFAULT 0,
    cost_consumed_cents     BIGINT NOT NULL DEFAULT 0,
    error_message           TEXT,
    created_at              TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at              TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    completed_at            TIMESTAMPTZ,
    deleted_at              TIMESTAMPTZ
);

CREATE INDEX IF NOT EXISTS idx_gm_drama_projects_user_id
    ON gm_drama.projects(user_id);
CREATE INDEX IF NOT EXISTS idx_gm_drama_projects_status
    ON gm_drama.projects(status);
CREATE INDEX IF NOT EXISTS idx_gm_drama_projects_deleted_at
    ON gm_drama.projects(deleted_at);

CREATE TABLE IF NOT EXISTS gm_drama.stage_sessions (
    id                      BIGSERIAL PRIMARY KEY,
    project_id              TEXT NOT NULL REFERENCES gm_drama.projects(project_id) ON DELETE CASCADE,
    stage_code              VARCHAR(50) NOT NULL,
    status                  VARCHAR(50) NOT NULL DEFAULT 'pending',
    interaction_version     INT NOT NULL DEFAULT 1,
    input_payload           JSONB NOT NULL DEFAULT '{}'::jsonb,
    output_payload          JSONB NOT NULL DEFAULT '{}'::jsonb,
    started_at              TIMESTAMPTZ,
    completed_at            TIMESTAMPTZ,
    created_at              TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at              TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_gm_drama_stage_sessions_project_stage
    ON gm_drama.stage_sessions(project_id, stage_code);

CREATE TABLE IF NOT EXISTS gm_drama.jobs (
    id                      BIGSERIAL PRIMARY KEY,
    project_id              TEXT NOT NULL REFERENCES gm_drama.projects(project_id) ON DELETE CASCADE,
    stage_code              VARCHAR(50),
    job_type                VARCHAR(100) NOT NULL,
    status                  VARCHAR(50) NOT NULL DEFAULT 'pending',
    worker_name             VARCHAR(100),
    idempotency_key         VARCHAR(255),
    request_payload         JSONB NOT NULL DEFAULT '{}'::jsonb,
    result_payload          JSONB NOT NULL DEFAULT '{}'::jsonb,
    error_payload           JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at              TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at              TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    started_at              TIMESTAMPTZ,
    completed_at            TIMESTAMPTZ
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_gm_drama_jobs_project_idempotency
    ON gm_drama.jobs(project_id, idempotency_key)
    WHERE idempotency_key IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_gm_drama_jobs_project_status
    ON gm_drama.jobs(project_id, status);

CREATE TABLE IF NOT EXISTS gm_drama.events (
    id                      BIGSERIAL PRIMARY KEY,
    project_id              TEXT NOT NULL REFERENCES gm_drama.projects(project_id) ON DELETE CASCADE,
    job_id                  BIGINT REFERENCES gm_drama.jobs(id) ON DELETE SET NULL,
    run_id                  VARCHAR(255),
    sequence                BIGINT NOT NULL,
    stage_code              VARCHAR(50),
    event_type              VARCHAR(100) NOT NULL,
    status                  VARCHAR(50),
    payload                 JSONB NOT NULL DEFAULT '{}'::jsonb,
    occurred_at             TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    created_at              TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_gm_drama_events_project_sequence
    ON gm_drama.events(project_id, sequence);
CREATE INDEX IF NOT EXISTS idx_gm_drama_events_project_run
    ON gm_drama.events(project_id, run_id);

CREATE TABLE IF NOT EXISTS gm_drama.cost_ledger (
    id                      BIGSERIAL PRIMARY KEY,
    project_id              TEXT NOT NULL REFERENCES gm_drama.projects(project_id) ON DELETE CASCADE,
    job_id                  BIGINT REFERENCES gm_drama.jobs(id) ON DELETE SET NULL,
    stage_code              VARCHAR(50),
    cost_type               VARCHAR(100) NOT NULL,
    provider                VARCHAR(100),
    amount_cents            BIGINT NOT NULL DEFAULT 0,
    currency                VARCHAR(16) NOT NULL DEFAULT 'cny_cent',
    payload                 JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at              TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_gm_drama_cost_ledger_project_id
    ON gm_drama.cost_ledger(project_id);

CREATE TABLE IF NOT EXISTS gm_drama.fallback_events (
    id                      BIGSERIAL PRIMARY KEY,
    project_id              TEXT NOT NULL REFERENCES gm_drama.projects(project_id) ON DELETE CASCADE,
    job_id                  BIGINT REFERENCES gm_drama.jobs(id) ON DELETE SET NULL,
    stage_code              VARCHAR(50),
    reason_category         VARCHAR(100),
    reason_detail           TEXT,
    from_provider           VARCHAR(100),
    to_provider             VARCHAR(100),
    payload                 JSONB NOT NULL DEFAULT '{}'::jsonb,
    occurred_at             TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    created_at              TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_gm_drama_fallback_events_project_id
    ON gm_drama.fallback_events(project_id);

CREATE TABLE IF NOT EXISTS gm_drama.assets (
    id                      BIGSERIAL PRIMARY KEY,
    project_id              TEXT NOT NULL REFERENCES gm_drama.projects(project_id) ON DELETE CASCADE,
    episode_id              BIGINT,
    storyboard_id           BIGINT,
    storyboard_num          INT,
    name                    VARCHAR(200) NOT NULL,
    description             TEXT,
    type                    VARCHAR(20) NOT NULL,
    category                VARCHAR(50),
    bucket                  VARCHAR(255) NOT NULL,
    object_key              TEXT NOT NULL,
    public_url              TEXT NOT NULL,
    thumbnail_url           TEXT,
    mime_type               VARCHAR(100),
    file_size               BIGINT,
    width                   INT,
    height                  INT,
    duration_seconds        INT,
    format                  VARCHAR(50),
    checksum                VARCHAR(128),
    source_job_id           BIGINT REFERENCES gm_drama.jobs(id) ON DELETE SET NULL,
    source_image_generation_id BIGINT,
    source_video_generation_id BIGINT,
    is_favorite             BOOLEAN NOT NULL DEFAULT FALSE,
    view_count              INT NOT NULL DEFAULT 0,
    metadata                JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at              TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at              TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    deleted_at              TIMESTAMPTZ
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_gm_drama_assets_bucket_object
    ON gm_drama.assets(bucket, object_key);
CREATE INDEX IF NOT EXISTS idx_gm_drama_assets_project_type
    ON gm_drama.assets(project_id, type);
CREATE INDEX IF NOT EXISTS idx_gm_drama_assets_deleted_at
    ON gm_drama.assets(deleted_at);

CREATE TABLE IF NOT EXISTS gm_drama.episodes (
    id                      BIGSERIAL PRIMARY KEY,
    project_id              TEXT NOT NULL REFERENCES gm_drama.projects(project_id) ON DELETE CASCADE,
    episode_number          INT NOT NULL,
    title                   VARCHAR(200) NOT NULL,
    script_content          TEXT,
    description             TEXT,
    duration_seconds        INT NOT NULL DEFAULT 0,
    status                  VARCHAR(20) NOT NULL DEFAULT 'draft',
    video_asset_id          BIGINT,
    thumbnail_asset_id      BIGINT,
    metadata                JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at              TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at              TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    deleted_at              TIMESTAMPTZ
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_gm_drama_episodes_project_episode
    ON gm_drama.episodes(project_id, episode_number);
CREATE INDEX IF NOT EXISTS idx_gm_drama_episodes_deleted_at
    ON gm_drama.episodes(deleted_at);

CREATE TABLE IF NOT EXISTS gm_drama.characters (
    id                      BIGSERIAL PRIMARY KEY,
    project_id              TEXT NOT NULL REFERENCES gm_drama.projects(project_id) ON DELETE CASCADE,
    name                    VARCHAR(100) NOT NULL,
    role                    VARCHAR(50),
    description             TEXT,
    appearance              TEXT,
    personality             TEXT,
    voice_style             VARCHAR(200),
    hero_asset_id           BIGINT,
    reference_images        JSONB NOT NULL DEFAULT '[]'::jsonb,
    seed_value              VARCHAR(100),
    sort_order              INT NOT NULL DEFAULT 0,
    metadata                JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at              TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at              TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    deleted_at              TIMESTAMPTZ
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_gm_drama_characters_project_name
    ON gm_drama.characters(project_id, name);
CREATE INDEX IF NOT EXISTS idx_gm_drama_characters_deleted_at
    ON gm_drama.characters(deleted_at);

CREATE TABLE IF NOT EXISTS gm_drama.scenes (
    id                      BIGSERIAL PRIMARY KEY,
    project_id              TEXT NOT NULL REFERENCES gm_drama.projects(project_id) ON DELETE CASCADE,
    episode_id              BIGINT REFERENCES gm_drama.episodes(id) ON DELETE SET NULL,
    location                VARCHAR(200) NOT NULL,
    time_label              VARCHAR(100) NOT NULL,
    prompt                  TEXT NOT NULL,
    storyboard_count        INT NOT NULL DEFAULT 1,
    image_asset_id          BIGINT,
    status                  VARCHAR(20) NOT NULL DEFAULT 'pending',
    metadata                JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at              TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at              TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    deleted_at              TIMESTAMPTZ
);

CREATE INDEX IF NOT EXISTS idx_gm_drama_scenes_project_id
    ON gm_drama.scenes(project_id);
CREATE INDEX IF NOT EXISTS idx_gm_drama_scenes_episode_id
    ON gm_drama.scenes(episode_id);
CREATE INDEX IF NOT EXISTS idx_gm_drama_scenes_deleted_at
    ON gm_drama.scenes(deleted_at);

CREATE TABLE IF NOT EXISTS gm_drama.props (
    id                      BIGSERIAL PRIMARY KEY,
    project_id              TEXT NOT NULL REFERENCES gm_drama.projects(project_id) ON DELETE CASCADE,
    name                    VARCHAR(100) NOT NULL,
    type                    VARCHAR(50),
    description             TEXT,
    prompt                  TEXT,
    image_asset_id          BIGINT,
    reference_images        JSONB NOT NULL DEFAULT '[]'::jsonb,
    metadata                JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at              TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at              TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    deleted_at              TIMESTAMPTZ
);

CREATE INDEX IF NOT EXISTS idx_gm_drama_props_project_id
    ON gm_drama.props(project_id);
CREATE INDEX IF NOT EXISTS idx_gm_drama_props_deleted_at
    ON gm_drama.props(deleted_at);

CREATE TABLE IF NOT EXISTS gm_drama.storyboards (
    id                      BIGSERIAL PRIMARY KEY,
    episode_id              BIGINT NOT NULL REFERENCES gm_drama.episodes(id) ON DELETE CASCADE,
    scene_id                BIGINT REFERENCES gm_drama.scenes(id) ON DELETE SET NULL,
    storyboard_number       INT NOT NULL,
    title                   VARCHAR(255),
    location                VARCHAR(255),
    time_label              VARCHAR(255),
    shot_type               VARCHAR(100),
    angle                   VARCHAR(100),
    movement                VARCHAR(100),
    action                  TEXT,
    result                  TEXT,
    atmosphere              TEXT,
    image_prompt            TEXT,
    video_prompt            TEXT,
    bgm_prompt              TEXT,
    sound_effect            VARCHAR(255),
    dialogue                TEXT,
    description             TEXT,
    duration_seconds        INT NOT NULL DEFAULT 5,
    composed_image_asset_id BIGINT,
    video_asset_id          BIGINT,
    status                  VARCHAR(20) NOT NULL DEFAULT 'pending',
    metadata                JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at              TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at              TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    deleted_at              TIMESTAMPTZ
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_gm_drama_storyboards_episode_number
    ON gm_drama.storyboards(episode_id, storyboard_number);
CREATE INDEX IF NOT EXISTS idx_gm_drama_storyboards_scene_id
    ON gm_drama.storyboards(scene_id);
CREATE INDEX IF NOT EXISTS idx_gm_drama_storyboards_deleted_at
    ON gm_drama.storyboards(deleted_at);

CREATE TABLE IF NOT EXISTS gm_drama.frame_prompts (
    id                      BIGSERIAL PRIMARY KEY,
    storyboard_id           BIGINT NOT NULL REFERENCES gm_drama.storyboards(id) ON DELETE CASCADE,
    frame_type              VARCHAR(20) NOT NULL,
    prompt                  TEXT NOT NULL,
    description             TEXT,
    layout                  VARCHAR(50),
    created_at              TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at              TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_gm_drama_frame_prompts_storyboard_type
    ON gm_drama.frame_prompts(storyboard_id, frame_type);

CREATE TABLE IF NOT EXISTS gm_drama.character_libraries (
    id                      BIGSERIAL PRIMARY KEY,
    project_id              TEXT REFERENCES gm_drama.projects(project_id) ON DELETE CASCADE,
    name                    VARCHAR(100) NOT NULL,
    category                VARCHAR(50),
    asset_id                BIGINT,
    public_url              TEXT,
    description             TEXT,
    tags                    VARCHAR(500),
    source_type             VARCHAR(20) NOT NULL DEFAULT 'generated',
    metadata                JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at              TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at              TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    deleted_at              TIMESTAMPTZ
);

CREATE INDEX IF NOT EXISTS idx_gm_drama_character_libraries_project_id
    ON gm_drama.character_libraries(project_id);
CREATE INDEX IF NOT EXISTS idx_gm_drama_character_libraries_deleted_at
    ON gm_drama.character_libraries(deleted_at);

CREATE TABLE IF NOT EXISTS gm_drama.ai_service_configs (
    id                      BIGSERIAL PRIMARY KEY,
    service_type            VARCHAR(50) NOT NULL,
    provider                VARCHAR(50),
    name                    VARCHAR(100) NOT NULL,
    base_url                TEXT NOT NULL,
    api_key                 TEXT NOT NULL,
    model                   JSONB NOT NULL DEFAULT '[]'::jsonb,
    endpoint                TEXT,
    query_endpoint          TEXT,
    priority                INT NOT NULL DEFAULT 0,
    is_default              BOOLEAN NOT NULL DEFAULT FALSE,
    is_active               BOOLEAN NOT NULL DEFAULT TRUE,
    settings                JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at              TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at              TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    deleted_at              TIMESTAMPTZ
);

CREATE INDEX IF NOT EXISTS idx_gm_drama_ai_service_configs_service_type
    ON gm_drama.ai_service_configs(service_type);
CREATE INDEX IF NOT EXISTS idx_gm_drama_ai_service_configs_deleted_at
    ON gm_drama.ai_service_configs(deleted_at);

CREATE TABLE IF NOT EXISTS gm_drama.ai_service_providers (
    id                      BIGSERIAL PRIMARY KEY,
    name                    VARCHAR(100) NOT NULL UNIQUE,
    display_name            VARCHAR(100) NOT NULL,
    service_type            VARCHAR(50) NOT NULL,
    default_url             TEXT,
    description             TEXT,
    is_active               BOOLEAN NOT NULL DEFAULT TRUE,
    created_at              TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at              TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    deleted_at              TIMESTAMPTZ
);

CREATE INDEX IF NOT EXISTS idx_gm_drama_ai_service_providers_service_type
    ON gm_drama.ai_service_providers(service_type);
CREATE INDEX IF NOT EXISTS idx_gm_drama_ai_service_providers_deleted_at
    ON gm_drama.ai_service_providers(deleted_at);

CREATE TABLE IF NOT EXISTS gm_drama.timelines (
    id                      BIGSERIAL PRIMARY KEY,
    project_id              TEXT NOT NULL REFERENCES gm_drama.projects(project_id) ON DELETE CASCADE,
    episode_id              BIGINT REFERENCES gm_drama.episodes(id) ON DELETE SET NULL,
    name                    VARCHAR(200) NOT NULL,
    description             TEXT,
    duration_seconds        INT NOT NULL DEFAULT 0,
    fps                     INT NOT NULL DEFAULT 30,
    resolution              VARCHAR(50),
    status                  VARCHAR(20) NOT NULL DEFAULT 'draft',
    created_at              TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at              TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    deleted_at              TIMESTAMPTZ
);

CREATE INDEX IF NOT EXISTS idx_gm_drama_timelines_project_id
    ON gm_drama.timelines(project_id);
CREATE INDEX IF NOT EXISTS idx_gm_drama_timelines_episode_id
    ON gm_drama.timelines(episode_id);
CREATE INDEX IF NOT EXISTS idx_gm_drama_timelines_deleted_at
    ON gm_drama.timelines(deleted_at);

CREATE TABLE IF NOT EXISTS gm_drama.timeline_tracks (
    id                      BIGSERIAL PRIMARY KEY,
    timeline_id             BIGINT NOT NULL REFERENCES gm_drama.timelines(id) ON DELETE CASCADE,
    name                    VARCHAR(100) NOT NULL,
    type                    VARCHAR(20) NOT NULL,
    track_order             INT NOT NULL DEFAULT 0,
    is_locked               BOOLEAN NOT NULL DEFAULT FALSE,
    is_muted                BOOLEAN NOT NULL DEFAULT FALSE,
    volume                  INT DEFAULT 100,
    created_at              TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at              TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    deleted_at              TIMESTAMPTZ
);

CREATE INDEX IF NOT EXISTS idx_gm_drama_timeline_tracks_timeline_id
    ON gm_drama.timeline_tracks(timeline_id);
CREATE INDEX IF NOT EXISTS idx_gm_drama_timeline_tracks_deleted_at
    ON gm_drama.timeline_tracks(deleted_at);

CREATE TABLE IF NOT EXISTS gm_drama.clip_transitions (
    id                      BIGSERIAL PRIMARY KEY,
    type                    VARCHAR(50) NOT NULL,
    duration_ms             INT NOT NULL DEFAULT 500,
    easing                  VARCHAR(50),
    config                  JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at              TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at              TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    deleted_at              TIMESTAMPTZ
);

CREATE INDEX IF NOT EXISTS idx_gm_drama_clip_transitions_deleted_at
    ON gm_drama.clip_transitions(deleted_at);

CREATE TABLE IF NOT EXISTS gm_drama.timeline_clips (
    id                      BIGSERIAL PRIMARY KEY,
    track_id                BIGINT NOT NULL REFERENCES gm_drama.timeline_tracks(id) ON DELETE CASCADE,
    asset_id                BIGINT,
    storyboard_id           BIGINT REFERENCES gm_drama.storyboards(id) ON DELETE SET NULL,
    name                    VARCHAR(200),
    start_time_ms           INT NOT NULL,
    end_time_ms             INT NOT NULL,
    duration_ms             INT NOT NULL,
    trim_start_ms           INT,
    trim_end_ms             INT,
    speed                   DOUBLE PRECISION DEFAULT 1.0,
    volume                  INT,
    is_muted                BOOLEAN NOT NULL DEFAULT FALSE,
    fade_in_ms              INT,
    fade_out_ms             INT,
    transition_in_id        BIGINT REFERENCES gm_drama.clip_transitions(id) ON DELETE SET NULL,
    transition_out_id       BIGINT REFERENCES gm_drama.clip_transitions(id) ON DELETE SET NULL,
    created_at              TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at              TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    deleted_at              TIMESTAMPTZ
);

CREATE INDEX IF NOT EXISTS idx_gm_drama_timeline_clips_track_id
    ON gm_drama.timeline_clips(track_id);
CREATE INDEX IF NOT EXISTS idx_gm_drama_timeline_clips_asset_id
    ON gm_drama.timeline_clips(asset_id);
CREATE INDEX IF NOT EXISTS idx_gm_drama_timeline_clips_storyboard_id
    ON gm_drama.timeline_clips(storyboard_id);
CREATE INDEX IF NOT EXISTS idx_gm_drama_timeline_clips_deleted_at
    ON gm_drama.timeline_clips(deleted_at);

CREATE TABLE IF NOT EXISTS gm_drama.clip_effects (
    id                      BIGSERIAL PRIMARY KEY,
    clip_id                 BIGINT NOT NULL REFERENCES gm_drama.timeline_clips(id) ON DELETE CASCADE,
    type                    VARCHAR(50) NOT NULL,
    name                    VARCHAR(100),
    is_enabled              BOOLEAN NOT NULL DEFAULT TRUE,
    effect_order            INT NOT NULL DEFAULT 0,
    config                  JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at              TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at              TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    deleted_at              TIMESTAMPTZ
);

CREATE INDEX IF NOT EXISTS idx_gm_drama_clip_effects_clip_id
    ON gm_drama.clip_effects(clip_id);
CREATE INDEX IF NOT EXISTS idx_gm_drama_clip_effects_deleted_at
    ON gm_drama.clip_effects(deleted_at);

CREATE TABLE IF NOT EXISTS gm_drama.asset_tags (
    id                      BIGSERIAL PRIMARY KEY,
    name                    TEXT NOT NULL,
    color                   TEXT,
    created_at              TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at              TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    deleted_at              TIMESTAMPTZ
);

CREATE INDEX IF NOT EXISTS idx_gm_drama_asset_tags_deleted_at
    ON gm_drama.asset_tags(deleted_at);

CREATE TABLE IF NOT EXISTS gm_drama.asset_collections (
    id                      BIGSERIAL PRIMARY KEY,
    project_id              TEXT REFERENCES gm_drama.projects(project_id) ON DELETE CASCADE,
    name                    TEXT NOT NULL,
    description             TEXT,
    created_at              TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at              TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    deleted_at              TIMESTAMPTZ
);

CREATE INDEX IF NOT EXISTS idx_gm_drama_asset_collections_project_id
    ON gm_drama.asset_collections(project_id);
CREATE INDEX IF NOT EXISTS idx_gm_drama_asset_collections_deleted_at
    ON gm_drama.asset_collections(deleted_at);

CREATE TABLE IF NOT EXISTS gm_drama.asset_tag_relations (
    asset_id                BIGINT NOT NULL,
    asset_tag_id            BIGINT NOT NULL REFERENCES gm_drama.asset_tags(id) ON DELETE CASCADE,
    created_at              TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (asset_id, asset_tag_id)
);

CREATE INDEX IF NOT EXISTS idx_gm_drama_asset_tag_relations_tag_id
    ON gm_drama.asset_tag_relations(asset_tag_id);

CREATE TABLE IF NOT EXISTS gm_drama.asset_collection_relations (
    asset_id                BIGINT NOT NULL,
    asset_collection_id     BIGINT NOT NULL REFERENCES gm_drama.asset_collections(id) ON DELETE CASCADE,
    created_at              TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (asset_id, asset_collection_id)
);

CREATE INDEX IF NOT EXISTS idx_gm_drama_asset_collection_relations_collection_id
    ON gm_drama.asset_collection_relations(asset_collection_id);

CREATE TABLE IF NOT EXISTS gm_drama.episode_characters (
    episode_id              BIGINT NOT NULL REFERENCES gm_drama.episodes(id) ON DELETE CASCADE,
    character_id            BIGINT NOT NULL REFERENCES gm_drama.characters(id) ON DELETE CASCADE,
    created_at              TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (episode_id, character_id)
);

CREATE TABLE IF NOT EXISTS gm_drama.storyboard_characters (
    storyboard_id           BIGINT NOT NULL REFERENCES gm_drama.storyboards(id) ON DELETE CASCADE,
    character_id            BIGINT NOT NULL REFERENCES gm_drama.characters(id) ON DELETE CASCADE,
    created_at              TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (storyboard_id, character_id)
);

CREATE TABLE IF NOT EXISTS gm_drama.storyboard_props (
    storyboard_id           BIGINT NOT NULL REFERENCES gm_drama.storyboards(id) ON DELETE CASCADE,
    prop_id                 BIGINT NOT NULL REFERENCES gm_drama.props(id) ON DELETE CASCADE,
    created_at              TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (storyboard_id, prop_id)
);

-- ============================================================================
-- hb_worker_runtime: worker execution/runtime tables
-- ============================================================================

CREATE TABLE IF NOT EXISTS hb_worker_runtime.async_tasks (
    id                      TEXT PRIMARY KEY,
    project_id              TEXT,
    resource_kind           VARCHAR(50),
    resource_id             TEXT,
    type                    VARCHAR(50) NOT NULL,
    status                  VARCHAR(20) NOT NULL DEFAULT 'pending',
    progress                INT NOT NULL DEFAULT 0,
    message                 VARCHAR(500) NOT NULL DEFAULT '',
    error                   TEXT,
    result                  JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at              TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at              TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    completed_at            TIMESTAMPTZ
);

CREATE INDEX IF NOT EXISTS idx_hb_worker_async_tasks_project_id
    ON hb_worker_runtime.async_tasks(project_id);
CREATE INDEX IF NOT EXISTS idx_hb_worker_async_tasks_status
    ON hb_worker_runtime.async_tasks(status);

CREATE TABLE IF NOT EXISTS hb_worker_runtime.provider_task_refs (
    id                      BIGSERIAL PRIMARY KEY,
    project_id              TEXT,
    job_id                  BIGINT,
    provider                VARCHAR(100) NOT NULL,
    task_type               VARCHAR(100) NOT NULL,
    provider_task_id        TEXT NOT NULL,
    status                  VARCHAR(50) NOT NULL DEFAULT 'submitted',
    payload                 JSONB NOT NULL DEFAULT '{}'::jsonb,
    last_polled_at          TIMESTAMPTZ,
    created_at              TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at              TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_hb_worker_provider_task_unique
    ON hb_worker_runtime.provider_task_refs(provider, provider_task_id);

CREATE TABLE IF NOT EXISTS hb_worker_runtime.image_generations (
    id                      BIGSERIAL PRIMARY KEY,
    project_id              TEXT NOT NULL,
    episode_id              BIGINT,
    storyboard_id           BIGINT,
    scene_id                BIGINT,
    character_id            BIGINT,
    prop_id                 BIGINT,
    image_type              VARCHAR(20) NOT NULL DEFAULT 'storyboard',
    frame_type              VARCHAR(20),
    provider                VARCHAR(50) NOT NULL,
    prompt                  TEXT NOT NULL,
    negative_prompt         TEXT,
    model                   VARCHAR(100),
    size                    VARCHAR(20),
    quality                 VARCHAR(20),
    style                   VARCHAR(50),
    steps                   INT,
    cfg_scale               DOUBLE PRECISION,
    seed                    BIGINT,
    public_url              TEXT,
    bucket                  VARCHAR(255),
    object_key              TEXT,
    scratch_path            TEXT,
    status                  VARCHAR(20) NOT NULL DEFAULT 'pending',
    provider_task_id        TEXT,
    error_msg               TEXT,
    width                   INT,
    height                  INT,
    reference_images        JSONB NOT NULL DEFAULT '[]'::jsonb,
    created_at              TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at              TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    completed_at            TIMESTAMPTZ
);

CREATE INDEX IF NOT EXISTS idx_hb_worker_image_generations_project_id
    ON hb_worker_runtime.image_generations(project_id);
CREATE INDEX IF NOT EXISTS idx_hb_worker_image_generations_status
    ON hb_worker_runtime.image_generations(status);
CREATE INDEX IF NOT EXISTS idx_hb_worker_image_generations_provider_task_id
    ON hb_worker_runtime.image_generations(provider_task_id);

CREATE TABLE IF NOT EXISTS hb_worker_runtime.video_generations (
    id                      BIGSERIAL PRIMARY KEY,
    project_id              TEXT NOT NULL,
    episode_id              BIGINT,
    storyboard_id           BIGINT,
    source_image_generation_id BIGINT,
    provider                VARCHAR(50) NOT NULL,
    prompt                  TEXT NOT NULL,
    model                   VARCHAR(100),
    reference_mode          VARCHAR(20),
    input_public_url        TEXT,
    first_frame_url         TEXT,
    last_frame_url          TEXT,
    reference_image_urls    JSONB NOT NULL DEFAULT '[]'::jsonb,
    duration_seconds        INT,
    fps                     INT,
    resolution              VARCHAR(50),
    aspect_ratio            VARCHAR(20),
    style                   VARCHAR(100),
    motion_level            INT,
    camera_motion           VARCHAR(100),
    seed                    BIGINT,
    public_url              TEXT,
    bucket                  VARCHAR(255),
    object_key              TEXT,
    scratch_path            TEXT,
    status                  VARCHAR(20) NOT NULL DEFAULT 'pending',
    provider_task_id        TEXT,
    error_msg               TEXT,
    width                   INT,
    height                  INT,
    created_at              TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at              TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    completed_at            TIMESTAMPTZ
);

CREATE INDEX IF NOT EXISTS idx_hb_worker_video_generations_project_id
    ON hb_worker_runtime.video_generations(project_id);
CREATE INDEX IF NOT EXISTS idx_hb_worker_video_generations_status
    ON hb_worker_runtime.video_generations(status);
CREATE INDEX IF NOT EXISTS idx_hb_worker_video_generations_provider_task_id
    ON hb_worker_runtime.video_generations(provider_task_id);

CREATE TABLE IF NOT EXISTS hb_worker_runtime.video_merges (
    id                      BIGSERIAL PRIMARY KEY,
    project_id              TEXT NOT NULL,
    episode_id              BIGINT,
    title                   VARCHAR(200),
    provider                VARCHAR(50) NOT NULL,
    model                   VARCHAR(100),
    status                  VARCHAR(20) NOT NULL DEFAULT 'pending',
    scenes_json             JSONB NOT NULL DEFAULT '[]'::jsonb,
    merged_public_url       TEXT,
    bucket                  VARCHAR(255),
    object_key              TEXT,
    scratch_path            TEXT,
    duration_seconds        INT,
    provider_task_id        TEXT,
    error_msg               TEXT,
    created_at              TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at              TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    completed_at            TIMESTAMPTZ
);

CREATE INDEX IF NOT EXISTS idx_hb_worker_video_merges_project_id
    ON hb_worker_runtime.video_merges(project_id);
CREATE INDEX IF NOT EXISTS idx_hb_worker_video_merges_status
    ON hb_worker_runtime.video_merges(status);

CREATE TABLE IF NOT EXISTS hb_worker_runtime.asset_cache (
    id                      BIGSERIAL PRIMARY KEY,
    project_id              TEXT,
    source_url              TEXT NOT NULL,
    scratch_path            TEXT NOT NULL,
    checksum                VARCHAR(128),
    mime_type               VARCHAR(100),
    size_bytes              BIGINT,
    expires_at              TIMESTAMPTZ,
    created_at              TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_hb_worker_asset_cache_project_id
    ON hb_worker_runtime.asset_cache(project_id);

CREATE TABLE IF NOT EXISTS hb_worker_runtime.merge_runtime_tasks (
    id                      BIGSERIAL PRIMARY KEY,
    project_id              TEXT NOT NULL,
    episode_id              BIGINT,
    merge_id                BIGINT,
    status                  VARCHAR(20) NOT NULL DEFAULT 'pending',
    scratch_dir             TEXT,
    payload                 JSONB NOT NULL DEFAULT '{}'::jsonb,
    result                  JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at              TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at              TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    completed_at            TIMESTAMPTZ
);

CREATE INDEX IF NOT EXISTS idx_hb_worker_merge_runtime_tasks_project_id
    ON hb_worker_runtime.merge_runtime_tasks(project_id);
