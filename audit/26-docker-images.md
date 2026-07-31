# Docker Images Audit — July 2026

> **Audit date:** 2026-07-31
> **Sector:** Docker images — image size, layer caching, runtime hardening, health checks, secrets, supply-chain pinning, CI scanning, and E2E reliability
> **Status:** AUDITED · container security and CI reliability findings require remediation
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

**Status:** Open

### DOCKER-02 — Base images and service images are mutable tag references

**Evidence:** `Dockerfile.server:18` uses `rust:1.88-slim` and `:116` uses `debian:bookworm-slim`; `apps/license-server/Dockerfile:14` uses `golang:1.25-alpine` and `:34` uses `alpine:3.20`. Compose uses `public.ecr.aws/docker/library/redis:7-alpine` at `docker-compose.yml:114` and `docker-compose.e2e.yml:68`, and `postgres:16-alpine` at `docker-compose.yml:133`. These tags can resolve to different bytes over time.

**Impact:** A rebuild may silently consume a changed OS, compiler, database, or cache image. This weakens reproducibility, complicates incident reconstruction, and permits an upstream tag move to change the release or E2E environment without a source change.

**Severity:** P2 · supply-chain and release integrity

**Affected files:** `Dockerfile.server`, `apps/license-server/Dockerfile`, `docker-compose.yml`, `docker-compose.e2e.yml`, and release/nightly Docker build workflows.

**Recommendation:** Pin production and CI-critical image references to reviewed immutable digests while retaining human-readable version comments. Update digests through a deliberate dependency-update process, record the base-image refresh date, and verify the pinned architecture matches each runner. Apply the same policy to the E2E Redis/PostgreSQL dependencies or provide a controlled internal mirror.

**Status:** Open

### DOCKER-03 — Vulnerability scanning is non-blocking and covers only the cloud image

**Evidence:** `.github/workflows/ci.yml:261-294` builds `oz-pos-cloud:ci`, then runs Trivy with `continue-on-error: true` and `exit-code: 0`. The scan covers only the cloud-server image; the license-server image built by `.github/workflows/e2e-pr.yml:89-97` is not included. `.github/workflows/security.yml` runs `cargo audit` and `cargo deny`, but has no container scan.

**Impact:** Critical or high vulnerabilities can be reported without failing a required gate, and vulnerabilities in the license image or its Alpine runtime are not covered by the container scan. The repository can therefore pass CI while shipping a known vulnerable runtime.

**Severity:** P1 · security quality gate

**Affected files:** `.github/workflows/ci.yml`, `.github/workflows/e2e-pr.yml`, `.github/workflows/security.yml`, `Dockerfile.server`, and `apps/license-server/Dockerfile`.

**Recommendation:** Make the release-relevant image scan blocking for agreed severities after an initial baseline is reviewed. Scan both cloud and license images, include dependency/license policy where appropriate, upload SARIF or an equivalent durable report, and keep an explicit documented exception mechanism for accepted vulnerabilities. Run the scan on the exact image digest that is released rather than only on a transient local tag.

**Status:** Open

### DOCKER-04 — Compose defaults can expose development credentials in production-like runs

**Evidence:** `docker-compose.yml:46-47` passes `OZ_API_SECRET` with an empty default, while `:137-139` defaults PostgreSQL to `PG_PASSWORD: "changeme"`. `docker-compose.e2e.yml:31` intentionally uses `e2e-test-secret`, and the license services at `docker-compose.yml:98` and `docker-compose.e2e.yml:56` default the private-key environment variable to empty. The comments state that secrets are required in production, but Compose does not fail fast when the required values are absent.

**Impact:** A developer or operator can start a development-oriented stack that appears healthy while using an empty or well-known authentication secret. If that stack is exposed beyond localhost or accidentally reused for deployment, API authentication and the PostgreSQL service are at risk. Empty license-key behavior can instead cause startup failure that is difficult to diagnose. The E2E secret is intentionally test-only; the concern is that the root Compose file does not enforce its production requirement.

