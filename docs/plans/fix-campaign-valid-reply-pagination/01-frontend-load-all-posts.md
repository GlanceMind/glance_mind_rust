# Module A — Frontend: load ALL crawler-results pages (web)

**Repo:** `glance_mind_front` · **App:** `apps/web`
**Standalone verification (load-bearing):** `cd /Users/jacksoom/programer/aihub/glance_mind_front/apps/web && npx vitest run src/pages/__tests__/CampaignDetail.loadAllResults.test.tsx src/pages/__tests__/campaignResultsPaging.unit.test.ts`

> Anti-gaming (binds this module): implementer MUST NOT weaken/remove assertions, add `.skip`/`xit`, or special-case test input to make a test pass. Failures are fixed in the loader/helper. Any assertion change needs `ASSERTION-CHANGE-JUSTIFIED:`. RED→GREEN evidence is a required deliverable per task.

## Two pagination layers (do not conflate)
- **Server fetch granularity** = `PAGE_SIZE` (this fix sends 50; backend default is 10). Used only to pull all posts into memory.
- **Client display granularity** = `itemsPerPage = 5` (`CampaignDetail.tsx:212`, unchanged). Drives the on-screen paginator.
- A 52-post campaign ⇒ server fetch = page1(50)+page2(2) = **2 requests**; client paginator = `ceil(52/5)` = **11 pages**.

## Files
- **New (prod):** `apps/web/src/pages/campaignResultsPaging.ts` — pure page-math + merge helpers (core logic).
- **Edit (prod):** `apps/web/src/pages/CampaignDetail.tsx` — `loadCrawlerData` (~286–314); add a `data-testid` to the results paginator text div (~893). Client pagination logic/render otherwise unchanged.
- **New (test, load-bearing):** `apps/web/src/pages/__tests__/campaignResultsPaging.unit.test.ts` — deterministic example/cap cases (plain Vitest, no extra deps).
- **New (test, load-bearing):** `apps/web/src/pages/__tests__/CampaignDetail.loadAllResults.test.tsx` — component RED→GREEN.
- **New (test, defense-in-depth):** `apps/web/src/pages/__tests__/campaignResultsPaging.property.test.ts` — fast-check property tests.
- **Edit (config, defense-in-depth):** `apps/web/package.json` (+`fast-check`, +`@stryker-mutator/*`), `apps/web/stryker.conf.json` (new, scoped).

---

## A1 — Pure helpers `campaignResultsPaging.ts`
Implement, no IO:
```ts
export function remainingPageNumbers(total: number, pageSize: number, maxPages = 40): { pages: number[]; capped: boolean };
export function mergePages<T>(pages: Array<{ list: T[] }>): T[];
```
Rules: clamp `pageSize >= 1`; `totalPages = Math.ceil(total / pageSize)`; `pages = [2..min(totalPages, maxPages)]` (empty when `totalPages <= 1`); `capped = totalPages > maxPages`. `mergePages` flattens in order.
- Anti-gaming: do not hardcode expected arrays to dodge the generator/cases.

## A1T — Deterministic unit test (LOAD-BEARING) — owns R005 cap + page-math
File: `apps/web/src/pages/__tests__/campaignResultsPaging.unit.test.ts` (plain Vitest, no fast-check).
Exact cases (assert all):
- `remainingPageNumbers(0,50)` → `{pages:[],capped:false}`; `remainingPageNumbers(50,50)` → `{pages:[],capped:false}`; `remainingPageNumbers(52,50)` → `{pages:[2],capped:false}` (mirrors the component scenario).
- `remainingPageNumbers(10,10)` → `{pages:[],capped:false}` (single page).
- **Cap (R005):** `remainingPageNumbers(2001,50,40)` → `capped===true` and `pages` ends at `40` (`pages.length === 39`, i.e. `[2..40]`). `remainingPageNumbers(2000,50,40)` → `capped===false`.
- `mergePages([{list:[1,2]},{list:[3]}])` → `[1,2,3]` (order preserved).
- **Expected RED (before A1):** import/symbol error, or wrong math (e.g. off-by-one → `expected [2] received [2,3]`). Record output.
- **GREEN:** `npx vitest run src/pages/__tests__/campaignResultsPaging.unit.test.ts`.

