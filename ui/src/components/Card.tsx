import type { ReactNode } from 'react';

// ── Types ──────────────────────────────────────────────────────────

/** Shadow elevation preset for the Card component. */
export type CardShadow = 'none' | 'xs' | 'sm' | 'md' | 'lg';
/** Inner padding preset for the Card component. */
export type CardPadding = 'none' | 'sm' | 'md' | 'lg';

/** Props for the Card container component. */
export interface CardProps {
  /** Card body content. */
  children: ReactNode;
  /** Optional header area. */
  header?: ReactNode;
  /** Optional footer area (typically action buttons). */
  footer?: ReactNode;
  /** Shadow elevation. @default 'none' */
  shadow?: CardShadow;
  /** Inner padding. @default 'md' */
  padding?: CardPadding;
  /** Additional CSS class. */
  className?: string;
}

// ── Component ─────────────────────────────────────────────────────

/**
 * A container card with optional header/footer regions.
 *
 * Examples:
 * ```tsx
 * <Card shadow="sm">Simple card</Card>
 * <Card header={<h2>Title</h2>} footer={<Button>Save</Button>}>
 *   Body content
 * </Card>
 * ```
 */
export function Card({
  children,
  header,
  footer,
  shadow = 'none',
  padding = 'md',
  className,
}: CardProps) {
  const classNames = [
    'card',
    padding !== 'none' ? `card--padding-${padding}` : '',
    shadow !== 'none' ? `card--shadow-${shadow}` : '',
    className ?? '',
  ]
    .filter(Boolean)
    .join(' ');

  return (
    <div className={classNames}>
      {header && <div className="card-header">{header}</div>}
      <div className="card-body">{children}</div>
      {footer && <div className="card-footer">{footer}</div>}
    </div>
  );
}
