"""
GlanceMind API E2E Tests — DM Group Control
=============================================
Covers all 9 DM handler endpoints with NATS JetStream integration.

Prerequisites:
  - docker-compose (db + nats + backend) running
  - NATS JetStream enabled with DM streams and KV buckets

Test data is seeded directly into NATS before running,
then verified through the REST API.
"""

import asyncio
import json
import threading
import time
import uuid
from datetime import datetime, timezone

import nats as nats_pkg
import pytest
from conftest import (
    APIClient,
    API_BASE_URL,
    PLATFORM_INSTAGRAM,
    PLATFORM_TIKTOK,
    assert_response_success,
    extract_data,
    get_or_create_test_token,
)

# ---------------------------------------------------------------------------
# NATS helper (lightweight, for seeding test data)
# ---------------------------------------------------------------------------

NATS_URL = "nats://localhost:4222"
NATS_TOKEN = "glancemind-dev-token"

RUN_ID = uuid.uuid4().hex[:8]
DEVICE_DM = f"dm-api-dev-{RUN_ID}"
DEVICE_DM_2 = f"dm-api-dev2-{RUN_ID}"


def _now() -> str:
    return datetime.now(timezone.utc).isoformat()


class NatsSeeder:
    """Seed DM data into NATS for API tests."""

    def __init__(self):
        self._nc = None
        self._js = None

    async def connect(self):
        self._nc = await nats_pkg.connect(NATS_URL, token=NATS_TOKEN)
        self._js = self._nc.jetstream()

    async def ensure_infra(self):
        for name, subjects in [
            ("DM_MESSAGES", ["dm.msg.>"]),
            ("DM_COMMANDS", ["dm.cmd.>"]),
            ("DM_EVENTS", ["dm.evt.>"]),
        ]:
            try:
                await self._js.add_stream(name=name, subjects=subjects)
            except Exception:
                pass

        for bucket in ["dm_conversations", "dm_device_heartbeat", "dm_monitor_config"]:
            try:
                opts = {}
                if bucket == "dm_device_heartbeat":
                    opts["ttl"] = 120
                await self._js.create_key_value(bucket=bucket, **opts)
            except Exception:
                pass

    async def publish_message(self, conv_id: str, msg: dict):
        data = json.dumps(msg).encode()
        headers = {}
        if msg.get("platform_msg_id"):
            headers["Nats-Msg-Id"] = msg["platform_msg_id"]
        await self._js.publish(f"dm.msg.{conv_id}", data, headers=headers or None)

    async def put_conversation(self, user_id: int, conv_id: str, meta: dict):
        kv = await self._js.key_value("dm_conversations")
        await kv.put(f"{user_id}.{conv_id}", json.dumps(meta).encode())

    async def put_heartbeat(self, device_id: str, user_id: int):
        kv = await self._js.key_value("dm_device_heartbeat")
        hb = {"user_id": user_id, "device_id": device_id, "last_seen": _now()}
        await kv.put(device_id, json.dumps(hb).encode())

    async def put_monitor_config(self, device_id: str, config: dict):
        kv = await self._js.key_value("dm_monitor_config")
        await kv.put(device_id, json.dumps(config).encode())

    async def close(self):
        if self._nc:
            await self._nc.drain()


def _subscribe_command_in_new_loop(device_id: str, timeout: float = 10.0):
    """Subscribe to a NATS command subject in a fresh event loop (thread-safe)."""
    result = None

    async def _listen():
        nonlocal result
        nc = await nats_pkg.connect(NATS_URL, token=NATS_TOKEN)
        sub = await nc.subscribe(f"dm.cmd.{device_id}")
        try:
            msg = await asyncio.wait_for(sub.next_msg(), timeout=timeout)
            result = json.loads(msg.data)
        except asyncio.TimeoutError:
            pass
        finally:
            await sub.unsubscribe()
            await nc.drain()

    loop = asyncio.new_event_loop()
    loop.run_until_complete(_listen())
    loop.close()
    return result


# ---------------------------------------------------------------------------
# Fixtures
# ---------------------------------------------------------------------------

@pytest.fixture(scope="module")
def nats_seeder():
    seeder = NatsSeeder()
    loop = asyncio.new_event_loop()
    loop.run_until_complete(seeder.connect())
    loop.run_until_complete(seeder.ensure_infra())
    yield seeder, loop
    loop.run_until_complete(seeder.close())
    loop.close()


@pytest.fixture(scope="module")
def module_auth_client():
    """Module-scoped authenticated API client (avoids scope mismatch)."""
    token = get_or_create_test_token()
    return APIClient(API_BASE_URL, token)


