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
| 2 | crates/oz-security | 2,068 | ⬜ pending | — | — |
| 3 | crates/oz-payment | 6,251 | ⬜ pending | — | — |
| 4 | crates/oz-core (sliced by subsystem) | 80,216 | ⬜ pending | — | — |
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

*This file is appended after each completed crate audit. Findings get IDs
prefixed by crate (`CRY-`, `SEC-`, `PAY-`, …) so they can be referenced in
commits and specs without ambiguity.*
