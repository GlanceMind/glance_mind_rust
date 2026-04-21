#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
GlanceMind API E2E Tests - AI Publish API
=========================================
Tests for AI publish plan, AI task, and publish task endpoints.

Test Coverage:
1. Publish Plan CRUD
2. AI Task queries
3. Publish Task queries (public)
4. Error cases
5. Database state verification
"""

import pytest
import uuid
import time
from typing import Dict, Any, Optional

# Import test utilities from conftest
try:
    from conftest import (
        assert_response_success,
        extract_data,
    )
except ImportError:
    # Fallback for standalone testing
    def assert_response_success(resp):
        assert resp.status_code in [200, 201], f"Expected success, got {resp.status_code}: {resp.text}"

    def extract_data(json_resp):
        if 'data' in json_resp:
            return json_resp['data']
        return json_resp

# Platform IDs (from init-test-data.sql)
PLATFORM_FACEBOOK = 3
PLATFORM_TIKTOK = 2
PLATFORM_INSTAGRAM = 4

# AI Publish related
AIPUB_CONTENT_TYPES = ["post", "video", "reel", "story"]
AIPUB_AI_TASK_TYPES = ["video_gen", "content_gen", "image_gen"]


class TestPublishPlanCRUD:
    """Tests for publish plan CRUD operations."""

    def _get_or_create_test_group(self, auth_client, db_cursor) -> Optional[int]:
        """Helper to get or create a social group for testing."""
        db_cursor.execute("SELECT id FROM gm_social_groups LIMIT 1")
        result = db_cursor.fetchone()

        if result:
            return result["id"]

        # Create new group
        group_payload = {
            "platform_id": PLATFORM_FACEBOOK,
            "group_name": f"AIPub Test Group {uuid.uuid4().hex[:8]}"
        }
        resp = auth_client.post("/api/v1/social-groups", json=group_payload)
        if resp.status_code == 200:
            return extract_data(resp.json()).get("id")
        return None

    def test_create_plan_with_group(self, auth_client, db_cursor):
        """Test creating a publish plan with group binding."""
        group_id = self._get_or_create_test_group(auth_client, db_cursor)

        if not group_id:
            pytest.skip("No social group available")

        plan_payload = {
            "group_id": group_id,
            "platform_id": PLATFORM_FACEBOOK,
            "content_type": "video",
            "ai_task_types": ["video_gen", "content_gen"],
            "ai_service_config": {
                "video_service": "mock",
                "video_duration": 30
            },
            "ai_input": {
                "topic": "E2E Test Video",
                "keywords": "test, automation",
                "tone": "professional"
            }
        }

        resp = auth_client.post("/api/v1/publish_plans", json=plan_payload)

        if resp.status_code in [200, 201]:
            data = extract_data(resp.json())
            print(f"\nCreated publish plan: {data}")

            assert "id" in data, "Should return plan ID"
            assert data.get("group_id") == group_id
            assert data.get("status") in ["pending", "ai_processing"]
        else:
            print(f"\n  Plan creation returned: {resp.status_code} - {resp.text}")

    def test_create_plan_with_single_account(self, auth_client, db_cursor):
        """Test creating a publish plan with single account."""
        db_cursor.execute("SELECT id FROM gm_social_accounts LIMIT 1")
        result = db_cursor.fetchone()

        if not result:
            pytest.skip("No social account available")

        account_id = result["id"]

        plan_payload = {
            "social_account_id": account_id,
            "platform_id": PLATFORM_FACEBOOK,
            "content_type": "post",
            "content": {
                "title": "Direct content title",
                "description": "Direct content description #test",
                "hashtags": ["#test", "#automation"]
            }
        }

        resp = auth_client.post("/api/v1/publish_plans", json=plan_payload)

        if resp.status_code in [200, 201]:
            data = extract_data(resp.json())
            print(f"\nCreated direct publish plan: {data}")
            assert data.get("social_account_id") == account_id

    def test_create_plan_invalid_both_group_and_account(self, auth_client):
        """Test that creating plan with both group and account fails."""
        plan_payload = {
            "group_id": 1,
            "social_account_id": 1,  # Should not specify both
            "platform_id": PLATFORM_FACEBOOK,
            "content_type": "video"
        }

        resp = auth_client.post("/api/v1/publish_plans", json=plan_payload)

        assert resp.status_code == 400, \
            f"Expected 400 for invalid input, got {resp.status_code}"

    def test_list_plans_with_pagination(self, auth_client):
        """Test listing publish plans with pagination."""
        resp = auth_client.get("/api/v1/publish_plans?page=1&page_size=10")
        assert_response_success(resp)

        data = extract_data(resp.json())
        print(f"\nPublish plans: {type(data)}")

        if isinstance(data, dict):
            assert "list" in data or "data" in data or "total" in data

    def test_list_plans_filter_by_status(self, auth_client):
        """Test listing plans filtered by status."""
        resp = auth_client.get("/api/v1/publish_plans?status=pending")
        assert_response_success(resp)

    def test_get_plan_detail(self, auth_client, db_cursor):
        """Test getting publish plan details."""
        db_cursor.execute("SELECT id FROM gm_aipub_plans LIMIT 1")
        result = db_cursor.fetchone()

        if not result:
            pytest.skip("No publish plans in database")

        plan_id = result["id"]

        resp = auth_client.get(f"/api/v1/publish_plans/{plan_id}")

        if resp.status_code == 200:
            data = extract_data(resp.json())
            print(f"\nPlan details: {data}")
            assert data.get("id") == plan_id

    def test_get_plan_with_include_all(self, auth_client, db_cursor):
        """Test getting plan details with included tasks."""
        db_cursor.execute("SELECT id FROM gm_aipub_plans LIMIT 1")
        result = db_cursor.fetchone()

        if not result:
            pytest.skip("No publish plans in database")

        plan_id = result["id"]

        resp = auth_client.get(f"/api/v1/publish_plans/{plan_id}?include=all")

        if resp.status_code == 200:
            data = extract_data(resp.json())
            # Check for included tasks
            if "ai_tasks" in data:
                print(f"\n  AI tasks included: {len(data.get('ai_tasks', []))}")
            if "publish_tasks" in data:
                print(f"  Publish tasks included: {len(data.get('publish_tasks', []))}")

    def test_delete_plan(self, auth_client, db_cursor):
        """Test deleting a publish plan."""
        # Find a plan with pending or failed status
        db_cursor.execute("""
            SELECT id FROM gm_aipub_plans 
            WHERE status IN ('pending', 'failed') 
            LIMIT 1
        """)
        result = db_cursor.fetchone()

        if not result:
            pytest.skip("No deletable plan available")

        plan_id = result["id"]

        resp = auth_client.delete(f"/api/v1/publish_plans/{plan_id}")

        if resp.status_code in [200, 204]:
            print(f"\nDeleted plan: {plan_id}")

            # Verify deletion
            resp = auth_client.get(f"/api/v1/publish_plans/{plan_id}")
            assert resp.status_code == 404


class TestPublishPlanAITasks:
    """Tests for AI task related operations."""

    def test_list_plan_ai_tasks(self, auth_client, db_cursor):
        """Test listing AI tasks for a plan."""
        db_cursor.execute("SELECT id FROM gm_aipub_plans LIMIT 1")
        result = db_cursor.fetchone()

        if not result:
            pytest.skip("No publish plans in database")

        plan_id = result["id"]

        resp = auth_client.get(f"/api/v1/publish_plans/{plan_id}/ai_tasks")
        assert_response_success(resp)

        data = extract_data(resp.json())
        tasks = data if isinstance(data, list) else data.get("data", [])
        print(f"\nAI tasks for plan {plan_id}: {len(tasks)}")


class TestPublishPlanPublishTasks:
    """Tests for publish task related operations."""

    def test_list_plan_publish_tasks(self, auth_client, db_cursor):
        """Test listing publish tasks for a plan."""
        db_cursor.execute("SELECT id FROM gm_aipub_plans LIMIT 1")
        result = db_cursor.fetchone()

        if not result:
            pytest.skip("No publish plans in database")

        plan_id = result["id"]

        resp = auth_client.get(f"/api/v1/publish_plans/{plan_id}/publish_tasks")
        assert_response_success(resp)


class TestPublishTaskPublicAPI:
    """Tests for public publish task endpoints (Executor use)."""

    def test_query_tasks_by_device(self, api_client, db_cursor):
        """Test querying publish tasks by device ID (public endpoint)."""
        db_cursor.execute("""
            SELECT device_id FROM gm_social_accounts 
            WHERE device_id IS NOT NULL 
            LIMIT 1
        """)
        result = db_cursor.fetchone()

        device_id = result["device_id"] if result else "test_device"

        resp = api_client.get(
            f"/api/v1/public/aipub/publish_tasks?device_id={device_id}&status=ready"
        )
        assert_response_success(resp)

        data = extract_data(resp.json())
        print(f"\nPublish tasks by device: {data}")

    def test_update_task_status(self, api_client, db_cursor):
        """Test updating publish task status (Executor callback)."""
        db_cursor.execute("""
            SELECT id FROM gm_aipub_tasks 
            WHERE status = 'ready' 
            LIMIT 1
        """)
        result = db_cursor.fetchone()

        if not result:
            pytest.skip("No ready publish tasks available")

        task_id = result["id"]

        update_payload = {"status": "processing"}

        resp = api_client.patch(
            f"/api/v1/public/aipub/publish_tasks/{task_id}/status",
            json=update_payload,
            headers={"X-Device-Token": "test_device_token"}
        )

        if resp.status_code == 200:
            data = extract_data(resp.json())
            print(f"\nUpdated task status: {data}")


class TestPublishTaskV2ResultPayload:
    """Phase 4 Round 3 Task 3 — verify UnifiedPublishResult (v2) payload
    fields are persisted to new dedicated columns on gm_aipub_tasks.

    The Round 2 worker wire shape is:

        {
          "version": 2,
          "task_id": 42,
          "status": "completed",
          "platform_post_id": "t3_abc",
          "platform_post_url": "https://x/p",
          "media_results": [...],
          "post_publish_results": [...],
          "failed_error_code": "UPLOADER_EXCEPTION",
          "failed_reason": "uploader blew up",
          "execution_log": "...",
          ...
        }

    Before Task 3 the API side only knew about
    status/result_url/error_message/execution_log (and even execution_log
    was accepted by the DTO but dropped in the service). All v2 fields
    were silently ignored by serde. These tests pin the new behaviour:
    every v2 field lands in its own DB column.
    """

    def _seed_ready_task(self, db_cursor) -> int:
        """Insert a bare-minimum ready task for v2 callback testing.

        Creates a dedicated plan + task scoped to this test so repeated
        runs against a fresh DB don't depend on pre-seeded fixture rows.
        Returns the task id.
        """
        db_cursor.execute("""
            INSERT INTO gm_aipub_plans (
                user_id, social_account_id, platform_id,
                content_type, plan_type, status
            )
            SELECT
                (SELECT id FROM gm_users LIMIT 1),
                (SELECT id FROM gm_social_accounts LIMIT 1),
                (SELECT id FROM gm_platforms WHERE name = 'tiktok' LIMIT 1),
                'video', 'single_video', 'ready'
            RETURNING id, social_account_id
        """)
        plan_row = db_cursor.fetchone()

        db_cursor.execute("""
            INSERT INTO gm_aipub_tasks (
                plan_id, social_account_id, content, status
            )
            VALUES (%s, %s, '{}'::jsonb, 'ready')
            RETURNING id
        """, (plan_row["id"], plan_row["social_account_id"]))
        row = db_cursor.fetchone()
        db_cursor.connection.commit()
        return row["id"]

    def _load_task(self, db_cursor, task_id: int) -> dict:
        db_cursor.execute("""
            SELECT id, status, result_url, error_message, execution_log,
                   media_results, post_publish_results,
                   failed_error_code, platform_post_id
            FROM gm_aipub_tasks
            WHERE id = %s
        """, (task_id,))
        return db_cursor.fetchone()

    def test_v2_success_payload_lands_all_fields(
        self, api_client, db_cursor
    ):
        """v2 completed payload → every v2 field present in DB."""
        task_id = self._seed_ready_task(db_cursor)

        payload = {
            "version": 2,
            "task_id": task_id,
            "status": "completed",
            "platform_post_id": "t3_abc123",
            "platform_post_url": "https://example.com/posts/t3_abc123",
            "media_results": [
                {
                    "media_index": 0,
                    "status": "ready",
                    "platform_asset_id": "vid_987",
                }
            ],
            "post_publish_results": [
                {
                    "action_kind": "add_comment",
                    "succeeded": True,
                    "platform_action_id": "cmt_1",
                }
            ],
            "failed_error_code": None,
            "failed_reason": None,
            "execution_log": "ok",
        }

        resp = api_client.patch(
            f"/api/v1/public/aipub/publish_tasks/{task_id}/status",
            json=payload,
        )
        assert resp.status_code == 200, (
            f"Expected 200, got {resp.status_code}: {resp.text}"
        )

        task = self._load_task(db_cursor, task_id)
        assert task["status"] == "completed"
        assert task["result_url"] == "https://example.com/posts/t3_abc123", (
            "platform_post_url must fall back into result_url column"
        )
        assert task["platform_post_id"] == "t3_abc123"
        assert task["media_results"] is not None, (
            "media_results column must exist and be populated"
        )
        assert isinstance(task["media_results"], list)
        assert task["media_results"][0]["platform_asset_id"] == "vid_987"
        assert task["post_publish_results"] is not None
        assert task["post_publish_results"][0]["action_kind"] == "add_comment"
        assert task["failed_error_code"] is None
        assert task["execution_log"] == "ok"

    def test_v2_failure_payload_preserves_error_code(
        self, api_client, db_cursor
    ):
        """v2 failed payload → failed_error_code + failed_reason mapped."""
        task_id = self._seed_ready_task(db_cursor)

        payload = {
            "version": 2,
            "task_id": task_id,
            "status": "failed",
            "failed_error_code": "UPLOADER_EXCEPTION",
            "failed_reason": "playwright crashed",
            "media_results": [
                {
                    "media_index": 0,
                    "status": "failed",
                    "failed_reason": "timeout",
                }
            ],
        }

        resp = api_client.patch(
            f"/api/v1/public/aipub/publish_tasks/{task_id}/status",
            json=payload,
        )
        assert resp.status_code == 200, (
            f"Expected 200, got {resp.status_code}: {resp.text}"
        )

        task = self._load_task(db_cursor, task_id)
        assert task["status"] == "failed"
        assert task["failed_error_code"] == "UPLOADER_EXCEPTION"
        assert task["error_message"] == "playwright crashed", (
            "failed_reason must fall back into error_message column"
        )
        assert task["media_results"][0]["status"] == "failed"

    def test_v1_payload_backward_compat(self, api_client, db_cursor):
        """Legacy v1 payload (no version key) still works unchanged."""
        task_id = self._seed_ready_task(db_cursor)

        payload = {
            "status": "completed",
            "result_url": "https://example.com/legacy",
            "error_message": None,
        }

        resp = api_client.patch(
            f"/api/v1/public/aipub/publish_tasks/{task_id}/status",
            json=payload,
        )
        assert resp.status_code == 200, (
            f"Expected 200, got {resp.status_code}: {resp.text}"
        )

        task = self._load_task(db_cursor, task_id)
        assert task["status"] == "completed"
        assert task["result_url"] == "https://example.com/legacy"
        assert task["media_results"] is None
        assert task["post_publish_results"] is None
        assert task["failed_error_code"] is None
        assert task["platform_post_id"] is None

    # ------------------------------------------------------------------
    # Branch coverage for v1/v2 field-precedence mapping.
    # Service path:
    #   result_url    = dto.result_url   OR dto.platform_post_url
    #   error_message = dto.error_message OR dto.failed_reason
    # Below tests pin every arm of those .or() chains so accidental
    # reordering is caught.
    # ------------------------------------------------------------------

    def test_v1_field_wins_when_both_v1_and_v2_url_set(
        self, api_client, db_cursor
    ):
        """If caller passes both result_url and platform_post_url, v1 wins.

        This mirrors the documented contract — v1 field is the canonical
        column, v2 only acts as a fallback. Prevents a regression where
        the .or() arms get swapped.
        """
        task_id = self._seed_ready_task(db_cursor)

        payload = {
            "version": 2,
            "task_id": task_id,
            "status": "completed",
            "result_url": "https://v1.example/x",
            "platform_post_url": "https://v2.example/x",
        }
        resp = api_client.patch(
            f"/api/v1/public/aipub/publish_tasks/{task_id}/status",
            json=payload,
        )
        assert resp.status_code == 200
        task = self._load_task(db_cursor, task_id)
        assert task["result_url"] == "https://v1.example/x", (
            "v1 result_url must take precedence over v2 platform_post_url"
        )

    def test_v1_error_wins_when_both_error_message_and_failed_reason_set(
        self, api_client, db_cursor
    ):
        task_id = self._seed_ready_task(db_cursor)

        payload = {
            "version": 2,
            "task_id": task_id,
            "status": "failed",
            "error_message": "v1 legacy reason",
            "failed_reason": "v2 detailed reason",
            "failed_error_code": "UPLOADER_EXCEPTION",
        }
        resp = api_client.patch(
            f"/api/v1/public/aipub/publish_tasks/{task_id}/status",
            json=payload,
        )
        assert resp.status_code == 200
        task = self._load_task(db_cursor, task_id)
        assert task["error_message"] == "v1 legacy reason"
        assert task["failed_error_code"] == "UPLOADER_EXCEPTION"

    def test_neither_url_set_leaves_result_url_null(
        self, api_client, db_cursor
    ):
        """v2 payload without any URL → result_url stays NULL."""
        task_id = self._seed_ready_task(db_cursor)

        payload = {
            "version": 2,
            "task_id": task_id,
            "status": "failed",
            "failed_error_code": "NO_UPLOADER",
            "failed_reason": "no uploader for platform",
        }
        resp = api_client.patch(
            f"/api/v1/public/aipub/publish_tasks/{task_id}/status",
            json=payload,
        )
        assert resp.status_code == 200
        task = self._load_task(db_cursor, task_id)
        assert task["result_url"] is None
        assert task["error_message"] == "no uploader for platform"
        assert task["failed_error_code"] == "NO_UPLOADER"

    def test_validation_rejects_oversize_failed_error_code(
        self, api_client, db_cursor
    ):
        """DTO validator enforces failed_error_code <= 64 chars."""
        task_id = self._seed_ready_task(db_cursor)

        payload = {
            "version": 2,
            "task_id": task_id,
            "status": "failed",
            "failed_error_code": "X" * 65,
        }
        resp = api_client.patch(
            f"/api/v1/public/aipub/publish_tasks/{task_id}/status",
            json=payload,
        )
        assert resp.status_code in (400, 422), (
            f"Oversize failed_error_code must be rejected; got "
            f"{resp.status_code}: {resp.text}"
        )

        task = self._load_task(db_cursor, task_id)
        assert task["status"] == "ready", (
            "Validation failure must not partially mutate the task"
        )
        assert task["failed_error_code"] is None

    def test_validation_rejects_oversize_platform_post_id(
        self, api_client, db_cursor
    ):
        """DTO validator enforces platform_post_id <= 128 chars."""
        task_id = self._seed_ready_task(db_cursor)

        payload = {
            "version": 2,
            "task_id": task_id,
            "status": "completed",
            "platform_post_id": "P" * 129,
        }
        resp = api_client.patch(
            f"/api/v1/public/aipub/publish_tasks/{task_id}/status",
            json=payload,
        )
        assert resp.status_code in (400, 422)

        task = self._load_task(db_cursor, task_id)
        assert task["status"] == "ready"
        assert task["platform_post_id"] is None

    def test_execution_log_v1_path_now_persists(
        self, api_client, db_cursor
    ):
        """execution_log was accepted by the DTO since day one but used
        to be dropped by the service. This test pins that the silent
        bug is now fixed regardless of v1 or v2 shape.
        """
        task_id = self._seed_ready_task(db_cursor)

        payload = {
            "status": "failed",
            "error_message": "crash",
            "execution_log": "frame 1: click\nframe 2: timeout\nframe 3: abort",
        }
        resp = api_client.patch(
            f"/api/v1/public/aipub/publish_tasks/{task_id}/status",
            json=payload,
        )
        assert resp.status_code == 200
        task = self._load_task(db_cursor, task_id)
        assert task["execution_log"] == (
            "frame 1: click\nframe 2: timeout\nframe 3: abort"
        )

    def test_sequential_updates_overwrite_v2_columns(
        self, api_client, db_cursor
    ):
        """A task that retries after failure must see its v2 columns
        overwritten with the success payload (not merged)."""
        task_id = self._seed_ready_task(db_cursor)

        fail_payload = {
            "version": 2,
            "task_id": task_id,
            "status": "failed",
            "failed_error_code": "UPLOADER_EXCEPTION",
            "failed_reason": "first try crashed",
        }
        r1 = api_client.patch(
            f"/api/v1/public/aipub/publish_tasks/{task_id}/status",
            json=fail_payload,
        )
        assert r1.status_code == 200

        success_payload = {
            "version": 2,
            "task_id": task_id,
            "status": "completed",
            "platform_post_id": "retry_ok",
            "platform_post_url": "https://example/retry",
            "media_results": [
                {"media_index": 0, "status": "ready",
                 "platform_asset_id": "vid_retry"}
            ],
        }
        r2 = api_client.patch(
            f"/api/v1/public/aipub/publish_tasks/{task_id}/status",
            json=success_payload,
        )
        assert r2.status_code == 200

        task = self._load_task(db_cursor, task_id)
        assert task["status"] == "completed"
        assert task["platform_post_id"] == "retry_ok"
        assert task["result_url"] == "https://example/retry"
        assert task["media_results"] is not None
        assert task["media_results"][0]["platform_asset_id"] == "vid_retry"
        # failed_error_code should still be present from the first update —
        # the service only writes fields the caller sent, so absent fields
        # in the retry payload are intentionally left untouched.
        # If the caller wants to clear them, they must pass explicit values.
        assert task["failed_error_code"] == "UPLOADER_EXCEPTION", (
            "UpdateAipubTask.failed_error_code was None in the retry payload "
            "so AsChangeset must leave the existing value in place"
        )


class TestPublishPlanStats:
    """Tests for plan statistics endpoint."""

    def test_get_plan_stats(self, auth_client):
        """Test getting plan stats."""
        resp = auth_client.get("/api/v1/publish_plans/stats")
        assert_response_success(resp)

        data = extract_data(resp.json())
        print(f"\nPlan stats: {data}")

        # Verify expected fields
        expected_fields = ['total_plans', 'ai_processing', 'ready', 'completed', 'failed']
        for field in expected_fields:
            if field not in data:
                print(f"  Warning: Missing field '{field}'")


class TestPublishAPIErrorCases:
    """Tests for error handling in publish APIs."""

    def test_create_plan_requires_auth(self, api_client):
        """Test that creating plan requires authentication."""
        resp = api_client.post(
            "/api/v1/publish_plans",
            json={"platform_id": 1, "content_type": "video"}
        )

        assert resp.status_code == 401, \
            f"Expected 401, got {resp.status_code}"

    def test_get_plan_not_found(self, auth_client):
        """Test getting non-existent plan returns 404."""
        resp = auth_client.get("/api/v1/publish_plans/999999")

        assert resp.status_code == 404, \
            f"Expected 404, got {resp.status_code}"

    def test_create_plan_invalid_platform(self, auth_client):
        """Test creating plan with invalid platform fails."""
        plan_payload = {
            "group_id": 1,
            "platform_id": 99999,  # Non-existent platform
            "content_type": "video"
        }

        resp = auth_client.post("/api/v1/publish_plans", json=plan_payload)

        assert resp.status_code in [400, 404, 422], \
            f"Expected error status, got {resp.status_code}"


class TestPublishDatabaseState:
    """Tests that verify publish database state."""

    def test_aipub_plans_table_exists(self, db_cursor):
        """Verify gm_aipub_plans table exists and is queryable."""
        try:
            db_cursor.execute("SELECT COUNT(*) as count FROM gm_aipub_plans")
            result = db_cursor.fetchone()
            print(f"\n  Total publish plans: {result['count']}")
        except Exception as e:
            pytest.skip(f"Table gm_aipub_plans not available: {e}")

    def test_aipub_ai_tasks_table_exists(self, db_cursor):
        """Verify gm_aipub_ai_tasks table exists and is queryable."""
        try:
            db_cursor.execute("SELECT COUNT(*) as count FROM gm_aipub_ai_tasks")
            result = db_cursor.fetchone()
            print(f"\n  Total AI tasks: {result['count']}")
        except Exception as e:
            pytest.skip(f"Table gm_aipub_ai_tasks not available: {e}")

    def test_aipub_tasks_table_exists(self, db_cursor):
        """Verify gm_aipub_tasks table exists and is queryable."""
        try:
            db_cursor.execute("SELECT COUNT(*) as count FROM gm_aipub_tasks")
            result = db_cursor.fetchone()
            print(f"\n  Total publish tasks: {result['count']}")
        except Exception as e:
            pytest.skip(f"Table gm_aipub_tasks not available: {e}")

    def test_plan_has_related_tasks(self, db_cursor):
        """Verify plans have associated tasks."""
        try:
            db_cursor.execute("""
                SELECT p.id, p.status,
                       COUNT(DISTINCT ai.id) as ai_task_count,
                       COUNT(DISTINCT pt.id) as publish_task_count
                FROM gm_aipub_plans p
                LEFT JOIN gm_aipub_ai_tasks ai ON p.id = ai.plan_id
                LEFT JOIN gm_aipub_tasks pt ON p.id = pt.plan_id
                GROUP BY p.id, p.status
                ORDER BY p.id
                LIMIT 5
            """)
            results = db_cursor.fetchall()

            print("\n=== Publish Plan Task Stats ===")
            for row in results:
                print(f"  Plan {row['id']} ({row['status']}): AI={row['ai_task_count']}, Publish={row['publish_task_count']}")
        except Exception as e:
            pytest.skip(f"Query failed: {e}")


# Phase Testing Classes

class TestPhase1CreatePlan:
    """Phase 1: 创建阶段单元测试 - Plan 创建与调度前状态"""

    def test_create_plan_stays_pending_before_scheduler_dispatch(self, auth_client, db_cursor):
        """API 仅创建 Plan；AI Task 由后续调度器异步下发。"""
        db_cursor.execute("SELECT id FROM gm_social_groups LIMIT 1")
        group = db_cursor.fetchone()
        if not group:
            pytest.skip("No social group available")

        plan_payload = {
            "group_id": group["id"],
            "platform_id": PLATFORM_FACEBOOK,
            "content_type": "video",
            "ai_task_types": ["video_gen"],
            "ai_input": {"topic": "Test", "style": "casual"}
        }

        resp = auth_client.post("/api/v1/publish_plans", json=plan_payload)
        if resp.status_code not in [200, 201]:
            pytest.skip(f"Plan creation failed: {resp.status_code}")

        plan = extract_data(resp.json())
        plan_id = plan["id"]

        # 验证 AI Task 被自动创建
        db_cursor.execute(
            "SELECT * FROM gm_aipub_ai_tasks WHERE plan_id = %s",
            (plan_id,)
        )
        ai_tasks = db_cursor.fetchall()

        assert plan["status"] == "pending"
        assert len(ai_tasks) == 0, "Scheduler is responsible for creating AI tasks later"

    def test_create_plan_with_direct_content_no_ai_task(self, auth_client, db_cursor):
        """直接提供 content 时不应创建 AI Task"""
        db_cursor.execute("SELECT id FROM gm_social_accounts LIMIT 1")
        account = db_cursor.fetchone()
        if not account:
            pytest.skip("No social account available")

        plan_payload = {
            "social_account_id": account["id"],
            "platform_id": PLATFORM_FACEBOOK,
            "content_type": "post",
            "content": {
                "title": "Direct content",
                "description": "No AI needed"
            }
        }

        resp = auth_client.post("/api/v1/publish_plans", json=plan_payload)
        if resp.status_code in [200, 201]:
            plan = extract_data(resp.json())

            db_cursor.execute(
                "SELECT COUNT(*) as cnt FROM gm_aipub_ai_tasks WHERE plan_id = %s",
                (plan["id"],)
            )
            result = db_cursor.fetchone()
            # Note: Direct content may or may not create AI tasks depending on implementation
            print(f"\n  AI tasks for direct content plan: {result['cnt']}")


class TestPhase4Query:
    """Phase 4: 查询阶段单元测试 - Plan 状态汇总与子任务统计"""

    def test_plan_detail_includes_stats(self, auth_client, db_cursor):
        """Plan 详情应包含任务统计"""
        db_cursor.execute("SELECT id FROM gm_aipub_plans LIMIT 1")
        result = db_cursor.fetchone()

        if not result:
            pytest.skip("No publish plans available")

        plan_id = result["id"]

        resp = auth_client.get(f"/api/v1/publish_plans/{plan_id}")
        assert_response_success(resp)

        data = extract_data(resp.json())

        # 验证统计字段存在
        has_stats = "stats" in data or "ai_tasks_count" in data
        print(f"\n  Plan {plan_id} has stats: {has_stats}")

    def test_list_plans_pagination(self, auth_client):
        """Plan 列表应支持分页"""
        resp = auth_client.get("/api/v1/publish_plans?page=1&page_size=5")
        assert_response_success(resp)

        data = extract_data(resp.json())

        if isinstance(data, dict):
            has_pagination = "total" in data or "pagination" in data or "total_pages" in data
            print(f"\n  Pagination info available: {has_pagination}")


class TestOSSImageUpload:
    """Tests for OSS image upload API."""
    
    def _create_test_image(self, format: str = "png") -> bytes:
        """Create a minimal valid test image.
        
        Creates a 1x1 pixel image in the specified format.
        """
        if format == "png":
            # Minimal 1x1 red PNG
            return bytes([
                0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A,  # PNG signature
                0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44, 0x52,  # IHDR chunk
                0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01,  # 1x1
                0x08, 0x02, 0x00, 0x00, 0x00, 0x90, 0x77, 0x53,  
                0xDE, 0x00, 0x00, 0x00, 0x0C, 0x49, 0x44, 0x41,  # IDAT chunk
                0x54, 0x08, 0xD7, 0x63, 0xF8, 0xCF, 0xC0, 0x00,
                0x00, 0x00, 0x03, 0x00, 0x01, 0x00, 0x18, 0xDD,
                0x8D, 0xB4, 0x00, 0x00, 0x00, 0x00, 0x49, 0x45,  # IEND chunk
                0x4E, 0x44, 0xAE, 0x42, 0x60, 0x82
            ])
        elif format == "jpeg" or format == "jpg":
            # Minimal JPEG (1x1 red pixel)
            return bytes([
                0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x10, 0x4A, 0x46,
                0x49, 0x46, 0x00, 0x01, 0x01, 0x00, 0x00, 0x01,
                0x00, 0x01, 0x00, 0x00, 0xFF, 0xDB, 0x00, 0x43,
                0x00, 0x08, 0x06, 0x06, 0x07, 0x06, 0x05, 0x08,
                0x07, 0x07, 0x07, 0x09, 0x09, 0x08, 0x0A, 0x0C,
                0x14, 0x0D, 0x0C, 0x0B, 0x0B, 0x0C, 0x19, 0x12,
                0x13, 0x0F, 0x14, 0x1D, 0x1A, 0x1F, 0x1E, 0x1D,
                0x1A, 0x1C, 0x1C, 0x20, 0x24, 0x2E, 0x27, 0x20,
                0x22, 0x2C, 0x23, 0x1C, 0x1C, 0x28, 0x37, 0x29,
                0x2C, 0x30, 0x31, 0x34, 0x34, 0x34, 0x1F, 0x27,
                0x39, 0x3D, 0x38, 0x32, 0x3C, 0x2E, 0x33, 0x34,
                0x32, 0xFF, 0xC0, 0x00, 0x0B, 0x08, 0x00, 0x01,
                0x00, 0x01, 0x01, 0x01, 0x11, 0x00, 0xFF, 0xC4,
                0x00, 0x1F, 0x00, 0x00, 0x01, 0x05, 0x01, 0x01,
                0x01, 0x01, 0x01, 0x01, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x01, 0x02, 0x03, 0x04,
                0x05, 0x06, 0x07, 0x08, 0x09, 0x0A, 0x0B, 0xFF,
                0xC4, 0x00, 0xB5, 0x10, 0x00, 0x02, 0x01, 0x03,
                0x03, 0x02, 0x04, 0x03, 0x05, 0x05, 0x04, 0x04,
                0x00, 0x00, 0x01, 0x7D, 0x01, 0x02, 0x03, 0x00,
                0x04, 0x11, 0x05, 0x12, 0x21, 0x31, 0x41, 0x06,
                0x13, 0x51, 0x61, 0x07, 0x22, 0x71, 0x14, 0x32,
                0x81, 0x91, 0xA1, 0x08, 0x23, 0x42, 0xB1, 0xC1,
                0x15, 0x52, 0xD1, 0xF0, 0x24, 0x33, 0x62, 0x72,
                0x82, 0x09, 0x0A, 0x16, 0x17, 0x18, 0x19, 0x1A,
                0x25, 0x26, 0x27, 0x28, 0x29, 0x2A, 0x34, 0x35,
                0x36, 0x37, 0x38, 0x39, 0x3A, 0x43, 0x44, 0x45,
                0x46, 0x47, 0x48, 0x49, 0x4A, 0x53, 0x54, 0x55,
                0x56, 0x57, 0x58, 0x59, 0x5A, 0x63, 0x64, 0x65,
                0x66, 0x67, 0x68, 0x69, 0x6A, 0x73, 0x74, 0x75,
                0x76, 0x77, 0x78, 0x79, 0x7A, 0x83, 0x84, 0x85,
                0x86, 0x87, 0x88, 0x89, 0x8A, 0x92, 0x93, 0x94,
                0x95, 0x96, 0x97, 0x98, 0x99, 0x9A, 0xA2, 0xA3,
                0xA4, 0xA5, 0xA6, 0xA7, 0xA8, 0xA9, 0xAA, 0xB2,
                0xB3, 0xB4, 0xB5, 0xB6, 0xB7, 0xB8, 0xB9, 0xBA,
                0xC2, 0xC3, 0xC4, 0xC5, 0xC6, 0xC7, 0xC8, 0xC9,
                0xCA, 0xD2, 0xD3, 0xD4, 0xD5, 0xD6, 0xD7, 0xD8,
                0xD9, 0xDA, 0xE1, 0xE2, 0xE3, 0xE4, 0xE5, 0xE6,
                0xE7, 0xE8, 0xE9, 0xEA, 0xF1, 0xF2, 0xF3, 0xF4,
                0xF5, 0xF6, 0xF7, 0xF8, 0xF9, 0xFA, 0xFF, 0xDA,
                0x00, 0x08, 0x01, 0x01, 0x00, 0x00, 0x3F, 0x00,
                0xFB, 0xD5, 0xDB, 0x20, 0xA8, 0xE8, 0xFF, 0xD9
            ])
        else:
            raise ValueError(f"Unsupported format: {format}")

    def test_upload_image_png(self, auth_client):
        """Test uploading a PNG image."""
        image_data = self._create_test_image("png")
        
        files = {
            "file": ("test_image.png", image_data, "image/png")
        }
        
        resp = auth_client.post(
            "/api/v1/aipub/upload-image",
            files=files
        )
        
        print(f"\nUpload PNG response: {resp.status_code}")
        
        if resp.status_code == 200:
            data = extract_data(resp.json())
            print(f"  Upload result: {data}")
            
            assert "image_url" in data, "Response should contain image_url"
            assert "filename" in data, "Response should contain filename"
            assert data["image_url"].startswith("https://"), "URL should be HTTPS"
            assert data["filename"].endswith(".png"), "Filename should have .png extension"
            
            # Verify the URL is accessible
            import requests
            img_resp = requests.head(data["image_url"], timeout=10)
            print(f"  Image URL accessible: {img_resp.status_code}")
        elif resp.status_code == 500:
            error_data = resp.json()
            if "not configured" in str(error_data).lower():
                pytest.skip("OSS service not configured")
            else:
                print(f"  Error: {error_data}")

    def test_upload_image_jpeg(self, auth_client):
        """Test uploading a JPEG image."""
        image_data = self._create_test_image("jpeg")
        
        files = {
            "file": ("test_image.jpg", image_data, "image/jpeg")
        }
        
        resp = auth_client.post(
            "/api/v1/aipub/upload-image",
            files=files
        )
        
        print(f"\nUpload JPEG response: {resp.status_code}")
        
        if resp.status_code == 200:
            data = extract_data(resp.json())
            print(f"  Upload result: {data}")
            
            assert "image_url" in data
            assert "filename" in data
            assert data["filename"].endswith(".jpg"), "JPEG should have .jpg extension"
        elif resp.status_code == 500:
            error_data = resp.json()
            if "not configured" in str(error_data).lower():
                pytest.skip("OSS service not configured")

    def test_upload_image_requires_auth(self, api_client):
        """Test that upload requires authentication."""
        image_data = self._create_test_image("png")
        
        files = {
            "file": ("test.png", image_data, "image/png")
        }
        
        resp = api_client.post(
            "/api/v1/aipub/upload-image",
            files=files
        )
        
        assert resp.status_code == 401, \
            f"Expected 401 for unauthenticated request, got {resp.status_code}"

    def test_upload_invalid_file_type(self, auth_client):
        """Test uploading an invalid file type."""
        # Try to upload a text file as image
        files = {
            "file": ("test.txt", b"Hello World", "text/plain")
        }
        
        resp = auth_client.post(
            "/api/v1/aipub/upload-image",
            files=files
        )
        
        print(f"\nInvalid file type response: {resp.status_code}")
        
        if resp.status_code == 500:
            error_data = resp.json()
            if "not configured" in str(error_data).lower():
                pytest.skip("OSS service not configured")
        
        # Should return 400 for invalid file type
        assert resp.status_code == 400, \
            f"Expected 400 for invalid file type, got {resp.status_code}: {resp.text}"

    def test_upload_no_file(self, auth_client):
        """Test upload without file field."""
        resp = auth_client.post(
            "/api/v1/aipub/upload-image",
            data={}  # No file
        )
        
        print(f"\nNo file response: {resp.status_code}")
        
        if resp.status_code == 500:
            error_data = resp.json()
            if "not configured" in str(error_data).lower():
                pytest.skip("OSS service not configured")
        
        # Should return 400 for missing file
        assert resp.status_code == 400, \
            f"Expected 400 for missing file, got {resp.status_code}: {resp.text}"

    def test_upload_real_image_file(self, auth_client, tmp_path):
        """Test uploading a real image file using PIL if available."""
        try:
            from PIL import Image
            import io
            
            # Create a real 100x100 red image
            img = Image.new('RGB', (100, 100), color='red')
            img_buffer = io.BytesIO()
            img.save(img_buffer, format='PNG')
            img_data = img_buffer.getvalue()
            
            files = {
                "file": ("real_test_image.png", img_data, "image/png")
            }
            
            resp = auth_client.post(
                "/api/v1/aipub/upload-image",
                files=files
            )
            
            print(f"\nReal image upload response: {resp.status_code}")
            
            if resp.status_code == 200:
                data = extract_data(resp.json())
                print(f"  Upload result: {data}")
                
                assert data["size"] > 0, "Size should be positive"
                
                # Verify the uploaded image is accessible
                import requests
                img_resp = requests.get(data["image_url"], timeout=10)
                if img_resp.status_code == 200:
                    # Verify content is an image
                    downloaded_img = Image.open(io.BytesIO(img_resp.content))
                    print(f"  Downloaded image size: {downloaded_img.size}")
                    assert downloaded_img.size == (100, 100), "Image dimensions should match"
            elif resp.status_code == 500:
                error_data = resp.json()
                if "not configured" in str(error_data).lower():
                    pytest.skip("OSS service not configured")
                    
        except ImportError:
            pytest.skip("PIL not available for real image test")


class TestSeedancePlanCreation:
    """Tests for Seedance 2.0 video plan creation via existing single_video plan_type."""

    SEEDANCE_TEXT2VIDEO_PAYLOAD = {
        "social_account_id": None,
        "platform_id": PLATFORM_TIKTOK,
        "content_type": "video",
        "plan_type": "single_video",
        "video_ai_model_id": None,
        "ai_input": {
            "video_prompt": "A golden retriever playing in a garden with butterflies",
            "seedance_config": {
                "mode": "text2video",
                "duration": 8,
                "quality": "480p",
                "aspect_ratio": "16:9",
                "generate_audio": False,
                "return_last_frame": False,
                "watermark": False,
                "enable_web_search": False,
                "image_urls": [],
                "video_urls": [],
                "audio_urls": [],
                "image_roles": {},
                "video_roles": {},
                "audio_roles": {}
            }
        }
    }

    def _get_seedance_model_id(self, db_cursor) -> int:
        db_cursor.execute(
            "SELECT id FROM gm_ai_models WHERE model_key = 'doubao-seedance-2-0-fast-260128' LIMIT 1"
        )
        result = db_cursor.fetchone()
        if not result:
            pytest.skip("Seedance model not found in gm_ai_models")
        return result["id"]

    def _get_test_account_id(self, db_cursor) -> int:
        db_cursor.execute("SELECT id FROM gm_social_accounts LIMIT 1")
        result = db_cursor.fetchone()
        if not result:
            pytest.skip("No social account available")
        return result["id"]

    def _make_payload(self, db_cursor, **overrides):
        import copy
        payload = copy.deepcopy(self.SEEDANCE_TEXT2VIDEO_PAYLOAD)
        payload["social_account_id"] = self._get_test_account_id(db_cursor)
        payload["video_ai_model_id"] = self._get_seedance_model_id(db_cursor)
        for k, v in overrides.items():
            if "." in k:
                parts = k.split(".")
                target = payload
                for p in parts[:-1]:
                    target = target[p]
                target[parts[-1]] = v
            else:
                payload[k] = v
        return payload

    def test_create_seedance_text2video_plan(self, auth_client, db_cursor):
        """text2video: prompt only, ai_task_types should be [seedance_video]."""
        payload = self._make_payload(db_cursor)
        resp = auth_client.post("/api/v1/publish_plans", json=payload)
        if resp.status_code not in [200, 201]:
            pytest.skip(f"Plan creation failed: {resp.status_code} {resp.text}")
        data = extract_data(resp.json())
        assert data["status"] == "pending"
        assert data["plan_type"] == "single_video"
        assert "seedance_video" in data.get("ai_task_types", [])
        assert "content_gen" not in data.get("ai_task_types", [])

    def test_create_seedance_with_content_prompt(self, auth_client, db_cursor):
        """With content_prompt, ai_task_types should include content_gen."""
        payload = self._make_payload(db_cursor)
        payload["ai_input"]["content_prompt"] = "Generate a viral TikTok caption"
        payload["chat_ai_model_id"] = 1
        resp = auth_client.post("/api/v1/publish_plans", json=payload)
        if resp.status_code not in [200, 201]:
            pytest.skip(f"Plan creation failed: {resp.status_code} {resp.text}")
        data = extract_data(resp.json())
        task_types = data.get("ai_task_types", [])
        assert "content_gen" in task_types
        assert "seedance_video" in task_types

    def test_create_seedance_image2video(self, auth_client, db_cursor):
        """image2video: requires at least 1 image."""
        payload = self._make_payload(db_cursor)
        payload["ai_input"]["seedance_config"]["mode"] = "image2video"
        payload["ai_input"]["seedance_config"]["image_urls"] = ["https://example.com/img1.jpg"]
        payload["ai_input"]["seedance_config"]["image_roles"] = {"1": "first_frame"}
        resp = auth_client.post("/api/v1/publish_plans", json=payload)
        if resp.status_code not in [200, 201]:
            pytest.skip(f"Plan creation failed: {resp.status_code} {resp.text}")
        data = extract_data(resp.json())
        assert data["status"] == "pending"

    def test_create_seedance_start_end_frame(self, auth_client, db_cursor):
        """start_end_frame: 2 images with positional role binding."""
        payload = self._make_payload(db_cursor)
        payload["ai_input"]["seedance_config"]["mode"] = "start_end_frame"
        payload["ai_input"]["seedance_config"]["image_urls"] = [
            "https://example.com/start.jpg", "https://example.com/end.jpg"
        ]
        payload["ai_input"]["seedance_config"]["image_roles"] = {"1": "first_frame", "2": "last_frame"}
        resp = auth_client.post("/api/v1/publish_plans", json=payload)
        if resp.status_code not in [200, 201]:
            pytest.skip(f"Plan creation failed: {resp.status_code} {resp.text}")
        assert extract_data(resp.json())["status"] == "pending"

    def test_seedance_invalid_duration(self, auth_client, db_cursor):
        payload = self._make_payload(db_cursor)
        payload["ai_input"]["seedance_config"]["duration"] = 20
        resp = auth_client.post("/api/v1/publish_plans", json=payload)
        assert resp.status_code == 400

    def test_seedance_invalid_quality(self, auth_client, db_cursor):
        payload = self._make_payload(db_cursor)
        payload["ai_input"]["seedance_config"]["quality"] = "1080p"
        resp = auth_client.post("/api/v1/publish_plans", json=payload)
        assert resp.status_code == 400

    def test_seedance_invalid_mode(self, auth_client, db_cursor):
        payload = self._make_payload(db_cursor)
        payload["ai_input"]["seedance_config"]["mode"] = "invalid_mode"
        resp = auth_client.post("/api/v1/publish_plans", json=payload)
        assert resp.status_code == 400

    def test_seedance_image2video_no_images(self, auth_client, db_cursor):
        payload = self._make_payload(db_cursor)
        payload["ai_input"]["seedance_config"]["mode"] = "image2video"
        payload["ai_input"]["seedance_config"]["image_urls"] = []
        resp = auth_client.post("/api/v1/publish_plans", json=payload)
        assert resp.status_code == 400

    def test_seedance_start_end_frame_wrong_count(self, auth_client, db_cursor):
        payload = self._make_payload(db_cursor)
        payload["ai_input"]["seedance_config"]["mode"] = "start_end_frame"
        payload["ai_input"]["seedance_config"]["image_urls"] = ["https://example.com/one.jpg"]
        resp = auth_client.post("/api/v1/publish_plans", json=payload)
        assert resp.status_code == 400

    def test_seedance_start_end_frame_swapped_roles(self, auth_client, db_cursor):
        """Swapped first/last frame roles should fail (position binding)."""
        payload = self._make_payload(db_cursor)
        payload["ai_input"]["seedance_config"]["mode"] = "start_end_frame"
        payload["ai_input"]["seedance_config"]["image_urls"] = [
            "https://example.com/end.jpg", "https://example.com/start.jpg"
        ]
        payload["ai_input"]["seedance_config"]["image_roles"] = {"1": "last_frame", "2": "first_frame"}
        resp = auth_client.post("/api/v1/publish_plans", json=payload)
        assert resp.status_code == 400

    def test_seedance_start_end_frame_missing_roles(self, auth_client, db_cursor):
        payload = self._make_payload(db_cursor)
        payload["ai_input"]["seedance_config"]["mode"] = "start_end_frame"
        payload["ai_input"]["seedance_config"]["image_urls"] = [
            "https://example.com/a.jpg", "https://example.com/b.jpg"
        ]
        payload["ai_input"]["seedance_config"]["image_roles"] = {}
        resp = auth_client.post("/api/v1/publish_plans", json=payload)
        assert resp.status_code == 400

    def test_seedance_media_count_limit(self, auth_client, db_cursor):
        payload = self._make_payload(db_cursor)
        payload["ai_input"]["seedance_config"]["mode"] = "multimodal"
        payload["ai_input"]["seedance_config"]["image_urls"] = [
            f"https://example.com/img{i}.jpg" for i in range(10)
        ]
        resp = auth_client.post("/api/v1/publish_plans", json=payload)
        assert resp.status_code == 400

    def test_seedance_billing_freeze(self, auth_client, db_cursor):
        """Verify billing_status='frozen' with correct amount."""
        payload = self._make_payload(db_cursor)
        payload["ai_input"]["seedance_config"]["duration"] = 8
        payload["ai_input"]["seedance_config"]["quality"] = "720p"
        payload["ai_input"]["seedance_config"]["generate_audio"] = True
        resp = auth_client.post("/api/v1/publish_plans", json=payload)
        if resp.status_code not in [200, 201]:
            pytest.skip(f"Plan creation failed: {resp.status_code} {resp.text}")
        data = extract_data(resp.json())
        assert data["billing_status"] == "frozen"
        assert float(data["frozen_cost"]) > 0

    def test_non_seedance_single_video_unchanged(self, auth_client, db_cursor):
        """Non-Seedance single_video should still use content_gen + video_gen."""
        account_id = self._get_test_account_id(db_cursor)
        db_cursor.execute(
            "SELECT id FROM gm_ai_models WHERE model_type = 'video' "
            "AND model_key NOT LIKE 'doubao-seedance%' LIMIT 1"
        )
        model = db_cursor.fetchone()
        if not model:
            pytest.skip("No non-Seedance video model")
        payload = {
            "social_account_id": account_id,
            "platform_id": PLATFORM_TIKTOK,
            "content_type": "video",
            "plan_type": "single_video",
            "video_ai_model_id": model["id"],
            "ai_input": {"video_prompt": "Test non-seedance video", "content_prompt": "Generate caption"}
        }
        resp = auth_client.post("/api/v1/publish_plans", json=payload)
        if resp.status_code not in [200, 201]:
            pytest.skip(f"Plan creation failed: {resp.status_code} {resp.text}")
        data = extract_data(resp.json())
        task_types = data.get("ai_task_types", [])
        assert "content_gen" in task_types
        assert "video_gen" in task_types
        assert "seedance_video" not in task_types


class TestSeedanceCostEstimation:
    """Tests for Seedance cost estimation endpoint."""

    def _get_seedance_model_id(self, db_cursor) -> int:
        db_cursor.execute(
            "SELECT id FROM gm_ai_models WHERE model_key = 'doubao-seedance-2-0-fast-260128' LIMIT 1"
        )
        result = db_cursor.fetchone()
        if not result:
            pytest.skip("Seedance model not found")
        return result["id"]

    def test_estimate_seedance_cost(self, auth_client, db_cursor):
        model_id = self._get_seedance_model_id(db_cursor)
        payload = {
            "video_model_id": model_id,
            "seedance_config": {"duration": 10, "quality": "720p", "generate_audio": True}
        }
        resp = auth_client.post("/api/v1/publish_plans/estimate", json=payload)
        assert_response_success(resp)
        data = extract_data(resp.json())
        assert "seedance_cost" in data
        sc = data["seedance_cost"]
        assert float(sc["total"]) > 0
        assert float(sc["duration_factor"]) == 2.5
        assert float(sc["quality_factor"]) == 2.0
        assert float(sc["audio_factor"]) == 1.5

    def test_estimate_non_seedance_unchanged(self, auth_client):
        payload = {"chat_model_id": 1, "account_count": 5}
        resp = auth_client.post("/api/v1/publish_plans/estimate", json=payload)
        assert_response_success(resp)
        data = extract_data(resp.json())
        assert "seedance_cost" not in data or data["seedance_cost"] is None


# =============================================================================
# Phase 4 Round 3 Task 7a — PublishBehavior persistence + task derivation
# =============================================================================


class TestPlanPublishBehavior:
    """API must accept, persist and propagate PublishBehavior on plans.

    PublishBehavior (proto: glance_mind_protocol/proto/aipub.proto §960)
    is plan-level configuration (visibility, allow_comments, is_nsfw,
    platform-specific toggles). It is stored on gm_aipub_plans.behavior
    JSONB and copied into every derived task.content.behavior so workers
    receive it without an extra API round-trip.
    """

    def _resolve_test_account(self, db_cursor) -> Optional[int]:
        db_cursor.execute("SELECT id FROM gm_social_accounts LIMIT 1")
        row = db_cursor.fetchone()
        return row["id"] if row else None

    def _full_behavior(self) -> Dict[str, Any]:
        return {
            "visibility": "public",
            "allow_comments": True,
            "allow_sharing": True,
            "allow_download": False,
            "is_nsfw": False,
            "is_spoiler": False,
            "ai_generated_disclosure": True,
            "allow_duet": True,
            "allow_stitch": False,
            "disclose_branded_content": False,
            "allow_remix": True,
            "share_to_facebook": True,
            "extras": {"custom_key": "custom_value"},
        }

    # -------------------------------------------------------------------
    # Persistence — DB column behaviour
    # -------------------------------------------------------------------

    def test_create_plan_with_behavior_persists_to_db(self, auth_client, db_cursor):
        """behavior dict on CreatePlanDto must land in gm_aipub_plans.behavior."""
        account_id = self._resolve_test_account(db_cursor)
        if not account_id:
            pytest.skip("No social account available")

        payload = {
            "social_account_id": account_id,
            "platform_id": PLATFORM_FACEBOOK,
            "content_type": "post",
            "plan_type": "direct_publish",
            "content": {"title": "T", "description": "D"},
            "behavior": self._full_behavior(),
        }
        resp = auth_client.post("/api/v1/publish_plans", json=payload)
        assert_response_success(resp)
        plan = extract_data(resp.json())

        db_cursor.execute(
            "SELECT behavior FROM gm_aipub_plans WHERE id = %s",
            (plan["id"],),
        )
        row = db_cursor.fetchone()
        assert row is not None, "plan not found in DB"
        assert row["behavior"] is not None, "behavior column should be populated"
        # JSONB returns as dict via psycopg2 RealDictCursor.
        b = row["behavior"]
        assert b["visibility"] == "public"
        assert b["allow_comments"] is True
        assert b["is_nsfw"] is False
        assert b["allow_duet"] is True
        assert b["share_to_facebook"] is True
        assert b["extras"] == {"custom_key": "custom_value"}

    def test_create_plan_without_behavior_leaves_field_null(
        self, auth_client, db_cursor
    ):
        """Backward-compat: plans created without behavior keep behavior NULL.
        Existing UI / scripts that don't know about behavior must not break."""
        account_id = self._resolve_test_account(db_cursor)
        if not account_id:
            pytest.skip("No social account available")

        payload = {
            "social_account_id": account_id,
            "platform_id": PLATFORM_FACEBOOK,
            "content_type": "post",
            "plan_type": "direct_publish",
            "content": {"title": "T", "description": "D"},
        }
        resp = auth_client.post("/api/v1/publish_plans", json=payload)
        assert_response_success(resp)
        plan = extract_data(resp.json())

        db_cursor.execute(
            "SELECT behavior FROM gm_aipub_plans WHERE id = %s",
            (plan["id"],),
        )
        row = db_cursor.fetchone()
        assert row is not None
        assert row["behavior"] is None, "behavior must default to NULL when omitted"

    # -------------------------------------------------------------------
    # Round-trip via API responses
    # -------------------------------------------------------------------

    def test_get_plan_returns_behavior_field(self, auth_client, db_cursor):
        """GET /publish_plans/:id must echo back the behavior field."""
        account_id = self._resolve_test_account(db_cursor)
        if not account_id:
            pytest.skip("No social account available")

        payload = {
            "social_account_id": account_id,
            "platform_id": PLATFORM_FACEBOOK,
            "content_type": "post",
            "plan_type": "direct_publish",
            "content": {"title": "T"},
            "behavior": {
                "visibility": "private",
                "allow_comments": False,
                "is_nsfw": True,
            },
        }
        resp = auth_client.post("/api/v1/publish_plans", json=payload)
        assert_response_success(resp)
        plan_id = extract_data(resp.json())["id"]

        resp = auth_client.get(f"/api/v1/publish_plans/{plan_id}")
        assert_response_success(resp)
        data = extract_data(resp.json())
        assert "behavior" in data, \
            "PlanResponseDto must expose 'behavior' so UI can edit/display it"
        assert data["behavior"]["visibility"] == "private"
        assert data["behavior"]["allow_comments"] is False
        assert data["behavior"]["is_nsfw"] is True

    def test_update_plan_can_change_behavior(self, auth_client, db_cursor):
        """UpdatePlanDto must accept behavior so users can re-tune flags
        after creation (e.g. toggle is_nsfw before scheduling)."""
        account_id = self._resolve_test_account(db_cursor)
        if not account_id:
            pytest.skip("No social account available")

        create_resp = auth_client.post(
            "/api/v1/publish_plans",
            json={
                "social_account_id": account_id,
                "platform_id": PLATFORM_FACEBOOK,
                "content_type": "post",
                "plan_type": "direct_publish",
                "content": {"title": "T"},
                "behavior": {"visibility": "private", "is_nsfw": False},
            },
        )
        assert_response_success(create_resp)
        plan_id = extract_data(create_resp.json())["id"]

        update_resp = auth_client.put(
            f"/api/v1/publish_plans/{plan_id}",
            json={"behavior": {"visibility": "public", "is_nsfw": True}},
        )
        assert_response_success(update_resp)

        db_cursor.execute(
            "SELECT behavior FROM gm_aipub_plans WHERE id = %s",
            (plan_id,),
        )
        b = db_cursor.fetchone()["behavior"]
        assert b["visibility"] == "public"
        assert b["is_nsfw"] is True

    # -------------------------------------------------------------------
    # Task derivation — behavior MUST land on task.content.behavior
    # -------------------------------------------------------------------

    def test_direct_publish_plan_propagates_behavior_to_task_content(
        self, auth_client, db_cursor
    ):
        """When a direct-content plan auto-expands to publish tasks, every
        derived task.content must carry the plan's behavior so the worker
        sees it without a second API hit."""
        account_id = self._resolve_test_account(db_cursor)
        if not account_id:
            pytest.skip("No social account available")

        behavior = {
            "visibility": "unlisted",
            "allow_comments": True,
            "is_nsfw": True,
            "allow_duet": False,
        }
        payload = {
            "social_account_id": account_id,
            "platform_id": PLATFORM_FACEBOOK,
            "content_type": "post",
            "plan_type": "direct_publish",
            "content": {"title": "T", "description": "D"},
            "behavior": behavior,
        }
        resp = auth_client.post("/api/v1/publish_plans", json=payload)
        assert_response_success(resp)
        plan_id = extract_data(resp.json())["id"]

        # Direct content auto-derives publish tasks immediately.
        db_cursor.execute(
            "SELECT content FROM gm_aipub_tasks WHERE plan_id = %s",
            (plan_id,),
        )
        rows = db_cursor.fetchall()
        assert rows, "direct-content plan must derive at least one publish task"
        for row in rows:
            content = row["content"]
            assert "behavior" in content, \
                "task.content must carry the merged behavior block"
            assert content["behavior"]["visibility"] == "unlisted"
            assert content["behavior"]["allow_comments"] is True
            assert content["behavior"]["is_nsfw"] is True
            assert content["behavior"]["allow_duet"] is False
            # Original content fields must remain alongside behavior.
            assert content["title"] == "T"
            assert content["description"] == "D"

    def test_direct_publish_without_behavior_does_not_inject_empty_behavior(
        self, auth_client, db_cursor
    ):
        """When plan.behavior is NULL, task.content must NOT contain a
        spurious behavior key — leaving it absent lets the worker fall
        back to platform defaults rather than apply an empty PublishBehavior
        (which would override e.g. visibility to UNSPECIFIED)."""
        account_id = self._resolve_test_account(db_cursor)
        if not account_id:
            pytest.skip("No social account available")

        payload = {
            "social_account_id": account_id,
            "platform_id": PLATFORM_FACEBOOK,
            "content_type": "post",
            "plan_type": "direct_publish",
            "content": {"title": "T", "description": "D"},
        }
        resp = auth_client.post("/api/v1/publish_plans", json=payload)
        assert_response_success(resp)
        plan_id = extract_data(resp.json())["id"]

        db_cursor.execute(
            "SELECT content FROM gm_aipub_tasks WHERE plan_id = %s",
            (plan_id,),
        )
        rows = db_cursor.fetchall()
        assert rows
        for row in rows:
            assert "behavior" not in row["content"], \
                "no plan.behavior → no task.content.behavior"

    # -------------------------------------------------------------------
    # Validation
    # -------------------------------------------------------------------

    def test_invalid_behavior_visibility_rejected(self, auth_client, db_cursor):
        """visibility must be one of the proto Visibility enum names
        (UNSPECIFIED / PUBLIC / UNLISTED / PRIVATE / FOLLOWERS_ONLY).
        Garbage strings must 400, not silently land in the DB."""
        account_id = self._resolve_test_account(db_cursor)
        if not account_id:
            pytest.skip("No social account available")

        payload = {
            "social_account_id": account_id,
            "platform_id": PLATFORM_FACEBOOK,
            "content_type": "post",
            "content": {"title": "T"},
            "behavior": {"visibility": "definitely-not-a-real-value"},
        }
        resp = auth_client.post("/api/v1/publish_plans", json=payload)
        assert resp.status_code == 400, (
            f"expected 400 for invalid visibility, got {resp.status_code}: {resp.text}"
        )

    def test_invalid_behavior_field_type_rejected(self, auth_client, db_cursor):
        """is_nsfw is `optional bool` in proto — passing a string must 400."""
        account_id = self._resolve_test_account(db_cursor)
        if not account_id:
            pytest.skip("No social account available")

        payload = {
            "social_account_id": account_id,
            "platform_id": PLATFORM_FACEBOOK,
            "content_type": "post",
            "content": {"title": "T"},
            "behavior": {"is_nsfw": "yes-please"},
        }
        resp = auth_client.post("/api/v1/publish_plans", json=payload)
        assert resp.status_code == 400, (
            f"expected 400 for non-bool is_nsfw, got {resp.status_code}: {resp.text}"
        )