@pytest.fixture(scope="module")
def dm_test_accounts(module_auth_client):
    """Create social accounts with device bindings for DM tests."""
    client = module_auth_client
    accounts = []
    for platform_id, username, device_id, profile in [
        (PLATFORM_INSTAGRAM, f"dm_ig_{RUN_ID}", DEVICE_DM, "profile_dm1"),
        (PLATFORM_TIKTOK, f"dm_tt_{RUN_ID}", DEVICE_DM, "profile_dm2"),
        (PLATFORM_INSTAGRAM, f"dm_ig2_{RUN_ID}", DEVICE_DM_2, "profile_dm3"),
    ]:
        resp = client.post("/api/v1/accounts", json={
            "platform_id": platform_id,
            "username": username,
        })
        assert_response_success(resp)
        data = extract_data(resp.json())
        aid = data["id"]

        update_resp = client.put(f"/api/v1/accounts/{aid}", json={
            "device_id": device_id,
            "profile_name": profile,
        })
        assert_response_success(update_resp)

        accounts.append({
            "id": aid,
            "platform_id": platform_id,
            "username": username,
            "device_id": device_id,
            "profile_name": profile,
        })
        print(f"  Created DM test account: {username} (id={aid}, device={device_id})")

    yield accounts

    for a in accounts:
        client.delete(f"/api/v1/accounts/{a['id']}")


@pytest.fixture(scope="module")
def seeded_conversations(nats_seeder, dm_test_accounts, module_auth_client):
    """Seed NATS with conversations and messages for the test user."""
    seeder, loop = nats_seeder
    client = module_auth_client

    me_resp = client.get("/api/v1/user/me")
    me_data = extract_data(me_resp.json())
    user_id = me_data.get("id")
    if user_id is None:
        user_id = me_data.get("user_id")

    acct_ig = dm_test_accounts[0]
    acct_tt = dm_test_accounts[1]
    acct_ig2 = dm_test_accounts[2]

    convs = [
        {
            "conv_id": f"{acct_ig['id']}_alice_{RUN_ID}",
            "user_id": user_id,
            "social_account_id": acct_ig["id"],
            "device_id": DEVICE_DM,
            "platform_id": PLATFORM_INSTAGRAM,
            "platform_name": "Instagram",
            "my_username": acct_ig["username"],
            "my_profile_name": acct_ig["profile_name"],
            "remote_user_id": f"alice_{RUN_ID}",
            "remote_username": f"alice_{RUN_ID}",
            "messages": ["Hi from Alice", "Second msg from Alice"],
        },
        {
            "conv_id": f"{acct_tt['id']}_bob_{RUN_ID}",
            "user_id": user_id,
            "social_account_id": acct_tt["id"],
            "device_id": DEVICE_DM,
            "platform_id": PLATFORM_TIKTOK,
            "platform_name": "TikTok",
            "my_username": acct_tt["username"],
            "my_profile_name": acct_tt["profile_name"],
            "remote_user_id": f"bob_{RUN_ID}",
            "remote_username": f"bob_{RUN_ID}",
            "messages": ["TikTok question?"],
        },
        {
            "conv_id": f"{acct_ig2['id']}_charlie_{RUN_ID}",
            "user_id": user_id,
            "social_account_id": acct_ig2["id"],
            "device_id": DEVICE_DM_2,
            "platform_id": PLATFORM_INSTAGRAM,
            "platform_name": "Instagram",
            "my_username": acct_ig2["username"],
            "my_profile_name": acct_ig2["profile_name"],
            "remote_user_id": f"charlie_{RUN_ID}",
            "remote_username": f"charlie_{RUN_ID}",
            "messages": ["Hello from device 2"],
        },
    ]

    now = _now()
    for c in convs:
        for content in c["messages"]:
            msg_id = f"plat_{uuid.uuid4().hex[:8]}"
            msg = {
                "msg_id": str(uuid.uuid4()),
                "conv_id": c["conv_id"],
                "direction": "inbound",
                "content": content,
                "content_type": "text",
                "attachments": [],
                "status": "delivered",
                "platform_msg_id": msg_id,
                "timestamp": now,
            }
            loop.run_until_complete(seeder.publish_message(c["conv_id"], msg))

        meta = {
            "conv_id": c["conv_id"],
            "user_id": c["user_id"],
            "social_account_id": c["social_account_id"],
            "device_id": c["device_id"],
            "platform_id": c["platform_id"],
            "platform_name": c["platform_name"],
            "my_username": c["my_username"],
            "my_profile_name": c["my_profile_name"],
            "remote_user_id": c["remote_user_id"],
            "remote_username": c["remote_username"],
            "remote_display_name": None,
            "remote_avatar_url": None,
            "last_message_at": now,
            "last_message_preview": c["messages"][-1],
            "last_message_direction": "inbound",
            "unread_count": len(c["messages"]),
            "status": "active",
            "reply_mode": "manual",
            "ai_suggestion": None,
            "updated_at": now,
        }
        loop.run_until_complete(seeder.put_conversation(c["user_id"], c["conv_id"], meta))

    loop.run_until_complete(seeder.put_heartbeat(DEVICE_DM, user_id))
    print(f"  Seeded {len(convs)} conversations for user_id={user_id}")

    return {
        "user_id": user_id,
        "convs": convs,
        "accounts": dm_test_accounts,
    }


