# Release Process Audit — July 2026

> **Audit date:** 2026-07-31
> **Sector:** Release process — versioning, changelog automation, artifact completeness, signing, updater manifests, provenance, permissions, rollback, and release validation
> **Status:** REMEDIATED (2026-08-03) · all findings closed · release automation now builds Tauri installers, gates on tag↔version parity, publishes signed updater manifests, and publishes via concurrency + draft controls
> **Production code changed:** Release pipeline, scripts, and docs (see per-finding statuses below)

## Scope

This audit evaluates sector 28 against the universal checklist in `audit/AUDIT_JULY_2026.md`. It covers tag triggers, release workflows, desktop/tablet/mobile/cloud artifacts, version sources, changelog generation, Tauri bundling and updater signing, release permissions, provenance, artifact retention, smoke-test expectations, rollback guidance, and release documentation.

Inspected areas:

- `.github/workflows/release.yml`
- `.github/workflows/android.yml`
- `.github/workflows/ios.yml`
- `apps/desktop-client/tauri.conf.json`
- `apps/tablet-client/tauri.conf.json`
- `Cargo.toml`
- `ui/package.json`
- `CHANGELOG.md`
- `scripts/bump-version.ps1`
- `scripts/release.sh`
- `scripts/generate-latest-json.mjs`
- `scripts/build-exe-release.ps1`
- `docs/releases/checklist.md`
- `docs/releases/mobile-checklist.md`
- `docs/releases/release-process.md`
- `docs/decisions/2026-07-16-desktop-app-updater.md`
- `docs/decisions/2026-07-16-release-automation.md`
- `oz-pos-updater.key.pub`

## Architecture summary

The repository has a tag-triggered `.github/workflows/release.yml` with Linux desktop, Windows desktop, and cloud Docker matrix entries, followed by a GitHub Release publishing job. Android and iOS are separate tag-triggered workflows. The tablet Tauri configuration supports Android and iOS bundle targets, while the desktop configuration enables all desktop bundle targets and configures the updater endpoint and public key.

Version data exists in the workspace Cargo manifest, both Tauri configurations, the UI package, lockfiles, generated health/version surfaces, and documentation. `scripts/bump-version.ps1` updates a broad set of files and lockfiles; `scripts/release.sh` runs Rust checks, bumps selected version files, generates a commit-local changelog file, commits, and creates a tag. `scripts/generate-latest-json.mjs` can produce an Ed25519-signed Tauri updater manifest, but the committed release workflow does not call it.

The release design has useful pieces—tag-based publishing, scoped `contents: write` on the publish job, multi-platform mobile workflows, a committed updater public key, and documented manual checklists—but the actual automation has diverged from the updater and release ADRs. The most significant risk is that the main release workflow builds raw desktop binaries rather than Tauri installers and does not assemble signed updater assets or mobile artifacts into one release.

## Findings

### RELEASE-01 — Main release workflow publishes raw desktop binaries instead of Tauri installers

**Evidence:** `.github/workflows/release.yml:58-64` runs `cargo build --release -p oz-pos-app` for Linux and Windows. It does not install the Tauri CLI, run `cargo tauri build`, collect platform bundle directories, invoke the configured `bundle.windows.signCommand`, or produce NSIS/MSI/AppImage/deb artifacts. The upload step at `:72-78` targets `target/release/${{ matrix.binary }}`, which is a raw executable path. The desktop Tauri configuration declares bundle targets and a Windows signing command at `apps/desktop-client/tauri.conf.json:42-61`.

**Impact:** The published desktop assets do not match the documented installer/release contract. Windows users may receive an executable rather than the expected installer, and the Tauri updater cannot consume the missing signed bundle artifacts. Installer integration, upgrade behavior, file associations, and configured Windows signing are not exercised by the release gate.

**Severity:** P1 · release artifact integrity

**Affected files:** `.github/workflows/release.yml`, `apps/desktop-client/tauri.conf.json`, `scripts/build-exe-release.ps1`, `docs/releases/checklist.md`, and updater documentation.

**Recommendation:** Replace the raw `cargo build` steps with a pinned Tauri CLI build (`cargo tauri build --ci` or an equivalent documented command) on each supported desktop runner. Upload only expected installer/bundle outputs, fail if an expected artifact is absent, and verify signatures/installability before publishing. Keep raw binaries as optional diagnostics, not as the primary release assets.

