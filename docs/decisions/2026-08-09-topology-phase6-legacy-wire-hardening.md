# ADR: Topology Phase 6 — Legacy Wire Hardening

**Date:** 2026-08-09
**Status:** Implemented

## Problem

Legacy topology rows may contain only geometric endpoints. Known identities can be migrated safely, but an arbitrary workspace-to-workspace wire has no reliable semantic meaning. The frontend already folded these rows to `legacy-out → legacy-in`, while the backend's legacy compatibility path could still persist them because semantic validation was skipped when no semantic fields were present.

## Decision

Known legacy identities remain loadable and saveable:

- Branch Location → workspace → Location;
- workspace → warehouse → stock routing;
- Restaurant POS → KDS → operation feed;
- KDS → hardware → ticket routing.

All other geometry-only wires with resolvable endpoints are rejected at Apply with `ambiguous-legacy-wire`. The frontend reports a repairable localized message instructing the user to delete and reconnect the wire using labeled ports. The Rust boundary enforces the same rule for direct IPC callers. Unknown endpoints continue to use the existing structural error instead of being misclassified as ambiguous.

## Verification

- Frontend contract test covers an ambiguous legacy workspace-to-workspace wire.
- Rust semantic save test rejects the same wire.
- Existing legacy Branch Location → workspace compatibility remains covered by the topology command suite.
- Rust formatting, i18n lint, and diff checks pass.