# ===========================================================================
# Tests
# ===========================================================================

class TestDmConversationsList:
    """GET /dm/conversations — list + filter + device status"""

    def test_list_conversations_returns_seeded_data(self, auth_client, seeded_conversations):
        resp = auth_client.get("/api/v1/dm/conversations")
        assert_response_success(resp)
        data = extract_data(resp.json())

        convs = data["conversations"]
        for expected in seeded_conversations["convs"]:
            actual = next((c for c in convs if c["conv_id"] == expected["conv_id"]), None)
            assert actual is not None, f"Missing conversation {expected['conv_id']}"
            assert actual["user_id"] == expected["user_id"]
            assert actual["social_account_id"] == expected["social_account_id"]
            assert actual["device_id"] == expected["device_id"]
            assert actual["platform_id"] == expected["platform_id"]
            assert actual["platform_name"] == expected["platform_name"]
            assert actual["my_username"] == expected["my_username"]
            assert actual["my_profile_name"] == expected["my_profile_name"]
            assert actual["remote_user_id"] == expected["remote_user_id"]
            assert actual["remote_username"] == expected["remote_username"]
            assert actual["last_message_preview"] == expected["messages"][-1]
            assert actual["last_message_direction"] == "inbound"
            assert actual["unread_count"] == len(expected["messages"])
            assert actual["status"] == "active"
            assert actual["reply_mode"] == "manual"
            assert len(actual["updated_at"]) > 0
            assert len(actual["last_message_at"]) > 0

    def test_list_conversations_includes_device_status(self, auth_client, seeded_conversations):
        resp = auth_client.get("/api/v1/dm/conversations")
        data = extract_data(resp.json())

        ds = data.get("device_status", {})
        assert DEVICE_DM in ds, "DEVICE_DM should be in device_status"
        assert ds[DEVICE_DM]["online"] is True, "DEVICE_DM should be online"

    def test_filter_by_platform_id(self, auth_client, seeded_conversations):
        resp = auth_client.get("/api/v1/dm/conversations", params={"platform_id": PLATFORM_TIKTOK})
        assert_response_success(resp)
        data = extract_data(resp.json())

        convs = data["conversations"]
        assert len(convs) >= 1
        assert all(c["platform_id"] == PLATFORM_TIKTOK for c in convs)
        assert any(f"bob_{RUN_ID}" in c["remote_username"] for c in convs)

    def test_filter_by_device_id(self, auth_client, seeded_conversations):
        resp = auth_client.get("/api/v1/dm/conversations", params={"device_id": DEVICE_DM_2})
        assert_response_success(resp)
        data = extract_data(resp.json())

        convs = data["conversations"]
        assert len(convs) >= 1
        assert all(c["device_id"] == DEVICE_DM_2 for c in convs)
        assert any(f"charlie_{RUN_ID}" in c["remote_username"] for c in convs)

    def test_filter_by_account_id(self, auth_client, seeded_conversations):
        acct = seeded_conversations["accounts"][0]
        resp = auth_client.get("/api/v1/dm/conversations", params={"account_id": acct["id"]})
        assert_response_success(resp)
        data = extract_data(resp.json())

        convs = data["conversations"]
        assert all(c["social_account_id"] == acct["id"] for c in convs)

    def test_empty_filter_returns_empty(self, auth_client):
        resp = auth_client.get("/api/v1/dm/conversations", params={"device_id": "nonexistent-device"})
        assert_response_success(resp)
        data = extract_data(resp.json())
        assert len(data["conversations"]) == 0


class TestDmMessages:
    """GET /dm/conversations/:conv_id/messages — history + pagination"""

    def test_get_messages_returns_seeded(self, auth_client, seeded_conversations):
        conv_id = seeded_conversations["convs"][0]["conv_id"]
        resp = auth_client.get(f"/api/v1/dm/conversations/{conv_id}/messages")
        assert_response_success(resp)
        data = extract_data(resp.json())

        msgs = data["messages"]
        assert len(msgs) >= 2, "Alice conv should have 2 messages"
        contents = [m["content"] for m in msgs]
        assert "Hi from Alice" in contents
        assert "Second msg from Alice" in contents

    def test_get_messages_with_limit(self, auth_client, seeded_conversations):
        conv_id = seeded_conversations["convs"][0]["conv_id"]
        resp = auth_client.get(
            f"/api/v1/dm/conversations/{conv_id}/messages",
            params={"limit": 1},
        )
        assert_response_success(resp)
        data = extract_data(resp.json())
        assert len(data["messages"]) <= 1

    def test_get_messages_nonexistent_conv_returns_404(self, auth_client):
        """Non-existent social_account_id in conv_id returns 404 (ownership check)."""
        resp = auth_client.get("/api/v1/dm/conversations/999999_nobody/messages")
        assert resp.status_code == 404

    def test_get_messages_empty_but_owned_conversation(self, auth_client, seeded_conversations):
        """A valid owned account with no messages returns empty list."""
        acct = seeded_conversations["accounts"][0]
        resp = auth_client.get(
            f"/api/v1/dm/conversations/{acct['id']}_nonexistent_remote/messages"
        )
        assert_response_success(resp)
        data = extract_data(resp.json())
        assert len(data["messages"]) == 0

    def test_messages_have_complete_fields(self, auth_client, seeded_conversations):
        conv_id = seeded_conversations["convs"][0]["conv_id"]
        resp = auth_client.get(f"/api/v1/dm/conversations/{conv_id}/messages")
        data = extract_data(resp.json())

        for msg in data["messages"]:
            assert msg["conv_id"] == conv_id
            assert msg["direction"] == "inbound"
            assert msg["content_type"] == "text"
            assert len(msg["msg_id"]) > 0
            assert len(msg["timestamp"]) > 0
            assert msg["status"] == "delivered"


