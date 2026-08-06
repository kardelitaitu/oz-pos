// ui/src/features/retail/retailShortcuts.ts
//
// Single source of truth for retail POS keyboard shortcuts (KEY-02).
//
// The retail function bar, the help overlay, and the keydown handler used to
// each define their own function-key map independently, which let them drift
// (the F11 contradiction: the overlay said "Toggle Fullscreen", the function
// bar said "Quick Return", and the keydown handler opened Quick Return while a
// global fullscreen listener also fired — KEY-01).
//
// Every shortcut listed here has exactly one owner per scope, so the parity
// test (retailShortcutParity.test.tsx) can assert that what is displayed
// matches what is implemented and that no key has multiple owners.

export type RetailShortcutScope = 'retail' | 'global';

export interface RetailShortcut {
  /** Canonical key label shown to users (e.g. "F1", "Ctrl+K", "?"). */
  key: string;
  /** Stable action identifier — the single implementation this key triggers. */
  action: string;
  /** Fluent message id for the localized description. */
  labelId: string;
  /** Who owns this binding. Retail-scoped keys must not be bound globally. */
  scope: RetailShortcutScope;
  /** Whether the shortcut is suppressed while the user is typing in an
   *  editable target (input/textarea/select/contenteditable). */
  editableGuard: boolean;
}

/**
 * Retail POS shortcut manifest — ordered as displayed in the help overlay.
 *
 * NOTE: F11 is owned by the retail keydown handler (Quick Return). The global
 * fullscreen F11 listener is disabled while the store-pos workspace is active
 * (see useFullscreen `enabled` option) so the key has exactly one owner here.
 */
export const RETAIL_SHORTCUTS: RetailShortcut[] = [
  { key: 'F1', action: 'pay', labelId: 'retail-shortcut-pay', scope: 'retail', editableGuard: true },
  { key: 'F2', action: 'void', labelId: 'retail-shortcut-clear', scope: 'retail', editableGuard: true },
  { key: 'F3', action: 'discount', labelId: 'retail-shortcut-discount', scope: 'retail', editableGuard: true },
  { key: 'F4', action: 'hold-resume', labelId: 'retail-shortcut-hold', scope: 'retail', editableGuard: true },
  { key: 'F5', action: 'focus-sku', labelId: 'retail-shortcut-sku', scope: 'retail', editableGuard: false },
  { key: 'F6', action: 'sales-history', labelId: 'retail-fn-history', scope: 'retail', editableGuard: true },
  { key: 'F7', action: 'customer-search', labelId: 'retail-fn-pelanggan', scope: 'retail', editableGuard: true },
  { key: 'F8', action: 'stock-inquiry', labelId: 'retail-fn-stok', scope: 'retail', editableGuard: true },
  { key: 'F9', action: 'shift', labelId: 'retail-shortcut-shift', scope: 'retail', editableGuard: true },
  { key: 'F10', action: 'options', labelId: 'retail-shortcut-options', scope: 'global', editableGuard: true },
  { key: 'F11', action: 'quick-return', labelId: 'retail-fn-quick-return', scope: 'retail', editableGuard: true },
  { key: 'F12', action: 'navigate-kds', labelId: 'kds-title', scope: 'retail', editableGuard: true },
  { key: '?', action: 'shortcut-list', labelId: 'retail-shortcut-list', scope: 'retail', editableGuard: true },
  { key: 'Ctrl+K', action: 'credit-list', labelId: 'retail-shortcut-credit', scope: 'retail', editableGuard: true },
  { key: 'Ctrl+L', action: 'filter-low-stock', labelId: 'retail-shortcut-low-stock', scope: 'retail', editableGuard: true },
  { key: 'Esc', action: 'close', labelId: 'retail-shortcut-close', scope: 'retail', editableGuard: false },
];

/** Shortcuts displayed in the retail help overlay. */
export const RETAIL_HELP_SHORTCUTS: RetailShortcut[] = RETAIL_SHORTCUTS;

/** Look up a manifest entry by its action identifier. */
export function getRetailShortcut(action: string): RetailShortcut | undefined {
  return RETAIL_SHORTCUTS.find((s) => s.action === action);
}
