---
num: 45
area: topology
title: ADR #45: Topology Semantic Contract v2 — Endpoint Predicates, Kind Registry, Deliberate Cold Start, and Theme Parity
status: Accepted — §1 implemented (2026-09-02); §2–§5 proposed
---
# ADR #45: Topology Semantic Contract v2

**Status:** Accepted — §1 implemented (2026-09-02); §2–§5 proposed
**Date:** 2026-09-02
**Author:** Architecture Team & OZ-POS Contributors
**Tags:** topology, semantic-contract, cross-language-parity, node-kind-registry, cold-start, theming

---

## Context

[ADR #34](./2026-08-07-business-logic-topology-builder.md) set the goal — "make
valid relationships discoverable and invalid relationships difficult or
impossible to create" — and [ADR #44](./2026-08-08-adr34-typed-connection-gating.md)
shipped the gate, declaring `topologySemantics.json` the single source of truth
for what may connect.

A design review of the topology surface (2026-09-02) found that the *row table*
is in fact a single source of truth, and is well guarded: the Rust crate vendors
a copy (`crates/oz-core/src/topologySemantics.json`, embedded via `include_str!`
at `crates/oz-core/src/topology.rs:35`), `scripts/verify-topology-parity.py:61-68`
byte-compares the two copies, and `crates/oz-core/src/topology_tests.rs:30` fails
the build on drift. The two files are byte-identical today.

What is **not** in that contract is the second half of every rule: which *node
kinds* may sit on each end of a row. Those predicates exist, are enforced, and
were written **six** separate times in two languages with no parity guard
between them. (Line numbers below are as they stood before this change.)

### The six shapes of one rule

| # | Where it lived | What it checked |
|---|---|---|
| 1 | `topologySemantics.json:11-19` | source semantic × target semantic × relationship |
| 2 | `topology.rs:266-296` — hand-written `match` | endpoint node types and workspace typeKeys, 5 rows |
| 3 | `topology.rs:510-521` — pre-filter | **target only**, then `continue` skipped the contract check at `:522` |
| 4 | `topologyCard.ts:346-359` — `operationRowAllowed` | **both** endpoints, generic row only |
| 5 | `topologyContract.ts:473-510` — `semanticNodesMatchWire` | a full duplicate `switch` over all rows |
| 6 | `topologyContract.ts:656-661` — pre-filter | target-only skip, mirroring #3 |

Three consequences follow directly from those lines:

1. **The `operation-out` match arm at `topology.rs:282-288` was unreachable.**
   Its two admitted shapes (`restaurant-pos → kds`, `store-pos → warehouse`) are
   exactly the cases the pre-filter at `:510-519` already skipped. Because the
   pre-filter never inspected the source, a wire from *any other* node kind into
   a KDS or warehouse operation input bypassed semantic validation in Rust
   entirely — while the TypeScript gate refused to offer the same wire.
2. **The TypeScript side had the same hole**, at `topologyContract.ts:656-661`,
   and its own second copy of every predicate at `:473-510`. The live-validation
   mirror that ADR #44 built to guarantee "badges agree with Apply" was therefore
   agreeing with Apply *by coincidence of two hand-written copies*, not by
   construction.
3. **The `location` row was not in the match at all.** It is enforced by a
   dedicated block (`topology.rs:450-480`) whose rule is a *reference*, not a
   kind: the wire must originate from the graph's single branch node
   (`from_node_id == branch_id`) and land on a workspace or warehouse id. No
   kind-based table can express that, which is part of why it drifted out.

### This has already bitten once

`CHANGELOG.md:144` and `JOURNAL.md:3953` record the same failure mode in an
earlier form: both sides read the same JSON, but the TypeScript capacity check
counted *any* stock-bearing wire while the Rust guard counted only wires landing
on `stock-in`/`transfer-in`. A stock-routing wire on the ownership port produced
`warehouse-at-capacity` from TypeScript and `invalid-semantic-connection` from
Rust — the same rejection, narrated differently to the merchant. It was masked in
editor flows because `inferredWire` normalizes ports, and surfaced only for
direct IPC callers. The residual drift was fixed rule-by-rule; the *shape* that
produced it was left in place. This ADR addresses the shape.

### Two adjacent findings from the same review

**Two kind vocabularies, no single owner.** `topologySemantics.json:9` declares
`workspaceTypeKeys: ["store-pos", "restaurant-pos", "kds"]`. The editor ignores
that field and hardcodes its own list at `NodeTopologyEditor.tsx:307` —
`['store-pos', 'restaurant-pos', 'kds', 'warehouse']`. Neither list is the
system's real enum: the seeded `workspace_types` table
(`crates/oz-core/migrations/20260813_init.pg.sql:1601-1605`) carries five keys —
`restaurant-pos`, `store-pos`, `warehouse`, `admin`, `kds`. So `warehouse` in
the editor's list is not a bug, and the contract field is not wrong either; it
is the *topology-relevant* subset wearing a name that implies exhaustiveness.
Three lists, three meanings, no declared owner — which is precisely the
condition that lets a fourth appear. §3 gives the registry that ownership.

A related trap surfaced while testing: `general` is a **`purpose_key`**, not a
`type_key` (`topology.rs:596` defaults purpose to `"general"`), yet it appears in
the topology test fixtures in the `typeKey` slot. Any rule written to admit
`general` there would be admitting an unregistered workspace type by accident.

**Per-type knowledge is spread across eleven surfaces.** `topologyCard.ts` opens
with the promise that "adding a new node type should be a one-entry change here —
not a touch in a dozen switch statements" (`:23-25`). In the same file it is
`leftPortVariants` (`:107`), `visiblePortsForNode` (`:116`), `leftPortLabelId`
(`:136`), `portLabelId` (`:157`), `semanticPortLabelId` (`:175`),
`portAriaLabelId` (`:198`), `semanticPortId` (`:212`), `socketSemanticIds`
(`:230`), `gatingSemanticId` (`:274`), `WORKSPACE_SETTINGS_CARD` (`:60`), and
`NODE_TYPE_ICON` (`:456`). Two of these fail *silently* rather than at compile
time: `settingsCardForTypeKey` returns the Store POS card for any unknown
typeKey (`:70`, `?? WORKSPACE_SETTINGS_CARD['store-pos']`), and
`socketSemanticIds` falls through to a generic `['stock-out','transfer-out']`
output set (`:261`). A new workspace typeKey that misses a registration renders a
plausible, wrong graph.

**One correction to the review's own first pass.** An earlier written summary of
this review claimed the editor stylesheet was dark-only by drift, with ~128
hardcoded colours, ~111 `!important`, and 15 ungated animations. Measured
directly, that is wrong and the record is corrected here:
`NodeTopologyEditor.css` is 3,308 lines with **32** hex literals, of which
**29 are `var()` fallbacks** (e.g. `var(--color-success, #10b981)`), and **2**
`!important`. Wire colours are already tokens (`stroke: var(--color-accent)` at
`:858`, `var(--color-info)` at `:884`). There are 9 `@keyframes`, 6
`prefers-reduced-motion` guards, and 2 infinite animations — one correctly gated
behind `no-preference` (`:1098-1102`), one not (`:1931-1933`). The surface is
token-driven by construction; §5 is about making that explicit, not re-skinning
it.

---

## Decision

### 1. Contract v2 — endpoint tuples move into the shared JSON

`schemaVersion` becomes `2`. Every pairing row gains an `endpoints` list of
explicit `{from, to}` **kind tokens**. A row is legal for a given wire iff the
wire's `(sourceKind, targetKind)` tuple appears in that row's list.

```json
{
  "schemaVersion": 2,
  "semanticPairings": [
    {
      "source": "operation-out", "target": "operation-in",
      "relationshipType": "generic", "labelId": "topology-relationship-operation",
      "endpoints": [
        { "from": "workspace:restaurant-pos", "to": "workspace:kds" },
        { "from": "workspace:store-pos",      "to": "warehouse" }
      ]
    },
    {
      "source": "stock-out", "target": "stock-in",
      "relationshipType": "stock-routing", "labelId": "topology-relationship-stock-routing",
      "endpoints": [
        { "from": "workspace:store-pos",      "to": "warehouse" },
        { "from": "workspace:restaurant-pos", "to": "warehouse" },
        { "from": "warehouse",                "to": "warehouse" }
      ]
    },
    {
      "source": "location-out", "target": "location-in",
      "relationshipType": "location", "labelId": "topology-relationship-location",
      "endpoints": [
        { "from": "@branch-root", "to": "workspace" },
        { "from": "@branch-root", "to": "warehouse" }
      ]
    }
  ]
}
```

Three constraints make this safe, and they are the substance of the decision:

- **A closed kind vocabulary, shared by both languages.** A node's kind is one
  string: `branch-location`, `warehouse`, `hardware`, or `workspace:<typeKey>`.
  Both sides already compute the two halves (`node.type` + `metadata.typeKey` in
  TypeScript; `semantic_node_type` + `semantic_type_key` in Rust at
  `topology.rs:261-264`); canonicalizing them into one token turns every
  predicate into a set lookup. `store` folds into `branch-location` at this one
  boundary, so the ADR #34 alias never reaches the contract.
- **Family matching, discovered during implementation.** An endpoint token
  written without a `:` suffix also covers that family: `workspace` admits
  `workspace:store-pos`, while `workspace:store-pos` admits only itself. The
  contract needs both registers — the Location row means "any workspace", the
  Operation row means "this one" — and one prefix rule keeps the comparison a
  line long in each language. Exact-token matching alone, which is what this ADR
  originally specified, refused every legitimate Branch → workspace location
  wire; the first run of the contract tests caught it.
- **No boolean logic in the contract.** Predicates are tuple membership only.
  Rows that need AND/OR — like `operation-out`, whose two admitted pairs are
  *not* the cross product of their endpoints — are written as an explicit pair
  list. An expression language in JSON would need two evaluators, and two
  evaluators is how this ADR's problem was created.
- **`@branch-root` is the only reserved token**, resolving to the graph's single
  Branch Location node. It exists so the location rule
  (`topology.rs:450-480`) can join the table instead of living beside it.
  Whether the graph has *exactly one* such node stays a separate graph-level
  rule (`multiple-branch-locations`), enforced before the wire loop on both
  sides. Unknown kinds, unknown `@`-tokens, and an empty `endpoints` list all
  **fail closed** — a payload that lost its endpoints degrades to "no wire may
  be authored" rather than silently to the looser row-only check, matching how
  unknown semantics already fail closed in `canSemanticPortsConnect`
  (`topologyCard.ts:310-315`).

The Rust `match` (`topology.rs:266-296`) and the TypeScript duplicate switch
(`topologyContract.ts:473-510`) are deleted; both languages call one evaluator
over the contract.

**The two pre-filters were tightened, not deleted** — a deliberate departure
from this ADR's original wording. They exist to route a genuine workspace
operational feed to the specialized `invalid-operation-source` and
`invalid-warehouse-operation-source` checks, so the merchant gets a specific
message instead of the broad one; deleting them would trade a correctness hole
for a worse error. The fix keeps the precedence and closes the hole by testing
the **source** as well as the target: a workspace-sourced operation wire still
skips the generic gate, and a hardware-, warehouse-, or branch-sourced one no
longer slips past it. Folding those two specialized source checks into the
contract is follow-up work, and until it lands the operation row's endpoints are
advisory for authoring while the specialized checks remain the Apply-boundary
authority for that row.

**Deferred, deliberately:** wire *cardinality* (the one-location-wire-per-pair
duplicate check at `topology.rs:450`, and the warehouse primary/operational input
split at `topologySemantics.json:3-8`) stays outside the pairing rows. Folding it
in would mean designing a second schema in the same change.

### 2. A generated cross-language corpus test

Byte parity proves the two files agree. It does not prove the two *evaluators*
agree, and the evaluators are what broke in 2026-08.

`verify-topology-parity.py` gains a second phase: for every combination of
(pairing row × source kind × target kind) drawn from the contract itself plus the
full kind vocabulary, assert that the TypeScript evaluator and the Rust evaluator
return the same verdict. The corpus is generated from the contract, so a new row
or kind is covered automatically — nobody has to remember to write the case that
would have caught them.

This is the part of the decision that makes §1 durable. A schema change without
it leaves the same gap with tidier furniture.

### 3. One node-kind registry, landed behind a behavior freeze

Per-type knowledge collapses into a single `Record<NodeKind, NodeKindSpec>`
keyed by the **same kind token** as the contract, replacing the eleven surfaces
listed above:

```ts
type NodeKindSpec = {
  ports:  { left: readonly SemanticPortId[]; right: readonly SemanticPortId[] };
  labels: Partial<Record<SemanticPortId, string>>;   // Fluent message ids
  icon:   ComponentType;
  settingsCard?: ComponentType;
  renameable: boolean;
};
```

Three specific properties matter more than the consolidation itself:

- **No domain defaults.** The `?? store-pos` settings-card fallback
  (`topologyCard.ts:70`) is removed. With a `Record` over a closed kind union, a
  missing kind is a compile error instead of a plausible wrong inspector.
  Display-layer fallbacks (`topologyUiString`, `:519-525`) stay — those guard
  against a stale locale bundle, which is a different job.
- **Derived registration.** `WORKSPACE_TYPE_KEYS` (`NodeTopologyEditor.tsx:307`)
  becomes a projection of the registry's workspace kinds, which also removes the
  `warehouse` divergence from `topologySemantics.json:9`.
- **A characterization test first.** Before any move, snapshot the current
  `(semantics, label, icon, settingsCard)` tuple for every kind × socket;
  refactor; assert the snapshot is unchanged. This is the spine of a 6,145-line
  editor and ~525 tests in its own suite; a refactor without a freeze is a
  redesign by accident.

### 4. Cold start is "start empty, build deliberately"

This supersedes the preset stance of [ADR #34](./2026-08-07-business-logic-topology-builder.md).
Presets are retired: the editor opens on an empty canvas, and every wire in a
saved graph is a relationship a human chose. That is the direct expression of
"invalid relationships difficult to create" — a graph the merchant did not
understand cannot be a graph they maintain.

Three obligations come with the stance, and they are the reason it is a decision
rather than a deletion:

- **A mandatory root is not a decision, so do not make the merchant place it.**
  The empty state authors the single Branch Location root (`@branch-root` exists
  exactly once per graph by contract) and points at its `location-out` socket.
  "Deliberate" governs wires, not the root the schema requires.
- **Templates must leave `localStorage`.** Saved diagram templates are keyed
  `ozpos.topology.templates.v1` in browser storage (`topologyExport.ts:16`,
  written at `:161`, read at `:171`, enumerated at `:182`). They are
  configuration, and today they do not survive a different device, a profile
  switch, or a reinstall. They move to persisted, per-branch storage alongside
  the graph.
- **The validation panel becomes the checklist.** It already computes the exact
  next omission (`warehouse-missing-stock-routing`, a workspace with no
  `location-in` wire). Presented as "next step" rather than as a red error, it
  carries cold start without a preset — the graph's own contract tells the
  merchant what to do next.

### 5. Theme parity is a verification task, not a re-skin

The editor is already token-driven (see the correction in Context). The decision
is to make that guarantee explicit and testable:

- Replace the two true literals — `stroke: #fff` (`:1574`) and `fill: #fff`
  (`:1579`), used on arrow and checkmark glyphs — with a token that flips.
- Audit the 29 `var()` fallbacks. They currently bake in *dark* values
  (`--color-bg-base, #0a0e1a` at `:3051`, `--color-border, #2a3050` at `:3066`),
  so a missing token silently re-introduces a dark surface in a light theme.
  Fallbacks become neutral or are dropped where the token is guaranteed.
- Gate the one ungated infinite animation, `port-glow` (`:1931-1933`), using the
  house pattern already present at `:1098-1102` (`prefers-reduced-motion:
  no-preference`).
- Delete `.topology-shortcuts-popover` from `ui/src/frontend/themes/components.css`;
  its component no longer exists. It is the only dead selector of the 11
  topology surfaces that file references.
- Verify the canvas, wires, and node cards in light theme as a gate, not by
  inspection.

---

## Consequences

**Positive**

- One rule, one place, both languages. Adding a node kind becomes: register it in
  the contract, register its `NodeKindSpec`, and the corpus test covers the new
  combinations without anyone writing them.
- The silent-failure paths (`?? store-pos`, the generic output fallthrough at
  `topologyCard.ts:261`, the source-blind pre-filter at `topology.rs:510`) become
  compile errors or test failures.
- The unreachable arm and the location-rule special case stop existing, so the
  "Apply button can never be a lie" property of ADR #44's live mirror stops
  depending on two hand-written rule sets staying in step.
- Cold start has a stated philosophy with three concrete obligations, instead of
  a preset removal that leaves a hole.

**Costs and risks**

- A `schemaVersion` bump means both evaluators must accept v1 payloads during the
  transition, or persisted graphs must migrate. v1 rows carry no `endpoints`, so
  the loader needs an explicit "v1 ⇒ admit the row, defer to the legacy path"
  rule that is itself fail-closed. This is the main risk in §1 and the reason §1
  lands before §3.
- The corpus test needs a way to invoke the Rust evaluator from the parity
  script. A `--export-pairing-matrix` debug command, or a Rust test that writes
  the matrix to a fixture the script reads, both work; the choice is an
  implementation detail, but the test is not optional.
- The registry refactor touches the same file as the in-flight preset removal.
  It must land after that work settles, or the characterization snapshot will
  capture a moving tree.
- Retiring presets makes the first-run experience worse until the §4 obligations
  ship. They are part of the decision, not follow-ups to it.

---

## Rollout

| Slice | Scope | Touches | Depends on |
|---|---|---|---|
| **A** | Contract v2 + both evaluators + corpus test; delete the Rust `match`, the pre-filter, and `operationRowAllowed` | `topologySemantics.json` ×2, `topologyCard.ts`, `topology.rs`, `verify-topology-parity.py` | — |
| **B** | Kind registry + characterization snapshot; derive `WORKSPACE_TYPE_KEYS` | `topologyCard.ts`, `NodeTopologyEditor.tsx` | A |
| **C** | Cold start: auto-place root, templates to persisted storage, validation panel as checklist | `TopologyScreen.tsx`, `topologyExport.ts`, api + IPC command | the preset removal settling |
| **D** | Theme parity: 2 literals, 29 fallbacks, 1 animation gate, 1 dead selector, light-theme verification | `NodeTopologyEditor.css`, `themes/components.css` | independent |

A and D are independent of each other and of B/C. A is first because it closes a
live, silent, cross-language failure mode with a recorded precedent.

---

## Non-goals

- No new node kinds, relationship types, or ports. This ADR changes where rules
  live, not what they permit.
- No canvas, selection, wire-editing, or viewport behaviour changes.
- No light-theme *design* pass — §5 makes the existing token discipline hold, it
  does not restyle the editor.
- No migration of persisted graphs to canonical `branch-location` naming. ADR #34
  §1 defers that; this ADR neither advances nor reverses it.

---

## Verification

- `python3 scripts/verify-topology-parity.py` — byte parity **and** the generated
  corpus verdict comparison.
- `cargo test -p oz-core topology` — the vendored-contract drift test plus the
  semantic validation suites.
- `npm run typecheck && npm run lint && npm run test` from `ui/` — the registry
  snapshot must be byte-identical to the pre-refactor capture.
- Manual: one graph per pairing row, applied through the real Apply gate, in both
  themes.

---

## Implementation record

### §1 — contract v2 (shipped 2026-09-02, `e93868e2` + `b4399aa9`)

`schemaVersion` is `2`; all seven rows carry `endpoints`. The TypeScript
evaluator is `nodeKindToken` / `nodeKindOf` / `pairingAllowsEndpoints` /
`pairingAdmitsKinds` / `pairingAllowsNodes` in `topologyCard.ts`, and the Rust
twin is `node_kind_token` / `kind_token_admits` / `pairing_admits_kinds` in
`topology.rs`. Deleted: `operationRowAllowed`, the `semanticNodesMatchWire`
switch, and the Rust `match` with its unreachable arm. Both pre-filters now test
the source as well as the target.

**One merchant-visible behaviour change, intended.** A workspace whose
`typeKey` the contract does not declare is no longer authorable as a stock or
transfer source. Previously the canvas offered that wire and the Rust gate
rejected it, so the merchant was invited to draw a relationship that could never
persist. An unregistered type becomes authorable by declaring its endpoint rows.

**Evidence.** `cargo test -p oz-core --lib topology` 47/47 (5 new);
`topologyCard.test.ts` + `topologyContract.test.ts` 94/94 (3 new); the eight
topology component suites 312/312; `tsc --noEmit` clean; `eslint` clean;
`verify-topology-parity.py` OK. In the full UI suite, 11 tests fail — all 11
reproduce with this change stashed, and every one names a preset fixture
(`Downtown Branch`, `New Retail POS`), a `.topology-shortcuts-popover` CSS rule
its component no longer has, or a hardcoded `18px` port padding: the concurrent
preset-removal work, not this contract.

### §2 — corpus test (not shipped)

Interim measure: the TypeScript and Rust evaluator tests assert the **same**
verdicts case by case, so a divergence in either language fails one of the two
suites. This is mirroring, not generation — it catches drift in the evaluator
but not drift in coverage when a row or kind is added. The generated corpus
remains the real fix and still gates closing this ADR.

### Follow-ups this slice surfaced

1. `socketSemanticIds` (`topologyCard.ts:230-262`) still hands an unregistered
   workspace type the generic `['stock-out','transfer-out']` output set
   (`:261`). The card therefore advertises sockets that can never legally
   connect — the exact "discoverable validity" violation ADR #34 §Context names.
   Folding this into the §3 registry is the fix.
2. The `invalid-operation-source` / `invalid-warehouse-operation-source` checks
   are a seventh and eighth statement of the operation row's endpoints. They
   give better messages and should stay, but the contract should generate them.
3. Three workspace-type lists with three meanings and no owner (Context above).
   §3 assigns the ownership.

---

## Related decisions

- [ADR #22: Visual Node-Based Store & Workspace Topology Builder](./2026-07-20-node-based-store-topology-builder.md)
- [ADR #34: Topology Editor as the Business Logic Builder](./2026-08-07-business-logic-topology-builder.md) — §Presets superseded by §4 here
- [ADR #44: Typed Connection Gating & Live Validation](./2026-08-08-adr34-typed-connection-gating.md) — §1 completes its single-source-of-truth goal
- [ADR #41: App Lifecycle, Device Onboarding, Dynamic Topology Workspaces](./2026-08-28-adr41-app-lifecycle-device-onboarding-topology-home-gating.md)
