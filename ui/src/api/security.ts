// ── Security: Key Rotation & Age ──────────────────────────────────

import { invoke } from '@tauri-apps/api/core';

/** Metadata about the current encryption key (no key material exposed). */
export interface KeyRotationStatus {
  /** Whether a key exists in the OS keyring. */
  hasKey: boolean;
  /** ISO 8601 timestamp of when the current key was created. */
  createdAt: string | null;
  /** Number of days since key creation (null if unknown). */
  ageDays: number | null;
}

/** Result of a successful key rotation. */
export interface RotationInfo {
  /** Key name (e.g. 'oz-pos/encryption-key'). */
  keyName: string;
  /** ISO 8601 timestamp of when the new key was created. */
  createdAt: string;
  /** Number of bytes in the generated key. */
  keyBytes: number;
}

/** Get the current key rotation status (key age, creation timestamp). */
export const getKeyRotationInfo = (): Promise<KeyRotationStatus> =>
  invoke<KeyRotationStatus>('get_key_rotation_info');

/** Rotate (re-generate) the encryption key, archiving the previous one. */
export const rotateEncryptionKey = (): Promise<RotationInfo> =>
  invoke<RotationInfo>('rotate_encryption_key');
