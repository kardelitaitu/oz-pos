import type { HTMLAttributes, CSSProperties } from 'react';

/** Shape variant for the Skeleton placeholder. */
export type SkeletonVariant = 'text' | 'circle' | 'block';

/** Props for the Skeleton loading placeholder component. */
export interface SkeletonProps extends HTMLAttributes<HTMLDivElement> {
  /** Shape variant. @default 'text' */
  variant?: SkeletonVariant;
  /** Explicit width (e.g. '100%', '200px'). */
  width?: string;
  /** Explicit height (e.g. '1em', '200px'). */
  height?: string;
}

/**
 * Loading placeholder that renders a pulsing shape.
 * Set `aria-hidden="true"` so assistive technology ignores it.
 */
export function Skeleton({
  variant = 'text',
  width,
  height,
  className,
  style,
  ...rest
}: SkeletonProps) {
  const classNames = [
    'skeleton',
    `skeleton--${variant}`,
    className ?? '',
  ]
    .filter(Boolean)
    .join(' ');

  const resolvedStyle: CSSProperties = {
    ...style,
    ...(width !== undefined ? { width } : {}),
    ...(height !== undefined ? { height } : {}),
  };

  return (
    <div
      className={classNames}
      aria-hidden="true"
      style={resolvedStyle}
      {...rest}
    />
  );
}
