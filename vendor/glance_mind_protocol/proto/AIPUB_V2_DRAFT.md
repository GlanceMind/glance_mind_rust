# AI Publish Protocol v2 — Unified Content Schema (DRAFT)

**Status**: DRAFT — protocol-only, no consumer changes shipped yet
**Date**: 2026-04-19
**Source of truth**: `proto/aipub.proto`, section "v2 — Unified Content Schema"

---

## 1. Why v2

v1 of the AI Publish protocol grew through flat field accretion:

- `AiPubInput.video_prompt` / `content_prompt` / `prompt` (three competing prompt fields)
- `AiPubInput.default_images.{start_frame_url, end_frame_url}` *and* `AiTaskInput.{start_image_url, end_image_url}` *and* `SeedanceVideoConfig.image_urls[]` *and* `RedditPostConfig.uploaded_image_urls[]` — **four ways to express "this is a frame image"**
- `AiPubInput.account_images: map<string, AiPubImageConfig>` — per-account image override only, no equivalent for video/audio
- `AiPubTaskContent.video_url` / `start_frame_url` / `end_frame_url` / `cover_url` / `reference_image_urls[]` — flat fields per role
- `oneof platform_config { RedditPostConfig … }` exists but only Reddit is filled in; other platforms drift into JSONB extras

The strongest evidence v1 has hit its limit is in `glance_mind_worker/glance_mind_scheduler/tests/reddit_aipub_integration_test.rs`:

> Reddit-specific JSON shapes for `gm_aipub_plans.ai_input` / `gm_aipub_tasks.content`. **These are no longer fields on `protocol_gen::AiPubInput` / `AiPubTaskContent`** but remain stored as JSONB for the Reddit publish pipeline — integration tests keep the same wire shape.

The typed protocol and the production JSON have **already forked** for Reddit, and Seedance embeds its own three-bucket media model (`image_urls / video_urls / audio_urls` + `*_roles` map) as a workaround. Every new platform or content shape will repeat this pattern.

v2 unifies the wire format around three uniform collections so that adding a new platform / content type is a config change, not a schema change.

---

## 2. Design at a glance

The schema is **asymmetric by intent**:

- **Input side (`UnifiedAiPubInput`)** describes *what to generate* — three typed spec lists (text / image / video), because each kind has a different shape (text needs role hints; image needs dimensions; video needs an engine + mode + conditioning frames).
- **Output side (`UnifiedPublishContent`)** describes *what to publish* — one uniform `media[]` bag (with `kind` discriminator) plus `texts[]` and `links[]`, because executors need uniform addressing.

```
UnifiedAiPubInput  (gm_aipub_plans.ai_input,  version=2)
├── text_generations:   repeated TextGenerationSpec   ── 0..N text jobs
├── image_generations:  repeated ImageGenerationSpec  ── 0..N image jobs
├── video_generations:  repeated VideoGenerationSpec  ── 0..N video jobs
├── initial_media:      repeated MediaItem            ── user uploads, refs, frames
├── account_media:      map<account_id, MediaItem[]>
├── platform_config:    oneof (RedditPostConfig | future)
└── generation_extras:  map<string, string>           ── plan-wide escape hatch

UnifiedPublishContent  (gm_aipub_tasks.content, version=2)
├── routing: platform / platform_id / content_type / plan_type
├── media:        repeated MediaItem        ── videos + images + audio + subtitles
├── texts:        repeated TextBlock        ── title, body, caption, hashtag, …
├── links:        repeated LinkItem         ── Reddit link posts, future embeds
├── tags:         repeated EntityTag        ── music / product / location / topic
├── mentions:     repeated UserMention      ── @-handles with byte offsets
├── behavior?:    PublishBehavior           ── visibility, NSFW, allow_comments, …
├── schedule?:    PublishSchedule           ── scheduled_at + IANA timezone
├── post_publish: repeated PostPublishAction── auto-first-comment, pin, crosspost
└── platform_extras: map<string, string>    ── escape hatch (long tail)

UnifiedPublishResult   (Executor → API, version=2)
├── task_id / status (PENDING/PROCESSING/SUCCEEDED/PARTIAL_SUCCESS/FAILED/RETRYING)
├── platform_post_id / platform_post_url / published_at
├── failure: failed_reason / failed_error_code / retry_count / next_retry_at
├── media_results:        repeated MediaPublishResult        ── per-asset upload outcome
├── post_publish_results: repeated PostPublishActionResult   ── per-action outcome
├── initial_metrics?:     PublishMetrics                     ── views/likes/comments
└── raw_response_json?:                                       ── debug payload
```

