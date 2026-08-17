#!/usr/bin/env bash
# scripts/verify-docker-digests.sh — pinned-image digest drift gate.
#
# Re-resolves every `image:tag@sha256:...` reference pinned across the
# OZ-POS Dockerfiles and Compose files (DOCKER-02) and fails when any
# upstream tag now resolves to a DIFFERENT digest than what the repo pins.
#
# The pinning policy (see .trivyignore and Dockerfile comments) is: pin
# immutable multi-arch index digests, refresh deliberately via a
# dependency-update process. This gate automates the "deliberate" part —
# a scheduled CI run alerts on drift so pins never silently rot.
#
# Usage:
#   bash scripts/verify-docker-digests.sh
#
# Requires a Docker daemon with BuildKit (docker buildx imagetools).
# Exit 0 = all pins current. Exit 1 = drift detected (references listed).

set -euo pipefail
cd "$(dirname "$0")/.."

# Files that carry `image:tag@sha256:...` pins (DOCKER-02).
FILES=(
    "Dockerfile.server"
    "Dockerfile.unified"
    "apps/license-server/Dockerfile"
    "docker-compose.yml"
    "docker-compose.pg.yml"
    "docker-compose.e2e.yml"
)

drift=0
declare -a drift_lines=()

echo "── Resolving pinned image digests ──"
for f in "${FILES[@]}"; do
    [ -f "$f" ] || continue
    # Match `repo:tag@sha256:hex` (also handles registry-hosted repos).
    while IFS= read -r ref; do
        [ -n "$ref" ] || continue
        tag="${ref%@*}"
        pinned="${ref#*@sha256:}"
        # Resolve the CURRENT multi-arch index digest for the mutable tag.
        # Retry up to 5x with growing backoff — the weekly job runs against
        # a cold registry connection and registries (esp. public ECR) can
        # 429-rate-limit bursts for 30-60s; transient errors must not
        # false-alarm. The error is echoed on the last attempt for clarity.
        current=""
        for attempt in 1 2 3 4 5; do
            err=$(docker buildx imagetools inspect --format '{{.Manifest.Digest}}' "$tag" 2>&1 || true)
            current=$(echo "$err" | sed 's/^sha256://' | grep -E '^[0-9a-f]{64}$' || true)
            [ -n "$current" ] && break
            [ "$attempt" -eq 5 ] && echo "  (last attempt: $(echo "$err" | tail -1 | head -c 120))"
            sleep $((attempt * 3))
        done
        if [ -z "$current" ]; then
            # Fail CLOSED: an unpinnable pin is a supply-chain concern.
            echo "  ✘ $ref — could not resolve after 5 attempts (is the registry reachable?)"
            drift=1
            drift_lines+=("$f: $tag  UNRESOLVABLE (network/registry error)")
            continue
        fi
        if [ "$current" = "$pinned" ]; then
            echo "  ✔ $tag → $pinned"
        else
            echo "  ✘ $tag DRIFT: pinned $pinned, now $current"
            drift=1
            drift_lines+=("$f: $tag  pinned=$pinned  current=$current")
        fi
    done < <(grep -oE '[A-Za-z0-9._/-]+:[A-Za-z0-9._-]+@sha256:[0-9a-f]{64}' "$f")
done

echo
if [ "$drift" -ne 0 ]; then
    echo "✘ DIGEST DRIFT DETECTED — refresh the pins deliberately (DOCKER-02 policy):"
    printf '   %s\n' "${drift_lines[@]}"
    exit 1
fi
echo "✔ All pinned image digests are current."
