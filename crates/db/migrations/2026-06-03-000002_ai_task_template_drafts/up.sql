-- Module D2: task-template draft lifecycle.
--
-- A draft is the persisted proposal of an assistant-assembled task config
-- (campaign or publish_plan). It carries the draft_config JSON (write-once),
-- the sample provenance, and a state-machine `status` that moves
-- proposed -> confirming -> confirmed (or -> cancelled/expired/superseded).
-- The CAS-style transitions (UPDATE ... WHERE id=? AND status=from) make
-- concurrent confirms safe: exactly one wins. `created_entity_id`/`result`
-- are filled in only when a draft reaches `confirmed`.
CREATE TABLE gm_ai_task_template_drafts (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    conversation_id INT NOT NULL REFERENCES gm_ai_conversations(id) ON DELETE CASCADE,
    message_id INT REFERENCES gm_ai_messages(id) ON DELETE SET NULL,
    user_id INT NOT NULL,
    task_kind TEXT NOT NULL CHECK (task_kind IN ('campaign','publish_plan')),
    draft_config JSONB NOT NULL,
    sample_source TEXT NOT NULL CHECK (sample_source IN ('ai_generated','preset','hybrid')),
    status TEXT NOT NULL DEFAULT 'proposed' CHECK (status IN ('proposed','confirming','confirmed','cancelled','expired','superseded')),
    created_entity_id INT,
    result JSONB,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX idx_tt_drafts_conv ON gm_ai_task_template_drafts(conversation_id, status);

-- Link the underlying created resource back to the draft that produced it.
-- The partial-unique index makes a draft -> entity creation idempotent: a
-- second create stamped with the same source_draft_id is rejected.
ALTER TABLE gm_campaigns ADD COLUMN source_draft_id UUID;
ALTER TABLE gm_aipub_plans ADD COLUMN source_draft_id UUID;
CREATE UNIQUE INDEX uq_campaigns_source_draft ON gm_campaigns(source_draft_id) WHERE source_draft_id IS NOT NULL;
CREATE UNIQUE INDEX uq_aipub_plans_source_draft ON gm_aipub_plans(source_draft_id) WHERE source_draft_id IS NOT NULL;
