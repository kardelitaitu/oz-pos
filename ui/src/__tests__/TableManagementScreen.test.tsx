import { describe, it, expect, vi, beforeEach } from 'vitest';
import { screen, waitFor } from '@testing-library/react';
import { renderWithWorkspace, MOCK_SESSION_TOKEN } from '@/test-utils';
import userEvent from '@testing-library/user-event';
import { FluentBundle, FluentResource } from '@fluent/bundle';
import { ReactLocalization, LocalizationProvider } from '@fluent/react';
import TableManagementScreen from '@/features/tables/TableManagementScreen';
import tablesFtl from '@/locales/tables.ftl?raw';
import type { Table } from '@/api/tables';

const { mockListTables, mockListSections, mockUpdateTableStatus, mockReleaseTable } = vi.hoisted(() => ({
  mockListTables: vi.fn(),
  mockListSections: vi.fn(),
  mockUpdateTableStatus: vi.fn(),
  mockReleaseTable: vi.fn(),
}));

vi.mock('@/api/tables', () => ({
  listTables: (section?: string) => mockListTables(section),
  listTablesScoped: (_token: string, section?: string) => mockListTables(section),
  listSections: () => mockListSections(),
  listSectionsScoped: (_token: string) => mockListSections(),
  updateTableStatus: (userId: string, id: string, status: string) =>
    mockUpdateTableStatus(userId, id, status),
  updateTableStatusScoped: (_token: string, id: string, status: string) =>
    mockUpdateTableStatus(_token, id, status),
  releaseTable: (userId: string, id: string) => mockReleaseTable(userId, id),
  releaseTableScoped: (_token: string, id: string) => mockReleaseTable(_token, id),
}));

vi.mock('@/contexts/AuthContext', () => ({
  useAuth: () => ({ session: { user_id: 'user-1' } }),
}));

const bundle = new FluentBundle('en-US');
bundle.addResource(new FluentResource(tablesFtl));
const l10n = new ReactLocalization([bundle]);

function renderScreen() {
  return renderWithWorkspace(
    <LocalizationProvider l10n={l10n}>
      <TableManagementScreen />
    </LocalizationProvider>,
  );
}

function makeTable(overrides: Partial<Table> = {}): Table {
  return {
    id: 't-1',
    name: 'Table 1',
    capacity: 4,
    pos_x: 10,
    pos_y: 20,
    shape: 'circle',
    width: 8,
    height: 8,
    status: 'available',
    active_sale_id: null,
    section: 'Main',
    active: true,
    sort_order: 1,
    ...overrides,
  };
}

