-- M0b-T1: Make idempotency concurrency-safe via composite UNIQUE constraint
-- Drop the single-column UNIQUE on idempotency_key and add composite UNIQUE on (user_id, idempotency_key)
-- This prevents two different users from colliding on the same key, and enables atomic upsert.

-- Drop existing UNIQUE constraint on idempotency_key
ALTER TABLE gm_openmontage_jobs
DROP CONSTRAINT gm_openmontage_jobs_idempotency_key_key;

-- Add composite UNIQUE constraint on (user_id, idempotency_key)
ALTER TABLE gm_openmontage_jobs
ADD CONSTRAINT gm_openmontage_jobs_user_idempotency_key UNIQUE (user_id, idempotency_key);

COMMENT ON CONSTRAINT gm_openmontage_jobs_user_idempotency_key ON gm_openmontage_jobs IS
  'Composite unique constraint on (user_id, idempotency_key) for concurrency-safe idempotency. Two users may reuse the same key.';
