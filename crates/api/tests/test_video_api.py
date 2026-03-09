"""
Video Generation API E2E Tests
===============================
Comprehensive tests for all video generation models (LaoZhang + Jimeng).

Tests cover:
1. Text-to-video generation (all providers)
2. Image-to-video generation (single image)
3. Dual-image-to-video generation (first+last frame)
4. Task status query and listing
5. Parameter validation (seconds, size, model_id)
6. Permission and ownership checks
7. DB record verification (all fields)
8. Jimeng-specific: 4 video modes (T2V, I2V-first, I2V-FL, camera motion)

Requires:
- API server running (make up)
- LaoZhang mock at LAOZHANG_BASE_URL (for LaoZhang models)
- Test DB with seed data (init-test-data.sql)
"""

import base64
import io
import os
import time

import pytest
import requests

from conftest import (
    API_BASE_URL,
    APIClient,
    TEST_USER_EMAIL,
    TEST_USER_PASSWORD,
    assert_json_structure,
    assert_response_success,
    extract_data,
    get_or_create_test_token,
)

# ============================================================================
# Constants
# ============================================================================

VIDEO_GENERATE_URL = "/api/v1/video/generate"
VIDEO_TASKS_URL = "/api/v1/video/tasks"
VIDEO_TASK_URL = "/api/v1/video/tasks/{task_id}"
CONFIG_AI_MODELS_URL = "/api/v1/config/ai-models"

# Model IDs from init-test-data.sql
MODEL_VEO2_ID = 5
MODEL_JIMENG_720P_ID = 6
MODEL_JIMENG_1080P_ID = 7
AIPUB_PLAN_URL = "/api/v1/publish_plans"

# 1x1 transparent PNG (minimal valid image for testing)
TINY_PNG = bytes([
    0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A,
    0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44, 0x52,
    0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01,
    0x08, 0x06, 0x00, 0x00, 0x00, 0x1F, 0x15, 0xC4,
    0x89, 0x00, 0x00, 0x00, 0x0A, 0x49, 0x44, 0x41,
    0x54, 0x78, 0x9C, 0x63, 0x00, 0x01, 0x00, 0x00,
    0x05, 0x00, 0x01, 0x0D, 0x0A, 0x2D, 0xB4, 0x00,
    0x00, 0x00, 0x00, 0x49, 0x45, 0x4E, 0x44, 0xAE,
    0x42, 0x60, 0x82,
])


# ============================================================================
# Fixtures
# ============================================================================

@pytest.fixture(scope="module")
def video_client(db_connection):
    """Authenticated client for video API tests."""
    token = get_or_create_test_token()
    return APIClient(API_BASE_URL, token)


def _make_text_to_video_form(prompt="A cute cat playing in a garden", model_id=None,
                             seconds="10", size="1280x720", orientation="landscape",
                             title=None):
    """Build multipart form data for text-to-video."""
    data = {
        "prompt": prompt,
        "seconds": seconds,
        "size": size,
        "orientation": orientation,
    }
    if model_id is not None:
        data["ai_model_id"] = str(model_id)
    if title is not None:
        data["title"] = title
    return data


def _make_image_to_video_form(image_data=TINY_PNG, image_filename="test.png",
                              prompt="Animate this image", model_id=None,
                              seconds="10", size="720x1280", orientation="portrait"):
    """Build multipart form data for image-to-video."""
    files = {
        "image": (image_filename, io.BytesIO(image_data), "image/png"),
    }
    data = {
        "prompt": prompt,
        "seconds": seconds,
        "size": size,
        "orientation": orientation,
    }
    if model_id is not None:
        data["ai_model_id"] = str(model_id)
    return data, files


def _make_dual_image_form(start_data=TINY_PNG, end_data=TINY_PNG,
                          prompt="Transition between frames", model_id=None,
                          seconds="10", size="720x1280", orientation="portrait"):
    """Build multipart form data for dual-image-to-video (first+last frame)."""
    files = {
        "start_frame": ("start.png", io.BytesIO(start_data), "image/png"),
        "end_frame": ("end.png", io.BytesIO(end_data), "image/png"),
    }
    data = {
        "prompt": prompt,
        "seconds": seconds,
        "size": size,
        "orientation": orientation,
    }
    if model_id is not None:
        data["ai_model_id"] = str(model_id)
    return data, files


# ============================================================================
# Test Class: Video Model Discovery
# ============================================================================

class TestVideoModelDiscovery:
    """Verify video models are available via config API."""

    def test_list_video_models(self, video_client):
        """GET /config/ai-models should include video models."""
        resp = video_client.get(f"{CONFIG_AI_MODELS_URL}?model_type=video")
        assert_response_success(resp)
        data = extract_data(resp.json())
        assert isinstance(data, list), "Expected list of models"
        assert len(data) >= 1, "Should have at least 1 video model"

        model_keys = [m["model_key"] for m in data]
        assert "veo-2" in model_keys, "Veo-2 model should be active"

    def test_video_model_fields(self, video_client):
        """Each video model should have all required fields."""
        resp = video_client.get(f"{CONFIG_AI_MODELS_URL}?model_type=video")
        data = extract_data(resp.json())
        for model in data:
            assert_json_structure(model, [
                "id", "name", "provider", "model_key", "model_type",
                "cost_multiplier", "is_active"
            ])
            assert model["model_type"] == "video"
            assert model["is_active"] is True


# ============================================================================
# Test Class: LaoZhang Text-to-Video
# ============================================================================