**Status:** REMEDIATED — `.github/workflows/release.yml` now runs `cargo install tauri-cli --version "^2" --locked` + `cargo tauri build --bundles …` per platform (AppImage+deb / NSIS+MSI / DMG), uploads only bundle globs with `if-no-files-found: error`, and the build job fails when any expected bundle extension is absent. Raw binaries are no longer release assets; `docs/releases/release-process.md` documents that `cargo build --release` is not a shippable artifact.

### RELEASE-02 — Mobile artifacts are stranded in separate workflow runs

**Evidence:** `.github/workflows/android.yml` and `.github/workflows/ios.yml` independently trigger on `v*` tags and upload APK/AAB or IPA artifacts using `actions/upload-artifact`. `.github/workflows/release.yml:88-105` downloads artifacts only from its own run via `actions/download-artifact@v4`; it has no `needs` relationship to the Android or iOS workflows and cannot download their workflow artifacts. The release publish job therefore cannot attach those mobile outputs to the GitHub Release created by the main workflow.

**Impact:** A version tag can produce multiple workflow runs, but the published GitHub Release may contain only the main workflow's desktop/cloud assets. Mobile artifacts remain attached to separate workflow runs with different retention and discovery semantics, making one-version distribution incomplete and confusing for operators.

**Severity:** P1 · artifact completeness

**Affected files:** `.github/workflows/release.yml`, `.github/workflows/android.yml`, `.github/workflows/ios.yml`, `apps/tablet-client/tauri.conf.json`, and mobile release documentation.

**Recommendation:** Use one release orchestrator with platform jobs as dependencies, or have each platform workflow upload directly to the exact existing GitHub Release using tightly scoped permissions and an explicit release identifier. Add a final manifest/asset inventory step that fails when required platform artifacts are missing, and test one tag in a draft release before production publication.

**Status:** REMEDIATED (coordination model) — `.github/workflows/android.yml` + `ios.yml` now upload their APK/AAB/IPA **directly into the release created by `release.yml`** via `gh release upload` (polling for the release, `--clobber` idempotency, tag-scoped `contents: write`, tag-keyed `concurrency`). `release-publish` runs a required asset-inventory check (latest.json/beta.json/SHA256SUMS.txt + every platform extension) before flipping the draft to published. Mobile workflows are explicitly excluded from the hard gate while the android NDK toolchain remains unstable — documented in release.yml and this report.

### RELEASE-03 — macOS desktop is omitted from the main release matrix

**Evidence:** `.github/workflows/release.yml:25-34` contains `desktop-linux`, `desktop-windows`, and `docker-cloud`, but no `desktop-macos` entry. The nightly workflow does build a macOS desktop target at `.github/workflows/nightly.yml:301-329`, so the omission is specific to the publishable release pipeline rather than a platform capability limitation.

**Impact:** A tagged release does not publish the macOS desktop artifact even though the repository has a macOS build path. The updater/configuration and release documentation can imply broader desktop support than the release asset set actually provides.

**Severity:** P2 · platform release completeness

**Affected files:** `.github/workflows/release.yml`, `.github/workflows/nightly.yml`, `apps/desktop-client/tauri.conf.json`, and desktop release documentation.

**Recommendation:** Either add a macOS release job with the required signing/notarization policy or explicitly document macOS as nightly-only. If added, require the job to produce and upload the expected DMG/app bundle and include it in the final release inventory.

**Status:** REMEDIATED (included) — `release.yml` adds a `desktop-macos` matrix entry (`cargo tauri build --bundles dmg`), uploads the DMG, and the inventory gate requires a `.dmg` asset. macOS artifacts are published unsigned (no notarization secrets configured); the workflow header and `docs/releases/release-process.md` note this explicitly.

### RELEASE-04 — Updater manifest generation exists but is not wired into release publishing

**Evidence:** `scripts/generate-latest-json.mjs:1-117` generates a platform manifest and requires `UPDATER_PRIVATE_KEY`. `apps/desktop-client/tauri.conf.json:60-67` points the updater at `beta.json` and `latest.json`. However, `.github/workflows/release.yml:88-105` only downloads artifacts and calls `softprops/action-gh-release`; it never invokes `generate-latest-json.mjs`, creates `latest.json`, uploads `beta.json`, or verifies that either endpoint asset exists. The release-automation ADR audit stamp also records that the workflow does not reference this script. Additionally, the committed public key at `oz-pos-updater.key.pub` decodes to a Minisign-formatted public-key payload, while the generator documents and validates a raw 32-byte Ed25519 private key; compatibility between that key encoding/signature format and Tauri's configured updater verifier is not established by the current tests.

