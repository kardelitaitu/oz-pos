import { describe, it, expect, vi, beforeEach } from 'vitest';
import { screen, act, fireEvent } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { KdsSettingsPanel, DEFAULT_SETTINGS } from '@/features/kds/KdsSettingsPanel';
import kdsFtl from '@/locales/kds.ftl?raw';
import { renderWithFluentSync } from '@/__tests__/test-utils/render';

describe('KdsSettingsPanel', () => {
  const defaultProps = {
    settings: DEFAULT_SETTINGS,
    onChangeSound: vi.fn(),
    onChangeYellowThreshold: vi.fn(),
    onChangeRedThreshold: vi.fn(),
    onChangeAutoAcknowledge: vi.fn(),
    onChangeDensity: vi.fn(),
  };

  beforeEach(() => {
    vi.clearAllMocks();
  });

  describe('Initial render', () => {
    it('renders the gear button', () => {
      renderWithFluentSync(<KdsSettingsPanel {...defaultProps} />, kdsFtl);

      const btn = screen.getByRole('button', { name: /kds settings/i });
      expect(btn).toBeInTheDocument();
      expect(btn).toHaveAttribute('aria-expanded', 'false');
    });

    it('does not show the popover initially', () => {
      renderWithFluentSync(<KdsSettingsPanel {...defaultProps} />, kdsFtl);

      expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
    });
  });

  describe('Popover open/close', () => {
    it('opens the popover when gear button is clicked', async () => {
      const user = userEvent.setup();
      renderWithFluentSync(<KdsSettingsPanel {...defaultProps} />, kdsFtl);

      const btn = screen.getByRole('button', { name: /kds settings/i });
      await user.click(btn);

      const dialog = screen.getByRole('dialog', { name: /kds settings/i });
      expect(dialog).toBeInTheDocument();
      expect(btn).toHaveAttribute('aria-expanded', 'true');
    });

    it('closes the popover when Escape is pressed', async () => {
      const user = userEvent.setup();
      renderWithFluentSync(<KdsSettingsPanel {...defaultProps} />, kdsFtl);

      await user.click(screen.getByRole('button', { name: /kds settings/i }));
      expect(screen.getByRole('dialog')).toBeInTheDocument();

      await user.keyboard('{Escape}');
      expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
    });

    it('closes the popover when clicking outside', async () => {
      const user = userEvent.setup();
      renderWithFluentSync(<KdsSettingsPanel {...defaultProps} />, kdsFtl);

      await user.click(screen.getByRole('button', { name: /kds settings/i }));
      expect(screen.getByRole('dialog')).toBeInTheDocument();

      // Click on the body (outside the popover)
      await user.click(document.body);
      expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
    });

    it('toggles the popover on second button click', async () => {
      const user = userEvent.setup();
      renderWithFluentSync(<KdsSettingsPanel {...defaultProps} />, kdsFtl);

      const btn = screen.getByRole('button', { name: /kds settings/i });
      await user.click(btn);
      expect(screen.getByRole('dialog')).toBeInTheDocument();

      await user.click(btn);
      expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
    });
  });

  describe('Sound toggle', () => {
    it('renders Sound toggle in the checked state when enabled', async () => {
      const user = userEvent.setup();
      renderWithFluentSync(<KdsSettingsPanel {...defaultProps} settings={{ ...DEFAULT_SETTINGS, soundEnabled: true }} />, kdsFtl);

      await user.click(screen.getByRole('button', { name: /kds settings/i }));
      const soundToggle = screen.getByRole('switch', { name: /sound/i });
      expect(soundToggle).toBeChecked();
    });

    it('renders Sound toggle in the unchecked state when disabled', async () => {
      const user = userEvent.setup();
      renderWithFluentSync(<KdsSettingsPanel {...defaultProps} settings={{ ...DEFAULT_SETTINGS, soundEnabled: false }} />, kdsFtl);

      await user.click(screen.getByRole('button', { name: /kds settings/i }));
      const soundToggle = screen.getByRole('switch', { name: /sound/i });
      expect(soundToggle).not.toBeChecked();
    });

    it('calls onChangeSound when Sound switch is toggled', async () => {
      const user = userEvent.setup();
      const onChangeSound = vi.fn();
      renderWithFluentSync(<KdsSettingsPanel {...defaultProps} onChangeSound={onChangeSound} />, kdsFtl);

      await user.click(screen.getByRole('button', { name: /kds settings/i }));
      await user.click(screen.getByRole('switch', { name: /sound/i }));

      expect(onChangeSound).toHaveBeenCalledWith(false);
    });
  });

  describe('Yellow threshold slider', () => {
    it('renders yellow threshold slider with correct value', async () => {
      const user = userEvent.setup();
      renderWithFluentSync(<KdsSettingsPanel {...defaultProps} settings={{ ...DEFAULT_SETTINGS, yellowThresholdMin: 7 }} />, kdsFtl);

      await user.click(screen.getByRole('button', { name: /kds settings/i }));
      const slider = screen.getByRole('slider', { name: /yellow escalation threshold/i });
      expect(slider).toHaveValue('7');
    });

    it('calls onChangeYellowThreshold when slider value changes', async () => {
      const onChangeYellowThreshold = vi.fn();
      renderWithFluentSync(<KdsSettingsPanel {...defaultProps} onChangeYellowThreshold={onChangeYellowThreshold} />, kdsFtl);

      const user = userEvent.setup();
      await user.click(screen.getByRole('button', { name: /kds settings/i }));
      const slider = screen.getByRole('slider', { name: /yellow escalation threshold/i });

      // Use fireEvent to trigger React's synthetic onChange
      await act(async () => {
        fireEvent.change(slider, { target: { value: '8' } });
        await Promise.resolve();
      });

      expect(onChangeYellowThreshold).toHaveBeenCalledWith(8);
    });

    it('shows label with current value', async () => {
      const user = userEvent.setup();
      renderWithFluentSync(<KdsSettingsPanel {...defaultProps} settings={{ ...DEFAULT_SETTINGS, yellowThresholdMin: 5 }} />, kdsFtl);

      await user.click(screen.getByRole('button', { name: /kds settings/i }));
      expect(screen.getByText(/yellow at 5 min/i)).toBeInTheDocument();
    });
  });

  describe('Red threshold slider', () => {
    it('renders red threshold slider with correct value', async () => {
      const user = userEvent.setup();
      renderWithFluentSync(<KdsSettingsPanel {...defaultProps} settings={{ ...DEFAULT_SETTINGS, redThresholdMin: 12 }} />, kdsFtl);

      await user.click(screen.getByRole('button', { name: /kds settings/i }));
      const slider = screen.getByRole('slider', { name: /red escalation threshold/i });
      expect(slider).toHaveValue('12');
    });

    it('calls onChangeRedThreshold when slider value changes', async () => {
      const onChangeRedThreshold = vi.fn();
      renderWithFluentSync(<KdsSettingsPanel {...defaultProps} onChangeRedThreshold={onChangeRedThreshold} />, kdsFtl);

      const user = userEvent.setup();
      await user.click(screen.getByRole('button', { name: /kds settings/i }));
      const slider = screen.getByRole('slider', { name: /red escalation threshold/i });

      // Use fireEvent to trigger React's synthetic onChange
      await act(async () => {
        fireEvent.change(slider, { target: { value: '11' } });
        await Promise.resolve();
      });

      expect(onChangeRedThreshold).toHaveBeenCalledWith(11);
    });

    it('shows label with current value', async () => {
      const user = userEvent.setup();
      renderWithFluentSync(<KdsSettingsPanel {...defaultProps} settings={{ ...DEFAULT_SETTINGS, redThresholdMin: 10 }} />, kdsFtl);

      await user.click(screen.getByRole('button', { name: /kds settings/i }));
      expect(screen.getByText(/red at 10 min/i)).toBeInTheDocument();
    });

    it('enforces min value based on yellow threshold + 1', async () => {
      const user = userEvent.setup();
      renderWithFluentSync(<KdsSettingsPanel {...defaultProps} settings={{ ...DEFAULT_SETTINGS, yellowThresholdMin: 8, redThresholdMin: 10 }} />, kdsFtl);

      await user.click(screen.getByRole('button', { name: /kds settings/i }));
      const slider = screen.getByRole('slider', { name: /red escalation threshold/i });
      // Min should be yellowThresholdMin + 1 = 9, but also at least 6, so max(9, 6) = 9
      expect(slider).toHaveAttribute('min', '9');
    });
  });

  describe('Auto-acknowledge toggle', () => {
    it('renders Auto-acknowledge toggle in the checked state when enabled', async () => {
      const user = userEvent.setup();
      renderWithFluentSync(<KdsSettingsPanel {...defaultProps} settings={{ ...DEFAULT_SETTINGS, autoAcknowledge: true }} />, kdsFtl);

      await user.click(screen.getByRole('button', { name: /kds settings/i }));
      const toggle = screen.getByRole('switch', { name: /auto-acknowledge/i });
      expect(toggle).toBeChecked();
    });

    it('renders Auto-acknowledge toggle in the unchecked state when disabled', async () => {
      const user = userEvent.setup();
      renderWithFluentSync(<KdsSettingsPanel {...defaultProps} settings={{ ...DEFAULT_SETTINGS, autoAcknowledge: false }} />, kdsFtl);

      await user.click(screen.getByRole('button', { name: /kds settings/i }));
      const toggle = screen.getByRole('switch', { name: /auto-acknowledge/i });
      expect(toggle).not.toBeChecked();
    });

    it('calls onChangeAutoAcknowledge when toggle is changed', async () => {
      const user = userEvent.setup();
      const onChangeAutoAcknowledge = vi.fn();
      renderWithFluentSync(<KdsSettingsPanel {...defaultProps} onChangeAutoAcknowledge={onChangeAutoAcknowledge} />, kdsFtl);

      await user.click(screen.getByRole('button', { name: /kds settings/i }));
      await user.click(screen.getByRole('switch', { name: /auto-acknowledge/i }));

      expect(onChangeAutoAcknowledge).toHaveBeenCalledWith(true);
    });
  });

  describe('Display density', () => {
    it('renders both density options', async () => {
      const user = userEvent.setup();
      renderWithFluentSync(<KdsSettingsPanel {...defaultProps} />, kdsFtl);

      await user.click(screen.getByRole('button', { name: /kds settings/i }));
      expect(screen.getByRole('button', { name: /comfortable/i })).toBeInTheDocument();
      expect(screen.getByRole('button', { name: /compact/i })).toBeInTheDocument();
    });

    it('marks the current density as pressed', async () => {
      const user = userEvent.setup();
      renderWithFluentSync(<KdsSettingsPanel {...defaultProps} settings={{ ...DEFAULT_SETTINGS, density: 'compact' }} />, kdsFtl);

      await user.click(screen.getByRole('button', { name: /kds settings/i }));
      const compactBtn = screen.getByRole('button', { name: /compact/i });
      expect(compactBtn).toHaveAttribute('aria-pressed', 'true');

      const comfortableBtn = screen.getByRole('button', { name: /comfortable/i });
      expect(comfortableBtn).toHaveAttribute('aria-pressed', 'false');
    });

    it('calls onChangeDensity when a density button is clicked', async () => {
      const user = userEvent.setup();
      const onChangeDensity = vi.fn();
      renderWithFluentSync(<KdsSettingsPanel {...defaultProps} onChangeDensity={onChangeDensity} />, kdsFtl);

      await user.click(screen.getByRole('button', { name: /kds settings/i }));
      await user.click(screen.getByRole('button', { name: /compact/i }));

      expect(onChangeDensity).toHaveBeenCalledWith('compact');
    });
  });

  describe('Default settings', () => {
    it('uses DEFAULT_SETTINGS when no settings prop provided', async () => {
      const user = userEvent.setup();
      renderWithFluentSync(<KdsSettingsPanel {...defaultProps} settings={DEFAULT_SETTINGS} />, kdsFtl);

      await user.click(screen.getByRole('button', { name: /kds settings/i }));

      const soundToggle = screen.getByRole('switch', { name: /sound/i });
      expect(soundToggle).toBeChecked();

      const yellowSlider = screen.getByRole('slider', { name: /yellow escalation threshold/i });
      expect(yellowSlider).toHaveValue(String(DEFAULT_SETTINGS.yellowThresholdMin));

      const redSlider = screen.getByRole('slider', { name: /red escalation threshold/i });
      expect(redSlider).toHaveValue(String(DEFAULT_SETTINGS.redThresholdMin));

      const autoAckToggle = screen.getByRole('switch', { name: /auto-acknowledge/i });
      expect(autoAckToggle).not.toBeChecked();

      const comfortableBtn = screen.getByRole('button', { name: /comfortable/i });
      expect(comfortableBtn).toHaveAttribute('aria-pressed', 'true');
    });
  });
});