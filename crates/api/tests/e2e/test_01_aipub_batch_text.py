"""
Full-Pipeline E2E: AIPub batch_text lifecycle
===============================================
Flow: API creates plan -> Scheduler picks up -> AI content_gen -> publish tasks created
Verify: ALL DB fields + wallet balance + transaction records
"""

import json
import pytest

from conftest import PLATFORM_TIKTOK
from gm_e2e_utils import (
    assert_recent,
    assert_wallet_delta,
    assert_transaction_exists,
    create_plan_via_api,
    load_plan,
    load_ai_tasks,
    load_publish_tasks,
    load_wallet,
    snapshot_wallet,
    wait_for_ai_tasks,
    wait_for_plan_status_in,
)

AI_MODEL_GPT52 = 2


@pytest.mark.aipub
class TestAIPubBatchTextPipeline:
    """batch_text: API -> Scheduler -> Agent content generation -> publish tasks."""

    def test_01_create_plan_and_verify_db(self, api_client, db, platform_groups, platform_accounts):
        user_id = api_client.e2e_user_id
        group_id = platform_groups[PLATFORM_TIKTOK]

        wallet_before = snapshot_wallet(db, user_id)
        self.__class__.wallet_before = wallet_before

        plan_id = create_plan_via_api(api_client, {
            "plan_type": "batch_text",
            "platform_id": PLATFORM_TIKTOK,
            "group_id": group_id,
            "content_type": "post",
            "chat_ai_model_id": AI_MODEL_GPT52,
            "ai_input": {
                "content_prompt": "Write an engaging travel post about hidden gems in Japan",
                "hashtags_count": 5,
            },
            "name": "Pipeline E2E batch_text test",
        })

        self.__class__.plan_id = plan_id
        self.__class__.group_id = group_id
        self.__class__.user_id = user_id

        plan = load_plan(db, plan_id)
        assert plan is not None, "Plan should exist in DB"
        assert plan["id"] == plan_id
        assert plan["user_id"] == user_id
        assert plan["group_id"] == group_id
        assert plan["social_account_id"] is None
        assert plan["platform_id"] == PLATFORM_TIKTOK
        assert plan["content_type"] == "post"
        assert plan["plan_type"] == "batch_text"
        assert plan["name"] == "Pipeline E2E batch_text test"
        assert plan["chat_ai_model_id"] == AI_MODEL_GPT52
        assert plan["video_ai_model_id"] is None
        assert plan["image_ai_model_id"] is None
        assert plan["status"] == "pending"
        assert plan["content"] is None

        ai_input = plan["ai_input"] if isinstance(plan["ai_input"], dict) else json.loads(plan["ai_input"])
        assert ai_input["content_prompt"] == "Write an engaging travel post about hidden gems in Japan"

        frozen_cost = float(plan["frozen_cost"])
        self.__class__.frozen_cost = frozen_cost
        assert plan["billing_status"] in ("none", "frozen")
        assert float(plan["consumed_cost"]) == 0
        assert_recent(plan["created_at"])

        if plan["billing_status"] == "frozen":
            assert frozen_cost > 0
            assert_recent(plan["frozen_at"])
            assert_wallet_delta(
                db, user_id, wallet_before,
                balance_delta=-frozen_cost, frozen_delta=frozen_cost,
                label="batch_text freeze",
            )
            assert_transaction_exists(
                db, user_id,
                tx_type="FREEZE",
                reference_id=plan_id,
                expected_amount=-frozen_cost,
                label="batch_text FREEZE",
            )

    def test_02_scheduler_creates_ai_task(self, db):
        plan_id = self.__class__.plan_id

        wait_for_ai_tasks(db, plan_id, min_count=1, timeout=30)
        wait_for_plan_status_in(db, plan_id, ["ready", "completed"], timeout=30)

        ai_tasks = load_ai_tasks(db, plan_id)
        assert len(ai_tasks) >= 1

        task = ai_tasks[0]
        self.__class__.ai_task_id = task["id"]

        assert task["plan_id"] == plan_id
        assert task["task_type"] == "content_gen"
        assert task["external_service"] is not None
        assert task["status"] == "completed"
        assert task["progress"] == 100
        assert task["sequence"] == 0
        assert task["retry_count"] >= 0
        assert task["error_message"] is None

        task_input = task["input"] if isinstance(task["input"], dict) else json.loads(task["input"])
        assert task_input is not None

        result = task["result"] if isinstance(task["result"], dict) else json.loads(task["result"])
        assert result is not None

        assert_recent(task["created_at"])
        assert_recent(task["completed_at"])
        assert task["updated_at"] is not None

    def test_03_publish_tasks_created(self, db, platform_accounts):
        plan_id = self.__class__.plan_id
        account_ids = platform_accounts[PLATFORM_TIKTOK]

        wait_for_plan_status_in(db, plan_id, ["ready", "completed"], timeout=30)

        pub_tasks = load_publish_tasks(db, plan_id)
        assert len(pub_tasks) == len(account_ids)

        for task in pub_tasks:
            assert task["plan_id"] == plan_id
            assert task["social_account_id"] in account_ids
            assert task["status"] == "ready"
            assert task["retry_count"] == 0
            assert task["error_message"] is None
            assert task["result_url"] is None
            assert task["published_at"] is None
            assert task["ai_task_id"] is not None

            content = task["content"] if isinstance(task["content"], dict) else json.loads(task["content"])
            assert content is not None
            assert_recent(task["created_at"])

    def test_04_plan_billing_state(self, db):
        plan_id = self.__class__.plan_id
        plan = load_plan(db, plan_id)

        assert plan["status"] in ("ready", "completed")
        assert_recent(plan["updated_at"])
        assert plan["billing_status"] in ("frozen", "consumed", "settled")
        assert float(plan["frozen_cost"]) >= 0
        assert float(plan["consumed_cost"]) >= 0

    def test_05_wallet_conservation(self, db):
        user_id = self.__class__.user_id

        wallet = load_wallet(db, user_id)
        assert wallet is not None
        balance = float(wallet["balance_points"])
        frozen = float(wallet["frozen_points"])

        assert balance > 0, f"Wallet balance should be positive, got {balance}"
        assert frozen >= 0, f"Frozen points should be non-negative, got {frozen}"

        wallet_before = self.__class__.wallet_before
        initial_total = wallet_before[0] + wallet_before[1]
        current_total = balance + frozen
        assert current_total <= initial_total + 0.01, (
            f"Wallet total should not exceed initial: before={initial_total}, after={current_total}"
        )
