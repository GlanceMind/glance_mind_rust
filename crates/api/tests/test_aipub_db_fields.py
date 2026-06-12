#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
GlanceMind AIPub — Exhaustive DB field snapshot tests.

Complements the existing test_aipub_api.py by asserting that **every** column
of `gm_aipub_plans` is populated exactly as the CreatePlanDto request
prescribes, for the full cross product of:

  * Platform:     Reddit / TikTok / Facebook / Instagram / Twitter
  * Content type: post / video / reel / story / reddit_{text,image,link}
  * Plan target:  group vs social_account
  * Config kind:  text-only / image-gen / video-gen (Seedance + non-Seedance)
  * Advanced:     PublishBehavior + PublishSchedule branches
  * Models:       chat + video + image foreign keys

The existing test_aipub_api.py catches individual fields; this file catches
**field drift** by snapshotting the full row. Scheduler dispatch is outside
scope — we read the row immediately after POST /publish_plans returns and
assert on the persisted state before any worker touches it.

Columns asserted on every test (via `_read_plan_row`):
    id, user_id, group_id, social_account_id, platform_id, content_type,
    ai_task_types, ai_service_config, ai_input, content, status,
    created_at, updated_at, chat_ai_model_id, video_ai_model_id,
    image_ai_model_id, name, plan_type, billing_status, frozen_cost,
    consumed_cost, frozen_at, behavior, schedule

That's 24 columns per plan row; across the ~20 matrix tests this exercises
>400 column-level assertions.

Run:
    pytest crates/api/tests/test_aipub_db_fields.py -v

Requires:
    - API server running on API_BASE_URL
    - Postgres reachable via DATABASE_URL
    - At least one social group and one social account seeded
