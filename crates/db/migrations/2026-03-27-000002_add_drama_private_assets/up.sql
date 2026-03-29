CREATE TABLE IF NOT EXISTS drama_private_characters (
    id TEXT PRIMARY KEY,
    user_id BIGINT NOT NULL REFERENCES gm_users(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    gender TEXT,
    age TEXT,
    appearance TEXT,
    personality TEXT,
    voice_id TEXT,
    reference_image_url TEXT,
    notes TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT uq_drama_private_characters_id_user_id UNIQUE (id, user_id)
);

CREATE INDEX IF NOT EXISTS idx_drama_private_characters_user_id
    ON drama_private_characters(user_id);

CREATE TABLE IF NOT EXISTS drama_private_scene_assets (
    id TEXT PRIMARY KEY,
    user_id BIGINT NOT NULL REFERENCES gm_users(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    category TEXT,
    location_description TEXT,
    time_of_day TEXT,
    mood TEXT,
    reference_image_urls JSONB NOT NULL DEFAULT '[]'::jsonb,
    camera_notes TEXT,
    notes TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT uq_drama_private_scene_assets_id_user_id UNIQUE (id, user_id)
);

CREATE INDEX IF NOT EXISTS idx_drama_private_scene_assets_user_id
    ON drama_private_scene_assets(user_id);

CREATE TABLE IF NOT EXISTS drama_private_style_assets (
    id TEXT PRIMARY KEY,
    user_id BIGINT NOT NULL REFERENCES gm_users(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    visual_style TEXT,
    color_tone TEXT,
    aspect_ratio TEXT,
    resolution TEXT,
    lighting_mood TEXT,
    reference_image_urls JSONB NOT NULL DEFAULT '[]'::jsonb,
    notes TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT uq_drama_private_style_assets_id_user_id UNIQUE (id, user_id)
);

CREATE INDEX IF NOT EXISTS idx_drama_private_style_assets_user_id
    ON drama_private_style_assets(user_id);

CREATE TABLE IF NOT EXISTS drama_project_character_links (
    project_id TEXT NOT NULL,
    user_id BIGINT NOT NULL REFERENCES gm_users(id) ON DELETE CASCADE,
    character_id TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (project_id, character_id),
    CONSTRAINT fk_drama_project_character_links_character_user
        FOREIGN KEY (character_id, user_id)
        REFERENCES drama_private_characters(id, user_id)
        ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_drama_project_character_links_user_id
    ON drama_project_character_links(user_id);

CREATE TABLE IF NOT EXISTS drama_project_scene_asset_links (
    project_id TEXT NOT NULL,
    user_id BIGINT NOT NULL REFERENCES gm_users(id) ON DELETE CASCADE,
    scene_asset_id TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (project_id, scene_asset_id),
    CONSTRAINT fk_drama_project_scene_asset_links_scene_asset_user
        FOREIGN KEY (scene_asset_id, user_id)
        REFERENCES drama_private_scene_assets(id, user_id)
        ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_drama_project_scene_asset_links_user_id
    ON drama_project_scene_asset_links(user_id);

CREATE TABLE IF NOT EXISTS drama_project_style_asset_links (
    project_id TEXT NOT NULL,
    user_id BIGINT NOT NULL REFERENCES gm_users(id) ON DELETE CASCADE,
    style_asset_id TEXT NOT NULL,
    is_primary BOOLEAN NOT NULL DEFAULT FALSE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (project_id, style_asset_id),
    CONSTRAINT fk_drama_project_style_asset_links_style_asset_user
        FOREIGN KEY (style_asset_id, user_id)
        REFERENCES drama_private_style_assets(id, user_id)
        ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_drama_project_style_asset_links_user_id
    ON drama_project_style_asset_links(user_id);

CREATE UNIQUE INDEX IF NOT EXISTS uq_drama_project_style_asset_links_primary_per_project
    ON drama_project_style_asset_links(project_id)
    WHERE is_primary = TRUE;
