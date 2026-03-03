"""
AI Chat Mode API Integration Tests.

Covers:
- Conversation CRUD with full field validation
- Message sending with SSE streaming
- Plan lifecycle: create (DB) → confirm (SSE) → verify execution
- Plan cancel and step update
- Security: cross-user isolation, message length limit, sensitive field redaction
- Conversation history ordering
- Cascade delete verification
"""
import json
import time

import psycopg2
import psycopg2.extras
import pytest

from conftest import (
    API_BASE_URL,
    APIClient,
    extract_data,
    get_or_create_test_token,
    TEST_USER_EMAIL,
    TEST_USER_PASSWORD,
)

API_PREFIX = "/api/v1/ai-chat"

BCRYPT_HASH_TEST_PASSWORD = (
    "$2b$12$Ikc.R4FMMGahbGhfHlLl4.PciMiV37qXfHpPNCjGGQg/yOEgk7k/e"
)
SECOND_USER_EMAIL = "aichat_iso_test@glancemind.test"


# ---------------------------------------------------------------------------
# DB helpers
# ---------------------------------------------------------------------------


def _ensure_clean(conn):
    """Reset connection if stuck in a failed transaction state."""
    if conn.info.transaction_status == psycopg2.extensions.TRANSACTION_STATUS_INERROR:
        conn.rollback()


def _q1(conn, sql, params=None):
    _ensure_clean(conn)
    cur = conn.cursor(cursor_factory=psycopg2.extras.RealDictCursor)
    cur.execute(sql, params)
    row = cur.fetchone()
    cur.close()
    return dict(row) if row else None


def _qall(conn, sql, params=None):
    _ensure_clean(conn)
    cur = conn.cursor(cursor_factory=psycopg2.extras.RealDictCursor)
    cur.execute(sql, params)
    rows = cur.fetchall()
    cur.close()
    return [dict(r) for r in rows]


def _exec(conn, sql, params=None):
    _ensure_clean(conn)
    cur = conn.cursor()
    cur.execute(sql, params)
    conn.commit()
    cur.close()


def _insert_ret(conn, sql, params=None):
    _ensure_clean(conn)
    cur = conn.cursor(cursor_factory=psycopg2.extras.RealDictCursor)
    cur.execute(sql, params)
    row = cur.fetchone()
    conn.commit()
    cur.close()
    return dict(row) if row else None


def parse_sse_events(resp):
    """Parse SSE events from a requests streaming response.

    Compatible with both ``event: name`` (space) and ``event:name`` (no space)
    formats produced by different SSE implementations (e.g. Axum).
    """
    resp.encoding = "utf-8"
    events = []
    current_event = ""
    for line in resp.iter_lines(decode_unicode=True):
        if not line:
            continue
        if line.startswith("event:"):
            current_event = line[6:].strip()
        elif line.startswith("data:"):
            data_str = line[5:].strip()
            try:
                data = json.loads(data_str)
            except (json.JSONDecodeError, ValueError):
                data = data_str
            events.append({"event": current_event, "data": data})
            current_event = ""
    return events


def assert_recent_timestamp(ts_str, label="timestamp", max_age_seconds=300):
    """Assert ISO timestamp string is recent and parseable."""
    assert ts_str is not None, f"{label} should not be None"
    assert isinstance(ts_str, str), f"{label} should be a string, got {type(ts_str)}"
    from datetime import datetime, timezone
    try:
        ts = datetime.fromisoformat(ts_str.replace("Z", "+00:00"))
        if ts.tzinfo is None:
            ts = ts.replace(tzinfo=timezone.utc)
        age = abs((datetime.now(timezone.utc) - ts).total_seconds())
        assert age <= max_age_seconds, f"{label} age {age:.0f}s exceeds {max_age_seconds}s"
    except ValueError:
        pytest.fail(f"{label} is not a valid ISO timestamp: {ts_str}")


def create_plan_via_db(conn, user_id, conv_id, title="Test Plan", steps=None):
    """Insert a draft plan with steps directly into the database.

    Returns (plan_row, [step_rows]).
    """
    plan = _insert_ret(
        conn,
        """INSERT INTO gm_ai_plans
           (conversation_id, user_id, title, description, status, created_at)
           VALUES (%s, %s, %s, %s, 'draft', NOW()) RETURNING *""",
        (conv_id, user_id, title, f"Test plan: {title}"),
    )
    if steps is None:
        steps = [
            {"tool_name": "list_platforms", "tool_params": "{}", "description": "列出支持的平台"},
            {"tool_name": "get_wallet_balance", "tool_params": "{}", "description": "查询钱包余额"},
        ]
    step_rows = []
    for i, s in enumerate(steps, 1):
        row = _insert_ret(
            conn,
            """INSERT INTO gm_ai_plan_steps
               (plan_id, step_order, tool_name, tool_params, description, status, created_at)
               VALUES (%s, %s, %s, %s::jsonb, %s, 'pending', NOW()) RETURNING *""",
            (plan["id"], i, s["tool_name"], s["tool_params"], s["description"]),
        )
        step_rows.append(row)
    return plan, step_rows


# ---------------------------------------------------------------------------
# Fixtures
# ---------------------------------------------------------------------------


@pytest.fixture(scope="module")
def auth_client(db_connection):
    token = get_or_create_test_token()
    cur = db_connection.cursor()
    cur.execute("UPDATE gm_users SET permissions = 15 WHERE email = %s", (TEST_USER_EMAIL,))
    db_connection.commit()
    cur.close()
    return APIClient(API_BASE_URL, token=token)


@pytest.fixture(scope="module")
def test_user_id(db_connection):
    row = _q1(db_connection, "SELECT id FROM gm_users WHERE email = %s", (TEST_USER_EMAIL,))
    assert row is not None, f"Test user {TEST_USER_EMAIL} not found in DB"
    return row["id"]


@pytest.fixture(scope="module")
def second_auth_client(db_connection):
    """Create and authenticate a second user for isolation tests."""
    _ensure_clean(db_connection)
    cur = db_connection.cursor(cursor_factory=psycopg2.extras.RealDictCursor)

    cur.execute("SELECT id FROM gm_users WHERE email = %s", (SECOND_USER_EMAIL,))
    existing = cur.fetchone()
    if existing is None:
        cur.execute(
            """INSERT INTO gm_users
               (email, username, password_hash, full_name, role, status, is_active, created_at)
               VALUES (%s, 'aichat_iso', %s, 'Isolation Test User', 'user', 'ACTIVE', true, NOW())""",
            (SECOND_USER_EMAIL, BCRYPT_HASH_TEST_PASSWORD),
        )

    cur.execute("SELECT id FROM gm_email_verifications WHERE email = %s", (SECOND_USER_EMAIL,))
    ev_row = cur.fetchone()
    if ev_row is None:
        cur.execute(
            """INSERT INTO gm_email_verifications (email, code, verified, expires_at, created_at)
               VALUES (%s, 'ISO001', true, NOW() + INTERVAL '1 year', NOW())""",
            (SECOND_USER_EMAIL,),
        )
    else:
        cur.execute(
            "UPDATE gm_email_verifications SET verified = true, expires_at = NOW() + INTERVAL '1 year' WHERE email = %s",
            (SECOND_USER_EMAIL,),
        )

    cur.execute(
        "UPDATE gm_users SET permissions = 15 WHERE email = %s",
        (SECOND_USER_EMAIL,),
    )
    db_connection.commit()
    cur.close()

    client = APIClient(API_BASE_URL)
    resp = client.post(
        "/api/v1/auth/login",
        json={"identifier": SECOND_USER_EMAIL, "password": TEST_USER_PASSWORD},
    )
    assert resp.status_code == 200, f"Second user login failed: {resp.status_code} {resp.text}"

    data = extract_data(resp.json())
    assert data and isinstance(data, dict), "Invalid login response for second user"

    token = data.get("token")
    if isinstance(token, dict):
        token = token.get("token")
    assert token, "No token in second user login response"

    return APIClient(API_BASE_URL, token=token)


@pytest.fixture(scope="module")
def second_user_id(db_connection):
    row = _q1(db_connection, "SELECT id FROM gm_users WHERE email = %s", (SECOND_USER_EMAIL,))
    assert row is not None, "Second user must exist — second_auth_client fixture should create it"
    return row["id"]


# ===========================================================================
# 1. Conversation CRUD — full field validation
# ===========================================================================


class TestConversationCRUD:
    """Test conversation create, list, get, update, delete with full field validation."""

    conv_id = None

    def test_create_conversation(self, auth_client, test_user_id, db_connection):
        resp = auth_client.post(f"{API_PREFIX}/conversations", json={"title": "测试对话"})
        assert resp.status_code == 200, f"Create failed: {resp.text}"
        data = extract_data(resp.json())

        assert isinstance(data["id"], int), "id should be an integer"
        assert data["title"] == "测试对话"
        assert data["status"] == "active"
        assert_recent_timestamp(data["created_at"], "created_at")

        TestConversationCRUD.conv_id = data["id"]

        row = _q1(db_connection, "SELECT * FROM gm_ai_conversations WHERE id = %s", (data["id"],))
        assert row is not None, "Conversation should exist in DB"
        assert row["user_id"] == test_user_id
        assert row["title"] == "测试对话"
        assert row["status"] == "active"
        assert row["created_at"] is not None

    def test_create_conversation_default_title(self, auth_client):
        resp = auth_client.post(f"{API_PREFIX}/conversations", json={})
        assert resp.status_code == 200
        data = extract_data(resp.json())
        assert data["title"] == "New Chat", "Default title should be 'New Chat'"
        assert data["status"] == "active"

    def test_list_conversations(self, auth_client):
        assert TestConversationCRUD.conv_id is not None, "test_create_conversation must pass first"
        resp = auth_client.get(f"{API_PREFIX}/conversations", params={"page": 1, "page_size": 10})
        assert resp.status_code == 200
        data = extract_data(resp.json())

        assert "list" in data, "Response must contain 'list'"
        assert "total" in data, "Response must contain 'total'"
        assert "total_pages" in data, "Response must contain 'total_pages'"
        assert "page" in data, "Response must contain 'page'"
        assert "page_size" in data, "Response must contain 'page_size'"
        assert data["total"] >= 1, "Should have at least 1 conversation"
        assert isinstance(data["list"], list)
        assert len(data["list"]) >= 1

        resp2 = auth_client.get(f"{API_PREFIX}/conversations/{TestConversationCRUD.conv_id}")
        assert resp2.status_code == 200, "Created conversation should be accessible via GET"
        conv = extract_data(resp2.json())
        assert conv["status"] == "active"
        assert conv["created_at"] is not None

    def test_list_conversations_pagination(self, auth_client):
        resp = auth_client.get(f"{API_PREFIX}/conversations", params={"page": 1, "page_size": 1})
        assert resp.status_code == 200
        data = extract_data(resp.json())
        assert len(data["list"]) <= 1, "page_size=1 should return at most 1 item"
        if data["total"] >= 2:
            assert data["total_pages"] >= 2, "With page_size=1 and 2+ convs, total_pages >= 2"

    def test_get_conversation(self, auth_client):
        cid = TestConversationCRUD.conv_id
        resp = auth_client.get(f"{API_PREFIX}/conversations/{cid}")
        assert resp.status_code == 200
        data = extract_data(resp.json())

        assert data["id"] == cid
        assert data["title"] == "测试对话"
        assert data["status"] == "active"
        assert data["created_at"] is not None

    def test_get_nonexistent_conversation(self, auth_client):
        resp = auth_client.get(f"{API_PREFIX}/conversations/999999")
        assert resp.status_code in [404, 400], f"Expected 404, got {resp.status_code}"

    def test_update_conversation(self, auth_client, db_connection):
        cid = TestConversationCRUD.conv_id
        resp = auth_client.patch(
            f"{API_PREFIX}/conversations/{cid}",
            json={"title": "更新后的标题"},
        )
        assert resp.status_code == 200
        data = extract_data(resp.json())
        assert data["title"] == "更新后的标题"

        row = _q1(db_connection, "SELECT * FROM gm_ai_conversations WHERE id = %s", (cid,))
        assert row["title"] == "更新后的标题"
        assert row["updated_at"] is not None, "updated_at should be set after update"

    def test_get_messages_empty(self, auth_client):
        cid = TestConversationCRUD.conv_id
        resp = auth_client.get(f"{API_PREFIX}/conversations/{cid}/messages")
        assert resp.status_code == 200
        data = extract_data(resp.json())
        assert isinstance(data, list), "Messages should be a list"

    def test_delete_conversation(self, auth_client, db_connection):
        resp = auth_client.post(f"{API_PREFIX}/conversations", json={"title": "要删除的对话"})
        data = extract_data(resp.json())
        del_id = data["id"]

        resp = auth_client.delete(f"{API_PREFIX}/conversations/{del_id}")
        assert resp.status_code == 200

        resp = auth_client.get(f"{API_PREFIX}/conversations/{del_id}")
        assert resp.status_code in [404, 400], "Deleted conversation should return 404"

        row = _q1(db_connection, "SELECT * FROM gm_ai_conversations WHERE id = %s", (del_id,))
        assert row is None, "Conversation should be removed from DB"

    def test_delete_nonexistent_conversation(self, auth_client):
        resp = auth_client.delete(f"{API_PREFIX}/conversations/999999")
        assert resp.status_code in [404, 400], "Deleting non-existent conv should fail"