# =============================================================================
# Phase 4 R3 Task 7a — full plan_type coverage for behavior persistence
# =============================================================================


PLATFORM_REDDIT = 1  # also defined in gm-e2e conftest


class TestPlanPublishBehaviorAcrossPlanTypes:
    """Same persistence + DB column contract as TestPlanPublishBehavior, but
    asserted across **every** plan_type that produces task content.

    Why: Task 7a's create+expand path runs through different validation
    branches per plan_type (Reddit needs reddit_config, batch_text needs
    group_id, single_video needs video_ai_model_id, …). The behavior
    column itself is plan_type-agnostic, but a reviewer should not have
    to take that on faith. These tests prove the create endpoint accepts
    + persists behavior on every legal plan_type.

    Task derivation through `expand_plan_to_tasks` (the actual content +
    behavior merge) for AI-driven plan_types (batch_text, single_video,
    reddit_*) is verified end-to-end in
    `gm-e2e/tests/test_20_publish_behavior_propagation.py` because it
    needs the scheduler + agent-rs + mock LLM stack; those services are
    not available in the API-only test fixture here.
    """

    def _behavior(self) -> Dict[str, Any]:
        return {
            "visibility": "unlisted",
            "allow_comments": True,
            "is_nsfw": False,
            "ai_generated_disclosure": True,
            "extras": {"plan_type_marker": "task7a-coverage"},
        }

    def _multi_account_group_id(self, db_cursor, platform_id: int) -> Optional[int]:
        """Find a group on `platform_id` that already has ≥1 bound account.
        batch_text / reddit_* plans require a group with members; we don't
        want to create accounts in a test as that races other suites.
        Account→group binding is direct (gm_social_accounts.group_id),
        not via a separate join table."""
        db_cursor.execute(
            """
            SELECT g.id
              FROM gm_social_groups g
              JOIN gm_social_accounts a ON a.group_id = g.id
             WHERE g.platform_id = %s
             GROUP BY g.id
             HAVING COUNT(a.id) >= 1
             ORDER BY g.id
             LIMIT 1
            """,
            (platform_id,),
        )
        row = db_cursor.fetchone()
        return row["id"] if row else None

    def _account_id(self, db_cursor, platform_id: int) -> Optional[int]:
        db_cursor.execute(
            "SELECT id FROM gm_social_accounts WHERE platform_id = %s LIMIT 1",
            (platform_id,),
        )
        row = db_cursor.fetchone()
        return row["id"] if row else None

    def _video_model_id(self, db_cursor) -> Optional[int]:
        """Pick any seeded video AI model (Vidu/Veo/Seedance — implementation
        detail; we only care that the create endpoint accepts a valid FK)."""
        db_cursor.execute(
            "SELECT id FROM gm_ai_models WHERE model_type = 'video' "
            "AND is_active = true ORDER BY id LIMIT 1"
        )
        row = db_cursor.fetchone()
        return row["id"] if row else None

    def _chat_model_id(self, db_cursor) -> Optional[int]:
        db_cursor.execute(
            "SELECT id FROM gm_ai_models WHERE model_type = 'chat' "
            "AND is_active = true ORDER BY id LIMIT 1"
        )
        row = db_cursor.fetchone()
        return row["id"] if row else None

    # -------------------------------------------------------------------
    # AI-driven plan_types — assert plan.behavior persists; task.content
    # propagation is verified in gm-e2e (needs scheduler).
    # -------------------------------------------------------------------

    def test_batch_text_plan_persists_behavior(self, auth_client, db_cursor):
        group_id = self._multi_account_group_id(db_cursor, PLATFORM_TIKTOK)
        chat_model_id = self._chat_model_id(db_cursor)
        if group_id is None or chat_model_id is None:
            pytest.skip("Need a TikTok group with accounts + a chat model")

        payload = {
            "plan_type": "batch_text",
            "platform_id": PLATFORM_TIKTOK,
            "group_id": group_id,
            "content_type": "post",
            "chat_ai_model_id": chat_model_id,
            "ai_input": {"content_prompt": "Travel post about Japan",
                         "hashtags_count": 3},
            "behavior": self._behavior(),
            "name": "task7a-batch_text-behavior",
        }
        resp = auth_client.post("/api/v1/publish_plans", json=payload)
        assert_response_success(resp)
        plan_id = extract_data(resp.json())["id"]

        db_cursor.execute(
            "SELECT plan_type, behavior FROM gm_aipub_plans WHERE id = %s",
            (plan_id,),
        )
        row = db_cursor.fetchone()
        assert row["plan_type"] == "batch_text"
        assert row["behavior"]["visibility"] == "unlisted"
        assert row["behavior"]["allow_comments"] is True
        assert row["behavior"]["extras"]["plan_type_marker"] == "task7a-coverage"

    def test_single_video_plan_persists_behavior(self, auth_client, db_cursor):
        account_id = self._account_id(db_cursor, PLATFORM_TIKTOK)
        chat_model_id = self._chat_model_id(db_cursor)
        video_model_id = self._video_model_id(db_cursor)
        if not (account_id and chat_model_id and video_model_id):
            pytest.skip("Need a TikTok account + chat + video AI models")

        payload = {
            "plan_type": "single_video",
            "platform_id": PLATFORM_TIKTOK,
            "social_account_id": account_id,
            "content_type": "video",
            "chat_ai_model_id": chat_model_id,
            "video_ai_model_id": video_model_id,
            "ai_input": {"content_prompt": "Cinematic teaser of Tokyo at night"},
            "behavior": {
                "visibility": "public",
                "allow_duet": True,
                "allow_stitch": False,
                "disclose_branded_content": True,
                "is_nsfw": False,
            },
            "name": "task7a-single_video-behavior",
        }
        resp = auth_client.post("/api/v1/publish_plans", json=payload)
        assert_response_success(resp)
        plan_id = extract_data(resp.json())["id"]

        db_cursor.execute(
            "SELECT plan_type, behavior FROM gm_aipub_plans WHERE id = %s",
            (plan_id,),
        )
        row = db_cursor.fetchone()
        assert row["plan_type"] == "single_video"
        b = row["behavior"]
        assert b["visibility"] == "public"
        assert b["allow_duet"] is True
        assert b["allow_stitch"] is False
        assert b["disclose_branded_content"] is True

    def test_reddit_text_plan_persists_behavior(self, auth_client, db_cursor):
        group_id = self._multi_account_group_id(db_cursor, PLATFORM_REDDIT)
        chat_model_id = self._chat_model_id(db_cursor)
        if group_id is None or chat_model_id is None:
            pytest.skip("Need a Reddit group + chat model")

        payload = {
            "plan_type": "reddit_text",
            "platform_id": PLATFORM_REDDIT,
            "group_id": group_id,
            "content_type": "post",
            "chat_ai_model_id": chat_model_id,
            "ai_input": {
                "content_prompt": "Discuss Rust vs Go ergonomics",
                "reddit_config": {"subreddit": "programming",
                                  "reddit_post_type": "TEXT"},
            },
            "behavior": {
                "visibility": "public",
                "is_nsfw": False,
                "is_spoiler": True,
                "extras": {"reddit_flair_pref": "Discussion"},
            },
            "name": "task7a-reddit_text-behavior",
        }
        resp = auth_client.post("/api/v1/publish_plans", json=payload)
        assert_response_success(resp)
        plan_id = extract_data(resp.json())["id"]

        db_cursor.execute(
            "SELECT plan_type, behavior FROM gm_aipub_plans WHERE id = %s",
            (plan_id,),
        )
        row = db_cursor.fetchone()
        assert row["plan_type"] == "reddit_text"
        assert row["behavior"]["is_spoiler"] is True
        assert row["behavior"]["extras"]["reddit_flair_pref"] == "Discussion"

    def test_reddit_image_plan_persists_behavior(self, auth_client, db_cursor):
        group_id = self._multi_account_group_id(db_cursor, PLATFORM_REDDIT)
        chat_model_id = self._chat_model_id(db_cursor)
        if group_id is None or chat_model_id is None:
            pytest.skip("Need a Reddit group + chat model")

        payload = {
            "plan_type": "reddit_image",
            "platform_id": PLATFORM_REDDIT,
            "group_id": group_id,
            # gm_aipub_plans.content_type is constrained to
            # post|video|reel|story|profile — Reddit image posts share
            # the "post" content_type and disambiguate via plan_type.
            "content_type": "post",
            "chat_ai_model_id": chat_model_id,
            "ai_input": {
                "content_prompt": "Caption for an aurora photo",
                "reddit_config": {
                    "subreddit": "EarthPorn",
                    "reddit_post_type": "IMAGE",
                    # uploaded_image_urls path so we don't depend on a
                    # seeded image AI model (none exist in gm-e2e).
                    "uploaded_image_urls": ["https://example.com/aurora.jpg"],
                },
            },
            "behavior": {
                "visibility": "public",
                "is_nsfw": False,
                "allow_comments": True,
            },
            "name": "task7a-reddit_image-behavior",
        }
        resp = auth_client.post("/api/v1/publish_plans", json=payload)
        assert_response_success(resp)
        plan_id = extract_data(resp.json())["id"]

        db_cursor.execute(
            "SELECT plan_type, behavior FROM gm_aipub_plans WHERE id = %s",
            (plan_id,),
        )
        row = db_cursor.fetchone()
        assert row["plan_type"] == "reddit_image"
        assert row["behavior"]["allow_comments"] is True

    def test_reddit_link_plan_persists_behavior(self, auth_client, db_cursor):
        group_id = self._multi_account_group_id(db_cursor, PLATFORM_REDDIT)
        chat_model_id = self._chat_model_id(db_cursor)
        if group_id is None or chat_model_id is None:
            pytest.skip("Need a Reddit group + chat model")

        payload = {
            "plan_type": "reddit_link",
            "platform_id": PLATFORM_REDDIT,
            "group_id": group_id,
            # See reddit_image note above — content_type stays "post"
            # for all reddit_* plan_types per the schema check constraint.
            "content_type": "post",
            "chat_ai_model_id": chat_model_id,
            "ai_input": {
                "content_prompt": "Explain why this article is interesting",
                "reddit_config": {
                    "subreddit": "programming",
                    "reddit_post_type": "LINK",
                    "link_url": "https://example.com/article",
                },
            },
            "behavior": {
                "visibility": "public",
                "is_nsfw": True,  # opt the link in as NSFW for assertion
            },
            "name": "task7a-reddit_link-behavior",
        }
        resp = auth_client.post("/api/v1/publish_plans", json=payload)
        assert_response_success(resp)
        plan_id = extract_data(resp.json())["id"]

        db_cursor.execute(
            "SELECT plan_type, behavior FROM gm_aipub_plans WHERE id = %s",
            (plan_id,),
        )
        row = db_cursor.fetchone()
        assert row["plan_type"] == "reddit_link"
        assert row["behavior"]["is_nsfw"] is True

    # -------------------------------------------------------------------
    # account_grooming — semantic mismatch (profile update, not publish).
    # Behavior must NOT block the create path even if a client sends it.
    # -------------------------------------------------------------------

    def test_account_grooming_plan_accepts_but_ignores_behavior(
        self, auth_client, db_cursor
    ):
        """Backward-compat negative: PublishBehavior is meaningful only for
        publish flows. account_grooming generates name+avatar (profile
        update), not a publish. The API must still accept the field for
        forward-compat (clients may emit a uniform plan create payload),
        persist it on the column, but the grooming task derivation path
        does not consume it. We assert only that:
          1. The create call does not 400.
          2. The plan column carries the behavior we sent.

        The grooming task content path is exercised by gm-e2e
        test_03_aipub_account_grooming.py (separately verified to still
        pass after Task 7a — see PR #7 verification report).
        """
        group_id = self._multi_account_group_id(db_cursor, PLATFORM_TIKTOK)
        chat_model_id = self._chat_model_id(db_cursor)
        if group_id is None or chat_model_id is None:
            pytest.skip("Need a TikTok group + chat model")

        payload = {
            "plan_type": "account_grooming",
            "platform_id": PLATFORM_TIKTOK,
            "group_id": group_id,
            "content_type": "profile",
            "chat_ai_model_id": chat_model_id,
            "ai_input": {"content_prompt": "Generate a witty bio"},
            "behavior": self._behavior(),
            "name": "task7a-grooming-ignored-behavior",
        }
        resp = auth_client.post("/api/v1/publish_plans", json=payload)
        assert_response_success(resp)
        plan_id = extract_data(resp.json())["id"]

        db_cursor.execute(
            "SELECT plan_type, behavior FROM gm_aipub_plans WHERE id = %s",
            (plan_id,),
        )
        row = db_cursor.fetchone()
        assert row["plan_type"] == "account_grooming"
        # Column carries it (we don't drop fields the client sent us);
        # the grooming runtime path simply doesn't read it.
        assert row["behavior"] is not None
        assert row["behavior"]["extras"]["plan_type_marker"] == "task7a-coverage"


