import { describe, it, expect, beforeEach, vi } from 'vitest';
import { renderHook, act, waitFor } from '@testing-library/react';
import {
  useTerminalHardware,
} from '@/hooks/useTerminalHardware';

// ── Mock @/api/settings with configurable IPC responses ─────────────

const mockGetHardwareSettings = vi.fn();
const mockSetHardwareSettings = vi.fn();

vi.mock('@/api/settings', () => ({
  getHardwareSettings: () => mockGetHardwareSettings(),
  setHardwareSettings: (...args: unknown[]) => mockSetHardwareSettings(...args),
}));

const defaultDto = {
  printerConnection: 'auto',
  printerDevicePath: '',
  printerPaperSize: '80',
  scannerDeviceId: '',
  scannerInputMode: 'auto',
} as const;

// ── Tests ─────────────────────────────────────────────────────────

describe('useTerminalHardware', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockGetHardwareSettings.mockResolvedValue(defaultDto);
    mockSetHardwareSettings.mockResolvedValue(undefined);
  });

  // ── Initial load ──────────────────────────────────────────────

  it('loads profile from IPC on mount', async () => {
    mockGetHardwareSettings.mockResolvedValue({
      ...defaultDto,
      printerConnection: 'network',
      printerDevicePath: '192.168.1.50',
    });

    const { result } = renderHook(() => useTerminalHardware('term-001'));

    await waitFor(() => {
      expect(result.current.isLoading).toBe(false);
    });

    expect(result.current.profile).not.toBeNull();
    expect(result.current.profile!.terminalId).toBe('term-001');
    expect(result.current.profile!.hardware.printer.connection).toBe('network');
    expect(result.current.profile!.hardware.printer.devicePath).toBe('192.168.1.50');
    // Fields not in IPC DTO get defaults
    expect(result.current.profile!.hardware.scale.connection).toBe('none');
    expect(result.current.profile!.localPrefs.soundVolume).toBe(80);
  });

  it('loads default profile when IPC fails', async () => {
    mockGetHardwareSettings.mockRejectedValue(new Error('IPC unavailable'));

    const { result } = renderHook(() => useTerminalHardware('term-002'));

    await waitFor(() => {
      expect(result.current.isLoading).toBe(false);
    });

    expect(result.current.profile).not.toBeNull();
    expect(result.current.profile!.terminalId).toBe('term-002');
    expect(result.current.profile!.hardware.printer.connection).toBe('auto');
    expect(result.current.profile!.hardware.scanner.mode).toBe('auto');
  });

  it('returns null for empty terminalId', async () => {
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

  // ── Update helpers (local state) ──────────────────────────────

  it('updatePrinter modifies local state without persisting', async () => {
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
    expect(mockSetHardwareSettings).not.toHaveBeenCalled();
  });

  it('updateScale modifies local state', async () => {
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

  // ── Save (persist to IPC) ─────────────────────────────────────

  it('save calls setHardwareSettings with DTO subset', async () => {
    const { result } = renderHook(() => useTerminalHardware('term-i'));

    await waitFor(() => {
      expect(result.current.isLoading).toBe(false);
    });

    act(() => {
      result.current.updatePrinter({ devicePath: '192.168.1.99' });
    });

    await act(async () => {
      await result.current.save('user-1');
    });

    await waitFor(() => {
      expect(result.current.isLoading).toBe(false);
    });

    expect(mockSetHardwareSettings).toHaveBeenCalledTimes(1);
    const call = mockSetHardwareSettings.mock.calls[0] as [Record<string, unknown>, string];
    const dto = call[0];
    const userId = call[1];
    expect(dto['printerDevicePath']).toBe('192.168.1.99');
    expect(dto['printerConnection']).toBe('auto');
    expect(userId).toBe('user-1');
    expect(result.current.error).toBeNull();
  });

  it('save reports error on IPC failure', async () => {
    mockSetHardwareSettings.mockRejectedValue(new Error('Disk full'));

    const { result } = renderHook(() => useTerminalHardware('term-k'));

    await waitFor(() => {
      expect(result.current.isLoading).toBe(false);
    });

    act(() => {
      result.current.updatePrinter({ devicePath: 'after-change' });
    });

    await act(async () => {
      await result.current.save();
    });

    await waitFor(() => {
      expect(result.current.isLoading).toBe(false);
    });

    expect(result.current.error).toBe('Disk full');
  });

  // ── Reload ──────────────────────────────────────────────────────

  it('reload re-reads from IPC', async () => {
    mockGetHardwareSettings.mockResolvedValue({
      ...defaultDto,
      printerDevicePath: 'v1',
    });

    const { result } = renderHook(() => useTerminalHardware('term-l'));

    await waitFor(() => {
      expect(result.current.isLoading).toBe(false);
    });
    expect(result.current.profile!.hardware.printer.devicePath).toBe('v1');

    // Change IPC response
    mockGetHardwareSettings.mockResolvedValue({
      ...defaultDto,
      printerDevicePath: 'v2',
    });

    act(() => {
      result.current.reload();
    });

    await waitFor(() => {
      expect(result.current.isLoading).toBe(false);
    });
    expect(result.current.profile!.hardware.printer.devicePath).toBe('v2');
  });

  // ── Edge cases ──────────────────────────────────────────────────

  it('save is no-op when profile is null', async () => {
    const { result } = renderHook(() => useTerminalHardware(''));

    await waitFor(() => {
      expect(result.current.isLoading).toBe(false);
    });

    await act(async () => {
      await result.current.save();
    });

    expect(mockSetHardwareSettings).not.toHaveBeenCalled();
  });
});
