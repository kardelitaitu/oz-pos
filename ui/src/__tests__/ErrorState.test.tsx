import { describe, it, expect, vi } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';
import { LocalizationProvider, ReactLocalization } from '@fluent/react';
import { FluentBundle, FluentResource } from '@fluent/bundle';
import { ErrorState } from '@/components/ErrorState';

// ── Static Fluent bundle for test isolation ──────────────────────
const bundle = new FluentBundle('en-US');
bundle.addResource(new FluentResource('error-state-retry = Retry\n'));
const testL10n = new ReactLocalization([bundle]);

function renderWithL10n(ui: React.ReactElement) {
  return render(
    <LocalizationProvider l10n={testL10n}>
      {ui}
    </LocalizationProvider>,
  );
}

describe('ErrorState', () => {
  it('renders title and message', () => {
    renderWithL10n(
      <ErrorState
        title="Something went wrong"
        message="The server returned a 500 error. Please try again."
      />,
    );
    expect(screen.getByText('Something went wrong')).toBeInTheDocument();
    expect(
      screen.getByText('The server returned a 500 error. Please try again.'),
    ).toBeInTheDocument();
  });

  it('renders an optional icon', () => {
    renderWithL10n(
      <ErrorState
        title="No results"
        icon={<span data-testid="custom-icon">&#x26A0;</span>}
      />,
    );
    expect(screen.getByTestId('custom-icon')).toBeInTheDocument();
  });

  it('has role="alert" for screen reader announcements', () => {
    renderWithL10n(<ErrorState title="Error occurred" />);
    expect(screen.getByRole('alert')).toBeInTheDocument();
  });

  it('renders a retry button when onRetry is provided', () => {
    const onRetry = vi.fn();
    renderWithL10n(<ErrorState title="Load failed" onRetry={onRetry} />);
    const btn = screen.getByRole('button', { name: 'Retry' });
    expect(btn).toBeInTheDocument();
  });

  it('calls onRetry when the retry button is clicked', () => {
    const onRetry = vi.fn();
    renderWithL10n(<ErrorState title="Load failed" onRetry={onRetry} />);
    fireEvent.click(screen.getByRole('button', { name: 'Retry' }));
    expect(onRetry).toHaveBeenCalledTimes(1);
  });

  it('uses custom retryLabel when provided', () => {
    const onRetry = vi.fn();
    renderWithL10n(
      <ErrorState
        title="Network error"
        onRetry={onRetry}
        retryLabel="Reconnect"
      />,
    );
    expect(screen.getByRole('button', { name: 'Reconnect' })).toBeInTheDocument();
  });

  it('does not render a retry button when onRetry is undefined', () => {
    renderWithL10n(<ErrorState title="Fatal error" />);
    expect(screen.queryByRole('button')).not.toBeInTheDocument();
  });

  it('renders additional children content', () => {
    renderWithL10n(
      <ErrorState title="No data">
        <p data-testid="extra-info">Check your connection settings.</p>
      </ErrorState>,
    );
    expect(screen.getByTestId('extra-info')).toBeInTheDocument();
  });
});
