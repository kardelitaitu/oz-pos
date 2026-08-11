# Roles and Permissions Plan

## 1. Design goals

The authorization model should remain stable while permissions, branches, and workspaces evolve.

- Keep roles small, predictable, and easy to understand.
- Express authorization through permissions, not hard-coded role names.
- **Never edit an existing role definition to add a feature.** Growing the system means adding entries to the permission registry, not touching roles. Acceptance test: a brand-new workspace must ship with its permission family and be granted to the right roles **without modifying any existing role definition**.
- **Authorization fails closed.** Every permission-sensitive action passes through one centralized backend gate, denied by default. Frontend role gating is presentation only and never a security boundary.
- Apply branch and workspace scopes to assignments.
- Enforce every permission-sensitive action on the backend.
- Support system roles and customer-defined custom roles without changing the core schema.

## 2. Core model

| Concept | Purpose |
| --- | --- |
| Organization (tenant) | The SaaS customer company. Cloud-only concept; deferred until real multi-tenant deployment exists. |
| Branch / location | A physical or logical site, such as a store, restaurant, or warehouse. |
| Workspace / module | A feature area, such as `retail-pos`, `resto-pos`, `warehouse`, or `kds`. Future examples include loyalty, accounting, and topology editor. |
| Permission | An atomic capability represented by a string key. |
| Permission family | A stable grouping of permission keys (e.g. `sales`, `kds`, `reports`). Families, not individual keys, are the unit of role stability. |
| Permission registry | The single source of truth mapping every key to its family, its sensitivity, and a description. Adding a feature = adding registry entries. |
| Role | A named collection of permission grants. Grants are expressed at family granularity for operational capabilities and as explicit keys for sensitive capabilities. |
| Assignment | A user-to-role relationship with a scope mode, branch and workspace scopes. |

This separation keeps roles stable while permissions and scopes grow.

## 3. Built-in roles

Keep the built-in role set intentionally small. New requirements should normally be handled through permission families, scopes, or custom roles.

### Quick reference for contributors

| Role | Scope mode | Can view | Can modify | Main restriction |
| --- | --- | --- | --- | --- |
| **Owner** | Global (cannot be narrowed) | Everything in the organization | Everything, including billing and ownership actions | Cannot be narrowed by assignment scope. |
| **Admin** | Global (cannot be narrowed) | Organization, branches, workspaces, reports, and audit data | Most operational and administrative settings | Cannot transfer ownership, manage billing, or perform irreversible organization actions by default. |
| **Manager** | Scoped (assigned branches and workspaces) | Operational data, reports, inventory, and assigned users | Staff and operational workflows within the assigned scope | Cannot manage organization settings, billing, or unassigned branches. |
| **Staff** | Scoped (assigned branches and workspaces) | Assigned operational information | Assigned day-to-day workflows | No user/role management, topology editing, or high-risk actions unless explicitly granted. |
| **Auditor** | Global, read-only | Business data, reports, configuration, and audit logs | Nothing | Cannot create, update, delete, approve, transact, or administer the organization. |

The Auditor role provides organization-wide visibility without write access. It should never expose credentials, secrets, or other data excluded by the system’s privacy policy.

> **Operational job titles are not system roles.** Cashier and kitchen are the *same* Staff role scoped to a workspace — a staff user assigned to `retail-pos` is the cashier-like operator, one assigned to `kds` is the kitchen-like operator. The username (`cashier-1`, `kitchen-1`) is a human label only; the backend never authorizes from names. New job flavors are composed as custom roles from registry keys.

### Owner

Full access to the organization, including:

- Billing and organization settings.
- Ownership transfer and irreversible organization actions.
- All users, roles, branches, workspaces, topology editing, and audit logs.

There are usually only one or a few owners. The Owner role cannot be narrowed by assignment scopes.

### Admin

Almost all Owner permissions, except ownership transfer, billing, and other irreversible organization actions unless explicitly configured.

- Full access across branches and workspaces (global scope, like Owner).
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

Examples include processing sales, viewing KDS orders, and picking or packing warehouse items. Staff should not receive user-management, topology-editor, or high-risk permissions such as unrestricted voids or refunds unless those permissions are explicitly granted. The same Staff role serves every operational job title (cashier, kitchen, picker) via workspace-scoped assignments.

### Auditor

Read-only visibility across the organization.