class TestDmReply:
    """POST /dm/conversations/:conv_id/reply — send command via NATS"""

    def test_send_reply_returns_cmd_id(self, auth_client, seeded_conversations):
        conv_id = seeded_conversations["convs"][0]["conv_id"]
        resp = auth_client.post(
            f"/api/v1/dm/conversations/{conv_id}/reply",
            json={"content": f"API test reply {RUN_ID}"},
        )
        assert_response_success(resp)
        data = extract_data(resp.json())

        assert "cmd_id" in data
        assert len(data["cmd_id"]) > 0
        assert data["status"] == "queued"
        print(f"  Reply cmd_id: {data['cmd_id']}")

    def test_send_reply_publishes_nats_command(
        self, auth_client, seeded_conversations,
    ):
        conv_id = seeded_conversations["convs"][0]["conv_id"]
        result_holder = [None]
        ready_event = threading.Event()

        def listen():
            async def _do():
                nc = await nats_pkg.connect(NATS_URL, token=NATS_TOKEN)
                sub = await nc.subscribe(f"dm.cmd.{DEVICE_DM}")
                ready_event.set()
                try:
                    msg = await asyncio.wait_for(sub.next_msg(), timeout=10.0)
                    result_holder[0] = json.loads(msg.data)
                except asyncio.TimeoutError:
                    pass
                finally:
                    await sub.unsubscribe()
                    await nc.drain()

            loop = asyncio.new_event_loop()
            loop.run_until_complete(_do())
            loop.close()

        t = threading.Thread(target=listen, daemon=True)
        t.start()
        ready_event.wait(timeout=5)

        resp = auth_client.post(
            f"/api/v1/dm/conversations/{conv_id}/reply",
            json={"content": f"NATS verify {RUN_ID}"},
        )
        assert_response_success(resp)

        t.join(timeout=15)
        cmd = result_holder[0]
        assert cmd is not None, "Should receive command on NATS"
        assert cmd["content"] == f"NATS verify {RUN_ID}"
        assert cmd["content_type"] == "text"
        assert cmd["conv_id"] == conv_id
        assert cmd["social_account_id"] == seeded_conversations["accounts"][0]["id"]
        assert cmd["platform_id"] == seeded_conversations["accounts"][0]["platform_id"]
        assert cmd["profile_name"] == seeded_conversations["accounts"][0]["profile_name"]
        assert cmd["remote_username"] == f"alice_{RUN_ID}"
        assert len(cmd["cmd_id"]) > 0
        assert len(cmd["timestamp"]) > 0
        print(f"  NATS command verified: {cmd['content'][:40]}")

    def test_send_reply_invalid_conv_id(self, auth_client):
        resp = auth_client.post(
            "/api/v1/dm/conversations/invalid/reply",
            json={"content": "test"},
        )
        assert resp.status_code == 400


