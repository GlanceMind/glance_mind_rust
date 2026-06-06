-- Module C: write-ahead idempotency ledger for the batch-create API.
-- Each row is inserted (status='in_progress') before any sub-task is created,
-- and updated to 'completed' (with the serialized BatchCreateResultDto in
-- `result`) once the batch finishes. The UNIQUE(user_id, idempotency_key)
-- constraint is what makes a replayed request deduplicate: a second
-- `begin` hits a unique violation and the service reads the existing row.
CREATE TABLE gm_ai_batch_creates (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id INT NOT NULL,
    idempotency_key TEXT NOT NULL,
    task_kind TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'in_progress' CHECK (status IN ('in_progress', 'completed')),
    result JSONB,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (user_id, idempotency_key)
);
