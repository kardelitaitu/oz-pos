---
name: ui-components
description: React + TypeScript UI conventions for the OZ-POS front-end — @fluent/react for all user-visible strings, ARIA labels, eslint-plugin-jsx-a11y, strict TypeScript. Use when adding or reviewing React components, hooks, or screens.
---

# React UI & Front-end Conventions

The OZ-POS front-end is a Tauri v2 webview running React 18 + TypeScript. The UI must be **accessible** (a cashier with a screen reader is a real user), **internationalized** (we ship in many locales), and **strictly typed** (a missing `prop` should be a compile error, not a runtime crash).

---

## When to use

- Adding or modifying a React component, screen, or modal.
- Writing a hook that calls into `pos.ts`.
- Adding user-visible strings (a label, a button, an error message).
- Reviewing a UI change for accessibility, i18n, or typing issues.
- Choosing component patterns (controlled vs uncontrolled, where state lives, etc.).

---

## Golden rules

| # | Rule | Why |
|---|------|-----|
| 1 | **All user-visible strings use `@fluent/react`.** | No hardcoded English. Period. |
| 2 | **Every interactive element has an accessible name** (label, `aria-label`, or visible text). | Screen readers and keyboard nav depend on it. |
| 3 | **Strict TypeScript is on.** No `any`, no `// @ts-ignore` without a `// FIXME: ...` comment. | We catch mistakes at compile time, not in production. |
| 4 | **Components are presentational; hooks own behavior.** | Easy to test, easy to reuse. |
| 5 | **No `invoke()` in components.** Use hooks that call `pos.ts`. | Mockable, testable, discoverable. |

---

## I18n with `@fluent/react`

Every user-visible string lives in `ui/src/locales/en-US.ftl` (and other locales). The component uses `<Localized>` or `useLocalization()` — never a string literal.

```tsx
import { Localized } from '@fluent/react';

export function PayButton({ onPay, disabled }: { onPay: () => void; disabled: boolean }) {
  return (
    <button onClick={onPay} disabled={disabled} aria-label="pay">
      <Localized id="sale-pay-button">
        <span>Pay</span>
      </Localized>
    </button>
  );
}
```

```fluent
# ui/src/locales/en-US.ftl
sale-pay-button = Pay
sale-pay-button-aria = Charge the customer for the current cart
```

**Rules:**
- IDs are `feature-element[-qualifier]`. Example: `sale-pay-button`, `sale-pay-button-aria`.
- The fallback text inside `<Localized>` is **only** used by English developers in dev. The runtime always reads from the active locale.
- Never `concat` translated strings. Use Fluent's `{ $count ->` plural variants and `{ $name }` substitutions.
- For one-off strings in non-component code (e.g., a notification), call `useLocalization()` and use `l10n.getString('id')`.
- Adding a new locale? Copy `en-US.ftl`, translate, and register the `LocalizationProvider` in `App.tsx`.

---

## Accessibility (ARIA + a11y)

OZ-POS passes `eslint-plugin-jsx-a11y` in CI. The plugin catches the most common mistakes; the rest is up to you.

### Forms & inputs

- Every `<input>` has a `<label htmlFor="...">` or `aria-label`.
- Required fields have `aria-required="true"`.
- Errors are linked via `aria-describedby` and announced via `aria-invalid`.

```tsx
<label htmlFor="sku-input">
  <Localized id="inventory-sku-label"><span>SKU</span></Localized>
</label>
<input
  id="sku-input"
  type="text"
  aria-required="true"
  aria-invalid={hasError ? 'true' : 'false'}
  aria-describedby={hasError ? 'sku-error' : undefined}
  value={sku}
  onChange={(e) => setSku(e.target.value)}
/>
{hasError && (
  <p id="sku-error" role="alert">
    <Localized id="inventory-sku-error"><span>SKU is required</span></Localized>
  </p>
)}
```

### Buttons & actions

- `<button>` for actions, `<a>` for navigation. Never `<div onClick>`.
- `aria-label` for icon-only buttons.
- `aria-busy="true"` while a long-running command is in flight.

### Live regions

- Use `role="status"` (polite) for non-critical updates (cart total changed).
- Use `role="alert"` (assertive) for errors that need immediate attention.

### Keyboard support