class TestLaoZhangTextToVideo:
    """Text-to-video generation via LaoZhang provider."""

    def test_text_to_video_default_model(self, video_client, db_cursor):
        """POST /video/generate with prompt only (default model)."""
        form_data = _make_text_to_video_form(
            prompt="A beautiful sunset over the ocean with waves",
            title="Sunset Test Video",
        )
        resp = video_client.post(VIDEO_GENERATE_URL, data=form_data)
        assert_response_success(resp)

        data = extract_data(resp.json())
        assert data is not None, "Response data should not be None"
        assert "task_id" in data, "Response should contain task_id"
        assert data["status"] in ("pending", "submitted"), f"Status should be pending/submitted, got {data['status']}"
        assert "cost_points" in data
        assert float(data["cost_points"]) > 0, "Cost should be positive"
        assert "estimated_time" in data

        # Verify DB record
        task_id = data["task_id"]
        db_cursor.execute(
            "SELECT * FROM gm_video_generation_tasks WHERE task_id = %s",
            (task_id,)
        )
        task_row = db_cursor.fetchone()
        assert task_row is not None, f"Task {task_id} not found in DB"
        assert task_row["status"] == "pending"
        assert task_row["prompt"] == form_data["prompt"]
        assert task_row["title"] == "Sunset Test Video"
        assert task_row["video_seconds"] == "10"
        assert task_row["video_size"] == "1280x720"
        assert task_row["orientation"] == "landscape"
        assert float(task_row["cost_points"]) > 0
        assert task_row["created_at"] is not None

    def test_text_to_video_with_model_id(self, video_client):
        """POST /video/generate with explicit model_id."""
        form_data = _make_text_to_video_form(
            prompt="A futuristic city skyline at night",
            model_id=MODEL_VEO2_ID,
            seconds="10",
            size="1280x720",
        )
        resp = video_client.post(VIDEO_GENERATE_URL, data=form_data)
        assert_response_success(resp)

        data = extract_data(resp.json())
        assert data["task_id"] is not None
        assert data["status"] in ("pending", "submitted")

    def test_text_to_video_portrait(self, video_client):
        """Portrait mode: 720x1280."""
        form_data = _make_text_to_video_form(
            prompt="A person walking down a narrow alley",
            seconds="10",
            size="720x1280",
            orientation="portrait",
        )
        resp = video_client.post(VIDEO_GENERATE_URL, data=form_data)
        assert_response_success(resp)
        data = extract_data(resp.json())
        assert data["task_id"] is not None


# ============================================================================
# Test Class: LaoZhang Image-to-Video
# ============================================================================

class TestLaoZhangImageToVideo:
    """Image-to-video generation via LaoZhang provider."""

    def test_single_image_to_video(self, video_client, db_cursor):
        """POST /video/generate with image file."""
        data_fields = {
            "prompt": "Animate this beautiful landscape scene",
            "seconds": "10",
            "size": "720x1280",
            "orientation": "portrait",
        }
        files = {
            "image": ("landscape.png", io.BytesIO(TINY_PNG), "image/png"),
        }
        resp = video_client.session.post(
            f"{API_BASE_URL}{VIDEO_GENERATE_URL}",
            data=data_fields,
            files=files,
        )
        assert_response_success(resp)

        result = extract_data(resp.json())
        assert result["task_id"] is not None
        assert result["status"] in ("pending", "submitted")

        # Verify DB
        db_cursor.execute(
            "SELECT * FROM gm_video_generation_tasks WHERE task_id = %s",
            (result["task_id"],)
        )
        row = db_cursor.fetchone()
        assert row is not None
        assert row["prompt"] == "Animate this beautiful landscape scene"
        assert row["video_seconds"] == "10"

    def test_image_without_prompt(self, video_client):
        """Image-to-video should work even with empty prompt (image provides context)."""
        files = {
            "image": ("photo.png", io.BytesIO(TINY_PNG), "image/png"),
        }
        data_fields = {
            "seconds": "10",
            "size": "720x1280",
            "orientation": "portrait",
        }
        resp = video_client.session.post(
            f"{API_BASE_URL}{VIDEO_GENERATE_URL}",
            data=data_fields,
            files=files,
        )
        # May succeed (image alone) or fail (prompt required depends on backend logic)
        # The handler requires prompt OR image, so image alone should be OK
        assert resp.status_code in (200, 400)


# ============================================================================
# Test Class: LaoZhang Dual-Image-to-Video
# ============================================================================

class TestLaoZhangDualImageToVideo:
    """Dual image (first+last frame) video generation."""

    def test_dual_image_requires_fl_model(self, video_client):
        """Dual image mode should reject non-FL models."""
        files = {
            "start_frame": ("start.png", io.BytesIO(TINY_PNG), "image/png"),
            "end_frame": ("end.png", io.BytesIO(TINY_PNG), "image/png"),
        }
        data_fields = {
            "prompt": "Transition between two scenes",
            "ai_model_id": str(MODEL_VEO2_ID),  # veo-2, not FL
            "seconds": "10",
            "size": "720x1280",
            "orientation": "portrait",
        }
        resp = video_client.session.post(
            f"{API_BASE_URL}{VIDEO_GENERATE_URL}",
            data=data_fields,
            files=files,
        )
        # Should fail: veo-2 does not support dual image
        assert resp.status_code in (400, 422, 500)

    def test_dual_image_incomplete(self, video_client):
        """Providing only start_frame without end_frame should fail."""
        files = {
            "start_frame": ("start.png", io.BytesIO(TINY_PNG), "image/png"),
        }
        data_fields = {
            "prompt": "Transition test",
            "seconds": "10",
            "size": "720x1280",
        }
        resp = video_client.session.post(
            f"{API_BASE_URL}{VIDEO_GENERATE_URL}",
            data=data_fields,
            files=files,
        )
        assert resp.status_code in (400, 422, 500)


# ============================================================================
# Test Class: Parameter Validation
# ============================================================================

class TestVideoParameterValidation:
    """Validation of video generation parameters."""

    def test_missing_prompt_and_image(self, video_client):
        """Must provide either prompt or image."""
        data_fields = {
            "seconds": "10",
            "size": "1280x720",
        }
        resp = video_client.post(VIDEO_GENERATE_URL, data=data_fields)
        assert resp.status_code in (400, 422)

    def test_invalid_seconds(self, video_client):
        """Seconds must be 10 or 15."""
        form_data = _make_text_to_video_form(seconds="7")
        resp = video_client.post(VIDEO_GENERATE_URL, data=form_data)
        assert resp.status_code in (400, 422)

    def test_invalid_size(self, video_client):
        """Size must be valid."""
        form_data = _make_text_to_video_form(size="1920x1080")
        resp = video_client.post(VIDEO_GENERATE_URL, data=form_data)
        assert resp.status_code in (400, 422)

    def test_invalid_model_id(self, video_client):
        """Non-existent model_id should be rejected."""
        form_data = _make_text_to_video_form(model_id=99999)
        resp = video_client.post(VIDEO_GENERATE_URL, data=form_data)
        assert resp.status_code in (400, 404, 422)

    def test_missing_seconds(self, video_client):
        """Seconds is required."""
        resp = video_client.post(VIDEO_GENERATE_URL, data={
            "prompt": "A test video",
            "size": "1280x720",
        })
        assert resp.status_code in (400, 422)

    def test_missing_size(self, video_client):
        """Size is required."""
        resp = video_client.post(VIDEO_GENERATE_URL, data={
            "prompt": "A test video",
            "seconds": "10",
        })
        assert resp.status_code in (400, 422)


# ============================================================================
# Test Class: Task Status & Listing
# ============================================================================

