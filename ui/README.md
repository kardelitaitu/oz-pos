# `ui/` — OZ-POS React/TypeScript front-end

The Tauri v2 webview. React 18 + Vite + TypeScript with `@fluent/react`
for internationalisation and `eslint-plugin-jsx-a11y` for accessibility.

## Stack

- React 18 + react-dom
- Vite 5 (dev server + bundler)
- TypeScript 5 (strict mode)
- @fluent/react (i18n; see `src/locales/en-US.ftl`)
- @tauri-apps/api 2 (Tauri v2 IPC)
- Vitest + @testing-library/react for tests
- ESLint + @typescript-eslint + jsx-a11y

## Scripts

```bash
npm install            # one-time
npm run dev            # vite dev server on http://localhost:1420
npm run typecheck      # tsc --noEmit
npm run lint           # eslint .
npm run test           # vitest run
npm run build          # tsc -b && vite build (output: dist/)
```

`npm run dev` is what `cargo tauri dev` (from `src-tauri/`) launches;
both must be running for live-reload development.

## Structure

```
ui/src/
├── api/
│   └── pos.ts            # ONLY place that calls invoke()
├── components/
│   └── Localized.tsx     # re-export of @fluent/react's <Localized>
├── features/
│   └── sales/
│       └── CartScreen.tsx
├── locales/
│   └── en-US.ftl
├── styles/
│   ├── reset.css
│   └── tokens.css        # design tokens (colors, spacing, etc.)
├── types/
│   └── domain.ts         # Money, CartId, Sku, AppError, formatMoney
├── __tests__/
│   └── CartScreen.test.tsx
├── App.tsx
├── main.tsx
└── test-setup.ts
```

## i18n

All user-visible strings live in `src/locales/en-US.ftl` and are
referenced via `<Localized id="...">`. Hardcoded English in JSX is a
build failure (enforced by code review, the `ui-components` skill, and
the `skill-drift-guard` Fluent check).

> last audited 28-06-26 by docs-auditor
