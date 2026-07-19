import { invoke } from '@tauri-apps/api/core';

/** Status of a Kitchen Display System order. */
export type KdsStatus = 'pending' | 'preparing' | 'ready' | 'served' | 'cancelled';

/** A Kitchen Display System order. */
export interface KdsOrder {
  id: string;
  sale_id: string;
  /** The store this order belongs to (ADR #8). Used for defense-in-depth filtering. */
  store_id: string | null;
  status: KdsStatus;
  items_summary: string;
  item_count: number;
  display_number: number | null;
  received_at: string;
  started_at: string | null;
  ready_at: string | null;
  served_at: string | null;
  prep_time_seconds: number;
  /** Kitchen zone this order is assigned to (e.g., "front", "back"). */
  kitchen_zone: string | null;
  notes: string;
}

/** List KDS orders, optionally filtered by status. */
export const listKdsOrders = (userId: string, status?: KdsStatus): Promise<KdsOrder[]> =>
  invoke<KdsOrder[]>('list_kds_orders', { userId, status: status ?? null });

/** List KDS orders (scoped — ADR #7). */
export const listKdsOrdersScoped = (sessionToken: string, status?: KdsStatus): Promise<KdsOrder[]> =>
  invoke<KdsOrder[]>('list_kds_orders_scoped', { sessionToken, status: status ?? null });

/** Get the KDS queue for the current user, optionally filtered by kitchen zone. */
export const getKdsQueue = (userId: string, kdsZone?: string): Promise<KdsOrder[]> =>
  invoke<KdsOrder[]>('get_kds_queue', { userId, kdsZone: kdsZone ?? null });

/** Get the KDS queue (scoped — ADR #7), optionally filtered by kitchen zone. */
export const getKdsQueueScoped = (sessionToken: string, kdsZone?: string): Promise<KdsOrder[]> =>
  invoke<KdsOrder[]>('get_kds_queue_scoped', { sessionToken, kdsZone: kdsZone ?? null });

/** Update a KDS order's status (e.g. preparing, ready, served). */
export const updateKdsStatus = (userId: string, id: string, status: KdsStatus): Promise<KdsOrder> =>
  invoke<KdsOrder>('update_kds_status', { userId, id, status });

/** Update a KDS order's status (scoped — ADR #7). */
export const updateKdsStatusScoped = (sessionToken: string, id: string, status: KdsStatus): Promise<KdsOrder> =>
  invoke<KdsOrder>('update_kds_status_scoped', { sessionToken, id, status });

/** Create KDS orders from a completed sale. Returns one order per kitchen zone. */
export const createKdsOrderFromSale = (userId: string, saleId: string): Promise<KdsOrder[]> =>
  invoke<KdsOrder[]>('create_kds_order_from_sale', { userId, saleId });

/** Create KDS orders from a sale (scoped — ADR #7). */
export const createKdsOrderFromSaleScoped = (sessionToken: string, saleId: string): Promise<KdsOrder[]> =>
  invoke<KdsOrder[]>('create_kds_order_from_sale_scoped', { sessionToken, saleId });

/** Get a single KDS order by its identifier. */
export const getKdsOrder = (userId: string, id: string): Promise<KdsOrder | null> =>
  invoke<KdsOrder | null>('get_kds_order', { userId, id });

/** Get a KDS order by id (scoped — ADR #7). */
export const getKdsOrderScoped = (sessionToken: string, id: string): Promise<KdsOrder | null> =>
  invoke<KdsOrder | null>('get_kds_order_scoped', { sessionToken, id });
