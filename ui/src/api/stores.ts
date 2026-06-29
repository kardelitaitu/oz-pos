import { invoke } from '@tauri-apps/api/core';

export interface StoreProfile {
  id: string;
  name: string;
  address: string;
  tax_id: string;
  currency: string;
  timezone: string;
  is_primary: boolean;
  created_at: string;
  updated_at: string;
}

export interface CreateStoreArgs {
  id: string;
  name: string;
  address?: string;
  tax_id?: string;
  currency?: string;
  timezone?: string;
}

export interface UpdateStoreArgs {
  id: string;
  name: string;
  address: string;
  tax_id: string;
  currency: string;
  timezone: string;
}

export const listStores = (): Promise<StoreProfile[]> =>
  invoke<StoreProfile[]>('list_store_profiles');

export const getStore = (id: string): Promise<StoreProfile | null> =>
  invoke<StoreProfile | null>('get_store_profile', { id });

export const getPrimaryStore = (): Promise<StoreProfile | null> =>
  invoke<StoreProfile | null>('get_primary_store');

export const createStore = (args: CreateStoreArgs): Promise<StoreProfile> =>
  invoke<StoreProfile>('create_store_profile', { args });

export const updateStore = (args: UpdateStoreArgs): Promise<StoreProfile> =>
  invoke<StoreProfile>('update_store_profile', { args });

export const setPrimaryStore = (id: string): Promise<StoreProfile> =>
  invoke<StoreProfile>('set_primary_store', { id });

export const deleteStore = (id: string): Promise<void> =>
  invoke<void>('delete_store_profile', { id });
