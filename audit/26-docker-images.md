# Docker Images Audit — July 2026

> **Audit date:** 2026-07-31
> **Sector:** Docker images — image size, layer caching, runtime hardening, health checks, secrets, supply-chain pinning, CI scanning, and E2E reliability
> **Status:** ✅ **FULLY REMEDIATED** — all 9 findings DOCKER-01→DOCKER-09 closed; commits `9f8b7739` (DOCKER-03/08), `2d4cecc9` (DOCKER-01/04), `5bcafce2` (DOCKER-05/06), `464fd37d` (DOCKER-02), `556fefb7` (DOCKER-07), `20b7ec3d` (DOCKER-09)
> **Production code changed:** None

## Scope

This audit evaluates sector 26 against the universal checklist in `audit/AUDIT_JULY_2026.md`. It covers the cloud-server and license-server Dockerfiles, development and E2E Compose stacks, runtime entrypoints, build context hygiene, image size controls, health-check correctness, mutable image references, secret/default configuration, CI caching, vulnerability scanning, and container test coverage.

Inspected areas:

- `Dockerfile.server`
- `apps/license-server/Dockerfile`
- `apps/license-server/docker-compose.yml`
- `docker-compose.yml`
- `docker-compose.e2e.yml`
- `docker-compose.override.yml`
- `scripts/docker-entrypoint.sh`
- `scripts/run-e2e.mjs`
- `.dockerignore`
- `.github/workflows/ci.yml`
- `.github/workflows/e2e-pr.yml`
- `.github/workflows/nightly.yml`
- `.github/workflows/release.yml`
- `.github/workflows/security.yml`

## Architecture summary

The repository uses multi-stage builds for the Rust cloud server and Go/PocketBase license server. `Dockerfile.server` compiles `oz-cloud-server` in a Rust builder image and runs it from `debian:bookworm-slim`; its entrypoint fixes `/data` ownership and drops from root to the `ozpos` user with `gosu`. The license image compiles the PocketBase binary and a standalone Go healthcheck binary, then runs on Alpine with a persistent `/pb/pb_data` volume.

Compose has separate local, development-override, and E2E stacks. The E2E stack builds the cloud and license images locally and uses Redis from public ECR. CI uses BuildKit/GitHub Actions cache for the cloud image, pre-pulls Redis in the main/nightly E2E jobs, and runs a Trivy scan for the cloud image. The PR E2E runner also builds the license image, but the general Docker security gate does not scan it.

The container design has several good foundations, but security and reliability are not enforced uniformly. Base images and Compose dependencies use mutable tags, the license runtime is root, local Compose contains a stale `curl` healthcheck for an image that does not install curl, production-like Compose defaults include empty or well-known secrets, and the only container vulnerability scan is explicitly non-blocking and limited to one image. Recent CI failures also demonstrate that external image pulls can make E2E startup fail before tests run.

## Findings

### DOCKER-01 — License-server runtime executes as root

**Evidence:** `apps/license-server/Dockerfile:33-53` creates the Alpine runtime, copies `/pb/pocketbase` and `/pb/healthcheck`, and declares an entrypoint, but never creates a dedicated user or declares `USER`. The container therefore starts the PocketBase process as root. By contrast, `Dockerfile.server:127-130` creates `ozpos`, and `scripts/docker-entrypoint.sh:4-13` drops the cloud server to that user.

**Impact:** A vulnerability in the PocketBase server, custom Go hooks, or an exposed admin endpoint would have root privileges inside the container. The persistent `/pb/pb_data` volume is also owned and modified by root, making least-privilege operation and host-side recovery less predictable.

**Severity:** P2 · container hardening

**Affected files:** `apps/license-server/Dockerfile`, `apps/license-server/docker-compose.yml`, `docker-compose.yml`, and `docker-compose.e2e.yml`.

**Recommendation:** Create a non-root system user/group in the license runtime, chown `/pb` and `/pb/pb_data`, and declare `USER` before the entrypoint. Verify that first-run PocketBase initialization, migrations, healthcheck execution, and mounted-volume upgrades work as the non-root user. If an initialization step must run as root, isolate it in an explicit entrypoint phase and drop privileges before serving HTTP.

