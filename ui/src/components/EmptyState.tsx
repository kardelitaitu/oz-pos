import type { ReactNode } from 'react';
import { Button } from './Button';

/** Props for the EmptyState placeholder component. */
export interface EmptyStateProps {
  /** Optional icon/illustration displayed above the title. */
  icon?: ReactNode;
  /** Heading text. */
  title: string;
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

/**
 * Placeholder screen shown when a list or view has no data.
 * Renders an optional icon, title, description, action button,
 * and additional children.
 */
export function EmptyState({
  icon,
  title,
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
      <h3 className="empty-state__title">{title}</h3>
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
