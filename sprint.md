# Topology Business-Logic Builder Sprint

**Status:** Complete — first semantic ownership slice implemented and verified  
**Owner:** Product + Architecture + Engineering  
**Last updated:** 2026-08-07  
**Primary ADR:** [`docs/decisions/2026-08-07-business-logic-topology-builder.md`](docs/decisions/2026-08-07-business-logic-topology-builder.md)  
**Current implementation ADR:** [`docs/decisions/2026-07-20-node-based-store-topology-builder.md`](docs/decisions/2026-07-20-node-based-store-topology-builder.md)

> This sprint plan describes the intended development sequence. It does not
> authorize a broad editor rewrite or change the current implementation by
> itself. Keep the existing topology editor working until the vertical slice
> reaches migration and validation parity.

---

## 1. Sprint objective

Evolve the topology editor from a geometric store/workspace diagram into a
validated, typed business-logic builder.

The first production slice must let a user configure one branch-scoped graph:

```text
Branch Location
  Location Out ──┬──> Location In · Workspace A
                ├──> Location In · Workspace B
                └──> Location In · Workspace C
```

The first slice must make ownership explicit and enforceable:

- one saved graph has exactly one Branch Location root;
- every workspace exposes a required input, resolved per type (POS types
  require `Location`; KDS requires `Operation`; inventory takes one flexible
  `Location` **or** `Operation` input);
- every workspace has exactly one valid ownership/feed connection;
- one Branch Location can own many workspaces;
- no primary-store or `default` fallback is used;
- semantic port identity, not screen geometry, defines the relationship; and
- Apply validates, compiles, and persists the graph within a defined transaction
  boundary; if workspace and diagram data cannot share one transaction, the
  design must provide a compensation/recovery strategy before rollout.

The first slice is declarative configuration of existing capabilities. It does
not generate arbitrary code, React screens, database tables, or executable
user-defined logic.

---

## 2. Guardrails

- Preserve the current editor, persistence, license checks, session scope, and
  atomic apply behavior while the new contract is introduced.
- Keep `store` as a serialized compatibility alias for `branch-location` during
  migration. New domain-facing UI and APIs use **Branch Location**.
- Never infer ownership from node proximity, geometric port names, display-name
  matching, or the primary store.
- Client validation improves UX; backend validation is authoritative and must
  reject malformed or unauthorized graphs.
- Do not add operational relationship types before the location-ownership slice
  is validated end-to-end.
- Every new public Rust symbol must have a doc comment, and every new Rust
  module needs unit coverage per repository standards.
- Use localized labels for user-visible port, workspace-purpose, and validation
  messages.

---

## 3. Target domain model

### 3.1 Workspace identity layers

Keep these concepts separate:

| Concept | Responsibility | Example |
|---|---|---|
| `type_key` | Technical application template and screen behavior | `store-pos`, `restaurant-pos`, `kds`, `inventory` |
| `purpose_key` | Controlled business purpose of the workspace | `checkout`, `returns`, `dining-room`, `stock-control` |
| `name` | Editable user-facing instance label | `Front Counter POS 1` |
| access policy | Who may access/use the instance | cashier, manager, kitchen staff |

`purpose_key` must not be confused with authorization roles. Authorization
continues to be governed by the existing RBAC/workspace-access model.

### 3.2 Initial semantic port registry

Start with a closed registry. Do not support arbitrary user-defined port types in
this sprint.

| Relationship type | Initial purpose | First-slice status |
|---|---|---|
| `location` | Branch Location owns a workspace | **Implement first** |
| `stock-routing` | Workspace routes stock deduction to inventory | Later |
| `ticket-routing` | Workspace routes tickets to KDS/printer | Later |
| `hardware-connection` | Workspace/service binds to hardware | Later |
| `inventory-transfer` | Inventory locations transfer stock | Later |

A port definition contains at least:

- stable port ID;
- localized label ID;
- `input` or `output` direction;
- relationship/data type;
- required flag; and
- cardinality.

The UI and Rust backend will validate from the same versioned contract, not from
an assumed shared implementation language. The implementation specification
must choose one of: a language-neutral schema with independently tested
validators, generated types/validators, or a canonical backend contract with a
strict frontend mirror.

Final connector vocabulary (implemented in the UX):

```text
Branch Location (store):  location-out → "Location"            (output, many)
POS / register:           location-in  → "Location"  (required) + workspace-out → "Operation"
Kitchen Display (KDS):    operation-in → "Operation" (required, sink, no output)
Inventory Manager:        flexible input ("Input" → "Location" | "Operation" by wire) + workspace-out → "Operation"
```

KDS `operation-in` and inventory feeds are operational edges; only the Branch
Location `location-out` edge establishes ownership scope and must never be
substituted by an operational feed.

