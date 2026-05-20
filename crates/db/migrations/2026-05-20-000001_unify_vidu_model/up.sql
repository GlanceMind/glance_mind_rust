-- M4: Unify vidu-* rows into a single canonical "vidu" model row.
-- Additive and non-destructive: legacy rows are kept but deactivated.

INSERT INTO gm_ai_models (name, provider, model_key, cost_multiplier, is_active, model_type, created_at)
SELECT 'Vidu', 'vidu', 'vidu', 1.0, true, 'video', NOW()
WHERE NOT EXISTS (SELECT 1 FROM gm_ai_models WHERE model_key = 'vidu');

UPDATE gm_ai_models SET is_active = false, updated_at = NOW()
WHERE model_key LIKE 'vidu-%';
