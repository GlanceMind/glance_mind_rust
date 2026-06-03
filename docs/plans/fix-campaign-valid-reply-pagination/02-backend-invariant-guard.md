# Module B — Backend invariant guard (real Postgres, pytest)

**Repo:** `glance_mind_rust` · **File:** `crates/api/tests/test_campaign_api.py` (extend)
**No backend production code change.** This locks the contract the frontend depends on and characterizes the bug shape.

> Honesty note (anti-gaming): the backend is **already correct**, so this is a **characterization + regression guard**, not a backend RED→GREEN bug fix. The RED-for-the-right-reason for the *bug* lives in Module A (frontend). Here, the "page-1-only Σ=0 vs all-pages Σ=stats.replies" assertion pair documents *why* summing a single page is wrong. Do not fake a RED by mutating backend code.

## Context (verified)
- `GET /campaigns/:id` → `stats.replies` = `get_ai_replies_count` (`campaign_repository.rs:330`), global COUNT by `campaign_id`.
- `GET /campaigns/:id/crawler-results` → per-item `valid_comment_count` (`crawler_repository.rs:493`), paginated (`PageRequest`, `default_page_size=10`).
- Existing harness: `conftest.py` fixtures `db_cursor`, `db_connection`, `auth_client`; helpers `_create_owned_campaign`, `_seed_platform_data`; existing assertions on `stats.replies` (`test_campaign_api.py:1853`) and per-item `valid_comment_count` (`:1876`).
- Seeding pattern: raw SQL inserts + `db_connection.commit()` (`:1538-1593`). Posts ordered `created_at DESC`.

## B1 — `test_sum_valid_comment_count_across_pages_equals_stats_replies` (R002/R004)
New class `TestCampaignValidReplyInvariant` (Facebook, `platform_id=3`).

**Seed (unique `suffix`):**
1. `campaign_id = _create_owned_campaign(auth_client, db_cursor, PLATFORM_FACEBOOK, suffix)`.
2. Insert **12 Facebook posts** under one task (`> default page_size 10` ⇒ ≥2 server pages). The endpoint orders `created_at DESC` (`crawler_repository.rs:411`), so set **explicit, distinct `created_at`** per post (e.g. `NOW() - (i * interval '1 minute')`) and give the **comment-bearing post the earliest `created_at`** so it deterministically sorts to page 2 (positions 11–12). Do NOT rely on `NOW()` for all rows — ties make ordering undefined and the post could land on page 1, silently invalidating the test. Provide required NOT NULL cols: `task_id`, `campaign_id`, `facebook_post_id` (unique per campaign), plus `comments_count` (raw) for realism.
3. Insert **exactly 3** `gm_agent_facebook_comments` on that one post (`post_db_id`, `campaign_id`, `facebook_comment_id`, `comment_text` NOT NULL; `status`, `suggested_reply` set). `db_connection.commit()`.

**Concrete seeding (executable — not pseudo; existing `_seed_platform_data` uses bare `NOW()` and will NOT give deterministic order):**
```python
suffix = uuid.uuid4().hex[:8]
campaign_id = self._create_owned_campaign(auth_client, db_cursor, PLATFORM_FACEBOOK, suffix)
# one task for the campaign (reuse however _seed_platform_data creates it, or insert directly)
post_ids = []
for i in range(12):  # i=11 is OLDEST -> sorts last under created_at DESC -> page 2
    db_cursor.execute(
        """
        INSERT INTO gm_agent_facebook_posts
          (task_id, campaign_id, facebook_post_id, post_type, url, message,
           timestamp, posted_at, reactions_count, comments_count, author_name, created_at)
        VALUES (%s,%s,%s,'photo',%s,%s, 1705300000, NOW(), 0, %s, %s, NOW() - (%s * interval '1 minute'))
        RETURNING id
        """,
        (task_id, campaign_id, f"fb_post_{suffix}_{i}",
         f"https://facebook.com/p/{suffix}/{i}", f"post {i}", (3 if i == 11 else 0),
         f"author_{i}", i),
    )
    post_ids.append(db_cursor.fetchone()["id"])
comment_post_id = post_ids[11]  # the oldest post (page 2)
for c in range(3):
    db_cursor.execute(
        """
        INSERT INTO gm_agent_facebook_comments
          (post_db_id, campaign_id, facebook_comment_id, comment_text, reason, suggested_reply, status)
        VALUES (%s,%s,%s,%s,'r','reply',3)
        """,
        (comment_post_id, campaign_id, f"fb_cmt_{suffix}_{c}", f"comment {c}"),
    )
db_connection.commit()
```

