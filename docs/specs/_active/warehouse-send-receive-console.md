# Warehouse — Send/Receive Console

> **Status:** Proposed · **Area:** inventory, warehouse · **Depends on:** existing `stock_transfers` domain (Rust), workspace instance binding, product listing

**Architecture rule:** each workspace type is independently evolvable. The warehouse workspace **copies** its UI structure from the retail POS (`features/sales/`) rather than sharing components — no imports from `features/sales/*`. This keeps warehouse and retail-pos free to diverge without coupling.

---

## 1. Goal

Replace the current WarehouseScreen (stock-view + manual adjust table) with a **POS-shaped daily operations console** where staff **send** goods outbound and **receive** goods inbound — the warehouse equivalent of ringing up sales.

---

## 2. UI layout (copied from POS, edited)

```
┌─────────────────────────────────────────────────────────┐
│  Warehouse · [SEND ▸] / [RECEIVE ◂] / [STOCK]    (tab) │
├───────────────────────────┬─────────────────────────────┤
│  PRODUCT GRID             │  CART PANEL                 │
│                           │                             │
│  ┌─────────────────────┐  │  SKU-001  Widget       5 ×  │
│  │ 🔍 Search products  │  │  SKU-042  Bolt          2 ×  │
│  └─────────────────────┘  │  SKU-103  Spring        1 ×  │
│                           │                             │
│  [SKU-001] [SKU-002]     │  ──────────                  │
│  [SKU-003] [SKU-004]     │  Items: 8                    │
│                           │                             │
│                           │  [Complete SEND]            │
│                           │  [Complete RECEIVE]         │
├───────────────────────────┴─────────────────────────────┤
│  (Stock transfer history list — collapsed accordion)     │
└─────────────────────────────────────────────────────────┘
```

### 2.1 Copied from POS (self-contained copy in `features/warehouse/`)

| POS artifact | Warehouse copy | Change |
|---|---|---|
| Product grid (search, tap-to-add) | `WarehouseProductGrid.tsx` | Drop pricing highlights; show stock qty; tap adds to transfer cart not sale cart |
| Cart panel (line items, qty editing, totals) | `WarehouseCartPanel.tsx` | Drop currency/tax rows; show simple item count; SEND/RECEIVE button replaces PAY button |
| Layout (split panel, resizable cart) | `WarehouseConsole.tsx` (main screen) | Same two-panel flex layout; 3 mode tabs instead of POS features |
| `posScreenUtils.ts` → cart width clamping | `warehouseUtils.ts` | Copied, same logic |
| `usePosState.ts` (cart state machine) | `useWarehouseCart.ts` | Simplified: no modifiers, no courses, no tax, no customer. State: items + mode (send/receive) |

### 2.2 NOT copied (irrelevant to warehouse)

- Payment modal, refund modal, price override, void orders
- Customer management, table management
- Restaurant-menu, course-bars
- Barcode scanner (deferred — can add later)
- Tax calculation, discount, promotion engine
- EOD report, sales history

---

## 3. Mode tabs

### SEND tab

1. Pick a **destination** from the topology (other warehouse/store instances — bound via `inventory_locations` + `workspace_instances`)
2. Tap products in the grid → adds to cart (lines become transfer line items)
3. Adjust quantities in cart
4. Tap **"Complete Send"** → creates a draft transfer (`create_stock_transfer_scoped` with `sourceLocation=this_location`, `destinationLocation=picked`), then immediately sends it (`send_stock_transfer_scoped`) → stock leaves this location, transfer → `in_transit`
5. Shows confirmation: "Sent! Transfer #WBL-042 — 8 items to Gudang Pusat"

**Design decisions:**
- SEND creates + sends in one step (no separate "save draft" for daily ops — draft is for the form-based StockTransfersScreen which is separate).
- The destination picker is a modal/popover listing available locations from `inventory_locations` (filtered by topology connections — warehouse → warehouse fallback wires, or warehouse → store for distributing stock).

### RECEIVE tab

1. Pick an **in-transit transfer** from the list (`list_in_transit_transfers_scoped`) — these are transfers destined for this location.
2. Select one → cart pre-fills with its lines (SKU, product name, qty shipped)
3. Staff adjusts the `received_qty` per line (partial receive supported by the existing domain)
4. Tap **"Complete Receive"** → `receive_stock_transfer_scoped` with the received lines → stock lands here, transfer → `received`