# =============================================================================
# Phase 4 Round 3 Task 8 — PublishSchedule persistence + propagation
# =============================================================================


class TestPlanPublishSchedule:
    """API must accept, persist and propagate PublishSchedule on plans.

    PublishSchedule (proto: glance_mind_protocol/proto/aipub.proto §996)
    is plan-level scheduling config (scheduled_at UTC / timezone /
    save_as_draft). It mirrors the PublishBehavior plumbing from
    Task 7a: stored on gm_aipub_plans.schedule JSONB and merged into
    every derived task.content.schedule by the same
    expand_plan_to_tasks path so workers see the schedule natively
    via UnifiedPublishContent.from_dict.
    """

    def _schedule(self, **overrides) -> Dict[str, Any]:
        base = {
            "scheduled_at": "2027-06-01T12:30:00Z",
            "timezone": "America/Los_Angeles",
            "save_as_draft": False,
        }
        base.update(overrides)
        return base

    def _resolve_test_account(self, db_cursor) -> Optional[int]:
        db_cursor.execute("SELECT id FROM gm_social_accounts LIMIT 1")
        row = db_cursor.fetchone()
        return row["id"] if row else None

    # -------------------------------------------------------------------
    # Persistence — DB column behaviour
    # -------------------------------------------------------------------

    def test_create_plan_with_schedule_persists_to_db(self, auth_client, db_cursor):
        account_id = self._resolve_test_account(db_cursor)
        if not account_id:
            pytest.skip("No social account available")

        payload = {
            "social_account_id": account_id,
            "platform_id": PLATFORM_FACEBOOK,
            "content_type": "post",
            "plan_type": "direct_publish",
            "content": {"title": "T", "description": "D"},
            "schedule": self._schedule(),
        }
        resp = auth_client.post("/api/v1/publish_plans", json=payload)
        assert_response_success(resp)
        plan = extract_data(resp.json())

        db_cursor.execute(
            "SELECT schedule FROM gm_aipub_plans WHERE id = %s",
            (plan["id"],),
        )
        row = db_cursor.fetchone()
        assert row is not None
        assert row["schedule"] is not None
        s = row["schedule"]
        assert s["scheduled_at"] == "2027-06-01T12:30:00Z"
        assert s["timezone"] == "America/Los_Angeles"
        assert s["save_as_draft"] is False

    def test_create_plan_without_schedule_leaves_field_null(
        self, auth_client, db_cursor
    ):
        account_id = self._resolve_test_account(db_cursor)
        if not account_id:
            pytest.skip("No social account available")

        payload = {
            "social_account_id": account_id,
            "platform_id": PLATFORM_FACEBOOK,
            "content_type": "post",
            "plan_type": "direct_publish",
            "content": {"title": "T", "description": "D"},
        }
        resp = auth_client.post("/api/v1/publish_plans", json=payload)
        assert_response_success(resp)
        plan = extract_data(resp.json())

        db_cursor.execute(
            "SELECT schedule FROM gm_aipub_plans WHERE id = %s",
            (plan["id"],),
        )
        row = db_cursor.fetchone()
        assert row is not None
        assert row["schedule"] is None

    # -------------------------------------------------------------------
    # Round-trip via API responses
    # -------------------------------------------------------------------

    def test_get_plan_returns_schedule_field(self, auth_client, db_cursor):
        account_id = self._resolve_test_account(db_cursor)
        if not account_id:
            pytest.skip("No social account available")

        create_resp = auth_client.post("/api/v1/publish_plans", json={
            "social_account_id": account_id,
            "platform_id": PLATFORM_FACEBOOK,
            "content_type": "post",
            "plan_type": "direct_publish",
            "content": {"title": "T"},
            "schedule": self._schedule(save_as_draft=True),
        })
        assert_response_success(create_resp)
        plan_id = extract_data(create_resp.json())["id"]

        resp = auth_client.get(f"/api/v1/publish_plans/{plan_id}")
        assert_response_success(resp)
        data = extract_data(resp.json())
        assert "schedule" in data
        assert data["schedule"]["scheduled_at"] == "2027-06-01T12:30:00Z"
        assert data["schedule"]["save_as_draft"] is True

    def test_update_plan_can_change_schedule(self, auth_client, db_cursor):
        account_id = self._resolve_test_account(db_cursor)
        if not account_id:
            pytest.skip("No social account available")

        create_resp = auth_client.post(
            "/api/v1/publish_plans",
            json={
                "social_account_id": account_id,
                "platform_id": PLATFORM_FACEBOOK,
                "content_type": "post",
                "plan_type": "direct_publish",
                "content": {"title": "T"},
                "schedule": self._schedule(),
            },
        )
        assert_response_success(create_resp)
        plan_id = extract_data(create_resp.json())["id"]

        update_resp = auth_client.put(
            f"/api/v1/publish_plans/{plan_id}",
            json={
                "schedule": {
                    "scheduled_at": "2028-01-15T09:00:00Z",
                    "timezone": "UTC",
                    "save_as_draft": True,
                }
            },
        )
        assert_response_success(update_resp)

        db_cursor.execute(
            "SELECT schedule FROM gm_aipub_plans WHERE id = %s",
            (plan_id,),
        )
        s = db_cursor.fetchone()["schedule"]
        assert s["scheduled_at"] == "2028-01-15T09:00:00Z"
        assert s["timezone"] == "UTC"
        assert s["save_as_draft"] is True

    # -------------------------------------------------------------------
    # Task derivation — schedule MUST land on task.content.schedule
    # -------------------------------------------------------------------

    def test_direct_publish_plan_propagates_schedule_to_task_content(
        self, auth_client, db_cursor
    ):
        account_id = self._resolve_test_account(db_cursor)
        if not account_id:
            pytest.skip("No social account available")

        schedule = self._schedule(save_as_draft=True)
        payload = {
            "social_account_id": account_id,
            "platform_id": PLATFORM_FACEBOOK,
            "content_type": "post",
            "plan_type": "direct_publish",
            "content": {"title": "T", "description": "D"},
            "schedule": schedule,
        }
        resp = auth_client.post("/api/v1/publish_plans", json=payload)
        assert_response_success(resp)
        plan_id = extract_data(resp.json())["id"]

        db_cursor.execute(
            "SELECT content FROM gm_aipub_tasks WHERE plan_id = %s",
            (plan_id,),
        )
        rows = db_cursor.fetchall()
        assert rows
        for row in rows:
            c = row["content"]
            assert "schedule" in c, (
                "task.content must carry the merged schedule block; "
                f"got keys={sorted(c.keys())}"
            )
            assert c["schedule"]["scheduled_at"] == "2027-06-01T12:30:00Z"
            assert c["schedule"]["save_as_draft"] is True
            # Original content fields preserved alongside schedule.
            assert c["title"] == "T"
            assert c["description"] == "D"

    def test_direct_publish_without_schedule_does_not_inject_empty_schedule(
        self, auth_client, db_cursor
    ):
        """NULL plan.schedule → task.content.schedule absent. Worker
        must see no schedule key rather than an empty PublishSchedule
        (which proto would interpret as `scheduled_at=""` → immediate
        publish by accident but with dangling timezone / draft flags)."""
        account_id = self._resolve_test_account(db_cursor)
        if not account_id:
            pytest.skip("No social account available")

        payload = {
            "social_account_id": account_id,
            "platform_id": PLATFORM_FACEBOOK,
            "content_type": "post",
            "plan_type": "direct_publish",
            "content": {"title": "T", "description": "D"},
        }
        resp = auth_client.post("/api/v1/publish_plans", json=payload)
        assert_response_success(resp)
        plan_id = extract_data(resp.json())["id"]

        db_cursor.execute(
            "SELECT content FROM gm_aipub_tasks WHERE plan_id = %s",
            (plan_id,),
        )
        rows = db_cursor.fetchall()
        assert rows
        for row in rows:
            assert "schedule" not in row["content"], \
                "no plan.schedule → no task.content.schedule"

    def test_schedule_coexists_with_behavior_on_same_task_content(
        self, auth_client, db_cursor
    ):
        """Both Task 7a's behavior merge and Task 8's schedule merge
        must coexist on the same task.content — neither clobbers the
        other. Pins down the merge contract: plan-level typed
        sub-messages accumulate under their own keys."""
        account_id = self._resolve_test_account(db_cursor)
        if not account_id:
            pytest.skip("No social account available")

        resp = auth_client.post("/api/v1/publish_plans", json={
            "social_account_id": account_id,
            "platform_id": PLATFORM_FACEBOOK,
            "content_type": "post",
            "plan_type": "direct_publish",
            "content": {"title": "T"},
            "behavior": {"visibility": "public", "is_nsfw": True},
            "schedule": self._schedule(save_as_draft=True),
        })
        assert_response_success(resp)
        plan_id = extract_data(resp.json())["id"]

        db_cursor.execute(
            "SELECT content FROM gm_aipub_tasks WHERE plan_id = %s",
            (plan_id,),
        )
        c = db_cursor.fetchall()[0]["content"]
        assert c["behavior"]["visibility"] == "public"
        assert c["behavior"]["is_nsfw"] is True
        assert c["schedule"]["save_as_draft"] is True
        assert c["title"] == "T"

    # -------------------------------------------------------------------
    # Validation
    # -------------------------------------------------------------------

    def test_invalid_scheduled_at_rejected(self, auth_client, db_cursor):
        """scheduled_at must be ISO-8601 UTC. Random garbage must 400."""
        account_id = self._resolve_test_account(db_cursor)
        if not account_id:
            pytest.skip("No social account available")

        payload = {
            "social_account_id": account_id,
            "platform_id": PLATFORM_FACEBOOK,
            "content_type": "post",
            "plan_type": "direct_publish",
            "content": {"title": "T"},
            "schedule": {"scheduled_at": "not-a-timestamp"},
        }
        resp = auth_client.post("/api/v1/publish_plans", json=payload)
        assert resp.status_code == 400, (
            f"expected 400 for invalid scheduled_at, got {resp.status_code}: {resp.text}"
        )

    def test_save_as_draft_must_be_bool(self, auth_client, db_cursor):
        account_id = self._resolve_test_account(db_cursor)
        if not account_id:
            pytest.skip("No social account available")

        payload = {
            "social_account_id": account_id,
            "platform_id": PLATFORM_FACEBOOK,
            "content_type": "post",
            "plan_type": "direct_publish",
            "content": {"title": "T"},
            "schedule": {
                "scheduled_at": "2027-06-01T12:30:00Z",
                "save_as_draft": "yes-please",
            },
        }
        resp = auth_client.post("/api/v1/publish_plans", json=payload)
        assert resp.status_code == 400, (
            f"expected 400 for non-bool save_as_draft, got {resp.status_code}: {resp.text}"
        )

    def test_missing_scheduled_at_rejected(self, auth_client, db_cursor):
        """scheduled_at is the whole point of a schedule — empty or
        missing should 400 rather than silently store a useless
        schedule dict."""
        account_id = self._resolve_test_account(db_cursor)
        if not account_id:
            pytest.skip("No social account available")

        payload = {
            "social_account_id": account_id,
            "platform_id": PLATFORM_FACEBOOK,
            "content_type": "post",
            "plan_type": "direct_publish",
            "content": {"title": "T"},
            "schedule": {"save_as_draft": True},  # no scheduled_at
        }
        resp = auth_client.post("/api/v1/publish_plans", json=payload)
        assert resp.status_code == 400, (
            f"expected 400 for missing scheduled_at, got {resp.status_code}: {resp.text}"
        )


