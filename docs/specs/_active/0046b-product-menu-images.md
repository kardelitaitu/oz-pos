# Spec 0046-bis — Product & Menu-Item Images (Optimized for Low-End Android)

**Status:** draft for review · **Created:** 2026-08-31 · **Scope:** desktop-client, tablet-client, ui/, oz-core, oz-api
**Related:** 0048 (workspace model), sync-conflict-dead-letter-recovery (0045), subscription tiers (staff/licensing untouched)

---

## 1. Goal

Let a merchant attach images to retail products and resto menu items —
**a menu item always has exactly 1 image; a product has 1 primary plus at
most 4 alternatives** — rendered flawlessly on cheap Indonesian tablets and
synced through the existing cloud topology, without bloating the SQLite/PG
rows, the IPC bridge, or the WebView heap.

Upload happens **at assignment time**: the merchant picks/edits an image in
the product or menu editor, and that action uploads + ingests the bytes.

**Non-goals (v1):** zoom/full-screen viewer, category images, printer
artwork on receipts/KDS chits, tablet camera capture (desktop editors
author; tablets render).

## 2. Ground truth in the codebase today

| Fact | Evidence |
|---|---|
| `products` table has **no image column**; `version INTEGER` exists (sync-friendly) | `crates/oz-core/migrations/20260813_init.sql:439-457` |
| Menu items **are products** (resto filters the same table; `kitchen_zone`, `product_type`) | `RestaurantMenu.tsx:921` over `restaurantProducts` |
| CSP in **both** apps already allows `img-src … asset: https://asset.localhost` — pre-armed but the **protocol itself is not enabled** (no `assetProtocol` key anywhere) | `apps/desktop-client/tauri.conf.json:29`, `apps/tablet-client/tauri.conf.json:15` |
| `react-window` v2.2.7 is already a dependency | `ui/package.json:50` |
| `RetailProductGrid` paginates (`pagedProducts`); `RestaurantMenu` renders `filtered.map` unvirtualized | `RetailProductGrid.tsx:611`, `RestaurantMenu.tsx:921` |
| Cloud product routes exist for sync (`list_products`, `create_product`, …) | `crates/oz-api/src/routes/products.rs` |
| Cloud byte store = Northflank volume (`VOLUME ["/data"]`, `OZ_DB_PATH=/data/oz-pos.db`) | `Dockerfile.server`, `docs/plans/northflank-p1-p7-plan.md` |
| Prod service `oz-cloud` = one combined container: **supervisord {caddy, license/PocketBase, sync/oz-api}** on the single `/data` volume; Caddy terminates auto-TLS with HTTP/2+3 | `Dockerfile.unified:179-215`, `apps/unified/supervisord.conf`, `.github/workflows/deploy.yml` |

## 3. Architecture (the three rules, made concrete)

### 3.1 Serve via Tauri v2 asset protocol — never base64 through IPC

- Enable Tauri v2's **built-in** asset protocol (it *is* the custom-scheme
  streaming path the guideline asks for — no hand-rolled handler needed):
  ```jsonc
  // apps/{desktop,tablet}-client/tauri.conf.json → app.security
  "assetProtocol": {
    "enable": true,
    "scope": ["$APPCACHE/images/**"]
  }
  ```
  CSP is already correct (`asset:` on Android, `https://asset.localhost` on
  Windows desktop).
- UI never receives bytes. A card renders
  `convertFileSrc(cachePath)` → `<img src="http://asset.localhost/…">`, and the
  Android WebView streams from disk asynchronously.
- **Image URLs are immutable cache keys** — filenames are content-addressed,
  so a new upload produces a new URL and `<img>` cache invalidation is free.

### 3.2 Rust-owned flat disk store; DB stores only hashes — content-addressed, deduped

- Directory: `$APPCACHE/images/` (per-profile app cache — OS-managed
  location, safe to wipe; cloud twin: `$OZ_IMAGE_DIR`).
- **File name = `{hash16}.webp`** (first 16 hex chars = first 64 bits of the
  sha-256 of the transcoded bytes; final-review fix — 32-bit hash8 has a
  birthday-bound collision at ~65k images and the cloud store is shared
  across tenants, so two products could silently resolve to the wrong
  image; 64 bits puts a 300k-image store at ~1e-9 collision probability).
  Content-addressed means:
  - the same image applied to several products/slots is stored **once**
    locally and once in the cloud (alternatives re-use is common in menus);
  - paths are device-independent, so "where is the file" never enters the DB;
  - GC is a reverse-index check, not per-file bookkeeping.
