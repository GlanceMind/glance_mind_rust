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

import io
import uuid
import zipfile

import pytest
from conftest import (
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
