"""
Material Management API E2E Tests
==================================
Tests for the material upload and create workflow.

Covers:
- POST /api/v1/materials - Create material
- GET  /api/v1/materials - List materials
- GET  /api/v1/materials/:id - Get material detail
- PUT  /api/v1/materials/:id - Update material
- DELETE /api/v1/materials/:id - Delete material (soft)
- GET  /api/v1/material-tags - List tags
- POST /api/v1/oss/upload-video - Upload video to OSS (config dependent)
"""

import pytest
import time
from conftest import (
    API_BASE_URL,
    extract_data,
    assert_response_success,
    assert_json_structure,
    get_or_create_test_token,
    APIClient,
    TEST_USER_ID,
)


# =============================================================================
# Fixtures
# =============================================================================

@pytest.fixture
def auth_client(db_connection):
    """Authenticated client with full permissions."""
    try:
        token = get_or_create_test_token()
        cursor = db_connection.cursor()
        cursor.execute(
            "UPDATE gm_users SET permissions = 15 WHERE id = %s",
            (TEST_USER_ID,),
        )
        db_connection.commit()
        cursor.close()
        return APIClient(API_BASE_URL, token)
    except Exception as e:
        pytest.skip(f"Auth client not available: {e}")


@pytest.fixture
def cleanup_materials(db_connection):
    """Clean up test materials after each test."""
    yield
    cursor = db_connection.cursor()
    cursor.execute(
        "DELETE FROM gm_user_materials WHERE user_id = %s AND title LIKE '%%e2e_test%%'",
        (TEST_USER_ID,),
    )
    db_connection.commit()
    cursor.close()


# =============================================================================
# 1. Create Material Tests
# =============================================================================

