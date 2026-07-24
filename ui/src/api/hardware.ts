// ── Hardware: Barcode scanner, cash drawer, printer ──────────────

import { loggedInvoke } from '@/utils/logged-invoke';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';

// ── Cash Drawer ──────────────────────────────────────────────────

/** Arguments for opening a cash drawer. */
export interface OpenCashDrawerArgs {
  deviceId?: string;
}

/** Result of attempting to open a cash drawer. */
export interface OpenCashDrawerResult {
  opened: boolean;
}

/** Open a cash drawer. */
export const openCashDrawer = (args: OpenCashDrawerArgs = {}): Promise<OpenCashDrawerResult> =>
  loggedInvoke<OpenCashDrawerResult>('open_cash_drawer', { args });

// ── Receipt Printing (raw) ───────────────────────────────────────

/** Arguments for printing a raw receipt. */
export interface PrintReceiptArgs {
  body: string;
}

/** Result of printing a raw receipt. */
export interface PrintReceiptResult {
  printedLines: number;
}

/** Print a raw text receipt on the configured printer. */
export const printReceipt = (args: PrintReceiptArgs): Promise<PrintReceiptResult> =>
  loggedInvoke<PrintReceiptResult>('print_receipt', { args });

/**
 * Arguments for printing a structured sales receipt.
 * @deprecated Prefer using `printSalesReceiptScoped` in multi-store deployments.
 */
export interface PrintSalesReceiptArgs {
  date: string;
  receiptNumber: string;
  items: {
    name: string;
    quantity: number;
    unitPrice: { minorUnits: number; currency: string };
    totalPrice: { minorUnits: number; currency: string };
    taxAmount?: { minorUnits: number; currency: string } | null;
  }[];
  subtotal: { minorUnits: number; currency: string };
  tax?: { minorUnits: number; currency: string } | null;
  total: { minorUnits: number; currency: string };
  payments: {
    method: string;
    amount: { minorUnits: number; currency: string };
    change?: { minorUnits: number; currency: string } | null;
  }[];
  tableNumber?: string | null;
}

/** Print a structured sales receipt (scoped — ADR #7). */
export const printSalesReceiptScoped = (
  sessionToken: string,
  args: PrintSalesReceiptArgs,
): Promise<{ printed: boolean }> =>
  loggedInvoke<{ printed: boolean }>('print_sales_receipt_scoped', { sessionToken, args });

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

/** Information about a connected barcode scanner. */
export interface ScannerInfo {
  id: string;
}

/** Payload delivered when a barcode is scanned. */
export interface BarcodeScannedPayload {
  code: string;
  symbology: string;
}

/** List all connected barcode scanners. */
export const listScanners = (): Promise<ScannerInfo[]> =>
  loggedInvoke<ScannerInfo[]>('list_scanners');

/** Start listening for barcode scans on a specific scanner. */
export const startScanner = (scannerId: string): Promise<void> =>
  loggedInvoke('start_scanner', { scannerId });

/** Stop listening for barcode scans. */
export const stopScanner = (): Promise<void> => loggedInvoke('stop_scanner');

/** Subscribe to barcode-scanned events. Returns an unsubscribe function. */
export const onBarcodeScanned = (handler: (payload: BarcodeScannedPayload) => void): Promise<UnlistenFn> =>
  listen<BarcodeScannedPayload>('barcode:scanned', (e) => handler(e.payload));

/** Subscribe to barcode scanner error events. Returns an unsubscribe function. */
export const onBarcodeError = (handler: (error: string) => void): Promise<UnlistenFn> =>
  listen<{ error: string }>('barcode:error', (e) => handler(e.payload.error));

// ── Customer Display ──────────────────────────────────────────────

/** Arguments for showing content on a customer-facing display. */
export interface DisplayShowArgs {
  displayId: string;
  line1: string;
  line2: string;
}

/** List all registered customer-facing pole displays. */
export const listDisplays = (): Promise<string[]> =>
  loggedInvoke<string[]>('list_displays');

/** Show content on a customer-facing pole display. */
export const displayShow = (args: DisplayShowArgs): Promise<void> =>
  loggedInvoke('display_show', { args });

/** Clear a customer-facing pole display. */
export const displayClear = (displayId: string): Promise<void> =>
  loggedInvoke('display_clear', { displayId });

// ── Weight Scale ────────────────────────────────────────────────────

/** A weight reading from a connected scale. */
export interface WeightReading {
  weightGrams: number;
  stable: boolean;
}

/** Read the current weight from the registered scale, or null if none is registered. */
export const readScaleWeight = (): Promise<WeightReading | null> =>
  loggedInvoke<WeightReading | null>('read_scale_weight');

// ── Device Discovery ──────────────────────────────────────────────────

/** Categories of USB hardware devices that can be discovered. */
export type DeviceCategory = 'Scanner' | 'Printer' | 'Scale' | 'Other';

/** Information about a discovered USB hardware device. */
export interface UsbDeviceInfo {
  vid: number;
  pid: number;
  manufacturer: string;
  product: string;
  serial: string;
  interfaceNumber: number;
  endpointIn: number;
  endpointOut: number | null;
  category: DeviceCategory;
  label: string;
}

/** Discover all connected USB hardware devices (scanners, printers, scales). */
export const discoverHardware = (): Promise<UsbDeviceInfo[]> =>
  loggedInvoke<UsbDeviceInfo[]>('discover_hardware');
