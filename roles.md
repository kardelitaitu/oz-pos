# Roles and Permissions Plan

## 1. Design goals

The authorization model should remain stable while permissions, branches, and workspaces evolve.

- Keep roles small, predictable, and easy to understand.
- Express authorization through permissions, not hard-coded role names.
- Apply branch and workspace scopes to assignments.
- Enforce every permission-sensitive action on the backend.
- Support system roles and customer-defined custom roles without changing the core schema.

## 2. Core model

| Concept | Purpose |
| --- | --- |
| Organization (tenant) | The SaaS customer company. |
| Branch / location | A physical or logical site, such as a store, restaurant, or warehouse. |
| Workspace / module | A feature area, such as `retail-pos`, `resto-pos`, `warehouse`, or `kds`. Future examples include loyalty, accounting, and topology editor. |
| Permission | An atomic capability represented by a string key or versioned enum. |
| Role | A named collection of permissions, with optional defaults. |
| Assignment | A user-to-role relationship with branch and workspace scopes. |

This separation keeps roles stable while permissions and scopes grow.

## 3. Built-in roles

Keep the built-in role set intentionally small. New requirements should normally be handled through permissions, scopes, or custom roles.

### Quick reference for contributors

| Role | Default scope | Can view | Can modify | Main restriction |
| --- | --- | --- | --- | --- |
| **Owner** | Entire organization | Everything in the organization | Everything, including billing and ownership actions | Cannot be narrowed by assignment scope. |
| **Admin** | Entire organization by default | Organization, branches, workspaces, reports, and audit data | Most operational and administrative settings | Cannot transfer ownership, manage billing, or perform irreversible organization actions by default. |
| **Manager** | Assigned branches and workspaces | Operational data, reports, inventory, and assigned users | Staff and operational workflows within the assigned scope | Cannot manage organization settings, billing, or unassigned branches. |
| **Staff** | Explicitly assigned branches and workspaces | Assigned operational information | Assigned day-to-day workflows | No user/role management, topology editing, or high-risk actions unless explicitly granted. |
| **Auditor** | Entire organization, read-only | Business data, reports, configuration, and audit logs | Nothing | Cannot create, update, delete, approve, transact, or administer the organization. |

The Auditor role provides organization-wide visibility without write access. It should never expose credentials, secrets, or other data excluded by the system’s privacy policy.

### Owner

Full access to the organization, including:

- Billing and organization settings.
- Ownership transfer and irreversible organization actions.
- All users, roles, branches, workspaces, topology editing, and audit logs.

There are usually only one or a few owners. The Owner role cannot be narrowed by assignment scopes.

### Admin

Almost all Owner permissions, except ownership transfer, billing, and other irreversible organization actions unless explicitly configured.

- Full access across branches and workspaces by default.
- Can manage managers and staff.
- Can configure enabled modules.
- Can use the topology editor.
- This is the recommended mapping for a CEO-level operational role.

Avoid creating separate `CEO` and `Admin` roles with identical permissions.

### Manager

Branch-oriented management access.

- Assigned to one or more branches by default.
- Has full or nearly full access to enabled workspaces within those branches.
- Can manage staff assigned under the manager’s scope.
- Can access reports, inventory, and operational workflows.
- Can be restricted to specific workspaces.
- Cannot manage organization settings, billing, or unrelated branches.

### Staff

Operational access only within explicitly assigned branches and workspaces.

Examples include processing sales, viewing KDS orders, and picking or packing warehouse items. Staff should not receive user-management, topology-editor, or high-risk permissions such as unrestricted voids or refunds unless those permissions are explicitly granted.

### Auditor

Read-only visibility across the organization.

- Can view all branches and workspaces.
- Can view reports, inventory, configuration, and audit logs.
- Can export data only when an explicit export permission is granted.
- Cannot create, update, delete, approve, or execute operational transactions.
- Cannot manage users, roles, billing, ownership, or topology configuration.

The Auditor role is useful for internal controls, compliance reviews, finance review, and external audits. It should be implemented as a permission set, not as a frontend-only restriction.

### Optional roles for later

These roles do not require a new role model if custom roles are supported:

- `Supervisor` / `Shift Lead` — a restricted subset of Manager.
- `Accountant` / `Viewer` — read-only reporting across selected scopes.
- `Integrator` / `API` — machine-user access with narrowly defined permissions.

## 4. Permission model

