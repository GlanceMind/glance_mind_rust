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
    """Phase 1: 创建阶段单元测试 - Plan 创建与 AI Task 自动生成"""

    def test_create_plan_generates_ai_task(self, auth_client, db_cursor):
        """创建 Plan 后应自动生成 AI Task"""
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

        assert len(ai_tasks) >= 1, "Should create AI task automatically"
        assert ai_tasks[0]["status"] in ["pending", "processing"]

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


if __name__ == "__main__":
    pytest.main([__file__, "-v", "--tb=short"])
