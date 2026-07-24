// ── IPC contract tests for the Tauri API layer ─────────────────────
//
// These tests verify the contract between the TypeScript API wrappers
// (ui/src/api/*.ts) and the Rust Tauri commands: the correct command
// name is invoked with the correct argument shape (camelCase keys).
// A mismatch in command name or argument key causes a silent runtime
// failure (undefined arg, command not found) that the type system
// cannot catch.
//
// The `loggedInvoke` wrapper delegates to `@tauri-apps/api/core`'s
// `invoke`, so we mock that and assert the (command, args) pair.

import { describe, it, expect, vi, beforeEach } from 'vitest';
import type { CartId } from '@/types/domain';

const { mockInvoke } = vi.hoisted(() => ({
  mockInvoke: vi.fn(),
}));

vi.mock('@tauri-apps/api/core', () => ({
  invoke: (cmd: string, args?: Record<string, unknown>) => mockInvoke(cmd, args),
}));

// ── sales.ts ───────────────────────────────────────────────────────

import {
  startSale,
  startSaleScoped,
  addLine,
  addLineScoped,
  completeSale,
  completeSaleScoped,
  voidSaleScoped,
  holdCartScoped,
  listSalesScoped,
  getSaleScoped,
  overrideLinePriceScoped,
  finalizeSale,
  voidPendingSale,
  setCartDiscountScoped,
} from '@/api/sales';

