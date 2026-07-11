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
FROM golang:1.23-alpine AS builder

RUN apk add --no-cache gcc musl-dev

WORKDIR /src

# Cache dependencies first.
COPY go.mod go.sum ./
RUN go mod download

# Copy source files (explicit filenames to avoid test files).
COPY main.go activate.go renew.go status.go helpers.go expiry.go ratelimit.go ./
COPY pb_schema.json ./

RUN CGO_ENABLED=1 go build -o /pb/pocketbase -ldflags="-s -w" .

# ── Stage 2: Runtime ──────────────────────────────────────────────────
FROM alpine:3.20

RUN apk add --no-cache ca-certificates tzdata curl

RUN mkdir -p /pb/pb_data
VOLUME ["/pb/pb_data"]

COPY --from=builder /pb/pocketbase /pb/pocketbase

EXPOSE 8080

HEALTHCHECK --interval=30s --timeout=5s --retries=3 \
    CMD curl -f http://localhost:8080/api/ || exit 1

ENTRYPOINT ["/pb/pocketbase", "serve", "--http=0.0.0.0:8080"]
