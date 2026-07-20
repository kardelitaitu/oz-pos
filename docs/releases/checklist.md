# Release Checklist — OZ-POS

> Follow these steps in order for every release. Mark each item as completed.

## Pre-Release

- [ ] All CI jobs pass (rust-fmt, rust-clippy, rust-test-fast, ui-lint, ui-typecheck, ui-test)
- [ ] `cargo nextest run --workspace --all-features --profile ci` passes locally
- [ ] `cd ui && npm run typecheck && npm run lint && npm run test` passes locally
- [ ] `bash scripts/lint-i18n.sh` clean (no duplicate FTL keys, no verbatim ID bundles)
- [ ] `CHANGELOG.md` updated with all changes since last release
- [ ] Version bumped in: `Cargo.toml` (workspace), `ui/package.json`, `tauri.conf.json` files
- [ ] Breaking changes documented with migration guide if needed

## Build Verification

- [ ] Docker image builds: `docker build -f Dockerfile.server -t oz-pos-cloud:latest .`
- [ ] Docker image size < 100 MB
- [ ] Desktop binary builds: `cargo build --release -p oz-pos-app`
- [ ] Desktop binary size < 50 MB
- [ ] UI bundle builds: `cd ui && npm run build`
- [ ] UI bundle size < 5 MB

## Smoke Test

- [ ] App launches without errors
- [ ] Login flow works (PIN entry → workspace picker → POS screen)
- [ ] Basic sale works (add product → pay → receipt)
- [ ] Settings page loads and saves
- [ ] Offline mode works (disable network, complete sale)

## Release

- [ ] Git tag created: `git tag -a vX.Y.Z -m "Release vX.Y.Z"`
- [ ] GitHub Release created with changelog notes
- [ ] Docker image pushed to GHCR
- [ ] Desktop installers built (if applicable)
- [ ] Release announced to team/channel