describe('sales.ts IPC contract', () => {
  beforeEach(() => mockInvoke.mockReset());

  it('startSale invokes "start_sale" with args', async () => {
    mockInvoke.mockResolvedValue({ cartId: 'cart-1' as CartId });
    await startSale({ currency: 'USD' });
    expect(mockInvoke).toHaveBeenCalledWith('start_sale', {
      args: { currency: 'USD' },
    });
  });

  it('startSaleScoped invokes "start_sale_scoped" with sessionToken + args', async () => {
    mockInvoke.mockResolvedValue({ cartId: 'cart-1' as CartId });
    await startSaleScoped('tok', { currency: 'IDR' });
    expect(mockInvoke).toHaveBeenCalledWith('start_sale_scoped', {
      sessionToken: 'tok',
      args: { currency: 'IDR' },
    });
  });

  it('addLine invokes "add_line" with camelCase args', async () => {
    mockInvoke.mockResolvedValue({ lineId: 'l1', lineTotal: null });
    await addLine({ cartId: 'c1' as CartId, sku: 'SKU-1', qty: 2, unitPriceMinor: 500 });
    expect(mockInvoke).toHaveBeenCalledWith('add_line', {
      args: { cartId: 'c1' as CartId, sku: 'SKU-1', qty: 2, unitPriceMinor: 500 },
    });
  });

  it('addLineScoped invokes "add_line_scoped" with sessionToken + args', async () => {
    mockInvoke.mockResolvedValue({ lineId: 'l1', lineTotal: null });
    await addLineScoped('tok', { cartId: 'c1' as CartId, sku: 'SKU-1', qty: 1, unitPriceMinor: 100 });
    expect(mockInvoke).toHaveBeenCalledWith('add_line_scoped', {
      sessionToken: 'tok',
      args: { cartId: 'c1' as CartId, sku: 'SKU-1', qty: 1, unitPriceMinor: 100 },
    });
  });

  it('completeSale invokes "complete_sale" with full args', async () => {
    mockInvoke.mockResolvedValue({ saleId: 's1', total: null, lineCount: 1 });
    await completeSale({
      cartId: 'c1' as CartId,
      paymentMethod: 'cash',
      tenderedMinor: 1000,
      userId: 'u1',
    });
    expect(mockInvoke).toHaveBeenCalledWith('complete_sale', {
      args: {
        cartId: 'c1' as CartId,
        paymentMethod: 'cash',
        tenderedMinor: 1000,
        userId: 'u1',
      },
    });
  });

  it('completeSaleScoped invokes "complete_sale_scoped" with sessionToken + args', async () => {
    mockInvoke.mockResolvedValue({ saleId: 's1', total: null, lineCount: 1 });
    await completeSaleScoped('tok', {
      cartId: 'c1' as CartId,
      paymentMethod: 'card',
      tenderedMinor: null,
    });
    expect(mockInvoke).toHaveBeenCalledWith('complete_sale_scoped', {
      sessionToken: 'tok',
      args: { cartId: 'c1' as CartId, paymentMethod: 'card', tenderedMinor: null },
    });
  });

  it('voidSaleScoped invokes "void_sale_scoped" with sessionToken + args(saleId, reason)', async () => {
    mockInvoke.mockResolvedValue({ id: 's1' });
    await voidSaleScoped('tok', 'sale-1', 'customer cancel');
    expect(mockInvoke).toHaveBeenCalledWith('void_sale_scoped', {
      sessionToken: 'tok',
      args: { saleId: 'sale-1', reason: 'customer cancel' },
    });
  });

  it('holdCartScoped invokes "hold_cart_scoped" with sessionToken + args', async () => {
    mockInvoke.mockResolvedValue({ id: 'held-1' });
    await holdCartScoped('tok', {
      label: 'Order #5',
      cart_data: '{}',
      item_count: 2,
      total_minor: 1500, currency: 'USD',
    });
    expect(mockInvoke).toHaveBeenCalledWith('hold_cart_scoped', {
      sessionToken: 'tok',
      args: {
        label: 'Order #5',
        cart_data: '{}',
        item_count: 2,
        total_minor: 1500, currency: 'USD',
      },
    });
  });

  it('listSalesScoped invokes "list_sales_scoped" with sessionToken', async () => {
    mockInvoke.mockResolvedValue([]);
    await listSalesScoped('tok');
    expect(mockInvoke).toHaveBeenCalledWith('list_sales_scoped', {
      sessionToken: 'tok',
    });
  });

  it('getSaleScoped invokes "get_sale_scoped" with sessionToken + id', async () => {
    mockInvoke.mockResolvedValue(null);
    await getSaleScoped('tok', 'sale-42');
    expect(mockInvoke).toHaveBeenCalledWith('get_sale_scoped', {
      sessionToken: 'tok',
      id: 'sale-42',
    });
  });

  it('overrideLinePriceScoped invokes "override_line_price_scoped" with args(cartId, lineId, newPriceMinor)', async () => {
    mockInvoke.mockResolvedValue(undefined);
    await overrideLinePriceScoped('tok', 'c1', 'l1', 750);
    expect(mockInvoke).toHaveBeenCalledWith('override_line_price_scoped', {
      sessionToken: 'tok',
      args: { cartId: 'c1' as CartId, lineId: 'l1', newPriceMinor: 750 },
    });
  });

  it('finalizeSale invokes "finalize_sale" with sessionToken + saleId', async () => {
    mockInvoke.mockResolvedValue(undefined);
    await finalizeSale('tok', 'sale-1');
    expect(mockInvoke).toHaveBeenCalledWith('finalize_sale', {
      sessionToken: 'tok',
      saleId: 'sale-1',
    });
  });

  it('voidPendingSale invokes "void_pending_sale" with sessionToken + saleId', async () => {
    mockInvoke.mockResolvedValue(undefined);
    await voidPendingSale('tok', 'sale-pending');
    expect(mockInvoke).toHaveBeenCalledWith('void_pending_sale', {
      sessionToken: 'tok',
      saleId: 'sale-pending',
    });
  });

  it('setCartDiscountScoped invokes "set_cart_discount_scoped" with sessionToken + args', async () => {
    mockInvoke.mockResolvedValue(undefined);
    await setCartDiscountScoped('tok', {
      cartId: 'c1' as CartId,
      percent: 10,
      label: 'Senior',
    });
    expect(mockInvoke).toHaveBeenCalledWith('set_cart_discount_scoped', {
      sessionToken: 'tok',
      args: { cartId: 'c1' as CartId, percent: 10, label: 'Senior' },
    });
  });

  it('propagates errors from the backend (does not swallow)', async () => {
    mockInvoke.mockRejectedValueOnce(new Error('DB locked'));
    await expect(startSale({ currency: 'USD' })).rejects.toThrow('DB locked');
  });
});