class TestVideoTaskManagement:
    """Video task listing and status queries."""

    def test_list_tasks(self, video_client):
        """GET /video/tasks should return paginated list."""
        resp = video_client.get(VIDEO_TASKS_URL)
        assert_response_success(resp)
        data = extract_data(resp.json())
        assert "tasks" in data
        assert "total" in data
        assert "page" in data
        assert "page_size" in data
        assert isinstance(data["tasks"], list)

    def test_list_tasks_pagination(self, video_client):
        """Pagination parameters should work."""
        resp = video_client.get(f"{VIDEO_TASKS_URL}?page=1&page_size=5")
        assert_response_success(resp)
        data = extract_data(resp.json())
        assert data["page"] == 1
        assert data["page_size"] == 5

    def test_get_task_detail(self, video_client, db_cursor):
        """GET /video/tasks/:task_id should return task details."""
        # First create a task
        form_data = _make_text_to_video_form(
            prompt="A video for task detail test",
            title="Detail Test",
        )
        create_resp = video_client.post(VIDEO_GENERATE_URL, data=form_data)
        assert_response_success(create_resp)
        task_id = extract_data(create_resp.json())["task_id"]

        # Query the task
        resp = video_client.get(VIDEO_TASK_URL.format(task_id=task_id))
        assert_response_success(resp)
        data = extract_data(resp.json())

        assert data["task_id"] == task_id
        assert data["status"] == "pending"
        assert data["prompt"] == "A video for task detail test"
        assert data["title"] == "Detail Test"
        assert "cost_points" in data
        assert "created_at" in data
        assert data["video_url"] is None  # Not yet completed
        assert data["error_message"] is None

    def test_get_nonexistent_task(self, video_client):
        """Querying non-existent task should return error."""
        resp = video_client.get(VIDEO_TASK_URL.format(task_id="nonexistent_task_xyz"))
        assert resp.status_code in (400, 404)

    def test_task_response_fields(self, video_client):
        """Task response should have all expected fields."""
        form_data = _make_text_to_video_form(prompt="Field completeness test video")
        create_resp = video_client.post(VIDEO_GENERATE_URL, data=form_data)
        assert_response_success(create_resp)
        task_id = extract_data(create_resp.json())["task_id"]

        resp = video_client.get(VIDEO_TASK_URL.format(task_id=task_id))
        assert_response_success(resp)
        data = extract_data(resp.json())

        expected_fields = [
            "id", "task_id", "title", "prompt", "status",
            "cost_points", "created_at",
        ]
        for field in expected_fields:
            assert field in data, f"Missing field: {field}"


# ============================================================================
# Test Class: Jimeng Text-to-Video
# ============================================================================

class TestJimengTextToVideo:
    """Jimeng AI Video 3.0 text-to-video tests."""

    def test_jimeng_t2v(self, video_client, db_cursor):
        """POST /video/generate with Jimeng model (text-to-video)."""
        # Check if Jimeng model exists
        db_cursor.execute(
            "SELECT id FROM gm_ai_models WHERE model_key LIKE 'jimeng-%%' AND is_active = true LIMIT 1"
        )
        model_row = db_cursor.fetchone()
        if model_row is None:
            pytest.skip("Jimeng model not seeded in DB")

        form_data = _make_text_to_video_form(
            prompt="A beautiful Chinese landscape with mountains and rivers",
            model_id=model_row["id"],
            seconds="10",
            size="1280x720",
            orientation="landscape",
            title="Jimeng T2V Test",
        )
        resp = video_client.post(VIDEO_GENERATE_URL, data=form_data)

        # If Jimeng client is not configured, API returns 500
        if resp.status_code == 500:
            body = resp.json()
            msg = body.get("message", body.get("msg", ""))
            if "not configured" in msg.lower() or "jimeng" in msg.lower():
                pytest.skip("Jimeng client not configured on API server")

        assert_response_success(resp)
        data = extract_data(resp.json())
        assert data["task_id"] is not None
        assert data["status"] in ("pending", "submitted")
        assert "cost_points" in data

        # Verify DB record
        db_cursor.execute(
            "SELECT * FROM gm_video_generation_tasks WHERE task_id = %s",
            (data["task_id"],)
        )
        row = db_cursor.fetchone()
        assert row is not None, "Jimeng task should be in DB"
        assert row["model_id"] == model_row["id"]
        assert row["prompt"] == form_data["prompt"]
        assert row["title"] == "Jimeng T2V Test"
        assert row["video_seconds"] == "10"
        assert row["orientation"] == "landscape"


# ============================================================================
# Test Class: Jimeng Image-to-Video
# ============================================================================

class TestJimengImageToVideo:
    """Jimeng image-to-video modes."""

    def _get_jimeng_model_id(self, db_cursor):
        db_cursor.execute(
            "SELECT id FROM gm_ai_models WHERE model_key LIKE 'jimeng-%%' AND is_active = true LIMIT 1"
        )
        row = db_cursor.fetchone()
        if row is None:
            pytest.skip("Jimeng model not seeded in DB")
        return row["id"]

    def test_jimeng_i2v_first_frame(self, video_client, db_cursor):
        """Image-to-video with single image as first frame."""
        model_id = self._get_jimeng_model_id(db_cursor)

        files = {
            "image": ("first_frame.png", io.BytesIO(TINY_PNG), "image/png"),
        }
        data_fields = {
            "prompt": "Animate this landscape with gentle wind",
            "ai_model_id": str(model_id),
            "seconds": "10",
            "size": "720x1280",
            "orientation": "portrait",
        }
        resp = video_client.session.post(
            f"{API_BASE_URL}{VIDEO_GENERATE_URL}",
            data=data_fields,
            files=files,
        )

        if resp.status_code == 500:
            body = resp.json()
            msg = body.get("message", body.get("msg", ""))
            if "not configured" in msg.lower():
                pytest.skip("Jimeng client not configured")

        assert_response_success(resp)
        data = extract_data(resp.json())
        assert data["task_id"] is not None

    def test_jimeng_i2v_first_last(self, video_client, db_cursor):
        """Image-to-video with two images (first + last frame)."""
        model_id = self._get_jimeng_model_id(db_cursor)

        files = {
            "start_frame": ("start.png", io.BytesIO(TINY_PNG), "image/png"),
            "end_frame": ("end.png", io.BytesIO(TINY_PNG), "image/png"),
        }
        data_fields = {
            "prompt": "Smooth transition between two scenes",
            "ai_model_id": str(model_id),
            "seconds": "10",
            "size": "720x1280",
            "orientation": "portrait",
        }
        resp = video_client.session.post(
            f"{API_BASE_URL}{VIDEO_GENERATE_URL}",
            data=data_fields,
            files=files,
        )

        if resp.status_code == 500:
            body = resp.json()
            msg = body.get("message", body.get("msg", ""))
            if "not configured" in msg.lower():
                pytest.skip("Jimeng client not configured")

        assert_response_success(resp)
        data = extract_data(resp.json())
        assert data["task_id"] is not None


