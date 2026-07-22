/**
 * Dev-mode mock for @tauri-apps/api/core
 *
 * Provides minimal Tauri IPC stubs so the app can be previewed in a
 * browser without the Rust backend running.  Reload the page after
 * editing this file.
 *
 * Usage — add to vite.config.ts:
 *   resolve: {
 *     alias: [
 *       { find: /^@tauri-apps\/api\/core$/, replacement: '/src/dev-mock/tauri-api.ts' },
 *       ...existing aliases,
 *     ],
 *   },
 */

// ── Mock staff data ────────────────────────────────────────────
const MOCK_STAFF: Record<string, {
  user_id: string;
  pin_hash: string;
  role: string;
  is_active: boolean;
}> = {
  'owner': { user_id: 'owner-1', pin_hash: '1234', role: 'owner', is_active: true },
  'admin': { user_id: 'admin-1', pin_hash: '9999', role: 'manager', is_active: true },
  'kasir': { user_id: 'kasir-1', pin_hash: '1234', role: 'cashier', is_active: true },
};

const MOCK_PRODUCTS = [
  { sku: 'LATTE', name: 'Caff\u00e8 Latte', category: 'Hot Drinks', price: { minor_units: 450, currency: 'USD' }, barcode: '4901234567890', in_stock: true, stock_qty: 50, tax_rate_ids: [], created_at: new Date().toISOString(), price_updated_at: new Date().toISOString(), product_type: 'restaurant' },
  { sku: 'MLATTE', name: 'Matcha Latte', category: 'Hot Drinks', price: { minor_units: 520, currency: 'USD' }, barcode: '4901234567891', in_stock: true, stock_qty: 30, tax_rate_ids: [], created_at: new Date().toISOString(), price_updated_at: new Date().toISOString(), product_type: 'restaurant' },
  { sku: 'ESPR', name: 'Espresso Shot', category: 'Hot Drinks', price: { minor_units: 300, currency: 'USD' }, barcode: '4901234567892', in_stock: true, stock_qty: 80, tax_rate_ids: [], created_at: new Date().toISOString(), price_updated_at: new Date().toISOString(), product_type: 'restaurant' },
  { sku: 'HCHOCO', name: 'Hot Chocolate', category: 'Hot Drinks', price: { minor_units: 420, currency: 'USD' }, barcode: '4901234567893', in_stock: true, stock_qty: 25, tax_rate_ids: [], created_at: new Date().toISOString(), price_updated_at: new Date().toISOString(), product_type: 'restaurant' },
  { sku: 'ICCOFF', name: 'Iced Coffee', category: 'Cold Drinks', price: { minor_units: 380, currency: 'USD' }, barcode: '4901234567894', in_stock: true, stock_qty: 40, tax_rate_ids: [], created_at: new Date().toISOString(), price_updated_at: new Date().toISOString(), product_type: 'restaurant' },
  { sku: 'ICTEA', name: 'Iced Tea', category: 'Cold Drinks', price: { minor_units: 250, currency: 'USD' }, barcode: '4901234567895', in_stock: true, stock_qty: 60, tax_rate_ids: [], created_at: new Date().toISOString(), price_updated_at: new Date().toISOString(), product_type: 'restaurant' },
  { sku: 'JUICE-O', name: 'Orange Juice', category: 'Cold Drinks', price: { minor_units: 350, currency: 'USD' }, barcode: '4901234567904', in_stock: true, stock_qty: 20, tax_rate_ids: [], created_at: new Date().toISOString(), price_updated_at: new Date().toISOString(), product_type: 'restaurant' },
  { sku: 'LEMONADE', name: 'Lemonade', category: 'Cold Drinks', price: { minor_units: 300, currency: 'USD' }, barcode: '4901234567897', in_stock: true, stock_qty: 35, tax_rate_ids: [], created_at: new Date().toISOString(), price_updated_at: new Date().toISOString(), product_type: 'restaurant' },
  { sku: 'PBAGEL', name: 'Plain Bagel', category: 'Food', price: { minor_units: 250, currency: 'USD' }, barcode: '4901234567898', in_stock: true, stock_qty: 15, tax_rate_ids: [], created_at: new Date().toISOString(), price_updated_at: new Date().toISOString(), product_type: 'restaurant' },
  { sku: 'SBAGEL', name: 'Sesame Bagel', category: 'Food', price: { minor_units: 280, currency: 'USD' }, barcode: '4901234567899', in_stock: true, stock_qty: 12, tax_rate_ids: [], created_at: new Date().toISOString(), price_updated_at: new Date().toISOString(), product_type: 'restaurant' },
  { sku: 'CROISS', name: 'Butter Croissant', category: 'Food', price: { minor_units: 320, currency: 'USD' }, barcode: '4901234567800', in_stock: true, stock_qty: 18, tax_rate_ids: [], created_at: new Date().toISOString(), price_updated_at: new Date().toISOString(), product_type: 'restaurant' },
  { sku: 'CSAND', name: 'Chicken Sandwich', category: 'Food', price: { minor_units: 550, currency: 'USD' }, barcode: '4901234567801', in_stock: true, stock_qty: 10, tax_rate_ids: [], created_at: new Date().toISOString(), price_updated_at: new Date().toISOString(), product_type: 'restaurant' },
  { sku: 'VSAND', name: 'Veggie Sandwich', category: 'Food', price: { minor_units: 480, currency: 'USD' }, barcode: '4901234567802', in_stock: true, stock_qty: 8, tax_rate_ids: [], created_at: new Date().toISOString(), price_updated_at: new Date().toISOString(), product_type: 'restaurant' },
  { sku: 'WATER-S', name: 'Sparkling Water', category: 'Cold Drinks', price: { minor_units: 180, currency: 'USD' }, barcode: '4901234567803', in_stock: true, stock_qty: 150, tax_rate_ids: [], created_at: new Date().toISOString(), price_updated_at: new Date().toISOString(), product_type: 'restaurant' },
  { sku: 'BROWNIE', name: 'Fudge Brownie', category: 'Snacks', price: { minor_units: 300, currency: 'USD' }, barcode: '4901234567804', in_stock: false, stock_qty: 0, tax_rate_ids: [], created_at: new Date().toISOString(), price_updated_at: new Date().toISOString(), product_type: 'restaurant' },
  { sku: 'CMUFFIN', name: 'Chocolate Muffin', category: 'Snacks', price: { minor_units: 280, currency: 'USD' }, barcode: '4901234567805', in_stock: false, stock_qty: 0, tax_rate_ids: [], created_at: new Date().toISOString(), price_updated_at: new Date().toISOString(), product_type: 'restaurant' },
  { sku: 'NUTS', name: 'Mixed Nuts', category: 'Snacks', price: { minor_units: 400, currency: 'USD' }, barcode: '4901234567806', in_stock: true, stock_qty: 22, tax_rate_ids: [], created_at: new Date().toISOString(), price_updated_at: new Date().toISOString(), product_type: 'restaurant' },
  { sku: 'CHIPS', name: 'Potato Chips', category: 'Snacks', price: { minor_units: 200, currency: 'USD' }, barcode: '4901234567807', in_stock: true, stock_qty: 55, tax_rate_ids: [], created_at: new Date().toISOString(), price_updated_at: new Date().toISOString(), product_type: 'restaurant' },
];

