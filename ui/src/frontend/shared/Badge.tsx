import type { HTMLAttributes, ReactNode } from 'react';

export type BadgeVariant = 'default' | 'success' | 'warning' | 'danger' | 'info';
export type BadgeSize = 'sm' | 'md';

export interface BadgeProps extends HTMLAttributes<HTMLSpanElement> {
  /** Visual variant. @default 'default' */
  variant?: BadgeVariant;
  /** Size preset. @default 'md' */
  size?: BadgeSize;
  /** Badge content. */
  children: ReactNode;
}

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