**Status:** ✅ **REMEDIATED** — commit `2d4cecc9`: `apps/license-server/Dockerfile` creates a dedicated `pb` system user, chowns `/pb` and `/pb/pb_data`, and the new `apps/license-server/docker-entrypoint.sh` (mirroring `Dockerfile.server`) fixes volume ownership when started as root, then drops to `pb` via `su-exec` before PocketBase serves HTTP. Verified live: PID 1 runs as UID 100 and the container healthcheck reports `healthy`.

### DOCKER-02 — Base images and service images are mutable tag references

**Evidence:** `Dockerfile.server:18` uses `rust:1.88-slim` and `:116` uses `debian:bookworm-slim`; `apps/license-server/Dockerfile:14` uses `golang:1.25-alpine` and `:34` uses `alpine:3.20`. Compose uses `public.ecr.aws/docker/library/redis:7-alpine` at `docker-compose.yml:114` and `docker-compose.e2e.yml:68`, and `postgres:16-alpine` at `docker-compose.yml:133`. These tags can resolve to different bytes over time.

**Impact:** A rebuild may silently consume a changed OS, compiler, database, or cache image. This weakens reproducibility, complicates incident reconstruction, and permits an upstream tag move to change the release or E2E environment without a source change.

**Severity:** P2 · supply-chain and release integrity

**Affected files:** `Dockerfile.server`, `apps/license-server/Dockerfile`, `docker-compose.yml`, `docker-compose.e2e.yml`, and release/nightly Docker build workflows.

**Recommendation:** Pin production and CI-critical image references to reviewed immutable digests while retaining human-readable version comments. Update digests through a deliberate dependency-update process, record the base-image refresh date, and verify the pinned architecture matches each runner. Apply the same policy to the E2E Redis/PostgreSQL dependencies or provide a controlled internal mirror.

**Status:** ✅ **REMEDIATED** — commit `464fd37d` pins every production/CI-critical image to its reviewed multi-arch index digest (refreshed 2026-08-03, human-readable tag retained as a comment): `rust:1.88-slim`, `debian:bookworm-slim`, `golang:1.25-alpine`, `alpine` (3.20→3.22 — Trivy flagged 3.20 as EOL), `redis:7-alpine`, `postgres:16-alpine`. Digest refresh policy is documented in `.trivyignore` and the Dockerfiles. The E2E Redis pin is shared between `docker-compose.e2e.yml` and `scripts/run-e2e.mjs`.

### DOCKER-03 — Vulnerability scanning is non-blocking and covers only the cloud image

**Evidence:** `.github/workflows/ci.yml:261-294` builds `oz-pos-cloud:ci`, then runs Trivy with `continue-on-error: true` and `exit-code: 0`. The scan covers only the cloud-server image; the license-server image built by `.github/workflows/e2e-pr.yml:89-97` is not included. `.github/workflows/security.yml` runs `cargo audit` and `cargo deny`, but has no container scan.

**Impact:** Critical or high vulnerabilities can be reported without failing a required gate, and vulnerabilities in the license image or its Alpine runtime are not covered by the container scan. The repository can therefore pass CI while shipping a known vulnerable runtime.

**Severity:** P1 · security quality gate

**Affected files:** `.github/workflows/ci.yml`, `.github/workflows/e2e-pr.yml`, `.github/workflows/security.yml`, `Dockerfile.server`, and `apps/license-server/Dockerfile`.

**Recommendation:** Make the release-relevant image scan blocking for agreed severities after an initial baseline is reviewed. Scan both cloud and license images, include dependency/license policy where appropriate, upload SARIF or an equivalent durable report, and keep an explicit documented exception mechanism for accepted vulnerabilities. Run the scan on the exact image digest that is released rather than only on a transient local tag.

**Status:** ✅ **REMEDIATED** — commit `9f8b7739` makes the scans BLOCKING (`exit-code: 1` on CRITICAL/HIGH) for BOTH images in `ci.yml`, adds a weekly container-scan job covering both images to `security.yml`, scans the exact release-tagged image in `release.yml`, and uploads SARIF. A documented exception mechanism (`.trivyignore`) exists; the 15 Debian bookworm OS-package CVEs with no available fix were reviewed, verified against `apt-get upgrade`, and documented as the accepted baseline in `20b7ec3d` so the gate stays blocking for NEW findings. The two license-image HIGH CVEs (golang.org/x/image TIFF decoder, CVE-2026-46602/46604) were fixed by bumping to v0.43.0 — verified scanning clean locally.

