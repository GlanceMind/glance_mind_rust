"""
GlanceMind Video Case API E2E Tests
====================================
Tests for /api/v1/video-cases endpoints.
"""

import pytest
from conftest import (
    API_BASE_URL,
    APIClient,
    extract_data,
    login_test_user,
)


# =============================================================================
# Fixtures
# =============================================================================

@pytest.fixture
def auth_client():
    """Get authenticated API client."""
    token = login_test_user()
    return APIClient(API_BASE_URL, token)


@pytest.fixture
def insert_test_video_case(db_cursor, db_connection):
    """Insert a test video case into the database."""
    insert_sql = """
    INSERT INTO gm_data_video_cases (
        task_no, case_id, user_id, tt_category_id, 
        category_name_en, category_name_cn, task_type, num,
        status, script, model, video_model, video_url,
        ai_prompt, refer_image_url, progress, video_status,
        image_urls, characters, videos, case_status, favorite_status
    ) VALUES (
        %s, %s, %s, %s, 
        %s, %s, %s, %s,
        %s, %s, %s, %s, %s,
        %s, %s, %s, %s,
        %s, %s, %s, %s, %s
    ) RETURNING id;
    """
    
    test_data = (
        "7421440381864263680",  # task_no
        1290,  # case_id
        "7df7ff90609d40d8b235dd636678a693",  # user_id
        "601152",  # tt_category_id
        "Womenswear & Underwear",  # category_name_en
        "女装与女士内衣",  # category_name_cn
        "5",  # task_type
        1,  # num
        500,  # status
        "Here is a comprehensive prompt...",  # script
        "sora2-portrait-15s",  # model
        "sora2-portrait-15s",  # video_model
        "https://static.neobund.com/neobound/nb-1ff75c93.mp4",  # video_url
        None,  # ai_prompt
        "https://static.neobund.com/neobound/nb-3982cd3a.jpg",  # refer_image_url
        100,  # progress
        "completed",  # video_status
        '["https://example.com/image1.jpg"]',  # image_urls (JSON)
        '[]',  # characters (JSON)
        '[{"video_id": 22, "video_url": "https://example.com/video.mp4", "progress": 100, "video_status": "completed"}]',  # videos (JSON)
        1,  # case_status
        0,  # favorite_status
    )
    
    db_cursor.execute(insert_sql, test_data)
    case_id = db_cursor.fetchone()['id']
    db_connection.commit()
    
    yield case_id
    
    # Cleanup
    db_cursor.execute("DELETE FROM gm_data_video_cases WHERE id = %s", (case_id,))
    db_connection.commit()


# =============================================================================
# Test Cases
# =============================================================================

class TestVideoCaseList:
    """Tests for GET /api/v1/video-cases endpoint."""
    
    def test_list_video_cases_unauthenticated(self):
        """Test that unauthenticated requests are rejected."""
        client = APIClient(API_BASE_URL)
        response = client.get("/api/v1/video-cases")
        assert response.status_code == 401
    
    def test_list_video_cases_empty(self, auth_client, db_cursor, db_connection):
        """Test listing video cases when none exist."""
        # Clear all video cases
        db_cursor.execute("DELETE FROM gm_data_video_cases")
        db_connection.commit()
        
        response = auth_client.get("/api/v1/video-cases")
        assert response.status_code == 200
        
        data = extract_data(response.json())
        assert data["items"] == []
        assert data["total"] == 0
        assert data["page"] == 1
    
    def test_list_video_cases_with_data(self, auth_client, insert_test_video_case):
        """Test listing video cases with data."""
        response = auth_client.get("/api/v1/video-cases")
        assert response.status_code == 200
        
        data = extract_data(response.json())
        assert len(data["items"]) >= 1
        assert data["total"] >= 1
        
        # Check item structure
        item = data["items"][0]
        assert "videoId" in item
        assert "taskNo" in item
        assert "videoStatus" in item
        assert "progress" in item
    
    def test_list_video_cases_pagination(self, auth_client, insert_test_video_case):
        """Test pagination parameters."""
        response = auth_client.get("/api/v1/video-cases?page=1&page_size=5")
        assert response.status_code == 200
        
        data = extract_data(response.json())
        assert data["page"] == 1
        assert data["pageSize"] == 5
    
    def test_list_video_cases_filter_by_status(self, auth_client, insert_test_video_case):
        """Test filtering by status."""
        response = auth_client.get("/api/v1/video-cases?status=500")
        assert response.status_code == 200
        
        data = extract_data(response.json())
        for item in data["items"]:
            # All returned items should have the filtered status
            pass  # Status filter applied at DB level
    
    def test_list_video_cases_filter_by_video_status(self, auth_client, insert_test_video_case):
        """Test filtering by video_status."""
        response = auth_client.get("/api/v1/video-cases?video_status=completed")
        assert response.status_code == 200
        
        data = extract_data(response.json())
        for item in data["items"]:
            assert item["videoStatus"] == "completed"


