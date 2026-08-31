import { useEffect, useRef, useState } from 'react';

/**
 * Command palette state and keyboard bindings for the analytics dashboard
 * — extracted from `AnalyticsScreen.tsx` (Phase 3 split). Owns the
 * open/closed state, the query text, the selection index, the Ctrl+K
 * toggle, the in-palette keyboard navigation, and the input focus.
 *
 * The caller feeds the current `filteredItems` and `runItem` action into
 * refs each render (the same pattern the original used for `runPaletteRef`),
 * so the keydown listener stays mounted once and always reads fresh values.
 */
export function useCommandPalette<T>() {
  const [paletteOpen, setPaletteOpen] = useState(false);
  const [paletteQuery, setPaletteQuery] = useState('');
  const [paletteIndex, setPaletteIndex] = useState(0);
  const paletteInputRef = useRef<HTMLInputElement | null>(null);

  /** Caller sets each render: the currently filtered item list. */
  const filteredItemsRef = useRef<T[]>([]);
  /** Caller sets each render: the run action for a palette item. */
  const runItemRef = useRef<(item: T) => void>(() => {});

  // Ctrl/Cmd+K toggles the palette
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if ((e.ctrlKey || e.metaKey) && (e.key === 'k' || e.key === 'K')) {
        e.preventDefault();
        setPaletteQuery('');
        setPaletteIndex(0);
        setPaletteOpen((o) => !o);
      }
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, []);

  // Keyboard navigation inside the open palette
  useEffect(() => {
    if (!paletteOpen) return;
    const onPaletteKey = (e: KeyboardEvent) => {
      if (e.key === 'ArrowDown') {
        e.preventDefault();
        setPaletteIndex((i) => Math.min(i + 1, filteredItemsRef.current.length - 1));
      } else if (e.key === 'ArrowUp') {
        e.preventDefault();
        setPaletteIndex((i) => Math.max(i - 1, 0));
      } else if (e.key === 'Enter') {
        e.preventDefault();
        const item = filteredItemsRef.current[paletteIndexRef.current];
        if (item) runItemRef.current(item);
      } else if (e.key === 'Escape') {
        setPaletteOpen(false);
        setPaletteQuery('');
      }
    };
    window.addEventListener('keydown', onPaletteKey);
    return () => window.removeEventListener('keydown', onPaletteKey);
  }, [paletteOpen]);

  // Keep the palette index readable from the keydown handler without
  // re-registering the listener on every selection change.
  const paletteIndexRef = useRef(paletteIndex);
  paletteIndexRef.current = paletteIndex;

  // Focus the search input when the palette opens
  useEffect(() => {
    if (paletteOpen) paletteInputRef.current?.focus();
  }, [paletteOpen]);

  // Keep the selection at the top when the query or palette changes
  useEffect(() => {
    setPaletteIndex(0);
  }, [paletteQuery, paletteOpen]);

  return {
    paletteOpen,
    paletteQuery,
    paletteIndex,
    paletteInputRef,
    setPaletteOpen,
    setPaletteQuery,
    setPaletteIndex,
    filteredItemsRef,
    runItemRef,
  };
}