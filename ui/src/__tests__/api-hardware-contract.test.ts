// ── IPC contract tests for hardware.ts ─────────────────────────
//
// Verifies every exported function calls loggedInvoke with the
// correct IPC command name and argument shape.

import { describe, it, expect, vi, beforeEach } from 'vitest';

const { mockInvoke } = vi.hoisted(() => ({
  mockInvoke: vi.fn(),
}));

vi.mock('@/utils/logged-invoke', () => ({
  loggedInvoke: (cmd: string, args?: Record<string, unknown>) => mockInvoke(cmd, args),
}));

vi.mock('@tauri-apps/api/event', () => ({
  listen: vi.fn(),
}));

import {
  openCashDrawer,
  printReceipt,
  printSalesReceiptScoped,
  listScanners,
  startScanner,
  stopScanner,
  listDisplays,
  displayShow,
  displayClear,
  readScaleWeight,
  discoverHardware,
} from '@/api/hardware';

describe('hardware.ts IPC contract', () => {
  beforeEach(() => mockInvoke.mockReset());

  // ── Cash Drawer ───────────────────────────────────────────

  it('openCashDrawer → open_cash_drawer with default empty args', async () => {
    mockInvoke.mockResolvedValue({ opened: true });
    await openCashDrawer();
    expect(mockInvoke).toHaveBeenCalledWith('open_cash_drawer', { args: {} });
  });

  it('openCashDrawer with deviceId → open_cash_drawer with args.deviceId', async () => {
    mockInvoke.mockResolvedValue({ opened: true });
    await openCashDrawer({ deviceId: 'drawer-1' });
    expect(mockInvoke).toHaveBeenCalledWith('open_cash_drawer', { args: { deviceId: 'drawer-1' } });
  });

  // ── Receipt Printing ──────────────────────────────────────

  it('printReceipt → print_receipt with body', async () => {
    mockInvoke.mockResolvedValue({ printedLines: 5 });
    await printReceipt({ body: 'Hello World' });
    expect(mockInvoke).toHaveBeenCalledWith('print_receipt', { args: { body: 'Hello World' } });
  });

  it('printSalesReceiptScoped → print_sales_receipt_scoped with sessionToken', async () => {
    mockInvoke.mockResolvedValue({ printed: true });
    await printSalesReceiptScoped('tok', { date: '2026-08-19', receiptNumber: 'R1', items: [], subtotal: { minorUnits: 0, currency: 'USD' }, total: { minorUnits: 0, currency: 'USD' }, payments: [] });
    expect(mockInvoke).toHaveBeenCalledWith('print_sales_receipt_scoped', { sessionToken: 'tok', args: expect.objectContaining({ receiptNumber: 'R1' }) });
  });

  // ── Barcode Scanner ───────────────────────────────────────

  it('listScanners → list_scanners (no args)', async () => {
    mockInvoke.mockResolvedValue([]);
    await listScanners();
    expect(mockInvoke).toHaveBeenCalledWith('list_scanners', undefined);
  });

  it('startScanner → start_scanner with scannerId', async () => {
    mockInvoke.mockResolvedValue(undefined);
    await startScanner('scanner-1');
    expect(mockInvoke).toHaveBeenCalledWith('start_scanner', { scannerId: 'scanner-1' });
  });

  it('stopScanner → stop_scanner (no args)', async () => {
    mockInvoke.mockResolvedValue(undefined);
    await stopScanner();
    expect(mockInvoke).toHaveBeenCalledWith('stop_scanner', undefined);
  });

  // ── Customer Display ──────────────────────────────────────

  it('listDisplays → list_displays (no args)', async () => {
    mockInvoke.mockResolvedValue([]);
    await listDisplays();
    expect(mockInvoke).toHaveBeenCalledWith('list_displays', undefined);
  });

  it('displayShow → display_show with args', async () => {
    mockInvoke.mockResolvedValue(undefined);
    await displayShow({ displayId: 'd1', line1: 'Total', line2: '$10.00' });
    expect(mockInvoke).toHaveBeenCalledWith('display_show', { args: { displayId: 'd1', line1: 'Total', line2: '$10.00' } });
  });

  it('displayClear → display_clear with displayId', async () => {
    mockInvoke.mockResolvedValue(undefined);
    await displayClear('d1');
    expect(mockInvoke).toHaveBeenCalledWith('display_clear', { displayId: 'd1' });
  });

  // ── Weight Scale ──────────────────────────────────────────

  it('readScaleWeight → read_scale_weight (no args)', async () => {
    mockInvoke.mockResolvedValue({ weightGrams: 500, stable: true });
    await readScaleWeight();
    expect(mockInvoke).toHaveBeenCalledWith('read_scale_weight', undefined);
  });

  // ── Device Discovery ──────────────────────────────────────

  it('discoverHardware → discover_hardware (no args)', async () => {
    mockInvoke.mockResolvedValue([]);
    await discoverHardware();
    expect(mockInvoke).toHaveBeenCalledWith('discover_hardware', undefined);
  });

  // ── Error propagation ─────────────────────────────────────

  it('propagates backend errors', async () => {
    mockInvoke.mockRejectedValueOnce(new Error('device not found'));
    await expect(listScanners()).rejects.toThrow('device not found');
  });
});
