import { loggedInvoke } from '@/utils/logged-invoke';

/** A store profile with location and configuration info. */
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

/** Arguments for creating a new store profile. */
export interface CreateStoreArgs {
  id: string;
  name: string;
  address?: string;
  tax_id?: string;
  currency?: string;
  timezone?: string;
}

/** Arguments for updating an existing store profile. */
export interface UpdateStoreArgs {
  id: string;
  name: string;
  address: string;
  tax_id: string;
  currency: string;
  timezone: string;
}

/** List all store profiles. */
export const listStores = (): Promise<StoreProfile[]> =>
  loggedInvoke<StoreProfile[]>('list_store_profiles');

/** Get a single store profile by its identifier. */
export const getStore = (id: string): Promise<StoreProfile | null> =>
  loggedInvoke<StoreProfile | null>('get_store_profile', { id });

/** Get the primary store profile. */
export const getPrimaryStore = (): Promise<StoreProfile | null> =>
  loggedInvoke<StoreProfile | null>('get_primary_store');

/** Create a new store profile. */
export const createStore = (args: CreateStoreArgs): Promise<StoreProfile> =>
  loggedInvoke<StoreProfile>('create_store_profile', { args });

/** Update an existing store profile. */
export const updateStore = (args: UpdateStoreArgs): Promise<StoreProfile> =>
  loggedInvoke<StoreProfile>('update_store_profile', { args });

/** Set a store as the primary store. */
export const setPrimaryStore = (id: string): Promise<StoreProfile> =>
  loggedInvoke<StoreProfile>('set_primary_store', { id });

/** Delete a store profile by its identifier. */
export const deleteStore = (id: string): Promise<void> =>
  loggedInvoke<void>('delete_store_profile', { id });
