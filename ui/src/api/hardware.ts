// ── Hardware: Barcode scanner, cash drawer, printer ──────────────

import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';

// ── Cash Drawer ──────────────────────────────────────────────────

export interface OpenCashDrawerArgs {
  deviceId?: string;
}

export interface OpenCashDrawerResult {
  opened: boolean;
}

export const openCashDrawer = (args: OpenCashDrawerArgs = {}): Promise<OpenCashDrawerResult> =>
  invoke<OpenCashDrawerResult>('open_cash_drawer', { args });

// ── Receipt Printing (raw) ───────────────────────────────────────

export interface PrintReceiptArgs {
  body: string;
}

export interface PrintReceiptResult {
  printedLines: number;
}

export const printReceipt = (args: PrintReceiptArgs): Promise<PrintReceiptResult> =>
  invoke<PrintReceiptResult>('print_receipt', { args });

// ── Barcode Scanner ──────────────────────────────────────────────

/**
 * Error thrown by scanner-related operations when the scanner hardware
 * encounters a recoverable or unrecoverable failure.
 *
 * The `code` field identifies the specific type of failure so callers
 * can surface appropriate diagnostics or recovery prompts.
 */
export class ScannerError extends Error {
  /**
   * @param message  Human-readable description of the failure.
   * @param code     Machine-readable error code (see `ScannerError.codes`).
   * @param scannerId  Optional id of the scanner that failed.
   */
  constructor(
    message: string,
    public readonly code: string,
    public readonly scannerId?: string,
  ) {
    super(message);
    this.name = 'ScannerError';
  }

  /** Canonical error codes emitted by the scanner subsystem. */
  static codes = {
    /** Scanner was physically disconnected mid-operation. */
    DISCONNECTED: 'SCANNER_DISCONNECTED',
    /** Scanner did not respond within the expected time frame. */
    TIMEOUT: 'SCANNER_TIMEOUT',
    /** Generic hardware failure (e.g. USB error, power issue). */
    HARDWARE_FAILURE: 'SCANNER_HARDWARE_FAILURE',
    /** Scanner is already claimed by another process. */
    CONFLICT: 'SCANNER_CONFLICT',
  } as const;
}

export interface ScannerInfo {
  id: string;
}

export interface BarcodeScannedPayload {
  code: string;
  symbology: string;
}

export const listScanners = (): Promise<ScannerInfo[]> =>
  invoke<ScannerInfo[]>('list_scanners');

export const startScanner = (scannerId: string): Promise<void> =>
  invoke('start_scanner', { scannerId });

export const stopScanner = (): Promise<void> => invoke('stop_scanner');

export const onBarcodeScanned = (handler: (payload: BarcodeScannedPayload) => void): Promise<UnlistenFn> =>
  listen<BarcodeScannedPayload>('barcode:scanned', (e) => handler(e.payload));

export const onBarcodeError = (handler: (error: string) => void): Promise<UnlistenFn> =>
  listen<{ error: string }>('barcode:error', (e) => handler(e.payload.error));

// ── Customer Display ──────────────────────────────────────────────

export interface DisplayShowArgs {
  displayId: string;
  line1: string;
  line2: string;
}

/** List all registered customer-facing pole displays. */
export const listDisplays = (): Promise<string[]> =>
  invoke<string[]>('list_displays');

/** Show content on a customer-facing pole display. */
export const displayShow = (args: DisplayShowArgs): Promise<void> =>
  invoke('display_show', { args });

/** Clear a customer-facing pole display. */
export const displayClear = (displayId: string): Promise<void> =>
  invoke('display_clear', { displayId });
