import { invoke } from '@tauri-apps/api/core';

export type LicenseVerificationStatus = 'valid' | 'expired' | 'gracePeriod' | 'invalidSignature' | 'clockTampered' | 'missing';

export interface LicenseStatusDto {
  is_active: boolean;
  status: LicenseVerificationStatus;
  payload: string | null;
  message: string | null;
}

export async function getLicenseStatus(): Promise<LicenseStatusDto> {
  return invoke('get_license_status');
}

export async function getMachineId(): Promise<string> {
  return invoke('get_machine_id');
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