# ===========================================================================
# 2. Delete cascade — messages, plans, steps all removed
# ===========================================================================


class TestDeleteConversationCascade:
    """Verify deleting a conversation cascades to messages, plans, and steps."""

    def test_cascade_delete(self, auth_client, test_user_id, db_connection):
        resp = auth_client.post(f"{API_PREFIX}/conversations", json={"title": "Cascade Test"})
        data = extract_data(resp.json())
        conv_id = data["id"]

        _exec(
            db_connection,
            """INSERT INTO gm_ai_messages (conversation_id, role, content, created_at)
               VALUES (%s, 'user', 'test message', NOW())""",
            (conv_id,),
        )
        _exec(
            db_connection,
            """INSERT INTO gm_ai_messages (conversation_id, role, content, created_at)
               VALUES (%s, 'assistant', 'response', NOW())""",
            (conv_id,),
        )

        plan, steps = create_plan_via_db(db_connection, test_user_id, conv_id, "Cascade Plan")

        msgs_before = _qall(db_connection, "SELECT * FROM gm_ai_messages WHERE conversation_id = %s", (conv_id,))
        assert len(msgs_before) == 2, "Should have 2 messages before delete"
        steps_before = _qall(db_connection, "SELECT * FROM gm_ai_plan_steps WHERE plan_id = %s", (plan["id"],))
        assert len(steps_before) == 2, "Should have 2 plan steps before delete"

        resp = auth_client.delete(f"{API_PREFIX}/conversations/{conv_id}")
        assert resp.status_code == 200

        msgs_after = _qall(db_connection, "SELECT * FROM gm_ai_messages WHERE conversation_id = %s", (conv_id,))
        assert len(msgs_after) == 0, "Messages should be cascade deleted"

        plan_after = _q1(db_connection, "SELECT * FROM gm_ai_plans WHERE id = %s", (plan["id"],))
        assert plan_after is None, "Plan should be cascade deleted"

        steps_after = _qall(db_connection, "SELECT * FROM gm_ai_plan_steps WHERE plan_id = %s", (plan["id"],))
        assert len(steps_after) == 0, "Plan steps should be cascade deleted"


# ===========================================================================
# 3. Send message + SSE stream format
# ===========================================================================


class TestSendMessageSSE:
    """Test SSE endpoint connectivity and event format."""

    def test_send_message_returns_sse_content_type(self, auth_client):
        resp = auth_client.post(f"{API_PREFIX}/conversations", json={"title": "SSE格式测试"})
        data = extract_data(resp.json())
        conv_id = data["id"]

        resp = auth_client.post(
            f"{API_PREFIX}/conversations/{conv_id}/messages",
            json={"content": "你好"},
            stream=True,
        )
        content_type = resp.headers.get("content-type", "")
        assert "text/event-stream" in content_type, f"Expected SSE, got {content_type}"
        resp.close()

    def test_send_message_sse_events_valid(self, auth_client):
        """Parse SSE events and validate structure."""
        resp = auth_client.post(f"{API_PREFIX}/conversations", json={"title": "SSE事件测试"})
        data = extract_data(resp.json())
        conv_id = data["id"]

        resp = auth_client.post(
            f"{API_PREFIX}/conversations/{conv_id}/messages",
            json={"content": "你好"},
            stream=True,
            timeout=60,
        )
        events = parse_sse_events(resp)

        assert len(events) > 0, "Should receive at least one SSE event"

        valid_event_types = {
            "message_start", "text_delta", "tool_call_start", "tool_call_result",
            "plan_created", "message_end", "error",
        }
        for evt in events:
            assert evt["event"] in valid_event_types, f"Unknown event type: {evt['event']}"
            assert evt["data"] is not None, "Event data should not be None"

        end_events = [e for e in events if e["event"] == "message_end"]
        error_events = [e for e in events if e["event"] == "error"]
        assert len(end_events) + len(error_events) >= 1, (
            "Stream should end with message_end or error"
        )

        if end_events:
            end = end_events[0]["data"]
            assert "message_id" in end, "message_end must contain message_id"
            assert "finish_reason" in end, "message_end must contain finish_reason"
            assert isinstance(end["message_id"], int)
            assert end["finish_reason"] == "stop"

    def test_send_message_tool_call_events(self, auth_client):
        """Send a query that should trigger tool calls; validate event structure."""
        resp = auth_client.post(f"{API_PREFIX}/conversations", json={"title": "Tool调用测试"})
        data = extract_data(resp.json())
        conv_id = data["id"]

        resp = auth_client.post(
            f"{API_PREFIX}/conversations/{conv_id}/messages",
            json={"content": "查看我的钱包余额是多少"},
            stream=True,
            timeout=60,
        )
        events = parse_sse_events(resp)

        tc_starts = [e for e in events if e["event"] == "tool_call_start"]
        tc_results = [e for e in events if e["event"] == "tool_call_result"]

        if tc_starts:
            start = tc_starts[0]["data"]
            assert "tool_call_id" in start, "tool_call_start must have tool_call_id"
            assert "tool_name" in start, "tool_call_start must have tool_name"
            assert isinstance(start["tool_call_id"], str)
            assert isinstance(start["tool_name"], str)

        if tc_results:
            result = tc_results[0]["data"]
            assert "tool_call_id" in result, "tool_call_result must have tool_call_id"
            assert "result" in result, "tool_call_result must have result"
            assert "success" in result, "tool_call_result must have success"
            assert isinstance(result["success"], bool)


# ===========================================================================
# 4. Conversation history
# ===========================================================================


class TestConversationHistory:
    """Test message history retrieval and ordering."""

    def test_messages_after_send(self, auth_client, test_user_id, db_connection):
        """Create messages via DB, verify GET /messages returns them in order."""
        resp = auth_client.post(f"{API_PREFIX}/conversations", json={"title": "History Test"})
        data = extract_data(resp.json())
        conv_id = data["id"]

        _exec(
            db_connection,
            """INSERT INTO gm_ai_messages (conversation_id, role, content, created_at)
               VALUES (%s, 'user', '第一条消息', NOW())""",
            (conv_id,),
        )
        _exec(
            db_connection,
            """INSERT INTO gm_ai_messages (conversation_id, role, content, created_at)
               VALUES (%s, 'assistant', '第一条回复', NOW() + INTERVAL '1 second')""",
            (conv_id,),
        )
        _exec(
            db_connection,
            """INSERT INTO gm_ai_messages (conversation_id, role, content, created_at)
               VALUES (%s, 'user', '第二条消息', NOW() + INTERVAL '2 seconds')""",
            (conv_id,),
        )

        resp = auth_client.get(f"{API_PREFIX}/conversations/{conv_id}/messages")
        assert resp.status_code == 200
        messages = extract_data(resp.json())

        assert isinstance(messages, list)
        assert len(messages) == 3, f"Expected 3 messages, got {len(messages)}"

        assert messages[0]["role"] == "user"
        assert messages[0]["content"] == "第一条消息"
        assert messages[1]["role"] == "assistant"
        assert messages[1]["content"] == "第一条回复"
        assert messages[2]["role"] == "user"
        assert messages[2]["content"] == "第二条消息"

        for msg in messages:
            assert isinstance(msg["id"], int)
            assert msg["conversation_id"] == conv_id
            assert msg["created_at"] is not None

    def test_messages_from_other_conversation_not_leaked(self, auth_client, db_connection):
        """Messages from one conversation must not appear in another."""
        resp1 = auth_client.post(f"{API_PREFIX}/conversations", json={"title": "Conv A"})
        cid_a = extract_data(resp1.json())["id"]

        resp2 = auth_client.post(f"{API_PREFIX}/conversations", json={"title": "Conv B"})
        cid_b = extract_data(resp2.json())["id"]

        _exec(
            db_connection,
            """INSERT INTO gm_ai_messages (conversation_id, role, content, created_at)
               VALUES (%s, 'user', 'Only in A', NOW())""",
            (cid_a,),
        )
        _exec(
            db_connection,
            """INSERT INTO gm_ai_messages (conversation_id, role, content, created_at)
               VALUES (%s, 'user', 'Only in B', NOW())""",
            (cid_b,),
        )

        resp = auth_client.get(f"{API_PREFIX}/conversations/{cid_a}/messages")
        msgs_a = extract_data(resp.json())
        assert all(m["conversation_id"] == cid_a for m in msgs_a)
        assert all("Only in B" != m["content"] for m in msgs_a)


# ===========================================================================
# 5. Plan lifecycle: cancel, confirm, step update
# ===========================================================================


