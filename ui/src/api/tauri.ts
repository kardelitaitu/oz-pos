/**
 * Single sanctioned re-export surface for the raw Tauri API modules
 * (core/event/app/window).
 *
 * UI-2 convention fix: project rules require front-end Tauri access to
 * go through `ui/src/api/` — components and hooks must not import
 * `@tauri-apps/api/*` directly. Routing these four primitives through
 * here keeps that rule enforceable (a lint/search for direct imports
 * outside `src/api/` now only ever matches this file) and gives the
 * dev-mock/test seams a single place to alias.
 *
 * `invoke` specifically should use `loggedInvoke` from
 * `@/utils/logged-invoke`, which wraps `invoke` from this module's
 * sibling surface with timing/telemetry logging.
 */
export { convertFileSrc, invoke } from '@tauri-apps/api/core';
export { listen } from '@tauri-apps/api/event';
export { getVersion } from '@tauri-apps/api/app';
export { getCurrentWindow } from '@tauri-apps/api/window';
