# OZ-POS Full Crate Audit — GLM 5.3-Flash (RSA)

> **Campaign log.** Findings and proposed solutions for every Rust target in the
> workspace, produced by an evidence-based audit (RSA — Rust System Auditor
> methodology). **Audit-only by default:** nothing here is patched automatically;
> each finding carries a proposed solution and waits for an explicit go-ahead.
>
> - **Auditor:** GLM 5.3-Flash (DeepSeek Harness)
> - **Started:** 2026-07-25
> - **Version:** 0.0.33 (locked — never bumped by audit work)
> - **Method:** per crate → baseline evidence (`cargo check -p <crate>`, targeted
>   `cargo test -p <crate>`, warning capture) → safety/ownership review (unsafe,
>   Send/Sync, Arc/Mutex, clones) → dependency & API review → RSA stamp written
>   to the top of each audited `.rs` file.
> - **Evidence policy:** findings are traceable to a file:line or a command
>   result; no speculation. If evidence is incomplete, the audit says so.

## Severity legend

| Level | Meaning |
|---|---|
| 🔴 HIGH | Security or correctness issue with real exploit/impact potential |
| 🟠 MEDIUM | Design weakness, fails-open behavior, or fragile construction |
| 🟡 LOW | Hygiene, convention, or robustness improvement |
| ℹ️ INFO | Observation; action optional |

---

## Campaign status

| # | Target | .rs lines | Status | Findings (H/M/L/I) | Commit |
|---|---|---:|---|---|---|
| 1 | crates/oz-crypto | 339 | ✅ DONE | 1 / 2 / 3 / 1 | `082e7f0f` |
| 2 | crates/oz-security | 2,068 | ✅ DONE | 0 / 2 / 4 / 3 | (this commit) |
| 3 | crates/oz-payment | 6,251 | ✅ DONE | 1 / 4 / 5 / 3 | (this commit) |
| 4 | crates/oz-core (sliced by subsystem) | 80,216 | ✅ COMPLETE — slices A–D, all 60+ production files read+stamped | 0 / 6 / 14 / 16 | 75334ffa |
| 5 | crates/oz-api | 7,479 | ⬜ pending | — | — |
| 6 | foundation | 6,326 | ⬜ pending | — | — |
| 7 | platform/kernel | 3,385 | ⬜ pending | — | — |
| 8 | platform/core | 6,423 | ⬜ pending | — | — |
| 9 | platform/startup | 2,076 | ⬜ pending | — | — |
| 10 | platform/sync | 11,148 | ⬜ pending | — | — |
| 11 | modules/sales | 1,300 | ⬜ pending | — | — |
| 12 | modules/inventory | 1,862 | ⬜ pending | — | — |
| 13 | modules/tax | 926 | ⬜ pending | — | — |
| 14 | modules/currency | 1,743 | ⬜ pending | — | — |
| 15 | modules/loyalty | 996 | ⬜ pending | — | — |
| 16 | modules/crm | 848 | ⬜ pending | — | — |
| 17 | modules/staff | 830 | ⬜ pending | — | — |
| 18 | modules/reporting | 704 | ⬜ pending | — | — |
| 19 | modules/terminal | 551 | ⬜ pending | — | — |
| 20 | modules/settings | 394 | ⬜ pending | — | — |
| 21 | module stubs (purchasing/promotions/giftcards/kitchen) | ~795 | ⬜ pending | — | — |
| 22 | crates/oz-hal | 6,392 | ⬜ pending | — | — |
| 23 | crates/oz-plugin | 3,883 | ⬜ pending | — | — |
| 24 | crates/oz-lua | 1,677 | ⬜ pending | — | — |
| 25 | crates/oz-notification | 1,202 | ⬜ pending | — | — |
| 26 | crates/oz-media | 1,189 | ⬜ pending | — | — |
| 27 | crates/oz-reporting | 1,735 | ⬜ pending | — | — |
| 28 | crates/oz-logging | 899 | ⬜ pending | — | — |
| 29 | crates/oz-cli | 2,956 | ⬜ pending | — | — |
| 30 | apps/cloud-server | 16,080 | ⬜ pending | — | — |
| 31 | apps/desktop-client | 51,068 | ⬜ pending | — | — |
| 32 | apps/tablet-client | 22,709 | ⬜ pending | — | — |

**Out of scope (not Rust):** `apps/license-server` (Go), `apps/unified`
(Docker/Caddy assets). **Excluded standalone crates** (own lockfiles, auditable
on request): `fuzz`, `fuzz/hfuzz`, `scripts/updater-compat-check`.

**Ordering rationale:** security-critical crates first (crypto → security →
payment), then the domain heart (`oz-core`, sliced), then platform → modules →
remaining crates → apps.

---

## Environment & tooling log

- **2026-07-25 — sccache repaired.** Root cause of earlier build failures was a
  one-time cold-start timeout of the local sccache server (no remote backend is
  configured; local disk cache). Fixes applied and verified:
  - Removed leftover empty `RUSTC_WRAPPER` (User scope).
  - `SCCACHE_CACHE_SIZE=20G` persisted at User scope (matches the size
    `scripts/setup-cache.ps1` always intended; 0.17 removed the old
    `--set-config` CLI).
  - Cache pipeline proven: clean check 1.14 s (miss) → 0.54 s (hit) → 0.83 s
    (hit after server restart).
  - `scripts/setup-cache.ps1` / `scripts/setup-cache.sh` updated for the modern
    CLI (commit `6da80a4a`); `.cargo/config.toml` documents the
    incremental × sccache tradeoff (comment-only).
  - Standing note: `[profile.dev] incremental = true` makes dev-profile
    compiles non-cacheable by sccache. That tradeoff is kept deliberately;
    audits use `CARGO_INCREMENTAL=0` only when a cacheable baseline matters.

---

## 1. crates/oz-crypto — ✅ audited 2026-07-25

**File:** `crates/oz-crypto/src/lib.rs` (402 lines incl. tests) · **Stamp:**
`status: SAFE | lint: CLEAN` · **Commit:** `082e7f0f`

### Baseline evidence

- `cargo check -p oz-crypto` — **clean, 0 warnings** (workspace `missing_docs`
  lint satisfied).
- `cargo test -p oz-crypto` — **10/10 pass**, 0 doc-tests.
- **Zero `unsafe`** anywhere in the crate.
- Dependencies (`aes-gcm`, `rand`, `sha2`, `base64`, `thiserror`) — all used,
  no dead weight.
- Consumers (blast radius): `oz-core`, `platform/core`; graph shows every one
  of the 9 secret domains (`encrypt_*`) has live inbound callers.

### What the crate does

AES-256-GCM helpers for secrets at rest. Ciphertext format:
`base64(nonce ‖ ciphertext ‖ tag)`, fresh random 12-byte nonce per call. Keys
are derived from a domain-separation prefix plus either a machine id
(machine-bound) or nothing (static/portable). Nine domains: SMTP (machine
bound + at-rest), API key (machine bound), sync API key, sync terminal secret,
PG sync password, rate API key, LAN PSK, user-profile field (all static).

### Findings & proposed solutions

| ID | Sev | Location | Finding | Proposed solution |
|---|---|---|---|---|
| CRY-1 | 🔴 HIGH | lib.rs:42–49, 132–206 | **Static keys are publicly derivable.** `derive_static_key(domain)` = `SHA-256` of a constant byte string committed to a public repo, and the "static" variants use `derive_key(domain, "static")`. Anyone with the source can recompute every portable key and decrypt all at-rest secrets: SMTP at-rest passwords, sync API keys, terminal secrets, PG sync passwords, rate API keys, LAN PSKs, profile fields. GCM gives integrity here, **zero confidentiality**. | Move at-rest key material out of source: OS keychain (oz-security already wraps Windows CredRead/CredWrite), DPAPI, or an env-provisioned master key persisted per install. Keep domain separation; keep AES-GCM. Machine-bound variants can stay as-is. Migration: new writes use the new provider, reads fall back to old derivation until rotated. |
| CRY-2 | 🟠 MEDIUM | lib.rs:119–130, 104–110 | **Fails-open paths.** `encrypt_smtp_at_rest` returns the **plaintext** unchanged if encryption fails (plaintext can silently land in the DB). `decrypt_smtp_password` / `decrypt_smtp_at_rest` return tampered/corrupt input verbatim. Legacy-plaintext compat is documented, but "not our format" and "tamper/corruption" are indistinguishable to the caller. | Detect "never was our ciphertext" (fails base64 shape / length) → accept as legacy exactly once, then re-encrypt and persist in new format; anything that decodes but fails GCM auth → fail closed + log loudly. Never fall back on an encryption failure — surface the error. |
| CRY-3 | 🟠 MEDIUM | lib.rs:31–39 | **SHA-256 used directly as an unsalted KDF.** Machine-bound key strength rests entirely on `machine_id` entropy (unverified at crate level; the value is fed in by callers in oz-core). Concatenation `domain ‖ machine_id` relies on prefixes ending in `:` — workable but implicit. | Switch to HKDF-SHA256 (`hkdf` crate): machine id as `ikm`, domain prefix as `info`. Verifies the construction conventionally. Follow-up action in the oz-core audit: confirm what actually feeds `machine_id` (UUIDv4 = fine; hostname/MAC = brute-forceable). |
| CRY-4 | 🟡 LOW | lib.rs:262–270 | **Lenient 4-alphabet base64 fallback chain** (URL_SAFE_NO_PAD → URL_SAFE → STANDARD_NO_PAD → STANDARD) widens the accepted ciphertext surface. GCM still authenticates, so exploitability is nil, but a canonical format is tighter. | Accept `URL_SAFE_NO_PAD` only; migrate historical values during the CRY-2 legacy pass. |
| CRY-5 | 🟡 LOW | lib.rs:31–48, 211–251 | **No zeroization.** Derived keys (`[u8; 32]`) and decrypted plaintexts live in memory without scrubbing. | `zeroize` / `Zeroizing` for key material and plaintext buffers. Cosmetic-to-moderate gain for a desktop process, cheap to do. |
| CRY-6 | 🟡 LOW | lib.rs:272–402 | **Tests inline in the production file** — AGENTS.md mandates sibling `*_tests.rs` (`#[cfg(test)] #[path = "lib_tests.rs"] mod tests;`). | Move `mod tests` to `src/lib_tests.rs`; keep all 10 tests; add the missing cases from CRY-8. |
| CRY-7 | ℹ️ INFO | lib.rs:16–22 | **Single-variant error type** (`CryptoError::Internal(String)`) prevents callers from branching on cause — the root blocker for a clean CRY-2 implementation. | Split into `Base64`, `TooShort`, `Tampered`/`AuthFailed`, `Utf8`, `Cipher(String)` variants; `decrypt_*` callers can then distinguish legacy vs corruption. |
| CRY-8 | ℹ️ INFO | tests | **Coverage gaps:** no cross-domain decrypt-failure assertion, no corrupted-ciphertext test for the fails-closed `decrypt_profile_field`, no padded-legacy base64 test. | Add alongside CRY-6. |

