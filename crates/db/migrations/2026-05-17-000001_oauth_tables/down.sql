-- Rollback OAuth tables migration
DROP TABLE IF EXISTS ota_config;
DROP TABLE IF EXISTS oauth_audit_log;
DROP TABLE IF EXISTS oauth_refresh_tokens;
DROP TABLE IF EXISTS oauth_codes;
