// ── Physical Inventory / Stock Counting ─────────────────────────────

import { invoke } from '@tauri-apps/api/core';

// ── DTOs ────────────────────────────────────────────────────────────

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

export interface CreateStockCountArgs {
  countType: string;
  notes: string;
  countedBy?: string | null;
}

export interface AddCountLineArgs {
  countId: string;
  sku: string;
  productName: string;
  expectedQty: number;
}

export interface UpdateCountLineArgs {
  lineId: string;
  countedQty?: number | null;
  notes: string;
}

export interface CompleteStockCountArgs {
  countId: string;
  completedBy?: string | null;
}

// ── Commands ────────────────────────────────────────────────────────

export const createStockCount = (args: CreateStockCountArgs): Promise<StockCountDto> =>
  invoke<StockCountDto>('create_stock_count', { args });

export const getStockCount = (id: string): Promise<StockCountDto | null> =>
  invoke<StockCountDto | null>('get_stock_count', { id });

export const listStockCounts = (): Promise<StockCountDto[]> =>
  invoke<StockCountDto[]>('list_stock_counts');

export const getCountLines = (countId: string): Promise<StockCountLineDto[]> =>
  invoke<StockCountLineDto[]>('get_count_lines', { countId });

export const addCountLine = (args: AddCountLineArgs): Promise<StockCountLineDto> =>
  invoke<StockCountLineDto>('add_count_line', { args });

export const updateCountLine = (args: UpdateCountLineArgs): Promise<void> =>
  invoke<void>('update_count_line', { args });

export const removeCountLine = (args: { lineId: string }): Promise<void> =>
  invoke<void>('remove_count_line', { args });

export const completeStockCount = (args: CompleteStockCountArgs): Promise<StockAdjustmentDto[]> =>
  invoke<StockAdjustmentDto[]>('complete_stock_count', { args });

export const updateStockCountStatus = (id: string, status: string): Promise<void> =>
  invoke<void>('update_stock_count_status', { id, status });

export const listStockAdjustments = (): Promise<StockAdjustmentDto[]> =>
  invoke<StockAdjustmentDto[]>('list_stock_adjustments');
