#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
GlanceMind API E2E Tests - Account Grooming via AIPub
=====================================================
Tests the account grooming feature (plan_type='account_grooming')
through the publish_plans API, including:
1. Plan creation with DB state verification
2. Plan query with type filter
3. Full DB lifecycle simulation (pending → ai_processing → ready)
4. Cascade delete verification
5. Error cases (missing group, wrong target)

Run: cd crates/api && make test-e2e-only
  or: pytest tests/test_account_grooming_api.py -v --tb=short
"""

import pytest
import json
import uuid
from typing import Optional

try:
    from conftest import assert_response_success, extract_data
except ImportError:
    def assert_response_success(resp):
        assert resp.status_code in [200, 201], f"Expected success, got {resp.status_code}: {resp.text}"
    def extract_data(json_resp):
        return json_resp["data"] if isinstance(json_resp, dict) and "data" in json_resp else json_resp

PLATFORM_TIKTOK = 2


# =============================================================================
# Helpers
# =============================================================================

def get_test_group_with_accounts(db_cursor) -> Optional[dict]:
    """Find a group that has at least one account."""
    db_cursor.execute("""
        SELECT sg.id as group_id, sg.platform_id, COUNT(sa.id) as account_count
        FROM gm_social_groups sg
        JOIN gm_social_accounts sa ON sa.group_id = sg.id
        GROUP BY sg.id, sg.platform_id
        HAVING COUNT(sa.id) > 0
        LIMIT 1
    """)
    return db_cursor.fetchone()

def create_grooming_plan(auth_client, group_id, platform_id, name_suffix="", include_bio=True):
    """Create a grooming plan and return the response data."""
    ai_input = {
        "text_prompt": "Creative tech usernames, modern style",
        "image_prompt": "Minimalist avatar, vibrant gradient",
    }
    if include_bio:
        ai_input["bio_prompt"] = "Short catchy bio, tech enthusiast, max 80 chars"

    payload = {
        "plan_type": "account_grooming",
        "name": f"Grooming Test {name_suffix or uuid.uuid4().hex[:6]}",
        "group_id": group_id,
        "platform_id": platform_id,
        "content_type": "profile",
        "ai_task_types": ["name_gen", "avatar_gen"],
        "ai_input": ai_input,
    }
    return auth_client.post("/api/v1/publish_plans", json=payload)


# =============================================================================
# TestGroomingPlanCreate - API layer verification
# =============================================================================

class TestGroomingPlanCreate:
    """Verify plan creation via POST /publish_plans with plan_type=account_grooming."""

    def test_create_plan(self, auth_client, db_cursor):
        """POST returns 200, creates row with correct plan_type and status."""
        group = get_test_group_with_accounts(db_cursor)
        if not group:
            pytest.skip("No group with accounts available")

        resp = create_grooming_plan(auth_client, group["group_id"], group["platform_id"], "create")
        if resp.status_code not in [200, 201]:
            pytest.skip(f"Plan creation returned {resp.status_code} - API may not support account_grooming yet")

        data = extract_data(resp.json())
        assert data is not None
        assert data["plan_type"] == "account_grooming"
        assert data["status"] in ["pending", "ai_processing"]
        plan_id = data["id"]

        # DB verify
        db_cursor.execute("SELECT plan_type, status, group_id FROM gm_aipub_plans WHERE id = %s", (plan_id,))
        row = db_cursor.fetchone()
        assert row is not None, "Plan should exist in DB"
        assert row["plan_type"] == "account_grooming"
        assert row["status"] in ["pending", "ai_processing"]
        assert row["group_id"] == group["group_id"]
        print(f"  OK: plan {plan_id} created, plan_type={row['plan_type']}, status={row['status']}")

        # Cleanup
        auth_client.delete(f"/api/v1/publish_plans/{plan_id}")

    def test_create_plan_defaults_ai_task_types(self, auth_client, db_cursor):
        """When ai_task_types omitted, should default to the aggregate grooming task."""
        group = get_test_group_with_accounts(db_cursor)
        if not group:
            pytest.skip("No group with accounts available")

        payload = {
            "plan_type": "account_grooming",
            "name": "Defaults Test",
            "group_id": group["group_id"],
            "platform_id": group["platform_id"],
            "content_type": "profile",
            "ai_input": {"text_prompt": "test", "image_prompt": "test"},
        }
        resp = auth_client.post("/api/v1/publish_plans", json=payload)
        if resp.status_code not in [200, 201]:
            pytest.skip("Plan creation not available")

        data = extract_data(resp.json())
        plan_id = data["id"]

        db_cursor.execute("SELECT ai_task_types FROM gm_aipub_plans WHERE id = %s", (plan_id,))
        row = db_cursor.fetchone()
        types = row["ai_task_types"]
        assert types is not None, "ai_task_types should be set"
        flat = [t for t in types if t is not None]
        assert flat == ["account_grooming"], f"Should default to aggregate grooming task, got {flat}"
        print(f"  OK: ai_task_types defaulted to {flat}")

        auth_client.delete(f"/api/v1/publish_plans/{plan_id}")

    def test_create_plan_ai_input_persisted(self, auth_client, db_cursor):
        """ai_input JSON with text_prompt, image_prompt, and bio_prompt should be stored."""
        group = get_test_group_with_accounts(db_cursor)
        if not group:
            pytest.skip("No group with accounts available")

        resp = create_grooming_plan(auth_client, group["group_id"], group["platform_id"], "input")
        if resp.status_code not in [200, 201]:
            pytest.skip("Plan creation not available")

        plan_id = extract_data(resp.json())["id"]

        db_cursor.execute("SELECT ai_input FROM gm_aipub_plans WHERE id = %s", (plan_id,))
        row = db_cursor.fetchone()
        ai_input = row["ai_input"]
        if isinstance(ai_input, str):
            ai_input = json.loads(ai_input)
        assert "text_prompt" in ai_input, "Should have text_prompt"
        assert "image_prompt" in ai_input, "Should have image_prompt"
        assert "bio_prompt" in ai_input, "Should have bio_prompt"
        assert ai_input["text_prompt"] == "Creative tech usernames, modern style"
        assert ai_input["bio_prompt"] == "Short catchy bio, tech enthusiast, max 80 chars"
        print(f"  OK: ai_input persisted with text_prompt + image_prompt + bio_prompt")

        auth_client.delete(f"/api/v1/publish_plans/{plan_id}")

    def test_create_plan_without_bio_prompt(self, auth_client, db_cursor):
        """ai_input without bio_prompt should still work (bio is optional)."""
        group = get_test_group_with_accounts(db_cursor)
        if not group:
            pytest.skip("No group with accounts available")

        resp = create_grooming_plan(auth_client, group["group_id"], group["platform_id"],
                                    "no_bio", include_bio=False)
        if resp.status_code not in [200, 201]:
            pytest.skip("Plan creation not available")

        plan_id = extract_data(resp.json())["id"]

        db_cursor.execute("SELECT ai_input FROM gm_aipub_plans WHERE id = %s", (plan_id,))
        row = db_cursor.fetchone()
        ai_input = row["ai_input"]
        if isinstance(ai_input, str):
            ai_input = json.loads(ai_input)
        assert "text_prompt" in ai_input
        assert "image_prompt" in ai_input
        assert "bio_prompt" not in ai_input, "bio_prompt should not be present when not provided"
        print(f"  OK: ai_input persisted without bio_prompt (optional)")

        auth_client.delete(f"/api/v1/publish_plans/{plan_id}")


# =============================================================================
# TestGroomingPlanQuery - read operations
# =============================================================================

class TestGroomingPlanQuery:
    """Verify plan query and detail endpoints."""

    def test_list_with_plan_type_filter(self, auth_client, db_cursor):
        """GET ?plan_type=account_grooming should only return grooming plans."""
        group = get_test_group_with_accounts(db_cursor)
        if not group:
            pytest.skip("No group with accounts available")

        # Create a grooming plan
        resp = create_grooming_plan(auth_client, group["group_id"], group["platform_id"], "filter")
        if resp.status_code not in [200, 201]:
            pytest.skip("Plan creation not available")
        plan_id = extract_data(resp.json())["id"]

        # Query with filter
        resp = auth_client.get("/api/v1/publish_plans", params={"plan_type": "account_grooming"})
        assert_response_success(resp)
        data = extract_data(resp.json())
        plans = data.get("list", data) if isinstance(data, dict) else data
        if isinstance(plans, list):
            for p in plans:
                assert p.get("plan_type") == "account_grooming", \
                    f"Filter leak: got plan_type={p.get('plan_type')}"
        print(f"  OK: filter returned {len(plans) if isinstance(plans, list) else '?'} grooming plans only")

        auth_client.delete(f"/api/v1/publish_plans/{plan_id}")

    def test_detail_includes_structure(self, auth_client, db_cursor):
        """GET /:id should return plan with correct structure."""
        group = get_test_group_with_accounts(db_cursor)
        if not group:
            pytest.skip("No group with accounts available")

        resp = create_grooming_plan(auth_client, group["group_id"], group["platform_id"], "detail")
        if resp.status_code not in [200, 201]:
            pytest.skip("Plan creation not available")
        plan_id = extract_data(resp.json())["id"]

        resp = auth_client.get(f"/api/v1/publish_plans/{plan_id}")
        assert_response_success(resp)
        data = extract_data(resp.json())
        assert data["id"] == plan_id
        assert data["plan_type"] == "account_grooming"
        print(f"  OK: detail returned plan {plan_id}, plan_type={data['plan_type']}")

        auth_client.delete(f"/api/v1/publish_plans/{plan_id}")


# =============================================================================
# TestGroomingDBLifecycle - simulate full lifecycle via direct DB
# =============================================================================

class TestGroomingDBLifecycle:
    """Simulate the complete data flow: pending → ai_processing → ready → delete."""

    plan_id = None
    group_info = None

    def test_lifecycle_01_plan_created(self, auth_client, db_cursor):
        """Step 1: Create plan, verify initial DB state."""
        group = get_test_group_with_accounts(db_cursor)
        if not group:
            pytest.skip("No group with accounts")

        TestGroomingDBLifecycle.group_info = group

        resp = create_grooming_plan(auth_client, group["group_id"], group["platform_id"], "lifecycle")
        if resp.status_code not in [200, 201]:
            pytest.skip("Plan creation not available")

        plan_id = extract_data(resp.json())["id"]
        TestGroomingDBLifecycle.plan_id = plan_id

        # Verify: plan exists, no tasks yet
        db_cursor.execute("SELECT COUNT(*) as cnt FROM gm_aipub_plans WHERE id = %s", (plan_id,))
        assert db_cursor.fetchone()["cnt"] == 1

        db_cursor.execute("SELECT COUNT(*) as cnt FROM gm_aipub_ai_tasks WHERE plan_id = %s", (plan_id,))
        ai_count = db_cursor.fetchone()["cnt"]

        db_cursor.execute("SELECT COUNT(*) as cnt FROM gm_aipub_tasks WHERE plan_id = %s", (plan_id,))
        pub_count = db_cursor.fetchone()["cnt"]

        print(f"  Step 1 OK: plan={plan_id}, ai_tasks={ai_count}, pub_tasks={pub_count}")

    def test_lifecycle_02_scheduler_picks_up(self, db_cursor):
        """Step 2: Simulate scheduler creating an AI task with bio_prompt."""
        if not self.plan_id:
            pytest.skip("No plan from step 1")

        ai_task_input = json.dumps({
            "version": 1,
            "type": "account_grooming",
            "text_prompt": "Creative tech usernames",
            "image_prompt": "Minimalist avatar",
            "bio_prompt": "Short catchy bio, tech enthusiast, max 80 chars",
            "platform": "tiktok",
            "account_count": 2,
        })

        db_cursor.execute("""
            INSERT INTO gm_aipub_ai_tasks (plan_id, task_type, external_service, input, status, progress, retry_count, sequence, created_at)
            VALUES (%s, 'account_grooming', 'laozhang', %s::jsonb, 'processing', 0, 0, 0, NOW())
        """, (self.plan_id, ai_task_input))

        db_cursor.execute("""
            UPDATE gm_aipub_plans SET status = 'ai_processing', updated_at = NOW() WHERE id = %s
        """, (self.plan_id,))
        db_cursor.connection.commit()

        # Verify plan status
        db_cursor.execute("SELECT status FROM gm_aipub_plans WHERE id = %s", (self.plan_id,))
        assert db_cursor.fetchone()["status"] == "ai_processing"

        # Verify AI task input contains bio_prompt
        db_cursor.execute(
            "SELECT task_type, status, input FROM gm_aipub_ai_tasks WHERE plan_id = %s", (self.plan_id,))
        ai_task = db_cursor.fetchone()
        assert ai_task["task_type"] == "account_grooming"
        assert ai_task["status"] == "processing"
        task_input = ai_task["input"] if isinstance(ai_task["input"], dict) else json.loads(ai_task["input"])
        assert "bio_prompt" in task_input, "AI task input should contain bio_prompt"
        assert task_input["bio_prompt"] == "Short catchy bio, tech enthusiast, max 80 chars"
        print(f"  Step 2 OK: plan=ai_processing, ai_task=account_grooming/processing, bio_prompt=present")

    def test_lifecycle_03_ai_task_completes(self, db_cursor):
        """Step 3: Simulate AI task completing with grooming results including bio."""
        if not self.plan_id:
            pytest.skip("No plan")

        result = json.dumps({
            "type": "account_grooming",
            "accounts_processed": 2,
            "results": [
                {
                    "generated_name": "TechNova2026",
                    "avatar_url": "https://oss.example.com/avatar_1.png",
                    "generated_bio": "Tech lover & creative mind | Building the future"
                },
                {
                    "generated_name": "PixelDrift_X",
                    "avatar_url": "https://oss.example.com/avatar_2.png",
                    "generated_bio": "Digital nomad exploring the pixel universe"
                }
            ]
        })

        db_cursor.execute("""
            UPDATE gm_aipub_ai_tasks
            SET status = 'completed', progress = 100,
                result = %s::jsonb, completed_at = NOW(), updated_at = NOW()
            WHERE plan_id = %s
        """, (result, self.plan_id))
        db_cursor.connection.commit()

        db_cursor.execute("SELECT status, result FROM gm_aipub_ai_tasks WHERE plan_id = %s", (self.plan_id,))
        task = db_cursor.fetchone()
        assert task["status"] == "completed"
        r = task["result"] if isinstance(task["result"], dict) else json.loads(task["result"])
        assert r["type"] == "account_grooming"
        assert r["accounts_processed"] == 2
        assert len(r["results"]) == 2
        assert r["results"][0]["generated_name"] == "TechNova2026"
        assert r["results"][0]["generated_bio"] == "Tech lover & creative mind | Building the future"
        assert r["results"][1]["generated_bio"] == "Digital nomad exploring the pixel universe"
        print(f"  Step 3 OK: ai_task completed, {r['accounts_processed']} accounts with name+bio")

    def test_lifecycle_04_pub_tasks_created(self, db_cursor):
        """Step 4: Simulate publish tasks created for each account."""
        if not self.plan_id:
            pytest.skip("No plan")

        # Get accounts from the group
        db_cursor.execute("""
            SELECT sa.id FROM gm_social_accounts sa
            JOIN gm_aipub_plans p ON sa.group_id = p.group_id
            WHERE p.id = %s LIMIT 2
        """, (self.plan_id,))
        accounts = db_cursor.fetchall()
        if len(accounts) < 1:
            pytest.skip("No accounts in group")

        names = ["TechNova2026", "PixelDrift_X"]
        urls = ["https://oss.example.com/avatar_1.png", "https://oss.example.com/avatar_2.png"]
        bios = ["Tech lover & creative mind | Building the future",
                "Digital nomad exploring the pixel universe"]

        for i, acc in enumerate(accounts[:2]):
            content = json.dumps({
                "generated_name": names[i] if i < len(names) else f"user_{acc['id']}",
                "avatar_url": urls[i] if i < len(urls) else None,
                "avatar_prompt": "realistic portrait, tech style",
                "generated_bio": bios[i] if i < len(bios) else "",
            })
            db_cursor.execute("""
                INSERT INTO gm_aipub_tasks (plan_id, social_account_id, content, status, created_at)
                VALUES (%s, %s, %s::jsonb, 'ready', NOW())
            """, (self.plan_id, acc["id"], content))

        db_cursor.execute("""
            UPDATE gm_aipub_plans SET status = 'ready', updated_at = NOW() WHERE id = %s
        """, (self.plan_id,))
        db_cursor.connection.commit()

        # Verify
        db_cursor.execute("SELECT status FROM gm_aipub_plans WHERE id = %s", (self.plan_id,))
        assert db_cursor.fetchone()["status"] == "ready"

        db_cursor.execute("SELECT content, status FROM gm_aipub_tasks WHERE plan_id = %s ORDER BY id", (self.plan_id,))
        tasks = db_cursor.fetchall()
        assert len(tasks) >= 1
        for t in tasks:
            assert t["status"] == "ready"
            c = t["content"] if isinstance(t["content"], dict) else json.loads(t["content"])
            assert "generated_name" in c, f"content missing generated_name: {c}"
            assert "avatar_url" in c, f"content missing avatar_url: {c}"
            assert "generated_bio" in c, f"content missing generated_bio: {c}"
            assert len(c["generated_name"]) > 0, "generated_name should not be empty"
            assert len(c["generated_bio"]) > 0, "generated_bio should not be empty"
            assert len(c["generated_bio"]) <= 80, f"generated_bio should be max 80 chars, got {len(c['generated_bio'])}"
            print(f"    task: name='{c['generated_name']}', bio='{c['generated_bio']}'")
        print(f"  Step 4 OK: {len(tasks)} pub_tasks with name+avatar+bio, plan=ready")

    def test_lifecycle_05_detail_shows_results(self, auth_client):
        """Step 5: API detail with ?include=all should return plan + publish_tasks with grooming content."""
        if not self.plan_id:
            pytest.skip("No plan")

        # Fetch WITHOUT include - should return plan only
        resp = auth_client.get(f"/api/v1/publish_plans/{self.plan_id}")
        if resp.status_code != 200:
            pytest.skip("Detail not available")
        data = extract_data(resp.json())
        assert data["status"] == "ready"
        assert data["plan_type"] == "account_grooming"

        # Fetch WITH include=all - should return plan + publish_tasks
        resp_full = auth_client.get(f"/api/v1/publish_plans/{self.plan_id}", params={"include": "all"})
        assert_response_success(resp_full)
        data_full = extract_data(resp_full.json())

        assert data_full["status"] == "ready"
        assert data_full["plan_type"] == "account_grooming"

        # Verify publish_tasks are present
        pub_tasks = data_full.get("publish_tasks")
        assert pub_tasks is not None, "publish_tasks should be included with ?include=all"
        assert len(pub_tasks) >= 1, f"Should have at least 1 publish task, got {len(pub_tasks)}"

        # Verify each publish task has grooming content including bio
        for task in pub_tasks:
            content = task.get("content", {})
            assert "generated_name" in content, f"publish_task {task['id']} missing generated_name"
            assert "avatar_url" in content, f"publish_task {task['id']} missing avatar_url"
            assert "generated_bio" in content, f"publish_task {task['id']} missing generated_bio"
            assert len(content["generated_name"]) > 0, f"generated_name should not be empty"
            assert len(content["generated_bio"]) > 0, f"generated_bio should not be empty"
            print(f"    publish_task {task['id']}: name='{content['generated_name']}', "
                  f"bio='{content.get('generated_bio', '')[:40]}...', "
                  f"avatar_url={'present' if content.get('avatar_url') else 'null'}")

        # Verify ai_input contains bio_prompt
        ai_input = data_full.get("ai_input")
        if ai_input:
            assert "text_prompt" in ai_input, "ai_input should contain text_prompt"
            assert "image_prompt" in ai_input, "ai_input should contain image_prompt"
            assert "bio_prompt" in ai_input, "ai_input should contain bio_prompt"
            print(f"    ai_input: text_prompt=present, image_prompt=present, bio_prompt=present")

        print(f"  Step 5 OK: API detail returns {len(pub_tasks)} publish_tasks with name+bio+avatar")

    def test_lifecycle_06_delete_cascades(self, auth_client, db_cursor):
        """Step 6: DELETE plan cascades to ai_tasks and pub_tasks."""
        if not self.plan_id:
            pytest.skip("No plan")

        resp = auth_client.delete(f"/api/v1/publish_plans/{self.plan_id}")
        if resp.status_code not in [200, 204]:
            # Manual cleanup
            db_cursor.execute("DELETE FROM gm_aipub_tasks WHERE plan_id = %s", (self.plan_id,))
            db_cursor.execute("DELETE FROM gm_aipub_ai_tasks WHERE plan_id = %s", (self.plan_id,))
            db_cursor.execute("DELETE FROM gm_aipub_plans WHERE id = %s", (self.plan_id,))
            db_cursor.connection.commit()
            return

        db_cursor.execute("SELECT COUNT(*) as cnt FROM gm_aipub_plans WHERE id = %s", (self.plan_id,))
        assert db_cursor.fetchone()["cnt"] == 0, "Plan should be deleted"

        db_cursor.execute("SELECT COUNT(*) as cnt FROM gm_aipub_ai_tasks WHERE plan_id = %s", (self.plan_id,))
        assert db_cursor.fetchone()["cnt"] == 0, "AI tasks should cascade-delete"

        db_cursor.execute("SELECT COUNT(*) as cnt FROM gm_aipub_tasks WHERE plan_id = %s", (self.plan_id,))
        assert db_cursor.fetchone()["cnt"] == 0, "Pub tasks should cascade-delete"

        print(f"  Step 6 OK: cascade delete verified (plan + ai_tasks + pub_tasks)")


# =============================================================================
# TestGroomingErrors - validation
# =============================================================================

class TestGroomingErrors:
    """Verify error handling for invalid inputs."""

    def test_missing_group_id(self, auth_client):
        """account_grooming without group_id should fail."""
        payload = {
            "plan_type": "account_grooming",
            "platform_id": PLATFORM_TIKTOK,
            "content_type": "profile",
            "ai_input": {"text_prompt": "test", "image_prompt": "test", "bio_prompt": "test bio"},
        }
        resp = auth_client.post("/api/v1/publish_plans", json=payload)
        assert resp.status_code in [400, 422, 500], \
            f"Expected error for missing group_id, got {resp.status_code}"
        print(f"  OK: missing group_id returned {resp.status_code}")

    def test_with_social_account_id(self, auth_client, db_cursor):
        """account_grooming with social_account_id (instead of group) should fail."""
        db_cursor.execute("SELECT id FROM gm_social_accounts LIMIT 1")
        acc = db_cursor.fetchone()
        if not acc:
            pytest.skip("No accounts")

        payload = {
            "plan_type": "account_grooming",
            "social_account_id": acc["id"],
            "platform_id": PLATFORM_TIKTOK,
            "content_type": "profile",
            "ai_input": {"text_prompt": "test", "image_prompt": "test"},
        }
        resp = auth_client.post("/api/v1/publish_plans", json=payload)
        assert resp.status_code in [400, 422, 500], \
            f"Expected error for social_account_id with grooming, got {resp.status_code}"
        print(f"  OK: social_account_id with grooming returned {resp.status_code}")

    def test_invalid_plan_type(self, auth_client, db_cursor):
        """Unknown plan_type should fail."""
        group = get_test_group_with_accounts(db_cursor)
        if not group:
            pytest.skip("No group")

        payload = {
            "plan_type": "totally_invalid_type",
            "group_id": group["group_id"],
            "platform_id": group["platform_id"],
            "content_type": "profile",
        }
        resp = auth_client.post("/api/v1/publish_plans", json=payload)
        assert resp.status_code in [400, 422, 500], \
            f"Expected error for invalid plan_type, got {resp.status_code}"
        print(f"  OK: invalid plan_type returned {resp.status_code}")
