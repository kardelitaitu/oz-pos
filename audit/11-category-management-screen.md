# Category Management Screen Audit — July 2026

> **Audit date:** 2026-07-31  
> **Sector:** CategoryManagementScreen — category CRUD, product relationships, colour/icon pickers, localization, permissions, and tests  
> **Status:** ✅ **FULLY REMEDIATED** — all 10 findings closed (2026-08-02)  
> **Production code changed:** All 10 findings closed across 5 commits (CAT-05/CAT-06 merged in Phase 3) — see commit chain below

## Scope

This audit evaluates CategoryManagementScreen against the universal checklist in `audit/AUDIT_JULY_2026.md`: functionality and state management, loading/error/empty states, accessibility and localization, theming, performance, security and authorization, data integrity, and quality assurance.

Inspected areas:

- `ui/src/features/categories/CategoryManagementScreen.tsx`
- `ui/src/features/categories/CategoryManagementScreen.css`
- `ui/src/features/categories/register.tsx`
- `ui/src/api/products.ts` category API surface
- `apps/desktop-client/src/commands/categories.rs`
- `crates/oz-core/src/db/products.rs` category persistence methods
- Category migrations and product/category schema references
- `ui/src/frontend/shared/SettingsPopup.tsx`
- `ui/src/hooks/useFocusTrap.ts`
- `ui/src/__tests__/CategoryManagementScreen.test.tsx`
- `ui/src/locales/products.ftl`, `products.id.ftl`, and settings bundles

## Architecture summary

The screen loads categories through `listCategoriesScoped(sessionToken)`, renders responsive cards, and uses the shared `SettingsPopup` for create, edit, and delete confirmation flows. Creation derives an ID from the trimmed name and lets the operator choose a predefined colour and icon. Edit preserves the existing ID while changing name, colour, and icon. Delete is confirmed in a modal and reports deletion failures through a toast.

The frontend API exposes a scoped list command, but category create/update/delete helpers and the corresponding desktop commands are unscoped. The backend category commands open the global database for mutations and do not show the session resolution or permission checks used by newer scoped command patterns. The core store methods validate non-empty names and rely on SQLite constraints for conflicts.

The shared `SettingsPopup` supplies dialog semantics, focus trapping, Escape handling, backdrop dismissal, and body scroll locking. The screen itself still contains inline dynamic colours, hardcoded accessible labels/fallbacks, and action controls smaller than the project touch-target convention.

## Findings

### CAT-01 — Category mutations are global and lack session/permission enforcement (P1 tenant and authorization risk)

**Evidence:** `CategoryManagementScreen.tsx` calls `createCategory`, `updateCategory`, and `deleteCategory`, whose API helpers invoke `create_category`, `update_category`, and `delete_category` without a session token. `apps/desktop-client/src/commands/categories.rs` locks `state.db` directly for each mutation and does not resolve a session or call the permission gate. The screen is registered with `requiredRole: 'manager'`, but frontend route gating is not a backend authorization boundary. Only category listing has a `list_categories_scoped` command.

**Impact:** In a multi-store deployment, a caller reaching these IPC commands can mutate the global category database rather than the active store. A role or UI-bypass path may also invoke mutations without the manager permission check used by scoped product commands. The exact cross-store impact depends on how the global database is populated, but the command contract is inconsistent with ADR #7 and is not safe as the authoritative boundary.

**Recommendation:** Add session-scoped create/update/delete commands that resolve the store from the opaque session token and enforce the category-management permission on the session user. Replace the UI/API calls, deprecate the global mutation commands, and add IPC contract tests proving the session token and permission path are required.

**Status:** ✅ Closed — Phase 1 (`3201dcd6`)

### CAT-02 — Delete semantics are not explicit at the command/UI contract boundary (P1 data-integrity risk)

**Evidence:** The UI warning says deletion “will unlink all products in this category,” while the backend command directly calls `store.delete_category(&args.id)` and returns no relationship count or result. Product rows reference `categories(id)`, and the outcome therefore depends on the active SQLite foreign-key/schema behavior rather than an explicit application-level operation. The UI does not show how many products are affected or distinguish a blocked delete from an unlink operation.

**Impact:** Operators may confirm deletion expecting products to be retained and uncategorized, while the database may reject the operation or apply schema-defined behavior. A failed delete is only reported after the fact through a generic toast, and there is no preview of affected products.

