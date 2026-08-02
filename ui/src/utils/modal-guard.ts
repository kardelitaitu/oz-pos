// ui/src/utils/modal-guard.ts
//
// Shared modal-ownership helper (KEY-04). AppShell and RetailPosScreen each
// ran their own `document.querySelector('[aria-modal="true"]')` on every key
// event, so ownership depended on DOM timing and on every overlay declaring
// aria-modal correctly. This centralizes the check so all shortcut surfaces
// agree on what "a modal is open" means, and provides the consume helper that
// prevents multiple listeners reacting to the same key (KEY-05).

/** True when any element with aria-modal="true" is present in the document. */
export function isAnyAriaModalOpen(): boolean {
  return document.querySelector('[aria-modal="true"]') !== null;
}

/**
 * Consume a shortcut event once a single winner has handled it, so other
 * document-level listeners (e.g. the global workspace Escape handler) do not
 * also react to the same key.
 */
export function consumeShortcut(e: KeyboardEvent): void {
  if (e.cancelable) e.preventDefault();
  e.stopPropagation();
}
