<!-- Audit stamp: 2026-07-25 · Hermes-Agent · status: ACCURATE (0 findings) · resolved F1: test counts updated to 228 files / 3476 tests · resolved F2: ui/src/locales/en-US.ftl -> per-feature .ftl bundles in ui/src/locales (48 files: en + id variants) · resolved F3: ui/src/styles/ -> ui/src/frontend/themes/ (reset.css/tokens.css/components.css/responsive.css) · resolved F4: "29 per-domain files" in api/ -> 34 .ts files · resolved F5: "Vite 5" -> ^6.0.0 in ui/package.json · verified accurate: React 18 + @fluent/react + @tauri-apps/api 2 + Vitest + eslint-plugin-jsx-a11y; api/pos.ts sole invoke() (AGENTS.md rule); formatMoney in types/domain.ts; no hardcoded colors rule -->

# `ui/` — OZ-POS Frontend

React 18 + TypeScript + Vite 6 + Tauri v2 webview.

## Stack

- **React 18** + react-dom
- **Vite 6** (dev server + bundler)
- **TypeScript 5** (strict: `exactOptionalPropertyTypes`, `noUncheckedIndexedAccess`)
- **@fluent/react** (i18n via `.ftl` files)
- **@tauri-apps/api 2** (IPC bridge)
- **Vitest** + **@testing-library/react** (tests)
- **ESLint** + `eslint-plugin-jsx-a11y` (accessibility enforced)

## Scripts

```bash
npm install            # one-time
npm run dev            # vite dev server on http://localhost:1420
npm run check:all      # chained validation: lint → typecheck → test → i18n → E2E*
npm run typecheck      # tsc --noEmit
npm run lint           # eslint .
npm run test           # vitest run (228 files, 3476 tests)
npm run build          # tsc -b && vite build
npm run e2e            # Full E2E suite: Docker → Vite → Playwright → cleanup
npm run e2e:headed     # E2E with browser visible
npm run e2e:api        # API integration tests only
npm run e2e:ui         # All UI E2E tests (excl. API)

# * E2E requires Docker; check:all skips it gracefully if Docker is unavailable
#   See e2e/README.md for full E2E documentation
```

## Install script approvals

The UI pins install-script approvals in `package.json` (`allowScripts`) so that `npm ci` does not prompt for every package with a postinstall script. This requires **npm 11+**; upgrade if `npm approve-scripts` is not recognised. When you add or update a dependency that has a postinstall script (for example, a new native-binary package), you must explicitly approve it before `npm ci` will run its install script locally:

```bash
# Approve the package for the currently installed version (recommended)
npm approve-scripts <package>

# Approve without version pinning (allows updates, less secure)
npm approve-scripts --no-allow-scripts-pin <package>
```

Only approve packages you trust and understand. The approval is written to `package.json` (`allowScripts`) and must be committed with the dependency change. The authoritative list is always in [`package.json`](./package.json); the examples below may become stale:

- `esbuild@0.25.12`
- `msw@2.15.0`

CI skips postinstall scripts entirely via `npm ci --ignore-scripts`, so these local approvals only affect development environments.

`npm run dev` is what `cargo tauri dev` (from `apps/desktop-client/`) launches.

## Structure

```
ui/src/
├── api/
│   └── (34 per-domain files)  # Typed invoke() wrappers — no invoke() in components
├── components/
│   ├── AppLayout.tsx    # Sidebar navigation, route definitions, feature gates
│   ├── Badge.tsx        # status/role badges
│   ├── Button.tsx
│   ├── Card.tsx
│   ├── RoleBadge.tsx
│   ├── ThemeProvider.tsx
│   ├── ThemeToggle.tsx
│   ├── Toast.tsx        # + ToastProvider + useToast hook
│   ├── UpdateBanner.tsx
│   └── ...              # EmptyState, ErrorState, Skeleton, Spinner
├── contexts/
│   └── AuthContext.tsx   # Staff login session state
├── features/
│   ├── audit/           # AuditLogScreen (paginated, searchable)
│   ├── auth/            # StaffLoginScreen
│   ├── categories/      # CategoryManagementScreen
│   ├── currency/        # ExchangeRateScreen (CRUD)
│   ├── customers/       # CustomerManagementScreen (WIP)
│   ├── design/          # DesignSystem showcase
│   ├── inventory/       # InventoryAdjustmentScreen
│   ├── products/        # ProductLookupScreen, ProductManagementScreen
│   ├── sales/           # PosScreen, SalesHistoryScreen, SalesDashboardScreen,
│   │                    # VoidOrdersScreen, EodReportScreen, PaymentModal
│   ├── settings/        # SettingsPage, FeatureToggleScreen, DataManagementScreen
│   ├── staff/           # StaffManagementScreen
│   ├── setup/           # SetupWizard
│   └── tax/             # TaxConfigurationScreen
├── hooks/
│   └── useFeatures.ts   # Feature flag hook for route gating
├── frontend/
│   └── themes/
│       ├── reset.css
│       ├── tokens.css   # CSS custom properties (colors, spacing, typography)
│       ├── components.css # Shared component styles
│       └── responsive.css
├── locales/
│   ├── shared.ftl       # Shared UI strings
│   ├── sales.ftl        # POS, cart, sales history
│   ├── products.ftl     # Product management
│   ├── settings.ftl     # Settings, setup wizard, sync
│   ├── ...              # Per-feature Fluent bundles (en + id variants; 48 files total)
│   └── index.ts         # Bundle loader
├── types/
│   └── domain.ts        # Money, CartId, Sku, LineId, Product, formatMoney
├── __tests__/           # Per-screen test files (228 files, 3476 tests)
├── App.tsx              # Root: setup guard → auth guard → AppLayout
└── main.tsx             # Entry: Fluent bundle registration + StrictMode
```

## IPC Rules

- **No `invoke()` in components** — every Tauri command has a typed wrapper in `api/pos.ts`
- Components call `pos.ts` functions; `pos.ts` owns the `invoke()` calls
- All args/results are statically typed via exported interfaces

## i18n

- User-visible strings live in per-feature Fluent bundles under `src/locales/` (e.g. `shared.ftl`, `sales.ftl`, `sales.id.ftl`)
- Bundles are loaded and merged by `src/locales/index.ts`
- Referenced via `<Localized id="...">` from `@fluent/react`
- Hardcoded English in JSX is a build failure (enforced by code review)
- Add a new locale: create the matching `.<code>.ftl` files for each bundle, then register the locale in `src/i18n/` and `src/main.tsx`

## Testing

- **Vitest** + `@testing-library/react`
- Each feature screen has a `__tests__/<Screen>.test.tsx` file
- IPC is mocked via `vi.hoisted()` → `vi.mock('@tauri-apps/api/core')`
- Fluent strings are provided inline via `FluentBundle` + `FluentResource`
- Run: `npm run test` (228 test files, 3476 tests, ~14s)

## Conventions

| Rule | Enforcement |
|------|-------------|
| No `any` or `// @ts-ignore` without `// FIXME` | TypeScript strict mode |
| ARIA labels on all interactive elements | ESLint jsx-a11y |
| No hardcoded colors/sizes | CSS custom property tokens only |
| Presentational components, hooks own behavior | Code review |
| Every screen has a test file | `__tests__/` audit |
| Money displayed via `formatMoney()` | Import from `types/domain.ts` |

> last audited 25-07-26 by Hermes-Agent
