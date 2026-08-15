# Dockerfile — OZ-POS License Server (PocketBase + Custom Go Hooks)
# ADR #9 Phase 3: Northflank deployment artifact
#
# Build:  docker build -t oz-pos/license-server apps/license-server
# Run:    docker run -p 8080:8080 -v pb_data:/pb/pb_data \
#           -e OZ_LICENSE_PRIVATE_KEY="$(cat crates/oz-core/oz-license-private.pem)" \
#           oz-pos/license-server
#
# Multi-stage build: compile the custom Go binary, then copy into a
# minimal runtime image. CGO is required by PocketBase's SQLite driver.

# ── Stage 1: Build ────────────────────────────────────────────────────
# Uses golang:1.25-alpine to satisfy fatih/color@v1.19.0 (requires go >= 1.25)
# DOCKER-02: base images pinned to immutable digests (refreshed 2026-08-03).
FROM golang:1.25-alpine@sha256:56961d79ea8129efddcc0b8643fd8a5416b4e6228cfd477e3fd61deb2672c587 AS builder

RUN apk add --no-cache gcc musl-dev

WORKDIR /src

# Cache dependencies first.
COPY go.mod go.sum ./
RUN go mod download

# Copy source files (explicit filenames to avoid test files).
COPY main.go activate.go renew.go status.go helpers.go expiry.go ratelimit.go health.go api_key.go contact.go web_otp.go paddle_webhook.go ./
COPY healthcheck.go ./
COPY pb_schema.json ./

RUN CGO_ENABLED=1 go build -o /pb/pocketbase -ldflags="-s -w" .
# P8-4: Build standalone healthcheck binary (no curl dependency in runtime).
RUN CGO_ENABLED=0 go build -o /pb/healthcheck -ldflags="-s -w" healthcheck.go

# ── Stage 2: Runtime ──────────────────────────────────────────────────
# alpine:3.22 tag — multi-arch index digest (refreshed 2026-08-03). Bumped
# from 3.20 (EOL per Trivy, no security updates) to a supported release.
FROM alpine:3.22@sha256:14358309a308569c32bdc37e2e0e9694be33a9d99e68afb0f5ff33cc1f695dce

# DOCKER-01: non-root runtime. su-exec is Alpine's gosu equivalent and is
# used by the entrypoint to drop from root (which only fixes volume
# ownership for pre-existing root-owned volumes) to the `pb` user before
# PocketBase serves HTTP — the same pattern Dockerfile.server uses with gosu.
RUN apk add --no-cache ca-certificates tzdata su-exec

RUN addgroup -S pb && adduser -S -G pb -h /pb pb

RUN mkdir -p /pb/pb_data && chown -R pb:pb /pb
VOLUME ["/pb/pb_data"]

COPY --from=builder /pb/pocketbase /pb/pocketbase
COPY --from=builder /pb/healthcheck /pb/healthcheck
COPY docker-entrypoint.sh /pb/docker-entrypoint.sh
RUN chmod +x /pb/docker-entrypoint.sh

EXPOSE 8080

# P8-4: Use Go healthcheck binary instead of curl. Pings /api/health with
# 5s interval, 5s timeout, 3 retries before marking container unhealthy.
HEALTHCHECK --interval=5s --timeout=5s --retries=3 \
    CMD /pb/healthcheck http://localhost:8080/api/health || exit 1

# Explicit HTTP keeps the E2E healthcheck on plain http://localhost:8080.
# A positional value is treated as an AutoTLS domain by PocketBase v0.39.6.
# The entrypoint drops privileges to `pb` before starting PocketBase
# (DOCKER-01) so the server never serves HTTP as root.
ENTRYPOINT ["/pb/docker-entrypoint.sh"]
CMD ["/pb/pocketbase", "serve", "--http=0.0.0.0:8080"]
