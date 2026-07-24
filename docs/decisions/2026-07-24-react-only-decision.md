# ADR 30 – React‑only UI decision

**Date:** 2026‑07‑24

## Context
- The project originally planned a migration from React to SolidJS (see `ARCHITECTURE.md`).
- Approximately 200 UI tests, a full component library, and many integrations already exist for React 18.
- No SolidJS code has been written yet; the migration is only a *planned* footnote.

## Decision
- Stay with **React** as the UI framework for the foreseeable future.
- Remove the SolidJS migration footnote from `ARCHITECTURE.md`.
- Document the decision in this ADR (number 30) and treat it as the authoritative record.

## Consequences
- **Positive:** No disruption to the current development flow; we can continue delivering the beta on schedule.
- **Negative:** The long‑term architectural goal of a framework‑agnostic UI is postponed. Future migration to SolidJS would require a separate effort later.
- **Action items:**
  1. Update `ARCHITECTURE.md` to show `React` only in the frontend table.
  2. Add this ADR to `docs/decisions/`.
  3. Ensure CI and documentation reference the updated architecture.
