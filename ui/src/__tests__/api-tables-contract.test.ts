// ── IPC contract tests for tables.ts ───────────────────────────

import { describe, it, expect, vi, beforeEach } from 'vitest';

const { mockInvoke } = vi.hoisted(() => ({
  mockInvoke: vi.fn(),
}));

vi.mock('@/utils/logged-invoke', () => ({
  loggedInvoke: (cmd: string, args?: Record<string, unknown>) => mockInvoke(cmd, args),
}));

import {
  listTables,
  listTablesScoped,
  getTable,
  getTableScoped,
  createTable,
  createTableScoped,
  updateTable,
  updateTableScoped,
  deleteTable,
  deleteTableScoped,
  updateTableStatus,
  updateTableStatusScoped,
} from '@/api/tables';

describe('tables.ts IPC contract', () => {
  beforeEach(() => mockInvoke.mockReset());

  it('listTables → list_tables with section', async () => {
    mockInvoke.mockResolvedValue([]);
    await listTables('indoor');
    expect(mockInvoke).toHaveBeenCalledWith('list_tables', { section: 'indoor' });
  });

  it('listTables without section → list_tables with null section', async () => {
    mockInvoke.mockResolvedValue([]);
    await listTables();
    expect(mockInvoke).toHaveBeenCalledWith('list_tables', { section: null });
  });

  it('listTablesScoped → list_tables_scoped with sessionToken + section', async () => {
    mockInvoke.mockResolvedValue([]);
    await listTablesScoped('tok', 'outdoor');
    expect(mockInvoke).toHaveBeenCalledWith('list_tables_scoped', { sessionToken: 'tok', section: 'outdoor' });
  });

  it('getTable → get_table with id', async () => {
    mockInvoke.mockResolvedValue(null);
    await getTable('t1');
    expect(mockInvoke).toHaveBeenCalledWith('get_table', { id: 't1' });
  });

  it('getTableScoped → get_table_scoped with sessionToken + id', async () => {
    mockInvoke.mockResolvedValue(null);
    await getTableScoped('tok', 't1');
    expect(mockInvoke).toHaveBeenCalledWith('get_table_scoped', { sessionToken: 'tok', id: 't1' });
  });

  it('createTable → create_table with userId + args', async () => {
    mockInvoke.mockResolvedValue({ id: 't1', number: 1, section: 'indoor' });
    await createTable('u1', { number: 1, section: 'indoor', capacity: 4 });
    expect(mockInvoke).toHaveBeenCalledWith('create_table', { userId: 'u1', args: expect.objectContaining({ number: 1 }) });
  });

  it('createTableScoped → create_table_scoped with sessionToken + table', async () => {
    mockInvoke.mockResolvedValue({ id: 't1', number: 2, section: 'outdoor' });
    await createTableScoped('tok', { number: 2, section: 'outdoor', capacity: 2 });
    expect(mockInvoke).toHaveBeenCalledWith('create_table_scoped', { sessionToken: 'tok', table: expect.objectContaining({ number: 2 }) });
  });

  it('updateTable → update_table with userId + table', async () => {
    mockInvoke.mockResolvedValue({ id: 't1', number: 1, capacity: 6 });
    await updateTable('u1', { id: 't1', number: 1, section: 'indoor', capacity: 6 });
    expect(mockInvoke).toHaveBeenCalledWith('update_table', { userId: 'u1', table: expect.objectContaining({ id: 't1' }) });
  });

  it('updateTableScoped → update_table_scoped with sessionToken + table', async () => {
    mockInvoke.mockResolvedValue({ id: 't1', number: 1, capacity: 8 });
    await updateTableScoped('tok', { id: 't1', number: 1, section: 'indoor', capacity: 8 });
    expect(mockInvoke).toHaveBeenCalledWith('update_table_scoped', { sessionToken: 'tok', table: expect.objectContaining({ id: 't1' }) });
  });

  it('deleteTable → delete_table with userId + id', async () => {
    mockInvoke.mockResolvedValue(undefined);
    await deleteTable('u1', 't1');
    expect(mockInvoke).toHaveBeenCalledWith('delete_table', { userId: 'u1', id: 't1' });
  });

  it('deleteTableScoped → delete_table_scoped with sessionToken + id', async () => {
    mockInvoke.mockResolvedValue(undefined);
    await deleteTableScoped('tok', 't1');
    expect(mockInvoke).toHaveBeenCalledWith('delete_table_scoped', { sessionToken: 'tok', id: 't1' });
  });

  it('updateTableStatus → update_table_status with userId + id + status', async () => {
    mockInvoke.mockResolvedValue({ id: 't1', status: 'occupied' });
    await updateTableStatus('u1', 't1', 'occupied');
    expect(mockInvoke).toHaveBeenCalledWith('update_table_status', { userId: 'u1', id: 't1', status: 'occupied' });
  });

  it('updateTableStatusScoped → update_table_status_scoped with sessionToken + id + status', async () => {
    mockInvoke.mockResolvedValue({ id: 't1', status: 'available' });
    await updateTableStatusScoped('tok', 't1', 'available');
    expect(mockInvoke).toHaveBeenCalledWith('update_table_status_scoped', { sessionToken: 'tok', id: 't1', status: 'available' });
  });

  it('propagates backend errors', async () => {
    mockInvoke.mockRejectedValueOnce(new Error('table not found'));
    await expect(getTable('missing')).rejects.toThrow('table not found');
  });
});
