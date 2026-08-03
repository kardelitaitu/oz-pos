# First-Release Runbook — OZ-POS

> **Purpose:** take a repository through its **first** real release, end to
> end: the secrets that must exist, the `release` environment approval gate,
> cutting the tag, and inspecting the draft before it goes public. Follow in
> order.
>
> **Companion docs:** `release-process.md` (operational runbook),
> `checklist.md` (per-release checklist), `mobile-checklist.md` (APK/IPA).

## 0. The pipeline at a glance

Pushing a `v*` tag triggers three workflows, all keyed to the tag by
concurrency (`release-<tag>`, `release-android-<tag>`, `release-ios-<tag>` —
no cancel-in-progress):

| Stage | Workflow · job | What it does |
|---|---|---|
| Gate | `release.yml` · `release-validate` | Tag ↔ version parity (`check-release-version.mjs`) + updater client compat check (`check-updater-compat.mjs`) |
| Build | `release.yml` · `release-build` | Matrix: Linux AppImage+deb, Windows NSIS+MSI, macOS DMG, Docker cloud + license images; per-artifact existence gate; blocking Trivy scans |
| Publish | `release.yml` · `release-publish` | Toolchain self-tests → signed `latest.json` + `beta.json` → signature verification → `SHA256SUMS.txt` → **draft release** → inventory gate → **auto-publish** → provenance attestation |
| Mobile | `android.yml` / `ios.yml` | Signed APK/AAB + IPA, uploaded into the **same** release via `gh release upload --clobber` (poll up to 60 min for the release to exist) |

`release-publish` is gated by `environment: release` (see §2).

---

## 1. Secrets that must exist

Set everything in **Settings → Secrets and variables → Actions** of the
repository, **before** cutting the tag — the workflows fail fast when a
required secret is missing.

### Desktop (`release.yml`)

| Secret | Required | Format | Used by |
|---|---|---|---|
| `UPDATER_PRIVATE_KEY` | ✅ | Ed25519 **seed** — 64 hex chars, or base64 of the 32-byte seed | `release-publish` manifest signing (`generate-latest-json.mjs`) |
| `UPDATER_CERT` | optional | base64-encoded PFX (Authenticode) | Windows `signtool.exe` installer signing |
| `UPDATER_CERT_PASSWORD` | optional | PFX password | documented in the workflow header; **not currently consumed** by the import step |
| `SIGNPATH_API_TOKEN` | optional* | SignPath API token (submitter) | Windows installer code-signing via **SignPath** (free public trust for OSS) |
| `SIGNPATH_ORGANIZATION_ID` / `SIGNPATH_PROJECT_SLUG` / `SIGNPATH_SIGNING_POLICY_SLUG` | optional* | repo **variables** (not secrets) | identify the SignPath org / project / signing policy |

