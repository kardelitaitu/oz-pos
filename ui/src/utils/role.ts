/** Normalised role variant used for badge styling, icon selection, and gating. */
export type RoleVariant = 'owner' | 'manager' | 'cashier' | 'kitchen' | 'staff';

/**
 * Normalizes any role string into a known variant key.
 */
export function normalizeRole(roleString?: string | null): RoleVariant {
  if (!roleString) return 'staff';
  const r = roleString.trim().toLowerCase();
  if (r === 'owner') return 'owner';
  if (r === 'manager') return 'manager';
  if (r === 'cashier') return 'cashier';
  if (r === 'kitchen' || r === 'kds' || r === 'chef') return 'kitchen';
  return 'staff';
}