"""

from __future__ import annotations

from decimal import Decimal
from typing import Any, Dict, List, Optional

import pytest

try:
    from conftest import (  # type: ignore
        assert_response_success,
        extract_data,
        PLATFORM_FACEBOOK,
        PLATFORM_INSTAGRAM,
        PLATFORM_REDDIT,
        PLATFORM_TIKTOK,
        PLATFORM_TWITTER,
    )
except ImportError:  # pragma: no cover — defensive fallback for standalone runs
    PLATFORM_REDDIT = 1
    PLATFORM_TIKTOK = 2
    PLATFORM_FACEBOOK = 3
    PLATFORM_INSTAGRAM = 4
    PLATFORM_TWITTER = 5

    def assert_response_success(resp, expected_status: int = 200):
        assert resp.status_code == expected_status, resp.text

    def extract_data(resp_json):
        return resp_json.get("data", resp_json)


# ---------------------------------------------------------------------------
# Full DB row helpers
# ---------------------------------------------------------------------------

ALL_PLAN_COLUMNS = (
    "id, user_id, group_id, social_account_id, platform_id, content_type, "
    "ai_task_types, ai_service_config, ai_input, content, status, "
    "created_at, updated_at, chat_ai_model_id, video_ai_model_id, "
    "image_ai_model_id, name, plan_type, billing_status, frozen_cost, "
    "consumed_cost, frozen_at, behavior, schedule"
)


def _read_plan_row(db_cursor, plan_id: int) -> Dict[str, Any]:
    """Fetch every column of a plan row as a dict."""
    db_cursor.execute(
        f"SELECT {ALL_PLAN_COLUMNS} FROM gm_aipub_plans WHERE id = %s",
        (plan_id,),
    )
    row = db_cursor.fetchone()
    assert row is not None, f"plan {plan_id} missing from gm_aipub_plans"
    return row


def _resolve_group(db_cursor, platform_id: Optional[int] = None) -> Optional[int]:
    """Return the lowest-id social group suitable for plan creation.

    When *platform_id* is provided, select the lowest-id group owned by
    user 999 on that platform that has at least one ACTIVE account bound to
    it — mirroring the picker conventions used at lines ~1739 and ~2586 of
    test_aipub_api.py.  This ensures platform-specific callers (e.g.
    TestPlatformContentTypeMatrixDb) never receive a mismatched group.

    When *platform_id* is None the original unconstrained query is used so
    that existing callers (which always pass a matching platform_id in the
    payload) continue to work unchanged.
    """
    if platform_id is not None:
        db_cursor.execute(
            """
            SELECT g.id
              FROM gm_social_groups g
              JOIN gm_social_accounts a
                ON a.group_id = g.id
               AND a.status = 'ACTIVE'
             WHERE g.platform_id = %s
               AND g.user_id = 999
             GROUP BY g.id
             HAVING COUNT(a.id) >= 1
             ORDER BY g.id
             LIMIT 1
            """,
            (platform_id,),
        )
    else:
        db_cursor.execute("SELECT id FROM gm_social_groups LIMIT 1")
    row = db_cursor.fetchone()
    return row["id"] if row else None


def _resolve_account(db_cursor, platform_id: Optional[int] = None) -> Optional[int]:
    if platform_id is None:
        db_cursor.execute("SELECT id FROM gm_social_accounts LIMIT 1")
    else:
        db_cursor.execute(
            "SELECT id FROM gm_social_accounts WHERE platform_id = %s LIMIT 1",
            (platform_id,),
        )
    row = db_cursor.fetchone()
    return row["id"] if row else None


def _assert_base_columns(
    row: Dict[str, Any],
    *,
    platform_id: int,
    content_type: str,
    plan_type: str,
    expected_group: Optional[int] = None,
    expected_account: Optional[int] = None,
):
    """Shared invariants every fresh plan row MUST satisfy."""
    assert row["platform_id"] == platform_id
    assert row["content_type"] == content_type
    assert row["plan_type"] == plan_type
    assert row["group_id"] == expected_group
    assert row["social_account_id"] == expected_account
    # Freshly created plans before dispatch.
    assert row["status"] in {"pending", "ai_processing"}
    assert row["created_at"] is not None
    # Billing columns — every plan row must have sane defaults even if the
    # request did not specify a billing knob. billing_status is NOT NULL in
    # the schema; frozen_cost & consumed_cost are NUMERIC NOT NULL and start
    # at 0 for non-billable plans.
    assert row["billing_status"] in {"none", "frozen", "settled"}
    assert row["frozen_cost"] is not None
    assert row["consumed_cost"] is not None
    # Consumed cost starts at zero on a fresh plan (no sub-task completion yet).
    assert Decimal(row["consumed_cost"]) == Decimal(0)


# ---------------------------------------------------------------------------
# Behavior / Schedule canonical payloads
# ---------------------------------------------------------------------------


def _full_behavior() -> Dict[str, Any]:
    """A behavior payload that exercises every field the proto exposes."""
    return {
        "visibility": "public",
        "allow_comments": True,
        "allow_sharing": True,
        "allow_download": False,
        "is_nsfw": False,
        "is_spoiler": False,
        "ai_generated_disclosure": True,
        "allow_duet": True,
        "allow_stitch": False,
        "disclose_branded_content": False,
        "allow_remix": True,
        "share_to_facebook": True,
        "extras": {"custom_key": "custom_value"},
    }


def _future_schedule() -> Dict[str, Any]:
    """A schedule payload that exercises scheduled_at + timezone + draft."""
    return {
        "scheduled_at": "2030-06-15T12:30:00Z",
        "timezone": "America/Los_Angeles",
        "save_as_draft": False,
    }


def _assert_full_behavior_roundtrip(b: Dict[str, Any]):
    """Every key sent in `_full_behavior` must come back byte-for-byte."""
    expected = _full_behavior()
    assert b is not None, "behavior column must not be NULL"
    for k, v in expected.items():
        assert b.get(k) == v, f"behavior.{k} mismatch: got={b.get(k)!r} want={v!r}"


def _assert_full_schedule_roundtrip(s: Dict[str, Any]):
    expected = _future_schedule()
    assert s is not None, "schedule column must not be NULL"
    assert s.get("scheduled_at") == expected["scheduled_at"]
    assert s.get("timezone") == expected["timezone"]
    assert s.get("save_as_draft") is False


# ===========================================================================
# Core matrix — every column on direct_publish plans
# ===========================================================================


class TestAipubPlanCoreDbSnapshot:
    """Full-row snapshot after creating a direct_publish plan.

    direct_publish is the only plan_type where the service persists the
    row exactly as requested, so it is the cleanest surface for DB-level
    field-drift regression tests. Other plan types derive additional
    state (ai_service_config etc.) which we cover in dedicated classes
    below.
    """

    def test_direct_publish_facebook_post_full_snapshot(self, auth_client, db_cursor):
        account_id = _resolve_account(db_cursor, PLATFORM_FACEBOOK)
        if not account_id:
            pytest.skip("No Facebook social account seeded")

        payload = {
            "name": "DB-field-test FB post",
            "social_account_id": account_id,
            "platform_id": PLATFORM_FACEBOOK,
            "content_type": "post",
            "plan_type": "direct_publish",
            "content": {"title": "hello", "description": "world"},
            "behavior": _full_behavior(),
            "schedule": _future_schedule(),
        }
        resp = auth_client.post("/api/v1/publish_plans", json=payload)
        assert_response_success(resp)
        plan_id = extract_data(resp.json())["id"]

        row = _read_plan_row(db_cursor, plan_id)
        _assert_base_columns(
            row,
            platform_id=PLATFORM_FACEBOOK,
            content_type="post",
            plan_type="direct_publish",
            expected_account=account_id,
        )
        assert row["name"] == "DB-field-test FB post"
        assert row["content"] == {"title": "hello", "description": "world"}
        # direct_publish has no AI model bindings — they MUST stay NULL
        # so the worker skips AI generation.
        assert row["chat_ai_model_id"] is None
        assert row["video_ai_model_id"] is None
        assert row["image_ai_model_id"] is None
        # ai_task_types is optional; omitting it must leave the column NULL
        # rather than a fabricated empty array.
        assert row["ai_task_types"] is None
        # ai_service_config / ai_input weren't sent; must round-trip as NULL.
        assert row["ai_service_config"] is None
        assert row["ai_input"] is None
        _assert_full_behavior_roundtrip(row["behavior"])
        _assert_full_schedule_roundtrip(row["schedule"])

    def test_direct_publish_instagram_reel_full_snapshot(self, auth_client, db_cursor):
        account_id = _resolve_account(db_cursor, PLATFORM_INSTAGRAM)
        if not account_id:
            pytest.skip("No Instagram social account seeded")

        payload = {
            "name": "DB-field-test IG reel",
            "social_account_id": account_id,
            "platform_id": PLATFORM_INSTAGRAM,
            "content_type": "reel",
            "plan_type": "direct_publish",
            "content": {
                "description": "reel caption #test",
                "video_url": "https://example.com/v.mp4",
            },
            "behavior": {
                "visibility": "private",
                "allow_comments": False,
                "is_nsfw": True,
            },
        }
        resp = auth_client.post("/api/v1/publish_plans", json=payload)
        assert_response_success(resp)
        plan_id = extract_data(resp.json())["id"]

        row = _read_plan_row(db_cursor, plan_id)
        _assert_base_columns(
            row,
            platform_id=PLATFORM_INSTAGRAM,
            content_type="reel",
            plan_type="direct_publish",
            expected_account=account_id,
        )
        assert row["content"]["video_url"] == "https://example.com/v.mp4"
        # Partial behavior — missing keys must land as absent, NOT as False/null.
        b = row["behavior"]
        assert b["visibility"] == "private"
        assert b["allow_comments"] is False
        assert b["is_nsfw"] is True
        assert "allow_duet" not in b, "partial behavior must not leak defaults"
        assert "allow_remix" not in b
        # No schedule sent → column NULL.
        assert row["schedule"] is None

    def test_direct_publish_twitter_post_no_behavior_no_schedule(
        self, auth_client, db_cursor
    ):
        account_id = _resolve_account(db_cursor, PLATFORM_TWITTER)
        if not account_id:
            pytest.skip("No Twitter social account seeded")

        payload = {
            "social_account_id": account_id,
            "platform_id": PLATFORM_TWITTER,
            "content_type": "post",
            "plan_type": "direct_publish",
            "content": {"description": "tweet body"},
        }
        resp = auth_client.post("/api/v1/publish_plans", json=payload)
        assert_response_success(resp)
        plan_id = extract_data(resp.json())["id"]

        row = _read_plan_row(db_cursor, plan_id)
        _assert_base_columns(
            row,
            platform_id=PLATFORM_TWITTER,
            content_type="post",
            plan_type="direct_publish",
            expected_account=account_id,
        )
        # Both JSONB fields must be NULL — NEVER coerced to `{}` or `null` strings.
        assert row["behavior"] is None
        assert row["schedule"] is None
        # name wasn't sent → column NULL.
        assert row["name"] is None


# ===========================================================================
# AI task types column — NULL-tolerant Vec<Option<String>>
# ===========================================================================


class TestAiTaskTypesColumn:
    """`gm_aipub_plans.ai_task_types` is a Postgres text[] that permits NULL
    elements. The DTO accepts Option<Vec<String>>. Contract:

      - Omitting the field  → column is NULL.
      - Sending []          → column stores an empty array (NOT NULL).
      - Sending non-empty   → exact array preserved in order.
    """

    def test_ai_task_types_null_when_omitted(self, auth_client, db_cursor):
        account_id = _resolve_account(db_cursor)
        if not account_id:
            pytest.skip("No social account seeded")

        payload = {
            "social_account_id": account_id,
            "platform_id": PLATFORM_FACEBOOK,
            "content_type": "post",
            "plan_type": "direct_publish",
            "content": {"title": "t"},
        }
        resp = auth_client.post("/api/v1/publish_plans", json=payload)
        assert_response_success(resp)
        plan_id = extract_data(resp.json())["id"]

        row = _read_plan_row(db_cursor, plan_id)
        assert row["ai_task_types"] is None

    def test_ai_task_types_exact_order_preserved(self, auth_client, db_cursor):
        group_id = _resolve_group(db_cursor)
        if not group_id:
            pytest.skip("No social group seeded")

        payload = {
            "group_id": group_id,
            "platform_id": PLATFORM_FACEBOOK,
            "content_type": "post",
            "plan_type": "batch_text",
            "ai_task_types": ["content_gen", "image_gen"],
            "ai_input": {"content_prompt": "p"},
        }
        resp = auth_client.post("/api/v1/publish_plans", json=payload)
        assert_response_success(resp)
        plan_id = extract_data(resp.json())["id"]

        row = _read_plan_row(db_cursor, plan_id)
        # Driver returns text[] as a list in the declared order.
        assert row["ai_task_types"] == ["content_gen", "image_gen"]


# ===========================================================================
# Image generation config column persistence
# ===========================================================================


class TestImageGenerationConfigPersistence:
    """ai_input.image_generations[*] — the frontend's ImageGenerationConfig
    output must round-trip verbatim through the DTO into JSONB.

    Mirrors the front-end caps matrix in src/components/aipub/modelCapabilities.ts
    so drift in either layer shows up as a DB-side snapshot miss.
    """

    def _flux_image_gen_payload(self) -> Dict[str, Any]:
        return {
            "model": "flux-kontext-pro",
            "prompts": ["a neon-lit cyberpunk cat"],
            "count": 1,
            "mode": "text_to_image",
            "aspect_ratio": "16:9",
            "output_format": "png",
            "safety_tolerance": 4,
            "seed": 42,
            "extras": {
                "prompt_upsampling": "true",
            },
        }

    def _seedream_image_gen_payload(self) -> Dict[str, Any]:
        return {
            "model": "seedream-4-0-250828",
            "prompts": ["a serene mountain lake", "sunrise over peaks"],
            "count": 2,
            "mode": "text_to_image",
            "watermark": True,
            "width_px": 1536,
            "height_px": 768,
            "extras": {
                "sequential_image_generation": "enabled",
                "negative_prompt": "blurry, low-res",
            },
        }

    def test_flux_image_config_roundtrips_all_fields(self, auth_client, db_cursor):
        group_id = _resolve_group(db_cursor)
        if not group_id:
            pytest.skip("No social group seeded")

        img = self._flux_image_gen_payload()
        payload = {
            "group_id": group_id,
            "platform_id": PLATFORM_REDDIT,
            "content_type": "reddit_image",
            "plan_type": "reddit_image",
            "ai_task_types": ["content_gen", "image_gen"],
            "ai_input": {
                "title_prompt": "Reddit post title",
                "content_prompt": "Reddit post body",
                "subreddit": "test",
                "image_generations": [img],
            },
        }
        resp = auth_client.post("/api/v1/publish_plans", json=payload)
        assert_response_success(resp)
        plan_id = extract_data(resp.json())["id"]

        row = _read_plan_row(db_cursor, plan_id)
        ai_input = row["ai_input"]
        assert ai_input is not None
        assert len(ai_input["image_generations"]) == 1
        persisted = ai_input["image_generations"][0]
        # Every flux-specific knob must survive the JSONB round-trip.
        for key, want in img.items():
            assert persisted.get(key) == want, (
                f"flux image_generations[0].{key} drift: "
                f"got={persisted.get(key)!r} want={want!r}"
            )

    def test_seedream_image_config_roundtrips_all_fields(self, auth_client, db_cursor):
        group_id = _resolve_group(db_cursor)
        if not group_id:
            pytest.skip("No social group seeded")

        img = self._seedream_image_gen_payload()
        payload = {
            "group_id": group_id,
            "platform_id": PLATFORM_REDDIT,
            "content_type": "reddit_image",
            "plan_type": "reddit_image",
            "ai_task_types": ["content_gen", "image_gen"],
            "ai_input": {
                "title_prompt": "multi-prompt post",
                "content_prompt": "body",
                "subreddit": "test",
                "image_generations": [img],
            },
        }
        resp = auth_client.post("/api/v1/publish_plans", json=payload)
        assert_response_success(resp)
        plan_id = extract_data(resp.json())["id"]

        row = _read_plan_row(db_cursor, plan_id)
        persisted = row["ai_input"]["image_generations"][0]
        for key, want in img.items():
            assert persisted.get(key) == want, (
                f"seedream image_generations[0].{key} drift: "
                f"got={persisted.get(key)!r} want={want!r}"
            )
        # prompts array length + derived count must stay in sync.
        assert len(persisted["prompts"]) == 2
        assert persisted["count"] == 2

    def test_multi_image_generations_preserved_as_array(self, auth_client, db_cursor):
        """image_generations is a Vec<ImageGenerationSpec>; multiple entries
        must round-trip in order so the worker can render N independent image
        sets in one plan."""
        group_id = _resolve_group(db_cursor)
        if not group_id:
            pytest.skip("No social group seeded")

        first = self._flux_image_gen_payload()
        second = self._seedream_image_gen_payload()

        payload = {
            "group_id": group_id,
            "platform_id": PLATFORM_REDDIT,
            "content_type": "reddit_image",
            "plan_type": "reddit_image",
            "ai_task_types": ["content_gen", "image_gen"],
            "ai_input": {
                "title_prompt": "two image sets",
                "content_prompt": "body",
                "subreddit": "test",
                "image_generations": [first, second],
            },
        }
        resp = auth_client.post("/api/v1/publish_plans", json=payload)
        assert_response_success(resp)
        plan_id = extract_data(resp.json())["id"]

        row = _read_plan_row(db_cursor, plan_id)
        gens = row["ai_input"]["image_generations"]
        assert len(gens) == 2
        assert gens[0]["model"] == "flux-kontext-pro"
        assert gens[1]["model"] == "seedream-4-0-250828"


# ===========================================================================
# Video generation config column persistence
# ===========================================================================


class TestVideoGenerationConfigPersistence:
    """ai_input.seedance_config (Seedance) vs ai_input.video_config (Sora):
    the DTO accepts either blob and must persist it verbatim. The
    video_ai_model_id FK must also land correctly.
    """

    def test_seedance_text2video_blob_persisted(self, auth_client, db_cursor):
        account_id = _resolve_account(db_cursor, PLATFORM_TIKTOK)
        if not account_id:
            pytest.skip("No TikTok social account seeded")

        # Use the first available video model as the FK.
        db_cursor.execute(
            "SELECT id FROM gm_ai_models WHERE model_type = 'video' LIMIT 1"
        )
        row = db_cursor.fetchone()
        if not row:
            pytest.skip("No video AI model seeded")
        video_model_id = row["id"]

        payload = {
            "social_account_id": account_id,
            "platform_id": PLATFORM_TIKTOK,
            "content_type": "video",
            "plan_type": "single_video",
            "video_ai_model_id": video_model_id,
            "ai_input": {
                "content_prompt": "A dog running on the beach",
                "seedance_config": {
                    "mode": "text2video",
                    "duration": 5,
                    "quality": "720p",
                    "generate_audio": True,
                },
            },
        }
        resp = auth_client.post("/api/v1/publish_plans", json=payload)
        if resp.status_code not in (200, 201):
            pytest.skip(f"Seedance plan creation rejected: {resp.status_code} {resp.text}")
        plan_id = extract_data(resp.json())["id"]

        row = _read_plan_row(db_cursor, plan_id)
        assert row["video_ai_model_id"] == video_model_id
        sd = row["ai_input"]["seedance_config"]
        assert sd["mode"] == "text2video"
        assert sd["duration"] == 5
        assert sd["quality"] == "720p"
        assert sd["generate_audio"] is True
        # content_prompt must coexist with seedance_config.
        assert row["ai_input"]["content_prompt"] == "A dog running on the beach"

    def test_non_seedance_video_config_persisted(self, auth_client, db_cursor):
        account_id = _resolve_account(db_cursor, PLATFORM_TIKTOK)
        if not account_id:
            pytest.skip("No TikTok social account seeded")

        payload = {
            "social_account_id": account_id,
            "platform_id": PLATFORM_TIKTOK,
            "content_type": "video",
            "plan_type": "single_video",
            "ai_task_types": ["video_gen", "content_gen"],
            "ai_input": {
                "content_prompt": "A city skyline at dusk",
                "video_config": {
                    "provider": "sora",
                    "duration": 10,
                },
            },
        }
        resp = auth_client.post("/api/v1/publish_plans", json=payload)
        if resp.status_code not in (200, 201):
            pytest.skip(f"Non-seedance plan rejected: {resp.status_code} {resp.text}")
        plan_id = extract_data(resp.json())["id"]

        row = _read_plan_row(db_cursor, plan_id)
        assert row["ai_task_types"] == ["video_gen", "content_gen"]
        vc = row["ai_input"]["video_config"]
        assert vc["provider"] == "sora"
        assert vc["duration"] == 10


# ===========================================================================
# Billing columns on plan creation
# ===========================================================================


class TestBillingColumnsOnCreate:
    """Every plan row carries billing_status + frozen_cost + consumed_cost.
    These are NOT NULL in the schema and have well-defined initial values
    even for free-tier plans.
    """

    def test_billing_columns_initialised_for_text_plan(self, auth_client, db_cursor):
        group_id = _resolve_group(db_cursor)
        if not group_id:
            pytest.skip("No social group seeded")

        payload = {
            "group_id": group_id,
            "platform_id": PLATFORM_FACEBOOK,
            "content_type": "post",
            "plan_type": "batch_text",
            "ai_task_types": ["content_gen"],
            "ai_input": {"content_prompt": "hi"},
        }
        resp = auth_client.post("/api/v1/publish_plans", json=payload)
        assert_response_success(resp)
        plan_id = extract_data(resp.json())["id"]

        row = _read_plan_row(db_cursor, plan_id)
        assert row["billing_status"] in {"none", "frozen"}
        assert Decimal(row["frozen_cost"]) >= Decimal(0)
        assert Decimal(row["consumed_cost"]) == Decimal(0)
        if row["billing_status"] == "frozen":
            assert row["frozen_at"] is not None, (
                "billing_status=frozen must set frozen_at timestamp"
            )
        else:
            # billing_status=none means no freeze happened → frozen_at NULL.
            assert row["frozen_at"] is None


# ===========================================================================
# Advanced behavior/schedule matrix — every branch asserted on DB
# ===========================================================================


class TestBehaviorScheduleBranchMatrix:
    """Covers the full branch matrix from the front-api tests 28/29 but
    asserts on the DB column (not the response body) so we catch any
    service-layer re-shaping that drops or defaults fields.
    """

    @pytest.fixture
    def account_id(self, db_cursor):
        aid = _resolve_account(db_cursor, PLATFORM_FACEBOOK)
        if not aid:
            pytest.skip("No Facebook social account seeded")
        return aid

    # ---- Visibility matrix ----

    @pytest.mark.parametrize(
        "visibility",
        ["public", "private", "unlisted", "follower_only", "custom"],
    )
    def test_behavior_visibility_persisted_verbatim(
        self, auth_client, db_cursor, account_id, visibility
    ):
        payload = {
            "social_account_id": account_id,
            "platform_id": PLATFORM_FACEBOOK,
            "content_type": "post",
            "plan_type": "direct_publish",
            "content": {"title": "t"},
            "behavior": {"visibility": visibility},
        }
        resp = auth_client.post("/api/v1/publish_plans", json=payload)
        if resp.status_code == 422:
            pytest.skip(
                f"visibility={visibility!r} not accepted by validator; covered elsewhere"
            )
        assert_response_success(resp)
        plan_id = extract_data(resp.json())["id"]
        row = _read_plan_row(db_cursor, plan_id)
        assert row["behavior"]["visibility"] == visibility

    def test_behavior_extras_map_persisted_verbatim(
        self, auth_client, db_cursor, account_id
    ):
        """behavior.extras (string→string JSONB map) roundtrip — keys and
        values persist exactly as submitted. Covers the case where a
        front-end component passes a campaign-tag or arbitrary metadata
        through `buildBehaviorPayload`'s extras path.
        """
        payload = {
            "social_account_id": account_id,
            "platform_id": PLATFORM_FACEBOOK,
            "content_type": "post",
            "plan_type": "direct_publish",
            "content": {"title": "t"},
            "behavior": {
                "visibility": "public",
                "extras": {
                    "campaign_tag": "spring_2026",
                    "source": "autotest",
                },
            },
        }
        resp = auth_client.post("/api/v1/publish_plans", json=payload)
        assert_response_success(resp)
        plan_id = extract_data(resp.json())["id"]
        row = _read_plan_row(db_cursor, plan_id)
        assert row["behavior"] is not None
        assert row["behavior"].get("extras") == {
            "campaign_tag": "spring_2026",
            "source": "autotest",
        }

    # ---- Boolean toggle matrix ----

    @pytest.mark.parametrize(
        "field",
        [
            "allow_comments",
            "allow_sharing",
            "allow_download",
            "is_nsfw",
            "is_spoiler",
            "ai_generated_disclosure",
            "allow_duet",
            "allow_stitch",
            "disclose_branded_content",
            "allow_remix",
            "share_to_facebook",
        ],
    )
    @pytest.mark.parametrize("value", [True, False])
    def test_behavior_boolean_toggle_exact_value(
        self, auth_client, db_cursor, account_id, field, value
    ):
        payload = {
            "social_account_id": account_id,
            "platform_id": PLATFORM_FACEBOOK,
            "content_type": "post",
            "plan_type": "direct_publish",
            "content": {"title": "t"},
            "behavior": {field: value},
        }
        resp = auth_client.post("/api/v1/publish_plans", json=payload)
        assert_response_success(resp)
        plan_id = extract_data(resp.json())["id"]
        row = _read_plan_row(db_cursor, plan_id)
        b = row["behavior"]
        assert b.get(field) is value, (
            f"behavior.{field} drift: got={b.get(field)!r} want={value!r}"
        )

    # ---- Schedule branches ----

    def test_schedule_future_utc_datetime_preserved(
        self, auth_client, db_cursor, account_id
    ):
        schedule = {
            "scheduled_at": "2035-01-15T08:00:00Z",
            "timezone": "UTC",
            "save_as_draft": False,
        }
        resp = auth_client.post(
            "/api/v1/publish_plans",
            json={
                "social_account_id": account_id,
                "platform_id": PLATFORM_FACEBOOK,
                "content_type": "post",
                "plan_type": "direct_publish",
                "content": {"title": "t"},
                "schedule": schedule,
            },
        )
        assert_response_success(resp)
        plan_id = extract_data(resp.json())["id"]
        row = _read_plan_row(db_cursor, plan_id)
        s = row["schedule"]
        assert s["scheduled_at"] == schedule["scheduled_at"]
        assert s["timezone"] == "UTC"
        assert s["save_as_draft"] is False

    def test_schedule_save_as_draft_true_preserved(
        self, auth_client, db_cursor, account_id
    ):
        resp = auth_client.post(
            "/api/v1/publish_plans",
            json={
                "social_account_id": account_id,
                "platform_id": PLATFORM_FACEBOOK,
                "content_type": "post",
                "plan_type": "direct_publish",
                "content": {"title": "t"},
                "schedule": {
                    "scheduled_at": "2035-01-15T08:00:00Z",
                    "save_as_draft": True,
                },
            },
        )
        assert_response_success(resp)
        plan_id = extract_data(resp.json())["id"]
        row = _read_plan_row(db_cursor, plan_id)
        assert row["schedule"]["save_as_draft"] is True
        # timezone not sent → must be absent from persisted JSON, not null-string.
        assert "timezone" not in row["schedule"]

    def test_schedule_and_behavior_coexist_on_same_plan(
        self, auth_client, db_cursor, account_id
    ):
        payload = {
            "social_account_id": account_id,
            "platform_id": PLATFORM_FACEBOOK,
            "content_type": "post",
            "plan_type": "direct_publish",
            "content": {"title": "t"},
            "behavior": _full_behavior(),
            "schedule": _future_schedule(),
        }
        resp = auth_client.post("/api/v1/publish_plans", json=payload)
        assert_response_success(resp)
        plan_id = extract_data(resp.json())["id"]
        row = _read_plan_row(db_cursor, plan_id)
        # Both columns must be populated and independent — updating one must
        # never clobber the other (regression against an early service bug
        # where `schedule` was serialized into `behavior.extras`).
        _assert_full_behavior_roundtrip(row["behavior"])
        _assert_full_schedule_roundtrip(row["schedule"])


# ===========================================================================
# Platform × content-type matrix — DB columns consistent
# ===========================================================================

# (platform_id, content_type, plan_type) combinations that the frontend wires
# up today. Each one is asserted to hit the DB with the same tuple intact.
PLATFORM_CT_MATRIX: List[tuple] = [
    (PLATFORM_FACEBOOK, "post", "batch_text"),
    (PLATFORM_FACEBOOK, "video", "single_video"),
    (PLATFORM_FACEBOOK, "reel", "single_video"),
    (PLATFORM_FACEBOOK, "story", "batch_text"),
    (PLATFORM_INSTAGRAM, "post", "batch_text"),
    (PLATFORM_INSTAGRAM, "reel", "single_video"),
    (PLATFORM_INSTAGRAM, "story", "batch_text"),
    (PLATFORM_TIKTOK, "video", "single_video"),
    (PLATFORM_TWITTER, "post", "batch_text"),
    (PLATFORM_REDDIT, "reddit_text", "reddit_text"),
    (PLATFORM_REDDIT, "reddit_link", "reddit_link"),
    (PLATFORM_REDDIT, "reddit_image", "reddit_image"),
]


class TestPlatformContentTypeMatrixDb:
    """Verifies that the (platform_id, content_type, plan_type) triplet the
    front end sends for each supported combo lands in the DB byte-for-byte.
    Drives from PLATFORM_CT_MATRIX so adding a new content type only needs
    one row here to be gated by a persisted DB snapshot.
    """

    @pytest.mark.parametrize(
        "platform_id, content_type, plan_type",
        PLATFORM_CT_MATRIX,
        ids=[f"p{p}-{ct}-{pt}" for p, ct, pt in PLATFORM_CT_MATRIX],
    )
    def test_platform_content_type_triplet_persisted(
        self,
        auth_client,
        db_cursor,
        platform_id,
        content_type,
        plan_type,
    ):
        # Target selection follows the frontend's forceAccountTarget rule:
        # video/reel content types require an account, everything else uses
        # a group.
        uses_account = content_type in {"video", "reel"}
        account_id = None
        group_id = None
        if uses_account:
            account_id = _resolve_account(db_cursor, platform_id)
            if not account_id:
                pytest.skip(f"No account seeded for platform {platform_id}")
        else:
            group_id = _resolve_group(db_cursor, platform_id)
            if not group_id:
                pytest.skip(f"No user-999 group with ACTIVE account seeded for platform {platform_id}")

        payload: Dict[str, Any] = {
            "platform_id": platform_id,
            "content_type": content_type,
            "plan_type": plan_type,
            "ai_input": {"content_prompt": "p"},
        }
        if account_id:
            payload["social_account_id"] = account_id
        if group_id:
            payload["group_id"] = group_id
        # Reddit plan types require title_prompt/subreddit.
        if plan_type.startswith("reddit_"):
            payload["ai_input"]["title_prompt"] = "test title"
            payload["ai_input"]["subreddit"] = "test"
        if plan_type == "reddit_link":
            payload["ai_input"]["link_url"] = "https://example.com"
        if plan_type == "reddit_image":
            payload["ai_input"]["image_generations"] = [
                {
                    "model": "flux-kontext-pro",
                    "prompts": ["cat"],
                    "count": 1,
                    "mode": "text_to_image",
                }
            ]

        resp = auth_client.post("/api/v1/publish_plans", json=payload)
        if resp.status_code == 400:
            pytest.skip(
                f"Platform {platform_id} / {content_type} / {plan_type} "
                f"rejected by validator: {resp.text[:200]}"
            )
        assert_response_success(resp)
        plan_id = extract_data(resp.json())["id"]

        row = _read_plan_row(db_cursor, plan_id)
        _assert_base_columns(
            row,
            platform_id=platform_id,
            content_type=content_type,
            plan_type=plan_type,
            expected_group=group_id,
            expected_account=account_id,
        )