Single plan composability: a Facebook post that wants *caption text + 3 carousel images + a cover video* fills all three lists in one `UnifiedAiPubInput`. Scheduler dispatches one `AiTask` per spec entry (× spec.count); each completed task pushes its asset into `UnifiedPublishContent.media[]` / `texts[]` with `ai_task_id` wired up. No more "is this a content_gen plan or a video_gen plan?" branching.

**`MediaItem`** carries `(kind, source, status, role, order, url, ai_task_id?, mime?, w/h/duration?)` so that:

- one bucket replaces v1's videos / images / audio split (extensible to GIF, livestream)
- `role` (PRIMARY / COVER / START_FRAME / END_FRAME / CAROUSEL_ITEM / REFERENCE / BACKGROUND / SOURCE) replaces v1's role-by-field-name pattern
- `status: PENDING/READY/FAILED` + `ai_task_id` lets a task carry **placeholders** for media still being generated, instead of forcing the whole task to wait
- `order` makes carousels / multi-frame deterministic
- `source: AI_GENERATED | USER_UPLOADED | REFERENCE` powers cost tracking, retry semantics, and "is this safe to re-emit on retry"

**`TextBlock`** has the same `(role, value, source, order)` discipline so that an executor never has to parse strings to decide "is this the title or the caption".

**`platform_extras: map<string, string>`** is the escape hatch — per-platform booleans (`is_nsfw`, `use_markdown`, `disclose_branded`) ship as map entries until a future minor version promotes them to typed fields. This is the pattern that Reddit's drift teaches us: **don't fight the JSONB**, give it a legal home.

**Generation specs** carry their own engine config. `VideoGenerationSpec` holds `video_config` / `vidu_config` / `seedance_config` / `reference_video` directly — no plan-level inheritance, every spec is fully self-describing. This kills v1's ambiguity around "which `seedance_config` wins when there's both a plan-level and a task-level one".

**Scalar-vs-array convention inside specs**: every *data* field on a spec is `repeated` even when v2.0 callers only pass length 1. This includes `prompts[]`, `start_image_urls[]`, `end_image_urls[]`, `reference_image_urls[]`. Configuration fields stay scalar (`model`, `count`, dimensions, mode, role_hint, engine sub-messages). Rationale: every multi-prompt / multi-keyframe extension we can already foresee (Vidu `multi_frame`, system+user prompt pairs, per-variation prompts, shot-list prompts) becomes a default-value change rather than a schema bump.

---

## 3. Per-platform / per-content-type mapping (covers all 13 v1 combos)

Goal: prove v2 can express every existing (`platform_id` × `content_type` × `plan_type`) combination without losing fidelity. For each row, **Input** is what the user/frontend submits; **Output** is what the executor receives per task.