- Every interactive control is reachable via Tab.
- Modal traps focus and restores on close.
- Custom shortcuts are documented in the help screen and respect platform conventions (Esc cancels, Enter confirms).

---

## Strict TypeScript

`ui/tsconfig.json` enables the strictest checks. Don't disable them.

```jsonc
{
  "compilerOptions": {
    "strict": true,
    "noUncheckedIndexedAccess": true,
    "noImplicitOverride": true,
    "exactOptionalPropertyTypes": true,
    "noFallthroughCasesInSwitch": true,
    "noPropertyAccessFromIndexSignature": true
  }
}
```

**Rules:**
- Never `any`. If you don't know the type, use `unknown` and narrow with a type guard.
- `// @ts-ignore` and `// @ts-expect-error` are forbidden. The latter requires a `// FIXME:` comment explaining when it can be removed.
- Discriminated unions over booleans: `{ kind: 'success', value: T } | { kind: 'error', error: AppError }`, not `{ ok: true, value: T } | { ok: false }`.
- Domain types are newtypes: `type CartId = string & { readonly __brand: 'CartId' }`. Don't pass a `Sku` where a `CartId` is expected.

---

## Component patterns

### Presentational components

```tsx
interface CartLineProps {
  sku: string;
  name: string;
  qty: number;
  unitPrice: Money;
  onRemove: (sku: string) => void;
}

export function CartLine({ sku, name, qty, unitPrice, onRemove }: CartLineProps) {
  return (
    <li>
      <span>{name}</span>
      <span>{qty} × {formatMoney(unitPrice)}</span>
      <button onClick={() => onRemove(sku)} aria-label={`remove ${name}`}>
        <Localized id="cart-line-remove"><span>Remove</span></Localized>
      </button>
    </li>
  );
}
```

- Take data and callbacks as props. Never read from context or call `pos.ts` here.
- Default to functional components. No class components.

### Hooks (behavior + state)

```tsx
export function useCart(cartId: CartId) {
  const [state, setState] = useState<UseCartState>({ status: 'loading' });
  useEffect(() => {
    let cancelled = false;
    posGetCart(cartId)
      .then((cart) => { if (!cancelled) setState({ status: 'success', cart }); })
      .catch((e: AppError) => { if (!cancelled) setState({ status: 'error', error: e }); });
    return () => { cancelled = true; };
  }, [cartId]);
  return state;
}
```

- State is a discriminated union: `'loading' | 'success' | 'error'`. No `isLoading: boolean` + `data: T | null` combos.
- Cancel in-flight requests on unmount or dependency change.

### State library

