import { describe, it, expect, vi } from 'vitest';
import { render, screen } from '@testing-library/react';
import { FluentBundle, FluentResource } from '@fluent/bundle';
import { ReactLocalization, LocalizationProvider } from '@fluent/react';
import { ErrorState } from '@/components/ErrorState';

const ftl = `
error-state-retry = Retry
`;

const bundle = new FluentBundle('en');
bundle.addResource(new FluentResource(ftl));
const l10n = new ReactLocalization([bundle]);

vi.mock('@/components/Button', () => ({
  Button: ({ children, onClick, variant }: Record<string, unknown>) => (
    <button onClick={onClick as () => void} className={`btn btn--${variant as string || 'primary'}`}>
      {children as React.ReactNode}
    </button>
  ),
}));

function renderErr(props: Parameters<typeof ErrorState>[0]) {
  return render(
    <LocalizationProvider l10n={l10n}>
      <ErrorState {...props} />
    </LocalizationProvider>,
  );
}

describe('ErrorState', () => {
  it('renders the title as an h3', () => {
    renderErr({ title: 'Something went wrong' });
    expect(screen.getByRole('heading', { level: 3 }).textContent).toBe('Something went wrong');
  });

  it('renders a message when provided', () => {
    renderErr({ title: 'Error', message: 'The server returned a 500 error.' });
    expect(screen.getByText('The server returned a 500 error.')).toBeTruthy();
  });

  it('does not render a message element when absent', () => {
    renderErr({ title: 'Error' });
    expect(document.querySelector('.error-state__message')).toBeNull();
  });

  it('renders the default Retry button when onRetry is provided', () => {
    const onRetry = vi.fn();
    renderErr({ title: 'Error', onRetry });
    expect(screen.getByText('Retry')).toBeTruthy();
  });

  it('renders a custom retry label when provided', () => {
    const onRetry = vi.fn();
    renderErr({ title: 'Error', onRetry, retryLabel: 'Try Again' });
    expect(screen.getByText('Try Again')).toBeTruthy();
  });

  it('does not render a retry button when onRetry is absent', () => {
    renderErr({ title: 'Error' });
    expect(document.querySelector('.error-state__action')).toBeNull();
  });

  it('renders an icon with aria-hidden', () => {
    renderErr({ title: 'Error', icon: <span data-testid="err-icon">⚠️</span> });
    expect(screen.getByTestId('err-icon')).toBeTruthy();
    expect(document.querySelector('.error-state__icon')!.getAttribute('aria-hidden')).toBe('true');
  });

  it('renders additional children', () => {
    renderErr({ title: 'Error', children: <em data-testid="extra">Contact support</em> });
    expect(screen.getByTestId('extra').textContent).toBe('Contact support');
  });

  it('has role="alert" on the container', () => {
    renderErr({ title: 'Error' });
    expect(screen.getByRole('alert')).toBeTruthy();
  });
});
