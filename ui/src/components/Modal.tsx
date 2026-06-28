import { useEffect, useRef, useCallback, useId, type ReactNode } from 'react';

// ── Types ──────────────────────────────────────────────────────────

export interface ModalProps {
  /** Whether the modal is visible. */
  open: boolean;
  /** Called when the modal should close (overlay click, Escape, close button). */
  onClose: () => void;
  /** Dialog title (rendered in the header). */
  title?: string;
  /** Dialog body content. */
  children: ReactNode;
  /** Optional footer content (typically action buttons). */
  footer?: ReactNode;
  /** Whether to show the close (X) button in the header. @default true */
  showCloseButton?: boolean;
}

// ── Component ─────────────────────────────────────────────────────

/**
 * Accessible modal dialog built with a React portal.
 *
 * Features:
 * - Focus trap: Tab/Shift+Tab cycle through focusable elements
 * - Escape key closes the modal
 * - Overlay click closes the modal
 * - ARIA `dialog` role with `aria-modal` and `aria-labelledby`
 * - Animated entrance (fade + slide)
 *
 * Examples:
 * ```tsx
 * const [open, setOpen] = useState(false);
 * return (
 *   <>
 *     <Button onClick={() => setOpen(true)}>Open</Button>
 *     <Modal open={open} onClose={() => setOpen(false)} title="Confirm">
 *       <p>Are you sure?</p>
 *     </Modal>
 *   </>
 * );
 * ```
 */
export function Modal({
  open,
  onClose,
  title,
  children,
  footer,
  showCloseButton = true,
}: ModalProps) {
  const panelRef = useRef<HTMLDivElement>(null);
  const titleId = useId();

  // ── Focus trap ──────────────────────────────────────────────
  const handleKeyDown = useCallback(
    (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        onClose();
        return;
      }
      if (e.key !== 'Tab' || !panelRef.current) return;

      const focusable = panelRef.current.querySelectorAll<HTMLElement>(
        'button, [href], input, select, textarea, [tabindex]:not([tabindex="-1"])',
      );
      if (focusable.length === 0) return;

      const first = focusable[0]!;
      const last = focusable[focusable.length - 1]!;

      if (e.shiftKey) {
        if (document.activeElement === first) {
          e.preventDefault();
          last.focus();
        }
      } else {
        if (document.activeElement === last) {
          e.preventDefault();
          first.focus();
        }
      }
    },
    [onClose],
  );

  // ── Side effects ─────────────────────────────────────────────
  useEffect(() => {
    if (!open) return;

    const panel = panelRef.current;
    if (!panel) return;

    // Focus the first focusable element inside the panel.
    const focusable = panel.querySelector<HTMLElement>(
      'button, [href], input, select, textarea, [tabindex]:not([tabindex="-1"])',
    );
    focusable?.focus();

    // Lock body scroll.
    const originalOverflow = document.body.style.overflow;
    document.body.style.overflow = 'hidden';

    // Listen for keyboard events.
    document.addEventListener('keydown', handleKeyDown);

    return () => {
      document.body.style.overflow = originalOverflow;
      document.removeEventListener('keydown', handleKeyDown);
    };
  }, [open, handleKeyDown]);

  if (!open) return null;

  return (
    <div
      className="modal-overlay"
      role="presentation"
      onClick={(e) => {
        // Only close when the overlay itself is clicked, not the panel.
        if (e.target === e.currentTarget) onClose();
      }}
    >
      <div
        ref={panelRef}
        className="modal-panel"
        role="dialog"
        aria-modal="true"
        aria-labelledby={title ? titleId : undefined}
      >
        {/* ── Header ────────────────────────────── */}
        <div className="modal-header">
          {title && (
            <h2 id={titleId} className="modal-title">
              {title}
            </h2>
          )}

          {showCloseButton && (
            <button
              type="button"
              className="modal-close-btn"
              onClick={onClose}
              aria-label="Close dialog"
            >
              {/* X icon */}
              <svg
                width="16"
                height="16"
                viewBox="0 0 24 24"
                fill="none"
                stroke="currentColor"
                strokeWidth="2"
                strokeLinecap="round"
                strokeLinejoin="round"
                aria-hidden="true"
              >
                <line x1="18" y1="6" x2="6" y2="18" />
                <line x1="6" y1="6" x2="18" y2="18" />
              </svg>
            </button>
          )}
        </div>

        {/* ── Body ──────────────────────────────── */}
        <div className="modal-body">{children}</div>

        {/* ── Footer ────────────────────────────── */}
        {footer && <div className="modal-footer">{footer}</div>}
      </div>
    </div>
  );
}