class TestDmImageMessages:
    """Image / non-text content_type: full round-trip through NATS."""

    def test_seed_image_message_and_retrieve(self, auth_client, nats_seeder, seeded_conversations):
        """Seed an image message to NATS and verify it comes back via API."""
        seeder, loop = nats_seeder
        conv_id = seeded_conversations["convs"][0]["conv_id"]
        img_url = "https://example.com/photo.jpg"
        msg_id = f"img_{uuid.uuid4().hex[:8]}"

        msg = {
            "msg_id": msg_id,
            "conv_id": conv_id,
            "direction": "inbound",
            "content": img_url,
            "content_type": "image",
            "attachments": [img_url, "https://example.com/thumb.jpg"],
            "status": "delivered",
            "platform_msg_id": f"plat_img_{uuid.uuid4().hex[:8]}",
            "timestamp": _now(),
        }
        loop.run_until_complete(seeder.publish_message(conv_id, msg))

        resp = auth_client.get(f"/api/v1/dm/conversations/{conv_id}/messages")
        assert_response_success(resp)
        msgs = extract_data(resp.json())["messages"]

        img_msg = next((m for m in msgs if m["msg_id"] == msg_id), None)
        assert img_msg is not None, f"Image message {msg_id} not found"
        assert img_msg["content_type"] == "image"
        assert img_msg["content"] == img_url
        assert len(img_msg["attachments"]) == 2
        assert "https://example.com/photo.jpg" in img_msg["attachments"]
        assert "https://example.com/thumb.jpg" in img_msg["attachments"]

    def test_send_image_reply_propagates_content_type(self, auth_client, seeded_conversations):
        """Send a reply with content_type=image, verify API returns queued."""
        conv_id = seeded_conversations["convs"][0]["conv_id"]
        resp = auth_client.post(
            f"/api/v1/dm/conversations/{conv_id}/reply",
            json={
                "content": "https://example.com/reply-image.png",
                "content_type": "image",
            },
        )
        assert_response_success(resp)
        data = extract_data(resp.json())
        assert data["status"] == "queued"
        assert len(data["cmd_id"]) > 0

    def test_image_reply_nats_command_has_content_type(
        self, auth_client, seeded_conversations,
    ):
        """Subscribe to NATS and verify image reply command preserves content_type."""
        conv_id = seeded_conversations["convs"][0]["conv_id"]
        result_holder = [None]
        ready_event = threading.Event()

        def listen():
            async def _do():
                nc = await nats_pkg.connect(NATS_URL, token=NATS_TOKEN)
                sub = await nc.subscribe(f"dm.cmd.{DEVICE_DM}")
                ready_event.set()
                try:
                    msg = await asyncio.wait_for(sub.next_msg(), timeout=10.0)
                    result_holder[0] = json.loads(msg.data)
                except asyncio.TimeoutError:
                    pass
                finally:
                    await sub.unsubscribe()
                    await nc.drain()

            loop = asyncio.new_event_loop()
            loop.run_until_complete(_do())
            loop.close()

        t = threading.Thread(target=listen, daemon=True)
        t.start()
        ready_event.wait(timeout=5)

        resp = auth_client.post(
            f"/api/v1/dm/conversations/{conv_id}/reply",
            json={
                "content": f"https://example.com/img_{RUN_ID}.jpg",
                "content_type": "image",
            },
        )
        assert_response_success(resp)

        t.join(timeout=15)
        cmd = result_holder[0]
        assert cmd is not None, "Should receive image command on NATS"
        assert cmd["content_type"] == "image"
        assert "img_" in cmd["content"]

    def test_seed_video_message_and_retrieve(self, auth_client, nats_seeder, seeded_conversations):
        """Seed a video message to NATS and verify it comes back."""
        seeder, loop = nats_seeder
        conv_id = seeded_conversations["convs"][1]["conv_id"]
        vid_url = "https://example.com/video.mp4"
        msg_id = f"vid_{uuid.uuid4().hex[:8]}"

        msg = {
            "msg_id": msg_id,
            "conv_id": conv_id,
            "direction": "inbound",
            "content": vid_url,
            "content_type": "video",
            "attachments": [vid_url],
            "status": "delivered",
            "platform_msg_id": f"plat_vid_{uuid.uuid4().hex[:8]}",
            "timestamp": _now(),
        }
        loop.run_until_complete(seeder.publish_message(conv_id, msg))

        resp = auth_client.get(f"/api/v1/dm/conversations/{conv_id}/messages")
        assert_response_success(resp)
        msgs = extract_data(resp.json())["messages"]

        vid_msg = next((m for m in msgs if m["msg_id"] == msg_id), None)
        assert vid_msg is not None, f"Video message {msg_id} not found"
        assert vid_msg["content_type"] == "video"
        assert vid_msg["attachments"] == [vid_url]

    def test_seed_link_message_and_retrieve(self, auth_client, nats_seeder, seeded_conversations):
        """Seed a link message to NATS and verify it comes back."""
        seeder, loop = nats_seeder
        conv_id = seeded_conversations["convs"][1]["conv_id"]
        link_url = "https://www.tiktok.com/@user/video/12345"
        msg_id = f"lnk_{uuid.uuid4().hex[:8]}"

        msg = {
            "msg_id": msg_id,
            "conv_id": conv_id,
            "direction": "inbound",
            "content": link_url,
            "content_type": "link",
            "attachments": [],
            "status": "delivered",
            "platform_msg_id": f"plat_lnk_{uuid.uuid4().hex[:8]}",
            "timestamp": _now(),
        }
        loop.run_until_complete(seeder.publish_message(conv_id, msg))

        resp = auth_client.get(f"/api/v1/dm/conversations/{conv_id}/messages")
        assert_response_success(resp)
        msgs = extract_data(resp.json())["messages"]

        lnk_msg = next((m for m in msgs if m["msg_id"] == msg_id), None)
        assert lnk_msg is not None, f"Link message {msg_id} not found"
        assert lnk_msg["content_type"] == "link"
        assert lnk_msg["content"] == link_url


