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


class TestSocialGroupPlatformFilter:
    """M1 tests: platform_id filter on group list + create validation.

    IT3a, IT3b, IT3c — GET /api/v1/social-groups?platform_id=<N>
    IT1, IT2         — POST /api/v1/social-groups with valid/invalid platform_id
    """

    # ------------------------------------------------------------------ #
    # Fixture: seed 3 groups owned by the auth user and clean up after   #
    # ------------------------------------------------------------------ #

    @pytest.fixture(autouse=True)
    def setup_groups(self, auth_client, db_cursor):
        """Seed 2 reddit (platform_id=1) + 1 facebook (platform_id=3) groups
        with unique names so tests are isolated.  Clean up on teardown."""
        suffix = uuid.uuid4().hex[:8]
        self._reddit1_name = f"m1_reddit_a_{suffix}"
        self._reddit2_name = f"m1_reddit_b_{suffix}"
        self._facebook_name = f"m1_facebook_{suffix}"
        self._group_ids = []

        for name, pid in [
            (self._reddit1_name, PLATFORM_REDDIT),
            (self._reddit2_name, PLATFORM_REDDIT),
            (self._facebook_name, PLATFORM_FACEBOOK),
        ]:
            resp = auth_client.post(
                "/api/v1/social-groups",
                json={"platform_id": pid, "group_name": name},
            )
            assert resp.status_code == 200, (
                f"Seed group creation failed: {resp.status_code} {resp.text}"
            )
            group_id = resp.json()["data"]["id"]
            self._group_ids.append(group_id)

        assert len(self._group_ids) == 3, (
            f"Expected 3 seeded groups, got {len(self._group_ids)}"
        )

        yield

        # Teardown: delete the groups we created (best-effort)
        for gid in self._group_ids:
            auth_client.delete(f"/api/v1/social-groups/{gid}")

    # ------------------------------------------------------------------ #
    # Helper                                                              #
    # ------------------------------------------------------------------ #

    @staticmethod
    def _extract_list_data(resp):
        assert_response_success(resp)
        data = extract_data(resp.json())
        return data.get("list", []), data

    # ------------------------------------------------------------------ #
    # IT3b — GREEN today (regression lock): no filter returns all seeded  #
    # ------------------------------------------------------------------ #

    def test_it3b_no_filter_returns_all_seeded_groups(self, auth_client):
        """IT3b: GET without platform_id returns all seeded groups and response items have the required fields."""
        # Paginate over ALL pages: other e2e suites accumulate groups for this
        # user across runs (>100 by now), so a single fixed-size page can miss
        # the groups seeded by this fixture.
        groups = []
        page = 1
        while True:
            page_groups, data = self._extract_list_data(
                auth_client.get(
                    "/api/v1/social-groups",
                    params={"page": page, "page_size": 100},
                )
            )
            if not page_groups:
                break
            groups.extend(page_groups)
            page += 1
        # At least 3 groups we just created
        assert data["total"] >= 3, (
            f"IT3b: expected total >= 3, got {data['total']}"
        )

        # Shape lock: verify all required fields are present
        for g in groups:
            assert "platform_id" in g, "IT3b: missing field platform_id"
            assert "group_name" in g, "IT3b: missing field group_name"
            assert "account_count" in g, "IT3b: missing field account_count"

        # Verify our seeded groups are present by name
        names = {g["group_name"] for g in groups}
        assert self._reddit1_name in names, (
            f"IT3b: seeded reddit group '{self._reddit1_name}' not in list"
        )
        assert self._reddit2_name in names, (
            f"IT3b: seeded reddit group '{self._reddit2_name}' not in list"
        )
        assert self._facebook_name in names, (
            f"IT3b: seeded facebook group '{self._facebook_name}' not in list"
        )
        print(
            f"\n  IT3b GREEN: total={data['total']}, "
            f"reddit+facebook groups present, shape OK"
        )

    # ------------------------------------------------------------------ #
    # IT3a — RED today: filter by platform_id=3 should return 1 group    #
    # ------------------------------------------------------------------ #

    def test_it3a_filter_by_platform_facebook(self, auth_client):
        """IT3a: GET ?platform_id=3 returns only facebook groups; run-seeded names confirm correct filtering."""
        groups, data = self._extract_list_data(
            auth_client.get(
                "/api/v1/social-groups",
                params={"platform_id": PLATFORM_FACEBOOK, "page_size": 100},
            )
        )
        names = {g["group_name"] for g in groups}
        assert self._facebook_name in names, (
            f"IT3a: seeded facebook group '{self._facebook_name}' not returned by platform_id=3 filter"
        )
        assert self._reddit1_name not in names, (
            f"IT3a: reddit group '{self._reddit1_name}' leaked into platform_id=3 results"
        )
        assert self._reddit2_name not in names, (
            f"IT3a: reddit group '{self._reddit2_name}' leaked into platform_id=3 results"
        )
        assert all(g["platform_id"] == PLATFORM_FACEBOOK for g in groups), (
            f"IT3a: some returned groups have platform_id != {PLATFORM_FACEBOOK}"
        )
        print(f"\n  IT3a: len={len(groups)} total={data['total']}")

    # ------------------------------------------------------------------ #
    # IT3c — RED today: edge cases for platform_id values                 #
    # ------------------------------------------------------------------ #

    def test_it3c_filter_platform_id_zero_returns_empty(self, auth_client):
        """IT3c: ?platform_id=0 should return 200 + empty list (no valid platform has id=0)."""
        groups, data = self._extract_list_data(
            auth_client.get(
                "/api/v1/social-groups",
                params={"platform_id": 0, "page_size": 100},
            )
        )
        assert len(groups) == 0, (
            f"IT3c(0): expected 0 groups for platform_id=0, got {len(groups)} — "
            f"backend probably ignores invalid platform_id"
        )
        assert data["total"] == 0, (
            f"IT3c(0): expected total==0, got {data['total']}"
        )

    def test_it3c_filter_platform_id_negative_returns_empty(self, auth_client):
        """IT3c: ?platform_id=-1 should return 200 + empty list (no valid platform has id<0)."""
        groups, data = self._extract_list_data(
            auth_client.get(
                "/api/v1/social-groups",
                params={"platform_id": -1, "page_size": 100},
            )
        )
        assert len(groups) == 0, (
            f"IT3c(-1): expected 0 groups for platform_id=-1, got {len(groups)} — "
            f"backend probably ignores invalid platform_id"
        )
        assert data["total"] == 0, (
            f"IT3c(-1): expected total==0, got {data['total']}"
        )

    def test_it3c_filter_platform_id_alpha_returns_400(self, auth_client):
        """IT3c: ?platform_id=abc should return HTTP 400 (non-integer must be rejected).

        NOTE: the 400 body may be a plain-text Axum rejection, not JSON.
        """
        resp = auth_client.get(
            "/api/v1/social-groups",
            params={"platform_id": "abc"},
        )
        assert resp.status_code == 400, (
            f"IT3c(abc): expected HTTP 400, got {resp.status_code} — "
            f"platform_id='abc' should be rejected as invalid integer"
        )

    # ------------------------------------------------------------------ #
    # IT1 — RED today: create with invalid platform_id should be 400     #
    # ------------------------------------------------------------------ #

    def test_it1_create_group_invalid_platform_id_rejected(
        self, auth_client, db_cursor
    ):
        """IT1: POST with platform_id=999 must return HTTP 400 with a message containing "invalid platform_id", and NO row should be created."""
        group_name = f"m1_invalid_platform_{uuid.uuid4().hex[:8]}"
        resp = auth_client.post(
            "/api/v1/social-groups",
            json={"platform_id": 999, "group_name": group_name},
        )

        # Assert: must be 400
        assert resp.status_code == 400, (
            f"IT1: expected HTTP 400 for platform_id=999, got {resp.status_code} — "
            f"body: {resp.text[:200]}"
        )

        # Assert: error message must mention "Invalid platform_id"
        body = resp.json()
        error_msg = (body.get("msg") or "").lower() + (body.get("msg_cn") or "").lower()
        assert "invalid platform_id" in error_msg, (
            f"IT1: expected error message to contain 'Invalid platform_id', got: {resp.text[:200]}"
        )

        # Assert: no row created in the DB
        db_cursor.execute(
            "SELECT id FROM gm_social_groups WHERE group_name = %s",
            (group_name,),
        )
        row = db_cursor.fetchone()
        assert row is None, (
            f"IT1: row with group_name='{group_name}' was created despite invalid platform_id"
        )
        print(f"\n  IT1: HTTP {resp.status_code}, no row in DB — good")

    # ------------------------------------------------------------------ #
    # IT2 — GREEN today (baseline): create with valid platform_id=3      #
    # ------------------------------------------------------------------ #

    def test_it2_create_group_valid_platform_id(self, auth_client, db_cursor):
        """IT2: POST with platform_id=3 (Facebook) must return HTTP 200 + code==1000 and persist the row with platform_id==3."""
        group_name = f"m1_fb_group_it2_{uuid.uuid4().hex[:8]}"
        resp = auth_client.post(
            "/api/v1/social-groups",
            json={"platform_id": PLATFORM_FACEBOOK, "group_name": group_name},
        )

        group_id = None
        try:
            assert resp.status_code == 200, (
                f"IT2: expected HTTP 200, got {resp.status_code} — body: {resp.text[:200]}"
            )
            body = resp.json()
            assert body.get("code") == 1000, (
                f"IT2: expected code==1000, got {body.get('code')}"
            )

            created = body.get("data", {})
            group_id = created.get("id")
            assert group_id is not None, "IT2: response missing 'id' in data"

            # DB verification
            db_cursor.execute(
                "SELECT id, platform_id FROM gm_social_groups WHERE id = %s",
                (group_id,),
            )
            row = db_cursor.fetchone()
            assert row is not None, (
                f"IT2: no row found in DB for group id={group_id}"
            )
            assert row["platform_id"] == PLATFORM_FACEBOOK, (
                f"IT2: DB row has platform_id={row['platform_id']}, expected {PLATFORM_FACEBOOK}"
            )

            print(
                f"\n  IT2 GREEN: HTTP 200, code=1000, "
                f"DB row id={group_id} platform_id={row['platform_id']}"
            )
        finally:
            # Cleanup: always delete the created group if it was created
            if group_id is not None:
                auth_client.delete(f"/api/v1/social-groups/{group_id}")


