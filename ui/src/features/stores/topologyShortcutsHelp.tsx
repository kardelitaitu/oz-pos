//! Shortcuts help popover (header "?" button) for the topology editor.
//!
//! Renders the F1 "?" button and its keyboard-shortcuts list. The open/close
//! state stays owned by the parent (the editor's central keydown handler
//! toggles it on F1), while this component owns the popover's Escape and
//! outside-click dismissal. Escape is `stopPropagation`'d so the canvas's own
//! Escape (deselect) does not also fire while the popover is open.

import { useEffect, useRef } from 'react';
import { useLocalization } from '@fluent/react';

/** Keyboard shortcuts listed in the header's help popover. `key` is the
 *  literal kbd text; `id` is the FTL description key (reuses existing
 *  topology strings where they already name the action). */
const TOPOLOGY_SHORTCUTS: { id: string; key: string }[] = [
  { id: 'topology-shortcuts-help', key: 'F1' },
  { id: 'topology-shortcuts-pan', key: 'Space + Drag' },
  { id: 'topology-shortcuts-duplicate-drag', key: 'Alt + Drag' },
  { id: 'topology-shortcuts-additive-marquee', key: 'Shift + Drag' },
  { id: 'topology-shortcuts-spawn', key: '1–4' },
  { id: 'topology-shortcuts-select-all', key: 'Ctrl+A' },
  { id: 'topology-shortcuts-duplicate', key: 'Ctrl+D' },
  { id: 'topology-shortcuts-copy', key: 'Ctrl+C' },
  { id: 'topology-shortcuts-paste', key: 'Ctrl+V' },
  { id: 'topology-shortcuts-rename', key: 'F2' },
  { id: 'topology-shortcuts-zoom-fit-100', key: 'Ctrl+0 / Ctrl+1' },
  { id: 'topology-shortcuts-zoom-step', key: 'Ctrl++ / Ctrl+-' },
  { id: 'topology-delete-selected', key: 'Del' },
  { id: 'topology-undo', key: 'Ctrl+Z' },
  { id: 'topology-redo', key: 'Ctrl+Y' },
  { id: 'topology-shortcuts-nudge', key: '← ↑ ↓ →' },
  { id: 'topology-shortcuts-esc', key: 'Esc' },
  { id: 'topology-shortcuts-inspector', key: 'Ctrl+I' },
  { id: 'topology-shortcuts-find', key: 'Ctrl+F' },
];

export interface TopologyShortcutsHelpProps {
  /** Whether the popover is visible (owned by the editor's F1 handler). */
  open: boolean;
  /** Toggle the popover (the "?" button's click action). */
  onToggle: () => void;
  /** Close the popover (Escape / outside click). */
  onClose: () => void;
}

/**
 * The header's help "?" button plus its keyboard-shortcuts popover.
 * Self-contains its outside-click and Escape dismissal (Escape is
 * `stopPropagation`'d so the canvas's own Escape handler is not also run).
 */
export function TopologyShortcutsHelp({ open, onToggle, onClose }: TopologyShortcutsHelpProps) {
  const { l10n } = useLocalization();
  const btnRef = useRef<HTMLButtonElement | null>(null);
  const popoverRef = useRef<HTMLDivElement | null>(null);

  useEffect(() => {
    if (!open) return;
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        e.stopPropagation();
        onClose();
      }
    };
    const handleClickOutside = (e: MouseEvent) => {
      if (
        popoverRef.current && !popoverRef.current.contains(e.target as Node) &&
        btnRef.current && !btnRef.current.contains(e.target as Node)
      ) {
        onClose();
      }
    };
    document.addEventListener('keydown', handleKeyDown);
    document.addEventListener('mousedown', handleClickOutside);
    return () => {
      document.removeEventListener('keydown', handleKeyDown);
      document.removeEventListener('mousedown', handleClickOutside);
    };
  }, [open, onClose]);

  return (
    <>
      <button
        ref={btnRef}
        type="button"
        className="topology-shortcuts-btn"
        onClick={onToggle}
        aria-label={l10n.getString('topology-shortcuts-aria')}
        aria-expanded={open}
        aria-controls="topology-shortcuts-popover"
      >
        <svg viewBox="0 0 20 20" fill="currentColor" width="16" height="16" aria-hidden="true">
          <path fillRule="evenodd" d="M18 10a8 8 0 11-16 0 8 8 0 0116 0zm-8-3a1 1 0 00-.867.5 1 1 0 11-1.731-1A3 3 0 0113 8a3.001 3.001 0 01-2 2.83V11a1 1 0 11-2 0v-1a1 1 0 011-1 1 1 0 100-2zm0 8a1 1 0 100-2 1 1 0 000 2z" clipRule="evenodd" />
        </svg>
      </button>
      {open && (
        <div
          id="topology-shortcuts-popover"
          ref={popoverRef}
          className="topology-shortcuts-popover"
          role="region"
          aria-label={l10n.getString('topology-shortcuts-title')}
        >
          <div className="topology-shortcuts-title">{l10n.getString('topology-shortcuts-title')}</div>
          {TOPOLOGY_SHORTCUTS.map((s) => (
            <div key={s.id} className="topology-shortcuts-row">
              <span className="topology-shortcuts-desc">{l10n.getString(s.id)}</span>
              <kbd className="topology-shortcuts-key">{s.key}</kbd>
            </div>
          ))}
        </div>
      )}
    </>
  );
}
