import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, waitFor, fireEvent } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { AppearanceSettings } from '@/features/settings/AppearanceSettings';

// ── Mocks ────────────────────────────────────────────────────────

const mockGetBrandSettings = vi.fn();
const mockSetBrandPrimaryColour = vi.fn();
const mockSetBrandLogoPath = vi.fn();
const mockSetBrandStoreName = vi.fn();
const mockPickLogoFile = vi.fn();
const mockRefreshBrandSettings = vi.fn();

vi.mock('@/api/branding', () => ({
  getBrandSettings: () => mockGetBrandSettings(),
  setBrandPrimaryColour: (c: string) => mockSetBrandPrimaryColour(c),
  setBrandLogoPath: (p: string) => mockSetBrandLogoPath(p),
  setBrandStoreName: (n: string) => mockSetBrandStoreName(n),
  pickLogoFile: () => mockPickLogoFile(),
}));

vi.mock('@/contexts/BrandContext', () => ({
  useBrand: () => ({
    settings: {
      primary_colour: '#10b981',
      logo_path: null,
      store_name: '',
    },
    refreshBrandSettings: () => mockRefreshBrandSettings(),
  }),
}));

vi.mock('@fluent/react', () => ({
  useLocalization: () => ({
    l10n: { getString: (id: string) => id },
  }),
  Localized: ({ id, children }: { id: string; children: React.ReactNode }) => <>{children}</>,
}));

const mockDeriveAccentPalette = vi.fn();
const mockApplyAccentPalette = vi.fn();

vi.mock('@/utils/color', () => ({
  deriveAccentPalette: (base: string) => mockDeriveAccentPalette(base),
  applyAccentPalette: (palette: unknown) => mockApplyAccentPalette(palette),
}));

vi.mock('@/components/Card', () => ({
  Card: ({ children, shadow }: { children: React.ReactNode; shadow?: string }) => (
    <div data-testid="card" data-shadow={shadow}>{children}</div>
  ),
}));

vi.mock('@/components/Button', () => ({
  Button: ({
    children,
    onClick,
    variant,
    disabled,
    'aria-label': ariaLabel,
  }: {
    children: React.ReactNode;
    onClick?: () => void;
    variant?: string;
    disabled?: boolean;
    'aria-label'?: string;
  }) => (
    <button
      onClick={onClick}
      className={`btn btn--${variant ?? 'primary'}`}
      disabled={disabled}
      aria-label={ariaLabel}
    >
      {children}
    </button>
  ),
}));

// ── Default brand settings ────────────────────────────────────────

const defaultBrandResponse = {
  primary_colour: '#10b981',
  logo_path: null,
  store_name: '',
};

// ── Tests ────────────────────────────────────────────────────────