## A2 — Rewrite `loadCrawlerData` to fetch all pages (owns R001/R003/R005)
Replace the single un-paginated results fetch (`CampaignDetail.tsx:294-305`) with:
1. Tasks fetch unchanged.
2. `const PAGE_SIZE = 50; // backend default is 10; larger size = fewer requests for typical tens–hundreds-of-posts campaigns`.
3. Fetch page 1 with explicit params:
   `apiClient.get<PageResponse<UnifiedContent>>(ENDPOINTS.CAMPAIGNS.CRAWLER_RESULTS(id), { params: { page: 1, page_size: PAGE_SIZE } })`.
4. `const first = extractData<PageResponse<UnifiedContent>>(resultsRes);`
5. **Use the server-returned page size** (robust if backend ever caps): `const serverPageSize = first?.page_size || PAGE_SIZE;` then `const { pages, capped } = remainingPageNumbers(first?.total ?? 0, serverPageSize);`
   - Backend honors the requested size today (`crawler_repository.rs:412` `.limit(page_size)`, no cap), so `serverPageSize === 50`. Reading it from the response keeps the loop correct even if a cap is later added.
   - `if (capped) console.warn('[CampaignDetail] crawler-results capped at MAX_PAGES; list may be incomplete', { total: first?.total });` (R005, no silent truncation).
6. `const rest = await Promise.all(pages.map(p => apiClient.get(ENDPOINTS.CAMPAIGNS.CRAWLER_RESULTS(id), { params: { page: p, page_size: serverPageSize } }).then(r => extractData<PageResponse<UnifiedContent>>(r))));`
7. `const all = mergePages([{ list: first?.list ?? [] }, ...rest.map(r => ({ list: r?.list ?? [] }))]);` → `setCampaignResults(all);`
8. **Failure + retry (F2 fix):** the early `if (!id || hasLoadedCrawlerData) return; setHasLoadedCrawlerData(true);` at the top prevents *concurrent* in-flight loads — keep it. In the existing `catch`, **also `setHasLoadedCrawlerData(false)`** so a failed multi-page load can be retried (Refresh button / re-scroll); show `toast.error('Failed to load crawler data')` and do **not** call `setCampaignResults` (never present a partial set as complete). It retries on the next user trigger, not in an auto-loop.
9. Add `data-testid="campaign-results-pagination"` to the **inner text `<div className="text-sm text-muted-foreground">`** at `CampaignDetail.tsx:~893` (the element rendering `{t.campaigns.page} {resultsPage} {t.campaigns.of} {totalResultPages}`), NOT the outer conditional wrapper at ~882. Anchors the A3 assertion locale-independently.
- Client display pagination (`445-448`, render `880-904`) is otherwise unchanged — it now slices the **complete** array.
- Anti-gaming: do not special-case `id`/fixtures.
- RED/GREEN evidence: captured by A3.

## A3 — Component test (mandatory deterministic feature mock test; RED→GREEN) — R001/R002/R003
File: `apps/web/src/pages/__tests__/CampaignDetail.loadAllResults.test.tsx`, modeled on `CampaignDetail.terminal-reason.test.tsx` (same `vi.mock('@gm/shared', ...)` with mocked `apiClient`, the local `apiResponse()` wrapper, `MemoryRouter`, `I18nProvider`).

