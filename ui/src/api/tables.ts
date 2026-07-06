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

export const getTable = (id: string) =>
  invoke<Table | null>('get_table', { id });

export const createTable = (userId: string, args: Table) =>
  invoke<Table>('create_table', { userId, args });

export const updateTable = (userId: string, table: Table) =>
  invoke<Table>('update_table', { userId, table });

export const deleteTable = (userId: string, id: string) =>
  invoke<void>('delete_table', { userId, id });

export const updateTableStatus = (userId: string, id: string, status: string) =>
  invoke<Table>('update_table_status', { userId, id, status });

export const assignTableOrder = (userId: string, tableId: string, saleId: string) =>
  invoke<Table>('assign_table_order', { userId, tableId, saleId });

export const releaseTable = (userId: string, tableId: string) =>
  invoke<Table>('release_table', { userId, tableId });

export const listSections = () =>
  invoke<string[]>('list_sections');