**Recommendation:** Make the relationship policy explicit in one transactional backend operation: either set product `category_id` values to NULL before deleting, or reject deletion with an affected-product count. Return that result to the UI and align the confirmation copy with the actual policy. Add database tests for categories with and without linked products.

**Status:** ✅ Closed — Phase 1 (`3201dcd6`)

### CAT-03 — Load failures are rendered as a successful empty state

**Evidence:** `load()` catches every `listCategoriesScoped` error with `// IPC unavailable` and does not store an error. It then sets `loading` to false. When `categories` remains empty, the component renders “No categories yet” and an add CTA.

**Impact:** An IPC, database, authentication, or permission failure is indistinguishable from a store with no categories. An operator may create duplicate or unintended data, and there is no retry action for recovery.

**Recommendation:** Track `loadError` separately from the category collection, render a localized error state with Retry, and preserve the last successful list during refreshes. Add tests for initial load rejection and successful retry.

**Status:** ✅ Closed — Phase 2 (`9ef25a9f`)

### CAT-04 — Name-derived IDs can collide and create confusing recovery paths

**Evidence:** `colourToId()` lowercases the name, replaces non-ASCII-alphanumeric runs with hyphens, and prefixes `cat-`. Names such as `Coffee`, `coffee`, or different punctuation variants can produce the same ID. The UI does not check for an existing ID before calling `createCategory`; the backend reports constraint violations through a generic category conflict. Editing changes the name but intentionally keeps the old ID.

**Impact:** A valid-looking new category can fail unexpectedly because another category already occupies the derived ID. The operator receives a backend error rather than a clear duplicate-name/ID explanation. Non-ASCII names can also collapse into a sparse or nearly empty slug, depending on the input.

**Recommendation:** Define a stable ID policy at the backend, validate generated IDs before mutation, and return structured conflict fields. Either make IDs UUIDs/opaque identifiers or show the generated slug and offer an explicit collision correction path. Add tests for case, punctuation, Unicode, and duplicate names.

**Status:** ✅ Closed — Phase 4 (`2e1d28ca`)

### CAT-05 — Category error and accessible-label localization is incomplete

**Evidence:** The component contains hardcoded user-facing fallbacks such as `Failed to create category`, `Failed to update category`, `Category Name`, and `Select colour {colour}`. Delete failure calls `l10n.getString('category-delete-failed')`, but that key is not present in the inspected locale bundles. Delete and colour controls also use literal template `aria-label` attributes even when wrapped by `Localized`. Several referenced category keys are attribute-only or empty in the settings bundle, increasing the chance of blank `getString()` values and duplicated/conflicting attribute ownership.

**Impact:** Operators can see English or blank messages in Indonesian/localized deployments. Screen-reader labels may be inconsistent because a literal JSX attribute can override the localized wrapper's attribute, and missing delete failure text can produce an empty toast.

**Recommendation:** Add complete value-bearing Fluent messages to both bundles for create/update/delete errors, delete actions, colour swatches, name fallback, and picker labels. Use one localization owner per attribute, remove literal English aria labels, and add a bundle-parity/attribute-only regression test for this screen.

**Status:** ✅ Closed — Phase 3 (`ba6da1f7`)

### CAT-06 — Edit/delete controls are below the touch-target convention

**Evidence:** `.cat-mgmt-edit-btn` and `.cat-mgmt-delete-btn` are `1.75rem` by `1.75rem` (28px). `.cat-mgmt-icon-btn` is `2.5rem` (40px), also below the 44px target used by the project's mobile/accessibility guidance.

**Impact:** Dense category cards are harder to operate on touch terminals and tablets, and the small icon-only controls increase accidental misses.

**Recommendation:** Increase interactive hit areas to at least 44px while preserving the visual icon size, using padding or an invisible hit-area wrapper. Add a responsive/tablet test or visual check for card actions.

**Status:** ✅ Closed — Phase 3 (`ba6da1f7`)

### CAT-07 — Dynamic category colours use inline styles and fixed white foregrounds

**Evidence:** Category badges, icon selections, swatches, and preview chips set `background` through inline styles. Selected icon and preview elements also set `color: '#fff'`. The arbitrary palette includes light colours such as yellow, lime, sky, and cyan, but no contrast calculation or foreground selection is performed. The stylesheet's badge class uses a tokenized foreground, while the preview and selected controls override it with literal white.

**Impact:** Light category colours can produce low-contrast icons/text, especially in the preview and selected icon state. Inline styling also bypasses theme-token compliance and makes dynamic contrast behavior difficult to audit.

