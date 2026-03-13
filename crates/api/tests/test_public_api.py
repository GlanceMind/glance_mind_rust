"""
GlanceMind API E2E Tests - Public API
=====================================
Tests for public (unauthenticated) endpoints.

Test Coverage:
1. Device comments endpoint (all platforms)
2. Tasks by device endpoint
3. Public config endpoints
"""

from decimal import Decimal

import pytest
from conftest import (
    assert_response_success,
    extract_data,
)


def _decimal_value(value) -> Decimal:
    return Decimal(str(value))


class TestDeviceComments:
    """Tests for public device comments endpoint."""

    def test_get_comments_by_device_default_platform(self, api_client):
        """Test getting comments by device ID with default platform (TikTok)."""
        device_id = "test_device_001"
        
        resp = api_client.get(
            f"/api/v1/public/comments/by-device?device_id={device_id}"
        )
        assert_response_success(resp)
        
        data = extract_data(resp.json())
        print(f"\nDevice comments (default): {type(data)}")
        
        # Response has campaign, comments, pagination structure
        assert "comments" in data or "pagination" in data, \
            f"Should have comments or pagination structure, got: {list(data.keys())}"

    @pytest.mark.parametrize("platform", [
        "tiktok",
        "facebook",
        "instagram",
        "reddit",
        "twitter",
    ])
    def test_get_comments_by_device_all_platforms(self, api_client, platform):
        """Test getting comments by device for all platforms."""
        device_id = "test_device_001"
        
        resp = api_client.get(
            f"/api/v1/public/comments/by-device?device_id={device_id}&platform={platform}"
        )
        assert_response_success(resp)
        
        data = extract_data(resp.json())
        print(f"\n  {platform}: {type(data)}")

    def test_get_comments_with_status_filter(self, api_client):
        """Test getting comments with status filter."""
        device_id = "test_device_001"
        
        resp = api_client.get(
            f"/api/v1/public/comments/by-device?device_id={device_id}&platform=tiktok&status=0"
        )
        assert_response_success(resp)
        
        data = extract_data(resp.json())
        print(f"\nFiltered comments: {type(data)}")

    def test_get_comments_with_pagination(self, api_client):
        """Test getting comments with custom pagination."""
        device_id = "test_device_001"
        
        resp = api_client.get(
            f"/api/v1/public/comments/by-device?device_id={device_id}&page=1&per_page=5"
        )
        assert_response_success(resp)
        
        data = extract_data(resp.json())
        
        # Verify pagination params are respected
        if "page" in data:
            assert data["page"] == 1
        if "per_page" in data:
            assert data["per_page"] == 5

    def test_get_comments_platform_case_insensitive(self, api_client):
        """Test that platform parameter is case-insensitive."""
        device_id = "test_device_001"
        
        resp = api_client.get(
            f"/api/v1/public/comments/by-device?device_id={device_id}&platform=TIKTOK"
        )
        assert_response_success(resp)

    def test_get_comments_unknown_platform_fallback(self, api_client):
        """Test that unknown platform falls back to TikTok."""
        device_id = "test_device_001"
        
        resp = api_client.get(
            f"/api/v1/public/comments/by-device?device_id={device_id}&platform=unknown_platform"
        )
        assert_response_success(resp)


class TestTasksByDevice:
    """Tests for public tasks by device endpoint."""

    def test_get_tasks_by_device(self, api_client):
        """Test getting tasks by device ID."""
        device_id = "test_device_001"
        
        resp = api_client.get(
            f"/api/v1/public/tasks/by-device?device_id={device_id}"
        )
        assert_response_success(resp)
        
        data = extract_data(resp.json())
        print(f"\nTasks by device: {type(data)}")
        
        # Should have pagination
        assert "list" in data or "total" in data

    def test_get_tasks_with_status_filter(self, api_client):
        """Test getting tasks with status filter."""
        device_id = "test_device_001"
        
        resp = api_client.get(
            f"/api/v1/public/tasks/by-device?device_id={device_id}&status=init"
        )
        assert_response_success(resp)
        
        data = extract_data(resp.json())
        
        # Empty list for non-existent device is OK
        tasks = data.get("list", [])
        print(f"\n  Tasks with status=init: {len(tasks)}")

    def test_get_tasks_nonexistent_device(self, api_client):
        """Test getting tasks for non-existent device returns empty."""
        resp = api_client.get(
            "/api/v1/public/tasks/by-device?device_id=nonexistent_device_xyz"
        )
        assert_response_success(resp)
        
        data = extract_data(resp.json())
        
        # Should return empty list
        assert data.get("total", 0) == 0 or len(data.get("list", [])) == 0


