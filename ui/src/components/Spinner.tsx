import { type HTMLAttributes } from 'react';
import { useLocalization } from '@fluent/react';

export type SpinnerSize = 'sm' | 'md' | 'lg';

export interface SpinnerProps extends HTMLAttributes<HTMLSpanElement> {
  /** Size preset. @default 'md' */
  size?: SpinnerSize;
  /** Optional accessible label. Defaults to "Loading". */
  label?: string;
}

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
