import { useState, useRef, useCallback, useId, type ReactNode, type ReactElement, cloneElement } from 'react';
import { createPortal } from 'react-dom';
import './Tooltip.css';

/** Props for the Tooltip component. */
export interface TooltipProps {
  /** Content to show inside the tooltip. */
  content: ReactNode;
  /** Position relative to the trigger element. */
  position?: 'right' | 'top' | 'bottom' | 'left';
  /** Delay in ms before showing the tooltip. Default 400ms. */
  showDelay?: number;
  /** Delay in ms before hiding the tooltip. Default 100ms. */
  hideDelay?: number;
  /** Maximum width of the tooltip bubble (CSS value). Default '280px'. */
  maxWidth?: string;
  /** Render the tooltip to document.body via a portal so it escapes
   *  overflow:hidden/scroll clipping. Use when the trigger is inside
   *  a scrollable or clipped container. */
  portal?: boolean;
  /** The element that triggers the tooltip on hover/focus. */
  children: ReactElement;
}

/**
 * A polished tooltip component that appears on hover and focus of its trigger.
 *
 * - Appears after a 400ms delay (configurable) so it doesn't flash during mouse passes
 * - Stays visible for 100ms after the cursor leaves (configurable)
 * - Supports `position` prop (right, top, bottom, left)
 * - Handles keyboard focus via `:focus-visible` on the trigger
 * - Includes a small arrow pointing toward the trigger
 * - Uses `role="tooltip"` for accessibility
 * - Optional `portal` mode renders to document.body via createPortal with
 *   position:fixed, escaping parent overflow clipping
 */
export default function Tooltip({
  content,
  position = 'right',
  showDelay = 400,
  hideDelay = 100,
  maxWidth = '280px',
  portal = false,
  children,
}: TooltipProps) {
  const [visible, setVisible] = useState(false);
  const uid = useId();
  const tooltipId = `tooltip-${uid}`;
  const showTimer = useRef<ReturnType<typeof setTimeout>>();
  const hideTimer = useRef<ReturnType<typeof setTimeout>>();
  const triggerRef = useRef<HTMLElement>(null);
  const tooltipRef = useRef<HTMLDivElement>(null);
  const [triggerRect, setTriggerRect] = useState<DOMRect | null>(null);

  const startShow = useCallback(() => {
    // Capture trigger position before showing (portal needs viewport coords)
    if (portal && triggerRef.current) {
      setTriggerRect(triggerRef.current.getBoundingClientRect());
    }
    clearTimeout(hideTimer.current);
    showTimer.current = setTimeout(() => setVisible(true), showDelay);
  }, [showDelay, portal]);

  const startHide = useCallback(() => {
    clearTimeout(showTimer.current);
    hideTimer.current = setTimeout(() => setVisible(false), hideDelay);
  }, [hideDelay]);

  const handleBlur = useCallback(
    (e: React.FocusEvent) => {
      // If focus moves to the tooltip itself, don't hide
      if (tooltipRef.current && tooltipRef.current.contains(e.relatedTarget as Node)) return;
      startHide();
    },
    [startHide],
  );

  // Portal: pass viewport coordinates as CSS custom properties for
  // position:fixed layout. Fall back gracefully when rect is null.
  const portalStyle: React.CSSProperties | undefined =
    portal && triggerRect
      ? ({
          '--trigger-top': `${triggerRect.top}px`,
          '--trigger-left': `${triggerRect.left}px`,
          '--trigger-width': `${triggerRect.width}px`,
          '--trigger-height': `${triggerRect.height}px`,
          '--trigger-bottom': `${triggerRect.bottom}px`,
          '--trigger-right': `${triggerRect.right}px`,
          maxWidth,
        } as React.CSSProperties)
      : undefined;

  const tooltipNode = (
    <div
      ref={tooltipRef}
      id={tooltipId}
      className={`tooltip-content tooltip-content--${position}${visible ? ' tooltip-content--visible' : ''}${portal ? ' tooltip-content--portal' : ''}`}
      style={portal ? portalStyle : (maxWidth ? { maxWidth } : undefined)}
      role="tooltip"
      onMouseEnter={() => {
        clearTimeout(hideTimer.current);
        clearTimeout(showTimer.current);
        setVisible(true);
      }}
      onMouseLeave={startHide}
      aria-hidden={!visible}
    >
      {content}
    </div>
  );

  return (
    <div
      className="tooltip-wrapper"
      onMouseEnter={startShow}
      onMouseLeave={startHide}
      onFocus={startShow}
      onBlur={handleBlur}
    >
      {cloneElement(children, {
        ref: triggerRef,
        'aria-describedby': visible ? tooltipId : undefined,
      } as Record<string, unknown>)}
      {portal && typeof document !== 'undefined'
        ? createPortal(tooltipNode, document.body)
        : tooltipNode}
    </div>
  );
}
