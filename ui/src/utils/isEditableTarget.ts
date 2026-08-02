// ui/src/utils/isEditableTarget.ts
//
// Shared editable-target guard (KEY-03). Retail POS, the app shell, and zoom
// each historically checked `document.activeElement?.tagName !== 'INPUT'`,
// which missed textarea/select/contenteditable and editable ARIA roles, so a
// cashier typing notes or a shift note could trigger a function key. This is
// the single implementation every shortcut surface uses.

/** ARIA roles that expose an editable text value. */
const EDITABLE_ROLES = new Set([
  'textbox',
  'searchbox',
  'combobox',
  'spinbutton',
  'grid',
  'treegrid',
  'listbox',
]);

/**
 * True when `target` is an element the user can type into — input, textarea,
 * select, a contenteditable element, or an element with an editable ARIA role.
 * Used to suppress shortcuts that must not fire during text entry.
 */
export function isEditableTarget(target: EventTarget | Node | null): boolean {
  if (!(target instanceof HTMLElement)) return false;
  if (target.isContentEditable) return true;

  const tag = target.tagName;
  if (tag === 'INPUT' || tag === 'TEXTAREA' || tag === 'SELECT') return true;

  const role = target.getAttribute('role');
  if (role && EDITABLE_ROLES.has(role)) return true;

  return false;
}
