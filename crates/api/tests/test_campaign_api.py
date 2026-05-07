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

import concurrent.futures
import io
import uuid
import zipfile

import pytest
from conftest import (
    API_BASE_URL,
    APIClient,
    assert_response_success,
    extract_data,
    PLATFORM_FACEBOOK,
    PLATFORM_INSTAGRAM,
    PLATFORM_REDDIT,
    PLATFORM_TIKTOK,
    PLATFORM_TWITTER,
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
        resp = api_client.get("/api/v1/config/platforms")
        platforms = extract_data(resp.json())

        platform_id = None
        region_id = None
        for platform in platforms:
            candidate_platform_id = platform["id"]
            resp = api_client.get(f"/api/v1/config/platforms/{candidate_platform_id}/regions")
            regions = extract_data(resp.json())
            if regions:
                platform_id = candidate_platform_id
                region_id = regions[0]["id"]
                break

        assert platform_id is not None and region_id is not None, (
            "Expected at least one platform with configured regions"
        )

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
            "product_prompt": "E2E Test Product",
            "max_scan_count": 1,
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

    def _create_reusable_template(self, auth_client, suffix=None):
        unique = suffix or uuid.uuid4().hex[:8]
        resp = auth_client.post(
            "/api/v1/reply-template-library",
            json={
                "name": f"Campaign reusable {unique}",
                "description": f"campaign reusable description {unique}",
                "weight": 50,
                "reply_prompt": f"campaign reusable reply {unique}",
            },
        )
        assert_response_success(resp)
        return extract_data(resp.json())

    def _create_campaign_payload(self, api_client, suffix=None, **overrides):
        platform_id, region_id, ai_model_id = self._get_config_ids(api_client)
        unique = suffix or uuid.uuid4().hex[:8]
        payload = {
            "name": f"Reply Template IDs Campaign {unique}",
            "platform_id": platform_id,
            "region_id": region_id,
            "ai_model_id": ai_model_id,
            "schedule_type": "ONCE",
            "product_prompt": f"Reply template IDs product {unique}",
            "max_scan_count": 1,
        }
        payload.update(overrides)
        return payload

    def _db_reply_template_ids(self, db_cursor, db_connection, campaign_id):
        db_connection.commit()
        db_cursor.execute(
            "SELECT reply_template_ids FROM gm_campaigns WHERE id = %s",
            (campaign_id,),
        )
        return db_cursor.fetchone()["reply_template_ids"]

    def test_campaign_create_accepts_reply_template_ids_and_deduplicates_preserving_order(
        self, auth_client, api_client, db_cursor, db_connection
    ):
        first = self._create_reusable_template(auth_client)
        second = self._create_reusable_template(auth_client)
        payload = self._create_campaign_payload(
            api_client,
            reply_template_ids=[first["id"], second["id"], first["id"]],
        )

        resp = auth_client.post("/api/v1/campaigns", json=payload)
        assert_response_success(resp)
        data = extract_data(resp.json())

        assert data["reply_template_ids"] == [first["id"], second["id"]]
        assert self._db_reply_template_ids(db_cursor, db_connection, data["id"]) == [
            first["id"],
            second["id"],
        ]

        detail_resp = auth_client.get(f"/api/v1/campaigns/{data['id']}")
        assert_response_success(detail_resp)
        detail = extract_data(detail_resp.json())
        assert detail["reply_template_ids"] == [first["id"], second["id"]]

    def test_create_campaign_null_reply_template_ids_serializes_empty(
        self, auth_client, api_client, db_cursor, db_connection
    ):
        payload = self._create_campaign_payload(api_client, reply_template_ids=None)

        resp = auth_client.post("/api/v1/campaigns", json=payload)
        assert_response_success(resp)
        data = extract_data(resp.json())

        assert data["reply_template_ids"] == []
        assert self._db_reply_template_ids(db_cursor, db_connection, data["id"]) == []

    def test_campaign_update_omitted_reply_template_ids_keeps_existing(
        self, auth_client, api_client, db_cursor, db_connection
    ):
        reusable = self._create_reusable_template(auth_client)
        db_side_reusable = self._create_reusable_template(auth_client)
        create_resp = auth_client.post(
            "/api/v1/campaigns",
            json=self._create_campaign_payload(api_client, reply_template_ids=[reusable["id"]]),
        )
        assert_response_success(create_resp)
        campaign_id = extract_data(create_resp.json())["id"]

        db_cursor.execute(
            """
            UPDATE gm_campaigns
            SET reply_template_ids = ARRAY[%s]::INTEGER[]
            WHERE id = %s
            """,
            (db_side_reusable["id"], campaign_id),
        )
        db_connection.commit()

        update_resp = auth_client.put(
            f"/api/v1/campaigns/{campaign_id}",
            json={"name": "Reply Template IDs Kept"},
        )
        assert_response_success(update_resp)
        data = extract_data(update_resp.json())

        assert data["reply_template_ids"] == [db_side_reusable["id"]]
        assert self._db_reply_template_ids(db_cursor, db_connection, campaign_id) == [
            db_side_reusable["id"]
        ]

    def test_update_campaign_null_reply_template_ids_keeps_existing(
        self, auth_client, api_client, db_cursor, db_connection
    ):
        reusable = self._create_reusable_template(auth_client)
        create_resp = auth_client.post(
            "/api/v1/campaigns",
            json=self._create_campaign_payload(api_client, reply_template_ids=[reusable["id"]]),
        )
        assert_response_success(create_resp)
        campaign_id = extract_data(create_resp.json())["id"]

        update_resp = auth_client.put(
            f"/api/v1/campaigns/{campaign_id}",
            json={
                "name": "Reply Template IDs Null Kept",
                "reply_template_ids": None,
            },
        )
        assert_response_success(update_resp)
        data = extract_data(update_resp.json())

        assert data["reply_template_ids"] == [reusable["id"]]
        assert self._db_reply_template_ids(db_cursor, db_connection, campaign_id) == [
            reusable["id"]
        ]

    def test_update_campaign_replaces_reply_template_ids_with_deduped_order(
        self, auth_client, api_client, db_cursor, db_connection
    ):
        original = self._create_reusable_template(auth_client)
        replacement_first = self._create_reusable_template(auth_client)
        replacement_second = self._create_reusable_template(auth_client)
        create_resp = auth_client.post(
            "/api/v1/campaigns",
            json=self._create_campaign_payload(api_client, reply_template_ids=[original["id"]]),
        )
        assert_response_success(create_resp)
        campaign_id = extract_data(create_resp.json())["id"]

        update_resp = auth_client.put(
            f"/api/v1/campaigns/{campaign_id}",
            json={
                "reply_template_ids": [
                    replacement_first["id"],
                    replacement_second["id"],
                    replacement_first["id"],
                ]
            },
        )
        assert_response_success(update_resp)
        data = extract_data(update_resp.json())

        expected_ids = [replacement_first["id"], replacement_second["id"]]
        assert data["reply_template_ids"] == expected_ids
        assert self._db_reply_template_ids(db_cursor, db_connection, campaign_id) == expected_ids

        detail_resp = auth_client.get(f"/api/v1/campaigns/{campaign_id}")
        assert_response_success(detail_resp)
        detail = extract_data(detail_resp.json())
        assert detail["reply_template_ids"] == expected_ids

    def test_campaign_update_empty_reply_template_ids_clears_associations(
        self, auth_client, api_client, db_cursor, db_connection
    ):
        reusable = self._create_reusable_template(auth_client)
        create_resp = auth_client.post(
            "/api/v1/campaigns",
            json=self._create_campaign_payload(api_client, reply_template_ids=[reusable["id"]]),
        )
        assert_response_success(create_resp)
        campaign_id = extract_data(create_resp.json())["id"]

        update_resp = auth_client.put(
            f"/api/v1/campaigns/{campaign_id}",
            json={"reply_template_ids": []},
        )
        assert_response_success(update_resp)
        data = extract_data(update_resp.json())

        assert data["reply_template_ids"] == []
        assert self._db_reply_template_ids(db_cursor, db_connection, campaign_id) == []

    def test_update_campaign_reply_template_ids_removes_stale_reusable_bindings_only(
        self, auth_client, api_client, db_cursor, db_connection
    ):
        kept = self._create_reusable_template(auth_client)
        stale = self._create_reusable_template(auth_client)
        create_resp = auth_client.post(
            "/api/v1/campaigns",
            json=self._create_campaign_payload(
                api_client,
                reply_template_ids=[kept["id"], stale["id"]],
            ),
        )
        assert_response_success(create_resp)
        campaign_id = extract_data(create_resp.json())["id"]

        for template in (kept, stale):
            assign_resp = auth_client.post(
                f"/api/v1/reply-template-library/{template['id']}/assign",
                json={"campaign_id": campaign_id},
            )
            assert_response_success(assign_resp)

        campaign_only_resp = auth_client.post(
            f"/api/v1/campaigns/{campaign_id}/templates",
            json={"reply_prompt": "campaign-only reply survives", "weight": 31},
        )
        assert_response_success(campaign_only_resp)
        campaign_only_id = extract_data(campaign_only_resp.json())["id"]

        update_resp = auth_client.put(
            f"/api/v1/campaigns/{campaign_id}",
            json={"reply_template_ids": [kept["id"]]},
        )
        assert_response_success(update_resp)
        assert extract_data(update_resp.json())["reply_template_ids"] == [kept["id"]]

        db_connection.commit()
        db_cursor.execute(
            """
            SELECT id, library_template_id
            FROM gm_campaign_templates
            WHERE campaign_id = %s
            ORDER BY library_template_id NULLS LAST, id
            """,
            (campaign_id,),
        )
        rows = db_cursor.fetchall()

        assert any(row["library_template_id"] == kept["id"] for row in rows)
        assert all(row["library_template_id"] != stale["id"] for row in rows)
        assert any(
            row["id"] == campaign_only_id and row["library_template_id"] is None
            for row in rows
        )
        assert self._db_reply_template_ids(db_cursor, db_connection, campaign_id) == [kept["id"]]

    def test_campaign_reply_template_ids_reject_unknown_or_foreign_ids(
        self, auth_client, api_client, db_cursor, db_connection
    ):
        user_id = self._get_auth_user_id(auth_client)
        unique = uuid.uuid4().hex[:8]
        db_cursor.execute(
            """
            INSERT INTO gm_users (
                email,
                username,
                password_hash,
                status,
                full_name,
                role,
                is_active,
                created_at,
                permissions
            )
            VALUES (%s, %s, 'not-used', 'active', 'Foreign User', 'user', true, NOW(), 0)
            RETURNING id
            """,
            (f"foreign-{unique}@example.com", f"foreign-{unique}"),
        )
        foreign_user_id = db_cursor.fetchone()["id"]
        db_cursor.execute(
            """
            INSERT INTO gm_reply_template_library (
                user_id, name, description, weight, reply_prompt, usage_count, created_at
            )
            VALUES (%s, 'Foreign reusable', NULL, 50, 'foreign reply', 0, NOW())
            RETURNING id
            """,
            (foreign_user_id,),
        )
        foreign_id = db_cursor.fetchone()["id"]
        db_connection.commit()

        for candidate_id in (2147480000, foreign_id):
            resp = auth_client.post(
                "/api/v1/campaigns",
                json=self._create_campaign_payload(api_client, reply_template_ids=[candidate_id]),
            )
            assert resp.status_code == 404, (
                f"Expected TemplateNotFound for user={user_id} template={candidate_id}, got {resp.text}"
            )
            body = resp.json()
            assert body["code"] == 4200
            assert "Template not found" in body["msg"]

    def test_campaign_reply_template_ids_reject_non_positive_and_more_than_100_ids(
        self, auth_client, api_client
    ):
        create_resp = auth_client.post(
            "/api/v1/campaigns",
            json=self._create_campaign_payload(api_client),
        )
        assert_response_success(create_resp)
        campaign_id = extract_data(create_resp.json())["id"]

        for ids in ([0], [-1], list(range(1, 102))):
            resp = auth_client.post(
                "/api/v1/campaigns",
                json=self._create_campaign_payload(api_client, reply_template_ids=ids),
            )
            assert resp.status_code == 400, f"Expected bad request for ids={ids[:3]}, got {resp.text}"

            update_resp = auth_client.put(
                f"/api/v1/campaigns/{campaign_id}",
                json={"reply_template_ids": ids},
            )
            assert update_resp.status_code == 400, (
                f"Expected update bad request for ids={ids[:3]}, got {update_resp.text}"
            )

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
                "product_prompt": "Test",
                "max_scan_count": 1,
            }
        )
        return extract_data(create_resp.json()).get("id")

    def _create_reusable_template(self, auth_client, suffix=None, **overrides):
        unique = suffix or uuid.uuid4().hex[:8]
        payload = {
            "name": f"E2E reusable reply style {unique}",
            "description": f"reusable description {unique}",
            "weight": 77,
            "reply_prompt": f"library reply prompt {unique}",
            "dm_prompt": f"library dm prompt {unique}",
            "reply_post_prompt": f"library post prompt {unique}",
        }
        payload.update(overrides)

        resp = auth_client.post("/api/v1/reply-template-library", json=payload)
        assert_response_success(resp)
        return extract_data(resp.json()), payload

    def _fill_campaign_reply_template_ids_to_cap(
        self, auth_client, db_cursor, db_connection, campaign_id
    ):
        user_id = self._get_auth_user_id(auth_client)
        capped_ids = list(range(10000, 10100))
        library_values = ",\n".join(
            "(%s, %s, %s, NULL, 50, NULL, %s, NULL, 0, NOW(), NULL)"
            for _template_id in capped_ids
        )
        params = []
        for template_id in capped_ids:
            params.extend(
                [
                    template_id,
                    user_id,
                    f"Cap reusable {template_id}",
                    f"Cap reply {template_id}",
                ]
            )
        db_cursor.execute(
            f"""
            INSERT INTO gm_reply_template_library (
                id,
                user_id,
                name,
                description,
                weight,
                dm_prompt,
                reply_prompt,
                reply_post_prompt,
                usage_count,
                created_at,
                updated_at
            )
            VALUES
                {library_values}
            ON CONFLICT (id) DO NOTHING
            """,
            params,
        )
        db_cursor.execute(
            """
            UPDATE gm_campaigns
            SET reply_template_ids = %s::INTEGER[]
            WHERE id = %s
            """,
            (capped_ids, campaign_id),
        )
        db_connection.commit()
        return capped_ids

    def _create_fresh_campaign(self, auth_client, api_client, name_suffix):
        platform_id, region_id, ai_model_id = TestCampaignCRUD()._get_config_ids(api_client)
        create_resp = auth_client.post(
            "/api/v1/campaigns",
            json={
                "name": f"Reusable Template E2E {name_suffix}",
                "platform_id": platform_id,
                "region_id": region_id,
                "ai_model_id": ai_model_id,
                "schedule_type": "ONCE",
                "product_prompt": f"Reusable template product {name_suffix}",
                "max_scan_count": 1,
            },
        )
        assert_response_success(create_resp)
        return extract_data(create_resp.json())["id"]

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

    def test_create_campaign_scoped_template_uses_path_campaign_id_when_body_omits_it(
        self, auth_client, api_client, db_cursor, db_connection
    ):
        """POST /campaigns/:id/templates must accept a body that omits campaign_id.

        Contract: the campaign id comes from the URL path. Before this was
        locked in, a minimal body (`reply_prompt` + `weight`) caused a 400
        because serde required `campaign_id` on the DTO. The full-stack
        scenario exercises this via gm-e2e, but we also need a hermetic
        API-level integration check that inspects the persisted row.
        """
        campaign_id = self._get_or_create_campaign(auth_client, api_client, db_cursor)
        if not campaign_id:
            pytest.skip("No campaign available")

        unique = uuid.uuid4().hex[:6]
        resp = auth_client.post(
            f"/api/v1/campaigns/{campaign_id}/templates",
            json={
                "weight": 11,
                "reply_prompt": f"campaign-only legacy reply {unique}",
            },
        )
        assert_response_success(resp)
        data = extract_data(resp.json())
        template_id = data["id"]

        assert data["campaign_id"] == campaign_id
        assert data["weight"] == 11
        assert data["reply_prompt"] == f"campaign-only legacy reply {unique}"
        assert data.get("library_template_id") in (None, 0)

        db_cursor.execute(
            """
            SELECT campaign_id, weight, reply_prompt, library_template_id
              FROM gm_campaign_templates
             WHERE id = %s
            """,
            (template_id,),
        )
        row = db_cursor.fetchone()
        assert row is not None, "template row must be persisted"
        assert row["campaign_id"] == campaign_id
        assert row["weight"] == 11
        assert row["reply_prompt"] == f"campaign-only legacy reply {unique}"
        assert row["library_template_id"] is None

        db_connection.commit()

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

    def test_create_library_backed_campaign_template_over_cap_returns_client_error(
        self, auth_client, api_client, db_cursor, db_connection
    ):
        campaign_id = self._create_fresh_campaign(auth_client, api_client, uuid.uuid4().hex[:8])
        capped_ids = self._fill_campaign_reply_template_ids_to_cap(
            auth_client, db_cursor, db_connection, campaign_id
        )
        reusable, _payload = self._create_reusable_template(auth_client)

        create_resp = auth_client.post(
            f"/api/v1/campaigns/{campaign_id}/templates",
            json={"library_template_id": reusable["id"], "weight": 12},
        )
        assert create_resp.status_code == 400, create_resp.text
        assert "reply_template_ids" in create_resp.text

        db_connection.commit()
        db_cursor.execute(
            """
            SELECT reply_template_ids
            FROM gm_campaigns
            WHERE id = %s
            """,
            (campaign_id,),
        )
        assert db_cursor.fetchone()["reply_template_ids"] == capped_ids
        db_cursor.execute(
            """
            SELECT COUNT(*) AS row_count
            FROM gm_campaign_templates
            WHERE campaign_id = %s
              AND library_template_id = %s
            """,
            (campaign_id, reusable["id"]),
        )
        assert db_cursor.fetchone()["row_count"] == 0

    def test_update_campaign_template_to_library_over_cap_returns_client_error(
        self, auth_client, api_client, db_cursor, db_connection
    ):
        campaign_id = self._create_fresh_campaign(auth_client, api_client, uuid.uuid4().hex[:8])
        capped_ids = self._fill_campaign_reply_template_ids_to_cap(
            auth_client, db_cursor, db_connection, campaign_id
        )
        reusable, _payload = self._create_reusable_template(auth_client)
        campaign_only_resp = auth_client.post(
            f"/api/v1/campaigns/{campaign_id}/templates",
            json={"reply_prompt": "campaign-only over-cap update", "weight": 9},
        )
        assert_response_success(campaign_only_resp)
        campaign_only = extract_data(campaign_only_resp.json())

        update_resp = auth_client.put(
            f"/api/v1/templates/{campaign_only['id']}",
            json={"library_template_id": reusable["id"]},
        )
        assert update_resp.status_code == 400, update_resp.text
        assert "reply_template_ids" in update_resp.text

        db_connection.commit()
        db_cursor.execute(
            """
            SELECT reply_template_ids
            FROM gm_campaigns
            WHERE id = %s
            """,
            (campaign_id,),
        )
        assert db_cursor.fetchone()["reply_template_ids"] == capped_ids
        db_cursor.execute(
            """
            SELECT library_template_id
            FROM gm_campaign_templates
            WHERE id = %s
            """,
            (campaign_only["id"],),
        )
        assert db_cursor.fetchone()["library_template_id"] is None

    def test_assign_reusable_template_appends_campaign_reply_template_id_once(
        self, auth_client, api_client, db_cursor, db_connection
    ):
        """Reusable template assignment should persist the DB shape consumed by agent_rs."""
        campaign_id = self._get_or_create_campaign(auth_client, api_client, db_cursor)
        if not campaign_id:
            pytest.skip("No campaign available")

        unique = uuid.uuid4().hex[:8]
        library_payload = {
            "name": f"E2E reusable reply style {unique}",
            "description": "front+api+agent_rs contract test",
            "weight": 77,
            "reply_prompt": f"library reply prompt {unique}",
            "dm_prompt": f"library dm prompt {unique}",
            "reply_post_prompt": f"library post prompt {unique}",
        }

        create_resp = auth_client.post("/api/v1/reply-template-library", json=library_payload)
        assert_response_success(create_resp)
        reusable = extract_data(create_resp.json())

        assign_resp = auth_client.post(
            f"/api/v1/reply-template-library/{reusable['id']}/assign",
            json={"campaign_id": campaign_id},
        )
        assert_response_success(assign_resp)
        assignment = extract_data(assign_resp.json())

        reassign_resp = auth_client.post(
            f"/api/v1/reply-template-library/{reusable['id']}/assign",
            json={"campaign_id": campaign_id, "weight": 23},
        )
        assert_response_success(reassign_resp)
        reassignment = extract_data(reassign_resp.json())

        assert assignment["campaign_id"] == campaign_id
        assert assignment["library_template_id"] == reusable["id"]
        assert assignment["weight"] == library_payload["weight"]
        assert assignment["reusable_template"]["reply_prompt"] == library_payload["reply_prompt"]
        assert assignment["reusable_template"]["usage_count"] == 1
        assert reassignment["reusable_template"]["usage_count"] == 1
        assert reassignment["weight"] == 23
        assert reassignment["id"] == assignment["id"]

        db_connection.commit()
        db_cursor.execute(
            """
            SELECT
                ct.id,
                ct.library_template_id,
                ct.reply_prompt AS campaign_reply_prompt,
                ct.dm_prompt AS campaign_dm_prompt,
                library.reply_prompt AS library_reply_prompt,
                library.dm_prompt AS library_dm_prompt,
                library.reply_post_prompt AS library_post_prompt,
                ct.weight AS assignment_weight,
                (
                    SELECT COUNT(*)
                    FROM gm_campaign_templates bound
                    WHERE bound.library_template_id = library.id
                ) AS binding_count
            FROM gm_campaign_templates ct
            JOIN gm_reply_template_library library ON library.id = ct.library_template_id
            WHERE ct.campaign_id = %s AND ct.library_template_id = %s
            """,
            (campaign_id, reusable["id"]),
        )
        row = db_cursor.fetchone()

        assert row is not None
        assert row["library_template_id"] == reusable["id"]
        assert row["campaign_reply_prompt"] == library_payload["reply_prompt"]
        assert row["library_reply_prompt"] == library_payload["reply_prompt"]
        assert row["library_dm_prompt"] == library_payload["dm_prompt"]
        assert row["library_post_prompt"] == library_payload["reply_post_prompt"]
        assert row["assignment_weight"] == 23
        assert row["binding_count"] == 1

        db_cursor.execute(
            "SELECT reply_template_ids FROM gm_campaigns WHERE id = %s",
            (campaign_id,),
        )
        assert db_cursor.fetchone()["reply_template_ids"] == [reusable["id"]]

        db_cursor.execute(
            """
            SELECT COUNT(*) AS row_count
            FROM gm_campaign_templates
            WHERE campaign_id = %s AND library_template_id = %s
            """,
            (campaign_id, reusable["id"]),
        )
        assert db_cursor.fetchone()["row_count"] == 1

        db_cursor.execute(
            """
            SELECT
                reply_template_ids,
                cardinality(reply_template_ids) AS reply_template_id_count
            FROM gm_campaigns
            WHERE id = %s
            """,
            (campaign_id,),
        )
        campaign_row = db_cursor.fetchone()
        assert campaign_row["reply_template_ids"] == [reusable["id"]]
        assert campaign_row["reply_template_id_count"] == 1

        reusable_list_resp = auth_client.get("/api/v1/reply-template-library?page=1&page_size=10")
        assert_response_success(reusable_list_resp)
        reusable_list = extract_data(reusable_list_resp.json())["list"]
        listed_reusable = next(item for item in reusable_list if item["id"] == reusable["id"])
        assert listed_reusable["usage_count"] == 1

        updated_reply_prompt = f"library reply prompt updated {unique}"
        update_resp = auth_client.put(
            f"/api/v1/reply-template-library/{reusable['id']}",
            json={
                "reply_prompt": updated_reply_prompt,
                "dm_prompt": None,
            },
        )
        assert_response_success(update_resp)
        updated_reusable = extract_data(update_resp.json())
        assert updated_reusable["reply_prompt"] == updated_reply_prompt
        assert updated_reusable["dm_prompt"] is None
        assert updated_reusable["usage_count"] == 1

        detail_resp = auth_client.get(f"/api/v1/templates/{assignment['id']}")
        assert_response_success(detail_resp)
        detail = extract_data(detail_resp.json())
        assert detail["reply_prompt"] == updated_reply_prompt
        assert detail["weight"] == 23
        assert detail["dm_prompt"] is None
        assert detail["reusable_template"]["reply_prompt"] == updated_reply_prompt
        assert detail["reusable_template"]["dm_prompt"] is None

        list_resp = auth_client.get(
            f"/api/v1/campaigns/{campaign_id}/templates?page=1&page_size=10"
        )
        assert_response_success(list_resp)
        template_list = extract_data(list_resp.json())["list"]
        listed_template = next(item for item in template_list if item["id"] == assignment["id"])
        assert listed_template["reply_prompt"] == updated_reply_prompt
        assert listed_template["weight"] == 23
        assert listed_template["dm_prompt"] is None
        assert listed_template["reusable_template"]["reply_prompt"] == updated_reply_prompt
        assert listed_template["reusable_template"]["dm_prompt"] is None

        reusable_detail_resp = auth_client.get(f"/api/v1/reply-template-library/{reusable['id']}")
        assert_response_success(reusable_detail_resp)
        assert extract_data(reusable_detail_resp.json())["usage_count"] == 1

        db_connection.commit()
        db_cursor.execute(
            """
            SELECT
                id,
                campaign_id,
                library_template_id,
                weight,
                reply_prompt,
                created_at,
                updated_at,
                dm_prompt,
                reply_post_prompt,
                name
            FROM gm_resolved_campaign_templates
            WHERE id = %s
            """,
            (assignment["id"],),
        )
        resolved = db_cursor.fetchone()

        assert resolved is not None
        assert resolved["campaign_id"] == campaign_id
        assert resolved["library_template_id"] == reusable["id"]
        assert resolved["weight"] == 23
        assert resolved["reply_prompt"] == updated_reply_prompt
        assert resolved["dm_prompt"] is None
        assert resolved["reply_post_prompt"] == library_payload["reply_post_prompt"]

    def test_assign_reusable_template_concurrent_calls_do_not_duplicate_campaign_ids(
        self, auth_client, api_client, db_cursor, db_connection
    ):
        """Concurrent assignment should converge to one campaign binding and one usage."""
        campaign_id = self._create_fresh_campaign(auth_client, api_client, uuid.uuid4().hex[:8])
        reusable, payload = self._create_reusable_template(auth_client)
        token = auth_client.token

        def assign(weight):
            client = APIClient(API_BASE_URL, token)
            resp = client.post(
                f"/api/v1/reply-template-library/{reusable['id']}/assign",
                json={"campaign_id": campaign_id, "weight": weight},
            )
            assert_response_success(resp)
            return extract_data(resp.json())

        with concurrent.futures.ThreadPoolExecutor(max_workers=8) as executor:
            assignments = list(executor.map(assign, range(20, 28)))

        assignment_ids = {item["id"] for item in assignments}
        assert len(assignment_ids) == 1
        assert {item["campaign_id"] for item in assignments} == {campaign_id}
        assert {item["library_template_id"] for item in assignments} == {reusable["id"]}

        detail_resp = auth_client.get(f"/api/v1/reply-template-library/{reusable['id']}")
        assert_response_success(detail_resp)
        assert extract_data(detail_resp.json())["usage_count"] == 1

        db_connection.commit()
        db_cursor.execute(
            """
            SELECT COUNT(*) AS row_count
            FROM gm_campaign_templates
            WHERE campaign_id = %s AND library_template_id = %s
            """,
            (campaign_id, reusable["id"]),
        )
        assert db_cursor.fetchone()["row_count"] == 1

        db_cursor.execute(
            """
            SELECT reply_template_ids
            FROM gm_campaigns
            WHERE id = %s
            """,
            (campaign_id,),
        )
        assert db_cursor.fetchone()["reply_template_ids"] == [reusable["id"]]

        db_cursor.execute(
            """
            SELECT reply_prompt, dm_prompt, reply_post_prompt
            FROM gm_resolved_campaign_templates
            WHERE campaign_id = %s AND library_template_id = %s
            """,
            (campaign_id, reusable["id"]),
        )
        resolved = db_cursor.fetchone()
        assert resolved["reply_prompt"] == payload["reply_prompt"]
        assert resolved["dm_prompt"] == payload["dm_prompt"]
        assert resolved["reply_post_prompt"] == payload["reply_post_prompt"]

    def test_reusable_template_assignment_over_cap_returns_client_error(
        self, auth_client, api_client, db_cursor, db_connection
    ):
        campaign_id = self._create_fresh_campaign(auth_client, api_client, uuid.uuid4().hex[:8])
        over_cap_reusable, _payload = self._create_reusable_template(auth_client)
        capped_ids = self._fill_campaign_reply_template_ids_to_cap(
            auth_client, db_cursor, db_connection, campaign_id
        )

        assign_resp = auth_client.post(
            f"/api/v1/reply-template-library/{over_cap_reusable['id']}/assign",
            json={"campaign_id": campaign_id},
        )
        assert assign_resp.status_code == 400, assign_resp.text
        assert "reply_template_ids" in assign_resp.text

        db_cursor.execute(
            """
            SELECT reply_template_ids
            FROM gm_campaigns
            WHERE id = %s
            """,
            (campaign_id,),
        )
        assert db_cursor.fetchone()["reply_template_ids"] == capped_ids

    def test_reusable_template_usage_counts_distinct_campaigns_from_new_field_and_legacy_table(
        self, auth_client, api_client, db_cursor, db_connection
    ):
        """usage_count should reflect distinct campaign assignments, not click count."""
        reusable, _payload = self._create_reusable_template(auth_client)
        campaign_ids = [
            self._create_fresh_campaign(auth_client, api_client, f"{uuid.uuid4().hex[:8]}-{index}")
            for index in range(3)
        ]

        for campaign_id in campaign_ids:
            resp = auth_client.post(
                f"/api/v1/reply-template-library/{reusable['id']}/assign",
                json={"campaign_id": campaign_id},
            )
            assert_response_success(resp)

        repeat_resp = auth_client.post(
            f"/api/v1/reply-template-library/{reusable['id']}/assign",
            json={"campaign_id": campaign_ids[0], "weight": 11},
        )
        assert_response_success(repeat_resp)

        detail_resp = auth_client.get(f"/api/v1/reply-template-library/{reusable['id']}")
        assert_response_success(detail_resp)
        assert extract_data(detail_resp.json())["usage_count"] == len(campaign_ids)

        db_connection.commit()
        db_cursor.execute(
            """
            SELECT COUNT(*) AS row_count, COUNT(DISTINCT campaign_id) AS campaign_count
            FROM gm_campaign_templates
            WHERE library_template_id = %s
            """,
            (reusable["id"],),
        )
        counts = db_cursor.fetchone()
        assert counts["row_count"] == len(campaign_ids)
        assert counts["campaign_count"] == len(campaign_ids)

        new_field_only_campaign_id = self._create_fresh_campaign(
            auth_client, api_client, f"new-field-only-{uuid.uuid4().hex[:8]}"
        )
        db_cursor.execute(
            """
            UPDATE gm_campaigns
            SET reply_template_ids = ARRAY[%s]::INTEGER[]
            WHERE id = %s
            """,
            (reusable["id"], new_field_only_campaign_id),
        )
        db_cursor.execute(
            """
            UPDATE gm_campaigns
            SET reply_template_ids = array_append(reply_template_ids, %s)
            WHERE id = %s
            """,
            (reusable["id"], campaign_ids[0]),
        )
        db_connection.commit()

        detail_resp = auth_client.get(f"/api/v1/reply-template-library/{reusable['id']}")
        assert_response_success(detail_resp)
        assert extract_data(detail_resp.json())["usage_count"] == len(campaign_ids) + 1

    def test_delete_campaign_template_removes_reusable_id_but_keeps_campaign_only_templates(
        self, auth_client, api_client, db_cursor, db_connection
    ):
        campaign_id = self._create_fresh_campaign(auth_client, api_client, uuid.uuid4().hex[:8])
        reusable, _payload = self._create_reusable_template(auth_client)

        assign_resp = auth_client.post(
            f"/api/v1/reply-template-library/{reusable['id']}/assign",
            json={"campaign_id": campaign_id},
        )
        assert_response_success(assign_resp)
        assignment = extract_data(assign_resp.json())

        campaign_only_resp = auth_client.post(
            f"/api/v1/campaigns/{campaign_id}/templates",
            json={"reply_prompt": "campaign-only reply", "weight": 9},
        )
        assert_response_success(campaign_only_resp)
        campaign_only = extract_data(campaign_only_resp.json())

        delete_resp = auth_client.delete(f"/api/v1/templates/{assignment['id']}")
        assert_response_success(delete_resp)

        db_connection.commit()
        db_cursor.execute(
            "SELECT reply_template_ids FROM gm_campaigns WHERE id = %s",
            (campaign_id,),
        )
        assert db_cursor.fetchone()["reply_template_ids"] == []

        db_cursor.execute(
            """
            SELECT id, library_template_id
            FROM gm_campaign_templates
            WHERE id = %s
            """,
            (campaign_only["id"],),
        )
        row = db_cursor.fetchone()
        assert row is not None
        assert row["library_template_id"] is None

    def test_delete_reusable_template_removes_id_from_all_user_campaigns(
        self, auth_client, api_client, db_cursor, db_connection
    ):
        reusable, _payload = self._create_reusable_template(auth_client)
        campaign_ids = [
            self._create_fresh_campaign(auth_client, api_client, f"{uuid.uuid4().hex[:8]}-{index}")
            for index in range(2)
        ]

        assignment_ids = []
        for campaign_id in campaign_ids:
            assign_resp = auth_client.post(
                f"/api/v1/reply-template-library/{reusable['id']}/assign",
                json={"campaign_id": campaign_id},
            )
            assert_response_success(assign_resp)
            assignment_ids.append(extract_data(assign_resp.json())["id"])

        delete_resp = auth_client.delete(f"/api/v1/reply-template-library/{reusable['id']}")
        assert_response_success(delete_resp)

        db_connection.commit()
        db_cursor.execute(
            """
            SELECT id, reply_template_ids
            FROM gm_campaigns
            WHERE id = ANY(%s)
            ORDER BY id
            """,
            (campaign_ids,),
        )
        assert [row["reply_template_ids"] for row in db_cursor.fetchall()] == [[], []]

        db_cursor.execute(
            """
            SELECT id, library_template_id
            FROM gm_campaign_templates
            WHERE id = ANY(%s)
            ORDER BY id
            """,
            (assignment_ids,),
        )
        legacy_rows = db_cursor.fetchall()
        assert len(legacy_rows) == len(assignment_ids)
        assert all(row["library_template_id"] is None for row in legacy_rows)

    def test_reusable_template_cleared_prompts_resolve_to_null_not_snapshot_fallback(
        self, auth_client, api_client, db_cursor, db_connection
    ):
        campaign_id = self._create_fresh_campaign(auth_client, api_client, uuid.uuid4().hex[:8])
        reusable, payload = self._create_reusable_template(auth_client)

        assign_resp = auth_client.post(
            f"/api/v1/reply-template-library/{reusable['id']}/assign",
            json={"campaign_id": campaign_id},
        )
        assert_response_success(assign_resp)
        assignment = extract_data(assign_resp.json())

        clear_resp = auth_client.put(
            f"/api/v1/reply-template-library/{reusable['id']}",
            json={
                "description": None,
                "dm_prompt": None,
                "reply_prompt": None,
                "reply_post_prompt": None,
            },
        )
        assert_response_success(clear_resp)
        cleared = extract_data(clear_resp.json())
        assert cleared["description"] is None
        assert cleared["dm_prompt"] is None
        assert cleared["reply_prompt"] is None
        assert cleared["reply_post_prompt"] is None

        detail_resp = auth_client.get(f"/api/v1/templates/{assignment['id']}")
        assert_response_success(detail_resp)
        detail = extract_data(detail_resp.json())
        assert detail["reply_prompt"] is None
        assert detail["dm_prompt"] is None
        assert detail["reply_post_prompt"] is None
        assert detail["reusable_template"]["reply_prompt"] is None
        assert detail["reusable_template"]["dm_prompt"] is None
        assert detail["reusable_template"]["reply_post_prompt"] is None

        db_connection.commit()
        db_cursor.execute(
            """
            SELECT
                ct.reply_prompt AS snapshot_reply_prompt,
                ct.dm_prompt AS snapshot_dm_prompt,
                ct.reply_post_prompt AS snapshot_post_prompt,
                resolved.reply_prompt AS resolved_reply_prompt,
                resolved.dm_prompt AS resolved_dm_prompt,
                resolved.reply_post_prompt AS resolved_post_prompt
            FROM gm_campaign_templates ct
            JOIN gm_resolved_campaign_templates resolved ON resolved.id = ct.id
            WHERE ct.id = %s
            """,
            (assignment["id"],),
        )
        row = db_cursor.fetchone()
        assert row["snapshot_reply_prompt"] == payload["reply_prompt"]
        assert row["snapshot_dm_prompt"] == payload["dm_prompt"]
        assert row["snapshot_post_prompt"] == payload["reply_post_prompt"]
        assert row["resolved_reply_prompt"] is None
        assert row["resolved_dm_prompt"] is None
        assert row["resolved_post_prompt"] is None