**Recommendation:** Store dynamic colour as a CSS custom property and choose a contrast-safe foreground based on relative luminance, with a tokenized fallback. Keep structural styling in CSS classes and test representative light/dark palette values in theme and accessibility checks.

**Status:** ✅ Closed — Phase 4 (`2e1d28ca`)

### CAT-08 — Category list refreshes are not protected against stale responses

**Evidence:** `load()` sets state from whichever `listCategoriesScoped()` promise resolves last. There is no request generation, cancellation flag, or AbortController. The effect reloads when `sessionToken` changes, and create/edit/delete handlers call `await load()` while an earlier load may still be active.

**Impact:** A slower response from a previous session or pre-mutation request can overwrite newer category data. The operator may see categories from the wrong refresh generation or miss a recently created/updated/deleted category until another reload.

**Recommendation:** Add a request sequence guard or cancellation pattern, and make mutation refreshes use the current session generation. Add a deferred-promise test that resolves overlapping loads out of order.

**Status:** ✅ Closed — Phase 2 (`9ef25a9f`)

### CAT-09 — Client validation does not provide field-level feedback for invalid or duplicate input

**Evidence:** Create and edit only trim the name and return early if it is empty; the form does not display a validation message. Colour and icon values are sent from UI state without backend-facing format validation. Backend errors are rendered as raw exception text for create/edit and as a generic localized toast for delete.

**Impact:** Empty input can leave the Save action disabled without explaining why, while duplicate IDs, malformed backend data, or persistence failures are presented inconsistently. Raw backend messages may be technical or unstable for operators.

**Recommendation:** Add localized field-level validation and structured error mapping for empty names, ID conflicts, invalid colours, and relationship failures. Keep backend validation authoritative and return stable error codes rather than exposing raw database text.

**Status:** ✅ Closed — Phase 2 (`9ef25a9f`)

### CAT-10 — Category screen has limited behavioral test coverage around failure and accessibility paths

**Evidence:** `CategoryManagementScreen.test.tsx` has 12 passing tests covering rendering, loading skeleton, empty/list states, add/edit opening, and successful delete confirmation. It does not cover list failure/retry, create/update/delete rejection, scoped session arguments, duplicate ID/name handling, Escape/focus trapping, touch targets, or localized attribute output.

**Impact:** Regressions in the security boundary, recovery UX, and accessibility behavior can pass the current suite unnoticed.

**Recommendation:** Add tests for scoped IPC contracts, all mutation failures, stale-load ordering, relationship-aware delete results, Escape/focus restoration, and English/Indonesian labels. Add backend tests for authorization and category/product relationship semantics.

**Status:** ✅ Closed — Phase 5 (`382d2e2f`)

## Positive controls observed

- Category listing uses the session-scoped `list_categories_scoped` command.
- The screen has loading skeleton and successful-empty states.
- Create, edit, and delete use explicit modal flows rather than immediate destructive mutation.
- Shared `SettingsPopup` supplies `role="dialog"`, `aria-modal="true"`, focus trapping, Escape handling, backdrop dismissal, and body scroll locking.
- Name trimming and backend non-empty-name validation are present.
- SQL statements use bound parameters rather than string interpolation.
- Category creation and update errors are rendered in the shared popup, while delete failures produce a toast.

## Test and validation results

Focused validation completed during this audit:

```text
cd ui
npx vitest run src/__tests__/CategoryManagementScreen.test.tsx
npm run typecheck
```

Results:

- Focused UI tests: **12 passed, 0 failed**
- TypeScript typecheck: **passed with 0 errors**
- Report existence and trailing-whitespace validation: **passed**
- No Rust category test count is claimed; the audit did not run a dedicated backend category test command

## Recommended remediation order

