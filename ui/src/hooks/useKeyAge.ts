import { useState, useEffect, useCallback } from 'react';
import { useToast } from '@/frontend/shared/Toast';

const KEY_CREATED_KEY = 'oz-key-created-at';
const ROTATION_DAYS = 90;
const WARN_DAYS = 85; // Start warning 5 days before expiry

/** Get the stored key creation timestamp. */
export function getKeyCreatedAt(): string | null {
  return localStorage.getItem(KEY_CREATED_KEY);
}

/** Set the key creation timestamp (call after initial key generation). */
export function setKeyCreatedAt(iso: string) {
  localStorage.setItem(KEY_CREATED_KEY, iso);
}

/** Calculate days since key creation. Returns null if not tracked. */
export function getKeyAgeDays(): number | null {
  const stored = getKeyCreatedAt();
  if (!stored) return null;
  const created = new Date(stored).getTime();
  const now = Date.now();
  return Math.floor((now - created) / (1000 * 60 * 60 * 24));
}

/** Check if the key needs rotation. */
export function isKeyRotationDue(): boolean {
  const age = getKeyAgeDays();
  if (age === null) return false;
  return age >= ROTATION_DAYS;
}

/** Get days until rotation is due (negative if overdue). */
export function getDaysUntilRotation(): number | null {
  const age = getKeyAgeDays();
  if (age === null) return null;
  return ROTATION_DAYS - age;
}

/**
 * Hook that shows a toast notification when the key is approaching
 * 90 days old. Checks once on mount and sets up a daily check.
 */
export function useKeyRotationReminder() {
  const { addToast } = useToast();
  const [dismissed, setDismissed] = useState(false);

  const check = useCallback(() => {
    if (dismissed) return;
    const age = getKeyAgeDays();
    if (age === null) return;

    const daysLeft = ROTATION_DAYS - age;
    if (daysLeft <= 0) {
      addToast({
        type: 'warning',
        message: 'Encryption key rotation is overdue. Please rotate keys from Security settings to maintain PCI-DSS compliance.',
        duration: 0, // Persistent until dismissed
      });
    } else if (daysLeft <= WARN_DAYS - (ROTATION_DAYS - age) + 5) {
      // Only warn when within 5 days of due date
      if (daysLeft <= 5) {
        addToast({
          type: 'info',
          message: `Encryption key rotation due in ${daysLeft} day${daysLeft === 1 ? '' : 's'}. Please rotate from Security settings.`,
          duration: 10000,
        });
        setDismissed(true);
      }
    }
  }, [addToast, dismissed]);

  useEffect(() => {
    // Check on mount
    check();
    // Check daily
    const interval = setInterval(check, 24 * 60 * 60 * 1000);
    return () => clearInterval(interval);
  }, [check]);
}