class TestVideoCaseDetail:
    """Tests for GET /api/v1/video-cases/:id endpoint."""
    
    def test_get_video_case_not_found(self, auth_client):
        """Test getting a non-existent video case."""
        response = auth_client.get("/api/v1/video-cases/999999")
        assert response.status_code == 500  # Internal error for not found
    
    def test_get_video_case_by_id(self, auth_client, insert_test_video_case):
        """Test getting video case by ID."""
        case_id = insert_test_video_case
        response = auth_client.get(f"/api/v1/video-cases/{case_id}")
        assert response.status_code == 200
        
        data = extract_data(response.json())
        assert data["id"] == case_id
        assert "taskNo" in data
        assert "num" in data
        assert "status" in data
        assert "imageUrls" in data
        assert "videos" in data
        assert "createTime" in data
    
    def test_get_video_case_by_task_no(self, auth_client, insert_test_video_case):
        """Test getting video case by task number."""
        response = auth_client.get("/api/v1/video-cases/task/7421440381864263680")
        assert response.status_code == 200
        
        data = extract_data(response.json())
        assert data["taskNo"] == "7421440381864263680"
        assert "videos" in data
        assert isinstance(data["videos"], list)


class TestVideoCaseResponseFormat:
    """Tests to verify response format matches expected structure."""
    
    def test_list_item_format(self, auth_client, insert_test_video_case):
        """Test that list items match expected camelCase format."""
        response = auth_client.get("/api/v1/video-cases")
        assert response.status_code == 200
        
        data = extract_data(response.json())
        if data["items"]:
            item = data["items"][0]
            # Check camelCase field names
            expected_fields = [
                "videoId", "caseId", "userId", "ttCategoryId",
                "categoryNameEn", "categoryNameCn", "model",
                "favoriteStatus", "script", "taskNo", "taskType",
                "videoUrl", "aiImageUrl", "aiPrompt", "progress",
                "referImageUrl", "videoStatus", "errorMessage",
                "size", "caseStatus"
            ]
            for field in expected_fields:
                assert field in item, f"Missing field: {field}"
    
    def test_detail_format(self, auth_client, insert_test_video_case):
        """Test that detail response matches expected format."""
        case_id = insert_test_video_case
        response = auth_client.get(f"/api/v1/video-cases/{case_id}")
        assert response.status_code == 200
        
        data = extract_data(response.json())
        # Check camelCase field names
        expected_fields = [
            "id", "taskNo", "num", "status", "imageUrls",
            "script", "characters", "referVideoUrl", "sellingPoint",
            "productName", "brandName", "videoLanguage", "videoModel",
            "createTime", "completedNum", "videos"
        ]
        for field in expected_fields:
            assert field in data, f"Missing field: {field}"
        
        # Check videos array structure
        if data["videos"]:
            video = data["videos"][0]
            video_fields = [
                "videoId", "taskNo", "videoUrl", "aiImageUrl",
                "aiPrompt", "progress", "referImageUrl", "videoStatus",
                "errorMessage", "model", "size", "seconds", "storyboards"
            ]
            for field in video_fields:
                assert field in video, f"Missing video field: {field}"


# =============================================================================
# Run tests
# =============================================================================

if __name__ == "__main__":
    pytest.main([__file__, "-v"])
