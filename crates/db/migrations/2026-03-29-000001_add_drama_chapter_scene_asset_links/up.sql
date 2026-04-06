CREATE TABLE IF NOT EXISTS drama_chapter_scene_asset_links (
    project_id TEXT NOT NULL,
    chapter_id TEXT NOT NULL,
    user_id BIGINT NOT NULL REFERENCES gm_users(id) ON DELETE CASCADE,
    scene_asset_id TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (project_id, chapter_id, scene_asset_id),
    CONSTRAINT fk_drama_chapter_scene_asset_links_scene_asset_user
        FOREIGN KEY (scene_asset_id, user_id)
        REFERENCES drama_private_scene_assets(id, user_id)
        ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_drama_chapter_scene_asset_links_user_id
    ON drama_chapter_scene_asset_links(user_id);

CREATE INDEX IF NOT EXISTS idx_drama_chapter_scene_asset_links_project_chapter
    ON drama_chapter_scene_asset_links(project_id, chapter_id);
