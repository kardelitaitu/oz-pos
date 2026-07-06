import { describe, expect, it, vi, beforeEach } from 'vitest';
import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { withFluent } from '@/locales/test-utils';

// Mock useFeatures to always return enabled=true for any feature.
vi.mock('@/hooks/useFeatures', () => ({
  useFeatures: () => ({
    isEnabled: vi.fn(() => true),
    enabled: new Set(['usb-scale']),
    loading: false,
    loaded: true,
    error: null,
    filterRoutes: (routes: string[]) => routes,
  }),
  FEATURES: {
    USB_SCALE: 'usb-scale',
  },
}));

// Mock the hardware API.
vi.mock('@/api/hardware', () => ({
  readScaleWeight: vi.fn(),
}));

// Mock the toast hook.
vi.mock('@/frontend/shared/Toast', () => ({
  useToast: () => ({ addToast: vi.fn() }),
}));

import { WeightScaleWidget } from '@/features/sales/WeightScaleWidget';
import { readScaleWeight } from '@/api/hardware';

const mockReadScaleWeight = readScaleWeight as ReturnType<typeof vi.fn>;

const scaleFtl = `
weight-scale-aria = Weight Scale
weight-scale-weigh = Weigh
weight-scale-weighing = Weighing…
weight-scale-weigh-aria = Read weight from scale
weight-scale-stable = Stable reading
weight-scale-unstable = Unstable reading
weight-scale-idle = —
weight-scale-error = Scale error
`;

const wrap = (children: React.ReactNode) => withFluent(children, scaleFtl);

describe('WeightScaleWidget', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('renders the weigh button and idle display when feature is enabled', () => {
    render(wrap(<WeightScaleWidget />));
    expect(screen.getByRole('button', { name: /read weight/i })).toBeInTheDocument();
    expect(screen.getByText('—')).toBeInTheDocument();
  });

  it('does not crash and renders the region aria-label', () => {
    render(wrap(<WeightScaleWidget />));
    expect(screen.getByRole('region', { name: 'Weight Scale' })).toBeInTheDocument();
  });

  it('calls readScaleWeight on weigh click', async () => {
    mockReadScaleWeight.mockResolvedValueOnce({ weight_grams: 500, stable: true });
    render(wrap(<WeightScaleWidget />));

    await userEvent.click(screen.getByRole('button', { name: /read weight/i }));
    expect(mockReadScaleWeight).toHaveBeenCalledTimes(1);
  });

  it('displays weight after successful read', async () => {
    mockReadScaleWeight.mockResolvedValueOnce({ weight_grams: 500, stable: true });
    render(wrap(<WeightScaleWidget />));

    await userEvent.click(screen.getByRole('button', { name: /read weight/i }));

    await vi.waitFor(() => {
      expect(screen.getByText('500.0 g')).toBeInTheDocument();
    });
  });

  it('displays kilograms for weights >= 1000g', async () => {
    mockReadScaleWeight.mockResolvedValueOnce({ weight_grams: 2500, stable: true });
    render(wrap(<WeightScaleWidget />));

    await userEvent.click(screen.getByRole('button', { name: /read weight/i }));

    await vi.waitFor(() => {
      expect(screen.getByText('2.500 kg')).toBeInTheDocument();
    });
  });

  it('shows stable indicator when reading is stable', async () => {
    mockReadScaleWeight.mockResolvedValueOnce({ weight_grams: 100, stable: true });
    render(wrap(<WeightScaleWidget />));

    await userEvent.click(screen.getByRole('button', { name: /read weight/i }));

    await vi.waitFor(() => {
      expect(screen.getByTitle('Stable reading')).toBeInTheDocument();
    });
  });

  it('shows unstable indicator when reading is not stable', async () => {
    mockReadScaleWeight.mockResolvedValueOnce({ weight_grams: 432, stable: false });
    render(wrap(<WeightScaleWidget />));

    await userEvent.click(screen.getByRole('button', { name: /read weight/i }));

    await vi.waitFor(() => {
      expect(screen.getByTitle('Unstable reading')).toBeInTheDocument();
      expect(screen.getByText('432.0 g')).toBeInTheDocument();
    });
  });

  it('shows error when read fails', async () => {
    mockReadScaleWeight.mockRejectedValueOnce(new Error('Device not found'));
    render(wrap(<WeightScaleWidget />));

    await userEvent.click(screen.getByRole('button', { name: /read weight/i }));

    await vi.waitFor(() => {
      expect(screen.getByText(/scale error/i)).toBeInTheDocument();
    });
  });

  it('calls onWeightObtained callback after successful read', async () => {
    const reading = { weight_grams: 750, stable: true };
    mockReadScaleWeight.mockResolvedValueOnce(reading);
    const onWeightObtained = vi.fn();

    render(wrap(<WeightScaleWidget onWeightObtained={onWeightObtained} />));

    await userEvent.click(screen.getByRole('button', { name: /read weight/i }));

    await vi.waitFor(() => {
      expect(onWeightObtained).toHaveBeenCalledWith(reading);
    });
  });

  it('disables button while weighing', async () => {
    // Never resolve — keeps weighing=true
    mockReadScaleWeight.mockReturnValueOnce(new Promise(() => {}));
    render(wrap(<WeightScaleWidget />));

    await userEvent.click(screen.getByRole('button', { name: /read weight/i }));

    await vi.waitFor(() => {
      expect(screen.getByRole('button', { name: /read weight/i })).toBeDisabled();
      expect(screen.getByText('Weighing…')).toBeInTheDocument();
    });
  });
});
