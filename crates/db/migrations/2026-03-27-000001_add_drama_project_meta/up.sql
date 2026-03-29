CREATE TABLE IF NOT EXISTS drama_project_meta (
    project_id TEXT PRIMARY KEY,
    user_id BIGINT NOT NULL REFERENCES gm_users(id) ON DELETE CASCADE,
    characters JSONB NOT NULL DEFAULT '[]'::jsonb,
    style_references JSONB NOT NULL DEFAULT '[]'::jsonb,
    text_materials JSONB NOT NULL DEFAULT '[]'::jsonb,
    visual_settings JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_drama_project_meta_user_id
    ON drama_project_meta(user_id);
