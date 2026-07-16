import { describe, expect, it, vi, beforeEach } from 'vitest';
import { screen, waitFor, act } from '@testing-library/react';
import { renderWithProvidersSync } from '@/__tests__/test-utils/render';
import MachineIdStatus from '@/components/MachineIdStatus';
import sharedFtl from '@/locales/shared.ftl?raw';

const mockGetMachineId = vi.fn();

vi.mock('@/api/license', () => ({
  getMachineId: (...args: unknown[]) => mockGetMachineId(...args),
}));

beforeEach(() => {
  mockGetMachineId.mockReset();

  Object.assign(navigator, {
    clipboard: { writeText: vi.fn().mockResolvedValue(undefined) },
  });
});

describe('MachineIdStatus', () => {
  it('shows loading state with dots', () => {
    mockGetMachineId.mockImplementation(() => new Promise(() => {}));
    renderWithProvidersSync(<MachineIdStatus />, sharedFtl);

    expect(screen.getByText('···············')).toBeTruthy();
  });

  it('displays the machine ID when loaded', async () => {
    mockGetMachineId.mockResolvedValue('abc-123-def');
    renderWithProvidersSync(<MachineIdStatus />, sharedFtl);

    await waitFor(() => {
      expect(screen.getByText('abc-123-def')).toBeTruthy();
    });
  });

  it('shows unavailable when API fails', async () => {
    mockGetMachineId.mockRejectedValue(new Error('Error'));
    renderWithProvidersSync(<MachineIdStatus />, sharedFtl);

    await waitFor(() => {
      expect(screen.getByText('unavailable')).toBeTruthy();
    });
  });

  it('has role="button" when machine ID is available', async () => {
    mockGetMachineId.mockResolvedValue('abc-123-def');
    renderWithProvidersSync(<MachineIdStatus />, sharedFtl);

    const chip = await screen.findByRole('button');
    expect(chip).toBeTruthy();
    expect(chip.getAttribute('tabindex')).toBe('0');
  });

  it('has tabIndex=-1 when unavailable', async () => {
    mockGetMachineId.mockRejectedValue(new Error('Error'));
    renderWithProvidersSync(<MachineIdStatus />, sharedFtl);

    await waitFor(() => {
      const chip = screen.getByText('unavailable').closest('[tabindex]');
      expect(chip?.getAttribute('tabindex')).toBe('-1');
    });
  });

  it('copies machine ID to clipboard on click', async () => {
    mockGetMachineId.mockResolvedValue('abc-123-def');
    const writeText = vi.fn().mockResolvedValue(undefined);
    Object.assign(navigator, { clipboard: { writeText } });

    renderWithProvidersSync(<MachineIdStatus />, sharedFtl);

    const chip = await screen.findByRole('button');
    await act(async () => { chip.click(); });

    expect(writeText).toHaveBeenCalledWith('abc-123-def');
  });

  it('copies machine ID on Enter key', async () => {
    mockGetMachineId.mockResolvedValue('abc-123-def');
    const writeText = vi.fn().mockResolvedValue(undefined);
    Object.assign(navigator, { clipboard: { writeText } });

    renderWithProvidersSync(<MachineIdStatus />, sharedFtl);

    const chip = await screen.findByRole('button');
    await act(async () => {
      chip.dispatchEvent(new KeyboardEvent('keydown', { key: 'Enter', bubbles: true }));
    });

    expect(writeText).toHaveBeenCalledWith('abc-123-def');
  });

  it('copies machine ID on Space key', async () => {
    mockGetMachineId.mockResolvedValue('abc-123-def');
    const writeText = vi.fn().mockResolvedValue(undefined);
    Object.assign(navigator, { clipboard: { writeText } });

    renderWithProvidersSync(<MachineIdStatus />, sharedFtl);

    const chip = await screen.findByRole('button');
    await act(async () => {
      chip.dispatchEvent(new KeyboardEvent('keydown', { key: ' ', bubbles: true }));
    });

    expect(writeText).toHaveBeenCalledWith('abc-123-def');
  });

  it('has correct aria-label with machine ID', async () => {
    mockGetMachineId.mockResolvedValue('abc-123-def');
    renderWithProvidersSync(<MachineIdStatus />, sharedFtl);

    const chip = await screen.findByRole('button');
    expect(chip.getAttribute('aria-label')).toContain('abc-123-def');
  });

  it('shows copy icon when machine ID is available', async () => {
    mockGetMachineId.mockResolvedValue('abc-123-def');
    const { container } = renderWithProvidersSync(<MachineIdStatus />, sharedFtl);

    await waitFor(() => {
      expect(container.querySelector('.machine-id-copy-icon')).toBeTruthy();
    });
  });

  it('hides copy icon when unavailable', async () => {
    mockGetMachineId.mockRejectedValue(new Error('Error'));
    const { container } = renderWithProvidersSync(<MachineIdStatus />, sharedFtl);

    await waitFor(() => {
      expect(container.querySelector('.machine-id-copy-icon')).toBeNull();
    });
  });
});
