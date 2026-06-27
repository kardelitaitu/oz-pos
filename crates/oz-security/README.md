# oz-security

Encryption, secrets, and PCI-DSS helpers for OZ-POS. At-rest encryption, key rotation, and the small set of PCI-DSS-related utilities the cashier flow needs (masked PAN display, audit logging, secure memory).

## Public API

- [`SecurityError`](src/error.rs) — `thiserror`-based error for the security subsystem.

## Planned surface

- AES-GCM helpers for at-rest encryption of `*.db` and exported session data.
- KEK/DEK envelope with key rotation support.
- PCI-DSS utilities: PAN masking, audit-log helpers, secure memory zeroing.
- Integration with the OS keystore (Windows Credential Manager, libsecret, Keychain).

## Status

Scaffold only. Production code lands in a follow-up once `oz-payment` defines the secret shape it needs.