### 3.3 Separate graph classes

```text
Ownership graph:
Branch Location ──> Workspace
Strict tree; exactly one parent per workspace.

Operational graph:
Workspace ──> Warehouse / KDS / Printer
Typed routing graph; rules vary by relationship type.
```

Only ownership edges determine the workspace's location scope. Operational
edges (`operation-in` KDS feeds, inventory feeds) must never substitute for
ownership.

The Branch Location identity must be stable and explicit. The preferred mapping
is for the semantic Branch Location node ID to equal the canonical
`store_profiles.id`; if a separate graph-node ID is required for migration or
presentation, the graph must carry an explicit `store_profile_id` reference.
Display names and synthetic IDs such as `store-1` must never be used to compile
ownership.

Distinguish these domain concepts explicitly:

- a **warehouse workspace** is a workspace instance whose `type_key` selects an
  inventory-oriented UI;
- a **warehouse/inventory-location node** is an operational domain node that
  represents a stock location; and
- a topology `workspace` node is not automatically the same thing as an
  inventory-location node merely because both may use the word warehouse.

---

## 4. Sprint workstreams

### Workstream A — Contract and schema

- [x] Define the semantic node-definition and port-definition types.
- [x] Define the closed initial relationship registry.
- [x] Add topology `schema_version` to the persisted graph contract.
- [x] Add semantic source/target port IDs and relationship type to wire payloads.
- [x] Preserve geometric anchor data only as visual/layout compatibility data.
- [x] Decide and document the canonical `store`/`branch-location` alias behavior.
- [x] Add `purpose_key` to workspace configuration after confirming the
      `workspace_instances` schema/API ownership and migration boundary.
- [x] Define localized label IDs for workspace purposes, ports, and validation
      errors in all supported bundles.

**Exit criteria:** The graph contract can represent a Branch Location,
workspace purpose, editable workspace label, semantic ports, and schema version
without relying on `top/right/bottom/left` as business meaning.

### Workstream B — Shared graph validation

- [x] Implement a pure validator that accepts the semantic graph and node
      definitions.
- [x] Require exactly one Branch Location root per graph.
- [x] Accept `store` as the legacy serialized alias for Branch Location.
- [x] Require every workspace type to expose its required input (POS:
      `Location`; KDS: `Operation`; inventory: one flexible input).
- [x] Require exactly one valid ownership edge per workspace (`Location Out →
      Location In` for POS types; one feed for KDS/inventory).
- [x] Permit Location Out fan-out to many workspaces.
- [x] Reject input-to-input, output-to-output, incompatible, duplicate, and
      missing-port connections.
- [x] Reject ownership cycles and unreachable workspace nodes (the closed
      first-slice ownership graph has only root-to-workspace edges, so invalid
      non-root ownership edges are rejected as unreachable/invalid).
- [x] Return structured validation errors with node ID, port ID, and localized
      message key where possible.
- [x] Add focused unit tests for valid ownership, missing ownership, fan-out,
      duplicates, multiple parents, and multiple roots. Cycles remain a follow-up
      because the first ownership relation is root-to-child only.

**Exit criteria:** The same validator can be called by UI-facing logic and the
backend Apply path, with deterministic results and no display-name lookup.

### Workstream C — UI authoring experience

- [x] Render semantic port labels (`Location`, `Operation`) on nodes, with
      inventory's flexible input relabeling itself from the attached wire.
- [x] Keep geometric placement as presentation only.
- [x] Highlight compatible target ports while a connection is being created.
- [x] De-emphasize or disable incompatible target ports.
- [x] Show a clear error when a workspace has no required input (Location for
      POS types, Operation for KDS, either for inventory).
- [x] Show a clear error when a single-input port already has a parent.
- [x] Show workspace label, purpose label, and technical type as separate fields.
- [x] Keep the editable workspace instance label as the primary node title.
- [x] Keep authorization role/access controls separate from workspace purpose.
- [x] Prevent Apply while the graph has unresolved validation errors.
- [x] Add focused UI tests for label rendering, target guidance, validation
      errors, and required ownership.

**Exit criteria:** A user can create the valid Branch Location → Workspace
relationship without guessing port meaning, and cannot Apply an unowned
workspace.

### Workstream D — Backend graph compiler and Apply

- [x] Validate the semantic graph again at the backend boundary.
- [x] Compile a valid ownership edge using stable IDs:
      `workspace.store_id = branch_location.id`.
- [x] Remove geometric-wire inference from ownership compilation.
- [x] Remove display-name matching from ownership compilation.
- [x] Remove primary-store/`default` fallback from the typed path.
- [x] Keep workspace `type_key` immutability and archive-plus-recreate behavior.
- [x] Preserve session-scope, entitlement, referential-integrity, and permission
      checks.