describe('TableManagementScreen', () => {
  beforeEach(() => {
    mockListTables.mockResolvedValue([]);
    mockListSections.mockResolvedValue([]);
    mockUpdateTableStatus.mockResolvedValue({});
    mockReleaseTable.mockResolvedValue({});
  });

  it('renders the title', async () => {
    renderScreen();
    await waitFor(() => expect(screen.getByText('Table Management')).toBeDefined());
  });

  it('shows All section button', async () => {
    renderScreen();
    await waitFor(() => expect(screen.getByText('All')).toBeDefined());
  });

  it('shows section buttons from the sections API (TBL-09)', async () => {
    mockListSections.mockResolvedValue(['Main', 'Patio']);
    renderScreen();
    await waitFor(() => {
      expect(screen.getByText('Main')).toBeDefined();
      expect(screen.getByText('Patio')).toBeDefined();
    });
  });

  it('keeps section buttons stable while a section is selected (TBL-09)', async () => {
    mockListSections.mockResolvedValue(['Main', 'Patio']);
    mockListTables.mockResolvedValue([makeTable({ section: 'Patio' })]);
    renderScreen();
    await waitFor(() => expect(screen.getByText('Patio')).toBeDefined());
    // Selecting Patio must not make Main disappear (sections come from
    // independent metadata, not from the filtered table page).
    await userEvent.click(screen.getByText('Patio'));
    await waitFor(() => {
      expect(screen.getByText('Main')).toBeDefined();
      expect(screen.getByText('Patio')).toBeDefined();
    });
    // listTablesScoped maps to mockListTables(section) — the token is
    // stripped inside the vi.mock, so only the section is passed through.
    expect(mockListTables).toHaveBeenLastCalledWith('Patio');
  });

  it('shows the loading state while the first request is in flight (TBL-10)', async () => {
    let resolveTables: (v: Table[]) => void;
    mockListTables.mockReturnValue(new Promise((res) => { resolveTables = res; }));
    renderScreen();
    expect(screen.getByRole('status')).toBeDefined();
    resolveTables!([]);
    await waitFor(() => expect(screen.queryByRole('status')).toBeNull());
  });

  it('shows the empty state after a successful zero-row load (TBL-10)', async () => {
    mockListTables.mockResolvedValue([]);
    renderScreen();
    await waitFor(() =>
      expect(screen.getByText('No tables configured yet.')).toBeDefined(),
    );
  });

  it('shows the filtered-empty state with an All action (TBL-10)', async () => {
    mockListSections.mockResolvedValue(['Main']);
    mockListTables.mockResolvedValue([]);
    renderScreen();
    await waitFor(() => expect(screen.getByText('Main')).toBeDefined());
    await userEvent.click(screen.getByText('Main'));
    await waitFor(() =>
      expect(screen.getByText('No tables in this section.')).toBeDefined(),
    );
  });

  it('shows a localized error and retries after a failed load (TBL-02)', async () => {
    mockListTables.mockRejectedValueOnce(new Error('db down'));
    renderScreen();
    await waitFor(() => expect(screen.getByRole('alert')).toBeDefined());
    expect(screen.getByText('Could not load the floor plan.')).toBeDefined();

    mockListTables.mockResolvedValueOnce([makeTable()]);
    await userEvent.click(screen.getByText('Retry'));
    await waitFor(() => expect(screen.getByText('Table 1')).toBeDefined());
    expect(screen.queryByRole('alert')).toBeNull();
  });

  it('drops a stale response that resolves after a newer request (TBL-02)', async () => {
    mockListSections.mockResolvedValue(['Main']);
    let resolveFirst: (v: Table[]) => void;
    // First request (mount, section undefined) stays pending.
    mockListTables.mockReturnValueOnce(new Promise((res) => { resolveFirst = res; }));
    renderScreen();
    await waitFor(() => expect(screen.getByText('Main')).toBeDefined());
    // Selecting a section fires a second request, which resolves first.
    mockListTables.mockResolvedValueOnce([makeTable({ id: 't-b', name: 'Table B' })]);
    await userEvent.click(screen.getByText('Main'));
    await waitFor(() => expect(screen.getByText('Table B')).toBeDefined());
    // The stale first response must be ignored, not overwrite Table B.
    resolveFirst!([makeTable({ id: 't-a', name: 'Table A' })]);
    await waitFor(() => expect(screen.queryByText('Table A')).toBeNull());
    expect(screen.getByText('Table B')).toBeDefined();
  });

  it('renders tables on the floor plan', async () => {
    mockListTables.mockResolvedValue([makeTable()]);
    renderScreen();
    await waitFor(() => expect(screen.getByText('Table 1')).toBeDefined());
  });

  it('shows table status on each table button', async () => {
    mockListTables.mockResolvedValue([makeTable({ status: 'occupied' })]);
    renderScreen();
    await waitFor(() => expect(screen.getByText('occupied')).toBeDefined());
  });

  it('uses absolute positioning from pos_x/pos_y props', async () => {
    mockListTables.mockResolvedValue([makeTable({ pos_x: 25, pos_y: 50 })]);
    renderScreen();
    await waitFor(() => {
      const btns = document.querySelectorAll('.tables-table');
      expect(btns.length).toBe(1);
      const style = (btns[0] as HTMLElement).style;
      expect(style.left).toBe('25%');
      expect(style.top).toBe('50%');
    });
  });

  it('applies status CSS class to table buttons', async () => {
    mockListTables.mockResolvedValue([
      makeTable({ id: 't-1', status: 'available' }),
      makeTable({ id: 't-2', status: 'occupied' }),
    ]);
    renderScreen();
    await waitFor(() => {
      const availableBtn = document.querySelector('.tables-table--available');
      const occupiedBtn = document.querySelector('.tables-table--occupied');
      expect(availableBtn).toBeDefined();
      expect(occupiedBtn).toBeDefined();
    });
  });

  it('applies shape CSS class', async () => {
    mockListTables.mockResolvedValue([
      makeTable({ shape: 'circle' }),
      makeTable({ id: 't-2', shape: 'rectangle' }),
    ]);
    renderScreen();
    await waitFor(() => {
      const circleBtn = document.querySelector('.tables-table--circle');
      const rectBtn = document.querySelector('.tables-table--rectangle');
      expect(circleBtn).toBeDefined();
      expect(rectBtn).toBeDefined();
    });
  });

  it('opens detail panel on table click', async () => {
    mockListTables.mockResolvedValue([makeTable()]);
    renderScreen();
    await waitFor(() => expect(screen.getByText('Table 1')).toBeDefined());

    const tableBtn = screen.getByText('Table 1').closest('button')!;
    await userEvent.click(tableBtn);

    await waitFor(() => {
      // Detail panel shows table name as h2
      const detailHeading = document.querySelector('.tables-detail h2');
      expect(detailHeading?.textContent).toBe('Table 1');
    });
  });

  it('shows Mark Available button for available tables in detail', async () => {
    mockListTables.mockResolvedValue([makeTable({ status: 'available' })]);
    renderScreen();
    await waitFor(() => expect(screen.getByText('Table 1')).toBeDefined());

    await userEvent.click(screen.getByText('Table 1').closest('button')!);

    await waitFor(() => expect(screen.getByText('Mark Available')).toBeDefined());
  });

  it('shows Release button for occupied tables in detail', async () => {
    mockListTables.mockResolvedValue([makeTable({ status: 'occupied' })]);
    renderScreen();
    await waitFor(() => expect(screen.getByText('Table 1')).toBeDefined());

    await userEvent.click(screen.getByText('Table 1').closest('button')!);

    await waitFor(() => {
      const releaseBtn = screen.getByText('Release');
      expect(releaseBtn).toBeDefined();
    });
  });

  it('dismisses detail panel on Close click', async () => {
    mockListTables.mockResolvedValue([makeTable()]);
    renderScreen();
    await waitFor(() => expect(screen.getByText('Table 1')).toBeDefined());

    await userEvent.click(screen.getByText('Table 1').closest('button')!);

    await waitFor(() => expect(screen.getByText('Close')).toBeDefined());

    const closeBtn = screen.getByText('Close').closest('button')!;
    await userEvent.click(closeBtn);

    await waitFor(() =>
      expect(document.querySelector('.tables-detail')).toBeNull(),
    );
  });

  it('calls updateTableStatus on context menu for available tables', async () => {
    mockListTables.mockResolvedValue([makeTable({ status: 'available' })]);
    renderScreen();
    await waitFor(() => expect(screen.getByText('Table 1')).toBeDefined());

    const tableBtn = screen.getByText('Table 1').closest('button')!;
    // Right-click triggers context menu → statusAction
    await userEvent.pointer({ keys: '[MouseRight]', target: tableBtn });

    await waitFor(() =>
      expect(mockUpdateTableStatus).toHaveBeenCalledWith(MOCK_SESSION_TOKEN, 't-1', 'occupied'),
    );
  });

  it('calls releaseTable on context menu for occupied tables', async () => {
    mockListTables.mockResolvedValue([makeTable({ status: 'occupied' })]);
    renderScreen();
    await waitFor(() => expect(screen.getByText('Table 1')).toBeDefined());

    const tableBtn = screen.getByText('Table 1').closest('button')!;
    await userEvent.pointer({ keys: '[MouseRight]', target: tableBtn });

    await waitFor(() =>
      expect(mockReleaseTable).toHaveBeenCalledWith(MOCK_SESSION_TOKEN, 't-1'),
    );
  });

  it('calls updateTableStatus to available for reserved tables via context menu', async () => {
    mockListTables.mockResolvedValue([makeTable({ status: 'reserved' })]);
    renderScreen();
    await waitFor(() => expect(screen.getByText('Table 1')).toBeDefined());

    const tableBtn = screen.getByText('Table 1').closest('button')!;
    await userEvent.pointer({ keys: '[MouseRight]', target: tableBtn });

    await waitFor(() =>
      expect(mockUpdateTableStatus).toHaveBeenCalledWith(MOCK_SESSION_TOKEN, 't-1', 'available'),
    );
  });

  it('has region role with accessible label', async () => {
    renderScreen();
    await waitFor(() =>
      expect(screen.getByRole('region', { name: 'Table management' })).toBeDefined(),
    );
  });

  it('shows capacity and status in detail panel', async () => {
    mockListTables.mockResolvedValue([makeTable({ capacity: 6, status: 'reserved', section: 'Patio' })]);
    renderScreen();
    await waitFor(() => expect(screen.getByText('Table 1')).toBeDefined());

    await userEvent.click(screen.getByText('Table 1').closest('button')!);

    // The detail panel shows "Capacity: 6", "Status: reserved", "Section: Patio"
    // via Fluent Localized vars; use DOM query since Fluent wraps in <span>
    await waitFor(() => {
      const detail = document.querySelector('.tables-detail');
      expect(detail?.textContent).toMatch(/6/);
      expect(detail?.textContent).toMatch(/reserved/);
      expect(detail?.textContent).toMatch(/Patio/);
    });
  });

  it('calls updateTableStatus to available for cleaning tables via context menu', async () => {
    mockListTables.mockResolvedValue([makeTable({ status: 'cleaning' })]);
    renderScreen();
    await waitFor(() => expect(screen.getByText('Table 1')).toBeDefined());

    const tableBtn = screen.getByText('Table 1').closest('button')!;
    await userEvent.pointer({ keys: '[MouseRight]', target: tableBtn });

    await waitFor(() =>
      expect(mockUpdateTableStatus).toHaveBeenCalledWith(MOCK_SESSION_TOKEN, 't-1', 'available'),
    );
  });
});