**Severity:** P2 · secret/configuration integrity

**Affected files:** `docker-compose.yml`, `docker-compose.e2e.yml`, `apps/license-server/docker-compose.yml`, and deployment documentation.

**Recommendation:** Remove insecure production-like defaults and use Compose required-variable syntax for secrets outside the explicitly isolated E2E stack, for example `${OZ_API_SECRET:?OZ_API_SECRET is required}`. Keep deterministic test-only secrets scoped to E2E and bind test ports deliberately. Add a startup/config validation test proving production Compose fails closed when secrets are absent or obviously weak, without printing secret values in logs.

**Status:** Open

### DOCKER-05 — The standalone license Compose healthcheck references an absent runtime dependency

**Evidence:** `apps/license-server/docker-compose.yml:28-33` defines `test: ["CMD", "curl", "-f", "http://localhost:8080/api/"]`. `apps/license-server/Dockerfile:36` installs only `ca-certificates` and `tzdata` in the runtime image; it does not install curl. The main and E2E Compose files instead use `/pb/healthcheck`, which is compiled and copied at `apps/license-server/Dockerfile:30-31` and `:41-42`.

**Impact:** The standalone local license stack can report unhealthy even when PocketBase is serving correctly, causing `docker compose up --wait` or dependent tooling to fail. This is a configuration drift bug between the standalone and root Compose definitions and can lead developers to debug the application when the healthcheck itself is invalid.

**Severity:** P2 · local development reliability

**Affected files:** `apps/license-server/docker-compose.yml`, `apps/license-server/Dockerfile`, `docker-compose.yml`, and `docker-compose.e2e.yml`.

**Recommendation:** Use the compiled `/pb/healthcheck` binary consistently, or explicitly install and pin curl if it is a deliberate requirement. Add a Compose validation smoke test that builds the license image, starts it with an ephemeral key and volume, waits for health, and verifies the endpoint used by the healthcheck. Keep one documented health endpoint contract across all Compose files.

**Status:** Open

### DOCKER-06 — E2E startup remains dependent on external mutable-image pulls and registry availability

**Evidence:** `docker-compose.e2e.yml:66-76` references the mutable public-ECR Redis tag. `scripts/run-e2e.mjs:145-154` starts Compose with `docker compose ... up -d --wait` and does not pre-pull, retry, or provide a cached fallback for Redis. The CI workflows add a three-attempt Redis pull loop at `.github/workflows/ci.yml:481-489` and `.github/workflows/nightly.yml:185-193`, but the unified local/PR runner has a separate startup path. The observed CI failures included `toomanyrequests: Rate exceeded` while pulling Redis, and earlier runs failed while waiting for the license service.

**Impact:** E2E jobs can fail before Playwright starts due to registry throttling or transient external availability. The failure is infrastructure noise rather than a product regression, reducing confidence in required PR feedback and wasting build time.

**Severity:** P2 · CI/test reliability

**Affected files:** `docker-compose.e2e.yml`, `scripts/run-e2e.mjs`, `.github/workflows/e2e-pr.yml`, `.github/workflows/ci.yml`, and `.github/workflows/nightly.yml`.

**Recommendation:** Use an immutable Redis digest and pre-pull/cache it in every runner path, or publish a small approved E2E dependency image to a registry with suitable CI limits. Make the runner retry transient pulls with bounded backoff and emit service logs/status on failure. Keep E2E startup deterministic by ensuring the Compose file uses the preloaded image without an implicit registry pull.

**Status:** Open

### DOCKER-07 — Container runtime hardening is incomplete beyond user identity

**Evidence:** `docker-compose.yml`, `docker-compose.e2e.yml`, and `apps/license-server/docker-compose.yml` do not set read-only root filesystems, drop Linux capabilities, set `security_opt`, or define resource limits. `Dockerfile.server` has a non-root execution path, but it still starts as root so the entrypoint can chown `/data`; the license image has no comparable hardening. The services also publish backend, license, Redis, and optional PostgreSQL ports directly to the host (`docker-compose.yml:38-39`, `:95-96`, `:115-116`, `:140-141`).