class TestPlanLifecycle:
    """Test plan creation (via DB), confirmation (SSE), cancellation, and step update."""

    def test_cancel_plan_normal_path(self, auth_client, test_user_id, db_connection):
        """Create a draft plan, cancel it, verify status changes."""
        resp = auth_client.post(f"{API_PREFIX}/conversations", json={"title": "Plan Cancel Test"})
        conv_id = extract_data(resp.json())["id"]

        plan, steps = create_plan_via_db(db_connection, test_user_id, conv_id, "要取消的计划")

        resp = auth_client.post(f"{API_PREFIX}/plans/{plan['id']}/cancel")
        assert resp.status_code == 200
        data = extract_data(resp.json())

        assert data["id"] == plan["id"]
        assert data["status"] == "cancelled"
        assert data["title"] == "要取消的计划"
        assert "steps" in data
        assert isinstance(data["steps"], list)
        assert len(data["steps"]) == 2

        row = _q1(db_connection, "SELECT * FROM gm_ai_plans WHERE id = %s", (plan["id"],))
        assert row["status"] == "cancelled"

    def test_cancel_nonexistent_plan(self, auth_client):
        resp = auth_client.post(f"{API_PREFIX}/plans/999999/cancel")
        assert resp.status_code in [404, 400], f"Expected error, got {resp.status_code}"

    def test_cancel_non_draft_plan_rejected(self, auth_client, test_user_id, db_connection):
        """Cannot cancel a plan that is not in draft status."""
        resp = auth_client.post(f"{API_PREFIX}/conversations", json={"title": "Non-draft Cancel"})
        conv_id = extract_data(resp.json())["id"]

        plan, _ = create_plan_via_db(db_connection, test_user_id, conv_id)
        _exec(
            db_connection,
            "UPDATE gm_ai_plans SET status = 'completed' WHERE id = %s",
            (plan["id"],),
        )

        resp = auth_client.post(f"{API_PREFIX}/plans/{plan['id']}/cancel")
        assert resp.status_code in [400, 422], "Cancelling non-draft plan should fail"

    def test_confirm_plan_execution(self, auth_client, test_user_id, db_connection):
        """Confirm a draft plan and verify each step executes via SSE."""
        resp = auth_client.post(f"{API_PREFIX}/conversations", json={"title": "Plan Confirm Test"})
        conv_id = extract_data(resp.json())["id"]

        plan, steps = create_plan_via_db(
            db_connection, test_user_id, conv_id, "确认执行计划",
            steps=[
                {"tool_name": "list_platforms", "tool_params": "{}", "description": "列出平台"},
                {"tool_name": "get_wallet_balance", "tool_params": "{}", "description": "查询余额"},
            ],
        )

        resp = auth_client.post(
            f"{API_PREFIX}/plans/{plan['id']}/confirm",
            stream=True,
            timeout=60,
        )
        content_type = resp.headers.get("content-type", "")
        assert "text/event-stream" in content_type

        events = parse_sse_events(resp)
        assert len(events) > 0, "Should receive SSE events during plan execution"

        valid_plan_events = {"step_start", "step_completed", "step_failed", "plan_completed", "error"}
        for evt in events:
            assert evt["event"] in valid_plan_events, f"Unexpected event: {evt['event']}"

        step_starts = [e for e in events if e["event"] == "step_start"]
        step_completes = [e for e in events if e["event"] == "step_completed"]
        plan_completes = [e for e in events if e["event"] == "plan_completed"]

        assert len(step_starts) == 2, f"Expected 2 step_start events, got {len(step_starts)}"

        for ss in step_starts:
            d = ss["data"]
            assert "step_id" in d
            assert "step_order" in d
            assert "description" in d

        if step_completes:
            for sc in step_completes:
                d = sc["data"]
                assert "step_id" in d
                assert "result" in d

        assert len(plan_completes) == 1, "Should have exactly 1 plan_completed event"
        pc = plan_completes[0]["data"]
        assert "plan_id" in pc
        assert pc["plan_id"] == plan["id"]
        assert "summary" in pc

        plan_row = _q1(db_connection, "SELECT * FROM gm_ai_plans WHERE id = %s", (plan["id"],))
        assert plan_row["status"] in ["completed", "failed"], f"Plan should be done, got {plan_row['status']}"

        step_rows = _qall(
            db_connection,
            "SELECT * FROM gm_ai_plan_steps WHERE plan_id = %s ORDER BY step_order",
            (plan["id"],),
        )
        for sr in step_rows:
            assert sr["status"] in ["completed", "failed"], f"Step {sr['id']} status: {sr['status']}"
            if sr["status"] == "completed":
                assert sr["result"] is not None, f"Completed step {sr['id']} should have result"
            if sr["status"] == "failed":
                assert sr["error_message"] is not None, f"Failed step {sr['id']} should have error"

    def test_confirm_non_draft_plan_rejected(self, auth_client, test_user_id, db_connection):
        resp = auth_client.post(f"{API_PREFIX}/conversations", json={"title": "Non-draft Confirm"})
        conv_id = extract_data(resp.json())["id"]

        plan, _ = create_plan_via_db(db_connection, test_user_id, conv_id)
        _exec(
            db_connection,
            "UPDATE gm_ai_plans SET status = 'cancelled' WHERE id = %s",
            (plan["id"],),
        )

        resp = auth_client.post(
            f"{API_PREFIX}/plans/{plan['id']}/confirm",
            stream=True,
            timeout=30,
        )
        events = parse_sse_events(resp)
        error_events = [e for e in events if e["event"] == "error"]
        assert len(error_events) >= 1, "Confirming non-draft plan should produce error"

    def test_update_plan_step_params(self, auth_client, test_user_id, db_connection):
        """Update a step's parameters and description on a draft plan."""
        resp = auth_client.post(f"{API_PREFIX}/conversations", json={"title": "Step Update Test"})
        conv_id = extract_data(resp.json())["id"]

        plan, steps = create_plan_via_db(db_connection, test_user_id, conv_id, "Step Update Plan")
        step = steps[0]

        new_desc = "Updated step description"
        new_params = {"model_type": "video"}
        resp = auth_client.patch(
            f"{API_PREFIX}/plans/{plan['id']}/steps/{step['id']}",
            json={"description": new_desc, "tool_params": new_params},
        )
        assert resp.status_code == 200
        data = extract_data(resp.json())

        assert data["description"] == new_desc
        actual_params = data["tool_params"]
        if isinstance(actual_params, str):
            actual_params = json.loads(actual_params)
        assert actual_params == new_params, f"Expected {new_params}, got {actual_params}"

        row = _q1(db_connection, "SELECT * FROM gm_ai_plan_steps WHERE id = %s", (step["id"],))
        assert row["description"] == new_desc

    def test_update_step_nonexistent(self, auth_client):
        resp = auth_client.patch(
            f"{API_PREFIX}/plans/999999/steps/999999",
            json={"description": "nope"},
        )
        assert resp.status_code in [404, 400]

    def test_update_step_on_non_draft_plan(self, auth_client, test_user_id, db_connection):
        resp = auth_client.post(f"{API_PREFIX}/conversations", json={"title": "Non-draft Step"})
        conv_id = extract_data(resp.json())["id"]

        plan, steps = create_plan_via_db(db_connection, test_user_id, conv_id)
        _exec(
            db_connection,
            "UPDATE gm_ai_plans SET status = 'completed' WHERE id = %s",
            (plan["id"],),
        )

        resp = auth_client.patch(
            f"{API_PREFIX}/plans/{plan['id']}/steps/{steps[0]['id']}",
            json={"description": "should fail"},
        )
        assert resp.status_code in [400, 422], "Updating step on non-draft plan should fail"

    def test_audit_logs_created_on_confirm(self, auth_client, test_user_id, db_connection):
        """After plan confirmation, audit logs should be created for each tool call."""
        resp = auth_client.post(f"{API_PREFIX}/conversations", json={"title": "Audit Log Test"})
        conv_id = extract_data(resp.json())["id"]

        plan, _ = create_plan_via_db(
            db_connection, test_user_id, conv_id, "Audit Plan",
            steps=[
                {"tool_name": "list_platforms", "tool_params": "{}", "description": "Step 1"},
            ],
        )

        resp = auth_client.post(
            f"{API_PREFIX}/plans/{plan['id']}/confirm",
            stream=True,
            timeout=60,
        )
        _ = parse_sse_events(resp)

        time.sleep(1)
        logs = _qall(
            db_connection,
            """SELECT * FROM gm_ai_tool_audit_logs
               WHERE conversation_id = %s AND tool_name = 'list_platforms'
               ORDER BY created_at DESC LIMIT 5""",
            (conv_id,),
        )
        assert len(logs) >= 1, "Audit log should be created for tool execution"
        log = logs[0]
        assert log["user_id"] == test_user_id
        assert log["tool_name"] == "list_platforms"
        assert log["safety_level"] == "read_only"
        assert log["success"] is True
        assert log["error_message"] is None


# ===========================================================================
# 6. Security: cross-user isolation
# ===========================================================================


class TestSecurityIsolation:
    """Verify users cannot access each other's conversations, plans, or messages."""

    conv_id_user1 = None
    plan_id_user1 = None
    step_id_user1 = None

    def test_setup_user1_data(self, auth_client, test_user_id, db_connection):
        resp = auth_client.post(f"{API_PREFIX}/conversations", json={"title": "User1 Private"})
        assert resp.status_code == 200
        data = extract_data(resp.json())
        TestSecurityIsolation.conv_id_user1 = data["id"]

        plan, steps = create_plan_via_db(
            db_connection, test_user_id, data["id"], "User1 Plan",
        )
        TestSecurityIsolation.plan_id_user1 = plan["id"]
        TestSecurityIsolation.step_id_user1 = steps[0]["id"]

    def test_cross_user_get_conversation(self, second_auth_client):
        cid = TestSecurityIsolation.conv_id_user1
        resp = second_auth_client.get(f"{API_PREFIX}/conversations/{cid}")
        assert resp.status_code in [404, 403, 400], (
            f"User2 should NOT access User1's conversation, got {resp.status_code}"
        )

    def test_cross_user_update_conversation(self, second_auth_client):
        cid = TestSecurityIsolation.conv_id_user1
        resp = second_auth_client.patch(
            f"{API_PREFIX}/conversations/{cid}",
            json={"title": "Hacked title"},
        )
        assert resp.status_code != 200, "User2 should NOT update User1's conversation"

    def test_cross_user_delete_conversation(self, auth_client, second_auth_client):
        cid = TestSecurityIsolation.conv_id_user1
        resp = second_auth_client.delete(f"{API_PREFIX}/conversations/{cid}")
        assert resp.status_code != 200, "User2 should NOT delete User1's conversation"

        resp_verify = auth_client.get(f"{API_PREFIX}/conversations/{cid}")
        assert resp_verify.status_code == 200, "Conversation should still exist for User1"

    def test_cross_user_get_messages(self, second_auth_client):
        cid = TestSecurityIsolation.conv_id_user1
        resp = second_auth_client.get(f"{API_PREFIX}/conversations/{cid}/messages")
        assert resp.status_code in [404, 403, 400], (
            "User2 should NOT access User1's messages"
        )

    def test_cross_user_cancel_plan(self, second_auth_client):
        pid = TestSecurityIsolation.plan_id_user1
        resp = second_auth_client.post(f"{API_PREFIX}/plans/{pid}/cancel")
        assert resp.status_code in [404, 403, 400], (
            "User2 should NOT cancel User1's plan"
        )

    def test_cross_user_confirm_plan(self, second_auth_client):
        pid = TestSecurityIsolation.plan_id_user1
        resp = second_auth_client.post(
            f"{API_PREFIX}/plans/{pid}/confirm",
            stream=True,
            timeout=10,
        )
        if resp.status_code in [404, 403, 400]:
            return
        events = parse_sse_events(resp)
        error_events = [e for e in events if e["event"] == "error"]
        assert len(error_events) >= 1 or resp.status_code != 200, (
            "User2 confirming User1's plan should produce HTTP error or SSE error"
        )

    def test_cross_user_update_plan_step(self, second_auth_client):
        pid = TestSecurityIsolation.plan_id_user1
        sid = TestSecurityIsolation.step_id_user1
        resp = second_auth_client.patch(
            f"{API_PREFIX}/plans/{pid}/steps/{sid}",
            json={"description": "hacked"},
        )
        assert resp.status_code != 200, "User2 should NOT update User1's plan step"

    def test_user2_list_does_not_contain_user1(self, second_auth_client):
        resp = second_auth_client.get(f"{API_PREFIX}/conversations", params={"page": 1, "page_size": 100})
        if resp.status_code == 200:
            data = extract_data(resp.json())
            ids = [c["id"] for c in data.get("list", [])]
            assert TestSecurityIsolation.conv_id_user1 not in ids, (
                "User1's conversation should NOT appear in User2's list"
            )


