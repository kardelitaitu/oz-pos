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

export const createTable = (args: Table) =>
  invoke<Table>('create_table', { args });

export const updateTable = (table: Table) =>
  invoke<Table>('update_table', { table });

export const deleteTable = (id: string) =>
  invoke<void>('delete_table', { id });

export const updateTableStatus = (id: string, status: string) =>
  invoke<Table>('update_table_status', { id, status });

export const assignTableOrder = (tableId: string, saleId: string) =>
  invoke<Table>('assign_table_order', { tableId, saleId });

export const releaseTable = (tableId: string) =>
  invoke<Table>('release_table', { tableId });

export const listSections = () =>
  invoke<string[]>('list_sections');
