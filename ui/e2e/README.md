# E2E Test Suite

Playwright-based end-to-end tests for OZ-POS. Tests run against the Vite
dev server with mocked Tauri IPC (`dev-mock/tauri-api.ts`) — no Rust backend
required.

## Quick Start

```bash
cd ui

# Start the dev server + run tests in one command (webServer auto-start)
npx playwright test --config e2e/playwright.config.ts

# Run a single spec
npx playwright test --config e2e/playwright.config.ts e2e/auth.spec.ts

# Headed mode (watch the browser)
npx playwright test --config e2e/playwright.config.ts --headed

# Playwright UI mode (debug with timeline, pick locators)
npx playwright test --config e2e/playwright.config.ts --ui

# Tablet viewport only
npx playwright test --config e2e/playwright.config.ts --project=tablet
```

## Architecture

### Dev-mock IPC

All Tauri `invoke()` calls are intercepted by `ui/src/dev-mock/tauri-api.ts`
via a Vite alias. The mock provides deterministic data for 18 products, 5
workspaces, 3 staff members, and cart/order lifecycle operations.

Each spec's `beforeEach` calls `page.goto('/')` which loads a fresh app
instance. The dev-mock resets on page load (no shared mutable state), so
tests are already isolated and parallel-safe.

### CSS Contract

Tests use stable CSS class selectors from the component source. When a
component adds `data-testid`, the helpers prefer `getByTestId`. The full
CSS contract is documented in each spec file's header comment.

| Component | Key selector | data-testid |
|-----------|-------------|-------------|
| StaffLoginScreen | `.staff-login-screen` | — |
| WorkspaceHome | `.workspace-home` | `workspace-home` |
| Workspace card | `.workspace-card` | — |
| ProductLookupScreen | `.product-card-btn` | — |
| RetailPosScreen cart | `.retail-cart-action-btn--pay` | `cart-panel` |
| CartPanel line | — | `cart-panel-line-item` |
| PaymentModal | `.payment-modal` | `payment-modal` |
| ReceiptPreview | `.receipt-preview-paper` | — |
| Settings sidebar | `.settings-sidebar` | `settings-sidebar` |
| Audit log | `.audit-log` | `audit-log-table` |
| Product mgmt | `.product-mgmt` | — |
| Shift mgmt | `.shift-mgmt` | — |
| KDS screen | `.kds` | — |
| Session lock | `.session-lock-card` | — |

### Test Isolation

Each test file is fully isolated:
- `page.goto('/')` resets the dev-mock state
- No shared mutable state between tests
- `storageState` in `fixtures.ts` provides per-worker auth caching
- `workers: 4` (local) or `workers: 2` (CI) runs tests in parallel

### CI Pipeline

The `e2e` job in `.github/workflows/ci.yml`:
1. Installs Playwright browsers (Chromium)
2. Starts Docker E2E backend (cloud-server + license-server)
3. Starts Vite dev server
4. Runs `npx playwright test --config e2e/playwright.config.ts --project=desktop`
5. Uploads traces on failure (7-day retention)

## Spec Files

| File | Coverage | Items |
|------|----------|-------|
| `auth.spec.ts` | Login, PIN, lockout, session persistence | E2E-4→8 |
| `sale.spec.ts` | Product grid, cart, payment, receipt | E2E-9→15 |
| `product.spec.ts` | Product list, create modal, form validation | E2E-16→19 |
| `settings.spec.ts` | Sidebar, navigation, dirty-state guard | E2E-20→22 |
| `shift.spec.ts` | Open/close shift, balance, summary | E2E-23→25 |
| `new-flows.spec.ts` | Workspace picker, session lock, KDS, audit | E2E-26→29 |
| `tablet-viewport.spec.ts` | Tablet viewport smoke, touch targets | E2E-30 |
| `api.spec.ts` | Cloud server / license server HTTP API | — |

## Writing New Tests

1. **Use hard assertions** — no `if (count > 0)` guards. Tests MUST fail on
   regressions.
2. **Prefer `waitForSelector` over `waitForTimeout`** — magic sleeps are the
   #1 cause of flaky tests. Use `expect(locator).toBeVisible()` which has
   built-in auto-wait.
3. **Use CSS class selectors** matching the component source. When a
   `data-testid` exists, prefer it over class selectors for robustness.
4. **Wrap logical steps in `test.step()`** for readable traces when a test
   fails.
5. **Clean up after yourself** — dismiss modals, close drawers, reset forms
   so subsequent tests in the same worker start clean.
