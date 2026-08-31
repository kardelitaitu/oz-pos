# Spec 0046-bis — Product & Menu-Item Images (Optimized for Low-End Android)

**Status:** draft for review · **Created:** 2026-08-31 · **Scope:** desktop-client, tablet-client, ui/, oz-core, oz-api, cloud worker
**Related:** 0048 (workspace model), sync-conflict-dead-letter-recovery (0045), subscription tiers (staff/licensing untouched)

---

## 1. Goal

Let a merchant attach one thumbnail image per product (retail POS) / menu item
(resto POS), render it flawlessly on cheap Indonesian tablets, and sync it
through the existing cloud topology — without bloating the SQLite/PG rows, the
IPC bridge, or the WebView heap.

**Non-goals (v1):** multiple images per product, zoom/full-screen viewer,
category images, printer artwork on receipts/KDS chits.

## 2. Ground truth in the codebase today

| Fact | Evidence |
|---|---|
| `products` table has **no image column**; `version INTEGER` exists (sync-friendly) | `crates/oz-core/migrations/20260813_init.sql:439-457` |
| Menu items **are products** (resto filters the same table; `kitchen_zone`, `product_type`) | `RestaurantMenu.tsx:921` over `restaurantProducts` |
| CSP in **both** apps already allows `img-src … asset: https://asset.localhost` — pre-armed but the **protocol itself is not enabled** (no `assetProtocol` key anywhere) | `apps/desktop-client/tauri.conf.json:29`, `apps/tablet-client/tauri.conf.json:15` |
| `react-window` v2.2.7 is already a dependency | `ui/package.json:50` |
| `RetailProductGrid` paginates (`pagedProducts`); `RestaurantMenu` renders `filtered.map` unvirtualized | `RetailProductGrid.tsx:611`, `RestaurantMenu.tsx:921` |
| Cloud product routes exist for sync (`list_products`, `create_product`, …) | `crates/oz-api/src/routes/products.rs` |

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
- **Image URLs are immutable cache keys** — a new upload produces a new file
  name, so `<img>` cache invalidation is free (no stale-image bugs).

### 3.2 Rust-owned flat disk cache; DB stores only the hash

- Directory: `$APPCACHE/images/` (per-profile app cache — OS-managed
  location, safe to wipe).
- File name: `{product_id}.{content_hash8}.webp` — flat, one file per image
  version; product id keeps it greppable, hash keeps it content-addressed.
- DB row stores **`image_hash TEXT NULL` only** — never an absolute path
  (paths differ per device), never bytes (keeps rows light and indexable).
  Sync then replicates one small string per product; each device resolves
  hash → local file (or downloads it — §3.4).
- Column added to `products` in **both** migrations (`.sql` + `.pg.sql`) as a
  trailing nullable `ALTER`-style append (existing table style) +
  `version` bump on image change so the delta sync picks it up.

### 3.3 Ingest pipeline in Rust (desktop is the authoring surface)

One IPC command, `products_register_image(product_id, bytes)`:

1. **Sniff, don't trust:** validate magic bytes (WebP `RIFF….WEBP`, JPEG
   `FFD8FF`, PNG `89504E47`) — extension is ignored.
2. **Caps:** ≤ 5 MB raw input, decoded dimensions ≤ 4096² (denial-of-heap
   guard — `image` crate `ImageReader::with_guessed_format().limits()`).
3. **Transcode & resize:** decode → EXIF orientation applied → EXIF stripped →
   resize to **256 px longest edge** (POS tiles never render larger) →
   encode **WebP q80**. Target output ≈ **8–20 KB** (the guideline's
   10–20 KB class), decoded ARGB ≈ **256 KB** per image in RAM.
4. **Write & commit:** write `{product_id}.{hash8}.webp` to the cache dir
   (temp + atomic rename), then `UPDATE products SET image_hash = ?,
   version = version + 1` inside a **rusqlite transaction** (repo rule).
5. **GC:** on startup and after deletes, sweep the dir — any file whose
   `{product_id}` no longer exists or whose `{hash8}` ≠ current row hash is
   removed after a grace period (simple: remove immediately except files
   touched in the last 24 h, so an in-flight sync never mid-air-collides).

### 3.4 Sync: hash first, bytes on demand

- Row-level product sync already carries `version`; adding `image_hash` is a
  free rider — **the hash syncs like any column**.
