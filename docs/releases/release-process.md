# Release Process — OZ-POS

This document captures the operational runbook for shipping a release of
OZ-POS. It exists because the updater pubkey is a security-critical value
that operators must know how to rotate safely (audit finding **L-4**).

## Publishing a release

1. **Bump version** — run `scripts/bump-version.ps1 <new-version>` from the
   release branch. This rewrites version strings in `Cargo.toml`,
   `tauri.conf.json`, `ui/package.json`, and `CHANGELOG.md` so they stay in
   sync.
2. **Build signed installers** — `cargo build --release` produces the
   platform binaries; the updater plugin signs them with the Ed25519 pubkey
   stored in `tauri.conf.json::plugins.updater.pubkey`.
3. **Generate release manifest** — the build pipeline uploads the signed
   artifacts and writes a `<version>.json` manifest at the update endpoint
   referenced from `tauri.conf.json::plugins.updater.endpoints`.
4. **Tag and announce** — push the signed release tag, publish the GitHub
   release notes (mirroring the new `CHANGELOG.md` section).

## Updater pubkey rotation

L-4 audit note: the Ed25519 pubkey for `tauri-plugin-updater` is currently
hardcoded in `tauri.conf.json::plugins.updater.pubkey`. The configuration
is sufficient for production but operators must know how to rotate the key
when the existing signing key is compromised or reaches end-of-life.

### When to rotate

- **Compromise** — the private signing key has been exposed, leaked, or
  exfiltrated (e.g. developer laptop loss). Rotate **immediately**; treat
  any artifact signed by the old key as untrusted.
- **Scheduled rotation** — every 24 months minimum, per the audit doc's
  recommendation for cryptographic-primitive supply-chain hygiene.
- **Algorithm rotation** — Tauri 2 currently uses Ed25519. If a future
  quantum-safe algorithm is adopted, the pubkey format and signing
  pipeline must be regenerated.

### How to rotate

1. **Generate a fresh signing keypair** using `cargo tauri signer generate`
   or `tauri signer generate -w ~/.tauri/oz-pos.key`. Save the **private**
   key in a secure secrets manager (not the repo).
2. **Encode the new public key** as the standard Tauri base64 string
   (one long line, no newlines).
3. **Edit `apps/desktop-client/tauri.conf.json`** and replace
   `plugins.updater.pubkey` with the new base64 string.
4. **Re-sign all release artifacts** for the next release — the new key
   cannot validate previously-signed artifacts, so clients installed
   via older releases must update through a Tauri-side delta or a fresh
   install.
5. **Document the rotation** in `CHANGELOG.md` under
   `### Security` with the old pubkey prefix (first 8 chars) and the new
   prefix so operators can audit.
6. **PR review** — the pubkey rotation change must be reviewed by at
   least two maintainers; the change is not a routine edit.

### Emergency revocation

If the private key is compromised mid-release:

1. **Publish an advisory** immediately via the project's `SECURITY.md`
   contact channel.
2. **Revoke at the update endpoint** — if the manifest endpoint is under
   operator control, take it offline. Existing client installations
   will fail to update, which is correct (better than running
   compromised artifacts).
3. **Coordinate a forced re-install** with every known downstream
   operator.

## Pre-release checklist

See [`docs/releases/checklist.md`](./checklist.md) and
[`docs/releases/mobile-checklist.md`](./mobile-checklist.md) for the
operational pre-release checks. Both are referenced from
`scripts/release.sh`.