- [x] Define the actual transaction boundary for diagram and workspace data.
- [x] When both data sets cannot participate in one transaction, implement and
      test an explicit compensation/recovery strategy before rollout, including
      a durable recovery journal for interrupted compensation.
- [x] Return structured validation errors without partially applying a graph.
- [x] Add Rust command/compiler tests for valid compile, invalid graphs,
      unsupported schema versions, and rollback/compensation behavior.
**Exit criteria:** A crafted or stale IPC request cannot bypass ownership rules,
and a failed Apply leaves both runtime configuration and diagram unchanged.

### Workstream E — Legacy migration and compatibility

- [x] Define the current geometric graph as the legacy schema version.
- [x] Define a migration safety matrix covering legacy node type, direction,
      missing/null ports, labels, and endpoint identity.
- [x] Add a deterministic migration only for matrix entries classified as
      unambiguous Store → Workspace edges.
- [x] Preserve `store` as a Branch Location compatibility alias during rollout.
- [x] Detect ambiguous legacy wires instead of guessing their semantics.
- [x] Provide an unresolved-relationship UI that blocks Apply until fixed.
- [x] Keep legacy layout coordinates for visual continuity.
- [x] Add migration tests for omitted/null ports, old payloads, valid upgrades,
      ambiguous upgrades, and unsupported future versions.
- [x] Document the migration safety matrix and rollback/recovery strategy before
      enabling it for live saved diagrams.

**Exit criteria:** Existing diagrams remain loadable, ambiguous relationships are
visible and actionable, and no legacy geometry is silently reinterpreted.

### Workstream F — Purpose labels and access model

This workstream depends on an explicit decision about the owner of
`purpose_key`. Do not implement purpose persistence until the implementation
specification selects `workspace_instances`, topology metadata, or a separate
workspace-profile/settings record and documents the migration/API impact.

- [x] Define the initial controlled `purpose_key` registry per workspace type.
- [x] Examples: `checkout`, `returns`, `dining-room`, `kitchen-hot-line`,
      `stock-control`, and `receiving`.
- [x] Localize purpose display labels; keep keys stable and non-localized.
- [x] Store the editable instance `name` independently from `purpose_key`.
- [x] Apply the selected persistence owner and document its migration/API impact.
- [x] Map purpose to default capabilities/ports only through explicit registry
      definitions.
- [x] Keep authorization roles and workspace purposes independent.
- [x] Test two workspaces with the same type but different purposes and labels.

**Exit criteria:** The topology clearly distinguishes technical type, business
purpose, instance label, and access policy.

### Workstream G — Later operational relationships

Do not begin until Workstreams A–F are complete for the first slice.

- [ ] Define stock-routing semantics and priority/fallback behavior.
- [ ] Define ticket-routing semantics for KDS and printers.
- [ ] Define hardware-connection lifecycle and device identity.
- [ ] Define inventory-transfer direction and cycle policy.
- [ ] Add relationship-specific compiler effects and backend records.
- [ ] Add relationship-specific cardinality and validation tests.
- [ ] Add simulation/debugger behavior only after runtime effects are reliable.

---

## 5. Current implementation boundary

Implemented in this slice:

- A TypeScript semantic graph contract with `store` as a compatibility alias.
- Pure validation for one canonical Branch Location, per-type required inputs
  (`Location` for POS types, `Operation` for KDS, one flexible input for
  inventory), one-parent ownership, fan-out, duplicate semantic wires, and
  stable location identity.
- Legacy geometric Store → Workspace wires normalize by node identity only;
  geometric anchors remain presentation data.
- Production `TopologyScreen` Apply rejects unowned workspaces and resolves
  `store_id` only from semantic `store_profile_id` references.
- Demo editor callers retain the old compatibility Apply path until backend
  contract parity is complete.

Completed in the backend parity slice:

- Rust IPC persistence retains the versioned envelope, semantic port fields,
  relationship type, and Branch Location identity without a second raw write.
- Save, load, and Apply validate semantic ownership before persistence or
  workspace mutation; canonical `store_profile_id` references are checked in
  the authoritative global store-profile database.
- Legacy geometric payloads remain readable while semantic payloads are strict.

Complete in the first-slice implementation:

- Semantic compatibility guidance highlights valid targets and de-emphasizes
  incompatible ports during connection creation.
- The workspace transaction and global topology write have an explicit
  cross-database compensation/recovery boundary; failures restore the prior
  diagram and touched workspace rows, and compensation failures are journaled
  for retry on the next Apply.
- `purpose_key` is persisted on `workspace_instances` through migration 122,
  carried by workspace DTOs and topology diffs, and remains separate from
  instance labels and authorization.
