import type { ReactNode } from 'react';
import { Button } from './Button';

export interface ErrorStateProps {
  /** Optional icon/illustration displayed above the title. */
  icon?: ReactNode;
  /** Heading text. */
  title: string;
  /** Detailed error message. */
  message?: string;
  /** Called when the user clicks the retry button. */
  onRetry?: () => void;
  /** Label for the retry button. @default 'Retry' */
  retryLabel?: string;
  /** Additional content. */
  children?: ReactNode;
}

export function ErrorState({
  icon,
  title,
  message,
  onRetry,
  retryLabel = 'Retry',
  children,
}: ErrorStateProps) {
  return (
    <div className="error-state" role="alert">
      {icon && (
        <div className="error-state__icon" aria-hidden="true">
          {icon}
        </div>
      )}
      <h3 className="error-state__title">{title}</h3>
      {message && <p className="error-state__message">{message}</p>}
      {onRetry && (
        <div className="error-state__action">
          <Button variant="primary" onClick={onRetry}>
            {retryLabel}
          </Button>
        </div>
      )}
      {children}
    </div>
  );
}
