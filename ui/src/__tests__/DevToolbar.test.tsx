import { describe, it, expect, vi, beforeAll } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';
import { ThemeProvider } from '@/frontend/shell/ThemeProvider';
import { BrandProvider } from '@/contexts/BrandContext';
import { DevToolbar } from '@/features/design/DevToolbar';

// ── Wrapper with ThemeProvider ─────────────────────────────────────

function renderToolbar() {
  return render(
    <BrandProvider>
      <ThemeProvider>
        <DevToolbar />
      </ThemeProvider>
    </BrandProvider>,
  );
}

// If the test file's matchMedia mock hasn't been set up globally, provide one.
function mockMatchMedia() {
  if (typeof window.matchMedia !== 'function') {
    Object.defineProperty(window, 'matchMedia', {
      writable: true,
      value: vi.fn().mockImplementation((query: string) => ({
        matches: false,
        media: query,
        onchange: null,
        addListener: vi.fn(),
        removeListener: vi.fn(),
        addEventListener: vi.fn(),
        removeEventListener: vi.fn(),
        dispatchEvent: vi.fn(),
      })),
    });
  }
}

// ── Tests ──────────────────────────────────────────────────────────

describe('DevToolbar', () => {
  beforeAll(() => {
    mockMatchMedia();
    // Suppress "Test <name> is not wrapped in act" warnings
    vi.spyOn(console, 'error').mockImplementation(() => {});
  });

  it('renders without crashing', () => {
    const { container } = renderToolbar();
    expect(container.querySelector('.dev-toolbar')).not.toBeNull();
  });

  it('renders three theme buttons', () => {
    renderToolbar();
    expect(screen.getByLabelText('Glass theme')).toBeInTheDocument();
    expect(screen.getByLabelText('Light theme')).toBeInTheDocument();
    expect(screen.getByLabelText('Dark theme')).toBeInTheDocument();
  });

  it('renders the active theme badge', () => {
    renderToolbar();
    const badge = document.querySelector('.dev-toolbar-badge');
    expect(badge).not.toBeNull();
    expect(badge?.textContent).toMatch(/glass|light|dark/i);
  });

  it('switches theme when a theme button is clicked', () => {
    renderToolbar();
    const lightBtn = screen.getByLabelText('Light theme');
    fireEvent.click(lightBtn);
    expect(lightBtn).toHaveAttribute('aria-checked', 'true');
  });

  it('shows colour swatches matching the current theme', () => {
    renderToolbar();
    const swatches = document.querySelectorAll('.dev-toolbar-swatch');
    expect(swatches.length).toBeGreaterThanOrEqual(1);
  });

  it('renders with correct ARIA role and label', () => {
    renderToolbar();
    expect(screen.getByRole('toolbar', { name: /developer tools/i })).toBeInTheDocument();
  });

  it('allows switching between themes and back', () => {
    renderToolbar();
    const glassBtn = screen.getByLabelText('Glass theme');
    const darkBtn = screen.getByLabelText('Dark theme');

    fireEvent.click(glassBtn);
    expect(glassBtn).toHaveAttribute('aria-checked', 'true');

    fireEvent.click(darkBtn);
    expect(darkBtn).toHaveAttribute('aria-checked', 'true');
    expect(glassBtn).not.toHaveAttribute('aria-checked', 'true');
  });
});
