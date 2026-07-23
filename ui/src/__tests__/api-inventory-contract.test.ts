// ── IPC contract tests for inventory.ts ───────────────────────────
//
// Verifies the Tauri command name and argument shape for every
// exported function in ui/src/api/inventory.ts (24 invoke calls, 0
// prior tests).

import { describe, it, expect, vi, beforeEach } from 'vitest';

const { mockInvoke } = vi.hoisted(() => ({
  mockInvoke: vi.fn(),
}));

vi.mock('@tauri-apps/api/core', () => ({
  invoke: (cmd: string, args?: Record<string, unknown>) => mockInvoke(cmd, args),
}));

import {
  createInventoryLocation,
  listInventoryLocations,
  updateInventoryLocation,
  deactivateInventoryLocation,
  setWorkspaceInventoryLocations,
  getWorkspaceInventoryLocations,
  startInventoryShift,
  endInventoryShift,
  getActiveInventoryShift,
  listInventoryShifts,
  createInventoryTransaction,
  listInventoryTransactions,
  setStockThreshold,
  getStockThresholds,
  deleteStockThreshold,
} from '@/api/inventory';

describe('inventory.ts IPC contract', () => {
  beforeEach(() => mockInvoke.mockReset());

  // ── Inventory locations ──

  it('createInventoryLocation invokes "create_inventory_location" with flat args', async () => {
    mockInvoke.mockResolvedValue('loc-1');
    await createInventoryLocation('tok', 'Warehouse A', 'warehouse', 'Main storage');
    expect(mockInvoke).toHaveBeenCalledWith('create_inventory_location', {
      sessionToken: 'tok',
      name: 'Warehouse A',
      locationType: 'warehouse',
      description: 'Main storage',
    });
  });

  it('listInventoryLocations invokes "list_inventory_locations" with sessionToken', async () => {
    mockInvoke.mockResolvedValue([]);
    await listInventoryLocations('tok');
    expect(mockInvoke).toHaveBeenCalledWith('list_inventory_locations', {
      sessionToken: 'tok',
    });
  });

  it('updateInventoryLocation invokes "update_inventory_location" with flat args', async () => {
    mockInvoke.mockResolvedValue(undefined);
    await updateInventoryLocation('tok', 'loc-1', 'Renamed', 'warehouse', 'Updated');
    expect(mockInvoke).toHaveBeenCalledWith('update_inventory_location', {
      sessionToken: 'tok',
      id: 'loc-1',
      name: 'Renamed',
      locationType: 'warehouse',
      description: 'Updated',
    });
  });

  it('deactivateInventoryLocation invokes "deactivate_inventory_location" with sessionToken + id', async () => {
    mockInvoke.mockResolvedValue(undefined);
    await deactivateInventoryLocation('tok', 'loc-1');
    expect(mockInvoke).toHaveBeenCalledWith('deactivate_inventory_location', {
      sessionToken: 'tok',
      id: 'loc-1',
    });
  });

  // ── Workspace location bindings ──

  it('setWorkspaceInventoryLocations invokes "set_workspace_inventory_locations" with sessionToken + instanceId + locations', async () => {
    mockInvoke.mockResolvedValue(undefined);
    const locations = [{ id: 'bl-1', instance_id: 'ws-1', location_id: 'loc-1', is_primary: true, allow_negative_stock: false, sort_order: 0 }];
    await setWorkspaceInventoryLocations('tok', 'ws-1', locations);
    expect(mockInvoke).toHaveBeenCalledWith('set_workspace_inventory_locations', {
      sessionToken: 'tok',
      instanceId: 'ws-1',
      locations,
    });
  });

  it('getWorkspaceInventoryLocations invokes "get_workspace_inventory_locations" with sessionToken + instanceId', async () => {
    mockInvoke.mockResolvedValue([]);
    await getWorkspaceInventoryLocations('tok', 'ws-1');
    expect(mockInvoke).toHaveBeenCalledWith('get_workspace_inventory_locations', {
      sessionToken: 'tok',
      instanceId: 'ws-1',
    });
  });

  // ── Inventory shifts ──

  it('startInventoryShift invokes "start_inventory_shift" with sessionToken + userId + locationId + notes', async () => {
    mockInvoke.mockResolvedValue({ id: 'shift-1', user_id: 'u1', status: 'active' });
    await startInventoryShift('tok', 'u1', 'loc-1', 'morning count');
    expect(mockInvoke).toHaveBeenCalledWith('start_inventory_shift', {
      sessionToken: 'tok',
      userId: 'u1',
      locationId: 'loc-1',
      notes: 'morning count',
    });
  });

  it('endInventoryShift invokes "end_inventory_shift" with sessionToken + shiftId', async () => {
    mockInvoke.mockResolvedValue(undefined);
    await endInventoryShift('tok', 'shift-1');
    expect(mockInvoke).toHaveBeenCalledWith('end_inventory_shift', {
      sessionToken: 'tok',
      shiftId: 'shift-1',
    });
  });

  it('getActiveInventoryShift invokes "get_active_inventory_shift" with sessionToken + userId', async () => {
    mockInvoke.mockResolvedValue(null);
    await getActiveInventoryShift('tok', 'u1');
    expect(mockInvoke).toHaveBeenCalledWith('get_active_inventory_shift', {
      sessionToken: 'tok',
      userId: 'u1',
    });
  });

  it('listInventoryShifts invokes "list_inventory_shifts" with sessionToken', async () => {
    mockInvoke.mockResolvedValue([]);
    await listInventoryShifts('tok');
    expect(mockInvoke).toHaveBeenCalledWith('list_inventory_shifts', {
      sessionToken: 'tok',
    });
  });

  // ── Inventory transactions ──

  it('createInventoryTransaction invokes "create_inventory_transaction" with flat args + lines', async () => {
    mockInvoke.mockResolvedValue('txn-1');
    const lines = [{ sku: 'SKU-1', product_name: 'Widget', qty: 5, delta: 5, barcode_scanned: null }];
    await createInventoryTransaction('tok', 'receive', 'loc-1', 'u1', 'restock', lines);
    expect(mockInvoke).toHaveBeenCalledWith('create_inventory_transaction', {
      sessionToken: 'tok',
      typeStr: 'receive',
      locationId: 'loc-1',
      staffId: 'u1',
      notes: 'restock',
      lines,
    });
  });

  it('listInventoryTransactions invokes "list_inventory_transactions" with sessionToken', async () => {
    mockInvoke.mockResolvedValue([]);
    await listInventoryTransactions('tok');
    expect(mockInvoke).toHaveBeenCalledWith('list_inventory_transactions', {
      sessionToken: 'tok',
    });
  });

  // ── Stock thresholds ──

  it('setStockThreshold invokes "set_stock_threshold" with sessionToken + productId + locationId + threshold + enabled', async () => {
    mockInvoke.mockResolvedValue(undefined);
    await setStockThreshold('tok', 'p1', 'loc-1', 10, true);
    expect(mockInvoke).toHaveBeenCalledWith('set_stock_threshold', {
      sessionToken: 'tok',
      productId: 'p1',
      locationId: 'loc-1',
      threshold: 10,
      enabled: true,
    });
  });

  it('getStockThresholds invokes "get_stock_thresholds" with sessionToken + locationId', async () => {
    mockInvoke.mockResolvedValue([]);
    await getStockThresholds('tok', 'loc-1');
    expect(mockInvoke).toHaveBeenCalledWith('get_stock_thresholds', {
      sessionToken: 'tok',
      locationId: 'loc-1',
    });
  });

  it('deleteStockThreshold invokes "delete_stock_threshold" with sessionToken + id', async () => {
    mockInvoke.mockResolvedValue(undefined);
    await deleteStockThreshold('tok', 'thresh-1');
    expect(mockInvoke).toHaveBeenCalledWith('delete_stock_threshold', {
      sessionToken: 'tok',
      id: 'thresh-1',
    });
  });

  // ── Error propagation ──

  it('propagates backend errors', async () => {
    mockInvoke.mockRejectedValueOnce(new Error('not found'));
    await expect(listInventoryLocations('tok')).rejects.toThrow('not found');
  });
});
