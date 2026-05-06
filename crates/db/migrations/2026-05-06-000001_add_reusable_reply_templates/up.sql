CREATE TABLE IF NOT EXISTS gm_reply_template_library (
    id SERIAL PRIMARY KEY,
    user_id INTEGER NOT NULL REFERENCES gm_users(id) ON DELETE CASCADE,
    name VARCHAR(255) NOT NULL,
    description TEXT,
    weight INTEGER NOT NULL DEFAULT 50,
    dm_prompt TEXT,
    reply_prompt TEXT,
    reply_post_prompt TEXT,
    usage_count INTEGER NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ
);

CREATE INDEX IF NOT EXISTS idx_reply_template_library_user_id
    ON gm_reply_template_library(user_id);

ALTER TABLE gm_campaign_templates
    ADD COLUMN IF NOT EXISTS library_template_id INTEGER
    REFERENCES gm_reply_template_library(id) ON DELETE SET NULL;

CREATE INDEX IF NOT EXISTS idx_campaign_templates_library_template_id
    ON gm_campaign_templates(library_template_id);

CREATE UNIQUE INDEX IF NOT EXISTS idx_campaign_templates_campaign_library_unique
    ON gm_campaign_templates(campaign_id, library_template_id)
    WHERE library_template_id IS NOT NULL;

DO $$
DECLARE
    template_row RECORD;
    new_library_id INTEGER;
BEGIN
    FOR template_row IN
        SELECT
            t.id,
            t.name,
            t.weight,
            t.dm_prompt,
            t.reply_prompt,
            t.reply_post_prompt,
            t.created_at,
            t.updated_at,
            c.user_id,
            c.name AS campaign_name
        FROM gm_campaign_templates t
        JOIN gm_campaigns c ON c.id = t.campaign_id
        WHERE t.library_template_id IS NULL
    LOOP
        INSERT INTO gm_reply_template_library (
            user_id,
            name,
            description,
            weight,
            dm_prompt,
            reply_prompt,
            reply_post_prompt,
            usage_count,
            created_at,
            updated_at
        )
        VALUES (
            template_row.user_id,
            COALESCE(template_row.name, 'Template #' || template_row.id),
            'Migrated from campaign: ' || template_row.campaign_name,
            template_row.weight,
            template_row.dm_prompt,
            template_row.reply_prompt,
            template_row.reply_post_prompt,
            1,
            template_row.created_at,
            template_row.updated_at
        )
        RETURNING id INTO new_library_id;

        UPDATE gm_campaign_templates
        SET library_template_id = new_library_id
        WHERE id = template_row.id;
    END LOOP;
END $$;

CREATE OR REPLACE VIEW gm_resolved_campaign_templates AS
SELECT
    campaign.id,
    campaign.campaign_id,
    campaign.library_template_id,
    campaign.weight,
    CASE
        WHEN library.id IS NOT NULL THEN library.reply_prompt
        ELSE campaign.reply_prompt
    END AS reply_prompt,
    campaign.created_at,
    campaign.updated_at,
    CASE
        WHEN library.id IS NOT NULL THEN library.dm_prompt
        ELSE campaign.dm_prompt
    END AS dm_prompt,
    CASE
        WHEN library.id IS NOT NULL THEN library.reply_post_prompt
        ELSE campaign.reply_post_prompt
    END AS reply_post_prompt,
    CASE
        WHEN library.id IS NOT NULL THEN library.name
        ELSE campaign.name
    END AS name
FROM gm_campaign_templates campaign
LEFT JOIN gm_reply_template_library library
    ON library.id = campaign.library_template_id;
