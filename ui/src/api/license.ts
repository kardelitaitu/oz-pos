import { invoke } from '@tauri-apps/api/core';

/** Possible license verification outcomes. */
export type LicenseVerificationStatus = 'valid' | 'expired' | 'gracePeriod' | 'invalidSignature' | 'clockTampered' | 'missing';

/** License verification status returned by the backend. */
export interface LicenseStatusDto {
  is_active: boolean;
  status: LicenseVerificationStatus;
  payload: string | null;
  message: string | null;
}

/** Get the current license activation and verification status. */
export async function getLicenseStatus(): Promise<LicenseStatusDto> {
  return invoke('get_license_status');
}

/** Get the unique machine identifier for device-bound license activation. */
export async function getMachineId(): Promise<string> {
  return invoke('get_machine_id');
}

/** Activate the license with a key, email, phone, and machine identifier. Returns true if activation succeeded. */
export async function activateLicense(
  key: string,
  email: string,
  machineId: string,
  phone: string
): Promise<boolean> {
  return invoke('activate_license', {
    key,
    email,
    machineId,
    phone,
  });
}