// ── topology.ts ───────────────────────────────────────────────────

import {
  saveTopology,
  loadTopology,
  applyTopologyDiff,
} from '@/api/topology';

describe('topology.ts IPC contract', () => {
  beforeEach(() => mockInvoke.mockReset());

  it('saveTopology invokes "save_topology" with nodes + wires', async () => {
    mockInvoke.mockResolvedValue(undefined);
    const nodes = [{ id: 'n1', type: 'store', name: 'Main', x: 0, y: 0 }];
    const wires = [{ id: 'w1', from_node_id: 'n1', to_node_id: 'n2', direction: 'one-way' }];
    await saveTopology(nodes, wires);
    expect(mockInvoke).toHaveBeenCalledWith('save_topology', { nodes, wires });
  });

  it('loadTopology invokes "load_topology" with no args', async () => {
    mockInvoke.mockResolvedValue(null);
    await loadTopology();
    expect(mockInvoke).toHaveBeenCalledWith('load_topology', undefined);
  });

  it('applyTopologyDiff invokes "apply_topology_diff" with full diff payload', async () => {
    mockInvoke.mockResolvedValue(undefined);
    const nodes = [{ id: 'n1', type: 'store', name: 'S', x: 0, y: 0 }];
    const wires = [{ id: 'w1', from_node_id: 'n1', to_node_id: 'n2', direction: 'one-way' }];
    const creations = [{ id: 'ws-1', type_key: 'restaurant-pos', store_id: 's1', name: 'POS' }];
    const updates = [{ id: 'ws-2', name: 'Renamed' }];
    const archives = ['ws-old'];
    await applyTopologyDiff('tok', creations, updates, archives, nodes, wires);
    expect(mockInvoke).toHaveBeenCalledWith('apply_topology_diff', {
      sessionToken: 'tok',
      workspaceCreations: creations,
      workspaceUpdates: updates,
      workspaceArchives: archives,
      diagramNodes: nodes,
      diagramWires: wires,
    });
  });

  it('loadTopology returns null when no topology saved', async () => {
    mockInvoke.mockResolvedValue(null);
    const result = await loadTopology();
    expect(result).toBeNull();
  });

  it('saveTopology propagates backend errors', async () => {
    mockInvoke.mockRejectedValueOnce(new Error('duplicate node id: n1'));
    await expect(
      saveTopology([{ id: 'n1', type: 'store', name: 'X', x: 0, y: 0 }], []),
    ).rejects.toThrow('duplicate node id');
  });
});

// ── settings.ts ───────────────────────────────────────────────────

import {
  getReceiptSettings,
  setReceiptSettings,
  getStoreSettings,
  setStoreSettings,
  getHardwareSettings,
  setHardwareSettings,
  getEnabledFeatures,
  completeSetup,
  dismissSetupWizard,
  getSetupStatus,
} from '@/api/settings';

