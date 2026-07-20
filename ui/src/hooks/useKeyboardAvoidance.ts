import { useCallback, useEffect, useRef } from 'react';

interface KeyboardAvoidanceOptions {
  /** Selector string or ref object identifying the input elements to watch. */
  selector?: string;
  /** Padding (px) between the focused input and the virtual keyboard. Default 16. */
  scrollPadding?: number;
}

interface KeyboardAvoidanceResult {
  /** Ref to attach to the scrollable container element. */
  containerRef: React.RefObject<HTMLDivElement | null>;
}

/**
 * Detects virtual keyboard open/close on mobile devices and scrolls the
 * active input into view so it isn't hidden behind the keyboard.
 *
 * Uses the `visualViewport` API when available (mobile browsers) and
 * falls back to a `focusin` / `focusout` scroll-into-view strategy.
 */
export function useKeyboardAvoidance({
  selector = 'input, textarea, select, [contenteditable]',
  scrollPadding = 16,
}: KeyboardAvoidanceOptions = {}): KeyboardAvoidanceResult {
  const containerRef = useRef<HTMLDivElement | null>(null);
  const activeElRef = useRef<Element | null>(null);
  const originalScrollMargin = useRef('');

  const scrollActiveIntoView = useCallback(() => {
    const el = activeElRef.current;
    const container = containerRef.current;
    if (!el || !container) return;

    const elRect = el.getBoundingClientRect();
    const containerRect = container.getBoundingClientRect();
    const keyboardTop =
      typeof window.visualViewport !== 'undefined' && window.visualViewport !== null
        ? window.visualViewport.height
        : window.innerHeight;

    // Check if the element is below the visible area (i.e., behind the keyboard)
    const visibleBottom = Math.min(containerRect.bottom, keyboardTop);
    if (elRect.bottom > visibleBottom - scrollPadding) {
      const scrollNeeded = elRect.bottom - visibleBottom + scrollPadding;
      if (typeof container.scrollBy === 'function') {
        container.scrollBy({ top: scrollNeeded, behavior: 'smooth' });
      }
    }
  }, [scrollPadding]);

  useEffect(() => {
    const container = containerRef.current;
    if (!container) return;

    const handleFocusIn = (e: FocusEvent) => {
      const target = e.target as Element;
      if (target.matches(selector)) {
        activeElRef.current = target;
        originalScrollMargin.current =
          (target as HTMLElement).style.scrollMargin ?? '';
        (target as HTMLElement).style.scrollMargin = `${scrollPadding}px`;

        // Small delay to let the keyboard open and layout settle
        setTimeout(scrollActiveIntoView, 350);
      }
    };

    const handleFocusOut = () => {
      if (activeElRef.current) {
        (activeElRef.current as HTMLElement).style.scrollMargin =
          originalScrollMargin.current;
      }
      activeElRef.current = null;
    };

    // visualViewport resize fires when keyboard opens/closes on mobile
    const handleViewportResize = () => {
      if (activeElRef.current) {
        scrollActiveIntoView();
      }
    };

    document.addEventListener('focusin', handleFocusIn);
    document.addEventListener('focusout', handleFocusOut);

    if (window.visualViewport) {
      window.visualViewport.addEventListener('resize', handleViewportResize);
    }

    return () => {
      document.removeEventListener('focusin', handleFocusIn);
      document.removeEventListener('focusout', handleFocusOut);
      if (window.visualViewport) {
        window.visualViewport.removeEventListener('resize', handleViewportResize);
      }
    };
  }, [selector, scrollPadding, scrollActiveIntoView]);

  return { containerRef };
}
