/** Normalised role variant used for badge styling, icon selection, and gating. */
export type RoleVariant = 'owner' | 'admin' | 'manager' | 'staff' | 'auditor';

/**
 * Normalizes any role string into a known variant key.
 *
 * Five-role taxonomy (ADR #35 D4 / spec 0048): owner, admin, manager,
 * staff, auditor. The retired cashier/kitchen roles (0048 2c) no longer
 * exist and are not recognized — unknown/legacy strings resolve to 'staff'
 * so they never gate above the checkout-operations floor.
 */
export function normalizeRole(roleString?: string | null): RoleVariant {
  if (!roleString) return 'staff';
  const r = roleString.trim().toLowerCase();
  if (r === 'owner' || r === 'role-owner') return 'owner';
  if (r === 'admin' || r === 'role-admin') return 'admin';
  if (r === 'manager' || r === 'role-manager') return 'manager';
  if (r === 'auditor' || r === 'role-auditor') return 'auditor';
  return 'staff';
}
