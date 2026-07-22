# 0.0.19 — Manual QA: Full App Walkthrough

> **Goal:** Open each page/screen in the running Tauri desktop app, verify it renders correctly, has no console errors, and all interactive elements work. Check off each item as you go.
>
> **Current state:** 2 / 45 pages checked · Updated 2026-07-22

---

## 🖥️ App Shell & Global

- [x] **Login flow + sync status** — StaffLoginScreen: username entry → PIN pad → sync connection dot (green/red/yellow) in bottom-right footer ✅
- [x] **Wrong PIN handling** — 3 failed attempts → rate-limit warning → lockout countdown
- [x] **Session lock + sync status** — Idle timeout → lock screen → sync connection dot (green/red/yellow) at bottom-right ✅
- [ ] **Session lock** — Re-enter PIN to unlock
- [ ] **AppShell layout** — Sidebar nav renders with all sections, active page highlighted, responsive collapse on narrow viewport
- [ ] **Header / StatusBar** — Shows current store name, version number, sync status dot (green/red/yellow), role badge with avatar
- [ ] **Dark/Light mode** — Theme toggle works, all pages render without visual glitches in both modes
- [ ] **Language selector** — Switch between EN and ID, all labels update, no Fluent errors in console
- [ ] **Workspace picker** — If multi-store: workspace selector shows available instances, switching reloads context
- [ ] **Global error boundary** — Navigate all pages, verify no unhandled crashes (check browser devtools console for errors)

---

## 🏪 Operations

- [ ] **POS Terminal** (`/sales`) — Product grid renders, add item to cart, adjust quantity, remove item, subtotal updates
- [ ] **Payment modal** — Cash (enter amount, change calculation), Card, QRIS QR code display, sale completes successfully
- [ ] **Receipt printing** — After sale completes, receipt prints (or shows preview if no printer)
- [ ] **KDS** (`/kds`) — Kitchen Display: incoming orders appear, status transitions (pending→preparing→ready→served), multi-layout switcher (Focus/Kanban/Metro)
- [ ] **Kiosk** (`/kiosk`) — Self-service mode: fullscreen, large product grid, no nav, add to cart, checkout, idle timeout returns to attract screen
- [ ] **Tables** (`/tables`) — Restaurant floor plan: table status colours (available/occupied/reserved), tap to open order, merge/split tables

---

## 📦 Products & Inventory

- [ ] **Product Lookup** (`/products`) — Search by name/SKU, barcode scan input, category filter chips, product cards with price/stock
- [ ] **Product Management** (`/inventory`) — Data table with all products, add/edit/delete, barcode field, stock level per product
- [ ] **Bundles** (`/bundles`) — Create/edit product bundles, bundle pricing, items listed on receipt
- [ ] **Categories** (`/categories`) — Colour-coded list, add/delete, colour picker
- [ ] **Suppliers** (`/suppliers`) — Supplier list, add/edit/delete, contact info fields
- [ ] **Purchase Orders** (`/purchase-orders`) — Create PO from supplier, add line items, receive/ship, status tracking
- [ ] **Stock Transfers** (`/stock-transfers`) — Transfer stock between locations, partial receive, audit trail
- [ ] **Stock Counts** (`/stock-counts`) — Create count sheet, enter quantities, reconcile differences
- [ ] **Inventory Adjustment** (`/inventory-adjustment`) — Manual stock-in/stock-out with reason, stock alert threshold config

---

## 💰 Sales & Orders

- [ ] **Sales History** (`/sales-history`) — Searchable list, date range filter, status filter chips, tap for detail view
- [ ] **Sales Dashboard** (`/sales-dashboard`) — Today's revenue card, orders count, top product widget, low-stock alert widget
- [ ] **EOD Report** (`/eod-report`) — End-of-Day: cash tally, payment breakdown, shift summary, print button
- [ ] **Orders/Void** (`/orders`) — Order list with search, status filters, detail view, void with reason picker, refund flow
- [ ] **Hold Order** — Park a sale, resume from held list in POS
- [ ] **Split Bill** — Divide order across payment methods, even-split, remaining tracker

---

## 💵 Finance

