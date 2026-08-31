<!-- Audit stamp: 2026-07-22 · Hermes-Agent · status: ACCURATE (0 findings, code matches doc) · verified against crates/oz-security/src: all 6 modules present (error.rs, mask.rs, tls.rs, linux.rs, macos.rs, windows.rs); Keyring trait at lib.rs:78; default_keyring() at lib.rs:139 returns platform-native or InMemoryKeyring (lib.rs:166); #![deny(unsafe_code)] at lib.rs:24 (windows.rs uses #![allow(unsafe_code)] for FFI, documented with SAFETY comments) · doc's "scaffold" label at docs/ARCHITECTURE.md:79 is itself stale (this crate is fully implemented) · RE-AUDITED 2026-08-31 by docs-auditor: all 6 modules + Keyring/default_keyring/InMemoryKeyring/deny(unsafe_code) re-confirmed against current HEAD; three post-08-29 commits are additive, not contradicting — b6692a92 mask_token (bearer-credential masking, covered by the "sensitive-data masking" row), fe655711 SEC-4 staged rotate_key + SEC-6 entropy scrubbing (rotate_key is an extra trait method beyond the illustrative set/get/delete example), 20dc2054 clippy lint repairs -->

# oz-security

Encryption, secrets, and PCI-DSS helpers for OZ-POS.

## Public API

| Module | What |
|--------|------|
| `error` | `SecurityError` (thiserror) |
| `mask` | PAN / sensitive-data masking |
| `tls` | TLS configuration helpers |
| `linux` | `LibSecretKeyring` — Linux Secret Service (libsecret/DBus) |
| `macos` | `MacOsKeychain` — macOS Keychain (Security framework) |
| `windows` | `WindowsCredentialManager` — Windows Credential Manager |

### Keyring trait

OS-level credential store abstraction:

```rust
use oz_security::Keyring;

let keyring = oz_security::default_keyring()?;
keyring.set_secret("api-key", "sk_live_abc123")?;
let secret = keyring.get_secret("api-key")?;
keyring.delete_secret("api-key")?;
```

`default_keyring()` returns the platform-native keyring. CI/dev fallback is `InMemoryKeyring` (not secure).

## Conventions

- `#![deny(unsafe_code)]` — platform modules may use FFI with `// SAFETY:`.

> last audited 31-08-26 by docs-auditor