const MOCK_CATEGORIES = [
  { id: 'cat-cold-drinks', name: 'Cold Drinks', colour: '#3498db', icon: 'dots-1' },
  { id: 'cat-hot-drinks', name: 'Hot Drinks', colour: '#e74c3c', icon: 'dots-2' },
  { id: 'cat-food', name: 'Food', colour: '#2ecc71', icon: 'dots-3' },
  { id: 'cat-snacks', name: 'Snacks', colour: '#f39c12', icon: 'dots-4' },
];

const MOCK_STORE = {
  id: 'store-1',
  name: 'TOKO TEST',
  address: 'Jl. Contoh No. 123',
  tax_id: 'TAX-001',
  currency: 'IDR',
  timezone: 'Asia/Jakarta',
  is_primary: true,
  created_at: new Date().toISOString(),
  updated_at: new Date().toISOString(),
};

const MOCK_CURRENCIES = [
  { code: 'IDR', name: 'Indonesian Rupiah', minor_exponent: 0, symbol: 'Rp' },
  { code: 'USD', name: 'US Dollar', minor_exponent: 2, symbol: '$' },
  { code: 'JPY', name: 'Japanese Yen', minor_exponent: 0, symbol: '¥' },
];

const MOCK_TERMINAL = {
  id: 'term-1',
  name: 'Terminal 1',
  deviceId: 'device-001',
  isActive: true,
  lastSeenAt: new Date().toISOString(),
  metadata: null,
  createdAt: new Date().toISOString(),
  updatedAt: new Date().toISOString(),
};

const MOCK_STAFF_LIST = [
  { id: 'staff-1', username: 'owner', display_name: 'Owner', role_id: '1', role_name: 'owner', is_active: true },
  { id: 'staff-2', username: 'kasir', display_name: 'Kasir', role_id: '3', role_name: 'cashier', is_active: true },
];

const MOCK_ROLES = [
  { id: '1', name: 'owner', description: 'Full system access' },
  { id: '2', name: 'manager', description: 'Management access' },
  { id: '3', name: 'cashier', description: 'POS operations' },
];

const MOCK_CUSTOMERS = [
  { id: 'cust-1', name: 'John Doe', email: 'john@example.com', phone: '08123456789', notes: 'Regular customer', created_at: new Date().toISOString(), updated_at: new Date().toISOString() },
  { id: 'cust-2', name: 'Jane Smith', email: 'jane@example.com', phone: '08987654321', notes: '', created_at: new Date().toISOString(), updated_at: new Date().toISOString() },
];

const MOCK_INVENTORY_LOCATIONS = [
  { id: 'loc-1', name: 'Main Store', type: 'store' as const, description: 'Main retail location', is_active: true, created_at: new Date().toISOString(), updated_at: new Date().toISOString() },
  { id: 'loc-2', name: 'Warehouse', type: 'warehouse' as const, description: 'Central warehouse', is_active: true, created_at: new Date().toISOString(), updated_at: new Date().toISOString() },
];

const MOCK_WORKSPACES = [
  { instance_id: 'ws-1', type_key: 'store-pos', store_id: 'store-1', store_name: 'TOKO TEST', name: 'Store POS', description: 'Point of Sale', icon: 'shopping-cart', layout_mode: 'default', colour: '#10b981', is_default: true },
  { instance_id: 'ws-2', type_key: 'restaurant-pos', store_id: 'store-1', store_name: 'TOKO TEST', name: 'Restaurant POS', description: 'Table service', icon: 'restaurant', layout_mode: 'fullscreen', colour: '#ef4444', is_default: false },
  { instance_id: 'ws-3', type_key: 'kds', store_id: 'store-1', store_name: 'TOKO TEST', name: 'Kitchen Display', description: 'Order display', icon: 'utensils', layout_mode: 'kds', colour: '#f59e0b', is_default: false },
  { instance_id: 'ws-4', type_key: 'inventory', store_id: 'store-1', store_name: 'TOKO TEST', name: 'Inventory Management', description: 'Stock management', icon: 'package', layout_mode: 'default', colour: '#3b82f6', is_default: false },
  { instance_id: 'ws-5', type_key: 'admin', store_id: 'store-1', store_name: 'TOKO TEST', name: 'Admin', description: 'Settings & management', icon: 'settings', layout_mode: 'default', colour: '#8b5cf6', is_default: false },
];