**Design decisions:**
- RECEIVE is **transfer-bound** (not free-form) — every receipt is tied to an originating send, keeping the audit trail intact.
- Partial receive is supported: the cart shows shipped_qty as a readonly reference and a received_qty as the editable field.

### STOCK tab (existing view, preserved)

Keeps the current location-scoped stock table (search/sort/filter/adjust) as a smaller tertiary tab — useful for quick checks and manual corrections but not the daily console.

---

## 4. Data flow

```
USE → picks mode + picks destination (SEND) or source transfer (RECEIVE)
  ↓
WarehouseCart (local React state) holds:
  { lines: [{sku, productName, qty}],
    mode: 'send' | 'receive',
    sourceLocation?: string,
    destLocation?: string,
    transferId?: string }  // for receive mode
  ↓  user clicks "Complete Send/Receive"
  ↓
if (mode === 'send') {
  create_stock_transfer_scoped(sessionToken, ...)  ← creates draft
  add_stock_transfer_line_scoped(sessionToken, ...) ← adds each line
  send_stock_transfer_scoped(sessionToken, id)     ← set to in_transit, decrements stock
} else if (mode === 'receive') {
  receive_stock_transfer_scoped(sessionToken, id, [{lineId, receivedQty}]) ← increments stock
}
```

**Important:** the Rust commands already exist and pass tests. The warehouse screen just calls them in the right sequence. No backend changes required.

---

## 5. Files to create

```
ui/src/features/warehouse/
├── WarehouseConsole.tsx      ← NEW: main screen (copied from PosScreen.tsx, edited)
├── WarehouseProductGrid.tsx  ← NEW: product grid component (copied from POS grid)
├── WarehouseCartPanel.tsx    ← NEW: cart panel (copied from POS cart, no pricing)
├── warehouseUtils.ts         ← NEW: cart width clamp, helpers (copied from posScreenUtils.ts)
├── useWarehouseCart.ts       ← NEW: cart state hook (copied from usePosState.ts, simplified)
├── WarehouseScreen.tsx       ← DELETE (replaced by WarehouseConsole)
├── WarehouseScreen.css       ← kept, expanded
├── register.tsx              ← kept (re-register warehouse route)
├── WarehouseSendDialog.tsx   ← NEW: destination picker modal for send
├── WarehouseReceiveDialog.tsx ← NEW: in-transit transfer picker for receive
└── (Fluent keys live in the shared warehouse FTL — see §6)
```

## 6. FTL keys (en)

```
warehouse-title = Warehouse
warehouse-mode-send = Send
warehouse-mode-receive = Receive
warehouse-mode-stock = Stock View

warehouse-send-destination = Send to…
warehouse-receive-transfer = Receive from transfer
warehouse-receive-no-transfers = No in-transit transfers

warehouse-cart-complete-send = Complete Send
warehouse-cart-complete-receive = Complete Receive
warehouse-cart-item-count = { $count } item{ $count ->
  [one] 
 *[other] s
}

warehouse-cart-line-qty = Qty
warehouse-send-confirmed = Sent! { $number } — { $count } items to { $destination }
warehouse-receive-confirmed = Received! { $number } — { $count } items
```

Same keys in the ID bundle with Indonesian translations.

---

## 7. Open decisions (to resolve during implementation)

1. **One-click Send vs. two-click** (create draft + send in one step vs. create draft, review, send) — lean one-click for daily operations; draft is for the separate StockTransfersScreen.
2. **Transfer number format** — reuse existing `transfer_number` from the domain or add a warehouse-specific prefix (e.g. `WBL-???`)? The domain already generates numbers.
3. **Receive confirmation display** — toast + cart reset, or a full SuccessScreen-like confirmation? Lean toast.
4. **Stock tab visibility** — header tab or a "View Stock" button inside the console? Lean tab.

---

## 8. Test level

- Vitest: unit-test the state hook (`useWarehouseCart` — add line, remove line, change qty, switch mode)
- Vitest: the two dialogs render and submit the right commands
- Vitest: the console renders both modes
- Rust: existing stock_transfer tests already cover send/receive — no Rust changes expected
