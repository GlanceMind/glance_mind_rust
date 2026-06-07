-- Revert M0b-T1: Restore single-column UNIQUE on idempotency_key

-- Drop composite UNIQUE constraint
ALTER TABLE gm_openmontage_jobs
DROP CONSTRAINT gm_openmontage_jobs_user_idempotency_key;

-- Re-add single-column UNIQUE on idempotency_key
ALTER TABLE gm_openmontage_jobs
ADD CONSTRAINT gm_openmontage_jobs_idempotency_key_key UNIQUE (idempotency_key);