| # | platform | content_type | plan_type | Input — `UnifiedAiPubInput` | Output — `UnifiedPublishContent` per task |
|---|---|---|---|---|---|
| 1 | TikTok | video | single_video | `video_generations[1×{prompts=[…], model="veo-3.1", mode=text_to_video, role_hint=PRIMARY}]` + `text_generations[1×{prompts=[…], count=N, target_roles=[TITLE,CAPTION,HASHTAG,LOCATION]}]` | `media[1×{VIDEO,AI_GENERATED,PRIMARY,ai_task_id}]` + `texts[TITLE,CAPTION,HASHTAG…,LOCATION?]` |
| 2 | Facebook | video | single_video | same as #1 | same as #1 |
| 3 | Facebook | reel | single_video | #1 + optional `image_generations[1×{prompts=[…], role_hint=COVER}]` | #1 output + `media[{IMAGE,*,COVER}]` |
| 4 | Instagram | reel | single_video | same as #3 | same as #3 |
| 5 | Facebook | post | batch_text | `image_generations[1×{prompts=[…], count=K, role_hint=CAROUSEL_ITEM}]` + `text_generations[1×{prompts=[…], count=N, target_roles=[CAPTION,HASHTAG]}]` | `media[K×{IMAGE,*,CAROUSEL_ITEM,order=i}]` + `texts[CAPTION,HASHTAG…]` |
| 6 | Instagram | post | batch_text | same as #5 (`requiresMedia=true`) | same as #5 |
| 7 | Facebook | story | batch_text | `image_generations[1×{prompts=[…], role_hint=PRIMARY}]` OR `video_generations[1×{prompts=[…], role_hint=PRIMARY}]` + optional `text_generations[…CAPTION]` | `media[{IMAGE\|VIDEO,*,PRIMARY}]` + `texts[CAPTION?]` |
| 8 | Instagram | story | batch_text | same as #7 | same as #7 |
| 9 | TikTok | profile | account_grooming | `image_generations[1×{prompts=[avatar_prompt], role_hint=PRIMARY, reference_image_urls=[]}]` + `text_generations[1×{prompts=[…], target_roles=[TITLE,BODY]}]` (name + bio) | `media[{IMAGE,AI_GENERATED,PRIMARY}]` + `texts[TITLE=name, BODY=bio?]` + `platform_extras{...}` |
| 10 | Reddit | post | reddit_text | `text_generations[1×{prompts=[…], target_roles=[TITLE,BODY]}]` + `platform_config.reddit_config_v2{subreddit, …}` | `texts[TITLE,BODY,SUBREDDIT,FLAIR?]` + `platform_extras{is_nsfw, is_spoiler, use_markdown, reddit_post_type=TEXT}` |
| 11 | Reddit | post | reddit_image | `text_generations[1×{prompts=[…], target_roles=[TITLE]}]` + `image_generations[1×{prompts=[…], count=K, role_hint=PRIMARY, reference_image_urls=[…]?}]` (or user uploads in `initial_media`) + `reddit_config_v2` | `texts[TITLE,SUBREDDIT,FLAIR?]` + `media[K×{IMAGE,AI_GENERATED\|USER_UPLOADED,PRIMARY,order=i}]` + `platform_extras{reddit_post_type=IMAGE, …}` |
| 12 | Reddit | post | reddit_link | `text_generations[1×{prompts=[…], target_roles=[TITLE]}]` + `reddit_config_v2{link_url}` (link goes to `initial_media`-style pre-known field) | `texts[TITLE,SUBREDDIT,FLAIR?]` + `links[{REDDIT_TARGET,url}]` + `platform_extras{reddit_post_type=LINK, …}` |
| 13 | (Seedance) any video | (any) | single_video | `video_generations[1×{prompts=[…], model="doubao-seedance-2-0-260128", seedance_config={mode, duration, aspect_ratio, image_urls, …}, start_image_urls=[…]?, reference_image_urls=[…]?}]` (engine config now lives ON the spec) + initial reference media in `initial_media[]` | `media[1×{VIDEO,AI_GENERATED,PRIMARY,ai_task_id, provider_asset_uri?}]` |
| 14 *(new)* | any | video / reel | future Vidu `multi_frame` | `video_generations[1×{prompts=[shot1, shot2, shot3], start_image_urls=[k1, k2, k3], generation_mode="multi_frame", model="vidu-q1"}]` — multi-keyframe natively supported | `media[1×{VIDEO,AI_GENERATED,PRIMARY}]` |

**Result**: 13/13 cover, including Reddit's three subtypes which currently live entirely in JSONB extras outside the v1 typed protocol. The combined-output cases (TikTok video + cover image, FB reel + cover, FB post with both carousel images and a video) are now expressible by populating multiple spec lists in one plan instead of needing to chain plans.

---

## 4. Output structure deep-dive — `UnifiedPublishContent` + `UnifiedPublishResult`

The first DRAFT modeled output as just `media[] + texts[] + links[] + platform_extras + scheduled_at`. Stress-testing against eight real platform shapes (TikTok video w/ music + product link, IG carousel w/ photo tags + collaborators, IG Reel w/ pinned-first-comment, FB scheduled post in PT timezone, Reddit crosspost, multi-language video w/ SRT, branded-content disclosure, draft-then-schedule) surfaced six gaps that v1 was filling by stuffing JSONB into `platform_extras`. v2 closes each gap with a typed sub-message — and keeps `platform_extras` strictly for the long tail.

### 4.1 What's now first-class on `UnifiedPublishContent`