class TestAccountGroupBinding:
    """IT4: M2 binding-invariant validation tests.

    Exercises PUT /api/v1/accounts/{id} group_id binding and
    POST /api/v1/accounts/batch with group_id + platform_id cross-check.

    Batch endpoint: POST /api/v1/accounts/batch  (BatchCreateAccountsDto)
    Fields: platform_id (int), username, device_id?, profile_start, profile_end,
            daily_max_replies (default 50), group_id? (int)

    Current state (before M2 implementation):
      IT4a, IT4b, IT4c, IT4f → RED (returns 200 instead of 4xx)
      IT4d, IT4e, IT4g        → GREEN baseline
    """

    # ---------------------------------------------------------------------- #
    # Fixture: seed users A resources                                         #
    # ---------------------------------------------------------------------- #

    @pytest.fixture(autouse=True)
    def setup_resources(self, auth_client, db_cursor, db_connection):
        """Seed resources for user A (auth_client) and user B (direct-insert).

        User A:
          - gF: facebook group (platform_id=3)
          - gR: reddit group   (platform_id=1)
          - a1: facebook account (platform_id=3), initially unbound

        User B (direct DB insert — no API token):
          - gB: facebook group (platform_id=3)
        """
        suffix = uuid.uuid4().hex[:8]
        self._created_account_ids = []
        self._created_group_ids = []
        self._batch_prefix = f"it4batch_{suffix}"

        # Resolve user A's id from DB (profile endpoint is POST-only)
        from conftest import resolve_test_user_id
        self._user_a_id = resolve_test_user_id(db_connection)

        # Create facebook group gF for user A
        resp = auth_client.post(
            "/api/v1/social-groups",
            json={"platform_id": PLATFORM_FACEBOOK, "group_name": f"it4_gF_{suffix}"},
        )
        assert resp.status_code == 200, f"Failed to create gF: {resp.text}"
        self._gF_id = resp.json()["data"]["id"]
        self._created_group_ids.append(self._gF_id)

        # Create reddit group gR for user A
        resp = auth_client.post(
            "/api/v1/social-groups",
            json={"platform_id": PLATFORM_REDDIT, "group_name": f"it4_gR_{suffix}"},
        )
        assert resp.status_code == 200, f"Failed to create gR: {resp.text}"
        self._gR_id = resp.json()["data"]["id"]
        self._created_group_ids.append(self._gR_id)

        # Create facebook account a1 for user A (unbound initially)
        resp = auth_client.post(
            "/api/v1/accounts",
            json={
                "platform_id": PLATFORM_FACEBOOK,
                "username": f"it4_a1_{suffix}",
                "profile_name": f"it4_a1_{suffix}",
                "daily_max_replies": 10,
            },
        )
        assert resp.status_code == 200, f"Failed to create a1: {resp.text}"
        self._a1_id = resp.json()["data"]["id"]
        self._created_account_ids.append(self._a1_id)

        # Create user B (direct DB insert)
        user_b_email = f"it4_userB_{suffix}@test.invalid"
        db_cursor.execute(
            """
            INSERT INTO gm_users (email, username, password_hash, permissions)
            VALUES (%s, %s, 'no-login', 0)
            RETURNING id
            """,
            (user_b_email, f"it4_userB_{suffix}"),
        )
        db_cursor.connection.commit()
        self._user_b_id = db_cursor.fetchone()["id"]

        # Create gB: facebook group owned by user B (direct DB insert)
        db_cursor.execute(
            """
            INSERT INTO gm_social_groups (user_id, platform_id, group_name)
            VALUES (%s, %s, %s)
            RETURNING id
            """,
            (self._user_b_id, PLATFORM_FACEBOOK, f"it4_gB_{suffix}"),
        )
        db_cursor.connection.commit()
        self._gB_id = db_cursor.fetchone()["id"]

        yield

        # ------------------------------------------------------------------ #
        # Teardown: delete created accounts (batch + single) and groups       #
        # ------------------------------------------------------------------ #
        # Delete batch-created accounts by prefix
        try:
            db_cursor.execute(
                """
                DELETE FROM gm_social_accounts
                WHERE user_id = %s AND username LIKE %s
                """,
                (self._user_a_id, f"{self._batch_prefix}%"),
            )
            db_cursor.connection.commit()
        except Exception:
            db_cursor.connection.rollback()

        # Delete individually tracked accounts
        for aid in self._created_account_ids:
            try:
                auth_client.delete(f"/api/v1/accounts/{aid}")
            except Exception:
                pass

        # Delete groups owned by user A
        for gid in self._created_group_ids:
            try:
                auth_client.delete(f"/api/v1/social-groups/{gid}")
            except Exception:
                pass

        # Delete user B's group and user B
        try:
            db_cursor.execute(
                "DELETE FROM gm_social_groups WHERE id = %s", (self._gB_id,)
            )
            db_cursor.execute(
                "DELETE FROM gm_users WHERE id = %s", (self._user_b_id,)
            )
            db_cursor.connection.commit()
        except Exception:
            db_cursor.connection.rollback()

    # ---------------------------------------------------------------------- #
    # IT4a: bind to non-existent group → 404                                 #
    # Current: RED (returns 200)                                              #
    # ---------------------------------------------------------------------- #

    def test_it4a_bind_nonexistent_group_returns_404(self, auth_client):
        """IT4a: PUT account/{a1} {"group_id": 99999999} must return 404.
        RED today: API returns 200 (no validation)."""
        resp = auth_client.put(
            f"/api/v1/accounts/{self._a1_id}",
            json={"group_id": 99999999},
        )
        assert resp.status_code == 404, (
            f"IT4a: expected HTTP 404 for non-existent group_id, "
            f"got {resp.status_code} — body: {resp.text[:300]}"
        )

    # ---------------------------------------------------------------------- #
    # IT4b: bind user B's group → 404                                        #
    # Current: RED (returns 200)                                              #
    # ---------------------------------------------------------------------- #

    def test_it4b_bind_other_users_group_returns_404(self, auth_client):
        """IT4b: PUT account/{a1} {"group_id": gB} (user B's group) must return 404.
        RED today: API returns 200 (no ownership check)."""
        resp = auth_client.put(
            f"/api/v1/accounts/{self._a1_id}",
            json={"group_id": self._gB_id},
        )
        assert resp.status_code == 404, (
            f"IT4b: expected HTTP 404 for another user's group, "
            f"got {resp.status_code} — body: {resp.text[:300]}"
        )

    # ---------------------------------------------------------------------- #
    # IT4c: bind platform-mismatched group → 400 + code 2001 + error msg     #
    # Current: RED (returns 200)                                              #
    # ---------------------------------------------------------------------- #

    def test_it4c_bind_platform_mismatch_returns_400(self, auth_client):
        """IT4c: PUT account/{a1 (facebook)} {"group_id": gR (reddit)} must return 400
        with code==2001 and error message containing 'does not match required platform'.
        RED today: API returns 200."""
        resp = auth_client.put(
            f"/api/v1/accounts/{self._a1_id}",
            json={"group_id": self._gR_id},
        )
        assert resp.status_code == 400, (
            f"IT4c: expected HTTP 400 for platform mismatch (facebook acc → reddit group), "
            f"got {resp.status_code} — body: {resp.text[:300]}"
        )
        body = resp.json()
        assert body.get("code") == 2001, (
            f"IT4c: expected code==2001 (BadRequest), got {body.get('code')}"
        )
        combined_msg = (body.get("msg") or "") + " " + (body.get("msg_cn") or "")
        assert "does not match required platform" in combined_msg, (
            f"IT4c: expected 'does not match required platform' in error message, "
            f"got: {combined_msg!r}"
        )

    # ---------------------------------------------------------------------- #
    # IT4d: bind valid same-platform group → 200 + DB reflects new group_id  #
    # GREEN baseline (currently works)                                        #
    # ---------------------------------------------------------------------- #

    def test_it4d_bind_valid_same_platform_group(self, auth_client, db_cursor):
        """IT4d: PUT account/{a1 (facebook)} {"group_id": gF (facebook)} must return 200
        and DB must reflect group_id==gF.  GREEN baseline today."""
        # Ensure a1 is unbound first
        auth_client.put(f"/api/v1/accounts/{self._a1_id}", json={"group_id": 0})

        resp = auth_client.put(
            f"/api/v1/accounts/{self._a1_id}",
            json={"group_id": self._gF_id},
        )
        assert resp.status_code == 200, (
            f"IT4d: expected HTTP 200 for valid same-platform bind, "
            f"got {resp.status_code} — body: {resp.text[:300]}"
        )

        # DB verification
        db_cursor.execute(
            "SELECT group_id FROM gm_social_accounts WHERE id = %s",
            (self._a1_id,),
        )
        row = db_cursor.fetchone()
        assert row is not None, f"IT4d: account {self._a1_id} not found in DB"
        assert row["group_id"] == self._gF_id, (
            f"IT4d: expected group_id=={self._gF_id} in DB, got {row['group_id']}"
        )
        print(f"\n  IT4d GREEN: account {self._a1_id} bound to group {self._gF_id}")

    # ---------------------------------------------------------------------- #
    # IT4e: unbind (group_id=0) → 200 + DB group_id IS NULL                 #
    # GREEN baseline                                                          #
    # ---------------------------------------------------------------------- #

    def test_it4e_unbind_sets_group_id_null(self, auth_client, db_cursor):
        """IT4e: PUT {"group_id": 0} must return 200 and set group_id to NULL in DB.
        GREEN baseline today (unbind semantics already implemented)."""
        # First bind to gF to ensure it is bound
        auth_client.put(
            f"/api/v1/accounts/{self._a1_id}",
            json={"group_id": self._gF_id},
        )

        resp = auth_client.put(
            f"/api/v1/accounts/{self._a1_id}",
            json={"group_id": 0},
        )
        assert resp.status_code == 200, (
            f"IT4e: expected HTTP 200 for unbind, "
            f"got {resp.status_code} — body: {resp.text[:300]}"
        )

        db_cursor.execute(
            "SELECT group_id FROM gm_social_accounts WHERE id = %s",
            (self._a1_id,),
        )
        row = db_cursor.fetchone()
        assert row is not None, f"IT4e: account {self._a1_id} not found in DB"
        assert row["group_id"] is None, (
            f"IT4e: expected group_id IS NULL after unbind, got {row['group_id']}"
        )
        print(f"\n  IT4e GREEN: account {self._a1_id} unbound (group_id IS NULL)")

    # ---------------------------------------------------------------------- #
    # IT4f: batch-create with platform_id=3 + group_id=gR (reddit) → 400    #
    # + ZERO accounts created                                                 #
    # Current: RED (returns 200 and creates accounts)                        #
    # ---------------------------------------------------------------------- #

    def test_it4f_batch_create_platform_mismatch_returns_400(self, auth_client, db_cursor):
        """IT4f: POST /api/v1/accounts/batch with platform_id=3 (facebook) and
        group_id=gR (reddit) must return 400 AND create ZERO accounts.
        RED today: succeeds and creates accounts."""
        prefix = self._batch_prefix + "_f_"
        payload = {
            "platform_id": PLATFORM_FACEBOOK,
            "username": prefix + "user",
            "profile_start": prefix + "p001",
            "profile_end": prefix + "p005",
            "daily_max_replies": 10,
            "group_id": self._gR_id,
        }
        resp = auth_client.post("/api/v1/accounts/batch", json=payload)
        assert resp.status_code == 400, (
            f"IT4f: expected HTTP 400 for batch with platform mismatch "
            f"(facebook accounts → reddit group), "
            f"got {resp.status_code} — body: {resp.text[:300]}"
        )

        # Verify zero accounts were created
        db_cursor.execute(
            "SELECT COUNT(*) AS cnt FROM gm_social_accounts WHERE user_id = %s AND username LIKE %s",
            (self._user_a_id, f"{prefix}%"),
        )
        count = db_cursor.fetchone()["cnt"]
        assert count == 0, (
            f"IT4f: expected 0 accounts created after 400, but found {count}"
        )

    # ---------------------------------------------------------------------- #
    # IT4g: batch-create with group_id=gF (same platform) → success          #
    # + every created row has group_id==gF                                   #
    # GREEN baseline candidate                                                #
    # ---------------------------------------------------------------------- #

    def test_it4g_batch_create_valid_group_succeeds(self, auth_client, db_cursor):
        """IT4g: POST /api/v1/accounts/batch with platform_id=3 and group_id=gF (facebook)
        must succeed and every created account must have group_id==gF.
        GREEN baseline today (no platform check, batch just assigns group)."""
        prefix = self._batch_prefix + "_g_"
        payload = {
            "platform_id": PLATFORM_FACEBOOK,
            "username": prefix + "user",
            "profile_start": prefix + "p001",
            "profile_end": prefix + "p003",
            "daily_max_replies": 10,
            "group_id": self._gF_id,
        }
        resp = auth_client.post("/api/v1/accounts/batch", json=payload)
        assert resp.status_code == 200, (
            f"IT4g: expected HTTP 200 for valid batch with same-platform group, "
            f"got {resp.status_code} — body: {resp.text[:300]}"
        )

        body = resp.json()
        result = body.get("data", {})
        created_ids = result.get("created_ids", [])
        assert len(created_ids) > 0, "IT4g: expected at least 1 account to be created"

        # DB verification: every created account has group_id==gF
        for acc_id in created_ids:
            db_cursor.execute(
                "SELECT group_id FROM gm_social_accounts WHERE id = %s",
                (acc_id,),
            )
            row = db_cursor.fetchone()
            assert row is not None, f"IT4g: created account id={acc_id} not found in DB"
            assert row["group_id"] == self._gF_id, (
                f"IT4g: account {acc_id} has group_id={row['group_id']}, "
                f"expected {self._gF_id}"
            )

        print(
            f"\n  IT4g GREEN: batch created {len(created_ids)} accounts, "
            f"all bound to gF={self._gF_id}"
        )


if __name__ == "__main__":
    pytest.main([__file__, "-v", "--tb=short"])
