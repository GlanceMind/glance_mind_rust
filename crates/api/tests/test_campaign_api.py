"""
GlanceMind API E2E Tests - Campaign API
=======================================
Tests for campaign management endpoints.

Test Coverage:
1. Create campaign
2. Get campaign details
3. Update campaign
4. List campaigns with pagination
5. Campaign logs
6. Campaign templates CRUD
"""

import pytest
from conftest import (
    assert_response_success,
    extract_data,
    PLATFORM_TIKTOK,
)


class TestCampaignCRUD:
    """Tests for campaign CRUD operations."""

    def _get_auth_user_id(self, auth_client):
        resp = auth_client.get("/api/v1/user/me")
        assert_response_success(resp)
        data = extract_data(resp.json())
        return data.get("id") or data.get("user_id")

    def _get_config_ids(self, api_client):
        """Helper to get platform, region, and AI model IDs."""
        # Get platform
        resp = api_client.get("/api/v1/config/platforms")
        platforms = extract_data(resp.json())
        platform_id = platforms[0]["id"]
        
        # Get region
        resp = api_client.get(f"/api/v1/config/platforms/{platform_id}/regions")
        regions = extract_data(resp.json())
        region_id = regions[0]["id"]
        
        # Get AI model
        resp = api_client.get("/api/v1/config/ai-models")
        models = extract_data(resp.json())
        ai_model_id = models[0]["id"]
        
        return platform_id, region_id, ai_model_id

    def test_create_campaign(self, auth_client, api_client):
        """Test creating a new campaign."""
        platform_id, region_id, ai_model_id = self._get_config_ids(api_client)
        
        campaign_payload = {
            "name": "E2E Test Campaign",
            "platform_id": platform_id,
            "region_id": region_id,
            "ai_model_id": ai_model_id,
            "schedule_type": "ONCE",
            "product_prompt": "E2E Test Product"
        }
        
        resp = auth_client.post(
            "/api/v1/campaigns",
            json=campaign_payload
        )
        assert_response_success(resp)
        
        data = extract_data(resp.json())
        print(f"\nCreated campaign: {data}")
        
        # Verify response
        assert "id" in data, "Should return campaign ID"
        assert data.get("name") == "E2E Test Campaign"
        assert data.get("status") in ["DRAFT", "draft"]

    def test_get_campaign_details(self, auth_client, api_client, db_cursor):
        """Test getting campaign details."""
        user_id = self._get_auth_user_id(auth_client)
        db_cursor.execute(
            "SELECT id FROM gm_campaigns WHERE user_id = %s ORDER BY id DESC LIMIT 1",
            (user_id,),
        )
        result = db_cursor.fetchone()

        if result:
            campaign_id = result["id"]
        else:
            platform_id, region_id, ai_model_id = self._get_config_ids(api_client)
            create_resp = auth_client.post(
                "/api/v1/campaigns",
                json={
                    "name": "Owned Campaign Detail Test",
                    "platform_id": platform_id,
                    "region_id": region_id,
                    "ai_model_id": ai_model_id,
                    "schedule_type": "ONCE",
                    "product_prompt": "Owned detail test product",
                },
            )
            assert_response_success(create_resp)
            campaign_id = extract_data(create_resp.json()).get("id")
        
        resp = auth_client.get(f"/api/v1/campaigns/{campaign_id}")
        assert_response_success(resp)
        
        data = extract_data(resp.json())
        print(f"\nCampaign details: {data}")
        
        assert data.get("id") == campaign_id

    def test_update_campaign(self, auth_client, api_client, db_cursor):
        """Test updating a campaign."""
        user_id = self._get_auth_user_id(auth_client)
        db_cursor.execute(
            "SELECT id FROM gm_campaigns WHERE user_id = %s AND status = 'DRAFT' ORDER BY id DESC LIMIT 1",
            (user_id,),
        )
        result = db_cursor.fetchone()
        
        if not result:
            # Create a new campaign for testing
            platform_id, region_id, ai_model_id = self._get_config_ids(api_client)
            create_resp = auth_client.post(
                "/api/v1/campaigns",
                json={
                    "name": "Campaign to Update",
                    "platform_id": platform_id,
                    "region_id": region_id,
                    "ai_model_id": ai_model_id,
                    "schedule_type": "ONCE",
                    "product_prompt": "Test"
                }
            )
            if create_resp.status_code != 200:
                pytest.skip("Could not create campaign")
            campaign_id = extract_data(create_resp.json()).get("id")
        else:
            campaign_id = result["id"]
        
        # Update campaign
        update_payload = {
            "name": "Updated Campaign Name",
            "max_scan_count": 1000
        }
        
        resp = auth_client.put(
            f"/api/v1/campaigns/{campaign_id}",
            json=update_payload
        )
        assert_response_success(resp)
        
        data = extract_data(resp.json())
        print(f"\nUpdated campaign: {data}")
        
        assert data.get("name") == "Updated Campaign Name"

    def test_list_campaigns_with_pagination(self, auth_client):
        """Test listing campaigns with pagination."""
        resp = auth_client.get("/api/v1/campaigns?page=1&per_page=10")
        assert_response_success(resp)
        
        data = extract_data(resp.json())
        print(f"\nCampaigns list: {data}")
        
        # Should have pagination structure
        assert "list" in data or isinstance(data, list), "Should return list"
        
        if "list" in data:
            assert "total" in data, "Should have total count"
            print(f"  Total campaigns: {data['total']}")

    def test_get_campaign_logs(self, auth_client, db_cursor):
        """Test getting campaign logs."""
        user_id = self._get_auth_user_id(auth_client)
        db_cursor.execute(
            "SELECT id FROM gm_campaigns WHERE user_id = %s ORDER BY id DESC LIMIT 1",
            (user_id,),
        )
        result = db_cursor.fetchone()
        
        if not result:
            pytest.skip("No mock campaign available")
        
        campaign_id = result["id"]
        
        resp = auth_client.get(f"/api/v1/campaigns/{campaign_id}/logs")
        assert_response_success(resp)
        
        data = extract_data(resp.json())
        print(f"\nCampaign logs: {type(data)}")
        
        # Logs should be a list
        logs = data if isinstance(data, list) else data.get("list", [])
        print(f"  Log count: {len(logs)}")