describe('AppearanceSettings', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockGetBrandSettings.mockResolvedValue(defaultBrandResponse);
    mockSetBrandPrimaryColour.mockResolvedValue(undefined);
    mockSetBrandLogoPath.mockResolvedValue(undefined);
    mockSetBrandStoreName.mockResolvedValue(undefined);
    mockPickLogoFile.mockResolvedValue(null);
    mockDeriveAccentPalette.mockReturnValue({});
    mockApplyAccentPalette.mockReturnValue(undefined);
  });

  // ── Rendering ──────────────────────────────────────────────────

  it('renders inside a Card wrapper in non-embedded mode', async () => {
    render(<AppearanceSettings />);
    await waitFor(() => {
      expect(screen.getByTestId('card')).toBeInTheDocument();
    });
  });

  it('renders without Card wrapper in embedded mode', () => {
    render(<AppearanceSettings embedded colour="#ff0000" storeName="Test Store" />);
    expect(screen.queryByTestId('card')).not.toBeInTheDocument();
  });

  it('shows "Appearance" section title in non-embedded mode', async () => {
    render(<AppearanceSettings />);
    await waitFor(() => {
      expect(screen.getByText('Appearance')).toBeInTheDocument();
    });
  });

  // ── Colour picker ──────────────────────────────────────────────

  it('renders colour picker with initial value', async () => {
    render(<AppearanceSettings />);
    await waitFor(() => {
      expect(screen.getByLabelText('Primary colour picker')).toBeInTheDocument();
    });

    const colourInput = screen.getByLabelText('Primary colour picker') as HTMLInputElement;
    expect(colourInput.value).toBe('#10b981');
  });

  it('renders hex text input with initial value', async () => {
    render(<AppearanceSettings />);
    await waitFor(() => {
      expect(screen.getByLabelText('Colour hex value')).toBeInTheDocument();
    });

    const hexInput = screen.getByLabelText('Colour hex value') as HTMLInputElement;
    expect(hexInput.value).toBe('#10b981');
  });

  it('updates colour picker value when user changes hex input', async () => {
    const user = userEvent.setup();
    render(<AppearanceSettings />);
    await waitFor(() => {
      expect(screen.getByLabelText('Colour hex value')).toBeInTheDocument();
    });

    const hexInput = screen.getByLabelText('Colour hex value') as HTMLInputElement;
    await user.clear(hexInput);
    await user.type(hexInput, '#ff5500');

    expect(hexInput.value).toBe('#ff5500');
    // Colour picker should sync.
    const colourInput = screen.getByLabelText('Primary colour picker') as HTMLInputElement;
    expect(colourInput.value).toBe('#ff5500');
  });

  // ── Store name ─────────────────────────────────────────────────

  it('renders store name input', async () => {
    mockGetBrandSettings.mockResolvedValue({ ...defaultBrandResponse, store_name: 'My Shop' });
    render(<AppearanceSettings />);

    await waitFor(() => {
      const input = screen.getByDisplayValue('My Shop');
      expect(input).toBeInTheDocument();
    });
  });

  it('updates store name when user types', async () => {
    const user = userEvent.setup();
    render(<AppearanceSettings />);
    await waitFor(() => {
      expect(screen.getByLabelText('Colour hex value')).toBeInTheDocument();
    });

    const storeNameInput = document.getElementById('store-name-display') as HTMLInputElement;
    await user.clear(storeNameInput);
    await user.type(storeNameInput, 'Acme POS');

    expect(storeNameInput.value).toBe('Acme POS');
  });

  // ── Logo section ──────────────────────────────────────────────

  it('renders "Choose Logo" button', async () => {
    render(<AppearanceSettings />);
    await waitFor(() => {
      expect(screen.getByLabelText('Pick logo file')).toBeInTheDocument();
    });
  });

  it('shows logo preview image when logo path is set', async () => {
    mockGetBrandSettings.mockResolvedValue({
      ...defaultBrandResponse,
      logo_path: '/path/to/logo.png',
    });
    render(<AppearanceSettings />);

    await waitFor(() => {
      expect(screen.getByAltText('Store logo')).toBeInTheDocument();
    });
  });

  it('shows logo file path text when logo is set', async () => {
    mockGetBrandSettings.mockResolvedValue({
      ...defaultBrandResponse,
      logo_path: '/path/to/logo.png',
    });
    render(<AppearanceSettings />);

    await waitFor(() => {
      expect(screen.getByText('/path/to/logo.png')).toBeInTheDocument();
    });
  });

  it('calls pickLogoFile when "Choose Logo" button is clicked', async () => {
    const user = userEvent.setup();
    render(<AppearanceSettings />);
    await waitFor(() => {
      expect(screen.getByLabelText('Pick logo file')).toBeInTheDocument();
    });

    await user.click(screen.getByLabelText('Pick logo file'));

    expect(mockPickLogoFile).toHaveBeenCalled();
  });

  it('sets logo path and refreshes brand when pickLogoFile returns a path', async () => {
    const user = userEvent.setup();
    mockPickLogoFile.mockResolvedValue('/new/logo.png');
    render(<AppearanceSettings />);
    await waitFor(() => {
      expect(screen.getByLabelText('Pick logo file')).toBeInTheDocument();
    });

    await user.click(screen.getByLabelText('Pick logo file'));

    await waitFor(() => {
      expect(mockSetBrandLogoPath).toHaveBeenCalledWith('/new/logo.png');
      expect(mockRefreshBrandSettings).toHaveBeenCalled();
    });
  });

  // ── Preview ────────────────────────────────────────────────────

  it('renders preview heading', async () => {
    render(<AppearanceSettings />);
    await waitFor(() => {
      expect(screen.getByText('Preview')).toBeInTheDocument();
    });
  });

  it('shows store name in preview when set', async () => {
    mockGetBrandSettings.mockResolvedValue({
      ...defaultBrandResponse,
      store_name: 'My Preview Store',
    });
    render(<AppearanceSettings />);

    await waitFor(() => {
      expect(screen.getByText('My Preview Store')).toBeInTheDocument();
    });
  });

  it('shows OZ-POS fallback in preview when store name is empty', async () => {
    render(<AppearanceSettings />);
    await waitFor(() => {
      expect(screen.getByText('OZ-POS')).toBeInTheDocument();
    });
  });

  // ── Save button ───────────────────────────────────────────────

  it('renders save button in non-embedded mode', async () => {
    render(<AppearanceSettings />);
    await waitFor(() => {
      expect(screen.getByLabelText('Save appearance')).toBeInTheDocument();
    });
  });

  it('does not render save button in embedded mode', () => {
    render(<AppearanceSettings embedded colour="#ff0000" storeName="Test" />);
    expect(screen.queryByLabelText('Save appearance')).not.toBeInTheDocument();
  });

  it('calls setBrandPrimaryColour and setBrandStoreName on save', async () => {
    const user = userEvent.setup();
    render(<AppearanceSettings />);
    await waitFor(() => {
      expect(screen.getByLabelText('Save appearance')).toBeInTheDocument();
    });

    await user.click(screen.getByLabelText('Save appearance'));

    await waitFor(() => {
      expect(mockSetBrandPrimaryColour).toHaveBeenCalledWith('#10b981');
      expect(mockSetBrandStoreName).toHaveBeenCalledWith('');
      expect(mockRefreshBrandSettings).toHaveBeenCalled();
    });
  });

  it('disables save button while saving', async () => {
    const user = userEvent.setup();
    // Make setBrandPrimaryColour hang so saving stays true.
    mockSetBrandPrimaryColour.mockReturnValue(new Promise(() => {}));
    render(<AppearanceSettings />);
    await waitFor(() => {
      expect(screen.getByLabelText('Save appearance')).toBeInTheDocument();
    });

    await user.click(screen.getByLabelText('Save appearance'));

    expect(screen.getByLabelText('Save appearance')).toBeDisabled();
  });

  it('calls setBrandPrimaryColour and setBrandStoreName with updated values on save', async () => {
    const user = userEvent.setup();
    render(<AppearanceSettings />);
    await waitFor(() => {
      expect(screen.getByLabelText('Colour hex value')).toBeInTheDocument();
    });

    // Change colour.
    const hexInput = screen.getByLabelText('Colour hex value') as HTMLInputElement;
    await user.clear(hexInput);
    await user.type(hexInput, '#aabbcc');

    // Save.
    await user.click(screen.getByLabelText('Save appearance'));

    await waitFor(() => {
      expect(mockSetBrandPrimaryColour).toHaveBeenCalledWith('#aabbcc');
    });
  });

  // ── Embedded mode ──────────────────────────────────────────────

  it('uses colour prop in embedded mode', () => {
    render(<AppearanceSettings embedded colour="#ff0000" storeName="Embedded" />);

    const colourInput = screen.getByLabelText('Primary colour picker') as HTMLInputElement;
    expect(colourInput.value).toBe('#ff0000');
  });

  it('uses storeName prop in embedded mode', () => {
    render(<AppearanceSettings embedded colour="#ff0000" storeName="Embedded Store" />);

    const storeNameInput = document.getElementById('store-name-display') as HTMLInputElement;
    expect(storeNameInput.value).toBe('Embedded Store');
  });

  it('calls onColourChange prop when colour changes in embedded mode', async () => {
    const onColourChange = vi.fn();
    render(
      <AppearanceSettings
        embedded
        colour="#ff0000"
        storeName="Test"
        onColourChange={onColourChange}
      />,
    );

    // Embedded mode is prop-driven — use fireEvent.change to set the exact value.
    fireEvent.change(screen.getByLabelText('Colour hex value'), {
      target: { value: '#00ff00' },
    });

    expect(onColourChange).toHaveBeenCalledWith('#00ff00');
  });

  it('calls onStoreNameChange prop when name changes in embedded mode', async () => {
    const onStoreNameChange = vi.fn();
    render(
      <AppearanceSettings
        embedded
        colour="#ff0000"
        storeName="Test"
        onStoreNameChange={onStoreNameChange}
      />,
    );

    fireEvent.change(document.getElementById('store-name-display')!, {
      target: { value: 'New Name' },
    });

    expect(onStoreNameChange).toHaveBeenCalledWith('New Name');
  });

  // ── Accent palette ─────────────────────────────────────────────

  it('applies accent palette on colour change', async () => {
    const user = userEvent.setup();
    render(<AppearanceSettings />);
    await waitFor(() => {
      expect(screen.getByLabelText('Colour hex value')).toBeInTheDocument();
    });

    const hexInput = screen.getByLabelText('Colour hex value') as HTMLInputElement;
    await user.clear(hexInput);
    await user.type(hexInput, '#334455');

    expect(mockDeriveAccentPalette).toHaveBeenCalledWith('#334455');
    expect(mockApplyAccentPalette).toHaveBeenCalled();
  });

  // ── Edge cases ─────────────────────────────────────────────────

  it('loads brand settings on mount in non-embedded mode', async () => {
    render(<AppearanceSettings />);
    await waitFor(() => {
      expect(mockGetBrandSettings).toHaveBeenCalled();
    });
  });

  it('does not load brand settings in embedded mode', () => {
    render(<AppearanceSettings embedded colour="#ff0000" storeName="Test" />);
    expect(mockGetBrandSettings).not.toHaveBeenCalled();
  });
});
