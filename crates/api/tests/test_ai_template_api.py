"""
Module D3 — task-template confirm / cancel live integration tests.

Mirrors test_ai_chat_api.py / test_batch_task_api.py. Requires a RUNNING API
(`API_BASE_URL`) + a reachable Postgres (`DATABASE_URL`). This is the
INTEGRATION GATE for the assistant-integration flow (gate→propose→confirm→batch
create), not a unit test; it is expected to be run in CI / locally with the
server up. The shared `auth_client` / `api_client` / `db_connection` fixtures
come from conftest.py.

Covers:
- POST /ai-chat/conversations/:id/task-template/:draft_id/confirm with edited
  fields against a seeded `proposed` draft ⇒ a gm_campaigns row appears, linked
  by source_draft_id, and the draft ends `confirmed`.
- POST /ai-chat/conversations/:id/task-template/:draft_id/cancel ⇒ the draft
  ends `cancelled` and no campaign is created.
"""
import json
import uuid

import psycopg2
import psycopg2.extras

from conftest import extract_data, resolve_test_user_id

API_PREFIX = "/api/v1/ai-chat"


# ---------------------------------------------------------------------------
# DB helpers (local; mirrors test_ai_chat_api.py)
# ---------------------------------------------------------------------------


def _ensure_clean(conn):
    if conn.info.transaction_status == psycopg2.extensions.TRANSACTION_STATUS_INERROR:
        conn.rollback()


def _q1(conn, sql, params=None):
    _ensure_clean(conn)
    cur = conn.cursor(cursor_factory=psycopg2.extras.RealDictCursor)
    cur.execute(sql, params)
    row = cur.fetchone()
    cur.close()
    return dict(row) if row else None


def _insert_ret(conn, sql, params=None):
    _ensure_clean(conn)
    cur = conn.cursor(cursor_factory=psycopg2.extras.RealDictCursor)
    cur.execute(sql, params)
    row = cur.fetchone()
    conn.commit()
    cur.close()
    return dict(row) if row else None


# ---------------------------------------------------------------------------
# Setup helpers
# ---------------------------------------------------------------------------


def _get_config_ids(api_client):
    """Resolve a (platform_id, region_id, ai_model_id) triple that the campaign
    create path will accept (mirrors test_campaign_api.py)."""
    resp = api_client.get("/api/v1/config/platforms")
    platforms = extract_data(resp.json())

    platform_id = None
    region_id = None
    for platform in platforms:
        candidate = platform["id"]
        rresp = api_client.get(f"/api/v1/config/platforms/{candidate}/regions")
        regions = extract_data(rresp.json())
        if regions:
            platform_id = candidate
            region_id = regions[0]["id"]
            break
    assert platform_id is not None and region_id is not None, (
        "expected at least one platform with configured regions"
    )

    mresp = api_client.get("/api/v1/config/ai-models")
    models = extract_data(mresp.json())
    ai_model_id = models[0]["id"]
    return platform_id, region_id, ai_model_id


def _create_conversation(auth_client):
    resp = auth_client.post(f"{API_PREFIX}/conversations", json={"title": "tmpl test"})
    assert resp.status_code == 200, f"create conversation failed: {resp.text}"
    return extract_data(resp.json())["id"]


def _seed_proposed_campaign_draft(conn, conv_id, user_id):
    """Insert a `proposed` campaign draft directly and return its UUID."""
    row = _insert_ret(
        conn,
        """
        INSERT INTO gm_ai_task_template_drafts
            (conversation_id, user_id, task_kind, draft_config, sample_source, status)
        VALUES (%s, %s, 'campaign', %s, 'ai_generated', 'proposed')
        RETURNING id
        """,
        (conv_id, user_id, json.dumps({"name": "seed"})),
    )
    return row["id"]


# ---------------------------------------------------------------------------
# Tests
# ---------------------------------------------------------------------------


def test_confirm_creates_task(auth_client, api_client, db_connection):
    """A confirm with edited fields against a seeded `proposed` draft creates a
    gm_campaigns row (linked by source_draft_id) and moves the draft to
    `confirmed`."""
    user_id = resolve_test_user_id(db_connection)
    platform_id, region_id, ai_model_id = _get_config_ids(api_client)

    conv_id = _create_conversation(auth_client)
    draft_id = _seed_proposed_campaign_draft(db_connection, conv_id, user_id)

    unique = uuid.uuid4().hex[:8]
    campaign_name = f"Confirmed Template Campaign {unique}"
    body = {
        "edited_fields": [
            {"key": "name", "value": campaign_name},
            {"key": "platform_id", "value": platform_id},
            {"key": "region_id", "value": region_id},
            {"key": "ai_model_id", "value": ai_model_id},
            {"key": "schedule_type", "value": "ONCE"},
            {"key": "product_prompt", "value": f"Edited product {unique}"},
            {"key": "max_scan_count", "value": 1},
        ]
    }

    resp = auth_client.post(
        f"{API_PREFIX}/conversations/{conv_id}/task-template/{draft_id}/confirm",
        json=body,
    )
    assert resp.status_code == 200, (
        f"confirm must succeed, got {resp.status_code}: {resp.text}"
    )

    # A gm_campaigns row, linked back to the draft, must now exist with the
    # edited name.
    campaign = _q1(
        db_connection,
        "SELECT id, name, source_draft_id FROM gm_campaigns WHERE source_draft_id = %s",
        (draft_id,),
    )
    assert campaign is not None, (
        f"a gm_campaigns row linked by source_draft_id={draft_id} must exist after confirm"
    )
    assert campaign["name"] == campaign_name, (
        f"the created campaign must carry the edited name, got {campaign['name']!r}"
    )

    # The draft must end `confirmed`, stamped with the created entity id.
    draft = _q1(
        db_connection,
        "SELECT status, created_entity_id FROM gm_ai_task_template_drafts WHERE id = %s",
        (draft_id,),
    )
    assert draft["status"] == "confirmed", (
        f"a successful confirm must move the draft to `confirmed`, got {draft['status']!r}"
    )
    assert draft["created_entity_id"] == campaign["id"], (
        "the draft must be stamped with the created campaign id"
    )


def test_cancel_marks_cancelled(auth_client, db_connection):
    """Cancel moves a `proposed` draft to `cancelled` and creates no campaign."""
    user_id = resolve_test_user_id(db_connection)
    conv_id = _create_conversation(auth_client)
    draft_id = _seed_proposed_campaign_draft(db_connection, conv_id, user_id)

    resp = auth_client.post(
        f"{API_PREFIX}/conversations/{conv_id}/task-template/{draft_id}/cancel",
    )
    assert resp.status_code == 200, (
        f"cancel must succeed, got {resp.status_code}: {resp.text}"
    )

    draft = _q1(
        db_connection,
        "SELECT status FROM gm_ai_task_template_drafts WHERE id = %s",
        (draft_id,),
    )
    assert draft["status"] == "cancelled", (
        f"cancel must move the draft to `cancelled`, got {draft['status']!r}"
    )

    # No campaign may have been created for this draft.
    campaign = _q1(
        db_connection,
        "SELECT id FROM gm_campaigns WHERE source_draft_id = %s",
        (draft_id,),
    )
    assert campaign is None, "cancel must NOT create a campaign for the draft"
