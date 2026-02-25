-- Add bitmask permissions column to gm_users
-- Bit layout:
--   bit 0 (1):  dm_control     (default OFF)
--   bit 1 (2):  ai_publish     (default ON)
--   bit 2 (4):  ai_content_gen (default ON)
--   bit 3 (8):  ai_lead_gen    (default ON)
-- Default value = 2+4+8 = 14
ALTER TABLE gm_users ADD COLUMN permissions BIGINT NOT NULL DEFAULT 14;

-- Drop the old relational permissions table
DROP TABLE IF EXISTS gm_user_permissions;
