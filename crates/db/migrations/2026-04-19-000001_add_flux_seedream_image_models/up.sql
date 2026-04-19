-- Seed Flux Kontext (Pro / Max) and SeeDream (4.0 / 4.5) image models for the
-- new image_providers V2 path in glance_mind_scheduler.
--
-- cost_multiplier baseline: gpt-4o-image = 1.00 (~ $0.025/img wholesale).
-- Multipliers below are derived from current LaoZhang public pricing
-- (https://docs.laozhang.ai/api-capabilities/flux-image-generation
--  https://docs.laozhang.ai/api-capabilities/seedream-image):
--   - Flux Kontext Pro:  $0.035/img -> 1.40
--   - Flux Kontext Max:  $0.070/img -> 2.80
--   - SeeDream 4.0:      $0.035/img -> 1.40
--   - SeeDream 4.5:      $0.045/img -> 1.80
-- Adjust if upstream rates shift; downstream consumers read this column.

INSERT INTO gm_ai_models (name, provider, model_key, model_type, cost_multiplier, is_active, created_at)
VALUES
    ('Flux Kontext Pro', 'laozhang', 'flux-kontext-pro',     'image', 1.40, true, NOW()),
    ('Flux Kontext Max', 'laozhang', 'flux-kontext-max',     'image', 2.80, true, NOW()),
    ('SeeDream 4.0',     'laozhang', 'seedream-4-0-250828',  'image', 1.40, true, NOW()),
    ('SeeDream 4.5',     'laozhang', 'seedream-4-5-251128',  'image', 1.80, true, NOW())
ON CONFLICT DO NOTHING;