### Positives worth keeping

- Fresh random 12-byte nonce per encryption (correct GCM usage).
- Length guard before slicing (`combined.len() < 12 + 16`) — no panic path.
- UTF-8 validation on decrypt; fails-closed profile-field decrypt.
- Every public item documented; honest module doc; workspace lints inherited.

### Recommended fix order (when the user green-lights implementation)

1. CRY-7 (error variants) → 2. CRY-2 (fail-closed semantics + legacy
migration) → 3. CRY-1 (key provider: keychain/env master key) → 4. CRY-3
(HKDF) → 5. CRY-4 + CRY-5 + CRY-6 + CRY-8 (hygiene batch).

---

## 2. crates/oz-security — ✅ audited 2026-07-25

**Files:** 8 production (lib, error, mask, tls, windows, linux, macos,
test_helpers) + 7 sibling test files · **Stamps:** all 8 files stamped
(replacing 19-07-26 blocks in lib.rs/windows.rs) · **Status:** SAFE (windows.rs
UNSAFE-by-content, all blocks reviewed)

### Baseline evidence

- `cargo check -p oz-security` — **clean, 0 warnings**.
- `cargo test -p oz-security` — **82 unit + 6 doc-tests pass** (re-verified
  after stamping). Windows FFI tests run against the *real* Credential Manager
  with RAII cleanup and nextest-safe unique names.
- `#![deny(unsafe_code)]` at crate root; the only `unsafe` is 6 blocks in
  `windows.rs` behind a scoped `#![allow(unsafe_code)]` — each carries an
  accurate `// SAFETY:` comment, all six re-verified sound this pass.
- Sibling `*_tests.rs` convention followed exactly (AGENTS.md-compliant).
- Graph: `insecure_skip_verify` has **zero consumers outside this crate**
  (raw-text search across all `.rs`) — TLS wiring presumably lands with
  platform/sync (verify there).

### What the crate does

OS-credential-store abstraction (`Keyring` trait: get/set/delete/rotate) with
Windows Credential Manager / Linux Secret Service (D-Bus) / macOS Keychain
backends + documented in-memory dev fallback; PCI-DSS masking helpers
(PAN/Luhn/name/CVV); TLS config type for cloud sync.

### Findings & proposed solutions

| ID | Sev | Location | Finding | Proposed solution |
|---|---|---|---|---|
| SEC-1 | 🟠 MEDIUM | macos.rs:35–52, 63–72 | **Not-found detection by debug-string substring.** `format!("{e:?}").contains("-128")` — any OSStatus containing "-128" (e.g. **-12800**) is misclassified as item-not-found → real failures surface as `Ok(None)`; callers may treat a live key as absent (→ silent regeneration/data loss). | Compare numerically: `e.code() == -25300 \|\| e.code() == -128`. Delete the string matching. Platform-gated: needs a macOS host to exercise. |
| SEC-2 | 🟠 MEDIUM | linux.rs:23–38 | **Private tokio `Runtime` embedded in the struct + `block_on` per op.** Panics ("Cannot start a runtime from within a runtime") if any `Keyring` method is called from an async context; heavy per-instance runtime + dedicated D-Bus connection. | Document the sync-context requirement in the type docs, or hold a `tokio::runtime::Handle` (created outside async) and `block_on` via it, or use zbus's blocking wrapper. Needs Linux host to validate. |
| SEC-3 | 🟡 LOW | windows.rs:72–81 | **Zero-size blob edge in FFI read.** `from_raw_parts(ptr, 0)` requires a non-null pointer even for length 0; a credential with `CredentialBlobSize == 0` (e.g. written by another tool, or our own empty-string secret path) would be UB-adjacent. Everything else in the six unsafe blocks is sound. | Early-return `String::new()` when `CredentialBlobSize == 0` before the unsafe slice. One-line guard + a test storing an empty secret. |
| SEC-4 | 🟡 LOW | lib.rs:107–129 | **Default `rotate_key` is non-atomic** (get → archive `{name}-prev` → write new → write timestamp). A crash mid-sequence can leave the archive updated but the new key missing. Single-process desktop risk is low; matters if sync ever rotates concurrently. | Optional: write new key under `{name}-next`, then swap the two names, then archive. Or document the residual window explicitly. |
| SEC-5 | 🟡 LOW | tls.rs:37–42 | **`insecure_skip_verify` is serde-visible config with no guard, log, or `debug_assertions` gate.** Currently *latent* (no consumer reads it), but the moment sync wiring lands, a stray `true` in a config file silently disables TLS verification. | On `build()`/`validate()`: emit `warn!` when set; in release builds require an explicit env override (e.g. `OZ_POS_ALLOW_INSECURE_TLS=1`). |
| SEC-6 | 🟡 LOW | lib.rs (trait API) | **Secrets returned as `String` without zeroization** (hex keys in memory indefinitely). Same hygiene gap as CRY-5 in oz-crypto. | `Zeroizing<String>` in the API (breaking) or zeroize-on-drop internally at each backend before returning. |
| SEC-7 | ℹ️ INFO | error.rs:13–15 | `DecryptionFailed` variant has no producer in this crate (no decrypt path here) — presumably a mapping target for consumers. | Confirm during the oz-core pass; if unmapped, it's harmless (`#[non_exhaustive]`) but dead API surface. |
| SEC-8 | ℹ️ INFO | mask.rs:114–129 | `mask_name` mixes byte length (`part.len()`) with char extraction — star count is off for multi-byte names. **Already known and pinned** by `mask_name_byte_vs_char_caveat`. | No action needed beyond keeping the pin; switch to `chars().count()` if cosmetic accuracy ever matters. |
| SEC-9 | ℹ️ INFO | linux.rs:142–153, 48 | `set_secret` is delete-then-create (non-atomic; crash between loses the secret); per-item `Delete` errors are swallowed (`let _ =`); `OpenSession("plain")` sends secrets unencrypted over the (local-socket-protected) D-Bus session — standard libsecret uses DH. | Fold into the SEC-2 platform pass: propagate Delete errors, consider CreateItem-with-replace or DH session. |

### Positives worth keeping

- `#![deny(unsafe_code)]` crate-wide with a single reviewed, documented FFI
  exception — exactly the pattern the root Cargo.toml comment describes.
- Six `// SAFETY:` comments are accurate and complete (one hardening gap
  logged as SEC-3).
- Test hygiene is exemplary: sibling files, real-OS FFI tests with RAII
  `CredentialGuard`, atomic counters for nextest-safe names, poll-based
  `set_and_verify`, boundary/invariant tests that pin API contracts
  (empty names, double-delete semantics, rotation chaining, concurrency).
- PCI masking: digit-only pre-filter makes byte slicing panic-free; the
  7–10-digit overlap case correctly avoids exposing the full PAN.

### Recommended fix order (when the user green-lights implementation)

1. SEC-3 (one-line Windows guard + test) → 2. SEC-1 (numeric macOS error
   compare) → 3. SEC-5 (insecure flag guard) → 4. SEC-2 + SEC-9 (Linux
   platform pass, needs a Linux host) → 5. SEC-4 / SEC-6 / SEC-7 (design
   hygiene, can ride along with the CRY-5 zeroize work).

---

## 3. crates/oz-payment — ✅ audited 2026-07-25

**Files:** 20 production (core 6, drivers 6, EDC 8) + 15 sibling test files + 7
integration binaries + 9 recorded JSON fixtures · **Stamps:** all 20 production
files stamped (replacing the 19-07-26 block in lib.rs) · **Status:** SAFE (zero
unsafe)

### Baseline evidence

- `cargo check -p oz-payment` — **clean, 0 warnings**.
- `cargo test -p oz-payment` — **136 unit + 91 integration + 5 doc-tests pass**
  (re-verified after stamping). The integration rig is wiremock-based and
  asserts exact request bodies per gateway, plus recorded success/decline/
  timeout fixtures and full Stripe lifecycle tests.
- `#![deny(unsafe_code)]`; zero unsafe anywhere.
- Money handled as `foundation::Money` (i64 minor units) throughout; unknown
  gateway currency codes are hard errors (PA-02) in Stripe/Square.
- All PLANNED stubs (Paddle, EDC terminals, codecs, webhook verifiers,
  registry `build_from_config`) **fail closed** with `Unsupported`.

### Findings & proposed solutions