**Insights-board stub (F3 fix):** the existing terminal-reason test replaces `CampaignInsightsBoard` with a stub button, which hides `perf-metric-ai-replies`. In THIS test, stub it to expose the headline AND trigger the load:
```tsx
vi.mock('../../components/campaign/CampaignInsightsBoard', () => ({
  default: ({ stats, onScansClick }: any) => (
    <div>
      <div data-testid="perf-metric-ai-replies">{stats.replies}</div>
      <button onClick={onScansClick}>load</button>
    </div>
  ),
}));
```
Then in the test, click `load` to trigger `loadCrawlerData` (mirrors how the real tile's scans click → `scrollToTasks()` → `loadCrawlerData()`).

**Fixture (F10 fix):** concrete 52-item array; only the LAST is comment-bearing:
```ts
const fixture = Array.from({ length: 52 }, (_, i) => ({
  id: i + 1, task_id: 1, content_id: `post_${i + 1}`, platform: 'facebook',
  title: `Post ${i + 1}`, author_name: 'a', comment_count: 0,
  valid_comment_count: i === 51 ? 3 : 0,
}));
```
**Mock `apiClient.get`:**
- `'/campaigns/123'` → campaign with `stats.replies = 3` (and minimal fields the page reads).
- `'/campaigns/123/crawler-results'` honoring params: `page = config?.params?.page ?? 1`, `page_size = config?.params?.page_size ?? 10`; return `apiResponse({ list: fixture.slice((page-1)*page_size, page*page_size), total: 52, page, page_size, total_pages: Math.ceil(52/page_size) })`. **The `?? 10` faithfully reproduces current no-param behavior for the RED run.**
- `'/campaigns/123/crawler-tasks'` → `apiResponse({ list: [], total: 0, page: 1, page_size: 10, total_pages: 0 })`.

**Assertions (must hold for the FIXED code):**
1. `expect(screen.getByTestId('campaign-results-pagination')).toHaveTextContent('11')` — client pages `= ceil(52/5) = 11` (proves all 52 loaded). *(R001/R003)*
2. Page to the last results page; `expect(screen.getByText('Post 52')).toBeInTheDocument()` and the post-52 row's valid cell (`data-testid="campaign-result-52-valid-comments"`, per `CampaignDetail.tsx:837`) shows `3`. *(R001)*
3. **User-visible reconciliation (R002, P2-2 fix):** walk all 11 client pages, collect every `screen.getAllByTestId(/^campaign-result-\d+-valid-comments$/)` value, sum them, and assert the sum `=== Number(screen.getByTestId('perf-metric-ai-replies').textContent)` `=== 3`. This proves the list's per-post 有效 now sums to the headline on the frontend.
- **Expected RED (run against current `CampaignDetail.tsx` BEFORE A2):** assertion 1 fails — paginator reads `of 2` (only 10 loaded) → `received "...Page 1 of 2"`; post 52 absent; the page-walk sum is `0 !== 3`. Record this output.
- **GREEN command:** `cd apps/web && npx vitest run src/pages/__tests__/CampaignDetail.loadAllResults.test.tsx`.
- Anti-gaming: these assertions are the contract — fix `loadCrawlerData` (A2), never relax them.

## A4 — Property test for page-math (DEFENSE-IN-DEPTH; CI/nightly) — hardens R005
Prereq A0: add `fast-check` devDep to `apps/web/package.json`.
File: `apps/web/src/pages/__tests__/campaignResultsPaging.property.test.ts` (fast-check).
Properties:
- ∀ `total∈[0,5000]`, `pageSize∈[1,200]`: `{1} ∪ remainingPageNumbers(total,pageSize).pages == {1..ceil(total/pageSize)}` when not capped; `pages` strictly increasing & contiguous from 2.
- `capped === (ceil(total/pageSize) > maxPages)`; when capped, `max(pages) === maxPages`.
- ∀ partition of an array into pages: `mergePages(pages)` equals the original concatenation (length + order).
- **Expected RED (before A1):** import error or counterexample (e.g. `Property failed ... total=10,pageSize=10 expected {1} got {1,2}`).
- **GREEN:** `cd apps/web && npx vitest run src/pages/__tests__/campaignResultsPaging.property.test.ts`.

## A5 — Scoped mutation gate (DEFENSE-IN-DEPTH; CI/nightly) — core module
- Add `@stryker-mutator/core` + `@stryker-mutator/vitest-runner`; `apps/web/stryker.conf.json` with `mutate: ["src/pages/campaignResultsPaging.ts"]`, vitest runner, `thresholds.high=90, low=80, break=80`.
- **Command:** `cd apps/web && npx stryker run`. **Gate:** ≥80% mutants killed on `campaignResultsPaging.ts`. Surviving mutants ⇒ strengthen A1T/A4 (never weaken).

---

## Tiering (per root §10)
- **Load-bearing (block merge):** A1, A1T, A2, A3 + web typecheck/build. These prove the bug is fixed AND the cap logic + reconciliation hold deterministically — independent of any new devDep.
- **Defense-in-depth (CI/nightly; needs new devDeps):** A0/A4 (fast-check property), A5 (Stryker).

## Standalone acceptance (Module A)
```
cd /Users/jacksoom/programer/aihub/glance_mind_front/apps/web
npx vitest run src/pages/__tests__/CampaignDetail.loadAllResults.test.tsx \
               src/pages/__tests__/campaignResultsPaging.unit.test.ts
# + web typecheck/build per apps/web/package.json scripts (e.g. `npm run build`)
# + (CI/nightly) npx vitest run src/pages/__tests__/campaignResultsPaging.property.test.ts ; npx stryker run
```
Deliver RED→GREEN logs for A1T and A3.