// ── Lockout state (for E2E rate-limit tests) ──────────────────
const loginAttempts: Record<string, number> = {};
const LOCKOUT_THRESHOLD = 5;
// LOCKOUT_DURATION_MS = 30_000 is defined for documentation;
// the mock uses a simple attempt-count lockout that resets on
// successful login to keep the dev loop fast.

// ── Cart state (for realistic E2E totals) ───────────────────────
interface CartLine {
  sku: string;
  name: string;
  price: { minor_units: number; currency: string };
  qty: number;
}
let cartState: { lines: CartLine[] } = { lines: [] };

// ── Active shift state (for pay-btn-enabled E2E test) ──────────
let mockActiveShift: Record<string, unknown> | null = {
  id: 'shift-1', userId: 'user-1', terminalId: null, openedAt: new Date().toISOString(), closedAt: null,
  openingBalanceMinor: 0, closingBalanceMinor: null, expectedCashMinor: null, cashDifferenceMinor: null,
  totalSalesMinor: 0, totalCashMinor: 0, totalCardMinor: 0, totalOtherMinor: 0,
  totalVoidsMinor: 0, totalRefundsMinor: 0, totalPayoutsMinor: 0, notes: '', status: 'open',
  createdAt: new Date().toISOString(), updatedAt: new Date().toISOString(),
};
const handlers: Record<string, (args: unknown) => unknown> = {
  // ═══════════════════════════════════════════════════════════════
  // AUTH / STAFF
  // ═══════════════════════════════════════════════════════════════

  'staff_check_username': (args) => {
    const { username } = args as { username: string };
    const staff = MOCK_STAFF[username.toLowerCase()];
    if (!staff) return { found: false, is_active: false };
    return { found: true, is_active: staff.is_active };
  },

  'staff_login': (args) => {
    const { username, pin } = args as { username: string; pin: string };
    const key = username.toLowerCase();
    const staff = MOCK_STAFF[key];

    // Check lockout.
    const attempts = loginAttempts[key] ?? 0;
    if (attempts >= LOCKOUT_THRESHOLD) {
      throw new Error('Account locked. Too many failed attempts.');
    }

    if (!staff || pin !== staff.pin_hash) {
      loginAttempts[key] = attempts + 1;
      throw new Error('Invalid credentials');
    }

    // Reset on success.
    delete loginAttempts[key];
    return {
      session: {
        user_id: staff.user_id,
        display_name: staff.role.charAt(0).toUpperCase() + staff.role.slice(1),
        role_name: staff.role,
        role_id: staff.role === 'owner' ? '1' : staff.role === 'manager' ? '2' : '3',
      },
    };
  },

  'bootstrap_owner': (_args) => {
    return {
      session: { user_id: 'owner-1', display_name: 'Owner', role_name: 'owner', role_id: '1' },
    };
  },

  'create_session': (args) => {
    const a = args as { args: { user_id: string; role_id: string; store_id: string; instance_id: string; type_key: string; terminal_id: string } };
    const { user_id, role_id, store_id, instance_id, type_key, terminal_id } = a.args ?? a;
    return {
      session_token: `mock-session-${Date.now()}`,
      context: { userId: user_id, roleId: role_id, storeId: store_id, instanceId: instance_id, typeKey: type_key, terminalId: terminal_id },
    };
  },

  'destroy_session': () => null,

  // ═══════════════════════════════════════════════════════════════
  // BOOT / SETUP
  // ═══════════════════════════════════════════════════════════════

  'resolve_boot_store': () => ({
    is_bound: true,
    store_id: 'store-1',
    instance_id: 'ws-1',
  }),

  'get_setup_status': () => ({ completed: true, preset: 'retail' }),
  'complete_setup': () => null,
  'dismiss_setup_wizard': () => null,

  // ═══════════════════════════════════════════════════════════════
  // SYSTEM / PING
  // ═══════════════════════════════════════════════════════════════

  'ping': () => 'pong',
  'version': () => ({ name: 'oz-pos', version: '0.0.9', rustVersion: '1.80', target: 'x86_64' }),
  'get_local_ip': () => '192.168.1.100',

  // ═══════════════════════════════════════════════════════════════
  // LICENSE
  // ═══════════════════════════════════════════════════════════════

  'list_all_features': () => ({
    features: [
      { key: 'sales', name: 'Sales', description: 'Point of sale transactions', group: 'Core', enabled: true, dependencies: [] },
      { key: 'inventory', name: 'Inventory', description: 'Stock management', group: 'Core', enabled: true, dependencies: ['sales'] },
      { key: 'reporting', name: 'Reporting', description: 'Sales and inventory reports', group: 'Reporting', enabled: false, dependencies: ['sales'] },
      { key: 'staff', name: 'Staff', description: 'Staff management', group: 'Staff', enabled: true, dependencies: [] },
      { key: 'settings', name: 'Settings', description: 'System settings', group: 'Core', enabled: true, dependencies: [] },
    ],
  }),
  'set_feature': () => ({ success: true, features: [], auto_enabled: [] }),
  'set_features_bulk': () => ({ features: [] }),

  'plugin:updater|check': () => null,

  'get_license_status': () => ({ is_valid: true, license_type: 'Pro', expires_at: null, is_active: true, status: 'valid', payload: null, message: null }),
  'check_license_status': () => ({ tenantId: 'tenant-1', status: 'active', tier: 'Pro', active: true, expiresAt: null, graceUntil: null, maxStores: 5 }),
  'get_machine_id': () => 'mock-machine-id-001',
  'get_device_id': () => 'mock-device-id-001',
  'activate_license': () => true,
  'renew_license': () => true,

  // ═══════════════════════════════════════════════════════════════
  // STORES
  // ═══════════════════════════════════════════════════════════════

  'list_store_profiles': () => [MOCK_STORE],
  'get_store_profile': () => MOCK_STORE,
  'get_primary_store': () => MOCK_STORE,
  'create_store_profile': (args) => ({ ...MOCK_STORE, ...(args as Record<string, unknown>) }),
  'update_store_profile': (args) => ({ ...MOCK_STORE, ...(args as Record<string, unknown>) }),
  'set_primary_store': () => MOCK_STORE,
  'delete_store_profile': () => null,

  // ═══════════════════════════════════════════════════════════════
  // WORKSPACES (ADR #4 / #7)
  // ═══════════════════════════════════════════════════════════════

  'list_workspaces': () => MOCK_WORKSPACES,
  'list_workspaces_scoped': () => MOCK_WORKSPACES,
  'list_workspace_screens': () => [],
  'list_workspace_screens_scoped': () => [],
  'get_workspace_instance_scoped': (args) => {
    const { instanceId } = args as { instanceId: string };
    return MOCK_WORKSPACES.find(w => w.instance_id === instanceId) ?? MOCK_WORKSPACES[0];
  },
  'create_workspace_instance_scoped': (args) => {
    const req = (args as { req: Record<string, unknown> }).req;
    return { instance_id: `ws-${Date.now()}`, ...req };
  },
  'update_workspace_instance_scoped': (args) => args,
  'delete_workspace_instance_scoped': () => null,
  'set_default_instance_scoped': () => null,
  'list_screens_scoped': () => [],

  // ═══════════════════════════════════════════════════════════════
  // TERMINALS
  // ═══════════════════════════════════════════════════════════════

  'list_terminals': () => [MOCK_TERMINAL],
  'list_terminals_scoped': () => [MOCK_TERMINAL],
  'get_terminal': () => MOCK_TERMINAL,
  'get_terminal_scoped': () => MOCK_TERMINAL,
  'register_terminal': () => ({ id: 'term-new' }),
  'register_terminal_scoped': () => ({ id: 'term-new' }),
  'update_terminal': () => ({ id: 'term-1' }),
  'update_terminal_scoped': () => ({ id: 'term-1' }),
  'ping_terminal': () => null,
  'ping_terminal_scoped': () => null,
  'delete_terminal': () => null,
  'delete_terminal_scoped': () => null,
  'get_terminal_profile': () => ({ terminalId: 'term-1', profileType: 'desktop', lockedScreen: null, updatedAt: new Date().toISOString() }),
  'get_terminal_profile_scoped': () => ({ terminalId: 'term-1', profileType: 'desktop', lockedScreen: null, updatedAt: new Date().toISOString() }),
  'set_terminal_profile': () => null,
  'set_terminal_profile_scoped': () => null,
  'list_terminal_profiles': () => [],
  'list_terminal_profiles_scoped': () => [],
  'delete_terminal_profile': () => null,
  'delete_terminal_profile_scoped': () => null,
  'list_terminal_overrides': () => [],
  'list_terminal_overrides_scoped': () => [],
  'set_terminal_override': () => null,
  'set_terminal_override_scoped': () => null,
  'delete_terminal_override': () => null,
  'delete_terminal_override_scoped': () => null,
  'get_device_binding': () => ({ bounded: true, boundStoreId: 'store-1', boundInstanceId: 'ws-1', signatureValid: true }),
  'get_device_binding_scoped': () => ({ bounded: true, boundStoreId: 'store-1', boundInstanceId: 'ws-1', signatureValid: true }),
  'set_device_binding': () => null,
  'set_device_binding_scoped': () => null,
  'clear_device_binding': () => null,
  'clear_device_binding_scoped': () => null,

  // ═══════════════════════════════════════════════════════════════
  // SETTINGS
  // ═══════════════════════════════════════════════════════════════

  'get_store_settings': () => ({
    name: 'TOKO TEST', address: 'Jl. Contoh No. 123', taxId: 'TAX-001', currency: 'IDR', branch: 'Cabang A', logo: '',
  }),
  'set_store_settings': () => null,
  'set_store_settings_scoped': () => null,

  'get_receipt_settings': () => ({
    showCurrency: true, decimalSeparator: 'dot', showTax: true, footer: 'Terima kasih',
    paperWidth: 'standard', showTableNumber: false,
    marginTop: 0, marginBottom: 0, marginLeft: 0, marginRight: 0,
  }),
  'set_receipt_settings': () => null,
  'get_report_schedule': () => ({
    enabled: false,
    cadence: 'daily',
    report_types: ['daily_revenue', 'top_products'],
    recipients: ['admin@example.com'],
    send_at_time: '08:00',
    timezone: 'UTC',
    lookback_days: 1,
  }),
  'save_report_schedule': () => null,

  'set_receipt_settings_scoped': () => null,

  'get_enabled_features': () => ({ features: ['sales', 'inventory', 'reporting', 'staff', 'settings'] }),
  'get_setting': () => '',

  'get_user_preferences': () => ({ cardsize: '2', fontsize: '1', 'font-smoothing': 'antialiased' }),
  'get_user_preferences_scoped': () => ({ cardsize: '2', fontsize: '1', 'font-smoothing': 'antialiased' }),
  'set_user_preferences': () => null,
  'set_user_preferences_scoped': () => null,

  'get_hardware_settings': () => ({
    printerConnection: 'usb', printerDevicePath: '', printerPaperSize: '80mm',
    scannerDeviceId: '', scannerInputMode: 'usb',
  }),
  'set_hardware_settings': () => null,
  'set_hardware_settings_scoped': () => null,

  'get_credit_settings': () => ({ enabled: false, reminderIntervalHours: 24, maxLimitMinor: 1000000 }),
  'set_credit_settings': () => null,
  'set_credit_settings_scoped': () => null,
  'list_credit_sales': () => [],
  'settle_credit': () => null,
  'settle_credit_scoped': () => null,

  'seed_default_roles_scoped': () => 3,

  // ═══════════════════════════════════════════════════════════════
  // BRANDING
  // ═══════════════════════════════════════════════════════════════

  'get_brand_settings': () => ({
    primary_colour: '#10b981',
    logo_path: null,
    store_name: 'OZ-POS Demo',
    colour_hover: null,
  }),
  'set_brand_primary_colour': () => null,
  'set_brand_logo_path': () => null,
  'set_brand_store_name': () => null,
  'pick_logo_file': () => null,

  // ═══════════════════════════════════════════════════════════════
  // PRODUCTS
  // ═══════════════════════════════════════════════════════════════

  'list_products': () => MOCK_PRODUCTS,
  'list_products_scoped': () => MOCK_PRODUCTS,
  'get_products': () => ({ products: MOCK_PRODUCTS }),
  'search_products': (_args) => ({ products: MOCK_PRODUCTS }),
  'create_product': () => ({ sku: 'SKU-NEW' }),
  'create_product_scoped': () => ({ sku: 'SKU-NEW' }),
  'update_product': () => ({ sku: 'SKU-UPD' }),
  'update_product_scoped': () => ({ sku: 'SKU-UPD' }),
  'delete_product': () => null,
  'delete_product_scoped': () => null,

  'lookup_product_by_sku': (args) => {
    const { sku } = args as { sku: string };
    return MOCK_PRODUCTS.find(p => p.sku === sku) ?? null;
  },
  'lookup_product_by_sku_scoped': (args) => {
    const { sku } = args as { sku: string };
    return MOCK_PRODUCTS.find(p => p.sku === sku) ?? null;
  },
  'lookup_by_barcode': (args) => {
    const { barcode } = args as { barcode: string };
    return MOCK_PRODUCTS.find(p => p.barcode === barcode) ?? null;
  },
  'lookup_by_barcode_scoped': (args) => {
    const { barcode } = args as { barcode: string };
    return MOCK_PRODUCTS.find(p => p.barcode === barcode) ?? null;
  },

  'get_product_track_serial': () => false,
  'get_product_stock': () => ({ quantity: 50 }),

  'adjust_stock': () => 50,
  'adjust_stock_scoped': () => 50,

  'list_product_variants': () => [],
  'get_product_variant': () => null,
  'create_product_variant': () => ({ sku: 'VAR-NEW' }),
  'update_product_variant': () => ({ sku: 'VAR-UPD' }),
  'delete_product_variant': () => null,

  // ═══════════════════════════════════════════════════════════════
  // CATEGORIES
  // ═══════════════════════════════════════════════════════════════

  'list_categories': () => MOCK_CATEGORIES,
  'create_category': () => ({ id: 'cat-new' }),
  'update_category': () => ({ id: 'cat-upd' }),
  'delete_category': () => null,

  // ═══════════════════════════════════════════════════════════════
  // SALES / CART
  // ═══════════════════════════════════════════════════════════════

  'start_sale': () => { cartState = { lines: [] }; return { cartId: `mock-cart-${Date.now()}`, deduction_location_id: 'default-loc', deductionLocationId: 'default-loc' }; },
  'start_sale_scoped': () => { cartState = { lines: [] }; return { cartId: `mock-cart-${Date.now()}`, deduction_location_id: 'default-loc', deductionLocationId: 'default-loc' }; },

  'add_line': (args) => {
    const { productSku, qty } = (args as { productSku?: string; qty?: number }) ?? {};
    const product = MOCK_PRODUCTS.find(p => p.sku === productSku);
    if (product) {
      const existing = cartState.lines.find(l => l.sku === productSku);
      if (existing) {
        existing.qty += qty ?? 1;
      } else {
        cartState.lines.push({ sku: product.sku, name: product.name, price: product.price, qty: qty ?? 1 });
      }
    }
    const lineTotal = product ? product.price.minor_units * (qty ?? 1) : 0;
    return { lineId: `mock-line-${Date.now()}`, lineTotal };
  },
  'add_line_scoped': (args) => {
    const { productSku, qty } = (args as { productSku?: string; qty?: number }) ?? {};
    const product = MOCK_PRODUCTS.find(p => p.sku === productSku);
    if (product) {
      const existing = cartState.lines.find(l => l.sku === productSku);
      if (existing) {
        existing.qty += qty ?? 1;
      } else {
        cartState.lines.push({ sku: product.sku, name: product.name, price: product.price, qty: qty ?? 1 });
      }
    }
    const lineTotal = product ? product.price.minor_units * (qty ?? 1) : 0;
    return { lineId: `mock-line-${Date.now()}`, lineTotal };
  },

  'complete_sale': () => {
    const minorTotal = cartState.lines.reduce((sum, l) => sum + l.price.minor_units * l.qty, 0);
    const lineCount = cartState.lines.length;
    cartState = { lines: [] };
    return { saleId: `mock-sale-${Date.now()}`, total: { minor_units: minorTotal, currency: 'USD' }, lineCount };
  },
  'complete_sale_scoped': () => {
    const minorTotal = cartState.lines.reduce((sum, l) => sum + l.price.minor_units * l.qty, 0);
    const lineCount = cartState.lines.length;
    cartState = { lines: [] };
    return { saleId: `mock-sale-${Date.now()}`, total: { minor_units: minorTotal, currency: 'USD' }, lineCount };
  },
  'complete_sale_with_resolved_shortfalls_scoped': () => {
    const minorTotal = cartState.lines.reduce((sum, l) => sum + l.price.minor_units * l.qty, 0);
    const lineCount = cartState.lines.length;
    cartState = { lines: [] };
    return { saleId: `mock-sale-${Date.now()}`, total: { minor_units: minorTotal, currency: 'USD' }, lineCount };
  },

  'get_sale': () => null,
  'get_sale_scoped': () => null,

  'set_cart_discount': () => null,
  'set_cart_discount_scoped': () => null,

  'override_line_price': () => null,
  'override_line_price_scoped': () => null,

  'hold_cart': () => ({ id: 'held-mock-1' }),
  'hold_cart_scoped': () => ({ id: 'held-mock-1' }),
  'list_active_carts': () => ({ carts: [] }),
  'get_active_cart': () => null,
  'list_held_carts': () => [],
  'list_held_carts_scoped': () => [],
  'list_open_bills': () => [],
  'list_open_bills_scoped': () => [],
  'get_held_cart': () => null,
  'get_held_cart_scoped': () => null,
  'delete_held_cart': () => null,
  'delete_held_cart_scoped': () => null,

  'list_sales': () => [],
  'list_sales_scoped': () => [],
  'void_sale': () => ({ id: 'voided-sale', status: 'voided', total: { minor_units: 0, currency: 'IDR' }, line_count: 0, created_at: new Date().toISOString() }),
  'void_sale_scoped': () => ({ id: 'voided-sale', status: 'voided', total: { minor_units: 0, currency: 'IDR' }, line_count: 0, created_at: new Date().toISOString() }),

  'lookup_sale_by_receipt_barcode': () => null,
  'lookup_sale_by_receipt_barcode_scoped': () => null,

  'process_refund': () => ({ refundId: 'refund-1', totalMinor: 0 }),
  'process_refund_scoped': () => ({ refundId: 'refund-1', totalMinor: 0 }),
  'list_refunds': () => [],
  'list_refunds_scoped': () => [],

  'export_daily_summary': () => [],
  'export_daily_summary_scoped': () => [],
  'export_sales_by_hour': () => [],
  'export_sales_by_hour_scoped': () => [],
  'export_eod_report': () => null,
  'export_eod_report_scoped': () => null,

  'print_sales_receipt': () => ({ printed: true }),

  'get_cart_deduction_location': () => ({ locationId: 'loc-1', locationName: 'Main Store' }),
  'override_cart_deduction_location_scoped': () => null,

  // ═══════════════════════════════════════════════════════════════
  // CURRENCY
  // ═══════════════════════════════════════════════════════════════

  'currency_info': () => ({ code: 'IDR', exponent: 0 }),
  'list_currencies': () => MOCK_CURRENCIES,
  'get_default_currency': () => 'IDR',
  'set_default_currency': () => null,
  'list_exchange_rates': () => [],
  'create_exchange_rate': () => null,
  'delete_exchange_rate': () => null,

  // ═══════════════════════════════════════════════════════════════
  // CUSTOMERS
  // ═══════════════════════════════════════════════════════════════

  'list_customers': () => MOCK_CUSTOMERS,
  'get_customer': (args) => {
    const { id } = args as { id: string };
    return MOCK_CUSTOMERS.find(c => c.id === id) ?? null;
  },
  'create_customer': () => ({ id: 'cust-new', name: 'New Customer', email: null, phone: null, notes: '', created_at: new Date().toISOString(), updated_at: new Date().toISOString() }),
  'update_customer': () => ({ id: 'cust-upd', name: 'Updated', email: null, phone: null, notes: '', created_at: new Date().toISOString(), updated_at: new Date().toISOString() }),
  'delete_customer': () => null,

  // ═══════════════════════════════════════════════════════════════
  // STAFF MANAGEMENT
  // ═══════════════════════════════════════════════════════════════

  'list_staff': () => MOCK_STAFF_LIST,
  'list_roles': () => MOCK_ROLES,
  'create_staff': () => ({ id: 'staff-new', username: 'newuser', display_name: 'New User', role_id: '3', role_name: 'cashier', is_active: true }),
  'update_staff': () => ({ id: 'staff-upd', username: 'updated', display_name: 'Updated', role_id: '3', role_name: 'cashier', is_active: true }),

  // ═══════════════════════════════════════════════════════════════
  // SHIFTS
  // ═══════════════════════════════════════════════════════════════

  'get_active_shift': () => mockActiveShift,
  'get_active_shift_scoped': () => mockActiveShift,
  'open_shift': () => {
    mockActiveShift = {
      id: 'shift-1', userId: 'user-1', terminalId: null, openedAt: new Date().toISOString(), closedAt: null,
      openingBalanceMinor: 0, closingBalanceMinor: null, expectedCashMinor: null, cashDifferenceMinor: null,
      totalSalesMinor: 0, totalCashMinor: 0, totalCardMinor: 0, totalOtherMinor: 0,
      totalVoidsMinor: 0, totalRefundsMinor: 0, totalPayoutsMinor: 0, notes: '', status: 'open',
      createdAt: new Date().toISOString(), updatedAt: new Date().toISOString(),
    };
    return mockActiveShift;
  },
  'open_shift_scoped': () => {
    mockActiveShift = {
      id: 'shift-1', userId: 'user-1', terminalId: null, openedAt: new Date().toISOString(), closedAt: null,
      openingBalanceMinor: 0, closingBalanceMinor: null, expectedCashMinor: null, cashDifferenceMinor: null,
      totalSalesMinor: 0, totalCashMinor: 0, totalCardMinor: 0, totalOtherMinor: 0,
      totalVoidsMinor: 0, totalRefundsMinor: 0, totalPayoutsMinor: 0, notes: '', status: 'open',
      createdAt: new Date().toISOString(), updatedAt: new Date().toISOString(),
    };
    return mockActiveShift;
  },
  'close_shift': () => {
    mockActiveShift = null;
    return {
      id: 'shift-1', userId: 'user-1', terminalId: null, openedAt: new Date().toISOString(), closedAt: new Date().toISOString(),
      openingBalanceMinor: 100000, closingBalanceMinor: 150000, expectedCashMinor: 150000, cashDifferenceMinor: 0,
      totalSalesMinor: 50000, totalCashMinor: 50000, totalCardMinor: 0, totalOtherMinor: 0,
      totalVoidsMinor: 0, totalRefundsMinor: 0, totalPayoutsMinor: 0, notes: '', status: 'closed',
      createdAt: new Date().toISOString(), updatedAt: new Date().toISOString(),
    };
  },
  'close_shift_scoped': () => {
    mockActiveShift = null;
    return {
      id: 'shift-1', userId: 'user-1', terminalId: null, openedAt: new Date().toISOString(), closedAt: new Date().toISOString(),
      openingBalanceMinor: 100000, closingBalanceMinor: 150000, expectedCashMinor: 150000, cashDifferenceMinor: 0,
      totalSalesMinor: 50000, totalCashMinor: 50000, totalCardMinor: 0, totalOtherMinor: 0,
      totalVoidsMinor: 0, totalRefundsMinor: 0, totalPayoutsMinor: 0, notes: '', status: 'closed',
      createdAt: new Date().toISOString(), updatedAt: new Date().toISOString(),
    };
  },
  'list_shifts': () => [],
  'get_shift': () => null,
  'get_shift_report': () => null,
  'create_cash_payout': () => null,

  // ═══════════════════════════════════════════════════════════════
  // INVENTORY
  // ═══════════════════════════════════════════════════════════════

  'create_inventory_location': () => 'loc-new',
  'list_inventory_locations': () => MOCK_INVENTORY_LOCATIONS,
  'update_inventory_location': () => null,
  'deactivate_inventory_location': () => null,

  'set_workspace_inventory_locations': () => null,
  'get_workspace_inventory_locations': () => [],

  'start_inventory_shift': () => ({
    id: 'inv-shift-1', user_id: 'user-1', location_id: 'loc-1', terminal_id: null,
    started_at: new Date().toISOString(), ended_at: null, status: 'active', notes: '',
  }),
  'end_inventory_shift': () => null,
  'get_active_inventory_shift': () => null,
  'list_inventory_shifts': () => [],

  'create_inventory_transaction': () => 'txn-new',
  'list_inventory_transactions': () => [],
  'get_inventory_transaction': () => null,

  'set_stock_threshold': () => null,
  'get_stock_thresholds': () => [],
  'delete_stock_threshold': () => null,

  'finalize_sale': () => null,
  'void_pending_sale': () => null,

  // ═══════════════════════════════════════════════════════════════
  // INVENTORY COUNTS
  // ═══════════════════════════════════════════════════════════════

  'create_stock_count': () => ({ id: 'count-1', count_number: 'SC-001', status: 'draft', count_type: 'full', notes: '', counted_by: null, created_at: new Date().toISOString(), completed_at: null, updated_at: new Date().toISOString() }),
  'get_stock_count': () => null,
  'list_stock_counts': () => [],
  'get_count_lines': () => [],
  'add_count_line': () => null,
  'update_count_line': () => null,
  'remove_count_line': () => null,
  'complete_stock_count': () => [],
  'update_stock_count_status': () => null,
  'list_stock_adjustments': () => [],

  // ═══════════════════════════════════════════════════════════════
  // STOCK TRANSFERS
  // ═══════════════════════════════════════════════════════════════

  'create_stock_transfer': () => null,
  'get_stock_transfer': () => null,
  'list_stock_transfers': () => [],
  'get_stock_transfer_lines': () => [],
  'add_stock_transfer_line': () => null,
  'remove_stock_transfer_line': () => null,
  'send_stock_transfer': () => null,
  'receive_stock_transfer': () => null,
  'cancel_stock_transfer': () => null,

  // ═══════════════════════════════════════════════════════════════
  // KDS
  // ═══════════════════════════════════════════════════════════════

  'list_kds_orders': () => [],
  'list_kds_orders_scoped': () => [],
  'get_kds_queue': () => [],
  'get_kds_queue_scoped': () => [],
  'update_kds_status': () => null,
  'update_kds_status_scoped': () => null,
  'create_kds_order_from_sale': () => [],
  'create_kds_order_from_sale_scoped': () => [],
  'get_kds_order': () => null,
  'get_kds_order_scoped': () => null,

  // ═══════════════════════════════════════════════════════════════
  // PROMOTIONS
  // ═══════════════════════════════════════════════════════════════

  'list_promotions': () => [],
  'list_promotions_scoped': () => [],
  'get_promotion': () => null,
  'get_promotion_scoped': () => null,
  'create_promotion': () => null,
  'create_promotion_scoped': () => null,
  'update_promotion': () => null,
  'update_promotion_scoped': () => null,
  'delete_promotion': () => null,
  'delete_promotion_scoped': () => null,
  'apply_promotion': () => null,
  'apply_promotion_scoped': () => null,
  'get_sale_promotions': () => [],
  'get_sale_promotions_scoped': () => [],

  // ═══════════════════════════════════════════════════════════════
  // PURCHASING / SUPPLIERS
  // ═══════════════════════════════════════════════════════════════

  'list_suppliers': () => [],
  'get_supplier': () => null,
  'create_supplier': () => null,
  'update_supplier': () => null,
  'list_purchase_orders': () => [],
  'get_purchase_order': () => null,
  'create_purchase_order': () => null,
  'update_po_status': () => null,
  'receive_purchase_order': () => null,

  // ═══════════════════════════════════════════════════════════════
  // REPORTS
  // ═══════════════════════════════════════════════════════════════

  'get_daily_revenue': () => [],
  'get_weekly_revenue': () => [],
  'get_monthly_revenue': () => [],
  'get_top_products': () => [],
  'get_hourly_heatmap': () => [],
  'get_low_stock_alerts': () => [],
  'get_category_breakdown': () => [],
  'get_menu_engineering': () => ({ rows: [], median_volume: 0, median_margin: 0 }),

  // ═══════════════════════════════════════════════════════════════
  // TAX
  // ═══════════════════════════════════════════════════════════════

  'compute_cart_tax': () => 0,
  'list_tax_rates': () => [],
  'create_tax_rate': () => null,
  'update_tax_rate': () => null,
  'delete_tax_rate': () => null,
  'list_category_tax_rates': () => [],
  'set_category_tax_rates': () => null,

  // ═══════════════════════════════════════════════════════════════
  // TABLES (restaurant floor plan)
  // ═══════════════════════════════════════════════════════════════

  'list_tables': () => [],
  'list_tables_scoped': () => [],
  'get_table': () => null,
  'get_table_scoped': () => null,
  'create_table': () => null,
  'create_table_scoped': () => null,
  'update_table': () => null,
  'update_table_scoped': () => null,
  'delete_table': () => null,
  'delete_table_scoped': () => null,
  'update_table_status': () => null,
  'update_table_status_scoped': () => null,
  'assign_table_order': () => null,
  'assign_table_order_scoped': () => null,
  'release_table': () => null,
  'release_table_scoped': () => null,
  'list_sections': () => [],
  'list_sections_scoped': () => [],

  // ═══════════════════════════════════════════════════════════════
  // LOYALTY
  // ═══════════════════════════════════════════════════════════════

  'get_loyalty_account': () => null,
  'list_loyalty_accounts': () => [],
  'earn_loyalty_points': () => null,
  'redeem_loyalty_points': () => null,
  'list_loyalty_tiers': () => [],
  'update_loyalty_tier': () => null,
  'get_points_value': () => 0,
  'get_or_create_loyalty_account': () => null,

  // ═══════════════════════════════════════════════════════════════
  // GIFT CARDS
  // ═══════════════════════════════════════════════════════════════

  'issue_gift_card': () => null,
  'get_gift_card': () => null,
  'list_gift_cards': () => [],
  'get_gift_card_balance': () => null,
  'redeem_gift_card': () => null,
  'top_up_gift_card': () => null,
  'freeze_gift_card': () => null,
  'unfreeze_gift_card': () => null,

  // ═══════════════════════════════════════════════════════════════
  // BUNDLES
  // ═══════════════════════════════════════════════════════════════

  'list_bundles': () => [],
  'get_bundle': () => null,
  'create_bundle': () => null,
  'update_bundle': () => null,
  'delete_bundle': () => null,
  'lookup_bundle_by_sku': () => null,

  // ═══════════════════════════════════════════════════════════════
  // HARDWARE
  // ═══════════════════════════════════════════════════════════════

  'open_cash_drawer': () => ({ opened: true }),
  'print_receipt': () => ({ printedLines: 3 }),
  'list_scanners': () => [{ id: 'scanner-1' }],
  'start_scanner': () => null,
  'stop_scanner': () => null,

  // ═══════════════════════════════════════════════════════════════
  // DATA MANAGEMENT
  // ═══════════════════════════════════════════════════════════════

  'get_backup_status': () => ({ lastBackup: null, lastBackupSize: null, dbPath: '/data/oz-pos.db' }),
  'create_backup': () => ({ path: '/backups/backup.db', sizeBytes: 1024 }),
  'export_data': () => ({ path: '/exports/data.ozpkg', sizeBytes: 512, types: ['products'] }),
  'import_preview': () => ({ storeName: 'Test Store', appVersion: '0.0.9', exportedAt: new Date().toISOString(), types: ['products'], productCount: 10, categoryCount: 2, saleCount: null, customerCount: null, userCount: null, settingCount: null }),
  'import_data': () => ({ productsImported: 10, categoriesImported: 2, salesImported: 0, customersImported: 0, usersImported: 0, settingsImported: 0 }),

  // ═══════════════════════════════════════════════════════════════
  // AUDIT
  // ═══════════════════════════════════════════════════════════════

  'list_audit_log': () => [],

  // ═══════════════════════════════════════════════════════════════
  // OFFLINE / SYNC
  // ═══════════════════════════════════════════════════════════════

  'enqueue_offline': () => null,
  'list_pending_offline': () => [],
  'list_all_offline': () => [],
  'pending_offline_count': () => 0,
  'retry_offline_sync': () => ({ synced: 0, failed: 0 }),
  'delete_offline_item': () => null,

  'get_sync_settings': () => ({ serverUrl: null, hasApiKey: false, enabled: false }),
  'update_sync_settings': () => null,
  'sync_run': () => ({ synced: 0, failed: 0, error: null }),
  'offline_queue_status_summary': () => ({ pendingCount: 0, syncedCount: 0, failedCount: 0, conflictCount: 0 }),

  'pending_sync_count': () => 0,
  'sync_pull': () => ({ productsPulled: 0, taxRatesPulled: 0, usersPulled: 0, error: null }),
  'test_sync_connection': () => ({ ok: true, status: 'connected', latencyMs: 12 }),
  'request_sync_token': () => ({ ok: true, token: 'mock-jwt-token', status: 'issued', expiresAt: new Date(Date.now() + 86400000).toISOString() }),

};

/** Mock Tauri invoke — handles common commands with mock data. */
export async function invoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  console.log('[TAURI MOCK] invoke:', cmd, args);

  // Small delay to simulate async IPC
  await new Promise((r) => setTimeout(r, 50));

  const handler = handlers[cmd];
  if (handler) {
    return handler(args?.['args'] ?? args) as T;
  }

  console.warn('[TAURI MOCK] Unhandled command:', cmd);
  return null as T;
}

/** Mock convertFileSrc — returns the path as-is in browser. */
export function convertFileSrc(path: string): string {
  return path;
}

export function isTauri(): boolean {
  return false;
}

export class Resource {}
export class Channel {}