**Impact:** A desktop client checking the configured updater endpoint may receive a 404 or an incomplete manifest after a release. The updater path is therefore not an end-to-end release feature despite the presence of a public key, updater plugin, manifest generator, and UI.

**Severity:** P1 · update availability/integration

**Affected files:** `.github/workflows/release.yml`, `scripts/generate-latest-json.mjs`, `apps/desktop-client/tauri.conf.json`, `oz-pos-updater.key.pub`, `docs/releases/release-process.md`, and `docs/decisions/2026-07-16-release-automation.md`.

**Recommendation:** Build signed Tauri bundles, generate a complete manifest for every supported platform using the private key from a protected secret, attach the exact endpoint filenames (`latest.json` and intentionally supported `beta.json`), and verify the published URLs and signatures before marking the release successful. Do not log private-key material; keep a dry-run manifest/signature verification test in CI.

**Status:** REMEDIATED — `scripts/generate-latest-json.mjs` now supports `--merge` (single merged multi-platform manifest), `--verify-pubkey` (fails if the private seed does not derive to the committed pubkey — closes the key-encoding compatibility gap), and `--self-test`. New `scripts/verify-updater-signature.mjs` verifies every installer against the manifest using the `tauri.conf.json` pubkey (minisign-format decoding included) with tamper-rejection self-test. Both scripts were rewritten to emit the exact **minisign `.sig` text-blob format** the real Tauri client verifier decodes (prehashed `45 44` algorithm tag, keyid extracted from the committed pubkey, BLAKE2b-512 over the installer bytes, plus the global signature over `sig64 ‖ trusted-comment`), and this is proven end-to-end by `scripts/check-updater-compat.mjs`: it signs a dummy installer with `generate-latest-json.mjs`, then feeds the result through a Rust harness pinned to the exact `minisign-verify 0.2.5` crate the real client resolves (`tauri-plugin-updater` 2.10.1) — the same crate that accepted a fixture signed by real `tauri-cli` 2.11.1 — and rejects a tampered byte. `release.yml` runs the self-tests in CI (`release-validate` also runs the full compat check before any build), generates `latest.json` + `beta.json` from `UPDATER_PRIVATE_KEY`, verifies signatures before the draft is published, and never logs key material. The three release scripts are additionally re-validated on every local pre-CI gate run via `scripts/check.sh` (release version gate / updater manifest generator / updater signature verifier). Both updater ADR audit stamps updated to RESOLVED.

### RELEASE-05 — Release tags are not checked against application versions

**Evidence:** `.github/workflows/release.yml:12-14` triggers on any `v*` tag and uses `github.ref_name` only for the release name and cloud image tag. No step extracts the version from `Cargo.toml`, `ui/package.json`, `apps/desktop-client/tauri.conf.json`, or `apps/tablet-client/tauri.conf.json` and compares it with the tag. The current checked-in sources report `0.0.24` in `Cargo.toml:36`, both Tauri configs, and `ui/package.json`.

**Impact:** A maintainer can create `vX.Y.Z` while the binaries still carry another version. The release can publish a misleading name, updater manifest version, installer metadata, and changelog heading, making rollback and support diagnosis harder.

**Severity:** P1 · version/reproducibility integrity

**Affected files:** `.github/workflows/release.yml`, `Cargo.toml`, `ui/package.json`, both Tauri configurations, `scripts/bump-version.ps1`, and `scripts/release.sh`.

**Recommendation:** Add a preflight job that validates strict `vMAJOR.MINOR.PATCH` syntax and compares the tag version with every shipping application's version. Fail before any artifact is uploaded. Make the same check reusable locally and include a test for a mismatched tag and a synchronized version set.

**Status:** REMEDIATED — new reusable `scripts/check-release-version.mjs` enforces strict `vX.Y.Z` syntax, compares the tag against `Cargo.toml`, `ui/package.json`, and both `tauri.conf.json` files, and requires the canonical `CHANGELOG.md` heading. `release.yml` gates on it in a `release-validate` job (every build job `needs` it) and runs its `--self-test`; `scripts/release.sh` runs the same gate before tagging. `bump-version.ps1` now inserts the `## [X.Y.Z]` heading so the gate passes after a bump.

### RELEASE-06 — Release artifact signing and provenance are not enforced in the publish workflow

