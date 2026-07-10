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

export const listKdsOrders = (userId: string, status?: KdsStatus): Promise<KdsOrder[]> =>
  invoke<KdsOrder[]>('list_kds_orders', { userId, status: status ?? null });

/** List KDS orders (scoped — ADR #7). */
export const listKdsOrdersScoped = (sessionToken: string, status?: KdsStatus): Promise<KdsOrder[]> =>
  invoke<KdsOrder[]>('list_kds_orders_scoped', { sessionToken, status: status ?? null });

export const getKdsQueue = (userId: string): Promise<KdsOrder[]> =>
  invoke<KdsOrder[]>('get_kds_queue', { userId });

/** Get the KDS queue (scoped — ADR #7). */
export const getKdsQueueScoped = (sessionToken: string): Promise<KdsOrder[]> =>
  invoke<KdsOrder[]>('get_kds_queue_scoped', { sessionToken });

export const updateKdsStatus = (userId: string, id: string, status: KdsStatus): Promise<KdsOrder> =>
  invoke<KdsOrder>('update_kds_status', { userId, id, status });

/** Update a KDS order's status (scoped — ADR #7). */
export const updateKdsStatusScoped = (sessionToken: string, id: string, status: KdsStatus): Promise<KdsOrder> =>
  invoke<KdsOrder>('update_kds_status_scoped', { sessionToken, id, status });

export const createKdsOrderFromSale = (userId: string, saleId: string): Promise<KdsOrder> =>
  invoke<KdsOrder>('create_kds_order_from_sale', { userId, saleId });

/** Create a KDS order from a sale (scoped — ADR #7). */
export const createKdsOrderFromSaleScoped = (sessionToken: string, saleId: string): Promise<KdsOrder> =>
  invoke<KdsOrder>('create_kds_order_from_sale_scoped', { sessionToken, saleId });

export const getKdsOrder = (userId: string, id: string): Promise<KdsOrder | null> =>
  invoke<KdsOrder | null>('get_kds_order', { userId, id });

/** Get a KDS order by id (scoped — ADR #7). */
export const getKdsOrderScoped = (sessionToken: string, id: string): Promise<KdsOrder | null> =>
  invoke<KdsOrder | null>('get_kds_order_scoped', { sessionToken, id });
