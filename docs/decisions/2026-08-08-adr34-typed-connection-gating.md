# ADR #34: Typed Connection Gating & Live Validation (Implementation)

**Status:** Implemented (2026-08-08)  \
**Date:** 2026-08-08  \
**Author:** Architecture Team & OZ-POS Contributors  \
**Tags:** topology, typed-ports, semantic-gating, live-validation, node-editor, adr-34

---

## Context

[ADR #34 — Topology Editor as the Business Logic Builder](2026-08-07-business-logic-topology-builder.md)
establishes the target contract: nodes expose named semantic ports, wires are
typed relationships, and the editor must make valid relationships discoverable
and invalid ones impossible to create. It explicitly defers the internal
schema — "the exact internal schema is intentionally deferred to the
implementation ADR or follow-up design" — to this document.

Before this work, the editor's connection gate was permissive: any output
could be dragged onto any input and a wire would be drawn. Compatibility was
checked only structurally (direction, endpoints), never semantically. A user
could wire a warehouse's stock rack into a workspace's Location input and the
canvas would accept it, leaving Apply to reject the graph later — after the
user had invested in an arrangement that was wrong from the start.

This implementation ADR records the four decisions that close that gap:

1. the closed **semantic pairing table** that defines which sockets may
   connect and what relationship the wire means;
2. the **`gatingSemanticId` vs `semanticPortId` split** — two resolvers with
   two different jobs that must never be conflated;
3. the **ticket-routing authoring slice** — how the originally load-only
   `ticket-out`/`ticket-in` pair became re-authorable; and
4. the **live-validation gate mirror** — the exact Apply gate running on the
   canvas while the user edits, so badges and Apply can never drift.

---

## Decision

### 1. The semantic pairing table (`SEMANTIC_PORT_PAIRINGS`)

A single ordered row list in `ui/src/features/stores/topologyCard.ts` is the
one source of truth for what may connect. Each row pairs a source semantic
with a target semantic, the typed relationship that combination represents,
and the Fluent id for its human-readable label:

| Source semantic | Target semantic | Relationship type | Label |
|---|---|---|---|
| `location-out` | `location-in` | `location` | Location |
| `location-out` | `operation-in` | `location` | Location |
| `stock-out` | `stock-in` | `stock-routing` | Stock routing |
| `transfer-out` | `transfer-in` | `inventory-transfer` | Transfer |
| `ticket-out` | `ticket-in` | `ticket-routing` | Ticket routing |
| `operation-out` | `operation-in` | `generic` | Operation |
| `device-out` | `generic-in` | `hardware-connection` | Device connection |
| `generic-out` | `generic-in` | `generic` | Generic |

Rules:

- **Inputs are never sources.** The table has no row with an input semantic on
  the source side, so a drag that starts from an input fails closed.
- **Unknown combinations fail closed.** `canSemanticPortsConnect(source,
  target)` returns `false` for any pair absent from the table — a Location
  feed into a stock rack, a stock feed into a Location input, an operation
  feed into a generic input.
- **The row is the relationship.** A wire created under a row records that
  row's `relationshipType` and the semantic ids (`fromPortId`/`toPortId`),
  which is exactly what the backend round-trips and the duplicate detector
  compares.
- **Order is meaningful.** The rows are the picker's option order — the
  primary relationship of a multi-semantic pair always comes first.

The table is consumed by two surfaces that must agree:

- `canSemanticPortsConnect(source, target)` — the boolean gate used while a
  wire is being dragged (compatible-target highlighting + reject-before-draw);
- `wireRelationshipOptions(source, sourcePort, target, targetPort)` — the
  admissible `(fromPortId, toPortId, relationshipType, labelId)` options for a
  drop. Zero options = incompatible; one option = auto-create the wire; two or
  more = the drop is ambiguous and the relationship picker asks the user.

### 2. `gatingSemanticId` vs `semanticPortId`

Two resolvers exist in `topologyCard.ts`, with deliberately different scopes:

- **`semanticPortId(node, port, variantIndex)`** — the *recording-side*
  resolver. It returns `undefined` for sockets that carry no persisted
  semantic (a store's output is the only output it resolves; workspace left
  inputs resolve; warehouse/hardware outputs and inputs resolve to
  `undefined`). Persisted wire semantics and duplicate detection stay stable
  because the recorded vocabulary never grows.
- **`gatingSemanticId(node, port, variantIndex)`** — the *gating-side*
  resolver. It resolves **every** authorable socket — POS/warehouse
  `stock-out`, KDS `ticket-out`, hardware `device-out`, warehouse `stock-in`,
  hardware `generic-in` included — so a drag in progress can tell a
  compatible target from an incompatible one before any wire is drawn.

Why not one resolver? If the gate resolved only what the recorder resolves,
unresolved sockets would gate open (or closed) by accident. If the recorder
resolved everything, persisted semantics would churn as the vocabulary grows
and duplicate detection would need to understand relationships it never
records. Keeping the two separate means the **gate can be complete** while the
**recording contract stays minimal** — the split is the safety property.

Both are derived from the same lower-level `socketSemanticIds(node, port)`,
which returns ALL semantics a socket admits in canonical order — a plain
workspace output is `['stock-out', 'transfer-out']`, a warehouse input is
`['stock-in', 'transfer-in']` — with `gatingSemanticId` defined as the
`[0]` primary. The gate and the multi-semantic picker therefore share one
resolution and can never disagree.

### 3. Ticket routing is authorable (load-only gap closed)

The pairing table admits `ticket-out → ticket-in` (`ticket-routing`), and the
Resto preset ships a loaded KDS → printer wire that records exactly those
semantic ids. The pair is now **re-authorable in the UI**:

- a Kitchen Display exposes a visible right **Ticket Out** output socket
  (`visiblePortsForNode` returns `['left', 'right']` for KDS — it consumes
  one Operation feed from the left and forwards ticket feeds from the
  right);
- hardware inputs admit the `ticket-in` semantic alongside `generic-in`
  (`socketSemanticIds` for a hardware left socket returns
  `['generic-in', 'ticket-in']`), so a KDS → hardware drop resolves to
  exactly one `ticket-routing` option — no picker, and the wire records the
  exact `ticket-out`/`ticket-in` format the preset persists.

A created KDS → printer wire carries the `topology-wire-label-ticket`
label ("Ticket Print"), matching the preset's hardcoded label. Duplicate
detection compares the recorded `toPortId`, so a second KDS → printer wire
is rejected even against a preset-loaded one.

`operation-out` is authorable from Restaurant POS and feeds a KDS's
`operation-in` socket. It remains a shared contract semantic for future
operational workspace outputs.

### 4. Live-validation gate mirror (`validateEditorGraph`)

The Apply gate and the on-canvas badges must never drift. A single shared
helper — `validateEditorGraph(nodes, wires, allowLegacyApply)` in
`NodeTopologyEditor.tsx` — is the only place the gate logic lives:

```text
validateEditorGraph(nodes, wires, allowLegacyApply)
  → normalizeTopologyGraph(nodes, wires)        # semantic graph + legacy normalization
  → validateTopologyGraph(semanticGraph)        # root identity, ownership, purpose/type
  → gated: hasCanonicalBranchIdentity || !allowLegacyApply
```

Both call sites pass through it:

- the **live badge surface** (a memo over `[nodes, wires, allowLegacyApply]`
  that renders per-node `.node-validation-note` cards and a graph-level
  `.topology-validation-banner`), and
- the **Apply handler**, which refuses to apply the same error set the badges
  display.

So an error the user sees on the canvas is exactly the error Apply will throw,
and vice versa. Both surfaces clear live as the canvas is edited — no Apply
round-trip needed. Legacy non-canonical canvases stay badge-free, mirroring
the gate: `allowLegacyApply=true` tolerates a missing canonical branch
identity, `false` (strict) requires it.

### Relationship picker (same pairing-table contract)

A drop that admits two or more relationships (workspace → warehouse:
`stock-routing` vs `transfer`) opens the `.topology-relationship-picker` — a
`role="dialog"` anchored at the target node's left edge — instead of drawing
a wire blindly. The chosen option flows through `commitWire`, the single
creation path shared with unambiguous drops, so duplicate detection and the
Pro-tier stock-routing fallback limit stay correct regardless of which
surface created the wire.

---

## Slice — Ticket-routing authoring (DONE)

**Implementation date:** 2026-08-08

This slice closes the original load-only gap for `ticket-out`/`ticket-in`:
KDS → printer ticket wires are now **authorable on the canvas**, recorded in
the exact format the Resto preset already persists.

### What changed

- **A KDS now exposes a visible right Ticket Out socket.**
  `visiblePortsForNode` returns `['left', 'right']` for KDS (it was a pure
  sink with only `['left']`). The socket resolves to `ticket-out`, so a drag
  can start from it. Its label is the new `topology-port-ticket-out` key
  ("Ticket Out") with a dedicated `topology-port-ticket-out-aria`.
- **Hardware inputs admit the `ticket-in` semantic.** `socketSemanticIds`
  for a hardware left socket is now `['generic-in', 'ticket-in']`. This was
  the key decision: rather than adding a new `ticket-out → generic-in`
  pairing row (which would record `generic-in` on the wire and break
  duplicate detection against the preset's `ticket-in`), the existing
  `ticket-out → ticket-in` row becomes reachable, and authored wires record
  the **exact preset format** (`ticket-out`/`ticket-in`/`ticket-routing`).
  The drop resolves to exactly one option — no relationship picker.
- **Labels follow the wire.** A hardware left input reads "Ticket In" when
  a ticket wire is attached, "Input" otherwise — mirroring the warehouse
  stock/transfer pattern. This also fixed a latent bug: hardware left
  previously fell through to the `['location-in']` variant, labeling a
  printer input "Location".
- **`commitWire` gives ticket wires a dedicated label.** A created KDS →
  printer wire carries `topology-wire-label-ticket` ("Ticket Print"),
  matching the preset's hardcoded label instead of the generic "Connected".

### Why hardware admits `ticket-in` instead of a new pairing row

A `ticket-out → generic-in` row would let the drop author, but the wire
would record `toPortId: 'generic-in'` — different from the preset's
`ticket-in`. Duplicate detection compares the recorded `toPortId`, so a
second KDS → printer drop would then slip past the preset-loaded wire.
Admitting `ticket-in` on the hardware input keeps the persisted vocabulary
canonical and duplicate detection intact.

### Tests

- `topologyCard.test.ts`: `visiblePortsForNode` for KDS is `['left',
  'right']`; `socketSemanticIds` for hardware left is `['generic-in',
  'ticket-in']`; `wireRelationshipOptions(kds, right → hw, left)` resolves
  to exactly one `ticket-routing` option; KDS right label/aria; hardware
  left label follows the attached wire; hardware aria stays generic
  `topology-port-aria`.
- `NodeTopologyEditor.test.tsx`: the KDS sink tests were updated (right
  Ticket Out socket present on standalone and preset KDS); a new authoring
  test drags KDS right → printer left and asserts the ticket-routing wire
  with its "Ticket Print" label appears with no picker; a new regression
  test loads the Resto preset and asserts a second KDS → printer drop is
  rejected as a duplicate (wire count stays 4).
- Editor suite **179/179** + adjacent topology suites green.

### Remaining

Restaurant POS now exposes `operation-out` on its right socket, so the
existing `operation-out → operation-in` row authorizes the Resto POS → KDS
connection. Future operational workspace types can reuse the same semantic
without changing the pairing table.

---

## Decision — Ticket-routing cardinality (2026-08-12)

Parent ADR open item 6 asks for the exact cardinality and cycle rules of each
non-ownership relationship, with the first default of explicit replacement
(never silent). This closes it for `ticket-routing`:

1. **KDS `ticket-out` fans out to many targets (`many`).** One KDS may route
tickets to any number of printer/ticket hardware nodes — a kitchen display
commonly drives a main printer plus remote/expo stations. This mirrors the
Branch Location `location-out` fan-out rule and required no gate change; it
is pinned by a contract test (one KDS → two printers validates clean).
2. **Hardware `ticket-in` accepts exactly one source (`one`).** A ticket
device receives tickets from a single KDS. Two KDS feeding one printer
interleave tickets with no source identity — the exact ambiguity class the
ownership exactly-one rules (`location-in`, `operation-in`) already
eliminate. A graph with a second ticket-routing source onto an already-
sourced printer fails `multiple-ticket-inputs`, scoped to the printer node
(one error per device, deterministic on the second wire), rendered as a
card note by the live badge mirror and refused by the same contract at
Apply.
3. **Replacement is explicit, never silent.** A drop that would exceed the
input cap is refused at drag time in `commitWire` with the
`topology-validation-multiple-ticket-inputs` toast — no wire is drawn and
no existing wire is removed. Same-source pairs were already refused as
duplicates; this closes the different-source case.
4. **No cycle rule is needed.** `semanticNodesMatchWire` restricts
ticket-routing to KDS → hardware, and hardware exposes no ticket-out, so
the relationship cannot participate in a directed cycle; the whole-graph
`cycle-detected` check covers any cross-relationship cycle.

The warehouse `stock-routing`/`inventory-transfer` rules (capacity,
servicing, hub-and-spoke, cycle) and the hardware-connection pairing remain
unchanged; their cardinality closes are future slices per item 6.

---

## Consequences

### Positive

- Invalid drops are rejected at drag time, not at Apply: incompatible targets
  never highlight and an invalid drop draws nothing, with the
  `topology-wire-incompatible` toast.
- One pairing table drives the gate, the picker, and the relationship labels
  — there is no second copy to rot.
- Multi-semantic sockets are explicit (a workspace output is
  `stock-out` **or** `transfer-out`) and disambiguated by an in-canvas picker
  instead of a hidden default.
- Wires persist their semantics (`fromPortId`/`toPortId`/`relationshipType`),
  matching what the presets already recorded, so round-trips are stable.
- The Apply gate is visible while editing; a user fixes the canvas before
  Apply instead of discovering the failure at save time.

### Negative

- Two resolvers (`semanticPortId` vs `gatingSemanticId`) must be maintained
  in parallel; their split is a documented invariant, not an accident.
- `operation-out` is currently authored by Restaurant POS; future operational
  workspace types must explicitly expose the semantic before it becomes
  authorable from their sockets.
- The closed table is intentionally static; new relationship types (or new
  socket semantics) require a deliberate row + socket change, which is the
  desired friction for a registry the backend must also understand.

---

## Verification

- `topologyCard.test.ts`: pairing-table rows, `canSemanticPortsConnect`
  positives/negatives, `socketSemanticIds`/`gatingSemanticId` resolution,
  `wireRelationshipOptions` (zero/one/multi), warehouse socket label.
- `NodeTopologyEditor.test.tsx`: incompatible-drop toast, compatible-only
  highlighting, live badges/banner + live clearing, legacy-canvas gate
  mirror, picker open/choose/cancel, stock + transfer coexistence on one
  pair, duplicate-of-same-relationship rejection, batch delete.
- Editor suite **179/179** (2026-08-08 working tree) + adjacent topology
  suites green; `tsc` and `eslint` clean; i18n lint clean (en + id bundles
  in parity).

## Related decisions

- [ADR #34: Topology Editor as the Business Logic Builder](2026-08-07-business-logic-topology-builder.md)
- [ADR #22: Visual Node-Based Store & Workspace Topology Builder](2026-07-20-node-based-store-topology-builder.md)
- [ADR #4: Store-First Tenancy & Workspace Type/Instance Architecture](2026-07-10-workspace-type-instance-design.md)

> last audited 09-08-26 by buffy
> audit: Phase 1 Core Architecture & API Docs Audit

> status: ACCURATE (0 findings) · verified accurate: cargo check passed, no structural orphans, no stale version headers

