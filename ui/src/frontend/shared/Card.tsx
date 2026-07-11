import type { ReactNode } from 'react';

export type CardShadow = 'none' | 'xs' | 'sm' | 'md' | 'lg';
export type CardPadding = 'none' | 'sm' | 'md' | 'lg';

export interface CardProps {
  children: ReactNode;
  header?: ReactNode;
  footer?: ReactNode;
  shadow?: CardShadow;
  padding?: CardPadding;
  className?: string;
}

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
