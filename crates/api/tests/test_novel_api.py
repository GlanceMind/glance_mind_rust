#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
GlanceMind API E2E Tests - AI Novel API
=======================================
Real integration tests for the Rust novel control-plane at /api/v1/novel/*.
These tests require the Rust API and Postgres to be running.
"""

import time
import os
import sys
import subprocess
import psycopg2
import psycopg2.extras
import pytest
import redis

from conftest import API_BASE_URL, APIClient, extract_data, resolve_test_user_id

NOVEL_BASE = "/api/v1/novel"
BCRYPT_HASH_TEST_PASSWORD = (
    "$2b$12$Ikc.R4FMMGahbGhfHlLl4.PciMiV37qXfHpPNCjGGQg/yOEgk7k/e"
)
SECOND_USER_EMAIL = "novel_iso_test@glancemind.test"
E2E_REDIS_URL = "redis://localhost:6381/0"


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


def _valid_create_body(**overrides):
    body = {
        "title": f"Novel Test {int(time.time())}",
        "topic": "A city survives through memory markets.",
        "genre": "科幻",
        "description": "A production-grade novel test project.",
        "num_chapters": 4,
        "target_words_per_chapter": 1200,
        "default_user_guidance": "Keep the tone tense and reflective.",
        "config_snapshot": {
            "proxy_setting": {},
            "webdav_config": {},
            "other_params": {},
        },
    }
    body.update(overrides)
    return body


def _create_project(client):
    resp = client.post(f"{NOVEL_BASE}/projects", json=_valid_create_body())
    assert resp.status_code == 201, f"create failed: {resp.status_code} {resp.text}"
    return extract_data(resp.json())["project_id"]


def _current_snapshot_id(client, project_id: str) -> int:
    resp = client.get(f"{NOVEL_BASE}/projects/{project_id}")
    assert resp.status_code == 200, f"detail failed: {resp.status_code} {resp.text}"
    data = extract_data(resp.json())
    return data["current_config_snapshot"]["id"]


def _create_llm_profile(client, name_suffix: str) -> int:
    resp = client.post(
        f"{NOVEL_BASE}/llm-profiles",
        json={
            "name": f"novel-e2e-{name_suffix}-{int(time.time())}",
            "interface_format": "openai",
            "base_url": "http://localhost:8085/mock-llm/v1",
            "api_key": "mock-key",
            "model_name": "gpt-4o-mini",
            "temperature": 0.7,
            "max_tokens": 2048,
            "timeout_seconds": 120,
        },
    )
    assert resp.status_code == 201, f"create llm profile failed: {resp.status_code} {resp.text}"
    return extract_data(resp.json())["id"]


def _create_snapshot_with_llm(client, project_id: str, llm_profile_id: int) -> int:
    resp = client.post(
        f"{NOVEL_BASE}/projects/{project_id}/config-snapshots",
        json={
            "architecture_llm_profile_id": llm_profile_id,
            "chapter_outline_llm_profile_id": llm_profile_id,
            "prompt_draft_llm_profile_id": llm_profile_id,
            "final_chapter_llm_profile_id": llm_profile_id,
            "consistency_review_llm_profile_id": llm_profile_id,
            "proxy_setting": {},
            "webdav_config": {},
            "other_params": {},
        },
    )
    assert resp.status_code == 201, f"create snapshot failed: {resp.status_code} {resp.text}"
    return extract_data(resp.json())["id"]


def _redis_client():
    return redis.Redis.from_url(E2E_REDIS_URL, decode_responses=True)


def _clear_novel_queue():
    client = _redis_client()
    client.delete("novel_worker_tasks")


def _peek_novel_queue():
    client = _redis_client()
    return client.lrange("novel_worker_tasks", 0, -1)


@pytest.fixture(scope="module")
def test_user_id(db_connection):
    return resolve_test_user_id(db_connection)


@pytest.fixture(scope="module")
def second_auth_client(db_connection):
    """Create and authenticate a second user for ownership tests."""
    _ensure_clean(db_connection)
    cur = db_connection.cursor(cursor_factory=psycopg2.extras.RealDictCursor)

    cur.execute("SELECT id FROM gm_users WHERE email = %s", (SECOND_USER_EMAIL,))
    existing = cur.fetchone()
    if existing is None:
        cur.execute(
            """INSERT INTO gm_users
               (email, username, password_hash, full_name, role, status, is_active, created_at)
               VALUES (%s, 'novel_iso', %s, 'Novel Isolation User', 'user', 'ACTIVE', true, NOW())""",
            (SECOND_USER_EMAIL, BCRYPT_HASH_TEST_PASSWORD),
        )

    cur.execute("SELECT id FROM gm_email_verifications WHERE email = %s", (SECOND_USER_EMAIL,))
    ev_row = cur.fetchone()
    if ev_row is None:
        cur.execute(
            """INSERT INTO gm_email_verifications (email, code, verified, expires_at, created_at)
               VALUES (%s, 'NISO01', true, NOW() + INTERVAL '1 year', NOW())""",
            (SECOND_USER_EMAIL,),
        )
    else:
        cur.execute(
            "UPDATE gm_email_verifications SET verified = true, expires_at = NOW() + INTERVAL '1 year' WHERE email = %s",
            (SECOND_USER_EMAIL,),
        )

    cur.execute("UPDATE gm_users SET permissions = 15 WHERE email = %s", (SECOND_USER_EMAIL,))
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
    assert token
    return APIClient(API_BASE_URL, token=token)


@pytest.fixture(scope="module", autouse=True)
def suspend_container_novel_worker():
    """Suspend the compose novel worker during this module to avoid queue races."""
    compose_file = "/Users/jacksoom/programer/aihub/glance_mind_front/e2e/docker-compose.full-e2e.yml"
    try:
        subprocess.run(
            [
                "docker",
                "compose",
                "-f",
                compose_file,
                "stop",
                "novel-worker-e2e",
            ],
            check=False,
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
        )
    except Exception:
        pass
    try:
        yield
    finally:
        try:
            subprocess.run(
                [
                    "docker",
                    "compose",
                    "-f",
                    compose_file,
                    "start",
                    "novel-worker-e2e",
                ],
                check=False,
                stdout=subprocess.DEVNULL,
                stderr=subprocess.DEVNULL,
            )
        except Exception:
            pass


@pytest.fixture(scope="module")
def novel_worker_process(suspend_container_novel_worker):
    """Run the real novel worker as a host subprocess against E2E DB/Redis."""
    compose_file = "/Users/jacksoom/programer/aihub/glance_mind_front/e2e/docker-compose.full-e2e.yml"

    env = os.environ.copy()
    env["PYTHONPATH"] = (
        "/Users/jacksoom/programer/aihub/glance_mind_worker:"
        "/Users/jacksoom/programer/aihub/gm_ai_novel"
    )
    env["DATABASE_URL"] = os.getenv("DATABASE_URL", "postgresql://aihub_e2e_user:aihub_e2e_password@localhost:5436/aihub_e2e_db")
    env["REDIS_URL"] = E2E_REDIS_URL
    env["NOVEL_ENGINE_PATH"] = "/Users/jacksoom/programer/aihub/gm_ai_novel"

    proc = subprocess.Popen(
        [sys.executable, "-m", "novel_worker.run"],
        env=env,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        text=True,
    )
    time.sleep(2)
    try:
        yield proc
    finally:
        proc.terminate()
        try:
            proc.communicate(timeout=5)
        except Exception:
            proc.kill()


class TestNovelProjectCrud:
    def test_create_project_success_and_db_fields(self, auth_client, db_connection, test_user_id):
        body = _valid_create_body(
            title="Novel CRUD Test",
            topic="A machine writes dreams into the sky.",
            genre="奇幻",
            num_chapters=6,
            target_words_per_chapter=1800,
        )
        resp = auth_client.post(f"{NOVEL_BASE}/projects", json=body)
        assert resp.status_code == 201

        data = extract_data(resp.json())
        project_id = data["project_id"]
        assert project_id
        assert data["status"] == "draft"
        assert data["title"] == body["title"]
        assert data["topic"] == body["topic"]
        assert data["genre"] == body["genre"]
        assert data["num_chapters"] == body["num_chapters"]
        assert data["target_words_per_chapter"] == body["target_words_per_chapter"]

        row = _q1(
            db_connection,
            "SELECT * FROM gm_novel_projects WHERE project_id = %s",
            (project_id,),
        )
        assert row is not None
        assert row["user_id"] == test_user_id
        assert row["title"] == body["title"]
        assert row["topic"] == body["topic"]
        assert row["genre"] == body["genre"]
        assert row["description"] == body["description"]
        assert row["num_chapters"] == body["num_chapters"]
        assert row["target_words_per_chapter"] == body["target_words_per_chapter"]
        assert row["status"] == "draft"
        assert row["created_at"] is not None
        assert row["updated_at"] is not None
        assert row["deleted_at"] is None

        snapshot = _q1(
            db_connection,
            "SELECT * FROM gm_novel_project_config_snapshots WHERE project_id = %s AND is_current = true",
            (project_id,),
        )
        assert snapshot is not None
        assert snapshot["proxy_setting"] == {}
        assert snapshot["webdav_config"] == {}
        assert snapshot["other_params"] == {}

    def test_list_projects_returns_page_response(self, auth_client):
        _create_project(auth_client)
        resp = auth_client.get(f"{NOVEL_BASE}/projects?page=1&page_size=20")
        assert resp.status_code == 200
        data = extract_data(resp.json())
        assert isinstance(data["list"], list)
        assert "total" in data
        assert data["page"] == 1
        assert data["page_size"] == 20

    def test_get_project_detail_includes_current_snapshot(self, auth_client):
        project_id = _create_project(auth_client)
        resp = auth_client.get(f"{NOVEL_BASE}/projects/{project_id}")
        assert resp.status_code == 200
        data = extract_data(resp.json())
        assert data["project_id"] == project_id
        assert data["status"] == "draft"
        assert "current_config_snapshot" in data
        assert data["current_config_snapshot"]["project_id"] == project_id

    def test_patch_project_updates_fields(self, auth_client, db_connection):
        project_id = _create_project(auth_client)
        resp = auth_client.patch(
            f"{NOVEL_BASE}/projects/{project_id}",
            json={
                "title": "Novel Updated",
                "description": "Updated description",
                "num_chapters": 8,
                "target_words_per_chapter": 2200,
            },
        )
        assert resp.status_code == 200
        data = extract_data(resp.json())
        assert data["title"] == "Novel Updated"
        assert data["description"] == "Updated description"
        assert data["num_chapters"] == 8
        assert data["target_words_per_chapter"] == 2200

        row = _q1(
            db_connection,
            "SELECT title, description, num_chapters, target_words_per_chapter FROM gm_novel_projects WHERE project_id = %s",
            (project_id,),
        )
        assert row["title"] == "Novel Updated"
        assert row["description"] == "Updated description"
        assert row["num_chapters"] == 8
        assert row["target_words_per_chapter"] == 2200

    def test_delete_project_soft_deletes(self, auth_client, db_connection):
        project_id = _create_project(auth_client)
        resp = auth_client.delete(f"{NOVEL_BASE}/projects/{project_id}")
        assert resp.status_code == 200
        data = extract_data(resp.json())
        assert data["deleted"] is True
        assert data["project_id"] == project_id

        row = _q1(
            db_connection,
            "SELECT deleted_at FROM gm_novel_projects WHERE project_id = %s",
            (project_id,),
        )
        assert row is not None
        assert row["deleted_at"] is not None

    def test_cancel_project_marks_jobs_cancel_requested(self, auth_client, db_connection):
        project_id = _create_project(auth_client)
        _ensure_clean(db_connection)
        cur = db_connection.cursor()
        cur.execute(
            """
            INSERT INTO gm_novel_jobs (project_id, stage_code, task_type, status, request_payload, result_payload, error_payload)
            VALUES (%s, 'n01_architecture', 'generate_architecture', 'pending', '{}'::jsonb, '{}'::jsonb, '{}'::jsonb)
            """,
            (project_id,),
        )
        db_connection.commit()
        cur.close()
        resp = auth_client.post(f"{NOVEL_BASE}/projects/{project_id}/cancel", json={})
        assert resp.status_code == 200
        data = extract_data(resp.json())
        assert data["status"] == "cancel_requested"
        rows = _qall(
            db_connection,
            "SELECT status FROM gm_novel_jobs WHERE project_id = %s",
            (project_id,),
        )
        assert rows
        statuses = [r["status"] for r in rows]
        assert "cancel_requested" in statuses
        assert "pending" not in statuses
        assert "running" not in statuses


class TestNovelOwnership:
    def test_other_user_cannot_get_project(self, auth_client, second_auth_client):
        project_id = _create_project(auth_client)
        resp = second_auth_client.get(f"{NOVEL_BASE}/projects/{project_id}")
        assert resp.status_code == 404

    def test_other_user_cannot_patch_project(self, auth_client, second_auth_client):
        project_id = _create_project(auth_client)
        resp = second_auth_client.patch(
            f"{NOVEL_BASE}/projects/{project_id}",
            json={"title": "Should Not Update"},
        )
        assert resp.status_code == 404

    def test_other_user_cannot_delete_project(self, auth_client, second_auth_client):
        project_id = _create_project(auth_client)
        resp = second_auth_client.delete(f"{NOVEL_BASE}/projects/{project_id}")
        assert resp.status_code == 404

    def test_other_user_cannot_cancel_project(self, auth_client, second_auth_client):
        project_id = _create_project(auth_client)
        resp = second_auth_client.post(f"{NOVEL_BASE}/projects/{project_id}/cancel", json={})
        assert resp.status_code == 404


class TestNovelValidationAndAuth:
    def test_create_without_auth_returns_401(self, api_client):
        resp = api_client.post(f"{NOVEL_BASE}/projects", json=_valid_create_body())
        assert resp.status_code == 401

    def test_create_with_missing_required_fields_returns_422_or_400(self, auth_client):
        resp = auth_client.post(
            f"{NOVEL_BASE}/projects",
            json={
                "title": "",
                "config_snapshot": {"proxy_setting": {}, "webdav_config": {}, "other_params": {}},
            },
        )
        assert resp.status_code in [400, 422]


class TestNovelConfigSnapshots:
    def test_list_config_snapshots_returns_current(self, auth_client):
        project_id = _create_project(auth_client)
        resp = auth_client.get(f"{NOVEL_BASE}/projects/{project_id}/config-snapshots")
        assert resp.status_code == 200
        data = extract_data(resp.json())
        assert isinstance(data, list)
        assert len(data) >= 1
        assert any(item["is_current"] for item in data)

    def test_create_and_activate_config_snapshot(self, auth_client, db_connection):
        project_id = _create_project(auth_client)
        create_resp = auth_client.post(
            f"{NOVEL_BASE}/projects/{project_id}/config-snapshots",
            json={
                "proxy_setting": {"http": "http://proxy.local:7890"},
                "webdav_config": {"endpoint": "https://dav.example.com"},
                "other_params": {"temperature_override": 0.3},
            },
        )
        assert create_resp.status_code == 201
        snapshot = extract_data(create_resp.json())
        snapshot_id = snapshot["id"]
        assert snapshot["project_id"] == project_id
        assert snapshot["is_current"] is True

        activate_resp = auth_client.post(
            f"{NOVEL_BASE}/projects/{project_id}/config-snapshots/{snapshot_id}/activate",
            json={},
        )
        assert activate_resp.status_code == 200
        data = extract_data(activate_resp.json())
        assert data["activated"] is True
        assert data["snapshot_id"] == snapshot_id

        active = _q1(
            db_connection,
            "SELECT id, is_current FROM gm_novel_project_config_snapshots WHERE project_id = %s AND is_current = true ORDER BY id DESC LIMIT 1",
            (project_id,),
        )
        assert active is not None
        assert active["id"] == snapshot_id
        assert active["is_current"] is True


class TestNovelReadEndpoints:
    def test_list_chapters_initially_empty(self, auth_client):
        project_id = _create_project(auth_client)
        resp = auth_client.get(f"{NOVEL_BASE}/projects/{project_id}/chapters")
        assert resp.status_code == 200
        data = extract_data(resp.json())
        assert isinstance(data, list)
        assert len(data) == 0


class TestNovelWorkerDispatch:
    def test_generate_architecture_enqueues_worker_task(self, auth_client, db_connection):
        project_id = _create_project(auth_client)
        snapshot_id = _current_snapshot_id(auth_client, project_id)
        _clear_novel_queue()

        resp = auth_client.post(
            f"{NOVEL_BASE}/projects/{project_id}/architecture/generate",
            json={
                "config_snapshot_id": snapshot_id,
                "override_user_guidance": "Focus on mystery and suspense.",
            },
        )
        assert resp.status_code == 201, f"architecture enqueue failed: {resp.status_code} {resp.text}"
        data = extract_data(resp.json())
        assert data["accepted"] is True
        job_id = data["job_id"]

        job = _q1(
            db_connection,
            "SELECT * FROM gm_novel_jobs WHERE id = %s",
            (job_id,),
        )
        assert job is not None
        assert job["project_id"] == project_id
        assert job["stage_code"] == "n01_architecture"
        assert job["task_type"] == "generate_architecture"
        assert job["status"] == "pending"
        assert job["created_by"] is not None

        queued = _peek_novel_queue()
        assert len(queued) == 1
        assert "\"task_type\":\"generate_architecture\"" in queued[0]
        assert f"\"project_id\":\"{project_id}\"" in queued[0]
        assert f"\"job_id\":{job_id}" in queued[0]

    def test_generate_blueprint_enqueues_worker_task(self, auth_client, db_connection):
        project_id = _create_project(auth_client)
        snapshot_id = _current_snapshot_id(auth_client, project_id)
        _clear_novel_queue()

        resp = auth_client.post(
            f"{NOVEL_BASE}/projects/{project_id}/blueprint/generate",
            json={
                "config_snapshot_id": snapshot_id,
                "override_user_guidance": "Keep chapter arcs concise.",
            },
        )
        assert resp.status_code == 201
        data = extract_data(resp.json())
        assert data["accepted"] is True
        job_id = data["job_id"]

        job = _q1(
            db_connection,
            "SELECT stage_code, task_type, status FROM gm_novel_jobs WHERE id = %s",
            (job_id,),
        )
        assert job["stage_code"] == "n02_blueprint"
        assert job["task_type"] == "generate_blueprint"
        assert job["status"] == "pending"

        queued = _peek_novel_queue()
        assert len(queued) == 1
        assert "\"task_type\":\"generate_blueprint\"" in queued[0]
        assert f"\"project_id\":\"{project_id}\"" in queued[0]

    def test_build_chapter_prompt_enqueues_chapter_scoped_task(self, auth_client, db_connection):
        project_id = _create_project(auth_client)
        snapshot_id = _current_snapshot_id(auth_client, project_id)
        _clear_novel_queue()

        resp = auth_client.post(
            f"{NOVEL_BASE}/projects/{project_id}/chapters/1/prompt/build",
            json={
                "config_snapshot_id": snapshot_id,
                "user_guidance": "Open with a tense hook.",
                "characters_involved": "Mira, Sol",
                "key_items": "mirror shard",
                "scene_location": "city archive",
                "time_constraint": "midnight",
            },
        )
        assert resp.status_code == 201
        data = extract_data(resp.json())
        assert data["accepted"] is True
        job_id = data["job_id"]

        job = _q1(
            db_connection,
            "SELECT chapter_number, stage_code, task_type, status FROM gm_novel_jobs WHERE id = %s",
            (job_id,),
        )
        assert job["chapter_number"] == 1
        assert job["stage_code"] == "n03_chapter_prompt"
        assert job["task_type"] == "build_chapter_prompt"
        assert job["status"] == "pending"

        queued = _peek_novel_queue()
        assert len(queued) == 1
        assert "\"task_type\":\"build_chapter_prompt\"" in queued[0]
        assert "\"chapter_number\":1" in queued[0]


class TestNovelWorkerClosedLoop:
    def _create_project_with_profiles(self, auth_client):
        project_id = _create_project(auth_client)
        arch_id = _create_llm_profile(auth_client, "arch")
        bp_id = _create_llm_profile(auth_client, "bp")
        draft_id = _create_llm_profile(auth_client, "draft")
        snapshot_id = _create_snapshot_with_llm(auth_client, project_id, arch_id)
        # overwrite snapshot so each stage can use different profile ids if needed
        resp = auth_client.post(
            f"{NOVEL_BASE}/projects/{project_id}/config-snapshots",
            json={
                "architecture_llm_profile_id": arch_id,
                "chapter_outline_llm_profile_id": bp_id,
                "prompt_draft_llm_profile_id": draft_id,
                "final_chapter_llm_profile_id": draft_id,
                "consistency_review_llm_profile_id": draft_id,
                "proxy_setting": {},
                "webdav_config": {},
                "other_params": {},
            },
        )
        assert resp.status_code == 201
        snapshot_id = extract_data(resp.json())["id"]
        return project_id, snapshot_id

    def _wait_job_status(self, db_connection, job_id: int, timeout: int = 50):
        deadline = time.time() + timeout
        row = None
        while time.time() < deadline:
            row = _q1(
                db_connection,
                "SELECT status, error_payload FROM gm_novel_jobs WHERE id = %s",
                (job_id,),
            )
            assert row is not None
            if row["status"] in ("completed", "failed", "partial_failed"):
                return row
            time.sleep(2)
        return row

    def test_generate_architecture_consumed_and_persisted(
        self, auth_client, db_connection, novel_worker_process
    ):
        project_id, snapshot_id = self._create_project_with_profiles(auth_client)
        _clear_novel_queue()

        resp = auth_client.post(
            f"{NOVEL_BASE}/projects/{project_id}/architecture/generate",
            json={
                "config_snapshot_id": snapshot_id,
                "override_user_guidance": "Use a mystery-driven structure.",
            },
        )
        assert resp.status_code == 201, f"generate architecture failed: {resp.status_code} {resp.text}"
        job_id = extract_data(resp.json())["job_id"]

        row = self._wait_job_status(db_connection, job_id)
        assert row["status"] == "completed", f"worker did not complete architecture job, final status={row['status']}"

        arch = _q1(
            db_connection,
            "SELECT full_text, is_current FROM gm_novel_architectures WHERE project_id = %s ORDER BY id DESC LIMIT 1",
            (project_id,),
        )
        assert arch is not None
        assert arch["is_current"] is True
        assert arch["full_text"] is not None
        assert len(arch["full_text"]) > 0

        events = _qall(
            db_connection,
            "SELECT event_type, stage_code FROM gm_novel_stage_events WHERE project_id = %s ORDER BY sequence",
            (project_id,),
        )
        assert len(events) >= 2
        assert events[0]["event_type"] == "stage_started"
        assert events[-1]["event_type"] == "stage_completed"
        assert all(e["stage_code"] == "n01_architecture" for e in events)

        stage_runs = _qall(
            db_connection,
            "SELECT status FROM gm_novel_stage_runs WHERE job_id = %s ORDER BY id",
            (job_id,),
        )
        assert stage_runs
        assert stage_runs[-1]["status"] == "completed"

    def test_generate_blueprint_consumed_and_persisted(
        self, auth_client, db_connection, novel_worker_process
    ):
        project_id, snapshot_id = self._create_project_with_profiles(auth_client)
        _clear_novel_queue()

        # architecture first
        resp = auth_client.post(
            f"{NOVEL_BASE}/projects/{project_id}/architecture/generate",
            json={"config_snapshot_id": snapshot_id},
        )
        assert resp.status_code == 201
        arch_job_id = extract_data(resp.json())["job_id"]
        arch_job = self._wait_job_status(db_connection, arch_job_id)
        assert arch_job["status"] == "completed"

        resp = auth_client.post(
            f"{NOVEL_BASE}/projects/{project_id}/blueprint/generate",
            json={"config_snapshot_id": snapshot_id},
        )
        assert resp.status_code == 201
        job_id = extract_data(resp.json())["job_id"]
        row = self._wait_job_status(db_connection, job_id)
        assert row["status"] == "completed"

        bp = _q1(
            db_connection,
            "SELECT raw_text, is_current FROM gm_novel_blueprints WHERE project_id = %s ORDER BY id DESC LIMIT 1",
            (project_id,),
        )
        assert bp is not None
        assert bp["is_current"] is True
        assert bp["raw_text"] is not None
        assert len(bp["raw_text"]) > 0

        events = _qall(
            db_connection,
            "SELECT event_type, stage_code FROM gm_novel_stage_events WHERE project_id = %s ORDER BY sequence",
            (project_id,),
        )
        assert any(e["stage_code"] == "n02_blueprint" and e["event_type"] == "stage_started" for e in events)
        assert any(e["stage_code"] == "n02_blueprint" and e["event_type"] == "stage_completed" for e in events)

    def test_build_prompt_and_generate_draft_consumed_and_persisted(
        self, auth_client, db_connection, novel_worker_process
    ):
        project_id, snapshot_id = self._create_project_with_profiles(auth_client)
        _clear_novel_queue()

        # architecture + blueprint first
        for path in [
            f"{NOVEL_BASE}/projects/{project_id}/architecture/generate",
            f"{NOVEL_BASE}/projects/{project_id}/blueprint/generate",
        ]:
            resp = auth_client.post(path, json={"config_snapshot_id": snapshot_id})
            assert resp.status_code == 201
            row = self._wait_job_status(db_connection, extract_data(resp.json())["job_id"])
            assert row["status"] == "completed"

        # build prompt
        resp = auth_client.post(
            f"{NOVEL_BASE}/projects/{project_id}/chapters/1/prompt/build",
            json={
                "config_snapshot_id": snapshot_id,
                "user_guidance": "Open with a strong mystery hook.",
                "characters_involved": "Mira, Sol",
                "key_items": "glass map",
                "scene_location": "old harbor",
                "time_constraint": "before sunrise",
            },
        )
        assert resp.status_code == 201
        row = self._wait_job_status(db_connection, extract_data(resp.json())["job_id"])
        assert row["status"] == "completed"

        prompt = _q1(
            db_connection,
            "SELECT id, prompt_text FROM gm_novel_chapter_prompts WHERE project_id = %s AND chapter_number = 1 AND is_current = true ORDER BY id DESC LIMIT 1",
            (project_id,),
        )
        assert prompt is not None
        assert prompt["prompt_text"] is not None
        assert len(prompt["prompt_text"]) > 0

        # generate draft
        resp = auth_client.post(
            f"{NOVEL_BASE}/projects/{project_id}/chapters/1/draft/generate",
            json={"config_snapshot_id": snapshot_id, "prompt_id": prompt["id"]},
        )
        assert resp.status_code == 201
        row = self._wait_job_status(db_connection, extract_data(resp.json())["job_id"])
        assert row["status"] == "completed"

        chapter = _q1(
            db_connection,
            "SELECT status, draft_text, draft_word_count FROM gm_novel_chapters WHERE project_id = %s AND chapter_number = 1 ORDER BY id DESC LIMIT 1",
            (project_id,),
        )
        assert chapter is not None
        assert chapter["status"] == "drafted"
        assert chapter["draft_text"] is not None
        assert len(chapter["draft_text"]) > 0
        assert chapter["draft_word_count"] > 0

        events = _qall(
            db_connection,
            "SELECT event_type, stage_code FROM gm_novel_stage_events WHERE project_id = %s ORDER BY sequence",
            (project_id,),
        )
        assert any(e["stage_code"] == "n03_chapter_prompt" and e["event_type"] == "stage_completed" for e in events)
        assert any(e["stage_code"] == "n04_chapter_draft" and e["event_type"] == "stage_completed" for e in events)

    def _run_architecture_and_blueprint_and_draft(self, auth_client, db_connection):
        """Helper: create project with profiles, run arch -> bp -> prompt -> draft, return (project_id, snapshot_id)."""
        project_id, snapshot_id = self._create_project_with_profiles(auth_client)
        _clear_novel_queue()

        for path in [
            f"{NOVEL_BASE}/projects/{project_id}/architecture/generate",
            f"{NOVEL_BASE}/projects/{project_id}/blueprint/generate",
        ]:
            resp = auth_client.post(path, json={"config_snapshot_id": snapshot_id})
            assert resp.status_code == 201
            row = self._wait_job_status(db_connection, extract_data(resp.json())["job_id"])
            assert row["status"] == "completed"

        resp = auth_client.post(
            f"{NOVEL_BASE}/projects/{project_id}/chapters/1/prompt/build",
            json={
                "config_snapshot_id": snapshot_id,
                "user_guidance": "A tense opening.",
                "characters_involved": "Mira",
                "key_items": "crystal key",
                "scene_location": "underground vault",
                "time_constraint": "midnight",
            },
        )
        assert resp.status_code == 201
        row = self._wait_job_status(db_connection, extract_data(resp.json())["job_id"])
        assert row["status"] == "completed"

        prompt = _q1(
            db_connection,
            "SELECT id FROM gm_novel_chapter_prompts WHERE project_id = %s AND chapter_number = 1 AND is_current = true ORDER BY id DESC LIMIT 1",
            (project_id,),
        )
        assert prompt is not None

        resp = auth_client.post(
            f"{NOVEL_BASE}/projects/{project_id}/chapters/1/draft/generate",
            json={"config_snapshot_id": snapshot_id, "prompt_id": prompt["id"]},
        )
        assert resp.status_code == 201
        row = self._wait_job_status(db_connection, extract_data(resp.json())["job_id"])
        assert row["status"] == "completed"

        return project_id, snapshot_id

    def test_enrich_chapter_consumed_and_persisted(
        self, auth_client, db_connection, novel_worker_process
    ):
        project_id, snapshot_id = self._run_architecture_and_blueprint_and_draft(auth_client, db_connection)

        chapter_before = _q1(
            db_connection,
            "SELECT draft_text, draft_word_count, is_enriched FROM gm_novel_chapters WHERE project_id = %s AND chapter_number = 1",
            (project_id,),
        )
        assert chapter_before is not None
        assert chapter_before["draft_text"] is not None
        assert chapter_before["is_enriched"] is False

        resp = auth_client.post(
            f"{NOVEL_BASE}/projects/{project_id}/chapters/1/enrich",
            json={"config_snapshot_id": snapshot_id, "target_words": 3000},
        )
        assert resp.status_code == 201, f"enrich enqueue failed: {resp.status_code} {resp.text}"
        job_id = extract_data(resp.json())["job_id"]

        row = self._wait_job_status(db_connection, job_id)
        assert row["status"] == "completed", f"enrich job did not complete, status={row['status']}"

        chapter_after = _q1(
            db_connection,
            "SELECT draft_text, draft_word_count, is_enriched, status FROM gm_novel_chapters WHERE project_id = %s AND chapter_number = 1",
            (project_id,),
        )
        assert chapter_after is not None
        assert chapter_after["is_enriched"] is True
        assert chapter_after["status"] == "drafted"
        assert chapter_after["draft_text"] is not None
        assert len(chapter_after["draft_text"]) > 0
        assert chapter_after["draft_word_count"] > 0

        events = _qall(
            db_connection,
            "SELECT event_type, stage_code FROM gm_novel_stage_events WHERE project_id = %s ORDER BY sequence",
            (project_id,),
        )
        assert any(e["stage_code"] == "n05_enrich" and e["event_type"] == "stage_completed" for e in events)

    def test_finalize_chapter_consumed_and_persisted(
        self, auth_client, db_connection, novel_worker_process
    ):
        project_id, snapshot_id = self._run_architecture_and_blueprint_and_draft(auth_client, db_connection)

        resp = auth_client.post(
            f"{NOVEL_BASE}/projects/{project_id}/chapters/1/finalize",
            json={"config_snapshot_id": snapshot_id, "use_current_draft_text": True},
        )
        assert resp.status_code == 201, f"finalize enqueue failed: {resp.status_code} {resp.text}"
        job_id = extract_data(resp.json())["job_id"]

        row = self._wait_job_status(db_connection, job_id)
        assert row["status"] == "completed", f"finalize job did not complete, status={row['status']}"

        chapter = _q1(
            db_connection,
            "SELECT status, final_text, final_word_count, finalized_at FROM gm_novel_chapters WHERE project_id = %s AND chapter_number = 1",
            (project_id,),
        )
        assert chapter is not None
        assert chapter["status"] == "finalized"
        assert chapter["final_text"] is not None
        assert len(chapter["final_text"]) > 0
        assert chapter["final_word_count"] > 0
        assert chapter["finalized_at"] is not None

        events = _qall(
            db_connection,
            "SELECT event_type, stage_code FROM gm_novel_stage_events WHERE project_id = %s ORDER BY sequence",
            (project_id,),
        )
        assert any(e["stage_code"] == "n06_finalize" and e["event_type"] == "stage_completed" for e in events)

        stage_runs = _qall(
            db_connection,
            "SELECT status FROM gm_novel_stage_runs WHERE job_id = %s ORDER BY id",
            (job_id,),
        )
        assert stage_runs
        assert stage_runs[-1]["status"] == "completed"

    def test_consistency_check_consumed_and_persisted(
        self, auth_client, db_connection, novel_worker_process
    ):
        project_id, snapshot_id = self._run_architecture_and_blueprint_and_draft(auth_client, db_connection)

        resp = auth_client.post(
            f"{NOVEL_BASE}/projects/{project_id}/chapters/1/consistency-checks",
            json={"config_snapshot_id": snapshot_id},
        )
        assert resp.status_code == 201, f"consistency check enqueue failed: {resp.status_code} {resp.text}"
        job_id = extract_data(resp.json())["job_id"]

        row = self._wait_job_status(db_connection, job_id)
        assert row["status"] == "completed", f"consistency check did not complete, status={row['status']}"

        check = _q1(
            db_connection,
            "SELECT * FROM gm_novel_consistency_checks WHERE project_id = %s AND chapter_number = 1 ORDER BY id DESC LIMIT 1",
            (project_id,),
        )
        assert check is not None
        assert check["status"] == "completed"
        assert check["result_text"] is not None
        assert len(check["result_text"]) > 0
        assert check["chapter_text"] is not None
        assert len(check["chapter_text"]) > 0
        assert check["novel_setting_text"] is not None
        assert check["chapter_number"] == 1

        chapter = _q1(
            db_connection,
            "SELECT consistency_status FROM gm_novel_chapters WHERE project_id = %s AND chapter_number = 1",
            (project_id,),
        )
        assert chapter is not None
        assert chapter["consistency_status"] == "checked"

        events = _qall(
            db_connection,
            "SELECT event_type, stage_code FROM gm_novel_stage_events WHERE project_id = %s ORDER BY sequence",
            (project_id,),
        )
        assert any(e["stage_code"] == "n08_consistency" and e["event_type"] == "stage_completed" for e in events)

    def test_clear_memory_consumed_and_persisted(
        self, auth_client, db_connection, novel_worker_process
    ):
        project_id, snapshot_id = self._create_project_with_profiles(auth_client)
        _clear_novel_queue()

        resp = auth_client.post(
            f"{NOVEL_BASE}/projects/{project_id}/memory/clear",
            json={},
        )
        assert resp.status_code == 201, f"clear memory enqueue failed: {resp.status_code} {resp.text}"
        job_id = extract_data(resp.json())["job_id"]

        row = self._wait_job_status(db_connection, job_id)
        assert row["status"] == "completed", f"clear memory did not complete, status={row['status']}"

        ops = _qall(
            db_connection,
            "SELECT operation_type, status FROM hb_novel_vector_operations WHERE project_id = %s ORDER BY id DESC",
            (project_id,),
        )
        assert any(op["operation_type"] == "clear" and op["status"] == "completed" for op in ops)

        events = _qall(
            db_connection,
            "SELECT event_type, stage_code FROM gm_novel_stage_events WHERE project_id = %s ORDER BY sequence",
            (project_id,),
        )
        assert any(e["stage_code"] == "n09_memory" and e["event_type"] == "stage_completed" for e in events)

    def test_enrich_then_finalize_full_chain(
        self, auth_client, db_connection, novel_worker_process
    ):
        """Full chain: arch -> bp -> prompt -> draft -> enrich -> finalize, verifying all artifacts."""
        project_id, snapshot_id = self._run_architecture_and_blueprint_and_draft(auth_client, db_connection)

        resp = auth_client.post(
            f"{NOVEL_BASE}/projects/{project_id}/chapters/1/enrich",
            json={"config_snapshot_id": snapshot_id, "target_words": 2000},
        )
        assert resp.status_code == 201
        row = self._wait_job_status(db_connection, extract_data(resp.json())["job_id"])
        assert row["status"] == "completed"

        enriched = _q1(
            db_connection,
            "SELECT draft_text, is_enriched FROM gm_novel_chapters WHERE project_id = %s AND chapter_number = 1",
            (project_id,),
        )
        assert enriched["is_enriched"] is True

        resp = auth_client.post(
            f"{NOVEL_BASE}/projects/{project_id}/chapters/1/finalize",
            json={"config_snapshot_id": snapshot_id, "use_current_draft_text": True},
        )
        assert resp.status_code == 201
        row = self._wait_job_status(db_connection, extract_data(resp.json())["job_id"])
        assert row["status"] == "completed"

        final_chapter = _q1(
            db_connection,
            "SELECT status, final_text, final_word_count, finalized_at FROM gm_novel_chapters WHERE project_id = %s AND chapter_number = 1",
            (project_id,),
        )
        assert final_chapter["status"] == "finalized"
        assert final_chapter["final_text"] is not None
        assert len(final_chapter["final_text"]) > 0
        assert final_chapter["final_word_count"] > 0
        assert final_chapter["finalized_at"] is not None

        jobs = _qall(
            db_connection,
            "SELECT stage_code, task_type, status FROM gm_novel_jobs WHERE project_id = %s ORDER BY id",
            (project_id,),
        )
        task_types_completed = [j["task_type"] for j in jobs if j["status"] == "completed"]
        assert "generate_architecture" in task_types_completed
        assert "generate_blueprint" in task_types_completed
        assert "build_chapter_prompt" in task_types_completed
        assert "generate_chapter_draft" in task_types_completed
        assert "enrich_chapter" in task_types_completed or "enrich_chapter_text" in task_types_completed
        assert "finalize_chapter" in task_types_completed

    def test_list_jobs_initially_empty(self, auth_client):
        project_id = _create_project(auth_client)
        resp = auth_client.get(f"{NOVEL_BASE}/projects/{project_id}/jobs")
        assert resp.status_code == 200
        data = extract_data(resp.json())
        assert isinstance(data, list)
        assert len(data) == 0
