// ── Physical Inventory / Stock Counting ─────────────────────────────

import { loggedInvoke } from '@/utils/logged-invoke';

// ── DTOs ────────────────────────────────────────────────────────────

/** A physical stock count session. */
export interface StockCountDto {
  id: string;
  count_number: string;
  status: 'draft' | 'in_progress' | 'completed' | 'cancelled';
  count_type: 'full' | 'cyclic' | 'spot';
  notes: string;
  counted_by: string | null;
  created_at: string;
  completed_at: string | null;
  updated_at: string;
}

/** A single line within a stock count. */
export interface StockCountLineDto {
  id: string;
  count_id: string;
  sku: string;
  product_name: string;
  expected_qty: number;
  counted_qty: number | null;
  difference: number;
  notes: string;
}

/** A stock adjustment resulting from a count or manual correction. */
export interface StockAdjustmentDto {
  id: string;
  count_id: string | null;
  sku: string;
  product_name: string;
  previous_qty: number;
  adjusted_qty: number;
  reason: string;
  created_by: string | null;
  created_at: string;
}

// ── Args ────────────────────────────────────────────────────────────

/** Arguments for creating a new stock count. */
export interface CreateStockCountArgs {
  countType: string;
  notes: string;
  countedBy?: string | null;
}

/** Arguments for adding a line to a stock count. */
export interface AddCountLineArgs {
  countId: string;
  sku: string;
  productName: string;
  expectedQty: number;
}

/** Arguments for updating a stock count line's counted quantity. */
export interface UpdateCountLineArgs {
  lineId: string;
  countedQty?: number | null;
  notes: string;
}

/** Arguments for completing a stock count and generating adjustments. */
export interface CompleteStockCountArgs {
  countId: string;
  completedBy?: string | null;
}

// ── Commands ────────────────────────────────────────────────────────

/** Create a new stock count session. */
export const createStockCount = (args: CreateStockCountArgs): Promise<StockCountDto> =>
  loggedInvoke<StockCountDto>('create_stock_count', { args });

/** Get a single stock count by its identifier. */
export const getStockCount = (id: string): Promise<StockCountDto | null> =>
  loggedInvoke<StockCountDto | null>('get_stock_count', { id });

/** List all stock counts. */
export const listStockCounts = (): Promise<StockCountDto[]> =>
  loggedInvoke<StockCountDto[]>('list_stock_counts');

/** Get all lines for a given stock count. */
export const getCountLines = (countId: string): Promise<StockCountLineDto[]> =>
  loggedInvoke<StockCountLineDto[]>('get_count_lines', { countId });

/** Add a line to a stock count. */
export const addCountLine = (args: AddCountLineArgs): Promise<StockCountLineDto> =>
  loggedInvoke<StockCountLineDto>('add_count_line', { args });

/** Update a stock count line's counted quantity. */
export const updateCountLine = (args: UpdateCountLineArgs): Promise<void> =>
  loggedInvoke<void>('update_count_line', { args });

/** Remove a line from a stock count. */
export const removeCountLine = (args: { lineId: string }): Promise<void> =>
  loggedInvoke<void>('remove_count_line', { args });

/** Complete a stock count, generating adjustments for any discrepancies. */
export const completeStockCount = (args: CompleteStockCountArgs): Promise<StockAdjustmentDto[]> =>
  loggedInvoke<StockAdjustmentDto[]>('complete_stock_count', { args });

/** Update a stock count's status directly (e.g. cancel). */
export const updateStockCountStatus = (id: string, status: string): Promise<void> =>
  loggedInvoke<void>('update_stock_count_status', { id, status });

/** List all stock adjustments. */
export const listStockAdjustments = (): Promise<StockAdjustmentDto[]> =>
  loggedInvoke<StockAdjustmentDto[]>('list_stock_adjustments');