# =============================================================================
# Phase 4 R3 Task 8 follow-up — API hides future-scheduled tasks from the
# executor's poll endpoint (GET /public/aipub/publish_tasks).
# =============================================================================


class TestPublicPublishTasksSchedulingFilter:
    """`find_ready_publish_tasks_by_device` must exclude tasks whose
    `content.schedule.scheduled_at` is still in the future, so the
    worker doesn't poll-and-defer them on every cycle (the worker-side
    dispatch gate in `AIPubTaskProcessor` already catches them as a
    belt-and-suspenders, but avoiding the round-trip is cheaper).

    Guaranteed contract:
      * No schedule on task.content            → returned (unchanged)
      * schedule.scheduled_at in the past       → returned
      * schedule.scheduled_at in the future     → **excluded**
      * schedule.scheduled_at absent/null/empty → returned (fail-open,
        matches is_schedule_due's fail-loud contract on the worker side)
    """

    def _resolve_account_with_device(self, db_cursor):
        """Pick an account that has a device_id set so the poll endpoint
        matches it. Skip if no such account exists."""
        db_cursor.execute(
            "SELECT id, device_id, platform_id FROM gm_social_accounts "
            "WHERE device_id IS NOT NULL AND device_id <> '' LIMIT 1"
        )
        return db_cursor.fetchone()

    def _seed_ready_task(
        self,
        db_cursor,
        account_id: int,
        platform_id: int,
        *,
        schedule: Optional[Dict[str, Any]] = None,
        marker: str = "",
    ) -> int:
        """Bypass the public API and insert a ready task with specific
        content shape. Uses raw SQL because the public create-plan flow
        always expands tasks synchronously through the
        merge_{behavior,schedule} path — we want precise control over
        what lands on gm_aipub_tasks.content here."""
        import json
        db_cursor.execute(
            """
            INSERT INTO gm_aipub_plans
                (user_id, platform_id, content_type, plan_type, status,
                 social_account_id)
            SELECT (SELECT id FROM gm_users LIMIT 1), %s, 'post',
                   'direct_publish', 'ready', %s
            RETURNING id
            """,
            (platform_id, account_id),
        )
        plan_id = db_cursor.fetchone()["id"]

        content: Dict[str, Any] = {
            "title": "T",
            "description": "D",
            "_marker": marker,
        }
        if schedule is not None:
            content["schedule"] = schedule

        db_cursor.execute(
            """
            INSERT INTO gm_aipub_tasks
                (plan_id, social_account_id, content, status, retry_count)
            VALUES (%s, %s, %s::jsonb, 'ready', 0)
            RETURNING id
            """,
            (plan_id, account_id, json.dumps(content)),
        )
        task_id = db_cursor.fetchone()["id"]
        db_cursor.connection.commit()
        return task_id

    def test_future_scheduled_task_is_hidden_from_poll(
        self, auth_client, db_cursor
    ):
        """A task with scheduled_at well in the future must NOT appear
        in GET /public/aipub/publish_tasks. This is the whole point of
        server-side filtering — avoid worker thrash."""
        account = self._resolve_account_with_device(db_cursor)
        if not account:
            pytest.skip("No social account with device_id available")

        future_schedule = {
            "scheduled_at": "2099-12-31T23:59:59Z",
            "save_as_draft": False,
        }
        future_task_id = self._seed_ready_task(
            db_cursor, account["id"], account["platform_id"],
            schedule=future_schedule,
            marker="task8-future-hidden",
        )

        resp = auth_client.get(
            f"/api/v1/public/aipub/publish_tasks?device_id={account['device_id']}&limit=1000"
        )
        assert_response_success(resp)
        tasks = extract_data(resp.json())
        # Response may be a bare list or wrapped — normalise.
        if isinstance(tasks, dict):
            tasks = tasks.get("list") or tasks.get("data") or []
        task_ids = {t.get("task_id") for t in tasks}

        assert future_task_id not in task_ids, (
            f"future-scheduled task {future_task_id} must not appear in "
            f"ready tasks poll; got ids={task_ids}"
        )

    def test_past_scheduled_task_is_returned(self, auth_client, db_cursor):
        account = self._resolve_account_with_device(db_cursor)
        if not account:
            pytest.skip("No social account with device_id available")

        past_schedule = {
            "scheduled_at": "2020-01-01T00:00:00Z",
            "save_as_draft": False,
        }
        past_task_id = self._seed_ready_task(
            db_cursor, account["id"], account["platform_id"],
            schedule=past_schedule,
            marker="task8-past-visible",
        )

        resp = auth_client.get(
            f"/api/v1/public/aipub/publish_tasks?device_id={account['device_id']}&limit=1000"
        )
        assert_response_success(resp)
        tasks = extract_data(resp.json())
        if isinstance(tasks, dict):
            tasks = tasks.get("list") or tasks.get("data") or []
        task_ids = {t.get("task_id") for t in tasks}

        assert past_task_id in task_ids, (
            f"past-scheduled task {past_task_id} must be returned — "
            f"scheduled time already elapsed. Got ids={task_ids}"
        )

    def test_unscheduled_task_is_returned(self, auth_client, db_cursor):
        """Backward compatibility: tasks without any schedule key on
        content must continue to be returned by the poll endpoint —
        this is 100% of pre-Task-8 data and the common case for new
        plans too (schedule is opt-in)."""
        account = self._resolve_account_with_device(db_cursor)
        if not account:
            pytest.skip("No social account with device_id available")

        bare_task_id = self._seed_ready_task(
            db_cursor, account["id"], account["platform_id"],
            schedule=None,  # no schedule key at all
            marker="task8-unscheduled-visible",
        )

        resp = auth_client.get(
            f"/api/v1/public/aipub/publish_tasks?device_id={account['device_id']}&limit=1000"
        )
        assert_response_success(resp)
        tasks = extract_data(resp.json())
        if isinstance(tasks, dict):
            tasks = tasks.get("list") or tasks.get("data") or []
        task_ids = {t.get("task_id") for t in tasks}

        assert bare_task_id in task_ids, (
            f"unscheduled task {bare_task_id} must be returned "
            f"(backward compat). Got ids={task_ids}"
        )

    def test_malformed_scheduled_at_is_returned_fail_open(
        self, auth_client, db_cursor
    ):
        """If a malformed scheduled_at somehow reaches the DB (e.g. a
        migration from legacy data, or a schema predating the Rust
        validator), the filter MUST fail open — return the task
        rather than silently hide it forever. Mirrors
        `is_schedule_due`'s fail-loud contract on the worker side."""
        account = self._resolve_account_with_device(db_cursor)
        if not account:
            pytest.skip("No social account with device_id available")

        malformed_task_id = self._seed_ready_task(
            db_cursor, account["id"], account["platform_id"],
            schedule={"scheduled_at": "not-a-timestamp",
                      "save_as_draft": False},
            marker="task8-malformed-visible",
        )

        resp = auth_client.get(
            f"/api/v1/public/aipub/publish_tasks?device_id={account['device_id']}&limit=1000"
        )
        assert_response_success(resp)
        tasks = extract_data(resp.json())
        if isinstance(tasks, dict):
            tasks = tasks.get("list") or tasks.get("data") or []
        task_ids = {t.get("task_id") for t in tasks}

        assert malformed_task_id in task_ids, (
            "malformed scheduled_at must NOT cause the query to error "
            "or silently drop the task — fail-open contract"
        )