- Bytes live in **Cloudflare R2** (deploy already targets Cloudflare —
  `scripts/wrangler-deploy.sh`, `public/_headers`). Cloud object key:
  `images/{tenant_id}/{product_id}.{hash8}.webp`.
- New worker/oz-api surface (admin-key-gated, same tier as catalog writes):
  - `PUT /api/v1/products/{id}/image` (desktop upload, ≤ 32 KB body)
  - `GET /api/v1/products/{id}/image?hash=…` (tablet download; immutable,
    `Cache-Control: max-age=31536000, immutable` — hashes are keys)
- Tablet pull: a tiny **download manager** in the tablet's Rust shell — on
  catalog apply, for each product with `image_hash` and no local file, queue a
  background GET (bounded concurrency 3, LRU eviction of the images dir at a
  configurable budget, default **64 MB** ≈ 3–6 k products). Offline-first
  holds: missing image degrades to the existing colored-initial tile.

### 3.5 UI: virtualize or the pixel RAM wins anyway

- The math that makes virtualization non-optional: a 256×256 decoded image is
  ~256 KB of ARGB; a 512×512 is ~1 MB (the guideline's 1.05 MB). A
  non-virtualized 300-item grid would pin ~75 MB of decoded bitmaps on a
  2 GB tablet even with tiny files on disk.
- **POS grids** (`RetailProductGrid`, resto ordering screen) switch the
  product tiles to `react-window` v2 (`FixedSizeGrid`) — already in the
  dependency tree — so off-screen tiles **unmount entirely** and Android
  reclaims their decoded bitmaps.
- Admin screens (`RestaurantMenu` catalog editor, `ProductLookupScreen`) get
  thumbnails lazily (`loading="lazy"` + `decoding="async"`) — full grid
  virtualization there is P3, not blocking.
- Placeholder: keep the existing colored-initial tile as the skeleton/miss
  state; no layout shift (fixed tile aspect-ratio).

## 4. Work breakdown (each phase = conventional commits, tests first)

| Phase | Deliverable | Touches |
|---|---|---|
| **P1 — storage spine** | `image_hash` column (sqlite+pg migrations, models, repository, `products_register_image` command with sniff/limits/transcode/atomic-write/txn, unit tests incl. malformed-magic + oversized rejection) | `oz-core` (migrations, db/products), `desktop-client/src/commands`, tauri.conf `assetProtocol` |
| **P2 — UI tiles** | `FixedSizeGrid` on POS + `<img convertFileSrc>` with hash-keyed URLs, miss→initial tile fallback, a11y (alt text from product name, aria-busy on load) | `ui/src/features/retail`, `restaurant`, shared `ProductThumb` component |
| **P3 — cloud sync** | R2 bucket + worker routes (upload/download, admin-key gate per API-4/G-1 pattern), tablet download manager + LRU, sync tests (hash rider, immutable ETag) | `gateway`/worker, `oz-api`, tablet-client Rust |
| **P4 — hygiene** | startup GC sweep, metrics (bytes dir, hit/miss latency), e2e on the E2E suite, docs (`docs/guides`), i18n strings for the editor UI (en+id FTL) | `desktop-client`, `ui`, docs |

**Est. budget:** P1–P2 make a fully local (single-device) feature — shippable
slice. P3 unlocks tablets. P4 is polish.

## 5. Decision points to confirm before P1

1. **256 px WebP q80 single variant** — good enough, or do we want a 512 px
   "detail" variant now? (Doubles storage + sync surface; my default: no.)
2. **R2 as the byte store** vs PG `bytea` (simpler, but blobs in PG fight the
   "DB stays light" rule and bloat snapshots).
3. **Upload authoring surface** — desktop settings screen only (v1), or also
   allow tablet camera capture (adds upload path from Android)?

## 6. Risks

- **`image` crate weight** on the Android binary (~1–2 MB) — acceptable;
  feature-gate to the desktop + tablet apps only (CLI/cloud never link it).
- **Cache wipe by OS** (app cache is clearable) — self-healing by design:
  hash still in DB, download manager refetches. Never store the only copy in
  `$APPCACHE`.
- **ConvertFileSrc on Android** returns `asset://localhost/...`; ensure the
  scope uses forward-slash glob relative to `$APPCACHE` so both platforms
  resolve.
