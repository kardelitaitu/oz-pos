// ── useAnimatedUndoStack contract tests ─────────────────────────
//
// Pins the race-safety contract formerly tested at the PosScreen
// level (where CartLineItem's own 200 ms exit timer blocked
// concurrent pushes from landing during the fade). With the state
// machine extracted into this hook, we exercise it directly via
// renderHook, which bypasses the CartLineItem layer entirely.
//
// Time control: `vi.useFakeTimers()` is activated AFTER
// `renderHookInAct` returns — the documented pattern that avoids
// React 18's `act()` boundary hanging on MessageChannel scheduling
// if fake timers are already active at mount.

import { describe, expect, it, vi, afterEach } from 'vitest';
import { act } from '@testing-library/react';
import { renderHookInAct } from '@/test-utils/renderInAct';
import { useAnimatedUndoStack } from '@/hooks/useAnimatedUndoStack';

interface Item {
  id: string;
  label: string;
}

const getId = (item: Item): string => item.id;
const item = (id: string, label?: string): Item => ({
  id,
  label: label ?? id,
});

describe('useAnimatedUndoStack', () => {
  afterEach(() => {
    vi.useRealTimers();
  });

  describe('initial state', () => {
    it('returns an empty stack with isExiting=false and shouldRender=false', async () => {
      vi.useFakeTimers();
      const { result } = await renderHookInAct(() =>
        useAnimatedUndoStack<Item>({ maxSize: 5, getId }),
      );

      expect(result.current.stack).toEqual([]);
      expect(result.current.isExiting).toBe(false);
      expect(result.current.shouldRender).toBe(false);
    });
  });

  describe('push', () => {
    it('appends new items to the top of the stack', async () => {
      vi.useFakeTimers();
      const { result } = await renderHookInAct(() =>
        useAnimatedUndoStack<Item>({ maxSize: 5, getId }),
      );

      act(() => { result.current.push(item('a')); });
      act(() => { result.current.push(item('b')); });

      // Most recent is at index 0.
      expect(result.current.stack.map(getId)).toEqual(['b', 'a']);
      expect(result.current.shouldRender).toBe(true);
    });

    it('drops the oldest entry when exceeding maxSize', async () => {
      vi.useFakeTimers();
      const { result } = await renderHookInAct(() =>
        useAnimatedUndoStack<Item>({ maxSize: 3, getId }),
      );

      act(() => {
        result.current.push(item('a'));
        result.current.push(item('b'));
        result.current.push(item('c'));
        result.current.push(item('d'));
      });

      expect(result.current.stack.map(getId)).toEqual(['d', 'c', 'b']);
    });
  });

  describe('pop', () => {
    it('returns undefined when the stack is empty', async () => {
      vi.useFakeTimers();
      const { result } = await renderHookInAct(() =>
        useAnimatedUndoStack<Item>({ maxSize: 5, getId }),
      );

      let popped: Item | undefined;
      act(() => { popped = result.current.pop(); });

      expect(popped).toBeUndefined();
      expect(result.current.isExiting).toBe(false);
    });

    it('returns the top and slices it off when the stack has > 1 entry (no fade)', async () => {
      vi.useFakeTimers();
      const { result } = await renderHookInAct(() =>
        useAnimatedUndoStack<Item>({ maxSize: 5, getId }),
      );

      act(() => { result.current.push(item('a')); });
      act(() => { result.current.push(item('b')); });
      act(() => { result.current.push(item('c')); });

      let popped: Item | undefined;
      act(() => { popped = result.current.pop(); });

      expect(popped?.id).toBe('c');
      expect(result.current.stack.map(getId)).toEqual(['b', 'a']);
      // No fade because the stack still has entries after the pop.
      expect(result.current.isExiting).toBe(false);
    });

    it('returns the top and runs the race-safe fade when popping empties the stack', async () => {
      vi.useFakeTimers();
      const { result } = await renderHookInAct(() =>
        useAnimatedUndoStack<Item>({ maxSize: 5, getId }),
      );

      act(() => { result.current.push(item('a')); });

      let popped: Item | undefined;
      act(() => { popped = result.current.pop(); });

      expect(popped?.id).toBe('a');
      // The fade begins immediately; isExiting is true.
      expect(result.current.isExiting).toBe(true);

      // After the full 200 ms with no race, the timer clears the stack.
      act(() => { vi.advanceTimersByTime(200); });

      expect(result.current.isExiting).toBe(false);
      expect(result.current.stack).toEqual([]);
      expect(result.current.shouldRender).toBe(false);
    });
  });

  describe('dismiss', () => {
    it('clears the stack after 200 ms when no concurrent push lands', async () => {
      vi.useFakeTimers();
      const { result } = await renderHookInAct(() =>
        useAnimatedUndoStack<Item>({ maxSize: 5, getId }),
      );

      act(() => { result.current.push(item('a')); });
      act(() => { result.current.dismiss(); });

      expect(result.current.isExiting).toBe(true);

      act(() => { vi.advanceTimersByTime(200); });

      expect(result.current.isExiting).toBe(false);
      expect(result.current.stack).toEqual([]);
      expect(result.current.shouldRender).toBe(false);
    });

    it('is a no-op when the stack is empty and clears any stray isExiting', async () => {
      vi.useFakeTimers();
      const { result } = await renderHookInAct(() =>
        useAnimatedUndoStack<Item>({ maxSize: 5, getId }),
      );

      act(() => { result.current.dismiss(); });

      expect(result.current.isExiting).toBe(false);
      expect(result.current.shouldRender).toBe(false);
    });

    it('debounces the timer when called twice: second dismiss resets the snapshot', async () => {
      vi.useFakeTimers();
      const { result } = await renderHookInAct(() =>
        useAnimatedUndoStack<Item>({ maxSize: 5, getId }),
      );

      act(() => { result.current.push(item('a')); });
      act(() => { result.current.push(item('b')); });

      // First dismiss captures snapshot = [b, a] and schedules 200 ms.
      act(() => { result.current.dismiss(); });
      act(() => { vi.advanceTimersByTime(100); });

      // Second dismiss (50 ms later in real time) re-snapshots to the
      // current [b, a] and starts a fresh 200 ms timer. The first
      // timer is cancelled by `clearTimeout` inside scheduleClearFade.
      act(() => { result.current.dismiss(); });
      // Advance past where the FIRST timer would have fired (100 ms
      // from now total 200 ms from first dismiss), but short of the
      // second timer's deadline (200 ms from second dismiss).
      act(() => { vi.advanceTimersByTime(50); });

      // Still exiting — second timer hasn't fired yet.
      expect(result.current.isExiting).toBe(true);
      expect(result.current.stack.map(getId)).toEqual(['b', 'a']);

      // Advance through the rest of the second timer.
      act(() => { vi.advanceTimersByTime(150); });

      expect(result.current.isExiting).toBe(false);
      expect(result.current.stack).toEqual([]);
    });
  });

  describe('race-safety contract', () => {
    it('preserves a new push onto the stack when a push lands during the exit fade', async () => {
      vi.useFakeTimers();
      const { result } = await renderHookInAct(() =>
        useAnimatedUndoStack<Item>({ maxSize: 5, getId }),
      );

      // Push the first item.
      act(() => { result.current.push(item('a')); });
      expect(result.current.stack.map(getId)).toEqual(['a']);

      // Schedule the dismiss fade. Snapshot = [a] captured, timer
      // scheduled for 200 ms.
      act(() => { result.current.dismiss(); });
      expect(result.current.isExiting).toBe(true);
      expect(result.current.shouldRender).toBe(true);

      // Mid-fade (100 ms in): another push lands. The live stack
      // ([b, a]) now diverges from the dismiss-time snapshot ([a]).
      act(() => {
        vi.advanceTimersByTime(100);
        result.current.push(item('b'));
      });
      expect(result.current.stack.map(getId)).toEqual(['b', 'a']);

      // Advance the remaining 100 ms. The id-set compare inside
      // the timer body detects liveIds.size=2 vs snapshot.length=1
      // and aborts the clear. The exiting flag still flips off so
      // the new stack keeps its entry animation.
      act(() => { vi.advanceTimersByTime(100); });

      expect(result.current.isExiting).toBe(false);
      expect(result.current.stack.map(getId)).toEqual(['b', 'a']);
      expect(result.current.shouldRender).toBe(true);
    });

    it('preserves multiple concurrent pushes during the exit fade', async () => {
      vi.useFakeTimers();
      const { result } = await renderHookInAct(() =>
        useAnimatedUndoStack<Item>({ maxSize: 5, getId }),
      );

      act(() => { result.current.push(item('a')); });
      act(() => { result.current.dismiss(); });

      act(() => {
        vi.advanceTimersByTime(100);
        result.current.push(item('b'));
        result.current.push(item('c'));
      });
      expect(result.current.stack.map(getId)).toEqual(['c', 'b', 'a']);

      act(() => { vi.advanceTimersByTime(100); });

      // All three items preserved.
      expect(result.current.isExiting).toBe(false);
      expect(result.current.stack.map(getId)).toEqual(['c', 'b', 'a']);
    });

    it('clears safely when concurrent pushes land then pop back to the snapshot', async () => {
      vi.useFakeTimers();
      const { result } = await renderHookInAct(() =>
        useAnimatedUndoStack<Item>({ maxSize: 5, getId }),
      );

      act(() => { result.current.push(item('a')); });
      act(() => { result.current.dismiss(); });
      // Snapshot = [a], exiting = true.

      // Mid-fade: push b.
      act(() => {
        vi.advanceTimersByTime(50);
        result.current.push(item('b'));
      });
      // Stack now [b, a].

      // Pop b — synchronous slice (no fade because length > 1). Live
      // stack ends up at [a], matching the dismiss-time snapshot.
      let popped: Item | undefined;
      act(() => { popped = result.current.pop(); });
      expect(popped?.id).toBe('b');
      expect(result.current.stack.map(getId)).toEqual(['a']);

      // The live stack now matches the dismiss-time snapshot
      // exactly, so the timer body treats it as no concurrent
      // activity and clears it.
      act(() => { vi.advanceTimersByTime(150); });

      expect(result.current.isExiting).toBe(false);
      expect(result.current.stack).toEqual([]);
    });
  });

  describe('lifecycle', () => {
    it('cancels the pending timer when the host unmounts mid-fade', async () => {
      vi.useFakeTimers();
      const { result, unmount } = await renderHookInAct(() =>
        useAnimatedUndoStack<Item>({ maxSize: 5, getId }),
      );

      act(() => { result.current.push(item('a')); });
      act(() => { result.current.dismiss(); });
      expect(result.current.isExiting).toBe(true);

      act(() => { vi.advanceTimersByTime(100); });

      // Unmount in the middle of the fade. The cleanup useEffect
      // cancels the pending timer so setState never fires against
      // an unmounted component.
      unmount();

      // Advancing the rest of the timer must not throw and must
      // not surface any "setState on unmounted" warnings.
      act(() => { vi.advanceTimersByTime(200); });
    });
  });
});
