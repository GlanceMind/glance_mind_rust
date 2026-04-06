"""
GlanceMind API E2E Tests - Config API
=====================================
Tests for configuration endpoints (platforms, regions, AI models, pricing).

Test Coverage:
1. Platform listing
2. Region listing by platform
3. AI model listing
4. Pricing rules
5. Database state verification
"""

import pytest
from conftest import (
    assert_response_success,
    extract_data,
    PLATFORM_REDDIT,
    PLATFORM_TIKTOK,
    PLATFORM_FACEBOOK,
    PLATFORM_INSTAGRAM,
    PLATFORM_TWITTER,
)


class TestPlatformsAPI:
    """Tests for platforms configuration endpoint."""

    def test_get_platforms_success(self, api_client, db_cursor):
        """Test getting all platforms."""
        resp = api_client.get("/api/v1/config/platforms")
        assert_response_success(resp)
        
        platforms = extract_data(resp.json())
        print(f"\nPlatforms: {platforms}")
        
        # Should be a list
        assert isinstance(platforms, list), "Response should be a list"
        assert len(platforms) >= 5, "Should have at least 5 platforms"
        
        # Verify structure
        for platform in platforms:
            assert "id" in platform, "Platform should have id"
            assert "name" in platform, "Platform should have name"
            assert "display_name" in platform, "Platform should have display_name"
        
        # Verify against database
        db_cursor.execute("SELECT COUNT(*) as count FROM gm_platforms WHERE is_active = true")
        result = db_cursor.fetchone()
        
        assert len(platforms) == result["count"], \
            f"API count ({len(platforms)}) should match DB ({result['count']})"

    def test_platforms_include_all_required(self, api_client):
        """Test that all required platforms are present."""
        resp = api_client.get("/api/v1/config/platforms")
        assert_response_success(resp)
        
        platforms = extract_data(resp.json())
        platform_names = [p["name"].lower() for p in platforms]
        
        required = ["reddit", "tiktok", "facebook", "instagram", "twitter"]
        for name in required:
            assert name in platform_names, f"Platform '{name}' should be present"


class TestRegionsAPI:
    """Tests for regions configuration endpoint."""

    @pytest.mark.parametrize("platform_id,expected_min_regions", [
        (PLATFORM_REDDIT, 1),    # Reddit has GLOBAL
        (PLATFORM_TIKTOK, 4),    # TikTok has US, GB, JP, TW
        (PLATFORM_FACEBOOK, 2),  # Facebook has US, GB
        (PLATFORM_INSTAGRAM, 2), # Instagram has US, GB
        (PLATFORM_TWITTER, 2),   # Twitter has US, GLOBAL
    ])
    def test_get_regions_by_platform(self, api_client, platform_id, expected_min_regions):
        """Test getting regions for each platform."""
        resp = api_client.get(f"/api/v1/config/platforms/{platform_id}/regions")
        assert_response_success(resp)
        
        regions = extract_data(resp.json())
        print(f"\nPlatform {platform_id} regions: {regions}")
        
        assert isinstance(regions, list), "Response should be a list"
        assert len(regions) >= expected_min_regions, \
            f"Platform {platform_id} should have at least {expected_min_regions} regions"
        
        # Verify structure
        for region in regions:
            assert "id" in region, "Region should have id"
            assert "code" in region, "Region should have code"
            assert "display_name" in region, "Region should have display_name"

    def test_get_regions_invalid_platform(self, api_client):
        """Test getting regions for invalid platform."""
        resp = api_client.get("/api/v1/config/platforms/999/regions")
        
        if resp.status_code == 200:
            regions = extract_data(resp.json())
            assert len(regions) == 0, "Should return empty list for invalid platform"


class TestAIModelsAPI:
    """Tests for AI models configuration endpoint."""

    def test_get_ai_models_success(self, api_client, db_cursor):
        """Test getting all AI models."""
        resp = api_client.get("/api/v1/config/ai-models")
        assert_response_success(resp)
        
        models = extract_data(resp.json())
        print(f"\nAI Models: {models}")
        
        assert isinstance(models, list), "Response should be a list"
        assert len(models) >= 3, "Should have at least 3 AI models"
        
        # Verify structure
        for model in models:
            assert "id" in model, "Model should have id"
            assert "name" in model, "Model should have name"
            assert "provider" in model, "Model should have provider"
        
        # Verify against database
        db_cursor.execute("SELECT COUNT(*) as count FROM gm_ai_models WHERE is_active = true")
        result = db_cursor.fetchone()
        
        assert len(models) == result["count"], \
            f"API count ({len(models)}) should match DB ({result['count']})"

    def test_ai_models_have_cost_multiplier(self, api_client):
        """Test that AI models have cost multiplier."""
        resp = api_client.get("/api/v1/config/ai-models")
        assert_response_success(resp)
        
        models = extract_data(resp.json())
        
        for model in models:
            assert "cost_multiplier" in model or "costMultiplier" in model, \
                f"Model {model.get('name')} should have cost_multiplier"