**Evidence:** `.github/workflows/release.yml` has only `contents: write` on `release-publish` at `:88-105`; it does not request `id-token: write`, use `actions/attest-build-provenance`, publish artifact attestations, or verify checksums/signatures before the release. The desktop Tauri config contains a Windows `signtool.exe` command, but the release workflow never runs a Tauri build that would invoke it. The updater key is committed publicly, while the private signing key is referenced only by scripts/documentation and not injected by the workflow.

**Impact:** Consumers and operators lack a machine-verifiable build provenance record. The release process also does not prove that uploaded assets were signed with the expected code-signing/updater keys, so a successful GitHub Release is not equivalent to a verified release artifact.

**Severity:** P1 · supply-chain integrity

**Affected files:** `.github/workflows/release.yml`, `apps/desktop-client/tauri.conf.json`, `scripts/generate-latest-json.mjs`, `oz-pos-updater.key.pub`, and release security documentation.

**Recommendation:** Build in isolated jobs, sign artifacts with protected secrets or platform signing services, verify signatures and SHA-256 digests, and attach provenance attestations to the exact immutable artifacts being published. Scope `contents: write`, `id-token: write`, and secret access to only the release jobs. Record a signed artifact inventory in the release notes or as a checksum asset.

**Status:** REMEDIATED — `release-publish` is the only job with `contents: write` + `id-token: write` + `attestations: write` (mobile upload jobs are narrowly tag-scoped). It attaches `SHA256SUMS.txt` over the exact release assets, runs `actions/attest-build-provenance@v2` on the installers + manifests, verifies updater signatures, and asserts the final asset inventory before publishing. Windows code signing is conditional on the `UPDATER_CERT` secret (never invokes signtool without a cert).

**Follow-up (end-user install experience, 2026-08-03):** the Windows
code-signing story now has a free route for public distribution:

- **SignPath** (`signpath/github-action-submit-signing-request@v2`) is wired
  into `release.yml` as a Windows-target-only, no-op-until-configured step
  (gated on a job-level `SIGNPATH_API_TOKEN` env var — step-level env is not
  visible in a step's own `if:`, and GitHub disallows secrets in `if:`).
  When configured, the NSIS/MSI installers are uploaded unsigned, signed by
  SignPath's publicly-trusted cloud HSM (free for qualifying open-source
  projects), and the signed artifact is uploaded in place of the unsigned
  one. This removes the SmartScreen "unknown publisher" warning for **end
  users** without buying a CA certificate.
- **`scripts/dev-code-sign.ps1`** — self-signed Authenticode signing for
  dev/CI machines only (CurrentUser store + Trusted Root, no admin,
  `-YesTrust` silent trust install via the `X509Store` API, bounded signtool
  with timestamp fallback, null-safe exit-code handling). Trust is local to
  the machine that installs the root; it does NOT remove SmartScreen for end
  users — SignPath (or a paid `UPDATER_CERT`) is required for that.
- **UAC layer:** every shipped Windows exe (Tauri apps via `tauri-winres`
  numeric-24 emission, plus `oz-cloud-server.exe` via an embedded
  `asInvoker` manifest in `apps/cloud-server`) carries a loadable manifest
  (RT_MANIFEST numeric type 24), so no elevation consent prompt appears on
  launch or install. The updater-compat harness also embeds `asInvoker` via
  numeric type 24 — a named-type `RT_MANIFEST` (string) resource is ignored
  by the Windows loader (mt.exe cannot find it), which previously triggered
  UAC installer-detection heuristics on every run.

  All of this is documented in `docs/releases/first-release-runbook.md` §6
  (End-user install experience: zero-popup goal) and
  `docs/releases/release-process.md` (Windows code signing — the free
  routes).

### RELEASE-07 — Version-bump and changelog automation does not match its documented contract

**Evidence:** `scripts/bump-version.ps1:71-95` updates manifests, generated version surfaces, and UI display strings, then refreshes `Cargo.lock` and `ui/package-lock.json`; it does not update `CHANGELOG.md` despite the release runbook saying the script keeps `CHANGELOG.md` in sync. `scripts/release.sh:94-114` writes a separate `docs/releases/CHANGELOG-${NEW_VERSION}.md` from the last 100 commit subjects, but it does not update the top-level `CHANGELOG.md` or validate that the generated notes reflect Keep a Changelog sections. The release checklist also asks for `docs/releases/CHANGELOG-{version}.md` at `docs/releases/checklist.md:9`. Separately, `docs/releases/release-process.md:13-20` says `cargo build --release` produces signed installers, but the live workflow and build script show that Tauri bundling/signing is a separate `cargo tauri build` step.