- **Data model (both migrations, sqlite + pg):**
  ```sql
  -- primary image, denormalized for grid reads + free row-sync ride
  ALTER TABLE products ADD COLUMN image_hash TEXT;          -- slot 1 mirror
  -- alternatives + authoritative slot list (1 = primary, 2..5 = alternatives)
  CREATE TABLE product_images (
      product_id TEXT NOT NULL REFERENCES products(id) ON DELETE CASCADE,
      slot       INTEGER NOT NULL CHECK (slot BETWEEN 1 AND 5),
      hash       TEXT NOT NULL,
      position   INTEGER NOT NULL DEFAULT 0,   -- display order of alternatives
      updated_at TEXT NOT NULL,
      PRIMARY KEY (product_id, slot)
  );
  ```
  - `products.image_hash` is a **mirror of slot 1** kept in the same
    transaction (grid queries read the product row only; no JOIN on the POS
    hot path). `product_images` is authoritative.
  - **Menu invariant** (`product_type = 'menu'`): exactly 1 image — enforced
    in the set/clear commands (clear refused if it would leave a menu item
    without a primary; UI hides the alternatives strip for menu items).
  - **Product invariant**: slots 1..5, alternatives ordered by `position`;
    clearing slot 1 while alternatives exist promotes the first alternative
    to primary (same transaction).
  - Assignments ride the product's existing `version` — any image-set change
    bumps `products.version`, so the delta sync ships the row + its image
    rows atomically per product.
- The DB never stores paths or bytes — hashes only — keeping rows light and
  indexable (guideline rule 3).

### 3.3 Ingest pipeline in Rust — one command per assignment

`products_set_image(product_id, slot, bytes)` (upload = apply, per decision):

1. **Sniff, don't trust:** validate magic bytes (WebP `RIFF….WEBP`, JPEG
   `FFD8FF`, PNG `89504E47`) — extension is ignored.
2. **Caps:** ≤ 5 MB raw input, decoded dimensions ≤ 4096² (denial-of-heap
   guard — `image` crate `ImageReader::with_guessed_format().limits()`).
3. **Transcode & resize:** decode → EXIF orientation applied → EXIF stripped →
   resize to **512 px longest edge** (decided — headroom for 2x-DPR tablet
   tiles) → encode **WebP q40**. Target output ≈ **10–25 KB** per image
   (aggressive but clean at tile render size), decoded ARGB ≈ **1 MB** per
   image in RAM — which is exactly why §3.5 virtualization is mandatory.
4. **Hash & write:** sha-256 → `{hash16}.webp` in the store (temp + atomic
   rename; pre-existing identical hash ⇒ skip the write, dedupe hit).
5. **Assign in one transaction (repo rule):** upsert
   `product_images(product_id, slot → hash)`, maintain the slot-1 mirror on
   `products.image_hash`, handle the promotion rule above, bump
   `products.version`. `products_clear_image(product_id, slot)` mirrors it.
6. **GC:** startup sweep — any file whose hash is referenced by **no**
   `products.image_hash` / `product_images.hash` row is deleted after a
   24 h grace period (an in-flight sync never mid-air-collides).

### 3.4 Sync: hashes first, bytes on demand (Northflank volume — decided)

- **Hashes sync like any column:** the product delta payload gains an
  `images: [{slot, hash, position}]` array; the byte store never participates
  in delta/conflict logic (immutable content-addressed files — the dead-letter
  recovery path of 0045 stays untouched).
- **Byte store: the Northflank persistent volume** (decided — not R2, not PG
  `bytea`). Image files live beside the DB at `OZ_IMAGE_DIR` (default
  `/data/images` in prod, `./data/images` in dev; oz-api `create_dir_all`s it on startup — no entrypoint change). Capacity math: ~20 KB per image (512 px q40) ⇒ the 6 GB volume is the
  ~300k-image *ceiling*, but the DB + pb_data share it — the practical cap
  is the 4 GB soft alert ⇒ **≈ 200k images** (a 5k-product catalog with
  full 5-image sets ≈ 500 MB, so ~40 such tenants fit under the alert).
  P4 ships the bytes-used metric (per §3.7 image_refs SUM).