**Impact:** A compromised service has a broader runtime capability and network exposure than necessary, and a runaway database/cache process can consume host resources. This is a production-profile gap, not proof that the developer and E2E stacks are intended to provide production isolation; it becomes higher risk if their topology is copied into staging or deployment.

**Severity:** P3 · production-profile defense in depth

**Affected files:** `Dockerfile.server`, `apps/license-server/Dockerfile`, `docker-compose.yml`, `docker-compose.e2e.yml`, and `apps/license-server/docker-compose.yml`.

**Recommendation:** Define a production deployment profile separate from developer Compose. For that profile, drop unnecessary capabilities, use a read-only root filesystem with explicit writable mounts, set `no-new-privileges`, add CPU/memory/pid limits appropriate to the service, and expose only the public API ports. Keep Redis/PostgreSQL and internal health endpoints on the private Compose network unless host access is required.

**Status:** Open

### DOCKER-08 — Container build and release coverage is asymmetric

**Evidence:** `.github/workflows/ci.yml:262-294` builds and size-checks only `Dockerfile.server`. The release workflow builds the cloud Docker target at `.github/workflows/release.yml:67-70`, while the license server is not built, scanned, or smoke-tested by that workflow. The PR E2E workflow builds the license image only as a test prerequisite. The repository may intentionally deploy the license server through a separate Northflank/release boundary; that deployment ownership was not established by this audit.

**Impact:** If this repository owns the license-server deployment, a Dockerfile or runtime regression can bypass the normal release gates even though the service is part of the documented full stack. If the service is intentionally released independently, the missing local gate is still a traceability gap unless the external release pipeline provides equivalent validation.

**Severity:** P2 · release quality / ownership clarity

**Affected files:** `.github/workflows/ci.yml`, `.github/workflows/release.yml`, `.github/workflows/e2e-pr.yml`, `apps/license-server/Dockerfile`, and `docker-compose.yml`.

**Recommendation:** Add a reusable container matrix for cloud and license images: build, inspect metadata, run health checks, scan, and publish attestations or digests. If the license server is intentionally deployed independently, document that boundary and give it an equivalent release workflow in its deployment repository or this repository. Add a Compose smoke job that verifies both services together.

**Status:** Open

### DOCKER-09 — Dockerfile cache optimization exists but is brittle to workspace changes

**Evidence:** `Dockerfile.server:32-58` manually copies a fixed list of Cargo manifests, then `:62-102` creates dummy source files and runs a best-effort dependency build before copying real sources at `:106-110`. This preserves dependency caching, and `.dockerignore:19-24` removes front-end and Tauri directories from the cloud build context. However, adding a workspace member or changing the manifest list requires a synchronized Dockerfile edit; the dummy package setup also intentionally suppresses the first build failure with `|| true` at line 102.

**Impact:** A newly added crate can be absent from the cache-priming layer or cause confusing cache misses/build failures. The best-effort dummy build can hide whether the cache prebuild is valid, making the optimization harder to maintain and diagnose.

**Severity:** P3 · build maintainability/performance

**Affected files:** `Dockerfile.server`, root `Cargo.toml`, workspace member manifests, and `.dockerignore`.

**Recommendation:** Add a CI check that every workspace manifest required by `oz-cloud-server` is represented in the cache stage, or use a maintained cargo-chef/BuildKit cache pattern after evaluating its reproducibility. Replace an unbounded `|| true` with an explicit expected cache-priming command and a documented reason for any intentionally ignored failure. Keep a cold-build and warm-build timing metric so the optimization remains justified.

**Status:** Open

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

This is an evidence-based audit report only. No production code was changed. Findings remain **Open** until remediation commits link each item to tests, container scans, health checks, and release validation results.
