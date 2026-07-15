import { describe, expect, it, vi } from 'vitest';
import { screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { renderWithFluentSync } from '@/__tests__/test-utils/render';
import inventoryFtl from '@/locales/inventory.ftl?raw';

// Mock the API module before importing the component.
vi.mock('@/api/inventoryCounts', () => ({
  createStockCount: vi.fn(),
}));

import StockCountForm from '@/features/inventory/StockCountForm';
import { createStockCount } from '@/api/inventoryCounts';

const scFtl = `
sc-new-count-title = New Stock Count
sc-type-label = Count Type
sc-type-full = Full Inventory
sc-type-cyclic = Cyclic Count
sc-type-spot = Spot Check
sc-type-aria = Select count type
sc-notes-label = Notes (optional)
sc-notes-placeholder = Enter any notes…
sc-error-create = Failed to create count
sc-cancel = Cancel
sc-start-count = Start Count
`;



describe('StockCountForm', () => {
  it('renders the form with title and count type options', () => {
    renderWithFluentSync(<StockCountForm onCreated={vi.fn()} onCancel={vi.fn()} />, inventoryFtl, scFtl);
    expect(screen.getByText('New Stock Count')).toBeInTheDocument();
    expect(screen.getByText('Full Inventory')).toBeInTheDocument();
    expect(screen.getByText('Cyclic Count')).toBeInTheDocument();
    expect(screen.getByText('Spot Check')).toBeInTheDocument();
    expect(screen.getByRole('radiogroup')).toBeInTheDocument();
  });

  it('renders notes textarea and action buttons', () => {
    renderWithFluentSync(<StockCountForm onCreated={vi.fn()} onCancel={vi.fn()} />, inventoryFtl, scFtl);
    expect(screen.getByRole('textbox')).toBeInTheDocument();
    expect(screen.getByText('Cancel')).toBeInTheDocument();
    expect(screen.getByText('Start Count')).toBeInTheDocument();
  });

  it('defaults count type to full', () => {
    renderWithFluentSync(<StockCountForm onCreated={vi.fn()} onCancel={vi.fn()} />, inventoryFtl, scFtl);
    const fullBtn = screen.getByRole('radio', { name: /full inventory/i });
    expect(fullBtn).toHaveAttribute('aria-checked', 'true');
    const cyclicBtn = screen.getByRole('radio', { name: /cyclic count/i });
    expect(cyclicBtn).toHaveAttribute('aria-checked', 'false');
  });

  it('allows selecting a different count type', async () => {
    renderWithFluentSync(<StockCountForm onCreated={vi.fn()} onCancel={vi.fn()} />, inventoryFtl, scFtl);
    await userEvent.click(screen.getByRole('radio', { name: /cyclic count/i }));
    expect(screen.getByRole('radio', { name: /cyclic count/i })).toHaveAttribute('aria-checked', 'true');
    expect(screen.getByRole('radio', { name: /full inventory/i })).toHaveAttribute('aria-checked', 'false');
  });

  it('allows entering notes', async () => {
    renderWithFluentSync(<StockCountForm onCreated={vi.fn()} onCancel={vi.fn()} />, inventoryFtl, scFtl);
    const textarea = screen.getByRole('textbox');
    await userEvent.type(textarea, 'Monday morning count');
    expect(textarea).toHaveValue('Monday morning count');
  });

  it('calls onCancel when cancel is clicked', async () => {
    const onCancel = vi.fn();
    renderWithFluentSync(<StockCountForm onCreated={vi.fn()} onCancel={onCancel} />, inventoryFtl, scFtl);
    await userEvent.click(screen.getByText('Cancel'));
    expect(onCancel).toHaveBeenCalledTimes(1);
  });

  it('calls createStockCount with correct args on submit', async () => {
    const mockCreate = createStockCount as ReturnType<typeof vi.fn>;
    const mockCount = { id: 'count-1', count_number: 'CNT-001', status: 'draft', count_type: 'full' };
    mockCreate.mockResolvedValueOnce(mockCount);
    const onCreated = vi.fn();

    renderWithFluentSync(<StockCountForm onCreated={onCreated} onCancel={vi.fn()} />, inventoryFtl, scFtl);
    await userEvent.click(screen.getByText('Start Count'));

    expect(mockCreate).toHaveBeenCalledTimes(1);
    expect(mockCreate).toHaveBeenCalledWith(
      expect.objectContaining({ countType: 'full' }),
    );

    await vi.waitFor(() => {
      expect(onCreated).toHaveBeenCalledWith(mockCount);
    });
  });

  it('calls createStockCount with selected count type and notes', async () => {
    const mockCreate = createStockCount as ReturnType<typeof vi.fn>;
    mockCreate.mockResolvedValueOnce({ id: 'c2' });
    const onCreated = vi.fn();

    renderWithFluentSync(<StockCountForm onCreated={onCreated} onCancel={vi.fn()} />, inventoryFtl, scFtl);
    await userEvent.click(screen.getByRole('radio', { name: /spot check/i }));
    await userEvent.type(screen.getByRole('textbox'), 'Urgent check');
    await userEvent.click(screen.getByText('Start Count'));

    expect(mockCreate).toHaveBeenCalledWith(
      expect.objectContaining({ countType: 'spot', notes: 'Urgent check' }),
    );
  });

  it('shows error when createStockCount fails', async () => {
    const mockCreate = createStockCount as ReturnType<typeof vi.fn>;
    mockCreate.mockRejectedValueOnce(new Error('Server error'));

    renderWithFluentSync(<StockCountForm onCreated={vi.fn()} onCancel={vi.fn()} />, inventoryFtl, scFtl);
    await userEvent.click(screen.getByText('Start Count'));

    await vi.waitFor(() => {
      expect(screen.getByRole('alert')).toHaveTextContent('Server error');
    });
  });

  it('disables buttons while saving', async () => {
    const mockCreate = createStockCount as ReturnType<typeof vi.fn>;
    // Never resolve — keeps saving=true
    mockCreate.mockReturnValueOnce(new Promise(() => {}));

    renderWithFluentSync(<StockCountForm onCreated={vi.fn()} onCancel={vi.fn()} />, inventoryFtl, scFtl);
    await userEvent.click(screen.getByText('Start Count'));

    // The start button should be disabled while saving.
    await vi.waitFor(() => {
      expect(screen.getByRole('button', { name: /start count/i })).toBeDisabled();
    });
  });
});