- Local UI state: `useState`, `useReducer`.
- Cross-component state: React Context, scoped to a feature.
- Cross-screen state: TanStack Query for server state, Zustand for client state. (Pick one and stick to it; don't mix.)

---

## Styling

- **CSS Modules** for component-scoped styles (`CartScreen.module.css`).
- **CSS variables** for design tokens: `--color-bg`, `--color-fg`, `--space-1`, `--radius-sm`.
- **No inline `style={{ ... }}`** for anything beyond dynamic values (e.g., a chart's bar height).
- **No hardcoded colors.** Always reference a CSS variable or a `tokens.ts` constant.
- **No `!important`.** If you need it, the cascade is wrong upstream.

```tsx
import styles from './CartScreen.module.css';

export function CartScreen() {
  return <div className={styles.root}>...</div>;
}
```

```css
/* CartScreen.module.css */
.root {
  display: grid;
  grid-template-columns: 1fr 320px;
  gap: var(--space-4);
  background: var(--color-bg);
  color: var(--color-fg);
}
```

---

## Testing

Component tests live in `ui/src/__tests__/` and mirror the source structure.

```tsx
// ui/src/__tests__/features/sales/CartLine.test.tsx
import { render, screen } from '@testing-library/react';
import { LocalizationProvider } from '@fluent/react';
import { CartLine } from '@/features/sales/CartLine';

test('renders line with formatted price', () => {
  render(
    <LocalizationProvider bundles={[]}>
      <CartLine sku="ABC" name="Coffee" qty={2}
                unitPrice={{ minor_units: 350, currency: 'USD' }}
                onRemove={() => {}} />
    </LocalizationProvider>
  );
  expect(screen.getByText(/Coffee/)).toBeInTheDocument();
  expect(screen.getByText(/\$3\.50/)).toBeInTheDocument();
});
```

**Rules:**
- Tests assert user-visible behavior, not implementation. Query by `getByRole`, `getByLabelText`, `getByText`.
- Always wrap in `<LocalizationProvider>` even with empty bundles — components may use `<Localized>`.
- Mock `pos.ts` at the module boundary, not `invoke()`. Use `vi.mock('@/api/pos', ...)`.

### Async state updates: use `renderInAct` / `renderHookInAct`

Any component or hook whose `useEffect` fires an async IPC on mount (e.g., loading settings from the secure settings DB, pulling from the cloud sync server, refreshing the offline queue) will trigger React's

> "An update to `<Component>` inside a test was not wrapped in act(...)."

…unless the initial `render()` / `renderHook()` is itself wrapped in an `act()` boundary. Use the shared helpers in `ui/src/test-utils/renderInAct.ts` instead of inlining `await act(async () => render(...))` everywhere:

```tsx
import { renderInAct, renderHookInAct } from '@/test-utils/renderInAct';

// For components:
await renderInAct(<MyComponent />);
await waitFor(() => expect(screen.getByText('…')).toBeInTheDocument());

// For isolated hook tests:
const { result } = await renderHookInAct(() => useMyHook());
expect(result.current.value).toBe(42);
```

Both helpers are async, so the calling `it` must be `async`. The async-`act` variant is used uniformly so that both sync and async mount-effect state updates are flushed before the test continues.

**Don't** fall back to plain `render()` / `renderHook()` in new tests — the project has a CI gate in `.github/workflows/ci.yml` that surfaces any act() warnings the test suite emits. The gate fails the build on act() warnings (currently warning-only while pre-existing warnings are being cleaned up; flip `exit 0` to `exit 1` once the suite is clean).

**For tests that deliberately use `vi.advanceTimersByTime` (not the async variant)** — e.g., a hook test that needs a promise to stay pending so an in-flight guard can be exercised — wrap the *resolve* step in `await act(async () => { ... })` (not just `act(...)`) so the microtask that runs the hook's `finally` block is inside the act() boundary.

---

## Folder structure

```
ui/
└── src/
    ├── api/
    │   └── pos.ts                 # only place that calls invoke()
    ├── features/
    │   └── <feature>/
    │       ├── <Feature>Screen.tsx
    │       ├── use<Feature>.ts
    │       ├── <Feature>Line.tsx  # presentational
    │       └── <Feature>.module.css
    ├── components/                # cross-feature presentational
    ├── hooks/                     # cross-feature hooks
    ├── locales/                   # en-US.ftl, id-ID.ftl, ...
    ├── types/
    │   └── domain.ts              # CartId, Sku, Money, AppError
    ├── styles/
    │   ├── tokens.css
    │   └── reset.css
    └── __tests__/
        └── <mirror of features/ and components/>
```

---

## Common pitfalls

1. **Hardcoded English in JSX** like `<button>Save</button>`. Always wrap in `<Localized>`.
2. **`<div onClick={...}>`** instead of `<button>`. Breaks keyboard nav and screen readers.
3. **`useEffect` with missing dependencies** — `eslint-plugin-react-hooks` will flag it. Add the dep or refactor.
4. **Passing `setState` directly as a prop** instead of an explicit handler. Couples parent and child too tightly.
5. **Floating-point math in `formatMoney`** — `0.1 + 0.2 !== 0.3`. Always format from `Money.minor_units`.
6. **Reading a context inside a render** without a memoized selector. Causes re-renders of every consumer.
7. **Forgetting `aria-busy` during async commands.** The button looks clickable while the request is in flight; the user clicks again.
8. **Styling with `px` everywhere** — the app must scale for tablets, large touch screens, and high-DPI. Use `rem`, `em`, or CSS variables for sizes.
9. **Rendering a component whose `useEffect` fires an async IPC without wrapping in `act()`.** Any mount effect that resolves a promise and calls `setState` will trigger a `Warning: An update to <Component> inside a test was not wrapped in act(...)`. Use `renderInAct` / `renderHookInAct` from `@/test-utils/renderInAct` instead of plain `render()` / `renderHook()` — see the "Async state updates" subsection above.

---

> last audited 19-07-26 by skill-drift-guard
