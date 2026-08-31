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

/** List all store profiles for the session's tenant (scoped — ADR #7). */
export const listStoresScoped = (sessionToken: string): Promise<StoreProfile[]> =>
  loggedInvoke<StoreProfile[]>('list_store_profiles_scoped', { sessionToken });

/** Get a single store profile by its identifier (scoped — ADR #7). */
export const getStoreProfileScoped = (sessionToken: string, id: string): Promise<StoreProfile | null> =>
  loggedInvoke<StoreProfile | null>('get_store_profile_scoped', { sessionToken, id });

/** Get the primary store profile (scoped — ADR #7). */
export const getPrimaryStoreScoped = (sessionToken: string): Promise<StoreProfile | null> =>
  loggedInvoke<StoreProfile | null>('get_primary_store_scoped', { sessionToken });

/** Create a new store profile (scoped — ADR #7). */
export const createStoreProfileScoped = (sessionToken: string, args: CreateStoreArgs): Promise<StoreProfile> =>
  loggedInvoke<StoreProfile>('create_store_profile_scoped', { sessionToken, args });

/** Update an existing store profile (scoped — ADR #7). */
export const updateStoreProfileScoped = (sessionToken: string, args: UpdateStoreArgs): Promise<StoreProfile> =>
  loggedInvoke<StoreProfile>('update_store_profile_scoped', { sessionToken, args });

/** Set a store as the primary store (scoped — ADR #7). */
export const setPrimaryStoreScoped = (sessionToken: string, id: string): Promise<StoreProfile> =>
  loggedInvoke<StoreProfile>('set_primary_store_scoped', { sessionToken, id });

/** Delete a store profile by its identifier (scoped — ADR #7). */
export const deleteStoreProfileScoped = (sessionToken: string, id: string): Promise<void> =>
  loggedInvoke<void>('delete_store_profile_scoped', { sessionToken, id });
