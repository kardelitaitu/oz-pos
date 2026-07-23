# Node Topology Editor — Weakness & Problem Analysis

Scope reviewed: `NodeTopologyEditor.tsx` (1213 lines), `NodeTopologyEditor.css`,
`TopologyScreen.tsx`, `api/topology.ts`, `commands/topology.rs`, both test files,
and ADR #22.

Each item below is a checkbox. Audit verdicts (verified against the `0.0.19`
branch on 2026-07-23) and detailed solutions are folded into each point.

Statuses: **VERIFIED** · **STRENGTHENED** · **NUANCED** · **REFUTED**

---

## Critical (data-loss / breakage)

- [x] **#1 — Changing a workspace's type is silently lost on save** · 🔴 · STRENGTHENED

  `TopologyScreen.tsx:118-123` — the update branch only diffs and persists `name`:

  ```typescript
  } else if (existing.name !== node.name) {
    await updateWorkspaceInstanceScoped(sessionToken, node.id, { name: node.name });
  ```

  The inspector lets the user switch `typeKey` (`store-pos` → `restaurant-pos` →
  `kds`) at `NodeTopologyEditor.tsx:1176-1185`, and the canvas re-renders the new
  settings card. But `type_key` is never sent on update, so on reload
  `workspaceSeed` (`TopologyScreen.tsx:62-73`) maps the old `type_key` back from
  the backend. The user's change is visually applied then silently reverted.

  **Why STRENGTHENED:** the surface symptom (screen omits `type_key`) is only
  half the story. `ui/src/api/workspaces.ts:101-106` documents the real
  constraint:

  ```ts
  /** Editable fields of a workspace instance. `type_key` and `store_id` are immutable. */
  export interface UpdateInstanceFields { name: string; description?: string; colour?: string; }
  ```

  `type_key` is **immutable by backend contract** — the update command does not
  accept it at all. So "just add `type_key` to the payload" is not a valid fix.
  The inspector's type selector is a visual-only control backed by no
  persistence path.

  **Solution:**
  1. **MVP (honest, low-risk):** make the type selector read-only for *persisted*
     workspace nodes (`metadata.persisted === true`) with a tooltip:
     "Workspace type can't be changed after creation. Archive and recreate to
     switch type." Allow selection only for freshly-added (`persisted: false`)
     nodes before first save. This stops the silent-revert footgun immediately.
  2. **Full fix:** implement archive + recreate on type change. In
     `handleTopologySave`, diff `typeKey` against `loadedById` (not just `name`).
     When a persisted node's `typeKey` changed:
     - `archiveWorkspaceInstanceScoped(token, oldId)`
     - `createWorkspaceInstanceScoped(token, { id: newId, type_key: newTypeKey, store_id, name })`
       where `newId = crypto.randomUUID()` (see #9)
     - remap every wire referencing `oldId` → `newId`
     - remap the saved diagram node id so position is preserved
  3. **ADR #22 amendment:** document that type change = archive + recreate, and
     that the diff modal (#4) must surface this as a destructive operation
     requiring confirm.

  **Files:** `TopologyScreen.tsx`, `NodeTopologyEditor.tsx` (inspector gate),
  `docs/decisions/2026-07-20-node-based-store-topology-builder.md`.

- [x] **#2 — Wire deletion is not undoable** · 🔴 · VERIFIED

  `NodeTopologyEditor.tsx:418-431` — `executeDelete` calls `pushHistory()` for
  node deletion (line 425) but **not** for wire deletion (lines 420-423). The
  keyboard path (line 380-383) routes wire-delete through the same
  `executeDelete`. Ctrl+Z restores node moves/adds/deletes but a deleted wire is
  gone forever. The toggle (633) and add (663) paths *do* push history, making
  this an inconsistent gap.

  **Solution:**
  1. Add `pushHistory();` as the first statement inside the
     `if (confirmDelete === '') {` branch of `executeDelete`
     (`NodeTopologyEditor.tsx:419`), before `setWires(...)`.
  2. Audit the other two wire-deletion entry points for parity:
     - immediate node-delete paths (376-378, 680-684) already call
       `pushHistory()` — leave as is; they don't touch wires anyway unless the
       node has wires (which routes to confirm dialog).
  3. Add a regression test in `NodeTopologyEditor.test.tsx`: delete a wire via
     label-area delete, click Undo, assert the wire path reappears in the SVG.

  **Files:** `NodeTopologyEditor.tsx`, `NodeTopologyEditor.test.tsx`.

- [x] **#3 — Keyboard shortcuts fire while editing text fields** · 🔴 · VERIFIED

  `NodeTopologyEditor.tsx:360-409` — the global `window.addEventListener('keydown', …)`
  handles `Delete`/`Backspace` (368), `Ctrl+Z` (385), and arrows (390) with **no
  guard** for `input`/`textarea`/`contentEditable` targets. The inspector's
  "Node Name" / "Subtitle" inputs (1145, 1157) sit inside the same component.
  While typing a node's name and pressing Backspace to fix a typo,
  `e.preventDefault()` (369) blocks the character deletion and the handler
  either opens the delete-confirm dialog (node has wires) or **deletes the entire
  node** (no wires, 376-378). Reproducible corruption.

  **Solution:**
  1. Add an editable-target guard at the top of `onKeyDown`:

     ```ts
     const onKeyDown = (e: KeyboardEvent) => {
       const t = e.target as HTMLElement | null;
       const editing =
         t && (t.tagName === 'INPUT' || t.tagName === 'TEXTAREA' || t.isContentEditable);
       if (editing) return; // let the field handle the keystroke
       // …existing Escape / Delete / Ctrl+Z / Arrow handlers
     };
     ```

  2. Place the guard *before* the Escape check so no canvas shortcut fires while
     a text field is focused. (Escape in an input will blur via default
     behaviour; if a "blur + clear selection" is desired later, handle it
     explicitly rather than via the canvas handler.)
  3. Add tests: focus the Node Name input, fire `Backspace`, assert no
     `.topology-node` is removed and the input value changes.

  **Files:** `NodeTopologyEditor.tsx`, `NodeTopologyEditor.test.tsx`.

- [x] **#4 — Non-atomic save sequence contradicts the "atomic commit" promise** · 🔴 · VERIFIED

  `TopologyScreen.tsx:96-164` — create / update / archive / `saveTopology` are
  sequential `await`s with no transaction or rollback. If
  `createWorkspaceInstanceScoped` succeeds for 2 of 3 new nodes and the 3rd
  throws, the `catch` (177) shows a toast but the partial creates are committed
  and the diagram (`saveTopology`) is never persisted. ADR #22 §4.5 promises a
  "TopologyDiffModal … before committing atomically" — neither the modal nor
  atomicity exists (no `TopologyDiffModal` component anywhere in `ui/src`).

  **Solution (two parts):**
  1. **Backend atomic diff command.** Add a Tauri command
     `apply_workspace_topology_diff` in `apps/desktop-client/src/commands/` that
     accepts the workspace node list, diffs against current instances inside the
     handler, and runs all create + update + archive operations in **one SQLite
     transaction** (`conn.transaction()`). On any error, roll back and return
     the error; on success, return the fresh `WorkspaceDto[]`. Register it in
     `lib.rs`. This replaces the N sequential `await`s with one transactional
     call — partial creates become impossible.
  2. **TopologyDiffModal.** Implement the promised preview modal: before
     calling the diff command, show a list of "will create / will rename / will
     archive" entries with a Confirm/Cancel. The modal calls the atomic command
     only on Confirm.
  3. Diagram persistence (`saveTopology`) stays a separate call, but run it
     *only after* the diff command succeeds, so the diagram never drifts from
     the relational truth. (If the diagram save then fails, the relational state
     is still consistent; show a non-fatal toast and keep the nodes as-is.)

  **Files:** `apps/desktop-client/src/commands/` (new command),
  `apps/desktop-client/src/lib.rs` (register), `api/workspaces.ts` or
  `api/topology.ts` (client fn), `TopologyScreen.tsx` (use new command),
  new `TopologyDiffModal.tsx`, `NodeTopologyEditor.tsx` (wire modal to Apply).
  This is large — may warrant its own spec/ADR update.

- [x] **#5 — `store_id` binding ignores topology wires** · 🔴 · VERIFIED

  `TopologyScreen.tsx:112-115` — a new workspace's `store_id` is picked from
  "the first loaded instance's store_id, else the first primary store." ADR §2
  specifies "Store ─> Workspace: Binds a workspace instance to a store location,"
  but the save logic never reads which `Store` node the workspace wire connects
  to. In a multi-store canvas, a register drawn under Store B gets silently
  bound to Store A. The visual graph is decorative for store binding.

  **Solution:**
  1. **Seed Store nodes from real `store_profiles`** (like workspaces are seeded
     from `workspace_instances`). `TopologyScreen.tsx` already loads `stores`
     (41-44). Build a `storeSeed` and pass it to the editor so each store node's
     `id` equals the real `store_id` (currently store nodes are diagram-only
     with arbitrary ids like `'store-1'`).
  2. **Resolve `store_id` from wires at save time.** For each new workspace
     node, find wires where the *other* endpoint is a `type === 'store'` node
     (either direction) and use that store node's `id` as `store_id`:

     ```ts
     const boundStoreId = wires
       .filter((w) => (w.fromNodeId === node.id || w.toNodeId === node.id))
       .map((w) => (w.fromNodeId === node.id ? w.toNodeId : w.fromNodeId))
       .map((id) => nodes.find((n) => n.id === id))
       .find((n) => n?.type === 'store')?.id;
     const store_id = boundStoreId ?? fallback;
     ```

  3. Keep the current fallback (first primary store) only when no Store→Workspace
     wire exists.
  4. Add tests in `TopologyScreen.test.tsx`: two store nodes + a workspace
     wired to Store B → assert `createWorkspaceInstanceScoped` is called with
     Store B's `store_id`.

  **Files:** `TopologyScreen.tsx`, `NodeTopologyEditor.tsx` (accept store seeds),
  `TopologyScreen.test.tsx`.

---

## Major (correctness / UX)

- [x] **#6 — Undo logic uses impure side-effects inside a state updater** · 🟠 · NUANCED

  `NodeTopologyEditor.tsx:330-342` — `popUndo` calls `setNodes`/`setWires`
  **inside** the `setHistory` updater, plus a `setTimeout` to flip
  `undoInProgressRef`. React 18 StrictMode double-invokes updaters;
  setState-via-side-effect-in-another-setState-updater is an anti-pattern.

  **Why NUANCED (corrected from the original "will break"):** `prev.slice(0, -1)`
  is pure, and the nested `setNodes(entry.nodes)` calls are idempotent, so the
  StrictMode double-fire commits once harmlessly. The `undoInProgressRef` +
  `setTimeout(0)` guard correctly suppresses `pushHistory` capturing the
  restore. So it works today — but it's fragile and unmaintainable, and it
  blocks a clean redo implementation (#7).

  **Solution:** refactor undo into a single owner of truth.
  1. **Preferred:** replace the four `useState` calls (`nodes`, `wires`,
     `history`, plus the redo stack from #7) with one `useReducer`:

     ```ts
     type TopologyState = { nodes: TopologyNodeData[]; wires: TopologyWireData[];
                            history: HistoryEntry[]; redo: HistoryEntry[]; };
     type TopologyAction =
       | { kind: 'add-node'; node: TopologyNodeData }
       | { kind: 'move-node'; id: string; x: number; y: number }
       | { kind: 'delete-node'; id: string }
       | { kind: 'add-wire'; wire: TopologyWireData }
       | { kind: 'toggle-wire'; id: string }
       | { kind: 'undo' } | { kind: 'redo' } | { kind: 'load'; … };
     ```

     Each action is a pure transition that also pushes/clears the history+redo
     stacks. Removes nested setState and the `undoInProgressRef` hack entirely.
  2. **Lighter alternative** if a full reducer is too big a change: keep
     `useState` but make `popUndo` read `history` from a ref (`historyRef`),
     then issue `setHistory`, `setNodes`, `setWires` as three sibling top-level
     calls (not nested). Drop `undoInProgressRef`/`setTimeout`.

  **Files:** `NodeTopologyEditor.tsx`.

- [x] **#7 — No Redo despite ADR commitment** · 🟠 · VERIFIED

  ADR §4.6 lists "Ctrl+Z / Ctrl+Y undo/redo." Only undo is implemented
  (`NodeTopologyEditor.tsx:385-388`). There's no redo stack and no
  `Ctrl+Y`/`Ctrl+Shift+Z` handler.

  **Solution:**
  1. Add `redo: HistoryEntry[]` to state (free if #6's reducer is adopted — the
     reducer carries both stacks).
  2. Standard undo/redo semantics:
     - every mutating action pushes the *previous* state to `history` and
       **clears `redo`** (a new edit invalidates the redo branch);
     - `undo` pushes current state to `redo`, pops `history` → restore;
     - `redo` pushes current state to `history`, pops `redo` → restore.
  3. Keyboard: add `Ctrl+Y` and `Ctrl+Shift+Z` → `redo()` in the `onKeyDown`
     handler (guarded by the #3 input-element check).
  4. UI: show a "Redo (Ctrl+Y)" button next to Undo, enabled when `redo.length > 0`.
  5. Tests: undo then redo, assert state returns to post-edit; make a new edit
     after undo, assert redo stack is cleared.

  **Files:** `NodeTopologyEditor.tsx`, `NodeTopologyEditor.test.tsx`.

- [x] **#8 — Post-save reload can clobber in-flight edits** · 🟠 · VERIFIED

  `NodeTopologyEditor.tsx:204-306` — the load effect depends on
  `[workspaceInstances]` (line 306). After save, `TopologyScreen` refreshes
  instances (`TopologyScreen.tsx:173`), the `workspaceSeed` memo recomputes, the
  prop changes, and the effect re-runs a full `setNodes`/`setWires` from the
  backend (lines 268-269). Any node moved *after* clicking Apply but before the
  reload resolves is overwritten. Also a redundant backend round-trip on every
  save.

  **Solution:**
  1. **Don't rebuild the diagram after our own save.** Add a `skipNextLoadRef`
     that the editor sets (via a callback prop or a `saveGeneration` counter)
     before `TopologyScreen` triggers its post-save refresh. In the load
     effect, if `skipNextLoadRef.current` is set, skip the full
     `setNodes`/`setWires` rebuild and only update `metadata.persisted = true`
     on the affected workspace nodes (a targeted map), then clear the flag.
  2. **Better separation:** split the effect into two — one mount-only effect
     (empty deps) that loads the saved diagram + instances the first time, and
     a second effect that reacts to `workspaceInstances` changes *only* to
     reconcile `persisted` flags and archive-removals, never overwriting local
     positions.
  3. If the backend diff command from #4 returns the fresh instance list,
     treat that as the single source and update `persisted` flags in place
     rather than re-issuing `listWorkspacesScoped`.

  **Files:** `NodeTopologyEditor.tsx`, `TopologyScreen.tsx`.

- [x] **#9 — Client-generated IDs become DB primary keys** · 🟠 · VERIFIED

  `NodeTopologyEditor.tsx:575` (`${type}-${Date.now()}`) and `:648`
  (`wire-${Date.now()}`) — two adds within the same millisecond collide. Worse,
  the node id is sent as `id: node.id` to `createWorkspaceInstanceScoped`
  (`TopologyScreen.tsx:110`), making a client timestamp the `instance_id`
  primary key.

  **Solution:**
  1. **Minimum viable (kills collisions):** replace `Date.now()` with
     `crypto.randomUUID()`:
     - `const id = \`${type}-${crypto.randomUUID()}\`;` (575)
     - `const newWireId = \`wire-${crypto.randomUUID()}\`;` (648)
     UUIDs are collision-resistant even across rapid adds. This alone removes
     the same-millisecond PK collision.
  2. **Stronger (backend owns the PK):** have `createWorkspaceInstanceScoped`
     return the created `WorkspaceDto`; in `handleTopologySave`, after a
     successful create, remap the canvas node `id` (and every wire endpoint
     referencing it) from the client temp id to the returned `instance_id`,
     then persist the diagram with the real id. This matches the project norm
     of backend-generated PKs and also fixes the "saved diagram references a
     temp id" drift. Coordinate with #1's archive+recreate remap.
  3. Tests: rapid double-add (same tick) asserts two distinct ids; save with a
     temp id asserts the diagram is persisted with the backend id.

  **Files:** `NodeTopologyEditor.tsx`, `TopologyScreen.tsx`,
  `TopologyScreen.test.tsx`.

- [x] **#10 — Legacy wires default `toPort` to `'right'`, drawing backwards curves** · 🟡 (re-severeanced from 🟠) · VERIFIED

  `NodeTopologyEditor.tsx:974` — `const toPort = wire.toPort ?? 'right';` matches
  the type comment, but a wire saved with no ports (legacy/demo) connects
  right-edge → right-edge, producing a visually wrong backwards arc. The
  natural target for an unspecified port is `'left'`.

  **Why re-severeanced:** every UI-created wire sets explicit `fromPort`/`toPort`
  (`handlePortClick`, 656), and both presets set explicit ports (104-105,
  119-122). So this only bites hand-edited or old persisted JSON — cosmetic, not
  a live bug.

  **Solution:**
  1. Change the render default at line 974 to `'left'`:
     `const toPort = wire.toPort ?? 'left';` (`fromPort ?? 'right'` at 973 is
     fine and stays).
  2. Optionally migrate on load: in the load effect, if a wire has no
     `fromPort`/`toPort`, assign `fromPort='right', toPort='left'` so the next
     save normalises it. Lower priority — the render default already hides it.

  **Files:** `NodeTopologyEditor.tsx` (one-line change at 974; optional load
  migration).

- [x] **#11 — Backend accepts any string as node type / port / direction** · 🟠 · VERIFIED

  `topology.rs:84` (`node_type: String`), `:120` (`direction: String`),
  `:128-131` (ports as `Option<String>`). Tests even codify `"bidirectional"`
  and `"usb"`/`"network"` being preserved (`topology.rs:938-942`, `:617`).
  There's no enum validation, so corrupt or hand-edited persisted JSON with
  `type: "foo"` loads silently and renders as a blank card. The frontend's
  union types (`NodeType`, `WireDirection`, `PortName`) are cast with `as` on
  load (`NodeTopologyEditor.tsx:214, 220, 222, 261, 264-265`) with no runtime
  check.

  **Solution:**
  1. **Backend enums.** Replace the loose `String` fields in `TopologyNodePayload`
     / `TopologyWirePayload` with serde enums:

     ```rust
     #[serde(rename_all = "kebab-case")]
     enum NodeType { Store, Workspace, Warehouse, Hardware }
     #[serde(rename_all = "kebab-case")]
     enum WireDirection { OneWay, TwoWay }
     #[serde(rename_all = "kebab-case")]
     enum PortName { Top, Right, Bottom, Left }
     ```

  2. **Backward compat:** to avoid breaking old saves with unknown values, add
     `#[serde(other)]` fallback variants (e.g. `NodeType::Unknown`) that
     `save_topology_data` rejects on write and `load_topology_data` coerces to a
     safe default with a logged warning. Decide fail-closed on save, coerce on
     load.
  3. **Frontend runtime guards.** Replace the bare `as NodeType` casts on load
     with a parser: `function parseNodeType(s: string): NodeType | null` and
     drop/flag nodes that don't match. Same for direction and ports.
  4. **Update tests** that codify `"bidirectional"`, `"usb"`, `"network"` being
     preserved — these must now assert rejection (save) or coercion (load)
     instead. Add tests for the enum round-trip of the four valid node types
     and two directions.

  **Files:** `apps/desktop-client/src/commands/topology.rs` + its tests,
  `NodeTopologyEditor.tsx` (load guards).

---

## Moderate (performance / i18n / a11y)

- [x] **#12 — Massive i18n violation — most strings are hardcoded English** · 🟡 · VERIFIED

  AGENTS.md mandates `@fluent/react` for all user-visible strings, but only **2**
  of ~30 are localized (`topology-builder-title` at 811,
  `workspace-type-selector-label` at 1171 — grep confirms exactly two
  `<Localized>` in the file). Hardcoded: every button ("Test Order Simulation",
  "Retail Preset", "Resto & KDS Preset", "Apply Topology Changes", "+ Store
  Node", "Fit All", "Reset View", "Undo (Ctrl+Z)"), all tool-rack descriptions,
  `WORKSPACE_TYPE_OPTIONS` labels (87-92), the `getTelemetry` badges ("Active",
  "Unconfigured", "KDS Ready", "Receipt ✓", "Receipt 58mm" — 753-770), and both
  `ConfirmDialog` messages (788-789, 803). These ship untranslated to `id`/`th`.

  **Solution:**
  1. **Enumerate every user-visible string** (~28) and assign a Fluent key.
  2. **Place keys** in the existing `ui/src/locales/multi-store.ftl` (where
     `topology-builder-title` already lives) plus its `.id.ftl` / `.th.ftl`
     siblings — lower-friction than a new bundle and matches current convention.
     (If the count is large, a dedicated `topology.ftl` bundle registered in the
     localization loader is cleaner; pick one and stay consistent.)
  3. **Wrap JSX** with `<Localized id="…">fallback</Localized>`.
  4. **Non-JSX strings** (telemetry badges, `WORKSPACE_TYPE_OPTIONS` labels) use
     `l10n.getString('…')` via `useLocalization()`.
  5. **Run the pre-commit i18n gates** (`scripts/lint-i18n.sh`,
     `scripts/verify-bundle-parity.py --staged-only`, `scripts/dedupe-ftl.py
     --dry-run`) so all three bundles stay in parity and no `<Localized id>`
     references a missing key.
  6. Tests: assert key presence in all three `.ftl` files for each new id.

  **Files:** `NodeTopologyEditor.tsx`, `multi-store.ftl` (+ `.id`/`.th`),
  possibly a new `topology.ftl` + loader registration.

- [x] **#13 — O(n·m) wire rendering + full re-render every drag tick** · 🟡 · VERIFIED

  `NodeTopologyEditor.tsx:967-980` — `wires.map` calls `nodes.find` for each wire
  (two finds per wire). With 200 nodes / 300 wires that's 60k lookups per
  render. During drag, `handleCanvasMouseMove` (474-476) calls `setNodes` with a
  full array map on every `mousemove`, re-rendering the entire node list + SVG
  each tick. No `React.memo` on node components, no `useMemo` on wire geometry.
  The simulation interval (353-356, 30ms) also re-renders the whole canvas every
  frame.

  **Solution:**
  1. **Node map.** `const nodeMap = useMemo(() => new Map(nodes.map(n => [n.id, n])), [nodes]);`
     and look up `nodeMap.get(wire.fromNodeId)` in the wires render — O(m)
     instead of O(n·m).
  2. **Memoize node components.** Extract a `TopologyNodeCard` child wrapped in
     `React.memo` keyed on the fields it consumes (`id, type, name, subtitle,
     x, y, selected, telemetry`). Dragging one node then re-renders only that
     card + the SVG, not all nodes.
  3. **Drag without per-tick setState.** During drag, mutate a transform via a
     ref / direct DOM style on the dragged card and commit to `setNodes` once on
     `mouseup` (or throttle with `requestAnimationFrame`). Eliminates the
     per-mousemove full-array map.
  4. **Isolate the simulation pulse.** Move the `<circle>` pulse into its own
     component subscribed only to `simPulseStep`, so the 30ms tick doesn't
     re-render the node list.
  5. **Memoize wire path geometry** with `useMemo` keyed on `nodeMap` + `wires`
     (or per-wire via the memoized card).
  6. Optional: add a perf test asserting render count stays constant as node
     count grows (React Profiler).

  **Files:** `NodeTopologyEditor.tsx` (refactor into memoized children + map).

- [x] **#14 — Fixed 5000×5000 SVG bounds silently clip** · 🟡 · VERIFIED

  `NodeTopologyEditor.css:204-205` — `.node-wires-svg { width: 5000px; height: 5000px; }`.
  Nodes positioned beyond x/y ≈ 5000 (possible after heavy panning/zooming out)
  have wires that vanish. The viewport is unbounded but the wire layer isn't.

  **Solution:**
  1. **Size the SVG to content bounds.** Compute the node bounding box in a
     `useMemo`:

     ```ts
     const svgBounds = useMemo(() => {
       if (nodes.length === 0) return { w: 0, h: 0 };
       const maxX = Math.max(...nodes.map(n => n.x + NODE_WIDTH));
       const maxY = Math.max(...nodes.map(n => n.y + NODE_HEIGHT));
       return { w: maxX + 200, h: maxY + 200 }; // padding
     }, [nodes]);
     ```

     and set `style={{ width: svgBounds.w, height: svgBounds.h }}` on the svg.
  2. Remove the fixed `width/height: 5000px` from the CSS (let inline style
     win, or delete the rules).
  3. Since the SVG sits inside the transformed `.node-canvas-viewport`, the
     bounds track the node layer exactly — no clipping regardless of pan/zoom.

  **Files:** `NodeTopologyEditor.tsx`, `NodeTopologyEditor.css`.

- [x] **#15 — `confirmDelete` uses a stringly-typed sentinel** · 🟡 · VERIFIED

  `NodeTopologyEditor.tsx:419` — `''` means "delete wire," a real id means
  "delete node," `null` means closed. `executeDelete` branches on
  `confirmDelete === ''` vs truthy. A discriminated union would remove the
  ambiguity.

  **Solution:**
  1. Replace the state type:

     ```ts
     type DeleteTarget =
       | { kind: 'node'; id: string }
       | { kind: 'wire' }
       | null;
     const [confirmDelete, setConfirmDelete] = useState<DeleteTarget>(null);
     ```

  2. Update `executeDelete` to switch on `confirmDelete?.kind`:

     ```ts
     if (confirmDelete?.kind === 'wire') { /* pushHistory + delete wire */ }
     else if (confirmDelete?.kind === 'node') { /* pushHistory + delete node + wires */ }
     setConfirmDelete(null);
     ```
     (Fold in the #2 `pushHistory()` fix for the wire branch here.)

  3. Update callers: keyboard handler (372-383) → `setConfirmDelete({kind:'node', id: selectedNodeId})`
     and `setConfirmDelete({kind:'wire'})`; `handleDeleteRequest` (674-688)
     likewise.
  4. Dialog render (785-790): `confirmDelete?.kind === 'node'` for title/message.
  5. Tests still pass structurally; add one asserting a node delete with an id
     that happens to be empty-string can't be confused with a wire delete.

  **Files:** `NodeTopologyEditor.tsx`.

---

## Minor

- [x] **`freshNodeIds` setTimeout leak** · VERIFIED · `NodeTopologyEditor.tsx:594-596`

  The 400ms cleanup fires `setFreshNodeIds` on an unmounted component if it
  unmounts first. Harmless in React 18 (no-op warning) but noisy.

  **Solution:** capture a mounted flag or clear the timeout on unmount. Add a
  ref `const freshTimersRef = useRef<Set<ReturnType<typeof setTimeout>>>(new Set())`,
  register each timer, and clear them all in the existing unmount effect
  (345-349).

- [x] **`zoomToFit` spread on empty** · VERIFIED · `NodeTopologyEditor.tsx:435-438`

  `Math.min(...[])` / `Math.max(...[])` returns `Infinity`/`-Infinity`. Guarded
  by the `nodes.length === 0` early return at 434, so not live, but fragile.

  **Solution:** harden the early-return guard and/or compute bounds via a reduce
  that defaults to 0 on empty: `nodes.reduce((acc, n) => Math.min(acc, n.x), 0)`.

- [x] **No focus-visible style on node cards** · VERIFIED · `NodeTopologyEditor.css:277-296`

  Ports (517) and the canvas container (183) have `:focus-visible` outlines, but
  `.topology-node` (the `role="button"` element) has none — keyboard users get no
  visible focus ring on nodes.

  **Solution:** add

  ```css
  .topology-node:focus-visible {
    outline: 2px solid var(--color-border-focus);
    outline-offset: 2px;
  }
  ```

- [x] **`panCleanupRef` unmount guard duplicates `handleMouseUp`** · VERIFIED · `NodeTopologyEditor.tsx:529-544`

  The cleanup mirrors the handler; the unmount effect (345-349) is correct but
  the dual cleanup is easy to drift out of sync.

  **Solution:** keep one cleanup path — let `handleMouseUp` call
  `panCleanupRef.current?.()` then null it, and the unmount effect also calls
  `panCleanupRef.current?.()`. Remove the inline `removeEventListener` duo from
  `handleMouseUp` so the logic lives in exactly one place
  (`panCleanupRef.current`).

- [x] **`getTelemetry` for `warehouse` is a stub** · VERIFIED · `NodeTopologyEditor.tsx:765-770`

  Always returns "Active / online" with a TODO for inventory — acknowledged in
  code, but ships a misleading badge.

  **Solution:** until the inventory scope is wired into `SettingsContext`
  (Phase 3+), show a neutral, honest badge like `"Inventory: n/a"` (or `null`,
  which the renderer already handles by falling back to `node.telemetryBadge`).
  When `settings.inventory` lands, replace with a real low-stock count + a
  `warning` status when below threshold.

---


No finding needs to be withdrawn; the recommended fix order still stands, with
#1 now understood to require an archive+recreate path (or an ADR #22 amendment)
rather than a one-line payload fix. Suggested sequence: #3 (input guard) and #2
(wire undo) first (small, high-impact), then #1 as a design decision, then the
larger #4/#5 block (atomic save + store binding — likely a spec/ADR).
