import type { ReactNode } from 'react';
import { Button } from '@/components/Button';

export interface EmptyStateProps {
  /** Optional icon/illustration displayed above the title. */
  icon?: ReactNode;
  /** Heading text. */
  title: string;
  /** Heading level — use 1/2/3 matching the page hierarchy. Default 3 for backward compat. */
  headingLevel?: 1 | 2 | 3;
  /** Supporting description. */
  description?: string;
  /** Optional primary action button. */
  action?: {
    label: string;
    onClick: () => void;
  };
  /** Additional content (e.g. custom CTA, tips). */
  children?: ReactNode;
}

export function EmptyState({
  icon,
  title,
  headingLevel = 3,
  description,
  action,
  children,
}: EmptyStateProps) {
  return (
    <div className="empty-state" role="status">
      {icon && (
        <div className="empty-state__icon" aria-hidden="true">
          {icon}
        </div>
      )}
      {headingLevel === 1 ? (
        <h1 className="empty-state__title">{title}</h1>
      ) : headingLevel === 2 ? (
        <h2 className="empty-state__title">{title}</h2>
      ) : (
        <h3 className="empty-state__title">{title}</h3>
      )}
      {description && <p className="empty-state__desc">{description}</p>}
      {action && (
        <div className="empty-state__action">
          <Button variant="primary" onClick={action.onClick}>
            {action.label}
          </Button>
        </div>
      )}
      {children}
    </div>
  );
}
