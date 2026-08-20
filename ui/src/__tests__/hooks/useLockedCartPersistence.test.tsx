import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { renderHook, act } from '@testing-library/react';
import { useLockedCartPersistence } from '@/features/sales/posScreenHooks';
import type { CartLine, LineId } from '@/types/domain';

describe('useLockedCartPersistence', () => {
  const mockSetLines = vi.fn();
  const mockSetDiscount = vi.fn();
  const mockSetTipPercent = vi.fn();
  const mockSetServiceCharge = vi.fn();
  const mockLogout = vi.fn();

  const sampleLines: CartLine[] = [
    { id: 'line-1' as LineId, sku: 'COFFEE' as any, name: 'Coffee', qty: 2, unit_price: { minor_units: 350, currency: 'USD' }, category: 'Drinks' },
    { id: 'line-2' as LineId, sku: 'BAGEL' as any, name: 'Bagel', qty: 1, unit_price: { minor_units: 250, currency: 'USD' }, category: 'Food' },
  ];

  beforeEach(() => {
    vi.clearAllMocks();
    localStorage.clear();
  });

  afterEach(() => {
    localStorage.clear();
  });

  it('handleLock saves cart to localStorage and calls logout', () => {
    const { result } = renderHook(() =>
      useLockedCartPersistence(
        sampleLines,
        10,
        '10% Discount',
        15,
        true,
        10,
        mockSetLines,
        mockSetDiscount,
        mockSetTipPercent,
        mockSetServiceCharge,
        mockLogout,
      ),
    );

    act(() => {
      result.current.handleLock();
    });

    expect(mockLogout).toHaveBeenCalled();
    const saved = localStorage.getItem('pos-locked-cart');
    expect(saved).toBeTruthy();
    const data = JSON.parse(saved!);
    expect(data.lines).toHaveLength(2);
    expect(data.discountPercent).toBe(10);
    expect(data.discountLabel).toBe('10% Discount');
    expect(data.tipPercent).toBe(15);
    expect(data.serviceChargeEnabled).toBe(true);
    expect(data.serviceChargePercent).toBe(10);
  });

  it('handleLock removes localStorage when cart is empty', () => {
    localStorage.setItem('pos-locked-cart', '{}');
    const { result } = renderHook(() =>
      useLockedCartPersistence(
        [],
        0,
        '',
        0,
        false,
        0,
        mockSetLines,
        mockSetDiscount,
        mockSetTipPercent,
        mockSetServiceCharge,
        mockLogout,
      ),
    );

    act(() => {
      result.current.handleLock();
    });

    expect(localStorage.getItem('pos-locked-cart')).toBeNull();
  });

  it('restoreLockedCart restores lines and settings', () => {
    const lockedData = {
      lines: [
        { sku: 'COFFEE', name: 'Coffee', category: 'Drinks', qty: 2, unit_price: { minor_units: 350, currency: 'USD' } },
        { sku: 'BAGEL', name: 'Bagel', category: 'Food', qty: 1, unit_price: { minor_units: 250, currency: 'USD' } },
      ],
      discountPercent: 10,
      discountLabel: '10% Off',
      tipPercent: 15,
      serviceChargeEnabled: true,
      serviceChargePercent: 10,
    };
    localStorage.setItem('pos-locked-cart', JSON.stringify(lockedData));

    const { result } = renderHook(() =>
      useLockedCartPersistence(
        [],
        0,
        '',
        0,
        false,
        0,
        mockSetLines,
        mockSetDiscount,
        mockSetTipPercent,
        mockSetServiceCharge,
        mockLogout,
      ),
    );

    act(() => {
      result.current.restoreLockedCart();
    });

    expect(mockSetLines).toHaveBeenCalled();
    const restoredLines = mockSetLines.mock.calls[0]?.[0];
    expect(restoredLines).toBeDefined();
    if (restoredLines) {
      expect(restoredLines).toHaveLength(2);
      expect(restoredLines[0].sku).toBe('COFFEE');
      expect(restoredLines[1].sku).toBe('BAGEL');
    }

    expect(mockSetDiscount).toHaveBeenCalledWith(10, '10% Off');
    expect(mockSetTipPercent).toHaveBeenCalledWith(15);
    expect(mockSetServiceCharge).toHaveBeenCalledWith(true, 10);

    // Should clear localStorage after restore
    expect(localStorage.getItem('pos-locked-cart')).toBeNull();
  });

  it('restoreLockedCart does nothing when no saved data', () => {
    const { result } = renderHook(() =>
      useLockedCartPersistence(
        [],
        0,
        '',
        0,
        false,
        0,
        mockSetLines,
        mockSetDiscount,
        mockSetTipPercent,
        mockSetServiceCharge,
        mockLogout,
      ),
    );

    act(() => {
      result.current.restoreLockedCart();
    });

    expect(mockSetLines).not.toHaveBeenCalled();
    expect(mockSetDiscount).not.toHaveBeenCalled();
  });

  it('restoreLockedCart handles malformed JSON gracefully', () => {
    localStorage.setItem('pos-locked-cart', 'not valid json');

    const { result } = renderHook(() =>
      useLockedCartPersistence(
        [],
        0,
        '',
        0,
        false,
        0,
        mockSetLines,
        mockSetDiscount,
        mockSetTipPercent,
        mockSetServiceCharge,
        mockLogout,
      ),
    );

    act(() => {
      result.current.restoreLockedCart();
    });

    expect(mockSetLines).not.toHaveBeenCalled();
  });

  it('restoreLockedCart handles missing optional fields', () => {
    const minimalData = {
      lines: [{ sku: 'TEA', name: 'Tea', qty: 1, unit_price: { minor_units: 200, currency: 'USD' } }],
    };
    localStorage.setItem('pos-locked-cart', JSON.stringify(minimalData));

    const { result } = renderHook(() =>
      useLockedCartPersistence(
        [],
        0,
        '',
        0,
        false,
        0,
        mockSetLines,
        mockSetDiscount,
        mockSetTipPercent,
        mockSetServiceCharge,
        mockLogout,
      ),
    );

    act(() => {
      result.current.restoreLockedCart();
    });

    expect(mockSetLines).toHaveBeenCalled();
    // Should not call setters for missing fields
    expect(mockSetDiscount).not.toHaveBeenCalled();
    expect(mockSetTipPercent).not.toHaveBeenCalled();
    expect(mockSetServiceCharge).not.toHaveBeenCalled();
  });
});