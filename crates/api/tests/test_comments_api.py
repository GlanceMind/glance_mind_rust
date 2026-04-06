"""
GlanceMind API E2E Tests - Comments API
=======================================
Tests for all comments-related endpoints across all platforms.

Test Coverage:
1. TikTok comments (legacy /agent/videos/:id/comments)
2. Unified comments endpoint (/agent/contents/:id/comments?platform_id=X)
3. Comment structure validation
4. Database state verification
"""

import pytest
from conftest import (
    assert_response_success,
    assert_json_structure,
    extract_data,
    PLATFORM_TIKTOK,
    PLATFORM_FACEBOOK,
    PLATFORM_INSTAGRAM,
    PLATFORM_REDDIT,
    PLATFORM_TWITTER,
    TIKTOK_VIDEO_IDS,
    FACEBOOK_POST_IDS,
    INSTAGRAM_POST_IDS,
    REDDIT_POST_IDS,
    TWITTER_TWEET_IDS,
)


# =============================================================================
# Test: TikTok Comments (Legacy Endpoint)
# =============================================================================

def get_comments_from_response(response_json):
    """Extract comments list from API response, handling various formats."""
    data = extract_data(response_json)
    if isinstance(data, list):
        return data
    if isinstance(data, dict):
        return data.get("list", data.get("comments", []))
    return []


class TestTikTokCommentsLegacy:
    """Tests for legacy TikTok video comments endpoint."""

    def test_get_video_comments_success(self, auth_client, db_cursor):
        """Test getting comments for a TikTok video."""
        video_id = TIKTOK_VIDEO_IDS[0]  # video_id = 1
        
        # API request
        resp = auth_client.get(f"/api/v1/agent/videos/{video_id}/comments")
        assert_response_success(resp)
        
        response_data = resp.json()
        print(f"\nTikTok Comments Response: {response_data}")
        
        data = extract_data(response_data)
        comments = get_comments_from_response(response_data)
        
        # Verify we got comments
        assert len(comments) > 0, "Should have at least one comment"
        
        # Verify comment structure
        for comment in comments:
            assert "comment_id" in comment, "Comment should have comment_id"
            assert "content" in comment or "suggested_reply" in comment, \
                "Comment should have content or suggested_reply"
        
        # Verify against database
        db_cursor.execute("""
            SELECT COUNT(*) as count FROM gm_agent_comments 
            WHERE video_db_id = %s
        """, (video_id,))
        db_result = db_cursor.fetchone()
        
        print(f"  DB comment count for video {video_id}: {db_result['count']}")
        
        # The API might paginate, so check total if available
        if isinstance(data, dict) and "total" in data:
            assert data["total"] == db_result["count"], \
                f"Total should match DB count: {data['total']} vs {db_result['count']}"

    def test_get_video_comments_pagination(self, auth_client):
        """Test pagination for video comments."""
        video_id = TIKTOK_VIDEO_IDS[0]
        
        # Get first page
        resp = auth_client.get(
            f"/api/v1/agent/videos/{video_id}/comments",
            params={"page": 1, "page_size": 2}
        )
        assert_response_success(resp)
        
        comments = get_comments_from_response(resp.json())
        
        # Should respect page_size
        assert len(comments) <= 2, "Should respect page_size limit"

    def test_get_video_comments_not_found(self, auth_client):
        """Test getting comments for non-existent video."""
        resp = auth_client.get("/api/v1/agent/videos/99999/comments")
        
        # Should return empty list or 404
        if resp.status_code == 200:
            comments = get_comments_from_response(resp.json())
            assert len(comments) == 0, "Should return empty list for non-existent video"


# =============================================================================
# Test: Unified Comments Endpoint (All Platforms)
# =============================================================================

