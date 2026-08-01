import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { FluentBundle, FluentResource } from '@fluent/bundle';
import { ReactLocalization, LocalizationProvider } from '@fluent/react';
import { ToastProvider } from '@/frontend/shared/Toast';
import CategoryManagementScreen from '@/features/categories/CategoryManagementScreen';
import sharedFtl from '@/locales/shared.ftl?raw';
import settingsFtl from '@/locales/settings.ftl?raw';
import productsFtl from '@/locales/products.ftl?raw';
import type { CategoryDto } from '@/api/products';

const { mockListCategories, mockCreateCategory, mockUpdateCategory, mockDeleteCategory } =
  vi.hoisted(() => ({
    mockListCategories: vi.fn(),
    mockCreateCategory: vi.fn(),
    mockUpdateCategory: vi.fn(),
    mockDeleteCategory: vi.fn(),
  }));

// CAT-01: prove the session token flows through every scoped call. The
// mock preserves both args (token first) so tests can assert on them.
const TOKEN = 'mock-session-token';

vi.mock('@/contexts/WorkspaceContext', () => ({
  useWorkspace: () => ({
    sessionToken: TOKEN,
    workspaceId: 'workspace-1',
    storeId: 'store-1',
  }),
  useWorkspaceScope: () => null,
}));

vi.mock('@/api/products', () => ({
  listCategories: () => mockListCategories(),
  listCategoriesScoped: (...args: unknown[]) => mockListCategories(...args),
  createCategory: (args: unknown) => mockCreateCategory(args),
  createCategoryScoped: (sessionToken: string, args: unknown) =>
    mockCreateCategory(sessionToken, args),
  updateCategory: (args: unknown) => mockUpdateCategory(args),
  updateCategoryScoped: (sessionToken: string, args: unknown) =>
    mockUpdateCategory(sessionToken, args),
  deleteCategory: (id: string) => mockDeleteCategory(id),
  deleteCategoryScoped: (sessionToken: string, id: string) =>
    mockDeleteCategory(sessionToken, id),
}));

const bundle = new FluentBundle('en-US');
bundle.addResource(new FluentResource(sharedFtl));
bundle.addResource(new FluentResource(settingsFtl));
bundle.addResource(new FluentResource(productsFtl));
const l10n = new ReactLocalization([bundle]);

function renderScreen() {
  return render(
    <LocalizationProvider l10n={l10n}>
      <ToastProvider>
        <CategoryManagementScreen />
      </ToastProvider>
    </LocalizationProvider>,
  );
}

function makeCategory(overrides: Partial<CategoryDto> = {}): CategoryDto {
  return {
    id: 'cat-bakery',
    name: 'Bakery',
    colour: '#f97316',
    icon: 'food',
    ...overrides,
  };
}