- [ ] **Tax Rates** (`/tax-config`) — Rate table, inclusive/exclusive toggle, category tax rates, add/edit/delete
- [ ] **Exchange Rates** (`/exchange-rates`) — Currency selector, rate display, last-updated timestamp
- [ ] **Promotions** (`/promotions`) — Promo list, create/edit (BuyXGetY, % off, fixed amount), schedule, enable/disable

---

## 👥 Customers

- [ ] **Customers** (`/customers`) — Searchable table, add/edit (name/email/phone/notes), delete
- [ ] **Gift Cards** (`/gift-cards`) — Card list, top-up flow, freeze/unfreeze, redeem at checkout
- [ ] **Loyalty** (`/loyalty`) — Account list, tier badge (Bronze/Silver/Gold/Platinum), point balance, redemption log

---

## ⚙️ Management

- [ ] **Staff** (`/staff`) — Staff table with role badges, add/edit modal with PIN hashing, deactivate/restore toggle
- [ ] **Terminals** (`/terminals`) — Terminal list, per-terminal feature overrides, online/offline status
- [ ] **Stores** (`/stores`) — Multi-store dashboard (if multi-store enabled): topology view, per-store revenue/orders/stock
- [ ] **Features** (`/features`) — Feature toggle panel with grouped switches, dependency resolution, toast feedback
- [ ] **Data Management** (`/data-management`) — Export wizard (select types, date range, password), import wizard (file upload, dry-run preview, progress)
- [ ] **Audit Log** (`/audit-log`) — Searchable log table, date range filter, event type icons
- [ ] **Offline Queue** (`/offline-queue`) — Pending/synced/failed items, retry button, sync status
- [ ] **Shifts** (`/shifts`) — Open shift with opening balance, close shift with cash count, EOD summary

---

## 📊 Reports

- [ ] **Reports Dashboard** (`/dashboard`) — Revenue chart, top products panel, inventory status widget
- [ ] **Sales Report** (`/reports`) — Bar chart (revenue), pie chart (category), heatmap (hourly), date range toggle, CSV export, print
- [ ] **Inventory Report** (`/inventory-report`) — Stock table with low-stock highlighting (amber/red), threshold input, CSV export
- [ ] **Menu Engineering** (`/menu-engineering`) — Volume vs margin scatter plot, quadrant classification (Star/Plowhorse/Puzzle/Dog), product breakdown table
- [ ] **Custom Report** (`/custom-report`) — NEW: Drag-and-drop column picker, 6 datasets (sales/inventory/customers/staff/tax_rates/shifts), search columns, run report, CSV export

---

## ⚡ Settings

- [ ] **Settings sidebar** — All categories expand/collapse, search filters sections, keyboard navigation (Arrow keys), active section highlighted
- [ ] **General settings** — Store name, address, tax ID, receipt footer, language selector
- [ ] **Appearance** — Dark/light toggle, brand colour picker with live preview, logo upload
- [ ] **Email Reports** — SMTP config (host/port/user/pass/TLS), test email button, schedule config (cadence/recipients/report types)
- [ ] **Topology Editor** — Node canvas with drag-to-move, wire connectors between nodes, zoom, pan background, simulation mode
- [ ] **Update banner** — Check for updates, dismissible banner, install action

---

## 🧪 Dev Pages

- [ ] **Design System** (`/design`) — Component showcase: buttons, inputs, cards, modals, badges, toasts, skeletons, spinners
- [ ] **Tooltip Preview** (`/tooltips`) — Tooltip positioning and content examples

---

## 🚀 Post-QA Items

- [ ] **Search all pages for console errors** — Open browser devtools console, navigate through all pages, fix any errors/warnings
- [ ] **Verify no Fluent key errors** — Check for `[@fluent/react] Error: The id "..." did not match` messages (fix missing FTL keys)
- [ ] **Check loading/empty/error states** — On every screen with lists: verify empty state renders, loading spinner shows during data fetch, error state shows on API failure
- [ ] **Responsive check (narrow viewport)** — Resize window to <768px: sidebar collapses, layouts stack vertically, touch targets remain ≥44px
- [ ] **Keyboard navigation** — Tab through interactive elements, verify focus indicators are visible, modal traps focus, Escape closes modals/dropdowns