- oz-api serves the bytes itself — **no new component**:
  - `PUT /api/v1/images` — the upload primitive (the batch lane in §3.6
    composes the same server-side logic; final-review fix: the gate is the
    *actual* catalog tier — bare JWT, matching `create_product` — NOT
    admin-key, which would break merchant self-service; images inherit the
    admin-key tier automatically when the D1 residual campaign extends to
    master data); ≤ 32 KB body; body must be the **transcoded** WebP; server
    re-verifies magic bytes + size and stores by the client-computed hash
    after recomputing sha-256 — hash mismatch ⇒ 409, a corrupt upload never
    enters the store). Atomic temp+rename **on the same volume**. Response:
    `{hash16}`.
  - `GET /api/v1/images/{hash16}` (tenant JWT is enough — hashes are
    unguessable and content is non-sensitive product art; strict hash-grammar
    validation kills directory traversal; immutable,
    `Cache-Control: max-age=31536000, immutable`; unknown hash ⇒ 404).
  - Natural dedupe: re-uploading an existing hash is a no-op success.
  - Caddy hygiene: the global encode directive must exclude `/api/v1/images*` — WebP is already compressed; re-compressing burns CPU for zero bytes saved.
- Tablet pull: a tiny **download manager** in the tablet's Rust shell — on
  catalog apply, for each referenced hash with no local file, queue a
  background GET (2 in flight, pooled keep-alive per §3.7; LRU eviction of the images dir at a
  configurable budget, default **64 MB** ≈ ~3k images ≈ ~600 products with
  full 5-image sets). Offline-first holds: missing image degrades to the
  existing colored-initial tile. POS tiles only ever need slot 1; the
  manager downloads primary images first, alternatives opportunistically.

### 3.5 UI: virtualize or the pixel RAM wins anyway