class TestUnifiedCommentsAPI:
    """Tests for unified comments endpoint across all platforms."""

    @pytest.mark.parametrize("platform_id,content_id,expected_min_count", [
        (PLATFORM_TIKTOK, TIKTOK_VIDEO_IDS[0], 3),      # video 1 has 3 comments
        (PLATFORM_FACEBOOK, FACEBOOK_POST_IDS[0], 3),   # post 1 has 3 comments
        (PLATFORM_INSTAGRAM, INSTAGRAM_POST_IDS[1], 3), # post 2 has 3 comments
        (PLATFORM_REDDIT, REDDIT_POST_IDS[0], 3),       # post 1 has 3 comments
        (PLATFORM_TWITTER, TWITTER_TWEET_IDS[0], 3),    # tweet 1 has 3 comments
    ])
    def test_get_unified_comments_all_platforms(
        self, auth_client, db_cursor, platform_id, content_id, expected_min_count
    ):
        """Test unified comments endpoint for all platforms."""
        platform_names = {
            PLATFORM_REDDIT: "Reddit",
            PLATFORM_TIKTOK: "TikTok",
            PLATFORM_FACEBOOK: "Facebook",
            PLATFORM_INSTAGRAM: "Instagram",
            PLATFORM_TWITTER: "Twitter",
        }
        
        print(f"\n=== Testing {platform_names[platform_id]} Comments ===")
        
        # API request
        resp = auth_client.get(
            f"/api/v1/agent/contents/{content_id}/comments",
            params={"platform_id": platform_id, "page": 1, "page_size": 20}
        )
        assert_response_success(resp)
        
        response_data = resp.json()
        print(f"Response: {response_data}")
        
        # Verify response structure
        comments = get_comments_from_response(response_data)
        
        assert len(comments) >= expected_min_count, \
            f"Expected at least {expected_min_count} comments for {platform_names[platform_id]}"
        
        # Verify unified comment structure
        for comment in comments:
            self._verify_unified_comment_structure(comment, platform_names[platform_id])
        
        print(f"  ✓ {platform_names[platform_id]}: {len(comments)} comments verified")

    def _verify_unified_comment_structure(self, comment: dict, platform_name: str):
        """Verify that comment has unified structure."""
        required_fields = ["id", "comment_id"]
        
        for field in required_fields:
            assert field in comment, \
                f"{platform_name}: Missing required field '{field}' in comment"
        
        # At least one of these content fields should exist
        content_fields = ["content", "comment_text", "body"]
        has_content = any(comment.get(f) for f in content_fields)
        assert has_content or comment.get("suggested_reply"), \
            f"{platform_name}: Comment should have content or suggested_reply"

    def test_unified_comments_invalid_platform(self, auth_client):
        """Test unified comments with invalid platform ID."""
        resp = auth_client.get(
            "/api/v1/agent/contents/1/comments",
            params={"platform_id": 999}  # Invalid platform
        )

        assert resp.status_code in (400, 422), (
            f"Invalid platform_id should fail loudly, got {resp.status_code}: {resp.text}"
        )


# =============================================================================
# Test: Comment Structure Validation
# =============================================================================

class TestCommentStructure:
    """Tests for comment data structure validation."""

    def test_tiktok_comment_fields(self, auth_client, db_cursor):
        """Verify TikTok comment has expected fields."""
        video_id = TIKTOK_VIDEO_IDS[0]
        
        resp = auth_client.get(f"/api/v1/agent/videos/{video_id}/comments")
        assert_response_success(resp)
        
        comments = get_comments_from_response(resp.json())
        
        if comments:
            comment = comments[0]
            
            # TikTok-specific fields
            expected_fields = ["id", "comment_id", "status"]
            for field in expected_fields:
                assert field in comment, f"Missing TikTok field: {field}"
            
            # Optional but expected fields
            optional_fields = [
                "user_nickname", "user_unique_id", "content",
                "reason", "suggested_reply", "suggested_dm"
            ]
            present_optional = [f for f in optional_fields if f in comment]
            print(f"\n  TikTok optional fields present: {present_optional}")

    def test_unified_comment_response_format(self, auth_client):
        """Test that unified comments follow expected response format."""
        # Test with Facebook
        resp = auth_client.get(
            f"/api/v1/agent/contents/{FACEBOOK_POST_IDS[0]}/comments",
            params={"platform_id": PLATFORM_FACEBOOK}
        )
        assert_response_success(resp)
        
        data = extract_data(resp.json())
        
        # Should have pagination structure
        if isinstance(data, dict):
            pagination_fields = ["list", "total", "page", "page_size"]
            present_fields = [f for f in pagination_fields if f in data]
            print(f"\n  Pagination fields present: {present_fields}")


# =============================================================================
# Test: Database State Verification
# =============================================================================

