-- ============================================================================
-- GlanceMind Database - Existing Database Migration Script
-- ============================================================================
-- 
-- Purpose: Run on existing production/staging databases to replace old
--          migration records with the new baseline migration record.
--
-- Usage:
--   psql $DATABASE_URL -f scripts/migrate_existing_db.sql
--
-- Or:
--   cd glance_mind_db
--   ./scripts/migrate-existing-db.sh
--
-- ============================================================================

-- Start transaction
BEGIN;

-- Show current status
SELECT '=== Current Migration Records ===' AS info;
SELECT COUNT(*) AS total_migrations FROM __diesel_schema_migrations;

-- Backup old migration records (optional)
-- CREATE TABLE IF NOT EXISTS __diesel_schema_migrations_backup AS 
-- SELECT * FROM __diesel_schema_migrations;

-- Delete all old migration records
DELETE FROM __diesel_schema_migrations;

-- Insert baseline migration record
-- Note: Version format must match Diesel generated format
INSERT INTO __diesel_schema_migrations (version, run_on) 
VALUES ('20260120134354', NOW());

-- Show new status
SELECT '=== Updated Migration Records ===' AS info;
SELECT * FROM __diesel_schema_migrations;

-- Commit transaction
COMMIT;

SELECT 'Migration completed! Existing database is now marked as baseline.' AS result;
