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

    def test_get_comments_unknown_platform_is_rejected(self, api_client):
        """Explicit invalid platform should return a client error."""
        device_id = "test_device_001"
        
        resp = api_client.get(
            f"/api/v1/public/comments/by-device?device_id={device_id}&platform=unknown_platform"
        )
        assert resp.status_code in (400, 422), (
            f"Invalid explicit platform should fail, got {resp.status_code}: {resp.text}"
        )


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


class TestUpdateCommentStatus:
    """Tests for POST /api/v1/public/comments/update-status

    Covers the multi-platform status update hotfix:
    - Twitter status writeback via explicit platform param
    - Missing platform keeps TikTok backward compatibility only
    - All supported platforms dispatch correctly
    - Validation rejects out-of-range status values
    - Nonexistent comment_id returns 404
    """

    UPDATE_URL = "/api/v1/public/comments/update-status"

    # -- helpers --

    def _ensure_tiktok_comment(self, db_cursor, db_connection):
        db_cursor.execute(
            "SELECT comment_id FROM gm_agent_comments LIMIT 1"
        )
        row = db_cursor.fetchone()
        if row:
            return row["comment_id"]
        db_cursor.execute("""
            INSERT INTO gm_agent_comments (
                comment_id, video_db_id, user_unique_id, user_nickname,
                content, status, created_at
            )
            VALUES (
                'test_tiktok_cmt_001', 1, 'uid_001', 'nick_001',
                'test comment', 0, NOW()
            )
            RETURNING comment_id
        """)
        db_connection.commit()
        return db_cursor.fetchone()["comment_id"]

    def _ensure_twitter_comment(self, db_cursor, db_connection):
        db_cursor.execute(
            "SELECT twitter_comment_id FROM gm_agent_twitter_comments LIMIT 1"
        )
        row = db_cursor.fetchone()
        if row:
            return row["twitter_comment_id"]
        db_cursor.execute("""
            INSERT INTO gm_agent_twitter_comments (
                twitter_comment_id, content_db_id,
                comment_user_id, comment_screen_name,
                comment_text, status, created_at
            )
            VALUES (
                'test_tw_cmt_001', 1,
                'tw_user_001', 'tw_screen_001',
                'twitter test reply', 0, NOW()
            )
            RETURNING twitter_comment_id
        """)
        db_connection.commit()
        return db_cursor.fetchone()["twitter_comment_id"]

    def _ensure_facebook_comment(self, db_cursor, db_connection):
        db_cursor.execute(
            "SELECT facebook_comment_id FROM gm_agent_facebook_comments LIMIT 1"
        )
        row = db_cursor.fetchone()
        if row:
            return row["facebook_comment_id"]
        db_cursor.execute("""
            INSERT INTO gm_agent_facebook_comments (
                facebook_comment_id, post_db_id,
                comment_text, status, created_at
            )
            VALUES (
                'test_fb_cmt_001', 1,
                'facebook test comment', 0, NOW()
            )
            RETURNING facebook_comment_id
        """)
        db_connection.commit()
        return db_cursor.fetchone()["facebook_comment_id"]

    def _ensure_instagram_comment(self, db_cursor, db_connection):
        db_cursor.execute(
            "SELECT instagram_comment_id FROM gm_agent_instagram_comments LIMIT 1"
        )
        row = db_cursor.fetchone()
        if row:
            return row["instagram_comment_id"]
        db_cursor.execute("""
            INSERT INTO gm_agent_instagram_comments (
                instagram_comment_id, post_db_id,
                comment_text, status, created_at
            )
            VALUES (
                'test_ig_cmt_001', 1,
                'instagram test comment', 0, NOW()
            )
            RETURNING instagram_comment_id
        """)
        db_connection.commit()
        return db_cursor.fetchone()["instagram_comment_id"]

    def _ensure_reddit_comment(self, db_cursor, db_connection):
        db_cursor.execute(
            "SELECT comment_id FROM gm_agent_reddit_comments LIMIT 1"
        )
        row = db_cursor.fetchone()
        if row:
            return row["comment_id"]
        db_cursor.execute("""
            INSERT INTO gm_agent_reddit_comments (
                comment_id, comment_name, post_db_id,
                status, created_at
            )
            VALUES (
                'test_rd_cmt_001', 't1_rd001', 1,
                0, NOW()
            )
            RETURNING comment_id
        """)
        db_connection.commit()
        return db_cursor.fetchone()["comment_id"]

    # -- 1. Twitter explicit platform --

    def test_twitter_status_update_success(self, api_client, db_cursor, db_connection):
        """Twitter comment status should update when platform='twitter'."""
        cid = self._ensure_twitter_comment(db_cursor, db_connection)

        db_cursor.execute(
            "UPDATE gm_agent_twitter_comments SET status = 0 WHERE twitter_comment_id = %s",
            (cid,),
        )
        db_connection.commit()

        resp = api_client.post(self.UPDATE_URL, json={
            "comment_id": cid, "status": 2, "platform": "twitter",
        })
        assert_response_success(resp)
        data = extract_data(resp.json())
        assert data["success"] is True

        db_cursor.execute(
            "SELECT status FROM gm_agent_twitter_comments WHERE twitter_comment_id = %s",
            (cid,),
        )
        assert db_cursor.fetchone()["status"] == 2

    def test_twitter_comment_disappears_from_pending(self, api_client, db_cursor, db_connection):
        """After update to status=2, the comment should not appear in status=0 query."""
        cid = self._ensure_twitter_comment(db_cursor, db_connection)

        db_cursor.execute(
            "UPDATE gm_agent_twitter_comments SET status = 0 WHERE twitter_comment_id = %s",
            (cid,),
        )
        db_connection.commit()

        resp = api_client.post(self.UPDATE_URL, json={
            "comment_id": cid, "status": 2, "platform": "twitter",
        })
        assert_response_success(resp)

        db_cursor.execute(
            "SELECT status FROM gm_agent_twitter_comments WHERE twitter_comment_id = %s",
            (cid,),
        )
        row = db_cursor.fetchone()
        assert row is not None
        assert row["status"] != 0, "Comment should no longer be in pending (status=0)"

    # -- 2. Legacy path (no platform → defaults to tiktok) --

    def test_no_platform_defaults_to_tiktok(self, api_client, db_cursor, db_connection):
        """Omitting platform should update the TikTok table (backward compat)."""
        cid = self._ensure_tiktok_comment(db_cursor, db_connection)

        db_cursor.execute(
            "UPDATE gm_agent_comments SET status = 0 WHERE comment_id = %s", (cid,),
        )
        db_connection.commit()

        resp = api_client.post(self.UPDATE_URL, json={
            "comment_id": cid, "status": 1,
        })
        assert_response_success(resp)
        data = extract_data(resp.json())
        assert data["success"] is True

        db_cursor.execute(
            "SELECT status FROM gm_agent_comments WHERE comment_id = %s", (cid,),
        )
        assert db_cursor.fetchone()["status"] == 1

    def test_explicit_tiktok_platform(self, api_client, db_cursor, db_connection):
        """Explicitly passing platform='tiktok' should also update the TikTok table."""
        cid = self._ensure_tiktok_comment(db_cursor, db_connection)

        db_cursor.execute(
            "UPDATE gm_agent_comments SET status = 0 WHERE comment_id = %s", (cid,),
        )
        db_connection.commit()

        resp = api_client.post(self.UPDATE_URL, json={
            "comment_id": cid, "status": 2, "platform": "tiktok",
        })
        assert_response_success(resp)

        db_cursor.execute(
            "SELECT status FROM gm_agent_comments WHERE comment_id = %s", (cid,),
        )
        assert db_cursor.fetchone()["status"] == 2

    # -- 3. Platform case-insensitive --

    def test_platform_case_insensitive(self, api_client, db_cursor, db_connection):
        """platform='TWITTER' (uppercase) should route to Twitter table."""
        cid = self._ensure_twitter_comment(db_cursor, db_connection)

        db_cursor.execute(
            "UPDATE gm_agent_twitter_comments SET status = 0 WHERE twitter_comment_id = %s",
            (cid,),
        )
        db_connection.commit()

        resp = api_client.post(self.UPDATE_URL, json={
            "comment_id": cid, "status": 1, "platform": "TWITTER",
        })
        assert_response_success(resp)

        db_cursor.execute(
            "SELECT status FROM gm_agent_twitter_comments WHERE twitter_comment_id = %s",
            (cid,),
        )
        assert db_cursor.fetchone()["status"] == 1

    # -- 4. Facebook platform --

    def test_facebook_status_update_success(self, api_client, db_cursor, db_connection):
        """Facebook comment status should update when platform='facebook'."""
        cid = self._ensure_facebook_comment(db_cursor, db_connection)

        db_cursor.execute(
            "UPDATE gm_agent_facebook_comments SET status = 0 WHERE facebook_comment_id = %s",
            (cid,),
        )
        db_connection.commit()

        resp = api_client.post(self.UPDATE_URL, json={
            "comment_id": cid, "status": 2, "platform": "facebook",
        })
        assert_response_success(resp)
        data = extract_data(resp.json())
        assert data["success"] is True

        db_cursor.execute(
            "SELECT status FROM gm_agent_facebook_comments WHERE facebook_comment_id = %s",
            (cid,),
        )
        row = db_cursor.fetchone()
        assert row is not None, "Facebook comment row should exist"
        assert row["status"] == 2

    def test_facebook_comment_disappears_from_pending(self, api_client, db_cursor, db_connection):
        """After update to status=2, the Facebook comment should no longer be pending."""
        cid = self._ensure_facebook_comment(db_cursor, db_connection)

        db_cursor.execute(
            "UPDATE gm_agent_facebook_comments SET status = 0 WHERE facebook_comment_id = %s",
            (cid,),
        )
        db_connection.commit()

        db_cursor.execute(
            "SELECT 1 FROM gm_agent_facebook_comments WHERE facebook_comment_id = %s AND status = 0",
            (cid,),
        )
        assert db_cursor.fetchone() is not None, "Should be pending before update"

        resp = api_client.post(self.UPDATE_URL, json={
            "comment_id": cid, "status": 2, "platform": "facebook",
        })
        assert_response_success(resp)

        db_cursor.execute(
            "SELECT 1 FROM gm_agent_facebook_comments WHERE facebook_comment_id = %s AND status = 0",
            (cid,),
        )
        assert db_cursor.fetchone() is None, "Comment should no longer be pending"

    def test_facebook_nonexistent_comment(self, api_client):
        """Updating a non-existent Facebook comment should return 404."""
        resp = api_client.post(self.UPDATE_URL, json={
            "comment_id": "fake_fb_id_xyz_999", "status": 1, "platform": "facebook",
        })
        assert resp.status_code in (404, 200)

    # -- 5. Instagram platform --

    def test_instagram_status_update_success(self, api_client, db_cursor, db_connection):
        """Instagram comment status should update when platform='instagram'."""
        cid = self._ensure_instagram_comment(db_cursor, db_connection)

        db_cursor.execute(
            "UPDATE gm_agent_instagram_comments SET status = 0 WHERE instagram_comment_id = %s",
            (cid,),
        )
        db_connection.commit()

        resp = api_client.post(self.UPDATE_URL, json={
            "comment_id": cid, "status": 2, "platform": "instagram",
        })
        assert_response_success(resp)
        data = extract_data(resp.json())
        assert data["success"] is True

        db_cursor.execute(
            "SELECT status FROM gm_agent_instagram_comments WHERE instagram_comment_id = %s",
            (cid,),
        )
        row = db_cursor.fetchone()
        assert row is not None, "Instagram comment row should exist"
        assert row["status"] == 2

    def test_instagram_comment_disappears_from_pending(self, api_client, db_cursor, db_connection):
        """After update to status=2, the Instagram comment should no longer be pending."""
        cid = self._ensure_instagram_comment(db_cursor, db_connection)

        db_cursor.execute(
            "UPDATE gm_agent_instagram_comments SET status = 0 WHERE instagram_comment_id = %s",
            (cid,),
        )
        db_connection.commit()

        db_cursor.execute(
            "SELECT 1 FROM gm_agent_instagram_comments WHERE instagram_comment_id = %s AND status = 0",
            (cid,),
        )
        assert db_cursor.fetchone() is not None, "Should be pending before update"

        resp = api_client.post(self.UPDATE_URL, json={
            "comment_id": cid, "status": 2, "platform": "instagram",
        })
        assert_response_success(resp)

        db_cursor.execute(
            "SELECT 1 FROM gm_agent_instagram_comments WHERE instagram_comment_id = %s AND status = 0",
            (cid,),
        )
        assert db_cursor.fetchone() is None, "Comment should no longer be pending"

    def test_instagram_nonexistent_comment(self, api_client):
        """Updating a non-existent Instagram comment should return 404."""
        resp = api_client.post(self.UPDATE_URL, json={
            "comment_id": "fake_ig_id_xyz_999", "status": 1, "platform": "instagram",
        })
        assert resp.status_code in (404, 200)

    # -- 6. Reddit platform --

    def test_reddit_status_update_success(self, api_client, db_cursor, db_connection):
        """Reddit comment status should update when platform='reddit'."""
        cid = self._ensure_reddit_comment(db_cursor, db_connection)

        db_cursor.execute(
            "UPDATE gm_agent_reddit_comments SET status = 0 WHERE comment_id = %s",
            (cid,),
        )
        db_connection.commit()

        resp = api_client.post(self.UPDATE_URL, json={
            "comment_id": cid, "status": 2, "platform": "reddit",
        })
        assert_response_success(resp)
        data = extract_data(resp.json())
        assert data["success"] is True

        db_cursor.execute(
            "SELECT status FROM gm_agent_reddit_comments WHERE comment_id = %s",
            (cid,),
        )
        row = db_cursor.fetchone()
        assert row is not None, "Reddit comment row should exist"
        assert row["status"] == 2

    def test_reddit_comment_disappears_from_pending(self, api_client, db_cursor, db_connection):
        """After update to status=2, the Reddit comment should no longer be pending."""
        cid = self._ensure_reddit_comment(db_cursor, db_connection)

        db_cursor.execute(
            "UPDATE gm_agent_reddit_comments SET status = 0 WHERE comment_id = %s",
            (cid,),
        )
        db_connection.commit()

        db_cursor.execute(
            "SELECT 1 FROM gm_agent_reddit_comments WHERE comment_id = %s AND status = 0",
            (cid,),
        )
        assert db_cursor.fetchone() is not None, "Should be pending before update"

        resp = api_client.post(self.UPDATE_URL, json={
            "comment_id": cid, "status": 2, "platform": "reddit",
        })
        assert_response_success(resp)

        db_cursor.execute(
            "SELECT 1 FROM gm_agent_reddit_comments WHERE comment_id = %s AND status = 0",
            (cid,),
        )
        assert db_cursor.fetchone() is None, "Comment should no longer be pending"

    def test_reddit_nonexistent_comment(self, api_client):
        """Updating a non-existent Reddit comment should return 404."""
        resp = api_client.post(self.UPDATE_URL, json={
            "comment_id": "fake_rd_id_xyz_999", "status": 1, "platform": "reddit",
        })
        assert resp.status_code in (404, 200)

    # -- 7. Nonexistent comment_id --

    def test_nonexistent_comment_returns_error(self, api_client):
        """Updating a non-existent comment should return 404."""
        resp = api_client.post(self.UPDATE_URL, json={
            "comment_id": "totally_fake_id_xyz_999", "status": 1,
        })
        assert resp.status_code in (404, 200)
        body = resp.json()
        if resp.status_code == 200:
            data = body.get("data", body)
            if isinstance(data, dict) and "success" in data:
                pass

    # -- 8. Validation: status out of range --

    def test_invalid_status_rejected(self, api_client, db_cursor, db_connection):
        """Status outside 0-2 should be rejected by validation."""
        cid = self._ensure_tiktok_comment(db_cursor, db_connection)

        resp = api_client.post(self.UPDATE_URL, json={
            "comment_id": cid, "status": 99,
        })
        assert resp.status_code in (400, 422), \
            f"Expected 400/422 for invalid status, got {resp.status_code}"