\* Both `UPDATER_CERT` and the SignPath route are **optional but recommended** —
see § End-user install experience below for what each one does (or doesn't) fix.
When neither is set, Windows installers ship **unsigned** (SmartScreen
"unknown publisher" appears on end-user machines).

**`UPDATER_PRIVATE_KEY` is the one secret a release cannot go without.**
It must be the raw Ed25519 **seed** whose public half equals the pubkey
embedded in `apps/desktop-client/tauri.conf.json::plugins.updater.pubkey`.
The workflow runs `--verify-pubkey` and aborts with `PUBKEY MISMATCH` if they
disagree. Before the first release, confirm you hold the seed for the
**current committed pubkey**; if you don't, follow the rotation procedure in
`release-process.md` (§ Updater pubkey rotation) to install a keypair you
control and commit the new pubkey **first**.

Local preflight of the key before tagging (fails fast on mismatch):

```bash
SEED="<64-hex or base64 seed>"
PUBKEY=$(node -e "const c=require('./apps/desktop-client/tauri.conf.json');process.stdout.write(c.plugins.updater.pubkey)")
echo "dummy" > /tmp/dummy-installer.bin
UPDATER_PRIVATE_KEY="$SEED" node scripts/generate-latest-json.mjs 0.0.24 preflight linux-x86_64 /tmp/dummy-installer.bin --verify-pubkey "$PUBKEY"
```

**`UPDATER_CERT` (optional):** base64-encoded Windows code-signing PFX.
Without it the workflow logs `::warning:: … installers will be UNSIGNED` and
uses a no-op `signCommand`, so the release still succeeds with **unsigned**
Windows installers. With it, `Import-PfxCertificate` imports the PFX and
Tauri signs via `signtool.exe`. Note the current import step does not pass
`UPDATER_CERT_PASSWORD` — a password-protected PFX may not import cleanly;
export the PFX without a password, or verify the step succeeds in a
`workflow_dispatch` dry run first.

### Android (`android.yml`)

| Secret | Required | Format |
|---|---|---|
| `ANDROID_KEYSTORE_BASE64` | ✅ | base64-encoded `.keystore` file |
| `KEYSTORE_PASSWORD` | ✅ | keystore master password |
| `KEY_PASSWORD` | ✅ (falls back to `KEYSTORE_PASSWORD`) | key password |
| `KEY_ALIAS` | ✅ | key alias in the keystore |

The workflow decodes the keystore to `apps/tablet-client/oz-pos.keystore` and
passes the passwords via `TAURI_ANDROID_KEYSTORE_PASSWORD` /
`TAURI_ANDROID_KEY_PASSWORD` / `TAURI_ANDROID_KEY_ALIAS` to
`cargo tauri android build`. Without the keystore the APK/AAB build is not
signed.

### iOS (`ios.yml`)

| Secret | Required | Format |
|---|---|---|
| `APPLE_TEAM_ID` | ✅ | Apple Developer team ID |
| `APPLE_BUNDLE_ID` | ✅ | bundle id, e.g. `com.ozpos.tablet` |
| `APPLE_CERT_BASE64` | ✅ | base64-encoded distribution certificate `.p12` |
| `APPLE_CERT_PASSWORD` | ✅ | p12 password |
| `APPLE_PROV_PROFILE_BASE64` | ✅ | base64-encoded provisioning profile |
| `KEYCHAIN_PASSWORD` | ✅ | temporary build-keychain password (any value) |

The workflow creates a throwaway keychain, imports the cert + profile, sets
`DEVELOPMENT_TEAM` / `PRODUCT_BUNDLE_IDENTIFIER` with `CODE_SIGN_STYLE=Manual`,
and builds a signed IPA. Apple signing is a hard requirement for a device
installable IPA — missing secrets fail the job.

---

## 2. Configure the `release` environment approval

`release-publish` declares `environment: release`. GitHub **auto-creates**
environments on first reference, so until you configure it the gate does
**nothing** — the publish job runs straight through. To make publishing
require human approval:

1. GitHub repo → **Settings → Environments** → **New environment**.
2. Name it exactly **`release`**.
3. **Required reviewers** → add at least one person or team.
4. (Optional) **Deployment branches** → `main`, so only tags reachable from
   main can deploy.

> **What the gate does and doesn't do:** a job-level environment gate pauses
> the whole `release-publish` job **before it executes** — before the draft
> exists, before any key material is used. It is a *pre-publish review
> point*, not a draft-inspection window. To inspect the draft itself, see §4.

---

## 3. Cut the tag

Version is locked at the current release (`0.0.24`) — only bump it when you
mean to cut.

1. **Bump versions** (Windows):
   ```powershell
   powershell -File scripts/bump-version.ps1 0.0.25
   ```
   Rewrites `Cargo.toml`, both `tauri.conf.json` files, `ui/package.json`,
   and inserts the canonical `## [0.0.25]` heading into `CHANGELOG.md`.
2. **Generate changelog + create tag:**
   ```bash
   bash scripts/release.sh --dry-run 0.0.25   # preview only
   bash scripts/release.sh 0.0.25             # writes docs/releases/CHANGELOG-0.0.25.md, refreshes heading, runs the version gate, creates the tag
   ```
   `release.sh` runs the AUDIT-28 version gate before tagging and creates the
   local annotated tag **without pushing**.
3. **Verify parity locally** (optional but cheap):
   ```bash
   node scripts/check-release-version.mjs v0.0.25
   ```
   Compares the tag against `Cargo.toml`, `ui/package.json`, both
   `tauri.conf.json` files, and the `CHANGELOG.md` heading.
4. **Push the tag** — this triggers the whole pipeline:
   ```bash
   git push origin main
   git push origin v0.0.25
   ```

---

## 4. Cut the tag as a draft and inspect it

**Current behavior:** `release-publish` creates the release as a **draft**
(`softprops/action-gh-release`, `draft: true`) and then **automatically**
flips it to published (`gh release edit "$TAG" --draft=false`) in the same
job, after its inventory gate passes. There is **no built-in pause** between
draft creation and publish. For the first release, use one of these:

### Option A — hold the draft (recommended for the first release)

1. Temporarily comment out the publish flip in `.github/workflows/release.yml`
   inside the **"Verify release assets and publish"** step:
   ```yaml
   # echo "Publishing draft release $TAG"
   # gh release edit "$TAG" --draft=false
   # echo "Release $TAG published"
   ```
   Keep the inventory `grep` checks above it — they still fail the job if an
   asset is missing.
2. Cut the tag (§3). The job builds everything, signs the manifest, creates
   the **draft**, and stops.
3. Inspect the draft:
   ```bash
   gh release view v0.0.25
   gh release download v0.0.25 -D /tmp/relcheck
   cd /tmp/relcheck
   sha256sum -c SHA256SUMS.txt
   # verify every platform's installer against the signed manifest
   # (run from the repo root so the scripts/ helper paths resolve):
   VERIFY="$PWD/scripts/verify-updater-signature.mjs"  # run from repo root
   cd /tmp/relcheck
   node "$VERIFY" latest.json linux-x86_64   OZ-POS_*.AppImage
   node "$VERIFY" latest.json windows-x86_64 OZ-POS_*.exe
   node "$VERIFY" latest.json darwin-aarch64 OZ-POS_*.dmg
   ```
   Confirm the full asset set: `latest.json`, `beta.json`, `SHA256SUMS.txt`,
   AppImage/deb, exe/msi, dmg — and, once the mobile jobs finish, the
   APK/AAB/IPA (they attach to the draft via `gh release upload --clobber`).
4. Publish when satisfied, then restore the workflow line:
   ```bash
   gh release edit v0.0.25 --draft=false
   ```

### Option B — environment gate only

Configure §2 reviewers. The publish job pauses **before** it runs; review the
Actions run (artifacts tab, Trivy SARIFs, test logs), then **Approve**. The
job then creates the draft and publishes back-to-back. You review before any
signing or publication, but you never see the draft itself.

---

## 5. Post-publish verification

- [ ] `gh release view v0.0.25` shows the release as **Published** with every
      expected asset attached.
- [ ] Desktop updater endpoint resolves: `latest.json` (and `beta.json`) are
      attached and their signatures verify against the committed pubkey
      (§4 commands).
- [ ] Mobile APK/AAB + IPA attached to the same release (android.yml / ios.yml
      completed without `::error::`).
- [ ] Trivy scans green; SHA-256 checksums verified; provenance attestations
      present (`attest-build-provenance` runs on every asset class).
- [ ] One smoke install per platform on a test terminal (see `checklist.md`).
- [ ] Rollback path confirmed (previous version's installers + manifest are
      still attached to their own tags; `release-process.md` § Rollback).

> First release should also be the one that proves the **draft** mechanics
> (Option A). After it, the normal flow is Option B or plain auto-publish —
> the environment reviewers remain the standing human gate.

---

## 6. End-user install experience: zero-popup goal

Two independent mechanisms decide what an end user sees when they install
or first run the app:

| Layer | Mechanism | How it's fixed | Status |
|---|---|---|---|
| UAC elevation prompt | Missing/ignored `requestedExecutionLevel` manifest → Windows installer-detection heuristics | Embed a manifest with `<requestedExecutionLevel level="asInvoker"/>` | ✅ **None** — shipped Tauri apps (numeric-24 via `tauri-winres`), `oz-cloud-server.exe` (embed-resource build.rs), the `oz` CLI (`crates/oz-cli`, same pattern), and the license-server Windows build (Go `.syso` via go-winres) all carry a loadable `asInvoker` manifest. |
| SmartScreen / "Publisher: Unknown" | Unsigned Authenticode | Sign with a **publicly-trusted** certificate | ⚠️ **None only when** `UPDATER_CERT` **or** SignPath is configured |

### The free routes (no paid CA)

1. **Self-signed cert — `scripts/dev-code-sign.ps1` (dev/CI only).**
   Generates a `CN=OZ-POS Development` code-signing cert in the **CurrentUser**
   store (no admin), installs it into the user's Trusted Root (`-YesTrust`
   does this silently via the `X509Store` API; without it you'll get Windows'
   standard "install this root certificate?" Security Warning once), signs
   exes with `signtool`, and verifies. This kills "Publisher: Unknown" **on
   that one machine** — end users elsewhere still see the warning unless they
   install the same root. Use for local dev and CI-internal builds.