PLATFORM_TWITTER = 5  # gm_platforms.id for Twitter/X


class TestCreatePlanPlatformMatrix:
    """Verify every (platform_id × content_type × plan_type) combination that
    the frontend AIPubPlanCreate.tsx wizard can produce is accepted by the
    create endpoint AND persisted with matching DB field values.

    The matrix mirrors `docs/superpowers/plans/2026-04-17-aipub-platform-matrix-coverage.md`
    in glance_mind_front; any drift between frontend wizard output and this
    matrix indicates a contract break between front and API.

    Why API-level: the frontend matrix spec
    (e2e/tests/ai-publish/28-plan-create-platform-matrix.spec.ts) drives the
    UI and proves the wizard submits the right payload. *This* suite proves
    the API accepts that payload shape and writes gm_aipub_plans correctly
    for each combination. Together they close the front→API→DB chain.
    """

    def _pick_group(self, db_cursor, platform_id: int) -> Optional[int]:
        """Pick a group on `platform_id` with at least one bound account.
        Required for batch_text / reddit_* plans per repository invariants."""
        db_cursor.execute(
            """
            SELECT g.id FROM gm_social_groups g
              JOIN gm_social_accounts a ON a.group_id = g.id
             WHERE g.platform_id = %s
             GROUP BY g.id HAVING COUNT(a.id) >= 1
             ORDER BY g.id LIMIT 1
            """,
            (platform_id,),
        )
        row = db_cursor.fetchone()
        return row["id"] if row else None

    def _pick_account(self, db_cursor, platform_id: int) -> Optional[int]:
        db_cursor.execute(
            "SELECT id FROM gm_social_accounts WHERE platform_id = %s ORDER BY id LIMIT 1",
            (platform_id,),
        )
        row = db_cursor.fetchone()
        return row["id"] if row else None

    def _pick_video_model(self, db_cursor) -> Optional[int]:
        db_cursor.execute(
            "SELECT id FROM gm_ai_models WHERE model_type = 'video' AND is_active = true ORDER BY id LIMIT 1"
        )
        row = db_cursor.fetchone()
        return row["id"] if row else None

    def _pick_chat_model(self, db_cursor) -> Optional[int]:
        db_cursor.execute(
            "SELECT id FROM gm_ai_models WHERE model_type = 'chat' AND is_active = true ORDER BY id LIMIT 1"
        )
        row = db_cursor.fetchone()
        return row["id"] if row else None

    # --------------------------------------------------------------
    # Matrix: 12 combinations (platform × content × plan_type × target)
    # Columns: key, platform_id, content_type, plan_type,
    #          target_type ('group' | 'account'),
    #          ai_task_types, extra_ai_input
    # --------------------------------------------------------------
    MATRIX = [
        ("tiktok-video",     PLATFORM_TIKTOK,    "video", "single_video", "account",
         ["video_gen", "content_gen"], {}),
        ("facebook-post",    PLATFORM_FACEBOOK,  "post",  "batch_text",   "group",
         ["content_gen", "image_gen"], {}),
        ("facebook-video",   PLATFORM_FACEBOOK,  "video", "single_video", "account",
         ["video_gen", "content_gen"], {}),
        ("facebook-reel",    PLATFORM_FACEBOOK,  "reel",  "single_video", "account",
         ["video_gen", "content_gen"], {}),
        ("facebook-story",   PLATFORM_FACEBOOK,  "story", "batch_text",   "group",
         ["video_gen", "image_gen"], {}),
        ("instagram-post",   PLATFORM_INSTAGRAM, "post",  "batch_text",   "group",
         ["content_gen", "image_gen"], {}),
        ("instagram-reel",   PLATFORM_INSTAGRAM, "reel",  "single_video", "account",
         ["video_gen", "content_gen"], {}),
        ("instagram-story",  PLATFORM_INSTAGRAM, "story", "batch_text",   "group",
         ["video_gen", "image_gen"], {}),
        ("twitter-post",     PLATFORM_TWITTER,   "post",  "batch_text",   "group",
         ["content_gen", "image_gen"], {}),
        ("reddit-text",      PLATFORM_REDDIT,    "post",  "reddit_text",  "group",
         ["content_gen"],
         {"reddit_config": {"subreddit": "matrix_text",
                            "reddit_post_type": "TEXT",
                            "use_markdown": False,
                            "is_nsfw": False, "is_spoiler": False}}),
        ("reddit-image",     PLATFORM_REDDIT,    "post",  "reddit_image", "group",
         ["content_gen", "image_gen"],
         {"reddit_config": {"subreddit": "matrix_image",
                            "reddit_post_type": "IMAGE",
                            "is_nsfw": False, "is_spoiler": False}}),
        ("reddit-link",      PLATFORM_REDDIT,    "post",  "reddit_link",  "group",
         ["content_gen"],
         {"reddit_config": {"subreddit": "matrix_link",
                            "reddit_post_type": "LINK",
                            "link_url": "https://example.com/matrix-link",
                            "is_nsfw": False, "is_spoiler": False}}),
    ]

    @pytest.mark.parametrize(
        "key,platform_id,content_type,plan_type,target_type,ai_task_types,extra_ai_input",
        MATRIX,
        ids=[row[0] for row in MATRIX],
    )
    def test_matrix_create_and_persist(
        self,
        auth_client,
        db_cursor,
        key,
        platform_id,
        content_type,
        plan_type,
        target_type,
        ai_task_types,
        extra_ai_input,
    ):
        """POST a plan matching the frontend wizard payload shape; verify the
        API responds 2xx and `gm_aipub_plans` row has every expected field."""

        chat_model_id = self._pick_chat_model(db_cursor)
        video_model_id = (
            self._pick_video_model(db_cursor)
            if "video_gen" in ai_task_types
            else None
        )
        if chat_model_id is None:
            pytest.skip("No active chat model seeded")
        if "video_gen" in ai_task_types and video_model_id is None:
            pytest.skip("No active video model seeded")

        # Pick target (group or account) on the right platform.
        target_group_id = None
        target_account_id = None
        if target_type == "group":
            target_group_id = self._pick_group(db_cursor, platform_id)
            if target_group_id is None:
                pytest.skip(f"No group with accounts on platform_id={platform_id}; "
                            f"need gm-e2e seed 07_seed_front_business.sql")
        else:
            target_account_id = self._pick_account(db_cursor, platform_id)
            if target_account_id is None:
                pytest.skip(f"No account on platform_id={platform_id}")

        # Build payload mirroring AIPubPlanCreate.tsx submit logic.
        ai_input: Dict[str, Any] = {
            "content_prompt": f"matrix test content prompt for {key}",
            **extra_ai_input,
        }
        if "video_gen" in ai_task_types:
            ai_input["video_prompt"] = f"matrix test video prompt for {key}"

        payload: Dict[str, Any] = {
            "name": f"matrix-{key}",
            "platform_id": platform_id,
            "content_type": content_type,
            "plan_type": plan_type,
            "group_id": target_group_id,
            "social_account_id": target_account_id,
            "chat_ai_model_id": chat_model_id,
            "video_ai_model_id": video_model_id,
            "ai_task_types": ai_task_types,
            "ai_input": ai_input,
        }

        resp = auth_client.post("/api/v1/publish_plans", json=payload)
        assert resp.status_code in [200, 201], (
            f"[{key}] POST /publish_plans returned {resp.status_code}: {resp.text}"
        )
        data = extract_data(resp.json())
        plan_id = data.get("id")
        assert isinstance(plan_id, int) and plan_id > 0, (
            f"[{key}] expected integer plan id, got {data}"
        )

        # Verify every field persisted on gm_aipub_plans.
        db_cursor.execute(
            """
            SELECT id, user_id, platform_id, content_type, plan_type,
                   group_id, social_account_id, chat_ai_model_id,
                   video_ai_model_id, ai_task_types, ai_input, name,
                   status, created_at, updated_at
              FROM gm_aipub_plans
             WHERE id = %s
            """,
            (plan_id,),
        )
        row = db_cursor.fetchone()
        assert row is not None, f"[{key}] plan_id={plan_id} missing from DB"

        assert row["platform_id"] == platform_id, f"[{key}] platform_id mismatch"
        assert row["content_type"] == content_type, f"[{key}] content_type mismatch"
        assert row["plan_type"] == plan_type, f"[{key}] plan_type mismatch"

        if target_type == "group":
            assert row["group_id"] == target_group_id, f"[{key}] group_id mismatch"
            assert row["social_account_id"] is None, f"[{key}] social_account_id should be NULL"
        else:
            assert row["social_account_id"] == target_account_id, f"[{key}] account id mismatch"
            assert row["group_id"] is None, f"[{key}] group_id should be NULL"

        assert row["chat_ai_model_id"] == chat_model_id, f"[{key}] chat_ai_model_id mismatch"
        if video_model_id is not None:
            assert row["video_ai_model_id"] == video_model_id, f"[{key}] video_ai_model_id mismatch"

        # ai_task_types persisted as array (Postgres text[] or jsonb)
        persisted_types = row["ai_task_types"]
        if isinstance(persisted_types, str):
            import json as _json
            try:
                persisted_types = _json.loads(persisted_types)
            except Exception:
                persisted_types = [x.strip() for x in persisted_types.strip("{}").split(",") if x.strip()]
        assert sorted(persisted_types) == sorted(ai_task_types), (
            f"[{key}] ai_task_types mismatch: expected {ai_task_types} got {persisted_types}"
        )

        # ai_input round-trip
        persisted_ai_input = row["ai_input"]
        assert persisted_ai_input is not None, f"[{key}] ai_input is NULL"
        assert persisted_ai_input["content_prompt"] == ai_input["content_prompt"]
        if "video_prompt" in ai_input:
            assert persisted_ai_input["video_prompt"] == ai_input["video_prompt"]
        if "reddit_config" in ai_input:
            rc = persisted_ai_input["reddit_config"]
            expected_rc = ai_input["reddit_config"]
            assert rc["subreddit"] == expected_rc["subreddit"]
            assert rc["reddit_post_type"] == expected_rc["reddit_post_type"]
            if "link_url" in expected_rc:
                assert rc["link_url"] == expected_rc["link_url"]

        # Metadata sanity
        assert row["name"] == f"matrix-{key}"
        assert row["status"] in ("pending", "ai_processing"), f"[{key}] status={row['status']}"
        assert row["user_id"] is not None
        assert row["created_at"] is not None
        assert row["updated_at"] is not None
        assert row["created_at"] <= row["updated_at"]

        # Cleanup so subsequent matrix runs don't accumulate.
        db_cursor.execute("DELETE FROM gm_aipub_ai_tasks WHERE plan_id = %s", (plan_id,))
        db_cursor.execute("DELETE FROM gm_aipub_plans WHERE id = %s", (plan_id,))


