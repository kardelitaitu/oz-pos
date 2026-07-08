// ── Shared test helper: exit-animation host pattern ────────────────
//
// Import this into any test file that needs to drive a dismissing
// surface (modal, banner, panel) through its exit-animation cycle
// with deterministic fake-timer control.
//
// The pattern: test creates a mutable `hostRef`, renders the surface
// inside `<ExitAnimHost>`, then calls dismiss handlers and asserts
// intermediate (mid-fade) + final (post-fade) DOM state.
//
// Usage:
//
//   import { ExitAnimHost, createExitAnimHostRef, advanceFade,
//           advanceFadeSync, expectExiting, expectNotExiting }
//     from '../test-utils/exitAnimHost';
//   import type { ExitAnimHostRef } from '../test-utils/exitAnimHost';
//
//   interface MyHostRef extends ExitAnimHostRef {
//     extraFlag: boolean;
//     setExtraFlag: (v: boolean) => void;
//   }
//
//   function makeMyHostRef(): MyHostRef {
//     return { ...createExitAnimHostRef(), extraFlag: false, setExtraFlag: () => {} };
//   }
//
//   function MyHost({ hostRef, initialOpen = true }) {
//     const [extraFlag, setExtraFlag] = useState(false);
//     hostRef.extraFlag = extraFlag;
//     hostRef.setExtraFlag = setExtraFlag;
//     return (
//       <ExitAnimHost hostRef={hostRef} initialOpen={initialOpen}>
//         {(open, setOpen) => (
//           <MySurface
//             open={open}
//             onClose={() => setOpen(false)}
//             onExtra={() => setExtraFlag(true)}
//           />
//         )}
//       </ExitAnimHost>
//     );
//   }

import { vi, expect } from 'vitest';
import { useState, type ReactNode } from 'react';
import { act } from '@testing-library/react';

// ── Mutable ref ───────────────────────────────────────────────────

/**
 * Mutable ref that mirrors React `open` and `setOpen` state from an
 * `<ExitAnimHost>`. Tests read/write these fields synchronously
 * without calling React hooks outside a render context.
 *
 * Extend this interface for surface-specific extra state:
 *
 *   interface MyRef extends ExitAnimHostRef {
 *     paid: boolean;
 *     setPaid: (v: boolean) => void;
 *   }
 */
export interface ExitAnimHostRef {
  /** Mirrors the parent `open` state after each render. */
  open: boolean;
  /** React dispatch for `open` state. Call to flip `open` from tests. */
  setOpen: (v: boolean) => void;
}

/**
 * Factory: returns a fresh `ExitAnimHostRef` with `open` defaulting to
 * `true`. The `setOpen` stub is replaced by `<ExitAnimHost>` on first
 * render.
 */
export function createExitAnimHostRef(initialOpen = true): ExitAnimHostRef {
  return { open: initialOpen, setOpen: () => {} };
}

// ── Host component ────────────────────────────────────────────────

/**
 * Renders a real React component that owns the `open` state, mirrors
 * it into the mutable `hostRef`, and passes `(open, setOpen)` to its
 * children as render-props.
 *
 * Each test file wraps this inside its own `HostModal` component to
 * add surface-specific extra state (e.g. `refunded`, `paid`) and
 * `<span data-testid>` trackers.
 */
export function ExitAnimHost({
  hostRef,
  initialOpen = true,
  children,
}: {
  hostRef: ExitAnimHostRef;
  initialOpen?: boolean;
  children: (open: boolean, setOpen: (v: boolean) => void) => ReactNode;
}) {
  const [open, setOpen] = useState(initialOpen);
  hostRef.open = open;
  hostRef.setOpen = setOpen;
  return <>{children(open, setOpen)}</>;
}

// ── Fake-timer helpers ────────────────────────────────────────────

/**
 * Advance vitest fake timers by `ms` milliseconds, flushing microtasks
 * (e.g. Promise chained from `downloadAndInstall`). Use this when the
 * dismiss handler has an `async` callback.
 *
 *   await advanceFade(200);
 */
export async function advanceFade(ms: number) {
  await act(async () => {
    await vi.advanceTimersByTimeAsync(ms);
  });
}

/**
 * Advance vitest fake timers by `ms` milliseconds **without** flushing
 * microtasks. Use this for synchronous dismiss × button handlers.
 *
 *   advanceFadeSync(199);  // mid-fade assertion
 *   advanceFadeSync(1);    // fade complete
 */
export function advanceFadeSync(ms: number) {
  act(() => {
    vi.advanceTimersByTime(ms);
  });
}

// ── DOM class assertions ──────────────────────────────────────────

/**
 * Assert that an element has the `--exiting` modifier class applied.
 *
 *   expectExiting(overlay, 'refund-overlay');
 *
 * Asserts `overlay.classList.contains('refund-overlay--exiting')`.
 */
export function expectExiting(el: Element | null, baseClass: string) {
  expect(el?.classList.contains(`${baseClass}--exiting`)).toBe(true);
}

/**
 * Assert that an element does NOT have the `--exiting` modifier class.
 *
 *   expectNotExiting(overlay, 'refund-overlay');
 */
export function expectNotExiting(el: Element | null, baseClass: string) {
  expect(el?.classList.contains(`${baseClass}--exiting`)).toBe(false);
}