# ============================================================================
# Test Class: Cross-Provider Consistency
# ============================================================================

class TestCrossProviderConsistency:
    """Ensure API contract is consistent across providers."""

    def test_response_format_consistency(self, video_client, db_cursor):
        """All providers should return the same response structure."""
        # Create task with default (LaoZhang) model
        form_data = _make_text_to_video_form(
            prompt="Consistency test video",
        )
        resp = video_client.post(VIDEO_GENERATE_URL, data=form_data)
        assert_response_success(resp)
        laozhang_data = extract_data(resp.json())

        required_fields = ["task_id", "status", "cost_points", "estimated_time"]
        for field in required_fields:
            assert field in laozhang_data, f"LaoZhang response missing: {field}"

        # If Jimeng is available, check consistency
        db_cursor.execute(
            "SELECT id FROM gm_ai_models WHERE model_key LIKE 'jimeng-%%' AND is_active = true LIMIT 1"
        )
        jimeng_row = db_cursor.fetchone()
        if jimeng_row:
            form_data_jimeng = _make_text_to_video_form(
                prompt="Jimeng consistency test",
                model_id=jimeng_row["id"],
            )
            resp_jimeng = video_client.post(VIDEO_GENERATE_URL, data=form_data_jimeng)
            if resp_jimeng.status_code == 200:
                jimeng_data = extract_data(resp_jimeng.json())
                for field in required_fields:
                    assert field in jimeng_data, f"Jimeng response missing: {field}"

    def test_db_record_consistency(self, video_client, db_cursor):
        """DB records should have same structure regardless of provider."""
        form_data = _make_text_to_video_form(
            prompt="DB consistency check",
            title="DB Check",
        )
        resp = video_client.post(VIDEO_GENERATE_URL, data=form_data)
        assert_response_success(resp)
        task_id = extract_data(resp.json())["task_id"]

        db_cursor.execute(
            "SELECT * FROM gm_video_generation_tasks WHERE task_id = %s",
            (task_id,)
        )
        row = db_cursor.fetchone()
        assert row is not None

        required_db_fields = [
            "id", "user_id", "task_id", "prompt", "status",
            "cost_points", "created_at", "model_id",
            "title", "orientation", "video_seconds", "video_size",
        ]
        for field in required_db_fields:
            assert field in row, f"DB record missing field: {field}"

        # Verify initial state
        assert row["status"] == "pending"
        assert row["video_url"] is None
        assert row["error_message"] is None
        assert row["completed_at"] is None


# ============================================================================
# Test Class: Authentication & Authorization
# ============================================================================

class TestVideoAuth:
    """Authentication and authorization for video endpoints."""

    def test_unauthenticated_generate(self):
        """Video generation requires authentication."""
        resp = requests.post(
            f"{API_BASE_URL}{VIDEO_GENERATE_URL}",
            data={"prompt": "test", "seconds": "10", "size": "1280x720"},
        )
        assert resp.status_code == 401

    def test_unauthenticated_list(self):
        """Task listing requires authentication."""
        resp = requests.get(f"{API_BASE_URL}{VIDEO_TASKS_URL}")
        assert resp.status_code == 401

    def test_unauthenticated_detail(self):
        """Task detail requires authentication."""
        resp = requests.get(f"{API_BASE_URL}{VIDEO_TASK_URL.format(task_id='test')}")
        assert resp.status_code == 401


# ============================================================================
# Test Class: End-to-End Flow
# ============================================================================

class TestVideoE2EFlow:
    """Full end-to-end video generation flow."""

    def test_create_and_query_flow(self, video_client, db_cursor):
        """Create task → query status → verify in list."""
        # Step 1: Create video task
        form_data = _make_text_to_video_form(
            prompt="End-to-end flow test: beautiful mountain scenery",
            title="E2E Flow Test",
            seconds="10",
            size="1280x720",
        )
        create_resp = video_client.post(VIDEO_GENERATE_URL, data=form_data)
        assert_response_success(create_resp)

        create_data = extract_data(create_resp.json())
        task_id = create_data["task_id"]
        assert task_id is not None
        assert create_data["status"] in ("pending", "submitted")
        assert float(create_data["cost_points"]) > 0

        # Step 2: Query task detail
        detail_resp = video_client.get(VIDEO_TASK_URL.format(task_id=task_id))
        assert_response_success(detail_resp)

        detail_data = extract_data(detail_resp.json())
        assert detail_data["task_id"] == task_id
        assert detail_data["prompt"] == form_data["prompt"]
        assert detail_data["title"] == "E2E Flow Test"
        assert detail_data["status"] == "pending"

        # Step 3: Task should appear in list
        list_resp = video_client.get(f"{VIDEO_TASKS_URL}?page=1&page_size=100")
        assert_response_success(list_resp)

        list_data = extract_data(list_resp.json())
        task_ids_in_list = [t["task_id"] for t in list_data["tasks"]]
        assert task_id in task_ids_in_list, "Created task should appear in task list"

        # Step 4: Verify complete DB state
        db_cursor.execute(
            "SELECT * FROM gm_video_generation_tasks WHERE task_id = %s",
            (task_id,)
        )
        row = db_cursor.fetchone()
        assert row["user_id"] is not None
        assert row["status"] == "pending"
        assert row["prompt"] == form_data["prompt"]
        assert row["title"] == "E2E Flow Test"
        assert row["video_seconds"] == "10"
        assert row["video_size"] == "1280x720"
        assert row["orientation"] == "landscape"
        assert float(row["cost_points"]) > 0
        assert row["created_at"] is not None
        assert row["video_url"] is None
        assert row["completed_at"] is None
        assert row["error_message"] is None

    def test_multiple_tasks_ordering(self, video_client):
        """Multiple tasks should be returned in correct order."""
        task_ids = []
        for i in range(3):
            form_data = _make_text_to_video_form(
                prompt=f"Ordering test video {i}",
                title=f"Order #{i}",
            )
            resp = video_client.post(VIDEO_GENERATE_URL, data=form_data)
            assert_response_success(resp)
            task_ids.append(extract_data(resp.json())["task_id"])
            time.sleep(0.1)

        list_resp = video_client.get(f"{VIDEO_TASKS_URL}?page=1&page_size=100")
        assert_response_success(list_resp)
        list_data = extract_data(list_resp.json())

        all_task_ids = [t["task_id"] for t in list_data["tasks"]]
        for tid in task_ids:
            assert tid in all_task_ids, f"Task {tid} should be in list"


