-- Revert Flux Kontext + SeeDream image model seeds.
DELETE FROM gm_ai_models WHERE model_key IN (
    'flux-kontext-pro',
    'flux-kontext-max',
    'seedream-4-0-250828',
    'seedream-4-5-251128'
);
