import { invoke } from '@tauri-apps/api/core';

export interface Table {
  id: string;
  name: string;
  capacity: number;
  pos_x: number;
  pos_y: number;
  shape: string;
  width: number;
  height: number;
  status: string;
  active_sale_id: string | null;
  section: string;
  active: boolean;
  sort_order: number;
}

export const listTables = (section?: string) =>
  invoke<Table[]>('list_tables', { section: section ?? null });

/** List tables (scoped — ADR #7). */
export const listTablesScoped = (sessionToken: string, section?: string) =>
  invoke<Table[]>('list_tables_scoped', { sessionToken, section: section ?? null });

export const getTable = (id: string) =>
  invoke<Table | null>('get_table', { id });

/** Get a table (scoped — ADR #7). */
export const getTableScoped = (sessionToken: string, id: string) =>
  invoke<Table | null>('get_table_scoped', { sessionToken, id });

export const createTable = (userId: string, args: Table) =>
  invoke<Table>('create_table', { userId, args });

/** Create a table (scoped — ADR #7). */
export const createTableScoped = (sessionToken: string, table: Table) =>
  invoke<Table>('create_table_scoped', { sessionToken, table });

export const updateTable = (userId: string, table: Table) =>
  invoke<Table>('update_table', { userId, table });

/** Update a table (scoped — ADR #7). */
export const updateTableScoped = (sessionToken: string, table: Table) =>
  invoke<Table>('update_table_scoped', { sessionToken, table });

export const deleteTable = (userId: string, id: string) =>
  invoke<void>('delete_table', { userId, id });

/** Delete a table (scoped — ADR #7). */
export const deleteTableScoped = (sessionToken: string, id: string) =>
  invoke<void>('delete_table_scoped', { sessionToken, id });

export const updateTableStatus = (userId: string, id: string, status: string) =>
  invoke<Table>('update_table_status', { userId, id, status });

/** Update table status (scoped — ADR #7). */
export const updateTableStatusScoped = (sessionToken: string, id: string, status: string) =>
  invoke<Table>('update_table_status_scoped', { sessionToken, id, status });

export const assignTableOrder = (userId: string, tableId: string, saleId: string) =>
  invoke<Table>('assign_table_order', { userId, tableId, saleId });

/** Assign order to table (scoped — ADR #7). */
export const assignTableOrderScoped = (sessionToken: string, tableId: string, saleId: string) =>
  invoke<Table>('assign_table_order_scoped', { sessionToken, tableId, saleId });

export const releaseTable = (userId: string, tableId: string) =>
  invoke<Table>('release_table', { userId, tableId });

/** Release a table (scoped — ADR #7). */
export const releaseTableScoped = (sessionToken: string, tableId: string) =>
  invoke<Table>('release_table_scoped', { sessionToken, tableId });

export const listSections = () =>
  invoke<string[]>('list_sections');

/** List sections (scoped — ADR #7). */
export const listSectionsScoped = (sessionToken: string) =>
  invoke<string[]>('list_sections_scoped', { sessionToken });
