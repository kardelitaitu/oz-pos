import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { FluentBundle, FluentResource } from '@fluent/bundle';
import { ReactLocalization, LocalizationProvider } from '@fluent/react';
import UpdateBanner from '@/components/UpdateBanner';
import sharedFtl from '@/locales/shared.ftl?raw';

// ── Mock the Tauri updater plugin ──────────────────────────────────

const mockDownloadAndInstall = vi.fn();
const mockCheck = vi.fn();

vi.mock('@tauri-apps/plugin-updater', () => ({
  check: () => mockCheck(),
}));

// ── Fluent setup ────────────────────────────────────────────────────

const bundle = new FluentBundle('en-US');
bundle.addResource(new FluentResource(sharedFtl));
const l10n = new ReactLocalization([bundle]);

function renderComponent() {
  return render(
    <LocalizationProvider l10n={l10n}>
      <UpdateBanner />
    </LocalizationProvider>,
  );
}

// ── Helpers ─────────────────────────────────────────────────────────

/** Simulate an available update. */
function mockUpdateAvailable(overrides: {
  version?: string;
  notes?: string;
} = {}): void {
  mockCheck.mockResolvedValue({
    version: overrides.version ?? '0.2.0',
    body: overrides.notes ?? null,
    downloadAndInstall: mockDownloadAndInstall,
  });
}

// ── Tests ───────────────────────────────────────────────────────────

describe('UpdateBanner', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockDownloadAndInstall.mockResolvedValue(undefined);
    // Default: no update available
    mockCheck.mockResolvedValue(null);
  });

  // ── Null states ────────────────────────────────────────────────

  it('returns null when no update is available', async () => {
    const { container } = renderComponent();
    // Wait for the dynamic import + check to resolve
    await waitFor(() => {
      expect(mockCheck).toHaveBeenCalled();
    });
    expect(container.textContent).toBe('');
  });

  it('returns null after dismiss button is clicked', async () => {
    mockUpdateAvailable();
    renderComponent();

    await waitFor(() => expect(screen.getByRole('alert')).toBeDefined());

    // Find and click the dismiss button (the × icon button)
    const dismissBtn = document.querySelector('.update-banner-btn--dismiss') as HTMLElement;
    expect(dismissBtn).toBeTruthy();
    await userEvent.click(dismissBtn);

    await waitFor(() => {
      expect(screen.queryByRole('alert')).toBeNull();
    });
  });

  // ── Rendering ──────────────────────────────────────────────────

  it('shows banner with version when update is available', async () => {
    mockUpdateAvailable({ version: '0.3.0' });
    renderComponent();

    await waitFor(() => {
      expect(screen.getByText('v0.3.0')).toBeDefined();
    });
  });

  it('shows "Update available" title', async () => {
    mockUpdateAvailable();
    renderComponent();

    await waitFor(() => {
      expect(screen.getByText('Update available')).toBeDefined();
    });
  });

  it('shows "New version" text when no version string', async () => {
    mockUpdateAvailable({ version: '' });
    renderComponent();

    await waitFor(() => {
      expect(screen.getByText('New version')).toBeDefined();
    });
  });

  it('shows release notes when provided', async () => {
    mockUpdateAvailable({ notes: 'Bug fixes and performance improvements.' });
    renderComponent();

    await waitFor(() => {
      expect(screen.getByText(/Bug fixes and performance improvements/)).toBeDefined();
    });
  });

  it('does not show notes when not provided', async () => {
    mockUpdateAvailable({});
    renderComponent();

    await waitFor(() => expect(screen.getByRole('alert')).toBeDefined());

    const notesEl = document.querySelector('.update-banner-notes');
    expect(notesEl).toBeNull();
  });

  // ── Install action ─────────────────────────────────────────────

  it('has an Install button that calls downloadAndInstall', async () => {
    mockUpdateAvailable();
    renderComponent();

    await waitFor(() => expect(screen.getByText('Install')).toBeDefined());

    await userEvent.click(screen.getByText('Install'));

    await waitFor(() => {
      expect(mockDownloadAndInstall).toHaveBeenCalled();
    });
  });

  it('shows "Installing…" text while installing', async () => {
    // Never resolves — stays in installing state
    mockDownloadAndInstall.mockReturnValue(new Promise(() => {}));
    mockUpdateAvailable();
    renderComponent();

    await waitFor(() => expect(screen.getByText('Install')).toBeDefined());

    await userEvent.click(screen.getByText('Install'));

    await waitFor(() => {
      expect(screen.getByText('Installing…')).toBeDefined();
    });
  });

  it('disables Install button while installing', async () => {
    mockDownloadAndInstall.mockReturnValue(new Promise(() => {}));
    mockUpdateAvailable();
    renderComponent();

    await waitFor(() => expect(screen.getByText('Install')).toBeDefined());

    await userEvent.click(screen.getByText('Install'));

    await waitFor(() => {
      const btn = screen.getByText('Installing…').closest('button')!;
      expect(btn.disabled).toBe(true);
    });
  });

  // ── ARIA ───────────────────────────────────────────────────────

  it('has role="alert" and aria-live="polite"', async () => {
    mockUpdateAvailable();
    renderComponent();

    await waitFor(() => {
      const banner = screen.getByRole('alert');
      expect(banner.getAttribute('aria-live')).toBe('polite');
    });
  });

  it('dismiss button has aria-label', async () => {
    mockUpdateAvailable();
    renderComponent();

    await waitFor(() => {
      const dismissBtn = document.querySelector('.update-banner-btn--dismiss') as HTMLElement;
      expect(dismissBtn.getAttribute('aria-label')).toBe('Dismiss update notification');
    });
  });
});
