import { loggedInvoke } from '@/utils/logged-invoke';

/** Status of a Kitchen Display System order. */
export type KdsStatus = 'pending' | 'preparing' | 'ready' | 'served' | 'cancelled';

/** A modifier choice attached to a line item. */
export interface KdsModifier {
  name: string;
  choice: string;
  price_minor: number;
}

/** A single line item on a KDS order ticket. */
export interface KdsLineItem {
  id: string;
  kds_order_id: string;
  sku: string;
  display_name: string;
  qty: number;
  /** Course assignment: "appetizer", "main", "dessert", "beverage", or null. */
  course: string | null;
  /** Modifier choices. */
  modifiers: KdsModifier[];
  line_position: number;
  item_status: string;
  started_at: string | null;
  ready_at: string | null;
  served_at: string | null;
  created_at: string;
}

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
  /** Table number assigned to this order (e.g., "T5"), or null for takeaway. */
  table_number: string | null;
  /** Priority/rush flag: when true the ticket visually escalates above normal SLA. */
  priority: boolean;
}

/** List KDS orders, optionally filtered by status. */
export const listKdsOrders = (userId: string, status?: KdsStatus): Promise<KdsOrder[]> =>
  loggedInvoke<KdsOrder[]>('list_kds_orders', { userId, status: status ?? null });

/** List KDS orders (scoped — ADR #7). */
export const listKdsOrdersScoped = (sessionToken: string, status?: KdsStatus): Promise<KdsOrder[]> =>
  loggedInvoke<KdsOrder[]>('list_kds_orders_scoped', { sessionToken, status: status ?? null });

/** Get the KDS queue for the current user, optionally filtered by kitchen zone. */
export const getKdsQueue = (userId: string, kdsZone?: string): Promise<KdsOrder[]> =>
  loggedInvoke<KdsOrder[]>('get_kds_queue', { userId, kdsZone: kdsZone ?? null });

/** Get the KDS queue (scoped — ADR #7), optionally filtered by kitchen zone. */
export const getKdsQueueScoped = (sessionToken: string, kdsZone?: string): Promise<KdsOrder[]> =>
  loggedInvoke<KdsOrder[]>('get_kds_queue_scoped', { sessionToken, kdsZone: kdsZone ?? null });

/** Update a KDS order's status (e.g. preparing, ready, served). */
export const updateKdsStatus = (userId: string, id: string, status: KdsStatus): Promise<KdsOrder> =>
  loggedInvoke<KdsOrder>('update_kds_status', { userId, id, status });

/** Update a KDS order's status (scoped — ADR #7). */
export const updateKdsStatusScoped = (sessionToken: string, id: string, status: KdsStatus): Promise<KdsOrder> =>
  loggedInvoke<KdsOrder>('update_kds_status_scoped', { sessionToken, id, status });

/** Create KDS orders from a completed sale. Returns one order per kitchen zone. */
export const createKdsOrderFromSale = (userId: string, saleId: string): Promise<KdsOrder[]> =>
  loggedInvoke<KdsOrder[]>('create_kds_order_from_sale', { userId, saleId });

/** Create KDS orders from a sale (scoped — ADR #7). */
export const createKdsOrderFromSaleScoped = (sessionToken: string, saleId: string): Promise<KdsOrder[]> =>
  loggedInvoke<KdsOrder[]>('create_kds_order_from_sale_scoped', { sessionToken, saleId });

/** Get a single KDS order by its identifier. */
export const getKdsOrder = (userId: string, id: string): Promise<KdsOrder | null> =>
  loggedInvoke<KdsOrder | null>('get_kds_order', { userId, id });

/** Get a KDS order by id (scoped — ADR #7). */
export const getKdsOrderScoped = (sessionToken: string, id: string): Promise<KdsOrder | null> =>
  loggedInvoke<KdsOrder | null>('get_kds_order_scoped', { sessionToken, id });

/** Input for creating a KDS line item (mirrors Rust CreateKdsLineItemInput). */
export interface CreateKdsLineItemInput {
  sku: string;
  display_name: string;
  qty: number;
  course: string | null;
  modifiers: KdsModifier[];
}

/** Input for updating items on an existing KDS order. */
export interface UpdateKdsOrderItemsInput {
  id: string;
  items_summary: string;
  item_count: number;
  /** Structured line items to replace kds_line_items. When provided, summary/count are re-derived. */
  line_items?: CreateKdsLineItemInput[] | null;
}

/** Update the items (summary + count) on an existing KDS order. */
export const updateKdsOrderItems = (userId: string, args: UpdateKdsOrderItemsInput): Promise<KdsOrder> =>
  loggedInvoke<KdsOrder>('update_kds_order_items', { userId, args });

/** Update KDS order items (scoped — ADR #7). */
export const updateKdsOrderItemsScoped = (sessionToken: string, args: UpdateKdsOrderItemsInput): Promise<KdsOrder> =>
  loggedInvoke<KdsOrder>('update_kds_order_items_scoped', { sessionToken, args });

/** Print a kitchen chit for a KDS order (scoped — ADR #7). */
export const printKdsChitScoped = (sessionToken: string, orderId: string): Promise<boolean> =>
  loggedInvoke<boolean>('print_kds_chit_scoped', { sessionToken, orderId });

/** Get all line items for a KDS order (scoped — ADR #7). */
export const getKdsOrderLinesScoped = (sessionToken: string, orderId: string): Promise<KdsLineItem[]> =>
  loggedInvoke<KdsLineItem[]>('get_kds_order_lines_scoped', { sessionToken, orderId });

/** Update the status of a single KDS line item (scoped — ADR #7). */
export const updateKdsLineItemStatusScoped = (
  sessionToken: string,
  itemId: string,
  status: KdsStatus,
): Promise<KdsLineItem> =>
  loggedInvoke<KdsLineItem>('update_kds_line_item_status_scoped', { sessionToken, itemId, status });
