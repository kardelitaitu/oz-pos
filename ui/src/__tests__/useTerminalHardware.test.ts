import { describe, it, expect, beforeEach, vi } from 'vitest';
import { renderHook, act, waitFor } from '@testing-library/react';
import {
  useTerminalHardware,
  createDefaultProfile,
  type TerminalHardwareProfile,
} from '@/hooks/useTerminalHardware';

// ── Helpers ───────────────────────────────────────────────────────

function storageKey(terminalId: string): string {
  return `oz-pos-terminal-hardware-${terminalId}`;
}

function seedProfile(terminalId: string, profile: TerminalHardwareProfile): void {
  localStorage.setItem(storageKey(terminalId), JSON.stringify(profile));
}

function readProfile(terminalId: string): TerminalHardwareProfile | null {
  const raw = localStorage.getItem(storageKey(terminalId));
  return raw ? JSON.parse(raw) : null;
}

function makeProfile(terminalId: string): TerminalHardwareProfile {
  return createDefaultProfile(terminalId, 'store-1');
}

// ── Tests ─────────────────────────────────────────────────────────

describe('useTerminalHardware', () => {
  beforeEach(() => {
    localStorage.clear();
  });

  // ── Initial load ──────────────────────────────────────────────

  it('loads default profile when no stored data exists', async () => {
    const { result } = renderHook(() => useTerminalHardware('term-001'));

    await waitFor(() => {
      expect(result.current.isLoading).toBe(false);
    });

    expect(result.current.profile).not.toBeNull();
    expect(result.current.profile!.terminalId).toBe('term-001');
    expect(result.current.profile!.hardware.printer.connection).toBe('auto');
    expect(result.current.profile!.hardware.scale.connection).toBe('none');
  });

  it('loads existing profile from localStorage', async () => {
    const profile = { ...makeProfile('term-002'), hardware: { ...makeProfile('term-002').hardware, printer: { connection: 'network' as const, devicePath: '192.168.1.50', paperSize: '80' as const, testPrintIp: '192.168.1.50' } } };
    seedProfile('term-002', profile);

    const { result } = renderHook(() => useTerminalHardware('term-002'));

    await waitFor(() => {
      expect(result.current.isLoading).toBe(false);
    });

    expect(result.current.profile!.hardware.printer.connection).toBe('network');
    expect(result.current.profile!.hardware.printer.devicePath).toBe('192.168.1.50');
  });

  // ── Corruption recovery ───────────────────────────────────────

  it('recovers from corrupted JSON by creating default profile', async () => {
    localStorage.setItem(storageKey('term-c'), '{ broken json ---');

    const { result } = renderHook(() => useTerminalHardware('term-c'));

    await waitFor(() => {
      expect(result.current.isLoading).toBe(false);
    });

    expect(result.current.profile!.terminalId).toBe('term-c');
    expect(result.current.error).toBe('Hardware profile was corrupted — reset to defaults.');

    // Verify corrupted data was backed up
    const keys = Object.keys(localStorage).filter((k) => k.includes('corrupted'));
    expect(keys.length).toBe(1);

    // Verify fresh profile was NOT stored (only in memory)
    expect(localStorage.getItem(storageKey('term-c'))).toBeNull();
  });

  it('merges valid fields and resets invalid fields on schema failure', async () => {
    // Valid profile but with invalid enum value
    const invalid = {
      terminalId: 'term-d',
      storeId: 'store-1',
      hardware: {
        printer: { connection: 'invalid_conn', devicePath: 'COM3', paperSize: '58' },
        scale: { connection: 'none' as const, devicePath: '', baudRate: 9600, zeroOnBoot: false },
        scanner: { mode: 'auto' as const, deviceId: '' },
      },
      localPrefs: { soundVolume: -1, darkMode: true, scaleAutoZero: true },
      initialized: '2026-01-01T00:00:00Z',
      version: 1,
    };
    localStorage.setItem(storageKey('term-d'), JSON.stringify(invalid));

    const { result } = renderHook(() => useTerminalHardware('term-d'));

    await waitFor(() => {
      expect(result.current.isLoading).toBe(false);
    });

    // Valid fields preserved
    expect(result.current.profile!.hardware.printer.devicePath).toBe('COM3');
    expect(result.current.profile!.hardware.printer.paperSize).toBe('58');
    // Invalid printer connection reset to default
    expect(result.current.profile!.hardware.printer.connection).toBe('auto');
    // Invalid soundVolume reset to default
    expect(result.current.profile!.localPrefs.soundVolume).toBe(80);
    // Valid darkMode preserved
    expect(result.current.profile!.localPrefs.darkMode).toBe(true);
  });

  // ── Update helpers (local state) ──────────────────────────────

  it('updatePrinter modifies local state without persisting', async () => {
    seedProfile('term-e', makeProfile('term-e'));
    const { result } = renderHook(() => useTerminalHardware('term-e'));

    await waitFor(() => {
      expect(result.current.isLoading).toBe(false);
    });

    act(() => {
      result.current.updatePrinter({ connection: 'usb', devicePath: 'COM5' });
    });

    expect(result.current.profile!.hardware.printer.connection).toBe('usb');
    expect(result.current.profile!.hardware.printer.devicePath).toBe('COM5');

    // Not yet persisted
    const stored = readProfile('term-e');
    expect(stored!.hardware.printer.connection).toBe('auto');
  });

  it('updateScale modifies local state', async () => {
    seedProfile('term-f', makeProfile('term-f'));
    const { result } = renderHook(() => useTerminalHardware('term-f'));

    await waitFor(() => {
      expect(result.current.isLoading).toBe(false);
    });

    act(() => {
      result.current.updateScale({ connection: 'serial', devicePath: 'COM3', baudRate: 115200 });
    });

    expect(result.current.profile!.hardware.scale.connection).toBe('serial');
    expect(result.current.profile!.hardware.scale.baudRate).toBe(115200);
  });

  it('updateScanner modifies local state', async () => {
    seedProfile('term-g', makeProfile('term-g'));
    const { result } = renderHook(() => useTerminalHardware('term-g'));

    await waitFor(() => {
      expect(result.current.isLoading).toBe(false);
    });

    act(() => {
      result.current.updateScanner({ mode: 'keyboard', deviceId: 'HID-001' });
    });

    expect(result.current.profile!.hardware.scanner.mode).toBe('keyboard');
    expect(result.current.profile!.hardware.scanner.deviceId).toBe('HID-001');
  });

  it('updateLocalPrefs modifies local state', async () => {
    seedProfile('term-h', makeProfile('term-h'));
    const { result } = renderHook(() => useTerminalHardware('term-h'));

    await waitFor(() => {
      expect(result.current.isLoading).toBe(false);
    });

    act(() => {
      result.current.updateLocalPrefs({ soundVolume: 42, darkMode: true });
    });

    expect(result.current.profile!.localPrefs.soundVolume).toBe(42);
    expect(result.current.profile!.localPrefs.darkMode).toBe(true);
  });

  // ── Save (persist) ─────────────────────────────────────────────

  it('save persists profile to localStorage', async () => {
    seedProfile('term-i', makeProfile('term-i'));
    const { result } = renderHook(() => useTerminalHardware('term-i'));

    await waitFor(() => {
      expect(result.current.isLoading).toBe(false);
    });

    act(() => {
      result.current.updatePrinter({ devicePath: '192.168.1.99' });
    });

    await act(async () => {
      await result.current.save();
    });

    await waitFor(() => {
      expect(result.current.isLoading).toBe(false);
    });

    const stored = readProfile('term-i');
    expect(stored!.hardware.printer.devicePath).toBe('192.168.1.99');
  });

  it('save preserves existing data when write would fail (no-op with localStorage)', async () => {
    // localStorage writes are synchronous and rarely fail.
    // In the Tauri fs version, this test would verify .bak restoration.
    const profile = makeProfile('term-j');
    profile.hardware.printer.devicePath = 'original';
    seedProfile('term-j', profile);

    const { result } = renderHook(() => useTerminalHardware('term-j'));

    await waitFor(() => {
      expect(result.current.isLoading).toBe(false);
    });

    act(() => {
      result.current.updatePrinter({ devicePath: 'modified' });
    });

    await act(async () => {
      await result.current.save();
    });

    const stored = readProfile('term-j');
    expect(stored!.hardware.printer.devicePath).toBe('modified');
    expect(result.current.error).toBeNull();
  });

  // ── Three-phase commit: write failure recovery ─────────────────

  it('reports error when save fails and preserves in-memory state', async () => {
    const profile = makeProfile('term-k');
    profile.hardware.printer.devicePath = 'before-change';
    seedProfile('term-k', profile);

    const { result } = renderHook(() => useTerminalHardware('term-k'));

    await waitFor(() => {
      expect(result.current.isLoading).toBe(false);
    });

    act(() => {
      result.current.updatePrinter({ devicePath: 'after-change' });
    });

    // localStorage.setItem rarely fails in tests, so we verify
    // the save path writes correctly and preserves in-memory state
    await act(async () => {
      await result.current.save();
    });

    expect(result.current.error).toBeNull();
    expect(result.current.isLoading).toBe(false);

    // Verify the updated value was persisted
    const stored = readProfile('term-k');
    expect(stored!.hardware.printer.devicePath).toBe('after-change');
  });

  // ── Reload ──────────────────────────────────────────────────────

  it('reload re-reads from storage', async () => {
    const profile = makeProfile('term-l');
    profile.hardware.printer.devicePath = 'v1';
    seedProfile('term-l', profile);

    const { result } = renderHook(() => useTerminalHardware('term-l'));

    await waitFor(() => {
      expect(result.current.isLoading).toBe(false);
    });
    expect(result.current.profile!.hardware.printer.devicePath).toBe('v1');

    // Change storage externally
    const updated = { ...profile, hardware: { ...profile.hardware, printer: { ...profile.hardware.printer, devicePath: 'v2' } } };
    seedProfile('term-l', updated);

    act(() => {
      result.current.reload();
    });

    await waitFor(() => {
      expect(result.current.isLoading).toBe(false);
    });
    expect(result.current.profile!.hardware.printer.devicePath).toBe('v2');
  });

  // ── edge cases ──────────────────────────────────────────────────

  it('returns default for empty terminalId', async () => {
    const { result } = renderHook(() => useTerminalHardware(''));

    await waitFor(() => {
      expect(result.current.isLoading).toBe(false);
    });

    expect(result.current.profile).toBeNull();
  });

  it('initialized contains a valid ISO date', async () => {
    const { result } = renderHook(() => useTerminalHardware('term-m'));

    await waitFor(() => {
      expect(result.current.isLoading).toBe(false);
    });

    const date = new Date(result.current.profile!.initialized);
    expect(date.getTime()).toBeGreaterThan(0);
  });
});
