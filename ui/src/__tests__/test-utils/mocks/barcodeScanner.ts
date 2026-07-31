// ── Shared useBarcodeScanner mock ────────────────────────────────
//
// Captures the onProductFound / onError callbacks passed to the real
// `useBarcodeScanner` hook so tests can drive simulated scans. The
// `mockedBarcode` singleton was previously duplicated (with slight
// variations) in 5 test files:
//
//   - RetailPosScreen.test.tsx
//   - RetailPosScreenInteractions.test.tsx
//   - RetailPosScreenCheckout.test.tsx
//   - PosScreen.test.tsx
//   - PosScreenDeductionLocation.test.tsx
//
// Usage:
//   import { mockedBarcode } from '@/__tests__/test-utils/mocks/barcodeScanner';
//
//   // In a vi.mock block (async import to avoid hoisting conflicts):
//   vi.mock('@/features/sales/useBarcodeScanner', async () => {
//     const { createBarcodeScannerModuleMock } =
//       await import('@/__tests__/test-utils/mocks/barcodeScanner');
//     return createBarcodeScannerModuleMock();
//   });
//
//   // beforeEach:
//   mockedBarcode.reset();
//
//   // In a test:
//   act(() => { mockedBarcode.triggerScan('8991002100110'); });

import { vi } from 'vitest';
import type { BarcodeScannedPayload } from '@/api/hardware';

export interface BarcodeScannerMockCallbacks {
  onProductFound: (payload: BarcodeScannedPayload) => void;
  onError?: (error: string) => void;
}

// Module-level callback slots — shared between the hoisted mock fn and
// the trigger helpers (same module instance, so the same closure).
let onProductFound: ((payload: BarcodeScannedPayload) => void) | null = null;
let onError: ((error: string) => void) | null = null;

/** Singleton driving simulated scans through the mocked hook. */
export const mockedBarcode = {
  /** Fire the captured onProductFound callback with a scan payload. */
  triggerScan(code: string) {
    onProductFound?.({ code, symbology: 'test' });
  },
  /** Fire the captured onError callback (scanner hardware error). */
  triggerError(error: string) {
    onError?.(error);
  },
  /** Drop captured callbacks so state doesn't leak between tests. */
  reset() {
    onProductFound = null;
    onError = null;
  },
  useBarcodeScanner: vi.fn((opts: BarcodeScannerMockCallbacks) => {
    onProductFound = opts.onProductFound;
    onError = opts.onError ?? null;
  }),
};

/** Module shape for `vi.mock('@/features/sales/useBarcodeScanner', …)`. */
export function createBarcodeScannerModuleMock() {
  return { useBarcodeScanner: mockedBarcode.useBarcodeScanner };
}