### DOCKER-04 — Compose defaults can expose development credentials in production-like runs

**Evidence:** `docker-compose.yml:46-47` passes `OZ_API_SECRET` with an empty default, while `:137-139` defaults PostgreSQL to `PG_PASSWORD: "changeme"`. `docker-compose.e2e.yml:31` intentionally uses `e2e-test-secret`, and the license services at `docker-compose.yml:98` and `docker-compose.e2e.yml:56` default the private-key environment variable to empty. The comments state that secrets are required in production, but Compose does not fail fast when the required values are absent.

**Impact:** A developer or operator can start a development-oriented stack that appears healthy while using an empty or well-known authentication secret. If that stack is exposed beyond localhost or accidentally reused for deployment, API authentication and the PostgreSQL service are at risk. Empty license-key behavior can instead cause startup failure that is difficult to diagnose. The E2E secret is intentionally test-only; the concern is that the root Compose file does not enforce its production requirement.

**Severity:** P2 · secret/configuration integrity

**Affected files:** `docker-compose.yml`, `docker-compose.e2e.yml`, `apps/license-server/docker-compose.yml`, and deployment documentation.

**Recommendation:** Remove insecure production-like defaults and use Compose required-variable syntax for secrets outside the explicitly isolated E2E stack, for example `${OZ_API_SECRET:?OZ_API_SECRET is required}`. Keep deterministic test-only secrets scoped to E2E and bind test ports deliberately. Add a startup/config validation test proving production Compose fails closed when secrets are absent or obviously weak, without printing secret values in logs.

**Status:** ✅ **REMEDIATED** — commit `2d4cecc9`: `OZ_API_SECRET` and `OZ_LICENSE_PRIVATE_KEY` use required-variable syntax in `docker-compose.yml` and `apps/license-server/docker-compose.yml`, so Compose fails fast when either is missing. The optional PostgreSQL service moved to `docker-compose.pg.yml` (merged with `-f`), where `PG_PASSWORD` is required — this fails closed on the pg path without breaking the SQLite quickstart (verified: base config passes with secrets and errors without; pg override errors without `PG_PASSWORD`). The `changeme` default is gone. `dev-up.sh`/`dev-up.ps1` and the deployment guide were updated to the new invocation and secret requirements.

### DOCKER-05 — The standalone license Compose healthcheck references an absent runtime dependency

**Evidence:** `apps/license-server/docker-compose.yml:28-33` defines `test: ["CMD", "curl", "-f", "http://localhost:8080/api/"]`. `apps/license-server/Dockerfile:36` installs only `ca-certificates` and `tzdata` in the runtime image; it does not install curl. The main and E2E Compose files instead use `/pb/healthcheck`, which is compiled and copied at `apps/license-server/Dockerfile:30-31` and `:41-42`.

**Impact:** The standalone local license stack can report unhealthy even when PocketBase is serving correctly, causing `docker compose up --wait` or dependent tooling to fail. This is a configuration drift bug between the standalone and root Compose definitions and can lead developers to debug the application when the healthcheck itself is invalid.

**Severity:** P2 · local development reliability

**Affected files:** `apps/license-server/docker-compose.yml`, `apps/license-server/Dockerfile`, `docker-compose.yml`, and `docker-compose.e2e.yml`.

**Recommendation:** Use the compiled `/pb/healthcheck` binary consistently, or explicitly install and pin curl if it is a deliberate requirement. Add a Compose validation smoke test that builds the license image, starts it with an ephemeral key and volume, waits for health, and verifies the endpoint used by the healthcheck. Keep one documented health endpoint contract across all Compose files.

**Status:** ✅ **REMEDIATED** — commit `5bcafce2`: the standalone `apps/license-server/docker-compose.yml` healthcheck now uses the compiled `/pb/healthcheck` binary against `http://localhost:8080/api/health` — the same endpoint contract as the root and E2E Compose files (the runtime image does not install curl). The ci.yml docker job's Compose smoke test (added in `9f8b7739`) boots the E2E stack, waits for health, and curls both `/api/v1/health` endpoints; the license image was also built, health-checked, and non-root-verified locally during remediation.