class TestPublicConfigEndpoints:
    """Tests for public configuration endpoints."""

    def test_platforms_is_public(self, api_client):
        """Test that platforms endpoint is public (no auth required)."""
        resp = api_client.get("/api/v1/config/platforms")
        assert_response_success(resp)
        
        data = extract_data(resp.json())
        assert isinstance(data, list) and len(data) >= 5

    def test_regions_is_public(self, api_client):
        """Test that regions endpoint is public."""
        resp = api_client.get("/api/v1/config/platforms/1/regions")
        assert_response_success(resp)

    def test_ai_models_is_public(self, api_client):
        """Test that AI models endpoint is public."""
        resp = api_client.get("/api/v1/config/ai-models")
        assert_response_success(resp)

    def test_pricing_is_public(self, api_client):
        """Test that pricing endpoint is public."""
        resp = api_client.get("/api/v1/config/pricing")
        assert_response_success(resp)
        data = extract_data(resp.json())
        assert isinstance(data, list) and len(data) > 0

    def test_pricing_contains_points_first_schedule(self, api_client):
        """Pricing payload exposes the refreshed platform and global rules."""
        resp = api_client.get("/api/v1/config/pricing")
        assert_response_success(resp)

        data = extract_data(resp.json())
        assert isinstance(data, list) and data

        def find_rule(action_type, platform_id):
            for rule in data:
                if rule["action_type"] == action_type and rule["platform_id"] == platform_id:
                    return rule
            return None

        scan_rule = next(
            (rule for rule in data if rule["action_type"] == "SCAN_POST" and rule["platform_id"] is not None),
            None,
        )
        assert scan_rule is not None, "Expected a platform-scoped SCAN_POST rule"
        platform_id = scan_rule["platform_id"]

        assert _decimal_value(scan_rule["cost_points"]) == Decimal("2.00")
        assert _decimal_value(find_rule("AI_ANALYZE", platform_id)["cost_points"]) == Decimal("1.00")
        assert _decimal_value(find_rule("REPLY_COMMENT", platform_id)["cost_points"]) == Decimal("0.00")
        assert _decimal_value(find_rule("POST_REPLY", platform_id)["cost_points"]) == Decimal("0.00")
        assert _decimal_value(find_rule("IMAGE", None)["cost_points"]) == Decimal("10.00")
        assert _decimal_value(find_rule("VIDEO_GENERATE", None)["cost_points"]) == Decimal("400.00")

    def test_video_models_expose_refreshed_multipliers(self, api_client):
        """Public video model discovery matches the new multiplier table."""
        resp = api_client.get("/api/v1/config/ai-models?model_type=video")
        assert_response_success(resp)

        data = extract_data(resp.json())
        models_by_key = {model["model_key"]: model for model in data}

        assert _decimal_value(models_by_key["veo-2"]["cost_multiplier"]) == Decimal("1.00")
        assert _decimal_value(models_by_key["vidu-multiframe"]["cost_multiplier"]) == Decimal("3.00")
        assert _decimal_value(models_by_key["vidu-ad-film"]["cost_multiplier"]) == Decimal("3.75")


class TestHealthEndpoint:
    """Tests for health check endpoint."""

    def test_health_endpoint(self, api_client):
        """Test health endpoint returns OK."""
        resp = api_client.get("/health")
        assert_response_success(resp)


if __name__ == "__main__":
    pytest.main([__file__, "-v", "--tb=short"])
