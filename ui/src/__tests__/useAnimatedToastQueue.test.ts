// ── useAnimatedToastQueue tests ─────────────────────────────────────
//
// Covers: enqueue, FIFO eviction, auto-dismiss, dismiss (idempotent,
// timer-based removal), clearAll (coordinated fade, race-safety),
// and edge cases (empty queue, no-op dismiss during clearAll).

import { describe, expect, it, vi, beforeEach, afterEach } from 'vitest';
import { renderHook, act } from '@testing-library/react';
import { useAnimatedToastQueue } from '@/hooks/useAnimatedToastQueue';

// ── Mocks ──────────────────────────────────────────────────────────

// Ensure animDuration returns the actual ms value (not zero for
// reduced-motion) so fake timer advances work predictably.
vi.mock('@/utils/animation', () => ({
  animDuration: (ms: number) => ms,
}));

// ── Test items ─────────────────────────────────────────────────────

interface TestItem {
  id: string;
  label: string;
}

const makeItem = (id: string): TestItem => ({ id, label: `Item ${id}` });
const getId = (item: TestItem): string => item.id;

// ── Helpers ────────────────────────────────────────────────────────

const DEFAULT_OPTIONS = {
  getId,
  fadeMs: 200,
} as const;

// ── Tests ──────────────────────────────────────────────────────────