### DOCKER-06 — E2E startup remains dependent on external mutable-image pulls and registry availability

**Evidence:** `docker-compose.e2e.yml:66-76` references the mutable public-ECR Redis tag. `scripts/run-e2e.mjs:145-154` starts Compose with `docker compose ... up -d --wait` and does not pre-pull, retry, or provide a cached fallback for Redis. The CI workflows add a three-attempt Redis pull loop at `.github/workflows/ci.yml:481-489` and `.github/workflows/nightly.yml:185-193`, but the unified local/PR runner has a separate startup path. The observed CI failures included `toomanyrequests: Rate exceeded` while pulling Redis, and earlier runs failed while waiting for the license service.

**Impact:** E2E jobs can fail before Playwright starts due to registry throttling or transient external availability. The failure is infrastructure noise rather than a product regression, reducing confidence in required PR feedback and wasting build time.

**Severity:** P2 · CI/test reliability

**Affected files:** `docker-compose.e2e.yml`, `scripts/run-e2e.mjs`, `.github/workflows/e2e-pr.yml`, `.github/workflows/ci.yml`, and `.github/workflows/nightly.yml`.

**Recommendation:** Use an immutable Redis digest and pre-pull/cache it in every runner path, or publish a small approved E2E dependency image to a registry with suitable CI limits. Make the runner retry transient pulls with bounded backoff and emit service logs/status on failure. Keep E2E startup deterministic by ensuring the Compose file uses the preloaded image without an implicit registry pull.

**Status:** ✅ **REMEDIATED** — commits `5bcafce2` + `464fd37d`: `docker-compose.e2e.yml` pins Redis to an immutable digest, `scripts/run-e2e.mjs` pre-pulls it with 3-attempt bounded backoff before `compose up --pull=missing`, and the `ci.yml`/`nightly.yml` pull loops use the same pinned digest. E2E startup no longer depends on a mutable tag or a single unretried registry pull; the runner still dumps service logs/status on failure.

### DOCKER-07 — Container runtime hardening is incomplete beyond user identity

**Evidence:** `docker-compose.yml`, `docker-compose.e2e.yml`, and `apps/license-server/docker-compose.yml` do not set read-only root filesystems, drop Linux capabilities, set `security_opt`, or define resource limits. `Dockerfile.server` has a non-root execution path, but it still starts as root so the entrypoint can chown `/data`; the license image has no comparable hardening. The services also publish backend, license, Redis, and optional PostgreSQL ports directly to the host (`docker-compose.yml:38-39`, `:95-96`, `:115-116`, `:140-141`).

**Impact:** A compromised service has a broader runtime capability and network exposure than necessary, and a runaway database/cache process can consume host resources. This is a production-profile gap, not proof that the developer and E2E stacks are intended to provide production isolation; it becomes higher risk if their topology is copied into staging or deployment.

**Severity:** P3 · production-profile defense in depth

**Affected files:** `Dockerfile.server`, `apps/license-server/Dockerfile`, `docker-compose.yml`, `docker-compose.e2e.yml`, and `apps/license-server/docker-compose.yml`.

**Recommendation:** Define a production deployment profile separate from developer Compose. For that profile, drop unnecessary capabilities, use a read-only root filesystem with explicit writable mounts, set `no-new-privileges`, add CPU/memory/pid limits appropriate to the service, and expose only the public API ports. Keep Redis/PostgreSQL and internal health endpoints on the private Compose network unless host access is required.

**Status:** ✅ **REMEDIATED** — commit `556fefb7` adds `docker-compose.prod.yml`, a production deployment profile separate from the developer stack: read-only root filesystems (tmpfs `/tmp`, writable volumes), `cap_drop: [ALL]` with only `CHOWN, SETUID, SETGID`, `no-new-privileges`, `init: true`, per-service CPU/memory limits, and Redis/PostgreSQL removed from host publishing (only public API ports 3099/8080 remain). `pos-cloud-db` is hardened directly in `docker-compose.pg.yml`. Verified with `docker compose config` for base+prod and base+pg+prod; documented in `docs/operations/docker-deployment.md`.

