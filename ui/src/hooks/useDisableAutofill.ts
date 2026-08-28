//! Disables browser autofill on all input elements globally.
//!
//! The Tauri POS app is a desktop application — browser autofill
//! suggestions are confusing and a security concern on login/PIN screens.
//! This hook sets `autocomplete="off"` on every input in the DOM on mount
//! and uses a MutationObserver to catch dynamically added inputs.

import { useEffect } from 'react';

/** Attribute value that suppresses autofill in all browsers. */
const AUTOCOMPLETE_OFF = 'off';

/**
 * Disables browser autofill on all `<input>`, `<select>`, and `<textarea>`
 * elements. Runs once on mount and observes DOM mutations for dynamically
 * added elements.
 */
export function useDisableAutofill(): void {
  useEffect(() => {
    /** Set autocomplete="off" on a single element if not already set. */
    const disable = (el: Element) => {
      if (
        el instanceof HTMLInputElement ||
        el instanceof HTMLSelectElement ||
        el instanceof HTMLTextAreaElement
      ) {
        if (el.autocomplete !== AUTOCOMPLETE_OFF) {
          el.autocomplete = AUTOCOMPLETE_OFF;
        }
      }
    };

    /** Process all existing inputs in the DOM. */
    const disableAll = () => {
      document.querySelectorAll('input, select, textarea').forEach(disable);
    };

    // Disable on mount
    disableAll();

    // Observe DOM mutations to catch dynamically added inputs
    const observer = new MutationObserver((mutations) => {
      for (const mutation of mutations) {
        for (const node of mutation.addedNodes) {
          if (node instanceof HTMLElement) {
            disable(node);
            node.querySelectorAll?.('input, select, textarea').forEach(disable);
          }
        }
      }
    });

    observer.observe(document.body, { childList: true, subtree: true });

    return () => observer.disconnect();
  }, []);
}