- Can view all branches and workspaces.
- Can view reports, inventory, configuration, and audit logs.
- Can export data only when an explicit export permission is granted.
- Cannot create, update, delete, approve, or execute operational transactions.
- Cannot manage users, roles, billing, ownership, or topology configuration.

The Auditor role is useful for internal controls, compliance reviews, finance review, and external audits. It should be implemented as a permission set, not as a frontend-only restriction. Auditor never sees sensitive profile fields — national ID, payroll, notes, or emergency contact data.

### Optional roles for later

These roles do not require a new role model if custom roles are supported:

- `Supervisor` / `Shift Lead` — a restricted subset of Manager.
- `Accountant` / `Viewer` — read-only reporting across selected scopes.
- `Integrator` / `API` — machine-user access with narrowly defined permissions.

## 4. Permission model

Do not encode rules such as “Managers can do X” throughout application logic. Application code should check permission keys and assignment scopes.

### Naming convention

Use `{resource}:{action}` with underscore actions. The resource is either a **workspace key** for workspace-owned capabilities (`kds:view`, `kds:update`) or a **domain key** for cross-cutting capabilities (`sales:process`, `customers:view`, `reports:view`). Dotted suffixes are allowed for new nested capabilities but are not required; existing keys are never renamed for style.

```text
# Cross-cutting (domain-keyed) families (illustrative, not exhaustive)
customers:view
customers:create
reports:view
reports:export
settings:read
settings:edit
staff:read
staff:create
staff:update
staff:manage_roles

# Workspace-owned families
retail-pos:sale
retail-pos:void
retail-pos:refund
kds:view
kds:update
warehouse:receive
warehouse:pick
warehouse:transfer

# Organization-level capabilities (explicitly granted, never wildcarded)
org:billing
org:ownership
org:roles.manage
org:topology.edit
audit-logs:view
audit-logs:export

# Sensitive staff-profile capabilities (explicitly granted, never wildcarded)
staff:read_identity
staff:read_payroll
staff:edit_notes
```

### Permission registry

Every permission key is registered: `key → (family, sensitive, description)`. The registry is the single source of truth for what a key means, which family it belongs to, and whether it is a high-risk capability. New features add keys to the registry; they never edit roles.

The registry lives in **code**, not the database — `rbac.rs` permission consts plus the per-module manifests (`modules/*/manifest.json`) — so it cannot be tampered with via a DB edit, and authorization fails closed at compile time. Role grants (DB rows) are validated against the registry at write time. Family wildcard matching already exists in `rbac.rs::has_permission` (e.g. `"sales:*"`, `"*"`); the work is classifying every existing key into a family and sensitivity, not building a new matching engine.

### Family grants (operational)

Roles grant at **family granularity** for operational capabilities — `sales:*`, `reports:*`, `customers:*` — instead of enumerating every action. When a new key is added inside an existing operational family, every role holding that family wildcard is automatically granted the new capability. This is the mechanism that keeps roles unchanged as the codebase grows.

### Explicit grants (sensitive)

Irreversible or financial capabilities — voids, refunds, billing, ownership transfer, role management, bulk export — are **never wildcarded**. Each sensitive key is enumerated explicitly on the roles that deserve it. Adding a sensitive capability costs exactly one grant line per affected role, and that cost is deliberate: privilege creep from implicit grants is worse than an occasional explicit line.

### The stability rule

- New operational action in an existing family → registry addition, zero role edits.
- New sensitive action → registry addition + explicit grants where intended.
- New workspace → register its permission family; operational families already granted to roles extend to it, zero role edits.

## 5. Assignment scopes

An assignment combines a user, a role, a scope mode, and scopes:

- **Global mode** — org-level roles (Owner, Admin, Auditor). The assignment ignores branch/workspace scope; Owner cannot be narrowed, Auditor is org-wide read-only.
- **Scoped mode** — operational roles (Manager, Staff). The assignment carries a branch dimension and a workspace dimension; each dimension is independently an explicit `all` or a list:
  - `all` branches and `all` workspaces — the explicit form of full access for a scoped role.
  - One or more specific branch IDs (all workspaces in those branches).
  - One or more specific workspace keys (all branches).
  - A combination, such as Manager for Branch A and Branch B, limited to `warehouse` and `retail-pos`.

Example:

> A staff member assigned to `resto-pos` and `kds` at the Downtown branch receives only the permissions granted by their role for those workspaces at that branch.

For each request, authorization should require all of the following:

