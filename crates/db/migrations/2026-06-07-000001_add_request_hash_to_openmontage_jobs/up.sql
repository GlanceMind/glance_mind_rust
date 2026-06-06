-- Add request_hash column to gm_openmontage_jobs for idempotency conflict detection
ALTER TABLE gm_openmontage_jobs
ADD COLUMN request_hash VARCHAR(64) NOT NULL DEFAULT '';

COMMENT ON COLUMN gm_openmontage_jobs.request_hash IS 'SHA-256 hash of canonical request body for conflict detection';