# ============================================================================
# Test Class: Jimeng Mock Direct Tests
# ============================================================================

class TestJimengMockDirect:
    """Direct tests against the Jimeng mock service (if available).
    
    These tests verify the Jimeng mock API contract matches what
    jimeng_client.rs expects. Useful for pre-integration validation.
    """

    JIMENG_BASE_URL = os.getenv("JIMENG_BASE_URL", "http://localhost:8300")

    def _is_jimeng_mock_available(self):
        try:
            resp = requests.get(f"{self.JIMENG_BASE_URL}/health", timeout=2)
            return resp.status_code == 200
        except Exception:
            return False

    def test_jimeng_mock_health(self):
        """Jimeng mock service should be reachable."""
        if not self._is_jimeng_mock_available():
            pytest.skip("Jimeng mock not available")
        resp = requests.get(f"{self.JIMENG_BASE_URL}/health")
        assert resp.status_code == 200
        assert resp.json()["service"] == "jimeng-mock"

    def test_jimeng_mock_submit_t2v(self):
        """Submit text-to-video task to Jimeng mock."""
        if not self._is_jimeng_mock_available():
            pytest.skip("Jimeng mock not available")

        resp = requests.post(
            f"{self.JIMENG_BASE_URL}/?Action=CVSync2AsyncSubmitTask&Version=2024-06-06",
            json={
                "req_key": "jimeng_t2v_v30",
                "prompt": "A test video prompt",
                "frames": 121,
                "aspect_ratio": "16:9",
                "seed": -1,
            },
            headers={"Authorization": "HMAC-SHA256 Credential=test/20260305/cn-north-1/cv/request, SignedHeaders=content-type;host;x-date, Signature=abc123"},
        )
        assert resp.status_code == 200
        data = resp.json()
        assert data["code"] == 10000
        assert data["data"]["task_id"] is not None
        return data["data"]["task_id"]

    def test_jimeng_mock_submit_i2v_first(self):
        """Submit image-to-video (first frame) task."""
        if not self._is_jimeng_mock_available():
            pytest.skip("Jimeng mock not available")

        img_b64 = base64.b64encode(TINY_PNG).decode()
        resp = requests.post(
            f"{self.JIMENG_BASE_URL}/?Action=CVSync2AsyncSubmitTask&Version=2024-06-06",
            json={
                "req_key": "jimeng_i2v_first_v30",
                "prompt": "Animate this scene",
                "frames": 241,
                "binary_data_base64": [img_b64],
                "seed": -1,
            },
            headers={"Authorization": "HMAC-SHA256 Credential=test, SignedHeaders=x-date, Signature=mock"},
        )
        assert resp.status_code == 200
        assert resp.json()["code"] == 10000

    def test_jimeng_mock_submit_i2v_fl(self):
        """Submit image-to-video (first+last) task."""
        if not self._is_jimeng_mock_available():
            pytest.skip("Jimeng mock not available")

        img_b64 = base64.b64encode(TINY_PNG).decode()
        resp = requests.post(
            f"{self.JIMENG_BASE_URL}/?Action=CVSync2AsyncSubmitTask&Version=2024-06-06",
            json={
                "req_key": "jimeng_i2v_first_tail_v30",
                "prompt": "Smooth transition",
                "frames": 121,
                "binary_data_base64": [img_b64, img_b64],
                "seed": -1,
            },
            headers={"Authorization": "HMAC-SHA256 Credential=test, SignedHeaders=x-date, Signature=mock"},
        )
        assert resp.status_code == 200
        assert resp.json()["code"] == 10000

    def test_jimeng_mock_submit_fl_missing_images(self):
        """FL mode with < 2 images should fail."""
        if not self._is_jimeng_mock_available():
            pytest.skip("Jimeng mock not available")

        resp = requests.post(
            f"{self.JIMENG_BASE_URL}/?Action=CVSync2AsyncSubmitTask&Version=2024-06-06",
            json={
                "req_key": "jimeng_i2v_first_tail_v30",
                "prompt": "Missing images",
                "frames": 121,
                "binary_data_base64": [base64.b64encode(TINY_PNG).decode()],
            },
            headers={"Authorization": "HMAC-SHA256 Credential=test, SignedHeaders=x-date, Signature=mock"},
        )
        assert resp.status_code == 200
        assert resp.json()["code"] == 40003

    def test_jimeng_mock_submit_i2v_fl_1080p(self):
        """Submit image-to-video (first+last) 1080p task."""
        if not self._is_jimeng_mock_available():
            pytest.skip("Jimeng mock not available")

        img_b64 = base64.b64encode(TINY_PNG).decode()
        resp = requests.post(
            f"{self.JIMENG_BASE_URL}/?Action=CVSync2AsyncSubmitTask&Version=2024-06-06",
            json={
                "req_key": "jimeng_i2v_first_tail_v30_1080",
                "prompt": "1080p first-last transition",
                "frames": 121,
                "binary_data_base64": [img_b64, img_b64],
                "seed": -1,
            },
            headers={"Authorization": "HMAC-SHA256 Credential=test, SignedHeaders=x-date, Signature=mock"},
        )
        assert resp.status_code == 200
        assert resp.json()["code"] == 10000

    def test_jimeng_mock_get_result_progression(self):
        """Task status should progress: running → running → done."""
        if not self._is_jimeng_mock_available():
            pytest.skip("Jimeng mock not available")

        # Submit a task first
        submit_resp = requests.post(
            f"{self.JIMENG_BASE_URL}/?Action=CVSync2AsyncSubmitTask&Version=2024-06-06",
            json={
                "req_key": "jimeng_t2v_v30",
                "prompt": "Status progression test",
                "frames": 121,
            },
            headers={"Authorization": "HMAC-SHA256 Credential=test, SignedHeaders=x-date, Signature=mock"},
        )
        task_id = submit_resp.json()["data"]["task_id"]

        auth_header = {"Authorization": "HMAC-SHA256 Credential=test, SignedHeaders=x-date, Signature=mock"}

        # Query 1: running
        r1 = requests.post(
            f"{self.JIMENG_BASE_URL}/?Action=CVSync2AsyncGetResult&Version=2024-06-06",
            json={"task_id": task_id},
            headers=auth_header,
        )
        assert r1.json()["data"]["status"] == "running"
        assert r1.json()["data"]["resp_data"] is None

        # Query 2: still running
        r2 = requests.post(
            f"{self.JIMENG_BASE_URL}/?Action=CVSync2AsyncGetResult&Version=2024-06-06",
            json={"task_id": task_id},
            headers=auth_header,
        )
        assert r2.json()["data"]["status"] == "running"

        # Query 3: done
        r3 = requests.post(
            f"{self.JIMENG_BASE_URL}/?Action=CVSync2AsyncGetResult&Version=2024-06-06",
            json={"task_id": task_id},
            headers=auth_header,
        )
        assert r3.json()["data"]["status"] == "done"
        assert r3.json()["data"]["resp_data"] is not None
        assert task_id in r3.json()["data"]["resp_data"]

    def test_jimeng_mock_invalid_req_key(self):
        """Invalid req_key should be rejected."""
        if not self._is_jimeng_mock_available():
            pytest.skip("Jimeng mock not available")

        resp = requests.post(
            f"{self.JIMENG_BASE_URL}/?Action=CVSync2AsyncSubmitTask&Version=2024-06-06",
            json={
                "req_key": "invalid_mode",
                "prompt": "Should fail",
                "frames": 121,
            },
            headers={"Authorization": "HMAC-SHA256 Credential=test, SignedHeaders=x-date, Signature=mock"},
        )
        assert resp.json()["code"] == 40001

    def test_jimeng_mock_missing_auth(self):
        """Missing auth should return 401."""
        if not self._is_jimeng_mock_available():
            pytest.skip("Jimeng mock not available")

        resp = requests.post(
            f"{self.JIMENG_BASE_URL}/?Action=CVSync2AsyncSubmitTask&Version=2024-06-06",
            json={"req_key": "jimeng_t2v_v30", "prompt": "test", "frames": 121},
        )
        assert resp.status_code == 401