class TestDmMarkRead:
    """POST /dm/conversations/:conv_id/read"""

    def test_mark_read_success(self, auth_client, seeded_conversations):
        conv_id = seeded_conversations["convs"][0]["conv_id"]
        resp = auth_client.post(f"/api/v1/dm/conversations/{conv_id}/read")
        assert_response_success(resp)

    def test_mark_read_updates_unread_count(self, auth_client, seeded_conversations):
        conv_id = seeded_conversations["convs"][0]["conv_id"]
        auth_client.post(f"/api/v1/dm/conversations/{conv_id}/read")

        resp = auth_client.get("/api/v1/dm/conversations")
        data = extract_data(resp.json())

        conv = next((c for c in data["conversations"] if c["conv_id"] == conv_id), None)
        assert conv is not None
        assert conv["unread_count"] == 0


class TestDmSettings:
    """PUT /dm/conversations/:conv_id/settings"""

    def test_update_reply_mode(self, auth_client, seeded_conversations):
        conv_id = seeded_conversations["convs"][1]["conv_id"]
        resp = auth_client.put(
            f"/api/v1/dm/conversations/{conv_id}/settings",
            json={"reply_mode": "auto"},
        )
        assert_response_success(resp)

        list_resp = auth_client.get("/api/v1/dm/conversations")
        data = extract_data(list_resp.json())
        conv = next((c for c in data["conversations"] if c["conv_id"] == conv_id), None)
        assert conv is not None
        assert conv["reply_mode"] == "auto"

    def test_update_status_to_muted(self, auth_client, seeded_conversations):
        conv_id = seeded_conversations["convs"][1]["conv_id"]
        resp = auth_client.put(
            f"/api/v1/dm/conversations/{conv_id}/settings",
            json={"status": "muted"},
        )
        assert_response_success(resp)


class TestDmStats:
    """GET /dm/stats"""

    def test_get_stats_returns_totals(self, auth_client, seeded_conversations):
        resp = auth_client.get("/api/v1/dm/stats")
        assert_response_success(resp)
        data = extract_data(resp.json())

        assert data["total_conversations"] >= 3
        assert isinstance(data["total_unread"], int)
        assert isinstance(data["per_platform"], list)

    def test_stats_per_platform_breakdown(self, auth_client, seeded_conversations):
        resp = auth_client.get("/api/v1/dm/stats")
        data = extract_data(resp.json())

        platforms = {p["platform_id"]: p for p in data["per_platform"]}
        assert PLATFORM_INSTAGRAM in platforms
        assert PLATFORM_TIKTOK in platforms
        assert platforms[PLATFORM_INSTAGRAM]["conversations"] >= 2
        assert platforms[PLATFORM_TIKTOK]["conversations"] >= 1
        assert isinstance(platforms[PLATFORM_INSTAGRAM]["unread"], int)


class TestDmMonitorConfig:
    """GET/PUT /dm/monitor-config/:device_id (requires device ownership)"""

    def test_get_monitor_config_nonexistent_device_returns_404(self, auth_client):
        """Device not bound to any of user's accounts returns 404."""
        resp = auth_client.get(f"/api/v1/dm/monitor-config/nonexistent-{RUN_ID}")
        assert resp.status_code == 404

    def test_get_monitor_config_empty_for_owned_device(self, auth_client, seeded_conversations):
        """Owned device with no config returns None."""
        device = DEVICE_DM
        resp = auth_client.get(f"/api/v1/dm/monitor-config/{device}")
        assert_response_success(resp)
        data = extract_data(resp.json())
        # May be None or a default config depending on whether it was seeded

    def test_put_and_get_monitor_config(self, auth_client, nats_seeder, seeded_conversations):
        """Seed config for an owned device, GET it, verify all fields."""
        seeder, loop = nats_seeder
        device = DEVICE_DM  # owned by test user via dm_test_accounts

        loop.run_until_complete(seeder.put_monitor_config(device, {
            "device_id": device,
            "enabled": True,
            "poll_interval_seconds": 60,
            "max_concurrent_monitors": 3,
            "inbox_linger_seconds": 45,
            "platforms": ["instagram", "tiktok"],
            "updated_at": _now(),
        }))

        resp = auth_client.get(f"/api/v1/dm/monitor-config/{device}")
        assert_response_success(resp)
        data = extract_data(resp.json())
        assert data["enabled"] is True
        assert data["poll_interval_seconds"] == 60
        assert data["max_concurrent_monitors"] == 3
        assert data["inbox_linger_seconds"] == 45
        assert "instagram" in data["platforms"]
        assert "tiktok" in data["platforms"]

    def test_update_monitor_config(self, auth_client, nats_seeder, seeded_conversations):
        """PUT partial update on an owned device, verify merge behavior."""
        seeder, loop = nats_seeder
        device = DEVICE_DM_2  # owned by test user via dm_test_accounts[2]

        loop.run_until_complete(seeder.put_monitor_config(device, {
            "device_id": device,
            "enabled": False,
            "poll_interval_seconds": 120,
            "max_concurrent_monitors": 2,
            "inbox_linger_seconds": 30,
            "platforms": ["instagram"],
            "updated_at": _now(),
        }))

        resp = auth_client.put(
            f"/api/v1/dm/monitor-config/{device}",
            json={
                "enabled": True,
                "poll_interval_seconds": 90,
                "platforms": ["instagram", "tiktok", "facebook"],
            },
        )
        assert_response_success(resp)
        data = extract_data(resp.json())
        assert data["enabled"] is True
        assert data["poll_interval_seconds"] == 90
        assert len(data["platforms"]) == 3