# ===========================================================================
# 7. Security: message length limit
# ===========================================================================


class TestMessageLengthLimit:
    """Verify overly long messages are rejected (MAX_MESSAGE_LENGTH = 4000)."""

    def test_long_message_rejected(self, auth_client):
        resp = auth_client.post(f"{API_PREFIX}/conversations", json={"title": "长消息测试"})
        data = extract_data(resp.json())
        conv_id = data["id"]

        long_msg = "x" * 5000
        resp = auth_client.post(
            f"{API_PREFIX}/conversations/{conv_id}/messages",
            json={"content": long_msg},
            stream=True,
            timeout=30,
        )
        if resp.status_code in [400, 422]:
            return
        events = parse_sse_events(resp)
        resp.close()
        error_events = [e for e in events if e["event"] == "error"]
        assert len(error_events) >= 1, (
            f"Message exceeding 4000 chars should be rejected via HTTP or SSE error, got status {resp.status_code} with {len(events)} events"
        )

    def test_message_at_limit_accepted(self, auth_client):
        resp = auth_client.post(f"{API_PREFIX}/conversations", json={"title": "边界消息测试"})
        data = extract_data(resp.json())
        conv_id = data["id"]

        msg = "x" * 4000
        resp = auth_client.post(
            f"{API_PREFIX}/conversations/{conv_id}/messages",
            json={"content": msg},
            stream=True,
            timeout=60,
        )
        content_type = resp.headers.get("content-type", "")
        assert "text/event-stream" in content_type, (
            "Message at exact limit (4000) should be accepted"
        )


# ===========================================================================
# 8. Security: sensitive field redaction
# ===========================================================================


class TestSensitiveFieldRedaction:
    """Verify tool results do not leak sensitive fields."""

    def test_no_password_hash_in_tool_result(self, auth_client):
        """Send a query that triggers list_social_accounts, check results don't contain sensitive data."""
        resp = auth_client.post(f"{API_PREFIX}/conversations", json={"title": "Redaction Test"})
        data = extract_data(resp.json())
        conv_id = data["id"]

        resp = auth_client.post(
            f"{API_PREFIX}/conversations/{conv_id}/messages",
            json={"content": "列出我的社交媒体账户"},
            stream=True,
            timeout=60,
        )
        events = parse_sse_events(resp)

        tc_results = [e for e in events if e["event"] == "tool_call_result"]
        for tcr in tc_results:
            result_str = json.dumps(tcr["data"])
            assert "password_hash" not in result_str, "password_hash must not appear in tool results"
            raw_cookie = '"cookie": "' 
            if raw_cookie in result_str:
                cookie_val = tcr["data"].get("result", {})
                assert_no_raw_sensitive(cookie_val)


def assert_no_raw_sensitive(data, path=""):
    """Recursively check no raw sensitive values leaked (un-redacted)."""
    sensitive_keys = {"password_hash", "jwt_secret"}
    if isinstance(data, dict):
        for k, v in data.items():
            if k in sensitive_keys and isinstance(v, str) and not v.startswith("****"):
                raise AssertionError(f"Sensitive field '{k}' at {path}.{k} is not redacted: {v[:20]}")
            assert_no_raw_sensitive(v, f"{path}.{k}")
    elif isinstance(data, list):
        for i, v in enumerate(data):
            assert_no_raw_sensitive(v, f"{path}[{i}]")


# ===========================================================================
# 9. Unauthenticated access
# ===========================================================================


class TestUnauthenticatedAccess:
    """Verify all endpoints require authentication."""

    def test_unauth_list_conversations(self):
        client = APIClient(API_BASE_URL)
        resp = client.get(f"{API_PREFIX}/conversations")
        assert resp.status_code in [401, 403], "Unauthenticated request should be rejected"

    def test_unauth_create_conversation(self):
        client = APIClient(API_BASE_URL)
        resp = client.post(f"{API_PREFIX}/conversations", json={"title": "no auth"})
        assert resp.status_code in [401, 403]

    def test_unauth_send_message(self):
        client = APIClient(API_BASE_URL)
        resp = client.post(f"{API_PREFIX}/conversations/1/messages", json={"content": "hi"})
        assert resp.status_code in [401, 403]

    def test_unauth_confirm_plan(self):
        client = APIClient(API_BASE_URL)
        resp = client.post(f"{API_PREFIX}/plans/1/confirm")
        assert resp.status_code in [401, 403]


# ===========================================================================
# 10. Core query tool tests via SSE (requires LLM)
# ===========================================================================


@pytest.mark.requires_llm
class TestCoreQueryTools:
    """Verify main platform features work via AI chat tool calls.

    These tests send natural language queries and verify the AI calls
    the correct tools. Requires a running LLM service.
    """

    def test_wallet_balance_query(self, auth_client):
        resp = auth_client.post(f"{API_PREFIX}/conversations", json={"title": "Wallet Q"})
        conv_id = extract_data(resp.json())["id"]

        resp = auth_client.post(
            f"{API_PREFIX}/conversations/{conv_id}/messages",
            json={"content": "我的钱包余额是多少"},
            stream=True,
            timeout=60,
        )
        events = parse_sse_events(resp)

        tc_starts = [e for e in events if e["event"] == "tool_call_start"]
        assert len(tc_starts) >= 1, "Should trigger tool call"

        wallet_calls = [e for e in tc_starts if e["data"].get("tool_name") == "get_wallet_balance"]
        assert len(wallet_calls) >= 1, "Should call get_wallet_balance"

        tc_results = [e for e in events if e["event"] == "tool_call_result"]
        for r in tc_results:
            if r["data"].get("tool_call_id") == wallet_calls[0]["data"]["tool_call_id"]:
                assert r["data"]["success"] is True, "Wallet balance query should succeed"

    def test_platforms_query(self, auth_client):
        resp = auth_client.post(f"{API_PREFIX}/conversations", json={"title": "Platform Q"})
        conv_id = extract_data(resp.json())["id"]

        resp = auth_client.post(
            f"{API_PREFIX}/conversations/{conv_id}/messages",
            json={"content": "列出所有支持的社交媒体平台"},
            stream=True,
            timeout=60,
        )
        events = parse_sse_events(resp)
        tc_starts = [e for e in events if e["event"] == "tool_call_start"]
        assert len(tc_starts) >= 1

    def test_dashboard_stats_query(self, auth_client):
        resp = auth_client.post(f"{API_PREFIX}/conversations", json={"title": "Dashboard Q"})
        conv_id = extract_data(resp.json())["id"]

        resp = auth_client.post(
            f"{API_PREFIX}/conversations/{conv_id}/messages",
            json={"content": "看看我的仪表盘数据统计"},
            stream=True,
            timeout=60,
        )
        events = parse_sse_events(resp)
        tc_starts = [e for e in events if e["event"] == "tool_call_start"]
        assert len(tc_starts) >= 1

    def test_campaigns_query(self, auth_client):
        resp = auth_client.post(f"{API_PREFIX}/conversations", json={"title": "Campaign Q"})
        conv_id = extract_data(resp.json())["id"]

        resp = auth_client.post(
            f"{API_PREFIX}/conversations/{conv_id}/messages",
            json={"content": "列出我的营销活动"},
            stream=True,
            timeout=60,
        )
        events = parse_sse_events(resp)
        tc_starts = [e for e in events if e["event"] == "tool_call_start"]
        assert len(tc_starts) >= 1

    def test_social_groups_query(self, auth_client):
        resp = auth_client.post(f"{API_PREFIX}/conversations", json={"title": "Group Q"})
        conv_id = extract_data(resp.json())["id"]

        resp = auth_client.post(
            f"{API_PREFIX}/conversations/{conv_id}/messages",
            json={"content": "我有哪些社交账号分组"},
            stream=True,
            timeout=60,
        )
        events = parse_sse_events(resp)
        tc_starts = [e for e in events if e["event"] == "tool_call_start"]
        assert len(tc_starts) >= 1


# ===========================================================================
# 11. Interactive follow-up scenarios (requires LLM)
# ===========================================================================


@pytest.mark.requires_llm
class TestInteractiveFollowUp:
    """Test that AI properly asks for missing info when creating resources."""

    def test_create_plan_triggers_query_or_followup(self, auth_client):
        """Vague 'create plan' should trigger platform query or text follow-up."""
        resp = auth_client.post(f"{API_PREFIX}/conversations", json={"title": "Follow-up Q"})
        conv_id = extract_data(resp.json())["id"]

        resp = auth_client.post(
            f"{API_PREFIX}/conversations/{conv_id}/messages",
            json={"content": "帮我创建一个发布计划"},
            stream=True,
            timeout=60,
        )
        events = parse_sse_events(resp)

        tc_starts = [e for e in events if e["event"] == "tool_call_start"]
        text_deltas = [e for e in events if e["event"] == "text_delta"]

        has_query = any(
            e["data"].get("tool_name") in ("list_platforms", "list_social_groups", "list_ai_models")
            for e in tc_starts
        )
        has_text = len(text_deltas) > 0

        assert has_query or has_text, (
            "AI should either query platforms/groups or ask user for details"
        )

    def test_ambiguous_delete_triggers_list_or_question(self, auth_client):
        """'Delete that campaign' should trigger list or ask which one."""
        resp = auth_client.post(f"{API_PREFIX}/conversations", json={"title": "Delete Q"})
        conv_id = extract_data(resp.json())["id"]

        resp = auth_client.post(
            f"{API_PREFIX}/conversations/{conv_id}/messages",
            json={"content": "帮我删掉那个活动"},
            stream=True,
            timeout=60,
        )
        events = parse_sse_events(resp)

        tc_starts = [e for e in events if e["event"] == "tool_call_start"]
        text_deltas = [e for e in events if e["event"] == "text_delta"]

        has_list = any(
            e["data"].get("tool_name") in ("list_campaigns",)
            for e in tc_starts
        )
        has_text = len(text_deltas) > 0

        assert has_list or has_text, (
            "AI should list campaigns or ask which one"
        )


# ===========================================================================
# 12. Tool direct execution via Plan confirm (no LLM needed)
#     Covers ALL 24 tools by creating plan steps and confirming.
# ===========================================================================