| Concern | v1 | v2 |
|---|---|---|
| Music / product / location / topic tags | ad-hoc keys in JSONB extras | `tags: repeated EntityTag` (kind ∈ MUSIC/PRODUCT/LOCATION/TOPIC/BRAND_PARTNER) with optional byte-offset positioning OR normalized media coords |
| User mentions (most-frequent tagging case) | parsed out of caption text by the executor | `mentions: repeated UserMention` with `(handle, platform_user_id?, text_index?, byte_offset?, byte_length?, hidden?)` — split from `EntityTag` because mentions deserve byte-positioned typing + per-platform validation |
| Visibility, NSFW, allow_comments, allow_duet, allow_remix, AI-disclosure, … | string-encoded entries in `platform_extras` | `behavior: PublishBehavior` (typed `Visibility` enum + booleans for the well-known toggles) — `behavior.extras` map remains for the long tail |
| Scheduled posts with timezone | `scheduled_at: string` (silently UTC) | `schedule: PublishSchedule { scheduled_at, timezone?, save_as_draft }` — IANA timezone preserved, draft mode supported on platforms that have it |
| Auto-first-comment, pin-to-profile, share-to-story, crosspost, webhook | not modeled — executor had no slot for "do these things AFTER publishing" | `post_publish: repeated PostPublishAction` with typed `PostPublishActionKind` enum and per-action result reporting |
| Multi-language subtitles | not modeled — captions had to be baked into the video | `MediaItem { kind=SUBTITLE, language="en", parent_media_index=0 }` — multiple SRT tracks per video, language-tagged |
| Carousel cover image | role inferred by position | `MediaItem { kind=IMAGE, role=COVER, parent_media_index=2 }` — explicit binding to a specific carousel item |

### 4.2 Why split `UserMention` out of `EntityTag`

Both could fit in one `EntityTag` enum, but mentions have three properties no other tag kind has:

1. **Frequency** — every IG/TikTok caption has 0..30 mentions; music/product tags appear on a fraction of posts.
2. **Resolution** — mentions need `@handle → platform_user_id` lookup at publish time (often the executor's responsibility); other tags are usually pre-resolved by the user.
3. **Hidden semantics** — IG collaborator invites and FB co-authors *don't appear* in caption text but *do* ping the user. `UserMention.hidden = true` models this directly; cramming it into `EntityTag.extras["hidden"]` would re-introduce the v1 anti-pattern we're trying to remove.

So the schema is: `mentions[]` for users, `tags[]` for everything else. Frontend forms map cleanly to this split (one mention picker, one tag picker).

### 4.3 Boundary: when to use `behavior.extras` / `platform_extras`

The escape hatch survives but with a clear graduation rule:

- New per-platform toggle that appears on **one** platform → `platform_extras["x"]` first, promote to typed if it spreads.
- New toggle that appears on **two or more** platforms → goes straight into `PublishBehavior` as a typed optional bool.
- Anything that ever reads `is_nsfw`, `allow_comments`, `visibility`, `is_spoiler`, `disclose_branded`, `allow_duet`, `allow_stitch`, `allow_remix`, `share_to_facebook` MUST go through `behavior.*` — these are no longer allowed in `platform_extras`.

### 4.4 `UnifiedPublishResult` — Executor → API

v1's `ExecutorTaskStatusUpdate { task_id, status, result_url?, error_message? }` is enough for "video uploaded, here's the URL", but breaks on:

- Carousel posts (10 separate uploads, some can fail)
- Posts with post-publish actions (e.g. AUTO_FIRST_COMMENT may fail while the primary post succeeds → `PARTIAL_SUCCESS`)
- Retry context for the API to decide "give up" vs "schedule another attempt"
- Initial-metrics snapshot (some platforms return view-count immediately)

`UnifiedPublishResult` adds:

- **Status taxonomy**: `PENDING / PROCESSING / SUCCEEDED / PARTIAL_SUCCESS / FAILED / RETRYING` (vs v1's free-form string)
- **Identification**: `(platform_post_id, platform_post_url, published_at)` instead of a single `result_url`
- **Failure detail**: `(failed_reason, failed_error_code, retry_count, next_retry_at)` so the API can drive its own retry/backoff
- **Per-asset detail**: `media_results[]` referencing back into `UnifiedPublishContent.media[]` by index, with each asset's own platform_id + status
- **Per-action detail**: `post_publish_results[]` mirroring the `post_publish[]` actions with succeeded/failed_reason
- **Initial metrics**: `PublishMetrics { views, likes, comments, shares, saves, plays, snapshot_at }` — populated on platforms that return them at publish; otherwise filled in by the patrol job
- **Raw response**: opaque `raw_response_json` string, truncated, kept for debug only — never parsed by the API

The result message reuses `MediaStatus` and `PostPublishActionKind` from the content-side schema so there is no enum duplication.

### 4.5 What stays in `platform_extras`

After the rewrite, the **only** things that legitimately live in `platform_extras` are:

- `reddit_post_type` (`TEXT`/`LINK`/`IMAGE`) — Reddit-internal subtype that doesn't map to any other platform
- `reddit_flair_id` — UUID-shaped flair handle when not using `texts[FLAIR]`
- TikTok `commerce_link_id` — pre-existing TikTok Shop product binding (legacy path)
- IG `cover_thumbnail_offset_ms` — IG-specific cover-from-frame option
- TikTok `music_id_override` — when `tags[]` carries a TikTok music tag but the caller wants to override volume/start

Everything else has a typed home now. If a new key shows up in `platform_extras` and applies to ≥2 platforms, it's promoted in the next minor release.

---

## 5. v1 → v2 field-by-field migration map

| v1 (will keep working) | v2 |
|---|---|
| `AiPubInput.video_prompt` | `UnifiedAiPubInput.video_generations[i].prompts[0]` (length-1 list) |
| `AiPubInput.content_prompt` | `UnifiedAiPubInput.text_generations[i].prompts[0]` |
| `AiPubInput.prompt` (legacy) | dropped — callers must populate the relevant spec's `prompts[]` |
| (no v1 equivalent — image generation was implicit) | `UnifiedAiPubInput.image_generations[i].prompts[]` |
| `AiPubInput.default_images.start_frame_url` | `UnifiedAiPubInput.initial_media[ {IMAGE, USER_UPLOADED, START_FRAME, order=0} ]` (plan-wide frame for ALL videos) OR `UnifiedAiPubInput.video_generations[i].start_image_urls[0]` (frame scoped to ONE video spec) |
| `AiPubInput.default_images.end_frame_url` | same with `END_FRAME` / `end_image_urls[0]` |
| `AiPubInput.account_images[acct_id].start_frame_url` | `UnifiedAiPubInput.account_media[acct_id].media[ {IMAGE, USER_UPLOADED, START_FRAME} ]` |
| `AiPubInput.reference_images[]` | `UnifiedAiPubInput.initial_media[ {IMAGE, REFERENCE, REFERENCE, order=i} ]` (plan-level refs) OR `UnifiedAiPubInput.video_generations[i].reference_image_urls[]` (refs scoped to one spec) |
| `AiPubInput.reference_video` | `UnifiedAiPubInput.video_generations[i].reference_video` (now per-spec) |
| `AiPubInput.video_config` | `UnifiedAiPubInput.video_generations[i].video_config` (per-spec, no plan-level default) |
| `AiPubInput.vidu_config` | `UnifiedAiPubInput.video_generations[i].vidu_config` |
| `AiPubInput.seedance_config` | `UnifiedAiPubInput.video_generations[i].seedance_config` |
| `AiPubInput.platform_config.reddit_config` | `UnifiedAiPubInput.platform_config.reddit_config_v2` (same `RedditPostConfig` message reused) |
| (no v1 equivalent — count was implicit per plan_type) | `text_generations[i].count` / `image_generations[i].count` / `video_generations[i].count` (explicit) |
| (no v1 equivalent — model was global config) | `*_generations[i].model` (per-spec model id) |
| `AiPubTaskContent.title` | `UnifiedPublishContent.texts[ {TITLE, value, source} ]` |
| `AiPubTaskContent.text_content` | `UnifiedPublishContent.texts[ {CAPTION, value, source} ]` |
| `AiPubTaskContent.hashtags[]` | `UnifiedPublishContent.texts[ {HASHTAG, value, order=i} ]` (one per item) |
| `AiPubTaskContent.location` | `UnifiedPublishContent.texts[ {LOCATION, value} ]` |
| `AiPubTaskContent.video_url` | `UnifiedPublishContent.media[ {VIDEO, AI_GENERATED, PRIMARY, url, ai_task_id} ]` |
| `AiPubTaskContent.start_frame_url` | `UnifiedPublishContent.media[ {IMAGE, *, START_FRAME} ]` |
| `AiPubTaskContent.end_frame_url` | same with role=END_FRAME |
| `AiPubTaskContent.reference_image_urls[]` | `UnifiedPublishContent.media[ {IMAGE, REFERENCE, REFERENCE, order=i} ]` |
| `AiPubTaskContent.video_generation_needed` | derivable from `media[].status == PENDING` — dropped |
| `AiPubTaskContent.video_submitted` | derivable from `media[].status == READY \|\| FAILED` + `ai_task_id` lookup — dropped |
| `AiPubTaskContent.ai_task_id` | promoted onto each `MediaItem.ai_task_id` (1-to-many is now expressible) |
| `RedditPublishContent.title / body` | `UnifiedPublishContent.texts[ {TITLE, BODY} ]` |
| `RedditPublishContent.subreddit` | `texts[ {SUBREDDIT} ]` |
| `RedditPublishContent.flair_text` | `texts[ {FLAIR} ]` |
| `RedditPublishContent.image_urls[]` | `media[ {IMAGE, *, PRIMARY, order=i} ]` |
| `RedditPublishContent.link_url` | `links[ {REDDIT_TARGET, url} ]` |
| `RedditPublishContent.is_nsfw` | `behavior.is_nsfw` (typed bool) |
| `RedditPublishContent.is_spoiler` | `behavior.is_spoiler` (typed bool) |
| `RedditPublishContent.use_markdown` | `platform_extras["use_markdown"]` (Reddit-only, stays in extras) |
| `RedditPublishContent.is_brand_affiliate` | `behavior.disclose_branded_content` (typed bool) |
| `RedditPublishContent.reddit_post_type` | `platform_extras["reddit_post_type"]` (Reddit-only enum, stays) |
| `AccountGroomingTaskContent.generated_name` | `texts[ {TITLE, value, source} ]` |
| `AccountGroomingTaskContent.generated_bio` | `texts[ {BODY, value, source} ]` |
| `AccountGroomingTaskContent.avatar_url` | `media[ {IMAGE, AI_GENERATED, PRIMARY} ]` |
| `AccountGroomingTaskContent.avatar_prompt` | `UnifiedAiPubInput.image_generations[i].prompts[]` |
| (no v1 equivalent — caption text inline) | `mentions[ {handle, platform_user_id?, text_index?, byte_offset?, byte_length?, hidden?} ]` |
| (no v1 equivalent — music_id in `platform_extras`) | `tags[ {kind=MUSIC, platform_id, display_name} ]` |
| (no v1 equivalent — product tag in `platform_extras`) | `tags[ {kind=PRODUCT, platform_id, display_name, media_index, media_x, media_y} ]` |
| (no v1 equivalent — location string in `texts[LOCATION]`) | still in `texts[LOCATION]` for free-text; `tags[ {kind=LOCATION, latitude, longitude, …} ]` for structured place tags |
| `AiPubTaskContent.scheduled_at` (string, UTC-implied) | `schedule.scheduled_at` (UTC, explicit) + `schedule.timezone` (IANA) + `schedule.save_as_draft` |
| (no v1 equivalent — auto-first-comment was unsupported) | `post_publish[ {kind=AUTO_FIRST_COMMENT, comment_text} ]` |
| (no v1 equivalent — pin-to-profile was unsupported) | `post_publish[ {kind=PIN_TO_PROFILE} ]` |
| (no v1 equivalent — Reddit crosspost was a separate plan) | `post_publish[ {kind=CROSSPOST, targets[]} ]` |
| (no v1 equivalent — multi-language captions baked in) | `media[ {kind=SUBTITLE, language="en", parent_media_index=i, url} ]` |
| `ExecutorTaskStatusUpdate.task_id` | `UnifiedPublishResult.task_id` |
| `ExecutorTaskStatusUpdate.status` (free-form string) | `UnifiedPublishResult.status` (typed `PublishResultStatus` enum) |
| `ExecutorTaskStatusUpdate.result_url` | `UnifiedPublishResult.platform_post_url` (+ `platform_post_id`, `published_at`) |
| `ExecutorTaskStatusUpdate.error_message` | `UnifiedPublishResult.failed_reason` (+ `failed_error_code`, `retry_count`, `next_retry_at`) |
| (no v1 equivalent — per-asset upload outcomes lost) | `UnifiedPublishResult.media_results[ {media_index, status, platform_asset_id, platform_asset_url, failed_reason?} ]` |
| (no v1 equivalent — initial metrics fetched separately by patrol) | `UnifiedPublishResult.initial_metrics: PublishMetrics` |

---

## 6. The integration step (NOT in this draft)

This DRAFT freezes the message shapes only. The following diff is intentionally **not applied** — it's the next PR after consumer code is ready:

```diff
 // Executor publish task
 message ExecutorPublishTask {
   ...
   oneof task_content {
     AiPubTaskContent publish_content = 10 [ json_name = "publish_content" ];
     AccountGroomingTaskContent grooming_content = 11 [ json_name = "grooming_content" ];
     RedditPublishContent reddit_content = 12 [ json_name = "reddit_content" ];
+    // v2 unified payload (see "v2 — Unified Content Schema" section).
+    // Selector is the inner `version` field of UnifiedPublishContent.
+    UnifiedPublishContent unified_content = 13 [ json_name = "unified_content" ];
   }
 }
```

Adding a oneof variant is wire-compatible: existing consumers fall through to a default branch and the API simply doesn't emit `unified_content` until the worker side is ready.

---

## 7. Rollout plan (3 phases, each independently deployable)

### Phase 1 — Protocol freeze (this PR)

- ✅ Add v2 messages to `proto/aipub.proto` (done)
- ✅ Bump `AiPubProtocolVersion` enum to include `AIPUB_VERSION_2` (done)
- ✅ `make validate` passes
- ⏭ **Don't** run `make sync` yet — that propagates generated code into Rust API, scheduler, agent, executor; do it only once consumers are willing to take the new types.

### Phase 2 — Backend dual-read

1. `make sync` to propagate `UnifiedPublishContent` / `UnifiedAiPubInput` Rust + Python types into all consumers.
2. `glance_mind_rust/crates/api/src/dto/aipub_dto.rs` `CreatePlanDto.ai_input` stays `Option<JsonValue>`; the **service layer** parses it as `UnifiedAiPubInput` first (when `version == 2`) and falls back to v1 `AiPubInput` parsing otherwise.
3. `aipub_service.rs` validation: write a `validate_unified()` that walks `texts[] / media[] / links[] / tags[] / mentions[] / behavior / schedule / post_publish[]` and enforces per-platform constraints (e.g. Reddit LINK requires `links[].role == REDDIT_TARGET`; IG Post `media[]` length ≥ 1 and ≤ 10 carousel items; AUTO_FIRST_COMMENT requires `comment_text.is_some()`; CROSSPOST requires `targets.len() ≥ 1`; SUBTITLE media requires `parent_media_index.is_some()` AND `language.is_some()`).
4. Scheduler: when assembling per-task content, **prefer** writing v2 `UnifiedPublishContent` to `gm_aipub_tasks.content`; keep v1 writer as fallback.
5. Add `oneof unified_content` to `ExecutorPublishTask` (the small diff above).

### Phase 3 — Executor + frontend cutover

- Executor: implement the **currently missing** `_publish_content` for IG Post / FB Post / Reddit by reading `UnifiedPublishContent.media[] / texts[] / links[] / tags[] / mentions[] / behavior / schedule / post_publish[]`. This is the place that pays the largest immediate dividend — v1's executor only really runs `_publish_video`, so unifying the protocol unlocks all the non-video paths in one go.
- Executor: emit `UnifiedPublishResult` instead of `ExecutorTaskStatusUpdate` when consuming a `version=2` task. Per-asset upload outcomes go into `media_results[]`; per-action outcomes into `post_publish_results[]`; initial metrics (when the platform returns them at publish time) into `initial_metrics`.
- Frontend: build a `usePublishContentDraft` hook + `PublishContentForm` component that produces `UnifiedAiPubInput` directly. Add typed sub-forms for `mentions[]`, `tags[]`, `behavior`, `schedule`, `post_publish[]`. Migrate `AIPubPlanCreate.tsx` page-by-page; old plans keep reading via the v1 detail-page types.
- Once 100% of new plans are v2, mark v1 as deprecated and schedule removal one major version later.

### Backout

Every phase is independently reversible:

- Phase 1 — `git revert` the proto edit; nothing else has been generated/synced.
- Phase 2 — feature-flag the v2 service path off (`AIPUB_V2_WRITES=false`); fall back to v1-only write; consumers keep reading v1.
- Phase 3 — front-end form sets `version=1` and emits v1 shape; backend dual-write keeps both alive.

---

## 8. Explicit non-goals of v2

To keep the draft small and shippable, v2 deliberately does **NOT**:

- Unify `AiTaskInput` / `AiTaskResult` (the AI-job-level contract). Those keep their v1 shape; `MediaItem.ai_task_id` is the bridge. A separate v3 can address them once v2 stabilises.
- Migrate JSONB column types in PostgreSQL. Columns stay `Jsonb`; partitioning is by the inner `version` field, not by a new column.
- Support video editing / extending workflows beyond what `MediaRole.SOURCE` already encodes; Seedance edit/extend modes still ride on `seedance_config` for engine-specific knobs.
- Define rich cost / token accounting on `MediaItem`. Future work.
- Define long-term metrics tracking on the result side. `UnifiedPublishResult.initial_metrics` is the publish-time snapshot only; ongoing metrics still live in the patrol pipeline tables.
- Model multi-step workflows where one plan produces N posts published at different times — each post is still a separate task; `post_publish[]` only covers things the executor does in the same publish session.

---

## 9. Open questions for review

1. **`version` as `int32` vs `AiPubProtocolVersion` enum** — current draft uses `int32` for forward-compat (consumers can recognise `version > 2` as "newer than I know"); enums in proto3 silently accept unknown numeric values, so either works. Final call before `make sync`.
2. **`ai_task_id` cardinality on `MediaItem`** — current model is one `ai_task_id` per `MediaItem`. With per-spec `count > 1`, a single `VideoGenerationSpec` produces multiple `MediaItem`s, but each one still maps to a single AiTask attempt. If providers like Vidu return 4 candidates from one API call, do we want `(ai_task_id, variation_index)` to deduplicate cost tracking? Could ship in v2.1.
3. **Should `UnifiedAiPubInput.platform_config.reddit_config_v2` be a fresh `RedditPostConfigV2`** (so we can drop deprecated fields), or keep reusing the v1 `RedditPostConfig` message? Current draft reuses for minimal blast radius — open to splitting.
4. **`*GenerationSpec.target_roles` as `string[]` vs `repeated TextRole`** — current draft uses string-name list (e.g. `["TITLE","CAPTION","HASHTAG"]`) so that consumers parsing JSON don't need the enum table. Switching to `repeated TextRole` enum is more type-safe but couples the wire format to the enum definition. Decide before sync.
5. **Should empty-input plans be valid?** v2 currently requires "at least one non-empty `*_generations` list OR at least one `initial_media` item". Reddit LINK posts have no media generation but do have `links[]` — the link URL lives in `RedditPostConfig.link_url` at input time and is materialized into `links[]` at task creation. Document this constraint in the validator, or model it differently (e.g. promote `links[]` to plan input)?
6. **Spec-level vs plan-level engine config inheritance** — current draft: NO inheritance, every `VideoGenerationSpec` is fully self-describing. This is verbose when a plan has 5 video generations all with the same Seedance config. A future v2.1 could add an optional plan-level `default_video_config` that specs `merge` over. Not in v2.0.
7. **`PublishBehavior.visibility`** — only one `Visibility` per post is modeled. Some platforms (e.g. FB) support per-audience targeting (specific friend lists, custom audiences). Pull into `PublishBehavior.audience_filter` later if needed; out of scope for v2.0.
8. **`PostPublishAction` ordering & failure policy** — currently the executor runs them sequentially. Should we add `continue_on_failure: bool` per action so e.g. CROSSPOST can proceed even if AUTO_FIRST_COMMENT fails? Current behavior: each action runs independently and reports its own result; failure of one does not stop the next (matches the `PARTIAL_SUCCESS` status semantics). Document explicitly in v2.0 release notes.
9. **`UnifiedPublishResult.media_results[]` indexing** — uses `media_index` (uint32) referencing the original `UnifiedPublishContent.media[]`. If the executor re-orders or drops media (e.g. one of 10 carousel images fails to upload and is silently skipped), the index alignment can break. Alternative: include the full `MediaItem` again in the result. Current draft: keep index-based, document that executor MUST NOT reorder.
10. **`raw_response_json` size cap** — currently no enforced cap in the proto. Recommend documenting "≤ 4 KiB; truncate with `…[truncated]` suffix" in the executor SDK rather than encoding the limit in the schema.
11. **Subtitle role on `MediaItem`** — `MediaKind.SUBTITLE` is added but `MediaRole` doesn't have a subtitle-specific role; current intent is to use `MediaRole.UNSPECIFIED` for SUBTITLE since the role is fully implied by `kind`. Should we add `MEDIA_ROLE_SUBTITLE_TRACK` for symmetry? Current draft: no, because it's redundant with `kind=SUBTITLE`.

---

## 10. Files touched by this draft

- `proto/aipub.proto` — additive: bumped `AiPubProtocolVersion` enum, appended "v2 — Unified Content Schema" section, expanded `MediaKind` (+SUBTITLE), expanded `MediaItem` (+language, +parent_media_index), added output-side enums + messages (`Visibility`, `EntityTagKind`, `PostPublishActionKind`, `PublishResultStatus`, `EntityTag`, `UserMention`, `PublishBehavior`, `PublishSchedule`, `PostPublishAction`, `MediaPublishResult`, `PostPublishActionResult`, `PublishMetrics`, `UnifiedPublishResult`), rewired `UnifiedPublishContent` (added `tags`, `mentions`, `behavior`, `schedule`, `post_publish`; removed flat `scheduled_at`). No existing message reused field number.
- `proto/AIPUB_V2_DRAFT.md` — this file (updated to cover output structure deep-dive + `UnifiedPublishResult`).

`generated/rust/`, `generated/python/`, and downstream consumer copies are **not** touched — `make sync` is intentionally deferred to Phase 2.