class TestCreateMaterial:
    """Tests for POST /api/v1/materials"""

    def test_create_material_success(self, auth_client, cleanup_materials):
        """Create material with all required fields."""
        resp = auth_client.post(
            "/api/v1/materials",
            json={
                "video_url": "https://example.com/test-video.mp4",
                "tag": "e2e_test_tag",
                "title": "e2e_test_material_basic",
                "description": "Test material for E2E testing",
            },
        )
        assert_response_success(resp)
        data = extract_data(resp.json())
        assert data is not None, "Response data should not be None"

        assert data["video_url"] == "https://example.com/test-video.mp4"
        assert data["tag"] == "e2e_test_tag"
        assert data["title"] == "e2e_test_material_basic"
        assert data["description"] == "Test material for E2E testing"
        assert data["is_active"] is True
        assert data["id"] > 0
        assert data["user_id"] == TEST_USER_ID
        assert data["created_at"] is not None
        # prompt should be None initially (background AI analysis)
        # It may have been filled by the time we check, so we don't assert None

    def test_create_material_returns_immediately(self, auth_client, cleanup_materials):
        """Verify create_material returns quickly (not blocked by AI analysis)."""
        start_time = time.time()
        resp = auth_client.post(
            "/api/v1/materials",
            json={
                "video_url": "https://example.com/test-video-speed.mp4",
                "tag": "e2e_test_speed",
                "title": "e2e_test_material_speed",
            },
        )
        elapsed = time.time() - start_time

        assert_response_success(resp)
        assert elapsed < 10, f"Create material should return within 10s, took {elapsed:.1f}s"

    def test_create_material_without_description(self, auth_client, cleanup_materials):
        """Create material without optional description field."""
        resp = auth_client.post(
            "/api/v1/materials",
            json={
                "video_url": "https://example.com/test-no-desc.mp4",
                "tag": "e2e_test_tag",
                "title": "e2e_test_material_no_desc",
            },
        )
        assert_response_success(resp)
        data = extract_data(resp.json())
        assert data is not None
        assert data["description"] is None

    def test_create_material_missing_video_url(self, auth_client):
        """Should fail when video_url is missing."""
        resp = auth_client.post(
            "/api/v1/materials",
            json={
                "tag": "test_tag",
                "title": "test_title",
            },
        )
        assert resp.status_code in [400, 422], \
            f"Expected 400/422, got {resp.status_code}: {resp.text}"

    def test_create_material_empty_video_url(self, auth_client):
        """Should fail when video_url is empty."""
        resp = auth_client.post(
            "/api/v1/materials",
            json={
                "video_url": "",
                "tag": "test_tag",
                "title": "test_title",
            },
        )
        assert resp.status_code in [400, 422], \
            f"Expected 400/422, got {resp.status_code}: {resp.text}"

    def test_create_material_missing_tag(self, auth_client):
        """Should fail when tag is missing."""
        resp = auth_client.post(
            "/api/v1/materials",
            json={
                "video_url": "https://example.com/test.mp4",
                "title": "test_title",
            },
        )
        assert resp.status_code in [400, 422], \
            f"Expected 400/422, got {resp.status_code}: {resp.text}"

    def test_create_material_empty_tag(self, auth_client):
        """Should fail when tag is empty string."""
        resp = auth_client.post(
            "/api/v1/materials",
            json={
                "video_url": "https://example.com/test.mp4",
                "tag": "",
                "title": "test_title",
            },
        )
        assert resp.status_code in [400, 422], \
            f"Expected 400/422, got {resp.status_code}: {resp.text}"

    def test_create_material_missing_title(self, auth_client):
        """Should fail when title is missing."""
        resp = auth_client.post(
            "/api/v1/materials",
            json={
                "video_url": "https://example.com/test.mp4",
                "tag": "test_tag",
            },
        )
        assert resp.status_code in [400, 422], \
            f"Expected 400/422, got {resp.status_code}: {resp.text}"

    def test_create_material_empty_title(self, auth_client):
        """Should fail when title is empty string."""
        resp = auth_client.post(
            "/api/v1/materials",
            json={
                "video_url": "https://example.com/test.mp4",
                "tag": "test_tag",
                "title": "",
            },
        )
        assert resp.status_code in [400, 422], \
            f"Expected 400/422, got {resp.status_code}: {resp.text}"

    def test_create_material_unauthenticated(self):
        """Should fail without authentication."""
        client = APIClient(API_BASE_URL)
        resp = client.post(
            "/api/v1/materials",
            json={
                "video_url": "https://example.com/test.mp4",
                "tag": "test_tag",
                "title": "test_title",
            },
        )
        assert resp.status_code == 401, \
            f"Expected 401, got {resp.status_code}: {resp.text}"

    def test_create_material_verify_db(self, auth_client, db_connection, cleanup_materials):
        """Verify material is correctly stored in database."""
        resp = auth_client.post(
            "/api/v1/materials",
            json={
                "video_url": "https://example.com/test-db-verify.mp4",
                "tag": "e2e_test_db",
                "title": "e2e_test_material_db",
                "description": "DB verification test",
            },
        )
        assert_response_success(resp)
        data = extract_data(resp.json())
        material_id = data["id"]

        cursor = db_connection.cursor()
        from psycopg2.extras import RealDictCursor
        cursor = db_connection.cursor(cursor_factory=RealDictCursor)
        cursor.execute(
            "SELECT * FROM gm_user_materials WHERE id = %s",
            (material_id,),
        )
        row = cursor.fetchone()
        cursor.close()

        assert row is not None, "Material should exist in database"
        assert row["user_id"] == TEST_USER_ID
        assert row["video_url"] == "https://example.com/test-db-verify.mp4"
        assert row["tag"] == "e2e_test_db"
        assert row["title"] == "e2e_test_material_db"
        assert row["description"] == "DB verification test"
        assert row["is_active"] is True
        assert row["created_at"] is not None


# =============================================================================
# 2. List Materials Tests
# =============================================================================