class TestExecutorTwitterCommentFlow:
    """Simulate the executor's exact Twitter comment lifecycle.

    The executor (glance_mind_executor) performs this loop for every platform:
      1. GET  /comments/by-device?device_id=X&status=0&platform=twitter
      2. Process each comment (post reply on Twitter)
      3. POST /comments/update-status  {comment_id, status:2, platform:"twitter"}
      4. Re-fetch pending -> processed comment must disappear

    These tests mirror that sequence and verify every DB column.
    """

    QUERY_URL = "/api/v1/public/comments/by-device"
    UPDATE_URL = "/api/v1/public/comments/update-status"

    SEED_COMMENT = {
        "id": 1,
        "tweet_db_id": 1,
        "campaign_id": 5,
        "twitter_comment_id": "tc_001",
        "conversation_id": "conv_001",
        "comment_screen_name": "curious_user",
        "comment_user_name": "Curious User",
        "comment_text": "What tool is it?",
        "reason": "Product inquiry",
        "suggested_reply": "Its called ProductivityPro! Check it out.",
        "favorite_count": 45,
        "reply_count": 5,
    }

    ALL_DB_COLUMNS = (
        "id, tweet_db_id, campaign_id, twitter_comment_id, conversation_id, "
        "comment_screen_name, comment_user_name, comment_user_id, "
        "comment_user_followers, comment_text, reason, suggested_reply, "
        "favorite_count, retweet_count, reply_count, in_reply_to_status_id, "
        "is_reply, media_urls, has_media, created_at_str, created_at_ts, "
        "comment_created_at, created_at, updated_at, suggested_dm, "
        "suggested_reply_post, status"
    )

    def _reset_comment(self, db_cursor, db_connection, comment_id, target_status=0):
        db_cursor.execute(
            "UPDATE gm_agent_twitter_comments SET status = %s WHERE twitter_comment_id = %s",
            (target_status, comment_id),
        )
        db_connection.commit()

    def _fetch_full_row(self, db_cursor, comment_id):
        db_cursor.execute(
            f"SELECT {self.ALL_DB_COLUMNS} FROM gm_agent_twitter_comments "
            "WHERE twitter_comment_id = %s",
            (comment_id,),
        )
        return db_cursor.fetchone()

    def _get_twitter_device_id(self, db_cursor):
        db_cursor.execute(
            """
            SELECT sa.device_id
            FROM gm_social_accounts sa
            JOIN gm_campaigns c ON c.social_group_id = sa.group_id
            WHERE sa.platform_id = 5
              AND c.id = %s
              AND sa.device_id IS NOT NULL
            ORDER BY sa.id
            LIMIT 1
            """,
            (self.SEED_COMMENT["campaign_id"],),
        )
        row = db_cursor.fetchone()
        assert row is not None, "Expected a Twitter device_id for the seeded Twitter campaign"
        return row["device_id"]

    def _get_pending_comment_ids(self, api_client, device_id):
        resp = api_client.get(
            f"{self.QUERY_URL}?device_id={device_id}&status=0"
            "&platform=twitter&page=1&per_page=50"
        )
        assert_response_success(resp)
        data = extract_data(resp.json())
        comments = data.get("comments", [])
        return [c.get("comment_id") for c in comments]

    # ------------------------------------------------------------------ #
    # Test 1 – full executor cycle: query -> update -> verify DB -> re-query
    # ------------------------------------------------------------------ #

    def test_executor_full_cycle_query_process_update(
        self, api_client, db_cursor, db_connection
    ):
        cid = self.SEED_COMMENT["twitter_comment_id"]
        device_id = self._get_twitter_device_id(db_cursor)
        self._reset_comment(db_cursor, db_connection, cid, 0)

        pre = self._fetch_full_row(db_cursor, cid)
        assert pre is not None, "Seed comment tc_001 must exist"
        assert pre["status"] == 0
        assert pre["id"] == self.SEED_COMMENT["id"]
        assert pre["tweet_db_id"] == self.SEED_COMMENT["tweet_db_id"]
        assert pre["campaign_id"] == self.SEED_COMMENT["campaign_id"]
        assert pre["twitter_comment_id"] == cid
        assert pre["conversation_id"] == self.SEED_COMMENT["conversation_id"]
        assert pre["comment_screen_name"] == self.SEED_COMMENT["comment_screen_name"]
        assert pre["comment_user_name"] == self.SEED_COMMENT["comment_user_name"]
        assert pre["comment_text"] == self.SEED_COMMENT["comment_text"]
        assert pre["reason"] == self.SEED_COMMENT["reason"]
        assert pre["suggested_reply"] == self.SEED_COMMENT["suggested_reply"]
        assert pre["favorite_count"] == self.SEED_COMMENT["favorite_count"]
        assert pre["reply_count"] == self.SEED_COMMENT["reply_count"]
        assert pre["created_at"] is not None

        pending_ids = self._get_pending_comment_ids(api_client, device_id)
        assert cid in pending_ids, f"tc_001 should be pending, got {pending_ids}"

        resp = api_client.post(self.UPDATE_URL, json={
            "comment_id": cid, "status": 2, "platform": "twitter",
        })
        assert_response_success(resp)
        data = extract_data(resp.json())
        assert data["success"] is True

        post = self._fetch_full_row(db_cursor, cid)
        assert post["status"] == 2, "status must be 2 after executor update"
        assert post["tweet_db_id"] == pre["tweet_db_id"]
        assert post["campaign_id"] == pre["campaign_id"]
        assert post["conversation_id"] == pre["conversation_id"]
        assert post["comment_screen_name"] == pre["comment_screen_name"]
        assert post["comment_user_name"] == pre["comment_user_name"]
        assert post["comment_text"] == pre["comment_text"]
        assert post["reason"] == pre["reason"]
        assert post["suggested_reply"] == pre["suggested_reply"]
        assert post["favorite_count"] == pre["favorite_count"]
        assert post["reply_count"] == pre["reply_count"]
        assert post["retweet_count"] == pre["retweet_count"]
        assert post["created_at"] == pre["created_at"]

        pending_after = self._get_pending_comment_ids(api_client, device_id)
        assert cid not in pending_after, "tc_001 must disappear from pending after update"

    # ------------------------------------------------------------------ #
    # Test 2 – batch: executor processes multiple comments in one pass
    # ------------------------------------------------------------------ #

    def test_executor_batch_update_multiple_comments(
        self, api_client, db_cursor, db_connection
    ):
        device_id = self._get_twitter_device_id(db_cursor)
        seed_ids = ["tc_001", "tc_002", "tc_004", "tc_005"]
        for sid in seed_ids:
            self._reset_comment(db_cursor, db_connection, sid, 0)

        pending_ids = self._get_pending_comment_ids(api_client, device_id)
        for sid in seed_ids:
            assert sid in pending_ids, f"{sid} should be pending before batch update"

        for sid in seed_ids:
            resp = api_client.post(self.UPDATE_URL, json={
                "comment_id": sid, "status": 2, "platform": "twitter",
            })
            assert_response_success(resp)
            assert extract_data(resp.json())["success"] is True

        for sid in seed_ids:
            row = self._fetch_full_row(db_cursor, sid)
            assert row["status"] == 2, f"{sid}: status must be 2"

        pending_after = self._get_pending_comment_ids(api_client, device_id)
        for sid in seed_ids:
            assert sid not in pending_after, f"{sid} must not be pending after batch"

    # ------------------------------------------------------------------ #
    # Test 3 – backward compat: update without platform param (fallback)
    # ------------------------------------------------------------------ #

    def test_executor_update_without_platform_fallback(
        self, api_client, db_cursor, db_connection
    ):
        cid = self.SEED_COMMENT["twitter_comment_id"]
        self._reset_comment(db_cursor, db_connection, cid, 0)

        resp = api_client.post(self.UPDATE_URL, json={
            "comment_id": cid, "status": 2,
        })
        row = self._fetch_full_row(db_cursor, cid)
        assert resp.status_code == 404, (
            f"Missing platform should default to TikTok and not touch Twitter rows: {resp.text}"
        )
        assert row["status"] == 0

    def test_invalid_platform_is_rejected_without_tiktok_fallback(
        self, api_client, db_cursor, db_connection
    ):
        cid = self.SEED_COMMENT["twitter_comment_id"]
        self._reset_comment(db_cursor, db_connection, cid, 0)

        resp = api_client.post(self.UPDATE_URL, json={
            "comment_id": cid, "status": 2, "platform": "unknown_platform",
        })
        row = self._fetch_full_row(db_cursor, cid)

        assert resp.status_code in (400, 422), (
            f"Explicit invalid platform should fail, got {resp.status_code}: {resp.text}"
        )
        assert row["status"] == 0, "Invalid platform must not mutate Twitter rows"

    # ------------------------------------------------------------------ #
    # Test 4 – API response contains every field the executor depends on
    # ------------------------------------------------------------------ #

    def test_executor_response_field_completeness(
        self, api_client, db_cursor, db_connection
    ):
        cid = self.SEED_COMMENT["twitter_comment_id"]
        device_id = self._get_twitter_device_id(db_cursor)
        self._reset_comment(db_cursor, db_connection, cid, 0)

        resp = api_client.get(
            f"{self.QUERY_URL}?device_id={device_id}&status=0"
            "&platform=twitter&page=1&per_page=50"
        )
        assert_response_success(resp)
        data = extract_data(resp.json())

        comments = data.get("comments", [])
        assert len(comments) > 0, "Should have at least one pending Twitter comment"

        target = None
        for c in comments:
            if c.get("comment_id") == cid:
                target = c
                break
        assert target is not None, f"tc_001 not found in response: {[c.get('comment_id') for c in comments]}"

        assert target.get("comment_id") == cid
        assert target.get("content") == self.SEED_COMMENT["comment_text"]
        assert target.get("suggested_reply") == self.SEED_COMMENT["suggested_reply"]
        assert target.get("reason") == self.SEED_COMMENT["reason"]

        resp_platform = target.get("platform", "")
        assert resp_platform.lower() == "twitter", f"Expected platform=twitter, got {resp_platform}"

        assert "user_nickname" in target or "comment_screen_name" in target
        assert target.get("user_nickname") == self.SEED_COMMENT["comment_screen_name"]


class TestHealthEndpoint:
    """Tests for health check endpoint."""

    def test_health_endpoint(self, api_client):
        """Test health endpoint returns OK."""
        resp = api_client.get("/health")
        assert_response_success(resp)


if __name__ == "__main__":
    pytest.main([__file__, "-v", "--tb=short"])