class TestToolDirectExecution:
    """Test every Tool's execution by inserting plan steps and confirming.

    This approach bypasses LLM entirely: we create a draft plan with steps
    referencing specific tools, confirm it, and verify each step completes.
    """

    def _confirm_and_collect(self, auth_client, plan_id):
        """Confirm a plan and return parsed SSE events."""
        resp = auth_client.post(
            f"{API_PREFIX}/plans/{plan_id}/confirm",
            stream=True,
            timeout=60,
        )
        return parse_sse_events(resp)

    def _assert_steps_completed(self, events, expected_count):
        starts = [e for e in events if e["event"] == "step_start"]
        completes = [e for e in events if e["event"] == "step_completed"]
        fails = [e for e in events if e["event"] == "step_failed"]
        plan_done = [e for e in events if e["event"] == "plan_completed"]

        assert len(starts) == expected_count, f"Expected {expected_count} step_start, got {len(starts)}"
        assert len(plan_done) == 1, "Should have plan_completed"

        for c in completes:
            assert "result" in c["data"], "step_completed must have result"

        return completes, fails

    # ── Query Tools Batch 1: No params needed ──────────────────

    def test_query_tools_batch1(self, auth_client, test_user_id, db_connection):
        """list_platforms, get_wallet_balance, get_dashboard_stats, list_social_groups, list_campaigns"""
        resp = auth_client.post(f"{API_PREFIX}/conversations", json={"title": "Tool Batch 1"})
        conv_id = extract_data(resp.json())["id"]

        tools = [
            ("list_platforms", "{}"),
            ("get_wallet_balance", "{}"),
            ("get_dashboard_stats", "{}"),
            ("list_social_groups", "{}"),
            ("list_campaigns", "{}"),
        ]
        plan, steps = create_plan_via_db(
            db_connection, test_user_id, conv_id, "Query Batch 1",
            [{"tool_name": t, "tool_params": p, "description": f"Execute {t}"} for t, p in tools],
        )

        events = self._confirm_and_collect(auth_client, plan["id"])
        completes, fails = self._assert_steps_completed(events, len(tools))

        assert len(fails) == 0, f"No tool should fail: {[f['data'] for f in fails]}"
        assert len(completes) == len(tools)

        for step_row in _qall(db_connection, "SELECT * FROM gm_ai_plan_steps WHERE plan_id = %s ORDER BY step_order", (plan["id"],)):
            assert step_row["status"] == "completed", f"Step {step_row['tool_name']} should be completed, got {step_row['status']}"
            assert step_row["result"] is not None, f"Step {step_row['tool_name']} should have result"

    # ── Query Tools Batch 2: No params needed ──────────────────

    def test_query_tools_batch2(self, auth_client, test_user_id, db_connection):
        """list_publish_plans, list_templates, get_wallet_transactions, list_materials, list_ai_models"""
        resp = auth_client.post(f"{API_PREFIX}/conversations", json={"title": "Tool Batch 2"})
        conv_id = extract_data(resp.json())["id"]

        tools = [
            ("list_publish_plans", "{}"),
            ("list_templates", "{}"),
            ("get_wallet_transactions", "{}"),
            ("list_materials", "{}"),
            ("list_ai_models", "{}"),
        ]
        plan, steps = create_plan_via_db(
            db_connection, test_user_id, conv_id, "Query Batch 2",
            [{"tool_name": t, "tool_params": p, "description": f"Execute {t}"} for t, p in tools],
        )

        events = self._confirm_and_collect(auth_client, plan["id"])
        completes, fails = self._assert_steps_completed(events, len(tools))
        assert len(fails) == 0, f"No tool should fail: {[f['data'] for f in fails]}"
        assert len(completes) == len(tools)

    # ── Query Tools Batch 3: No params needed ──────────────────

    def test_query_tools_batch3(self, auth_client, test_user_id, db_connection):
        """list_video_tasks, get_plan_stats, list_social_accounts, get_account_statistics"""
        resp = auth_client.post(f"{API_PREFIX}/conversations", json={"title": "Tool Batch 3"})
        conv_id = extract_data(resp.json())["id"]

        tools = [
            ("list_video_tasks", "{}"),
            ("get_plan_stats", "{}"),
            ("list_social_accounts", "{}"),
            ("get_account_statistics", "{}"),
        ]
        plan, steps = create_plan_via_db(
            db_connection, test_user_id, conv_id, "Query Batch 3",
            [{"tool_name": t, "tool_params": p, "description": f"Execute {t}"} for t, p in tools],
        )

        events = self._confirm_and_collect(auth_client, plan["id"])
        completes, fails = self._assert_steps_completed(events, len(tools))
        assert len(fails) == 0, f"No tool should fail: {[f['data'] for f in fails]}"

    # ── create_plan_proposal tool ──────────────────────────────

    def test_tool_create_plan_proposal(self, auth_client, test_user_id, db_connection):
        """Test create_plan_proposal tool execution directly."""
        resp = auth_client.post(f"{API_PREFIX}/conversations", json={"title": "Proposal Tool"})
        conv_id = extract_data(resp.json())["id"]

        proposal_params = json.dumps({
            "title": "Test Proposal",
            "description": "Testing proposal tool",
            "steps": [
                {"tool_name": "list_platforms", "tool_params": {}, "description": "step1"}
            ]
        })
        plan, _ = create_plan_via_db(
            db_connection, test_user_id, conv_id, "Proposal Exec",
            [{"tool_name": "create_plan_proposal", "tool_params": proposal_params, "description": "Create proposal"}],
        )

        events = self._confirm_and_collect(auth_client, plan["id"])
        completes = [e for e in events if e["event"] == "step_completed"]
        assert len(completes) == 1
        result = completes[0]["data"]["result"]
        assert "title" in result or "title" in str(result), "Proposal result should contain title"

    # ── Mutation Tool: create_social_group ──────────────────────

    def test_tool_create_social_group(self, auth_client, test_user_id, db_connection):
        """Test create_social_group via plan confirm."""
        resp = auth_client.post(f"{API_PREFIX}/conversations", json={"title": "Create Group Tool"})
        conv_id = extract_data(resp.json())["id"]

        params = json.dumps({"group_name": "AI Chat Test Group", "platform_id": 2})
        plan, _ = create_plan_via_db(
            db_connection, test_user_id, conv_id, "Create Group",
            [{"tool_name": "create_social_group", "tool_params": params, "description": "Create group"}],
        )

        events = self._confirm_and_collect(auth_client, plan["id"])
        completes = [e for e in events if e["event"] == "step_completed"]
        assert len(completes) == 1

        result = completes[0]["data"]["result"]
        assert result is not None

        row = _q1(db_connection, "SELECT * FROM gm_social_groups WHERE group_name = 'AI Chat Test Group' AND user_id = %s", (test_user_id,))
        assert row is not None, "Group should be created in DB"
        assert row["platform_id"] == 2

    # ── Mutation Tool: create_social_account ────────────────────

    def test_tool_create_social_account(self, auth_client, test_user_id, db_connection):
        """Test create_social_account via plan confirm."""
        resp = auth_client.post(f"{API_PREFIX}/conversations", json={"title": "Create Account Tool"})
        conv_id = extract_data(resp.json())["id"]

        params = json.dumps({"platform_id": 2, "username": "aichat_test_account"})
        plan, _ = create_plan_via_db(
            db_connection, test_user_id, conv_id, "Create Account",
            [{"tool_name": "create_social_account", "tool_params": params, "description": "Create account"}],
        )

        events = self._confirm_and_collect(auth_client, plan["id"])
        completes = [e for e in events if e["event"] == "step_completed"]
        assert len(completes) == 1

        row = _q1(db_connection, "SELECT * FROM gm_social_accounts WHERE username = 'aichat_test_account' AND user_id = %s", (test_user_id,))
        assert row is not None, "Account should be created in DB"
        assert row["platform_id"] == 2

    # ── Mutation Tool: create_campaign ──────────────────────────

    def test_tool_create_campaign(self, auth_client, test_user_id, db_connection):
        """Test create_campaign via plan confirm."""
        resp = auth_client.post(f"{API_PREFIX}/conversations", json={"title": "Create Campaign Tool"})
        conv_id = extract_data(resp.json())["id"]

        params = json.dumps({"name": "AI Chat Test Campaign", "platform_id": 2, "region_id": 1, "ai_model_id": 1})
        plan, _ = create_plan_via_db(
            db_connection, test_user_id, conv_id, "Create Campaign",
            [{"tool_name": "create_campaign", "tool_params": params, "description": "Create campaign"}],
        )

        events = self._confirm_and_collect(auth_client, plan["id"])
        completes = [e for e in events if e["event"] == "step_completed"]
        fails = [e for e in events if e["event"] == "step_failed"]
        assert len(completes) + len(fails) == 1, f"Step should complete or fail, got {len(completes)} completes, {len(fails)} fails"

        if completes:
            row = _q1(db_connection, "SELECT * FROM gm_campaigns WHERE name = 'AI Chat Test Campaign' AND user_id = %s", (test_user_id,))
            assert row is not None, "Campaign should be created in DB"

    # ── Mutation Tool: update_campaign_status ───────────────────

    def test_tool_update_campaign_status(self, auth_client, test_user_id, db_connection):
        """Test update_campaign_status via plan confirm."""
        campaign = _q1(db_connection, "SELECT id FROM gm_campaigns WHERE user_id = %s LIMIT 1", (test_user_id,))
        if campaign is None:
            campaign = _insert_ret(db_connection,
                "INSERT INTO gm_campaigns (user_id, name, platform_id, region_id, ai_model_id, status, created_at) VALUES (%s, 'prereq_campaign', 2, 1, 1, 'ACTIVE', NOW()) RETURNING *",
                (test_user_id,))
        assert campaign is not None, "Must have a campaign for update_status test"

        resp = auth_client.post(f"{API_PREFIX}/conversations", json={"title": "Update Status Tool"})
        conv_id = extract_data(resp.json())["id"]

        params = json.dumps({"campaign_id": campaign["id"], "status": "paused"})
        plan, _ = create_plan_via_db(
            db_connection, test_user_id, conv_id, "Update Status",
            [{"tool_name": "update_campaign_status", "tool_params": params, "description": "Pause campaign"}],
        )

        events = self._confirm_and_collect(auth_client, plan["id"])
        completes = [e for e in events if e["event"] == "step_completed"]
        fails = [e for e in events if e["event"] == "step_failed"]
        assert len(completes) + len(fails) == 1

    # ── Mutation Tool: delete_campaign ──────────────────────────

    def test_tool_delete_campaign(self, auth_client, test_user_id, db_connection):
        """Test delete_campaign via plan confirm (note: current impl is soft/fake delete)."""
        campaign = _q1(db_connection, "SELECT id FROM gm_campaigns WHERE user_id = %s AND status = 'ACTIVE' LIMIT 1", (test_user_id,))
        if campaign is None:
            pytest.skip("No active campaign available for delete test")

        resp = auth_client.post(f"{API_PREFIX}/conversations", json={"title": "Delete Campaign Tool"})
        conv_id = extract_data(resp.json())["id"]

        params = json.dumps({"campaign_id": campaign["id"]})
        plan, _ = create_plan_via_db(
            db_connection, test_user_id, conv_id, "Delete Campaign",
            [{"tool_name": "delete_campaign", "tool_params": params, "description": "Delete campaign"}],
        )

        events = self._confirm_and_collect(auth_client, plan["id"])
        completes = [e for e in events if e["event"] == "step_completed"]
        assert len(completes) == 1
        result = completes[0]["data"]["result"]
        if isinstance(result, dict):
            assert result.get("deleted") is True, f"Expected deleted=True, got {result}"
        else:
            assert "deleted" in str(result), f"Expected 'deleted' in result: {result}"

    # ── Query Tool with params: get_campaign_detail ─────────────

    def test_tool_get_campaign_detail(self, auth_client, test_user_id, db_connection):
        """Test get_campaign_detail with valid campaign_id."""
        campaign = _q1(db_connection, "SELECT id FROM gm_campaigns WHERE user_id = %s LIMIT 1", (test_user_id,))
        if campaign is None:
            campaign = _insert_ret(db_connection,
                "INSERT INTO gm_campaigns (user_id, name, platform_id, region_id, ai_model_id, status, created_at) VALUES (%s, 'detail_test_campaign', 2, 1, 1, 'ACTIVE', NOW()) RETURNING *",
                (test_user_id,))
        assert campaign is not None

        resp = auth_client.post(f"{API_PREFIX}/conversations", json={"title": "Campaign Detail Tool"})
        conv_id = extract_data(resp.json())["id"]

        params = json.dumps({"campaign_id": campaign["id"]})
        plan, _ = create_plan_via_db(
            db_connection, test_user_id, conv_id, "Campaign Detail",
            [{"tool_name": "get_campaign_detail", "tool_params": params, "description": "Get detail"}],
        )

        events = self._confirm_and_collect(auth_client, plan["id"])
        completes = [e for e in events if e["event"] == "step_completed"]
        assert len(completes) == 1

    # ── Query Tool with params: get_publish_plan_detail ──────────

    def test_tool_get_publish_plan_detail(self, auth_client, test_user_id, db_connection):
        """Test get_publish_plan_detail with a valid plan_id from gm_aipub_plans."""
        aipub_plan = _q1(db_connection, "SELECT id FROM gm_aipub_plans WHERE user_id = %s LIMIT 1", (test_user_id,))
        if aipub_plan is None:
            aipub_plan = _insert_ret(db_connection,
                "INSERT INTO gm_aipub_plans (user_id, name, platform_id, content_type, plan_type, status, group_id, created_at) VALUES (%s, 'prereq_plan', 2, 'video', 'batch_text', 'pending', (SELECT id FROM gm_social_groups WHERE user_id = %s LIMIT 1), NOW()) RETURNING *",
                (test_user_id, test_user_id)
            )
        if aipub_plan is None:
            aipub_plan = _insert_ret(db_connection,
                "INSERT INTO gm_aipub_plans (user_id, name, platform_id, content_type, plan_type, status, social_account_id, created_at) VALUES (%s, 'prereq_plan', 2, 'video', 'batch_text', 'pending', (SELECT id FROM gm_social_accounts WHERE user_id = %s LIMIT 1), NOW()) RETURNING *",
                (test_user_id,))
        assert aipub_plan is not None

        resp = auth_client.post(f"{API_PREFIX}/conversations", json={"title": "Plan Detail Tool"})
        conv_id = extract_data(resp.json())["id"]

        params = json.dumps({"plan_id": aipub_plan["id"]})
        plan, _ = create_plan_via_db(
            db_connection, test_user_id, conv_id, "Plan Detail",
            [{"tool_name": "get_publish_plan_detail", "tool_params": params, "description": "Get plan detail"}],
        )

        events = self._confirm_and_collect(auth_client, plan["id"])
        completes = [e for e in events if e["event"] == "step_completed"]
        assert len(completes) == 1

    # ── Mutation Tool: retry_publish_plan ────────────────────────

    def test_tool_retry_publish_plan(self, auth_client, test_user_id, db_connection):
        """Test retry_publish_plan with a failed plan (created if needed)."""
        failed_plan = _q1(db_connection, "SELECT id FROM gm_aipub_plans WHERE user_id = %s AND status = 'failed' LIMIT 1", (test_user_id,))
        if failed_plan is None:
            failed_plan = _insert_ret(db_connection,
                "INSERT INTO gm_aipub_plans (user_id, name, platform_id, content_type, plan_type, status, group_id, created_at) VALUES (%s, 'retry_test_plan', 2, 'video', 'batch_text', 'failed', (SELECT id FROM gm_social_groups WHERE user_id = %s LIMIT 1), NOW()) RETURNING *",
                (test_user_id, test_user_id)
            )
        if failed_plan is None:
            failed_plan = _insert_ret(db_connection,
                "INSERT INTO gm_aipub_plans (user_id, name, platform_id, content_type, plan_type, status, social_account_id, created_at) VALUES (%s, 'retry_test_plan', 2, 'video', 'batch_text', 'failed', (SELECT id FROM gm_social_accounts WHERE user_id = %s LIMIT 1), NOW()) RETURNING *",
                (test_user_id,))
        assert failed_plan is not None

        resp = auth_client.post(f"{API_PREFIX}/conversations", json={"title": "Retry Plan Tool"})
        conv_id = extract_data(resp.json())["id"]

        params = json.dumps({"plan_id": failed_plan["id"]})
        plan, _ = create_plan_via_db(
            db_connection, test_user_id, conv_id, "Retry Plan",
            [{"tool_name": "retry_publish_plan", "tool_params": params, "description": "Retry"}],
        )

        events = self._confirm_and_collect(auth_client, plan["id"])
        assert len(events) > 0

    # ── Unknown tool should fail gracefully ─────────────────────

    def test_tool_unknown_name_fails(self, auth_client, test_user_id, db_connection):
        """A plan step with an unknown tool name should fail with error."""
        resp = auth_client.post(f"{API_PREFIX}/conversations", json={"title": "Unknown Tool"})
        conv_id = extract_data(resp.json())["id"]

        plan, _ = create_plan_via_db(
            db_connection, test_user_id, conv_id, "Unknown Tool Plan",
            [{"tool_name": "nonexistent_tool", "tool_params": "{}", "description": "Should fail"}],
        )

        events = self._confirm_and_collect(auth_client, plan["id"])
        fails = [e for e in events if e["event"] == "step_failed"]
        assert len(fails) == 1, "Unknown tool should produce step_failed"
        assert "error" in fails[0]["data"]

        plan_row = _q1(db_connection, "SELECT status FROM gm_ai_plans WHERE id = %s", (plan["id"],))
        assert plan_row["status"] == "failed"

    # ── Sensitive field redaction in tool results ────────────────

    def test_tool_results_redact_sensitive_fields(self, auth_client, test_user_id, db_connection):
        """Verify tool execution results don't contain raw sensitive data."""
        resp = auth_client.post(f"{API_PREFIX}/conversations", json={"title": "Redaction Check"})
        conv_id = extract_data(resp.json())["id"]

        plan, _ = create_plan_via_db(
            db_connection, test_user_id, conv_id, "Redaction Plan",
            [{"tool_name": "list_social_accounts", "tool_params": "{}", "description": "Check redaction"}],
        )

        events = self._confirm_and_collect(auth_client, plan["id"])
        completes = [e for e in events if e["event"] == "step_completed"]

        for c in completes:
            result_str = json.dumps(c["data"].get("result", {}))
            assert "password_hash" not in result_str, "password_hash must be redacted"
            for sensitive in ["jwt_secret"]:
                if sensitive in result_str:
                    assert "****" in result_str, f"{sensitive} must be redacted"

    # ── Audit logs for every tool call ──────────────────────────

    def test_audit_logs_for_all_tools(self, auth_client, test_user_id, db_connection):
        """Verify audit logs are created for each tool execution in plan confirm."""
        resp = auth_client.post(f"{API_PREFIX}/conversations", json={"title": "Audit Batch"})
        conv_id = extract_data(resp.json())["id"]

        plan, _ = create_plan_via_db(
            db_connection, test_user_id, conv_id, "Audit Batch Plan",
            [
                {"tool_name": "list_platforms", "tool_params": "{}", "description": "s1"},
                {"tool_name": "get_wallet_balance", "tool_params": "{}", "description": "s2"},
                {"tool_name": "list_ai_models", "tool_params": "{}", "description": "s3"},
            ],
        )

        _ = self._confirm_and_collect(auth_client, plan["id"])
        time.sleep(1)

        logs = _qall(
            db_connection,
            "SELECT * FROM gm_ai_tool_audit_logs WHERE conversation_id = %s ORDER BY created_at",
            (conv_id,),
        )
        tool_names_logged = [l["tool_name"] for l in logs]
        assert "list_platforms" in tool_names_logged
        assert "get_wallet_balance" in tool_names_logged
        assert "list_ai_models" in tool_names_logged

        for log in logs:
            assert log["user_id"] == test_user_id
            assert log["success"] is True
            assert log["safety_level"] == "read_only"