1. The assigned role contains the requested permission (family wildcard or explicit key).
2. The requested branch is included in the assignment scope, or the role is global-mode.
3. The requested workspace is included in the assignment scope, or the role is global-mode.

Represent “all branches” and “all workspaces” explicitly, rather than relying only on empty lists. This avoids ambiguity between “global” and “not configured.” The username and display name carry job flavor for humans only and are never consulted by the authorization path.

## 6. Why this model scales

- **New workspace:** Register its permission family and enable the workspace on selected branches. Existing roles remain valid — an operational family wildcard already covers it.
- **New fine-grained action:** Add a registry entry such as `sales:discount.approve`; wildcard holders of `sales:*` are automatically granted unless the key is sensitive.
- **Multi-branch responsibility:** Reuse Manager with multiple branch scopes.
- **Workspace restriction:** Reuse the same role with a limited workspace scope.
- **Customer-specific job:** Create a custom role such as `Barista` or `Stock Clerk` from a permission subset.
- **Cashier / kitchen:** The same Staff role scoped to `retail-pos` vs `kds`, with the username as the human label.
- **Topology editor:** Protect it with `org:topology.edit`, normally granted only to Owner and Admin.
- **Acceptance test:** adding a hypothetical new workspace end-to-end (registry, grants, enforcement) must require **zero edits to existing role definitions**.

## 7. User profile data

Every user record carries a **required set** enforced at creation and an **optional set**. Required fields are validated at creation time (field-specific errors, no partial writes); the database columns stay nullable so existing rows — the seeded owner and current staff — are not forced to fabricate values. A user with missing required data is in an **incomplete-profile state** until the fields are filled.

### Required at creation

| Field | Type | Notes |
| --- | --- | --- |
| `username` | string | Unique login handle (already `NOT NULL UNIQUE` today). |
| `full_name` | string | Display name (`display_name` today). |
| `date_of_birth` | date | Must not be in the future. Sensitive. |
| `phone` | string | E.164 format; not unique — shared and family lines are real. |
| `national_id_type` + `national_id` | enum + string | Exactly one required pair. The type is the discriminator for “SSN or KTP if Indonesian”: `ssn` = 9 digits, `nik` = 16 digits; the type list grows per country (NRIC, TFN, …). `national_id` is UNIQUE when present — it is the duplicate-enrollment detector. Sensitive. |
| `email` | string | Unique; the login identifier for management roles and the invitation target. |
| `monthly_take_home_minor` | money | Take-home pay in i64 minor units, in the store’s currency, strictly positive. Sensitive. |
| `emergency_contact_name` | string | Sensitive: PII about a third party who did not consent to being in the POS. |
| `emergency_contact_phone` | string | Same sensitivity as the contact name. |

