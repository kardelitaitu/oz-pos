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

import { emit } from './tauri-event';

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
  { sku: 'CPU-R7-7800X3D', name: 'AMD Ryzen 7 7800X3D 8-Core', category: 'Processors (CPU)', price: { minor_units: 6250000, currency: 'IDR' }, barcode: '730143314930', in_stock: true, stock_qty: 15, tax_rate_ids: [], created_at: new Date().toISOString(), price_updated_at: new Date().toISOString(), product_type: 'retail' },
  { sku: 'CPU-I7-14700K', name: 'Intel Core i7-14700K 20-Core', category: 'Processors (CPU)', price: { minor_units: 6450000, currency: 'IDR' }, barcode: '503203727850', in_stock: true, stock_qty: 10, tax_rate_ids: [], created_at: new Date().toISOString(), price_updated_at: new Date().toISOString(), product_type: 'retail' },
  { sku: 'CPU-R5-7600', name: 'AMD Ryzen 5 7600 6-Core', category: 'Processors (CPU)', price: { minor_units: 3150000, currency: 'IDR' }, barcode: '730143314503', in_stock: true, stock_qty: 25, tax_rate_ids: [], created_at: new Date().toISOString(), price_updated_at: new Date().toISOString(), product_type: 'retail' },
  { sku: 'GPU-RTX4070TS', name: 'ASUS TUF RTX 4070 Ti Super 16GB', category: 'Graphics Cards (GPU)', price: { minor_units: 14850000, currency: 'IDR' }, barcode: '195553554890', in_stock: true, stock_qty: 8, tax_rate_ids: [], created_at: new Date().toISOString(), price_updated_at: new Date().toISOString(), product_type: 'retail' },
  { sku: 'GPU-RX7800XT', name: 'Sapphire PULSE RX 7800 XT 16GB', category: 'Graphics Cards (GPU)', price: { minor_units: 8450000, currency: 'IDR' }, barcode: '489517350567', in_stock: true, stock_qty: 12, tax_rate_ids: [], created_at: new Date().toISOString(), price_updated_at: new Date().toISOString(), product_type: 'retail' },
  { sku: 'GPU-RTX4060', name: 'MSI Ventus 2X RTX 4060 8GB', category: 'Graphics Cards (GPU)', price: { minor_units: 4750000, currency: 'IDR' }, barcode: '824142323456', in_stock: true, stock_qty: 20, tax_rate_ids: [], created_at: new Date().toISOString(), price_updated_at: new Date().toISOString(), product_type: 'retail' },
  { sku: 'RAM-D5-32GB-CR', name: 'Corsair Vengeance DDR5 32GB 6000MHz', category: 'Memory (RAM)', price: { minor_units: 1850000, currency: 'IDR' }, barcode: '840006698765', in_stock: true, stock_qty: 30, tax_rate_ids: [], created_at: new Date().toISOString(), price_updated_at: new Date().toISOString(), product_type: 'retail' },
  { sku: 'RAM-D5-64GB-GS', name: 'G.Skill Trident Z5 RGB 64GB DDR5', category: 'Memory (RAM)', price: { minor_units: 3450000, currency: 'IDR' }, barcode: '848354041234', in_stock: true, stock_qty: 14, tax_rate_ids: [], created_at: new Date().toISOString(), price_updated_at: new Date().toISOString(), product_type: 'retail' },
  { sku: 'RAM-D4-16GB-KF', name: 'Kingston Fury Beast 16GB DDR4 3200', category: 'Memory (RAM)', price: { minor_units: 680000, currency: 'IDR' }, barcode: '740617319800', in_stock: true, stock_qty: 45, tax_rate_ids: [], created_at: new Date().toISOString(), price_updated_at: new Date().toISOString(), product_type: 'retail' },
  { sku: 'SSD-990PRO-2TB', name: 'Samsung 990 PRO 2TB NVMe M.2 SSD', category: 'Storage (SSD/HDD)', price: { minor_units: 2750000, currency: 'IDR' }, barcode: '887276722340', in_stock: true, stock_qty: 22, tax_rate_ids: [], created_at: new Date().toISOString(), price_updated_at: new Date().toISOString(), product_type: 'retail' },
  { sku: 'SSD-P3P-1TB', name: 'Crucial P3 Plus 1TB M.2 NVMe SSD', category: 'Storage (SSD/HDD)', price: { minor_units: 1150000, currency: 'IDR' }, barcode: '649528918900', in_stock: true, stock_qty: 35, tax_rate_ids: [], created_at: new Date().toISOString(), price_updated_at: new Date().toISOString(), product_type: 'retail' },
  { sku: 'HDD-ST-4TB', name: 'Seagate BarraCuda 4TB 3.5" HDD', category: 'Storage (SSD/HDD)', price: { minor_units: 1350000, currency: 'IDR' }, barcode: '763649112340', in_stock: true, stock_qty: 18, tax_rate_ids: [], created_at: new Date().toISOString(), price_updated_at: new Date().toISOString(), product_type: 'retail' },
  { sku: 'MB-B650-ROG', name: 'ASUS ROG Strix B650-A Gaming WiFi', category: 'Motherboards', price: { minor_units: 3650000, currency: 'IDR' }, barcode: '195553948760', in_stock: true, stock_qty: 9, tax_rate_ids: [], created_at: new Date().toISOString(), price_updated_at: new Date().toISOString(), product_type: 'retail' },
  { sku: 'MB-Z790-MSI', name: 'MSI MAG Z790 Tomahawk WiFi LGA1700', category: 'Motherboards', price: { minor_units: 4250000, currency: 'IDR' }, barcode: '824142301230', in_stock: true, stock_qty: 7, tax_rate_ids: [], created_at: new Date().toISOString(), price_updated_at: new Date().toISOString(), product_type: 'retail' },
  { sku: 'PSU-RM850X', name: 'Corsair RM850x 850W 80+ Gold Modular', category: 'Power Supply', price: { minor_units: 2150000, currency: 'IDR' }, barcode: '840006601234', in_stock: true, stock_qty: 16, tax_rate_ids: [], created_at: new Date().toISOString(), price_updated_at: new Date().toISOString(), product_type: 'retail' },
  { sku: 'COOL-PA120', name: 'Thermalright Peerless Assassin 120 SE', category: 'Cooling & Cases', price: { minor_units: 580000, currency: 'IDR' }, barcode: '784562098120', in_stock: true, stock_qty: 40, tax_rate_ids: [], created_at: new Date().toISOString(), price_updated_at: new Date().toISOString(), product_type: 'retail' },
  { sku: 'COOL-KRAKEN360', name: 'NZXT Kraken Elite 360 RGB AIO Liquid', category: 'Cooling & Cases', price: { minor_units: 4450000, currency: 'IDR' }, barcode: '815671018900', in_stock: true, stock_qty: 12, tax_rate_ids: [], created_at: new Date().toISOString(), price_updated_at: new Date().toISOString(), product_type: 'retail' },
  { sku: 'PASTE-MX6', name: 'Arctic MX-6 Thermal Paste 4g', category: 'Cooling & Cases', price: { minor_units: 125000, currency: 'IDR' }, barcode: '872767004500', in_stock: true, stock_qty: 60, tax_rate_ids: [], created_at: new Date().toISOString(), price_updated_at: new Date().toISOString(), product_type: 'retail' },
  // Restaurant-menu items (product_type: 'restaurant') so the Restaurant POS
  // menu is populated in the E2E dev-mock and a completed restaurant sale
  // feeds the Kitchen Display (KDS) ticket queue. The retail grid also shows
  // these — harmless for the artificial mock catalog.
  { sku: 'LATTE', name: 'Caffè Latte', category: 'Hot Drinks', price: { minor_units: 45000, currency: 'IDR' }, barcode: '4901234567890', in_stock: true, stock_qty: 50, tax_rate_ids: [], created_at: new Date().toISOString(), price_updated_at: new Date().toISOString(), product_type: 'restaurant' },
  { sku: 'CAPPU', name: 'Cappuccino', category: 'Hot Drinks', price: { minor_units: 42000, currency: 'IDR' }, barcode: '4901234567891', in_stock: true, stock_qty: 40, tax_rate_ids: [], created_at: new Date().toISOString(), price_updated_at: new Date().toISOString(), product_type: 'restaurant' },
  { sku: 'ESPR', name: 'Espresso Shot', category: 'Hot Drinks', price: { minor_units: 28000, currency: 'IDR' }, barcode: '4901234567892', in_stock: true, stock_qty: 60, tax_rate_ids: [], created_at: new Date().toISOString(), price_updated_at: new Date().toISOString(), product_type: 'restaurant' },
  { sku: 'CROISS', name: 'Butter Croissant', category: 'Food', price: { minor_units: 35000, currency: 'IDR' }, barcode: '4901234567896', in_stock: true, stock_qty: 45, tax_rate_ids: [], created_at: new Date().toISOString(), price_updated_at: new Date().toISOString(), product_type: 'restaurant' },
  { sku: 'BAGEL', name: 'Plain Bagel', category: 'Food', price: { minor_units: 25000, currency: 'IDR' }, barcode: '4901234567894', in_stock: true, stock_qty: 100, tax_rate_ids: [], created_at: new Date().toISOString(), price_updated_at: new Date().toISOString(), product_type: 'restaurant' },
  { sku: 'SANDW-C', name: 'Chicken Sandwich', category: 'Food', price: { minor_units: 75000, currency: 'IDR' }, barcode: '4901234567899', in_stock: true, stock_qty: 15, tax_rate_ids: [], created_at: new Date().toISOString(), price_updated_at: new Date().toISOString(), product_type: 'restaurant' },
];

