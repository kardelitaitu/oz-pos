import { describe, it, expect, vi, beforeEach } from 'vitest';

const mockInvoke = vi.fn();
vi.mock('@/utils/logged-invoke', () => ({
  loggedInvoke: (...args: unknown[]) => mockInvoke(...args),
}));

import {
  listTables,
  listTablesScoped,
  createTable,
  createTableScoped,
  updateTable,
  deleteTable,
  updateTableStatus,
  updateTableStatusScoped,
} from '@/api/tables';

describe('tables.ts API contract', () => {
  const TOKEN = 'tok_tables';
  const USER_ID = 'u1';

  beforeEach(() => {
    vi.clearAllMocks();
  });

  const table = { id: 't1', name: 'Table 1', capacity: 4, pos_x: 100, pos_y: 200, shape: 'rect', width: 80, height: 60, status: 'available', active_sale_id: null };

  it('listTables calls correct command (no args)', async () => {
    mockInvoke.mockResolvedValue([]);
    await listTables();
    expect(mockInvoke).toHaveBeenCalledWith('list_tables');
  });

  it('listTablesScoped calls correct command', async () => {
    mockInvoke.mockResolvedValue([]);
    await listTablesScoped(TOKEN);
    expect(mockInvoke).toHaveBeenCalledWith('list_tables_scoped', { sessionToken: TOKEN });
  });

  it('createTable calls correct command', async () => {
    mockInvoke.mockResolvedValue(table);
    const result = await createTable(USER_ID, table);
    expect(mockInvoke).toHaveBeenCalledWith('create_table', { userId: USER_ID, table });
    expect(result.id).toBe('t1');
  });

  it('createTableScoped calls correct command', async () => {
    mockInvoke.mockResolvedValue(table);
    await createTableScoped(TOKEN, table);
    expect(mockInvoke).toHaveBeenCalledWith('create_table_scoped', { sessionToken: TOKEN, table });
  });

  it('updateTable calls correct command', async () => {
    mockInvoke.mockResolvedValue(table);
    await updateTable(USER_ID, table);
    expect(mockInvoke).toHaveBeenCalledWith('update_table', { userId: USER_ID, table });
  });

  it('deleteTable calls correct command', async () => {
    mockInvoke.mockResolvedValue(undefined);
    await deleteTable(USER_ID, 't1');
    expect(mockInvoke).toHaveBeenCalledWith('delete_table', { userId: USER_ID, id: 't1' });
  });

  it('updateTableStatus calls correct command', async () => {
    mockInvoke.mockResolvedValue(undefined);
    await updateTableStatus(USER_ID, 't1', 'occupied');
    expect(mockInvoke).toHaveBeenCalledWith('update_table_status', { userId: USER_ID, id: 't1', status: 'occupied' });
  });

  it('updateTableStatusScoped calls correct command', async () => {
    mockInvoke.mockResolvedValue(undefined);
    await updateTableStatusScoped(TOKEN, 't1', 'cleaning');
    expect(mockInvoke).toHaveBeenCalledWith('update_table_status_scoped', { sessionToken: TOKEN, id: 't1', status: 'cleaning' });
  });

  it('propagates errors', async () => {
    mockInvoke.mockRejectedValue(new Error('table in use'));
    await expect(updateTableStatus(USER_ID, 't1', 'occupied')).rejects.toThrow('table in use');
  });
});
