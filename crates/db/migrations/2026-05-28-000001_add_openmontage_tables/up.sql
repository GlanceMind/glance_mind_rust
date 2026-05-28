-- =============================================================================
-- OpenMontage Professional Video Module - Database Migration
-- =============================================================================

-- -----------------------------------------------------------------------------
-- 1. OpenMontage Jobs Table
-- -----------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS public.gm_openmontage_jobs (
    id SERIAL PRIMARY KEY,
    job_id VARCHAR(200) NOT NULL UNIQUE,
    project_id VARCHAR(200) NOT NULL,
    user_id INTEGER NOT NULL,
    tenant_id VARCHAR(200) NOT NULL,
    request_id VARCHAR(200) NOT NULL,
    idempotency_key VARCHAR(200) NOT NULL UNIQUE,

    -- Request configuration
    pipeline VARCHAR(100) NOT NULL,
    input_mode VARCHAR(50),
    status VARCHAR(50) NOT NULL,
    cancel_requested BOOLEAN DEFAULT false NOT NULL,

    -- Progress tracking
    current_stage VARCHAR(100),
    progress_pct INTEGER DEFAULT 0 NOT NULL,

    -- Execution config (from request)
    render_runtime VARCHAR(50),
    approval_policy VARCHAR(50),
    budget_limit_usd DECIMAL(10, 2),

    -- Event sequencing (append-only log)
    last_event_sequence BIGINT DEFAULT 0 NOT NULL,
    next_event_sequence BIGINT DEFAULT 1 NOT NULL,
    sync_required BOOLEAN DEFAULT false NOT NULL,

    -- Snapshots
    snapshot_json JSONB NOT NULL,
    error_json JSONB,

    -- Timestamps
    created_at TIMESTAMPTZ DEFAULT NOW() NOT NULL,
    updated_at TIMESTAMPTZ,

    CONSTRAINT gm_openmontage_jobs_progress_pct_check CHECK (progress_pct >= 0 AND progress_pct <= 100)
);

-- Indexes
CREATE INDEX idx_openmontage_jobs_user ON gm_openmontage_jobs(user_id);
CREATE INDEX idx_openmontage_jobs_status ON gm_openmontage_jobs(status);
CREATE INDEX idx_openmontage_jobs_tenant ON gm_openmontage_jobs(tenant_id);
CREATE INDEX idx_openmontage_jobs_created_at ON gm_openmontage_jobs(created_at DESC);

-- Auto-update updated_at
SELECT diesel_manage_updated_at('gm_openmontage_jobs');

-- -----------------------------------------------------------------------------
-- 2. OpenMontage Job Events Table (append-only event log)
-- -----------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS public.gm_openmontage_job_events (
    id SERIAL PRIMARY KEY,
    job_id VARCHAR(200) NOT NULL,
    sequence BIGINT NOT NULL,
    event_id VARCHAR(200) NOT NULL UNIQUE,
    event_type VARCHAR(100) NOT NULL,

    -- Event payload
    status VARCHAR(50),
    stage VARCHAR(100),
    progress_pct INTEGER,
    event_json JSONB NOT NULL,

    -- Timestamps
    emitted_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ DEFAULT NOW() NOT NULL,

    CONSTRAINT gm_openmontage_job_events_unique_seq UNIQUE(job_id, sequence),
    CONSTRAINT gm_openmontage_job_events_progress_check CHECK (progress_pct IS NULL OR (progress_pct >= 0 AND progress_pct <= 100))
);

-- Indexes
CREATE INDEX idx_openmontage_job_events_job ON gm_openmontage_job_events(job_id);
CREATE INDEX idx_openmontage_job_events_sequence ON gm_openmontage_job_events(job_id, sequence);
CREATE INDEX idx_openmontage_job_events_created_at ON gm_openmontage_job_events(created_at DESC);

-- -----------------------------------------------------------------------------
-- 3. OpenMontage Assets Table
-- -----------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS public.gm_openmontage_assets (
    id SERIAL PRIMARY KEY,
    asset_id VARCHAR(200) NOT NULL UNIQUE,
    user_id INTEGER NOT NULL,

    -- Asset metadata
    kind VARCHAR(50) NOT NULL,
    role VARCHAR(50) NOT NULL,
    uri VARCHAR(1000) NOT NULL,
    mime_type VARCHAR(100),
    bytes BIGINT,

    -- Media dimensions
    width_px INTEGER,
    height_px INTEGER,
    duration_ms INTEGER,

    -- Timestamps
    created_at TIMESTAMPTZ DEFAULT NOW() NOT NULL,

    CONSTRAINT gm_openmontage_assets_dimensions_check CHECK (
        (width_px IS NULL OR width_px > 0) AND
        (height_px IS NULL OR height_px > 0) AND
        (duration_ms IS NULL OR duration_ms > 0)
    )
);

-- Indexes
CREATE INDEX idx_openmontage_assets_user ON gm_openmontage_assets(user_id);
CREATE INDEX idx_openmontage_assets_kind ON gm_openmontage_assets(kind);
CREATE INDEX idx_openmontage_assets_created_at ON gm_openmontage_assets(created_at DESC);

-- -----------------------------------------------------------------------------
-- Comments
-- -----------------------------------------------------------------------------
COMMENT ON TABLE gm_openmontage_jobs IS 'OpenMontage professional video jobs';
COMMENT ON TABLE gm_openmontage_job_events IS 'OpenMontage job event log (append-only)';
COMMENT ON TABLE gm_openmontage_assets IS 'OpenMontage input/output assets';

COMMENT ON COLUMN gm_openmontage_jobs.job_id IS 'Unique job identifier (client-facing)';
COMMENT ON COLUMN gm_openmontage_jobs.project_id IS 'OpenMontage project ID (format: omx-{job_id})';
COMMENT ON COLUMN gm_openmontage_jobs.idempotency_key IS 'Idempotency key for duplicate detection';
COMMENT ON COLUMN gm_openmontage_jobs.last_event_sequence IS 'Last received event sequence';
COMMENT ON COLUMN gm_openmontage_jobs.next_event_sequence IS 'Expected next event sequence';
COMMENT ON COLUMN gm_openmontage_jobs.sync_required IS 'Set true when gap detected in event sequence';

COMMENT ON COLUMN gm_openmontage_job_events.sequence IS 'Event sequence number (monotonic per job)';
COMMENT ON COLUMN gm_openmontage_job_events.event_id IS 'Unique event identifier (for idempotency)';
COMMENT ON COLUMN gm_openmontage_job_events.event_type IS 'Event type (status_change, progress, artifact, etc.)';