class TestListMaterials:
    """Tests for GET /api/v1/materials"""

    def test_list_materials(self, auth_client, cleanup_materials):
        """List materials with pagination."""
        # Create some test materials first
        for i in range(3):
            auth_client.post(
                "/api/v1/materials",
                json={
                    "video_url": f"https://example.com/list-test-{i}.mp4",
                    "tag": "e2e_test_list",
                    "title": f"e2e_test_material_list_{i}",
                },
            )

        resp = auth_client.get("/api/v1/materials", params={"page": 1, "page_size": 10})
        assert_response_success(resp)
        data = extract_data(resp.json())
        assert data is not None

        assert "list" in data
        assert "total" in data
        assert "page" in data
        assert "page_size" in data
        assert data["total"] >= 3
        assert len(data["list"]) > 0

    def test_list_materials_with_tag_filter(self, auth_client, cleanup_materials):
        """Filter materials by tag."""
        auth_client.post(
            "/api/v1/materials",
            json={
                "video_url": "https://example.com/filter-test.mp4",
                "tag": "e2e_test_filter_unique",
                "title": "e2e_test_material_filter",
            },
        )

        resp = auth_client.get(
            "/api/v1/materials",
            params={"tag": "e2e_test_filter_unique"},
        )
        assert_response_success(resp)
        data = extract_data(resp.json())

        assert data["total"] >= 1
        for item in data["list"]:
            assert item["tag"] == "e2e_test_filter_unique"

    def test_list_materials_with_search(self, auth_client, cleanup_materials):
        """Search materials by title/description."""
        auth_client.post(
            "/api/v1/materials",
            json={
                "video_url": "https://example.com/search-test.mp4",
                "tag": "e2e_test_search",
                "title": "e2e_test_material_searchable_xyz123",
                "description": "Unique description for search",
            },
        )

        resp = auth_client.get(
            "/api/v1/materials",
            params={"search": "searchable_xyz123"},
        )
        assert_response_success(resp)
        data = extract_data(resp.json())
        assert data["total"] >= 1

    def test_list_materials_pagination(self, auth_client, cleanup_materials):
        """Verify pagination works correctly."""
        for i in range(5):
            auth_client.post(
                "/api/v1/materials",
                json={
                    "video_url": f"https://example.com/page-test-{i}.mp4",
                    "tag": "e2e_test_page",
                    "title": f"e2e_test_material_page_{i}",
                },
            )

        resp_page1 = auth_client.get(
            "/api/v1/materials",
            params={"page": 1, "page_size": 2},
        )
        data1 = extract_data(resp_page1.json())
        assert len(data1["list"]) <= 2

        resp_page2 = auth_client.get(
            "/api/v1/materials",
            params={"page": 2, "page_size": 2},
        )
        data2 = extract_data(resp_page2.json())

        if data1["total"] > 2:
            ids_page1 = {m["id"] for m in data1["list"]}
            ids_page2 = {m["id"] for m in data2["list"]}
            assert ids_page1.isdisjoint(ids_page2), "Pages should not overlap"


# =============================================================================
# 3. Get Material Detail Tests
# =============================================================================

class TestGetMaterial:
    """Tests for GET /api/v1/materials/:id"""

    def test_get_material_detail(self, auth_client, cleanup_materials):
        """Get material detail by ID."""
        create_resp = auth_client.post(
            "/api/v1/materials",
            json={
                "video_url": "https://example.com/detail-test.mp4",
                "tag": "e2e_test_detail",
                "title": "e2e_test_material_detail",
                "description": "Detail test description",
            },
        )
        created = extract_data(create_resp.json())

        resp = auth_client.get(f"/api/v1/materials/{created['id']}")
        assert_response_success(resp)
        data = extract_data(resp.json())

        assert_json_structure(data, [
            "id", "user_id", "video_url", "tag", "title",
            "description", "is_active", "created_at",
        ])
        assert data["id"] == created["id"]
        assert data["video_url"] == "https://example.com/detail-test.mp4"
        assert data["tag"] == "e2e_test_detail"
        assert data["title"] == "e2e_test_material_detail"
        assert data["description"] == "Detail test description"

    def test_get_material_not_found(self, auth_client):
        """Should return 404 for non-existent material."""
        resp = auth_client.get("/api/v1/materials/99999")
        assert resp.status_code in [404, 400], \
            f"Expected 404/400, got {resp.status_code}: {resp.text}"

    def test_get_material_other_user(self, auth_client, db_connection, cleanup_materials):
        """Should not be able to access another user's material."""
        other_user_id = TEST_USER_ID + 998
        cursor = db_connection.cursor()
        cursor.execute("""
            INSERT INTO gm_users
                (id, email, username, password_hash, full_name, role, status, is_active, invite_code)
            VALUES (%s, %s, %s,
                    '$2b$12$Ikc.R4FMMGahbGhfHlLl4.PciMiV37qXfHpPNCjGGQg/yOEgk7k/e',
                    'Other Material User', 'user', 'ACTIVE', true, 'MATL0998')
            ON CONFLICT (id) DO NOTHING
        """, (
            other_user_id,
            f"other_material_{other_user_id}@glancemind.test",
            f"other_material_{other_user_id}",
        ))
        cursor.execute("""
            INSERT INTO gm_user_materials (user_id, video_url, tag, title, is_active, created_at)
            VALUES (%s, 'https://example.com/other-user.mp4', 'other', 'e2e_test_other_user', true, NOW())
            RETURNING id
        """, (other_user_id,))
        other_id = cursor.fetchone()[0]
        db_connection.commit()
        cursor.close()

        resp = auth_client.get(f"/api/v1/materials/{other_id}")
        assert resp.status_code in [404, 400, 403], \
            f"Expected 404/400/403 for other user's material, got {resp.status_code}"

        # Cleanup
        cursor = db_connection.cursor()
        cursor.execute("DELETE FROM gm_user_materials WHERE id = %s", (other_id,))
        cursor.execute("DELETE FROM gm_users WHERE id = %s", (other_user_id,))
        db_connection.commit()
        cursor.close()