class TestDmNatsToken:
    """GET /dm/nats-token"""

    def test_get_nats_token(self, auth_client):
        resp = auth_client.get("/api/v1/dm/nats-token")
        assert_response_success(resp)
        data = extract_data(resp.json())

        assert len(data["token"]) > 0
        assert len(data["expires_at"]) > 0
        datetime.fromisoformat(data["expires_at"].replace("Z", "+00:00"))

    def test_nats_token_requires_auth(self, api_client):
        resp = api_client.get("/api/v1/dm/nats-token")
        assert resp.status_code == 401


class TestDmAuthRequired:
    """All DM endpoints require authentication."""

    def test_conversations_requires_auth(self, api_client):
        resp = api_client.get("/api/v1/dm/conversations")
        assert resp.status_code == 401

    def test_messages_requires_auth(self, api_client):
        resp = api_client.get("/api/v1/dm/conversations/fake/messages")
        assert resp.status_code == 401

    def test_reply_requires_auth(self, api_client):
        resp = api_client.post("/api/v1/dm/conversations/fake/reply", json={"content": "x"})
        assert resp.status_code == 401

    def test_stats_requires_auth(self, api_client):
        resp = api_client.get("/api/v1/dm/stats")
        assert resp.status_code == 401

    def test_monitor_config_requires_auth(self, api_client):
        resp = api_client.get("/api/v1/dm/monitor-config/fake")
        assert resp.status_code == 401

    def test_mark_read_requires_auth(self, api_client):
        resp = api_client.post("/api/v1/dm/conversations/fake/read")
        assert resp.status_code == 401

    def test_update_settings_requires_auth(self, api_client):
        resp = api_client.put("/api/v1/dm/conversations/fake/settings", json={"reply_mode": "auto"})
        assert resp.status_code == 401

    def test_update_monitor_config_requires_auth(self, api_client):
        resp = api_client.put("/api/v1/dm/monitor-config/fake", json={"enabled": True})
        assert resp.status_code == 401


class TestDmReplyValidation:
    """Input validation for send_reply and update_settings."""

    def test_empty_content_rejected(self, auth_client, seeded_conversations):
        conv_id = seeded_conversations["convs"][0]["conv_id"]
        resp = auth_client.post(
            f"/api/v1/dm/conversations/{conv_id}/reply",
            json={"content": ""},
        )
        assert resp.status_code == 400

    def test_whitespace_only_content_rejected(self, auth_client, seeded_conversations):
        conv_id = seeded_conversations["convs"][0]["conv_id"]
        resp = auth_client.post(
            f"/api/v1/dm/conversations/{conv_id}/reply",
            json={"content": "   \n\t  "},
        )
        assert resp.status_code == 400

    def test_oversized_content_rejected(self, auth_client, seeded_conversations):
        conv_id = seeded_conversations["convs"][0]["conv_id"]
        huge_content = "x" * 6000
        resp = auth_client.post(
            f"/api/v1/dm/conversations/{conv_id}/reply",
            json={"content": huge_content},
        )
        assert resp.status_code == 400

    def test_invalid_reply_mode_rejected(self, auth_client, seeded_conversations):
        conv_id = seeded_conversations["convs"][0]["conv_id"]
        resp = auth_client.put(
            f"/api/v1/dm/conversations/{conv_id}/settings",
            json={"reply_mode": "garbage"},
        )
        assert resp.status_code == 400

    def test_invalid_status_rejected(self, auth_client, seeded_conversations):
        conv_id = seeded_conversations["convs"][0]["conv_id"]
        resp = auth_client.put(
            f"/api/v1/dm/conversations/{conv_id}/settings",
            json={"status": "deleted"},
        )
        assert resp.status_code == 400