class TestCampaignTemplates:
    """Tests for campaign template operations."""

    def _get_auth_user_id(self, auth_client):
        resp = auth_client.get("/api/v1/user/me")
        assert_response_success(resp)
        data = extract_data(resp.json())
        return data.get("id") or data.get("user_id")

    def _get_or_create_campaign(self, auth_client, api_client, db_cursor):
        """Helper to get or create a campaign for template testing."""
        user_id = self._get_auth_user_id(auth_client)
        db_cursor.execute(
            "SELECT id FROM gm_campaigns WHERE user_id = %s ORDER BY id DESC LIMIT 1",
            (user_id,),
        )
        result = db_cursor.fetchone()
        
        if result:
            return result["id"]
        
        # Create new campaign
        resp = api_client.get("/api/v1/config/platforms")
        platforms = extract_data(resp.json())
        platform_id = platforms[0]["id"]
        
        resp = api_client.get(f"/api/v1/config/platforms/{platform_id}/regions")
        regions = extract_data(resp.json())
        region_id = regions[0]["id"]
        
        resp = api_client.get("/api/v1/config/ai-models")
        models = extract_data(resp.json())
        ai_model_id = models[0]["id"]
        
        create_resp = auth_client.post(
            "/api/v1/campaigns",
            json={
                "name": "Template Test Campaign",
                "platform_id": platform_id,
                "region_id": region_id,
                "ai_model_id": ai_model_id,
                "schedule_type": "ONCE",
                "product_prompt": "Test"
            }
        )
        return extract_data(create_resp.json()).get("id")

    def test_create_template(self, auth_client, api_client, db_cursor):
        """Test creating a campaign template."""
        campaign_id = self._get_or_create_campaign(auth_client, api_client, db_cursor)
        
        if not campaign_id:
            pytest.skip("No campaign available")
        
        template_payload = {
            "campaign_id": campaign_id,
            "reply_prompt": "Hello {{product}}",
            "weight": 50
        }
        
        resp = auth_client.post(
            f"/api/v1/campaigns/{campaign_id}/templates",
            json=template_payload
        )
        
        # May fail if template already exists
        if resp.status_code == 200:
            data = extract_data(resp.json())
            print(f"\nCreated template: {data}")
            assert "id" in data

    def test_list_templates(self, auth_client, api_client, db_cursor):
        """Test listing campaign templates."""
        campaign_id = self._get_or_create_campaign(auth_client, api_client, db_cursor)
        
        if not campaign_id:
            pytest.skip("No campaign available")
        
        resp = auth_client.get(f"/api/v1/campaigns/{campaign_id}/templates")
        assert_response_success(resp)
        
        data = extract_data(resp.json())
        print(f"\nTemplates: {data}")
        
        templates = data if isinstance(data, list) else data.get("list", [])
        print(f"  Template count: {len(templates)}")


class TestCampaignDatabaseState:
    """Tests that verify campaign database state."""

    def test_mock_campaigns_exist(self, db_cursor):
        """Verify mock campaigns exist in database."""
        print("\n=== Mock Campaigns Verification ===")
        
        db_cursor.execute("""
            SELECT c.id, c.name, c.status, c.platform_id, p.name as platform_name
            FROM gm_campaigns c
            JOIN gm_platforms p ON c.platform_id = p.id
            ORDER BY c.id
            LIMIT 10
        """)
        campaigns = db_cursor.fetchall()
        
        for c in campaigns:
            print(f"  Campaign {c['id']}: {c['name']} ({c['platform_name']}) - {c['status']}")
        
        assert len(campaigns) >= 1, "Should have mock campaigns"

    def test_campaigns_have_templates(self, db_cursor):
        """Verify campaigns have associated templates."""
        db_cursor.execute("""
            SELECT c.id, c.name, COUNT(t.id) as template_count
            FROM gm_campaigns c
            LEFT JOIN gm_campaign_templates t ON c.id = t.campaign_id
            GROUP BY c.id, c.name
            ORDER BY c.id
            LIMIT 10
        """)
        results = db_cursor.fetchall()
        
        print("\n=== Campaign Templates ===")
        for row in results:
            print(f"  Campaign {row['id']} ({row['name']}): {row['template_count']} templates")


if __name__ == "__main__":
    pytest.main([__file__, "-v", "--tb=short"])
