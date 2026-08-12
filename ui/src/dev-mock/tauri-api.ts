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

const RAW_MOCK_PRODUCTS = [
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

// ADR #36/#37: enrich the raw mock catalog with the retail attribute fields
// (brand/rack/notes/unit/cost) and a deterministic popularity score so the
// default popularity sort has visible ordering in dev/demo.
const MOCK_PRODUCTS = RAW_MOCK_PRODUCTS.map((p, i) => ({
  ...p,
  cost_minor: 0,
  brand: p.name.includes('AMD') || p.name.includes('Ryzen') ? 'AMD' : null,
  rack_location: `R-${String(Math.floor(i / 4) + 1).padStart(2, '0')}`, // R-01..R-05
  notes: null,
  unit: 'pcs',
  is_active: true,
  default_supplier_id: null,
  popularity_score: RAW_MOCK_PRODUCTS.length - i, // descending demo ranking
}));

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

/** Mutable store-profile list backing the mock — renames/creates persist
 *  for the session exactly like the real DB (dev preview parity). */
let mockStores: Array<typeof MOCK_STORE> = [{ ...MOCK_STORE }];

/** Unwrap the `{ args }` envelope the API wrappers send, tolerating a
 *  bare payload for direct calls. The real commands take a named `args`
 *  argument, so the envelope is the wire shape. */
function unwrapArgs<T extends Record<string, unknown> = Record<string, unknown>>(args: unknown): T {
  const boxed = (args ?? {}) as { args?: T };
  return boxed.args ?? ((args as T | undefined) ?? ({} as T));
}

/** Live floor-plan snapshot for the analytics occupancy card: 5 of 12
 *  active tables occupied (2 seated, 1 reserved, 4 free, 1 cleaning). */
function tablesSnapshot(): Array<{
  id: string; name: string; capacity: number; pos_x: number; pos_y: number;
  shape: string; width: number; height: number; status: string;
  active_sale_id: string | null; section: string; active: boolean;
  sort_order: number;
}> {
  const statuses = [
    'occupied', 'occupied', 'occupied', 'occupied', 'occupied',
    'available', 'available', 'available', 'available',
    'reserved', 'cleaning', 'available',
  ];
  return statuses.map((status, i) => ({
    id: `table-${String(i + 1).padStart(2, '0')}`,
    name: `Table ${i + 1}`,
    capacity: i % 3 === 0 ? 6 : 4,
    pos_x: 10 + (i % 4) * 22,
    pos_y: 15 + Math.floor(i / 4) * 30,
    shape: 'circle',
    width: 8,
    height: 8,
    status,
    active_sale_id: status === 'occupied' ? `sale-table-${i + 1}` : null,
    section: i < 6 ? 'Indoor' : 'Patio',
    active: true,
    sort_order: i + 1,
  }));
}

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

// Stateful workspace instances — the real backend persists
// workspace_instances rows, and apply_topology_diff mutates them. Seed
// from localStorage so previews round-trip instance creates/archives the
// same way the real store DB does.
const MOCK_WORKSPACES_SEED = [
  { instance_id: 'ws-1', type_key: 'store-pos', store_id: 'store-1', store_name: 'TOKO TEST', name: 'Store POS', description: 'Point of Sale', icon: 'shopping-cart', layout_mode: 'default', colour: '#10b981', is_default: true },
  { instance_id: 'ws-2', type_key: 'restaurant-pos', store_id: 'store-1', store_name: 'TOKO TEST', name: 'Restaurant POS', description: 'Table service', icon: 'restaurant', layout_mode: 'fullscreen', colour: '#ef4444', is_default: false },
  { instance_id: 'ws-3', type_key: 'kds', store_id: 'store-1', store_name: 'TOKO TEST', name: 'Kitchen Display', description: 'Order display', icon: 'utensils', layout_mode: 'kds', colour: '#f59e0b', is_default: false },
  { instance_id: 'ws-4', type_key: 'inventory', store_id: 'store-1', store_name: 'TOKO TEST', name: 'Inventory Management', description: 'Stock management', icon: 'package', layout_mode: 'default', colour: '#3b82f6', is_default: false },
  { instance_id: 'ws-5', type_key: 'admin', store_id: 'store-1', store_name: 'TOKO TEST', name: 'Admin', description: 'Settings & management', icon: 'settings', layout_mode: 'default', colour: '#8b5cf6', is_default: false },
];
const MOCK_WORKSPACES_KEY = 'oz-dev-mock:workspaces';
function loadMockWorkspaces(): typeof MOCK_WORKSPACES_SEED {
  try {
    const raw = localStorage.getItem(MOCK_WORKSPACES_KEY);
    if (raw) return JSON.parse(raw) as typeof MOCK_WORKSPACES_SEED;
  } catch {
    // storage unavailable — start from seed
  }
  return MOCK_WORKSPACES_SEED;
}
function saveMockWorkspaces(): void {
  try {
    localStorage.setItem(MOCK_WORKSPACES_KEY, JSON.stringify(mockWorkspaces));
  } catch {
    // storage unavailable — keep in-memory copy for this session
  }
}
const mockWorkspaces: typeof MOCK_WORKSPACES_SEED = loadMockWorkspaces();

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
// ── Mock KDS line items (course-grouped for per-item advance) ──
const _initialKdsLineItems: Record<string, Array<Record<string, unknown>>> = {
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

// The real backend persists the kitchen queue (kds_orders 032), per-item
// line statuses (kds_line_items 105), and a daily display counter
// (kds_daily_counters 032), so a restart resumes exactly where the kitchen
// left off. Persist the whole KDS state under one key (same stateful pattern
// as the cart / sales / active-shift mocks above) so previews mirror the DB
// across reloads — previously a reload wiped the queue, reverted every
// status, and restarted ticket numbering at 104.
const MOCK_KDS_KEY = 'oz-dev-mock:kds';
function loadMockKdsState(): {
  orders: Record<string, unknown>[];
  lineItems: Record<string, Array<Record<string, unknown>>>;
} {
  try {
    const raw = localStorage.getItem(MOCK_KDS_KEY);
    if (raw) {
      const parsed = JSON.parse(raw) as {
        orders: Record<string, unknown>[];
        lineItems: Record<string, Array<Record<string, unknown>>>;
      };
      if (Array.isArray(parsed.orders) && parsed.lineItems && typeof parsed.lineItems === 'object') {
        return parsed;
      }
    }
  } catch {
    // storage unavailable or corrupt — fall through to the seed
  }
  // First load: seed the queue (and its line items) so the KDS preview
  // renders without a completed sale. Shallow-clone each line so mutations
  // never bleed into the seed literal.
  return {
    orders: [..._initialKdsOrders],
    lineItems: Object.fromEntries(
      Object.entries(_initialKdsLineItems).map(([orderId, lines]) => [
        orderId,
        lines.map((l) => ({ ...l })),
      ]),
    ),
  };
}
function saveMockKdsState(): void {
  try {
    localStorage.setItem(
      MOCK_KDS_KEY,
      JSON.stringify({ orders: mockKdsOrders, lineItems: mockKdsLineItems }),
    );
  } catch {
    // storage unavailable — keep the in-memory copies for this session
  }
}
const mockKdsState = loadMockKdsState();
const mockKdsOrders: Record<string, unknown>[] = mockKdsState.orders;
const mockKdsLineItems: Record<string, Array<Record<string, unknown>>> = mockKdsState.lineItems;
// Next ticket number = one past the highest persisted display_number (the
// backend's per-day counter), never below the seed baseline of 104.
const maxDisplay = mockKdsOrders.reduce((max, o) => {
  const n = Number(o['display_number']);
  return Number.isFinite(n) ? Math.max(max, n) : max;
}, 103);
let kdsDisplayCounter = maxDisplay + 1;

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
  // Persist the new order + its line items so a reload keeps the queue.
  saveMockKdsState();
}

// ── Lockout state (for E2E rate-limit tests) ──────────────────
// The real backend persists login attempts (login_attempts 074 + device
// 111), so a reload cannot bypass an active lockout. Persist the flat
// attempt counter (same stateful pattern as the other mocks) so a
// reloaded preview keeps enforcing the threshold — previously a reload
// cleared the counter and defeated the lockout entirely.
const MOCK_LOGIN_ATTEMPTS_KEY = 'oz-dev-mock:login-attempts';
function loadMockLoginAttempts(): Record<string, number> {
  try {
    const raw = localStorage.getItem(MOCK_LOGIN_ATTEMPTS_KEY);
    if (raw) return JSON.parse(raw) as Record<string, number>;
  } catch {
    // storage unavailable or corrupt — start with a clean counter
  }
  return {};
}
function saveMockLoginAttempts(): void {
  try {
    localStorage.setItem(MOCK_LOGIN_ATTEMPTS_KEY, JSON.stringify(loginAttempts));
  } catch {
    // storage unavailable — keep the in-memory copy for this session
  }
}
const loginAttempts: Record<string, number> = loadMockLoginAttempts();
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
// Persisted so a reloaded preview keeps its in-progress cart — the real
// backend stores active-cart lines in the store DB. Same stateful pattern
// as the user-prefs / active-shift mocks below.
const MOCK_CART_KEY = 'oz-dev-mock:cart';
function loadMockCart(): { lines: CartLine[] } {
  try {
    const raw = localStorage.getItem(MOCK_CART_KEY);
    if (raw) return JSON.parse(raw) as { lines: CartLine[] };
  } catch {
    // storage unavailable — start with an empty cart
  }
  return { lines: [] };
}
function saveMockCart(): void {
  try {
    localStorage.setItem(MOCK_CART_KEY, JSON.stringify(cartState));
  } catch {
    // storage unavailable — keep the in-memory copy for this session
  }
}
let cartState: { lines: CartLine[] } = loadMockCart();

// ── Held carts (persisted so hold/resume previews mirror SQLite) ──
interface MockHeldCart {
  id: string;
  label: string;
  cart_data: string;
  item_count: number;
  total_minor: number;
  currency: string;
  created_at: string;
  bill_type: string;
  customer_name: string | null;
  deduction_location_id: string | null;
}

const MOCK_HELD_CARTS_KEY = 'oz-dev-mock:held-carts';

function isMockHeldCart(value: unknown): value is MockHeldCart {
  if (!value || typeof value !== 'object') return false;
  const row = value as Record<string, unknown>;
  const hasNullableString = (field: string): boolean =>
    row[field] === null || typeof row[field] === 'string';

  if (
    typeof row['id'] !== 'string' || row['id'].trim() === ''
    || typeof row['label'] !== 'string'
    || typeof row['cart_data'] !== 'string'
    || typeof row['item_count'] !== 'number' || !Number.isSafeInteger(row['item_count']) || row['item_count'] < 0
    || typeof row['total_minor'] !== 'number' || !Number.isSafeInteger(row['total_minor'])
    || typeof row['currency'] !== 'string' || row['currency'].trim() === ''
    || typeof row['created_at'] !== 'string' || Number.isNaN(Date.parse(row['created_at']))
    || typeof row['bill_type'] !== 'string' || row['bill_type'].trim() === ''
    || !hasNullableString('customer_name')
    || !hasNullableString('deduction_location_id')
  ) {
    return false;
  }

  try {
    const cart = JSON.parse(row['cart_data']);
    return cart !== null && typeof cart === 'object';
  } catch {
    return false;
  }
}

function loadMockHeldCarts(): MockHeldCart[] {
  try {
    const raw = localStorage.getItem(MOCK_HELD_CARTS_KEY);
    if (raw) {
      const parsed = JSON.parse(raw) as unknown;
      if (Array.isArray(parsed)) return parsed.filter(isMockHeldCart);
    }
  } catch {
    // storage unavailable or corrupt — start with an empty hold list
  }
  return [];
}

function createMockHeldCartId(): string {
  try {
    return `held-mock-${crypto.randomUUID()}`;
  } catch {
    // Older preview runtimes may not expose randomUUID; retain uniqueness
    // with a timestamp plus a random suffix rather than array length, which
    // can repeat after a deletion in the same clock tick.
    return `held-mock-${Date.now()}-${Math.random().toString(36).slice(2)}`;
  }
}

let mockHeldCarts: MockHeldCart[] = loadMockHeldCarts();

function saveMockHeldCarts(): void {
  try {
    localStorage.setItem(MOCK_HELD_CARTS_KEY, JSON.stringify(mockHeldCarts));
  } catch {
    // storage unavailable — keep the in-memory copy for this session
  }
}

function holdMockCart(args: unknown): { id: string } {
  const input = unwrapArgs<{
    label?: unknown;
    cart_data?: unknown;
    item_count?: unknown;
    total_minor?: unknown;
    currency?: unknown;
    bill_type?: unknown;
    customer_name?: unknown;
    deduction_location_id?: unknown;
  }>(args);
  const id = createMockHeldCartId();
  mockHeldCarts.push({
    id,
    label: String(input.label ?? '').trim(),
    cart_data: String(input.cart_data ?? '{}'),
    item_count: Number(input.item_count ?? 0),
    total_minor: Number(input.total_minor ?? 0),
    currency: String(input.currency ?? 'IDR'),
    created_at: new Date().toISOString(),
    bill_type: String(input.bill_type ?? 'hold'),
    customer_name: typeof input.customer_name === 'string' ? input.customer_name : null,
    deduction_location_id: typeof input.deduction_location_id === 'string' ? input.deduction_location_id : null,
  });
  saveMockHeldCarts();
  return { id };
}

function heldCartSummary(cart: MockHeldCart): Omit<MockHeldCart, 'cart_data' | 'deduction_location_id'> {
  const { cart_data: _cartData, deduction_location_id: _deductionLocationId, ...summary } = cart;
  return summary;
}

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
interface MockSaleDetails extends MockCompletedSale {
  subtotal: { minor_units: number; currency: string };
  taxTotal: { minor_units: number; currency: string };
  tenderedMinor: number;
  lines: Array<{
    id: string;
    sku: string;
    name: string;
    qty: number;
    unit_price: { minor_units: number; currency: string };
    total_minor: number;
    tax_amount: null;
    tax_rate_id: null;
  }>;
}
// Persisted alongside the cart so sales history (and the per-sale detail
// view) survives a reload exactly like the store DB does.
const MOCK_SALES_KEY = 'oz-dev-mock:sales';
function seedMockSalesStore(): { sales: MockCompletedSale[]; details: Record<string, MockSaleDetails> } {
  const createdAt = new Date(Date.now() - 3600000).toISOString();
  // Pre-seeded sale so sales history always has at least one row.
  const seed: MockCompletedSale = {
    id: 'seed-sale-001',
    total: { minor_units: 1250, currency: 'USD' },
    lineCount: 2,
    status: 'Completed',
    paymentMethod: 'cash',
    userId: 'admin-1',
    createdAt,
  };
  return {
    sales: [seed],
    details: {
      'seed-sale-001': {
        ...seed,
        subtotal: { minor_units: 1250, currency: 'USD' },
        taxTotal: { minor_units: 0, currency: 'USD' },
        tenderedMinor: 2000,
        lines: [
          { id: 'seed-line-1', sku: 'LATTE', name: 'Caffè Latte', qty: 1, unit_price: { minor_units: 450, currency: 'USD' }, total_minor: 450, tax_amount: null, tax_rate_id: null },
          { id: 'seed-line-2', sku: 'CROISS', name: 'Butter Croissant', qty: 2, unit_price: { minor_units: 320, currency: 'USD' }, total_minor: 640, tax_amount: null, tax_rate_id: null },
        ],
      },
    },
  };
}
function loadMockSalesStore(): { sales: MockCompletedSale[]; details: Record<string, MockSaleDetails> } {
  try {
    const raw = localStorage.getItem(MOCK_SALES_KEY);
    if (raw) return JSON.parse(raw) as { sales: MockCompletedSale[]; details: Record<string, MockSaleDetails> };
  } catch {
    // storage unavailable — fall through to the seed
  }
  return seedMockSalesStore();
}
const mockSalesStore = loadMockSalesStore();
const completedSales: MockCompletedSale[] = mockSalesStore.sales;
const saleDetails: Record<string, MockSaleDetails> = mockSalesStore.details;
function saveMockSales(): void {
  try {
    localStorage.setItem(MOCK_SALES_KEY, JSON.stringify({ sales: completedSales, details: saleDetails }));
  } catch {
    // storage unavailable — keep the in-memory copies for this session
  }
}

// ── Date helpers (for seeded report data) ───────────────────────
/** List every ISO date (YYYY-MM-DD) from startDate to endDate inclusive. */
function isoDays(startDate: string, endDate: string): string[] {
  const out: string[] = [];
  const s = new Date(`${startDate}T00:00:00`);
  const e = new Date(`${endDate}T00:00:00`);
  for (let d = new Date(s); d <= e; d.setDate(d.getDate() + 1)) {
    out.push(d.toISOString().slice(0, 10));
  }
  return out;
}

/** Deterministic pseudo-random minor-unit value for mock revenue rows. */
function mockRevenue(i: number): number {
  return 2_500_000 + ((i * 7919) % 4_500_000);
}

// ── Active shift state (for pay-btn-enabled E2E test) ──────────
// The real backend persists the open shift (with its opened_at) to the
// store-scoped `shifts` table, so a restart resumes the elapsed clock from
// the original opening time. The mock previously built a fresh shift with
// `openedAt: new Date()` at module load — every page reload reset the
// resto-POS "Current Order" shift duration to 0m. Seed from localStorage
// (same stateful pattern as user prefs below) so previews behave like a
// real store DB across reloads.
const MOCK_ACTIVE_SHIFT_KEY = 'oz-dev-mock:active-shift';
// Persisted marker for an explicitly-closed shift. Without it, a reload
// after close would re-seed a fresh open shift (the demo convenience below
// applies only on the very first load) and resurrect the clock the user
// just stopped — the real DB returns no open shift after close.
const MOCK_SHIFT_CLOSED_SENTINEL = '__closed__';
function loadMockActiveShift(): Record<string, unknown> | null {
  try {
    const raw = localStorage.getItem(MOCK_ACTIVE_SHIFT_KEY);
    if (raw === MOCK_SHIFT_CLOSED_SENTINEL) return null;
    if (raw) return JSON.parse(raw) as Record<string, unknown>;
  } catch {
    // storage unavailable — fall through to the in-session default
  }
  return null;
}
function hasPersistedShiftState(): boolean {
  try {
    return localStorage.getItem(MOCK_ACTIVE_SHIFT_KEY) !== null;
  } catch {
    return false;
  }
}
function saveMockActiveShift(shift: Record<string, unknown> | null): void {
  try {
    if (shift) localStorage.setItem(MOCK_ACTIVE_SHIFT_KEY, JSON.stringify(shift));
    else localStorage.setItem(MOCK_ACTIVE_SHIFT_KEY, MOCK_SHIFT_CLOSED_SENTINEL);
  } catch {
    // storage unavailable — keep in-memory copy for this session
  }
}
let mockActiveShift: Record<string, unknown> | null = loadMockActiveShift();
if (!hasPersistedShiftState()) {
  // First-ever load in this browser (nothing persisted yet): seed an open
  // shift so the pay button and the "Current Order" shift duration render
  // in dev previews without a manual open/close cycle. Explicitly closing
  // it (or never opening one) leaves the sentinel, so reloads stay closed.
  mockActiveShift = {
    id: 'shift-1', userId: 'user-1', terminalId: null, openedAt: new Date().toISOString(), closedAt: null,
    openingBalanceMinor: 0, closingBalanceMinor: null, expectedCashMinor: null, cashDifferenceMinor: null,
    totalSalesMinor: 0, totalCashMinor: 0, totalCardMinor: 0, totalOtherMinor: 0,
    totalVoidsMinor: 0, totalRefundsMinor: 0, totalPayoutsMinor: 0, notes: '', status: 'open',
    createdAt: new Date().toISOString(), updatedAt: new Date().toISOString(),
  };
  saveMockActiveShift(mockActiveShift);
}
// Closed-shift history so the reconciliation spec can verify shifts appear
// in the Shift History table after closing. One pre-seeded closed shift
// guarantees the history table renders on every fresh page load (the older
// shift.spec asserts .shift-mgmt-table without running an open/close cycle).
// The real backend keeps closed shifts in the `shifts` table, so persist
// the history (same stateful pattern as the other mocks) — previously a
// reload reverted to just the seed and every reconciliation record
// vanished.
const MOCK_SHIFT_HISTORY_KEY = 'oz-dev-mock:shift-history';
const _initialShiftHistory: Array<Record<string, unknown>> = [
  {
    id: 'shift-seed-1', userId: 'user-1', terminalId: null,
    openedAt: new Date(Date.now() - 3600000).toISOString(), closedAt: new Date(Date.now() - 1800000).toISOString(),
    openingBalanceMinor: 100000, closingBalanceMinor: 150000, expectedCashMinor: 150000, cashDifferenceMinor: 0,
    totalSalesMinor: 50000, totalCashMinor: 50000, totalCardMinor: 0, totalOtherMinor: 0,
    totalVoidsMinor: 0, totalRefundsMinor: 0, totalPayoutsMinor: 0, notes: '', status: 'closed',
    createdAt: new Date(Date.now() - 3600000).toISOString(), updatedAt: new Date(Date.now() - 1800000).toISOString(),
  },
];
function loadMockShiftHistory(): Array<Record<string, unknown>> {
  try {
    const raw = localStorage.getItem(MOCK_SHIFT_HISTORY_KEY);
    if (raw) {
      const parsed = JSON.parse(raw) as Array<Record<string, unknown>>;
      if (Array.isArray(parsed)) return parsed;
    }
  } catch {
    // storage unavailable or corrupt — fall through to the seed
  }
  // First load: seed one closed shift so the history table renders without
  // an open/close cycle. Shallow-clone so pushes never bleed into the seed.
  return _initialShiftHistory.map((s) => ({ ...s }));
}
function saveMockShiftHistory(): void {
  try {
    localStorage.setItem(MOCK_SHIFT_HISTORY_KEY, JSON.stringify(mockShiftHistory));
  } catch {
    // storage unavailable — keep the in-memory copy for this session
  }
}
const mockShiftHistory: Array<Record<string, unknown>> = loadMockShiftHistory();
// ── User preferences (stateful mock) ─────────────────────────────
// The real backend persists per-user preferences to the store-scoped
// `user_preferences` table. The mock previously returned static values
// while discarding writes, which made the restaurant-menu hamburger
// configuration (sort / card size / font size) revert on every reload.
// Seed from localStorage so previews behave like a real store DB.
const MOCK_USER_PREFS_KEY = 'oz-dev-mock:user-prefs';
function loadMockUserPrefs(): Record<string, string> {
  try {
    const raw = localStorage.getItem(MOCK_USER_PREFS_KEY);
    if (raw) return JSON.parse(raw) as Record<string, string>;
  } catch {
    // storage unavailable — start empty
  }
  return {};
}
function saveMockUserPrefs(prefs: Record<string, string>): void {
  try {
    localStorage.setItem(MOCK_USER_PREFS_KEY, JSON.stringify(prefs));
  } catch {
    // storage unavailable — keep in-memory copy for this session
  }
}
const mockUserPrefs: Record<string, string> = loadMockUserPrefs();

// ── Topology diagram (stateful mock) ─────────────────────────────
// The real backend persists the node/wire diagram as JSON under the
// `oz-pos/topology` settings key. The mock previously returned hardcoded
// positions (and used the wrong payload shape: `label` instead of `name`,
// `from`/`to` instead of `from_node_id`/`to_node_id` — so wires never even
// loaded in the preview) while discarding saves, which made node locations
// revert on every reload. Seed from localStorage so previews round-trip
// positions exactly like a real store DB.
interface MockTopologyNode {
  id: string;
  type: string;
  name: string;
  subtitle?: string;
  x: number;
  y: number;
  tier_requirement?: string;
  telemetry_badge?: string;
  telemetry_status?: string;
  metadata?: Record<string, unknown>;
}
interface MockTopologyWire {
  id: string;
  from_node_id: string;
  to_node_id: string;
  direction: string;
  label?: string;
  from_port?: string;
  to_port?: string;
}
interface MockTopology {
  revision?: number;
  resolved_issue_keys?: string[];
  nodes: MockTopologyNode[];
  wires: MockTopologyWire[];
}

const MOCK_TOPOLOGY_KEY = 'oz-dev-mock:topology';

/** First-run canvas: matches the current preview's starting topology.
 *  Cards are 240px wide/tall, so positions sit on a spread grid (rows 80/320,
 *  columns 80/380) that never overlaps on load. Wires carry labels so the
 *  first-run canvas demonstrates the labeled-wire UX instead of empty pills. */
const MOCK_TOPOLOGY_SEED: MockTopology = {
  revision: 0,
  resolved_issue_keys: [],
  nodes: [
    { id: 'store-1', type: 'store', name: 'TOKO TEST', subtitle: 'Primary Store', x: 80, y: 80 },
    { id: 'ws-1', type: 'workspace', name: 'Store POS', subtitle: 'Point of Sale', x: 380, y: 80, metadata: { typeKey: 'store-pos', persisted: true } },
    { id: 'ws-2', type: 'workspace', name: 'Restaurant', subtitle: 'Table service', x: 380, y: 320, metadata: { typeKey: 'restaurant-pos', persisted: true } },
  ],
  wires: [
    { id: 'wire-1', from_node_id: 'store-1', from_port: 'right', to_node_id: 'ws-1', to_port: 'left', direction: 'one-way', label: 'Binds Store' },
    { id: 'wire-2', from_node_id: 'store-1', from_port: 'right', to_node_id: 'ws-2', to_port: 'left', direction: 'one-way', label: 'Binds Store' },
  ],
};

function loadMockTopology(): MockTopology {
  try {
    const raw = localStorage.getItem(MOCK_TOPOLOGY_KEY);
    if (raw) return JSON.parse(raw) as MockTopology;
  } catch {
    // storage unavailable — start from seed
  }
  return MOCK_TOPOLOGY_SEED;
}
function saveMockTopology(topology: MockTopology): void {
  try {
    localStorage.setItem(MOCK_TOPOLOGY_KEY, JSON.stringify(topology));
  } catch {
    // storage unavailable — keep in-memory copy for this session
  }
}
const mockTopology: MockTopology = loadMockTopology();

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
      saveMockLoginAttempts();
      throw new Error('Invalid credentials');
    }

    // Reset on success — persisted so a reloaded preview stays unlocked.
    delete loginAttempts[key];
    saveMockLoginAttempts();
    return {
      session: {
        user_id: staff.user_id,
        display_name: staff.role.charAt(0).toUpperCase() + staff.role.slice(1),
        role_name: staff.role,
        role_id: staff.role === 'owner' ? '1' : staff.role === 'manager' ? '2' : '3',
        // Granted keys mirror the role presets (Owner = global wildcard;
        // manager carries analytics:view; staff/cashier does not) so the
        // dev preview gates on permissions like the real backend.
        permissions: staff.role === 'owner'
          ? ['*']
          : staff.role === 'manager'
            ? ['sales:process', 'sales:view', 'reports:view', 'analytics:view', 'staff:read']
            : ['sales:process', 'sales:view'],
      },
      // audit/06 parity: the real backend mints a short-lived picker
      // ticket at login; without it the workspace picker never loads
      // (WorkspaceProvider bails when pickerTicket is null). The mock
      // must return one so browser dev previews work like the client.
      picker_ticket: `mock-picker-${staff.user_id}-${Date.now()}`,
    };
  },

  'bootstrap_owner': (_args) => {
    return {
      session: {
        user_id: 'owner-1',
        display_name: 'Owner',
        role_name: 'owner',
        role_id: '1',
        permissions: ['*'],
      },
      // audit/06 parity: the first-owner flow also mints a picker ticket.
      picker_ticket: `mock-picker-owner-1-${Date.now()}`,
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

  'list_store_profiles': () => mockStores.map((s) => ({ ...s })),
  'get_store_profile': (args) => {
    const { id } = unwrapArgs<{ id?: string }>(args);
    return mockStores.find((s) => s.id === id) ?? MOCK_STORE;
  },
  'get_primary_store': () => mockStores.find((s) => s.is_primary) ?? mockStores[0] ?? MOCK_STORE,
  'create_store_profile': (args) => {
    const payload = unwrapArgs<Partial<typeof MOCK_STORE>>(args);
    const created = {
      ...MOCK_STORE,
      ...payload,
      id: (payload.id as string | undefined) ?? `store-${Date.now()}`,
      is_primary: mockStores.length === 0,
      created_at: new Date().toISOString(),
      updated_at: new Date().toISOString(),
    };
    mockStores.push(created);
    return { ...created };
  },
  'update_store_profile': (args) => {
    const { id, ...rest } = unwrapArgs<Partial<typeof MOCK_STORE> & { id?: string }>(args);
    // id-mismatch falls back to the first profile (mock laxness — the real
    // backend returns an error for unknown ids).
    const existing = mockStores.find((s) => s.id === id) ?? mockStores[0] ?? MOCK_STORE;
    const updated = { ...existing, ...rest, id: existing.id, updated_at: new Date().toISOString() };
    mockStores = mockStores.map((s) => (s.id === updated.id ? updated : s));
    if (!mockStores.some((s) => s.id === updated.id)) mockStores.push(updated);
    return { ...updated };
  },
  'set_primary_store': (args) => {
    const { id } = unwrapArgs<{ id?: string }>(args);
    mockStores = mockStores.map((s) => ({ ...s, is_primary: s.id === id }));
    return { ...(mockStores.find((s) => s.id === id) ?? mockStores[0] ?? MOCK_STORE) };
  },
  // Deletes mutate the stateful store list so a reload (or the topology
  // editor's branch seed) no longer sees the removed branch — same
  // persistence contract as the real store_profiles row.
  'delete_store_profile': (args) => {
    const { id } = unwrapArgs<{ id?: string }>(args);
    mockStores = mockStores.filter((s) => s.id !== id);
    return null;
  },

  // ═══════════════════════════════════════════════════════════════
  // WORKSPACES (ADR #4 / #7)
  // ═══════════════════════════════════════════════════════════════

  'list_workspaces': () => mockWorkspaces,
  'list_workspaces_scoped': () => mockWorkspaces,
  'list_workspace_screens': () => [],
  'list_workspace_screens_scoped': () => [],
  'get_workspace_instance_scoped': (args) => {
    const { instanceId } = args as { instanceId: string };
    return mockWorkspaces.find(w => w.instance_id === instanceId) ?? mockWorkspaces[0];
  },
  'create_workspace_instance_scoped': (args) => {
    const req = (args as { req: Record<string, unknown> }).req;
    return { instance_id: `ws-${Date.now()}`, ...req };
  },
  // Renames mutate the stateful workspace list so a reload keeps the new
  // name — same persistence contract as the real workspace_instances row.
  'update_workspace_instance_scoped': (args) => {
    const { instanceId, name } = (args ?? {}) as { instanceId?: string; name?: string };
    const existing = mockWorkspaces.find((w) => w.instance_id === instanceId) ?? mockWorkspaces[0];
    if (existing && name !== undefined) existing.name = name;
    return existing ?? null;
  },
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

  'can_save_topology': () => true,
  'load_topology': () => ({
    revision: mockTopology.revision ?? 0,
    resolved_issue_keys: [...(mockTopology.resolved_issue_keys ?? [])],
    nodes: mockTopology.nodes.map((n) => ({ ...n })),
    wires: mockTopology.wires.map((w) => ({ ...w })),
  }),
  // The editor's Apply button saves through this command. Mirror the real
  // backend's atomic diff: apply instance creates/updates/archives AND
  // persist the diagram (node positions included) so reloads keep both the
  // node layout and the workspace instances.
  'apply_topology_diff': (args) => {
    const { workspaceCreations, workspaceUpdates, workspaceArchives, diagramNodes, diagramWires, resolvedIssueKeys, baseRevision } = (args as {
      workspaceCreations?: Array<{ id: string; type_key: string; store_id: string; name: string; description?: string; colour?: string }>;
      workspaceUpdates?: Array<{ id: string; name: string }>;
      workspaceArchives?: string[];
      diagramNodes?: MockTopologyNode[];
      diagramWires?: MockTopologyWire[];
      resolvedIssueKeys?: string[];
      baseRevision?: number;
    }) ?? {};
    // Mirror the backend's optimistic-concurrency gate (topology.rs, round
    // 133): a stale baseRevision can NEVER retry successfully, so reject
    // with the typed conflict the editor's recovery path detects (round
    // 137). Skipped when the field is absent — the real command requires
    // base_revision, so only callers that send it opt into the guard.
    const currentRevision = mockTopology.revision ?? 0;
    if (baseRevision !== undefined && baseRevision !== currentRevision) {
      throw {
        kind: 'topologyValidation',
        code: 'topology-revision-conflict',
        nodeId: null,
        wireId: null,
        portId: null,
        message: `topology revision conflict: expected ${baseRevision}, current ${currentRevision}`,
      };
    }
    for (const c of workspaceCreations ?? []) {
      mockWorkspaces.push({
        instance_id: c.id,
        type_key: c.type_key,
        store_id: c.store_id,
        store_name: 'TOKO TEST',
        name: c.name,
        description: c.description ?? '',
        icon: 'shopping-cart',
        layout_mode: 'default',
        colour: c.colour ?? '#10b981',
        is_default: false,
      });
    }
    for (const u of workspaceUpdates ?? []) {
      const inst = mockWorkspaces.find((w) => w.instance_id === u.id);
      if (inst) inst.name = u.name;
    }
    for (const id of workspaceArchives ?? []) {
      const idx = mockWorkspaces.findIndex((w) => w.instance_id === id);
      if (idx >= 0) mockWorkspaces.splice(idx, 1);
    }
    if (workspaceCreations?.length || workspaceUpdates?.length || workspaceArchives?.length) {
      saveMockWorkspaces();
    }
    if (diagramNodes) mockTopology.nodes = diagramNodes.map((n) => ({ ...n }));
    if (diagramWires) mockTopology.wires = diagramWires.map((w) => ({ ...w }));
    if (resolvedIssueKeys) mockTopology.resolved_issue_keys = [...resolvedIssueKeys];
    mockTopology.revision = (mockTopology.revision ?? 0) + 1;
    saveMockTopology(mockTopology);
    return { revision: mockTopology.revision };
  },

  'set_receipt_settings_scoped': () => null,

  'get_enabled_features': () => ({ features: ['sales', 'inventory', 'reporting', 'staff', 'settings'] }),
  'get_setting': () => '',
  'set_setting_scoped': () => null,

  'get_user_preferences': () => ({ ...mockUserPrefs }),
  'get_user_preferences_scoped': () => ({ ...mockUserPrefs }),
  'set_user_preferences': (args) => {
    const { prefs } = (args as { prefs?: Array<{ key: string; value: string }> }) ?? {};
    for (const p of prefs ?? []) mockUserPrefs[p.key] = p.value;
    saveMockUserPrefs(mockUserPrefs);
    return null;
  },
  'set_user_preferences_scoped': (args) => {
    const { prefs } = (args as { prefs?: Array<{ key: string; value: string }> }) ?? {};
    for (const p of prefs ?? []) mockUserPrefs[p.key] = p.value;
    saveMockUserPrefs(mockUserPrefs);
    return null;
  },

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

  // ADR #37 D3: fire-and-forget popularity search signal.
  'record_product_search_scoped': () => null,

  // ADR #38 D3: browser opening — dev-mock keeps window.open fallback
  // client-side; the real backend performs the open in production.
  'open_product_images_scoped': () => null,

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

  'start_sale': () => { cartState = { lines: [] }; saveMockCart(); return { cartId: `mock-cart-${Date.now()}`, deduction_location_id: 'default-loc', deductionLocationId: 'default-loc' }; },
  'start_sale_scoped': () => { cartState = { lines: [] }; saveMockCart(); return { cartId: `mock-cart-${Date.now()}`, deduction_location_id: 'default-loc', deductionLocationId: 'default-loc' }; },

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
    saveMockCart();
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
    saveMockCart();
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
    // Persist the completed sale and the now-empty cart so a reload keeps
    // history and doesn't resurrect the just-completed cart.
    saveMockSales();
    cartState = { lines: [] };
    saveMockCart();
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
    // Persist the completed sale and the now-empty cart so a reload keeps
    // history and doesn't resurrect the just-completed cart.
    saveMockSales();
    cartState = { lines: [] };
    saveMockCart();
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
    // Persist the completed sale and the now-empty cart so a reload keeps
    // history and doesn't resurrect the just-completed cart.
    saveMockSales();
    cartState = { lines: [] };
    saveMockCart();
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

  'hold_cart': (args) => holdMockCart(args),
  'hold_cart_scoped': (args) => holdMockCart(args),
  'list_active_carts': () => ({ carts: [] }),
  'get_active_cart': () => null,
  'list_held_carts': () => mockHeldCarts.map(heldCartSummary),
  'list_held_carts_scoped': () => mockHeldCarts.map(heldCartSummary),
  'list_open_bills': () => mockHeldCarts.filter((cart) => cart.bill_type === 'open_bill').map(heldCartSummary),
  'list_open_bills_scoped': () => mockHeldCarts.filter((cart) => cart.bill_type === 'open_bill').map(heldCartSummary),
  'get_held_cart': (args) => {
    const id = (args as { id?: string })?.id;
    return id ? (mockHeldCarts.find((cart) => cart.id === id) ?? null) : null;
  },
  'get_held_cart_scoped': (args) => {
    const id = (args as { id?: string })?.id;
    return id ? (mockHeldCarts.find((cart) => cart.id === id) ?? null) : null;
  },
  'delete_held_cart': (args) => {
    const id = (args as { id?: string })?.id;
    const next = mockHeldCarts.filter((cart) => cart.id !== id);
    if (next.length !== mockHeldCarts.length) {
      mockHeldCarts = next;
      saveMockHeldCarts();
    }
    return null;
  },
  'delete_held_cart_scoped': (args) => {
    const id = (args as { id?: string })?.id;
    const next = mockHeldCarts.filter((cart) => cart.id !== id);
    if (next.length !== mockHeldCarts.length) {
      mockHeldCarts = next;
      saveMockHeldCarts();
    }
    return null;
  },

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
    { id: '1', name: 'Owner', description: 'Full access to all settings', permissions: ['*'] },
    { id: '2', name: 'Manager', description: 'Daily operations and reports', permissions: ['sales:view', 'reports:view', 'analytics:view', 'staff:read'] },
    { id: '3', name: 'Cashier', description: 'Process sales and refunds', permissions: ['sales:process', 'sales:view'] },
    { id: '4', name: 'Kitchen', description: 'Kitchen display access', permissions: ['kds:view'] },
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
    saveMockActiveShift(mockActiveShift);
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
    saveMockActiveShift(mockActiveShift);
    return mockActiveShift;
  },
  'close_shift': () => {
    mockActiveShift = null;
    saveMockActiveShift(null);
    const closed: Record<string, unknown> = {
      id: `shift-${mockShiftHistory.length + 1}`, userId: 'user-1', terminalId: null, openedAt: new Date().toISOString(), closedAt: new Date().toISOString(),
      openingBalanceMinor: 100000, closingBalanceMinor: 150000, expectedCashMinor: 150000, cashDifferenceMinor: 0,
      totalSalesMinor: 50000, totalCashMinor: 50000, totalCardMinor: 0, totalOtherMinor: 0,
      totalVoidsMinor: 0, totalRefundsMinor: 0, totalPayoutsMinor: 0, notes: '', status: 'closed',
      createdAt: new Date().toISOString(), updatedAt: new Date().toISOString(),
    };
    mockShiftHistory.push(closed);
    // Persist the closed shift so a reload keeps the reconciliation record.
    saveMockShiftHistory();
    return closed;
  },
  'close_shift_scoped': () => {
    mockActiveShift = null;
    saveMockActiveShift(null);
    const closed: Record<string, unknown> = {
      id: `shift-${mockShiftHistory.length + 1}`, userId: 'user-1', terminalId: null, openedAt: new Date().toISOString(), closedAt: new Date().toISOString(),
      openingBalanceMinor: 100000, closingBalanceMinor: 150000, expectedCashMinor: 150000, cashDifferenceMinor: 0,
      totalSalesMinor: 50000, totalCashMinor: 50000, totalCardMinor: 0, totalOtherMinor: 0,
      totalVoidsMinor: 0, totalRefundsMinor: 0, totalPayoutsMinor: 0, notes: '', status: 'closed',
      createdAt: new Date().toISOString(), updatedAt: new Date().toISOString(),
    };
    mockShiftHistory.push(closed);
    // Persist the closed shift so a reload keeps the reconciliation record.
    saveMockShiftHistory();
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
  'list_stock_transfers_scoped': () => [
    { id: 'st-1', transfer_number: 'ST-001', status: 'draft', source_location: 'Warehouse A', destination_location: 'Store B', source_terminal_id: null, destination_terminal_id: null, notes: '', created_by: 'admin-1', received_by: null, created_at: new Date().toISOString(), sent_at: null, received_at: null, updated_at: new Date().toISOString() },
  ],
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
      saveMockKdsState();
      void emit('kds:orders-changed', null);
    }
    return order ?? null;
  },
  'update_kds_status_scoped': (args) => {
    const { id, status } = (args as { id?: string; status?: string }) ?? {};
    const order = mockKdsOrders.find((o) => o['id'] === id);
    if (order && status) {
      order['status'] = status;
      saveMockKdsState();
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
      saveMockKdsState();
      void emit('kds:orders-changed', null);
    }
    return item ?? null;
  },
  'update_kds_line_item_status_scoped': (args) => {
    const { itemId, status } = (args as { itemId?: string; status?: string }) ?? {};
    const item = Object.values(mockKdsLineItems).flat().find((i) => i['id'] === itemId);
    if (item && status) {
      item['item_status'] = status;
      saveMockKdsState();
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
    { id: 'po-1', po_number: 'PO-001', supplier_id: 'supplier-1', supplier_name: 'PT Teknologi Maju', status: 'pending', order_date: new Date().toISOString(), expected_date: new Date(Date.now() + 86400000).toISOString(), received_date: null, subtotal_minor: 5000000, tax_minor: 0, total_minor: 5000000, notes: '', created_by: null, created_at: new Date().toISOString(), updated_at: new Date().toISOString(), lines: [{ id: 'po-line-1', po_id: 'po-1', sku: 'CPU-R7-7800X3D', product_name: 'AMD Ryzen 7 7800X3D 8-Core', qty: 2, unit_cost_minor: 2500000, line_total_minor: 5000000 }] },
  ],
  'list_purchase_orders_scoped': () => [
    { id: 'po-1', po_number: 'PO-001', supplier_id: 'supplier-1', supplier_name: 'PT Teknologi Maju', status: 'pending', order_date: new Date().toISOString(), expected_date: new Date(Date.now() + 86400000).toISOString(), received_date: null, subtotal_minor: 5000000, tax_minor: 0, total_minor: 5000000, notes: '', created_by: null, created_at: new Date().toISOString(), updated_at: new Date().toISOString(), lines: [{ id: 'po-line-1', po_id: 'po-1', sku: 'CPU-R7-7800X3D', product_name: 'AMD Ryzen 7 7800X3D 8-Core', qty: 2, unit_cost_minor: 2500000, line_total_minor: 5000000 }] },
  ],
  'get_purchase_order': () => null,
  'create_purchase_order': () => null,
  'update_po_status': () => null,
  'receive_purchase_order': () => null,

  // ═══════════════════════════════════════════════════════════════
  // REPORTS
  // ═══════════════════════════════════════════════════════════════

  // Seeded with realistic rows (not empty arrays) so dashboard/report
  // charts actually render in browser-mode E2E. The screens guard .length
  // so empty arrays are safe, but the dashboard weekly chart is an empty
  // 0-height div ("hidden") when there are no rows.
  'get_daily_revenue': (args) => {
    const { startDate, endDate } = (args ?? {}) as { startDate?: string; endDate?: string };
    const days = isoDays(startDate ?? '2026-01-01', endDate ?? '2026-01-07');
    return days.map((date, i) => ({
      date,
      total_minor: mockRevenue(i),
      currency: 'IDR',
      sale_count: 6 + (i % 12),
    }));
  },
  'get_weekly_revenue': (args) => {
    const { startDate, endDate } = (args ?? {}) as { startDate?: string; endDate?: string };
    const days = isoDays(startDate ?? '2026-01-01', endDate ?? '2026-01-28');
    // One row per ISO week (Monday start) within the range.
    const weeks = new Map<string, { week_start: string; total_minor: number; sale_count: number }>();
    days.forEach((date, i) => {
      const d = new Date(`${date}T00:00:00`);
      const dow = (d.getDay() + 6) % 7; // Monday = 0
      const monday = new Date(d);
      monday.setDate(d.getDate() - dow);
      const key = monday.toISOString().slice(0, 10);
      const existing = weeks.get(key);
      const total = mockRevenue(i);
      if (existing) {
        existing.total_minor += total;
        existing.sale_count += 1;
      } else {
        weeks.set(key, { week_start: key, total_minor: total, sale_count: 1 });
      }
    });
    return [...weeks.values()].map((w) => ({ ...w, currency: 'IDR' }));
  },
  'get_monthly_revenue': (args) => {
    const { startDate, endDate } = (args ?? {}) as { startDate?: string; endDate?: string };
    const days = isoDays(startDate ?? '2026-01-01', endDate ?? '2026-06-01');
    const months = new Map<string, { month: string; total_minor: number; sale_count: number }>();
    days.forEach((date, i) => {
      const key = date.slice(0, 7); // YYYY-MM
      const total = mockRevenue(i);
      const existing = months.get(key);
      if (existing) {
        existing.total_minor += total;
        existing.sale_count += 1;
      } else {
        months.set(key, { month: key, total_minor: total, sale_count: 1 });
      }
    });
    return [...months.values()].map((m) => ({ ...m, currency: 'IDR' }));
  },
  'get_top_products': (args) => {
    const { orderBy } = (args ?? {}) as { orderBy?: 'revenue' | 'profit' };
    const rows = MOCK_PRODUCTS.slice(0, 5).map((p, i) => {
      const qty = 3 + (i * 7) % 30;
      const total_minor = p.price.minor_units * qty;
      const cogs_minor = (p.cost_minor ?? 0) * qty;
      return {
        product_id: p.sku,
        sku: p.sku,
        name: p.name,
        total_qty: qty,
        total_minor,
        cogs_minor,
        gross_profit_minor: total_minor - cogs_minor,
        gross_margin_percent: total_minor > 0
          ? ((total_minor - cogs_minor) / total_minor) * 100
          : 0,
      };
    });
    return orderBy === 'profit'
      ? rows.sort((a, b) => b.gross_profit_minor - a.gross_profit_minor)
      : rows;
  },
  'get_category_popularity': (args) => {
    const { topPerCategory } = (args ?? {}) as { topPerCategory?: number };
    const top = Math.max(1, Math.min(topPerCategory ?? 3, 20));
    // Deterministic pseudo-scores so the mock preview looks alive: earlier
    // products (higher stock) get higher popularity.
    const scored = MOCK_PRODUCTS.map((p, i) => ({
      sku: p.sku,
      name: p.name,
      category: p.category ?? '',
      popularity_score: Math.round((10 - (i % 10)) * 10) / 10,
    }));
    const byCat = new Map<string, typeof scored>();
    for (const s of scored) {
      const list = byCat.get(s.category) ?? [];
      list.push(s);
      byCat.set(s.category, list);
    }
    const all = scored.map((s) => s.popularity_score);
    const catalogMean = all.length
      ? all.reduce((a, b) => a + b, 0) / all.length
      : 0;
    const rows = [...byCat.entries()].map(([category, items]) => {
      items.sort((a, b) => b.popularity_score - a.popularity_score);
      const mean = items.reduce((s, it) => s + it.popularity_score, 0) / items.length;
      return {
        category_id: category || '',
        category_name: category || null,
        product_count: items.length,
        mean_score: Math.round(mean * 10) / 10,
        catalog_ratio: catalogMean > 0 ? Math.round((mean / catalogMean) * 10) / 10 : 0,
        top_products: items.slice(0, top).map((it, i) => ({
          sku: it.sku,
          name: it.name,
          popularity_score: it.popularity_score,
          rank: i + 1,
          percentile: items.length > 1
            ? Math.round(((items.length - 1 - i) / (items.length - 1)) * 100) / 100
            : 1,
        })),
      };
    });
    rows.sort((a, b) => b.mean_score - a.mean_score);
    return rows;
  },
  'get_category_popularity_trend': (args) => {
    const { startDate, endDate, granularity, topCategories } = (args ?? {}) as {
      startDate?: string;
      endDate?: string;
      granularity?: string;
      topCategories?: number;
    };
    const top = Math.max(1, Math.min(topCategories ?? 5, 10));
    // Reuse the same pseudo-scores as the standings handler, then draw a
    // small rising/falling series per category across the requested range.
    const cats = MOCK_CATEGORIES.slice(0, top).map((c, i) => ({
      id: c.id,
      name: c.name,
      base: 10 - i * 2,
    }));
    const start = new Date(startDate ?? new Date().toISOString().slice(0, 10));
    const end = new Date(endDate ?? start.toISOString().slice(0, 10));
    const step = granularity === 'monthly' ? 30 : granularity === 'weekly' ? 7 : 1;
    const points = [];
    const cursor = new Date(start);
    let guard = 0;
    while (cursor <= end && guard < 60) {
      for (const c of cats) {
        const wave = Math.sin((guard + c.base) / 3) * 2;
        points.push({
          period_start: cursor.toISOString().slice(0, 10),
          category_id: c.id,
          category_name: c.name,
          score: Math.max(0.5, Math.round((c.base + wave) * 10) / 10),
          units_sold: Math.max(0, Math.round(c.base * 3 + wave * 2)),
          distinct_transactions: Math.max(0, Math.round(c.base + wave)),
          searches: Math.max(0, Math.round(c.base / 2)),
          edits: Math.max(0, Math.round(c.base / 4)),
        });
      }
      cursor.setDate(cursor.getDate() + step);
      guard += 1;
    }
    return points;
  },
  'get_category_forecast': (args) => {
    const { startDate, endDate, granularity, topCategories } = (args ?? {}) as {
      startDate?: string;
      endDate?: string;
      granularity?: string;
      topCategories?: number;
    };
    const top = Math.max(1, Math.min(topCategories ?? 5, 10));
    // Forecast from the same pseudo-series as the trend handler: each
    // category's slope over its generated points, projected one period.
    const trend = handlers['get_category_popularity_trend']!({
      startDate,
      endDate,
      granularity,
      topCategories: top,
    }) as Array<{
      category_id: string;
      category_name: string | null;
      units_sold: number;
      score: number;
    }>;
    const byCat = new Map<string, { name: string | null; units: number[] }>();
    for (const p of trend) {
      const e = byCat.get(p.category_id) ?? { name: p.category_name, units: [] };
      e.units.push(p.units_sold);
      byCat.set(p.category_id, e);
    }
    const rows = [...byCat.entries()].map(([category_id, e]) => {
      const units = e.units;
      const n = units.length;
      const avg = n ? units.reduce((a, b) => a + b, 0) / n : 0;
      // Least-squares slope over period indices.
      const meanX = (n - 1) / 2;
      let num = 0;
      let den = 0;
      for (let i = 0; i < n; i++) {
        num += (i - meanX) * (units[i]! - avg);
        den += (i - meanX) * (i - meanX);
      }
      const slope = den > 0 ? num / den : 0;
      const forecast = Math.max(0, Math.round(avg + slope * ((n - 1) / 2)));
      return {
        category_id,
        category_name: e.name,
        forecast_units: n ? forecast : 0,
        trend_per_period: Math.round(slope * 10) / 10,
        recent_avg_units: Math.round(avg * 10) / 10,
      };
    });
    rows.sort((a, b) => b.forecast_units - a.forecast_units);
    return rows;
  },
  'get_hourly_heatmap': () => [0, 3, 5, 8, 11].flatMap((day) =>
    [9, 12, 15, 18].map((hour, i) => ({
      day_of_week: day,
      hour,
      total_minor: mockRevenue(day * 24 + hour) % 3_000_000,
      sale_count: (i * 3) % 14,
    })),
  ),
  'get_low_stock_alerts': () => [
    { product_id: 'RAM-D4-16GB-KF', sku: 'RAM-D4-16GB-KF', name: 'Kingston Fury Beast 16GB DDR4 3200', current_qty: 3, threshold: 10, currency: 'IDR', price_minor: 450000, cost_minor: 390000 },
    { product_id: 'MB-B650-ROG', sku: 'MB-B650-ROG', name: 'ASUS ROG Strix B650-A Gaming WiFi', current_qty: 5, threshold: 10, currency: 'IDR', price_minor: 2850000, cost_minor: 2500000 },
  ],
  'get_category_breakdown': () => {
    const byCat = new Map<string, { category_id: string | null; category_name: string; total_minor: number; sale_count: number }>();
    MOCK_PRODUCTS.forEach((p, i) => {
      const total = mockRevenue(i) % 4_000_000;
      const existing = byCat.get(p.category);
      if (existing) {
        existing.total_minor += total;
        existing.sale_count += 1;
      } else {
        byCat.set(p.category, { category_id: p.category, category_name: p.category, total_minor: total, sale_count: 1 });
      }
    });
    const rows = [...byCat.values()];
    const grand = rows.reduce((s, r) => s + r.total_minor, 0) || 1;
    return rows.map((r) => ({ ...r, percentage: (r.total_minor / grand) * 100 }));
  },
  'get_menu_engineering': () => ({
    rows: MOCK_PRODUCTS.slice(0, 6).map((p, i) => ({
      product_id: p.sku,
      sku: p.sku,
      name: p.name,
      total_volume: 2 + (i * 5) % 40,
      unit_price_minor: p.price.minor_units,
      unit_cost_minor: Math.floor(p.price.minor_units * 0.6),
      margin_per_unit: Math.floor(p.price.minor_units * 0.4),
      total_margin_minor: Math.floor(p.price.minor_units * 0.4) * (2 + (i * 5) % 40),
      total_revenue_minor: p.price.minor_units * (2 + (i * 5) % 40),
    })),
    median_volume: 15,
    median_margin: 500_000,
  }),
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

  // Live snapshot: 5 of 12 active tables occupied → ~42% occupancy for
  // the analytics occupancy card in browser mode.
  'list_tables': () => tablesSnapshot(),
  'list_tables_scoped': () => tablesSnapshot(),
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
  // One seeded account + tiers so the Loyalty screen's real table renders
  // deterministically (the table only exists when accounts.length > 0;
  // otherwise the empty state shows and the E2E races the loading skeleton).
  'list_loyalty_accounts_scoped': () => [
    {
      account: {
        id: 'loyalty-acc-1', customer_id: 'cust-1', points: 250, lifetime_points: 1200,
        tier_id: 'tier-1', updated_at: new Date().toISOString(), created_at: new Date().toISOString(),
      },
      tier: {
        id: 'tier-1', name: 'Gold', min_points: 100, points_per_unit: 1000,
        earn_multiplier: 1.5, colour: '#f59e0b', sort_order: 1, created_at: new Date().toISOString(),
      },
      recent_transactions: [],
      next_tier: null,
      points_to_next_tier: 0,
    },
  ],
  'earn_loyalty_points_scoped': () => null,
  'redeem_loyalty_points_scoped': () => null,
  'list_loyalty_tiers_scoped': () => [
    {
      id: 'tier-1', name: 'Gold', min_points: 100, points_per_unit: 1000,
      earn_multiplier: 1.5, colour: '#f59e0b', sort_order: 1, created_at: new Date().toISOString(),
    },
    {
      id: 'tier-2', name: 'Platinum', min_points: 500, points_per_unit: 1000,
      earn_multiplier: 2, colour: '#8b5cf6', sort_order: 2, created_at: new Date().toISOString(),
    },
  ],
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

  'list_bundles': () => [
    {
      bundle: {
        id: 'bundle-1', bundle_sku: 'BNDL-PC-1', name: 'PC Starter Bundle',
        description: 'CPU + RAM + SSD combo', bundle_price_minor: 11500000, currency: 'IDR',
        active: true, created_at: new Date().toISOString(), updated_at: new Date().toISOString(),
      },
      items: [
        { id: 'bundle-item-1', bundle_id: 'bundle-1', sku: 'CPU-R5-7600', qty: 1, unit_price_minor: 3150000 },
        { id: 'bundle-item-2', bundle_id: 'bundle-1', sku: 'RAM-D5-32GB-CR', qty: 1, unit_price_minor: 1850000 },
      ],
    },
  ],
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
  // Must return the AuditLogPageDto shape ({ items, total, has_more }) — the
  // screen calls setEntries(page.items), so an array return would make items
  // undefined and crash the render on entries.length (flippy E2E: passes only
  // when the loading skeleton is caught before the crash lands).
  'list_audit_log_scoped': () => ({
    items: [
      { id: 'audit-1', user_id: 'admin-1', action: 'sale.completed', target_type: 'sale', target_id: 'seed-sale-001', details: 'Sale completed', outcome: 'success', created_at: new Date(Date.now() - 60000).toISOString() },
      { id: 'audit-2', user_id: 'owner-1', action: 'shift.opened', target_type: 'shift', target_id: 'shift-1', details: 'Shift opened', outcome: 'success', created_at: new Date(Date.now() - 120000).toISOString() },
    ],
    total: 2,
    has_more: false,
  }),
  'get_audit_review_status_scoped': () => ({ checkpoint: null, unreviewed_count: 0 }),
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
  'list_remote_failures': () => [],
  'requeue_remote_failure': () => null,

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

// ── Scoped aliases (ADR #7) ──────────────────────────────────────
// The API layer calls the *_scoped variant for nearly every command, but
// several were only ever registered unscoped. Without these, invoke()
// returns `null` and report/inventory screens crash on `.length` reads
// (e.g. revenueData.length in DashboardScreen / SalesReportScreen),
// surfacing as error-boundary failures in browser-mode E2E. Alias the
// missing scoped names to their unscoped twins so they resolve to the
// same empty-shaped data instead of null.
const SCOPED_ALIASES: Array<[string, string]> = [
  // Reports (reporting-workflows / dashboard)
  ['get_daily_revenue_scoped', 'get_daily_revenue'],
  ['get_weekly_revenue_scoped', 'get_weekly_revenue'],
  ['get_monthly_revenue_scoped', 'get_monthly_revenue'],
  ['get_top_products_scoped', 'get_top_products'],
  ['get_category_popularity_scoped', 'get_category_popularity'],
  ['get_category_popularity_trend_scoped', 'get_category_popularity_trend'],
  ['get_category_forecast_scoped', 'get_category_forecast'],
  ['get_hourly_heatmap_scoped', 'get_hourly_heatmap'],
  ['get_category_breakdown_scoped', 'get_category_breakdown'],
  ['get_menu_engineering_scoped', 'get_menu_engineering'],
  ['build_custom_report_scoped', 'build_custom_report'],
  ['get_low_stock_alerts_scoped', 'get_low_stock_alerts'],
  // Stock counts (inventory-workflows)
  ['create_stock_count_scoped', 'create_stock_count'],
  ['get_stock_count_scoped', 'get_stock_count'],
  ['list_stock_counts_scoped', 'list_stock_counts'],
  ['get_count_lines_scoped', 'get_count_lines'],
  ['add_count_line_scoped', 'add_count_line'],
  ['update_count_line_scoped', 'update_count_line'],
  ['remove_count_line_scoped', 'remove_count_line'],
  ['complete_stock_count_scoped', 'complete_stock_count'],
  ['update_stock_count_status_scoped', 'update_stock_count_status'],
  // Categories / customers (admin screens)
  ['create_category_scoped', 'create_category'],
  ['update_category_scoped', 'update_category'],
  ['delete_category_scoped', 'delete_category'],
  ['create_customer_scoped', 'create_customer'],
  ['update_customer_scoped', 'update_customer'],
  ['delete_customer_scoped', 'delete_customer'],
];
for (const [scoped, base] of SCOPED_ALIASES) {
  if (handlers[scoped] === undefined && handlers[base] !== undefined) {
    handlers[scoped] = handlers[base];
  }
}

// Scoped commands without an unscoped twin get minimal direct stubs.
handlers['search_customers_scoped'] = (args) => {
  const { query } = (args ?? {}) as { query?: string };
  const q = (query ?? '').toLowerCase();
  const items = MOCK_CUSTOMERS.filter(
    (c) => !q || c.name.toLowerCase().includes(q),
  );
  return { items, total: items.length };
};
handlers['get_customer_history_scoped'] = (args) => {
  const { customerId } = (args ?? {}) as { customerId?: string };
  const customer =
    MOCK_CUSTOMERS.find((c) => c.id === customerId) ?? MOCK_CUSTOMERS[0]!;
  return { customer, loyalty: null, sales: [], sales_total: 0 };
};
handlers['list_in_transit_transfers_scoped'] = () => [];
handlers['print_kds_chit_scoped'] = () => true;
// HPP exposure: no historical sale lines exist in the mock, so the margin
// report is empty (the UI hides the Cost/Margin columns when it is).
handlers['get_sale_line_margins_scoped'] = () => [];
// Analytics dashboard cards — scoped commands with no unscoped twin.
// Plausible fixed shapes so the analytics grid renders in browser mode
// instead of resolving null and crashing card layouts.
handlers['get_staff_analytics_scoped'] = () => [
  { user_id: 'u1', display_name: 'Rina W.', shift_count: 12, closed_shift_count: 11, shift_sales_minor: 48000000, sale_count: 96, sale_total_minor: 92000000 },
  { user_id: 'u2', display_name: 'Budi S.', shift_count: 11, closed_shift_count: 10, shift_sales_minor: 43000000, sale_count: 88, sale_total_minor: 86000000 },
  { user_id: 'u3', display_name: 'Sari A.', shift_count: 9, closed_shift_count: 9, shift_sales_minor: 38000000, sale_count: 74, sale_total_minor: 74000000 },
  { user_id: 'u4', display_name: 'Andi P.', shift_count: 8, closed_shift_count: 7, shift_sales_minor: 31000000, sale_count: 63, sale_total_minor: 63000000 },
];
handlers['get_customer_split_scoped'] = () => ({ new_count: 84, returning_count: 47 });
handlers['get_payment_method_breakdown_scoped'] = () => [
  { payment_method: 'qris', total_minor: 98000000, sale_count: 142 },
  { payment_method: 'cash', total_minor: 74000000, sale_count: 118 },
  { payment_method: 'card', total_minor: 61000000, sale_count: 89 },
  { payment_method: 'ewallet', total_minor: 39000000, sale_count: 57 },
];
handlers['get_discounts_summary_scoped'] = () => ({
  sale_count: 406,
  discounted_sale_count: 96,
  share_percent: 6.4,
  codes: [
    { label: 'WELCOME10', redeemed_count: 41 },
    { label: 'PROMO8.8', redeemed_count: 28 },
    { label: 'LOYALTY15', redeemed_count: 17 },
    { label: 'FREESHIP', redeemed_count: 10 },
  ],
});
handlers['get_voided_sales_summary_scoped'] = () => ({ void_count: 23, void_total_minor: 5400000 });
handlers['get_basket_size_scoped'] = () => ({ sale_count: 406, avg_line_count: 3.2 });
handlers['get_inventory_turnover_scoped'] = () => ({ units_sold: 1280, stock_on_hand: 340, sku_count: 486, range_days: 30 });
handlers['get_inventory_trend_scoped'] = () => {
  const days: string[] = [];
  for (let i = 6; i >= 0; i--) {
    const d = new Date();
    d.setDate(d.getDate() - i);
    days.push(d.toISOString().slice(0, 10));
  }
  return days.map((date, i) => ({ date, units_sold: 30 + ((i * 17) % 40) }));
};
handlers['get_voided_items_scoped'] = () => [
  { name: 'Caffè Latte', qty: 6 },
  { name: 'Iced Coffee', qty: 5 },
  { name: 'Avocado Toast', qty: 4 },
  { name: 'Smoothie', qty: 3 },
];
handlers['update_kds_order_items_scoped'] = (args) => {
  const raw = (args ?? {}) as { id?: string; args?: { id?: string } };
  const id = raw.id ?? raw.args?.id ?? '';
  return mockKdsOrders.find((o) => o['id'] === id) ?? null;
};

/**
 * True when running inside a real Tauri webview (packaged app or `tauri dev`).
 *
 * The mock is aliased in for the dev server, but a real webview provides
 * `window.__TAURI_INTERNALS__` — in that case we MUST delegate to the actual
 * Rust backend instead of serving mock data (the Jul 2026 regression where
 * the unconditional alias shipped mock IPC into production builds).
 */
function hasTauriInternals(): boolean {
  try {
    return (
      typeof window !== 'undefined' &&
      typeof (window as unknown as { __TAURI_INTERNALS__?: { invoke?: unknown } })
        .__TAURI_INTERNALS__?.invoke === 'function'
    );
  } catch {
    return false;
  }
}

/** Mock Tauri invoke — delegates to real IPC in a webview, else mock data. */
export async function invoke<T>(
  cmd: string,
  args?: Record<string, unknown>,
  options?: unknown,
): Promise<T> {
  if (hasTauriInternals()) {
    // Real Tauri webview — pass straight through to the Rust backend.
    // Default args to {} like the real invoke(cmd, args = {}, options).
    return (window as unknown as {
      __TAURI_INTERNALS__: { invoke: (c: string, a?: Record<string, unknown>, o?: unknown) => Promise<T> };
    }).__TAURI_INTERNALS__.invoke(cmd, args ?? {}, options);
  }

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

/** Mock convertFileSrc — delegates to real IPC in a webview, else path as-is. */
export function convertFileSrc(path: string, protocol = 'asset'): string {
  if (hasTauriInternals()) {
    return (window as unknown as {
      __TAURI_INTERNALS__: { convertFileSrc: (p: string, pr: string) => string };
    }).__TAURI_INTERNALS__.convertFileSrc(path, protocol);
  }
  return path;
}

export function isTauri(): boolean {
  return hasTauriInternals();
}

export class Resource {}
export class Channel {}