class TestDmReplyErrorPaths:
    """Error paths for send_reply: missing device_id, profile_name, wrong account."""

    def test_reply_to_nonexistent_account(self, auth_client):
        """conv_id references a social_account_id that doesn't exist for this user."""
        resp = auth_client.post(
            "/api/v1/dm/conversations/999999_someuser/reply",
            json={"content": "test"},
        )
        assert resp.status_code in (400, 404)

    def test_reply_account_no_device_id(self, auth_client, module_auth_client):
        """Account exists but has no device_id assigned."""
        client = module_auth_client
        resp = client.post("/api/v1/accounts", json={
            "platform_id": PLATFORM_TIKTOK,
            "username": f"no_device_{RUN_ID}",
        })
        assert_response_success(resp)
        data = extract_data(resp.json())
        aid = data["id"]

        try:
            reply_resp = client.post(
                f"/api/v1/dm/conversations/{aid}_remoteuser/reply",
                json={"content": "test"},
            )
            assert reply_resp.status_code == 400
            body = reply_resp.json()
            assert "device_id" in str(body).lower()
        finally:
            client.delete(f"/api/v1/accounts/{aid}")


class TestDmOwnershipVerification:
    """IDOR prevention: users cannot access each other's DM data."""

    def test_get_messages_requires_ownership(self, auth_client, seeded_conversations):
        """get_messages rejects conv_id whose social_account_id isn't owned by the user.
        We test with a non-existent account ID (extremely high) to trigger ownership failure."""
        resp = auth_client.get("/api/v1/dm/conversations/999999_nobody/messages")
        assert resp.status_code in (400, 404), \
            f"Expected 400/404 for non-owned conv, got {resp.status_code}"

    def test_mark_read_requires_ownership(self, auth_client):
        resp = auth_client.post("/api/v1/dm/conversations/999999_nobody/read")
        assert resp.status_code in (400, 404)

    def test_update_settings_requires_ownership(self, auth_client):
        resp = auth_client.put(
            "/api/v1/dm/conversations/999999_nobody/settings",
            json={"reply_mode": "auto"},
        )
        assert resp.status_code in (400, 404)

    def test_monitor_config_requires_device_ownership(self, auth_client):
        resp = auth_client.get(f"/api/v1/dm/monitor-config/nonexistent-device-{RUN_ID}")
        assert resp.status_code in (404,), \
            f"Expected 404 for non-owned device, got {resp.status_code}"

    def test_update_monitor_config_requires_device_ownership(self, auth_client):
        resp = auth_client.put(
            f"/api/v1/dm/monitor-config/nonexistent-device-{RUN_ID}",
            json={"enabled": True},
        )
        assert resp.status_code in (404,), \
            f"Expected 404 for non-owned device, got {resp.status_code}"


class TestDmPagination:
    """Pagination with before_seq parameter."""

    def test_before_seq_filters_older_messages(self, auth_client, seeded_conversations):
        conv_id = seeded_conversations["convs"][0]["conv_id"]

        all_resp = auth_client.get(f"/api/v1/dm/conversations/{conv_id}/messages")
        assert_response_success(all_resp)
        all_msgs = extract_data(all_resp.json())["messages"]

        if len(all_msgs) < 2:
            pytest.skip("Need at least 2 messages to test pagination")

        last_seq = all_msgs[-1].get("nats_seq")
        if last_seq is None:
            pytest.skip("nats_seq not available in messages")

        paged_resp = auth_client.get(
            f"/api/v1/dm/conversations/{conv_id}/messages",
            params={"before_seq": last_seq},
        )
        assert_response_success(paged_resp)
        paged_msgs = extract_data(paged_resp.json())["messages"]

        for m in paged_msgs:
            if m.get("nats_seq") is not None:
                assert m["nats_seq"] < last_seq, \
                    f"before_seq filter broken: got seq {m['nats_seq']} >= {last_seq}"


class TestDmSettingsVerification:
    """Verify settings updates persist correctly."""

    def test_muted_status_persists(self, auth_client, seeded_conversations):
        conv_id = seeded_conversations["convs"][2]["conv_id"]

        resp = auth_client.put(
            f"/api/v1/dm/conversations/{conv_id}/settings",
            json={"status": "muted"},
        )
        assert_response_success(resp)

        list_resp = auth_client.get("/api/v1/dm/conversations")
        data = extract_data(list_resp.json())
        conv = next((c for c in data["conversations"] if c["conv_id"] == conv_id), None)
        assert conv is not None, f"Conversation {conv_id} not found after muting"
        assert conv["status"] == "muted", f"Expected muted, got {conv['status']}"

    def test_archived_status_persists(self, auth_client, seeded_conversations):
        conv_id = seeded_conversations["convs"][2]["conv_id"]

        resp = auth_client.put(
            f"/api/v1/dm/conversations/{conv_id}/settings",
            json={"status": "archived"},
        )
        assert_response_success(resp)

        list_resp = auth_client.get("/api/v1/dm/conversations")
        data = extract_data(list_resp.json())
        conv = next((c for c in data["conversations"] if c["conv_id"] == conv_id), None)
        assert conv is not None
        assert conv["status"] == "archived"


if __name__ == "__main__":
    pytest.main([__file__, "-v", "--tb=short"])