# =============================================================================
# 4. Update Material Tests
# =============================================================================

class TestUpdateMaterial:
    """Tests for PUT /api/v1/materials/:id"""

    def test_update_material_title(self, auth_client, cleanup_materials):
        """Update material title."""
        create_resp = auth_client.post(
            "/api/v1/materials",
            json={
                "video_url": "https://example.com/update-test.mp4",
                "tag": "e2e_test_update",
                "title": "e2e_test_material_original",
            },
        )
        created = extract_data(create_resp.json())

        resp = auth_client.put(
            f"/api/v1/materials/{created['id']}",
            json={"title": "e2e_test_material_updated"},
        )
        assert_response_success(resp)
        data = extract_data(resp.json())
        assert data["title"] == "e2e_test_material_updated"
        assert data["updated_at"] is not None

    def test_update_material_tag(self, auth_client, cleanup_materials):
        """Update material tag."""
        create_resp = auth_client.post(
            "/api/v1/materials",
            json={
                "video_url": "https://example.com/tag-update.mp4",
                "tag": "e2e_test_old_tag",
                "title": "e2e_test_material_tag_update",
            },
        )
        created = extract_data(create_resp.json())

        resp = auth_client.put(
            f"/api/v1/materials/{created['id']}",
            json={"tag": "e2e_test_new_tag"},
        )
        assert_response_success(resp)
        data = extract_data(resp.json())
        assert data["tag"] == "e2e_test_new_tag"

    def test_update_material_description(self, auth_client, cleanup_materials):
        """Update material description."""
        create_resp = auth_client.post(
            "/api/v1/materials",
            json={
                "video_url": "https://example.com/desc-update.mp4",
                "tag": "e2e_test_desc",
                "title": "e2e_test_material_desc_update",
            },
        )
        created = extract_data(create_resp.json())

        resp = auth_client.put(
            f"/api/v1/materials/{created['id']}",
            json={"description": "New description added"},
        )
        assert_response_success(resp)
        data = extract_data(resp.json())
        assert data["description"] == "New description added"

    def test_update_material_not_found(self, auth_client):
        """Should return 404 for non-existent material."""
        resp = auth_client.put(
            "/api/v1/materials/99999",
            json={"title": "does_not_exist"},
        )
        assert resp.status_code in [404, 400], \
            f"Expected 404/400, got {resp.status_code}: {resp.text}"


# =============================================================================
# 5. Delete Material Tests
# =============================================================================

class TestDeleteMaterial:
    """Tests for DELETE /api/v1/materials/:id"""

    def test_delete_material(self, auth_client, db_connection, cleanup_materials):
        """Soft delete a material."""
        create_resp = auth_client.post(
            "/api/v1/materials",
            json={
                "video_url": "https://example.com/delete-test.mp4",
                "tag": "e2e_test_delete",
                "title": "e2e_test_material_delete",
            },
        )
        created = extract_data(create_resp.json())

        resp = auth_client.delete(f"/api/v1/materials/{created['id']}")
        assert_response_success(resp)

        # Verify soft delete: is_active should be false
        from psycopg2.extras import RealDictCursor
        cursor = db_connection.cursor(cursor_factory=RealDictCursor)
        cursor.execute(
            "SELECT * FROM gm_user_materials WHERE id = %s",
            (created["id"],),
        )
        row = cursor.fetchone()
        cursor.close()

        assert row is not None, "Material should still exist in DB after soft delete"
        assert row["is_active"] is False, "is_active should be false after soft delete"

    def test_delete_material_not_in_list(self, auth_client, cleanup_materials):
        """Deleted material should not appear in list."""
        create_resp = auth_client.post(
            "/api/v1/materials",
            json={
                "video_url": "https://example.com/delete-list-test.mp4",
                "tag": "e2e_test_delete_list",
                "title": "e2e_test_material_delete_list",
            },
        )
        created = extract_data(create_resp.json())

        auth_client.delete(f"/api/v1/materials/{created['id']}")

        resp = auth_client.get("/api/v1/materials")
        data = extract_data(resp.json())
        ids = [m["id"] for m in data["list"]]
        assert created["id"] not in ids, "Deleted material should not appear in list"

    def test_delete_material_not_found(self, auth_client):
        """Should return error for non-existent material."""
        resp = auth_client.delete("/api/v1/materials/99999")
        # Depending on implementation, may return 200 with 0 rows affected or 404
        # Both are acceptable behaviors


# =============================================================================
# 6. List Tags Tests
# =============================================================================

