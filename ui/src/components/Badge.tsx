import type { HTMLAttributes, ReactNode } from 'react';

/** Visual variant for the Badge component. */
export type BadgeVariant = 'default' | 'success' | 'warning' | 'danger' | 'info';
/** Size preset for the Badge component. */
export type BadgeSize = 'sm' | 'md';

/** Props for the Badge component. */
export interface BadgeProps extends HTMLAttributes<HTMLSpanElement> {
  /** Visual variant. @default 'default' */
  variant?: BadgeVariant;
  /** Size preset. @default 'md' */
  size?: BadgeSize;
  /** Badge content. */
  children: ReactNode;
}

/**
 * Inline badge / pill for status labelling.
 * Renders a `<span>` with variant and size CSS classes.
 */
export function Badge({
  variant = 'default',
  size = 'md',
  children,
  className,
  ...rest
}: BadgeProps) {
  const classNames = [
    'badge',
    `badge--${variant}`,
    `badge--${size}`,
    className ?? '',
  ]
    .filter(Boolean)
    .join(' ');

  return (
    <span className={classNames} {...rest}>
      {children}
    </span>
  );
}
