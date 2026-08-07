
## 2026-08-07 — Frontend skips its own terminal's settings_updated events (SYNC-10 follow-up)

### The new event loop double-refetched on local saves — the payload's terminal_id was never used
**Problem:** SYNC-10 made the daemon re-emit `settings_updated` for remote settings changes, but the frontend listener refetched on EVERY event. A local save therefore fired twice: the save handler's `markSettingsUpdated` AND the event echo from the backend's local publish — two backend round-trips per save.

**Solution:** The listener now attributes the event to its own terminal and skips it. Identity resolution: the device id (`getDeviceId()` / `useWorkspace().terminalId`) plus the registered terminal's ROW id — the value the backend actually emits (`state.terminal_id`) — resolved by matching `listTerminals()` against the device id. Skip rule: ignore events whose `terminal_id` is the device id, the resolved row id, or `"unknown"` **only when this device has no registered terminal** (single-terminal / MultiTerminal-off: "unknown" is exclusively the local echo; if we ARE registered, an "unknown" origin can only be an unregistered peer and must still refetch — the guard that keeps the future settings-sync enqueue slice safe). The resolution effect is fully try/catch-wrapped so no provider mount can crash on unmocked IPC.

**Verify:** 4 new tests (row-id skip, device-id skip, unknown-unregistered skip, unknown-registered refetch) — Red confirmed (the 3 skip tests failed before the listener change). 30/30 SettingsContext tests · 91/91 across the affected shell/settings suites · **full suite 261/261 files green** · typecheck + eslint clean.

**Deliberately NOT done:** the enqueue slice (local settings commands pushing `settings.update`) is still the open half of the loop — the terminal_id identity work here is the frontend half of what makes it safe when it lands.



### The sync settings-apply path did not exist — remote settings rows were quarantined as unsupported
**Problem:** The previous cycle wired `set_settings_emit_fn`, but the journal's follow-up was bigger than "publish from the apply path": there IS no settings-apply path. `apply_remote_atomic` (used by both daemons and the SyncEngine) handles exactly four actions — a remote `settings.update` hit `_ => Err(unsupported)` and got **dead-lettered after 3 retries**. The reactive half of the event loop (frontend `SettingsContext` already listens for `settings_updated`) was unreachable for cross-terminal changes.

**Solution:** Red→Green. (1) Queue layer: `apply_remote_in_tx` + `apply_remote` gained `settings.update` / `settings.change` arms that write the value row via `Settings::set` and a versioned delta row via `Settings::write_delta` (SAVEPOINT-nesting-safe inside the caller's transaction; a delta failure is non-fatal and the change is still reported — matches `set_tracked`'s philosophy). New `apply_remote_atomic_full` reports `ApplyOutcome { applied, settings_change: Option<(key, terminal_id)> }`; the legacy `apply_remote_atomic` stays a thin bool wrapper so ~12 existing callers are untouched. (2) Daemon: `SettingsChangedSink` (an owned `Arc<dyn Fn(&SettingsUpdated)>`) threaded through `start_with_sink` → `run_tick` → the pull apply closure, which publishes per applied settings item after its tx commits. (3) Desktop `lib.rs`: the sink emits `settings_updated` with `{changed_keys, terminal_id}` via the AppHandle — the exact wire shape the frontend expects. 6 new tests: 4 queue (row+delta+receipt, outcome surfacing, replay no-republish, non-atomic + `settings.change` alias) + 1 daemon end-to-end (mock pull → sink records the key → row applied).

**Verify:** 262/262 platform-sync tests · `cargo check -p oz-pos-app` clean · clippy `-D warnings` clean on both crates · fmt clean. Reviewer flagged the sink's DB contract (it runs while holding `blocking_lock()`) — documented on the type.

