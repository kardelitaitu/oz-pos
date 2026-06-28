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

> last audited 28-06-26 by docs-auditor