describe('settings.ts IPC contract', () => {
  beforeEach(() => mockInvoke.mockReset());

  it('getReceiptSettings invokes "get_receipt_settings" with no args', async () => {
    mockInvoke.mockResolvedValue({
      showCurrency: false,
      decimalSeparator: 'dot',
      showTax: true,
      footer: '',
      paperWidth: 'standard',
      showTableNumber: false,
      marginTop: 0,
      marginBottom: 0,
      marginLeft: 0,
      marginRight: 0,
    });
    await getReceiptSettings();
    expect(mockInvoke).toHaveBeenCalledWith('get_receipt_settings', undefined);
  });

  it('setReceiptSettings invokes "set_receipt_settings" with args + userId', async () => {
    mockInvoke.mockResolvedValue(undefined);
    const args = {
      showCurrency: true,
      decimalSeparator: 'comma',
      showTax: true,
      footer: 'Thanks!',
      paperWidth: 'narrow',
      showTableNumber: false,
      marginTop: 1,
      marginBottom: 1,
      marginLeft: 1,
      marginRight: 1,
    };
    await setReceiptSettings(args, 'u1');
    expect(mockInvoke).toHaveBeenCalledWith('set_receipt_settings', {
      args,
      userId: 'u1',
    });
  });

  it('getStoreSettings invokes "get_store_settings" with no args', async () => {
    mockInvoke.mockResolvedValue({
      name: 'Test',
      address: '',
      taxId: '',
      currency: 'USD',
      branch: '',
      logo: '',
    });
    await getStoreSettings();
    expect(mockInvoke).toHaveBeenCalledWith('get_store_settings', undefined);
  });

  it('setStoreSettings invokes "set_store_settings" with args + userId', async () => {
    mockInvoke.mockResolvedValue(undefined);
    const args = {
      name: 'New Name',
      address: '123 St',
      taxId: 'TAX-1',
      currency: 'IDR',
      branch: 'main',
      logo: '',
    };
    await setStoreSettings(args, 'u1');
    expect(mockInvoke).toHaveBeenCalledWith('set_store_settings', { args, userId: 'u1' });
  });

  it('getHardwareSettings invokes "get_hardware_settings" with no args', async () => {
    mockInvoke.mockResolvedValue({
      printerConnection: 'auto',
      printerDevicePath: '',
      printerPaperSize: '80',
      scannerDeviceId: '',
      scannerInputMode: 'auto',
    });
    await getHardwareSettings();
    expect(mockInvoke).toHaveBeenCalledWith('get_hardware_settings', undefined);
  });

  it('setHardwareSettings invokes "set_hardware_settings" with args + userId', async () => {
    mockInvoke.mockResolvedValue(undefined);
    const args = {
      printerConnection: 'usb',
      printerDevicePath: '/dev/usb0',
      printerPaperSize: '58',
      scannerDeviceId: 'scanner-1',
      scannerInputMode: 'keyboard',
    };
    await setHardwareSettings(args, 'u1');
    expect(mockInvoke).toHaveBeenCalledWith('set_hardware_settings', { args, userId: 'u1' });
  });

  it('getEnabledFeatures invokes "get_enabled_features" with no args', async () => {
    mockInvoke.mockResolvedValue({ features: {} });
    await getEnabledFeatures();
    expect(mockInvoke).toHaveBeenCalledWith('get_enabled_features', undefined);
  });

  it('completeSetup invokes "complete_setup" with args', async () => {
    mockInvoke.mockResolvedValue(undefined);
    const args = { preset: 'retail', features: ['cloud_sync'], default_currency: 'USD' };
    await completeSetup(args);
    expect(mockInvoke).toHaveBeenCalledWith('complete_setup', { args });
  });

  it('dismissSetupWizard invokes "dismiss_setup_wizard" with no args', async () => {
    mockInvoke.mockResolvedValue(undefined);
    await dismissSetupWizard();
    expect(mockInvoke).toHaveBeenCalledWith('dismiss_setup_wizard', undefined);
  });

  it('getSetupStatus invokes "get_setup_status" with no args', async () => {
    mockInvoke.mockResolvedValue({ completed: true });
    await getSetupStatus();
    expect(mockInvoke).toHaveBeenCalledWith('get_setup_status', undefined);
  });

  it('propagates backend errors', async () => {
    mockInvoke.mockRejectedValueOnce(new Error('permission denied'));
    await expect(getReceiptSettings()).rejects.toThrow('permission denied');
  });
});

// ── products.ts ───────────────────────────────────────────────────

import {
  listProducts,
  lookupProductBySku,
  createProduct,
  deleteProduct,
  adjustStock,
} from '@/api/products';