**Login semantics:** username + PIN remain the checkout credential (today's argon2 PIN flow). Email is a unique *identifier* — the login target for management roles and invitations — not a second PIN/password credential in phase 1.

### Optional

| Field | Type | Notes |
| --- | --- | --- |
| `job_title` | string | Human label (“Cashier”, “Kitchen”). Presentation only — never parsed for authorization. Rename-safe and localizable, unlike deriving the title from the username. |
| `notes` | string | Free text, `TEXT NOT NULL DEFAULT ''` like `customers.notes`. Where HR incidents get written: manager-only edit, never visible to Auditor. |
| `address` | string | Operational. |
| `language` | string | Per-user UI locale. Operational. |
| `avatar` | blob/path | Cashier and KDS screens; may be deferred to phase 2. Operational. |
| `tax_id` | string | Indonesian NPWP (and equivalents elsewhere). Payroll compliance. Sensitive. |
| `national_id_expires_at` | date | KTPs expire; compliance tracking. Sensitive. |
| `emergency_contact_relationship` | string | Spouse / parent / sibling — tells the caller who they are dialing. |
| `hire_date` | date | Cheap and useful for reports. Operational. |

**Deliberately not collected:** gender, religion, marital status, ethnicity, and blood type — legally and ethically sensitive in most jurisdictions with zero authorization value; do not collect what the product cannot use and protect. Bank account is deferred until direct deposit is a real feature. Shift/availability belongs to a scheduling feature, not the profile.

### Sensitivity and enforcement

- `national_id`, `date_of_birth`, `monthly_take_home_minor`, and `tax_id` are sensitive profile fields: **never covered by a family wildcard**, never visible to the Auditor role. Access requires the explicit `staff:read_identity` (national ID, DOB) and `staff:read_payroll` (payroll fields) grants.
- `notes` is sensitive by content: reads ride the `staff:read` family (any manager role); edit requires the explicit `staff:edit_notes` grant (manager-scoped); Auditor never reads it.
- `national_id` and `monthly_take_home_minor` are encrypted at rest via `oz-security` (keyring-backed), consistent with the existing license-API-key encryption precedent.
- `job_title`, `address`, `language`, and `avatar` are operational and ride the `staff:read` family.
- Emergency contact data is third-party PII: out of Auditor scope and out of exports by default.
- All validation is field-specific and fails closed, like every other input path (no partial writes).

## 8. Implementation recommendations

### Data model

- `roles`: `id`, `name`, `description`, `is_system`, and timestamps. (`org_id` deferred — see below.)
- `role_permissions`: `role_id`, `permission_key` — keys may be family wildcards (`sales:*`) or explicit sensitive keys, validated against the code-resident registry at write time. (No `permission_registry` table — the registry lives in code.)
- `assignments`: `id`, `user_id`, `role_id`, `scope_mode` (`global` | `scoped`), and optional `expires_at` (expiry deferred to phase 2).
- `assignment_branches`: `assignment_id`, `branch_id`.
- `assignment_workspaces`: `assignment_id`, `workspace_key`.

Prefer normalized join tables for permissions and scopes so queries, constraints, and future reporting remain reliable at scale. If the storage layer already standardizes serialized collections, arrays may be acceptable as an implementation detail, but the authorization semantics must remain explicit.

### Authorization and operations

- Seed the system roles when the installation is initialized (first-owner bootstrap) — no org layer is required.
- **Centralize enforcement:** a single backend `require_permission(permission)` gate that every command must pass, deny-by-default and fail-closed. New commands that skip the gate are caught at review time, not audit time.
- Allow Owners and authorized Admins to create custom roles.
- Evaluate effective permissions centrally; resolve family wildcards and scope_mode in one place.
- Cache effective permissions in Redis or memory, with invalidation on role or assignment changes (deferred until multi-tenant or scale demands it).
- Enforce authorization on the backend; the frontend should only hide or disable unavailable UI.
- Audit every permission-sensitive action — sensitive actions especially.
- Support invitations by email with role and scope selection (deferred until the org layer exists).
- Add tests for role permissions, family wildcards, sensitive exclusions, branch scopes, workspace scopes, and high-risk actions.

**Deferred (do not build yet):** org-scoped roles and tables, invitations, permission caching, assignment expiry, role inheritance. Role inheritance is out of scope by design — flat roles with family grants are easier to reason about and audit than a hierarchy, and hierarchies are what force role edits as the system grows.

## 9. Mapping from the original role ideas

| Original idea | Recommended model |
| --- | --- |
| `owner` | Owner system role. |
| `ceo` | Admin system role, unless the business needs a genuinely different permission set. |
| `manager` | Manager role with branch scopes. |
| `staff` | Staff role with branch and workspace scopes; operational job titles (cashier, kitchen) are workspace assignments plus username labels, not roles. |
| `auditor` | Auditor system role with organization-wide read-only scope. |

## 10. Recommended implementation sequence

1. Define the permission registry; classify every current key into a family and mark it operational (wildcardable) or sensitive (explicit-only). Keep today's `{resource}:{action}` strings — renames churn enforcement call sites for zero user value.
2. Centralize enforcement: one fail-closed backend gate every permission-sensitive command passes through.
3. Add the assignment model (role, scope_mode, branch/workspace scopes); migrate `users.role_id` to a default global-mode assignment so existing databases are unchanged in behavior.
4. Align the role taxonomy: seed Owner, Admin, and Auditor alongside the existing Manager and Staff (`role-staff` survives); fold cashier/kitchen into Staff + workspace assignments; retire `role-cashier`/`role-kitchen` via the migration in step 3.
5. Add custom roles, UI/API integration, and focused authorization tests across branch and workspace combinations.
6. Add the user profile fields (required + optional above) with the incomplete-profile state; encrypt `national_id` and `monthly_take_home_minor` at rest via `oz-security`; register `staff:read_identity`, `staff:read_payroll`, and `staff:edit_notes` as sensitive keys.
7. Deferred: org-tenant layer, invitations, permission caching, assignment expiry.

The result is a small, stable role model with enough flexibility for modular workspaces, multi-branch users, and customer-specific roles — where growing the codebase means growing the permission registry, never editing roles.
