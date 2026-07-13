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

/** Server-authoritative license status (from the license server). */
export interface ServerLicenseStatus {
  tenantId: string;
  status: string;
  tier: string;
  active: boolean;
  expiresAt: string | null;
  graceUntil: string | null;
  maxStores: number;
}

/** Get the current license activation and verification status. */
export async function getLicenseStatus(): Promise<LicenseStatusDto> {
  return invoke('get_license_status');
}

/** Check license status against the PocketBase server for authoritative current state. */
export async function checkLicenseStatus(): Promise<ServerLicenseStatus> {
  return invoke('check_license_status');
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

/** Renew an existing license with a new license key. Returns true if renewal succeeded. */
export async function renewLicense(newKey: string): Promise<boolean> {
  return invoke('renew_license', { newKey });
}
