"""
GlanceMind API E2E Tests - Social Account & Group API
=====================================================
Tests for social account and social group endpoints.

Test Coverage:
1. Social accounts CRUD
2. Social groups CRUD
3. Account-group association
4. Account by device queries
5. Account list filter parameters (username, platform_id, status, device_id)
"""

import pytest
import uuid
from conftest import (
    assert_response_success,
    extract_data,
    PLATFORM_TIKTOK,
    PLATFORM_REDDIT,
    PLATFORM_FACEBOOK,
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


class TestAccountListFilters:
    """Tests for account list filter parameters.

    Covers: username, platform_id, status, device_id filters
    and their combinations with pagination.
    """

    # ------------------------------------------------------------------ #
    # Fixture: create a known set of accounts for the authenticated user  #
    # ------------------------------------------------------------------ #

    @pytest.fixture(autouse=True)
    def setup_filter_accounts(self, auth_client, db_cursor):
        """Create accounts with distinct attributes so filters can be verified."""
        self._created_ids = []

        accounts = [
            {
                "platform_id": PLATFORM_TIKTOK,
                "username": "filter_alice_tk",
                "device_id": "uuid-aaaa-1111",
                "profile_name": "alice_tiktok",
                "daily_max_replies": 10,
            },
            {
                "platform_id": PLATFORM_TIKTOK,
                "username": "filter_bob_tk",
                "device_id": "uuid-bbbb-2222",
                "profile_name": "bob_tiktok",
                "daily_max_replies": 10,
            },
            {
                "platform_id": PLATFORM_REDDIT,
                "username": "filter_alice_rd",
                "device_id": "uuid-aaaa-3333",
                "profile_name": "alice_reddit",
                "daily_max_replies": 10,
            },
            {
                "platform_id": PLATFORM_FACEBOOK,
                "username": "filter_charlie_fb",
                "device_id": "uuid-cccc-4444",
                "profile_name": "charlie_facebook",
                "daily_max_replies": 10,
            },
        ]

        for acc in accounts:
            resp = auth_client.post("/api/v1/accounts", json=acc)
            if resp.status_code == 200:
                data = extract_data(resp.json())
                self._created_ids.append(data["id"])

        assert len(self._created_ids) == 4, (
            f"Expected to create 4 accounts, got {len(self._created_ids)}"
        )

        # Mark one account as UNAVAILABLE so we can filter by status
        unavailable_id = self._created_ids[3]  # filter_charlie_fb
        resp = auth_client.put(
            f"/api/v1/accounts/{unavailable_id}",
            json={"status": "UNAVAILABLE"},
        )
        assert resp.status_code == 200, (
            f"Failed to update account status: {resp.status_code}"
        )

        yield

        # Cleanup: delete created accounts
        for aid in self._created_ids:
            auth_client.delete(f"/api/v1/accounts/{aid}")

    # ------------------------------------------------------------------ #
    # Helper                                                              #
    # ------------------------------------------------------------------ #

    @staticmethod
    def _get_list(resp):
        """Extract list from paginated response."""
        assert_response_success(resp)
        data = extract_data(resp.json())
        return data.get("list", []), data

    # ------------------------------------------------------------------ #
    # 1. Filter by username (partial, case-insensitive)                   #
    # ------------------------------------------------------------------ #

    def test_filter_by_username_partial(self, auth_client):
        """Filtering by partial username should return matching accounts."""
        accounts, _ = self._get_list(
            auth_client.get("/api/v1/accounts", params={"username": "alice"})
        )
        usernames = [a["username"] for a in accounts]
        print(f"\n  username filter 'alice' => {usernames}")
        assert len(accounts) >= 2, "Should match filter_alice_tk and filter_alice_rd"
        assert all("alice" in u for u in usernames)

    def test_filter_by_username_exact(self, auth_client):
        """Filtering by exact username should return that account."""
        accounts, _ = self._get_list(
            auth_client.get("/api/v1/accounts", params={"username": "filter_bob_tk"})
        )
        assert len(accounts) >= 1
        assert accounts[0]["username"] == "filter_bob_tk"

    def test_filter_by_username_case_insensitive(self, auth_client):
        """Username filter should be case-insensitive."""
        accounts, _ = self._get_list(
            auth_client.get("/api/v1/accounts", params={"username": "ALICE"})
        )
        assert len(accounts) >= 2, "Case-insensitive match should find alice accounts"

    def test_filter_by_username_no_match(self, auth_client):
        """Non-existent username should return empty list."""
        accounts, data = self._get_list(
            auth_client.get(
                "/api/v1/accounts",
                params={"username": "nonexistent_xyz_12345"},
            )
        )
        assert len(accounts) == 0
        assert data["total"] == 0

    # ------------------------------------------------------------------ #
    # 2. Filter by platform_id                                            #
    # ------------------------------------------------------------------ #

    def test_filter_by_platform_tiktok(self, auth_client):
        """Filtering by TikTok platform_id should return only TikTok accounts."""
        accounts, _ = self._get_list(
            auth_client.get(
                "/api/v1/accounts", params={"platform_id": PLATFORM_TIKTOK}
            )
        )
        print(f"\n  platform_id={PLATFORM_TIKTOK} => {len(accounts)} accounts")
        assert len(accounts) >= 2, "Should have at least 2 TikTok accounts"
        assert all(a["platform_id"] == PLATFORM_TIKTOK for a in accounts)

    def test_filter_by_platform_reddit(self, auth_client):
        """Filtering by Reddit platform_id should return only Reddit accounts."""
        accounts, _ = self._get_list(
            auth_client.get(
                "/api/v1/accounts", params={"platform_id": PLATFORM_REDDIT}
            )
        )
        assert len(accounts) >= 1
        assert all(a["platform_id"] == PLATFORM_REDDIT for a in accounts)

    def test_filter_by_platform_no_match(self, auth_client):
        """Filtering by a platform with no accounts returns empty."""
        accounts, data = self._get_list(
            auth_client.get(
                "/api/v1/accounts", params={"platform_id": 9999}
            )
        )
        assert len(accounts) == 0
        assert data["total"] == 0

    # ------------------------------------------------------------------ #
    # 3. Filter by status                                                 #
    # ------------------------------------------------------------------ #

    def test_filter_by_status_active(self, auth_client):
        """Filtering by ACTIVE status should only return active accounts."""
        accounts, _ = self._get_list(
            auth_client.get("/api/v1/accounts", params={"status": "ACTIVE"})
        )
        print(f"\n  status=ACTIVE => {len(accounts)} accounts")
        assert len(accounts) >= 3, "Should have at least 3 ACTIVE accounts"
        assert all(a["status"] == "ACTIVE" for a in accounts)

    def test_filter_by_status_unavailable(self, auth_client):
        """Filtering by UNAVAILABLE status should return only unavailable accounts."""
        accounts, _ = self._get_list(
            auth_client.get("/api/v1/accounts", params={"status": "UNAVAILABLE"})
        )
        assert len(accounts) >= 1, "Should have at least 1 UNAVAILABLE account"
        assert all(a["status"] == "UNAVAILABLE" for a in accounts)

    def test_filter_by_status_no_match(self, auth_client):
        """Filtering by a status with no accounts returns empty."""
        accounts, data = self._get_list(
            auth_client.get(
                "/api/v1/accounts", params={"status": "NONEXISTENT_STATUS"}
            )
        )
        assert len(accounts) == 0
        assert data["total"] == 0

    # ------------------------------------------------------------------ #
    # 4. Filter by device_id (partial, case-insensitive)                  #
    # ------------------------------------------------------------------ #

    def test_filter_by_device_id_partial(self, auth_client):
        """Filtering by partial device_id should match accounts."""
        accounts, _ = self._get_list(
            auth_client.get("/api/v1/accounts", params={"device_id": "uuid-aaaa"})
        )
        print(f"\n  device_id filter 'uuid-aaaa' => {len(accounts)} accounts")
        assert len(accounts) >= 2, (
            "Should match uuid-aaaa-1111 and uuid-aaaa-3333"
        )
        assert all("uuid-aaaa" in (a.get("device_id") or "") for a in accounts)

    def test_filter_by_device_id_exact(self, auth_client):
        """Filtering by exact device_id should return that account."""
        accounts, _ = self._get_list(
            auth_client.get(
                "/api/v1/accounts", params={"device_id": "uuid-bbbb-2222"}
            )
        )
        assert len(accounts) >= 1
        assert accounts[0]["device_id"] == "uuid-bbbb-2222"

    def test_filter_by_device_id_case_insensitive(self, auth_client):
        """Device_id filter should be case-insensitive."""
        accounts, _ = self._get_list(
            auth_client.get(
                "/api/v1/accounts", params={"device_id": "UUID-CCCC"}
            )
        )
        assert len(accounts) >= 1, "Case-insensitive match should find cccc account"

    def test_filter_by_device_id_no_match(self, auth_client):
        """Non-existent device_id should return empty list."""
        accounts, data = self._get_list(
            auth_client.get(
                "/api/v1/accounts",
                params={"device_id": "no-such-device-xyz"},
            )
        )
        assert len(accounts) == 0
        assert data["total"] == 0

    # ------------------------------------------------------------------ #
    # 5. Combined filters                                                 #
    # ------------------------------------------------------------------ #

    def test_filter_username_and_platform(self, auth_client):
        """Combining username + platform_id should intersect results."""
        accounts, _ = self._get_list(
            auth_client.get(
                "/api/v1/accounts",
                params={"username": "alice", "platform_id": PLATFORM_TIKTOK},
            )
        )
        assert len(accounts) >= 1
        for a in accounts:
            assert "alice" in a["username"]
            assert a["platform_id"] == PLATFORM_TIKTOK

    def test_filter_username_and_status(self, auth_client):
        """Combining username + status should intersect results."""
        # charlie is UNAVAILABLE
        accounts, _ = self._get_list(
            auth_client.get(
                "/api/v1/accounts",
                params={"username": "charlie", "status": "UNAVAILABLE"},
            )
        )
        assert len(accounts) >= 1
        for a in accounts:
            assert "charlie" in a["username"]
            assert a["status"] == "UNAVAILABLE"

    def test_filter_platform_and_status(self, auth_client):
        """Combining platform_id + status should intersect results."""
        accounts, _ = self._get_list(
            auth_client.get(
                "/api/v1/accounts",
                params={
                    "platform_id": PLATFORM_TIKTOK,
                    "status": "ACTIVE",
                },
            )
        )
        assert len(accounts) >= 2
        for a in accounts:
            assert a["platform_id"] == PLATFORM_TIKTOK
            assert a["status"] == "ACTIVE"

    def test_filter_device_id_and_platform(self, auth_client):
        """Combining device_id + platform_id should intersect results."""
        accounts, _ = self._get_list(
            auth_client.get(
                "/api/v1/accounts",
                params={
                    "device_id": "uuid-aaaa",
                    "platform_id": PLATFORM_REDDIT,
                },
            )
        )
        assert len(accounts) >= 1
        for a in accounts:
            assert "uuid-aaaa" in (a.get("device_id") or "")
            assert a["platform_id"] == PLATFORM_REDDIT

    def test_filter_all_params(self, auth_client):
        """Combining all four filters should narrow results precisely."""
        accounts, _ = self._get_list(
            auth_client.get(
                "/api/v1/accounts",
                params={
                    "username": "alice",
                    "platform_id": PLATFORM_TIKTOK,
                    "status": "ACTIVE",
                    "device_id": "uuid-aaaa-1111",
                },
            )
        )
        assert len(accounts) == 1
        a = accounts[0]
        assert a["username"] == "filter_alice_tk"
        assert a["platform_id"] == PLATFORM_TIKTOK
        assert a["status"] == "ACTIVE"
        assert a["device_id"] == "uuid-aaaa-1111"

    def test_combined_filter_no_match(self, auth_client):
        """Contradictory combined filters should return empty."""
        accounts, data = self._get_list(
            auth_client.get(
                "/api/v1/accounts",
                params={
                    "username": "alice",
                    "platform_id": PLATFORM_FACEBOOK,
                },
            )
        )
        assert len(accounts) == 0
        assert data["total"] == 0

    # ------------------------------------------------------------------ #
    # 6. Pagination with filters                                          #
    # ------------------------------------------------------------------ #

    def test_filter_with_pagination(self, auth_client):
        """Filters should work correctly together with pagination."""
        # Get page 1 with page_size=1 for TikTok accounts
        _, data_p1 = self._get_list(
            auth_client.get(
                "/api/v1/accounts",
                params={
                    "platform_id": PLATFORM_TIKTOK,
                    "page": 1,
                    "page_size": 1,
                },
            )
        )
        assert data_p1["page"] == 1
        assert data_p1["page_size"] == 1
        assert len(data_p1["list"]) == 1
        assert data_p1["total"] >= 2, "Total should reflect all matching records"
        assert data_p1["total_pages"] >= 2

        # Get page 2
        _, data_p2 = self._get_list(
            auth_client.get(
                "/api/v1/accounts",
                params={
                    "platform_id": PLATFORM_TIKTOK,
                    "page": 2,
                    "page_size": 1,
                },
            )
        )
        assert data_p2["page"] == 2
        assert len(data_p2["list"]) == 1

        # Page 1 and page 2 should return different accounts
        assert data_p1["list"][0]["id"] != data_p2["list"][0]["id"]

    def test_filter_pagination_beyond_range(self, auth_client):
        """Requesting a page beyond total should return empty list."""
        accounts, data = self._get_list(
            auth_client.get(
                "/api/v1/accounts",
                params={
                    "platform_id": PLATFORM_TIKTOK,
                    "page": 999,
                    "page_size": 10,
                },
            )
        )
        assert len(accounts) == 0
        assert data["total"] >= 2, "Total should still be accurate"

    # ------------------------------------------------------------------ #
    # 7. No filter returns all (non-DELETED) accounts                     #
    # ------------------------------------------------------------------ #

    def test_no_filter_returns_all(self, auth_client):
        """Without filters, all non-DELETED accounts are returned."""
        accounts, data = self._get_list(
            auth_client.get("/api/v1/accounts", params={"page_size": 100})
        )
        assert data["total"] >= 4, "Should have at least 4 accounts from setup"
        assert all(a["status"] != "DELETED" for a in accounts)

    # ------------------------------------------------------------------ #
    # 8. DB-level verification for filter accuracy                        #
    # ------------------------------------------------------------------ #

    def test_filter_matches_db_count(self, auth_client, db_cursor):
        """API filter result count should match direct DB query."""
        # Get user_id for the authenticated user
        resp = auth_client.get("/api/v1/user/profile")
        if resp.status_code != 200:
            pytest.skip("Cannot get user profile to determine user_id")
        profile = extract_data(resp.json())
        user_id = profile.get("id")

        # DB count: ACTIVE TikTok accounts for this user
        db_cursor.execute(
            """
            SELECT COUNT(*) AS cnt
            FROM gm_social_accounts
            WHERE user_id = %s
              AND platform_id = %s
              AND status = 'ACTIVE'
              AND status != 'DELETED'
            """,
            (user_id, PLATFORM_TIKTOK),
        )
        db_count = db_cursor.fetchone()["cnt"]

        # API count
        _, data = self._get_list(
            auth_client.get(
                "/api/v1/accounts",
                params={
                    "platform_id": PLATFORM_TIKTOK,
                    "status": "ACTIVE",
                    "page_size": 100,
                },
            )
        )
        assert data["total"] == db_count, (
            f"API total ({data['total']}) != DB count ({db_count})"
        )


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
