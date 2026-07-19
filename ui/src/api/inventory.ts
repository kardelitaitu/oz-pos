import { invoke } from '@tauri-apps/api/core';

export interface InventoryLocation {
  id: string;
  name: string;
  type: 'store' | 'warehouse' | 'transit' | 'damaged' | 'virtual';
  description: string;
  is_active: boolean;
  created_at: string;
  updated_at: string;
}

export interface WorkspaceInventoryLocation {
  id: string;
  instance_id: string;
  location_id: string;
  is_primary: boolean;
  allow_negative_stock: boolean;
  sort_order: number;
}

export interface InventoryShift {
  id: string;
  user_id: string;
  location_id: string;
  terminal_id: string | null;
  started_at: string;
  ended_at: string | null;
  status: 'active' | 'ended';
  notes: string;
}

export interface InventoryTransaction {
  id: string;
  type: 'sale' | 'void' | 'refund' | 'transfer' | 'purchase-order-receive' | 'stock-count' | 'manual-adjustment';
  location_id: string;
  staff_id: string;
  transfer_id: string | null;
  purchase_order_id: string | null;
  notes: string;
  created_at: string;
}

export interface InventoryTransactionLine {
  id: string;
  transaction_id: string;
  sku: string;
  product_name: string;
  qty: number;
  barcode_scanned: string | null;
  sort_order: number;
}

export interface InventoryTransactionLineInput {
  sku: string;
  product_name: string;
  qty: number;
  delta: number;
  barcode_scanned: string | null;
}

export interface StockThreshold {
  id: string;
  product_id: string;
  location_id: string | null;
  threshold: number;
  enabled: boolean;
  created_at: string;
  updated_at: string;
}

// ── Locations CRUD ──

export const createInventoryLocation = (
  sessionToken: string,
  name: string,
  locationType: string,
  description: string
): Promise<string> =>
  invoke<string>('create_inventory_location', { sessionToken, name, locationType, description });

export const listInventoryLocations = (sessionToken: string): Promise<InventoryLocation[]> =>
  invoke<InventoryLocation[]>('list_inventory_locations', { sessionToken });

export const updateInventoryLocation = (
  sessionToken: string,
  id: string,
  name: string,
  locationType: string,
  description: string
): Promise<void> =>
  invoke<void>('update_inventory_location', { sessionToken, id, name, locationType, description });

export const deactivateInventoryLocation = (sessionToken: string, id: string): Promise<void> =>
  invoke<void>('deactivate_inventory_location', { sessionToken, id });

// ── Workspace Location Bindings ──

export const setWorkspaceInventoryLocations = (
  sessionToken: string,
  instanceId: string,
  locations: WorkspaceInventoryLocation[]
): Promise<void> =>
  invoke<void>('set_workspace_inventory_locations', { sessionToken, instanceId, locations });

export const getWorkspaceInventoryLocations = (
  sessionToken: string,
  instanceId: string
): Promise<WorkspaceInventoryLocation[]> =>
  invoke<WorkspaceInventoryLocation[]>('get_workspace_inventory_locations', { sessionToken, instanceId });

// ── Inventory Shifts ──

export const startInventoryShift = (
  sessionToken: string,
  userId: string,
  locationId: string,
  notes: string
): Promise<InventoryShift> =>
  invoke<InventoryShift>('start_inventory_shift', { sessionToken, userId, locationId, notes });

export const endInventoryShift = (sessionToken: string, shiftId: string): Promise<void> =>
  invoke<void>('end_inventory_shift', { sessionToken, shiftId });

export const getActiveInventoryShift = (sessionToken: string, userId: string): Promise<InventoryShift | null> =>
  invoke<InventoryShift | null>('get_active_inventory_shift', { sessionToken, userId });

export const listInventoryShifts = (sessionToken: string): Promise<InventoryShift[]> =>
  invoke<InventoryShift[]>('list_inventory_shifts', { sessionToken });

// ── Inventory Transaction Logs ──