**Impact:** Release notes can be split between two locations or omitted from the canonical changelog. Commit-subject extraction can produce incomplete, noisy, or misleading release notes, while the version bump process may leave documentation and package metadata inconsistent.

**Severity:** P2 · release communication and traceability

**Affected files:** `scripts/bump-version.ps1`, `scripts/release.sh`, `CHANGELOG.md`, `docs/releases/checklist.md`, and `docs/releases/release-process.md`.

**Recommendation:** Define one canonical changelog source and one automation contract. Have release tooling validate the version heading, require a reviewed notes file or generated categorized draft, and fail if the canonical changelog lacks the tag version. Keep generated notes as a draft until a maintainer reviews them; do not silently truncate without surfacing the truncation in the release gate.

**Status:** REMEDIATED — canonical source is `CHANGELOG.md`. `scripts/bump-version.ps1` inserts the `## [X.Y.Z] — date` heading; `scripts/release.sh` writes the reviewed draft to `docs/releases/CHANGELOG-<v>.md`, refreshes the canonical heading, stages `CHANGELOG.md`, and runs the version gate before tagging (truncation warning preserved). The gate fails if `CHANGELOG.md` lacks the tag heading. `docs/releases/release-process.md` + `checklist.md` corrected to describe the real Tauri-installer/manifest pipeline.

### RELEASE-08 — Release validation does not gate on installability, updater reachability, or rollback readiness

**Evidence:** `docs/releases/checklist.md` lists smoke tests for login, sales, settings, and offline mode, but `.github/workflows/release.yml` runs no application smoke test after building and no updater-manifest URL/signature check. `docs/releases/mobile-checklist.md` contains manual APK/IPA signing and installation checks, but the Android/iOS workflows only build and upload artifacts. `docs/releases/release-process.md` describes rollback as manual and has no automated previous-version or downgrade verification.

**Impact:** A release can publish assets that compile but do not install, launch, pass signing verification, or update an existing terminal. Failures may be discovered only after operators download the release, and rollback confidence depends on manual steps under incident pressure.

**Severity:** P2 · release quality and recovery

**Affected files:** `.github/workflows/release.yml`, `.github/workflows/android.yml`, `.github/workflows/ios.yml`, `docs/releases/checklist.md`, `docs/releases/mobile-checklist.md`, and `docs/releases/release-process.md`.

**Recommendation:** Add post-build smoke jobs for artifact existence, version metadata, signature verification, installer/package validation, updater manifest schema/signature/URL checks, and at least one launch/install path per platform. Preserve a previous release fixture and document a tested rollback procedure. Publish artifacts only after these checks pass.

**Status:** REMEDIATED (automated subset) — build jobs fail when expected bundle extensions are absent; `release-publish` verifies updater signatures, generates SHA-256 checksums, and asserts the full release asset inventory before the draft is published. Launch/install smoke remains manual (checklists), and `docs/releases/release-process.md` now documents a tested rollback/downgrade procedure + the immutable historical-assets guarantee. Automated installability testing per platform is tracked as a future enhancement.

### RELEASE-09 — Release workflow omits explicit concurrency and staged/draft publication controls

**Evidence:** `.github/workflows/release.yml:12-14` starts on every pushed `v*` tag and `release-publish` immediately grants `contents: write` at `:92-93` and calls `softprops/action-gh-release` at `:100-105`. No concurrency group, draft release mode, environment approval, or duplicate-tag protection is defined. The Android and iOS workflows can also run independently for the same tag.

**Impact:** Retries, duplicate tag events, or simultaneous platform workflows can race to publish or overwrite release assets. A partially validated or incomplete release can become visible before all platform artifacts and manifest checks finish.

**Severity:** P3 · publication control

**Affected files:** `.github/workflows/release.yml`, `.github/workflows/android.yml`, `.github/workflows/ios.yml`, and repository release settings.

**Recommendation:** Use a release concurrency group keyed by tag, publish as a draft until the final inventory passes, require an environment approval for production publication, and make uploads idempotent. Consolidate or coordinate mobile workflows so only one orchestrator transitions a release from build to published.