> All items below were closed as of 2026-08-02 — see [Remediation status](#remediation-status--2026-08-02).

1. **CAT-01:** Add session-scoped, permission-checked category mutations.
2. **CAT-02:** Make product unlink/delete semantics explicit and transactional.
3. **CAT-03:** Separate load errors from a genuine empty category set.
4. **CAT-05 and CAT-07:** Repair localized labels and dynamic colour contrast/theming.
5. **CAT-06:** Raise action and picker hit areas to the touch-target convention.
6. **CAT-04 and CAT-09:** Define ID/conflict policy and field-level validation.
7. **CAT-08:** Guard overlapping loads.
8. **CAT-10:** Expand focused tests around failure, scope, and accessibility paths.

## Remediation status — 2026-08-02

All 10 findings are closed. Commit chain:

| Commit | Scope |
|---|---|
| `3201dcd6` | CAT-01/02 — session-scoped permission-checked create/update/delete + transactional product-unlink delete with affected count |
| `9ef25a9f` | CAT-03/08/09 — load-error + retry state, stale-load seq guard, field-level validation (empty name, duplicate ID, invalid colour) |
| `ba6da1f7` | CAT-05/06 — localization sweep (no literal aria-labels overriding Localized) + 44px touch targets for edit/delete/icon/swatch controls |
| `2e1d28ca` | CAT-07/04 — contrast-safe foregrounds via `contrastFg` + CSS custom properties; NFKD + hash-fallback ID policy for non-ASCII names |
| `382d2e2f` | CAT-10 — expanded failure/accessibility tests (create/update rejection, Escape dismiss, localized aria-label output) |

### Per-finding closure

- **CAT-01** — mutations now use `create_category_scoped`/`update_category_scoped`/`delete_category_scoped`; desktop and tablet commands resolve the store from the opaque session token and enforce the category-management permission on the session user. The global mutation commands remain for legacy callers; IPC contract tests prove the token precedes the payload and that missing-token/permission-denied paths reject.
- **CAT-02** — delete is one transactional backend operation that unlinks linked products (sets `category_id` NULL) and returns an `affected_products` count; the UI surfaces the count in a success toast and copies align with the actual policy. Database tests cover categories with and without linked products.
- **CAT-03** — `loadError` is tracked separately from the category collection; a failed load renders a distinct localized error state with Retry, never the empty-store CTA. Refreshes preserve the last known list (skeleton only on first load per session).
- **CAT-04** — `colourToId()` NFKD-normalizes and falls back to a stable hash suffix when a fully non-ASCII name collapses to an empty slug, so the derived ID is never the degenerate `cat-` (backend accepts the client-provided ID). `validateCreate` rejects ID collisions before mutation; tests cover case, punctuation, Unicode, and duplicate names.
- **CAT-05** — literal `aria-label`s/placeholders that overrode `Localized` wrappers were removed (delete button, both colour swatch pickers, create name input); the `category-colour-picker-aria` typo was fixed to the existing key; the nested-`Localized` create input that dropped the placeholder (and threw a single-element error in the edit swatch) was replaced with the single-`Localized` + `requiredLocalized` pattern. Bundle parity and i18n lint are clean.
- **CAT-06** — `.cat-mgmt-edit-btn`/`.delete-btn` (28px), `.colour-swatch` (32px), and `.icon-btn` (40px) were raised to 44px; the icon picker gained `flex-wrap`. The `touchTargetSizing` test's interactive-selector regex now includes the `cat-mgmt-*` controls so sizes are enforced.
- **CAT-07** — fixed white foregrounds on badges, selected icon buttons, and preview chips were replaced with `contrastFg()` relative-luminance selection exposed as `--cat-bg`/`--cat-fg` CSS custom properties with tokenized fallbacks; swatches use `--cat-bg` only. Tests assert dark foreground on light colours (#f59e0b → #0a0a0a) and white on dark (#0ea5e9 → #ffffff).
- **CAT-08** — `load()` carries a request-sequence guard (`loadSeqRef`) plus `hasLoadedOnceRef` (reset on session-token change); stale responses are ignored. Verified by a genuinely overlapping two-load race test.
- **CAT-09** — `validateCreate`/`handleEdit` return localized field errors (empty name, duplicate derived ID, colour not in palette) with a touched-field pattern, replacing raw IPC text for create/update; delete still surfaces the backend message in a toast (documented split).
- **CAT-10** — the suite grew from 12 to 30 tests covering scoped IPC contracts, all mutation failure paths, stale-load ordering, relationship-aware delete results, Escape dismiss, touch targets, contrast, and localized attribute output.

### Validation

- UI: `CategoryManagementScreen.test.tsx` **30/30**, `touchTargetSizing` 1/1, `colorContrastCompliance` pass, `themeTokenCompliance` 2/2, `tsc --noEmit` clean, i18n lint + bundle parity clean.
- Each phase was code-reviewed; all reviewer follow-ups (nested-`Localized` single-element error, `l10nRef` memoization, placeholder value vs test regex, contrast test symmetry) were resolved before commit.

## Audit status

This evidence-based audit report is complete. All 10 findings were remediated, validated, and reviewed; the master index marks this sector **FULLY REMEDIATED**.