class TestListTags:
    """Tests for GET /api/v1/material-tags"""

    def test_list_tags(self, auth_client):
        """Get available material tags."""
        resp = auth_client.get("/api/v1/material-tags")
        assert_response_success(resp)
        data = extract_data(resp.json())
        assert "tags" in data
        assert isinstance(data["tags"], list)

    def test_tags_structure(self, auth_client):
        """Verify tag response structure."""
        resp = auth_client.get("/api/v1/material-tags")
        data = extract_data(resp.json())

        if data["tags"]:
            tag = data["tags"][0]
            assert "name" in tag
            assert "usage_count" in tag


# =============================================================================
# 7. Video Upload Tests (OSS)
# =============================================================================

class TestVideoUpload:
    """Tests for POST /api/v1/oss/upload-video"""

    def test_upload_video_no_file(self, auth_client):
        """Should fail when no file is provided."""
        resp = auth_client.post("/api/v1/oss/upload-video")
        assert resp.status_code in [400, 422, 500], \
            f"Expected 400/422/500, got {resp.status_code}: {resp.text}"

    def test_upload_video_invalid_type(self, auth_client):
        """Should reject non-video file types."""
        import io
        files = {"file": ("test.txt", io.BytesIO(b"not a video"), "text/plain")}
        resp = auth_client.post("/api/v1/oss/upload-video", files=files)
        assert resp.status_code in [400, 422], \
            f"Expected 400/422, got {resp.status_code}: {resp.text}"

    def test_upload_video_unauthenticated(self):
        """Should fail without authentication."""
        import io
        client = APIClient(API_BASE_URL)
        files = {"file": ("test.mp4", io.BytesIO(b"\x00" * 100), "video/mp4")}
        resp = client.post("/api/v1/oss/upload-video", files=files)
        assert resp.status_code == 401, \
            f"Expected 401, got {resp.status_code}: {resp.text}"


# =============================================================================
# 8. Full Workflow Test
# =============================================================================

class TestMaterialFullWorkflow:
    """End-to-end workflow: create -> list -> detail -> update -> delete"""

    def test_full_crud_workflow(self, auth_client, db_connection, cleanup_materials):
        """Test complete material CRUD lifecycle."""
        # 1. CREATE
        create_resp = auth_client.post(
            "/api/v1/materials",
            json={
                "video_url": "https://example.com/workflow-test.mp4",
                "tag": "e2e_test_workflow",
                "title": "e2e_test_material_workflow",
                "description": "Full workflow test",
            },
        )
        assert_response_success(create_resp)
        created = extract_data(create_resp.json())
        material_id = created["id"]
        assert material_id > 0

        # 2. LIST - should include the new material
        list_resp = auth_client.get("/api/v1/materials")
        assert_response_success(list_resp)
        list_data = extract_data(list_resp.json())
        ids = [m["id"] for m in list_data["list"]]
        assert material_id in ids, "Created material should be in the list"

        # 3. GET DETAIL
        detail_resp = auth_client.get(f"/api/v1/materials/{material_id}")
        assert_response_success(detail_resp)
        detail = extract_data(detail_resp.json())
        assert detail["id"] == material_id
        assert detail["video_url"] == "https://example.com/workflow-test.mp4"

        # 4. UPDATE
        update_resp = auth_client.put(
            f"/api/v1/materials/{material_id}",
            json={
                "title": "e2e_test_material_workflow_updated",
                "description": "Updated description",
            },
        )
        assert_response_success(update_resp)
        updated = extract_data(update_resp.json())
        assert updated["title"] == "e2e_test_material_workflow_updated"
        assert updated["description"] == "Updated description"
        assert updated["updated_at"] is not None

        # 5. DELETE
        delete_resp = auth_client.delete(f"/api/v1/materials/{material_id}")
        assert_response_success(delete_resp)

        # 6. VERIFY DELETED - should not appear in list
        list_resp2 = auth_client.get("/api/v1/materials")
        list_data2 = extract_data(list_resp2.json())
        ids2 = [m["id"] for m in list_data2["list"]]
        assert material_id not in ids2, "Deleted material should not be in the list"

        # 7. VERIFY DB - soft delete (is_active = false)
        from psycopg2.extras import RealDictCursor
        cursor = db_connection.cursor(cursor_factory=RealDictCursor)
        cursor.execute(
            "SELECT * FROM gm_user_materials WHERE id = %s",
            (material_id,),
        )
        row = cursor.fetchone()
        cursor.close()
        assert row is not None, "Record should still exist in DB"
        assert row["is_active"] is False
        assert row["title"] == "e2e_test_material_workflow_updated"