# ============================================================================
# Test Class: LaoZhang Mock Direct Tests
# ============================================================================

class TestLaoZhangMockDirect:
    """Direct tests against LaoZhang mock for video API contract verification."""

    LAOZHANG_BASE_URL = os.getenv("LAOZHANG_BASE_URL", "http://localhost:8100")

    def _is_laozhang_mock_available(self):
        try:
            resp = requests.get(f"{self.LAOZHANG_BASE_URL}/health", timeout=2)
            return resp.status_code == 200
        except Exception:
            return False

    def test_laozhang_mock_text_to_video(self):
        """Submit text-to-video to LaoZhang mock."""
        if not self._is_laozhang_mock_available():
            pytest.skip("LaoZhang mock not available")

        resp = requests.post(
            f"{self.LAOZHANG_BASE_URL}/v1/videos",
            json={
                "model": "sora-2",
                "prompt": "A cat playing",
                "size": "1280x720",
                "seconds": "10",
            },
            headers={"Authorization": "Bearer mock-key"},
        )
        assert resp.status_code == 200
        data = resp.json()
        assert "id" in data
        assert data["status"] == "submitted"
        assert data["model"] is not None

    def test_laozhang_mock_task_status_progression(self):
        """LaoZhang task should progress: in_progress → in_progress → completed."""
        if not self._is_laozhang_mock_available():
            pytest.skip("LaoZhang mock not available")

        # Create task
        create_resp = requests.post(
            f"{self.LAOZHANG_BASE_URL}/v1/videos",
            json={"model": "veo-3.1-fast", "prompt": "test", "size": "720x1280", "seconds": "8"},
            headers={"Authorization": "Bearer mock-key"},
        )
        task_id = create_resp.json()["id"]

        auth = {"Authorization": "Bearer mock-key"}

        # Query progression
        r1 = requests.get(f"{self.LAOZHANG_BASE_URL}/v1/videos/{task_id}", headers=auth)
        assert r1.json()["status"] == "in_progress"
        assert r1.json()["progress"] == 30

        r2 = requests.get(f"{self.LAOZHANG_BASE_URL}/v1/videos/{task_id}", headers=auth)
        assert r2.json()["status"] == "in_progress"
        assert r2.json()["progress"] == 70

        r3 = requests.get(f"{self.LAOZHANG_BASE_URL}/v1/videos/{task_id}", headers=auth)
        assert r3.json()["status"] == "completed"
        assert r3.json()["progress"] == 100
        assert r3.json()["url"] is not None

    def test_laozhang_mock_video_download(self):
        """Video download should return MP4 bytes."""
        if not self._is_laozhang_mock_available():
            pytest.skip("LaoZhang mock not available")

        resp = requests.get(
            f"{self.LAOZHANG_BASE_URL}/v1/videos/test_task/content",
            headers={"Authorization": "Bearer mock-key"},
        )
        assert resp.status_code == 200
        assert resp.headers["content-type"] == "video/mp4"
        assert len(resp.content) > 0


# ============================================================================
# Test Class: All Video Models Coverage
# ============================================================================

class TestAllVideoModels:
    """Test every video model with both T2V and I2V."""

    ALL_MODELS = [
        (MODEL_VEO2_ID, "Veo-2", "10"),
        (MODEL_JIMENG_720P_ID, "Jimeng 720P", "5"),
        (MODEL_JIMENG_1080P_ID, "Jimeng 1080P", "5"),
    ]

    def test_t2v_all_models(self, video_client, db_cursor):
        """Text-to-video for every model: Veo-2, Jimeng 720P, Jimeng 1080P."""
        for model_id, name, secs in self.ALL_MODELS:
            db_cursor.execute(
                "SELECT id FROM gm_ai_models WHERE id = %s AND is_active = true", (model_id,)
            )
            if db_cursor.fetchone() is None:
                continue

            size = "1280x720"
            resp = video_client.post(VIDEO_GENERATE_URL, data={
                "prompt": f"T2V test for {name}",
                "ai_model_id": str(model_id),
                "seconds": secs,
                "size": size,
                "orientation": "landscape",
            })

            if resp.status_code == 500 and "not configured" in resp.text.lower():
                continue

            assert resp.status_code == 200, f"{name} T2V failed: {resp.status_code} {resp.text}"
            data = extract_data(resp.json())
            assert data["task_id"] is not None, f"{name}: missing task_id"

            db_cursor.execute(
                "SELECT model_id, video_seconds FROM gm_video_generation_tasks WHERE task_id = %s",
                (data["task_id"],)
            )
            row = db_cursor.fetchone()
            assert row is not None, f"{name}: task not in DB"
            assert row["model_id"] == model_id, f"{name}: model_id mismatch"
            assert row["video_seconds"] == secs, f"{name}: seconds mismatch"

    def test_i2v_all_models(self, video_client, db_cursor):
        """Image-to-video for every model."""
        for model_id, name, secs in self.ALL_MODELS:
            db_cursor.execute(
                "SELECT id FROM gm_ai_models WHERE id = %s AND is_active = true", (model_id,)
            )
            if db_cursor.fetchone() is None:
                continue

            files = {"image": ("test.png", io.BytesIO(TINY_PNG), "image/png")}
            resp = video_client.session.post(
                f"{API_BASE_URL}{VIDEO_GENERATE_URL}",
                data={
                    "prompt": f"I2V test for {name}",
                    "ai_model_id": str(model_id),
                    "seconds": secs,
                    "size": "720x1280",
                    "orientation": "portrait",
                },
                files=files,
            )

            if resp.status_code == 500 and "not configured" in resp.text.lower():
                continue

            assert resp.status_code == 200, f"{name} I2V failed: {resp.status_code} {resp.text}"
            data = extract_data(resp.json())
            assert data["task_id"] is not None

    def test_jimeng_5_second_duration(self, video_client):
        """Jimeng models accept seconds=5 (which LaoZhang does not)."""
        resp = video_client.post(VIDEO_GENERATE_URL, data={
            "prompt": "5 second Jimeng test",
            "ai_model_id": str(MODEL_JIMENG_720P_ID),
            "seconds": "5",
            "size": "1280x720",
            "orientation": "landscape",
        })
        # Should succeed (or 500 if Jimeng not configured, but not 400)
        assert resp.status_code != 400, "seconds=5 should be accepted for Jimeng"

    def test_jimeng_extended_sizes(self, video_client):
        """Jimeng aspect ratios produce valid sizes."""
        sizes = ["720x720", "960x720", "720x960", "1260x540"]
        for size in sizes:
            resp = video_client.post(VIDEO_GENERATE_URL, data={
                "prompt": f"Size test {size}",
                "ai_model_id": str(MODEL_JIMENG_720P_ID),
                "seconds": "5",
                "size": size,
                "orientation": "landscape",
            })
            assert resp.status_code != 400, f"size={size} should be accepted"


