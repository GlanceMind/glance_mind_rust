"""
GlanceMind API E2E Tests - Social Account & Group API
=====================================================
Tests for social account and social group endpoints.

Test Coverage:
1. Social accounts CRUD
2. Social groups CRUD
3. Account-group association
4. Account by device queries
"""

import pytest
import uuid
from conftest import (
    assert_response_success,
    extract_data,
    PLATFORM_TIKTOK,
    PLATFORM_REDDIT,
)


class TestSocialGroups:
    """Tests for social group operations."""

    def test_create_social_group(self, auth_client):
        """Test creating a social group."""
        group_payload = {
            "platform_id": PLATFORM_TIKTOK,
            "group_name": f"E2E Test Group {uuid.uuid4().hex[:8]}"
        }
        
        resp = auth_client.post(
            "/api/v1/social-groups",
            json=group_payload
        )
        assert_response_success(resp)
        
        data = extract_data(resp.json())
        print(f"\nCreated group: {data}")
        
        assert "id" in data, "Should return group ID"

    def test_list_social_groups(self, auth_client):
        """Test listing social groups."""
        resp = auth_client.get("/api/v1/social-groups")
        assert_response_success(resp)
        
        data = extract_data(resp.json())
        print(f"\nSocial groups: {type(data)}")
        
        groups = data if isinstance(data, list) else data.get("list", [])
        print(f"  Group count: {len(groups)}")

    def test_get_social_group_by_id(self, auth_client, db_cursor):
        """Test getting a specific social group."""
        # Get existing group from database
        db_cursor.execute("SELECT id FROM gm_social_groups LIMIT 1")
        result = db_cursor.fetchone()
        
        if not result:
            pytest.skip("No social groups in database")
        
        group_id = result["id"]
        
        resp = auth_client.get(f"/api/v1/social-groups/{group_id}")
        
        if resp.status_code == 200:
            data = extract_data(resp.json())
            print(f"\nGroup details: {data}")
            assert data.get("id") == group_id


class TestSocialAccounts:
    """Tests for social account operations."""

    def test_create_social_account(self, auth_client):
        """Test creating a social account."""
        device_id = f"e2e_device_{uuid.uuid4().hex[:8]}"
        
        account_payload = {
            "platform_id": PLATFORM_TIKTOK,
            "username": f"e2e_user_{uuid.uuid4().hex[:8]}",
            "cookie": "test_cookie_data",
            "device_id": device_id,
            "profile_name": "e2e_profile",
            "daily_max_replies": 10
        }
        
        resp = auth_client.post(
            "/api/v1/accounts",
            json=account_payload
        )
        
        if resp.status_code == 200:
            data = extract_data(resp.json())
            print(f"\nCreated account: {data}")
            assert "id" in data
        else:
            print(f"\n  Account creation returned: {resp.status_code}")

    def test_list_social_accounts(self, auth_client):
        """Test listing social accounts."""
        resp = auth_client.get("/api/v1/accounts")
        assert_response_success(resp)
        
        data = extract_data(resp.json())
        print(f"\nSocial accounts: {type(data)}")
        
        accounts = data if isinstance(data, list) else data.get("list", [])
        print(f"  Account count: {len(accounts)}")

    def test_update_social_account(self, auth_client, db_cursor):
        """Test updating a social account."""
        # Get existing account
        db_cursor.execute("SELECT id FROM gm_social_accounts LIMIT 1")
        result = db_cursor.fetchone()
        
        if not result:
            pytest.skip("No social accounts in database")
        
        account_id = result["id"]
        
        update_payload = {
            "daily_max_replies": 20
        }
        
        resp = auth_client.put(
            f"/api/v1/accounts/{account_id}",
            json=update_payload
        )
        
        if resp.status_code == 200:
            data = extract_data(resp.json())
            print(f"\nUpdated account: {data}")


class TestAccountGroupAssociation:
    """Tests for account-group association."""

    def test_assign_account_to_group(self, auth_client, db_cursor):
        """Test assigning an account to a group."""
        # Get existing account and group
        db_cursor.execute("SELECT id FROM gm_social_accounts LIMIT 1")
        account = db_cursor.fetchone()
        
        db_cursor.execute("SELECT id FROM gm_social_groups LIMIT 1")
        group = db_cursor.fetchone()
        
        if not account or not group:
            pytest.skip("Need both account and group for this test")
        
        account_id = account["id"]
        group_id = group["id"]
        
        # Assign to group
        update_payload = {
            "group_id": group_id
        }
        
        resp = auth_client.put(
            f"/api/v1/accounts/{account_id}",
            json=update_payload
        )
        
        if resp.status_code == 200:
            data = extract_data(resp.json())
            print(f"\nAssigned account to group: {data}")
            assert data.get("group_id") == group_id

    def test_unassign_account_from_group(self, auth_client, db_cursor):
        """Test unassigning an account from a group (set group_id to 0)."""
        # Get existing account with a group
        db_cursor.execute("""
            SELECT id FROM gm_social_accounts 
            WHERE group_id IS NOT NULL 
            LIMIT 1
        """)
        result = db_cursor.fetchone()
        
        if not result:
            pytest.skip("No account with group assignment")
        
        account_id = result["id"]
        
        # Unassign (set group_id to 0)
        update_payload = {
            "group_id": 0
        }
        
        resp = auth_client.put(
            f"/api/v1/accounts/{account_id}",
            json=update_payload
        )
        
        if resp.status_code == 200:
            data = extract_data(resp.json())
            print(f"\nUnassigned account from group: {data}")


class TestSocialAccountRoutes:
    """Tests that social account routes exist."""

    def test_accounts_route_requires_auth(self, api_client):
        """Test that accounts route requires authentication."""
        resp = api_client.get("/api/v1/accounts")
        
        assert resp.status_code == 401, \
            f"Expected 401, got {resp.status_code}"

    def test_social_groups_route_requires_auth(self, api_client):
        """Test that social groups route requires authentication."""
        resp = api_client.get("/api/v1/social-groups")
        
        assert resp.status_code == 401, \
            f"Expected 401, got {resp.status_code}"


class TestSocialDatabaseState:
    """Tests that verify social account/group database state."""

    def test_mock_social_groups_exist(self, db_cursor):
        """Verify mock social groups exist in database."""
        print("\n=== Mock Social Groups ===")
        
        db_cursor.execute("""
            SELECT g.id, g.group_name, g.platform_id, p.name as platform_name
            FROM gm_social_groups g
            JOIN gm_platforms p ON g.platform_id = p.id
            ORDER BY g.id
        """)
        groups = db_cursor.fetchall()
        
        for g in groups:
            print(f"  Group {g['id']}: {g['group_name']} ({g['platform_name']})")
        
        assert len(groups) >= 1, "Should have mock social groups"

    def test_mock_social_accounts_exist(self, db_cursor):
        """Verify mock social accounts exist in database."""
        print("\n=== Mock Social Accounts ===")
        
        db_cursor.execute("""
            SELECT a.id, a.username, a.platform_id, a.group_id, a.status
            FROM gm_social_accounts a
            ORDER BY a.id
            LIMIT 5
        """)
        accounts = db_cursor.fetchall()
        
        for a in accounts:
            print(f"  Account {a['id']}: {a['username']} - status={a['status']}, group={a['group_id']}")
        
        assert len(accounts) >= 1, "Should have mock social accounts"


if __name__ == "__main__":
    pytest.main([__file__, "-v", "--tb=short"])
