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
  tenantId: string,
  machineId: string,
  businessName?: string,
  contactName?: string,
  email?: string
): Promise<boolean> {
  return invoke('activate_license', {
    key,
    tenantId,
    machineId,
    businessName: businessName || null,
    contactName: contactName || null,
    email: email || null,
  });
}
