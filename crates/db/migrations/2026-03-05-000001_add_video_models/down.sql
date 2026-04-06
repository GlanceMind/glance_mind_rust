-- Deactivate Jimeng video models added by this migration
UPDATE gm_ai_models
SET is_active = false
WHERE id IN (202, 203, 204);
