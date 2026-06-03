# JPlan ROOT — Fix: campaign "AI 回复" headline ≠ Σ per-post "有效" (valid_comment_count)

**Status:** DRAFT → under review
**Owner:** (assign)
**Repos:** `glance_mind_front` (web, primary change) · `glance_mind_rust` (backend test only)
**Related debug session:** campaign 221 production investigation (root cause = pagination vs aggregate)

---

## 1. Problem & confirmed root cause (evidence, not guess)

On `/apps/social-monitor/campaigns/:id` the header shows **"AI 回复" = `stats.replies`** and the captured-content list shows a per-post **"有效" = `valid_comment_count`**. Users expect `headline == Σ(per-post 有效)`.

Production evidence (campaign 221, Facebook):
- Backend is **internally consistent**: `get_ai_replies_count(221)=3`; `Σ valid_comment_count over all 20 posts = 3`. DB has **zero** `campaign_id` divergence on Facebook (845 comments, 0 NULL, 0 orphan).
- The **only** post with comments (post 516, valid=3) sorts to **position 16** of 20 in `created_at DESC`.
- `default_page_size() = 10` (`crates/api/src/dto/common.rs:18`).
- The web page fetches `/campaigns/:id/crawler-results` **once with no page params** (`apps/web/src/pages/CampaignDetail.tsx:297`) → backend returns **page 1 / 10 posts only** → stores `resultsData.list` into `campaignResults` → then paginates **client-side** at 5/page (`CampaignDetail.tsx:445-448`).
- Net: posts 11–20 (incl. post 516) are **never loaded**; every visible "有效" = 0, while the headline (a global aggregate) = 3. The client paginator even reads "Page 1 of 2" while silently missing 10/20 posts.

**Conclusion:** Not a DB bug, not a `campaign_id` tagging bug. It is a **frontend data-completeness bug**: a *global aggregate* (headline) is compared against a *single server page* of detail rows. The user's mental model (`headline == Σ list`) is correct; the list is just incomplete.

**Fix shape:** make the web captured-content list load **all** posts for the campaign, so the per-post "有效" values are complete and reconcile with the headline. Lock the invariant with a backend integration test on real Postgres.

---

## 2. Scope

### In scope
- **Module A (frontend):** `glance_mind_front/apps/web` — load all crawler-results pages in `loadCrawlerData`; client-side display pagination now spans the complete set. + deterministic component test (RED→GREEN), pure-helper property test, scoped mutation gate.
- **Module B (backend test only):** `glance_mind_rust/crates/api/tests/test_campaign_api.py` — real-Postgres integration test locking `Σ(valid_comment_count over all pages) == stats.replies` when the comment-bearing post is on a non-first page. **No backend production code change.**

### Out of scope (explicit — do NOT bundle)
- **OOS1** — 5 Instagram comments where `comment.campaign_id ≠ post.campaign_id` (separate data-integrity issue; TikTok/Twitter/Reddit/Facebook are clean). Track separately. **Note:** on Instagram these 5 rows are counted in the headline (by `comment.campaign_id`) but attach to a post under a different campaign, so even after this frontend fix an Instagram campaign touched by them may still show a small headline-vs-list gap until the backend corrects the `campaign_id` values. Facebook (the reported platform) is unaffected.
- **OOS2** — Low comment-collection depth (post 516: 3 stored of 6,245 raw FB comments). Separate provider/collection concern.
- **OOS3** — Desktop app: already correct (uses `/campaigns/:id/contents` with true server-side pagination + refetch). No change.
- **OOS4** — Converting the web list to true server-side pagination, or adding a backend `total_valid_comment_count` aggregate field. Deferred scalability/hardening; current fix loads all pages (campaigns are bounded to tens–low hundreds of posts). Revisit if campaign post counts grow large.

---

## 3. Requirement ledger

| ID | Requirement | Source | Owner module | Acceptance evidence |
|----|-------------|--------|--------------|---------------------|
| R001 | Web campaign-detail captured-content list MUST load **all** collected posts of the campaign, not only server page 1. | Debug RC | A | Component test: paginator/total reflects full count; helper property test. |
| R002 | For Facebook (reported platform; `comment.campaign_id == post.campaign_id` holds), `Σ(per-post valid_comment_count over ALL posts) == headline stats.replies`. | User expectation | A+B | Backend pytest invariant (real DB) green; component test list complete. |
| R003 | The client-side results paginator MUST reflect the **true** post count (no "Page 1 of 2" while posts are silently missing). | Debug RC | A | Component test asserts total pages = ceil(allPosts/5). |
| R004 | A real-Postgres integration test MUST lock R002 for a campaign whose only comment-bearing post is on a **non-first** server page. | Anti-regression | B | `pytest ... -v` green; characterizes page-1-only Σ=0 vs all-pages Σ=stats.replies. |
| R005 | The "load all pages" loop MUST be **bounded** and MUST NOT silently truncate (no silent caps); hitting the cap surfaces a warning. | Anti-slop hygiene | A | Property test asserts page-math + `capped` flag; code logs on cap. |