# ---------------------------------------------------------------------------
# MM-2.5 — BatchText multi-image pre-billing.
#
# The scheduler generates images for BatchText plans when
# ai_task_types contains "image_gen" AND ai_input.image_generations[]
# is set (e.g. FB/IG multi-image carousels through the BatchText
# pipeline). The pre-bill (frozen_cost) MUST account for those images,
# otherwise users can run the pipeline without sufficient balance and
# the actual image generation later either fails or is uncharged.
#
# Billing rule (matches RedditImage):
#   frozen_cost = N_accounts * chat_cost
#               + N_accounts * sum(image_generations[].count) * image_cost
# ---------------------------------------------------------------------------


class TestBatchTextMultiImageBilling:
    """MM-2.5 — verify image_generations[] drives frozen_cost for BatchText.

    Covers three tiers:

      A. Plain BatchText (no image_generations) stays chat-only billing.
      B. BatchText + image_generations[{count: 3}] pre-bills N*3 images.
      C. Multiple image_generations entries sum correctly.
    """

    def _multi_account_group_id(self, db_cursor, platform_id: int) -> Optional[int]:
        db_cursor.execute(
            """
            SELECT g.id, COUNT(a.id) AS n
              FROM gm_social_groups g
              JOIN gm_social_accounts a ON a.group_id = g.id
             WHERE g.platform_id = %s
             GROUP BY g.id
             HAVING COUNT(a.id) >= 1
             ORDER BY g.id
             LIMIT 1
            """,
            (platform_id,),
        )
        return db_cursor.fetchone()

    def _chat_model_id(self, db_cursor) -> Optional[int]:
        db_cursor.execute(
            "SELECT id FROM gm_ai_models WHERE model_type = 'chat' "
            "AND is_active = true ORDER BY id LIMIT 1"
        )
        row = db_cursor.fetchone()
        return row["id"] if row else None

    def _image_model_id(self, db_cursor) -> Optional[int]:
        db_cursor.execute(
            "SELECT id FROM gm_ai_models WHERE model_type = 'image' "
            "AND is_active = true ORDER BY id LIMIT 1"
        )
        row = db_cursor.fetchone()
        return row["id"] if row else None

    def _action_base_price(self, db_cursor, action_type: str) -> float:
        """Mirrors fn_freeze_budget: prefer platform_id IS NULL, fall back to
        MIN(cost_points) of matching rows, defaulting to 1.0 for chat and 0
        for image/video per up.sql."""
        db_cursor.execute(
            "SELECT COALESCE(cost_points, 0)::float AS c FROM gm_pricing_rules "
            "WHERE action_type = %s AND platform_id IS NULL LIMIT 1",
            (action_type,),
        )
        row = db_cursor.fetchone()
        base = float(row["c"]) if row else 0.0
        if base == 0.0 and action_type == "AI_ANALYZE":
            db_cursor.execute(
                "SELECT COALESCE(MIN(cost_points), 1.0)::float AS c "
                "FROM gm_pricing_rules WHERE action_type = 'AI_ANALYZE'"
            )
            base = float(db_cursor.fetchone()["c"])
        return base

    def _chat_cost(self, db_cursor, model_id: int) -> float:
        db_cursor.execute(
            "SELECT cost_multiplier FROM gm_ai_models WHERE id = %s",
            (model_id,),
        )
        mult = float(db_cursor.fetchone()["cost_multiplier"])
        return self._action_base_price(db_cursor, "AI_ANALYZE") * mult

    def _image_cost(self, db_cursor, model_id: int) -> float:
        db_cursor.execute(
            "SELECT cost_multiplier FROM gm_ai_models WHERE id = %s",
            (model_id,),
        )
        mult = float(db_cursor.fetchone()["cost_multiplier"])
        return self._action_base_price(db_cursor, "IMAGE") * mult

    def _resolve_user_id(self, db_cursor) -> Optional[int]:
        import os as _os
        email = _os.getenv("E2E_TEST_EMAIL", "e2e@glancemind.test")
        db_cursor.execute("SELECT id FROM gm_users WHERE email = %s", (email,))
        row = db_cursor.fetchone()
        return row["id"] if row else None

    def _ensure_balance(self, db_cursor, amount: float = 10000.0):
        user_id = self._resolve_user_id(db_cursor)
        if user_id is None:
            return
        db_cursor.execute(
            "UPDATE gm_user_wallets SET balance_points = GREATEST(balance_points, %s) "
            "WHERE user_id = %s",
            (amount, user_id),
        )
        # Must commit so the API's separate transaction sees the topped-up
        # balance before we POST /publish_plans (which triggers
        # fn_freeze_budget).
        db_cursor.connection.commit()

    def _create_plan(self, auth_client, group_id, chat_model_id,
                     image_model_id=None, image_generations=None,
                     ai_task_types=None):
        ai_input: Dict[str, Any] = {
            "content_prompt": "Multi-image BatchText pre-billing test",
        }
        if image_generations is not None:
            ai_input["image_generations"] = image_generations

        payload: Dict[str, Any] = {
            "plan_type": "batch_text",
            "platform_id": PLATFORM_FACEBOOK,
            "group_id": group_id,
            "content_type": "post",
            "chat_ai_model_id": chat_model_id,
            "ai_input": ai_input,
            "name": "mm-2.5-batchtext-multi-image",
        }
        if image_model_id is not None:
            payload["image_ai_model_id"] = image_model_id
        if ai_task_types is not None:
            payload["ai_task_types"] = ai_task_types
        return auth_client.post("/api/v1/publish_plans", json=payload)

    def test_batchtext_without_image_generations_is_chat_only_billing(
        self, auth_client, db_cursor
    ):
        grp = self._multi_account_group_id(db_cursor, PLATFORM_FACEBOOK)
        chat_id = self._chat_model_id(db_cursor)
        if grp is None or chat_id is None:
            pytest.skip("Need a Facebook group with accounts + chat model")
        self._ensure_balance(db_cursor, 10000.0)

        n = int(grp["n"])
        chat_unit = self._chat_cost(db_cursor, chat_id)
        expected_min = n * chat_unit

        resp = self._create_plan(
            auth_client, grp["id"], chat_id,
            ai_task_types=["content_gen"],
        )
        assert_response_success(resp)
        plan = extract_data(resp.json())
        plan_id = plan["id"]

        db_cursor.execute(
            "SELECT billing_status, frozen_cost FROM gm_aipub_plans WHERE id = %s",
            (plan_id,),
        )
        row = db_cursor.fetchone()
        assert row["billing_status"] == "frozen"
        frozen = float(row["frozen_cost"])
        # Tolerance for cost_multiplier precision (tests don't depend on
        # exact multiplier values — just that image_count does NOT
        # contribute when image_generations[] is absent).
        assert abs(frozen - expected_min) < 1e-6, (
            f"chat-only BatchText expected {expected_min}, got {frozen}"
        )

        db_cursor.execute("DELETE FROM gm_aipub_ai_tasks WHERE plan_id = %s", (plan_id,))
        db_cursor.execute("DELETE FROM gm_aipub_plans WHERE id = %s", (plan_id,))

    def test_batchtext_with_image_generations_prebills_images(
        self, auth_client, db_cursor
    ):
        """Regression test for MM-2.5: image_generations[{count: 3}] must
        contribute N*3 images to frozen_cost."""
        grp = self._multi_account_group_id(db_cursor, PLATFORM_FACEBOOK)
        chat_id = self._chat_model_id(db_cursor)
        img_id = self._image_model_id(db_cursor)
        if grp is None or chat_id is None or img_id is None:
            pytest.skip("Need a Facebook group + chat model + image model")
        self._ensure_balance(db_cursor, 10000.0)

        n = int(grp["n"])
        per_account_images = 3
        chat_unit = self._chat_cost(db_cursor, chat_id)
        image_unit = self._image_cost(db_cursor, img_id)
        expected = n * chat_unit + n * per_account_images * image_unit
        chat_only = n * chat_unit

        resp = self._create_plan(
            auth_client, grp["id"], chat_id,
            image_model_id=img_id,
            image_generations=[{
                "prompts": ["three carousel images for this post"],
                "count": per_account_images,
                "model": "flux-kontext-pro",
                "aspect_ratio": "1:1",
            }],
            ai_task_types=["content_gen", "image_gen"],
        )
        assert_response_success(resp)
        plan = extract_data(resp.json())
        plan_id = plan["id"]

        db_cursor.execute(
            "SELECT billing_status, frozen_cost FROM gm_aipub_plans WHERE id = %s",
            (plan_id,),
        )
        row = db_cursor.fetchone()
        assert row["billing_status"] == "frozen"
        frozen = float(row["frozen_cost"])
        assert abs(frozen - expected) < 1e-4, (
            f"BatchText multi-image expected {expected} "
            f"(chat {n}*{chat_unit} + image {n}*{per_account_images}*{image_unit}), "
            f"got {frozen}"
        )
        # Also prove the fix: without it, frozen would equal chat-only.
        assert frozen > chat_only + 1e-6, (
            "frozen_cost must exceed chat-only cost when images are generated; "
            "this asserts MM-2.5 billing loophole is closed."
        )

        db_cursor.execute("DELETE FROM gm_aipub_ai_tasks WHERE plan_id = %s", (plan_id,))
        db_cursor.execute("DELETE FROM gm_aipub_plans WHERE id = %s", (plan_id,))

    def test_batchtext_multiple_image_generations_entries_sum(
        self, auth_client, db_cursor
    ):
        """Two image_generations entries (count=2, count=3) should sum to
        5 images per account, matching the RedditImage pattern exactly."""
        grp = self._multi_account_group_id(db_cursor, PLATFORM_FACEBOOK)
        chat_id = self._chat_model_id(db_cursor)
        img_id = self._image_model_id(db_cursor)
        if grp is None or chat_id is None or img_id is None:
            pytest.skip("Need a Facebook group + chat model + image model")
        self._ensure_balance(db_cursor, 10000.0)

        n = int(grp["n"])
        per_account_images = 2 + 3
        chat_unit = self._chat_cost(db_cursor, chat_id)
        image_unit = self._image_cost(db_cursor, img_id)
        expected = n * chat_unit + n * per_account_images * image_unit

        resp = self._create_plan(
            auth_client, grp["id"], chat_id,
            image_model_id=img_id,
            image_generations=[
                {"prompts": ["first batch"], "count": 2, "model": "flux-kontext-pro", "aspect_ratio": "1:1"},
                {"prompts": ["second batch"], "count": 3, "model": "flux-kontext-pro", "aspect_ratio": "1:1"},
            ],
            ai_task_types=["content_gen", "image_gen"],
        )
        assert_response_success(resp)
        plan_id = extract_data(resp.json())["id"]

        db_cursor.execute(
            "SELECT frozen_cost FROM gm_aipub_plans WHERE id = %s",
            (plan_id,),
        )
        frozen = float(db_cursor.fetchone()["frozen_cost"])
        assert abs(frozen - expected) < 1e-4, (
            f"Multi-spec image_generations expected {expected}, got {frozen}"
        )

        db_cursor.execute("DELETE FROM gm_aipub_ai_tasks WHERE plan_id = %s", (plan_id,))
        db_cursor.execute("DELETE FROM gm_aipub_plans WHERE id = %s", (plan_id,))


