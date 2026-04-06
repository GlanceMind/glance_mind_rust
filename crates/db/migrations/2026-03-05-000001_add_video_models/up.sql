-- Add 3 Jimeng video models to production (720P / 1080P / Pro).
-- These were previously only in test seed data and never migrated to production.

INSERT INTO gm_ai_models (id, name, provider, model_key, model_type, cost_multiplier, is_active)
VALUES
  (202, 'Jimeng 3.0 720P',    'jimeng',  'jimeng-video-3.0-720p',   'video', 2.00, true),
  (203, 'Jimeng 3.0 1080P',   'jimeng',  'jimeng-video-3.0-1080p',  'video', 3.00, true),
  (204, 'Jimeng 3.0 Pro',     'jimeng',  'jimeng-video-3.0-pro',    'video', 5.00, true)
ON CONFLICT (id) DO UPDATE SET
  name            = EXCLUDED.name,
  provider        = EXCLUDED.provider,
  model_key       = EXCLUDED.model_key,
  model_type      = EXCLUDED.model_type,
  cost_multiplier = EXCLUDED.cost_multiplier,
  is_active       = EXCLUDED.is_active;