const MOCK_CATEGORIES = [
  { id: 'cat-cpu', name: 'Processors (CPU)', colour: '#e74c3c', icon: 'cpu-1' },
  { id: 'cat-gpu', name: 'Graphics Cards (GPU)', colour: '#2ecc71', icon: 'gpu-1' },
  { id: 'cat-ram', name: 'Memory (RAM)', colour: '#9b59b6', icon: 'ram-1' },
  { id: 'cat-storage', name: 'Storage (SSD/HDD)', colour: '#3498db', icon: 'hdd-1' },
  { id: 'cat-mb', name: 'Motherboards', colour: '#f39c12', icon: 'mb-1' },
  { id: 'cat-psu', name: 'Power Supply', colour: '#1abc9c', icon: 'psu-1' },
  { id: 'cat-cooling', name: 'Cooling & Cases', colour: '#34495e', icon: 'cool-1' },
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

// ── Mock KDS orders ──────────────────────────────────────────────
// ── Mock KDS orders ──────────────────────────────────────────────
// Use let + mutable array so complete_sale can push new orders for E2E tests.
const _initialKdsOrders = [
  {
    id: 'kds-order-1',
    display_number: 101,
    status: 'pending',
    received_at: new Date(Date.now() - 60000).toISOString(),
    items_summary: '1x Caffè Latte, 1x Butter Croissant',
    item_count: 2,
    order_type: 'dine_in',
    table_number: 'T3',
    notes: null,
    store_id: 'store-1',
  },
  {
    id: 'kds-order-2',
    display_number: 102,
    status: 'preparing',
    received_at: new Date(Date.now() - 300000).toISOString(),
    items_summary: '2x Espresso Shot, 1x Iced Coffee',
    item_count: 3,
    order_type: 'takeaway',
    table_number: null,
    notes: 'No ice please',
    store_id: 'store-1',
  },
  {
    id: 'kds-order-3',
    display_number: 103,
    status: 'ready',
    received_at: new Date(Date.now() - 600000).toISOString(),
    items_summary: '1x Matcha Latte',
    item_count: 1,
    order_type: 'dine_in',
    table_number: 'T7',
    notes: null,
    store_id: 'store-1',
  },
];
const mockKdsOrders: Record<string, unknown>[] = [..._initialKdsOrders];
let kdsDisplayCounter = 104;

// ── Mock KDS line items (course-grouped for per-item advance) ──
const mockKdsLineItems: Record<string, Array<Record<string, unknown>>> = {
  'kds-order-1': [
    { id: 'kds-line-1-1', kds_order_id: 'kds-order-1', sku: 'LATTE', display_name: 'Caffè Latte', qty: 1, course: 'beverage', modifiers: [], line_position: 1, item_status: 'pending', started_at: null, ready_at: null, served_at: null, created_at: new Date().toISOString() },
    { id: 'kds-line-1-2', kds_order_id: 'kds-order-1', sku: 'CROISS', display_name: 'Butter Croissant', qty: 1, course: 'main', modifiers: [], line_position: 2, item_status: 'pending', started_at: null, ready_at: null, served_at: null, created_at: new Date().toISOString() },
  ],
  'kds-order-2': [
    { id: 'kds-line-2-1', kds_order_id: 'kds-order-2', sku: 'ESPR', display_name: 'Espresso Shot', qty: 2, course: 'beverage', modifiers: [], line_position: 1, item_status: 'pending', started_at: null, ready_at: null, served_at: null, created_at: new Date().toISOString() },
    { id: 'kds-line-2-2', kds_order_id: 'kds-order-2', sku: 'ICED', display_name: 'Iced Coffee', qty: 1, course: 'beverage', modifiers: [], line_position: 2, item_status: 'pending', started_at: null, ready_at: null, served_at: null, created_at: new Date().toISOString() },
    { id: 'kds-line-2-3', kds_order_id: 'kds-order-2', sku: 'TOAST', display_name: 'Avocado Toast', qty: 1, course: 'main', modifiers: [], line_position: 3, item_status: 'pending', started_at: null, ready_at: null, served_at: null, created_at: new Date().toISOString() },
  ],
  'kds-order-3': [
    { id: 'kds-line-3-1', kds_order_id: 'kds-order-3', sku: 'MATCHA', display_name: 'Matcha Latte', qty: 1, course: 'beverage', modifiers: [], line_position: 1, item_status: 'pending', started_at: null, ready_at: null, served_at: null, created_at: new Date().toISOString() },
  ],
};

/** Push a new KDS order derived from cart lines into the mock queue. */
function pushKdsOrderFromCart(lines: CartLine[], storeId: string) {
  const displayNumber = kdsDisplayCounter++;
  const itemsSummary = lines.map((l) => `${l.qty}x ${l.name}`).join(', ');
  const itemCount = lines.reduce((sum, l) => sum + l.qty, 0);
  const now = new Date().toISOString();
  const orderId = `kds-order-e2e-${Date.now()}`;
  mockKdsOrders.push({
    id: orderId,
    display_number: displayNumber,
    status: 'pending',
    received_at: now,
    items_summary: itemsSummary,
    item_count: itemCount,
    order_type: 'dine_in',
    table_number: 'T' + (Math.floor(Math.random() * 20) + 1),
    notes: null,
    store_id: storeId,
  });
  // Seed course-grouped line items so the KDS ticket renders real item
  // names (KdsTicketCard fetches via get_kds_order_lines_scoped).
  // Derive the course from the product category — 'beverage' for hot
  // drinks, 'main' for food and everything else (incl. retail).
  const courseForSku = (sku: string): string => {
    const p = MOCK_PRODUCTS.find((prod) => prod.sku === sku);
    const category = p?.category ?? '';
    if (category === 'Hot Drinks') return 'beverage';
    if (category === 'Food') return 'main';
    return 'main';
  };
  mockKdsLineItems[orderId] = lines.map((l, i) => ({
    id: `kds-line-e2e-${orderId}-${i}`,
    kds_order_id: orderId,
    sku: l.sku,
    display_name: l.name,
    qty: l.qty,
    course: courseForSku(l.sku),
    modifiers: [],
    line_position: i + 1,
    item_status: 'pending',
    started_at: null,
    ready_at: null,
    served_at: null,
    created_at: now,
  }));
}

// ── Lockout state (for E2E rate-limit tests) ──────────────────
const loginAttempts: Record<string, number> = {};
const LOCKOUT_THRESHOLD = 4;
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

// ── Completed sales (persisted so sales history + refund e2e work) ─
interface MockCompletedSale {
  id: string;
  total: { minor_units: number; currency: string };
  lineCount: number;
  status: string;
  paymentMethod: string;
  userId: string;
  createdAt: string;
}
const completedSales: MockCompletedSale[] = [
  // Pre-seeded sale so sales history always has at least one row.
  {
    id: 'seed-sale-001',
    total: { minor_units: 1250, currency: 'USD' },
    lineCount: 2,
    status: 'Completed',
    paymentMethod: 'cash',
    userId: 'admin-1',
    createdAt: new Date(Date.now() - 3600000).toISOString(),
  },
];
const saleDetails: Record<string, MockCompletedSale & { subtotal: { minor_units: number; currency: string }; taxTotal: { minor_units: number; currency: string }; tenderedMinor: number; lines: Array<{ id: string; sku: string; name: string; qty: number; unit_price: { minor_units: number; currency: string }; total_minor: number; tax_amount: null; tax_rate_id: null }> }> = {
  'seed-sale-001': {
    id: 'seed-sale-001',
    total: { minor_units: 1250, currency: 'USD' },
    subtotal: { minor_units: 1250, currency: 'USD' },
    taxTotal: { minor_units: 0, currency: 'USD' },
    lineCount: 2,
    status: 'Completed',
    paymentMethod: 'cash',
    tenderedMinor: 2000,
    userId: 'admin-1',
    createdAt: new Date(Date.now() - 3600000).toISOString(),
    lines: [
      { id: 'seed-line-1', sku: 'LATTE', name: 'Caffè Latte', qty: 1, unit_price: { minor_units: 450, currency: 'USD' }, total_minor: 450, tax_amount: null, tax_rate_id: null },
      { id: 'seed-line-2', sku: 'CROISS', name: 'Butter Croissant', qty: 2, unit_price: { minor_units: 320, currency: 'USD' }, total_minor: 640, tax_amount: null, tax_rate_id: null },
    ],
  },
};

// ── Active shift state (for pay-btn-enabled E2E test) ──────────
let mockActiveShift: Record<string, unknown> | null = {
  id: 'shift-1', userId: 'user-1', terminalId: null, openedAt: new Date().toISOString(), closedAt: null,
  openingBalanceMinor: 0, closingBalanceMinor: null, expectedCashMinor: null, cashDifferenceMinor: null,
  totalSalesMinor: 0, totalCashMinor: 0, totalCardMinor: 0, totalOtherMinor: 0,
  totalVoidsMinor: 0, totalRefundsMinor: 0, totalPayoutsMinor: 0, notes: '', status: 'open',
  createdAt: new Date().toISOString(), updatedAt: new Date().toISOString(),
};
// Closed-shift history so the reconciliation spec can verify shifts appear
// in the Shift History table after closing.
const mockShiftHistory: Array<Record<string, unknown>> = [];
const handlers: Record<string, (args: unknown) => unknown> = {
  // ═══════════════════════════════════════════════════════════════
  // AUTH / STAFF
  // ═══════════════════════════════════════════════════════════════

  // STAFF-06: uniform pre-auth response — never reveals account existence or
  // activation state (enumeration oracle closed).
  'staff_check_username': (_args) => ({ proceed: true }),

  'staff_login': (args) => {
    const { username, pin } = args as { username: string; pin: string };
    const key = username.toLowerCase();
    const staff = MOCK_STAFF[key];

    // Check lockout.
    const attempts = loginAttempts[key] ?? 0;
    if (attempts >= LOCKOUT_THRESHOLD) {
      throw new Error('Account locked. Too many failed attempts. Try again in 30s');
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
  'version_scoped': () => ({ name: 'oz-pos', version: '0.0.9', rustVersion: '1.80', target: 'x86_64' }),
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
  'archive_workspace_instance_scoped': () => null,
  'set_default_instance_scoped': () => null,
  'list_screens_scoped': () => [],
  'list_all_workspaces_scoped': () => [
    { key: 'store-pos', name: 'Store POS', description: 'Point of Sale', icon: 'shopping-cart' },
    { key: 'restaurant-pos', name: 'Restaurant POS', description: 'Table service', icon: 'restaurant' },
    { key: 'kds', name: 'Kitchen Display', description: 'Order display', icon: 'utensils' },
    { key: 'inventory', name: 'Inventory Management', description: 'Stock management', icon: 'package' },
    { key: 'admin', name: 'Admin', description: 'Settings & management', icon: 'settings' },
  ],
  'get_user_workspace_instances_scoped': () => [],
  'set_user_workspace_instances_scoped': () => null,
  'get_user_workspaces_scoped': () => [],
  'set_user_workspaces_scoped': () => null,

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
  'get_store_settings_scoped': () => ({
    name: 'TOKO TEST', address: 'Jl. Contoh No. 123', taxId: 'TAX-001', currency: 'IDR', branch: 'Cabang A', logo: '',
  }),
  'set_store_settings': () => null,
  'set_store_settings_scoped': () => null,

  'get_receipt_settings': () => ({
    showCurrency: true, decimalSeparator: 'dot', showTax: true, footer: 'Terima kasih',
    paperWidth: 'standard', showTableNumber: false,
    marginTop: 0, marginBottom: 0, marginLeft: 0, marginRight: 0,
  }),
  'get_receipt_settings_scoped': () => ({
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
  'send_test_report': () => 'Email sent',

  'load_topology': () => ({
    nodes: [
      { id: 'store-1', label: 'TOKO TEST', type: 'store', x: 100, y: 100, storeProfileId: 'store-1', licenseTier: 'pro', nodeColor: null },
      { id: 'ws-1', label: 'Store POS', type: 'workspace', x: 300, y: 50, storeProfileId: 'store-1', licenseTier: 'pro', nodeColor: null },
      { id: 'ws-2', label: 'Restaurant', type: 'workspace', x: 300, y: 180, storeProfileId: 'store-1', licenseTier: 'pro', nodeColor: null },
    ],
    wires: [
      { id: 'wire-1', from: 'store-1', fromPort: 'right', to: 'ws-1', toPort: 'left' },
      { id: 'wire-2', from: 'store-1', fromPort: 'bottom', to: 'ws-2', toPort: 'left' },
    ],
  }),
  'save_topology': () => null,
  'apply_topology_diff': () => null,

  'set_receipt_settings_scoped': () => null,

  'get_enabled_features': () => ({ features: ['sales', 'inventory', 'reporting', 'staff', 'settings'] }),
  'get_setting': () => '',
  'set_setting_scoped': () => null,

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
  'list_credit_sales_scoped': () => [],
  'settle_credit': () => null,
  'settle_credit_scoped': () => null,

  'seed_default_roles_scoped': () => 3,

  // ═══════════════════════════════════════════════════════════════
  // SECURITY / ENCRYPTION
  // ═══════════════════════════════════════════════════════════════

  'get_key_rotation_info': () => ({
    last_rotated_at: null,
    rotation_due: false,
    key_algorithm: 'aes-256-gcm',
    can_rotate: true,
  }),
  'rotate_encryption_key': () => ({
    success: true,
    rotated_at: new Date().toISOString(),
    key_algorithm: 'aes-256-gcm',
  }),

  // ═══════════════════════════════════════════════════════════════
  // BRANDING
  // ═══════════════════════════════════════════════════════════════

  'get_brand_settings': () => ({
    primary_colour: '#10b981',
    logo_path: null,
    store_name: 'OZ-POS Demo',
    colour_hover: null,
  }),
  'get_brand_settings_scoped': () => ({
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
  'get_product_track_serial_scoped': () => false,
  'get_product_track_serial_batch': (args) => {
    const { skus } = args as { skus: string[] };
    return (skus ?? []).map((sku) => ({ sku, track_serial: false }));
  },
  'get_product_track_serial_batch_scoped': (args) => {
    const { skus } = args as { skus: string[] };
    return (skus ?? []).map((sku) => ({ sku, track_serial: false }));
  },
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
  'list_categories_scoped': () => MOCK_CATEGORIES,
  'create_category': () => ({ id: 'cat-new' }),
  'update_category': () => ({ id: 'cat-upd' }),
  'delete_category': () => null,

  // ═══════════════════════════════════════════════════════════════
  // SALES / CART
  // ═══════════════════════════════════════════════════════════════

  'start_sale': () => { cartState = { lines: [] }; return { cartId: `mock-cart-${Date.now()}`, deduction_location_id: 'default-loc', deductionLocationId: 'default-loc' }; },
  'start_sale_scoped': () => { cartState = { lines: [] }; return { cartId: `mock-cart-${Date.now()}`, deduction_location_id: 'default-loc', deductionLocationId: 'default-loc' }; },

  'add_line': (args) => {
    // The API sends { args: { cartId, sku, qty, unitPriceMinor } } — read `sku`
    // (with a `productSku` fallback for older callers). Previously the mock only
    // read `productSku`, so cartState stayed empty and mock sale totals were 0.
    const raw = (args as { args?: { sku?: string; productSku?: string; qty?: number } })?.args ?? (args as { sku?: string; productSku?: string; qty?: number });
    const skuKey = raw?.sku ?? raw?.productSku;
    const qty = raw?.qty ?? 1;
    const product = MOCK_PRODUCTS.find(p => p.sku === skuKey);
    if (product) {
      const existing = cartState.lines.find(l => l.sku === skuKey);
      if (existing) {
        existing.qty += qty;
      } else {
        cartState.lines.push({ sku: product.sku, name: product.name, price: product.price, qty });
      }
    }
    const lineTotal = product ? product.price.minor_units * qty : 0;
    return { lineId: `mock-line-${Date.now()}`, lineTotal };
  },
  'add_line_scoped': (args) => {
    const raw = (args as { args?: { sku?: string; productSku?: string; qty?: number } })?.args ?? (args as { sku?: string; productSku?: string; qty?: number });
    const skuKey = raw?.sku ?? raw?.productSku;
    const qty = raw?.qty ?? 1;
    const product = MOCK_PRODUCTS.find(p => p.sku === skuKey);
    if (product) {
      const existing = cartState.lines.find(l => l.sku === skuKey);
      if (existing) {
        existing.qty += qty;
      } else {
        cartState.lines.push({ sku: product.sku, name: product.name, price: product.price, qty });
      }
    }
    const lineTotal = product ? product.price.minor_units * qty : 0;
    return { lineId: `mock-line-${Date.now()}`, lineTotal };
  },

  'complete_sale': () => {
    const minorTotal = cartState.lines.reduce((sum, l) => sum + l.price.minor_units * l.qty, 0);
    const lineCount = cartState.lines.length;
    // Currency follows the cart (products are IDR) so history totals and
    // receipts render correctly in E2E.
    const currency = cartState.lines[0]?.price.currency ?? 'IDR';
    const saleId = `mock-sale-${Date.now()}`;
    // Persist into completed sales so sales history / refund e2e work.
    const now = new Date().toISOString();
    completedSales.push({
      id: saleId, total: { minor_units: minorTotal, currency }, lineCount,
      status: 'Completed', paymentMethod: 'cash', userId: 'admin-1', createdAt: now,
    });
    saleDetails[saleId] = {
      id: saleId, total: { minor_units: minorTotal, currency },
      subtotal: { minor_units: minorTotal, currency },
      taxTotal: { minor_units: 0, currency }, lineCount, status: 'Completed',
      paymentMethod: 'cash', tenderedMinor: minorTotal + 500, userId: 'admin-1', createdAt: now,
      lines: cartState.lines.map((l, i) => ({
        id: `mock-line-${i}-${saleId}`, sku: l.sku, name: l.name, qty: l.qty,
        unit_price: l.price, total_minor: l.price.minor_units * l.qty,
        tax_amount: null, tax_rate_id: null,
      })),
    };
    // Push a KDS mock order so POS → KDS E2E flow works.
    pushKdsOrderFromCart(cartState.lines, 'store-1');
    cartState = { lines: [] };
    return { saleId, total: { minor_units: minorTotal, currency }, lineCount };
  },
  'complete_sale_scoped': () => {
    const minorTotal = cartState.lines.reduce((sum, l) => sum + l.price.minor_units * l.qty, 0);
    const lineCount = cartState.lines.length;
    const currency = cartState.lines[0]?.price.currency ?? 'IDR';
    const saleId = `mock-sale-${Date.now()}`;
    const now = new Date().toISOString();
    completedSales.push({
      id: saleId, total: { minor_units: minorTotal, currency }, lineCount,
      status: 'Completed', paymentMethod: 'cash', userId: 'admin-1', createdAt: now,
    });
    saleDetails[saleId] = {
      id: saleId, total: { minor_units: minorTotal, currency },
      subtotal: { minor_units: minorTotal, currency },
      taxTotal: { minor_units: 0, currency }, lineCount, status: 'Completed',
      paymentMethod: 'cash', tenderedMinor: minorTotal + 500, userId: 'admin-1', createdAt: now,
      lines: cartState.lines.map((l, i) => ({
        id: `mock-line-${i}-${saleId}`, sku: l.sku, name: l.name, qty: l.qty,
        unit_price: l.price, total_minor: l.price.minor_units * l.qty,
        tax_amount: null, tax_rate_id: null,
      })),
    };
    pushKdsOrderFromCart(cartState.lines, 'store-1');
    cartState = { lines: [] };
    return { saleId, total: { minor_units: minorTotal, currency }, lineCount };
  },
  'complete_sale_with_resolved_shortfalls_scoped': () => {
    const minorTotal = cartState.lines.reduce((sum, l) => sum + l.price.minor_units * l.qty, 0);
    const lineCount = cartState.lines.length;
    const currency = cartState.lines[0]?.price.currency ?? 'IDR';
    const saleId = `mock-sale-${Date.now()}`;
    const now = new Date().toISOString();
    completedSales.push({
      id: saleId, total: { minor_units: minorTotal, currency }, lineCount,
      status: 'Completed', paymentMethod: 'cash', userId: 'admin-1', createdAt: now,
    });
    saleDetails[saleId] = {
      id: saleId, total: { minor_units: minorTotal, currency },
      subtotal: { minor_units: minorTotal, currency },
      taxTotal: { minor_units: 0, currency }, lineCount, status: 'Completed',
      paymentMethod: 'cash', tenderedMinor: minorTotal + 500, userId: 'admin-1', createdAt: now,
      lines: cartState.lines.map((l, i) => ({
        id: `mock-line-${i}-${saleId}`, sku: l.sku, name: l.name, qty: l.qty,
        unit_price: l.price, total_minor: l.price.minor_units * l.qty,
        tax_amount: null, tax_rate_id: null,
      })),
    };
    pushKdsOrderFromCart(cartState.lines, 'store-1');
    cartState = { lines: [] };
    return { saleId, total: { minor_units: minorTotal, currency }, lineCount };
  },

  'get_sale': (args) => {
    const { id } = (args as { id?: string }) ?? {};
    return id ? (saleDetails[id] ?? null) : null;
  },
  'get_sale_scoped': (args) => {
    const { id } = (args as { id?: string }) ?? {};
    return id ? (saleDetails[id] ?? null) : null;
  },

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

  'list_sales': () => [...completedSales],
  'list_sales_scoped': () => [...completedSales],
  'void_sale': () => ({ id: 'voided-sale', status: 'voided', total: { minor_units: 0, currency: 'IDR' }, line_count: 0, created_at: new Date().toISOString() }),
  'void_sale_scoped': () => ({ id: 'voided-sale', status: 'voided', total: { minor_units: 0, currency: 'IDR' }, line_count: 0, created_at: new Date().toISOString() }),

  'lookup_sale_by_receipt_barcode': () => null,
  'lookup_sale_by_receipt_barcode_scoped': () => null,

  'process_refund': (args) => {
    const a = (args as { args?: { lines?: Array<{ lineTotalMinor: number }> } })?.args ?? (args as { lines?: Array<{ lineTotalMinor: number }> }) ?? {};
    const lines = a.lines ?? [];
    const totalMinor = lines.reduce((sum, l) => sum + (l.lineTotalMinor ?? 0), 0);
    return { refundId: `refund-${Date.now()}`, totalMinor };
  },
  'process_refund_scoped': (args) => {
    const a = (args as { args?: { lines?: Array<{ lineTotalMinor: number }> } })?.args ?? (args as { lines?: Array<{ lineTotalMinor: number }> }) ?? {};
    const lines = a.lines ?? [];
    const totalMinor = lines.reduce((sum, l) => sum + (l.lineTotalMinor ?? 0), 0);
    return { refundId: `refund-${Date.now()}`, totalMinor };
  },
  'list_refunds': () => [],
  'list_refunds_scoped': () => [],

  'export_daily_summary': () => [],
  'export_daily_summary_scoped': () => [],
  'export_sales_by_hour': () => [],
  'export_sales_by_hour_scoped': () => [],
  'export_eod_report': () => null,
  'export_eod_report_scoped': () => null,

  'print_sales_receipt': () => ({ printed: true }),
  'print_sales_receipt_scoped': () => ({ printed: true }),

  'get_cart_deduction_location': () => ({ locationId: 'loc-1', locationName: 'Main Store' }),
  'override_cart_deduction_location_scoped': () => null,

  // ═══════════════════════════════════════════════════════════════
  // CURRENCY
  // ═══════════════════════════════════════════════════════════════

  'currency_info': () => ({ code: 'IDR', exponent: 0 }),
  'list_currencies': () => MOCK_CURRENCIES,
  'list_currencies_scoped': () => MOCK_CURRENCIES,
  'get_default_currency': () => 'IDR',
  'set_default_currency': () => null,
  'list_exchange_rates': () => [
    { id: 'rate-1', from_currency: 'USD', to_currency: 'IDR', rate_millionths: 1_6000000, source: 'manual', effective_date: '2026-08-01', created_at: new Date().toISOString() },
  ],
  'create_exchange_rate': () => null,
  'delete_exchange_rate': () => null,

  // ═══════════════════════════════════════════════════════════════
  // CUSTOMERS
  // ═══════════════════════════════════════════════════════════════

  'list_customers': () => MOCK_CUSTOMERS,
  'list_customers_scoped': () => MOCK_CUSTOMERS,
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

  'list_staff_scoped': () => [
    { id: 'staff-1', username: 'owner', display_name: 'Owner', role_id: '1', role_name: 'owner', is_active: true },
    { id: 'staff-2', username: 'admin', display_name: 'Admin', role_id: '2', role_name: 'manager', is_active: true },
    { id: 'staff-3', username: 'kasir', display_name: 'Cashier', role_id: '3', role_name: 'cashier', is_active: true },
  ],
  'list_roles_scoped': () => [
    { id: '1', name: 'Owner', description: 'Full access to all settings' },
    { id: '2', name: 'Manager', description: 'Daily operations and reports' },
    { id: '3', name: 'Cashier', description: 'Process sales and refunds' },
    { id: '4', name: 'Kitchen', description: 'Kitchen display access' },
  ],
  'create_staff_scoped': (args) => {
    const a = (args as { username?: string; display_name?: string; role_id?: string; pin?: string }) ?? {};
    return {
      id: `staff-${Date.now()}`,
      username: a.username ?? 'newstaff',
      display_name: a.display_name ?? 'New Staff',
      role_id: a.role_id ?? '3',
      role_name: a.role_id === '1' ? 'owner' : a.role_id === '2' ? 'manager' : 'cashier',
      is_active: true,
    };
  },
  'update_staff_scoped': (args) => {
    const a = (args as { id?: string; username?: string; display_name?: string; role_id?: string; is_active?: boolean }) ?? {};
    return {
      id: a.id ?? 'staff-1',
      username: a.username ?? 'owner',
      display_name: a.display_name ?? 'Owner',
      role_id: a.role_id ?? '1',
      role_name: a.role_id === '1' ? 'owner' : a.role_id === '2' ? 'manager' : 'cashier',
      is_active: a.is_active ?? true,
    };
  },

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
    const closed: Record<string, unknown> = {
      id: `shift-${mockShiftHistory.length + 1}`, userId: 'user-1', terminalId: null, openedAt: new Date().toISOString(), closedAt: new Date().toISOString(),
      openingBalanceMinor: 100000, closingBalanceMinor: 150000, expectedCashMinor: 150000, cashDifferenceMinor: 0,
      totalSalesMinor: 50000, totalCashMinor: 50000, totalCardMinor: 0, totalOtherMinor: 0,
      totalVoidsMinor: 0, totalRefundsMinor: 0, totalPayoutsMinor: 0, notes: '', status: 'closed',
      createdAt: new Date().toISOString(), updatedAt: new Date().toISOString(),
    };
    mockShiftHistory.push(closed);
    return closed;
  },
  'close_shift_scoped': () => {
    mockActiveShift = null;
    const closed: Record<string, unknown> = {
      id: `shift-${mockShiftHistory.length + 1}`, userId: 'user-1', terminalId: null, openedAt: new Date().toISOString(), closedAt: new Date().toISOString(),
      openingBalanceMinor: 100000, closingBalanceMinor: 150000, expectedCashMinor: 150000, cashDifferenceMinor: 0,
      totalSalesMinor: 50000, totalCashMinor: 50000, totalCardMinor: 0, totalOtherMinor: 0,
      totalVoidsMinor: 0, totalRefundsMinor: 0, totalPayoutsMinor: 0, notes: '', status: 'closed',
      createdAt: new Date().toISOString(), updatedAt: new Date().toISOString(),
    };
    mockShiftHistory.push(closed);
    return closed;
  },
  'list_shifts': () => mockShiftHistory,
  'list_shifts_scoped': () => mockShiftHistory,
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
  'get_workspace_locations_scoped': () => [],
  'invalidate_location_cache_scoped': () => null,
  'get_low_stock_alerts_at_location_scoped': () => [],
  'active_stock_alerts_scoped': () => [],
  'acknowledge_stock_alert_scoped': () => null,

  'start_inventory_shift': () => ({
    id: 'inv-shift-1', user_id: 'user-1', location_id: 'loc-1', terminal_id: null,
    started_at: new Date().toISOString(), ended_at: null, status: 'active', notes: '',
  }),
  'end_inventory_shift': () => null,
  'get_active_inventory_shift': () => null,
  'list_inventory_shifts': () => [],

  'create_inventory_transaction': () => 'txn-new',
  'list_inventory_transactions': () => [],
  'list_inventory_transactions_for_shift': () => [],
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
  'list_stock_adjustments': () => [
    { id: 'adj-1', sku: 'CPU-R7-7800X3D', product_name: 'AMD Ryzen 7 7800X3D', qty_change: 5, reason: 'restock', created_at: new Date().toISOString() },
  ],
  'list_stock_adjustments_scoped': () => [
    { id: 'adj-1', sku: 'CPU-R7-7800X3D', product_name: 'AMD Ryzen 7 7800X3D', qty_change: 5, reason: 'restock', created_at: new Date().toISOString() },
  ],

  // ═══════════════════════════════════════════════════════════════
  // STOCK TRANSFERS
  // ═══════════════════════════════════════════════════════════════

  'create_stock_transfer_scoped': () => null,
  'get_stock_transfer_scoped': () => null,
  'list_stock_transfers_scoped': () => [],
  'get_stock_transfer_lines_scoped': () => [],
  'add_stock_transfer_line_scoped': () => null,
  'remove_stock_transfer_line_scoped': () => null,
  'send_stock_transfer_scoped': () => null,
  'receive_stock_transfer_scoped': () => null,
  'cancel_stock_transfer_scoped': () => null,

  // ═══════════════════════════════════════════════════════════════
  // KDS
  // ═══════════════════════════════════════════════════════════════

  'list_kds_orders': () => mockKdsOrders,
  'list_kds_orders_scoped': () => mockKdsOrders,
  'get_kds_queue': () => mockKdsOrders,
  'get_kds_queue_scoped': () => mockKdsOrders,
  'update_kds_status': (args) => {
    const { id, status } = (args as { id?: string; status?: string }) ?? {};
    const order = mockKdsOrders.find((o) => o['id'] === id);
    if (order && status) {
      order['status'] = status;
      void emit('kds:orders-changed', null);
    }
    return order ?? null;
  },
  'update_kds_status_scoped': (args) => {
    const { id, status } = (args as { id?: string; status?: string }) ?? {};
    const order = mockKdsOrders.find((o) => o['id'] === id);
    if (order && status) {
      order['status'] = status;
      void emit('kds:orders-changed', null);
    }
    return order ?? null;
  },
  'create_kds_order_from_sale': () => [],
  'create_kds_order_from_sale_scoped': () => [],
  'get_kds_order': () => null,
  'get_kds_order_scoped': () => null,
  'get_kds_order_lines': (args) => {
    const { id } = (args as { id?: string }) ?? {};
    return (id ? mockKdsLineItems[id] : undefined) ?? [];
  },
  'get_kds_order_lines_scoped': (args) => {
    const { orderId } = (args as { orderId?: string }) ?? {};
    return (orderId ? mockKdsLineItems[orderId] : undefined) ?? [];
  },
  'update_kds_line_item_status': (args) => {
    const { itemId, status } = (args as { itemId?: string; status?: string }) ?? {};
    const item = Object.values(mockKdsLineItems).flat().find((i) => i['id'] === itemId);
    if (item && status) {
      item['item_status'] = status;
      void emit('kds:orders-changed', null);
    }
    return item ?? null;
  },
  'update_kds_line_item_status_scoped': (args) => {
    const { itemId, status } = (args as { itemId?: string; status?: string }) ?? {};
    const item = Object.values(mockKdsLineItems).flat().find((i) => i['id'] === itemId);
    if (item && status) {
      item['item_status'] = status;
      void emit('kds:orders-changed', null);
    }
    return item ?? null;
  },

  // ═══════════════════════════════════════════════════════════════
  // PROMOTIONS
  // ═══════════════════════════════════════════════════════════════

  'list_promotions': () => [
    {
      id: 'promo-1', name: 'Buy 1 Get 1', description: 'Free croissant with any latte', promo_type: 'buy_x_get_y',
      value_minor: 0, min_qty: 1, trigger_sku: 'LATTE', reward_sku: 'CROISS', reward_qty: 1,
      starts_at: new Date().toISOString(), ends_at: null, min_order_minor: 0, category_id: null,
      active: true, created_at: new Date().toISOString(), updated_at: new Date().toISOString(),
    },
  ],
  'list_promotions_scoped': () => [
    {
      id: 'promo-1', name: 'Buy 1 Get 1', description: 'Free croissant with any latte', promo_type: 'buy_x_get_y',
      value_minor: 0, min_qty: 1, trigger_sku: 'LATTE', reward_sku: 'CROISS', reward_qty: 1,
      starts_at: new Date().toISOString(), ends_at: null, min_order_minor: 0, category_id: null,
      active: true, created_at: new Date().toISOString(), updated_at: new Date().toISOString(),
    },
  ],
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

  'list_suppliers': () => [
    { id: 'supplier-1', name: 'PT Teknologi Maju', contact_person: 'Budi', phone: '021-1234567', email: 'budi@teknologi.com', address: 'Jl. Merdeka No. 1', is_active: true, created_at: new Date().toISOString(), updated_at: new Date().toISOString() },
    { id: 'supplier-2', name: 'CV Distribusi Utama', contact_person: 'Siti', phone: '021-7654321', email: 'siti@distribusi.com', address: 'Jl. Sudirman No. 45', is_active: true, created_at: new Date().toISOString(), updated_at: new Date().toISOString() },
  ],
  'list_suppliers_scoped': () => [
    { id: 'supplier-1', name: 'PT Teknologi Maju', contact_person: 'Budi', phone: '021-1234567', email: 'budi@teknologi.com', address: 'Jl. Merdeka No. 1', is_active: true, created_at: new Date().toISOString(), updated_at: new Date().toISOString() },
    { id: 'supplier-2', name: 'CV Distribusi Utama', contact_person: 'Siti', phone: '021-7654321', email: 'siti@distribusi.com', address: 'Jl. Sudirman No. 45', is_active: true, created_at: new Date().toISOString(), updated_at: new Date().toISOString() },
  ],
  'get_supplier': () => null,
  'create_supplier': () => null,
  'update_supplier': () => null,
  'list_purchase_orders': () => [
    { id: 'po-1', po_number: 'PO-001', supplier_id: 'supplier-1', supplier_name: 'PT Teknologi Maju', status: 'pending', total_minor: 5000000, tax_minor: 0, line_count: 2, created_at: new Date().toISOString(), updated_at: new Date().toISOString() },
  ],
  'list_purchase_orders_scoped': () => [
    { id: 'po-1', po_number: 'PO-001', supplier_id: 'supplier-1', supplier_name: 'PT Teknologi Maju', status: 'pending', total_minor: 5000000, tax_minor: 0, line_count: 2, created_at: new Date().toISOString(), updated_at: new Date().toISOString() },
  ],
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
  'build_custom_report': () => ({
    rows: [], columns: [], total: 0, page: 1, pageSize: 50, totalPages: 1,
  }),

  // ═══════════════════════════════════════════════════════════════
  // TAX
  // ═══════════════════════════════════════════════════════════════

  'compute_cart_tax_scoped': () => 0,
  'list_tax_rates_scoped': () => [],
  'create_tax_rate_scoped': () => null,
  'update_tax_rate_scoped': () => null,
  'delete_tax_rate_scoped': () => null,
  'get_tax_rate_dependency_counts_scoped': () => ({ products: 0, categories: 0, sale_lines: 0 }),
  'list_category_tax_rates_scoped': () => [],
  'set_category_tax_rates_scoped': () => null,

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

  'get_loyalty_account_scoped': () => null,
  'list_loyalty_accounts_scoped': () => [],
  'earn_loyalty_points_scoped': () => null,
  'redeem_loyalty_points_scoped': () => null,
  'list_loyalty_tiers_scoped': () => [],
  'update_loyalty_tier_scoped': () => null,
  'get_points_value_scoped': () => 0,
  'get_or_create_loyalty_account_scoped': () => null,

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
  'list_displays': () => ['display-1', 'display-2'],
  'display_show': () => null,
  'display_clear': () => null,
  'read_scale_weight': () => ({ grams: 150, stable: true }),
  'discover_hardware': () => [],
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
  'list_audit_log_scoped': () => [],
  'get_audit_review_status_scoped': () => ({ reviewed: false }),
  'mark_audit_reviewed_scoped': () => null,
  'export_audit_log_scoped': () => '',

  // ═══════════════════════════════════════════════════════════════
  // OFFLINE / SYNC
  // ═══════════════════════════════════════════════════════════════

  'enqueue_offline': () => null,
  'list_pending_offline': () => [],
  'list_all_offline': () => [],
  'pending_offline_count': () => 0,
  'retry_offline_sync': () => ({ syncedCount: 0, failedCount: 0, totalCount: 0 }),
  'delete_offline_item': () => null,

  'get_sync_settings': () => ({ serverUrl: null, hasApiKey: false, enabled: false }),
  'get_sync_settings_scoped': () => ({ serverUrl: null, hasApiKey: false, enabled: false }),
  'update_sync_settings': () => null,
  'sync_run': () => ({ synced: 0, failed: 0, error: null }),
  'offline_queue_status_summary': () => ({ pendingCount: 0, syncedCount: 0, failedCount: 0, conflictCount: 0 }),

  'pending_sync_count': () => 0,
  'sync_pull': (args: unknown) => {
    // SYNC-03: reject without explicit destructive consent, mirroring the
    // backend command contract so dev-mode behaviour matches production.
    const a = (args ?? {}) as { confirmDestructive?: boolean };
    if (!a.confirmDestructive) {
      throw new Error('confirmDestructive must be true to proceed with sync pull');
    }
    return { productsPulled: 0, taxRatesPulled: 0, usersPulled: 0, error: null };
  },
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
