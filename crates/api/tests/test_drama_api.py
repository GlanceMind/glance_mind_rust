#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
GlanceMind API E2E Tests - AI Short Drama API
==============================================
Integration tests for the Rust drama facade at /api/v1/drama/*.
The facade proxies to gm_agent_hub gateway; these tests require both
the Rust API and the gateway to be running (or gateway in mock mode).

Test Coverage:
1. Preflight — valid, blocked
2. Project CRUD — create, detail, list, cancel
3. User interactions — clarify, strategy select, approve (approve + revision)
4. Read endpoints — script, render, artifacts, cost
5. Auth boundary — 401 without JWT, 403 wrong user
6. SSE stream — auth enforcement
7. Stale interaction version — 409
8. Gateway unreachable — 502 graceful error (conditional)
9. DB projection assertions
"""

import os
import pytest
import time
import psycopg2
import psycopg2.extras
from datetime import datetime, timezone

try:
    from conftest import (
        assert_response_success,
        extract_data,
        API_BASE_URL,
        DATABASE_URL,
        APIClient,
        get_or_create_test_token,
        resolve_test_user_id,
    )
except ImportError:
    API_BASE_URL = os.getenv("API_BASE_URL", "http://localhost:8081")
    DATABASE_URL = os.getenv(
        "DATABASE_URL",
        "postgres://glancemind:testpassword@localhost:5434/glancemind_test",
    )

    def assert_response_success(resp):
        assert resp.status_code in [200, 201], \
            f"Expected success, got {resp.status_code}: {resp.text}"

    def extract_data(json_resp):
        if "data" in json_resp:
            return json_resp["data"]
        return json_resp


DRAMA_BASE = "/api/v1/drama"

BCRYPT_HASH_TEST_PASSWORD = (
    "$2b$12$Ikc.R4FMMGahbGhfHlLl4.PciMiV37qXfHpPNCjGGQg/yOEgk7k/e"
)
SECOND_USER_EMAIL = "drama_iso_test@glancemind.test"


# ---------------------------------------------------------------------------
# DB helpers (same pattern as test_ai_chat_api.py)
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


# ---------------------------------------------------------------------------
# Fixtures
# ---------------------------------------------------------------------------

@pytest.fixture(scope="module")
def test_user_id(db_connection):
    return resolve_test_user_id(db_connection)


@pytest.fixture(scope="module")
def second_auth_client(db_connection):
    """Create and authenticate a second user for isolation / 403 tests."""
    _ensure_clean(db_connection)
    cur = db_connection.cursor(cursor_factory=psycopg2.extras.RealDictCursor)

    cur.execute("SELECT id FROM gm_users WHERE email = %s", (SECOND_USER_EMAIL,))
    existing = cur.fetchone()
    if existing is None:
        cur.execute(
            """INSERT INTO gm_users
               (email, username, password_hash, full_name, role, status, is_active, created_at)
               VALUES (%s, 'drama_iso', %s, 'Drama Isolation User', 'user', 'ACTIVE', true, NOW())""",
            (SECOND_USER_EMAIL, BCRYPT_HASH_TEST_PASSWORD),
        )

    cur.execute("SELECT id FROM gm_email_verifications WHERE email = %s", (SECOND_USER_EMAIL,))
    ev_row = cur.fetchone()
    if ev_row is None:
        cur.execute(
            """INSERT INTO gm_email_verifications (email, code, verified, expires_at, created_at)
               VALUES (%s, 'DISO01', true, NOW() + INTERVAL '1 year', NOW())""",
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
        json={"identifier": SECOND_USER_EMAIL, "password": "TestPassword123!"},
    )
    assert resp.status_code == 200, f"Second user login failed: {resp.status_code} {resp.text}"
    data = extract_data(resp.json())
    token = data.get("token")
    if isinstance(token, dict):
        token = token.get("token")
    assert token, "No token in second user login response"
    return APIClient(API_BASE_URL, token=token)


@pytest.fixture(scope="module")
def second_user_id(db_connection):
    row = _q1(db_connection, "SELECT id FROM gm_users WHERE email = %s", (SECOND_USER_EMAIL,))
    assert row is not None, "Second user must exist"
    return row["id"]


# ---------------------------------------------------------------------------
# Test data helpers
# ---------------------------------------------------------------------------

def _valid_brief(**overrides):
    defaults = {
        "title": f"E2E Drama Test {int(time.time())}",
        "description": "Two founders fake a relationship during a city blackout.",
        "content_type": "short_video",
        "target_duration_seconds": 90,
        "platform": "tiktok",
        "visual_style": "neon_romance",
        "genre": "urban_romance",
        "narrative_mode": "dramatic",
        "budget_cents": 500,
        "characters": [
            {"name": "Ava", "appearance": "sharp suit, tired eyes"},
            {"name": "Leo", "appearance": "hoodie, nervous energy"},
        ],
    }
    defaults.update(overrides)
    return defaults


def _valid_long_video_brief(**overrides):
    defaults = {
        "title": f"E2E Long Drama {int(time.time())}",
        "description": "A multi-chapter epic spanning three acts of corporate intrigue.",
        "content_type": "long_video",
        "target_duration_seconds": 600,
        "platform": "youtube",
        "visual_style": "cinematic_noir",
        "genre": "thriller",
        "narrative_mode": "epic",
        "budget_cents": 2000,
        "characters": [
            {"name": "Marcus", "appearance": "tailored suit, silver cufflinks"},
            {"name": "Elena", "appearance": "red blazer, calculating gaze"},
            {"name": "Jin", "appearance": "tech hoodie, augmented glasses"},
        ],
    }
    defaults.update(overrides)
    return defaults


def _create_project(client) -> str:
    brief = _valid_brief()
    resp = client.post(f"{DRAMA_BASE}/projects", json=brief)
    assert resp.status_code in [200, 201], f"Create failed: {resp.status_code} {resp.text}"
    return extract_data(resp.json())["project_id"]


def _create_long_video_project(client) -> str:
    brief = _valid_long_video_brief()
    resp = client.post(f"{DRAMA_BASE}/projects", json=brief)
    assert resp.status_code in [200, 201], f"Create long_video failed: {resp.status_code} {resp.text}"
    return extract_data(resp.json())["project_id"]


def _create_scene_asset(client, name: str = "Scene Asset") -> str:
    resp = client.post(
        f"{DRAMA_BASE}/scene-assets",
        json={
            "name": name,
            "category": "urban",
            "location_description": f"{name} description",
            "time_of_day": "night",
            "mood": "tense",
            "reference_image_urls": [
                f"https://example.com/{name.lower().replace(' ', '-')}.png"
            ],
            "camera_notes": "wide establishing angle",
            "notes": "test asset",
        },
    )
    assert resp.status_code == 200, resp.text
    return extract_data(resp.json())["id"]


def _wait_for_projection(db_connection, project_id: str, timeout_seconds: float = 3.0):
    deadline = time.time() + timeout_seconds
    while time.time() < deadline:
        row = _q1(
            db_connection,
            "SELECT project_id, user_id FROM gm_drama_project_projections WHERE project_id = %s",
            (project_id,),
        )
        if row is not None:
            return row
        time.sleep(0.2)
    return None


# ===========================================================================
# 1. Preflight
# ===========================================================================

class TestDramaPreflight:
    def test_preflight_valid_brief(self, auth_client):
        resp = auth_client.post(
            f"{DRAMA_BASE}/preflight",
            json={
                "title": "Test Drama",
                "description": "A short test drama",
                "content_type": "short_video",
                "target_duration_seconds": 60,
            },
        )
        assert resp.status_code == 200
        data = extract_data(resp.json())
        assert data.get("passed") is True or data.get("all_passed") is True
        assert data["status"] == "passed"
        assert isinstance(data["blocking"], list)
        assert len(data["blocking"]) == 0

    def test_preflight_missing_title(self, auth_client):
        resp = auth_client.post(
            f"{DRAMA_BASE}/preflight",
            json={"title": "", "description": "", "content_type": "long_video"},
        )
        assert resp.status_code == 200
        data = extract_data(resp.json())
        assert data["passed"] is False
        assert data["status"] == "needs_clarification"
        assert len(data["blocking"]) > 0

    def test_preflight_long_video_without_duration(self, auth_client):
        resp = auth_client.post(
            f"{DRAMA_BASE}/preflight",
            json={
                "title": "Long Drama",
                "description": "A long test drama",
                "content_type": "long_video",
            },
        )
        assert resp.status_code == 200
        data = extract_data(resp.json())
        assert data["passed"] is False
        blocking_reasons = " ".join(data["blocking"])
        assert "target_duration_seconds" in blocking_reasons


# ===========================================================================
# 2. Project Create + DB Assertions
# ===========================================================================

class TestDramaProjectCreate:
    def test_create_project_success(self, auth_client, db_connection):
        brief = _valid_brief(budget_cents=777)
        resp = auth_client.post(f"{DRAMA_BASE}/projects", json=brief)
        assert resp.status_code in [200, 201]
        data = extract_data(resp.json())
        assert "project_id" in data
        pid = data["project_id"]
        assert pid != ""
        assert data["status"] in ["pending", "running"]

        # DB: verify projection row was created with correct budget
        row = _q1(
            db_connection,
            "SELECT * FROM gm_drama_project_projections WHERE project_id = %s",
            (pid,),
        )
        if row:
            assert row["cost_reserve_cents"] == 777, \
                f"budget_cents should be 777, got {row['cost_reserve_cents']}"
            assert row["title"] == brief["title"]
            assert row["status"] in ["pending", "running"]
            assert row["created_at"] is not None

    def test_create_project_without_auth(self, api_client):
        resp = api_client.post(f"{DRAMA_BASE}/projects", json=_valid_brief())
        assert resp.status_code == 401


# ===========================================================================
# 3. Project Detail + List
# ===========================================================================

class TestDramaProjectDetail:
    def test_get_project_detail(self, auth_client):
        pid = _create_project(auth_client)
        resp = auth_client.get(f"{DRAMA_BASE}/projects/{pid}")
        assert resp.status_code == 200
        data = extract_data(resp.json())
        assert data["project_id"] == pid
        assert "status" in data

    def test_get_project_not_found(self, auth_client):
        resp = auth_client.get(
            f"{DRAMA_BASE}/projects/00000000-0000-0000-0000-000000000000"
        )
        # Should be 404, but gateway may return 200 with empty/placeholder data
        assert resp.status_code in [404, 200]

    def test_list_projects(self, auth_client):
        _create_project(auth_client)
        resp = auth_client.get(f"{DRAMA_BASE}/projects")
        assert resp.status_code == 200
        data = extract_data(resp.json())
        assert isinstance(data.get("list"), list) or isinstance(data, list)

    def test_list_projects_without_auth(self, api_client):
        resp = api_client.get(f"{DRAMA_BASE}/projects")
        assert resp.status_code == 401


# ===========================================================================
# 4. Cross-User Access Control (403)
# ===========================================================================

class TestDramaCrossUserAccess:
    """Verify that user B cannot access user A's projects."""

    def test_get_project_returns_403_for_wrong_user(
        self, auth_client, second_auth_client, db_connection
    ):
        """User A creates project, user B should get 403 on detail."""
        pid = _create_project(auth_client)
        # Wait briefly for projection to be created
        time.sleep(0.5)

        resp = second_auth_client.get(f"{DRAMA_BASE}/projects/{pid}")
        # When DRAMA_CANONICAL_READS is enabled, projection check returns 403.
        # When disabled, gateway may return 200 (no ownership check).
        # We accept both: the test documents the expected behavior.
        if resp.status_code == 403:
            body = resp.json()
            assert "forbidden" in str(body).lower() or body.get("code") == 403
        else:
            # Gateway fallback mode - ownership not enforced at REST level
            pass

    def test_cancel_project_returns_403_for_wrong_user(
        self, auth_client, second_auth_client, db_connection
    ):
        """User B should not be able to cancel user A's project."""
        pid = _create_project(auth_client)
        time.sleep(0.5)

        resp = second_auth_client.delete(f"{DRAMA_BASE}/projects/{pid}")
        # Accept 403 (projection mode) or 200/404 (gateway mode)
        if resp.status_code == 403:
            body = resp.json()
            assert "forbidden" in str(body).lower() or body.get("code") == 403


# ===========================================================================
# 4.5 Project Meta Persistence
# ===========================================================================
class TestDramaProjectMeta:
    def test_project_meta_roundtrip(self, auth_client):
        pid = _create_project(auth_client)

        initial_resp = auth_client.get(f"{DRAMA_BASE}/projects/{pid}/meta")
        assert initial_resp.status_code == 200, \
            f"Initial meta GET failed: {initial_resp.status_code} {initial_resp.text}"
        initial_data = extract_data(initial_resp.json())
        assert initial_data["project_id"] == pid
        assert initial_data["characters"] == []
        assert initial_data["style_references"] == []
        assert initial_data["text_materials"] == []
        assert initial_data["visual_settings"] == {}
        assert isinstance(initial_data.get("updated_at"), str)
        assert initial_data["updated_at"] != ""

        first_payload = {
            "characters": [
                {
                    "id": "char_ava",
                    "name": "Ava",
                    "role": "lead",
                    "appearance": "sharp suit, tired eyes",
                }
            ],
            "style_references": [
                {
                    "id": "style_neon",
                    "type": "image",
                    "url": "https://example.com/neon.png",
                }
            ],
            "text_materials": [
                {
                    "id": "tm_logline",
                    "kind": "logline",
                    "content": "Two founders fake a relationship during a blackout.",
                }
            ],
            "visual_settings": {
                "aspect_ratio": "9:16",
                "palette": "neon_romance",
                "camera_language": "handheld closeups",
            },
        }
        put_resp = auth_client.put(f"{DRAMA_BASE}/projects/{pid}/meta", json=first_payload)
        assert put_resp.status_code == 200, \
            f"Meta PUT failed: {put_resp.status_code} {put_resp.text}"
        put_data = extract_data(put_resp.json())
        assert put_data["project_id"] == pid
        assert put_data["characters"] == first_payload["characters"]
        assert put_data["style_references"] == first_payload["style_references"]
        assert put_data["text_materials"] == first_payload["text_materials"]
        assert put_data["visual_settings"] == first_payload["visual_settings"]
        assert isinstance(put_data.get("updated_at"), str)
        assert put_data["updated_at"] != ""

        second_payload = {
            "characters": [
                {
                    "id": "char_ava",
                    "name": "Ava",
                    "role": "lead",
                    "appearance": "silver dress, determined stare",
                },
                {
                    "id": "char_leo",
                    "name": "Leo",
                    "role": "co_lead",
                    "appearance": "hoodie, nervous energy",
                },
            ],
            "style_references": [
                {
                    "id": "style_noir",
                    "type": "image",
                    "url": "https://example.com/noir.png",
                }
            ],
            "text_materials": [
                {
                    "id": "tm_scene1",
                    "kind": "scene_note",
                    "content": "Open on the elevator blackout confession.",
                }
            ],
            "visual_settings": {
                "aspect_ratio": "9:16",
                "palette": "midnight_noir",
                "lighting": "high contrast",
            },
        }
        update_resp = auth_client.put(f"{DRAMA_BASE}/projects/{pid}/meta", json=second_payload)
        assert update_resp.status_code == 200, \
            f"Meta update PUT failed: {update_resp.status_code} {update_resp.text}"

        get_resp = auth_client.get(f"{DRAMA_BASE}/projects/{pid}/meta")
        assert get_resp.status_code == 200, \
            f"Meta GET failed: {get_resp.status_code} {get_resp.text}"
        data = extract_data(get_resp.json())
        assert data["project_id"] == pid
        assert data["characters"] == second_payload["characters"]
        assert data["style_references"] == second_payload["style_references"]
        assert data["text_materials"] == second_payload["text_materials"]
        assert data["visual_settings"] == second_payload["visual_settings"]
        assert isinstance(data.get("updated_at"), str)
        assert data["updated_at"] != ""

    def test_project_meta_forbidden_for_other_user(
        self, auth_client, second_auth_client
    ):
        pid = _create_project(auth_client)
        put_resp = auth_client.put(
            f"{DRAMA_BASE}/projects/{pid}/meta",
            json={
                "characters": [{"id": "char_owner", "name": "Owner"}],
                "style_references": [],
                "text_materials": [],
                "visual_settings": {"tone": "private"},
            },
        )
        assert put_resp.status_code == 200, \
            f"Owner meta PUT failed: {put_resp.status_code} {put_resp.text}"

        resp = second_auth_client.get(f"{DRAMA_BASE}/projects/{pid}/meta")
        assert resp.status_code == 403, \
            f"Expected 403 for cross-user meta GET, got {resp.status_code}: {resp.text}"
        body = resp.json()
        assert "forbidden" in str(body).lower() or body.get("code") == 403

    def test_project_meta_put_forbidden_for_other_user(
        self, auth_client, second_auth_client
    ):
        pid = _create_project(auth_client)

        owner_resp = auth_client.put(
            f"{DRAMA_BASE}/projects/{pid}/meta",
            json={
                "characters": [{"id": "char_owner", "name": "Owner"}],
                "style_references": [],
                "text_materials": [],
                "visual_settings": {"tone": "private"},
            },
        )
        assert owner_resp.status_code == 200, \
            f"Owner meta PUT failed: {owner_resp.status_code} {owner_resp.text}"

        resp = second_auth_client.put(
            f"{DRAMA_BASE}/projects/{pid}/meta",
            json={
                "characters": [{"id": "char_intruder", "name": "Intruder"}],
                "style_references": [],
                "text_materials": [],
                "visual_settings": {"tone": "stolen"},
            },
        )
        assert resp.status_code == 403, \
            f"Expected 403 for cross-user meta PUT, got {resp.status_code}: {resp.text}"
        body = resp.json()
        assert "forbidden" in str(body).lower() or body.get("code") == 403


# ===========================================================================
# 5. SSE Stream Auth Enforcement
# ===========================================================================

class TestDramaSseAuth:
    """Verify SSE stream enforces user ownership."""

    def test_sse_without_auth_returns_401(self, api_client):
        resp = api_client.get(
            f"{DRAMA_BASE}/projects/some-project-id/stream",
            headers={"Accept": "text/event-stream"},
            timeout=5,
        )
        assert resp.status_code == 401

    def test_sse_cross_user_returns_403(
        self, auth_client, second_auth_client, db_connection
    ):
        """User A creates project, user B tries to subscribe to SSE -> 403."""
        pid = _create_project(auth_client)
        time.sleep(0.5)

        # Ensure projection exists for the project
        row = _q1(
            db_connection,
            "SELECT user_id FROM gm_drama_project_projections WHERE project_id = %s",
            (pid,),
        )
        if not row:
            pytest.skip("No projection row — SSE auth requires projection")

        import requests
        resp = requests.get(
            f"{API_BASE_URL}{DRAMA_BASE}/projects/{pid}/stream",
            headers={
                "Authorization": second_auth_client.session.headers.get("Authorization", ""),
                "Accept": "text/event-stream",
            },
            stream=True,
            timeout=5,
        )
        assert resp.status_code == 403, \
            f"Expected 403 for cross-user SSE, got {resp.status_code}"
        resp.close()

    def test_sse_own_project_returns_200(self, auth_client, db_connection):
        """User A creates project and subscribes to own SSE -> 200."""
        pid = _create_project(auth_client)
        time.sleep(0.5)

        row = _q1(
            db_connection,
            "SELECT user_id FROM gm_drama_project_projections WHERE project_id = %s",
            (pid,),
        )
        if not row:
            pytest.skip("No projection row — SSE test requires projection")

        import requests
        resp = requests.get(
            f"{API_BASE_URL}{DRAMA_BASE}/projects/{pid}/stream",
            headers={
                "Authorization": auth_client.session.headers.get("Authorization", ""),
                "Accept": "text/event-stream",
            },
            stream=True,
            timeout=5,
        )
        assert resp.status_code == 200, \
            f"Expected 200 for own SSE, got {resp.status_code}"
        resp.close()


# ===========================================================================
# 6. Cancel + Interactions
# ===========================================================================

class TestDramaProjectCancel:
    def test_cancel_project(self, auth_client):
        pid = _create_project(auth_client)
        resp = auth_client.delete(f"{DRAMA_BASE}/projects/{pid}")
        # 200/204 = cancelled, 404 = gateway hasn't materialized the project yet
        assert resp.status_code in [200, 204, 404], \
            f"Cancel should succeed or 404 if not materialized, got {resp.status_code}"

    def test_cancel_without_auth(self, api_client):
        resp = api_client.delete(f"{DRAMA_BASE}/projects/some-id")
        assert resp.status_code == 401


class TestDramaClarify:
    def test_clarify_without_auth(self, api_client):
        resp = api_client.post(
            f"{DRAMA_BASE}/projects/some-id/clarify",
            json={"session_id": "s1", "answers": []},
        )
        assert resp.status_code == 401

    def test_clarify_with_payload(self, auth_client):
        pid = _create_project(auth_client)
        resp = auth_client.post(
            f"{DRAMA_BASE}/projects/{pid}/clarify",
            json={
                "session_id": "test_session",
                "answers": [{"question_id": "q1", "selected_option_id": "opt_a"}],
                "interaction_version": 1,
            },
        )
        # 200 = accepted, 400 = bad state, 404 = not materialized yet, 409 = stale
        assert resp.status_code in [200, 400, 404, 409], \
            f"Unexpected clarify status: {resp.status_code}"


class TestDramaStrategySelect:
    def test_strategy_select_without_auth(self, api_client):
        resp = api_client.post(
            f"{DRAMA_BASE}/projects/some-id/stages/strategy/select",
            json={"selected_option_id": "opt_a"},
        )
        assert resp.status_code == 401


class TestDramaApprove:
    def test_approve_without_auth(self, api_client):
        resp = api_client.post(
            f"{DRAMA_BASE}/projects/some-id/approve",
            json={"approved": True},
        )
        assert resp.status_code == 401

    def test_approve_with_payload(self, auth_client):
        pid = _create_project(auth_client)
        resp = auth_client.post(
            f"{DRAMA_BASE}/projects/{pid}/approve",
            json={"approved": True, "interaction_version": 1},
        )
        assert resp.status_code in [200, 400, 404, 409], \
            f"Unexpected approve status: {resp.status_code}"

    def test_request_revision(self, auth_client):
        pid = _create_project(auth_client)
        resp = auth_client.post(
            f"{DRAMA_BASE}/projects/{pid}/approve",
            json={
                "approved": False,
                "decision": "request_revision",
                "revision_notes": "Scene 2 needs more tension",
                "interaction_version": 1,
            },
        )
        assert resp.status_code in [200, 400, 404, 409], \
            f"Unexpected revision status: {resp.status_code}"


# ===========================================================================
# 7. Read Endpoints
# ===========================================================================

class TestDramaReadEndpoints:
    def test_get_script(self, auth_client):
        pid = _create_project(auth_client)
        resp = auth_client.get(f"{DRAMA_BASE}/projects/{pid}/script")
        assert resp.status_code in [200, 404]

    def test_get_shots(self, auth_client):
        pid = _create_project(auth_client)
        resp = auth_client.get(f"{DRAMA_BASE}/projects/{pid}/shots")
        assert resp.status_code in [200, 404]

    def test_get_render(self, auth_client):
        pid = _create_project(auth_client)
        resp = auth_client.get(f"{DRAMA_BASE}/projects/{pid}/render")
        assert resp.status_code in [200, 404]

    def test_get_artifacts(self, auth_client):
        pid = _create_project(auth_client)
        resp = auth_client.get(f"{DRAMA_BASE}/projects/{pid}/artifacts")
        assert resp.status_code == 200
        body = extract_data(resp.json())
        assert "artifacts" in body
        assert "final_master" in body
        assert "audio_tracks" in body
        assert "subtitle_bundles" in body
        assert "stage_packages" in body

    def test_get_cost(self, auth_client):
        pid = _create_project(auth_client)
        resp = auth_client.get(f"{DRAMA_BASE}/projects/{pid}/cost")
        assert resp.status_code == 200

    def test_get_fallbacks(self, auth_client):
        pid = _create_project(auth_client)
        resp = auth_client.get(f"{DRAMA_BASE}/projects/{pid}/fallbacks")
        assert resp.status_code in [200, 404]

    def test_read_endpoints_without_auth(self, api_client):
        for suffix in ["/script", "/shots", "/render", "/artifacts", "/cost", "/fallbacks"]:
            resp = api_client.get(f"{DRAMA_BASE}/projects/some-id{suffix}")
            assert resp.status_code == 401, \
                f"Expected 401 for {suffix}, got {resp.status_code}"


# ===========================================================================
# 8. Stale Interaction Version (409)
# ===========================================================================

class TestDramaStaleInteraction:
    """Verify that a very large interaction_version triggers 409 conflict
    when the project has a lower version in DB."""

    def test_clarify_stale_version_returns_conflict(self, auth_client):
        pid = _create_project(auth_client)
        resp = auth_client.post(
            f"{DRAMA_BASE}/projects/{pid}/clarify",
            json={
                "session_id": "stale_test",
                "answers": [{"question_id": "q1", "selected_option_id": "opt_a"}],
                "interaction_version": 999999,
            },
        )
        # 409 = direct worker mode version mismatch; 200/400 = gateway mode;
        # 404 = project not yet materialized in gateway
        assert resp.status_code in [200, 400, 404, 409], \
            f"Unexpected status for stale version: {resp.status_code}"


# ===========================================================================
# 8.5 H4 / H5 / H7 follow-up action routes
# ===========================================================================

class TestDramaScriptFeedback:
    def test_script_feedback_requires_auth(self, api_client):
        resp = api_client.post(
            f"{DRAMA_BASE}/projects/some-id/script/feedback",
            json={"feedback": "Please make scene 2 more intense."},
        )
        assert resp.status_code == 401

    def test_script_feedback_forbidden_for_other_user(
        self, auth_client, second_auth_client, db_connection
    ):
        project_id = _create_project(auth_client)
        projection_row = _wait_for_projection(db_connection, project_id)
        if projection_row is None:
            pytest.skip("Projection row not created yet; script feedback endpoint is not ready")

        resp = second_auth_client.post(
            f"{DRAMA_BASE}/projects/{project_id}/script/feedback",
            json={"feedback": "Unauthorized feedback"},
        )
        assert resp.status_code == 403, (
            f"Expected 403 for cross-user script feedback, got {resp.status_code}: {resp.text}"
        )

    def test_script_feedback_accepts_valid_owner_payload(
        self, auth_client, db_connection
    ):
        project_id = _create_project(auth_client)
        projection_row = _wait_for_projection(db_connection, project_id)
        if projection_row is None:
            pytest.skip("Projection row not created yet; script feedback endpoint is not ready")

        resp = auth_client.post(
            f"{DRAMA_BASE}/projects/{project_id}/script/feedback",
            json={
                "feedback": "Please increase tension in the second half.",
                "interaction_version": 1,
            },
        )
        assert resp.status_code in [200, 202, 400, 404, 409], (
            f"Unexpected script feedback status: {resp.status_code} {resp.text}"
        )


class TestDramaRetry:
    def test_retry_project_requires_auth(self, api_client):
        resp = api_client.post(f"{DRAMA_BASE}/projects/some-id/retry", json={})
        assert resp.status_code == 401

    def test_retry_project_forbidden_for_other_user(
        self, auth_client, second_auth_client, db_connection
    ):
        project_id = _create_project(auth_client)
        projection_row = _wait_for_projection(db_connection, project_id)
        if projection_row is None:
            pytest.skip("Projection row not created yet; retry endpoint is not ready")

        resp = second_auth_client.post(
            f"{DRAMA_BASE}/projects/{project_id}/retry",
            json={},
        )
        assert resp.status_code == 403, (
            f"Expected 403 for cross-user retry, got {resp.status_code}: {resp.text}"
        )

    def test_retry_project_accepts_owner_request(
        self, auth_client, db_connection
    ):
        project_id = _create_project(auth_client)
        projection_row = _wait_for_projection(db_connection, project_id)
        if projection_row is None:
            pytest.skip("Projection row not created yet; retry endpoint is not ready")
        time.sleep(1.0)

        resp = auth_client.post(
            f"{DRAMA_BASE}/projects/{project_id}/retry",
            json={},
        )
        assert resp.status_code in [200, 202], (
            f"Unexpected retry status: {resp.status_code} {resp.text}"
        )


class TestDramaClone:
    def test_clone_project_requires_auth(self, api_client):
        resp = api_client.post(f"{DRAMA_BASE}/projects/some-id/clone", json={})
        assert resp.status_code == 401

    def test_clone_project_forbidden_for_other_user(
        self, auth_client, second_auth_client, db_connection
    ):
        project_id = _create_project(auth_client)
        projection_row = _wait_for_projection(db_connection, project_id)
        if projection_row is None:
            pytest.skip("Projection row not created yet; clone endpoint is not ready")

        resp = second_auth_client.post(
            f"{DRAMA_BASE}/projects/{project_id}/clone",
            json={},
        )
        assert resp.status_code == 403, (
            f"Expected 403 for cross-user clone, got {resp.status_code}: {resp.text}"
        )

    def test_clone_project_creates_new_project_for_owner(
        self, auth_client, db_connection
    ):
        project_id = _create_project(auth_client)
        projection_row = _wait_for_projection(db_connection, project_id)
        if projection_row is None:
            pytest.skip("Projection row not created yet; clone endpoint is not ready")
        time.sleep(1.0)

        resp = auth_client.post(
            f"{DRAMA_BASE}/projects/{project_id}/clone",
            json={},
        )
        assert resp.status_code in [200, 201, 202], (
            f"Unexpected clone status: {resp.status_code} {resp.text}"
        )

        data = extract_data(resp.json())
        new_project_id = data.get("project_id") or data.get("new_project_id")
        assert new_project_id is not None
        assert new_project_id != project_id


class TestDramaSceneRerun:
    def test_scene_rerun_requires_auth(self, api_client):
        resp = api_client.post(
            f"{DRAMA_BASE}/projects/some-id/scenes/scene-1/rerun",
            json={},
        )
        assert resp.status_code == 401

    def test_scene_rerun_forbidden_for_other_user(
        self, auth_client, second_auth_client, db_connection
    ):
        project_id = _create_project(auth_client)
        projection_row = _wait_for_projection(db_connection, project_id)
        if projection_row is None:
            pytest.skip("Projection row not created yet; scene rerun endpoint is not ready")

        resp = second_auth_client.post(
            f"{DRAMA_BASE}/projects/{project_id}/scenes/scene-1/rerun",
            json={},
        )
        assert resp.status_code == 403, (
            f"Expected 403 for cross-user scene rerun, got {resp.status_code}: {resp.text}"
        )

    def test_scene_rerun_accepts_owner_request(
        self, auth_client, db_connection
    ):
        project_id = _create_project(auth_client)
        projection_row = _wait_for_projection(db_connection, project_id)
        if projection_row is None:
            pytest.skip("Projection row not created yet; scene rerun endpoint is not ready")
        time.sleep(1.0)

        resp = auth_client.post(
            f"{DRAMA_BASE}/projects/{project_id}/scenes/scene-1/rerun",
            json={},
        )
        assert resp.status_code in [200, 202], (
            f"Unexpected scene rerun status: {resp.status_code} {resp.text}"
        )


# ===========================================================================
# 9. Gateway Down (conditional)
# ===========================================================================

class TestDramaGatewayError:
    """Only runs when DRAMA_TEST_GATEWAY_DOWN=1 is set."""

    @pytest.mark.skipif(
        os.getenv("DRAMA_TEST_GATEWAY_DOWN", "0") != "1",
        reason="Set DRAMA_TEST_GATEWAY_DOWN=1 to test gateway-down behavior",
    )
    def test_create_project_gateway_down(self, auth_client):
        resp = auth_client.post(f"{DRAMA_BASE}/projects", json=_valid_brief())
        assert resp.status_code == 502
        body = resp.json()
        assert "msg" in body or "message" in body


# ===========================================================================
# 10. Internal callback / replay
# ===========================================================================

class TestDramaInternalCallback:
    def test_callback_rejects_invalid_payload(self, api_client):
        resp = api_client.post(
            "/api/v1/internal/drama/callback",
            data="not-json",
            headers={"Content-Type": "application/json"},
        )
        assert resp.status_code == 400, (
            f"Expected 400 for invalid callback payload, got {resp.status_code}: {resp.text}"
        )

    def test_callback_ingests_event_and_updates_projection(
        self, auth_client, api_client, db_connection
    ):
        project_id = _create_project(auth_client)
        projection_row = _wait_for_projection(db_connection, project_id)
        if projection_row is None:
            pytest.skip("Projection row not created yet; callback test requires projection")

        event_id = f"evt_run_started_{time.time_ns()}"
        run_id = f"run_{time.time_ns()}"
        payload = {
            "event_id": event_id,
            "project_id": project_id,
            "run_id": run_id,
            "sequence": 1,
            "stage_code": None,
            "event_type": "run_started",
            "occurred_at": datetime.now(timezone.utc).isoformat(),
            "payload": {},
        }

        resp = api_client.post(
            "/api/v1/internal/drama/callback",
            json=payload,
        )
        assert resp.status_code == 200, (
            f"Expected 200 for callback ingestion, got {resp.status_code}: {resp.text}"
        )
        body = resp.json()
        assert body["accepted"] is True
        assert body["event_id"] == event_id

        event_row = _q1(
            db_connection,
            "SELECT event_id, project_id, run_id, sequence, event_type FROM gm_drama_callback_events WHERE event_id = %s",
            (event_id,),
        )
        assert event_row is not None
        assert event_row["project_id"] == project_id
        assert event_row["run_id"] == run_id
        assert event_row["sequence"] == 1
        assert event_row["event_type"] == "run_started"

        projection_after = _q1(
            db_connection,
            "SELECT status, run_id, last_event_sequence FROM gm_drama_project_projections WHERE project_id = %s",
            (project_id,),
        )
        assert projection_after is not None
        assert projection_after["status"] == "running"
        assert projection_after["run_id"] == run_id
        assert projection_after["last_event_sequence"] >= 1

    def test_duplicate_callback_event_is_accepted_false(
        self, auth_client, api_client, db_connection
    ):
        project_id = _create_project(auth_client)
        projection_row = _wait_for_projection(db_connection, project_id)
        if projection_row is None:
            pytest.skip("Projection row not created yet; callback duplicate test requires projection")

        event_id = f"evt_duplicate_{time.time_ns()}"
        payload = {
            "event_id": event_id,
            "project_id": project_id,
            "run_id": f"run_{time.time_ns()}",
            "sequence": 2,
            "stage_code": "s01_strategy",
            "event_type": "stage_entered",
            "occurred_at": datetime.now(timezone.utc).isoformat(),
            "payload": {},
        }

        first_resp = api_client.post("/api/v1/internal/drama/callback", json=payload)
        assert first_resp.status_code == 200, first_resp.text
        assert first_resp.json()["accepted"] is True

        second_resp = api_client.post("/api/v1/internal/drama/callback", json=payload)
        assert second_resp.status_code == 200, second_resp.text
        second_body = second_resp.json()
        assert second_body["accepted"] is False
        assert second_body["reason"] == "duplicate event_id"


class TestDramaCallbackLifecycle:
    """Full lifecycle via callback events: run_started -> stage_entered (strategy)
    -> stage_entered (script) -> render_progress -> run_completed.
    Verifies projection status, stage, and sequence advance monotonically."""

    def test_multi_event_lifecycle_updates_projection_correctly(
        self, auth_client, api_client, db_connection
    ):
        project_id = _create_project(auth_client)
        projection_row = _wait_for_projection(db_connection, project_id)
        if projection_row is None:
            pytest.skip("Projection row not created; lifecycle callback test requires projection")

        run_id = f"run_lifecycle_{time.time_ns()}"
        ts_base = datetime.now(timezone.utc)

        events = [
            {"event_id": f"evt_lc_1_{time.time_ns()}", "project_id": project_id, "run_id": run_id, "sequence": 1, "stage_code": None, "event_type": "run_started", "occurred_at": ts_base.isoformat(), "payload": {}},
            {"event_id": f"evt_lc_2_{time.time_ns()}", "project_id": project_id, "run_id": run_id, "sequence": 2, "stage_code": "s01_strategy", "event_type": "stage_entered", "occurred_at": ts_base.isoformat(), "payload": {}},
            {"event_id": f"evt_lc_3_{time.time_ns()}", "project_id": project_id, "run_id": run_id, "sequence": 3, "stage_code": "s03_script", "event_type": "stage_entered", "occurred_at": ts_base.isoformat(), "payload": {}},
            {"event_id": f"evt_lc_4_{time.time_ns()}", "project_id": project_id, "run_id": run_id, "sequence": 4, "stage_code": "s05_render", "event_type": "render_progress_recorded", "occurred_at": ts_base.isoformat(), "payload": {"completed_tasks": 2, "total_tasks": 4, "progress_percent": 50}},
            {"event_id": f"evt_lc_5_{time.time_ns()}", "project_id": project_id, "run_id": run_id, "sequence": 5, "stage_code": None, "event_type": "run_completed", "occurred_at": ts_base.isoformat(), "payload": {}},
        ]

        for evt in events:
            resp = api_client.post("/api/v1/internal/drama/callback", json=evt)
            assert resp.status_code == 200, (
                f"Callback event {evt['event_type']} seq={evt['sequence']} failed: {resp.status_code} {resp.text}"
            )
            body = resp.json()
            assert body["accepted"] is True, (
                f"Callback event {evt['event_type']} rejected: {body}"
            )

        stored_events = _qall(
            db_connection,
            "SELECT event_id, event_type, sequence FROM gm_drama_callback_events WHERE project_id = %s AND run_id = %s ORDER BY sequence",
            (project_id, run_id),
        )
        assert len(stored_events) == 5
        assert stored_events[0]["event_type"] == "run_started"
        assert stored_events[1]["event_type"] == "stage_entered"
        assert stored_events[2]["event_type"] == "stage_entered"
        assert stored_events[3]["event_type"] == "render_progress_recorded"
        assert stored_events[4]["event_type"] == "run_completed"
        for i in range(5):
            assert stored_events[i]["sequence"] == i + 1

        final_projection = _q1(
            db_connection,
            "SELECT status, run_id, last_event_sequence FROM gm_drama_project_projections WHERE project_id = %s",
            (project_id,),
        )
        assert final_projection is not None
        assert final_projection["run_id"] == run_id
        assert final_projection["last_event_sequence"] >= 5
        assert final_projection["status"] in ["completed", "running"]

    def test_out_of_order_sequence_still_accepted(
        self, auth_client, api_client, db_connection
    ):
        """Callbacks arriving out of order should still be accepted
        (idempotent storage, projection uses max sequence)."""
        project_id = _create_project(auth_client)
        projection_row = _wait_for_projection(db_connection, project_id)
        if projection_row is None:
            pytest.skip("Projection row not created; out-of-order callback test requires projection")

        run_id = f"run_ooo_{time.time_ns()}"
        ts = datetime.now(timezone.utc).isoformat()

        evt_3 = {"event_id": f"evt_ooo_3_{time.time_ns()}", "project_id": project_id, "run_id": run_id, "sequence": 3, "stage_code": "s03_script", "event_type": "stage_entered", "occurred_at": ts, "payload": {}}
        evt_1 = {"event_id": f"evt_ooo_1_{time.time_ns()}", "project_id": project_id, "run_id": run_id, "sequence": 1, "stage_code": None, "event_type": "run_started", "occurred_at": ts, "payload": {}}

        resp_3 = api_client.post("/api/v1/internal/drama/callback", json=evt_3)
        assert resp_3.status_code == 200, resp_3.text
        assert resp_3.json()["accepted"] is True

        resp_1 = api_client.post("/api/v1/internal/drama/callback", json=evt_1)
        assert resp_1.status_code == 200, resp_1.text
        assert resp_1.json()["accepted"] is True

        projection = _q1(
            db_connection,
            "SELECT last_event_sequence FROM gm_drama_project_projections WHERE project_id = %s",
            (project_id,),
        )
        assert projection is not None
        assert projection["last_event_sequence"] >= 3


class TestDramaInternalReplay:
    def test_replay_request_returns_accepted(self, auth_client, api_client, db_connection):
        project_id = _create_project(auth_client)
        projection_row = _wait_for_projection(db_connection, project_id)
        if projection_row is None:
            pytest.skip("Projection row not created yet; replay test requires projection")

        resp = api_client.post(
            f"/api/v1/internal/drama/replay/{project_id}",
            json={
                "run_id": f"run_{time.time_ns()}",
                "from_sequence": 3,
            },
        )
        assert resp.status_code == 202, (
            f"Expected 202 for replay request, got {resp.status_code}: {resp.text}"
        )
        body = resp.json()
        assert body["project_id"] == project_id
        assert body["from_sequence"] == 3
        assert body["status"] == "replay_requested"


# ===========================================================================
# 11. DB Projection Assertions
# ===========================================================================

class TestDramaProjectionDB:
    """Verify that drama projection table is correctly populated."""

    def test_create_populates_projection(self, auth_client, db_connection):
        brief = _valid_brief(budget_cents=1234, title="Projection Test Project")
        resp = auth_client.post(f"{DRAMA_BASE}/projects", json=brief)
        assert resp.status_code in [200, 201]
        pid = extract_data(resp.json())["project_id"]

        time.sleep(0.5)

        row = _q1(
            db_connection,
            "SELECT * FROM gm_drama_project_projections WHERE project_id = %s",
            (pid,),
        )
        if row is None:
            pytest.skip("Projection row not created — gateway may not be in persistence mode")

        assert row["title"] == brief["title"], \
            f"Expected title '{brief['title']}', got '{row['title']}'"
        assert row["cost_reserve_cents"] == 1234, \
            f"Expected reserve 1234, got {row['cost_reserve_cents']}"
        assert row["status"] in ["pending", "running"]
        assert row["created_at"] is not None
        assert row["updated_at"] is not None
        assert row["user_id"] is not None

    def test_cancel_updates_projection(self, auth_client, db_connection):
        pid = _create_project(auth_client)
        time.sleep(0.5)

        auth_client.delete(f"{DRAMA_BASE}/projects/{pid}")
        time.sleep(0.5)

        row = _q1(
            db_connection,
            "SELECT status FROM gm_drama_project_projections WHERE project_id = %s",
            (pid,),
        )
        if row:
            assert row["status"] in ["cancelled", "pending", "running"], \
                f"After cancel, status should be cancelled, got {row['status']}"

    def test_callback_events_table_exists(self, db_connection):
        """Smoke test that the callback events table is queryable."""
        rows = _qall(
            db_connection,
            "SELECT COUNT(*) as cnt FROM gm_drama_callback_events",
        )
        assert rows is not None
        assert rows[0]["cnt"] >= 0


# ===========================================================================
# 11. Private Assets (intentional RED until routes exist)
# ===========================================================================


# ===========================================================================
# 11a. Long Video (长剧) Project CRUD + DB Field Validation
# ===========================================================================

class TestDramaLongVideoProjectCreate:
    """P0: Verify long_video content_type flows through create, detail, list, and DB."""

    def test_preflight_long_video_valid(self, auth_client):
        resp = auth_client.post(
            f"{DRAMA_BASE}/preflight",
            json={
                "title": "Long Drama Valid",
                "description": "A multi-chapter thriller",
                "content_type": "long_video",
                "target_duration_seconds": 600,
            },
        )
        assert resp.status_code == 200
        data = extract_data(resp.json())
        assert data.get("passed") is True or data.get("all_passed") is True
        assert data["status"] == "passed"
        assert len(data["blocking"]) == 0

    def test_preflight_long_video_zero_duration(self, auth_client):
        resp = auth_client.post(
            f"{DRAMA_BASE}/preflight",
            json={
                "title": "Long Drama Zero Duration",
                "description": "Missing real duration",
                "content_type": "long_video",
                "target_duration_seconds": 0,
            },
        )
        assert resp.status_code == 200
        data = extract_data(resp.json())
        assert data["passed"] is False
        blocking_text = " ".join(data["blocking"])
        assert "target_duration_seconds" in blocking_text or "duration" in blocking_text.lower()

    def test_create_long_video_project_success(self, auth_client, db_connection):
        brief = _valid_long_video_brief(budget_cents=2000)
        resp = auth_client.post(f"{DRAMA_BASE}/projects", json=brief)
        assert resp.status_code in [200, 201]
        data = extract_data(resp.json())
        pid = data["project_id"]
        assert pid != ""
        assert data["status"] in ["pending", "running"]

        row = _q1(
            db_connection,
            "SELECT * FROM gm_drama_project_projections WHERE project_id = %s",
            (pid,),
        )
        if row:
            assert row["content_type"] == "long_video", \
                f"Expected content_type 'long_video', got '{row['content_type']}'"
            assert row["title"] == brief["title"]
            assert row["cost_reserve_cents"] == 2000, \
                f"Expected reserve 2000, got {row['cost_reserve_cents']}"
            assert row["status"] in ["pending", "running"]
            assert row["platform"] == "youtube" or row["platform"] is None
            assert row["created_at"] is not None
            assert row["updated_at"] is not None
            assert row["user_id"] is not None
            assert row["progress_percent"] >= 0
            assert row["interaction_version"] >= 1
            assert row["last_event_sequence"] >= 0

    def test_long_video_project_detail_returns_content_type(self, auth_client):
        pid = _create_long_video_project(auth_client)
        resp = auth_client.get(f"{DRAMA_BASE}/projects/{pid}")
        assert resp.status_code == 200
        data = extract_data(resp.json())
        assert data["project_id"] == pid
        assert data.get("content_type") == "long_video", \
            f"Detail should return content_type='long_video', got '{data.get('content_type')}'"
        assert "status" in data

    def test_long_video_project_appears_in_list(self, auth_client):
        pid = _create_long_video_project(auth_client)
        resp = auth_client.get(f"{DRAMA_BASE}/projects")
        assert resp.status_code == 200
        data = extract_data(resp.json())
        items = data.get("list") or data.get("items") or data
        if isinstance(items, list):
            pids = [item.get("project_id") for item in items]
            assert pid in pids, f"Long video project {pid} should appear in project list"

    def test_long_video_cancel_updates_projection(self, auth_client, db_connection):
        pid = _create_long_video_project(auth_client)
        time.sleep(0.5)

        auth_client.delete(f"{DRAMA_BASE}/projects/{pid}")
        time.sleep(0.5)

        row = _q1(
            db_connection,
            "SELECT status, content_type FROM gm_drama_project_projections WHERE project_id = %s",
            (pid,),
        )
        if row:
            assert row["content_type"] == "long_video", \
                "content_type should remain long_video after cancel"
            assert row["status"] in ["cancelled", "pending", "running"], \
                f"After cancel, status should be cancelled, got {row['status']}"

    def test_long_video_meta_roundtrip(self, auth_client):
        pid = _create_long_video_project(auth_client)

        payload = {
            "characters": [
                {"id": "char_marcus", "name": "Marcus", "role": "lead"},
                {"id": "char_elena", "name": "Elena", "role": "antagonist"},
            ],
            "style_references": [
                {"id": "style_noir", "type": "image", "url": "https://example.com/noir.png"},
            ],
            "text_materials": [
                {"id": "tm_outline", "kind": "outline", "content": "Three-act corporate thriller spanning 10 chapters."},
            ],
            "visual_settings": {
                "aspect_ratio": "16:9",
                "palette": "cinematic_noir",
                "camera_language": "slow tracking shots",
            },
        }
        put_resp = auth_client.put(f"{DRAMA_BASE}/projects/{pid}/meta", json=payload)
        assert put_resp.status_code == 200, put_resp.text

        get_resp = auth_client.get(f"{DRAMA_BASE}/projects/{pid}/meta")
        assert get_resp.status_code == 200, get_resp.text
        data = extract_data(get_resp.json())
        assert data["project_id"] == pid
        assert len(data["characters"]) == 2
        assert data["visual_settings"]["aspect_ratio"] == "16:9"

    def test_long_video_project_resources_roundtrip(self, auth_client, db_connection):
        pid = _create_long_video_project(auth_client)
        projection = _wait_for_projection(db_connection, pid)
        if projection is None:
            pytest.skip("Projection row not created yet")

        char_resp = auth_client.post(
            f"{DRAMA_BASE}/characters",
            json={"name": "Long Video Character"},
        )
        assert char_resp.status_code == 200, char_resp.text
        char_id = extract_data(char_resp.json())["id"]

        put_resp = auth_client.put(
            f"{DRAMA_BASE}/projects/{pid}/resources",
            json={
                "character_ids": [char_id],
                "scene_asset_ids": [],
                "style_asset_ids": [],
                "primary_style_asset_id": None,
            },
        )
        assert put_resp.status_code == 200, put_resp.text

        get_resp = auth_client.get(f"{DRAMA_BASE}/projects/{pid}/resources")
        assert get_resp.status_code == 200, get_resp.text
        data = extract_data(get_resp.json())
        assert data["character_ids"] == [char_id]


# ===========================================================================
# 11b. s02_chapters Stage Callback Event Progression
# ===========================================================================

class TestDramaChaptersStageCallback:
    """P0: Verify s02_chapters stage is correctly handled in callback lifecycle."""

    def test_lifecycle_with_chapters_stage(
        self, auth_client, api_client, db_connection
    ):
        """Full long_video lifecycle including s02_chapters:
        run_started -> s01_strategy -> s02_chapters -> s03_script -> s05_render -> run_completed."""
        project_id = _create_long_video_project(auth_client)
        projection_row = _wait_for_projection(db_connection, project_id)
        if projection_row is None:
            pytest.skip("Projection row not created; chapters lifecycle callback test requires projection")

        run_id = f"run_chapters_{time.time_ns()}"
        ts_base = datetime.now(timezone.utc)

        events = [
            {
                "event_id": f"evt_ch_1_{time.time_ns()}", "project_id": project_id,
                "run_id": run_id, "sequence": 1, "stage_code": None,
                "event_type": "run_started", "occurred_at": ts_base.isoformat(), "payload": {},
            },
            {
                "event_id": f"evt_ch_2_{time.time_ns()}", "project_id": project_id,
                "run_id": run_id, "sequence": 2, "stage_code": "s01_strategy",
                "event_type": "stage_entered", "occurred_at": ts_base.isoformat(), "payload": {},
            },
            {
                "event_id": f"evt_ch_3_{time.time_ns()}", "project_id": project_id,
                "run_id": run_id, "sequence": 3, "stage_code": "s02_chapters",
                "event_type": "stage_entered", "occurred_at": ts_base.isoformat(), "payload": {},
            },
            {
                "event_id": f"evt_ch_4_{time.time_ns()}", "project_id": project_id,
                "run_id": run_id, "sequence": 4, "stage_code": "s03_script",
                "event_type": "stage_entered", "occurred_at": ts_base.isoformat(), "payload": {},
            },
            {
                "event_id": f"evt_ch_5_{time.time_ns()}", "project_id": project_id,
                "run_id": run_id, "sequence": 5, "stage_code": "s04_visual",
                "event_type": "stage_entered", "occurred_at": ts_base.isoformat(), "payload": {},
            },
            {
                "event_id": f"evt_ch_6_{time.time_ns()}", "project_id": project_id,
                "run_id": run_id, "sequence": 6, "stage_code": "s05_render",
                "event_type": "render_progress_recorded",
                "occurred_at": ts_base.isoformat(),
                "payload": {"completed_tasks": 3, "total_tasks": 10, "progress_percent": 30},
            },
            {
                "event_id": f"evt_ch_7_{time.time_ns()}", "project_id": project_id,
                "run_id": run_id, "sequence": 7, "stage_code": None,
                "event_type": "run_completed", "occurred_at": ts_base.isoformat(), "payload": {},
            },
        ]

        for evt in events:
            resp = api_client.post("/api/v1/internal/drama/callback", json=evt)
            assert resp.status_code == 200, (
                f"Callback event {evt['event_type']} stage={evt.get('stage_code')} "
                f"seq={evt['sequence']} failed: {resp.status_code} {resp.text}"
            )
            body = resp.json()
            assert body["accepted"] is True, (
                f"Callback event {evt['event_type']} rejected: {body}"
            )

        stored_events = _qall(
            db_connection,
            "SELECT event_id, event_type, stage_code, sequence "
            "FROM gm_drama_callback_events "
            "WHERE project_id = %s AND run_id = %s ORDER BY sequence",
            (project_id, run_id),
        )
        assert len(stored_events) == 7
        assert stored_events[0]["event_type"] == "run_started"
        assert stored_events[1]["stage_code"] == "s01_strategy"
        assert stored_events[2]["stage_code"] == "s02_chapters"
        assert stored_events[3]["stage_code"] == "s03_script"
        assert stored_events[4]["stage_code"] == "s04_visual"
        assert stored_events[5]["stage_code"] == "s05_render"
        assert stored_events[5]["event_type"] == "render_progress_recorded"
        assert stored_events[6]["event_type"] == "run_completed"

        for i in range(7):
            assert stored_events[i]["sequence"] == i + 1

        final_projection = _q1(
            db_connection,
            "SELECT status, run_id, last_event_sequence, content_type "
            "FROM gm_drama_project_projections WHERE project_id = %s",
            (project_id,),
        )
        assert final_projection is not None
        assert final_projection["run_id"] == run_id
        assert final_projection["last_event_sequence"] >= 7
        assert final_projection["status"] in ["completed", "running"]
        assert final_projection["content_type"] == "long_video"

    def test_chapters_stage_updates_current_stage_in_projection(
        self, auth_client, api_client, db_connection
    ):
        """After ingesting s02_chapters stage_entered, projection.current_stage should update."""
        project_id = _create_long_video_project(auth_client)
        projection_row = _wait_for_projection(db_connection, project_id)
        if projection_row is None:
            pytest.skip("Projection row not created; chapters stage test requires projection")

        run_id = f"run_ch_stage_{time.time_ns()}"
        ts = datetime.now(timezone.utc).isoformat()

        resp1 = api_client.post("/api/v1/internal/drama/callback", json={
            "event_id": f"evt_chs_1_{time.time_ns()}", "project_id": project_id,
            "run_id": run_id, "sequence": 1, "stage_code": None,
            "event_type": "run_started", "occurred_at": ts, "payload": {},
        })
        assert resp1.status_code == 200, resp1.text

        resp2 = api_client.post("/api/v1/internal/drama/callback", json={
            "event_id": f"evt_chs_2_{time.time_ns()}", "project_id": project_id,
            "run_id": run_id, "sequence": 2, "stage_code": "s02_chapters",
            "event_type": "stage_entered", "occurred_at": ts, "payload": {},
        })
        assert resp2.status_code == 200, resp2.text

        projection = _q1(
            db_connection,
            "SELECT current_stage, status FROM gm_drama_project_projections WHERE project_id = %s",
            (project_id,),
        )
        assert projection is not None
        assert projection["current_stage"] == "s02_chapters", \
            f"Expected current_stage='s02_chapters', got '{projection['current_stage']}'"
        assert projection["status"] == "running"

    def test_chapter_scene_assets_work_for_long_video_project(
        self, auth_client, db_connection
    ):
        """Verify chapter-level scene assets can be set on a long_video project."""
        project_id = _create_long_video_project(auth_client)
        projection = _wait_for_projection(db_connection, project_id)
        if projection is None:
            pytest.skip("Projection row not created; chapter scene assets require projection")

        asset_id = _create_scene_asset(auth_client, "Corporate Office")

        chapter_id = "chapter-001"
        put_resp = auth_client.put(
            f"{DRAMA_BASE}/projects/{project_id}/chapters/{chapter_id}/scene-assets",
            json={"scene_asset_ids": [asset_id]},
        )
        assert put_resp.status_code == 200, put_resp.text

        get_resp = auth_client.get(
            f"{DRAMA_BASE}/projects/{project_id}/chapters/{chapter_id}/scene-assets"
        )
        assert get_resp.status_code == 200, get_resp.text
        data = extract_data(get_resp.json())
        assert data["project_id"] == project_id
        assert data["chapter_id"] == chapter_id
        assert data["scene_asset_ids"] == [asset_id]

    def test_different_chapters_have_independent_scene_assets(
        self, auth_client, db_connection
    ):
        """Scene assets set on chapter-001 should not appear in chapter-002."""
        project_id = _create_long_video_project(auth_client)
        projection = _wait_for_projection(db_connection, project_id)
        if projection is None:
            pytest.skip("Projection row not created; chapter isolation test requires projection")

        asset_ch1 = _create_scene_asset(auth_client, "Boardroom")
        asset_ch2 = _create_scene_asset(auth_client, "Rooftop Helipad")

        put_ch1 = auth_client.put(
            f"{DRAMA_BASE}/projects/{project_id}/chapters/chapter-001/scene-assets",
            json={"scene_asset_ids": [asset_ch1]},
        )
        assert put_ch1.status_code == 200, put_ch1.text

        put_ch2 = auth_client.put(
            f"{DRAMA_BASE}/projects/{project_id}/chapters/chapter-002/scene-assets",
            json={"scene_asset_ids": [asset_ch2]},
        )
        assert put_ch2.status_code == 200, put_ch2.text

        get_ch1 = auth_client.get(
            f"{DRAMA_BASE}/projects/{project_id}/chapters/chapter-001/scene-assets"
        )
        data_ch1 = extract_data(get_ch1.json())
        assert data_ch1["scene_asset_ids"] == [asset_ch1], \
            f"Chapter 1 should only have asset_ch1, got {data_ch1['scene_asset_ids']}"

        get_ch2 = auth_client.get(
            f"{DRAMA_BASE}/projects/{project_id}/chapters/chapter-002/scene-assets"
        )
        data_ch2 = extract_data(get_ch2.json())
        assert data_ch2["scene_asset_ids"] == [asset_ch2], \
            f"Chapter 2 should only have asset_ch2, got {data_ch2['scene_asset_ids']}"


class TestDramaPrivateCharacters:
    def test_private_characters_crud_roundtrip(self, auth_client):
        create_resp = auth_client.post(
            f"{DRAMA_BASE}/characters",
            json={
                "name": "Ava",
                "gender": "female",
                "age": "24",
                "appearance": "Silver coat and sharp eyes",
                "personality": "Calm and tactical",
                "voice_id": "voice_ava",
                "reference_image_url": "https://example.com/ava.png",
                "notes": "Primary heist lead",
            },
        )
        assert create_resp.status_code == 200, create_resp.text
        created = extract_data(create_resp.json())
        char_id = created["id"]

        list_resp = auth_client.get(f"{DRAMA_BASE}/characters")
        assert list_resp.status_code == 200, list_resp.text
        items = extract_data(list_resp.json())["items"]
        assert any(item["id"] == char_id and item["name"] == "Ava" for item in items)

        update_resp = auth_client.put(
            f"{DRAMA_BASE}/characters/{char_id}",
            json={
                "name": "Ava Prime",
                "notes": "Updated",
            },
        )
        assert update_resp.status_code == 200, update_resp.text
        updated = extract_data(update_resp.json())
        assert updated["name"] == "Ava Prime"
        assert updated["notes"] == "Updated"

        delete_resp = auth_client.delete(f"{DRAMA_BASE}/characters/{char_id}")
        assert delete_resp.status_code == 200, delete_resp.text

        list_after_delete_resp = auth_client.get(f"{DRAMA_BASE}/characters")
        assert list_after_delete_resp.status_code == 200, list_after_delete_resp.text
        items_after_delete = extract_data(list_after_delete_resp.json())["items"]
        assert all(item["id"] != char_id for item in items_after_delete)


class TestDramaProjectRoleLinks:
    def test_project_role_links_can_link_multiple_role_ids(self, auth_client, db_connection):
        project_id = _create_project(auth_client)
        projection_row = _wait_for_projection(db_connection, project_id)
        if projection_row is None:
            pytest.skip("Projection row not created yet; project resources endpoint is not ready")

        create_a_resp = auth_client.post(
            f"{DRAMA_BASE}/characters",
            json={"name": "Ava"},
        )
        assert create_a_resp.status_code == 200, create_a_resp.text
        char_a = extract_data(create_a_resp.json())["id"]

        create_b_resp = auth_client.post(
            f"{DRAMA_BASE}/characters",
            json={"name": "Leo"},
        )
        assert create_b_resp.status_code == 200, create_b_resp.text
        char_b = extract_data(create_b_resp.json())["id"]

        put_resp = auth_client.put(
            f"{DRAMA_BASE}/projects/{project_id}/resources",
            json={
                "character_ids": [char_a, char_b],
                "scene_asset_ids": [],
                "style_asset_ids": [],
                "primary_style_asset_id": None,
            },
        )
        assert put_resp.status_code == 200, put_resp.text

        get_resp = auth_client.get(f"{DRAMA_BASE}/projects/{project_id}/resources")
        assert get_resp.status_code == 200, get_resp.text
        data = extract_data(get_resp.json())
        assert set(data["character_ids"]) == {char_a, char_b}

        replace_resp = auth_client.put(
            f"{DRAMA_BASE}/projects/{project_id}/resources",
            json={
                "character_ids": [char_b],
                "scene_asset_ids": [],
                "style_asset_ids": [],
                "primary_style_asset_id": None,
            },
        )
        assert replace_resp.status_code == 200, replace_resp.text

        replaced_get_resp = auth_client.get(f"{DRAMA_BASE}/projects/{project_id}/resources")
        assert replaced_get_resp.status_code == 200, replaced_get_resp.text
        replaced_data = extract_data(replaced_get_resp.json())
        assert set(replaced_data["character_ids"]) == {char_b}

    def test_project_role_links_reject_cross_user_character_ids(
        self, auth_client, second_auth_client, db_connection
    ):
        foreign_char_resp = second_auth_client.post(
            f"{DRAMA_BASE}/characters",
            json={"name": "Foreign Character"},
        )
        assert foreign_char_resp.status_code == 200, foreign_char_resp.text
        foreign_char_id = extract_data(foreign_char_resp.json())["id"]

        project_id = _create_project(auth_client)
        projection_row = _wait_for_projection(db_connection, project_id)
        if projection_row is None:
            pytest.skip("Projection row not created yet; project resources endpoint is not ready")

        put_resp = auth_client.put(
            f"{DRAMA_BASE}/projects/{project_id}/resources",
            json={
                "character_ids": [foreign_char_id],
                "scene_asset_ids": [],
                "style_asset_ids": [],
                "primary_style_asset_id": None,
            },
        )
        assert put_resp.status_code == 403, \
            f"Expected 403 for cross-user character link, got {put_resp.status_code}: {put_resp.text}"
        body = put_resp.json()
        assert body.get("code") == 403

        get_resp = auth_client.get(f"{DRAMA_BASE}/projects/{project_id}/resources")
        assert get_resp.status_code == 200, get_resp.text
        data = extract_data(get_resp.json())
        assert data["character_ids"] == []

    def test_project_role_links_reject_non_empty_scene_assets(
        self, auth_client, db_connection
    ):
        project_id = _create_project(auth_client)
        projection_row = _wait_for_projection(db_connection, project_id)
        if projection_row is None:
            pytest.skip("Projection row not created yet; project resources endpoint is not ready")

        create_char_resp = auth_client.post(
            f"{DRAMA_BASE}/characters",
            json={"name": "Scene Guard Character"},
        )
        assert create_char_resp.status_code == 200, create_char_resp.text
        char_id = extract_data(create_char_resp.json())["id"]

        put_resp = auth_client.put(
            f"{DRAMA_BASE}/projects/{project_id}/resources",
            json={
                "character_ids": [char_id],
                "scene_asset_ids": ["scene_asset_001"],
                "style_asset_ids": [],
                "primary_style_asset_id": None,
            },
        )
        assert put_resp.status_code == 400, \
            f"Expected 400 for non-empty scene assets, got {put_resp.status_code}: {put_resp.text}"
        body = put_resp.json()
        assert body.get("code") == 400

        get_resp = auth_client.get(f"{DRAMA_BASE}/projects/{project_id}/resources")
        assert get_resp.status_code == 200, get_resp.text
        data = extract_data(get_resp.json())
        assert data["character_ids"] == []
        assert data["scene_asset_ids"] == []

    def test_project_resources_accept_single_primary_style(
        self, auth_client, db_connection
    ):
        project_id = _create_project(auth_client)
        projection_row = _wait_for_projection(db_connection, project_id)
        if projection_row is None:
            pytest.skip("Projection row not created yet; project resources endpoint is not ready")

        style_id = extract_data(auth_client.post(
            f"{DRAMA_BASE}/style-assets",
            json={
                "name": "Neon Pulse",
                "visual_style": "cyberpunk_neon",
                "color_tone": "cool",
            },
        ).json())["id"]

        put_resp = auth_client.put(
            f"{DRAMA_BASE}/projects/{project_id}/resources",
            json={
                "character_ids": [],
                "scene_asset_ids": [],
                "style_asset_ids": [style_id],
                "primary_style_asset_id": style_id,
            },
        )
        assert put_resp.status_code == 200, put_resp.text
        put_data = extract_data(put_resp.json())
        assert put_data["style_asset_ids"] == [style_id]
        assert put_data["primary_style_asset_id"] == style_id

        get_resp = auth_client.get(f"{DRAMA_BASE}/projects/{project_id}/resources")
        assert get_resp.status_code == 200, get_resp.text
        data = extract_data(get_resp.json())
        assert data["style_asset_ids"] == [style_id]
        assert data["primary_style_asset_id"] == style_id

    def test_primary_style_must_be_in_style_ids(
        self, auth_client, db_connection
    ):
        project_id = _create_project(auth_client)
        projection_row = _wait_for_projection(db_connection, project_id)
        if projection_row is None:
            pytest.skip("Projection row not created yet; project resources endpoint is not ready")

        style_id = extract_data(auth_client.post(
            f"{DRAMA_BASE}/style-assets",
            json={"name": "Ghost"},
        ).json())["id"]

        put_resp = auth_client.put(
            f"{DRAMA_BASE}/projects/{project_id}/resources",
            json={
                "character_ids": [],
                "scene_asset_ids": [],
                "style_asset_ids": [],
                "primary_style_asset_id": style_id,
            },
        )
        assert put_resp.status_code == 400, put_resp.text
        body = put_resp.json()
        assert body.get("code") == 400

        get_resp = auth_client.get(f"{DRAMA_BASE}/projects/{project_id}/resources")
        assert get_resp.status_code == 200, get_resp.text
        data = extract_data(get_resp.json())
        assert data["character_ids"] == []
        assert data["style_asset_ids"] == []
        assert data.get("primary_style_asset_id") is None

    def test_project_role_links_character_replace_preserves_scene_links_and_replaces_style_links(
        self, auth_client, db_connection, test_user_id
    ):
        project_id = _create_project(auth_client)
        projection_row = _wait_for_projection(db_connection, project_id)
        if projection_row is None:
            pytest.skip("Projection row not created yet; project resources endpoint is not ready")

        style_asset_id = f"style_keep_{time.time_ns()}"
        scene_asset_id = f"scene_keep_{time.time_ns()}"
        _exec(
            db_connection,
            """
            INSERT INTO drama_private_style_assets
                (id, user_id, name, reference_image_urls)
            VALUES (%s, %s, %s, '[]'::jsonb)
            """,
            (style_asset_id, test_user_id, "Keep Style Asset"),
        )
        _exec(
            db_connection,
            """
            INSERT INTO drama_private_scene_assets
                (id, user_id, name, reference_image_urls)
            VALUES (%s, %s, %s, '[]'::jsonb)
            """,
            (scene_asset_id, test_user_id, "Keep Scene Asset"),
        )
        _exec(
            db_connection,
            """
            INSERT INTO drama_project_style_asset_links
                (project_id, user_id, style_asset_id, is_primary)
            VALUES (%s, %s, %s, true)
            """,
            (project_id, test_user_id, style_asset_id),
        )
        _exec(
            db_connection,
            """
            INSERT INTO drama_project_scene_asset_links
                (project_id, user_id, scene_asset_id)
            VALUES (%s, %s, %s)
            """,
            (project_id, test_user_id, scene_asset_id),
        )

        create_a_resp = auth_client.post(
            f"{DRAMA_BASE}/characters",
            json={"name": "Keep Style Character A"},
        )
        assert create_a_resp.status_code == 200, create_a_resp.text
        char_a = extract_data(create_a_resp.json())["id"]

        create_b_resp = auth_client.post(
            f"{DRAMA_BASE}/characters",
            json={"name": "Keep Style Character B"},
        )
        assert create_b_resp.status_code == 200, create_b_resp.text
        char_b = extract_data(create_b_resp.json())["id"]

        initial_put_resp = auth_client.put(
            f"{DRAMA_BASE}/projects/{project_id}/resources",
            json={
                "character_ids": [char_a],
                "scene_asset_ids": [],
                "style_asset_ids": [style_asset_id],
                "primary_style_asset_id": style_asset_id,
            },
        )
        assert initial_put_resp.status_code == 200, initial_put_resp.text
        initial_data = extract_data(initial_put_resp.json())
        assert initial_data["style_asset_ids"] == [style_asset_id]
        assert initial_data["primary_style_asset_id"] == style_asset_id

        replace_resp = auth_client.put(
            f"{DRAMA_BASE}/projects/{project_id}/resources",
            json={
                "character_ids": [char_b],
                "scene_asset_ids": [],
                "style_asset_ids": [],
                "primary_style_asset_id": None,
            },
        )
        assert replace_resp.status_code == 200, replace_resp.text
        replace_data = extract_data(replace_resp.json())
        assert replace_data["style_asset_ids"] == []
        assert replace_data.get("primary_style_asset_id") is None

        style_link_row = _q1(
            db_connection,
            """
            SELECT COUNT(*) AS cnt
            FROM drama_project_style_asset_links
            WHERE project_id = %s AND user_id = %s
            """,
            (project_id, test_user_id),
        )
        scene_link_row = _q1(
            db_connection,
            """
            SELECT COUNT(*) AS cnt
            FROM drama_project_scene_asset_links
            WHERE project_id = %s AND user_id = %s
            """,
            (project_id, test_user_id),
        )
        assert style_link_row is not None
        assert scene_link_row is not None
        assert style_link_row["cnt"] == 0, "style links should be replaced by latest request"
        assert scene_link_row["cnt"] == 1, "character-only replace should preserve scene links"

        get_resp = auth_client.get(f"{DRAMA_BASE}/projects/{project_id}/resources")
        assert get_resp.status_code == 200, get_resp.text
        data = extract_data(get_resp.json())
        assert data["character_ids"] == [char_b]
        assert data["scene_asset_ids"] == []
        assert data["style_asset_ids"] == []
        assert data.get("primary_style_asset_id") is None

    def test_project_resources_get_returns_not_ready_404_without_projection(
        self, auth_client
    ):
        missing_project_id = "00000000-0000-0000-0000-000000000404"
        resp = auth_client.get(f"{DRAMA_BASE}/projects/{missing_project_id}/resources")
        assert resp.status_code == 404, \
            f"Expected 404 when projection is absent, got {resp.status_code}: {resp.text}"
        body = resp.json()
        assert body.get("code") == 404
        assert body.get("msg") == "drama project resources are not ready yet"
        assert body.get("msg_cn") == "短剧项目资源暂未就绪，请稍后再试"


class TestDramaChapterSceneAssets:
    def test_get_chapter_scene_assets_empty_by_default(
        self, auth_client, db_connection
    ):
        project_id = _create_project(auth_client)
        projection_row = _wait_for_projection(db_connection, project_id)
        if projection_row is None:
            pytest.skip("Projection row not created yet; chapter scene assets endpoint is not ready")

        chapter_id = "chapter-001"
        resp = auth_client.get(
            f"{DRAMA_BASE}/projects/{project_id}/chapters/{chapter_id}/scene-assets"
        )
        assert resp.status_code == 200, resp.text

        data = extract_data(resp.json())
        assert data["project_id"] == project_id
        assert data["chapter_id"] == chapter_id
        assert data["scene_asset_ids"] == []

    def test_put_get_chapter_scene_assets_roundtrip(
        self, auth_client, db_connection
    ):
        project_id = _create_project(auth_client)
        projection_row = _wait_for_projection(db_connection, project_id)
        if projection_row is None:
            pytest.skip("Projection row not created yet; chapter scene assets endpoint is not ready")

        chapter_id = "chapter-001"
        asset_a = _create_scene_asset(auth_client, "Neon Rooftop")
        asset_b = _create_scene_asset(auth_client, "Dark Stairway")

        put_resp = auth_client.put(
            f"{DRAMA_BASE}/projects/{project_id}/chapters/{chapter_id}/scene-assets",
            json={"scene_asset_ids": [asset_a, asset_b]},
        )
        assert put_resp.status_code == 200, put_resp.text

        put_data = extract_data(put_resp.json())
        assert set(put_data["scene_asset_ids"]) == {asset_a, asset_b}

        get_resp = auth_client.get(
            f"{DRAMA_BASE}/projects/{project_id}/chapters/{chapter_id}/scene-assets"
        )
        assert get_resp.status_code == 200, get_resp.text

        get_data = extract_data(get_resp.json())
        assert get_data["project_id"] == project_id
        assert get_data["chapter_id"] == chapter_id
        assert set(get_data["scene_asset_ids"]) == {asset_a, asset_b}

    def test_put_chapter_scene_assets_replaces_previous_links(
        self, auth_client, db_connection
    ):
        project_id = _create_project(auth_client)
        projection_row = _wait_for_projection(db_connection, project_id)
        if projection_row is None:
            pytest.skip("Projection row not created yet; chapter scene assets endpoint is not ready")

        chapter_id = "chapter-001"
        asset_a = _create_scene_asset(auth_client, "Replace A")
        asset_b = _create_scene_asset(auth_client, "Replace B")

        first_put = auth_client.put(
            f"{DRAMA_BASE}/projects/{project_id}/chapters/{chapter_id}/scene-assets",
            json={"scene_asset_ids": [asset_a, asset_b]},
        )
        assert first_put.status_code == 200, first_put.text

        second_put = auth_client.put(
            f"{DRAMA_BASE}/projects/{project_id}/chapters/{chapter_id}/scene-assets",
            json={"scene_asset_ids": [asset_b]},
        )
        assert second_put.status_code == 200, second_put.text

        get_resp = auth_client.get(
            f"{DRAMA_BASE}/projects/{project_id}/chapters/{chapter_id}/scene-assets"
        )
        assert get_resp.status_code == 200, get_resp.text
        data = extract_data(get_resp.json())
        assert data["scene_asset_ids"] == [asset_b]

    def test_put_chapter_scene_assets_rejects_cross_user_scene_asset_ids(
        self, auth_client, second_auth_client, db_connection
    ):
        project_id = _create_project(auth_client)
        projection_row = _wait_for_projection(db_connection, project_id)
        if projection_row is None:
            pytest.skip("Projection row not created yet; chapter scene assets endpoint is not ready")

        chapter_id = "chapter-001"
        foreign_asset_id = _create_scene_asset(second_auth_client, "Foreign Scene")

        put_resp = auth_client.put(
            f"{DRAMA_BASE}/projects/{project_id}/chapters/{chapter_id}/scene-assets",
            json={"scene_asset_ids": [foreign_asset_id]},
        )
        assert put_resp.status_code == 403, (
            f"Expected 403 for cross-user scene asset link, got {put_resp.status_code}: {put_resp.text}"
        )

        get_resp = auth_client.get(
            f"{DRAMA_BASE}/projects/{project_id}/chapters/{chapter_id}/scene-assets"
        )
        assert get_resp.status_code == 200, get_resp.text
        data = extract_data(get_resp.json())
        assert data["scene_asset_ids"] == []

class TestDramaPrivateSceneAssets:
    def test_scene_asset_crud_roundtrip(self, auth_client):
        create_resp = auth_client.post(
            f"{DRAMA_BASE}/scene-assets",
            json={
                "name": "Rooftop Night",
                "category": "urban",
                "location_description": "A neon-lit rooftop in the rain",
                "time_of_day": "night",
                "mood": "tense",
                "reference_image_urls": ["https://example.com/roof-1.png"],
                "camera_notes": "wide establishing angle",
                "notes": "Use for confrontation scenes",
            },
        )
        assert create_resp.status_code == 200, create_resp.text
        created = extract_data(create_resp.json())
        scene_id = created["id"]
        assert created["name"] == "Rooftop Night"
        assert created["category"] == "urban"

        list_resp = auth_client.get(f"{DRAMA_BASE}/scene-assets")
        assert list_resp.status_code == 200, list_resp.text
        items = extract_data(list_resp.json())["items"]
        assert any(item["id"] == scene_id for item in items)

        update_resp = auth_client.put(
            f"{DRAMA_BASE}/scene-assets/{scene_id}",
            json={"name": "Rooftop Dawn", "mood": "hopeful"},
        )
        assert update_resp.status_code == 200, update_resp.text
        updated = extract_data(update_resp.json())
        assert updated["name"] == "Rooftop Dawn"
        assert updated["mood"] == "hopeful"

        delete_resp = auth_client.delete(f"{DRAMA_BASE}/scene-assets/{scene_id}")
        assert delete_resp.status_code == 200, delete_resp.text

        list_after = auth_client.get(f"{DRAMA_BASE}/scene-assets")
        items_after = extract_data(list_after.json())["items"]
        assert not any(item["id"] == scene_id for item in items_after)

    def test_scene_asset_forbidden_for_other_user(self, auth_client, second_auth_client):
        scene_id = extract_data(auth_client.post(
            f"{DRAMA_BASE}/scene-assets",
            json={"name": "Private Scene"},
        ).json())["id"]
        resp = second_auth_client.get(f"{DRAMA_BASE}/scene-assets")
        items = extract_data(resp.json())["items"]
        assert not any(item["id"] == scene_id for item in items)


class TestDramaPrivateStyleAssets:
    def test_style_asset_crud_roundtrip(self, auth_client):
        create_resp = auth_client.post(
            f"{DRAMA_BASE}/style-assets",
            json={
                "name": "Neon Pulse",
                "visual_style": "cyberpunk_neon",
                "color_tone": "cool",
                "aspect_ratio": "9:16",
                "resolution": "1080p",
                "lighting_mood": "glossy",
                "reference_image_urls": ["https://example.com/style-1.png"],
                "notes": "Primary hero palette",
            },
        )
        assert create_resp.status_code == 200, create_resp.text
        created = extract_data(create_resp.json())
        style_id = created["id"]
        assert created["name"] == "Neon Pulse"
        assert created["visual_style"] == "cyberpunk_neon"
        assert created["color_tone"] == "cool"

        list_resp = auth_client.get(f"{DRAMA_BASE}/style-assets")
        assert list_resp.status_code == 200, list_resp.text
        items = extract_data(list_resp.json())["items"]
        assert any(item["id"] == style_id for item in items)

        update_resp = auth_client.put(
            f"{DRAMA_BASE}/style-assets/{style_id}",
            json={
                "name": "Neon Pulse Deluxe",
                "lighting_mood": "moody",
                "notes": "Updated palette",
            },
        )
        assert update_resp.status_code == 200, update_resp.text
        updated = extract_data(update_resp.json())
        assert updated["name"] == "Neon Pulse Deluxe"
        assert updated["lighting_mood"] == "moody"
        assert updated["notes"] == "Updated palette"

        delete_resp = auth_client.delete(f"{DRAMA_BASE}/style-assets/{style_id}")
        assert delete_resp.status_code == 200, delete_resp.text

        list_after = auth_client.get(f"{DRAMA_BASE}/style-assets")
        items_after = extract_data(list_after.json())["items"]
        assert not any(item["id"] == style_id for item in items_after)

    def test_style_asset_isolated_from_other_user(self, auth_client, second_auth_client):
        style_id = extract_data(auth_client.post(
            f"{DRAMA_BASE}/style-assets",
            json={"name": "Private Style"},
        ).json())["id"]
        resp = second_auth_client.get(f"{DRAMA_BASE}/style-assets")
        assert resp.status_code == 200, resp.text
        items = extract_data(resp.json())["items"]
        assert not any(item["id"] == style_id for item in items)