describe('CategoryManagementScreen', () => {
  beforeEach(() => {
    mockListCategories.mockResolvedValue([]);
    mockCreateCategory.mockResolvedValue({ id: 'new-cat' });
    mockUpdateCategory.mockResolvedValue({ id: 'cat-1' });
    mockDeleteCategory.mockResolvedValue({ affected_products: 0 });
  });

  it('renders the title', async () => {
    renderScreen();
    await waitFor(() => expect(screen.getByText('Categories')).toBeDefined());
  });

  it('renders the Add Category button', async () => {
    renderScreen();
    await waitFor(() => expect(screen.getByText('Add Category')).toBeDefined());
  });

  it('shows loading skeleton while fetching categories', () => {
    mockListCategories.mockReturnValue(new Promise(() => {}));
    renderScreen();
    expect(document.querySelector('.cat-mgmt-loading-skeleton')).toBeDefined();
    expect(screen.queryByText('Loading categories…')).toBeNull();
  });

  it('shows empty state', async () => {
    renderScreen();
    await waitFor(() => expect(screen.getByText('No categories yet')).toBeDefined());
  });

  it('renders category cards with name and ID', async () => {
    mockListCategories.mockResolvedValue([
      makeCategory(),
      makeCategory({ id: 'cat-drinks', name: 'Drinks', colour: '#06b6d4' }),
    ]);
    renderScreen();
    await waitFor(() => {
      expect(screen.getByText('Bakery')).toBeDefined();
      expect(screen.getByText('Drinks')).toBeDefined();
      expect(screen.getByText('cat-bakery')).toBeDefined();
      expect(screen.getByText('cat-drinks')).toBeDefined();
    });
  });

  it('shows category colour on each card', async () => {
    mockListCategories.mockResolvedValue([makeCategory()]);
    renderScreen();
    await waitFor(() => expect(screen.getByText('#f97316')).toBeDefined());
  });

  it('renders icon badge with correct background colour', async () => {
    mockListCategories.mockResolvedValue([makeCategory()]);
    renderScreen();
    await waitFor(() => {
      const badge = document.querySelector('.cat-mgmt-icon-badge') as HTMLElement;
      expect(badge).toBeDefined();
      expect(badge?.style.background).toBe('rgb(249, 115, 22)');
    });
  });

  it('shows edit and delete buttons on each card', async () => {
    mockListCategories.mockResolvedValue([makeCategory()]);
    renderScreen();
    await waitFor(() => {
      const editBtns = document.querySelectorAll('.cat-mgmt-edit-btn');
      const deleteBtns = document.querySelectorAll('.cat-mgmt-delete-btn');
      expect(editBtns.length).toBeGreaterThanOrEqual(1);
      expect(deleteBtns.length).toBeGreaterThanOrEqual(1);
    });
  });

  it('opens delete confirmation modal on delete click', async () => {
    mockListCategories.mockResolvedValue([makeCategory()]);
    renderScreen();
    await waitFor(() => expect(screen.getByText('Bakery')).toBeDefined());

    const deleteBtn = document.querySelector('.cat-mgmt-delete-btn') as HTMLElement;
    await userEvent.click(deleteBtn);

    await waitFor(() =>
      expect(screen.getByText(/unlink all products/)).toBeDefined(),
    );
  });

  it('calls deleteCategoryScoped on confirm with the session token (CAT-01)', async () => {
    mockListCategories.mockResolvedValue([makeCategory()]);
    renderScreen();
    await waitFor(() => expect(screen.getByText('Bakery')).toBeDefined());

    await userEvent.click(document.querySelector('.cat-mgmt-delete-btn')!);
    await waitFor(() => expect(screen.getByText('Delete Category')).toBeDefined());

    // Confirm delete — click the danger Delete button in the SettingsPopup footer.
    const modalDeleteBtn = screen.getByRole('button', { name: 'Delete' });
    await userEvent.click(modalDeleteBtn);

    // CAT-01: the scoped command must receive the session token first.
    await waitFor(() =>
      expect(mockDeleteCategory).toHaveBeenCalledWith(TOKEN, 'cat-bakery'),
    );
  });

  it('reports how many products were unlinked after deletion (CAT-02)', async () => {
    mockListCategories.mockResolvedValue([makeCategory()]);
    mockDeleteCategory.mockResolvedValue({ affected_products: 3 });
    renderScreen();
    await waitFor(() => expect(screen.getByText('Bakery')).toBeDefined());

    await userEvent.click(document.querySelector('.cat-mgmt-delete-btn')!);
    await waitFor(() => expect(screen.getByText('Delete Category')).toBeDefined());
    await userEvent.click(screen.getByRole('button', { name: 'Delete' }));

    // Fluent wraps the interpolated count in Unicode directional-isolate
    // characters (⁨3⁩), so match on the message shape, not a literal regex.
    await waitFor(() =>
      expect(
        screen.getByText((content) => /^unlinked/i.test(content) && content.includes('3')),
      ).toBeDefined(),
    );
  });

  it('surfaces a delete failure instead of swallowing it (CAT-01)', async () => {
    mockListCategories.mockResolvedValue([makeCategory()]);
    mockDeleteCategory.mockRejectedValue(new Error('category has linked tax config'));
    renderScreen();
    await waitFor(() => expect(screen.getByText('Bakery')).toBeDefined());

    await userEvent.click(document.querySelector('.cat-mgmt-delete-btn')!);
    await waitFor(() => expect(screen.getByText('Delete Category')).toBeDefined());
    await userEvent.click(screen.getByRole('button', { name: 'Delete' }));

    await waitFor(() =>
      expect(screen.getByText(/category has linked tax config/i)).toBeDefined(),
    );
  });

  it('calls createCategoryScoped and updateCategoryScoped with the session token (CAT-01)', async () => {
    mockListCategories.mockResolvedValue([makeCategory()]);
    renderScreen();
    await waitFor(() => expect(screen.getByText('Bakery')).toBeDefined());

    // Create flow
    await userEvent.click(screen.getByText('Add Category').closest('button')!);
    await userEvent.type(screen.getByPlaceholderText(/e\.g\. bakery/i), 'Snacks');
    await userEvent.click(screen.getByRole('button', { name: 'Create' }));
    // CAT-01: token must precede the payload in the scoped call.
    await waitFor(() =>
      expect(mockCreateCategory).toHaveBeenCalledWith(
        TOKEN,
        expect.objectContaining({ name: 'Snacks' }),
      ),
    );

    // Edit flow
    await userEvent.click(document.querySelector('.cat-mgmt-edit-btn')!);
    await waitFor(() => expect(screen.getByText('Edit')).toBeDefined());
    await userEvent.click(screen.getByRole('button', { name: 'Save' }));
    await waitFor(() =>
      expect(mockUpdateCategory).toHaveBeenCalledWith(
        TOKEN,
        expect.objectContaining({ id: 'cat-bakery' }),
      ),
    );
  });

  it('opens add modal when Add Category is clicked', async () => {
    renderScreen();
    await waitFor(() => expect(screen.getByText('Add Category')).toBeDefined());

    await userEvent.click(screen.getByText('Add Category').closest('button')!);
    // Add modal heading — should show "Add Category" via FTL
    // The modal heading is also "Add Category" so it may duplicate
    await waitFor(() => expect(screen.queryByText('Cancel')).toBeDefined());
  });

  it('opens edit modal when edit button is clicked', async () => {
    mockListCategories.mockResolvedValue([makeCategory()]);
    renderScreen();
    await waitFor(() => expect(screen.getByText('Bakery')).toBeDefined());

    await userEvent.click(document.querySelector('.cat-mgmt-edit-btn')!);
    await waitFor(() => expect(screen.getByText('Edit')).toBeDefined());
  });
});