- Legacy geometric Store → Workspace edges are normalized by stable endpoint
  identity only; ambiguous/non-ownership relationships remain actionable
  validation errors and cannot be applied.


---

## 6. Suggested implementation order

```text
A. Contract/schema
   ↓
B. Pure validator
   ↓
F. Workspace purpose + labels
   ↓
C. UI guidance
   ↓
D. Backend compiler + atomic Apply
   ↓
E. Legacy migration and rollout
   ↓
G. Operational relationships
```

The validator and compiler were developed test-first. Do not make the UI
appear to support a relationship before the backend has a defined runtime effect.

The first-slice checklist intentionally leaves Workstream G unchecked: those
items are later operational relationships, not requirements of the completed
location-ownership sprint. They require separate runtime semantics and a new
implementation plan.

---

## 7. Definition of done

The first vertical slice is complete only when all of the following are true:

- [x] One saved graph has exactly one Branch Location root.
- [x] Every existing workspace type exposes `Location In`.
- [x] Every workspace has exactly one valid location ownership edge.
- [x] One Location Out can fan out to multiple workspaces.
- [x] Invalid and duplicate connections are rejected in the UI and backend.
- [x] Ownership is based on stable IDs and semantic ports, never labels or geometry.
- [x] Purpose keys, instance labels, and authorization roles are distinct.
- [x] Schema-versioned graphs load and save correctly.
- [x] Legacy graphs migrate deterministically or are blocked with actionable errors.
- [x] Apply compiles and persists valid graphs within the documented transaction
      and compensation boundary.
- [x] Failed Apply either rolls back without partial workspace/diagram state
      under one transaction, or exercises the documented compensation/recovery
      path without leaving unrecoverable partial state.
- [x] UI and backend contract tests pass, including migration and compiler
      contract coverage.
- [x] Documentation and supported localization bundles are updated.

---

## 8. Risks and mitigations

| Risk | Mitigation |
|---|---|
| Existing geometric data has ambiguous meaning | Versioned migration; block unresolved graphs; never guess. |
| Workspace purpose is confused with authorization role | Separate `purpose_key` from RBAC/access policy in types and UI. |
| UI and backend validation diverge | Share a pure validator contract and duplicate authoritative backend enforcement. |
| New node types create unsupported runtime behavior | Closed registry; relationship enabled only after compiler/tests exist. |
| Workspace type changes break backend invariants | Retain archive-plus-recreate for immutable `type_key`. |
| Partial Apply corrupts diagram/runtime state | Keep atomic backend transaction and rollback tests. |
| Broad rewrite destabilizes current editor | Deliver one vertical slice and preserve legacy rendering until migration is proven. |

---

## 9. Open decisions for the implementation specification

These are intentionally not resolved by this sprint plan:

1. Exact TypeScript/Rust representation of node definitions and semantic ports.
2. Exact persisted graph envelope and `schema_version` values.
3. Exact storage location and migration for `purpose_key`.
4. Exact per-relationship cardinality and cycle rules.
5. Exact legacy migration safety matrix, inference rules, and unresolved-wire
   UX.
6. Exact compiler mapping for operational relationship types.
7. Whether a later chain-level editor composes multiple branch-scoped graphs.

Resolve these in an implementation specification before production rollout.

First-slice implementation decisions are now recorded in the primary ADR:
`purpose_key` is stored on `workspace_instances`; legacy geometric edges are
normalized only when endpoint identity makes Store → Workspace ownership
unambiguous; and Apply uses cross-database compensation when a single SQLite
transaction cannot span workspace and topology data.

Workstream G remains intentionally out of scope for this completed sprint. Its
unchecked items are later operational relationships requiring separate runtime
semantics, compiler effects, and rollout tests.

---

## 10. Related documentation

- [`docs/decisions/2026-08-07-business-logic-topology-builder.md`](docs/decisions/2026-08-07-business-logic-topology-builder.md)
- [`docs/decisions/2026-07-20-node-based-store-topology-builder.md`](docs/decisions/2026-07-20-node-based-store-topology-builder.md)
- [`docs/decisions/2026-07-10-workspace-type-instance-design.md`](docs/decisions/2026-07-10-workspace-type-instance-design.md)
- [`docs/specs/workspace-settings-phase-2-topology-integration.md`](docs/specs/workspace-settings-phase-2-topology-integration.md)
- [`ui/src/features/stores/NodeTopologyEditor.tsx`](ui/src/features/stores/NodeTopologyEditor.tsx)
- [`ui/src/features/stores/TopologyScreen.tsx`](ui/src/features/stores/TopologyScreen.tsx)
- [`ui/src/api/topology.ts`](ui/src/api/topology.ts)
- [`apps/desktop-client/src/commands/topology.rs`](apps/desktop-client/src/commands/topology.rs)
