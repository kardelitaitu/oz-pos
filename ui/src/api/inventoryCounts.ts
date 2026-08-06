// ── Physical Inventory / Stock Counting ─────────────────────────────

import { loggedInvoke } from '@/utils/logged-invoke';

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

/** Arguments for creating a new stock count. The actor is session-derived. */
export interface CreateStockCountArgs {
  countType: string;
  notes: string;
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

/** Arguments for completing a stock count. The actor is session-derived. */
export interface CompleteStockCountArgs {
  countId: string;
}

/** Create a stock count in the store resolved from the session token. */
export const createStockCount = (
  sessionToken: string,
  args: CreateStockCountArgs,
): Promise<StockCountDto> =>
  loggedInvoke<StockCountDto>('create_stock_count_scoped', { sessionToken, args });

/** Get a stock count from the store resolved from the session token. */
export const getStockCount = (
  sessionToken: string,
  id: string,
): Promise<StockCountDto | null> =>
  loggedInvoke<StockCountDto | null>('get_stock_count_scoped', { sessionToken, id });

/** List stock counts from the store resolved from the session token. */
export const listStockCounts = (sessionToken: string): Promise<StockCountDto[]> =>
  loggedInvoke<StockCountDto[]>('list_stock_counts_scoped', { sessionToken });

/** Get count lines from the store resolved from the session token. */
export const getCountLines = (
  sessionToken: string,
  countId: string,
): Promise<StockCountLineDto[]> =>
  loggedInvoke<StockCountLineDto[]>('get_count_lines_scoped', { sessionToken, countId });

/** Add a line to a count in the store resolved from the session token. */
export const addCountLine = (
  sessionToken: string,
  args: AddCountLineArgs,
): Promise<StockCountLineDto> =>
  loggedInvoke<StockCountLineDto>('add_count_line_scoped', { sessionToken, args });

/** Update a count line in the store resolved from the session token. */
export const updateCountLine = (
  sessionToken: string,
  args: UpdateCountLineArgs,
): Promise<void> =>
  loggedInvoke<void>('update_count_line_scoped', { sessionToken, args });

/** Remove a count line in the store resolved from the session token. */
export const removeCountLine = (
  sessionToken: string,
  args: { lineId: string },
): Promise<void> =>
  loggedInvoke<void>('remove_count_line_scoped', { sessionToken, args });

/** Complete a count in the store resolved from the session token. */
export const completeStockCount = (
  sessionToken: string,
  args: CompleteStockCountArgs,
): Promise<StockAdjustmentDto[]> =>
  loggedInvoke<StockAdjustmentDto[]>('complete_stock_count_scoped', { sessionToken, args });

/** Update a count lifecycle status in the store resolved from the session token. */
export const updateStockCountStatus = (
  sessionToken: string,
  id: string,
  status: string,
): Promise<void> =>
  loggedInvoke<void>('update_stock_count_status_scoped', { sessionToken, id, status });

/** List adjustments from the store resolved from the session token. */
export const listStockAdjustments = (
  sessionToken: string,
): Promise<StockAdjustmentDto[]> =>
  loggedInvoke<StockAdjustmentDto[]>('list_stock_adjustments_scoped', { sessionToken });