# ===========================================================================
# 13. Edge case / boundary tests
# ===========================================================================


class TestEdgeCases:
    """Boundary condition tests."""

    def test_empty_message_handled(self, auth_client):
        """Empty string message should be rejected or handled gracefully."""
        resp = auth_client.post(f"{API_PREFIX}/conversations", json={"title": "Empty Msg"})
        conv_id = extract_data(resp.json())["id"]

        resp = auth_client.post(
            f"{API_PREFIX}/conversations/{conv_id}/messages",
            json={"content": ""},
            stream=True,
            timeout=60,
        )
        if resp.status_code in [400, 422]:
            return
        events = parse_sse_events(resp)
        resp.close()
        error_events = [e for e in events if e["event"] == "error"]
        end_events = [e for e in events if e["event"] == "message_end"]
        assert len(error_events) >= 1 or len(end_events) >= 1, (
            "Empty message should produce error or complete normally"
        )

    def test_special_characters_in_title(self, auth_client):
        """Conversation title with special chars should not cause SQL injection."""
        special = "'; DROP TABLE gm_ai_conversations; --"
        resp = auth_client.post(f"{API_PREFIX}/conversations", json={"title": special})
        assert resp.status_code == 200
        data = extract_data(resp.json())
        assert data["title"] == special

        resp = auth_client.get(f"{API_PREFIX}/conversations/{data['id']}")
        assert resp.status_code == 200
        assert extract_data(resp.json())["title"] == special

    def test_unicode_emoji_in_message(self, auth_client):
        """Unicode emoji in messages should be handled correctly."""
        resp = auth_client.post(f"{API_PREFIX}/conversations", json={"title": "Emoji 🎉"})
        conv_id = extract_data(resp.json())["id"]
        assert resp.status_code == 200

        resp = auth_client.post(
            f"{API_PREFIX}/conversations/{conv_id}/messages",
            json={"content": "测试 emoji 🤖💬🎉"},
            stream=True,
            timeout=30,
        )
        content_type = resp.headers.get("content-type", "")
        assert "text/event-stream" in content_type
        resp.close()

    def test_message_exactly_4000_chars(self, auth_client):
        """Message at exact limit (4000) should be accepted."""
        resp = auth_client.post(f"{API_PREFIX}/conversations", json={"title": "4000 chars"})
        conv_id = extract_data(resp.json())["id"]

        resp = auth_client.post(
            f"{API_PREFIX}/conversations/{conv_id}/messages",
            json={"content": "a" * 4000},
            stream=True,
            timeout=60,
        )
        content_type = resp.headers.get("content-type", "")
        assert "text/event-stream" in content_type
        resp.close()

    def test_message_4001_chars_rejected(self, auth_client):
        """Message at 4001 chars should be rejected."""
        resp = auth_client.post(f"{API_PREFIX}/conversations", json={"title": "4001 chars"})
        conv_id = extract_data(resp.json())["id"]

        resp = auth_client.post(
            f"{API_PREFIX}/conversations/{conv_id}/messages",
            json={"content": "a" * 4001},
            stream=True,
            timeout=30,
        )
        if resp.status_code in [400, 422]:
            return
        events = parse_sse_events(resp)
        resp.close()
        error_events = [e for e in events if e["event"] == "error"]
        assert len(error_events) >= 1, (
            f"4001-char message should be rejected via HTTP or SSE error, got status {resp.status_code}"
        )

    def test_send_message_to_nonexistent_conversation(self, auth_client):
        """Sending to non-existent conversation should fail."""
        resp = auth_client.post(
            f"{API_PREFIX}/conversations/999999/messages",
            json={"content": "hello"},
        )
        assert resp.status_code in [404, 400]

    def test_confirm_nonexistent_plan(self, auth_client):
        """Confirming non-existent plan should fail."""
        resp = auth_client.post(
            f"{API_PREFIX}/plans/999999/confirm",
            stream=True,
            timeout=10,
        )
        events = parse_sse_events(resp)
        resp.close()
        error_events = [e for e in events if e["event"] == "error"]
        assert resp.status_code in [404, 400] or len(error_events) >= 1