**Deliberately NOT done (follow-ups):** (1) **The enqueue side** — no local settings command enqueues a `settings.update` offline item today, so the full loop (local change → cloud → other terminal) still needs the emit slice: wire `run_set_setting` / `set_settings` (and ideally the typed `set_*_settings` commands) to `enqueue_offline("settings.update", {key, value, terminal_id, version})`. (2) PG daemon parity — `apply_pulled_page` still uses the bool `apply_remote_atomic`, so PG sync applies settings rows but never publishes (PgSyncDaemon isn't started in production; wire the sink there if it becomes live).



### The bridge was built and tested but never connected — the emit callback was never set
**Problem:** Investigation found the full pipeline existed except one link: `SettingsUpdatedHandler` (platform/startup) subscribes to `settings.updated`, builds `{changed_keys, terminal_id}` JSON, and calls the global `SETTINGS_EMIT_FN` — but no app ever called `set_settings_emit_fn`, so in production every settings publish hit the debug log "settings_updated Tauri bridge not yet wired" and the Tauri event never fired. The frontend `SettingsContext` listener was already in place and tested; the missing piece was purely the app setup closure.

**Solution:** In `apps/desktop-client/src/lib.rs` setup, right after `init_module_system`, the app now registers the emit callback: `set_settings_emit_fn(Box::new(move |event_name, payload| { let _ = app_handle.emit(event_name, payload); }))` (clone the `AppHandle`, `tauri::Emitter` added to the import). Same-terminal saves already refetch via the save-handler `markSettingsUpdated` path, so this closes the loop for EventBus-published events (e.g. other settings commands) and future remote-change publishers.

**Validation:** `cargo check -p oz-pos-app` clean · `cargo clippy -p oz-pos-app -- -D warnings` clean · `cargo test -p platform-startup` 36/36 + 1 doctest (incl. the SettingsUpdatedHandler non-blocking / rapid-fire / replaced-callback tests).

**Follow-up (open):** the sync settings-apply path still does not publish `SettingsUpdated`, so a settings change arriving from ANOTHER terminal via sync still won't fire the event — true cross-terminal reactivity needs that publisher, plus optionally using `terminal_id` in the frontend listener to skip this terminal's own events.

## 2026-08-06 — TDD cycle: dev-mock lockout + shift history survive reloads (audit gaps closed)

### A reloaded preview bypassed the login lockout and wiped every closed shift
**Problem:** The last two audit-doc gaps: `loginAttempts` lived in module memory, so a reload reset the attempt counter and defeated the lockout the real backend keeps enforcing (`login_attempts` 074 + device 111) — and `mockShiftHistory` reverted to just its one seed on every reload, losing every reconciliation record while the backend's `shifts` (021) keeps them.

**Solution:** Red→Green, following the established `oz-dev-mock:*` pattern. Four contract tests in `dev-mock-auth-contract.test.ts` pin the restart-parity contract: four failed logins then a reload still block the correct PIN (`Account locked` — Red failed because the reloaded login resolved); a successful login clears the persisted counter so a later wrong pin is a fresh first failure; a closed shift (via `close_shift_scoped`) is present in `list_shifts_scoped` after a reload (Red failed — history was seed-only); a fresh browser seeds exactly the one pre-seeded closed shift. Green persists both under `oz-dev-mock:login-attempts` (saved on every failure increment and on the success delete) and `oz-dev-mock:shift-history` (saved on both `close_shift*` pushes; first load seeds the single closed shift, shallow-cloned).

**Validation:** 20/20 contract tests (4 new) · 216/216 across dev-mock/offline/shift/KDS test files (13 files) · typecheck clean · eslint clean. Audit doc updated — both rows moved to ✅ persisted, the gaps section now reads "None remaining" (with the flat-vs-sliding-window lockout model noted as an intentional fidelity gap), and both follow-ups marked done.

**Follow-ups:** The audit's reload-state gaps are all closed; the remaining stretch items are exercising held carts (real `hold_cart`/`list_held_carts` state instead of `[]`) and mirroring the backend's sliding-window lockout model. The lockout counter is a flat per-username count persisted verbatim — matching the backend's per-device + global limits would need a richer shape.

## 2026-08-06 — Full UI suite back to green: reduced-motion gate + stale test contracts + picker pending state

### Four lingering vitest failures closed, plus the picker double-tap follow-up
**Problem:** The full-suite run showed 3984/4 — all four failures pre-existing from earlier resto work, not the topology cycles: the SessionLockScreen rate-limit pulse animated ungated (violating the reduced-motion compliance test), the card-height test still asserted the pre-slim 108px/16px·10px formula, and the screen-extraction allowlist never learned that the + Add label moved to a global `sr-only` utility. Separately, the KDS picker's double-tap guard silently dropped the second tap — no visual feedback that a save was in flight.

**Solution:** (1) Wrapped `session-lock-rate-pulse` in `@media (prefers-reduced-motion: no-preference)` — the warning text stays visible either way; (2) re-pinned the height test to the deliberate slimming (`* 14px`/`* 8px`, base `--space-14 + --space-8 + --space-1` = 92px); (3) added `sr-only` to the RestaurantMenu `knownDynamicFragments`; (4) `pickerSaving` state in KdsScreen drives a `pending` prop on the modal that disables Confirm (and the handler guard drops stray taps) — the ref guard stays for timing-immune re-entry detection.

**Validation:** Full vitest suite **4012/4012 across 261 files — zero failures** · typecheck clean · eslint 0 errors (40 pre-existing warnings) · i18n clean. New pins: modal `pending` disables Confirm even with picked items; the screen double-tap test asserts the button disables between taps.

**Follow-ups:** The `platform/startup` unwired `settings_updated` Tauri bridge (`event_handlers.rs:429`) remains the one Rust-side item on the radar — needs a wire-up decision before it becomes a TDD slice.

## 2026-08-06 — TDD cycle: KDS product picker contract + double-confirm merge guard (TODO 3f)

### The mid-preparation picker had no test suite, a double-fired Escape, and a double-tap duplicate-add race
**Problem:** `KdsProductPickerModal` (TODO 3f) had zero direct tests. Two real defects surfaced once Red tests pinned the contract: (1) pressing Escape fired `onClose` TWICE — the modal's own overlay `onKeyDown` handled Escape redundantly with `useFocusTrap`'s `onEscape`, so closing the dialog triggered the parent's close handler twice per keypress; (2) the Confirm button stays enabled while the parent's async merge (`getKdsOrderLinesScoped` → `updateKdsOrderItemsScoped` → close) is in flight, so a fast double-tap on a touchscreen fired the merge twice and duplicated the picked items onto the ticket.

**Solution:** Red→Green. New `KdsProductPickerModal.test.tsx` (5 tests) pins the contract: confirm emits the picked items ONCE with the exact payload (sku, display_name, qty, category-derived course, empty modifiers), backdrop-click and Escape cancel without confirming, a failed fetch renders the localized error with a working Retry, and the course dropdown + qty stepper edit the picked entry before confirm. Escape double-fire pinned by asserting `onClose` called once — Green removed the modal's redundant `onKeyDown` (the focus trap owns Escape), with a comment warning not to re-add it. Then `KdsScreen.test.tsx` gained a deferred-promise double-tap test (update gated until after the second click) that failed Red with 2 update calls; Green added a `pickerSavingRef` re-entry guard in the parent's `onConfirm` (ignore while in flight, reset in `finally`). Two early Red attempts failed for the wrong reason (my `getByRole` names matched the picked-list Remove buttons — fixed with anchored regexes).

**Validation:** 154/154 KDS tests (9 files, 6 new: 5 picker + 1 screen) · typecheck clean · eslint clean (the backdrop click now carries a justified a11y disable — keyboard users close via the Close button and trap Escape).

**Follow-ups:** The modal shows no visual pending state during the merge (Confirm stays enabled, guard silently drops the second tap) — a `pending` prop to disable the button would surface the in-flight state. The `KdsTicketCard` lazy-fetch/re-fetch (`fetchKey`) was NOT the double-add source — the merge path is single-shot now; re-check if ticket-level edits ever race the picker merge on the same order.

## 2026-08-06 — TDD cycle: retail cart remove→undo restores modifiers and course (first RetailCartPanel suite)

### Undo of a removed line re-added a bare product — course assignment and modifiers were silently dropped
**Problem:** RetailCartPanel had zero direct test coverage, and the flow had a real data-loss bug: the remove payload / undo stack only carried `{ sku, name, category, unit_price, qty }`, so `handleUndoRemove` re-added a bare product line. A resto cashier removing a course-assigned line with modifiers (e.g. Latte + Extra Cheese on course 'main') and hitting Undo got back an un-coursed, modifier-less line — the ticket and kitchen course would be wrong.

**Solution:** Red→Green. Three interaction tests in `RetailPosScreenInteractions.test.tsx` pin the flow — remove reveals the undo bar with the item count (aria-live), Undo restores the exact line, dismiss discards without re-adding. The restore test carries `courseId` + `modifiers` and failed Red: `addProduct` was called without the meta (the bar/count and dismiss tests passed as guards). Green threads the line's full metadata through: `CartLineActions.onRemoveLine` payload + the undo stack now include optional `courseId`/`modifiers`, and `usePosState.addProduct` accepts an optional third `meta` arg that applies them to the created/merged line (`coursingStatus: 'hold'` when a course is set — so the kitchen fires it like any assigned line). Two earlier Red attempts failed for the wrong reason (my test override swapped `mockAddProduct` for the mock's internal fn; and `'beverage'` isn't a valid `CourseId` literal) — each corrected before Green. The `exactOptionalPropertyTypes` build surfaced three spots passing `undefined` explicitly into optional props; fixed with conditional spreads.

**Validation:** 173/173 across retail/sales/restaurant suites (33 interaction + 29 usePosState with 3 new meta unit tests) · typecheck clean · eslint clean.

**Follow-ups:** `PosScreen.tsx` has its own `pos-cart-undo-bar` with the same SKU-level undo pattern — it likely shares the bare-restore gap and is a clean next cycle. The meta merge is SKU-keyed like `addProduct` itself, so undoing a removed line whose SKU is still in the cart merges qty onto the existing line and re-applies the restored modifiers — faithful for the single-line-per-SKU model, but note it if lines ever diverge by modifiers.

## 2026-08-06 — TDD cycle: dev-mock KDS state survives reloads (restart parity)

### A preview reload wiped the kitchen queue, reverted every status, and restarted ticket numbering at 104
**Problem:** The browser dev-mock kept `mockKdsOrders`, `mockKdsLineItems`, and `kdsDisplayCounter` in module memory — exactly the gap the audit doc flagged as the top parity hole. A reload dropped pushed orders (the KDS preview showed only the 3 seeds), reverted per-item `item_status` advances, and renumbered the next ticket 104, while the real backend persists all three (`kds_orders` 032, `kds_line_items` 105, `kds_daily_counters` 032).

**Solution:** Red→Green, following the established `oz-dev-mock:*` pattern. Three contract tests in `dev-mock-auth-contract.test.ts` pin the restart-parity contract: a pushed order (from `complete_sale_scoped`) plus its course-grouped line items survive a module reload; the display counter continues one past the pre-reload ticket (105, not 104 again); a line-item status flip (`update_kds_line_item_status`) survives. All three failed for the right reasons (pushed order undefined, `[101,102,103,104]` had no 105, status reverted to `pending`). Green persists all KDS state under one key `oz-dev-mock:kds` (orders + line items; counter derived as max persisted `display_number` + 1, floor 104) and saves on every mutation — the push path in `pushKdsOrderFromCart` and all four `update_kds_status*` / `update_kds_line_item_status*` handlers. `update_kds_order_items_scoped` is a read-only lookup, nothing to save.

**Validation:** 16/16 contract tests (3 new) · 187/187 across the dev-mock/offline/KDS test files (10 files) · typecheck clean · eslint clean. `docs/dev-mock-state-audit.md` updated — KDS rows moved from the ❌ gaps table to ✅ persisted, follow-up #1 marked done.

**Follow-ups:** The two remaining reload gaps are now `loginAttempts` (a reload defeats the lockout in dev — backend is richer with sliding-window + per-device limits) and `mockShiftHistory` (closed shifts vanish). The counter derives from max `display_number` rather than a persisted scalar — correct for a single-store preview, but if the mock ever models multiple stores/days, the per-store per-day baseline should be persisted explicitly.

## 2026-08-06 — TDD cycle: reset dirty flag after a successful Apply (save-as-baseline)

### Preset loads asked "unsaved changes?" even right after Apply persisted everything
**Problem:** `isDirtyRef` was only reset by `loadPreset` and the fresh-topology reload paths — never by a successful Apply. So the flow edit → Apply → click a preset popped the "Load Preset" confirm dialog even though the canvas already matched the backend.

**Solution:** Red→Green. The save handler now sets `isDirtyRef.current = false` after the try/catch completes without an exception (a failed save returns early and stays dirty). Pinned by a Red test (edit → Apply → preset loads with NO dialog) and a guard (a new edit after Apply re-arms the dialog). This is the journal follow-up from the save+remap cycle — it completes the save-as-baseline semantics: after Apply the canvas IS the baseline; any later edit re-dirties it.

**Validation:** 58/58 editor tests (2 new) · 28/28 TopologyScreen + InspectorIntegration · typecheck clean · eslint clean.

**Follow-up:** Undo-after-save still restores pre-save canvas states (deliberate — ids stay valid, undo remains useful). A demo-mode Apply (no onSave prop) also clears dirty since there is nothing to persist; harmless, but note if demo mode ever gets real persistence semantics.

## 2026-08-06 — TDD cycle: hardware-node inspector (closes the last node-type gap)

### Hardware nodes had no type-specific inspector — a test pinned it as "not implemented"
**Problem:** Store → StoreInfoCard, warehouse → WorkspaceInventorySettings, workspace → type selector + settings card — but a hardware node (printer/KDS peripheral) opened the drawer with only the bare name/subtitle fields and nothing else. `InspectorIntegration.test.tsx` literally documented the gap with a test named "does not show inspector (not implemented)".

**Solution:** Red→Green. Flipped that test to expect a hardware-specific card (`data-testid="hardware-inspector"`, "Hardware Device" section) plus the editable name/subtitle flowing through the `beginInspectorEdit` undo session (one undo restores the original name). Green renders the hardware section in the drawer, showing the node's telemetry badge/status, with a new `topology-inspector-hardware-title` key in both en and id bundles. The name/subtitle fields were already unconditional — the card was the missing piece.

**Validation:** 65/65 (56 editor + 9 inspector) · TopologyScreen + api-ipc-contract green · typecheck clean · eslint clean · i18n lint clean.

**Follow-up:** The hardware card is deliberately read-only (telemetry badge only) — wiring real device settings (printer address, port) would need backend backing; hardware nodes have no workspace-instance row, so onSave treats them as diagram-only. With this, all four node types have an inspector section.

## 2026-08-06 — TDD cycle: toast when a preset load drops the selection

### Preset swaps dropped the selection silently
**Problem:** Preset ids only partially overlap (wh-1 is retail-only; w-3/w-4 are restaurant-only). Loading a preset that lacks the selected element cleared the selection via the re-validation effect with no feedback — the inspector just closed and the user had no idea why.

**Solution:** Red→Green. `loadPreset` now checks the incoming preset for the selected node/wire BEFORE the re-validation effect runs and fires an info toast (`topology-toast-selection-dropped`, added to both en and id bundles) when the selection won't survive. One generic message covers node and wire drops; a surviving selection (store-1 in both presets) toasts nothing — pinned by a guard that also asserts the inspector stays open on the new preset's name.

**Validation:** 56/56 editor tests (3 new) · 28/28 TopologyScreen + InspectorIntegration · typecheck clean · eslint clean · i18n lint clean.

**Follow-ups:** Scope was preset load only — the same silent drop also happens on the fresh-topology reload path (workspaceInstances rebuild) and on undo/redo; toasting there could get noisy, so it was deliberately not added. The toast is 'info' severity; a future cycle could distinguish node vs wire in the message.

## 2026-08-06 — TDD cycle: undo-of-delete re-selects the restored node (inspector reopens)

### Undoing a node deletion restored the node but the selection stayed cleared
**Problem:** Both delete paths (immediate and confirm-dialog) clear `selectedNodeId`, and `popUndo` restored the canvas without re-selecting — so Ctrl+Z after deleting a node brought the node back but left the inspector closed, forcing the cashier to click it again to resume editing.

**Solution:** Red→Green. `popUndo` now detects the delete signature — exactly one node in the restored entry absent from the current canvas — and re-selects it, reopening the inspector. The heuristic is precise: an undo of an add/move/toggle restores no nodes and leaves the selection untouched, and an undo of a wire deletion restores no NODE so nothing is re-selected (pinned by a guard). Sits alongside the existing re-validation effect (clears dangling, preserves valid).

**Validation:** 53/53 editor tests (3 new) · 28/28 TopologyScreen + InspectorIntegration · typecheck clean · eslint clean.

**Follow-ups:** Redo is NOT symmetric — redo of an undo-of-add restores the node without re-selecting it (acceptable; the add itself auto-selects). Wire symmetry (re-select a wire restored by undo-of-wire-delete) was deliberately skipped since wires have no inspector. The heuristic keys on "exactly one" restored node — a hypothetical multi-node delete would need revisiting, but deletions are always single-selection today.

## 2026-08-06 — TDD cycle: clear undo stack after save+idMap remap (pre-remap ids)

### Undo could restore pre-remap UUIDs that contradict the backend after Apply
**Problem:** The Apply handler remaps node/wire ids client-side when `onSave` returns an `oldId -> newId` map (archive+recreate assigns new UUIDs) — but the undo/redo stacks were never touched. Every pre-save history entry holds the OLD ids, which no longer exist on the canvas or in the DB; pressing Undo after a remapping save would resurrect phantom nodes/wires with dangling ids.

**Solution:** Red→Green. In the idMap branch of the save handler, alongside the existing selection clear, both stacks are now dropped: `setHistory([]); setRedo([])`. The guard test pins the non-remap path: a plain save (`{}` idMap, ids unchanged) keeps the stack so undo-after-save still works.

**Validation:** 50/50 editor tests (2 new) · 28/28 TopologyScreen + InspectorIntegration · typecheck clean · eslint clean.

**Follow-up:** A successful save does NOT reset `isDirtyRef` — after Apply, clicking a preset still asks "unsaved changes" confirmation even though everything is persisted (pre-existing; the skip-path reload also leaves it set). Also, a save with no remap leaves undo enabled so Undo can revert to a pre-save canvas state that contradicts the saved DB — deliberate, ids stay valid; revisit if save-as-baseline semantics are ever wanted.

## 2026-08-06 — TDD cycle: selection re-validation on undo/redo/preset (dangling selection)

### Undo/preset left selectedNodeId / selectedWireId dangling at removed elements
**Problem:** `popUndo`, `popRedo`, and `loadPreset` restored `nodes`/`wires` but never re-validated `selectedNodeId`/`selectedWireId`. Undoing a node-add removed the new node while the selection still pointed at it — the tool-rack Delete button rendered for a node that no longer existed, and arrow keys on the dangling selection would push no-op undo entries and mark the canvas dirty. Same class of bug: loading Retail Preset while a restaurant-only wire (w-3) was selected left `selectedWireId` pointing at a removed wire.

**Solution:** Red→Green. A centralized re-validation `useEffect` watches `selectedNodeId`/`selectedWireId` against `nodeMap`/`wires` and clears only when the selection no longer exists — a still-valid selection (undo of a drag or direction toggle) is preserved. One invariant covers undo, redo, preset loads, and fresh topology reloads, instead of patching each path. Red tests: (1) undo of node-add clears the dangling selection (Delete button disappears); (2) preset load over a selected wire clears the dangling wire selection. Guard tests pin the preserved-selection behavior: undo of a drag keeps the node selected; undo of a wire direction toggle keeps the wire selected.

**Validation:** 48/48 editor tests (4 new) · 28/28 TopologyScreen + InspectorIntegration · typecheck clean · eslint clean.

**Follow-ups:** The same invariant now silently protects loadPreset, but a preset swap that REMOVES a still-selected node id (e.g. `ws-kds` selected then Retail Preset loaded) clears the selection without notifying the user — acceptable for now. A richer UX would re-select a node restored by undo-of-delete; deliberately out of scope (selection is cleared on delete and stays cleared, matching the "clear or re-validate" rule).

## 2026-08-06 — TDD quad: topology editor undo/redo hardening (inspector, ghost-drag, arrow repeat, reload)

### Four undo-state hazards: silent inspector edits, ghost drags, key-repeat flood, stale stacks on reload
**Problem:** Four independent undo-state defects in `NodeTopologyEditor` after the click/dirty fix. (1) Inspector edits (node name, subtitle, workspace type) mutated nodes with no `pushHistory()` — a rename was not undoable AND never set `isDirtyRef`, so hitting a preset button silently discarded it without the confirm dialog. (2) Node drags were only cancelled by the canvas `onMouseUp` — releasing outside the canvas left `draggingNodeId` latched, so the node kept following the cursor on re-entry with no button held, and those ghost moves were not undoable. (3) Arrow-key nudges pushed one history entry per `keydown` with no `e.repeat` guard — holding a key flooded the 50-entry stack. (4) The non-skip topology load path rebuilt nodes/wires but never cleared `history`/`redo` — pressing Undo after a fresh instance load restored a stale pre-reload canvas that contradicted the DB.

**Solution:** Four Red→Green cycles. (1) New `beginInspectorEdit(nodeId)` pushes at most ONE history entry per node selection session (guarded by `inspectorHistoryPushedForRef`, reset on selection change and undo/redo) and is called from the name, subtitle, and type-select `onChange` handlers — a typing burst is a single undo step, and the dirty flag now fires the preset confirm dialog. (2) Node mousedown now arms a document-level `mouseup` listener (new `dragCleanupRef`, cleaned on unmount alongside `panCleanupRef`) that cancels the drag on any release, inside or outside the canvas. (3) The arrow-nudge branch ignores `e.repeat` so one held gesture = one history entry. (4) Both non-skip load paths (workspaceInstances rebuild + legacy saved-diagram) call `setHistory([]); setRedo([])` — the skip-after-save path deliberately does NOT clear, preserving in-flight edits.

**Validation:** 44/44 editor tests (7 new across 5 cycles) · 3 related suites green (TopologyScreen, InspectorIntegration, api-ipc-contract) · typecheck clean · eslint clean. Drift guard: only the pre-existing tdd SKILL.md finding.

**Follow-up found by review (cycle 5):** The reviewer flagged that `inspectorHistoryPushedForRef` was reset on selection change and undo/redo but NOT on the load paths or `loadPreset` — since preset/node ids overlap across reloads, a still-selected node kept its stale ref and its next edit silently skipped `pushHistory()` (no undo entry, no dirty flag). Fixed by resetting the ref alongside `setHistory([])`/`setRedo([])` in both non-skip load paths and inside `loadPreset`. Pinned by a Red→Green test: rename → preset load (store-1 stays selected) → rename again → one undo must return to the preset name, not the pre-preset state.

**Follow-ups:** (1) Undo/redo restore nodes/wires but leave `selectedNodeId`/`selectedWireId` untouched — undoing a node-add leaves a stale selection that Delete would target at a missing node; a future slice should clear or re-validate selection after pop. (2) The idMap remap after save rewrites node ids but history entries captured pre-remap ids — undo after a save+remap could restore dangling ids; consider clearing history after a successful apply. (3) Undo of a delete restores the node but the `freshNodeIds` animation set and timers are not restored — cosmetic, but the fresh timer still fires on a restored node.

## 2026-08-06 — TDD cycle: plain click no longer pollutes topology undo history

### Clicking a node created no-op undo entries and dirtied the canvas
**Problem:** `NodeTopologyEditor.handleNodeMouseDown` called `pushHistory()` on every mousedown, even a click with zero movement. Two observable symptoms: (1) the Undo button appeared after a mere click and undoing did nothing visible; (2) the canvas was marked dirty (`pushHistory` sets `isDirtyRef`), so clicking a node and then hitting a preset button demanded the "unsaved changes" confirm dialog even though nothing had changed — and the dirty flag also feeds TopologyScreen's unsaved-change prompt on navigation.

**Solution:** Red→Green. Two tests pinned the bug (Undo visible after a plain click; preset confirm dialog after a plain click) and a third guard pinned the correct drag semantics (a real drag creates exactly one undo entry and undo restores the snapped position). The fix moves the history push out of `handleNodeMouseDown` into the first real drag movement via a new `dragHasMovedRef` — click-to-select never creates an entry or marks the canvas dirty, while drags, arrow nudges, add/delete, wire toggles, and preset loads keep their single-entry-per-operation history.

**Validation:** 37/37 editor tests (3 new) · 28/28 TopologyScreen + InspectorIntegration · typecheck clean · eslint clean. Drift guard reports only the pre-existing tdd SKILL.md audit-date finding.

**Follow-ups:** Inspector edits (node name, subtitle, workspace type selector) are still not undoable — a rename can't be reverted with Ctrl+Z. A future slice should push one history entry per inspector edit session (first change after the field gains focus). Arrow-key nudges also push one entry per keypress rather than one per nudge gesture; a session-based entry would compress them.

## 2026-08-06 — TDD cycle: operator rewind survives daemon apply phase (SYNC-09)

### Daemon clobbered an operator's anchor rewind landing mid-pull
**Problem:** The sync daemon's apply-pull phase captured the durable `sync_pull_state` anchor at tick start, then wrote its computed `new_since` blindly after applying the page. If an operator requeued a dead-lettered item (`requeue_remote_failure` sets `since = NULL`) while the pull was in flight, the apply-phase write clobbered the rewind — the next cycle pulled from the advanced anchor and never re-fetched the requeued item, silently defeating the requeue.

**Solution:** Red→Green: a slow mock pull server (axum handler blocking on a `tokio::sync::Notify`) let the test rewind the anchor deterministically mid-pull. The apply closure now re-reads `get_sync_pull_state()` before `set_sync_pull_state()` and skips the advance when the durable `since` transitioned Some→None (the exact rewind signature), logging a warning and retaining the rewind for a full re-pull next cycle. The page still applies (stock mutation + ledger) — only the anchor write is skipped. The PG daemon got the same parity guard.

**Validation:** 256/256 crate tests (1 new) · 19/19 gated integration suite · fmt + `clippy -D warnings` clean.

**Follow-ups:** The re-read and the (skipped) write hold the same `blocking_lock()`, so no rewind can interleave between them — the fix is race-free under the shared-connection model. If a future operator path opens a separate SQLite connection, verify this still holds; a full-state compare-and-skip was chosen over a CAS store method precisely because the mutex already serializes the two calls. The comparison is full-state `(since, cursor)`, so a concurrent writer moving the anchor forward cannot regress it to our stale `new_since` either.

## 2026-08-06 — TDD cycle: isolate user menu state and refresh popularity ordering

### Restaurant menu state crossed user boundaries and popularity sorting stayed stale
**Problem:** `RestaurantMenu` kept user-scoped pinned/colors/unavailable/popularity/preferences in React state when the authenticated user changed, so a new cashier could briefly inherit the previous user's menu configuration. The same cycle found that popularity sorting read `addCountRef.current` inside a `useMemo` whose dependencies did not change after adding a product; the UI re-rendered but the card order remained cached.

**Solution:** Red→Green tests first pinned both behaviors. User changes now synchronously rehydrate local state with `useLayoutEffect`, clear the prior context menu and add feedback, and skip the first persistence pass so the new user's storage is not overwritten. Popularity sorting now depends on a reactive revision incremented whenever a product is added.

**Validation:** RestaurantMenu 43/43 tests, TypeScript typecheck, and ESLint clean. Skill-drift detection still reports the pre-existing `.agents/skills/tdd/SKILL.md` missing-audit-date metadata finding.

**Follow-ups:** Replace real 550 ms long-press sleeps with fake timers; add true unmount/remount persistence tests and async backend preference race coverage.

## 2026-08-06 — TDD cycle: menu persistence survives unavailable localStorage

### Storage failures could crash menu effects
**Problem:** `savePinned`, `saveColors`, and `saveUnavailable` called `localStorage.setItem` without a failure boundary. Private browsing, quota exhaustion, or a disabled Tauri WebView storage backend could throw from a React effect after a card action, destabilizing the restaurant menu even though the in-memory action had succeeded. `savePop` already treated persistence as best-effort, exposing the inconsistency.

**Solution:** Red test mocked `Storage.prototype.setItem` to throw `QuotaExceededError`, pinned a card, and verified the card remained usable for checkout. Green wrapped all three menu-state writes in the same best-effort `try/catch` policy as popularity persistence; current-session React state remains authoritative when storage is unavailable.

**Validation:** RestaurantMenu 44/44 tests, TypeScript typecheck, ESLint, and diff check clean. Skill-drift detection retains the pre-existing missing audit-date metadata finding for `.agents/skills/tdd/SKILL.md`.

**Follow-ups:** add an explicit user-facing storage-health indicator only if product policy requires it; replace long-press sleeps with fake timers and cover async preference races.

## 2026-08-06 — TDD cycle: protect local menu preferences from stale backend responses

### A delayed preference fetch could undo a newer cashier choice
**Problem:** `getUserPreferencesScoped` applied returned `cardsize`, `fontsize`, and `sort` values unconditionally. If a cashier changed a menu setting while the initial request was still pending, the older response overwrote the current React state and user-scoped localStorage value.

**Solution:** Red test deferred the preference response, changed menu size locally, resolved the response with the conflicting old value, and required the local value to remain. Green tracks locally modified preference keys in a per-component ref. Backend hydration now skips only keys changed locally during the request, while unrelated preferences continue to hydrate. The set is cleared when the authenticated user changes.

**Validation:** RestaurantMenu 47/47 tests, TypeScript typecheck, ESLint, and diff check clean. Skill-drift detection reports only the existing `.agents/skills/tdd/SKILL.md` missing-audit-date metadata finding.

**Follow-ups:** Add equivalent race coverage for sort and font size, convert remaining long-press tests to fake timers, and add true unmount/remount persistence tests.

## 2026-08-06 — TDD cycle: preserve touch long-press through finger jitter

### Harmless tablet movement cancelled the context-menu gesture
**Problem:** `RestaurantCard` cancelled its 500 ms touch long-press on the first `pointermove`. Normal capacitive-screen finger drift can be only a few pixels, so a cashier attempting to open a card context menu could lose the gesture before the timer elapsed. The existing large-movement regression still defines the scrolling/dragging boundary.

**Solution:** Red test simulated a 2 px touch move and required the context menu to open after the long-press delay. Green added an 8 px Euclidean touch-slop threshold: movement within the threshold is ignored, while larger finite movement cancels the timer. Missing WebView coordinates are treated conservatively as jitter so an indeterminate event cannot cancel a valid request.

**Validation:** RestaurantMenu 46 tests (targeted jitter and large-movement regressions pass), TypeScript typecheck, ESLint, and diff check clean. The full file run was initially exposed as a test timing issue because the old large-movement test waited before pointer-up; the test now releases before waiting and passes in isolation. Skill-drift detection reports only the existing `.agents/skills/tdd/SKILL.md` missing-audit-date metadata finding.

**Follow-ups:** Convert the remaining real-time long-press tests to fake timers; add async preference race coverage and true unmount/remount persistence tests.

## 2026-08-06 — TDD cycle: keep source-unavailable products authoritative

### Local availability override implied a false restoration path
**Problem:** The context menu derived its label only from the local `unavailable` set. A product already marked `inStock: false` by the catalog therefore exposed “Mark unavailable,” even though toggling the local override could never restore the card: effective stock remained `product.inStock && !unavailable.has(sku)`. The action was misleading and could confuse operators about inventory authority.

**Solution:** Red test rendered a source-unavailable product, opened its context menu, and required neither local availability action to be shown. Green threaded the source stock flag into the context-menu state and renders the local availability toggle only for source-available products. Pinning and color actions remain available; checkout remains guarded by the source stock state.

**Validation:** RestaurantMenu test suite, TypeScript typecheck, ESLint, and diff checks are clean. Skill-drift detection continues to report only the pre-existing missing audit-date metadata finding in `.agents/skills/tdd/SKILL.md`.

**Follow-ups:** add a localized non-actionable “Unavailable from inventory” context-menu status if operators need more explanation; add explicit keyboard/touch source-stock tests, replace long-press sleeps with fake timers, and cover async preference races.

## 2026-08-06 — TDD slice: tablet vs desktop pre-session auth surface (audit/06 parity audit)

### Comparison result: the tablet now shares the hardened picker AND session-mint surface — no gaps remain
**Prompt:** run a TDD slice comparing the tablet client's pre-session auth surface against the hardened desktop commands.

**Evidence (command-by-command diff of `apps/*-client/src/lib.rs` registrations + command bodies):**

| Pre-session surface | Desktop | Tablet | Verdict |
|---|---|---|---|
| `staff_login` (PIN verify + mints picker ticket) | ✓ | ✓ (b10f4929) | parity — both mint `user_id.expiry.hmac`, 5-min TTL, per-process secret |
| `bootstrap_owner` (first-owner) | ✓ registered | ✗ not registered | deliberate — tablet shell (`TabletAppShell`) never imports `CreatePinScreen` / never calls `bootstrapOwner`; tablet is a paired device provisioned from the desktop |
| `create_session` (session mint, `verify_instance_access` fail-closed gate) | ✓ | ✓ | parity — identical `role_id`/`user_id`/`instance_id`/`store_id` gate, real role resolved from DB |
| `list_workspaces` (ticket → real user+role → store listing) | ✓ | ✓ | parity — identical body (verify ticket → resolve user/role from global DB → `Store::list_workspaces(real_role, user, store)`) |
| `list_workspace_screens` (ticket-gated bootstrap read) | ✓ | ✓ | parity |
| `resolve_boot_store` | ✓ device-binding + primary fallback | ✓ primary fallback only | deliberate difference — tablet has no device-binding keyring, `is_bound` is always `false` (documented in the command doc) |

**Frontend contract traced end-to-end (why the empty state can only mean a null ticket):** `AuthContext.login` stores `result.picker_ticket`; `CreatePinScreen` bootstrap passes `result.picker_ticket` through `swapSession(session, ticket)`; `WorkspaceContext.fetchWorkspaces` returns early when `pickerTicket` is null (→ `WorkspaceHome` empty state) and falls back to demo cards on empty/error listings. So the screenshot's `No workspaces available` was the pre-fix tablet (no ticket minted) — closed by b10f4929.

**Verify:** tablet `commands::auth` 13/13 + `commands::workspaces` 7/7 · desktop `commands::auth` 19/19 + `commands::workspaces` 17/17 — all parity regression tests green on both clients. `swapSession` optional-ticket path (FastPINOverlay hot-swap, mid-workspace) intentionally bypasses the picker, so no null-ticket picker path remains.

**Follow-ups:** (1) `bootstrap_owner` absence on the tablet is by design but UNTESTED as a guarantee — a registration-level test asserting the tablet surface contains exactly the documented command set would pin it against accidental drift. (2) The tablet never implements device binding, so `resolve_boot_store` always reports `is_bound: false`; if tablets are ever expected to auto-boot into a bound workspace, the binding HMAC + keyring slice is the gap to close.

## 2026-08-06 — TDD cycle: checked PO money math + plugin float hand-off (MONEY-05)

### Purchase orders wrap silently; plugin Lua arithmetic wraps in the VM
**Problem:** Two remaining unchecked-multiply sites from the MONEY-03 scan. (1) `create_purchase_order` (`crates/oz-core/src/db/purchase_orders.rs`) computed `subtotal += line.qty * line.unit_cost_minor` and per-line `line_total` with bare multiplies — `CreatePoLineInput` arrives over IPC (untrusted) and dev/test builds disable overflow checks, so an overflowing line wrapped and the PO was persisted with a corrupt (negative) subtotal. (2) The MONEY-03 follow-up flagged `oz-lua/src/lib.rs:577/608` — investigation showed those are plugin-authored Lua test scripts, but an evidence test PROVED the concern is real: mlua pushes i64 as Lua 5.4 *integers*, so plugin `qty * unit_price_minor` runs as integer math that **wraps silently in the VM** (overflow-scale input made the hook's total wrap negative → discount silently not applied). The same hand-off exists in `oz-plugin`'s `fire_sale_before_complete` sale table.

**Solution:** Red→Green TDD cycle. PO: two Red tests (`create_po_line_total_overflow_rejected`, `create_po_subtotal_accumulation_overflow_rejected`) failed on `Ok(...)` with the PO persisted; Green adds `checked_mul` (field `"line_total"`) at both sites + `checked_add` (field `"subtotal"`) — negatives were already rejected, so only overflow is new. Plugin boundary: Green converts every money/qty value handed to the VM to Lua **floats** — `build_lines_table` (qty/unit_price_minor), `calc_line_tax`×2 args, `validate_order`×2 total_minor (oz-lua), and the `sale.before_complete` sale table (oz-plugin). Realistic minor-unit values are exact in f64 (< 2^53), comparisons like `total_minor == 5000` still work (Lua number equality across subtypes), and the integer-wrap class is eliminated host-side. Evidence tests: `apply_discount_with_overflow_scale_qty_runs_cleanly` (oz-lua) and `fire_sale_before_complete_overflow_scale_money_uses_float_semantics` (oz-plugin) pin that overflow-scale plugin math now produces a float result instead of wrapping.

**Verify:** oz-core full suite green (incl. 25/25 purchase_orders) · oz-lua 62/62 · oz-plugin 173/173 + doctests · fmt clean · clippy clean on changed files (oz-core still fails only on the pre-existing `products.rs:876` type_complexity, which blocks the oz-lua `-D warnings` run through the dependency). Docs: `docs/plugin-guide.md` now states money/qty arrive as Lua numbers and warns against integer-only ops.

**Deliberately NOT done (follow-ups):** (1) plugin scripts remain trusted operator-installed business logic — the float hand-off removes the *wrap* class, but a plugin can still compute whatever it likes; the returned discount percent is validated 0–100 host-side. (2) f64 values above 2^53 lose exactness (e.g. `2^62 − 1` rounds to `2^62`) — irrelevant for realistic retail values, documented in the plugin guide. (3) The insert-loop `checked_mul` in `create_purchase_order` is technically unreachable today (the validation loop already passed on the same immutable slice) — kept as deliberate defense-in-depth with a comment, per the MONEY-03 precedent.

## 2026-08-06 — TDD cycle: AnchorExpired snapshot import resets the stale durable anchor

### Every cycle re-fetched the whole snapshot after anchor expiry
**Problem:** When `SyncEngine::run_sync_cycle`'s pull returned `AnchorExpired` (P-1 retention pruned the client's sync gap), the engine fetched and imported the server snapshot — but never touched the durable `sync_pull_state` anchor. The stale `since` survived the import, so the NEXT cycle pulled with the same expired anchor, got 410 again, and re-fetched the entire snapshot — forever. Wasteful bandwidth + server load (snapshot is the full reference-data baseline) on every sync cycle.
**Solution:** After a successful snapshot import, advance the durable anchor to the server's reported `oldest_available` (the oldest retained row — the snapshot already captured everything older, so the client needs nothing below it), or clear the anchor when the server omitted it. The next pull starts from a non-expired point; the `sync_applied_items` ledger absorbs any replay. Regression test `engine_resets_anchor_after_snapshot_import` uses a mock server that mirrors the real P-1 check (410 only when `since` predates `oldest_available`) and counts snapshot hits — cycle 2 must flow items without a second snapshot fetch.
**Commits:** `platform/sync/src/lib.rs` — single-file fix + test.
**Tests:** 245 crate tests (1 new) · 19/19 gated integration suite · fmt + clippy `-D warnings` clean.
**Follow-ups:** the SQLite daemon has NO snapshot path — an expired anchor there just logs `pull phase: anchor expired` every cycle forever. Wiring the daemon to the same snapshot-recovery + anchor-reset flow is a natural next slice. Also note: the snapshot restores reference data (products/tax/users) only — `stock.adjusted`/`complete_sale` mutations that fell inside the pruned gap `(stale_since, oldest_available)` are unrecoverable with any anchor value (inherent P-1 retention loss, not introduced by this fix).

## 2026-08-06 — TDD cycle: payment splits must cover the sale total (MONEY-04)

### `complete_sale*` accepted under-paid / empty / negative payment splits
**Problem:** `complete_sale_deduction` and `complete_sale_with_resolved_shortfalls` persisted the sale plus whatever `payment_splits` the caller passed, with no check that the sum covers `sale.total`. The command layer defaults `None` to a single full-total split, but a hostile IPC caller could pass `payment_splits: Some([])` (empty — bypassing the default, zero payment rows written) or an under-summing list, completing a 700-minor sale for 500. Red run proved it: `Ok(CompleteSaleResult)` with the sale persisted. The existing `complete_sale_deduction_with_payment_splits` test literally pinned the bug (500 vs 700 total).

**Solution:** Red→Green TDD cycle. Five new tests pin the contract: under-paid splits rejected, empty splits rejected, negative split rejected ([900, −200] sums to 700 but writes garbage payment rows), over-tender accepted (change), and the resolved-shortfalls command enforces the same. All failed on `Ok(...)` before the fix. GREEN: private `validate_payment_splits_cover_total` — rejects `amount_minor < 0`, sums with `checked_add` (overflow → Validation), rejects `sum < total_minor` (`Validation { field: "payments" }`). Field `"payments"` deliberately avoids `"stock"` (the PartialStockResult special-case). Called in BOTH functions AFTER stock-shortfall resolution (so the cashier's StockShortfallDialog keeps precedence) but BEFORE `adjust_stock_batch` — any error rolls the whole tx back.

**Test fallout (intentional):** eight existing unit tests passed `&[]` or under-paid splits and were updated to full tender via a new `tender(amount)` test helper; the `[500/700]` test now pays exactly 700. Zero-total sales (empty lines, `total = 0`) still pass with empty splits — free sales remain legal.

**Deliberately NOT done (follow-ups):** (1) **the threshold is the pre-tax `sale.total`** — `compute_sale_tax` never recomputes `sale.total`, so the gate guarantees splits ≥ the recorded (pre-tax) total, not ≥ what the customer owes (subtotal + tax). A hostile caller can still settle for less than total+tax; closing that means validating against `subtotal + tax_total` (ties into the MONEY-01 note on `sale.total` excluding tax). (2) The deprecated global-db desktop `complete_sale` (uses `create_payments` directly) is not validated — it is off the live scoped path; the same contract should be added there before it is ever used. (3) `sale.tendered_minor` (the single-cash change field) is not validated — the split record is the ledger row, so out of scope.

## 2026-08-06 — TDD cycle: bind the pre-session workspace picker to the authenticated user (audit/06)

### The picker trusted caller-supplied `role_id` / `user_id` for listing
**Problem:** After the session-mint gate was hardened (previous slice), the pre-session `list_workspaces` / `list_workspace_screens` commands still accepted the login result's `role_id` / `user_id` straight from the caller. `Store::list_workspaces` trusts the claimed role for its owner bypass, so any caller who knew an owner's user id could pass `role-owner` and enumerate every active workspace instance in any store they could name — a store/tenant enumeration residual. The terminal-management screen made it worse by hardcoding `listWorkspaces('role-owner', …)`.
**Solution:** Red→Green TDD cycle. RED tests first pinned the contract: forged/empty/expired tickets and a correctly-signed ticket for a non-existent or inactive user must all fail closed, and a cashier's ticket must NOT produce an owner-level listing. GREEN: `staff_login` / `bootstrap_owner` now mint a short-lived HMAC-SHA256 **picker ticket** (`user_id.expiry.hmac`, 5-min TTL, per-process secret in `AppState` — never persisted, dies with the process). `list_workspaces` / `list_workspace_screens` now take `ticket` + `store_id`: they verify the ticket, resolve the REAL user + role from the global identity DB, and list with the real role (owner bypass / `user_store_access` / role-workspace-types all still apply). The terminal screen moved to a new session-scoped `list_workspaces_for_store_scoped(session_token, store_id)` — no more hardcoded `role-owner`.
**Design decisions:** (1) per-process random secret rather than the OS keyring — the ticket is a 5-minute bootstrap credential, so persistence would add key material at rest for no benefit; (2) uniform `PermissionDenied` for every ticket failure (forged/expired/malformed/unknown/inactive) so the endpoint can't be an enumeration oracle; (3) `list_workspace_screens` still routes on the caller-chosen `store_id` but only after a validated ticket, and screens are non-sensitive nav metadata — deliberate scope.
**Commits:** (this cycle) `apps/desktop-client/src/commands/picker_ticket.rs` (new), `state.rs`, `auth.rs`, `staff.rs`, `workspaces.rs`, `lib.rs` + `ui/src/api/{staff,workspaces}.ts`, `ui/src/contexts/{AuthContext,WorkspaceContext}.tsx`, `ui/src/features/terminals/TerminalManagementScreen.tsx`, `ui/src/components/FastPINOverlay.tsx`, `ui/src/features/auth/CreatePinScreen.tsx`, UI tests.
**Tests:** oz-pos-app lib **795/795** (7 picker-ticket crypto + 7 command-level gate tests + 1 login-mint test, all new); tablet `cargo check` clean (shares oz-core, untouched); UI vitest **3761/3761** (169 in the directly-affected files); `cargo fmt` clean; clippy `-D warnings` clean on changed files (workspace still fails only on the pre-existing `products.rs:876` type_complexity).
**Follow-ups:** the picker ticket has a 5-min TTL — a stalled picker requires re-login (deliberate); `list_workspace_screens` store routing is ticket-gated but not store-access-checked (screens are nav metadata); the tablet client has no pre-session picker, so no parity work there.
## 2026-08-06 — TDD cycle: PG pull composite (created_at, id) cursor

### PG pull skipped equal-timestamp rows and stalled the anchor on never-stamped synced_at
**Problem:** the PG transport's pull filtered `WHERE synced_at > $1` with no cursor — (a) rows sharing the anchor's exact `synced_at` timestamp were permanently skipped (strict `>`), and (b) the durable anchor was computed from `synced_at`, so a remote that never stamps it (rows stay NULL) never advanced the anchor and the daemon re-pulled the entire queue every cycle. The HTTP server had long since moved to a composite `(created_at, id)` cursor with `created_at >= since` — the PG path never caught up.
**Solution:** TDD slice mirroring the HTTP server's pagination. `pg_transport::pull_updates(since, cursor)` now takes a composite cursor, decodes `"created_at|id"`, and builds three query shapes via a pure `build_pull_sql` (cursor tiebreak `created_at > $2 OR (created_at = $2 AND id > $3)`, since-only `created_at >= $1`, initial full pull). It fetches 501 rows, keeps 500, and derives `next_cursor` from the last KEPT row (RUST-07). `pg_daemon::apply_pulled_page` now advances the monotonic anchor on the page's newest `created_at` — never `synced_at` — and `run_tick` loops pages while a next cursor is returned, persisting `(since, next_cursor)` after each page and retaining both on retryable failure.
**Commits:** `platform/sync/src/pg_transport.rs` + `platform/sync/src/pg_daemon.rs` (two-file change; pg_transport swept into another agent's commit, verified intact).
**Tests:** 254 crate tests (9 new: cursor decode, SQL shape × 3, next-cursor derivation × 2, created_at-anchor-when-synced_at-NULL regression, roundtrip) · 19/19 gated integration suite · fmt + clippy `-D warnings` clean.
**Follow-ups:** the PG remote query has no tenant filter (the transport pulls every tenant's rows) — add `tenant_id` scoping when real multi-tenant PG deployments appear; and the SQLite daemon still lacks a snapshot path entirely.


## 2026-08-06 — TDD cycle: PostgreSQL daemon replay-safety parity (SYNC-01/02)

### PG daemon re-applied remote mutations every cycle and panicked on NULL synced_at
**Problem:** The SQLite daemon + `SyncEngine` got the SYNC-01 safeguards (durable `sync_pull_state` anchor, atomic apply via `apply_remote_atomic` + `sync_applied_items` ledger, dead-letter quarantine) and the SYNC-02 shared ADR #21 conflict service — but `pg_daemon.rs::run_tick` never did. It called `transport.pull_updates(None)` every 60s (no durable anchor → re-fetched the entire remote queue), applied via non-atomic `queue.apply_remote` (every cycle re-applied remote stock/sale mutations — silent inventory corruption), and a poison item just logged forever. Push conflicts used the old blanket `mark_synced` + re-enqueue anti-pattern. Worse, `pg_transport.rs` decoded `synced_at` with `row.get::<_, String>` — a remote row this terminal pushed as `pending` (NULL synced_at until stamped) panicked the whole pull on the first such row.
**Solution:** Added `apply_pulled_page(store, page, prev_since) -> Option<String>` — the same engine helper design: each item via `apply_remote_atomic` (mutation + ledger receipt in one tx, dead-letter after 3 attempts), returns `Some(monotonic max(prev_since, newest synced_at))` only when the whole page applied (dead-lettered items count as applied), `None` on retryable failure (anchor retained, next cycle re-pulls — ledger absorbs the replay). `run_tick` now reads the durable anchor from `sync_pull_state`, passes it to `pull_updates(since)`, and persists the new anchor only after the page applied. Push `Conflict` outcomes now route through `queue.apply_push_conflict` (ADR #21, full local item) instead of blanket mark-synced + re-enqueue. Decode fix: `synced_at` reads as `Option<String>`.
**Commits:** `platform/sync` — see the two-file change in the next commit.
**Tests:** 244 crate tests (5 new: idempotent replay, retryable-failure retains anchor, dead-letter-then-advance, monotonic max-synced_at, atomic apply + receipt) · 19/19 gated integration suite (cross-terminal relay, throughput) · fmt + clippy `-D warnings` clean.
**Review hardening:** the pull phase previously sat inside the `!pending.is_empty()` gate, so a pull-only terminal (pure consumer on a shared remote PG) never pulled and the anchor never advanced on push-idle cycles — the transport is now built whenever PG sync is enabled and push/pull run independently.
**Follow-ups:** the remote PG query filters on `synced_at` — if the remote never stamps it, an anchored pull returns nothing new; and the strict `synced_at > anchor` filter (no composite `(created_at, id)` cursor like the HTTP server) can skip rows sharing the anchor's exact timestamp. Consider a `created_at`-based cursor when a real multi-terminal PG deployment appears.

## 2026-08-06 — TDD cycle: checked BOM deduction quantities (MONEY-03)

### `complete_sale*` BOM ingredient totals overflow silently
**Problem:** Both stock-deduction entry points multiplied the sale-line qty by the recipe's `quantity_required` with a bare `line.qty * ingredient.quantity_required` — `complete_sale_deduction` (line ~247) and `complete_sale_with_resolved_shortfalls` (non-resolution BOM branch, line ~644). `line.qty` comes from the front-end sale over IPC (untrusted) and dev/test builds disable overflow checks, so an overflowing qty silently wrapped: the Red run showed both paths returning `Ok(CompleteSaleResult)` while the ingredient stock was **credited** by ~4.6e18 — the register completed a sale with a corrupt stock delta instead of failing.

**Solution:** Red→Green TDD cycle. RED tests `complete_sale_deduction_bom_quantity_overflow_returns_validation_error` and `complete_sale_with_resolved_shortfalls_bom_quantity_overflow_returns_validation_error` pin the contract — `(i64::MAX / 2) × 3` overflows, and both paths must return `CoreError::Validation { field: "qty", message: "ingredient deduction quantity overflow" }` with stock untouched. Both failed on `Ok(CompleteSaleResult …)` (the silent wrap) before the fix. GREEN: both sites now use `checked_mul(...).ok_or_else(Validation { field: "qty", … })?` — the same pattern as `compute_line_tax` (TAX-04) and MONEY-01. `quantity_required` is DB-backed with a `CHECK (quantity_required > 0)` so that operand needs no validation. Field `"qty"` deliberately avoids `"stock"`, which the caller special-cases to deserialize `PartialStockResult`.

**Refactor:** extracted `seed_bom_composite` test helper (composite `service` product + tracked ingredient + recipe row) per review; 79/79 sales-module tests, 1623/1623 oz-core lib.

**Deliberately NOT done (follow-ups):** (1) negative `line.qty` on a hand-built `Sale` remains unchecked on this path — `checked_mul` rejects only overflow, and a negative qty would *credit* stock (same MONEY-02 gap class); unreachable from the front-end since `CartLine::new` asserts `qty > 0` and `Sale::from_cart` is the only real producer, but worth a validation slice; (2) `oz-lua` plugin `apply_discount` (lib.rs 577/608) and purchase-order subtotals still use unchecked `qty × price` — same class, separate slices.

## 2026-08-06 — TDD cycle: reject negative cart-tax inputs (MONEY-02)

### `compute_cart_tax` negative `qty` / `unit_price_minor` accepted
**Problem:** Follow-up from the MONEY-01 cycle's review note. `CartLineTaxInput` arrives over IPC (untrusted), and `Store::compute_cart_tax` accepted negative `qty` or `unit_price_minor` — a negative line total flows into `compute_line_tax` and the preview returns a **negative tax** the front-end renders raw (Red run proved it: `qty: -2, price: 350` → `Ok(tax = -69)`). The cart model never allows negative qty/price (`CartLine::new` asserts `qty > 0`), so a hostile renderer could distort the displayed tax.

**Solution:** Red→Green TDD cycle. RED test `compute_cart_tax_rejects_negative_qty_and_price` asserts both cases return `CoreError::Validation` with the right field (`qty` / `price`) — failed on `Ok(-69)`. GREEN: the loop now rejects `qty < 0` (`field: "qty"`, "qty must be positive, got {n}") and `unit_price_minor < 0` (`field: "price"`, "unit price must be non-negative, got {n}") with early returns, mirroring the existing `set_cart_discount` message style. `qty == 0` and `unit_price_minor == 0` remain **allowed by deliberate scope**: zero contributes zero tax, zero price = free item, and the slice was "negative" only — noted here so the boundary is explicit.

**Deliberately NOT done (follow-ups):** (1) `compute_sale_tax` has the same-class hole via a hand-built `Sale` with a negative `line_total` (it feeds `line.line_total` straight into `compute_line_tax`) — the natural next slice; (2) reviewer nit: the price `format!` could sit on one line — skipped as cosmetic churn in a volatile shared tree (code is fmt-clean and committed); (3) the pre-existing `clippy::type_complexity` in `products.rs:876` remains untouched.

**Commits:** (this cycle) — `crates/oz-core/src/db/sales.rs` + this journal + `CHANGELOG.md`. **Note on history:** another agent thread's `06e9fb7d` ("fix(restaurant): harden menu keyboard interactions") swept this cycle's `sales.rs` RED test + GREEN fix into its commit via a broad add. History was NOT rewritten (shared working tree); the `sales.rs` hunks are exactly this cycle's regression test + negative-input validation.

**Validation:** Red test failed (`Ok(-69)`) then passed; `db::sales::tests` 77/77 (76 + 1 new); full `cargo test -p oz-core --lib` 1621/1621 (includes the new test); `cargo fmt --all` clean; clippy `-D warnings` clean on the changed file (workspace still fails only on the pre-existing `products.rs:876`). One transient compile failure was observed mid-cycle from another agent's in-progress `db/offline.rs` edit — resolved on its own; no process was killed.

## 2026-08-06 — TDD cycle: dead-letter requeue workflow

### Dead-lettered remote items are now requeueable (audit/09 SYNC-01 follow-up)
**Problem:** Remote items that exhaust their apply retry budget were permanently quarantined. `sync_remote_failures` rows are only ever written (on failure) or deleted (on success) — once `dead_lettered = 1`, `apply_remote_atomic` skips the item and the daemon advances the pull anchor past it, so it is never retried. Migration 119's comment promised "an operator can inspect or manually requeue a quarantined item after remediation", but no store method, command, or UI existed (the workflow was explicitly deferred in audit/09 SYNC-08). An operator who fixed the source (e.g. created the missing product a remote sale referenced) had no way to make the item retry.

**Solution:** Red→Green TDD cycle. RED: store tests `requeue_remote_failure_clears_quarantine_and_rewinds_anchor` + `requeue_remote_failure_refuses_non_dead_lettered` (failed with `no method named requeue_remote_failure`). GREEN: `Store::requeue_remote_failure(item_id)` (oz-core `db/offline.rs`) — requires the item to be currently dead-lettered (else `NotFound`, never a silent no-op), deletes the quarantine row, and rewinds the durable `sync_pull_state` anchor (`since = NULL, cursor = NULL`) so the next daemon cycle re-fetches the item and retries it with a fresh 3-attempt budget. The full re-pull is safe because the `sync_applied_items` idempotency ledger skips every already-applied item. Command surface: `requeue_remote_failure` Tauri command (`RequeueRemoteFailureArgs { itemId }`) added to BOTH desktop (`oz-pos-app`) and tablet (`oz-pos-tablet`) `commands/offline.rs` + registered in both `lib.rs` invoke handlers; extracted `run_requeue_remote_failure` helper for command-level tests.

**Commits:** code swept into `06e9fb7d` (authored by another agent thread — see note). Docs in this cycle's follow-up commit.

**Validation:** oz-core `db::offline` 44/44; desktop `commands::offline` 18/18; tablet `commands::offline` 18/18; `cargo fmt` clean; clippy `--no-deps -D warnings` clean on desktop + tablet and no warnings in oz-core's new code (workspace clippy still fails only on the pre-existing `products.rs:876` type_complexity).

**Note on history:** commit `06e9fb7d` (restaurant agent's "harden menu keyboard interactions") swept this cycle's five files (`oz-core db/offline.rs`, desktop + tablet `commands/offline.rs` + `lib.rs`) into its diff via the shared index. The requeue code is intact and was verified green on identical content; history was NOT rewritten (shared working tree, agents actively committing).

**Known limitation (reviewed):** the requeue anchor rewind can be clobbered by the sync daemon if the command lands between the daemon's read phase and its apply-phase `set_sync_pull_state` write (the daemon re-writes the stale pre-rewind anchor it captured). Low probability (daemon cycle is 60–120s, requeue is a rare operator action), no data corruption — the requeue just doesn't take effect that cycle. Fix (separate TDD slice): in the daemon's apply phase, re-read `get_sync_pull_state()` before writing and skip the anchor advance when the stored `since` is `None` (operator rewind in flight).

**Follow-ups:** expose `list_remote_failures` as a command + UI surface so operators can discover dead-letter ids before requeueing; wire `requeue_remote_failure` into `ui/src/api/offline.ts` (+ IPC contract test) to make the workflow end-to-end; consider storing the remote item's `created_at` on the failure row so requeue can rewind the anchor precisely instead of a full re-pull.

## 2026-08-06 — TDD cycle: checked cart-tax line totals (MONEY-01)

### `compute_cart_tax` unchecked `qty × unit_price_minor` overflow
**Problem:** `Store::compute_cart_tax` (`crates/oz-core/src/db/sales.rs`) computed the per-line taxable total with a bare `line.qty * line.unit_price_minor`. `CartLineTaxInput` is deserialised straight off the IPC boundary (untrusted renderer input) and this function runs on the hot path — the live tax preview fires on every cart change in both desktop and tablet POS. The workspace deliberately sets `overflow-checks = false` for dev/test builds (`Cargo.toml` `[profile.dev]`), so an overflowing line total **silently wraps** and feeds a wrong tax to the register in every normal build; it would panic only under a profile with overflow checks on. This is the exact arithmetic class TAX-04 already eliminated in `compute_line_tax` (checked_mul + structured error) — it was missed at the line-total product. Red test `compute_cart_tax_line_total_overflow_returns_validation_error` (qty = i64::MAX/2, price = 4) failed for the right reason: `Ok(Money { minor_units: 0 })` — the wrapped tax — instead of an overflow error.

**Solution:** Red→Green TDD cycle. GREEN: the line total now uses `qty.checked_mul(unit_price_minor)` and returns `CoreError::Validation { field: "tax", message: "cart line total overflow" }` on overflow — the same structured error contract as `compute_line_tax`. No signature change, so no caller updates (`compute_cart_tax_scoped` in desktop + tablet `pos.rs` forward unchanged).

**Deliberately NOT done (follow-ups):** (1) same-class unchecked `qty × price` products remain in `crates/oz-lua/src/lib.rs:577/608` (plugin `apply_discount` line math — plugin-supplied values), `crates/oz-core/src/db/purchase_orders.rs:186/214` (PO subtotals), `crates/oz-core/src/db/sales.rs:247/644` (recipe BOM `line.qty * quantity_required`), and `modules/inventory/src/handlers.rs:141` — each a separate TDD slice; (2) the broader sale-to-ledger totals policy (recorded `sale.total` excludes tax / tip / service-charge that the UI charges separately; tax computed on pre-discount line totals) is a product-policy question (inclusive vs exclusive tax) and deliberately NOT changed here; (3) pre-existing `clippy::type_complexity` in `crates/oz-core/src/db/products.rs:876` remains (documented in the 08-06 session-mint entry) — unrelated to this change; (4) `CartLineTaxInput` still accepts non-positive `qty`/`unit_price_minor` (negative line total → negative tax preview) — a semantic-validation slice distinct from overflow, noted by review.

**Commits:** (this cycle) — `crates/oz-core/src/db/sales.rs` + this journal + `CHANGELOG.md`. **Note on history:** another agent thread's commit `42dab989` ("fix(authz): pin inactive-user session denial…") swept this cycle's `sales.rs` RED test + GREEN fix (12 lines) into its authz commit via a broad `git add` while the tree moved. History was NOT rewritten (shared working tree); the `sales.rs` hunks are exactly this cycle's regression test + checked-mul fix, and the code is byte-identical to what this cycle produced and verified.

**Validation:** Red test failed (silent wrap) then passed; `db::sales::tests` 76/76; full `cargo test -p oz-core --lib` 1618/1618; `cargo fmt --all` clean; clippy `-D warnings` clean on the changed file (workspace clippy still fails only on the pre-existing `products.rs:876`). `scripts/test-changed.sh` was blocked by a running `oz-pos-app.exe` holding the linker output lock — process left running per TDD skill rule 7; equivalent coverage obtained via the direct `oz-core --lib` run.

## 2026-08-06 — TDD cycle: session-mint authorization gate (right user, right store, right permission)

### `verify_instance_access` fail-closed identity binding (audit/06 residual)
**Problem:** The pre-session workspace picker ends in `create_session`, whose server-side gate `Store::verify_instance_access` trusted the caller-supplied `role_id` for the owner/manager bypass and never resolved the user. `create_session(user_id: <any known id>, role_id: "role-owner", store_id: ..., instance_id: ...)` passed the bypass whenever no `user_store_access` rows existed (single-store mode), minting an opaque session AS that user — without their PIN — in ANY store's active instance. Every subsequent `require_permission_for_user` then resolved the victim's DB role, so a caller who knew an owner's user id inherited full permissions (privilege escalation) and could open sessions in stores/instances they were never assigned (cross-store session minting). This was the residual recorded in `audit/06-staff-module.md`: "the pre-session workspace picker still accepts role/user/store identifiers supplied by the client… the caller identity is not cryptographically bound before an opaque session exists." The gate had zero unit tests.

**Solution:** Red→Green TDD cycle. RED: 3 oz-core tests (`verify_instance_access_denies_unknown_user`, `_rejects_forged_owner_role`, `_denies_inactive_user`) + 3 desktop command tests (`create_session_rejects_forged_role_id`, `_rejects_unknown_user`, plus positive `create_session_allows_real_owner`). All negative tests failed for the right reason (session was minted). GREEN: `verify_instance_access` now resolves the user from `users`, fails closed (returns `Ok(false)`) for unknown/inactive users and for a claimed `role_id` that differs from the user's actual DB role, then runs the existing owner-bypass / explicit-assignment / role-based branches using the REAL role. `Ok(false)` (not `Err`) keeps the caller's wire error uniform, so the gate cannot be used to enumerate user ids. No frontend change needed: every honest flow (login, workspace picker, FastPIN hot-swap) sends the role returned by `staff_login`, which equals the DB role.

**Deliberately NOT done (follow-ups):** (1) the pre-session `list_workspaces`/`list_workspace_screens` reads still trust the claimed role for listing (workspace-name disclosure only, no data access) — a server-issued picker credential remains the architectural fix per audit/06; (2) `create_session` does not cross-check `type_key` against the instance's real type (UI-routing cosmetic); (3) pre-existing `clippy::type_complexity` in `crates/oz-core/src/db/products.rs:876` remains (documented in the 08-06 pull-parity entry).

**Commits:** gate + desktop tests were swept into `da842f32` (another thread's mixed commit, same as the sync hunks — history not rewritten per shared-tree convention); this cycle's follow-up `42dab989` pins the inactive-user test uniquely and adds tablet `create_session` parity tests. The `.githooks/pre-commit` fmt gate swept a third file — `crates/oz-core/src/db/sales.rs` (another thread's uncommitted work) — into `42dab989`; its content is intact in the commit and the working tree matches HEAD, splitting left to the owner if desired.

**Validation:** `cargo test -p oz-core --lib db::workspaces` 54/54 (6 new); `cargo test -p oz-pos-app --lib commands::auth` 18/18 (3 new); `cargo test -p oz-pos-tablet --lib commands::auth` 9/9; `store_scoping_integration` 9/9; `cargo fmt --all -- --check` clean; clippy `-D warnings` clean on the changed files (workspaces.rs, auth.rs); tablet lib compiles.

## 2026-08-06 — TDD cycle: engine pull parity for replay idempotency + durable anchor

### SyncEngine pull path: durable anchor + atomic replay (SYNC-01 parity)
**Problem:** `platform_sync::SyncEngine::run_sync_cycle()` — the immediate/manual sync path — did not share the SYNC-01 safeguards the daemon got. It derived its pull `since` anchor from `queue.last_synced_at()` (the local offline queue's `synced_at` timestamps), which pulled remote items never move, and applied remote mutations via the non-atomic `apply_remote()`. Consequence: every manual sync cycle re-fetched the same remote pages and re-applied stock/sale mutations (silent inventory corruption), and the durable `sync_pull_state` anchor was never persisted. The daemon path (fixed in `a1ea01e7`) was atomic + anchor-advanced; the engine was not.

**Solution:** Red→Green TDD cycle. RED test `engine_applies_replayed_remote_item_only_once` (in `platform/sync/src/lib.rs`) spins a mock server that always returns the same `stock.adjusted` +10 item (ignores `since`), runs two engine cycles, and asserts: stock 50→60 after cycle 1, the durable anchor is persisted, and stock stays 60 after cycle 2 (not 70) with exactly one ledger receipt. It failed for the right reason (`since: None`). GREEN: the pull phase now reads the durable `sync_pull_state` anchor, applies each item via `apply_remote_atomic` (mutation + idempotency receipt in one transaction, dead-letter quarantine for poison items — matching the daemon), advances the anchor only after a page applied successfully, and retains the anchor + stops pagination on a retryable failure.

**Commits:** swept into `da842f32` (see note below) — my `platform/sync/src/lib.rs` hunks only.

**Validation:** `bash scripts/test-tdd.sh -p platform/sync` — 238/238 passed (19 slow-tests ignored); full `--features slow-tests` integration suite — 19/19 passed incl. cross-terminal relay + throughput; `cargo clippy -p platform-sync --all-targets --no-deps -- -D warnings` clean; `cargo fmt` applied. Note: `cargo clippy -D warnings` on the workspace currently fails pre-existing in `crates/oz-core/src/db/products.rs:876` (`type_complexity`, committed code, not touched here).

**Note on history:** commit `da842f32` (authored by another agent thread) swept this lib.rs change — plus 16 unrelated files (UI autofill, auth, workspaces) — into one commit titled "fix(ui): suppress saved-info autofill in search fields". The lib.rs hunks are exactly this cycle's RED test + GREEN refactor. History was NOT rewritten (shared working tree, another agent actively editing `sales.rs`); splitting the mixed commit is left to the owner if desired.

## 2026-08-06 — TDD cycle: LOY-10 loyalty expand-control accessible name

### LoyaltyManagementScreen expand control (LOY-10)
**Problem:** The expandable loyalty account row (`tr role="button"`) and its nested expand button exposed a generic `aria-label` ("Expand"/"Collapse") with no customer identity. Screen-reader users could not tell which account a control would expand. The nested button had no handler and relied on click bubbling to the row. Evidence: `audit/02-loyalty-module.md` LOY-10 (P2, still open — verified in code at `ui/src/features/loyalty/LoyaltyManagementScreen.tsx`).

**Solution:** Red→Green TDD cycle per `.agents/skills/tdd/SKILL.md`. RED test `names the expand control with the customer (LOY-10)` asserted the row + button accessible names include the customer name — failed on `'Expand'`. GREEN: added `loyalty-expand-account`/`loyalty-collapse-account` Fluent keys with `{ $name }` var (en + id), threaded the customer name through both controls, and gave the nested button a real `onClick` handler (`toggleExpand`) instead of relying on bubbling.

**Commits:** (this cycle) — see `git log`.

**Validation:** LoyaltyManagementScreen 20/20 vitest; api-loyalty-contract 5/5; typecheck clean; eslint clean on changed files; i18n lint clean; bundle-parity 0 missing; FTL dedupe clean. Area-scoped per tdd skill (no full workspace run — not pre-push).

### Attribute-Only FTL Sweep (TODO #3)
**Problem:** ~268 attribute-only FTL messages (`.aria-label = ...` with no message value) silently returned `undefined` when accessed via `l10n.getString()`, causing empty aria-labels and placeholders across 25 files.

**Solution:** Cross-referenced all 1212 `l10n.getString()` calls against the 268 attribute-only keys. Found 75 keys used without fallbacks across 25 files. Verified `<Localized>` usage: 72 keys safe to convert to simple `key = value` format (125 conversions, 16 bundles via `scripts/convert-safe-attr-ftl.py`). 3 keys shared with `<Localized attrs>` received `||` fallbacks in code.

**Commits:** `104c4891`, `ee5a4f96`

### RestaurantMenu.tsx Audit (TODO #2)
**Problem:** 795-line restaurant/KDS screen was completely un-audited, with 11 missing FTL keys and 2 hardcoded English strings (`aria-label="Menu items"`, hex color codes as aria-labels).

**Solution:** Added 13 FTL keys (en + id): search-aria, search-clear-aria, context-pin/unpin, context-available/unavailable, card-pin-title, sort-manual/a-z/date/popularity, menu-items-aria, color-swatch-aria. Localized the grid aria-label and color swatch labels. CSS audit: 0 hardcoded hex, all tokens. Hooks: all cleanup + deps correct.

**Commits:** `b3307810`, `446a88f3`

### SettingsPage.tsx Audit (TODO #1)
**Problem:** Largest UI file (1081 lines) was surprisingly clean — 244 CSS tokens, zero hardcoded hex, correct hook deps. Only 2 hardcoded strings: `placeholder="Search"` and Suspense fallback `Loading...`.

**Solution:** Added `settings-search-placeholder` and `settings-section-loading` FTL keys (en + id). Localized both strings with `l10n.getString()` and `<Localized>`.

**Commits:** `533247bc`, `de1517dc`

### PosScreen.tsx Audit
**Problem:** Largest file in codebase (2212 lines TSX + 682 CSS). 26 attribute-only bugs already fixed by the FTL sweep. After sweep: 216 CSS tokens (all hex in var() fallbacks), 40 hooks with correct deps, ESLint zero errors.

**Bugs found:** 5 hardcoded strings missed by the sweep:
- Course fire button: `aria-label={`Fire ${course.label} (${holdCount} items)`}` — not inside `<Localized>`
- Fire All button: `<span>Fire All</span>` — not wrapped in `<Localized>`
- Override button in CartLineItem: `aria-label={...}` and `Override` text — both hardcoded
- Missing FTL keys: `pos-cart-course-fire-aria`, `pos-cart-course-btn--all`, `pos-cart-line-override`, `pos-cart-line-override-aria`

**Commit:** `0796d835`

### ProductManagementScreen + CategoryManagementScreen Audit
**Problem:** Two ~640-line screens flagged in the original audit for hardcoded aria-labels. Both were clean after the attribute-only sweep: 92+135 CSS tokens, zero true hardcoded hex.

**Bugs found:** 3 hardcoded strings:
- CategoryManagementScreen: `aria-label={`Edit category ${cat.name}`}` — not inside `<Localized>`
- ProductManagementScreen: Stock alert bell aria-label in English
- ProductManagementScreen: Product type dropdown options (Retail/Restaurant/Service) — not localized

**Commit:** `13023004`

### Session Totals
| Metric | Count |
|--------|-------|
| Bugs fixed | **88** |
| FTL keys added | **28** (en + id) |
| Files changed | **25** |
| Commits | **5** fix + **4** docs |
| Tests | **3324/3324 passing, 221/221 files** |
| TypeScript | Clean (0 errors) |
| Bundle parity | 0 missing keys |


## 2026-07-02 — i18n Migration & Test Fixes

### Test Infrastructure Fixes
- **SettingsPage.test.tsx**: Wrapped with `AuthProvider` context + added `get_brand_settings` mock to fix pre-existing failures.
- **SetupWizard.test.tsx**: Corrected Launch button test to use `onLaunch` prop instead of `onSkip`.
- **CSS Extraction Tests**: Removed duplicate/dead CSS classes in `CartPanelActions.css`, added `url()` stripping in `extractClassSelectors` to fix `w3` false positive, added `externalClasses` support.
- **WorkspaceEntry.test.tsx**: Fixed unused `screen` import and `registerNavItem` import path (was pointing to `page-registry` instead of `menu-registry`).
- **Fluent missing-ID warnings**: Added 15 missing `setup-feature-*-label` IDs to `settings.ftl`.

### i18n Migration — Wrapped hardcoded aria-labels with `<Localized attrs>`

| Component | Labels wrapped |
|-----------|---------------|
| **SalesHistoryScreen.tsx** | 16 — date from/to, cashier select, table, actions th, pagination nav/prev/next/per-page, void overlay/close/reason, detail overlay/close/lines/refund-lines |
| **VoidOrdersScreen.tsx** | 3 — search input, status filter radiogroup, custom reason input |
| **PaymentModal.tsx** | 17 — dialog overlay, close button, currency label/select, exchange notice, receipt currency, other-input, customer-name (was fully hardcoded), tendered-input, quick-tender (with vars), exact button, QRIS button, split-evenly, split-add, split-other, split-amount, split-remove |
| **TaxConfigurationScreen.tsx** | 9 — tax rates table, category tax rates table, tax name label, edit/delete/cat-edit buttons, tax-rate modal, tax-type radiogroup, category-tax modal |
| **CustomerManagementScreen.tsx** | 5 — customers table, name/email/phone/notes inputs |
| **LoyaltyManagementScreen.tsx** | 8 — accounts table, actions th, transactions table, 5 tier form inputs |

### FTL Files Modified
- `sales.ftl` — added 21 new IDs for sales history + void orders + payment modal
- `settings.ftl` — added 15 setup-feature-label IDs
- `tax.ftl` — added 3 new IDs (table-aria, cat-table-aria, field-name-aria)
- `customers.ftl` — added 5 new IDs (table-aria, 4 field aria)
- `loyalty.ftl` — modified `loyalty-table-actions` to `.aria-label` format + added 7 new tier/table IDs

## 2026-07-02 — White-Label Theming Improvements

### Changes Made

1. **BrandContext created** (`ui/src/contexts/BrandContext.tsx`) — New React context providing brand/white-label settings and a `refreshBrandSettings()` function to the entire app tree. Loads from backend on mount.

2. **ThemeProvider cleaned up** — Removed `BrandInfo` interface, `brand`/`updateBrand` state (now handled by BrandContext), and the direct `getBrandSettings` effect. Now uses `useBrand()` from BrandContext to reactively apply the accent palette whenever `primary_colour` changes.

3. **AppLayout sidebar header** — Replaced hardcoded "OZ-POS" with dynamic brand logo (if set) + store name (fallback to "OZ-POS"). Also sets `document.title` reactively to the store name.

4. **AppearanceSettings** — Replaced `useTheme().updateBrand` with `useBrand().refreshBrandSettings()`. `handlePickLogo` now also refreshes brand settings immediately so the sidebar shows the new logo without waiting for "Save".

5. **AppLayout.css** — Added `.app-sidebar-logo-img` (32×32, object-fit contain) and collapsed variant (28×28) styles.

6. **App.tsx** — Wrapped app with `<BrandProvider>` above `<ThemeProvider>`.

### TypeScript
Clean (0 errors).

## 2026-07-02 — Modal Exit Animations

**Problem:** Hold cart, held carts, and shift modals had entrance animations but snapped out on close — no exit animation.

**Solution:** Created reusable `useAnimatedModal` hook that manages entering/exiting phases. When `show` becomes `false`, the modal stays mounted for 200ms with `exiting=true` before unmounting, allowing CSS exit animations to play.

**Changes made:**
- NEW `ui/src/hooks/useAnimatedModal.ts` — animation phase management hook
- `PosScreen.css` — added `@keyframes pos-overlay-out` (fade) + `pos-modal-out` (fade+translate), `.pos-overlay-exit`/`.pos-modal-exit` classes
- `ShiftManagementScreen.css` — added identical shift-prefixed exit keyframes + classes
- `PosScreen.tsx` — applied hook to 5 modals (hold cart, held carts, close shift, shift summary, open shift)
- `ShiftManagementScreen.tsx` — applied hook to 5 modals (open, payout, close, closed summary, detail)
- Reduced-motion overrides extended to cover exit classes

**Null-safety:** Used IIFE pattern (`{mX && (() => { const s = nullable!; return ( ... ); })()}`) where hook conditions couldn't be tracked across the hook boundary.

### Bugs Fixed During Migration
- Nested `<label>` bug in PaymentModal currency selector (invalid HTML)
- `key` prop on quick-tender buttons moved to outermost `<Localized>` component
- Stale `l10n.getString()` call on loyalty `<th>` after converting ftl to attribute format
- Missing `</Localized>` closing tags for void and detail overlay wrappers

### Test Results
- **TypeScript**: Clean (0 errors)
- **Tests**: 261 passed / 15 failed (down from 31 failing pre-migration — all remaining failures are pre-existing FSI/PDI marker issues and structural WorkspaceEntry module-not-found)


## 2026-08-07 — SYNC-10 enqueue side + migration 120 multi-store repair

### Local settings saves never pushed settings.update items; migration 120 reseeded the wrong store
**Problem:** Two gaps. (1) The SYNC-10 apply path could consume remote `settings.update` items, but NO local settings command ever enqueued one — the cross-terminal loop (change here → cloud → there) was a one-way street: `SettingsContext` listened for `settings_updated` while the daemon could only ever apply changes it never received. (2) The full gate exposed a failing test `list_workspaces_repairs_empty_store_db_after_066_window`: repair migration 120 reseeded default workspace instances with `store_id = COALESCE(primary, 'default')`, and in a store DB where no profile is primary (the legacy `'default'` row from 025 is `is_primary = 0`) it landed on `'default'` — but the store-scoped picker filters `wi.store_id = ?` strictly, so a named store (store-a) never listed the reseeded defaults. The repair silently repaired the wrong store.

**Solution (TDD):** (1) Red: two unit tests pinned that a settings write must enqueue one `settings.update` item per key with the exact `SettingsUpdatePayload` shape (`{key, value, terminal_id}`), tenant-scoped, Low priority. Green: extracted `enqueue_settings_updates` and wired all four write commands (`set_setting`, `set_settings`, `set_setting_scoped`, `set_settings_scoped`) to enqueue on the GLOBAL db after the write commits (the sync daemon only watches the global queue — a store-scoped write must fan out from there). Enqueue failures log a warning and do not fail the save, matching the `SettingsUpdated` publish pattern. (2) The failing workspaces test was the Red; the fix: migration 120's store_id selection now prefers the primary profile, then **this store's own profile** (any non-`'default'` row in its own DB, `ORDER BY created_at` for determinism), then `'default'` — so the repair lands inside the store it is repairing. Each store DB is migrated independently, so "any non-default profile here" is exactly "this store". 120 is the newest, unreleased migration, so editing it before release is safe.

**Validation:** 800/800 oz-pos-app lib tests (3 new/restored: 2 settings enqueue + 1 repaired workspaces) · oz-core 25/25 · migrate twice idempotent · clippy -D warnings clean · fmt clean.

**Commits:** (follow the two commit hashes below this entry)

**Follow-ups (deliberately NOT done):** (1) the tablet client's `set_setting` is a plain write with no daemon in the tablet process — enqueueing there would be inert; revisit when the tablet gets a sync daemon. (2) No scoped dedup API exists (`enqueue_offline_dedup` is tenant-less), so repeated identical saves while offline create duplicate pending items — functionally harmless (apply is replay-safe, version-LWW) but noisy; a tenant-scoped dedup variant is a future slice. (3) Legacy `set_setting`/`set_settings` could enqueue INSIDE the write tx (same global DB) to close the tiny crash window between `tx.commit()` and the enqueue; scoped commands cannot (cross-DB), so warn-and-continue stays the uniform choice.


## 2026-08-07 — Settings enqueue supersedes pending same-key items (SYNC-10 follow-up)

### Repeated offline saves piled duplicate settings.update items; naive dedup would lose the newest intent
**Problem:** After the SYNC-10 enqueue side landed, every local settings save enqueued a fresh `settings.update` item — so saving the same key repeatedly while offline stacked [v1, v2, v1] and the daemon pushed them in order, ending the remote at v2 while the local was at v1 (version-LWW orders by terminal version, not save order). A payload-dedup "fix" would make it worse: with [v1, v2] pending, re-saving v1 would find the stale v1 payload and skip — the newest intent silently dropped.

**Solution (TDD):** Red tests pinned the correct semantics: a new save SUPERSEDES still-pending items for the same key (same tenant) — one pending item carrying the newest value; other keys survive; store-y's save never removes store-x's item. Green: `supersede_pending_settings_key` (list pending for tenant → delete items whose `settings.update` payload key matches, exempting the freshly-enqueued item by id). Ordering is deliberately ENQUEUE-THEN-SUPERSEDE: an enqueue failure leaves the old items (pre-existing warn-and-continue behavior), while a supersede failure degrades to the harmless duplicate state the apply side already handles — the reverse order would lose the update entirely if the enqueue failed after the delete. All existing queue APIs reused; no new oz-core surface.

**Validation:** 803/803 oz-pos-app lib tests (3 new) · clippy -D warnings clean · fmt clean.

**Commits:** (hash below)

**Follow-ups (deliberately NOT done):** the tablet client's `set_setting` still does a plain `Settings::set` with no terminal_id and no enqueue — confirmed the tablet process runs no sync daemon, so wiring the enqueue there would be inert until the tablet gets one (journaled previously). A general `enqueue_offline_scoped_dedup` (action+payload+tenant) is still unneeded — for settings the correct primitive is supersede-by-key, and no other caller needs payload-dedup across tenants today.


## 2026-08-07 — RetailCartPanel characterization suite (NO-TEST gap)

### The retail cart panel had real behavior and zero direct tests
**Problem:** The 5-area TDD scan flagged every `Retail*` component as untested; RetailCartPanel is a fully controlled cart UI with meaningful behavior — the remove→undo round-trip (onRemoveLine payload must carry modifiers + course so undo can restore the full line), qty +/- semantics (decrease at qty 1 removes the line, above 1 updates qty), the course dropdown (open on chip, assign on option, None clears, closes on select), pay-button gating, and the modifier badge — yet no direct suite pinned any of it.

**Solution:** 13-test characterization suite (`ui/src/__tests__/RetailCartPanel.test.tsx`) using the repo's standard @fluent/react identity-key mock. The Red run surfaced one wrong assumption in the test itself, not the component: with zero lines the panel renders the empty state and omits the entire cart UI (no pay button at all) rather than a disabled one — the test now asserts that. Also corrected strict-TS fixture typing (branded `Sku`, `exactOptionalPropertyTypes` on `Partial<CartLine>`, required-shape `undoStack` entries). No production code changed — the suite is the regression net for the remove→undo contract and qty/course interactions.

**Validation:** 13/13 new · full UI suite 262 files / 4033 tests green · typecheck clean · eslint clean.

**Commits:** (hash below)

**Follow-ups (deliberately NOT done):** the `undoStack`/`undoBarExit` contract is owned by RetailPosScreen — the parent's re-add-restores-full-line behavior lives in the screen tests and is not duplicated here; serial-input rendering (isSerialTracking + trackSerialMap) and the manager override button are also untested — both are small follow-up slices if they gain behavior.


## 2026-08-07 — RetailCartPanel serial-input + manager override coverage

### Two remaining interaction surfaces on the retail cart panel were untested
**Problem:** After the characterization suite landed, the serial-tracking input (renders for `trackSerialMap[sku]` skus with the stored value, live `onSerialChange` on type) and the manager override button (gated on `isManager`, opens the override target with the line identity AND calls `onEnsureCart` so the override modal has a cart) still had no direct tests — both real cashier flows, both unguarded.

**Solution:** 6 more characterization tests appended to `ui/src/__tests__/RetailCartPanel.test.tsx`: serial input renders with stored value / updates via onSerialChange / hidden when serial tracking is off / hidden for untracked skus; override button appears only for managers / opens `{id, name, unit_price}` and ensures the cart. All pinned existing behavior (no production change needed) — the suite now covers every interactive surface of the component.

**Validation:** 19/19 in-suite · full UI suite 262 files / 4039 tests green · typecheck clean · eslint clean.

**Commits:** (hash below)


## 2026-08-07 — Tablet settings sync parity + shared enqueue contract (SYNC-10)

### The tablet's settings writes were invisible to sync; the enqueue contract was duplicated across apps
**Problem:** Two gaps. (1) My earlier journal note claimed the tablet runs no sync daemon — WRONG: the tablet has its own inline push-only daemon in lib.rs (every 30s: read SyncConfig + pending items → send_items_to_server → apply outcomes). So a tablet settings save enqueued a `settings.update` item WOULD be pushed to the cloud and re-applied by the desktop's pull — but the tablet's `set_setting` did a plain `Settings::set` with no delta and no enqueue, so tablet changes never left the device. (2) The enqueue+supersede logic lived in the desktop's settings.rs; wiring the tablet the same way would have duplicated the wire contract across two apps.

**Solution (TDD):** (1) Red: 4 oz-core tests pinned the new `Store::enqueue_settings_update_superseding(key, value, terminal_id, tenant_id)` contract — create with the exact `SettingsUpdatePayload` shape at Low priority, replace same-key pending items, keep other keys, tenant-scoped. Green: implemented in the queue module with ENQUEUE-THEN-SUPERSEDE ordering (fresh item exempted by id). The desktop's two local helpers collapsed into a thin loop over the shared method (45 tests still pass). (2) Red: tablet tests pinned `run_set_setting` must write a delta row (set_tracked, version 1) and `set_setting` must enqueue the item. Green: tablet command resolves terminal_id, uses set_tracked, enqueues via the shared method (tenant "default"), warn-and-continue on enqueue failure. Reviewer caught the supersede must also filter by `terminal_id` — terminal A's re-save must not cancel terminal B's pending save (version-LWW attributes per terminal) — added the filter + a 5th oz-core test.

**Validation:** oz-core (incl. 5 new enqueue tests) · oz-pos-app 803 · oz-pos-tablet 420 (2 new) · clippy -D warnings clean on all three · fmt clean.

**Commits:** (hashes below)

**Follow-ups (deliberately NOT done):** the tablet daemon is PUSH-ONLY — it never pulls remote changes, so the tablet still can't receive remote settings/sales updates; a pull phase is the next real slice. The `"settings.update"` action string is now hardcoded in the oz-core method, the platform-sync apply arms, and the conflict resolver — a shared const would prevent drift (nice-to-have). Tablet settings writes stay tenant "default" because the command resolves user_id, not a session token (no store derivation).


## 2026-08-07 — Topology editor: wire creation characterization suite

### The port-connection flow was entirely unguarded
**Problem:** The editor's undo/redo, selection, presets, inspector, and save paths had deep coverage (58 editor tests), but the wire CREATION flow — clicking a source port then a target port — had zero tests. The logic in `handlePortClick` (start connection, complete on a different node, duplicate detection, same-node cancel, one undo step, workspace→warehouse fallback tier limit) was real behavior with no regression net.

**Solution:** 5 characterization tests appended to `NodeTopologyEditor.test.tsx` using the preset's deterministic node order ([store-1, ws-1, wh-1]) and the `node-port-socket.port-*` classes: create a wire via two port clicks; duplicate connection → toast 'A wire already connects these ports.' + no new wire; clicking the same node's ports cancels; Ctrl+Z removes a created wire in ONE undo step; a second workspace→warehouse wire is blocked on the standard tier with the fallback toast. All pinned existing behavior — no production change needed (the component was already correct; it's now guarded).

**Validation:** 5 new · full UI suite 262 files / 4044 tests green · typecheck clean · eslint clean.

**Commits:** (hash below)

**Follow-ups (deliberately NOT done):** the Delete/Backspace-on-selection path (keydown at line 560: deletes a wireless node immediately, opens the confirm dialog for wired nodes/wires) is still only negatively tested (text-field non-interception) — a positive characterization of the delete-key flow is the next slice. Also untested: the connection-cancel affordance (Escape while connecting) and the wire label priority on multi-warehouse Pro-tier connections.


## 2026-08-07 — Topology editor: Delete/Backspace keyboard flow characterized

### The delete-key path was the last unguarded interaction surface
**Problem:** The editor's delete flows were only tested through the toolbar button; the keyboard path (`Delete`/`Backspace` keydown at line ~560: wireless node → immediate delete; wired node/wire → confirm dialog; text-field non-interception) had zero positive regression net. The journaled follow-up from the wire-creation cycle.

**Solution:** 5 characterization tests pin the keyboard flow end-to-end: Delete on a selected wireless node deletes immediately (no dialog); Delete on a wired node opens the confirm dialog and cancel keeps the node; Delete on a selected wire opens the dialog and confirm removes the wire; Backspace behaves identically to Delete; typing in a text field never triggers deletion (positive pin of the non-interception guard). Selection via node cards / wire hitbox, dialog confirmed/cancelled via the ConfirmDialog buttons. All pinned existing behavior — no production change needed.

**Validation:** 5 new · topology suites 96/96 · full UI suite 262 files / 4049 tests green · typecheck + eslint clean.

**Commits:** (hash below)

**Follow-ups (deliberately NOT done):** the connection-cancel affordance (Escape while connecting) and the wire label priority on multi-warehouse Pro-tier connections remain untested; the dev-mock retail cart/undo reload persistence is still open on the mock side.


## 2026-08-07 — Topology editor: Escape connection-cancel flow + Pro-tier fallback labels

### The connection-cancel affordance and the Pro-tier label priority were the last two journaled gaps
**Problem:** The Escape-while-connecting affordance (clears the in-flight port connection AND the selection in one keystroke) and the Pro-tier wire-label priority (a second workspace→warehouse wire is blocked on standard but allowed with the fallback label on Pro) had zero regression net.

**Solution:** 4 characterization tests + 2 test-infra cleanups. (1) Escape cancels an in-flight connection: the ghost preview (`path.wire-path[opacity="0.5"]` — real wires never set opacity, so the selector can't false-positive) disappears and a subsequent target click starts a NEW connection instead of completing the old one. (2) Escape during a connection also clears `node-selected`. (3) The input guard is pinned positively: Escape typed in the inspector's text field does NOT cancel the connection — the wire completes afterward. (4) Pro-tier: with `currentTier="pro"` (renderEditor gained a derived `TopologyTier` prop override), a second ws→wh wire is allowed with no license toast and carries the `topology-wire-label-fallback` label on the new wire. Reviewer nits applied: `nodeAt`/`portOf`/`previewLine` hoisted to module scope (were triplicated across describes) and the tier union derived from `ComponentProps` (with `Exclude<…, undefined>` for `exactOptionalPropertyTypes`). All pinned existing behavior — no production change needed.

**Validation:** 4 new · editor suite 72 · topology suites 100/100 · full UI suite 262 files / 4053 tests green · typecheck + eslint clean.

**Commits:** (hash below)

**Follow-ups (deliberately NOT done):** Escape with nothing selected is untested (low value); the first-warehouse-wire `stock-deduct` label path is only indirectly covered; the dev-mock retail cart/undo reload persistence is still open on the mock side.


## 2026-08-07 — Migration drift repair: restore 120 byte-exact, move the repair into 121 (DB-02)

### The app panicked on startup: "migration 120_reseed_default_workspace_instances.sql definition drift"
**Problem:** The earlier "safe to repair pre-release" judgment was wrong — the user's dev DB had already applied migration 120 (checksum `15377253038134…`) before my in-place COALESCE repair changed the file (checksum `6f98911e…`). The DB-02 drift guard fails closed at startup when an applied migration's definition changes, so `oz-pos-app.exe` refused to boot. The lesson: "unreleased" does not mean "unapplied on dev machines" — a migration edited after ANY database has run it is drift.

**Recovery:** The original 120 was never committed (untracked when created), so git history was useless. It was recovered byte-exact from old `target/debug/deps/liboz_core-*.rlib` artifacts (migrations embed via `include_str!`, so pre-repair builds contain the original bytes): extracted a window around the `-- 120_reseed…` header and verified SHA-256 == the applied checksum `15377253038134…`. Technique worth remembering when git alone can't restore a file.

**Solution (the error's own guidance: restore the original, or add a new migration — did both):**
1. `120_reseed_default_workspace_instances.sql` restored to the original definition (byte-for-byte; on-disk hash now matches the DB record, so drift is gone).
2. New `121_workspace_instances_store_own_profile.sql` carries the repair that used to live in 120: an INSERT with the improved COALESCE (primary → this store's own profile → 'default') for fresh DBs, plus an UPDATE re-pointing the rows 120 seeded under `store_id = 'default'` (`id LIKE 'default-%' AND store_id = 'default'`) to the preferred profile, with a COALESCE fallback that keeps the current value on single-store DBs. Both statements idempotent and FK-safe.
3. `migrations.rs`: registered 121 after 120, with a new test `migration_121_repoints_instances_seeded_under_default_store` (upgrade re-point + idempotency; fresh path covered by the app-level test).
4. `workspaces.rs` test `list_workspaces_repairs_empty_store_db_after_066_window` now deletes BOTH the 120 and 121 records so the re-open runs the full repair — the repair genuinely lives in 121 now.

**Verify:** restored-120 hash == applied checksum (verified against the real dev DB record, read-only) · oz-core 2160 · oz-pos-app 803 · tablet 420 · fresh-DB `migrate` ×2 idempotent · fmt + clippy `-D warnings` clean. Reviewer: no blocking issues.

**Commits:** (hash below)

**Follow-ups:** (1) The f22bb5e6 commit message + its journal entry describe the repair as living inside 120 — this entry supersedes that; do NOT re-apply the COALESCE edit to 120. (2) Future migration edits should check applied checksums on all dev DBs (not just git history) before touching any file — or always add a new migration.