---

## 4. Invariant / failure matrix

| Concern | Decision |
|---|---|
| State key `campaignResults` | **Single writer:** `loadCrawlerData`. **Readers:** client pagination slice `currentResults`, render rows, paginator `totalResultPages`. |
| Idempotency | `hasLoadedCrawlerData` guard retained — one load per mount. |
| Partial-failure (a page fetch rejects mid-loop) | Whole results-load fails → existing `toast.error('Failed to load crawler data')`; `campaignResults` NOT set to a partial set (never present an incomplete list as complete). |
| Bounded fan-out | `page 1` then `Promise.all(pages 2..ceil(total/PAGE_SIZE))`, `PAGE_SIZE=50`, `MAX_PAGES=40` (≤2000 posts). Exceeding cap → load up to cap **and `console.warn`** (R005); real campaigns never reach it. |
| Ordering | Merge preserves server order (`created_at DESC`); client pagination unchanged. |
| Security/scope | Endpoints stay JWT-protected + user-scoped; no change to auth. |
| Observability | `console.warn` on cap; no new telemetry required. |

---

## 5. Test Suite Ledger

| Req | Unit/pure (deterministic) | Component (deterministic mock) | Integration / real-dep | Edge/failure |
|-----|---------------------------|--------------------------------|------------------------|--------------|
| R001/R003 | A1T unit (exact page-math, load-bearing) + A4 property (fast-check, defense) | A3: paginator total pages = ceil(allPosts/5); page-2-origin post reachable | — | — |
| R002 | — | A3: Σ(rendered per-post 有效 cells) == headline tile == 3 | **B:** pytest real Postgres: Σ valid over all pages == stats.replies | — |
| R004 | — | — | **B:** characterizes page-1 Σ=0 ≠ total | — |
| R005 | **A1T unit (load-bearing):** `capped` true iff pages>MAX, `pages` ends at MAX; A4 property (defense) | — | — | A2: a page fetch rejects → toast, guard reset for retry, list not partially set |

**Mandatory deterministic feature mock test (R001/R002/R003):** `apps/web/src/pages/__tests__/CampaignDetail.loadAllResults.test.tsx` (mocks `apiClient.get`, no live services). Names + RED/GREEN in Module A.

---

## 6. Anti-Gaming Test Quality Ledger (per `~/.claude/TESTING_CONSTRAINTS.md` — embedded §11)

| Feature | RED→GREEN evidence (exact expected RED) | Assertion-immutability rule | Mutation gate (cmd + threshold) | Property test |
|---|---|---|---|---|
| A: load all pages | Component test (A3) against **current** code: `getByTestId('campaign-results-pagination')` reads `Page 1 of 2` (RED) + page-walk Σ `0 !== 3`; post 52 absent. Unit (A1T) RED before helper exists. After fix → GREEN. | Implementer MUST NOT weaken/remove assertions or add skip to pass; fix the loader, not the test. | `npx stryker run` scoped to `campaignResultsPaging.ts`, **≥80%** killed (CI/nightly, Module A task A5). | **load-bearing** `campaignResultsPaging.unit.test.ts` (exact + cap cases) **+ defense** `campaignResultsPaging.property.test.ts` (fast-check) on `remainingPageNumbers`/`mergePages`. |
| B: invariant guard | Characterization: assert page-1-only `Σ valid == 0` while `stats.replies == 3` (bug shape), then all-pages `Σ valid == 3 == stats.replies`. | Same. This is a **regression/characterization guard** (backend already correct) — labeled honestly, no faked RED. | mutmut **N/A** (E2E, non-deterministic harness) — justified per TESTING_CONSTRAINTS §4. | N/A (integration). |

---

## 7. Production Dependency Test Ledger

| Dependency | Prod path | Feature covered | Real test | Cred/env gate | Idempotency/cleanup | Cost/rate | Pass evidence |
|---|---|---|---|---|---|---|---|
| **PostgreSQL** (test DB :5434 `glancemind_test` / CI `aihub_db`) | `GET /campaigns/:id`, `GET /campaigns/:id/crawler-results` over real DB rows | R002/R004 | Module B pytest (`test_campaign_api.py`) | `DATABASE_URL`, `API_BASE_URL`, JWT via `auth_client`/login (`conftest.py`) | unique `suffix` per run; `db_cursor` rollback fixture | none | `pytest ... -v` green |

**Provider Test Ledger:** N/A — this fix touches no external 3rd-party provider (TikHub/Facebook/LLM). Justified: only the internal REST API + Postgres are exercised; both are covered by Module B (real DB) and Module A (mock API).

**External-LLM API Boundary Ledger:** N/A — no LLM request/response path is added or changed. Justified.

