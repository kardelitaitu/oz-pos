import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { renderHookInAct } from '@/test-utils/renderInAct';
import { useBarcodeScanner } from '@/features/sales/useBarcodeScanner';

const mocks = vi.hoisted(() => ({
  startScanner: vi.fn(),
  stopScanner: vi.fn(),
  onBarcodeScanned: vi.fn(),
  onBarcodeError: vi.fn(),
  listScanners: vi.fn(),
  lookupByBarcode: vi.fn(),
}));

vi.mock('@/api/hardware', () => ({
  startScanner: (...args: unknown[]) => mocks.startScanner(...args),
  stopScanner: (...args: unknown[]) => mocks.stopScanner(...args),
  onBarcodeScanned: (...args: unknown[]) => mocks.onBarcodeScanned(...args),
  onBarcodeError: (...args: unknown[]) => mocks.onBarcodeError(...args),
  listScanners: (...args: unknown[]) => mocks.listScanners(...args),
}));

vi.mock('@/api/products', () => ({
  lookupByBarcode: (...args: unknown[]) => mocks.lookupByBarcode(...args),
}));

function makeOpts(overrides: Record<string, unknown> = {}) {
  return {
    onProductFound: vi.fn(),
    onProductNotFound: vi.fn(),
    onError: vi.fn(),
    ...overrides,
  };
}

beforeEach(() => {
  mocks.startScanner.mockResolvedValue(undefined);
  mocks.stopScanner.mockResolvedValue(undefined);
  mocks.listScanners.mockResolvedValue([{ id: 'scanner-1' }]);
  mocks.onBarcodeScanned.mockResolvedValue(() => {});
  mocks.onBarcodeError.mockResolvedValue(() => {});
  mocks.lookupByBarcode.mockResolvedValue({ sku: 'LATTE', name: 'Latte' });
});

afterEach(() => {
  vi.clearAllMocks();
  vi.useRealTimers();
});

