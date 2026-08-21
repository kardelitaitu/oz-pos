import { describe, expect, it, vi, beforeEach } from 'vitest';
import { screen, fireEvent } from '@testing-library/react';
import { renderWithProvidersSync } from '@/__tests__/test-utils/render';
import settingsFtl from '@/locales/settings.ftl?raw';
import ExitSurveyModal from '@/components/ExitSurveyModal';

const { mockInvoke } = vi.hoisted(() => ({
  mockInvoke: vi.fn(),
}));

vi.mock('@/utils/logged-invoke', () => ({
  loggedInvoke: (...args: unknown[]) => mockInvoke(...args),
}));

vi.mock('@/contexts/WorkspaceContext', () => ({
  useWorkspace: () => ({ sessionToken: 'test-token' }),
}));

vi.mock('@/contexts/BrandContext', () => ({
  BrandProvider: ({ children }: { children: React.ReactNode }) => children,
  useBrand: () => ({ settings: { primary_colour: '#0066ff' } }),
}));

beforeEach(() => {
  mockInvoke.mockReset();
  mockInvoke.mockResolvedValue(undefined);
});

describe('ExitSurveyModal', () => {
  const defaultProps = {
    open: true,
    onClose: vi.fn(),
    onConfirm: vi.fn(),
  };

  beforeEach(() => {
    vi.clearAllMocks();
  });

  // ── Render tests ──────────────────────────────────────────────

  it('renders nothing when open is false', () => {
    renderWithProvidersSync(
      <ExitSurveyModal {...defaultProps} open={false} />,
      settingsFtl,
    );
    expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
  });

  it('renders the dialog with title and message when open', () => {
    renderWithProvidersSync(<ExitSurveyModal {...defaultProps} />, settingsFtl);
    expect(screen.getByRole('dialog')).toBeInTheDocument();
    expect(screen.getByText('Before you pause...')).toBeInTheDocument();
    expect(
      screen.getByText(/help us improve/i),
    ).toBeInTheDocument();
  });

  it('renders all 6 exit survey reason options', () => {
    renderWithProvidersSync(<ExitSurveyModal {...defaultProps} />, settingsFtl);
    expect(screen.getByText('Too expensive')).toBeInTheDocument();
    expect(screen.getByText('Not enough features')).toBeInTheDocument();
    expect(screen.getByText('Switching to a competitor')).toBeInTheDocument();
    expect(screen.getByText('Business closed')).toBeInTheDocument();
    expect(screen.getByText('Taking a temporary break')).toBeInTheDocument();
    expect(screen.getByText('Other')).toBeInTheDocument();
  });

  it('renders the cancel and submit buttons', () => {
    renderWithProvidersSync(<ExitSurveyModal {...defaultProps} />, settingsFtl);
    expect(screen.getByRole('button', { name: /go back/i })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /pause subscription/i })).toBeInTheDocument();
  });

  // ── Reason selection ──────────────────────────────────────────

  it('enables the submit button when a reason is selected', () => {
    renderWithProvidersSync(<ExitSurveyModal {...defaultProps} />, settingsFtl);
    const submitBtn = screen.getByRole('button', { name: /pause subscription/i });
    expect(submitBtn).toBeDisabled();

    fireEvent.click(screen.getByText('Too expensive'));
    expect(submitBtn).not.toBeDisabled();
  });

  it('shows the "Other" textarea when "Other" reason is selected', () => {
    renderWithProvidersSync(<ExitSurveyModal {...defaultProps} />, settingsFtl);
    expect(screen.queryByRole('textbox')).not.toBeInTheDocument();

    fireEvent.click(screen.getByText('Other'));
    expect(screen.getByRole('textbox')).toBeInTheDocument();
  });

  it('hides the "Other" textarea when a different reason is selected', () => {
    renderWithProvidersSync(<ExitSurveyModal {...defaultProps} />, settingsFtl);
    fireEvent.click(screen.getByText('Other'));
    expect(screen.getByRole('textbox')).toBeInTheDocument();

    fireEvent.click(screen.getByText('Too expensive'));
    expect(screen.queryByRole('textbox')).not.toBeInTheDocument();
  });

  // ── IPC submit ────────────────────────────────────────────────

  it('calls IPC with selected reason and then onConfirm', async () => {
    renderWithProvidersSync(<ExitSurveyModal {...defaultProps} />, settingsFtl);

    fireEvent.click(screen.getByText('Too expensive'));
    fireEvent.click(screen.getByRole('button', { name: /pause subscription/i }));

    // Wait for the async IPC call
    await vi.waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledWith('set_exit_survey_response', {
        sessionToken: 'test-token',
        reason: 'too_expensive',
        detail: undefined,
      });
    });
    expect(defaultProps.onConfirm).toHaveBeenCalled();
  });

  it('includes detail text when "Other" reason is selected', async () => {
    renderWithProvidersSync(<ExitSurveyModal {...defaultProps} />, settingsFtl);

    fireEvent.click(screen.getByText('Other'));
    fireEvent.change(screen.getByRole('textbox'), {
      target: { value: 'Switching to competitor X' },
    });
    fireEvent.click(screen.getByRole('button', { name: /pause subscription/i }));

    await vi.waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledWith('set_exit_survey_response', {
        sessionToken: 'test-token',
        reason: 'other',
        detail: 'Switching to competitor X',
      });
    });
    expect(defaultProps.onConfirm).toHaveBeenCalled();
  });

  it('still calls onConfirm even if IPC fails (best-effort)', async () => {
    mockInvoke.mockRejectedValue(new Error('IPC error'));
    const consoleSpy = vi.spyOn(console, 'warn').mockImplementation(() => {});

    renderWithProvidersSync(<ExitSurveyModal {...defaultProps} />, settingsFtl);

    fireEvent.click(screen.getByText('Business closed'));
    fireEvent.click(screen.getByRole('button', { name: /pause subscription/i }));

    await vi.waitFor(() => {
      expect(defaultProps.onConfirm).toHaveBeenCalled();
    });
    expect(consoleSpy).toHaveBeenCalledWith('Failed to save exit survey response');
    consoleSpy.mockRestore();
  });

  // ── Cancel ────────────────────────────────────────────────────

  it('calls onClose when cancel button is clicked', () => {
    renderWithProvidersSync(<ExitSurveyModal {...defaultProps} />, settingsFtl);
    fireEvent.click(screen.getByRole('button', { name: /go back/i }));
    expect(defaultProps.onClose).toHaveBeenCalled();
    expect(defaultProps.onConfirm).not.toHaveBeenCalled();
  });

  it('does not submit IPC when cancel is clicked without selecting a reason', () => {
    renderWithProvidersSync(<ExitSurveyModal {...defaultProps} />, settingsFtl);
    fireEvent.click(screen.getByRole('button', { name: /go back/i }));
    expect(mockInvoke).not.toHaveBeenCalled();
  });

  // ── Submit button disabled state ───────────────────────────────

  it('disables submit when no reason is selected', () => {
    renderWithProvidersSync(<ExitSurveyModal {...defaultProps} />, settingsFtl);
    expect(screen.getByRole('button', { name: /pause subscription/i })).toBeDisabled();
  });

  // ── Accessibility ─────────────────────────────────────────────

  it('has a radiogroup with accessible label', () => {
    renderWithProvidersSync(<ExitSurveyModal {...defaultProps} />, settingsFtl);
    expect(screen.getByRole('radiogroup', { name: /before you pause/i })).toBeInTheDocument();
  });
});
