-- Remove request_hash column from gm_openmontage_jobs
ALTER TABLE gm_openmontage_jobs
DROP COLUMN request_hash;