**Non-Code Verification Exception Ledger:** none — every task is code/behavior with a test.

---

## 8. Framework Test Research Ledger

| Tech | Native/official tooling | Suite shape | Deterministic vs live | Chosen | Rejected |
|---|---|---|---|---|---|
| Web unit/component | **Vitest** + `@testing-library/react` (already configured: `apps/web/vite.config.ts:137`) | jsdom, module-mock `@gm/shared.apiClient` (pattern: `CampaignDetail.terminal-reason.test.tsx`) | deterministic | Vitest + RTL | MSW (not used in repo) |
| Web property | **fast-check** (TESTING_CONSTRAINTS §4 JS) — **add devDep** | property over pure page-math helpers | deterministic | fast-check | hand-enumerated cases only (weaker) |
| Web mutation | **Stryker** (`@stryker-mutator/core` + vitest runner) — **add devDep + scoped config** | mutate only `campaignResultsPaging.ts` | deterministic, **CI/nightly** | Stryker scoped | full-repo Stryker (too heavy) |
| Backend E2E | **pytest** + `psycopg2` (existing `conftest.py`, `auth_client`, `_create_owned_campaign`, `_seed_platform_data`) | seed real rows → call API → assert | live/credentialed (real Postgres) | pytest | Rust `#[ignore]` live-server tests (heavier setup for this assertion) |

> `researcher`/`dependency-expert` delegation: not required — official tooling for each stack is already present in-repo (Vitest config, pytest conftest) or is the canonical choice named in TESTING_CONSTRAINTS §4 (fast-check/Stryker). Adding fast-check/Stryker is a devDependency + config addition, tracked as Module A tasks A0/A5.

---

## 9. Execution order & final chain gate

1. **Module A** (frontend fix + tests) — primary; independently implementable & testable.
2. **Module B** (backend invariant guard) — independent; may run in parallel; locks the contract.

**Final acceptance (all must pass before "done"):**
- A — load-bearing: `npx vitest run` on `CampaignDetail.loadAllResults.test.tsx` + `campaignResultsPaging.unit.test.ts` **green**, with recorded RED→GREEN evidence; web typecheck/build green. (A1T covers the R005 cap deterministically, so the bound is verified pre-merge without the fast-check devDep.)
- A — defense-in-depth (CI/nightly, not a merge blocker by default — see §10 tiering): fast-check property green; Stryker ≥80% on the helper.
- B — `pytest test_campaign_api.py::TestCampaignValidReplyInvariant -v` **green** against the real test DB; `db-field-validation-guard` + `api-contract-guard` consulted; `rust-verify-change` run.
- No P0/P1 review findings open.

---

## 10. Honest tiering (karpathy-hygiene + jplan rigor reconciled)

- **Load-bearing (block merge):** Module A component RED→GREEN test (A3) + deterministic page-math/cap unit test (A1T); Module B real-DB invariant; web typecheck/build; `rust-verify-change`. These directly prove the bug is fixed, the cap is bounded, and the invariant holds — with no dependency on newly-added devDeps.
- **Defense-in-depth (recommended; may land in a fast-follow or CI/nightly):** fast-check property test on page-math, scoped Stryker gate. These harden the pure helper but require new devDeps; flagged for owner decision in §6/§8. **They are specified fully (commands + thresholds) and are not optional language — only their scheduling (merge vs CI/nightly) is the owner's call.**

---

## 11. Embedded global anti-test-gaming constraints (binding on every task & implementer)

From `~/.claude/TESTING_CONSTRAINTS.md` / `~/.claude/CLAUDE.md` — non-negotiable:
1. No weakening/removing/commenting/deleting existing assertions to pass. Fix production code; or correct a genuinely wrong expectation with `ASSERTION-CHANGE-JUSTIFIED: <reason>`; or delete a truly obsolete test wholesale with explanation.
2. No `skip/xfail/.skip/xit/#[ignore]/t.Skip/@Disabled` to bypass failure.
3. No faked pass (`assert True`, swallowed exceptions, expected←actual, prod special-casing test input).
4. RED-for-the-right-reason first: every behavior test fails for the correct reason before GREEN; keep RED→GREEN evidence.
5. Report only observed results; not "passed" unless all assertions verified.
6. Separation of duties: the context that writes/owns tests ≠ the context that edits production code; implementer may not edit test assertions.
7. Any assertion change is high-risk and needs written justification.

Core-logic property tests and core-module mutation gates are required (Module A helper). Local `assertion_guard.py`/`bash_assertion_guard.py` hooks are speed-bumps; CI is the real enforcement.

---

## 12. Module index

- [`01-frontend-load-all-posts.md`](01-frontend-load-all-posts.md) — Module A (web fix + tests).
- [`02-backend-invariant-guard.md`](02-backend-invariant-guard.md) — Module B (real-Postgres invariant test).