export const createInventoryTransaction = (
  sessionToken: string,
  typeStr: string,
  locationId: string,
  staffId: string,
  notes: string,
  lines: InventoryTransactionLineInput[]
): Promise<string> =>
  invoke<string>('create_inventory_transaction', { sessionToken, typeStr, locationId, staffId, notes, lines });

export const listInventoryTransactions = (sessionToken: string): Promise<InventoryTransaction[]> =>
  invoke<InventoryTransaction[]>('list_inventory_transactions', { sessionToken });

export const getInventoryTransaction = (
  sessionToken: string,
  id: string
): Promise<[InventoryTransaction, InventoryTransactionLine[]] | null> =>
  invoke<[InventoryTransaction, InventoryTransactionLine[]] | null>('get_inventory_transaction', { sessionToken, id });

// ── Stock Thresholds ──

export const setStockThreshold = (
  sessionToken: string,
  productId: string,
  locationId: string | null,
  threshold: number,
  enabled: boolean
): Promise<void> =>
  invoke<void>('set_stock_threshold', { sessionToken, productId, locationId, threshold, enabled });

export const getStockThresholds = (sessionToken: string, locationId: string | null): Promise<StockThreshold[]> =>
  invoke<StockThreshold[]>('get_stock_thresholds', { sessionToken, locationId });

export const deleteStockThreshold = (sessionToken: string, id: string): Promise<void> =>
  invoke<void>('delete_stock_threshold', { sessionToken, id });

// ── Workspace Location Bindings (Unified Resolver) ──

export interface WorkspaceLocationBinding {
  location_id: string;
  location_name: string;
  is_primary: boolean;
  allow_negative_stock: boolean;
}

/** ADR-19 §10: Resolve locations bound to a workspace instance. */
export const getWorkspaceLocations = (
  sessionToken: string,
  instanceId: string,
  typeKey: string
): Promise<WorkspaceLocationBinding[]> =>
  invoke<WorkspaceLocationBinding[]>('get_workspace_locations_scoped', { sessionToken, instanceId, typeKey });

/** ADR-19 §4: Invalidate the location resolver cache. */
export const invalidateLocationCache = (sessionToken: string): Promise<void> =>
  invoke<void>('invalidate_location_cache_scoped', { sessionToken });

// ── Per-Location Low Stock Alerts ──

export const getLowStockAlertsAtLocation = (
  sessionToken: string,
  locationId: string,
  defaultThreshold: number
): Promise<LowStockAlert[]> =>
  invoke<LowStockAlert[]>('get_low_stock_alerts_at_location_scoped', { sessionToken, locationId, defaultThreshold });

export interface LowStockAlert {
  product_id: string;
  sku: string;
  name: string;
  current_qty: number;
  threshold: number;
}

// ── Stock Alert Events ──

export interface StockAlertEvent {
  id: string;
  threshold_id: string;
  product_id: string;
  location_id: string;
  current_qty: number;
  threshold: number;
  status: 'active' | 'acknowledged' | 'resolved';
  triggered_at: string;
  acknowledged_at: string | null;
  resolved_at: string | null;
  acknowledged_by: string | null;
  product_sku: string;
  product_name: string;
}

export const getActiveStockAlerts = (
  sessionToken: string,
  locationId: string
): Promise<StockAlertEvent[]> =>
  invoke<StockAlertEvent[]>('active_stock_alerts_scoped', { sessionToken, locationId });

export const acknowledgeStockAlert = (
  sessionToken: string,
  alertId: string
): Promise<void> =>
  invoke<void>('acknowledge_stock_alert_scoped', { sessionToken, alertId });

// ── Pending Sale Capture / Void ──

export const finalizeSale = (sessionToken: string, saleId: string): Promise<void> =>
  invoke<void>('finalize_sale', { sessionToken, saleId });

export const voidPendingSale = (sessionToken: string, saleId: string): Promise<void> =>
  invoke<void>('void_pending_sale', { sessionToken, saleId });
