// ui/src/utils/keyboard-modifier.ts
//
// Platform-aware modifier normalization (KEY-08). Retail/shell shortcuts
// historically checked `ctrlKey` only, so macOS-like hardware (where the
// primary command modifier is Meta) behaved differently. Use this helper so a
// shortcut declared as "Ctrl+…" also matches Meta on such keyboards.

/**
 * True when the primary command modifier is held — Ctrl on Windows/Linux,
 * Meta (⌘) on macOS-like keyboards.
 */
export function isCommandModifier(e: KeyboardEvent): boolean {
  return e.ctrlKey || e.metaKey;
}
