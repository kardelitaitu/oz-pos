import type { ReactNode } from 'react';
import { useLocalization } from '@fluent/react';
import { Button } from '@/components/Button';

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
      <h3 className="error-state__title">{title}</h3>
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
