import { useEffect, useRef, useCallback } from 'react';
import {
  startScanner,
  stopScanner,
  onBarcodeScanned,
  onBarcodeError,
  listScanners,
  type BarcodeScannedPayload,
} from '@/api/hardware';
import { lookupByBarcode } from '@/api/products';

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

  // Keep callbacks in refs so the event subscription doesn't re-register
  // every time the parent passes a fresh inline callback (e.g. on every
  // cart change in RetailPosScreen). Matches the ref pattern used by
  // useFocusTrap and useExitAnimation.
  const onProductFoundRef = useRef(onProductFound);
  onProductFoundRef.current = onProductFound;
  const onProductNotFoundRef = useRef(onProductNotFound);
  onProductNotFoundRef.current = onProductNotFound;
  const onErrorRef = useRef(onError);
  onErrorRef.current = onError;

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
        stopScanner().catch(() => {
          // Cleanup on unmount — scanner may already be stopped.
        });
        startedRef.current = false;
      }
    };
  }, [preferredId]);

  const handleScan = useCallback(
    async (payload: BarcodeScannedPayload) => {
      try {
        const product = await lookupByBarcode(payload.code);
        if (product) {
          onProductFoundRef.current(payload);
        } else {
          onProductNotFoundRef.current?.(payload.code);
        }
      } catch {
        onProductNotFoundRef.current?.(payload.code);
      }
    },
    [], // stable — reads latest callbacks via refs
  );

  const handleError = useCallback(
    (error: string) => {
      onErrorRef.current?.(error);
    },
    [], // stable — reads latest callback via ref
  );

  // Subscribe to barcode events once on mount — stable callbacks.
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
    const scanners = await listScanners();
    return scanners[0]?.id ?? null;
  } catch {
    return null;
  }
}
