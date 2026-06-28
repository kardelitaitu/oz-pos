import { useEffect, useRef, useCallback } from 'react';
import {
  startScanner,
  stopScanner,
  onBarcodeScanned,
  onBarcodeError,
  lookupByBarcode,
  type BarcodeScannedPayload,
} from '@/api/pos';

export interface UseBarcodeScannerOptions {
  /** Scanner device id. Defaults to auto-select first available. */
  scannerId?: string;
  /** Called when a barcode is decoded and the product is found. */
  onProductFound: (payload: BarcodeScannedPayload) => void;
  /** Called when a barcode is decoded but no product matches. */
  onProductNotFound?: (code: string) => void;
  /** Called on scanner errors. */
  onError?: (error: string) => void;
}

/**
 * Subscribe to `barcode:scanned` events from the Tauri backend and
 * auto-lookup the product by barcode.
 *
 * Starts the scanner on mount and stops it on unmount.
 */
export function useBarcodeScanner({
  scannerId: preferredId,
  onProductFound,
  onProductNotFound,
  onError,
}: UseBarcodeScannerOptions) {
  const startedRef = useRef(false);

  useEffect(() => {
    let cancelled = false;

    (async () => {
      // Auto-detect scanner if no id was given.
      const scannerId = preferredId ?? (await autoDetectScanner());

      if (!scannerId || cancelled) return;

      await startScanner(scannerId);
      startedRef.current = true;
    })();

    return () => {
      cancelled = true;
      if (startedRef.current) {
        stopScanner().catch(() => {});
        startedRef.current = false;
      }
    };
  }, [preferredId]);

  const handleScan = useCallback(
    async (payload: BarcodeScannedPayload) => {
      try {
        const product = await lookupByBarcode(payload.code);
        if (product) {
          onProductFound(payload);
        } else {
          onProductNotFound?.(payload.code);
        }
      } catch {
        onProductNotFound?.(payload.code);
      }
    },
    [onProductFound, onProductNotFound],
  );

  const handleError = useCallback(
    (error: string) => {
      onError?.(error);
    },
    [onError],
  );

  // Subscribe to barcode events while mounted.
  useEffect(() => {
    const unsubScan = onBarcodeScanned(handleScan);
    const unsubErr = onBarcodeError(handleError);
    return () => {
      unsubScan.then((fn) => fn());
      unsubErr.then((fn) => fn());
    };
  }, [handleScan, handleError]);
}

async function autoDetectScanner(): Promise<string | null> {
  try {
    const scanners = await import('@/api/pos').then((m) => m.listScanners());
    return scanners[0]?.id ?? null;
  } catch {
    return null;
  }
}
