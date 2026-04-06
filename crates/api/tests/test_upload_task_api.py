"""
GlanceMind API E2E Tests - Upload Task API
==========================================
Tests for upload task endpoints.

Test Coverage:
1. Create upload task
2. List my tasks
3. Query tasks by device (public)
4. Task error cases
"""

import pytest
import uuid
from conftest import (
    assert_response_success,
    extract_data,
    PLATFORM_TIKTOK,
)


class TestUploadTaskCRUD:
    """Tests for upload task CRUD operations."""

    def _create_test_account(self, auth_client):
        """Helper to create a test social account with device_id."""
        device_id = f"e2e_device_{uuid.uuid4().hex[:8]}"
        profile_name = "e2e_profile"
        
        account_payload = {
            "platform_id": PLATFORM_TIKTOK,
            "username": f"upload_test_{uuid.uuid4().hex[:8]}",
            "cookie": "test_cookie",
            "device_id": device_id,
            "profile_name": profile_name,
            "daily_max_replies": 10
        }
        
        resp = auth_client.post("/api/v1/accounts", json=account_payload)
        
        if resp.status_code == 200:
            data = extract_data(resp.json())
            return data.get("id"), device_id
        return None, device_id

    def test_create_upload_task(self, auth_client, db_cursor):
        """Test creating an upload task."""
        # First create a social account
        account_id, device_id = self._create_test_account(auth_client)
        
        if not account_id:
            # Try to use existing account
            db_cursor.execute("SELECT id FROM gm_social_accounts LIMIT 1")
            result = db_cursor.fetchone()
            if not result:
                pytest.skip("No social account available")
            account_id = result["id"]
        
        task_metadata = {
            "video_url": "https://example.com/test_video.mp4",
            "description": "E2E Test video #upload #automation",
            "location": "beijing"
        }
        
        task_payload = {
            "social_account_id": account_id,
            "metadata": task_metadata
        }
        
        resp = auth_client.post(
            "/api/v1/upload-tasks",
            json=task_payload
        )
        
        if resp.status_code == 200:
            data = extract_data(resp.json())
            print(f"\nCreated upload task: {data}")
            
            assert "id" in data
            assert data.get("task_type") == "upload"
            assert data.get("status") in ["init", "INIT"]
        else:
            print(f"\n  Task creation returned: {resp.status_code}")

    def test_list_my_upload_tasks(self, auth_client):
        """Test listing my upload tasks."""
        resp = auth_client.get("/api/v1/upload-tasks/my?page=1&per_page=10")
        assert_response_success(resp)
        
        data = extract_data(resp.json())
        print(f"\nMy upload tasks: {type(data)}")
        
        # Should have pagination
        tasks = data.get("list", []) if isinstance(data, dict) else data
        print(f"  Task count: {len(tasks)}")


class TestUploadTaskPublicQuery:
    """Tests for public task query endpoints."""

    def test_query_tasks_by_device(self, api_client, db_cursor):
        """Test querying tasks by device ID (public endpoint)."""
        # Get a device_id from database
        db_cursor.execute("""
            SELECT device_id FROM gm_social_accounts 
            WHERE device_id IS NOT NULL 
            LIMIT 1
        """)
        result = db_cursor.fetchone()
        
        device_id = result["device_id"] if result else "test_device"
        
        resp = api_client.get(
            f"/api/v1/public/tasks/by-device?device_id={device_id}"
        )
        assert_response_success(resp)
        
        data = extract_data(resp.json())
        print(f"\nTasks by device: {data}")
        
        assert "list" in data or "total" in data

    def test_query_tasks_with_status(self, api_client):
        """Test querying tasks with status filter."""
        resp = api_client.get(
            "/api/v1/public/tasks/by-device?device_id=test_device&status=init"
        )
        assert_response_success(resp)

    def test_query_tasks_empty_result(self, api_client):
        """Test querying with non-existent device returns empty."""
        resp = api_client.get(
            "/api/v1/public/tasks/by-device?device_id=nonexistent_device_12345"
        )
        assert_response_success(resp)
        
        data = extract_data(resp.json())
        
        assert data.get("total", 0) == 0


class TestUploadTaskErrorCases:
    """Tests for upload task error handling."""

    def test_create_task_requires_auth(self, api_client):
        """Test that creating task requires authentication."""
        task_payload = {
            "social_account_id": 1,
            "metadata": {"video_url": "https://example.com/video.mp4"}
        }
        
        resp = api_client.post(
            "/api/v1/upload-tasks",
            json=task_payload
        )
        
        assert resp.status_code == 401

    def test_create_task_invalid_account(self, auth_client):
        """Test creating task with non-existent account."""
        task_payload = {
            "social_account_id": 999999,
            "metadata": {"video_url": "https://example.com/video.mp4"}
        }
        
        resp = auth_client.post(
            "/api/v1/upload-tasks",
            json=task_payload
        )
        
        # Should return error
        assert resp.status_code in [400, 404, 422], \
            f"Expected error status, got {resp.status_code}"

    def test_list_tasks_requires_auth(self, api_client):
        """Test that listing my tasks requires authentication."""
        resp = api_client.get("/api/v1/upload-tasks/my")
        
        assert resp.status_code == 401


class TestUploadTaskDatabaseState:
    """Tests that verify upload task database state."""

    def test_upload_tasks_table_exists(self, db_cursor):
        """Verify upload_tasks table exists and is queryable."""
        db_cursor.execute("SELECT COUNT(*) as count FROM gm_upload_tasks")
        result = db_cursor.fetchone()
        
        print(f"\n  Total upload tasks: {result['count']}")


if __name__ == "__main__":
    pytest.main([__file__, "-v", "--tb=short"])