describe('useBarcodeScanner', () => {
  describe('scanner lifecycle', () => {
    it('auto-detects and starts the first available scanner on mount', async () => {
      await renderHookInAct(() => useBarcodeScanner(makeOpts()));

      expect(mocks.listScanners).toHaveBeenCalled();
      expect(mocks.startScanner).toHaveBeenCalledWith('scanner-1');
    });

    it('uses the provided scannerId instead of auto-detecting', async () => {
      await renderHookInAct(() => useBarcodeScanner(makeOpts({ scannerId: 'my-scanner' })));

      expect(mocks.listScanners).not.toHaveBeenCalled();
      expect(mocks.startScanner).toHaveBeenCalledWith('my-scanner');
    });

    it('does not start scanner when auto-detect returns no scanners', async () => {
      mocks.listScanners.mockResolvedValue([]);

      await renderHookInAct(() => useBarcodeScanner(makeOpts()));

      expect(mocks.startScanner).not.toHaveBeenCalled();
    });

    it('does not start scanner when auto-detect throws', async () => {
      mocks.listScanners.mockRejectedValue(new Error('no backend'));

      await renderHookInAct(() => useBarcodeScanner(makeOpts()));

      expect(mocks.startScanner).not.toHaveBeenCalled();
    });

    it('stops the scanner on unmount when it was started', async () => {
      const { unmount } = await renderHookInAct(() => useBarcodeScanner(makeOpts()));

      unmount();

      expect(mocks.stopScanner).toHaveBeenCalled();
    });

    it('does not stop the scanner on unmount when it was never started', async () => {
      mocks.listScanners.mockResolvedValue([]);
      const { unmount } = await renderHookInAct(() => useBarcodeScanner(makeOpts()));

      unmount();

      expect(mocks.stopScanner).not.toHaveBeenCalled();
    });
  });

  describe('event subscriptions', () => {
    it('subscribes to barcode:scanned on mount', async () => {
      await renderHookInAct(() => useBarcodeScanner(makeOpts()));

      expect(mocks.onBarcodeScanned).toHaveBeenCalled();
    });

    it('subscribes to barcode:error on mount', async () => {
      await renderHookInAct(() => useBarcodeScanner(makeOpts()));

      expect(mocks.onBarcodeError).toHaveBeenCalled();
    });

    it('unsubscribes from both events on unmount', async () => {
      const unsubScan = vi.fn();
      const unsubErr = vi.fn();
      mocks.onBarcodeScanned.mockResolvedValue(unsubScan);
      mocks.onBarcodeError.mockResolvedValue(unsubErr);

      const { unmount } = await renderHookInAct(() => useBarcodeScanner(makeOpts()));

      unmount();

      expect(unsubScan).toHaveBeenCalled();
      expect(unsubErr).toHaveBeenCalled();
    });
  });

  describe('handleScan callback', () => {
    it('calls onProductFound when barcode matches a product', async () => {
      const onProductFound = vi.fn();
      const payload = { code: '4901234567890', symbology: 'ean13' };
      mocks.lookupByBarcode.mockResolvedValue({ sku: 'LATTE', name: 'Latte' });

      await renderHookInAct(() => useBarcodeScanner(makeOpts({ onProductFound })));

      const scanHandler = mocks.onBarcodeScanned.mock.calls[0][0];
      await scanHandler(payload);

      expect(onProductFound).toHaveBeenCalledWith(payload);
    });

    it('calls onProductNotFound when barcode matches no product', async () => {
      const onProductNotFound = vi.fn();
      const payload = { code: '0000000000000', symbology: 'ean13' };
      mocks.lookupByBarcode.mockResolvedValue(null);

      await renderHookInAct(() => useBarcodeScanner(makeOpts({ onProductNotFound })));

      const scanHandler = mocks.onBarcodeScanned.mock.calls[0][0];
      await scanHandler(payload);

      expect(onProductNotFound).toHaveBeenCalledWith('0000000000000');
    });

    it('calls onProductNotFound when lookup throws', async () => {
      const onProductNotFound = vi.fn();
      const payload = { code: '0000000000000', symbology: 'ean13' };
      mocks.lookupByBarcode.mockRejectedValue(new Error('db error'));

      await renderHookInAct(() => useBarcodeScanner(makeOpts({ onProductNotFound })));

      const scanHandler = mocks.onBarcodeScanned.mock.calls[0][0];
      await scanHandler(payload);

      expect(onProductNotFound).toHaveBeenCalledWith('0000000000000');
    });

    it('calls onProductNotFound when lookup returns a dto with null sku', async () => {
      const onProductNotFound = vi.fn();
      const payload = { code: '0000000000000', symbology: 'ean13' };
      mocks.lookupByBarcode.mockResolvedValue(null);

      await renderHookInAct(() => useBarcodeScanner(makeOpts({ onProductNotFound })));

      const scanHandler = mocks.onBarcodeScanned.mock.calls[0][0];
      await scanHandler(payload);

      expect(onProductNotFound).toHaveBeenCalledWith('0000000000000');
    });

    it('does not call onProductNotFound when handler is not provided', async () => {
      const payload = { code: '0000000000000', symbology: 'ean13' };
      mocks.lookupByBarcode.mockResolvedValue(null);

      await renderHookInAct(() => useBarcodeScanner(makeOpts({ onProductNotFound: undefined })));

      const scanHandler = mocks.onBarcodeScanned.mock.calls[0][0];

      await expect(scanHandler(payload)).resolves.toBeUndefined();
    });
  });

  describe('handleError callback', () => {
    it('calls onError when a scanner error is received', async () => {
      const onError = vi.fn();

      await renderHookInAct(() => useBarcodeScanner(makeOpts({ onError })));

      const errorHandler = mocks.onBarcodeError.mock.calls[0][0];
      errorHandler('scanner disconnected');

      expect(onError).toHaveBeenCalledWith('scanner disconnected');
    });

    it('does not throw when onError is not provided', async () => {
      await renderHookInAct(() => useBarcodeScanner(makeOpts({ onError: undefined })));

      const errorHandler = mocks.onBarcodeError.mock.calls[0][0];

      expect(() => errorHandler('some error')).not.toThrow();
    });
  });

  describe('idempotency', () => {
    it('re-starts scanner when preferredId changes', async () => {
      mocks.startScanner.mockClear();
      const { rerender } = await renderHookInAct(
        ({ scannerId }: { scannerId?: string }) => useBarcodeScanner(makeOpts({ scannerId })),
        { initialProps: { scannerId: 'scanner-1' } },
      );

      mocks.startScanner.mockClear();
      rerender({ scannerId: 'scanner-2' });

      expect(mocks.startScanner).toHaveBeenCalledWith('scanner-2');
      await vi.waitFor(() => {});
    });
  });
});