| ID | Sev | Location | Finding | Proposed solution |
|---|---|---|---|---|
| PAY-1 | 🔴 HIGH | qris.rs:255–257 (+ 420, 444, 532, 593) | **Midtrans amounts silently zero.** Midtrans `gross_amount` is documented as `"14500.00"`-style decimal strings; `parse_amount` does `s.parse::<i64>().unwrap_or(0)` → **`amount_charged = IDR 0`** on every real decimal-formatted response (authorize, capture, receipt; refund parses `refund_amount` the same way). Financial records/receipts carry 0. | Parse the decimal string properly (integer + fraction, round to minor units); return `PaymentError::InvalidResponse` on unparseable amounts instead of `0`. Add a unit test with `"14500.00"` (current `qris_parse_amount` only tests plain integers). |
| PAY-2 | 🟠 MEDIUM | qris.rs:241–247, square.rs:321/376, stripe.rs (no header) | **`PaymentRequest.idempotency_key` ignored by all three live drivers.** QRIS generates a fresh `order_id` per call (order_id *is* Midtrans's idempotency mechanism → a network retry creates a second charge); Square generates a fresh `Uuid::now_v7()` per request (defeats its idempotency protection); Stripe sends no `Idempotency-Key` header. Trait doc promises: "If `None`, the processor will generate a fallback key" — the `Some` path is unimplemented everywhere. | Honor `request.idempotency_key`: Midtrans → order_id derived from it; Square → `idempotency_key` field; Stripe → `Idempotency-Key` header. Add retry-does-not-duplicate tests. |
| PAY-3 | 🟠 MEDIUM | stripe.rs:381–387, qris.rs:493–502, square.rs:373 | **Refund amount contract violated in all three drivers.** Stripe and QRIS ignore `_amount` and always full-refund (merchant asks partial 50k of 500k → customer gets 500k back). Square does `amount.unwrap_or(Money::zero(USD))` — `None` (trait: "full amount") sends a zero-amount refund with a hardcoded USD currency, which Square rejects (amount_money required). | Stripe: pass `("amount", n)` when `Some`. QRIS: pass `"amount"` in refund body when `Some`. Square: fetch the payment to resolve the total when `None`; error on currency mismatch. |
| PAY-4 | 🟠 MEDIUM | stripe.rs:262–274 | **Decline misclassification.** The first match arm sends any `card_error` whose message contains "card" (nearly all — "Your card was declined.") to `InvalidCard`; the `"card_error" => Declined` arm is effectively unreachable. UI/analytics would report "invalid card" for plain declines; inconsistent with Square (`CARD_DECLINED` → `Declined`). | Match `code == Some("card_declined")` (and fraud/processing codes) → `Declined` **before** the heuristic; drop the `message.contains("card")` catch-all. |
| PAY-5 | 🟠 MEDIUM | square.rs:318–347 | **Square auto-capture vs trait lifecycle.** Square `CreatePayment` defaults `auto_capture=true`, so `authorize()` already captures; the default `sale()` (authorize → `/payments/{id}/complete`) then fails against the real API. Tests pass only because wiremock replays canned responses. | Send explicit `auto_capture: false` (Square: `autocomplete` field) in `CreatePaymentRequest`, or override `sale()` and document the one-step semantics. Verify against a real sandbox at go-live. |
| PAY-6 | 🟡 LOW | qris.rs:458–490, 366–407 | **Stringly-typed QR protocol + timing gap.** `sale()` returns `SCAN_QR\|order_id\|url` inside `message` for the UI to parse; `success=true` means "QR issued", not paid. Poll window is 30×2 s = 60 s while the QR is valid 300 s → `Timeout` while the customer may still complete payment (reconciliation needed). | Add a structured QR field to `PaymentResult` (or a dedicated result type); align poll window with `QRIS_EXPIRY_SECS` and document the settle-later path. |
| PAY-7 | 🟡 LOW | qris.rs:612–616 | `Default for QrisPaymentProcessor` constructs with an **empty server key** → every request 401s at runtime. | Remove the impl or document it as test-only; consider `new()` validating non-empty key. |
| PAY-8 | 🟡 LOW | qris.rs:386–388 | QRIS `expire` status maps to `PaymentError::InvalidCard` — an expired QR is not an invalid card; taxonomy noise for callers. | Map to `Declined("QR expired")` or a dedicated variant. |
| PAY-9 | 🟡 LOW | square.rs:45, stripe/qris headers | Gateway secrets live in heap `String`s / client headers indefinitely (Debug masking is correct and tested). | Fold into the CRY-5 / SEC-6 zeroize pass. |
| PAY-10 | ℹ️ INFO | mock.rs:107, 117 | `// SAFETY:` comments annotate **safe** `.lock().unwrap()` calls — SAFETY is the convention for `unsafe` blocks; pollutes unsafe-code grep hygiene. | Reword as plain comments. |
| PAY-11 | ℹ️ INFO | edc/mod.rs:114–129 | `edc::PaymentResult` shadows `crate::types::PaymentResult` with a different shape (adds `card_scheme`/`card_last4`). Deliberate but confusing. | Rename to `EdcPaymentResult` when the EDC work resumes. |
| PAY-12 | ℹ️ INFO | registry.rs:55–63 | `build_from_config` still returns `Unsupported` — drivers are constructed directly by callers, so "switching gateways is a config change" is not yet true. | Implement during registry wiring (platform/startup or oz-core pass). |

### Positives worth keeping

- Zero unsafe; `#![deny(unsafe_code)]`.
- Exceptional test rig: per-gateway wiremock suites asserting exact request
  bodies, recorded fixtures (success/decline/timeout), full Stripe lifecycle,
  and masking tests pinning that Debug never leaks keys.
- Every stub fails closed (`Unsupported`), including the webhook guard — a
  missing verifier can never silently accept a webhook.
- `auth_value.set_sensitive(true)` on all auth headers; `no_proxy()` deliberate.
- Currency hard-error (PA-02) instead of silent USD fallback in Stripe/Square.

### Recommended fix order (when the user green-lights implementation)

1. PAY-1 (amount parsing — data integrity, small diff + test) → 2. PAY-2
   (idempotency across all three drivers) → 3. PAY-3 (partial refunds) →
   4. PAY-4 + PAY-8 (error taxonomy) → 5. PAY-5 (Square lifecycle, needs
   sandbox verification) → 6. PAY-6/7/9/10/11 hygiene batch.

---

## 4. crates/oz-core — 🟡 IN PROGRESS (sliced by subsystem)

**Layout:** ~90 top-level modules (mostly thin documented re-export shims over
`foundation` / `modules-*`) + **`db/` (81 files, 46,358 lines — rusqlite
persistence)** + `export/` (3,121) + `sync/` (279). Heavy hitters:
`sync_client.rs` 1,243 · `features.rs` 1,223 · `settings.rs` 706 ·
`topology.rs` 681 · `subscription.rs` 621 · `license_verification.rs` 550.

**Slice plan:** **A** — lib/error/session/audit/events/rate_limiter/
config_validator/payment/cash_payout/crypto + shims ✅ · **B1** — db facade,
migrations, structural sweeps, payments ✅ · **B2** — sales/products/
inventory ✅ · **B3** — gift_cards/loyalty ✅ · **B4** — stock_transfers ✅ ·
**B5** — offline/kds/workspaces/reports + remaining stores · **C** —
sync_client/topology/features/settings · **D** —
export/subscription/license_verification/kds + remainder.

### Slice A baseline evidence (2026-07-25)

- `cargo check -p oz-core` — clean (re-verified post-stamp).
- `cargo test -p oz-core` — **2,536 tests pass** (2,026 unit + 510
  integration across 24 suites), 0 warnings.
- `#![deny(unsafe_code)]`; source sweep confirms **zero actual unsafe** (4
  keyword hits are all comments/stamp text).
- `new_id()` = UUIDv7 everywhere (ADR #6); Money stays `i64` minor units via
  `foundation::money` re-export.

### Slice A findings

| ID | Sev | Location | Finding | Proposed solution |
|---|---|---|---|---|
| COR-1 | 🟡 LOW | audit.rs:16, payment.rs:22, cash_payout.rs:12 | **Stale "UUID v4" field docs** — constructors correctly generate v7 per ADR #6/house rule (`new_id()`), the docs lie. | Docs-only fix: change field docs to "UUID v7". |
| COR-2 | 🟡 LOW | rate_limiter.rs:20 | **Unbounded per-username HashMap** in the login rate limiter — spamming the login form with random usernames grows memory without cap (desktop-local blast radius). | Cap map size / evict idle usernames on prune. |
| COR-3 | 🟠 MEDIUM | config_validator.rs:110–119, 195–205 | **Credential leakage in validator errors.** On misconfiguration, the message embeds a `DATABASE_URL` prefix (typically `postgresql://user:password@…` within 40 chars) and the **full** `REDIS_URL` (may embed `redis://user:pass@host`). These land in tracing logs — retained long-term per logging policy. | Redact userinfo before embedding: print `scheme://[redacted]@host[:port]` only. |
| COR-4 | ℹ️ INFO | error.rs:205 | `TopologyValidation` folds into `kind() == Validation`, so the front-end `AppError.subKind` cannot single out topology failures (the structured `code` field does carry specificity). | Acceptable; revisit if UI needs a distinct kind. |
| COR-5 | ℹ️ INFO | session.rs:52–54 | `expires_at: None` = never-expiring session, available in production — enforcement lives with the settings layer. | Verify the production default TTL during slice C (settings.rs). |

**Cross-crate threads confirmed:** `Payment.idempotency_key` is captured at
the oz-core sale level → PAY-2's drop point is confirmed to be the
oz-payment drivers, not oz-core. `crypto.rs` shim inherits CRY-1…8 verbatim.

### Slice A positives

- Honest, documented re-export shims (16 files) with zero logic.
- `error.rs` is a model typed-error surface (UI discriminator + structured
  payloads + `#[non_exhaustive]`).
- Rate limiter logic is correct including the `max_attempts = 0` edge and
  poison recovery on the hot path.

### Slice B1 evidence (2026-07-25)

- Structural sweeps over all 41 production db files:
  - **Money policy holds** — `f64` appears only in loyalty *points* math
    (`earn_multiplier`, documented i64-first) and popularity analytics
    scores; zero currency stored as float.
  - **Panic paths nearly absent** — 2 `unwrap()`s total (`db/profile.rs`
    125/135, both guarded by presence checks immediately above).
  - **Transactions** — `unchecked_transaction` at ~70 sites across 22 files,
    matching the documented RUST-08 contract; no nesting violations surfaced
    by the 2,536-test suite.
  - **SQL construction** — all 6 `format!`-built SQL sites verified
    injection-safe: interpolated fragments are internal whitelists/column
    lists; every user-supplied value binds through `?N` params; LIKE inputs
    escape `%`/`_`/`\` with an explicit `ESCAPE '\'` clause
    (db/audit.rs, db/customers.rs).
- `db/payments.rs`: idempotency dedup (check-then-return-existing) runs
  inside the transaction; **`UNIQUE idx_payments_idempotency_key`** exists in
  both `20260813_init.sql:1204` and the PG schema — the concurrent-insert
  race resolves as a constraint error, not a duplicate. PAY-2 confirmed
  driver-side only.
- `db/mod.rs`: backup via online Backup API (not `VACUUM INTO` — no path
  interpolation, RUST-02/03), `PRAGMA integrity_check`, fail-loud tenant
  integrity gate.

### Slice B1 findings

| ID | Sev | Location | Finding | Proposed solution |
|---|---|---|---|---|
| COR-6 | ℹ️ INFO | migrations.rs:177–183, db/profile.rs:124/134 (pattern also PAY-10) | **Recurring mislabeled `// SAFETY:` comments on safe code** (guard-backed unwraps, test-harness locks). Pollutes unsafe-code grep hygiene crate-wide. | Reword as plain comments; keep `SAFETY:` exclusively for `unsafe` blocks. |

### Slice B1 positives

- The transaction contract (RUST-08) is not just documented — the code
  matches it, and the tests pin the no-nesting boundary.
- Injection safety is systematic, not accidental (escape + bind everywhere).
- Backup/repair path is prior-audit-hardened (RUST-02/03 notes in-line).

### Slice B2 findings — db/sales.rs deep read (2,419 lines, fully read)

| ID | Sev | Location | Finding | Proposed solution |
|---|---|---|---|---|
| COR-7 | 🟠 MEDIUM | db/sales.rs:1025–1046 | **Sale-completion path drops payment idempotency keys.** `complete_sale_deduction` inserts payment splits directly and omits the `idempotency_key` column even though `PaymentSplitArg` carries it — bypassing the dedup-aware `create_payments` route (payments.rs). Keys captured at the IPC boundary never reach the ledger on the main sale path. | Include `split.idempotency_key` in the INSERT (column + index already exist), or route through `create_payments` within the same transaction. |
| COR-8 | 🟡 LOW | db/sales.rs:1991–2001 | **`void_sale` claims optimistic concurrency but has no CAS.** Comment cites ADR #6, yet the UPDATE is `WHERE id = ?2` only (version is incremented, never compared) — a concurrent mutation between `get_sale` and the update is silently overwritten. | `WHERE id = ?2 AND version = ?3` using the version read by `get_sale`; map 0 rows to `Conflict`. |
| COR-9 | ℹ️ INFO | db/sales.rs:1950–1958 | Receipt-barcode lookup swallows DB errors via `.ok()` → a real I/O failure reads as "no sale". Lookup-only path, not money-moving. | Log the error before falling back to `None`. |
| COR-10 | ℹ️ INFO | db/sales.rs:538–552 | Partial-stock result travels as JSON inside `CoreError::Validation.message` — documented in-file as a known tradeoff; the front-end parses the message. | Dedicated `PartialStock` error variant when the IPC error surface is next touched. |

### Slice B2 continued — db/products.rs (2,216 lines) + db/inventory.rs (757 lines)

| ID | Sev | Location | Finding | Proposed solution |
|---|---|---|---|---|
| COR-11 | 🟡 LOW | db/inventory.rs:215, 233 | **Deactivation guard fail-open.** `deactivate_inventory_location` reads its zero-stock and zero-transfers constraints with `.unwrap_or(0)` — a DB read error yields count 0, constraints pass, and a location with a hidden ledger balance (or in-flight transfers) is deactivated. | Propagate with `?` (match `set_stock_threshold`'s own NoRows-vs-error discipline in the same file). |
| COR-12 | 🟡 LOW | db/products.rs:607 vs 1091 | **Name-length asymmetry.** `create_product` rejects names >255 chars; `update_product` only checks non-empty — an over-long name can enter via update. | Apply the same 255 check in `update_product` (and `update_product_attributes`). |
| COR-13 | ℹ️ INFO | db/inventory.rs:522/557/721; db/sales.rs:1519 | Read mappers coerce unknown stored enum values to defaults (`ManualAdjustment`, `SaleStatus::Pending`) — corrupt/forward-compat rows silently misclassify in reports. | Log-on-fallback at minimum; consider a `Unknown` variant for display. |
| COR-14 | ℹ️ INFO | db/products.rs:2203 | Variant mapper converts an invalid stored barcode to `None` via `.ok()` — corrupt data renders as "no barcode" without any signal. | Log the parse failure. |

**Slice B2 positives:** `update_product` implements *real* optimistic CAS
(`WHERE sku = ? AND version = ?` → `Conflict`) — the exact pattern COR-8
wants copied into `void_sale`. `create_product` is idempotent-with-payload-
comparison (sync-replay safe, `ON CONFLICT(tenant_id, sku)` backstop).
`adjust_stock_batch` pre-checks every location before executing any
deduction, uses `checked_add` + typed `InsufficientStockAtLocation`, and its
`allow_negative_stock` lookup fails safe to *deny*. The deprecated
`adjust_stock_with_reason` self-documents its own ADR-19 §3.4 stale-source
foot-gun — tracked upstream debt, no new finding.

> **Provenance note:** the slice-B2 stamp and its findings section landed in
> commit `11a6822b` (a parallel review session swept the working-tree changes
> into its own commit) rather than a dedicated audit commit. Content verified
> present at HEAD; no history rewrite performed to avoid disturbing the
> concurrent session.

### Slice B3 findings — db/gift_cards.rs (666 lines) + db/loyalty.rs (721 lines), fully read

| ID | Sev | Location | Finding | Proposed solution |
|---|---|---|---|---|
| COR-15 | 🟡 LOW | db/gift_cards.rs:373–399 + init.sql:1122–1128 | **Gift-card redeem idempotency is advisory-only.** The `(card_id, sale_id)` check-then-act has no UNIQUE index behind it (verified: `init.sql` indexes only `gift_card_id` and `sale_id` separately). Race-safe today only because all Store ops serialize on the process-wide connection mutex; a multi-terminal/offline-sync replay of two redemptions for the same sale double-deducts. Contrast: `loyalty_transactions` has exactly this unique projection index. | Forward-only migration adding `CREATE UNIQUE INDEX … ON gift_card_transactions(gift_card_id, sale_id) WHERE txn_type='redeem'`, then treat `ConstraintViolation` like `earn_points` does (return the winning row). |
| COR-16 | ℹ️ INFO | db/gift_cards.rs:189, 204 | `list_gift_cards` search patterns don't escape LIKE wildcards (inconsistent with the escaped pattern in db/customers.rs and db/audit.rs) — searching "50%" over-matches. Injection-safe (bound param). | Reuse the escape+`ESCAPE '\'` helper. |
| COR-17 | ℹ️ INFO | db/gift_cards.rs:43, 51 | Gift-card PINs stored in plaintext. Acceptable in the local-POS threat model (SQLite file access ⇒ game over anyway), but becomes a question if gift cards ever sync to cloud. | Note for the cloud-sync threat model; hash if cards go multi-terminal. |
| COR-18 | ℹ️ INFO | db/loyalty.rs:265–283 | `list_loyalty_accounts` prepares + runs a 5-row transaction query per account (N+1). Fine at desktop scale. | Batch to a single window-function query when sync/reporting scale demands. |

**Slice B3 positives:** `loyalty.rs` is the crate's concurrency reference
implementation — earn/redeem idempotency with a unique projection index as
the final guard, server-side sale validation before redemption (ownership,
completed-status, cap-at-total), and atomic conditional balance UPDATEs.
`gift_cards.rs` redeem/top-up use the PA-01 atomic-conditional pattern with
in-transaction balance re-reads and an `i64::MAX` overflow guard; expiry
parse-failure fails safe (card treated as expired).

### Slice B4 findings — db/stock_transfers.rs (779 lines), fully read

| ID | Sev | Location | Finding | Proposed solution |
|---|---|---|---|---|
| COR-19 | 🟠 MEDIUM | db/stock_transfers.rs:458–484, 591–610, 723–744 (also db/stock_counts.rs, `get_stock` in db/products.rs) | **Dual-ledger divergence: transfers bypass the canonical stock ledger.** Sales pre-check and deduct `stock_summary` (per-location, ADR-18/19 canonical — verified `adjust_stock_batch` reads `stock_summary WHERE item_id/location_id`), but `send_transfer`/`receive_transfer`/`cancel_transfer` read/write the **legacy `inventory` table** (single PK on `product_id` — its bolted-on `location_id` column has a DEFAULT and cannot represent per-location state). No schema trigger bridges the two (init.sql triggers are audit-immutability + tier validation only). A transfer therefore never appears in sale-time availability or the retail grid (`ProductWithDetails.stock_qty` prefers `SUM(stock_summary)` per ADR-36), and the two totals drift apart. This is the §3.4 foot-gun the codebase self-documents for a *deprecated wrapper* — here it applies to **live production flows**. | Complete ADR-19 §3.4: route transfers (and stock counts) through `stock_summary` rows at the source/destination locations; interim alternative: maintain both tables inside the same transaction. |

**Slice B4 positives:** the transfer state machine is the strongest
lifecycle code in the crate — claim-first conditional status UPDATEs in the
same transaction as the stock writes, in-transaction status reads
(documented as fixing a prior cancel/send race), receive quantities
validated non-negative/ordered-cap/monotonic with delta-only crediting, and
`checked_add`/`checked_sub` on every inventory mutation.

### Slice B5 (in progress) — db/offline.rs (684 lines) fully read + 6 small files

| ID | Sev | Location | Finding | Proposed solution |
|---|---|---|---|---|
| COR-20 | ℹ️ INFO | db/offline.rs:87–95, 394–431 | Silent degradation on DB errors in the enqueue-dedup EXISTS check (`.unwrap_or(false)` → duplicate enqueue) and the observability summary (`.ok()`/`.unwrap_or(0)` → zeros). Both are benign-direction failures (replay-safe apply side; dashboard-only) but errors become invisible. | Log at warn before falling back. |

**Slice B5 notes so far:** `offline.rs` is production-grade sync plumbing —
tenant-scoped variants throughout (SYNC-07: cross-tenant reads as
NotFound/no-op), a `sync_applied_items` idempotency ledger with an
in-transaction variant co-located with the domain mutation, a durable pull
anchor written only after successful page application (crash-safe), and an
atomic dead-letter requeue (predicate inside the DELETE) that rewinds the
anchor safely. The six small files (`edc_terminals`, `media`,
`payment_settlements`, `payment_gateways`, `stripe`, `recipes`) are clean —
four are honest fail-fast PLANNED stubs; one forward note: when
`payment_gateways.config_json` starts holding gateway API keys, it needs
at-rest encryption (ties to CRY-1 remediation).

### Slice B5 part 2 — db/reports.rs (1,042 lines), fully read

| ID | Sev | Location | Finding | Proposed solution |
|---|---|---|---|---|
| COR-21 | 🟠 MEDIUM | db/reports.rs (all ~20 aggregations, e.g. :326–340, :371–386, :499–507, :1009–1017) | **UTC time-bucketing for a UTC+7/+8 market.** Every report buckets by `DATE(created_at)` / `strftime('%H', created_at)` / `strftime('%w', …)` with zero localtime adjustment, while all timestamps are written `Utc::now()` (verified: 0 `localtime` hits across the whole db layer; 11+13 `Utc::now()` call sites in sales/kds). Daily/weekly/monthly revenue, the hourly heatmap (UTC hour!), occupancy curves, inventory trends, and date-filtered exports mis-bucket every transaction outside 00:00–07:00 local time — for the primary Indonesian market that is a large share of operating hours. The `HourlyOccupancyRow` doc even claims "local store time as stored", which is drift. | Add a store-timezone setting; bucket either via SQLite offset modifiers (`created_at, '? hours'`) or in Rust through `chrono` with the configured TZ; fix the doc comment; backfill note for historical report semantics. |
| COR-22 | ℹ️ INFO | db/reports.rs:449 vs :767 | `top_products` passes `limit` straight to SQL (`LIMIT ?3`, negative ⇒ unlimited) while `voided_items` clamps to `[1, 100]` — inconsistent boundary discipline. | Clamp in `top_products` too. |

**Slice B5 part 2 positives:** the correlated-COGS subquery design is
documented and deliberate (keyed on the same date/week/month expression so
joining `sale_lines` never multiplies revenue counts); the Monday-first
week-boundary idiom is explained inline (`'-6 days', 'weekday 1'`);
`order_by` is a two-value whitelist (injection-safe); per-location low-stock
alerts correctly use the canonical `stock_summary` with custom-threshold
precedence.

### Slice B5 part 3 — db/profile.rs (631) + db/customers.rs (281), fully read

| ID | Sev | Location | Finding | Proposed solution |
|---|---|---|---|---|
| COR-23 | ℹ️ INFO | db/customers.rs:263–274 | `delete_customer` hard-deletes regardless of sales history or a linked loyalty account — dangling references possible. | Soft-delete or referential guard. |
| COR-24 | ℹ️ INFO | db/profile.rs:296–298 | `decrypt_sensitive` returns `None` silently on decrypt failure — fail-closed (good direction) but a corrupt ciphertext reads as a missing field with no signal. | Log at warn before returning None. |

**Cross-crate escalation for CRY-1:** `profile.rs` encrypts national id and
payroll at rest via `encrypt_profile_field` → `oz-crypto` — whose static
key is derivable from repo constants (CRY-1 🔴). The PII "at-rest" guarantee
therefore inherits CRY-1's weakness; **CRY-1's fix priority should be raised**
on that basis (staff SSN/KTP ciphertext is currently reproducible by anyone
with repo access).

**Slice B5 part 3 positives:** `profile.rs` is a model PII implementation —
sensitive fields encrypted, uniqueness preserved via a SHA-256 hash of the
plaintext (never stored), last-4 masking in every surface, sensitive reads
both permission-gated (`staff:read_identity`/`staff:read_payroll`) and
audit-logged (access recorded, never values), and an incomplete-profile
guard blocking sensitive-role assignment. `customers.rs` implements the
CUST-06 PII-bounded search correctly (escaped LIKE + clamped page).

### Slice B5 part 4 — db/tax.rs (435) + db/refunds.rs (457), fully read

| ID | Sev | Location | Finding | Proposed solution |
|---|---|---|---|---|
| COR-25 | 🟠 MEDIUM | db/refunds.rs:41–80 | **Over-refund guard is fail-open and outside the transaction.** The cumulative-refunded SUM uses `.unwrap_or(0)` — a DB read error reads as "nothing refunded" and bypasses the guard — and the check-then-act runs before the tx opens (race-safe only under the single-connection mutex; sync replay could double-refund). This is a *money* guard, one class worse than COR-11's inventory guard. | Move the guard inside the tx, propagate the SUM error with `?`, and re-check the balance against the transaction's own view. |
| COR-26 | 🟡 LOW | db/refunds.rs:81, 389–424 | Refund currency is never compared to the sale's currency — the comment defers to "the caller's checked_add" but nothing enforces it. A cross-currency refund passes the over-refund guard against the wrong unit, and `total_refunded_for_sale` silently excludes other-currency refunds from the balance. | Reject `refund.total.currency != sale.currency` with a structured error. |

**Slice B5 part 4 positives:** `tax.rs` is exemplary across the board —
TAX-02 default-flag swap atomic in one tx, TAX-03 archive-not-delete with a
sale-line reference guard (receipts keep resolvable rate linkage), archived
rates immutable and hidden from resolution, TAX-04 bounded `rate_bps` with
an explicit overflow rationale, and a documented batch junction query
(PROD-12) that kills the N+1. `refunds.rs` stock restoration implements
ADR-19 §5.3 faithfully (FIFO for full refunds, reverse for partial, with a
`qty ≤ total_deducted` guard and a warn-audited legacy fallback).

### Slice B5 part 5 — db/shifts.rs (479 lines), fully read

| ID | Sev | Location | Finding | Proposed solution |
|---|---|---|---|---|
| COR-27 | 🟡 LOW | db/shifts.rs:58–78 + init.sql:1259–1263 | **Open-shift dedup is advisory-only.** `open_shift` does COUNT-then-INSERT with no partial unique index behind it (verified: `shifts` has only plain indexes; the sibling `inventory_shifts` has exactly the needed `idx_inv_shifts_active_per_user_location`). Two concurrent opens for one user could both insert an `open` shift. | Forward-only migration: `CREATE UNIQUE INDEX … ON shifts(user_id) WHERE status='open'`. |

**Slice B5 part 5 positives:** `close_shift` runs every aggregation read
and the final write inside one transaction; expected-cash correctly
subtracts cash refunds (in-line documented fix of a false-shortage
accounting bug) and includes safe-drop payouts; the shift report's gross
profit matches the reporting layer's cost semantics; hour *labels* are UTC
(inside COR-21's blast radius) but totals are unaffected.

### Slice B5 part 6 — 14 remaining small/mid db files read + stamped

Files fully read: `cash_payouts` (86), `plans` (98), `settings` (280 — pure
CurrencyRepository delegation), `store_profiles` (213), `tables` (330),
`assignments` (273), `terminals` (260), `suppliers` (253), `promotions`
(252), `product_bundles` (251), `analytics` (234), `cart` (211),
`terminal_overrides` (133), `terminal_profiles` (104). Consolidated sweep
confirmed: all parameterized, zero unwraps, zero format!-SQL.

| ID | Sev | Location | Finding | Proposed solution |
|---|---|---|---|---|
| COR-28 | ℹ️ INFO | db/cash_payouts.rs:28–49 | Open-shift check runs outside the insert — a concurrently closed shift can still receive a payout (TOCTOU, advisory class, low stakes). | Move check+insert into one tx. |

**Notes:** `promotions.update_promotion` validates only the name while
create validates type/amounts too (COR-12-class asymmetry, INFO).
`terminals.terminal_secret` plaintext (same class as COR-17).
`store_profiles.timezone` **exists in the schema** — the COR-21 fix already
has a data home that reports never consult. `analytics.rs` DATE() bucketing
joins COR-21's blast radius. Clean highlights: `assignments.rs` fail-closed
scope evaluation matches ADR #35 D5 exactly; `cart.rs` preserves the
ADR-19 §5.1 location lock via COALESCE upsert; `tables.rs` TBL-08 geometry
validation is exemplary; `store_profiles` swaps the primary invariant in a
transaction with rollback.

### Slice B5 finale — db/audit.rs (479) + db/purchase_orders.rs (520) + db/stock_counts.rs (609), fully read

| ID | Sev | Location | Finding | Proposed solution |
|---|---|---|---|---|
| COR-29 | 🟡 LOW | db/purchase_orders.rs:452 | `received + damaged > line.qty` uses plain `+` — the one unchecked add in an otherwise fully checked file; i64 overflow wraps negative in release builds and bypasses the ordered-qty cap (stock itself stays guarded by `checked_add` inside the stock API, so corruption is limited to PO line records). | `checked_add` to match the file's own MONEY-05 contract. |

**Slice B5 finale positives:** `audit.rs` is a model append-only log —
AUD-06 secret redaction (20 sensitive keys, case-insensitive, with a
byte-preserving fast path), payload truncation with an explicit marker,
keyset `(created_at, id)` pagination, a 100k-row export bound, review
checkpoints committing with their own audit event, and schema-level
immutability triggers backing the append-only claim. `purchase_orders.rs`
implements the MONEY-05 checked-arithmetic contract at the IPC boundary
(doctored dev-build overflow rationale included) with atomic create and
atomic receive; `stock_counts.rs` allocates count numbers inside the INSERT
under `BEGIN IMMEDIATE` (with dangling-transaction rollback discipline),
claims completion conditionally, and is checked-arithmetic throughout.

> **Slice B5 complete:** all 41 production files in `db/` + `migrations.rs`
> are now read and stamped. Remaining oz-core: `kds.rs` (1,312),
> `workspaces.rs` (1,106), `popularity.rs` (813), `staff.rs` (636) — the
> last four mid files — then slices C and D.

### Slice B5 closeout — db/staff.rs (678) + db/popularity.rs (858), fully read — **db layer 100% done**

Both exemplary, zero new findings. `staff.rs`: STAFF-07 three-tier login
rate limiter (per-account / per-device / global + exponential backoff, all
tunables in one `LoginLimits` object), role-preset upsert-sync that
converges existing databases onto current grants, fail-closed permission
resolution (an unresolvable role denies instead of crashing), normalized
usernames with conflict → typed error, and a default global assignment per
ADR #35 D5. User PINs are stored hashed (`pin_hash`) — contrast COR-17's
plaintext *gift-card* PINs. `popularity.rs`: the B5-part-6 flagged
`format!`-SQL spots verified injection-safe (whitelisted period-expression
trio), full-catalog pass runs as 4 grouped queries with atomic score writes
in one transaction, category-smoothed means per ADR #37 D6, breadth factor
documented; day buckets UTC (COR-21 family, tracked).

**The entire `db/` layer — 43 files, ~19.5k production lines — is now
audited and stamped.** oz-core remaining: `workspaces.rs` (1,106),
`kds.rs` (1,312), then slices C and D.

### Slice B final — db/workspaces.rs (1,196 lines), fully read

| ID | Sev | Location | Finding | Proposed solution |
|---|---|---|---|---|
| COR-30 | 🟡 LOW | db/workspaces.rs:385, 395, 1169, 1188 | **Access-resolution guards fail toward the permissive tier.** Four `.unwrap_or(false)` sites treat a DB error as "no rows" and fall through to the next (broader) resolution tier — e.g. an explicit-instance check error promotes the user to role-type fallback. Same family as COR-11/COR-25, rare under the single-connection mutex. | Propagate errors with `?`, or fail closed (deny) in the access path. Also: the hardcoded 8-variant role-id allowlist for the owner bypass is fragile if `ROLE_PRESETS` changes — derive it from the presets. |

**Workspaces positives:** the ADR #4 type/instance resolution chain is
clearly layered with documented semantics, quota enforcement validates the
*signed entitlement's* `allowed_types` (C3.2) rather than trusting static
tier defaults, the no-nesting transaction caveat is documented *and* pinned
by a test, and the dynamic-SQL fragments interpolate only internal
parameter markers (injection-safe).

### Slice B final — db/kds.rs (1,431 lines), fully read — **SLICE B COMPLETE**

Zero significant findings. The two flagged `format!`-SQL sites interpolate
only match-derived internal timestamp columns (injection-safe — closes the
B5-part-6 flag). Line-item status transitions are enforced by an explicit
`allowed()` state machine (order-level updates rely on the frontend's fixed
status set — INFO). Pairing tokens stored hashed, prep-time clamped ≥ 0,
fan-out normalized via `kds_order_targets` (no duplicate tickets), stale
devices auto-deactivated with logging.

> **SLICE B COMPLETE: the entire `db/` layer — 44 files, ~21k production
> lines — read and stamped.** oz-core remaining: slices C
> (sync_client/topology/features/settings + location_resolver,
> subscription, license_verification) and D (export/, cache, top-level kds,
> and the remainder).

---

### Slice C1 — sync_client.rs (1,337 lines), fully read

| ID | Sev | Location | Finding | Proposed solution |
|---|---|---|---|---|
| COR-31 | 🟡 LOW | sync_client.rs:1138 | `fetch_snapshot_from_server` builds `reqwest::Client::new()` with **no timeout** — the single path that downloads a large payload (the authoritative snapshot) can hang indefinitely on a stalled connection. Every other client in the file carries 10/15/30s timeouts. | Give it the same explicit timeout (60s, sized for the payload). |

**Slice C1 positives:** the sync-auth-hardening ADR is fully realized —
typed `SyncHttpError` classification (401 expired ⇒ refresh-and-retry-once;
401 invalid ⇒ configuration problem, never masked; 403 `plan_required` ⇒
terminal upgrade state with no retry or quarantine), admin-key mint gating
(P2), client-credentials path (P3). Snapshot pull applies in **one**
transaction with model credential hygiene: server user rows upsert with a
placeholder pin hash that can never verify, `pin_hash` is omitted from the
UPDATE clause, and `deny_unknown_fields` makes a misbehaving server fail
loudly instead of silently importing credential material.

### Slice C2 — topology.rs (711) + location_resolver.rs (540), fully read

| ID | Sev | Location | Finding | Proposed solution |
|---|---|---|---|---|
| COR-32 | 🟡 LOW-MED | location_resolver.rs:41–78, 334, 361, 371 | The 30s-TTL `LOCATION_CACHE` has **no production invalidation caller** — `invalidate_location_cache()` is invoked from tests only. After an admin rebinds a workspace (single or primary binding), terminals keep resolving the **old** location for up to 30 seconds and stock deductions land on the stale target. | Call `invalidate_location_cache()` (or per-key eviction) from every binding mutator: `set_workspace_inventory_locations`, `bound_location_id` updates, and topology apply. |

**Slice C2 positives:** `topology.rs` is a model pure-validation engine —
fail-closed vendored contract init with a documented INVARIANT, every
frontend-parity gate annotated with its rationale (ungated wire direction,
zero-vs-multiple branch codes), O(N+W) single-index gates, a closed
semantic pairing matrix mirrored at the IPC boundary, Kahn cycle detection,
and structured `TopologyValidation` error codes. `location_resolver.rs`
implements the strict ADR-19 §4 priority tree with split-brain detection in
both resolver paths, and its chain resolver degrades fail-closed for
display purposes.

### Slice C3 — settings.rs (836 lines), fully read

No new finding IDs — a clean typed delegation facade over
`platform_core::settings::Settings` (raw KV + delta ledger per ADR #22 +
feature-flag integration). The rounding-mode parse falls back to the
documented TAX-05 default while the wire-string setter rejects garbage;
`set_batch` is one transaction; `prune_stale_features` loops individual
removes without a tx (RUST-08 advisory, INFO). **Note:** secrets (sync API
key, terminal secret, PG password, Redis URL, exchange-rate key) are stored
plaintext in the unencrypted `settings` table — the same local-POS threat
model accepted for COR-17/COR-30, tracked as a single revisit item when
multi-terminal/cloud sync hardening happens.

### Slice C4 — features.rs (1,349 lines), fully read

| ID | Sev | Location | Finding | Proposed solution |
|---|---|---|---|---|
| COR-33 | ℹ️ INFO | features.rs:691–1349 | **~660 lines of inline `#[test]` + proptest code live inside the production file**, despite the file also declaring the sibling `features_tests.rs` — violates the AGENTS.md test-organization rule and pushes the file past the 1,000-line ceiling. | Move the inline tests into the existing sibling test file. |

**Slice C4 notes:** the production half (1–685) is clean — a dependency
DAG with recursive enable and documented non-cascading disable, kebab-case
settings keys with exhaustive round-trip mapping, and a `FeatureGuard`
veto registry protecting open KDS tickets and unreconciled shifts. The
guard COUNT queries use `.unwrap_or(0)`, so a DB error lets the veto pass
(fail-open on a safety guard — COR-11/25 family, INFO). Property-based
tests verify the dependency invariant, disable semantics, and settings
round-trip — good test design, wrong location.

### Slice C5 — subscription.rs (680) + license_verification.rs (605), fully read — **SLICE C COMPLETE**

No new finding IDs — both files exemplary. `subscription.rs`: RSA
verification delegation, the 30-second clock-skew tolerance (tightened
from the M1 audit's 5-minute window, rationale documented), ledger-based
rollback detection (MAX over `sales` + `audit_log`), canceled-never-in-grace
with entitlements reverting to Free out-of-grace, and typed `QuotaError`s
with actionable upgrade messaging. `license_verification.rs`: RSA-2048
PKCS1v15/SHA-256 over a build-embedded key, the `BOOTSTRAP_FREE` sentinel
accepted **only in debug builds** (release requires a real signature), every
server response signature-verified **before** trust, credentials carried
only in Authorization headers (documented body-log-leak rationale), and
timeouts on all five HTTP clients. The api_key sits plaintext in
`tenant_subscription` — the accepted local threat model (COR-17/30 family).

### Slice D1 — export/mod.rs (780) + export/email_sender.rs (334), fully read

| ID | Sev | Location | Finding | Proposed solution |
|---|---|---|---|---|
| COR-34 | 🟡 LOW | export/email_sender.rs:60–79 | `build_smtp_transport` falls back to `builder_dangerous` (plaintext SMTP) when `use_tls=false` and port ≠ 465 — SMTP credentials would traverse the network unencrypted. | Warn on config save, or refuse credentialed plaintext SMTP. |

**Slice D1 notes:** `export/mod.rs`'s custom report builder is
injection-safe by construction — hardcoded per-dataset tables and column
whitelists (unknown columns silently dropped), parameterized dates,
clamped `u32` limit/offset — with correct CSV quote-doubling.
`email_sender.rs` resolves timezones through a ~20-zone fixed-offset table
with **no DST handling** (europe/london documented as a UTC approximation;
chrono-tz would fix) and falls back to UTC on unknown names (COR-21
family); the 2-minute send window with same-date dedup skips the day when
the app is closed at send time (INFO).

### Slice D2 — export/cloud_destination.rs (608) + export/email_report.rs (527), fully read

| ID | Sev | Location | Finding | Proposed solution |
|---|---|---|---|---|
| COR-35 | 🟠 MED | export/cloud_destination.rs:316–351 | **Snowflake export builds INSERT statements by string concatenation** with quote-only `sql_escape`. Snowflake treats `\` as an escape character inside string literals, so a user-controlled value ending in a backslash (product/store names) escapes the closing quote and breaks out of the literal — SQL injection into the customer's warehouse. | Use Snowflake API bind variables, or extend the escaper to double backslashes as well as quotes. |
| COR-36 | 🟡 LOW-MED | export/email_report.rs:434–436 | `render_text` truncates with **byte slicing** (`&row.name[..21]`, guarded by a byte-length check) — panics when byte 21 falls mid-UTF-8 (multi-byte product names crash scheduled email rendering). | Truncate on char boundaries (`chars().take(21)`). |

**Slice D2 positives:** the BigQuery path is exemplary — service-account
JWT (RS256) minted correctly with the `rsa` crate, token exchanged at
Google's OAuth endpoint, rows sent via bearer-authenticated `insertAll`.
`email_report.rs` encrypts the SMTP password at rest via `crate::crypto`
with transparent decryption and a test-pinned legacy-plaintext fallback,
and escapes every user-controlled HTML cell. All four HTTP clients in
`cloud_destination.rs` use `Client::new()` without timeouts (COR-31
family); the service-account key and Snowflake password persist in the
settings JSON (base64 ≠ encryption — COR-17/30 family).

### Slice D3 — cache.rs (417) + kds.rs top-level (342), fully read

No new finding IDs — both clean. `cache.rs`: the `Cache` trait with a
`NoopCache` fallback and a feature-gated `RedisCache` where every Redis
error degrades to a miss (fail-safe direction), the pub/sub listener runs
with 5-second read timeouts, skips its own terminal's messages, and exits
cleanly on shutdown. `kds.rs`: pure domain types plus
`resolve_kds_targets` station routing with documented broadcast fallback
and deduplication; pairing tokens arrive as SHA-256 hashes.

### oz-core closeout — COR-5 TTL verification + campaign tally

**COR-5 RESOLVED:** the production default session TTL is enforced at
startup — `apps/desktop-client/src/state.rs:246-250` reads
`session.ttl_seconds` with `unwrap_or(86400)` (**24 hours**), and
`auth.rs:383/552` sets `expires_at = now + ttl` whenever ttl > 0. A
never-expiring session now requires an *explicit* `session.ttl_seconds = 0`
setting (documented behaviour), not a missing value — the "None means
development mode" wording in `session.rs` is a doc nit: expiry is
config-driven, not build-mode-driven.

**oz-core final tally:** 36 tabled findings — **0 HIGH / 6 MED (COR-3,
COR-7, COR-19, COR-21, COR-25, COR-35) / 14 LOW / 16 INFO** —
plus the CRY-1 escalation (gift-card PIN plaintext) logged under oz-crypto.
All 60+ production files in `crates/oz-core/src` are read and stamped
(slices A, B, B1–B5+closeout+final, C1–C5, D1–D3). oz-core is COMPLETE;
the campaign proceeds to the remaining 27 targets.

---

## 5. crates/oz-api — REST server (JWT, Postgres/SQLite dual backend)

Baseline: 13 test files, ~3.6k production lines across lib/auth/pg/routes.
Slice A (lib, auth, routes/settings, routes/tokens) deep-read.

| ID | Sev | Location | Finding | Proposed solution |
|---|---|---|---|---|
| API-1 | 🟠 MED | oz-api/src/auth.rs:75–81 | **Hard-coded dev JWT signing secret fallback** (`"oz-pos-dev-secret-change-in-production"`) when `OZ_API_SECRET` is unset — anyone who knows the constant can forge valid tokens for every protected route on a misconfigured public server. There is no startup enforcement. | Refuse to serve (or log-a-fatal warn) when `OZ_PRODUCTION` is set and `OZ_API_SECRET` is missing; consider the same gate for `OZ_ADMIN_KEY`. |
| API-2 | ℹ️ INFO | oz-api/src/routes/tokens.rs:57–66, routes/settings.rs:118–124 | Admin-key comparison is non-constant-time (`==`), dev-open mode when `OZ_ADMIN_KEY` is unset (documented), and `GET /api/v1/settings` returns the tenant's SMTP password **decrypted** — a misconfigured dev-open deployment discloses credentials. | Constant-time compare; require admin key in production; document the decrypted-GET tradeoff. |

**Slice A positives:** security headers on every response (nosniff, DENY,
CSP, prod-only HSTS), fail-closed CORS parsing with documented dev opt-in,
structured 401 taxonomy per sync-auth-hardening P4, tenant-scoped settings
with charset validation and no-half-applied writes, and the terminal
client-credentials path sourcing tenant from the registration — never the
request body.

### Slice B — pg.rs (1,307 lines): module doc + helpers + tenant-plan +
terminal-verify + product-mapper regions fully read; remainder verified by
structural sweep (all `format!` sites build error strings or static SELECT
prefixes only; RLS `set_config` coverage counted; `unwrap_or` sites reviewed)

| ID | Sev | Location | Finding | Proposed solution |
|---|---|---|---|---|
| API-3 | ℹ️ INFO | oz-api/src/pg.rs:610, 892, 1149, 1211–1212 | Sale/product reads silently default on schema drift: `tip_minor`/`service_charge_minor` `.unwrap_or(0)` (money columns read as zero), `product_type`/`status` enum fallbacks (`ProductType::default`, `SaleStatus::Pending`). | Propagate column-read errors instead of defaulting money fields. |

**Slice B positives:** the RLS contract is exemplary — every tenant-scoped
function opens a transaction and sets `oz.tenant_id` as a LOCAL setting
(verified: 12 sites), so pooled connections never leak one tenant's scope
to the next; `verify_terminal_credentials` performs the documented
pre-tenant lookup through a scoped discovery role (`SET LOCAL ROLE` inside
a read-only transaction) and compares SHA-256 digests in SQL rather than
process-memory secrets; all SQL is parameterized; `PgError` maps cleanly
to 409/404/400/500.

### Slice C — route handlers: products (303), sales (227), users (122) fully
read; plans/terminals/tax_rates/categories/health verified structurally
(admin-key gating confirmed at plans.rs:94 and terminals.rs:105 via targeted
grep; handlers total ~600 further lines)

| ID | Sev | Location | Finding | Proposed solution |
|---|---|---|---|---|
| API-4 | 🟡 LOW-MED | oz-api/src/routes/users.rs:69–118 | `POST /api/v1/users` requires **any** valid JWT but no privilege check — a leaked label or terminal-scoped token can create a user with `role-owner` and then obtain owner sessions. | Gate on admin-minted tokens or an owner-scope claim; keep the endpoint out of general terminal tokens. |

**Slice C notes:** sales accept client-supplied unit prices (the automation
API contract — any valid token can book sales at arbitrary prices; INFO).
products/users stamp `tenant_id` via a follow-up UPDATE with a documented
warn-only degrade on the SQLite path. Plans and terminal registration are
confirmed admin-key-gated. All handlers use typed error→HTTP mapping with
no leaked internals.

> **oz-api complete** (lib, auth, pg, 12 route files — all read or
> structurally verified with targeted gate checks). Findings: API-1
> (🟠 dev-secret fallback), API-2/3 (ℹ️), API-4 (🟡 user-creation
> privilege gap). Campaign proceeds to crates/oz-foundation.

---

## 6. foundation — domain primitives (Money, Percentage, Cart, validation)

Baseline: ~5.4k production lines across 15 files. Slice A
(money.rs 326, percentage.rs 390) deep-read — the codebase's two most
safety-critical value objects. Both carry prior MONEY-AUDIT stamps that
were re-verified intact.

**No new findings.** `money.rs`: checked arithmetic everywhere with
documented `i64::MIN` edges, currency-mismatch → `None` (never silent
cross-currency math), a deliberate no-`Ord` design with documented
`PartialOrd`-only rationale, `i64::MIN`-safe rendering via `unsigned_abs`,
and the minor-unit exponent table's four sync points listed in-doc.
`percentage.rs`: the MONEY-AUDIT-2 overflow-free decomposition holds
(100% of `i64::MAX` = `i64::MAX`, edge-tested), arithmetic is total, and
construction is bounded including the serde path. Both files carry the
COR-33 inline-test pattern note where applicable.

### Slice B — cart.rs (1,205 lines: production 1–353 fully read; 355–1205
are inline tests, boundary confirmed by structure scan)

**No new findings.** The MONEY-AUDIT-3 fixes hold: `CartLine::total()`
fails closed on a serde-bypassed `qty <= 0` (no zero/negative totals from
corrupt persisted carts), and `discount_amount()` propagates failures
instead of masking with `.or(Some(zero))`. Fixed discounts are capped at
the payable total via `Money::min`, percentage and fixed discounts are
mutually exclusive by construction, and `debug_assert!` currency guards
catch direct-field mutation in dev builds. The ~850-line inline test
block repeats the COR-33 pattern (AGENTS.md wants sibling test files).

### Slice C — validation.rs (1,000 lines: production 1–441 fully read;
442–1000 are the inline test block)

**No new finding IDs.** All validators fail closed with field-named errors,
regexes compile once via `LazyLock`, and every function carries doctests.
One INFO note: `validate_min_length`/`validate_max_length` count **bytes**,
not chars, so multi-byte UTF-8 display names hit length caps early (a 50
"character" cap admits ~16 CJK characters) — a chars-vs-bytes decision is
worth making explicitly for display-name fields.

### Slice D — sku.rs (223), barcode.rs (237), events.rs (265), errors.rs +
constants.rs + lib.rs verified — all clean

No new findings. The `Sku`/`Barcode` newtypes validate on construction AND
serde (trim + non-empty, fail-closed); domain events are typed with `i64`
minor units and serde defaults; `errors.rs` is three thiserror structs;
`constants.rs` documents every magic number (basis-point denominator,
length limits) with doctests. The lib-level old stamp was replaced; the
crate re-verifies the original audit's claims (zero unsafe, no FFI/IO,
doc-enforced). `dto/contracts/contact/enums` remain for slice E.

### Slice E — contracts.rs (476), enums.rs (359), contact.rs (393),
dto.rs (447) — **foundation COMPLETE**

No new findings. `contracts.rs` defines the Module/Service/EventHandler
contracts with documented topological dependency semantics;
`enums.rs` carries an exhaustive sale-status transition matrix with
fail-closed `from_stored_str` (returns `None`, never a default) and
round-trip serde tests; `contact.rs`/`dto.rs` are validated newtypes and
convention-documented DTOs whose only `unwrap()`s are inside test modules
(line-range verified).

> **foundation COMPLETE** — all 15 production files (~5.4k lines) read or
> structurally verified and stamped. Zero new findings beyond the
> chars-vs-bytes INFO note; the prior MONEY-AUDIT 1–3 fixes verified
> intact. Campaign proceeds to platform/* (core, kernel, startup, sync).

---

## 7. platform/core — RBAC, permissions, settings, database

Baseline: ~5.7k production lines (settings/tests.rs excluded). Slice A
(rbac.rs 1,408: 1–910 fully read; 911–1408 is the permission-constant
catalog, verified structurally).

**No new findings.** The wildcard resolver is a closed three-level design
(global `*`, domain `x:*`, exact — malformed strings match exactly only),
and `Role::has_permission` treats malformed stored JSON as **deny-all**
(fail-closed — the correct contrast to COR-30's fail-open access guards).
The preset catalog is pinned by invariant tests: no retired
cashier/kitchen roles, Admin is never a wildcard and never holds
staff-deletion by default, Staff is checkout-only (40+ negative
assertions), Auditor is read-only.

### Slice B — auth.rs (202) + permission_registry.rs (753), fully read

**No new findings — both exemplary.** `auth.rs`: Argon2id with per-hash
salts; malformed hashes *and* the sync snapshot placeholder fail closed to
`Ok(false)` (test-pinned, cross-referenced with oz-core's
`SNAPSHOT_PIN_HASH_PLACEHOLDER`, so an imported operator can never log in
without a real credential). `permission_registry.rs`: the ADR #35 D3
single-source-of-truth registry — sensitive keys (voids, refunds,
settlement, role management, staff deletion, bulk export) are structurally
ung&shy;r&shy;antable through family wildcards, the global `*` is reserved
for the Owner seed, and duplicate keys are invariant-tested.

### Slice C — settings/raw.rs (281) + settings/typed.rs (634) fully read;
settings/keys.rs (153) + mod.rs verified — **no new finding IDs**

`raw.rs` implements the DB-08 delta-ledger concurrency contract exactly as
documented: the UNIQUE `(key, terminal_id, version)` index turns
concurrent allocations into a constraint collision, standalone writers
retry under `BEGIN IMMEDIATE` (bounded, 32 attempts), nested callers use a
savepoint with lingering-savepoint logging, and delta loss is documented
non-fatal with a sync reconstruction path. `typed.rs` **narrows the
earlier COR-17/30-family note**: the sync API key, terminal secret, and PG
password are transparently **encrypted at rest** via `oz_crypto` (SMTP
password likewise via `encrypt_smtp_at_rest`), so the "secrets plaintext"
note now applies only to gift-card PINs (COR-17) and terminal secrets in
`oz-core`'s own `terminals` table — not the platform settings store. One
INFO: a decrypt failure silently falls back to the raw value instead of
surfacing an error, so a corrupted ciphertext yields garbage without an
alert.

### Slice D — database/migrations.rs (836: production 1–337 fully read;
338+ inline tests)

**No new findings — exemplary.** DB-02 checksum verification with
legacy-checksum migration and drift re-apply (the idempotency requirement
is documented and the re-apply is transaction-atomic, so partial DDL is
impossible — worst case is a clean failure requiring operator action).
DB-05 FK isolation wraps every apply and rollback (the PRAGMA-inside-
transaction no-op is the documented rationale), the caller's prior setting
is restored even on failure, and a restore error never masks the original.
Rollback is last-migration-only, preventing out-of-order reverts.

### Slice E — database/manager.rs (370), database/pool.rs (181),
terminal_profile.rs (388), error.rs (67), lib.rs — **platform-core COMPLETE**

| ID | Sev | Location | Finding | Proposed solution |
|---|---|---|---|---|
| PC-1 | ℹ️ INFO | platform/core/src/database/manager.rs:161, terminal_profile.rs:173 | `store_db_path` and the terminal-profile writer interpolate ids into filesystem paths without sanitization (`data_dir.join(format!("store-{id}.sqlite"))`, `{terminal_id}.json`). Ids are UUID-minted in normal flows, but snapshot-imported ids could path-traverse file creation on the local desktop. | Validate ids against a UUID/slug charset before any path join. |

`manager.rs` holds the cache guard across the check-then-insert span
(documented TOCTOU-safe) and recovers from partial creation via idempotent
migration runs; `pool.rs` is the correct single-connection SQLite wrapper
with WAL/FK pragmas; `error.rs`/`lib.rs` are taxonomy/re-exports.

> **platform-core COMPLETE** — 12 production files, ~5.7k lines, all read
> or structurally verified and stamped. One INFO (PC-1). Campaign proceeds
> to platform/kernel, platform/startup, platform/sync.

---

## 8. platform/kernel — module system (event bus, manifest, lifecycle)

Baseline: ~2.1k production lines. Slice A (event_bus.rs 835: production
1–327 fully read; 328+ inline tests).

**No new findings — exemplary.** The synchronous topic bus prevents
reentrant deadlocks by snapshotting the handler list under a short-lived
read lock and dispatching after release (Bug #2 documented); handler
panics are isolated via `catch_unwind` with structured logs (the
publisher never dies); handler errors are logged-not-propagated per the
documented fire-and-forget contract; module-scoped unsubscribe removes
handlers atomically across topics; RwLock poisoning is recovered via
`into_inner` as a documented design choice.

### Slice B — kernel/lifecycle.rs (608: production 1–300 fully read, tail
structurally verified), manifest.rs (527, verified — unwraps test-only at
206+), error.rs — **platform-kernel COMPLETE**

**No new findings.** The kernel honors the Service contract precisely —
`stop()` runs only on services that actually started (partial `start_all`
failures tracked via `started_service_ids`), shutdown continues past the
first error, registration enforces manifest/module id matches, and
dependency resolution fails with `MissingDependency`. `manifest.rs`
mirrors the formal JSON Schema with validation; `error.rs` is taxonomy.

> **platform-kernel COMPLETE** — 6 production files, ~2.1k lines, all read
> or verified and stamped, zero findings. Campaign proceeds to
> platform/startup and platform/sync.

---

## 9. platform/startup — shared client bootstrap

Baseline: ~1.8k production lines. Slice A (lib.rs 334 fully read;
event_handlers.rs 1,219: production 1–470 fully read, 471+ inline tests).

**No new findings.** lib.rs pins the module-registration set with a parity
test against `modules/*/manifest.json` (so a new module without a
manifest-registered sibling fails startup), documents the
loyalty-registration fix, and runs daemons through a panic-isolated
`spawn_daemon` harness that logs instead of crashing. The six shared
event handlers share one uniform lock→Store→enqueue-or-audit pattern with
poison-safe lock mapping and structured logs; sale completions are
enqueued at `SyncPriority::Critical` per P-2. One pattern note: the
1,219-line `event_handlers.rs` carries inline tests (COR-33 family).

### Slice B — rate_sync.rs (405: production 1–317 fully read, tests 319+),
console.rs + metrics.rs verified — **platform-startup COMPLETE**

**No new findings — exemplary.** The exchange-rate daemon re-reads its
settings every tick (configuration changes need no restart), converts
API `f64` rates to `i64` millionths via documented fixed-point rounding
with a bounded-range safety rationale, applies RUST-07 poison recovery on
both DB phases, isolates blocking work in `spawn_blocking` with join-error
handling, shuts down through a watch channel, and records per-rate upsert
failures without aborting the cycle.

> **platform-startup COMPLETE** — 5 production files, ~1.8k lines, all read
> or verified and stamped; zero new finding IDs. Campaign proceeds to
> platform/sync.

---

---

---

---

---

---

---

---

---

---

---

---

---

---

---

---

---

---

---

---

> **SLICE C COMPLETE:** sync_client, topology, location_resolver, settings,
> features, subscription, license_verification — 7 files, ~6,100 lines,
> all read and stamped. oz-core remaining: slice D (export/ ~3,121,
> cache.rs, top-level kds.rs, and the remainder).

---

---

---

---

---

*This file is appended after each completed crate audit. Findings get IDs
prefixed by crate (`CRY-`, `SEC-`, `PAY-`, `COR-`, …) so they can be referenced
in commits and specs without ambiguity.*
