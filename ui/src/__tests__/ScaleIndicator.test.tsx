// ── ScaleIndicator tests ───────────────────────────────────────────
//
// Covers: idle state (no scale), error state, stable/unstable
// readings, weigh-target add/clear buttons, and weight formatting.

import { describe, expect, it, vi, beforeEach } from 'vitest';
import { render, screen, act, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import ScaleIndicator from '@/features/retail/ScaleIndicator';
import type { WeightReading } from '@/api/hardware';
// ── Mocks ──────────────────────────────────────────────────────────

const mockReadScaleWeight = vi.fn();

vi.mock('@/api/hardware', () => ({
  readScaleWeight: () => mockReadScaleWeight(),
}));

vi.mock('@fluent/react', () => ({
  useLocalization: () => ({
    l10n: {
      getString: (id: string, vars?: Record<string, string>) => {
        if (vars?.name) return `Weigh & add ${vars.name}`;
        return id;
      },
    },
  }),
  Localized: ({ children }: { children: React.ReactNode }) => <>{children}</>,
}));

// ── Test data ──────────────────────────────────────────────────────

const testSku = 'TEST-001' as string;

const stableReading: WeightReading = { weightGrams: 500, stable: true };
const unstableReading: WeightReading = { weightGrams: 320, stable: false };

// ── Tests ──────────────────────────────────────────────────────────

describe('ScaleIndicator', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockReadScaleWeight.mockResolvedValue(null);
  });

  // ── Idle state ─────────────────────────────────────────────────

  it('shows idle state when no scale is connected', async () => {
    mockReadScaleWeight.mockResolvedValue(null);

    render(
      <ScaleIndicator
        weighTarget={null}
        onWeighAdd={vi.fn()}
        onClearWeighTarget={vi.fn()}
      />,
    );

    await waitFor(() => {
      expect(screen.getByText('scale-idle')).toBeInTheDocument();
    });

    const container = document.querySelector('.scale-indicator--idle');
    expect(container).toBeInTheDocument();
  });

  // ── Error state ────────────────────────────────────────────────

  it('shows error state when scale read fails', async () => {
    mockReadScaleWeight.mockRejectedValue(new Error('Hardware error'));

    render(
      <ScaleIndicator
        weighTarget={null}
        onWeighAdd={vi.fn()}
        onClearWeighTarget={vi.fn()}
      />,
    );

    await waitFor(() => {
      expect(screen.getByText('scale-read-error')).toBeInTheDocument();
    });

    const container = document.querySelector('.scale-indicator--error');
    expect(container).toBeInTheDocument();
  });

  // ── Stable reading ─────────────────────────────────────────────

  it('shows stable weight and \"Stable\" label', async () => {
    mockReadScaleWeight.mockResolvedValue(stableReading);

    render(
      <ScaleIndicator
        weighTarget={null}
        onWeighAdd={vi.fn()}
        onClearWeighTarget={vi.fn()}
      />,
    );

    await waitFor(() => {
      expect(screen.getByText('500 g')).toBeInTheDocument();
    });

    expect(screen.getByText('scale-stable')).toBeInTheDocument();
    expect(
      document.querySelector('.scale-indicator--stable'),
    ).toBeInTheDocument();
  });

  // ── Unstable reading ───────────────────────────────────────────

  it('shows unstable weight and \"…\" label', async () => {
    mockReadScaleWeight.mockResolvedValue(unstableReading);

    render(
      <ScaleIndicator
        weighTarget={null}
        onWeighAdd={vi.fn()}
        onClearWeighTarget={vi.fn()}
      />,
    );

    await waitFor(() => {
      expect(screen.getByText('320 g')).toBeInTheDocument();
    });

    expect(screen.getByText('scale-unstable')).toBeInTheDocument();
    expect(
      document.querySelector('.scale-indicator--unstable'),
    ).toBeInTheDocument();
  });

  // ── Weight formatting ──────────────────────────────────────────

  it('formats weights >= 1000g as kg', async () => {
    mockReadScaleWeight.mockResolvedValue({
      weightGrams: 1500,
      stable: true,
    });

    render(
      <ScaleIndicator
        weighTarget={null}
        onWeighAdd={vi.fn()}
        onClearWeighTarget={vi.fn()}
      />,
    );

    await waitFor(() => {
      expect(screen.getByText('1.50 kg')).toBeInTheDocument();
    });
  });

  // ── Weigh target actions ───────────────────────────────────────

  it('shows weigh-target actions when reading is stable and positive', async () => {
    mockReadScaleWeight.mockResolvedValue(stableReading);
    const onWeighAdd = vi.fn();
    const onClearWeighTarget = vi.fn();

    render(
      <ScaleIndicator
        weighTarget={{ sku: testSku, name: 'Test Product' }}
        onWeighAdd={onWeighAdd}
        onClearWeighTarget={onClearWeighTarget}
      />,
    );

    await waitFor(() => {
      expect(screen.getByText('scale-weigh-add')).toBeInTheDocument();
    });

    expect(screen.getByText('Test Product')).toBeInTheDocument();
  });

  it('calls onWeighAdd when weigh button is clicked', async () => {
    mockReadScaleWeight.mockResolvedValue(stableReading);
    const onWeighAdd = vi.fn();

    render(
      <ScaleIndicator
        weighTarget={{ sku: testSku, name: 'Test Product' }}
        onWeighAdd={onWeighAdd}
        onClearWeighTarget={vi.fn()}
      />,
    );

    await waitFor(() => {
      expect(screen.getByText('scale-weigh-add')).toBeInTheDocument();
    });

    await userEvent.click(screen.getByText('scale-weigh-add'));

    expect(onWeighAdd).toHaveBeenCalledWith(testSku, stableReading.weightGrams);
  });

  it('does not show weigh-target actions when reading is unstable', async () => {
    mockReadScaleWeight.mockResolvedValue(unstableReading);

    render(
      <ScaleIndicator
        weighTarget={{ sku: testSku, name: 'Test Product' }}
        onWeighAdd={vi.fn()}
        onClearWeighTarget={vi.fn()}
      />,
    );

    await waitFor(() => {
      expect(screen.getByText('320 g')).toBeInTheDocument();
    });

    // Actions should NOT appear for unstable readings.
    expect(screen.queryByText('scale-weigh-add')).not.toBeInTheDocument();
  });

  it('calls onClearWeighTarget when clear button is clicked', async () => {
    mockReadScaleWeight.mockResolvedValue(stableReading);
    const onClearWeighTarget = vi.fn();

    render(
      <ScaleIndicator
        weighTarget={{ sku: testSku, name: 'Test Product' }}
        onWeighAdd={vi.fn()}
        onClearWeighTarget={onClearWeighTarget}
      />,
    );

    await waitFor(() => {
      expect(screen.getByText('scale-weigh-add')).toBeInTheDocument();
    });

    const clearBtn = document.querySelector('.scale-indicator-clear-btn');
    expect(clearBtn).toBeInTheDocument();
    await userEvent.click(clearBtn!);

    expect(onClearWeighTarget).toHaveBeenCalledTimes(1);
  });

  // ── ARIA ───────────────────────────────────────────────────────

  it('has status role and ARIA label', async () => {
    mockReadScaleWeight.mockResolvedValue(null);

    render(
      <ScaleIndicator
        weighTarget={null}
        onWeighAdd={vi.fn()}
        onClearWeighTarget={vi.fn()}
      />,
    );

    await waitFor(() => {
      const el = document.querySelector('[role="status"]');
      expect(el).toBeInTheDocument();
      expect(el?.getAttribute('aria-label')).toBe('scale-indicator-aria');
    });
  });
});
