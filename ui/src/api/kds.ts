import { invoke } from '@tauri-apps/api/core';

export type KdsStatus = 'pending' | 'preparing' | 'ready' | 'served' | 'cancelled';

export interface KdsOrder {
  id: string;
  sale_id: string;
  status: KdsStatus;
  items_summary: string;
  item_count: number;
  display_number: number | null;
  received_at: string;
  started_at: string | null;
  ready_at: string | null;
  served_at: string | null;
  prep_time_seconds: number;
  notes: string;
}

export const listKdsOrders = (status?: KdsStatus): Promise<KdsOrder[]> =>
  invoke<KdsOrder[]>('list_kds_orders', { status: status ?? null });

export const getKdsQueue = (): Promise<KdsOrder[]> =>
  invoke<KdsOrder[]>('get_kds_queue');

export const updateKdsStatus = (id: string, status: KdsStatus): Promise<KdsOrder> =>
  invoke<KdsOrder>('update_kds_status', { id, status });

export const createKdsOrderFromSale = (saleId: string): Promise<KdsOrder> =>
  invoke<KdsOrder>('create_kds_order_from_sale', { saleId });

export const getKdsOrder = (id: string): Promise<KdsOrder | null> =>
  invoke<KdsOrder | null>('get_kds_order', { id });
