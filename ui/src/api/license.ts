import { invoke } from '@tauri-apps/api/core';

export interface LicenseStatusDto {
  is_active: boolean;
  payload: string | null;
}

export async function getLicenseStatus(): Promise<LicenseStatusDto> {
  return invoke('get_license_status');
}

export async function activateLicense(
  key: string,
  email: string,
  machineId: string
): Promise<boolean> {
  return invoke('activate_license', {
    key,
    email,
    machineId,
  });
}
