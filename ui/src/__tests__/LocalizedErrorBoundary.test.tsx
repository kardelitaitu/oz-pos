import type { ReactElement } from 'react';
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';
import { LocalizationProvider, ReactLocalization } from '@fluent/react';
import { FluentBundle, FluentResource } from '@fluent/bundle';
import { LocalizedErrorBoundary } from '@/components/LocalizedErrorBoundary';

// ── Static Fluent bundles for test isolation ─────────────────────
const enBundle = new FluentBundle('en-US');
enBundle.addResource(
  new FluentResource('error-boundary-title = Something went wrong\nerror-boundary-retry = Try Again\n'),
);
const enL10n = new ReactLocalization([enBundle]);

const idBundle = new FluentBundle('id');
idBundle.addResource(
  new FluentResource('error-boundary-title = Terjadi kesalahan\nerror-boundary-retry = Coba Lagi\n'),
);
const idL10n = new ReactLocalization([idBundle]);

function BrokenComponent(): ReactElement {
  throw new Error('Test error message');
}

function renderWithL10n(ui: React.ReactElement, l10n: ReactLocalization) {
  return render(<LocalizationProvider l10n={l10n}>{ui}</LocalizationProvider>);
}

describe('LocalizedErrorBoundary (ERR-02)', () => {
  const preventJsdomError = (e: ErrorEvent) => e.preventDefault();
  // jsdom's Location#reload is non-configurable (cannot be spied), so we
  // swap the whole window.location object for one with a spyable reload.
  const originalLocation = window.location;
  let reloadSpy: ReturnType<typeof vi.fn>;

  beforeEach(() => {
    reloadSpy = vi.fn();
    Object.defineProperty(window, 'location', {
      configurable: true,
      value: { ...originalLocation, reload: reloadSpy },
    });
    vi.spyOn(console, 'error').mockImplementation(() => {});
    window.addEventListener('error', preventJsdomError);
  });

  afterEach(() => {
    window.removeEventListener('error', preventJsdomError);
    Object.defineProperty(window, 'location', {
      configurable: true,
      value: originalLocation,
    });
    vi.restoreAllMocks();
  });

  it('renders children when there is no error', () => {
    renderWithL10n(
      <LocalizedErrorBoundary>
        <p>Normal content</p>
      </LocalizedErrorBoundary>,
      enL10n,
    );
    expect(screen.getByText('Normal content')).toBeInTheDocument();
  });

  it('injects the English fallback copy from the active locale', () => {
    renderWithL10n(
      <LocalizedErrorBoundary>
        <BrokenComponent />
      </LocalizedErrorBoundary>,
      enL10n,
    );
    expect(screen.getByRole('heading', { name: 'Something went wrong' })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Try Again' })).toBeInTheDocument();
  });

  it('injects the Indonesian fallback copy when the active locale is id', () => {
    renderWithL10n(
      <LocalizedErrorBoundary>
        <BrokenComponent />
      </LocalizedErrorBoundary>,
      idL10n,
    );
    expect(screen.getByRole('heading', { name: 'Terjadi kesalahan' })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Coba Lagi' })).toBeInTheDocument();
    // English emergency copy must not leak.
    expect(screen.queryByText('Something went wrong')).not.toBeInTheDocument();
  });

  it('fallback uses token-backed CSS classes with no inline styles', () => {
    renderWithL10n(
      <LocalizedErrorBoundary>
        <BrokenComponent />
      </LocalizedErrorBoundary>,
      enL10n,
    );
    const alert = screen.getByRole('alert');
    expect(alert.className).toBe('error-boundary');
    expect(alert.getAttribute('style')).toBeNull();
  });

  it('clicking the retry button without onReset triggers a hard reload', () => {
    renderWithL10n(
      <LocalizedErrorBoundary>
        <BrokenComponent />
      </LocalizedErrorBoundary>,
      enL10n,
    );
    expect(screen.getByRole('alert')).toBeInTheDocument();

    fireEvent.click(screen.getByRole('button', { name: 'Try Again' }));
    expect(reloadSpy).toHaveBeenCalledTimes(1);
  });

  it('forwards onReset when provided and skips the hard reload', () => {
    const onReset = vi.fn();
    renderWithL10n(
      <LocalizedErrorBoundary onReset={onReset}>
        <BrokenComponent />
      </LocalizedErrorBoundary>,
      enL10n,
    );
    fireEvent.click(screen.getByRole('button', { name: 'Try Again' }));
    expect(onReset).toHaveBeenCalledTimes(1);
    expect(reloadSpy).not.toHaveBeenCalled();
  });
});
