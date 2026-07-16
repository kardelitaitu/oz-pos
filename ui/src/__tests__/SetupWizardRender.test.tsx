// ── SetupWizard sync render tests ─────────────────────────────────
//
// Covers: initial render of preset selection, skip button visibility,
// and absence of Back button on step 1. Fast synchronous tests
// extracted from SetupWizard.test.tsx for parallel execution. 3 tests.

import { describe, expect, it, vi } from 'vitest';
import { render, screen } from '@testing-library/react';
import { FluentBundle, FluentResource } from '@fluent/bundle';
import { LocalizationProvider, ReactLocalization } from '@fluent/react';
import type { ReactNode } from 'react';
import SetupWizard from '@/features/setup/SetupWizard';
import settingsFtl from '@/locales/settings.ftl?raw';
import salesFtl from '@/locales/sales.ftl?raw';

const bundle = new FluentBundle('en-US');
bundle.addResource(new FluentResource(settingsFtl));
bundle.addResource(new FluentResource(salesFtl));
const l10n = new ReactLocalization([bundle]);

function FluentWrapper({ children }: { children: ReactNode }) {
  return <LocalizationProvider l10n={l10n}>{children}</LocalizationProvider>;
}

describe('SetupWizard — rendering', () => {
  it('renders the preset selection step on mount', () => {
    render(<SetupWizard />, { wrapper: FluentWrapper });

    expect(
      screen.getByText('What kind of store are you running?'),
    ).toBeInTheDocument();
    expect(
      screen.getByRole('radiogroup', { name: /store preset/i }),
    ).toBeInTheDocument();

    const cards = screen.getAllByRole('radio');
    expect(cards).toHaveLength(6);
    expect(cards[0]).toHaveTextContent('Simple Retail');
    expect(cards[1]).toHaveTextContent('Restaurant');
    expect(cards[2]).toHaveTextContent('Full Store');
    expect(cards[3]).toHaveTextContent('Cafe / Bakery');
    expect(cards[4]).toHaveTextContent('Franchise');
    expect(cards[5]).toHaveTextContent('Custom');
  });

  it('shows the skip button on step 1 when no preset is selected', () => {
    const onSkip = vi.fn();
    render(<SetupWizard onSkip={onSkip} />, { wrapper: FluentWrapper });

    expect(
      screen.getByRole('button', { name: /skip setup/i }),
    ).toBeInTheDocument();
  });

  it('Back button is not shown on step 1', () => {
    render(<SetupWizard />, { wrapper: FluentWrapper });

    expect(
      screen.queryByRole('button', { name: /back/i }),
    ).not.toBeInTheDocument();
  });
});