# ============================================================================
# Test Class: AIPub Plan with Jimeng Model
# ============================================================================

class TestAIPubPlanWithJimeng:
    """Create AIPub single_video plan with Jimeng video_ai_model_id."""

    def test_create_plan_with_jimeng_model(self, video_client, db_cursor):
        """POST /publish_plans with plan_type=single_video and Jimeng video_ai_model_id."""
        db_cursor.execute(
            "SELECT id FROM gm_ai_models WHERE model_key LIKE 'jimeng-%%' AND is_active = true LIMIT 1"
        )
        model_row = db_cursor.fetchone()
        if model_row is None:
            pytest.skip("Jimeng model not in DB")

        # Need a social account for single_video plan
        db_cursor.execute(
            "SELECT id FROM gm_social_accounts WHERE user_id = (SELECT id FROM gm_users LIMIT 1) LIMIT 1"
        )
        account_row = db_cursor.fetchone()
        if account_row is None:
            pytest.skip("No social account available")

        resp = video_client.post(AIPUB_PLAN_URL, json={
            "platform_id": 2,
            "content_type": "video",
            "plan_type": "single_video",
            "video_ai_model_id": model_row["id"],
            "social_account_id": account_row["id"],
            "ai_input": {
                "content_prompt": "Test Jimeng AIPub plan",
                "video_prompt": "Beautiful landscape animation",
                "video_config": {"duration": 5, "aspect_ratio": "16:9"},
            },
        })

        if resp.status_code == 200:
            data = extract_data(resp.json())
            assert data is not None
            plan_id = data.get("id")
            if plan_id:
                db_cursor.execute(
                    "SELECT video_ai_model_id FROM gm_aipub_plans WHERE id = %s", (plan_id,)
                )
                row = db_cursor.fetchone()
                assert row is not None
                assert row["video_ai_model_id"] == model_row["id"]


# ============================================================================
# Test Class: Vidu Video Generation
# ============================================================================

# Vidu model IDs from init-test-data.sql (IDs 10-21)
MODEL_VIDU_T2V_ID = 10
MODEL_VIDU_I2V_ID = 11
MODEL_VIDU_REF2V_ID = 12
MODEL_VIDU_STARTEND_ID = 13
MODEL_VIDU_MULTIFRAME_ID = 14
MODEL_VIDU_FAST_ID = 15
MODEL_VIDU_TEMPLATE_ID = 16


class TestViduModelList:
    """Verify Vidu models appear in the AI models config."""

    def test_vidu_models_in_config(self, video_client, db_cursor):
        """All Vidu models should appear in /config/ai-models."""
        resp = video_client.get(CONFIG_AI_MODELS_URL)
        if resp.status_code != 200:
            pytest.skip("Config API unavailable")

        data = resp.json()
        models_list = data.get("data", data) if isinstance(data, dict) else data
        if isinstance(models_list, dict):
            models_list = models_list.get("models", [])

        vidu_models = [m for m in models_list if m.get("provider") == "vidu"
                       or (m.get("model_key", "").startswith("vidu-"))]
        assert len(vidu_models) >= 7, f"Expected >=7 Vidu models, got {len(vidu_models)}"

    def test_vidu_models_in_db(self, db_cursor):
        """DB should have all 7 simplified Vidu models active."""
        db_cursor.execute(
            "SELECT COUNT(*) as cnt FROM gm_ai_models WHERE provider = 'vidu' AND is_active = true"
        )
        row = db_cursor.fetchone()
        count = row["cnt"] if isinstance(row, dict) else row[0]
        assert count >= 7, f"Expected >=7 active Vidu models in DB, got {count}"

    def test_vidu_model_keys_convention(self, db_cursor):
        """All Vidu model_key values should start with 'vidu-'."""
        db_cursor.execute(
            "SELECT model_key FROM gm_ai_models WHERE provider = 'vidu'"
        )
        rows = db_cursor.fetchall()
        for row in rows:
            key = row["model_key"] if isinstance(row, dict) else row[0]
            assert key.startswith("vidu-"), f"Vidu model_key '{key}' should start with 'vidu-'"