### DOCKER-08 — Container build and release coverage is asymmetric

**Evidence:** `.github/workflows/ci.yml:262-294` builds and size-checks only `Dockerfile.server`. The release workflow builds the cloud Docker target at `.github/workflows/release.yml:67-70`, while the license server is not built, scanned, or smoke-tested by that workflow. The PR E2E workflow builds the license image only as a test prerequisite. The repository may intentionally deploy the license server through a separate Northflank/release boundary; that deployment ownership was not established by this audit.

**Impact:** If this repository owns the license-server deployment, a Dockerfile or runtime regression can bypass the normal release gates even though the service is part of the documented full stack. If the service is intentionally released independently, the missing local gate is still a traceability gap unless the external release pipeline provides equivalent validation.

**Severity:** P2 · release quality / ownership clarity

**Affected files:** `.github/workflows/ci.yml`, `.github/workflows/release.yml`, `.github/workflows/e2e-pr.yml`, `apps/license-server/Dockerfile`, and `docker-compose.yml`.

**Recommendation:** Add a reusable container matrix for cloud and license images: build, inspect metadata, run health checks, scan, and publish attestations or digests. If the license server is intentionally deployed independently, document that boundary and give it an equivalent release workflow in its deployment repository or this repository. Add a Compose smoke job that verifies both services together.

**Status:** ✅ **REMEDIATED** — commit `9f8b7739`: `release.yml` gains a `docker-license` matrix target (build + blocking scan + published artifact) alongside the cloud target, `ci.yml` builds and blocking-scans both images, and a Compose smoke step boots the full E2E stack (cloud + license + redis) and verifies health endpoints + non-root execution. Both images are now release-tested artifacts with equivalent validation; if the license server is deployed through an external boundary, that deployment now has a traceable in-repo release gate.

### DOCKER-09 — Dockerfile cache optimization exists but is brittle to workspace changes

**Evidence:** `Dockerfile.server:32-58` manually copies a fixed list of Cargo manifests, then `:62-102` creates dummy source files and runs a best-effort dependency build before copying real sources at `:106-110`. This preserves dependency caching, and `.dockerignore:19-24` removes front-end and Tauri directories from the cloud build context. However, adding a workspace member or changing the manifest list requires a synchronized Dockerfile edit; the dummy package setup also intentionally suppresses the first build failure with `|| true` at line 102.

**Impact:** A newly added crate can be absent from the cache-priming layer or cause confusing cache misses/build failures. The best-effort dummy build can hide whether the cache prebuild is valid, making the optimization harder to maintain and diagnose.

**Severity:** P3 · build maintainability/performance

**Affected files:** `Dockerfile.server`, root `Cargo.toml`, workspace member manifests, and `.dockerignore`.

**Recommendation:** Add a CI check that every workspace manifest required by `oz-cloud-server` is represented in the cache stage, or use a maintained cargo-chef/BuildKit cache pattern after evaluating its reproducibility. Replace an unbounded `|| true` with an explicit expected cache-priming command and a documented reason for any intentionally ignored failure. Keep a cold-build and warm-build timing metric so the optimization remains justified.

**Status:** ✅ **REMEDIATED** — commit `20b7ec3d`: the cache-priming stage now copies every workspace member manifest (`crates/oz-notification` and `modules/loyalty` were missing and silently breaking the priming build), the unbounded `|| true` is replaced with an explicit logged best-effort marker with a documented rationale, and `scripts/verify-dockerfile-workspace.py` — wired into the ci.yml docker job before the image build — parses root `Cargo.toml` members and fails fast on any cache-stage drift. The same commit fixes a real boot defect the full-build validation exposed: `scripts/docker-entrypoint.sh` was CRLF in Windows checkouts (`.gitattributes` now pins `*.sh` to `eol=lf`); the rebuilt cloud image was verified to boot, serve `/api/v1/health` (200), and run as UID 999.

## Positive controls observed