- The math that makes virtualization non-optional: the chosen 512 px variant
  decodes to ~1 MB of ARGB per tile (the guideline's 1.05 MB). A
  non-virtualized 300-item grid would pin ~300 MB of decoded bitmaps on a
  2 GB tablet even with 15 KB files on disk — the disk size is irrelevant;
  the decoded bitmap is what eats RAM.
- **POS grids** (`RetailProductGrid`, resto ordering screen) switch the
  product tiles to `react-window` v2 (`FixedSizeGrid`) — already in the
  dependency tree — so off-screen tiles **unmount entirely** and Android
  reclaims their decoded bitmaps. Tiles render **slot 1 only**.
- **Alternatives appear on interaction, not on the grid:** the product-detail
  strip / menu editor shows the 4 alternatives as small lazy thumbnails
  (`loading="lazy"` + `decoding="async"`), mounted only while open — never
  in the hot grid.
- Admin screens (`RestaurantMenu` catalog editor, `ProductLookupScreen`) get
  primary thumbnails lazily — full grid virtualization there is P3, not
  blocking.
- Placeholder: keep the existing colored-initial tile as the skeleton/miss
  state; no layout shift (fixed tile aspect-ratio). Menu editor enforces the
  always-1-image rule in its save flow.


### 3.6 Push & pull scheduling — batching + jitter (server efficiency)

Image bytes get their **own scheduler lane**, separate from the metadata
delta sync (§3.4): byte transfers are the expensive ops, so they must never
delay metadata sync or hold the DB lock. Both lanes run inside the existing
background sync daemon pattern (`sync_bootstrap` / tablet daemon).

**Push (desktop → cloud):**

- `products_set_image` enqueues into a persisted `image_push_queue`
  `(hash16 PK, size_bytes, attempts, next_attempt_at, enqueued_at)` — the
  apply action commits locally and returns immediately; the network leg is
  never on the UI path.
- A drain loop wakes on **`next_run = now + jitter(60 s .. 300 s)`** (full
  jitter — uniformly random per cycle, per device) and drains up to
  **16 images / 512 KB per batch** via `POST /api/v1/images:batch`
  (same gate as PUT — bare JWT today, per §3.4; length-prefixed binary frames). The server re-verifies
  magic bytes + sha-256 per file and answers per-hash
  `{hash, stored|duplicate|rejected}` — partial success is allowed, the
  queue keeps only the failures. One auth check, one connection, one
  transaction-free static-file store per batch.
- Idempotency makes retries free: re-uploading a hash the server already
  has answers `duplicate` = success.
- Failure backoff: `full jitter(60..300) × 2^attempts`, capped at 30 min;
  after 8 attempts the entry dead-letters (0045 pattern) and the desktop UI
  surfaces "image pending upload" on the product row.
- Server-driven nudge: the catalog delta response already tells the server
  which hashes product rows reference — it appends `missing_hashes` to the
  delta response so the desktop prioritizes exactly what the cloud lacks in
  its next push batch (no polling, no full-scan).

**Pull (tablet ← cloud; desktop only after a cache wipe):**

- The missing-hash set is computed at catalog apply (referenced hashes minus
  files present locally) — no polling of "what's new".
- A drain loop wakes on the same **jitter(60..300 s)** window and downloads
  at most **40 images (~1 MB) per cycle**, 2 GETs in flight, **slot-1
  primaries first**, alternatives opportunistically. Per-hash GET (not a
  multipart batch) keeps `Cache-Control: immutable` + per-hash 404
  granularity; the *scheduler* is the batch, not the response format.
- Same backoff/dead-letter ladder; a dead-lettered hash degrades the tile to
  the colored-initial fallback until the next catalog apply recomputes the
  missing set (self-healing).
- **Jitter seeding:** each device persists a random phase offset at first
  boot, so after a menu update the 100 tablets do not wake in the same
  second — the herd spreads across the full 4-minute window by construction.

**Server-load accounting (why these constants):** 100 devices × (1 push
batch + ≤ 40 GETs) spread over a 3-minute mean cycle ≈ **8 req/s steady
worst case** of immutable static-file serving on the volume — negligible for
the axum server, and the jitter removes the herd spike entirely. Constants
are env-tunable per repo convention: `OZ_IMG_PUSH_JITTER_SECS`,
`OZ_IMG_PULL_JITTER_SECS` (both `60..300`), `OZ_IMG_PUSH_BATCH` (16),
`OZ_IMG_PULL_CYCLE_CAP` (40).


### 3.7 SOTA server-efficiency stack (proposed for ratification)

"State of the art" here means: the strongest mechanism at each layer,
sized to this fleet (single-region, <= hundreds of devices, <= ~50k images
per tenant) — not maximal machinery. Verified topology first: devices hit
Caddy (HTTP/2/3, auto-TLS) -> the supervisord `sync` program = oz-api ->
the `/data` volume. Every mechanism below is chosen against that reality.

1. **Immutable content-addressing everywhere.** The sha-256 of the
   transcoded bytes is simultaneously the filename, the ETag, the DB value,
   and the cache key. Zero invalidation logic exists anywhere in the
   system, ever — every other mechanism below cheapens against this.
2. **Exact server-side content spine — `image_refs`.** The cloud keeps
   (tenant_id, hash, refcount, bytes), maintained transactionally when
   catalog deltas apply. Four features fall out of one table:
   `missing_hashes` in the delta response becomes an exact SQL
   set-difference (no bloom filters — at <= 50k hashes/tenant, exact is
   cheaper); PUT dedupe is refcount>0 AND file-exists => duplicate; GC is
   refcount=0 after grace; per-tenant byte accounting for the 4 GB soft
   alert is SUM(bytes).
3. **Conditional GETs.** `ETag: "{hash16}"` (free — computed at ingest) +
   `If-None-Match` on the puller => 304 with a string compare, no disk
   read. Protects every cache layer between the tablet LRU and the volume.
4. **HTTP/2 multiplexing + pooled keep-alive.** Free at Caddy; the Rust
   client (reqwest pool max 4/host, TLS session resumption) multiplexes
   the 2-in-flight pull GETs over one connection.
5. **Pack endpoint for cold start** — `GET /api/v1/images:pack?hashes=...`
   (<= 64 files / <= 2 MB, length-prefixed frames, immutable): a fresh
   tablet provisioning a 5k-image catalog does ~80 pack requests instead
   of 5k GETs. Steady state stays per-hash GET for cache granularity.
   Git packfile thinking, sized down.
6. **Backpressure contract.** The client ladder honors 429/503 +
   `Retry-After` — load shedding at Caddy or the OS passes through to the
   jitter/backoff loop with one line of client code.
7. **Compression hygiene.** No `Content-Encoding` on image routes (WebP
   is the compression); Caddy's encode directive excludes the image path
   (see §3.4).
8. **Deliberate non-features** (SOTA is also refusing the wrong machinery):
   no CDN/edge cache (single-region fleet; immutable + 304 already zeroes
   repeat egress, and a CDN would add an auth-bypass surface for
   tenant-scoped assets); no LAN tablet-to-desktop shortcut (audit finding
   c4 — LAN is plaintext today; adding mDNS+TLS is a campaign, not a
   phase); no application-level singleflight on GETs (the kernel page
   cache already coalesces concurrent identical reads — the only true miss
   is the first read after a restart); no bloom-filter reconciliation
   (exact SQL wins at this cardinality).

## 4. Work breakdown (each phase = conventional commits, tests first)

| Phase | Deliverable | Touches |
|---|---|---|
| **P1 — storage spine** | `products.image_hash` + `product_images` table (sqlite+pg migrations, models, repository), `products_set_image` / `products_clear_image` commands with sniff/limits/transcode/hash/atomic-write/txn + promotion + menu-invariant logic, unit tests (malformed magic, oversized, slot bounds, promotion, menu-clear refusal, dedupe) | `oz-core` (migrations, db/products), `desktop-client/src/commands`, tauri.conf `assetProtocol` |
| **P2 — UI** | `FixedSizeGrid` on POS rendering slot 1 via `convertFileSrc`, miss→initial-tile fallback; editor flows: assign-at-apply for menu (1 required) and product (primary + ≤4 alternatives with reorder); alternatives strip lazy-mounted; a11y (alt from product name, aria-busy) | `ui/src/features/retail`, `restaurant`, shared `ProductThumb` |
| **P3 — cloud sync** | `PUT/GET /api/v1/images` on the Northflank volume (`OZ_IMAGE_DIR`, admin-key gate, hash re-verification), image array in the product delta payload, image_refs content spine + missing_hashes nudge, push/pull scheduler lanes with batching + jitter (§3.6), pack endpoint for cold start, cloud GC via image_refs refcount=0 + grace sweep, tablet download manager (primary-first, LRU), sync tests | `oz-api`, `Dockerfile.unified` volume note, tablet-client Rust |
| **P4 — hygiene** | startup GC sweep (reverse-index + grace), metrics (bytes dir, hit/miss latency, 4 GB soft alert), e2e, docs (`docs/guides`), i18n strings for the editor UI (en+id FTL) | `desktop-client`, `ui`, docs |

**Est. budget:** P1–P2 make a fully local (single-device) feature — shippable
slice. P3 unlocks tablets. P4 is polish.

## 5. Decision log

1. ~~256 px q80 vs 512 px detail~~ **RESOLVED: single 512 px WebP q40
   variant** — decided 2026-08-31; 2x-DPR headroom on tablet tiles, q40
   keeps bytes at 10–25 KB so storage/sync surface stays tiny.
2. ~~R2 vs PG bytea~~ **RESOLVED: Northflank persistent volume** — decided
   2026-08-31; bytes at `/data/images` via `OZ_IMAGE_DIR`, oz-api serves
   them itself, no new component.
3. ~~Bulk import vs upload-at-assignment~~ **RESOLVED: upload happens when
   the image is applied** — decided 2026-08-31; the editor's apply action
   ingests + assigns in one step (no bulk importer, no staging area).
   Model: **menu item = exactly 1 image (always); product = 1 primary +
   max 4 alternatives** (slots 1..5, slot 1 mirrored to the product row).
   Authoring device: desktop editors (tablet capture stays a non-goal for v1).
4. **RESOLVED: batched, jittered byte transfer** — decided 2026-08-31; push
   batches ≤ 16 images / 512 KB per request, pull cycles cap ≤ 40 images,
   both lanes wake on full random jitter in the 1–5 min window (§3.6).

## 6. Risks

- **`image` crate weight** on the Android binary (~1–2 MB) — acceptable;
  feature-gate to the desktop + tablet apps only (CLI/cloud never link it).
- **Cache wipe by OS** (app cache is clearable) — self-healing by design:
  hashes still in DB, download manager refetches. Never store the only copy
  in `$APPCACHE`.
- **ConvertFileSrc on Android** returns `asset://localhost/...`; ensure the
  scope uses forward-slash glob relative to `$APPCACHE` so both platforms
  resolve.
- **Volume is the single copy in the cloud** — P3 must document a volume
  backup cadence (Northflank volume snapshots) alongside the DB backup;
  bytes are re-uploadable from the desktop authoring device as a last resort
  (hashes in the DB prove exactly which files are missing).
- **The 6 GB volume is shared** (oz-pos.db + pb_data + images/) — image growth can starve the DB's WAL headroom; the 4 GB soft alert exists for exactly this, and the volume backup cadence covers all three at once.
- **Slot-1 mirror drift** (products.image_hash vs product_images slot 1) —
  both writes live in the same transaction, and P1 ships a consistency
  assertion test; the mirror is a read cache, `product_images` is truth.