# ===========================================================================
# 14. Streaming output verification (requires LLM)
# ===========================================================================


@pytest.mark.requires_llm
class TestStreaming:
    """Verify streaming output produces multiple text_delta events."""

    def test_streaming_multiple_text_deltas(self, auth_client):
        """After streaming fix, text_delta events should be >= 3 (not one bulk)."""
        resp = auth_client.post(f"{API_PREFIX}/conversations", json={"title": "Stream Multi"})
        conv_id = extract_data(resp.json())["id"]

        resp = auth_client.post(
            f"{API_PREFIX}/conversations/{conv_id}/messages",
            json={"content": "请用三段话介绍一下人工智能的发展历史"},
            stream=True,
            timeout=90,
        )
        events = parse_sse_events(resp)

        deltas = [e for e in events if e["event"] == "text_delta"]
        assert len(deltas) >= 3, (
            f"Streaming should produce >= 3 text_delta events, got {len(deltas)}. "
            "If only 1, the backend may still be using non-streaming chat_completion."
        )

    def test_streaming_delta_concatenation(self, auth_client, db_connection):
        """All text_delta.delta concatenated should equal the DB assistant message content."""
        resp = auth_client.post(f"{API_PREFIX}/conversations", json={"title": "Stream Concat"})
        conv_id = extract_data(resp.json())["id"]

        resp = auth_client.post(
            f"{API_PREFIX}/conversations/{conv_id}/messages",
            json={"content": "你好"},
            stream=True,
            timeout=90,
        )
        events = parse_sse_events(resp)

        deltas = [e for e in events if e["event"] == "text_delta"]
        concatenated = "".join(
            (e["data"].get("delta", "") if isinstance(e["data"], dict) else str(e["data"]))
            for e in deltas
        )

        time.sleep(1)
        row = _q1(
            db_connection,
            "SELECT content FROM gm_ai_messages WHERE conversation_id = %s AND role = 'assistant' ORDER BY id DESC LIMIT 1",
            (conv_id,),
        )
        if row:
            assert row["content"] == concatenated, "Concatenated deltas must match DB content"

    def test_streaming_event_ordering(self, auth_client):
        """Events must be ordered: text_delta(s) then message_end."""
        resp = auth_client.post(f"{API_PREFIX}/conversations", json={"title": "Stream Order"})
        conv_id = extract_data(resp.json())["id"]

        resp = auth_client.post(
            f"{API_PREFIX}/conversations/{conv_id}/messages",
            json={"content": "你好"},
            stream=True,
            timeout=90,
        )
        events = parse_sse_events(resp)

        saw_message_end = False
        for evt in events:
            if evt["event"] == "message_end":
                saw_message_end = True
            if evt["event"] == "text_delta" and saw_message_end:
                pytest.fail("text_delta must not appear after message_end")

    def test_streaming_tool_then_stream(self, auth_client):
        """Tool call scenario: tool events first, then streamed text_delta(s), then message_end."""
        resp = auth_client.post(f"{API_PREFIX}/conversations", json={"title": "Stream Tool"})
        conv_id = extract_data(resp.json())["id"]

        resp = auth_client.post(
            f"{API_PREFIX}/conversations/{conv_id}/messages",
            json={"content": "查看我的钱包余额"},
            stream=True,
            timeout=90,
        )
        events = parse_sse_events(resp)

        tc_starts = [e for e in events if e["event"] == "tool_call_start"]
        deltas = [e for e in events if e["event"] == "text_delta"]
        ends = [e for e in events if e["event"] == "message_end"]

        if tc_starts:
            assert len(deltas) >= 1, "After tool call, should have text summary"
            assert len(ends) >= 1, "Must end with message_end"

            last_tool_idx = max(
                i for i, e in enumerate(events)
                if e["event"] in ("tool_call_start", "tool_call_result")
            )
            first_delta_idx = next(
                (i for i, e in enumerate(events) if e["event"] == "text_delta"),
                len(events),
            )
            assert first_delta_idx > last_tool_idx, "text_delta must come after tool events"


# ===========================================================================
# 15. Tool call completeness
# ===========================================================================


class TestToolCompleteness:
    """Verify tool_call_start/result pairing and execution completeness."""

    @pytest.mark.requires_llm
    def test_tool_call_start_has_matching_result(self, auth_client):
        """Every tool_call_start must have a corresponding tool_call_result."""
        resp = auth_client.post(f"{API_PREFIX}/conversations", json={"title": "Tool Pair"})
        conv_id = extract_data(resp.json())["id"]

        resp = auth_client.post(
            f"{API_PREFIX}/conversations/{conv_id}/messages",
            json={"content": "看看我的仪表盘数据"},
            stream=True,
            timeout=90,
        )
        events = parse_sse_events(resp)

        starts = {e["data"]["tool_call_id"] for e in events if e["event"] == "tool_call_start"}
        results = {e["data"]["tool_call_id"] for e in events if e["event"] == "tool_call_result"}

        if starts:
            assert starts == results, f"Unmatched tool calls: starts={starts}, results={results}"

    def test_delete_campaign_actually_stops(self, auth_client, test_user_id, db_connection):
        """delete_campaign tool should update campaign status to stopped."""
        campaign = _q1(
            db_connection,
            "SELECT * FROM gm_campaigns WHERE user_id = %s AND status = 'ACTIVE' ORDER BY id DESC LIMIT 1",
            (test_user_id,),
        )
        if campaign is None:
            pytest.skip("No active campaign available for delete-verify test")

        resp = auth_client.post(f"{API_PREFIX}/conversations", json={"title": "Delete Verify"})
        conv_id = extract_data(resp.json())["id"]

        params = json.dumps({"campaign_id": campaign["id"]})
        plan, _ = create_plan_via_db(
            db_connection, test_user_id, conv_id, "Delete Verify",
            [{"tool_name": "delete_campaign", "tool_params": params, "description": "Stop campaign"}],
        )

        resp = auth_client.post(
            f"{API_PREFIX}/plans/{plan['id']}/confirm",
            stream=True,
            timeout=60,
        )
        events = parse_sse_events(resp)

        completes = [e for e in events if e["event"] == "step_completed"]
        assert len(completes) == 1

        row = _q1(db_connection, "SELECT status FROM gm_campaigns WHERE id = %s", (campaign["id"],))
        assert row is not None
        assert row["status"].lower() in ("stopped", "deleted"), (
            f"Expected stopped/deleted, got {row['status']}"
        )

    def test_plan_with_many_steps(self, auth_client, test_user_id, db_connection):
        """Plan with > 10 steps should execute all steps (no per-turn limit in plan confirm)."""
        resp = auth_client.post(f"{API_PREFIX}/conversations", json={"title": "Many Steps"})
        conv_id = extract_data(resp.json())["id"]

        steps = [
            {"tool_name": "list_platforms", "tool_params": "{}", "description": f"Step {i + 1}"}
            for i in range(12)
        ]
        plan, _ = create_plan_via_db(db_connection, test_user_id, conv_id, "12 Steps", steps)

        resp = auth_client.post(
            f"{API_PREFIX}/plans/{plan['id']}/confirm",
            stream=True,
            timeout=120,
        )
        events = parse_sse_events(resp)

        starts = [e for e in events if e["event"] == "step_start"]
        completes = [e for e in events if e["event"] == "step_completed"]
        plan_done = [e for e in events if e["event"] == "plan_completed"]

        assert len(starts) == 12, f"All 12 steps should start, got {len(starts)}"
        assert len(completes) == 12, f"All 12 steps should complete, got {len(completes)}"
        assert len(plan_done) == 1

        plan_row = _q1(db_connection, "SELECT status FROM gm_ai_plans WHERE id = %s", (plan["id"],))
        assert plan_row["status"] == "completed"


# ===========================================================================
# 16. Auto title management
# ===========================================================================


class TestAutoTitle:
    """Verify conversation title management behavior."""

    def test_title_unchanged_after_send_message(self, auth_client, db_connection):
        """Backend does NOT auto-update title on message send."""
        resp = auth_client.post(f"{API_PREFIX}/conversations", json={})
        conv_id = extract_data(resp.json())["id"]

        resp = auth_client.post(
            f"{API_PREFIX}/conversations/{conv_id}/messages",
            json={"content": "请帮我查看钱包余额"},
            stream=True,
            timeout=60,
        )
        _ = parse_sse_events(resp)

        row = _q1(db_connection, "SELECT title FROM gm_ai_conversations WHERE id = %s", (conv_id,))
        assert row["title"] == "New Chat", "Backend should not auto-update title"

    def test_manual_title_update_after_message(self, auth_client, db_connection):
        """After sending a message, title can be updated via PATCH API."""
        resp = auth_client.post(f"{API_PREFIX}/conversations", json={})
        conv_id = extract_data(resp.json())["id"]

        resp = auth_client.post(
            f"{API_PREFIX}/conversations/{conv_id}/messages",
            json={"content": "帮我看看仪表盘数据"},
            stream=True,
            timeout=60,
        )
        _ = parse_sse_events(resp)

        new_title = "仪表盘数据查询"
        resp = auth_client.patch(
            f"{API_PREFIX}/conversations/{conv_id}",
            json={"title": new_title},
        )
        assert resp.status_code == 200
        data = extract_data(resp.json())
        assert data["title"] == new_title

        row = _q1(db_connection, "SELECT title, updated_at FROM gm_ai_conversations WHERE id = %s", (conv_id,))
        assert row["title"] == new_title
        assert row["updated_at"] is not None, "updated_at should be set after title update"

    def test_custom_title_preserved_after_message(self, auth_client, db_connection):
        """Conversation with custom title should keep it after messages."""
        custom = "我的自定义标题"
        resp = auth_client.post(f"{API_PREFIX}/conversations", json={"title": custom})
        conv_id = extract_data(resp.json())["id"]

        resp = auth_client.post(
            f"{API_PREFIX}/conversations/{conv_id}/messages",
            json={"content": "你好"},
            stream=True,
            timeout=60,
        )
        _ = parse_sse_events(resp)

        row = _q1(db_connection, "SELECT title FROM gm_ai_conversations WHERE id = %s", (conv_id,))
        assert row["title"] == custom, "Custom title must be preserved after message"


# ===========================================================================
# 17. Interactive create flows — multi-turn with DB verification
# ===========================================================================


