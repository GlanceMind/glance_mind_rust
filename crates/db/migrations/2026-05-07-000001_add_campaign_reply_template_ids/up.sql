ALTER TABLE gm_campaigns
    ADD COLUMN IF NOT EXISTS reply_template_ids INTEGER[] NOT NULL DEFAULT '{}'::INTEGER[];

ALTER TABLE gm_campaigns
    ADD CONSTRAINT chk_campaign_reply_template_ids_sane
    CHECK (
        array_position(reply_template_ids, NULL) IS NULL
        AND cardinality(reply_template_ids) <= 100
    );

CREATE INDEX IF NOT EXISTS idx_campaigns_reply_template_ids_gin
    ON gm_campaigns USING GIN (reply_template_ids);

WITH ranked_bindings AS (
    SELECT DISTINCT ON (campaign_id, library_template_id)
        campaign_id,
        library_template_id,
        weight,
        id
    FROM gm_campaign_templates
    WHERE library_template_id IS NOT NULL
    ORDER BY campaign_id, library_template_id, weight DESC, id ASC
),
aggregated AS (
    SELECT
        campaign_id,
        ARRAY_AGG(library_template_id ORDER BY weight DESC, id ASC) AS ids
    FROM ranked_bindings
    GROUP BY campaign_id
)
UPDATE gm_campaigns campaign
SET reply_template_ids = aggregated.ids
FROM aggregated
WHERE campaign.id = aggregated.campaign_id
  AND cardinality(campaign.reply_template_ids) = 0;