Do not encode rules such as “Managers can do X” throughout application logic. Application code should check permission keys and assignment scopes.

Use the naming convention `{resource}:{action}`. Workspace-specific actions use the workspace key as the resource; dotted suffixes are allowed for nested capabilities.

```text
org:read
org:manage
org:billing
org:users.manage
org:roles.manage
org:topology.edit
audit-logs:view
audit-logs:export

branch:create
branch:read
branch:manage
branch:assign

workspace:access
workspace:configure

retail-pos:sale
retail-pos:void
retail-pos:refund
retail-pos:reports

resto-pos:order
resto-pos:kds.view
resto-pos:kds.manage

warehouse:receive
warehouse:pick
warehouse:adjust
warehouse:transfer

inventory:view
inventory:adjust
reports:view
reports:export
```

Roles contain permission keys. New permissions can be added without invalidating existing roles.

## 5. Assignment scopes

An assignment combines a user, role, and scope:

- Global access.
- One or more specific branch IDs.
- One or more specific workspace keys.
- A combination, such as Manager for Branch A and Branch B, limited to `warehouse` and `retail-pos`.

Example:

> A staff member assigned to `resto-pos` and `kds` at the Downtown branch receives only the permissions granted by their role for those workspaces at that branch.

For each request, authorization should require all of the following:

1. The assigned role contains the requested permission.
2. The requested branch is included in the assignment scope.
3. The requested workspace is included in the assignment scope and enabled for that branch.

Represent “all branches” and “all workspaces” explicitly, rather than relying only on empty lists. This avoids ambiguity between “global” and “not configured.”

## 6. Why this model scales

- **New workspace:** Add permission keys and enable the workspace on selected branches. Existing roles remain valid.
- **New fine-grained action:** Add a permission such as `retail-pos:discount.approve` and grant it to the appropriate roles.
- **Multi-branch responsibility:** Reuse Manager with multiple branch scopes.
- **Workspace restriction:** Reuse the same role with a limited workspace scope.
- **Customer-specific job:** Create a custom role such as `Barista` or `Stock Clerk` from a permission subset.
- **Topology editor:** Protect it with `org:topology.edit`, normally granted only to Owner and Admin.

## 7. Implementation recommendations

### Data model

- `roles`: `id`, `org_id` (nullable for system roles), `name`, `is_system`, and timestamps.
- `role_permissions`: `role_id`, `permission_key`.
- `assignments`: `id`, `user_id`, `role_id`, explicit scope mode, and optional `expires_at`.
- `assignment_branches`: `assignment_id`, `branch_id`.
- `assignment_workspaces`: `assignment_id`, `workspace_key`.

Prefer normalized join tables for permissions and scopes so queries, constraints, and future reporting remain reliable at scale. If the storage layer already standardizes serialized collections, arrays may be acceptable as an implementation detail, but the authorization semantics must remain explicit.

### Authorization and operations

- Seed the five system roles when an organization is created.
- Allow Owners and authorized Admins to create custom roles.
- Centralize effective-permission evaluation in a backend authorization service.
- Cache effective permissions in Redis or memory, with invalidation on role or assignment changes.
- Enforce authorization on the backend; the frontend should only hide or disable unavailable UI.
- Audit every permission-sensitive action.
- Support invitations by email with role and scope selection.
- Add tests for role permissions, branch scopes, workspace scopes, expiry, and high-risk actions.

Defer role inheritance until explicit use cases require it. Explicit permission sets are easier to reason about and audit during the initial implementation.

## 8. Mapping from the original role ideas

| Original idea | Recommended model |
| --- | --- |
| `owner` | Owner system role. |
| `ceo` | Admin system role, unless the business needs a genuinely different permission set. |
| `manager` | Manager role with branch scopes. |
| `staff` | Staff role with branch and workspace scopes. |
| `auditor` | Auditor system role with organization-wide read-only scope. |

## 9. Recommended implementation sequence

1. Confirm the Owner/Admin boundary, system-role mutability, and high-risk permissions.
2. Define the permission registry and explicit scope semantics.
3. Add the role, permission, assignment, and scope persistence model.
4. Implement centralized backend authorization, seeding, invitations, and audit events.
5. Add UI/API integration and focused authorization tests across branch and workspace combinations.

The result is a small, stable role model with enough flexibility for modular workspaces, multi-branch users, and customer-specific roles.
