// ── ui/src/features/warehouse/warehouseShortcuts.ts ──────────────────────
// Single source of truth for warehouse console keyboard shortcuts (KEY-02).
// Every shortcut listed here has exactly one owner per scope.
// Copied from retailShortcuts.ts — self-contained, no shared imports.

export type WarehouseShortcutScope = 'warehouse' | 'global';

export interface WarehouseShortcut {
  /** Canonical key label shown to users (e.g. "F1", "?"). */
  key: string;
  /** Stable action identifier — the single implementation this key triggers. */
  action: string;
  /** Fluent message id for the localized description. */
  labelId: string;
  /** Who owns this binding. Warehouse-scoped keys must not be bound globally. */
  scope: WarehouseShortcutScope;
  /** Whether the shortcut is suppressed while the user is typing in an
   *  editable target (input/textarea/select/contenteditable). */
  editableGuard: boolean;
  /** True for unbound keys that only render in the FnBar as placeholders. */
  placeholder?: boolean;
}

/**
 * Warehouse console shortcut manifest — ordered as displayed in the help overlay.
 *
 * F1–F5 are bound actions. F6–F10 and F12 are placeholders (rendered in
 * the FnBar with no keydown handler). F11 appears for display but the
 * global shell fullscreen binding owns it (KEY-01), so the warehouse
 * keydown handler never binds F11.
 */
export const WAREHOUSE_SHORTCUTS: WarehouseShortcut[] = [
  // ── Bound actions ───────────────────────────────────────
  { key: 'F1',  action: 'receive-popup',  labelId: 'warehouse-fn-receive',  scope: 'warehouse', editableGuard: true },
  { key: 'F2',  action: 'send-popup',     labelId: 'warehouse-fn-send',     scope: 'warehouse', editableGuard: true },
  { key: 'F3',  action: 'count-popup',    labelId: 'warehouse-fn-count',    scope: 'warehouse', editableGuard: true },
  { key: 'F4',  action: 'stock',          labelId: 'warehouse-fn-stock',    scope: 'warehouse', editableGuard: true },
  { key: 'F5',  action: 'print',          labelId: 'warehouse-fn-print',    scope: 'warehouse', editableGuard: true },

  // ── Placeholders (rendered, no handler) ──────────────────
  { key: 'F6',  action: 'placeholder-f6',  labelId: 'warehouse-fn-reserved', scope: 'warehouse', editableGuard: true, placeholder: true },
  { key: 'F7',  action: 'placeholder-f7',  labelId: 'warehouse-fn-reserved', scope: 'warehouse', editableGuard: true, placeholder: true },
  { key: 'F8',  action: 'placeholder-f8',  labelId: 'warehouse-fn-reserved', scope: 'warehouse', editableGuard: true, placeholder: true },
  { key: 'F9',  action: 'placeholder-f9',  labelId: 'warehouse-fn-reserved', scope: 'warehouse', editableGuard: true, placeholder: true },
  { key: 'F10', action: 'placeholder-f10', labelId: 'warehouse-fn-reserved', scope: 'warehouse', editableGuard: true, placeholder: true },

  // ── Display-only (shell-owned or future) ────────────────
  { key: 'F11', action: 'fullscreen',     labelId: 'warehouse-fn-fullscreen', scope: 'global',   editableGuard: true, placeholder: true },
  { key: 'F12', action: 'placeholder-f12', labelId: 'warehouse-fn-reserved',  scope: 'warehouse', editableGuard: true, placeholder: true },

  // ── Help overlay ────────────────────────────────────────
  { key: '?',   action: 'shortcut-list',  labelId: 'warehouse-shortcut-list', scope: 'warehouse', editableGuard: false },
  { key: 'Esc', action: 'close',          labelId: 'warehouse-shortcut-close', scope: 'warehouse', editableGuard: false },
];

/** Shortcuts displayed in the help overlay. */
export const WAREHOUSE_HELP_SHORTCUTS: WarehouseShortcut[] =
  WAREHOUSE_SHORTCUTS;

/** Active (non-placeholder) actions — the ones the keydown handler binds. */
export const ACTIVE_SHORTCUT_ACTIONS = new Set(
  WAREHOUSE_SHORTCUTS.filter((s) => !s.placeholder).map((s) => s.action),
);

/** Look up a manifest entry by its action identifier. */
export function getWarehouseShortcut(action: string): WarehouseShortcut | undefined {
  return WAREHOUSE_SHORTCUTS.find((s) => s.action === action);
}