**Assertions (the contract — immutable):**
1. `detail = GET /api/v1/campaigns/{id}` → `detail["stats"]["replies"] == 3`.
2. **Characterization assertion (GREEN-now — documents the bug shape; expects the comment-bearing post to be OFF page 1):** `page1 = GET /campaigns/{id}/crawler-results?page=1&page_size=10` → `len(page1["list"]) == 10` and `sum(item["valid_comment_count"] for item in page1["list"]) == 0`. This proves *why* a single-page sum mismatches the headline (it is GREEN against today's correct backend; it would only go RED if a backend regression moved the post onto page 1).
3. **Invariant (the fix target) — explicit page loop (do NOT single-fetch):**
   ```python
   all_items, page = [], 1
   while True:
       resp = auth_client.get(f"/api/v1/campaigns/{campaign_id}/crawler-results",
                              params={"page": page, "page_size": 10})
       assert_response_success(resp)
       data = extract_data(resp.json())
       all_items.extend(data["list"])
       if page >= data["total_pages"]:
           break
       page += 1
   assert len(all_items) == 12                                   # all posts reachable
   assert sum(it["valid_comment_count"] for it in all_items) == 3
   assert sum(it["valid_comment_count"] for it in all_items) == detail["stats"]["replies"]
   ```
4. (Optional cross-route) repeat the all-pages loop for `/campaigns/{id}/contents` → same `== 3`.

**Commands**
- Single test (DB up + API running per harness):
  `cd /Users/jacksoom/programer/aihub/glance_mind_rust/crates/api/tests && python3 -m pytest test_campaign_api.py::TestCampaignValidReplyInvariant::test_sum_valid_comment_count_across_pages_equals_stats_replies -v --tb=short`
- Full harness (spins DB+API, applies migrations+seed): `cd crates/api && make test-e2e` (or `bash tests/run_tests.sh`).
- Env: `DATABASE_URL=postgres://glancemind:testpassword@localhost:5434/glancemind_test`, `API_BASE_URL`, JWT via `auth_client`/login (`JWT_SECRET` from harness env). CI uses `aihub_user:aihub_password@localhost:15432/aihub_db` and applies migrations+seed before DB tests.

**Expected results**
- Assertion 2 demonstrates the bug shape (page-1 Σ=0 ≠ 3).
- Assertion 3 is **GREEN on current backend** (backend correct) — this is the regression lock. If it ever goes RED, a real backend regression (or `campaign_id`/`post_db_id` divergence) has been introduced — fix backend, never relax the assertion.

## Guards & verification (per `glance_mind_rust/CLAUDE.md`)
- **`db-field-validation-guard`** — this is a DB-writing integration test (seeds `gm_campaigns`, `gm_agent_facebook_posts`, `gm_agent_facebook_comments`): consult before/after to confirm required fields + FK correctness.
- **`api-contract-guard`** — the test reads `stats.replies` and `valid_comment_count`; confirm no response-shape assumption drift (read-only assertion, no contract change intended).
- **`rust-verify-change`** — run before claiming completion.
- mutmut: **N/A** (live-harness E2E, non-deterministic) — justified per TESTING_CONSTRAINTS §4.

## Standalone acceptance (Module B)
Test runs against the real test Postgres and is **green**; RED-shape (assertion 2) recorded; guards consulted; `rust-verify-change` clean.
