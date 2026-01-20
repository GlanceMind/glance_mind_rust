-- Baseline migration down
-- WARNING: This will drop ALL tables! Use with extreme caution!
-- In production, you should NEVER run this.

-- This is intentionally left mostly empty to prevent accidental data loss.
-- If you truly need to revert the baseline (which would destroy all data),
-- uncomment the DROP statements below.

-- DROP SCHEMA public CASCADE;
-- CREATE SCHEMA public;

SELECT 'Baseline revert is disabled for safety. Edit down.sql if you really need to drop all tables.';
