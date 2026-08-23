// ── ui/src/features/warehouse/useWarehouseScanner.ts ─────────────────────
// Barcode scanner integration for the warehouse console.
// Self-contained copy of sales/useBarcodeScanner.ts — same mechanics,
// no shared imports. The warehouse resolves barcodes against products
// and feeds them into the warehouse session.

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

export interface UseWarehouseScannerOptions {
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
 * auto-lookup the product by barcode. Starts on mount, stops on unmount.
 */
export function useWarehouseScanner({
  scannerId: preferredId,
  onProductFound,
  onProductNotFound,
  onError,
}: UseWarehouseScannerOptions) {
  const startedRef = useRef(false);

  const onProductFoundRef = useRef(onProductFound);
  onProductFoundRef.current = onProductFound;
  const onProductNotFoundRef = useRef(onProductNotFound);
  onProductNotFoundRef.current = onProductNotFound;
  const onErrorRef = useRef(onError);
  onErrorRef.current = onError;

  useEffect(() => {
    let cancelled = false;

    (async () => {
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

  const handleScan = useCallback(async (payload: BarcodeScannedPayload) => {
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
  }, []);

  const handleError = useCallback((error: string) => {
    onErrorRef.current?.(error);
  }, []);

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