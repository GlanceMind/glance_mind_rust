-- M4 rollback: restore vidu-* rows to active and remove unified "vidu" row.

UPDATE gm_ai_models SET is_active = true, updated_at = NOW() WHERE model_key LIKE 'vidu-%';
DELETE FROM gm_ai_models WHERE model_key = 'vidu';