describe('useAnimatedToastQueue', () => {
  describe('initial state', () => {
    it('starts with empty items and exitingIds', () => {
      const { result } = renderHook(() =>
        useAnimatedToastQueue(DEFAULT_OPTIONS),
      );

      expect(result.current.items).toEqual([]);
      expect(result.current.exitingIds.size).toBe(0);
    });
  });

  // ── enqueue ────────────────────────────────────────────────────

  describe('enqueue', () => {
    it('adds items in insertion order', () => {
      const { result } = renderHook(() =>
        useAnimatedToastQueue(DEFAULT_OPTIONS),
      );

      act(() => {
        result.current.enqueue(makeItem('a'));
        result.current.enqueue(makeItem('b'));
        result.current.enqueue(makeItem('c'));
      });

      expect(result.current.items.map(getId)).toEqual(['a', 'b', 'c']);
    });

    it('evicts oldest item when over maxSize (FIFO)', () => {
      const { result } = renderHook(() =>
        useAnimatedToastQueue({ ...DEFAULT_OPTIONS, maxSize: 2 }),
      );

      act(() => {
        result.current.enqueue(makeItem('a'));
        result.current.enqueue(makeItem('b'));
        result.current.enqueue(makeItem('c')); // evicts 'a'
      });

      expect(result.current.items.map(getId)).toEqual(['b', 'c']);
    });
  });

  describe('enqueue auto-dismiss', () => {
    beforeEach(() => {
      vi.useFakeTimers();
    });

    afterEach(() => {
      vi.useRealTimers();
    });

    it('schedules auto-dismiss when getAutoDismissMs returns positive', () => {
      const getAutoDismissMs = vi.fn((_item: TestItem) => 500);
      const { result } = renderHook(() =>
        useAnimatedToastQueue({
          ...DEFAULT_OPTIONS,
          getAutoDismissMs,
        }),
      );

      act(() => {
        result.current.enqueue(makeItem('auto'));
      });

      expect(result.current.items).toHaveLength(1);

      // Advance past the auto-dismiss delay.
      act(() => {
        vi.advanceTimersByTime(500);
      });

      // Wait for fade (200ms) after auto-dismiss triggers dismiss().
      act(() => {
        vi.advanceTimersByTime(200);
      });

      expect(result.current.items).toHaveLength(0);
    });

    it('cancels pending auto-dismiss when dismissed manually', () => {
      const getAutoDismissMs = vi.fn((_item: TestItem) => 500);
      const { result } = renderHook(() =>
        useAnimatedToastQueue({
          ...DEFAULT_OPTIONS,
          getAutoDismissMs,
        }),
      );

      act(() => {
        result.current.enqueue(makeItem('x'));
      });
      expect(result.current.items).toHaveLength(1);

      // Manually dismiss before the auto-dismiss timer fires.
      act(() => {
        result.current.dismiss('x');
      });

      // Advance past the auto-dismiss delay — nothing should happen
      // because the manual dismiss cancelled the auto-dismiss timer.
      act(() => {
        vi.advanceTimersByTime(500);
      });
      // Advance past the fade.
      act(() => {
        vi.advanceTimersByTime(200);
      });

      // Item removed exactly once (by the manual dismiss).
      expect(result.current.items).toHaveLength(0);
    });
  });

  // ── dismiss ────────────────────────────────────────────────────

  describe('dismiss', () => {
    beforeEach(() => {
      vi.useFakeTimers();
    });

    afterEach(() => {
      vi.useRealTimers();
    });

    it('adds id to exitingIds immediately', () => {
      const { result } = renderHook(() =>
        useAnimatedToastQueue(DEFAULT_OPTIONS),
      );

      act(() => {
        result.current.enqueue(makeItem('x'));
      });
      act(() => {
        result.current.dismiss('x');
      });

      expect(result.current.exitingIds.has('x')).toBe(true);
      // Item still in items until fade completes.
      expect(result.current.items).toHaveLength(1);
    });

    it('removes item after fadeMs', () => {
      const { result } = renderHook(() =>
        useAnimatedToastQueue(DEFAULT_OPTIONS),
      );

      act(() => {
        result.current.enqueue(makeItem('x'));
      });
      act(() => {
        result.current.dismiss('x');
      });

      // Advance past fade duration.
      act(() => {
        vi.advanceTimersByTime(200);
      });

      expect(result.current.items).toHaveLength(0);
      expect(result.current.exitingIds.has('x')).toBe(false);
    });

    it('is idempotent — calling dismiss during fade is a no-op', () => {
      const { result } = renderHook(() =>
        useAnimatedToastQueue(DEFAULT_OPTIONS),
      );

      act(() => {
        result.current.enqueue(makeItem('x'));
      });
      act(() => {
        result.current.dismiss('x');
      });

      const exitingCount = result.current.exitingIds.size;

      // Second dismiss during fade — should be ignored.
      act(() => {
        result.current.dismiss('x');
      });

      // exitingIds count unchanged.
      expect(result.current.exitingIds.size).toBe(exitingCount);
    });
  });

  // ── clearAll ───────────────────────────────────────────────────

  describe('clearAll', () => {
    beforeEach(() => {
      vi.useFakeTimers();
    });

    afterEach(() => {
      vi.useRealTimers();
    });

    it('marks all items as exiting immediately', () => {
      const { result } = renderHook(() =>
        useAnimatedToastQueue(DEFAULT_OPTIONS),
      );

      act(() => {
        result.current.enqueue(makeItem('a'));
        result.current.enqueue(makeItem('b'));
        result.current.enqueue(makeItem('c'));
      });
      act(() => {
        result.current.clearAll();
      });

      expect(result.current.exitingIds.has('a')).toBe(true);
      expect(result.current.exitingIds.has('b')).toBe(true);
      expect(result.current.exitingIds.has('c')).toBe(true);
      // Items still visible during fade.
      expect(result.current.items).toHaveLength(3);
    });

    it('removes all items after fadeMs', () => {
      const { result } = renderHook(() =>
        useAnimatedToastQueue(DEFAULT_OPTIONS),
      );

      act(() => {
        result.current.enqueue(makeItem('a'));
        result.current.enqueue(makeItem('b'));
      });
      act(() => {
        result.current.clearAll();
      });

      act(() => {
        vi.advanceTimersByTime(200);
      });

      expect(result.current.items).toHaveLength(0);
      expect(result.current.exitingIds.size).toBe(0);
    });

    it('is a no-op on an empty queue', () => {
      const { result } = renderHook(() =>
        useAnimatedToastQueue(DEFAULT_OPTIONS),
      );

      expect(() => {
        act(() => {
          result.current.clearAll();
        });
      }).not.toThrow();

      expect(result.current.items).toHaveLength(0);
    });

    it('race-safety: items enqueued during fade survive', () => {
      const { result } = renderHook(() =>
        useAnimatedToastQueue(DEFAULT_OPTIONS),
      );

      act(() => {
        result.current.enqueue(makeItem('a'));
        result.current.enqueue(makeItem('b'));
      });
      act(() => {
        result.current.clearAll();
      });

      // During the fade, enqueue a new item — it should survive.
      act(() => {
        result.current.enqueue(makeItem('survivor'));
      });

      // Advance past fade — only snapshot items ('a', 'b') removed.
      act(() => {
        vi.advanceTimersByTime(200);
      });

      const ids = result.current.items.map(getId);
      expect(ids).toContain('survivor');
      expect(ids).not.toContain('a');
      expect(ids).not.toContain('b');
    });
  });
});
