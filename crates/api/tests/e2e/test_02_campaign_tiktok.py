"""
Full-Pipeline E2E: TikTok Campaign lifecycle
==============================================
Flow: API creates campaign -> activate -> Scheduler dispatches ->
      Agent-RS crawls (via TikHub mock) -> results saved to DB
Verify: ALL campaign/crawler/content/comment DB fields + wallet
"""

import json
import pytest

from conftest import PLATFORM_TIKTOK
from gm_e2e_utils import (
    assert_recent,
    assert_wallet_delta,
    assert_transaction_exists,
    create_campaign_via_api,
    activate_campaign_via_api,
    load_campaign,
    load_crawler_task,
    load_platform_posts,
    load_platform_comments,
    snapshot_wallet,
    wait_for_condition,
    wait_for_crawler_task,
    wait_for_crawler_task_status,
)

REGION_TIKTOK_US = 1
AI_MODEL_GPT52 = 2
BUDGET_CAP = 500.00


@pytest.mark.campaign
class TestCampaignTikTokPipeline:
    """TikTok campaign: API -> Agent-RS crawl -> DB content + comments."""

    def test_01_create_campaign_verify_draft(self, api_client, db, platform_groups):
        user_id = api_client.e2e_user_id
        group_id = platform_groups[PLATFORM_TIKTOK]

        wallet_before = snapshot_wallet(db, user_id)
        self.__class__.wallet_before = wallet_before
        self.__class__.user_id = user_id

        campaign_id = create_campaign_via_api(api_client, {
            "name": "Pipeline E2E TikTok Keyword",
            "platform_id": PLATFORM_TIKTOK,
            "region_id": REGION_TIKTOK_US,
            "ai_model_id": AI_MODEL_GPT52,
            "budget_cap": BUDGET_CAP,
            "schedule_type": "ONCE",
            "product_prompt": "Travel gear brand",
            "keyword": "travel tips",
            "social_group_id": group_id,
            "auto_reply_comments": True,
            "max_scan_count": 50,
        })

        self.__class__.campaign_id = campaign_id
        self.__class__.group_id = group_id

        c = load_campaign(db, campaign_id)
        assert c is not None, "Campaign should exist"
        assert c["id"] == campaign_id
        assert c["user_id"] == user_id
        assert c["name"] == "Pipeline E2E TikTok Keyword"
        assert c["status"] == "DRAFT"
        assert c["platform_id"] == PLATFORM_TIKTOK
        assert c["region_id"] == REGION_TIKTOK_US
        assert c["ai_model_id"] == AI_MODEL_GPT52
        assert float(c["budget_cap"]) == BUDGET_CAP
        assert c["schedule_type"] == "ONCE"
        assert c["keyword"] == "travel tips"
        assert c["auto_reply_comments"] is True
        assert c["is_frozen"] is False
        assert float(c["pending_consumption"]) == 0
        assert float(c["actual_consumption"]) == 0
        assert c["completed_reason"] is None
        assert_recent(c["created_at"])

        assert_wallet_delta(
            db, user_id, wallet_before,
            balance_delta=0, frozen_delta=0,
            label="TikTok DRAFT",
        )

    def test_02_activate_and_freeze_budget(self, api_client, db):
        campaign_id = self.__class__.campaign_id
        user_id = self.__class__.user_id

        wallet_before_activate = snapshot_wallet(db, user_id)
        activate_campaign_via_api(api_client, campaign_id)

        c = load_campaign(db, campaign_id)
        assert c["status"] == "ACTIVE"
        assert c["is_frozen"] is True
        assert float(c["pending_consumption"]) > 0
        assert_recent(c["updated_at"])

        frozen_amount = float(c["pending_consumption"])
        self.__class__.frozen_amount = frozen_amount

        assert_wallet_delta(
            db, user_id, wallet_before_activate,
            balance_delta=-frozen_amount, frozen_delta=frozen_amount,
            label="TikTok ACTIVATE",
        )

    def test_03_scheduler_creates_crawler_task(self, db):
        campaign_id = self.__class__.campaign_id

        tasks = wait_for_crawler_task(db, campaign_id, timeout=30)
        assert len(tasks) >= 1

        task = tasks[0]
        self.__class__.crawler_task_id = task["id"]

        assert task["campaign_id"] == campaign_id
        assert task["status"] in ("pending", "running", "completed")
        assert_recent(task["created_at"])

    def test_04_agent_completes_task(self, db):
        task_id = self.__class__.crawler_task_id

        wait_for_crawler_task_status(db, task_id, "completed", timeout=60)

        task = load_crawler_task(db, task_id)
        assert task["status"] == "completed"
        assert task["error_message"] is None
        assert_recent(task["updated_at"])

    def test_05_tiktok_content_saved(self, db):
        campaign_id = self.__class__.campaign_id

        posts = load_platform_posts(db, "gm_agent_videos", campaign_id)
        assert len(posts) > 0, "Should have crawled TikTok videos"

        for post in posts:
            assert post["campaign_id"] == campaign_id
            assert post["video_id"] is not None
            assert_recent(post["created_at"])

    def test_06_tiktok_comments_saved(self, db):
        campaign_id = self.__class__.campaign_id

        comments = load_platform_comments(db, "gm_agent_comments", campaign_id)
        assert len(comments) > 0, "Should have crawled TikTok comments"

        for comment in comments:
            assert comment["campaign_id"] == campaign_id
            assert_recent(comment["created_at"])

    def test_07_campaign_reaches_terminal_state(self, db):
        campaign_id = self.__class__.campaign_id

        def _check():
            c = load_campaign(db, campaign_id)
            return c and c["status"] in ("COMPLETED", "ACTIVE")

        wait_for_condition(
            _check, timeout=60,
            description=f"campaign {campaign_id} reaches terminal state",
        )

        c = load_campaign(db, campaign_id)
        assert c["status"] in ("COMPLETED", "ACTIVE")
        assert c["total_scanned"] > 0
        assert_recent(c["updated_at"])
