// ── renderInAct — shared test helper ─────────────────────────────
//
// Wraps `@testing-library/react`'s `render()` in `await act()` so that
// async mount-effect state updates (e.g., a `useEffect` that fires an
// async IPC on mount) are included in the same act() flush. Without
// this, vitest emits:
//
//   "An update to <Component> inside a test was not wrapped in
//    act(...). When testing, code that causes React state updates
//    should be wrapped into act(...): act(() => { ... })"
//
// Usage:
//
//   import { renderInAct } from '@/test-utils/renderInAct';
//   …
//   await renderInAct(<MyComponent />);
//   await waitFor(() => expect(screen.getByText('…')).toBeInTheDocument());

import type { ReactElement } from 'react';
import {
  act,
  render,
  renderHook,
  type RenderHookOptions,
  type RenderHookResult,
  type RenderResult,
} from '@testing-library/react';

/**
 * Render `ui` inside an `await act()` boundary so async mount-effect
 * state updates are included in the same act() flush. Returns the
 * standard `@testing-library/react` `RenderResult` so callers can use
 * the `rerender` / `unmount` methods when needed (most tests just
 * use `screen` directly and ignore the return value).
 */
export async function renderInAct(ui: ReactElement): Promise<RenderResult> {
  let result!: RenderResult;
  await act(async () => {
    result = render(ui);
  });
  return result;
}

/**
 * Render a hook via `renderHook` inside an `await act()` boundary.
 * Same rationale as `renderInAct` — async mount-effect state updates
 * (e.g., a hook whose `useEffect` fires an async IPC on mount) are
 * included in the same act() flush so vitest doesn't emit
 *
 *   "An update to <Component> inside a test was not wrapped in act(...)."
 *
 * Usage:
 *
 *   const { result } = await renderHookInAct(() => useMyHook());
 *   expect(result.current.value).toBe(42);
 */
export async function renderHookInAct<Props = undefined, Result = unknown>(
  callback: (props?: Props) => Result,
  options?: RenderHookOptions<Props>,
): Promise<RenderHookResult<Result, Props>> {
  let result!: RenderHookResult<Result, Props>;
  await act(async () => {
    result = renderHook(callback, options);
  });
  return result;
}