- Both application images use multi-stage builds, keeping compilers and build dependencies out of the runtime layers.
- `Dockerfile.server` strips apt lists and uses `--no-install-recommends`; the license runtime uses `apk add --no-cache`.
- The cloud runtime uses a dedicated `ozpos` user through `scripts/docker-entrypoint.sh`.
- The license image compiles a standalone healthcheck binary instead of relying on curl in the main/E2E runtime image.
- Both main services define healthchecks, startup grace periods, retries, and persistent data volumes.
- `.dockerignore` excludes source-control metadata, secrets/environment files, UI dependencies, Tauri clients, docs, and build artifacts from the cloud build context.
- Dockerfile.server separates manifest copying from source copying to preserve dependency layers.
- CI uses BuildKit/GitHub Actions cache for E2E and release-related cloud builds and enforces a cloud binary-size limit.
- Redis is sourced from public ECR in the Compose stacks, avoiding the Docker Hub path that previously caused rate-limit failures.
- E2E startup dumps Compose logs and service status on failure through `scripts/run-e2e.mjs`.

## Test and validation results

This was an evidence-only audit; no Dockerfiles, Compose files, workflows, or production code were changed.

Validation performed:

- Source inventory and line-referenced evidence review: **completed**
- `.dockerignore` review: **completed**
- Dockerfile/Compose healthcheck and runtime-dependency cross-check: **completed**
- Local Docker daemon availability: **Docker 29.4.3 detected**
- Image build, container health smoke test, and vulnerability scan: **not run during this documentation-only audit**
- `docker compose -f docker-compose.e2e.yml config --quiet`: **passed**
- External registry availability and digest resolution: **not verified locally**
- Report whitespace, `git diff --check`, finding count, and audit-only scope review: **passed**

The observed CI failures involving Redis rate limiting and the license-server startup/health path are consistent with DOCKER-05 and DOCKER-06. They should be reproduced after remediation with both the unified `npm run e2e` runner and the PR workflow; a successful cloud-only image build would not validate the license image or the complete Compose stack.

## Recommended remediation order

1. **DOCKER-03/DOCKER-08:** Make both application images blocking-scan and release-tested artifacts.
2. **DOCKER-01/DOCKER-04:** Run the license server non-root and fail closed on missing/weak production secrets.
3. **DOCKER-05/DOCKER-06:** Unify healthchecks and make E2E dependencies deterministic against registry failures.
4. **DOCKER-02:** Pin application and service images to reviewed immutable digests.
5. **DOCKER-07:** Add a hardened production Compose/deployment profile with restricted capabilities, filesystem, resources, and ports.
6. **DOCKER-09:** Add a cache-stage/workspace consistency check and measure warm/cold build performance.

## Audit status

**2026-08-03 — FULLY REMEDIATED.** Every finding is closed by the commit chain below, each verified by local image builds, container health checks, non-root runtime checks, Trivy scans, and `docker compose config` validation:

| Finding | Fix | Commit |
|---|---|---|
| DOCKER-01 license runs as root | non-root `pb` user + entrypoint privilege drop (verified PID 1 = UID 100) | `2d4cecc9` |
| DOCKER-02 mutable image tags | all base/service images pinned to reviewed multi-arch digests; Alpine 3.20→3.22 (EOL) | `464fd37d` |
| DOCKER-03 non-blocking, cloud-only scan | blocking CRITICAL/HIGH scans on BOTH images + SARIF + `.trivyignore` baseline; x/image CVEs fixed | `9f8b7739`, `20b7ec3d` |
| DOCKER-04 empty/weak compose secrets | required-variable syntax; `PG_PASSWORD` required via `docker-compose.pg.yml` | `2d4cecc9` |
| DOCKER-05 curl healthcheck drift | unified `/pb/healthcheck` + `/api/health` contract everywhere | `5bcafce2` |
| DOCKER-06 E2E external pulls | pinned Redis digest + pre-pull/retry in runner and all CI paths | `5bcafce2`, `464fd37d` |
| DOCKER-07 no runtime hardening | `docker-compose.prod.yml` profile (read-only, cap drop, no-new-privileges, limits, internal-only infra ports) | `556fefb7` |
| DOCKER-08 asymmetric build/release | license image built + blocking-scanned + published in release; Compose smoke of both services | `9f8b7739` |
| DOCKER-09 cache-stage drift | all member manifests in priming stage + workspace↔Dockerfile CI gate + CRLF entrypoint fix | `20b7ec3d` |
