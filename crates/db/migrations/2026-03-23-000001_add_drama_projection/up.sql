-- ============================================================================
-- Drama Projection Tables
-- ============================================================================
-- Supports H1: Full Rust-owned user-visible projection for drama projects.
-- These tables are written by glance_mind_rust when it ingests callback events
-- from gm_agent_hub, and read by the public drama API to serve front-end polls.
--
-- gm_agent_hub still owns gm_video_* tables for workflow runtime truth.
-- These projection tables are the API-owned user-visible truth.
-- ============================================================================

-- 1. Drama project projection — one row per project, updated on every event
CREATE TABLE IF NOT EXISTS gm_drama_project_projections (
    id                    SERIAL PRIMARY KEY,
    project_id            VARCHAR(255) NOT NULL UNIQUE,
    user_id               INT NOT NULL,
    title                 VARCHAR(500) NOT NULL DEFAULT '',
    description           TEXT NOT NULL DEFAULT '',
    status                VARCHAR(50) NOT NULL DEFAULT 'pending',
    content_type          VARCHAR(50),
    platform              VARCHAR(50),
    current_stage         VARCHAR(50),
    pending_stage         VARCHAR(50),
    progress_percent      REAL NOT NULL DEFAULT 0,
    run_id                VARCHAR(255),
    interaction_version   INT NOT NULL DEFAULT 1,
    last_event_sequence   BIGINT NOT NULL DEFAULT 0,
    error_message         TEXT,
    cost_reserve_cents    BIGINT NOT NULL DEFAULT 0,
    cost_consumed_cents   BIGINT NOT NULL DEFAULT 0,
    interaction_payload   JSONB,
    created_at            TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at            TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    completed_at          TIMESTAMPTZ
);

CREATE INDEX IF NOT EXISTS idx_drama_proj_user ON gm_drama_project_projections(user_id);
CREATE INDEX IF NOT EXISTS idx_drama_proj_status ON gm_drama_project_projections(status);

-- 2. Drama callback event log — append-only, used for deduplication and replay
CREATE TABLE IF NOT EXISTS gm_drama_callback_events (
    id                    BIGSERIAL PRIMARY KEY,
    event_id              VARCHAR(255) NOT NULL UNIQUE,
    project_id            VARCHAR(255) NOT NULL,
    run_id                VARCHAR(255) NOT NULL,
    sequence              BIGINT NOT NULL,
    stage_code            VARCHAR(50),
    event_type            VARCHAR(100) NOT NULL,
    payload               JSONB NOT NULL DEFAULT '{}',
    occurred_at           TIMESTAMPTZ NOT NULL,
    ingested_at           TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_drama_cb_project ON gm_drama_callback_events(project_id, run_id, sequence);

-- 3. Drama cost events — append-only cost ledger owned by the API
CREATE TABLE IF NOT EXISTS gm_drama_cost_events (
    id                    BIGSERIAL PRIMARY KEY,
    project_id            VARCHAR(255) NOT NULL,
    run_id                VARCHAR(255),
    cost_type             VARCHAR(100) NOT NULL,
    amount_cents          BIGINT NOT NULL DEFAULT 0,
    stage_code            VARCHAR(50),
    provider              VARCHAR(100),
    event_id              VARCHAR(255),
    created_at            TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_drama_cost_project ON gm_drama_cost_events(project_id);
