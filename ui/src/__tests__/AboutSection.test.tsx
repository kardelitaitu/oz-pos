import { describe, it, expect, vi } from 'vitest';
import { render, screen } from '@testing-library/react';
import AboutSection from '@/features/settings/sections/AboutSection';

// ── Mocks ──────────────────────────────────────────────────────────────

vi.mock('@fluent/react', () => ({
  Localized: ({ children }: { id: string; children: React.ReactNode }) => <>{children}</>,
}));

// ── Helpers ────────────────────────────────────────────────────────────

function renderAbout(overrides: Partial<React.ComponentProps<typeof AboutSection>> = {}) {
  const defaults = {
    appVersion: '0.0.28',
    updateState: 'idle' as const,
    updateVersion: '',
    handleCheckUpdates: vi.fn(),
    handleInstallUpdate: vi.fn(),
  };
  return render(<AboutSection {...defaults} {...overrides} />);
}

// ── Tests ──────────────────────────────────────────────────────────────

describe('AboutSection', () => {
  it('renders app version', () => {
    renderAbout({ appVersion: '1.2.3' });
    expect(screen.getByText('1.2.3')).toBeInTheDocument();
  });

  it('renders system & license header', () => {
    renderAbout();
    expect(screen.getByText('System & License Ownership')).toBeInTheDocument();
  });

  it('renders updates section header', () => {
    renderAbout();
    expect(screen.getByText('Updates')).toBeInTheDocument();
  });

  describe('update state rendering', () => {
    it('shows "Not checked" when idle', () => {
      renderAbout({ updateState: 'idle' });
      expect(screen.getByText('Not checked')).toBeInTheDocument();
    });

    it('shows "Checking…" when checking', () => {
      renderAbout({ updateState: 'checking' });
      expect(screen.getByText('Checking…')).toBeInTheDocument();
    });

    it('shows "Up to date" when up-to-date', () => {
      renderAbout({ updateState: 'up-to-date' });
      expect(screen.getByText('Up to date')).toBeInTheDocument();
    });

    it('shows available version when update is available', () => {
      renderAbout({ updateState: 'available', updateVersion: '1.0.0' });
      expect(screen.getByText('1.0.0 available')).toBeInTheDocument();
    });

    it('shows "Check failed" when error', () => {
      renderAbout({ updateState: 'error' });
      expect(screen.getByText('Check failed')).toBeInTheDocument();
    });
  });

  describe('buttons', () => {
    it('shows "Check for Updates" button when idle', () => {
      renderAbout({ updateState: 'idle' });
      expect(screen.getByText('Check for Updates')).toBeInTheDocument();
    });

    it('shows "Retry" button when error', () => {
      renderAbout({ updateState: 'error' });
      expect(screen.getByText('Retry')).toBeInTheDocument();
    });

    it('shows "Install Now" button when update available', () => {
      renderAbout({ updateState: 'available' });
      expect(screen.getByText('Install Now')).toBeInTheDocument();
    });

    it('disables check button when checking', () => {
      renderAbout({ updateState: 'checking' });
      // The button renders 'Check for Updates' in an sr-only span when loading
      const btn = screen.getByRole('button', { name: 'Check for Updates' });
      expect(btn).toBeDisabled();
    });

    it('hides check button when installing', () => {
      renderAbout({ updateState: 'installing' });
      expect(screen.queryByText('Check for Updates')).not.toBeInTheDocument();
    });

    it('shows installing buttons when installing', () => {
      renderAbout({ updateState: 'installing' });
      expect(screen.getByText('Checking…')).toBeInTheDocument();
      expect(screen.getByText('Installing…')).toBeInTheDocument();
    });

    it('does not show Install Now when not available', () => {
      renderAbout({ updateState: 'idle' });
      expect(screen.queryByText('Install Now')).not.toBeInTheDocument();
    });
  });

  describe('button callbacks', () => {
    it('calls handleCheckUpdates when check button clicked', async () => {
      const handleCheck = vi.fn();
      renderAbout({ updateState: 'idle', handleCheckUpdates: handleCheck });
      
      const btn = screen.getByText('Check for Updates').closest('button')!;
      btn.click();
      expect(handleCheck).toHaveBeenCalledTimes(1);
    });

    it('calls handleInstallUpdate when install button clicked', async () => {
      const handleInstall = vi.fn();
      renderAbout({ updateState: 'available', handleInstallUpdate: handleInstall });
      
      const btn = screen.getByText('Install Now').closest('button')!;
      btn.click();
      expect(handleInstall).toHaveBeenCalledTimes(1);
    });
  });
});