2. **SignPath — `release.yml` (public free route).** Free code signing for
   qualifying open-source projects, cloud-HSM key, publicly trusted chain.
   The workflow now uploads the Windows NSIS/MSI installers unsigned, submits
   them to SignPath (`signpath/github-action-submit-signing-request@v2`),
   and uploads the signed result — a **no-op until** `SIGNPATH_API_TOKEN`
   (secret) plus the three `SIGNPATH_*` variables are configured. This is the
   only free route that removes SmartScreen for **end users**.
3. **Paid `UPDATER_CERT` (OV/EV).** Existing optional path; unchanged.

### What still pops for end users if nothing is configured

- **SmartScreen "Windows protected your PC" / "unknown publisher":** appears
  for **unsigned** installers (the default until `UPDATER_CERT` or SignPath
  is configured). Configure SignPath to eliminate it for free on the public
  release.
- **UAC consent:** does **not** appear — the `asInvoker` manifests handle it.

### One-time setup for the SignPath route

1. Apply for free open-source signing at signpath.org (org + project +
   signing policy must be approved; GitHub App or API token required).
2. Configure the SignPath **Artifact Configuration** as `<zip-file>` root —
   `actions/upload-artifact@v4` uploads ZIP archives by default, and SignPath
   needs the artifact configuration to unwrap them (SignPath docs note this
   explicitly).
3. Set `SIGNPATH_API_TOKEN` as a **secret**; the three `SIGNPATH_*` values as
   **variables**.
4. Verify the workflow's "Sign installers with SignPath" step runs on the
   next release (it's Windows-target only and skipped when the token is
   unset). The step is `continue-on-error: true`, so a SignPath outage
   degrades to the unsigned fallback upload instead of failing the release.
