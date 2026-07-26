# Shadow Banding Audit — Task List

<!-- Audit stamp: 2026-07-26 · Hermes-Agent · status: ACCURATE (0 findings) · verified accurate: noise overlay covers .card::after/.modal-panel::after/.staff-login-card::after/.noise-dither::after in components.css:270-275 (matters §1 Phase 1 list matches those 4 base selectors); cited working-state commit 9a5696b exists ("fix(shadows): eliminate 8-bit GPU banding with single-layer uniform blur + noise dither"); files referenced (WorkspaceHome.css/RetailPosScreen.css/TableManagementScreen.css/SettingsPopup.css) exist; this is a living task list, not a code-claim doc -->

## Noise overlay coverage gaps

The SVG feTurbulence noise overlay at 10% opacity currently targets:
- `.card::after` ✓
- `.staff-login-card::after` ✓
- `.modal-panel::after` ✓
- `.noise-dither::after` ✓

The following elevated surfaces are NOT covered and may show banding.

## 🔴 Phase 1 — HIGH risk (--shadow-2xl / --shadow-xl)

- [ ] 1. `WorkspaceHome.css` — `.ws-grid-item` uses `--shadow-2xl`
- [ ] 2. `RetailPosScreen.css` — 6× `--shadow-2xl` on custom modal classes
- [ ] 3. `TableManagementScreen.css` — `.table-modal` uses `--shadow-2xl`
- [ ] 4. `SettingsPopup.css` — `.settings-popup` uses `--shadow-2xl`
- [ ] 5. `LicenseActivationScreen.css` — custom selector, `--shadow-2xl`
- [ ] 6. `GiftCardsScreen.css` — custom selector, `--shadow-xl`
- [ ] 7. `PromotionManagementScreen.css` — custom selector, `--shadow-xl`
- [ ] 8. `ProductManagementScreen.css` — custom selector, `--shadow-xl`
- [ ] 9. `PurchaseOrderForm.css` — custom selector, `--shadow-xl`
- [ ] 10. `SalesHistoryScreen.css` — custom selector, `--shadow-xl`
- [ ] 11. `ShiftManagementScreen.css` — custom selector, `--shadow-xl`
- [ ] 12. `StockTransfersScreen.css` — custom selector, `--shadow-xl`
- [ ] 13. `PaymentModal.css` — `.payment-modal`, `--shadow-xl`
- [ ] 14. `PriceOverrideModal.css` — custom selector, `--shadow-xl`
- [ ] 15. `DevToolbar.css` — `.dev-toolbar`, `--shadow-xl`

## 🟡 Phase 2 — MEDIUM risk (--shadow-lg)

- [ ] 16. `PosScreen.css` — 3× `--shadow-lg`
- [ ] 17. `RestaurantMenu.css` — `--shadow-lg`
- [ ] 18. `SettingsPage.css` — `--shadow-lg`
- [ ] 19. `ContextMenu.css` — `--shadow-lg`
- [ ] 20. `Tooltip.css` — `--shadow-lg`
- [ ] 21. `SettingsSelect.css` — `--shadow-lg`

## 🟢 Phase 3 — LOW risk (--shadow-md / --shadow-sm)

- [ ] 22. `MultiStoreDashboardScreen.css` — `--shadow-md`
- [ ] 23. `MenuEngineeringScreen.css` — `--shadow-md`, `--shadow-lg`
- [ ] 24. `ProductLookupScreen.css` — `--shadow-md`
- [ ] 25. `KioskScreen.css` — `--shadow-md`
- [ ] 26. `RetailPosScreen.css` — `--shadow-sm` (×2, already counted for 2xl)
- [ ] 27. `SetupWizard.css` — `--shadow-sm` (×2)
- [ ] 28. `CartPanelFooterTotals.css` — `--shadow-xs`
- [ ] 29. `CartPanelLineItem.css` — `--shadow-xs` (via CSS variable)
- [ ] 30. `PermissionDenied.css` — `--shadow-md`

## Approach

For each file:
1. Check if the component already uses `.card` or `.modal-panel` class in JSX
2. If yes — no change needed (already covered)
3. If no — either:
   a. Add the CSS selector to the noise overlay list in `components.css`
   b. Or add `.noise-dither` class to the component's JSX
4. Run `npm run lint && npm run typecheck && npm test`
5. Commit with `fix(shadows): add noise overlay to [component]`

## Verified working state (current commit)
- `9a5696b` — Base shadow tokens + ADR
