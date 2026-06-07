-- Revert M0b-T1: Restore single-column UNIQUE on idempotency_key

-- Drop composite UNIQUE constraint (robust: IF EXISTS)
ALTER TABLE gm_openmontage_jobs
DROP CONSTRAINT IF EXISTS gm_openmontage_jobs_user_idempotency_key;

-- Re-add single-column UNIQUE on idempotency_key with the original constraint name
-- (matches the auto-generated name from the original migration's `idempotency_key VARCHAR(200) NOT NULL UNIQUE`)
ALTER TABLE gm_openmontage_jobs
ADD CONSTRAINT gm_openmontage_jobs_idempotency_key_key UNIQUE (idempotency_key);
