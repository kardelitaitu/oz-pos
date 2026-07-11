import { type HTMLAttributes } from 'react';
import { useLocalization } from '@fluent/react';

/** Size preset for the Spinner component. */
export type SpinnerSize = 'sm' | 'md' | 'lg';

/** Props for the Spinner loading indicator component. */
export interface SpinnerProps extends HTMLAttributes<HTMLSpanElement> {
  /** Size preset. @default 'md' */
  size?: SpinnerSize;
  /** Optional accessible label. Defaults to "Loading". */
  label?: string;
}

/**
 * Accessible loading spinner with a configurable size and label.
 * Uses `role="status"` and an `aria-label` for screen readers.
 */
export function Spinner({
  size = 'md',
  label,
  className,
  ...rest
}: SpinnerProps) {
  const { l10n } = useLocalization();
  const classNames = [
    'spinner',
    `spinner--${size}`,
    className ?? '',
  ]
    .filter(Boolean)
    .join(' ');

  return (
    <span
      role="status"
      className={classNames}
      aria-label={label ?? l10n.getString('spinner-label')}
      {...rest}
    >
      {label && <span className="spinner__label">{label}</span>}
    </span>
  );
}
