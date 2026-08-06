import type { ReactNode } from 'react';
import { useLocalization } from '@fluent/react';
import { Button } from './Button';

/** Props for the ErrorState display component. */
export interface ErrorStateProps {
  /** Optional icon/illustration displayed above the title. */
  icon?: ReactNode;
  /** Heading text. */
  title: string;
  /** Heading level — use 1/2/3 matching the page hierarchy. Default 3 for backward compat. */
  headingLevel?: 1 | 2 | 3;
  /** Detailed error message. */
  message?: string;
  /** Called when the user clicks the retry button. */
  onRetry?: () => void;
  /** Label for the retry button. @default 'Retry' */
  retryLabel?: string;
  /** Additional content. */
  children?: ReactNode;
}

/**
 * Error state screen with an optional icon, title, detailed message,
 * retry button, and additional children.
 */
export function ErrorState({
  icon,
  title,
  headingLevel = 3,
  message,
  onRetry,
  retryLabel,
  children,
}: ErrorStateProps) {
  const { l10n } = useLocalization();
  return (
    <div className="error-state" role="alert">
      {icon && (
        <div className="error-state__icon" aria-hidden="true">
          {icon}
        </div>
      )}
      {headingLevel === 1 ? (
        <h1 className="error-state__title">{title}</h1>
      ) : headingLevel === 2 ? (
        <h2 className="error-state__title">{title}</h2>
      ) : (
        <h3 className="error-state__title">{title}</h3>
      )}
      {message && <p className="error-state__message">{message}</p>}
      {onRetry && (
        <div className="error-state__action">
          <Button variant="primary" onClick={onRetry}>
            {retryLabel ?? l10n.getString('error-state-retry')}
          </Button>
        </div>
      )}
      {children}
    </div>
  );
}
