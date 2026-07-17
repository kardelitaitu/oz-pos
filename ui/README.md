# `ui/` — OZ-POS Frontend

React 18 + TypeScript + Vite 5 + Tauri v2 webview.

## Stack

- **React 18** + react-dom
- **Vite 5** (dev server + bundler)
- **TypeScript 5** (strict: `exactOptionalPropertyTypes`, `noUncheckedIndexedAccess`)
- **@fluent/react** (i18n via `.ftl` files)
- **@tauri-apps/api 2** (IPC bridge)
- **Vitest** + **@testing-library/react** (tests)
- **ESLint** + `eslint-plugin-jsx-a11y` (accessibility enforced)

## Scripts

```bash
npm install            # one-time
npm run dev            # vite dev server on http://localhost:1420
npm run typecheck      # tsc --noEmit
npm run lint           # eslint .
npm run test           # vitest run (164 files, 2533+ tests)
npm run build          # tsc -b && vite build
```

`npm run dev` is what `cargo tauri dev` (from `apps/desktop-client/`) launches.

## Structure

```
ui/src/
├── api/
│   └── (29 per-domain files)  # Typed invoke() wrappers — no invoke() in components
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
├── locales/
│   └── en-US.ftl        # Primary locale (1900+ IDs across 25 .ftl files)
├── styles/
│   ├── reset.css
│   ├── tokens.css       # CSS custom properties (colors, spacing, typography)
│   └── components.css   # Shared component styles
├── types/
│   └── domain.ts        # Money, CartId, Sku, LineId, Product, formatMoney
├── __tests__/           # Per-screen test files (164 files, 2533+ tests)
├── App.tsx              # Root: setup guard → auth guard → AppLayout
└── main.tsx             # Entry: Fluent bundle registration + StrictMode
```

## IPC Rules

- **No `invoke()` in components** — every Tauri command has a typed wrapper in `api/pos.ts`
- Components call `pos.ts` functions; `pos.ts` owns the `invoke()` calls
- All args/results are statically typed via exported interfaces

## i18n

- All user-visible strings live in `src/locales/en-US.ftl`
- Referenced via `<Localized id="...">` from `@fluent/react`
- Hardcoded English in JSX is a build failure (enforced by code review)
- Add a new locale: copy `en-US.ftl`, translate, register in `main.tsx`

## Testing

- **Vitest** + `@testing-library/react`
- Each feature screen has a `__tests__/<Screen>.test.tsx` file
- IPC is mocked via `vi.hoisted()` → `vi.mock('@tauri-apps/api/core')`
- Fluent strings are provided inline via `FluentBundle` + `FluentResource`
- Run: `npm run test` (164 test files, 2533+ tests, ~14s)

## Conventions

| Rule | Enforcement |
|------|-------------|
| No `any` or `// @ts-ignore` without `// FIXME` | TypeScript strict mode |
| ARIA labels on all interactive elements | ESLint jsx-a11y |
| No hardcoded colors/sizes | CSS custom property tokens only |
| Presentational components, hooks own behavior | Code review |
| Every screen has a test file | `__tests__/` audit |
| Money displayed via `formatMoney()` | Import from `types/domain.ts` |

> last audited 2026-07-17 by docs-auditor