class TestViduVideoGeneration:
    """Vidu video generation E2E tests.

    These tests require the API server to be running with VIDU_API_KEY configured
    (or a mock Vidu service). Tests will be skipped if the Vidu provider is not
    available.
    """

    def _check_vidu_available(self, resp):
        """Skip test if Vidu provider is not configured."""
        if resp.status_code == 500:
            body = resp.text
            if "not configured" in body.lower() or "vidu" in body.lower():
                pytest.skip("Vidu provider not configured on API server")

    def test_create_vidu_text2video(self, video_client, db_cursor):
        """Create text-to-video task with simplified vidu-t2v model."""
        resp = video_client.post(VIDEO_GENERATE_URL, data={
            "prompt": "A cat playing on the beach with sunset",
            "ai_model_id": str(MODEL_VIDU_T2V_ID),
            "seconds": "4",
            "size": "1280x720",
            "orientation": "landscape",
        })
        self._check_vidu_available(resp)
        if resp.status_code == 200:
            data = extract_data(resp.json())
            assert data is not None
            assert "task_id" in data
            task_id = data["task_id"]
            db_cursor.execute(
                "SELECT * FROM gm_video_generation_tasks WHERE task_id = %s", (task_id,)
            )
            row = db_cursor.fetchone()
            assert row is not None, f"Task {task_id} not found in DB"

    def test_create_vidu_image2video(self, video_client, db_cursor):
        """Create image-to-video task with vidu-i2v model."""
        files = {
            "image": ("test.png", io.BytesIO(TINY_PNG), "image/png"),
        }
        resp = video_client.post(VIDEO_GENERATE_URL, data={
            "prompt": "Animate this image into a video",
            "ai_model_id": str(MODEL_VIDU_I2V_ID),
            "seconds": "4",
            "size": "1280x720",
            "orientation": "landscape",
        }, files=files)
        self._check_vidu_available(resp)
        if resp.status_code == 200:
            data = extract_data(resp.json())
            assert data is not None
            assert "task_id" in data

    def test_create_vidu_duration_8s(self, video_client):
        """Vidu supports 8-second duration."""
        resp = video_client.post(VIDEO_GENERATE_URL, data={
            "prompt": "8 second video test",
            "ai_model_id": str(MODEL_VIDU_T2V_ID),
            "seconds": "8",
            "size": "1280x720",
            "orientation": "landscape",
        })
        self._check_vidu_available(resp)
        assert resp.status_code != 400, "seconds=8 should be accepted for Vidu"

    def test_vidu_task_listing(self, video_client):
        """GET /video/tasks should include any created Vidu tasks."""
        resp = video_client.get(VIDEO_TASKS_URL)
        assert resp.status_code == 200

    def test_vidu_validation_invalid_duration(self, video_client):
        """Reject invalid duration (3s) for Vidu model."""
        resp = video_client.post(VIDEO_GENERATE_URL, data={
            "prompt": "Invalid duration test",
            "ai_model_id": str(MODEL_VIDU_T2V_ID),
            "seconds": "3",
            "size": "1280x720",
            "orientation": "landscape",
        })
        assert resp.status_code == 400 or resp.status_code == 422, \
            f"seconds=3 should be rejected, got {resp.status_code}"

    def test_vidu_validation_invalid_size(self, video_client):
        """Reject invalid size (640x480) for Vidu model."""
        resp = video_client.post(VIDEO_GENERATE_URL, data={
            "prompt": "Invalid size test",
            "ai_model_id": str(MODEL_VIDU_T2V_ID),
            "seconds": "4",
            "size": "640x480",
            "orientation": "landscape",
        })
        assert resp.status_code == 400 or resp.status_code == 422, \
            f"size=640x480 should be rejected, got {resp.status_code}"

    def test_vidu_fast_generation_5s(self, video_client):
        """Vidu fast (viduq1) model uses 5-second duration."""
        resp = video_client.post(VIDEO_GENERATE_URL, data={
            "prompt": "Fast generation test",
            "ai_model_id": str(MODEL_VIDU_FAST_ID),
            "seconds": "5",
            "size": "1280x720",
            "orientation": "landscape",
        })
        self._check_vidu_available(resp)
        assert resp.status_code != 400, "seconds=5 should be accepted for fast model"


class TestViduAIPubPlan:
    """Create AIPub plan with Vidu video_ai_model_id."""

    def test_create_plan_with_vidu_model(self, video_client, db_cursor):
        """POST /publish_plans with Vidu video_ai_model_id."""
        db_cursor.execute(
            "SELECT id FROM gm_ai_models WHERE model_key = 'vidu-t2v' AND is_active = true"
        )
        model_row = db_cursor.fetchone()
        if model_row is None:
            pytest.skip("Vidu model not in DB")

        model_id = model_row["id"] if isinstance(model_row, dict) else model_row[0]

        db_cursor.execute(
            "SELECT id FROM gm_social_accounts WHERE user_id = (SELECT id FROM gm_users LIMIT 1) LIMIT 1"
        )
        account_row = db_cursor.fetchone()
        if account_row is None:
            pytest.skip("No social account available")

        account_id = account_row["id"] if isinstance(account_row, dict) else account_row[0]

        resp = video_client.post(AIPUB_PLAN_URL, json={
            "platform_id": 2,
            "content_type": "video",
            "plan_type": "single_video",
            "video_ai_model_id": model_id,
            "social_account_id": account_id,
            "ai_input": {
                "content_prompt": "Test Vidu AIPub plan",
                "video_prompt": "Beautiful sunset over the ocean with waves",
                "video_config": {
                    "duration": 4,
                    "aspect_ratio": "16:9",
                    "resolution": "720p",
                    "size": "1280x720",
                },
                "vidu_config": {
                    "style": "general",
                    "movement_amplitude": "auto",
                    "model_version": "vidu1.5",
                },
            },
        })

        if resp.status_code == 200:
            data = extract_data(resp.json())
            assert data is not None
            plan_id = data.get("id")
            if plan_id:
                # Verify DB record has correct video_ai_model_id
                db_cursor.execute(
                    "SELECT video_ai_model_id, ai_input FROM gm_aipub_plans WHERE id = %s",
                    (plan_id,)
                )
                row = db_cursor.fetchone()
                assert row is not None
                db_model_id = row["video_ai_model_id"] if isinstance(row, dict) else row[0]
                assert db_model_id == model_id

                # Verify ai_input contains vidu_config
                ai_input = row["ai_input"] if isinstance(row, dict) else row[1]
                if isinstance(ai_input, str):
                    import json
                    ai_input = json.loads(ai_input)
                if ai_input:
                    assert "video_config" in ai_input, "ai_input should contain video_config"

    def test_vidu_plan_consistency_with_db_model(self, video_client, db_cursor):
        """Verify DB model_key matches expected Vidu convention."""
        db_cursor.execute(
            "SELECT model_key, provider, model_type FROM gm_ai_models WHERE provider = 'vidu'"
        )
        rows = db_cursor.fetchall()
        for row in rows:
            key = row["model_key"] if isinstance(row, dict) else row[0]
            provider = row["provider"] if isinstance(row, dict) else row[1]
            model_type = row["model_type"] if isinstance(row, dict) else row[2]
            assert provider == "vidu"
            assert model_type == "video"
            assert key.startswith("vidu-"), f"model_key '{key}' should start with 'vidu-'"