describe('products.ts IPC contract', () => {
  beforeEach(() => mockInvoke.mockReset());

  it('listProducts invokes "list_products" with no args', async () => {
    mockInvoke.mockResolvedValue([]);
    await listProducts();
    expect(mockInvoke).toHaveBeenCalledWith('list_products', undefined);
  });

  it('lookupProductBySku invokes "lookup_product_by_sku" with sku', async () => {
    mockInvoke.mockResolvedValue(null);
    await lookupProductBySku('SKU-001');
    expect(mockInvoke).toHaveBeenCalledWith('lookup_product_by_sku', { sku: 'SKU-001' });
  });

  it('createProduct invokes "create_product" with CreateProductArgs (includes userId)', async () => {
    mockInvoke.mockResolvedValue({ sku: 'NEW' });
    await createProduct({
      userId: 'u1',
      sku: 'NEW',
      name: 'New',
      priceMinor: 500,
      currency: 'USD',
      initialStock: 10,
      taxRateIds: [],
    });
    expect(mockInvoke).toHaveBeenCalledWith('create_product', {
      args: {
        userId: 'u1',
        sku: 'NEW',
        name: 'New',
        priceMinor: 500,
        currency: 'USD',
        initialStock: 10,
        taxRateIds: [],
      },
    });
  });

  it('deleteProduct invokes "delete_product" with args(userId, sku)', async () => {
    mockInvoke.mockResolvedValue(undefined);
    await deleteProduct({ userId: 'u1', sku: 'OLD' });
    expect(mockInvoke).toHaveBeenCalledWith('delete_product', {
      args: { userId: 'u1', sku: 'OLD' },
    });
  });

  it('adjustStock invokes "adjust_stock" with AdjustStockArgs(sku, delta, reason)', async () => {
    mockInvoke.mockResolvedValue(20);
    await adjustStock({ sku: 'SKU-1', delta: 10, reason: 'restock' });
    expect(mockInvoke).toHaveBeenCalledWith('adjust_stock', {
      args: { sku: 'SKU-1', delta: 10, reason: 'restock' },
    });
  });

  it('propagates backend errors', async () => {
    mockInvoke.mockRejectedValueOnce(new Error('not found'));
    await expect(lookupProductBySku('MISSING')).rejects.toThrow('not found');
  });
});

// ── workspaces.ts ─────────────────────────────────────────────────

import {
  listWorkspacesScoped,
  createWorkspaceInstanceScoped,
  updateWorkspaceInstanceScoped,
  archiveWorkspaceInstanceScoped,
} from '@/api/workspaces';

describe('workspaces.ts IPC contract', () => {
  beforeEach(() => mockInvoke.mockReset());

  it('listWorkspacesScoped invokes "list_workspaces_scoped" with sessionToken', async () => {
    mockInvoke.mockResolvedValue([]);
    await listWorkspacesScoped('tok');
    expect(mockInvoke).toHaveBeenCalledWith('list_workspaces_scoped', { sessionToken: 'tok' });
  });

  it('createWorkspaceInstanceScoped invokes "create_workspace_instance_scoped" with sessionToken + req', async () => {
    mockInvoke.mockResolvedValue(undefined);
    await createWorkspaceInstanceScoped('tok', {
      id: 'ws-1',
      type_key: 'restaurant-pos',
      store_id: 's1',
      name: 'POS 1',
    });
    expect(mockInvoke).toHaveBeenCalledWith('create_workspace_instance_scoped', {
      sessionToken: 'tok',
      req: { id: 'ws-1', type_key: 'restaurant-pos', store_id: 's1', name: 'POS 1' },
    });
  });

  it('updateWorkspaceInstanceScoped invokes "update_workspace_instance_scoped" with sessionToken + instanceId + spread fields', async () => {
    mockInvoke.mockResolvedValue(undefined);
    await updateWorkspaceInstanceScoped('tok', 'ws-1', { name: 'Renamed' });
    expect(mockInvoke).toHaveBeenCalledWith('update_workspace_instance_scoped', {
      sessionToken: 'tok',
      instanceId: 'ws-1',
      name: 'Renamed',
      description: null,
      colour: null,
    });
  });

  it('archiveWorkspaceInstanceScoped invokes "archive_workspace_instance_scoped" with sessionToken + instanceId', async () => {
    mockInvoke.mockResolvedValue(undefined);
    await archiveWorkspaceInstanceScoped('tok', 'ws-old');
    expect(mockInvoke).toHaveBeenCalledWith('archive_workspace_instance_scoped', {
      sessionToken: 'tok',
      instanceId: 'ws-old',
    });
  });

  it('propagates backend errors', async () => {
    mockInvoke.mockRejectedValueOnce(new Error('conflict'));
    await expect(listWorkspacesScoped('tok')).rejects.toThrow('conflict');
  });
});
