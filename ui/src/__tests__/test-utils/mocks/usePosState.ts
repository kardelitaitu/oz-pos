// ── Shared usePosState mock factory ──────────────────────────────
//
// Factory for the `usePosState` hook (retail/restaurant cart state).
// Previously the full ~20-line mock return object was duplicated in
// RetailPosScreen.test.tsx, RetailPosScreenInteractions.test.tsx, and
// RetailPosScreenCheckout.test.tsx. Call this factory to get a fresh
// default cart (empty lines, no money, vi.fn() action handlers) and
// pass overrides to customise per-test.
//
// Usage:
//   import { createUsePosStateMock } from '@/__tests__/test-utils/mocks/usePosState';
//
//   // In a vi.mock block (async import to avoid hoisting conflicts):
//   vi.mock('@/features/sales/usePosState', async () => {
//     const { createUsePosStateMock } =
//       await import('@/__tests__/test-utils/mocks/usePosState');
//     return { usePosState: vi.fn(() => createUsePosStateMock()) };
//   });
//
//   // Per-test override:
//   vi.mocked(posState.usePosState).mockReturnValue(createUsePosStateMock({
//     lines: [{ id: 'line-1' as LineId, sku: 'SKU-001' as Sku, name: 'X', qty: 1,
//               unit_price: { minor_units: 3500, currency: 'IDR' } }],
//     total: { minor_units: 3500, currency: 'IDR' },
//     subtotal: { minor_units: 3500, currency: 'IDR' },
//     addProduct: mockAddProduct,
//   }));

import { vi } from 'vitest';
import type { CartLine, Money } from '@/types/domain';
// Type-only import — erased at runtime, so no cycle with the mocked module.
import type { usePosState } from '@/features/sales/usePosState';

type UsePosStateReturn = ReturnType<typeof usePosState>;

export interface UsePosStateMockOverrides {
  lines?: CartLine[];
  total?: Money | null;
  subtotal?: Money | null;
  discountPercent?: number;
  discountLabel?: string;
  discountAmount?: Money | null;
  tipPercent?: number;
  tipAmount?: Money | null;
  serviceChargeEnabled?: boolean;
  serviceChargePercent?: number;
  serviceChargeAmount?: Money | null;
  addProduct?: ReturnType<typeof vi.fn>;
  removeLine?: ReturnType<typeof vi.fn>;
  updateQty?: ReturnType<typeof vi.fn>;
  setDiscount?: ReturnType<typeof vi.fn>;
  updateLinePrice?: ReturnType<typeof vi.fn>;
  setTipPercent?: ReturnType<typeof vi.fn>;
  setServiceCharge?: ReturnType<typeof vi.fn>;
  resetCart?: ReturnType<typeof vi.fn>;
  setLines?: ReturnType<typeof vi.fn>;
  assignCourse?: ReturnType<typeof vi.fn>;
  fireCourse?: ReturnType<typeof vi.fn>;
  fireAllCourses?: ReturnType<typeof vi.fn>;
}

/**
 * Fresh `usePosState` mock — empty cart, no money, vi.fn() actions.
 * Returns the real hook's return type so `mockReturnValue` typechecks
 * without per-test `as any` casts.
 */
export function createUsePosStateMock(overrides: UsePosStateMockOverrides = {}): UsePosStateReturn {
  return {
    lines: [] as CartLine[],
    total: null as Money | null,
    subtotal: null as Money | null,
    discountPercent: 0,
    discountLabel: '',
    discountAmount: null as Money | null,
    tipPercent: 0,
    tipAmount: null as Money | null,
    serviceChargeEnabled: false,
    serviceChargePercent: 0,
    serviceChargeAmount: null as Money | null,
    addProduct: vi.fn(),
    removeLine: vi.fn(),
    updateQty: vi.fn(),
    setDiscount: vi.fn(),
    updateLinePrice: vi.fn(),
    setTipPercent: vi.fn(),
    setServiceCharge: vi.fn(),
    resetCart: vi.fn(),
    setLines: vi.fn(),
    assignCourse: vi.fn(),
    fireCourse: vi.fn(),
    fireAllCourses: vi.fn(),
    ...overrides,
  } as unknown as UsePosStateReturn;
}