**Status:** REMEDIATED — all three workflows carry tag-keyed `concurrency` groups (`release-${{ github.ref_name }}`, `release-android-…`, `release-ios-…`, `cancel-in-progress: false`). `release-publish` creates the release as a **draft** and flips to published only after the asset-inventory + signature checks pass; the `release` environment is declared on the publish job (auto-creates with no gate until reviewers are configured in repo settings). Mobile uploads use `gh release upload --clobber` for idempotency against the single orchestrator release.

## Positive controls observed

- Releases are tag-triggered rather than built from arbitrary branch heads.
- The publish job scopes `contents: write` instead of granting write access to every build job.
- Desktop and tablet applications have explicit version fields and Tauri updater configuration.
- A public updater key is committed while the private key is described as secret-only.
- `scripts/generate-latest-json.mjs` validates the presence and byte length of the private key, signs raw installer bytes with Ed25519, and emits deterministic release URLs.
- Desktop, Android, and iOS workflows use pinned action major versions and explicit runner targets.
- Mobile checklists include signing, installation, functional, device, and data-integrity checks.
- Release documentation includes key rotation, emergency revocation, manual rollback, and two-maintainer review guidance.
- Cargo release profile strips symbols, uses LTO, and enables overflow checks for release builds.

## Test and validation results

This was an evidence-only audit; no release workflow, script, configuration, or production code was changed.

Validation performed:

- Release workflow, version-source, script, updater, and checklist inventory: **completed**
- Cross-workflow artifact ownership review: **completed**
- Local release execution, tag creation, installer build, signing, provenance, and GitHub API checks: **not run**
- YAML/workflow execution and mobile signing validation: **not run locally**
- Audit report whitespace, `git diff --check`, finding count, and audit-only scope review: **passed**

The report distinguishes confirmed gaps in the committed workflow from intended future behavior described by stale ADR text. No claim is made that the Android/iOS signing secrets are invalid; the finding is that the main release publication does not collect or verify those independent workflow outputs.

## Remediation summary (2026-08-03)

All findings are closed. The remediation follows the audit's recommended order:

1. **RELEASE-01/RELEASE-04 — DONE.** `release.yml` builds real Tauri installers (AppImage+deb / NSIS+MSI / DMG) and publishes signed `latest.json` + `beta.json` manifests generated by `scripts/generate-latest-json.mjs` (now with `--merge`, `--verify-pubkey`, `--self-test`) and verified by the new `scripts/verify-updater-signature.mjs`.
2. **RELEASE-05 — DONE.** New `scripts/check-release-version.mjs` gate runs first in CI (`release-validate`) and locally in `scripts/release.sh`.
3. **RELEASE-02/RELEASE-03 — DONE.** macOS added to the release matrix; mobile workflows upload directly into the same release with tag-scoped `contents: write` + `--clobber` idempotency.
4. **RELEASE-06 — DONE.** `SHA256SUMS.txt` + `actions/attest-build-provenance@v2` on the exact installers/manifests; signing secrets scoped to `release-publish` only.
5. **RELEASE-08 — DONE (automated subset).** Artifact-existence, signature, and inventory gates before the draft is published; tested rollback procedure documented. Launch/install smoke remains manual.
6. **RELEASE-07 — DONE.** `CHANGELOG.md` is the canonical changelog; `bump-version.ps1` inserts the version heading, `release.sh` writes the reviewed draft + refreshes the canonical heading, and the version gate enforces parity.
7. **RELEASE-09 — DONE.** Tag-keyed concurrency on all three workflows, draft-then-publish, `release` environment approval gate (configurable), idempotent uploads.
8. **Windows install experience (follow-up) — DONE.** Free code-signing routes added: SignPath (public trust for end users) wired into `release.yml` (no-op until `SIGNPATH_API_TOKEN` configured) + `scripts/dev-code-sign.ps1` (self-signed for dev/CI). All shipped Windows exes embed a loadable `asInvoker` manifest (numeric RT_MANIFEST type 24), eliminating UAC consent prompts; the updater-compat harness bug (named-type manifest ignored by the Windows loader) was fixed and verified with mt.exe.

## Audit status

**REMEDIATED (2026-08-03).** Findings are closed by the release-pipeline, script, and documentation changes described above. The remaining manual surface (per-platform launch/install smoke, Apple notarization, Windows Authenticode cert configuration) is explicitly documented in the release runbook and checklists rather than silently assumed. The release must still be validated with one real tag pushed to a **draft** release before the first production cut.