@pytest.mark.requires_llm
class TestInteractiveCreateFlows:
    """Test complete interactive creation flows with multi-turn conversations
    and full database verification after plan confirmation."""

    def _send_and_collect(self, auth_client, conv_id, content, timeout=90):
        resp = auth_client.post(
            f"{API_PREFIX}/conversations/{conv_id}/messages",
            json={"content": content},
            stream=True,
            timeout=timeout,
        )
        return parse_sse_events(resp)

    def _extract_plan_id(self, events):
        plan_events = [e for e in events if e["event"] == "plan_created"]
        return plan_events[0]["data"]["plan_id"] if plan_events else None

    def _confirm_plan(self, auth_client, plan_id):
        resp = auth_client.post(
            f"{API_PREFIX}/plans/{plan_id}/confirm",
            stream=True,
            timeout=90,
        )
        return parse_sse_events(resp)

    def test_create_campaign_interactive(self, auth_client, test_user_id, db_connection):
        """Multi-turn: create campaign via AI chat -> confirm plan -> verify DB."""
        ts = int(time.time())
        campaign_name = f"API Chat Campaign {ts}"

        resp = auth_client.post(f"{API_PREFIX}/conversations", json={"title": "Campaign Create Flow"})
        conv_id = extract_data(resp.json())["id"]

        # Turn 1: provide intent + details
        events_1 = self._send_and_collect(
            auth_client, conv_id,
            f"帮我创建一个 TikTok 营销活动，名称叫 {campaign_name}",
        )
        plan_id = self._extract_plan_id(events_1)

        if plan_id is None:
            events_2 = self._send_and_collect(
                auth_client, conv_id,
                f"平台 TikTok，名称 {campaign_name}，预算 500",
            )
            plan_id = self._extract_plan_id(events_2)

        if plan_id is None:
            events_3 = self._send_and_collect(
                auth_client, conv_id,
                "确认创建",
            )
            plan_id = self._extract_plan_id(events_3)
            assert plan_id is not None, "Must get plan_created after 3 turns"

        # Verify draft
        plan_row = _q1(db_connection, "SELECT * FROM gm_ai_plans WHERE id = %s", (plan_id,))
        assert plan_row is not None
        assert plan_row["status"] == "draft"

        # Confirm
        confirm_events = self._confirm_plan(auth_client, plan_id)
        plan_done = [e for e in confirm_events if e["event"] == "plan_completed"]
        assert len(plan_done) == 1

        time.sleep(1)

        # DB: plan completed
        plan_row = _q1(db_connection, "SELECT status FROM gm_ai_plans WHERE id = %s", (plan_id,))
        assert plan_row["status"] in ("completed", "failed")

        # DB: step
        steps = _qall(db_connection, "SELECT * FROM gm_ai_plan_steps WHERE plan_id = %s", (plan_id,))
        create_step = next((s for s in steps if s["tool_name"] == "create_campaign"), None)
        assert create_step is not None

        if create_step["status"] == "completed":
            assert create_step["result"] is not None
            campaign = _q1(
                db_connection,
                "SELECT * FROM gm_campaigns WHERE user_id = %s ORDER BY id DESC LIMIT 1",
                (test_user_id,),
            )
            assert campaign is not None, "Campaign must exist in DB"

        # DB: audit
        logs = _qall(
            db_connection,
            "SELECT * FROM gm_ai_tool_audit_logs WHERE conversation_id = %s",
            (conv_id,),
        )
        assert len(logs) >= 1

    def test_create_social_group_interactive(self, auth_client, test_user_id, db_connection):
        """Multi-turn: create social group via AI chat -> confirm -> verify DB."""
        ts = int(time.time())
        group_name = f"API Chat Group {ts}"

        resp = auth_client.post(f"{API_PREFIX}/conversations", json={"title": "Group Create Flow"})
        conv_id = extract_data(resp.json())["id"]

        events_1 = self._send_and_collect(
            auth_client, conv_id,
            f"帮我创建一个 TikTok 账号分组，名称叫 {group_name}",
        )
        plan_id = self._extract_plan_id(events_1)

        if plan_id is None:
            events_2 = self._send_and_collect(
                auth_client, conv_id,
                f"确认，TikTok 平台，分组名 {group_name}",
            )
            plan_id = self._extract_plan_id(events_2)

        if plan_id is None:
            events_3 = self._send_and_collect(auth_client, conv_id, "确认创建")
            plan_id = self._extract_plan_id(events_3)
            assert plan_id is not None, "Must get plan_created for group"

        plan_row = _q1(db_connection, "SELECT * FROM gm_ai_plans WHERE id = %s", (plan_id,))
        assert plan_row["status"] == "draft"

        confirm_events = self._confirm_plan(auth_client, plan_id)
        assert len([e for e in confirm_events if e["event"] == "plan_completed"]) == 1

        time.sleep(1)

        plan_row = _q1(db_connection, "SELECT status FROM gm_ai_plans WHERE id = %s", (plan_id,))
        assert plan_row["status"] in ("completed", "failed")

        steps = _qall(db_connection, "SELECT * FROM gm_ai_plan_steps WHERE plan_id = %s", (plan_id,))
        group_step = next((s for s in steps if s["tool_name"] == "create_social_group"), None)
        assert group_step is not None

        if group_step["status"] == "completed":
            row = _q1(
                db_connection,
                "SELECT * FROM gm_social_groups WHERE group_name = %s AND user_id = %s",
                (group_name, test_user_id),
            )
            assert row is not None, f"Group '{group_name}' must exist in DB"

        logs = _qall(
            db_connection,
            "SELECT * FROM gm_ai_tool_audit_logs WHERE conversation_id = %s AND tool_name = 'create_social_group'",
            (conv_id,),
        )
        assert len(logs) >= 1

    def test_create_social_account_interactive(self, auth_client, test_user_id, db_connection):
        """Multi-turn: create social account via AI chat -> confirm -> verify DB."""
        ts = int(time.time())
        username = f"api_chat_acct_{ts}"

        resp = auth_client.post(f"{API_PREFIX}/conversations", json={"title": "Account Create Flow"})
        conv_id = extract_data(resp.json())["id"]

        events_1 = self._send_and_collect(
            auth_client, conv_id,
            f"帮我添加一个 TikTok 社交账号，用户名 {username}",
        )
        plan_id = self._extract_plan_id(events_1)

        if plan_id is None:
            events_2 = self._send_and_collect(
                auth_client, conv_id,
                f"确认，TikTok，用户名 {username}",
            )
            plan_id = self._extract_plan_id(events_2)

        if plan_id is None:
            events_3 = self._send_and_collect(auth_client, conv_id, "确认创建")
            plan_id = self._extract_plan_id(events_3)
            assert plan_id is not None, "Must get plan_created for account"

        plan_row = _q1(db_connection, "SELECT * FROM gm_ai_plans WHERE id = %s", (plan_id,))
        assert plan_row["status"] == "draft"

        confirm_events = self._confirm_plan(auth_client, plan_id)
        assert len([e for e in confirm_events if e["event"] == "plan_completed"]) == 1

        time.sleep(1)

        steps = _qall(db_connection, "SELECT * FROM gm_ai_plan_steps WHERE plan_id = %s", (plan_id,))
        acct_step = next((s for s in steps if s["tool_name"] == "create_social_account"), None)
        assert acct_step is not None

        if acct_step["status"] == "completed":
            row = _q1(
                db_connection,
                "SELECT * FROM gm_social_accounts WHERE username = %s AND user_id = %s",
                (username, test_user_id),
            )
            assert row is not None, f"Account '{username}' must exist in DB"

        logs = _qall(
            db_connection,
            "SELECT * FROM gm_ai_tool_audit_logs WHERE conversation_id = %s AND tool_name = 'create_social_account'",
            (conv_id,),
        )
        assert len(logs) >= 1

    def test_create_publish_plan_full_flow(self, auth_client, test_user_id, db_connection):
        """Multi-turn: create publish plan with full interactive flow and DB verification."""
        resp = auth_client.post(f"{API_PREFIX}/conversations", json={"title": "Publish Plan Flow"})
        conv_id = extract_data(resp.json())["id"]

        before_count = _q1(
            db_connection,
            "SELECT count(*) as cnt FROM gm_aipub_plans WHERE user_id = %s",
            (test_user_id,),
        )["cnt"]

        # Turn 1: intent
        events_1 = self._send_and_collect(auth_client, conv_id, "帮我创建一个发布计划")
        text_1 = "".join(
            (e["data"].get("delta", "") if isinstance(e["data"], dict) else str(e["data"]))
            for e in events_1 if e["event"] == "text_delta"
        )
        assert len(text_1) > 0, "Turn 1 must have text response"

        # Turn 2: platform + type
        events_2 = self._send_and_collect(auth_client, conv_id, "TikTok，视频类型")
        plan_id = self._extract_plan_id(events_2)

        if plan_id is None:
            # Turn 3: group
            group = _q1(
                db_connection,
                "SELECT group_name FROM gm_social_groups WHERE user_id = %s LIMIT 1",
                (test_user_id,),
            )
            group_name = group["group_name"] if group else "默认分组"
            events_3 = self._send_and_collect(auth_client, conv_id, group_name)
            plan_id = self._extract_plan_id(events_3)

        if plan_id is None:
            # Turn 4: prompt
            events_4 = self._send_and_collect(
                auth_client, conv_id,
                "发一个关于美食探店的短视频",
            )
            plan_id = self._extract_plan_id(events_4)

        if plan_id is None:
            events_5 = self._send_and_collect(auth_client, conv_id, "确认创建，用默认模型")
            plan_id = self._extract_plan_id(events_5)
            assert plan_id is not None, "Must get plan_created for publish plan"

        plan_row = _q1(db_connection, "SELECT * FROM gm_ai_plans WHERE id = %s", (plan_id,))
        assert plan_row["status"] == "draft"

        steps = _qall(db_connection, "SELECT * FROM gm_ai_plan_steps WHERE plan_id = %s", (plan_id,))
        pub_step = next((s for s in steps if s["tool_name"] == "create_publish_plan"), None)
        assert pub_step is not None, "Must have create_publish_plan step"

        confirm_events = self._confirm_plan(auth_client, plan_id)
        assert len([e for e in confirm_events if e["event"] == "plan_completed"]) == 1

        time.sleep(2)

        plan_row = _q1(db_connection, "SELECT status FROM gm_ai_plans WHERE id = %s", (plan_id,))
        assert plan_row["status"] in ("completed", "failed")

        steps = _qall(db_connection, "SELECT * FROM gm_ai_plan_steps WHERE plan_id = %s", (plan_id,))
        pub_step = next((s for s in steps if s["tool_name"] == "create_publish_plan"), None)

        if pub_step and pub_step["status"] == "completed":
            after_count = _q1(
                db_connection,
                "SELECT count(*) as cnt FROM gm_aipub_plans WHERE user_id = %s",
                (test_user_id,),
            )["cnt"]
            assert after_count > before_count, (
                f"gm_aipub_plans must increase: {before_count} -> {after_count}"
            )

            newest = _q1(
                db_connection,
                "SELECT * FROM gm_aipub_plans WHERE user_id = %s ORDER BY id DESC LIMIT 1",
                (test_user_id,),
            )
            assert newest is not None
            assert newest["platform_id"] == 2
            assert newest["content_type"] == "video"

        logs = _qall(
            db_connection,
            "SELECT * FROM gm_ai_tool_audit_logs WHERE conversation_id = %s",
            (conv_id,),
        )
        assert len(logs) >= 1

        msgs = _qall(
            db_connection,
            "SELECT role FROM gm_ai_messages WHERE conversation_id = %s",
            (conv_id,),
        )
        user_msgs = [m for m in msgs if m["role"] == "user"]
        assert len(user_msgs) >= 2, f"Should have >= 2 user messages, got {len(user_msgs)}"
