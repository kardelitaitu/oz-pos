# Release Checklist — OZ-POS

> Follow these steps in order for every release. Mark each item as completed.

## Pre-Release

- [ ] All CI jobs pass (rust-fmt, rust-clippy, rust-test-fast, ui-lint, ui-typecheck, ui-test)
- [ ] `cargo nextest run --workspace --all-features --profile ci` passes locally
- [ ] `cd ui && npm run typecheck && npm run lint && npm run test` passes locally
- [ ] `bash scripts/lint-i18n.sh` clean (no duplicate FTL keys, no verbatim ID bundles)
- [ ] Changelog generated at `docs/releases/CHANGELOG-{version}.md` with all changes since last release
- [ ] `## [{version}]` heading present in canonical `CHANGELOG.md` (the release version gate `node scripts/check-release-version.mjs v{version}` passes locally)
- [ ] Version bumped in: `Cargo.toml` (workspace), `ui/package.json`, `tauri.conf.json` files (via `scripts/bump-version.ps1 {version}`)
- [ ] Breaking changes documented with migration guide if needed

## Build Verification

- [ ] Docker image builds: `docker build -f Dockerfile.server -t oz-pos-cloud:latest .`
- [ ] Docker image size < 100 MB
- [ ] Desktop **installers** build (raw `cargo build --release` is not a release artifact):
  - Linux: `cargo tauri build --bundles appimage,deb`
  - Windows: `cargo tauri build --bundles nsis,msi` (code-signed when `UPDATER_CERT` is set)
  - macOS: `cargo tauri build --bundles dmg`
- [ ] Desktop binary size < 50 MB
- [ ] UI bundle builds: `cd ui && npm run build`
- [ ] UI bundle size < 5 MB

## Updater Manifest (RELEASE-04)

- [ ] Release contains `latest.json` + `beta.json` (matching `tauri.conf.json` updater endpoints)
- [ ] `node scripts/generate-latest-json.mjs --self-test` and `node scripts/verify-updater-signature.mjs --self-test` pass
- [ ] `node scripts/verify-updater-signature.mjs latest.json <platform> <installer>` verifies for every shipped platform
- [ ] `UPDATER_PRIVATE_KEY` secret derives to the pubkey embedded in `tauri.conf.json` (the workflow fails otherwise)
- [ ] Release contains `SHA256SUMS.txt` and provenance attestations for the exact installers

## Smoke Test

- [ ] App launches without errors
- [ ] Login flow works (PIN entry → workspace picker → POS screen)
- [ ] Basic sale works (add product → pay → receipt)
- [ ] Settings page loads and saves
- [ ] Offline mode works (disable network, complete sale)

## Release

- [ ] Git tag created: `git tag -a vX.Y.Z -m "Release vX.Y.Z"`
- [ ] Release workflow's version gate passed (`release-validate` job, tag ↔ app versions ↔ `CHANGELOG.md`)
- [ ] GitHub Release created (draft → published only after asset inventory passes)
- [ ] Docker image pushed to GHCR
- [ ] Desktop installers built + attached (AppImage/deb, NSIS/MSI, DMG)
- [ ] Mobile APK/AAB + IPA attached to the same release (via their tag workflows)
- [ ] Rollback verified: previous version installer reinstalls cleanly on a test terminal (see `release-process.md`)
- [ ] Release announced to team/channel
