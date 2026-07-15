// ── useWorkspaceNav tests ──────────────────────────────────────────
//
// useWorkspaceNav is a thin wrapper around useWorkspace() that provides
// goToWorkspacePicker, which calls setActiveWorkspace(null) to navigate
// back to the workspace selection screen.

import { describe, expect, it, vi, beforeEach } from 'vitest';
import { renderHook, act } from '@testing-library/react';
import { useWorkspaceNav } from '@/hooks/useWorkspaceNav';

// ── Hoisted mocks ──────────────────────────────────────────────────

const mockSetActiveWorkspace = vi.fn();

vi.mock('@/contexts/WorkspaceContext', () => ({
  useWorkspace: () => ({
    setActiveWorkspace: mockSetActiveWorkspace,
  }),
}));

// ── Tests ──────────────────────────────────────────────────────────

describe('useWorkspaceNav', () => {
  it('returns goToWorkspacePicker function', () => {
    const { result } = renderHook(() => useWorkspaceNav());
    expect(typeof result.current.goToWorkspacePicker).toBe('function');
  });

  it('calls setActiveWorkspace(null) when goToWorkspacePicker is invoked', () => {
    const { result } = renderHook(() => useWorkspaceNav());

    act(() => {
      result.current.goToWorkspacePicker();
    });

    expect(mockSetActiveWorkspace).toHaveBeenCalledTimes(1);
    expect(mockSetActiveWorkspace).toHaveBeenCalledWith(null);
  });

  it('goToWorkspacePicker reference is stable across re-renders', () => {
    const { result, rerender } = renderHook(() => useWorkspaceNav());

    const firstRef = result.current.goToWorkspacePicker;

    rerender();

    expect(result.current.goToWorkspacePicker).toBe(firstRef);
  });

  it('can be called multiple times', () => {
    const { result } = renderHook(() => useWorkspaceNav());

    act(() => {
      result.current.goToWorkspacePicker();
    });
    act(() => {
      result.current.goToWorkspacePicker();
    });
    act(() => {
      result.current.goToWorkspacePicker();
    });

    expect(mockSetActiveWorkspace).toHaveBeenCalledTimes(3);
    expect(mockSetActiveWorkspace).toHaveBeenCalledWith(null);
  });
});