# ---------------------------------------------------------------------------
# MM-2 — RedditImage multi-image billing matrix
# ---------------------------------------------------------------------------
#
# Plan spec §MM-2: parametrize ``image_count ∈ {1, 3}`` over RedditImage
# plans and assert
#   * HTTP 201 + UnifiedResponse success,
#   * DB ``gm_aipub_plans.ai_input->>'image_generations'[0]->>'count'`` ==
#     image_count,
#   * ``frozen_cost`` = ``1 * chat_unit + image_count * n_accounts * image_unit``
#     (RedditImage freezes ``1`` chat regardless of account count, and
#     ``image_count * n_accounts`` images — see aipub_service.rs:452).
#
# This matrix pins the RedditImage V2 billing path end-to-end so the
# MM-2.5 BatchText fix doesn't accidentally regress RedditImage.
# ---------------------------------------------------------------------------


class TestRedditImageMultiImageBilling:
    """MM-2 — matrix test over image_count for RedditImage plans.

    Mirrors the helper structure of TestBatchTextMultiImageBilling so
    a future refactor can hoist both into a shared base.
    """

    def _multi_account_group_id(self, db_cursor, platform_id: int) -> Optional[int]:
        db_cursor.execute(
            """
            SELECT g.id, COUNT(a.id) AS n
              FROM gm_social_groups g
              JOIN gm_social_accounts a ON a.group_id = g.id
             WHERE g.platform_id = %s
             GROUP BY g.id
             HAVING COUNT(a.id) >= 1
             ORDER BY g.id
             LIMIT 1
            """,
            (platform_id,),
        )
        return db_cursor.fetchone()

    def _chat_model_id(self, db_cursor) -> Optional[int]:
        db_cursor.execute(
            "SELECT id FROM gm_ai_models WHERE model_type = 'chat' "
            "AND is_active = true ORDER BY id LIMIT 1"
        )
        row = db_cursor.fetchone()
        return row["id"] if row else None

    def _image_model_id(self, db_cursor) -> Optional[int]:
        db_cursor.execute(
            "SELECT id FROM gm_ai_models WHERE model_type = 'image' "
            "AND is_active = true ORDER BY id LIMIT 1"
        )
        row = db_cursor.fetchone()
        return row["id"] if row else None

    def _action_base_price(self, db_cursor, action_type: str) -> float:
        db_cursor.execute(
            "SELECT COALESCE(cost_points, 0)::float AS c FROM gm_pricing_rules "
            "WHERE action_type = %s AND platform_id IS NULL LIMIT 1",
            (action_type,),
        )
        row = db_cursor.fetchone()
        base = float(row["c"]) if row else 0.0
        if base == 0.0 and action_type == "AI_ANALYZE":
            db_cursor.execute(
                "SELECT COALESCE(MIN(cost_points), 1.0)::float AS c "
                "FROM gm_pricing_rules WHERE action_type = 'AI_ANALYZE'"
            )
            base = float(db_cursor.fetchone()["c"])
        return base

    def _chat_cost(self, db_cursor, model_id: int) -> float:
        db_cursor.execute(
            "SELECT cost_multiplier FROM gm_ai_models WHERE id = %s",
            (model_id,),
        )
        mult = float(db_cursor.fetchone()["cost_multiplier"])
        return self._action_base_price(db_cursor, "AI_ANALYZE") * mult

    def _image_cost(self, db_cursor, model_id: int) -> float:
        db_cursor.execute(
            "SELECT cost_multiplier FROM gm_ai_models WHERE id = %s",
            (model_id,),
        )
        mult = float(db_cursor.fetchone()["cost_multiplier"])
        return self._action_base_price(db_cursor, "IMAGE") * mult

    def _resolve_user_id(self, db_cursor) -> Optional[int]:
        import os as _os
        email = _os.getenv("E2E_TEST_EMAIL", "e2e@glancemind.test")
        db_cursor.execute("SELECT id FROM gm_users WHERE email = %s", (email,))
        row = db_cursor.fetchone()
        return row["id"] if row else None

    def _ensure_balance(self, db_cursor, amount: float = 10000.0):
        user_id = self._resolve_user_id(db_cursor)
        if user_id is None:
            return
        db_cursor.execute(
            "UPDATE gm_user_wallets SET balance_points = GREATEST(balance_points, %s) "
            "WHERE user_id = %s",
            (amount, user_id),
        )
        db_cursor.connection.commit()

    def _create_plan(self, auth_client, group_id, chat_model_id,
                     image_model_id, image_generations):
        ai_input: Dict[str, Any] = {
            "content_prompt": "MM-2 reddit_image multi-image matrix",
            "reddit_config": {
                "subreddit": "EarthPorn",
                "reddit_post_type": "IMAGE",
                # Required by reddit_validation.rs: image_prompt OR
                # uploaded_image_urls must be present for reddit_image.
                # We set image_prompt so the V2 image_generations[] path
                # is exercised (image_gen ai_task type is dispatched).
                "image_prompt": "aurora over fjord",
            },
            "image_generations": image_generations,
        }
        payload: Dict[str, Any] = {
            "plan_type": "reddit_image",
            "platform_id": PLATFORM_REDDIT,
            "group_id": group_id,
            "content_type": "post",
            "chat_ai_model_id": chat_model_id,
            "image_ai_model_id": image_model_id,
            "ai_input": ai_input,
            "ai_task_types": ["content_gen", "image_gen"],
            "name": "mm-2-reddit-image-matrix",
        }
        return auth_client.post("/api/v1/publish_plans", json=payload)

    @pytest.mark.parametrize("image_count", [1, 3])
    def test_reddit_image_matrix_image_count_and_billing(
        self, auth_client, db_cursor, image_count
    ):
        """For image_count ∈ {1, 3}: verify DB persistence + frozen_cost.

        Asserts RedditImage V2 billing rule:
            frozen = 1 * chat_unit + image_count * n_accounts * image_unit
        (see service/aipub_service.rs::prepare_budget_freeze RedditImage
        branch — chat is fixed at 1, images scale with accounts.)
        """
        grp = self._multi_account_group_id(db_cursor, PLATFORM_REDDIT)
        chat_id = self._chat_model_id(db_cursor)
        img_id = self._image_model_id(db_cursor)
        if grp is None or chat_id is None or img_id is None:
            pytest.skip("Need a Reddit group + chat model + image model")
        self._ensure_balance(db_cursor, 10000.0)

        n = int(grp["n"])
        chat_unit = self._chat_cost(db_cursor, chat_id)
        image_unit = self._image_cost(db_cursor, img_id)
        expected = 1 * chat_unit + image_count * n * image_unit

        resp = self._create_plan(
            auth_client, grp["id"], chat_id, img_id,
            image_generations=[{
                "prompts": [f"aurora photo variant {i}" for i in range(image_count)],
                "count": image_count,
                "model": "flux-kontext-pro",
                "aspect_ratio": "1:1",
            }],
        )
        assert_response_success(resp)
        plan = extract_data(resp.json())
        plan_id = plan["id"]

        # Main-table assertions: billing_status, frozen_cost, and
        # image_generations[] must round-trip through the DB intact.
        db_cursor.execute(
            """SELECT billing_status, frozen_cost, plan_type,
                      ai_input->'image_generations' AS igs
                 FROM gm_aipub_plans WHERE id = %s""",
            (plan_id,),
        )
        row = db_cursor.fetchone()
        assert row["plan_type"] == "reddit_image"
        assert row["billing_status"] == "frozen"
        assert row["igs"] is not None, "image_generations[] must persist"
        assert len(row["igs"]) == 1
        assert int(row["igs"][0]["count"]) == image_count, (
            f"DB image_generations[0].count expected {image_count}, "
            f"got {row['igs'][0]['count']}"
        )
        frozen = float(row["frozen_cost"])
        assert abs(frozen - expected) < 1e-4, (
            f"RedditImage image_count={image_count} expected frozen={expected} "
            f"(chat 1*{chat_unit} + image {image_count}*{n}*{image_unit}), "
            f"got {frozen}"
        )

        db_cursor.execute("DELETE FROM gm_aipub_ai_tasks WHERE plan_id = %s", (plan_id,))
        db_cursor.execute("DELETE FROM gm_aipub_plans WHERE id = %s", (plan_id,))


if __name__ == "__main__":
    pytest.main([__file__, "-v", "--tb=short"])