class TestCampaignPlatformRouting:
    """Cross-platform regression coverage for campaign stats, contents, and export."""

    PLATFORM_CASES = [
        (PLATFORM_REDDIT, "reddit"),
        (PLATFORM_TIKTOK, "tiktok"),
        (PLATFORM_FACEBOOK, "facebook"),
        (PLATFORM_INSTAGRAM, "instagram"),
        (PLATFORM_TWITTER, "twitter"),
    ]

    def _get_auth_user_id(self, auth_client):
        resp = auth_client.get("/api/v1/user/me")
        assert_response_success(resp)
        data = extract_data(resp.json())
        return data.get("id") or data.get("user_id")

    def _get_region_and_model_ids(self, db_cursor, platform_id):
        db_cursor.execute(
            """
            SELECT id
            FROM gm_regions
            WHERE platform_id = %s AND is_active = true
            ORDER BY id
            LIMIT 1
            """,
            (platform_id,),
        )
        region_row = db_cursor.fetchone()
        assert region_row is not None, f"No region for platform_id={platform_id}"

        db_cursor.execute("SELECT id FROM gm_ai_models ORDER BY id LIMIT 1")
        ai_model_row = db_cursor.fetchone()
        assert ai_model_row is not None, "Expected at least one AI model"
        return region_row["id"], ai_model_row["id"]

    def _create_owned_campaign(self, auth_client, db_cursor, platform_id, suffix):
        region_id, ai_model_id = self._get_region_and_model_ids(db_cursor, platform_id)
        resp = auth_client.post(
            "/api/v1/campaigns",
            json={
                "name": f"Cross Platform Routing {platform_id} {suffix}",
                "platform_id": platform_id,
                "region_id": region_id,
                "ai_model_id": ai_model_id,
                "schedule_type": "ONCE",
                "product_prompt": f"Cross platform routing test {suffix}",
            },
        )
        assert_response_success(resp)
        return extract_data(resp.json())["id"]

    def _seed_platform_data(self, db_cursor, db_connection, campaign_id, platform_id, suffix):
        db_cursor.execute(
            """
            INSERT INTO gm_crawler_tasks (
                campaign_id, keywords, max_count, process_count, status, search_offset, search_limit
            )
            VALUES (%s, ARRAY[%s], 50, 0, 'completed', 0, 20)
            RETURNING id
            """,
            (campaign_id, f"routing-{suffix}"),
        )
        task_id = db_cursor.fetchone()["id"]

        if platform_id == PLATFORM_TIKTOK:
            db_cursor.execute(
                """
                INSERT INTO gm_agent_videos (
                    video_id, author, description, task_id, campaign_id,
                    like_count, comment_count, share_count, play_count, publish_time,
                    author_unique_id, url
                )
                VALUES
                    (%s, %s, %s, %s, %s, 10, 3, 1, 100, 1705000000, %s, %s),
                    (%s, %s, %s, %s, %s, 12, 2, 2, 120, 1705000100, %s, %s)
                RETURNING id, video_id
                """,
                (
                    f"tt_video_{suffix}_1",
                    f"tt_author_{suffix}_1",
                    f"TikTok routing test {suffix} 1",
                    task_id,
                    campaign_id,
                    f"tt_author_uid_{suffix}_1",
                    f"https://tiktok.com/@tt_author_{suffix}_1/video/{suffix}1",
                    f"tt_video_{suffix}_2",
                    f"tt_author_{suffix}_2",
                    f"TikTok routing test {suffix} 2",
                    task_id,
                    campaign_id,
                    f"tt_author_uid_{suffix}_2",
                    f"https://tiktok.com/@tt_author_{suffix}_2/video/{suffix}2",
                ),
            )
            videos = db_cursor.fetchall()
            video_one_id, video_one_key = videos[0]["id"], videos[0]["video_id"]
            video_two_id, video_two_key = videos[1]["id"], videos[1]["video_id"]
            comment_ids = [
                f"tt_cmt_{suffix}_1",
                f"tt_cmt_{suffix}_2",
                f"tt_cmt_{suffix}_3",
            ]
            db_cursor.execute(
                """
                INSERT INTO gm_agent_comments (
                    video_db_id, comment_id, user_nickname, user_unique_id, content,
                    reason, suggested_reply, create_time, campaign_id, status, suggested_dm
                )
                VALUES
                    (%s, %s, %s, %s, %s, 'routing', 'reply one', NOW(), %s, 0, NULL),
                    (%s, %s, %s, %s, %s, 'routing', 'reply two', NOW(), %s, 1, NULL),
                    (%s, %s, %s, %s, %s, 'routing', 'reply three', NOW(), %s, 2, NULL)
                """,
                (
                    video_one_id,
                    comment_ids[0],
                    f"tt_user_{suffix}_1",
                    f"tt_uid_{suffix}_1",
                    "TikTok comment one",
                    campaign_id,
                    video_one_id,
                    comment_ids[1],
                    f"tt_user_{suffix}_2",
                    f"tt_uid_{suffix}_2",
                    "TikTok comment two",
                    campaign_id,
                    video_one_id,
                    comment_ids[2],
                    f"tt_user_{suffix}_3",
                    f"tt_uid_{suffix}_3",
                    "TikTok comment three",
                    campaign_id,
                ),
            )
            db_connection.commit()
            return {
                "platform": "tiktok",
                "task_id": task_id,
                "expected_scans": 2,
                "expected_replies": 3,
                "content_ids": [video_one_key, video_two_key],
                "comment_ids": comment_ids,
                "expected_comment_counts": {
                    video_one_key: 3,
                    video_two_key: 2,
                },
                "expected_valid_comment_counts": {
                    video_one_key: 3,
                    video_two_key: 0,
                },
            }

        if platform_id == PLATFORM_FACEBOOK:
            content_ids = [f"fb_post_{suffix}_1", f"fb_post_{suffix}_2"]
            db_cursor.execute(
                """
                INSERT INTO gm_agent_facebook_posts (
                    task_id, campaign_id, facebook_post_id, post_type, url, message, timestamp, posted_at,
                    reactions_count, comments_count, author_name
                )
                VALUES
                    (%s, %s, %s, 'photo', %s, %s, 1705300000, NOW(), 20, 3, %s),
                    (%s, %s, %s, 'video', %s, %s, 1705300100, NOW(), 25, 2, %s)
                RETURNING id
                """,
                (
                    task_id,
                    campaign_id,
                    content_ids[0],
                    f"https://facebook.com/posts/{suffix}1",
                    f"Facebook routing test {suffix} 1",
                    f"fb_author_{suffix}_1",
                    task_id,
                    campaign_id,
                    content_ids[1],
                    f"https://facebook.com/posts/{suffix}2",
                    f"Facebook routing test {suffix} 2",
                    f"fb_author_{suffix}_2",
                ),
            )
            posts = db_cursor.fetchall()
            comment_ids = [f"fb_cmt_{suffix}_1", f"fb_cmt_{suffix}_2", f"fb_cmt_{suffix}_3"]
            db_cursor.execute(
                """
                INSERT INTO gm_agent_facebook_comments (
                    post_db_id, campaign_id, facebook_comment_id, comment_text,
                    reason, suggested_reply, status, comment_username, like_count, reply_count
                )
                VALUES
                    (%s, %s, %s, 'Facebook comment one', 'routing', 'reply one', 0, %s, 5, 1),
                    (%s, %s, %s, 'Facebook comment two', 'routing', 'reply two', 1, %s, 6, 0),
                    (%s, %s, %s, 'Facebook comment three', 'routing', 'reply three', 2, %s, 7, 2)
                """,
                (
                    posts[0]["id"],
                    campaign_id,
                    comment_ids[0],
                    f"fb_user_{suffix}_1",
                    posts[0]["id"],
                    campaign_id,
                    comment_ids[1],
                    f"fb_user_{suffix}_2",
                    posts[0]["id"],
                    campaign_id,
                    comment_ids[2],
                    f"fb_user_{suffix}_3",
                ),
            )
            db_connection.commit()
            return {
                "platform": "facebook",
                "task_id": task_id,
                "expected_scans": 2,
                "expected_replies": 3,
                "content_ids": content_ids,
                "comment_ids": comment_ids,
                "expected_comment_counts": {
                    content_ids[0]: 3,
                    content_ids[1]: 2,
                },
                "expected_valid_comment_counts": {
                    content_ids[0]: 3,
                    content_ids[1]: 0,
                },
            }

        if platform_id == PLATFORM_INSTAGRAM:
            content_ids = [f"IG_ROUTE_{suffix}_1", f"IG_ROUTE_{suffix}_2"]
            db_cursor.execute(
                """
                INSERT INTO gm_agent_instagram_posts (
                    task_id, campaign_id, code, instagram_id, media_type,
                    caption_text, owner_username, like_count, comment_count, posted_at
                )
                VALUES
                    (%s, %s, %s, %s, 1, %s, %s, 30, 2, NOW()),
                    (%s, %s, %s, %s, 2, %s, %s, 40, 1, NOW())
                RETURNING id
                """,
                (
                    task_id,
                    campaign_id,
                    content_ids[0],
                    f"ig_media_{suffix}_1",
                    f"Instagram routing test {suffix} 1",
                    f"ig_owner_{suffix}_1",
                    task_id,
                    campaign_id,
                    content_ids[1],
                    f"ig_media_{suffix}_2",
                    f"Instagram routing test {suffix} 2",
                    f"ig_owner_{suffix}_2",
                ),
            )
            posts = db_cursor.fetchall()
            comment_ids = [f"ig_cmt_{suffix}_1", f"ig_cmt_{suffix}_2", f"ig_cmt_{suffix}_3"]
            db_cursor.execute(
                """
                INSERT INTO gm_agent_instagram_comments (
                    post_db_id, campaign_id, instagram_comment_id, comment_text,
                    reason, suggested_reply, status, comment_username, like_count, child_comment_count
                )
                VALUES
                    (%s, %s, %s, 'Instagram comment one', 'routing', 'reply one', 0, %s, 8, 1),
                    (%s, %s, %s, 'Instagram comment two', 'routing', 'reply two', 1, %s, 9, 0),
                    (%s, %s, %s, 'Instagram comment three', 'routing', 'reply three', 2, %s, 10, 2)
                """,
                (
                    posts[0]["id"],
                    campaign_id,
                    comment_ids[0],
                    f"ig_user_{suffix}_1",
                    posts[0]["id"],
                    campaign_id,
                    comment_ids[1],
                    f"ig_user_{suffix}_2",
                    posts[0]["id"],
                    campaign_id,
                    comment_ids[2],
                    f"ig_user_{suffix}_3",
                ),
            )
            db_connection.commit()
            return {
                "platform": "instagram",
                "task_id": task_id,
                "expected_scans": 2,
                "expected_replies": 3,
                "content_ids": content_ids,
                "comment_ids": comment_ids,
                "expected_comment_counts": {
                    content_ids[0]: 2,
                    content_ids[1]: 1,
                },
                "expected_valid_comment_counts": {
                    content_ids[0]: 3,
                    content_ids[1]: 0,
                },
            }

        if platform_id == PLATFORM_REDDIT:
            content_ids = [f"rd_post_{suffix}_1", f"rd_post_{suffix}_2"]
            db_cursor.execute(
                """
                INSERT INTO gm_agent_reddit_posts (
                    task_id, campaign_id, post_id, post_name, title, selftext,
                    author, subreddit, url, score, num_comments, post_created_at
                )
                VALUES
                    (%s, %s, %s, %s, %s, %s, %s, 'marketing', %s, 50, 2, NOW()),
                    (%s, %s, %s, %s, %s, %s, %s, 'technology', %s, 60, 1, NOW())
                RETURNING id
                """,
                (
                    task_id,
                    campaign_id,
                    content_ids[0],
                    f"t3_{suffix}_1",
                    f"Reddit routing test {suffix} 1",
                    "Reddit body one",
                    f"rd_author_{suffix}_1",
                    f"https://reddit.com/r/marketing/{suffix}1",
                    task_id,
                    campaign_id,
                    content_ids[1],
                    f"t3_{suffix}_2",
                    f"Reddit routing test {suffix} 2",
                    "Reddit body two",
                    f"rd_author_{suffix}_2",
                    f"https://reddit.com/r/technology/{suffix}2",
                ),
            )
            posts = db_cursor.fetchall()
            comment_ids = [f"rd_cmt_{suffix}_1", f"rd_cmt_{suffix}_2", f"rd_cmt_{suffix}_3"]
            db_cursor.execute(
                """
                INSERT INTO gm_agent_reddit_comments (
                    post_db_id, campaign_id, comment_id, comment_name, author, body,
                    reason, suggested_reply, status, score, depth, comment_created_at
                )
                VALUES
                    (%s, %s, %s, %s, %s, 'Reddit comment one', 'routing', 'reply one', 0, 4, 0, NOW()),
                    (%s, %s, %s, %s, %s, 'Reddit comment two', 'routing', 'reply two', 1, 5, 0, NOW()),
                    (%s, %s, %s, %s, %s, 'Reddit comment three', 'routing', 'reply three', 2, 6, 1, NOW())
                """,
                (
                    posts[0]["id"],
                    campaign_id,
                    comment_ids[0],
                    f"t1_{suffix}_1",
                    f"rd_user_{suffix}_1",
                    posts[0]["id"],
                    campaign_id,
                    comment_ids[1],
                    f"t1_{suffix}_2",
                    f"rd_user_{suffix}_2",
                    posts[0]["id"],
                    campaign_id,
                    comment_ids[2],
                    f"t1_{suffix}_3",
                    f"rd_user_{suffix}_3",
                ),
            )
            db_connection.commit()
            return {
                "platform": "reddit",
                "task_id": task_id,
                "expected_scans": 2,
                "expected_replies": 3,
                "content_ids": content_ids,
                "comment_ids": comment_ids,
                "expected_comment_counts": {
                    content_ids[0]: 2,
                    content_ids[1]: 1,
                },
                "expected_valid_comment_counts": {
                    content_ids[0]: 3,
                    content_ids[1]: 0,
                },
            }

        if platform_id == PLATFORM_TWITTER:
            content_ids = [f"tw_post_{suffix}_1", f"tw_post_{suffix}_2"]
            db_cursor.execute(
                """
                INSERT INTO gm_agent_twitter_tweets (
                    task_id, campaign_id, twitter_tweet_id, conversation_id, full_text,
                    screen_name, user_name, favorite_count, retweet_count, reply_count, tweet_created_at
                )
                VALUES
                    (%s, %s, %s, %s, %s, %s, %s, 70, 7, 2, NOW()),
                    (%s, %s, %s, %s, %s, %s, %s, 80, 8, 1, NOW())
                RETURNING id
                """,
                (
                    task_id,
                    campaign_id,
                    content_ids[0],
                    f"conv_{suffix}_1",
                    f"Twitter routing test {suffix} 1",
                    f"tw_screen_{suffix}_1",
                    f"Twitter User {suffix} 1",
                    task_id,
                    campaign_id,
                    content_ids[1],
                    f"conv_{suffix}_2",
                    f"Twitter routing test {suffix} 2",
                    f"tw_screen_{suffix}_2",
                    f"Twitter User {suffix} 2",
                ),
            )
            tweets = db_cursor.fetchall()
            comment_ids = [f"tw_cmt_{suffix}_1", f"tw_cmt_{suffix}_2", f"tw_cmt_{suffix}_3"]
            db_cursor.execute(
                """
                INSERT INTO gm_agent_twitter_comments (
                    tweet_db_id, campaign_id, twitter_comment_id, conversation_id,
                    comment_screen_name, comment_user_name, comment_text, reason,
                    suggested_reply, status, favorite_count, reply_count
                )
                VALUES
                    (%s, %s, %s, %s, %s, %s, 'Twitter comment one', 'routing', 'reply one', 0, 3, 1),
                    (%s, %s, %s, %s, %s, %s, 'Twitter comment two', 'routing', 'reply two', 1, 4, 0),
                    (%s, %s, %s, %s, %s, %s, 'Twitter comment three', 'routing', 'reply three', 2, 5, 2)
                """,
                (
                    tweets[0]["id"],
                    campaign_id,
                    comment_ids[0],
                    f"conv_{suffix}_1",
                    f"tw_commenter_{suffix}_1",
                    f"Twitter Commenter {suffix} 1",
                    tweets[0]["id"],
                    campaign_id,
                    comment_ids[1],
                    f"conv_{suffix}_1",
                    f"tw_commenter_{suffix}_2",
                    f"Twitter Commenter {suffix} 2",
                    tweets[0]["id"],
                    campaign_id,
                    comment_ids[2],
                    f"conv_{suffix}_1",
                    f"tw_commenter_{suffix}_3",
                    f"Twitter Commenter {suffix} 3",
                ),
            )
            db_connection.commit()
            return {
                "platform": "twitter",
                "task_id": task_id,
                "expected_scans": 2,
                "expected_replies": 3,
                "content_ids": content_ids,
                "comment_ids": comment_ids,
                "expected_comment_counts": {
                    content_ids[0]: 2,
                    content_ids[1]: 1,
                },
                "expected_valid_comment_counts": {
                    content_ids[0]: 3,
                    content_ids[1]: 0,
                },
            }

        raise AssertionError(f"Unsupported platform_id for test seeding: {platform_id}")

    @pytest.mark.parametrize("platform_id,platform_name", PLATFORM_CASES)
    def test_campaign_stats_are_platform_aware(
        self, auth_client, db_cursor, db_connection, platform_id, platform_name
    ):
        suffix = uuid.uuid4().hex[:8]
        campaign_id = self._create_owned_campaign(auth_client, db_cursor, platform_id, suffix)
        seed = self._seed_platform_data(db_cursor, db_connection, campaign_id, platform_id, suffix)

        detail_resp = auth_client.get(f"/api/v1/campaigns/{campaign_id}")
        assert_response_success(detail_resp)
        detail = extract_data(detail_resp.json())
        assert detail["stats"]["scans"] == seed["expected_scans"]
        assert detail["stats"]["replies"] == seed["expected_replies"]

        list_resp = auth_client.get("/api/v1/campaigns?page=1&per_page=100")
        assert_response_success(list_resp)
        list_data = extract_data(list_resp.json())
        campaigns = list_data["list"] if isinstance(list_data, dict) else list_data
        matching = next((item for item in campaigns if item["id"] == campaign_id), None)
        assert matching is not None, f"{platform_name} campaign should appear in campaign list"
        assert matching["stats"]["scans"] == seed["expected_scans"]
        assert matching["stats"]["replies"] == seed["expected_replies"]

    @pytest.mark.parametrize("platform_id,platform_name", PLATFORM_CASES)
    def test_campaign_content_routes_include_valid_comment_count(
        self, auth_client, db_cursor, db_connection, platform_id, platform_name
    ):
        suffix = uuid.uuid4().hex[:8]
        campaign_id = self._create_owned_campaign(auth_client, db_cursor, platform_id, suffix)
        seed = self._seed_platform_data(db_cursor, db_connection, campaign_id, platform_id, suffix)

        assert 0 in seed["expected_valid_comment_counts"].values()
        assert any(
            count > 0 for count in seed["expected_valid_comment_counts"].values()
        )

        route_cases = [
            (
                f"/api/v1/campaigns/{campaign_id}/crawler-results",
                f"{platform_name} crawler-results",
            ),
            (
                f"/api/v1/campaigns/{campaign_id}/contents",
                f"{platform_name} contents",
            ),
            (
                f"/api/v1/crawler-tasks/{seed['task_id']}/results",
                f"{platform_name} task results",
            ),
        ]

        for route, label in route_cases:
            resp = auth_client.get(route)
            assert_response_success(resp)
            payload = extract_data(resp.json())
            items = payload["list"] if isinstance(payload, dict) else payload
            assert len(items) == seed["expected_scans"], (
                f"{label} should read {seed['expected_scans']} platform rows"
            )
            if isinstance(payload, dict):
                assert payload["total"] == seed["expected_scans"]
            assert {item["platform"] for item in items} == {seed["platform"]}
            assert {item["content_id"] for item in items} == set(seed["content_ids"])

            items_by_content_id = {item["content_id"]: item for item in items}
            for content_id in seed["content_ids"]:
                item = items_by_content_id[content_id]
                assert item["task_id"] == seed["task_id"]
                assert item["comment_count"] == seed["expected_comment_counts"][content_id]
                assert (
                    item["valid_comment_count"]
                    == seed["expected_valid_comment_counts"][content_id]
                ), f"{label} should expose stored comment counts for {content_id}"

    def test_campaign_export_uses_platform_specific_rows(
        self, auth_client, db_cursor, db_connection
    ):
        suffix = uuid.uuid4().hex[:8]
        campaign_id = self._create_owned_campaign(auth_client, db_cursor, PLATFORM_FACEBOOK, suffix)
        seed = self._seed_platform_data(
            db_cursor, db_connection, campaign_id, PLATFORM_FACEBOOK, suffix
        )

        resp = auth_client.get(f"/api/v1/campaigns/{campaign_id}/export")
        assert_response_success(resp)
        assert "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet" in resp.headers.get(
            "Content-Type", ""
        )

        workbook_xml = ""
        with zipfile.ZipFile(io.BytesIO(resp.content)) as zf:
            for name in zf.namelist():
                if name.endswith(".xml"):
                    workbook_xml += zf.read(name).decode("utf-8", errors="ignore")

        assert seed["content_ids"][0] in workbook_xml
        assert seed["comment_ids"][0] in workbook_xml


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
