# Warehouse — Send/Receive Console

> **Status:** Proposed · **Area:** inventory, warehouse · **Depends on:** existing `stock_transfers` domain (Rust), workspace instance binding, product listing

**Architecture rule:** each workspace type is independently evolvable. The warehouse workspace **copies** its UI structure from the retail POS (`features/sales/`) rather than sharing components — no imports from `features/sales/*`. This keeps warehouse and retail-pos free to diverge without coupling.

---

## 1. Goal

Replace the current WarehouseScreen (stock-view + manual adjust table) with a **POS-shaped daily operations console** where staff **send** goods outbound and **receive** goods inbound — the warehouse equivalent of ringing up sales.

---

## 1b. Quick menu (function keys) — popup sessions

The console has a **function-key quick menu** for fast daily operations, mirroring the retail workspace's F-key bar pattern (retail uses `retailShortcuts.ts` manifest + `RetailFnBar.tsx`; warehouse gets its own **copy** — `warehouseShortcuts.ts` + `WarehouseFnBar.tsx`, no shared imports, KEY-02 single-source rule applies).

| Key | Action | Opens |
|---|---|---|
| **F1** | `receive-popup` | **Incoming popup session** — receive popup |
| **F2** | `send-popup` | **Outgoing popup session** — send popup |
| **F3–F10** | *(placeholder)* | Reserved — rendered in the FnBar as placeholders, no behavior yet |
| **F11** | `fullscreen` | Toggle fullscreen — **owned by the global shell binding** (`useFullscreen`), which is already active for every workspace except `store-pos` (retail claims F11 there, KEY-01). The warehouse does not re-bind F11; the FnBar shows an F11 badge pointing at the existing fullscreen toggle. |
| **F12** | *(reserved)* | Reserved for future use |

### Popup session behavior

- **F1 / F2** toggle a **persistent popup session** (a floating overlay, not a full navigation) that **stays open until explicitly dismissed** — "can be held open as needed".
- The popup is a **mini console**: product grid + cart inside the overlay, sized to be usable while the main screen stays visible behind it.
- It can be **held** (left open) while the operator does something else (checks the stock tab, another popup), and **dismissed** with Esc or the ✕ button.
- Multiple sessions: F1 and F2 can be open **simultaneously** (incoming + outgoing at the same time), each a separate overlay — or one at a time if a single-overlay constraint is preferred (open decision).
- Opening the same popup again (F1 while F1 is open) **focuses/brings it to front** instead of stacking duplicates.
- Draggable/pinnable (open decision): a pinned popup stays attached; unpinned ones can be moved.

### Why popups instead of tabs-only

- Operators often process **interleaved** send and receive actions during a day (a truck arrives while outgoing picks are being packed). Popups let them switch instantly with two keys instead of tab-switching.
- The **SEND / RECEIVE / STOCK tabs remain** in the main console for full-screen workflows; the F-keys are the fast path.

### Implementation notes

- Manifest: `warehouseShortcuts.ts` exports `WAREHOUSE_SHORTCUTS` (key, action, labelId, scope, editableGuard) + `getWarehouseShortcut()` — the FnBar, help overlay, and keydown handler all read from it (KEY-02 parity test included). F3–F10 and F12 entries carry placeholder actions so they render in the FnBar with a reserved label.
- Keydown handler: `e.key === 'F1'` / `'F2'` with `editableGuard: true` (suppressed while typing in an input). F11 is **not** bound here — the global shell fullscreen binding already owns it for the warehouse workspace (KEY-01 single-owner rule).
- Help overlay: `?` opens a shortcut list rendered from the same manifest.
- `WarehouseFnBar.tsx`: bottom toolbar showing `F1 Receive` / `F2 Send`, F3–F10 placeholders, and an `F11 Fullscreen` badge (calls `useFullscreen().toggleFullscreen`), pure presentational with callbacks wired in the console.



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
├── WarehouseFnBar.tsx        ← NEW: F-key quick-menu toolbar (copied from RetailFnBar.tsx, edited)
├── warehouseShortcuts.ts     ← NEW: F-key manifest (copied from retailShortcuts.ts, edited)
├── warehouseShortcutParity.test.tsx ← NEW: manifest ⇄ FnBar ⇄ keydown parity test
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

warehouse-fn-receive = Receive
warehouse-fn-send = Send
warehouse-fn-reserved = { $key }
warehouse-fn-fullscreen = Fullscreen
warehouse-fn-bar-aria = Function keys
warehouse-popup-receive-title = Incoming session
warehouse-popup-send-title = Outgoing session
warehouse-popup-pin = Pin popup
warehouse-popup-close = Close
```

Same keys in the ID bundle with Indonesian translations.

---

## 7. Open decisions (to resolve during implementation)

1. **One-click Send vs. two-click** (create draft + send in one step vs. create draft, review, send) — lean one-click for daily operations; draft is for the separate StockTransfersScreen.
2. **Transfer number format** — reuse existing `transfer_number` from the domain or add a warehouse-specific prefix (e.g. `WBL-???`)? The domain already generates numbers.
3. **Receive confirmation display** — toast + cart reset, or a full SuccessScreen-like confirmation? Lean toast.
4. **Stock tab visibility** — header tab or a "View Stock" button inside the console? Lean tab.
5. **Popup concurrency** — can F1 (incoming) and F2 (outgoing) popups be open simultaneously, or one overlay at a time? Lean simultaneous.
6. **Popup hold/pin** — should popups be draggable and pinnable (stay open), or fixed position? Lean: fixed position, stays open until Esc/✕ (hold = just don't close it); pinning is a nice-to-have.

---

## 8. Test level

- Vitest: unit-test the state hook (`useWarehouseCart` — add line, remove line, change qty, switch mode)
- Vitest: the two dialogs render and submit the right commands
- Vitest: the console renders both modes
- Vitest: F-key parity test — `warehouseShortcuts` manifest ⇄ `WarehouseFnBar` labels ⇄ keydown handler agree (KEY-02 pattern); F3–F10/F12 are placeholders (no keydown handlers), F11 is owned by the shell (asserted, not re-bound)
- Vitest: F1/F2 open and dismiss popup sessions; Esc closes; re-press focuses instead of stacking
- Rust: existing stock_transfer tests already cover send/receive — no Rust changes expected