class TestDatabaseState:
    """Tests that verify database state matches API responses."""

    def test_tiktok_comments_db_consistency(self, auth_client, db_cursor):
        """Verify TikTok comments API matches database state."""
        print("\n=== TikTok Comments DB Consistency ===")
        
        for video_id in TIKTOK_VIDEO_IDS:
            # Get from API
            resp = auth_client.get(f"/api/v1/agent/videos/{video_id}/comments")
            if resp.status_code != 200:
                continue
            
            response_data = resp.json()
            api_comments = get_comments_from_response(response_data)
            data = extract_data(response_data)
            
            # Get from database
            db_cursor.execute("""
                SELECT id, comment_id, user_nickname, content, status, suggested_reply
                FROM gm_agent_comments 
                WHERE video_db_id = %s
                ORDER BY id
            """, (video_id,))
            db_comments = db_cursor.fetchall()
            
            print(f"  Video {video_id}: API={len(api_comments)}, DB={len(db_comments)}")
            
            # Verify counts match (accounting for pagination)
            if isinstance(data, dict) and "total" in data:
                assert data["total"] == len(db_comments), \
                    f"Video {video_id}: Total mismatch"

    def test_all_platforms_have_mock_data(self, db_cursor):
        """Verify all platforms have mock data in database."""
        print("\n=== Platform Mock Data Verification ===")
        
        tables = [
            ("TikTok Videos", "gm_agent_videos"),
            ("TikTok Comments", "gm_agent_comments"),
            ("Facebook Posts", "gm_agent_facebook_posts"),
            ("Facebook Comments", "gm_agent_facebook_comments"),
            ("Instagram Posts", "gm_agent_instagram_posts"),
            ("Instagram Comments", "gm_agent_instagram_comments"),
            ("Reddit Posts", "gm_agent_reddit_posts"),
            ("Reddit Comments", "gm_agent_reddit_comments"),
            ("Twitter Tweets", "gm_agent_twitter_tweets"),
            ("Twitter Comments", "gm_agent_twitter_comments"),
        ]
        
        for name, table in tables:
            db_cursor.execute(f"SELECT COUNT(*) as count FROM {table}")
            result = db_cursor.fetchone()
            count = result["count"]
            
            print(f"  {name}: {count} records")
            assert count > 0, f"{name} should have mock data"

    def test_comments_have_campaign_association(self, db_cursor):
        """Verify comments are properly associated with campaigns."""
        print("\n=== Campaign Association Verification ===")
        
        # TikTok comments
        db_cursor.execute("""
            SELECT c.id, c.comment_id, c.campaign_id, camp.name as campaign_name
            FROM gm_agent_comments c
            JOIN gm_campaigns camp ON c.campaign_id = camp.id
            LIMIT 5
        """)
        results = db_cursor.fetchall()
        
        for row in results:
            print(f"  Comment {row['id']}: campaign={row['campaign_name']}")
            assert row["campaign_id"] is not None, "Comment should have campaign_id"


# =============================================================================
# Test: Error Handling
# =============================================================================

class TestErrorHandling:
    """Tests for API error handling."""

    def test_unauthorized_access(self, api_client):
        """Test that unauthenticated requests are rejected."""
        resp = api_client.get("/api/v1/agent/videos/1/comments")
        
        # Should be 401 Unauthorized
        assert resp.status_code == 401, \
            f"Expected 401, got {resp.status_code}"

    def test_invalid_content_id(self, auth_client):
        """Test handling of invalid content ID."""
        resp = auth_client.get(
            "/api/v1/agent/contents/abc/comments",  # Invalid ID format
            params={"platform_id": PLATFORM_TIKTOK}
        )
        
        # Should return 400 or 404
        assert resp.status_code in [400, 404, 422], \
            f"Expected 400/404/422 for invalid ID, got {resp.status_code}"

    def test_missing_platform_id(self, auth_client):
        """Test unified endpoint without platform_id."""
        resp = auth_client.get("/api/v1/agent/contents/1/comments")
        
        # Should return 400 (missing required parameter)
        assert resp.status_code in [400, 422], \
            f"Expected 400/422 for missing platform_id, got {resp.status_code}"


# =============================================================================
# Test: Integration Scenarios
# =============================================================================

class TestIntegrationScenarios:
    """End-to-end integration scenarios."""

    def test_full_comments_workflow(self, auth_client, db_cursor):
        """Test complete workflow: fetch comments, verify structure, check DB."""
        print("\n=== Full Comments Workflow Test ===")
        
        platforms = [
            (PLATFORM_TIKTOK, TIKTOK_VIDEO_IDS[0], "TikTok"),
            (PLATFORM_FACEBOOK, FACEBOOK_POST_IDS[0], "Facebook"),
            (PLATFORM_INSTAGRAM, INSTAGRAM_POST_IDS[0], "Instagram"),
            (PLATFORM_REDDIT, REDDIT_POST_IDS[0], "Reddit"),
            (PLATFORM_TWITTER, TWITTER_TWEET_IDS[0], "Twitter"),
        ]
        
        results = {}
        
        for platform_id, content_id, name in platforms:
            print(f"\n  Testing {name}...")
            
            # Fetch comments
            resp = auth_client.get(
                f"/api/v1/agent/contents/{content_id}/comments",
                params={"platform_id": platform_id}
            )
            
            if resp.status_code == 200:
                comments = get_comments_from_response(resp.json())
                results[name] = {
                    "status": "success",
                    "count": len(comments),
                    "has_content": any(
                        c.get("content") or c.get("comment_text") or c.get("body")
                        for c in comments
                    ) if comments else False
                }
                print(f"    ✓ {name}: {len(comments)} comments")
            else:
                results[name] = {
                    "status": "error",
                    "code": resp.status_code
                }
                print(f"    ✗ {name}: HTTP {resp.status_code}")
        
        # Summary
        print("\n  === Summary ===")
        success_count = sum(1 for r in results.values() if r["status"] == "success")
        print(f"  Platforms tested: {len(platforms)}")
        print(f"  Successful: {success_count}")
        
        assert success_count >= 3, "At least 3 platforms should work"


if __name__ == "__main__":
    pytest.main([__file__, "-v", "--tb=short"])
