# Settings Consolidation — Plan & Status

## Architectural principle

> **If it's a page/tool → home "Tools" area + dedicated full page.**
> **If it's a configuration → stay on Settings.**
>
> Home "Tools" tiles can be shown or hidden based on **subscription tier**
> (C2.2 capabilities) and **user role**.

## Status: DONE

| Item | Change | Commit |
|---|---|---|
| 12 management tabs removed from settings hub | features, data, staff, terminals, stores, audit, offline, shifts, tax, exchange, promotions, kds cases deleted | `9196fb69` |
| Deep-link hash reader whitelisted | `#/settings/<section>` only resolves to the 11 kept tabs | `a4265229` |
| Settings hub categories | Management removed; topology folded into System | `a4265229` |
| New `tools` sidebar section | SectionName + SECTION_LABELS + SECTION_ORDER added; 8 nav items re-homed from `settings` to `tools` | `ffbcd5e2` |
| `settings` section now contains ONLY the hub | route `settings` (General) is the only item | `ffbcd5e2` |
| Home "Tools" area expanded | 10 missing tools added (terminals, stores, shifts, tax-config, exchange-rates, promotions, offline-queue, features, data-management, kds) | `4de55e4d` |
| Home tool gating | New `cap` field gates each tool by subscription capability; existing `minRole` gates by role | `4de55e4d` |
| FTL keys | `nav-section-tools` (EN/ID) + 20 `workspace-home-<tool>-{title,desc}` keys (EN/ID) | `ffbcd5e2`, `4de55e4d` |
| Tests | groupBySection, SettingsNavTree, WorkspaceHome all updated + passing | various |

## Final architecture

- **Settings** (route `settings`) — the hub with 11 configuration tabs:
  Business (general, appearance) · Operations (receipt, sync, email, store-pos, restaurant-pos, inventory) · System (about, license, topology)
- **Tools** — each a dedicated full page with its own route, reachable from:
  1. Home screen "Tools" area (role + subscription gated)
  2. Sidebar `tools` section (staff, audit, terminals, stores, shifts, offline, features, data) + their original sections (tax → finance, exchange → finance, promotions → finance, kds → operations)
- **No management screen is a settings tab anymore.**

## Remaining (optional / not blocking)

- The audit page KPI strip front-end (backend `Store::audit_summary` is staged).
- `AnalyticsScreen.tsx` has an in-progress "no workspace selected" feature from concurrent work that currently breaks `npm run typecheck` (unbalanced JSX) — unrelated to this consolidation; fix when that work lands.