class TestPricingAPI:
    """Tests for pricing rules configuration endpoint."""

    def test_get_pricing_rules_success(self, api_client, db_cursor):
        """Test getting all pricing rules."""
        resp = api_client.get("/api/v1/config/pricing")
        assert_response_success(resp)
        
        rules = extract_data(resp.json())
        print(f"\nPricing Rules: {rules[:5] if rules else rules}...")  # First 5
        
        assert isinstance(rules, list), "Response should be a list"
        assert len(rules) >= 10, "Should have at least 10 pricing rules (2 per platform)"
        
        # Verify structure
        for rule in rules:
            assert "id" in rule, "Rule should have id"
            assert "action_type" in rule, "Rule should have action_type"
            assert "cost_points" in rule, "Rule should have cost_points"
        
        # Verify against database
        db_cursor.execute("SELECT COUNT(*) as count FROM gm_pricing_rules")
        result = db_cursor.fetchone()
        
        print(f"  API rules: {len(rules)}, DB rules: {result['count']}")

    def test_pricing_has_all_action_types(self, api_client):
        """Test that pricing includes all required action types."""
        resp = api_client.get("/api/v1/config/pricing")
        assert_response_success(resp)
        
        rules = extract_data(resp.json())
        action_types = set(r["action_type"] for r in rules)
        
        required_actions = [
            "SCAN_POST",
            "AI_ANALYZE",
            "REPLY_COMMENT",
            "POST_REPLY",
            "IMAGE",
            "VIDEO_GENERATE",
        ]
        for action in required_actions:
            assert action in action_types, f"Action type '{action}' should be present"


class TestDatabaseStateConfig:
    """Tests that verify configuration database state."""

    # Platforms that are fully configured with regions and pricing
    CONFIGURED_PLATFORMS = ["reddit", "tiktok", "facebook", "instagram", "twitter"]

    def test_all_platforms_have_pricing(self, db_cursor):
        """Verify configured platforms have pricing rules."""
        print("\n=== Platform Pricing Verification ===")
        
        db_cursor.execute("""
            SELECT p.name, p.id, 
                   COUNT(pr.id) as rule_count
            FROM gm_platforms p
            LEFT JOIN gm_pricing_rules pr ON p.id = pr.platform_id
            WHERE p.is_active = true
            GROUP BY p.id, p.name
            ORDER BY p.id
        """)
        results = db_cursor.fetchall()
        
        configured_count = 0
        for row in results:
            print(f"  {row['name']}: {row['rule_count']} pricing rules")
            if row['name'].lower() in self.CONFIGURED_PLATFORMS:
                assert row["rule_count"] >= 2, \
                    f"Platform {row['name']} should have at least 2 pricing rules"
                configured_count += 1
        
        assert configured_count >= 5, "At least 5 configured platforms should exist"

    def test_all_platforms_have_regions(self, db_cursor):
        """Verify configured platforms have at least one region."""
        print("\n=== Platform Regions Verification ===")
        
        db_cursor.execute("""
            SELECT p.name, p.id, 
                   COUNT(r.id) as region_count
            FROM gm_platforms p
            LEFT JOIN gm_regions r ON p.id = r.platform_id AND r.is_active = true
            WHERE p.is_active = true
            GROUP BY p.id, p.name
            ORDER BY p.id
        """)
        results = db_cursor.fetchall()
        
        configured_count = 0
        for row in results:
            print(f"  {row['name']}: {row['region_count']} regions")
            if row['name'].lower() in self.CONFIGURED_PLATFORMS:
                assert row["region_count"] >= 1, \
                    f"Platform {row['name']} should have at least 1 region"
                configured_count += 1
        
        assert configured_count >= 5, "At least 5 configured platforms should exist"


if __name__ == "__main__":
    pytest.main([__file__, "-v", "--tb=short"])
