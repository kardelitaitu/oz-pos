# Changelog — OZ-POS 0.0.36

**Release date:** 2026-09-04
**Commits since 0.0.35:** 48 (`d318e4ea..HEAD`)
**Scale:** 112 files changed, +4,039 / −500

---

## Highlights

A small release dominated by two things: finishing the **website performance
arc** that PR #95 carried, and a sustained pass on **making the repo's own
instructions and gates tell the truth**. Much of the second category is
correction of claims that had silently drifted — gate counts, version locks,
CI coverage, and one quality gate that had been red on `main` for weeks because
nothing in CI ran it.

The later half of the release is a chain that started from one timezone failure
on PR #95 and kept turning out wider than the previous link suggested: five
routed screens anchoring report windows to the wrong calendar, five test files
whose mocks silently swallowed unmocked commands, and seven components reading a
context the test harness never stubs — where the tests had gone on to assert the
resulting empty session token as specification.

| Area | Commits | Character |
|---|---|---|
| Website performance & correctness | 8 | Lighthouse follow-ups: caching, CSS deferral, island waterfall |
| Documentation accuracy | 10 | instruction files, plans, backlog, release notes, guide |
| Local gate + CI enforcement | 9 | path routing, rewritten `pre-push`, `commit-msg` gate, exec bits, asset guard |
| Test integrity (mocks, harness, tz) | 4 | unmocked IPC commands now fail loudly; empty-token assertions corrected |
| Store-zone date anchoring | 3 | report windows anchored to the store's calendar, not the device's |
| Rust / dependency hygiene | 4 | clippy, refactor, lockfile entry |
| API write-audit + served-store selector | 3 | new audit hook, store picker end-to-end |
| Non-conforming messages | 4 | see [Known issues](#known-issues) |
| Asset hygiene · Release · Admin | 1 each | 1.0 MB dead SVG removed; version bump; stats typedef |

*Counts derived by partitioning `d318e4ea..HEAD` so every commit lands in exactly
one bucket and the total equals the range count — not by hand-tallying. This
table previously summed to 37 against a range that actually held 38; the script
now asserts the sum equals `git rev-list` and reports any commit matching no rule
instead of dropping it silently.*

---

## Website Performance & Correctness

Continuation of the Lighthouse work; each item is a distinct fix, several of
which unblocked the others.

- **Cache headers and deferred render-blocking CSS** (`3b505842`) — static
  assets now carry explicit cache-lifetime headers (`196f801b` added the
  `/videos/*`, `/_astro/*`, `/admin/*`, `og-image`, `favicon` rules) and
  render-blocking stylesheets are deferred.
- **Critical request chain depth reduced** (`5e13054c`) — the depth of the
  request chain the browser must resolve before first paint was flattened.
- **Island JS waterfall flattened via `modulepreload` injection** (`4afa18c0`) —
  a build step now injects `<link rel="modulepreload">` into the emitted HTML
  (69 files on the current build) so hydrated islands stop serializing their
  module fetches.
- **Mobile unstyled-content flash killed** (`7edba61e`) — stylesheets are inlined
  to remove the flash of unstyled content on mobile.
- **`serveStatic` returned 500 on static assets** (`5a7708e9`) — fixed.
- **CSP `font-src` missing `data:`** (`548a4eca`) — Fontsource emits data-URI
  fonts; the policy rejected them until `data:` was added.
- **Emoji icons removed from the mobile hamburger** (`2bc737c0`).

## Local Gate and CI Enforcement

The most consequential change in this release is structural: **three quality
gates had no CI backstop and nobody had noticed.**

- **`dev-ci.yml` now routes jobs by changed paths** (`c5ec6381`) — a `changes`
  job computes the PR's file list once and emits `rust`/`ui`/`i18n`/`website`;
  every other job gates on the matching output. Measured motivation: PR #95's
  three pushes cost 75.2 runner-minutes, and the last added a markdown file and
  spent **22.0 minutes** re-running Rust jobs that cannot fail on it. Rust is
  ~77% of every run. Hand-rolled rather than `dorny/paths-filter` because every
  action in this repo is pinned to a full SHA and a new third-party dependency
  is an explicit supply-chain decision. Routing logic is tested by extracting
  the real shell body from the YAML and running it against 11 synthetic diffs.
- **`.githooks/pre-push` rewritten as a tiered, path-aware gate** (`bab9a420`) —
  it previously ran two checks that `pre-commit` already runs, adding ~4s and no
  coverage. Now Tier 0 is 13 static gates (~27s, always) and Tier 1 is
  path-gated Rust/UI/website/i18n. Measured: docs-only push **27s**, full matrix
  197s, and it correctly **blocks** on an injected violation. Rust uses plain
  `cargo check --workspace`, deliberately weaker than CI's
  `--all-targets --all-features`, which OOMs `rustc` on Windows/MSVC here.
- **All three hooks were non-executable** (`5caa4a14`) — committed as mode
  `100644`, so on Linux/macOS git ≥2.36 skips them entirely. `pre-commit` has
  been **Windows-only this whole time**; invisible on this machine because
  `core.fileMode=false` makes git ignore mode bits.
- **`verify-scoped-coverage.sh` was red on `main`** (`619a3392`) — four topology
  template commands from `d8209477` had no `_scoped` variant and no allowlist
  entry. Allowlisted on evidence, not preference: `_scoped` means per-store
  scoping, and topology is a global branch-keyed tool (`state.db` locked 19×,
  `resolve_store` 0×), as are its four already-allowlisted siblings. The gate
  was verified to still *fail* when the entries are removed.
- **`check.sh` findings cleared** (`aa38a2af`) from the first full local gate run.
- **Empty-stdin behaviour documented** (`527e3d7a`) — garbage OIDs fail *closed*
  into the full matrix; only a genuine no-ref push exits early.
- **Conventional commits are now actually enforced** (`9cf14190`) — `AGENTS.md`
  had always said the format was enforced and listed the allowed types, but
  nothing checked it; enforcement was prose. `.githooks/commit-msg` validates the
  subject and prints the list on failure. Prerequisite resolved first: `style`
  was in use across 8 commits but missing from the documented list, so the list
  was corrected rather than the gate rejecting existing practice. Git-generated
  subjects (`Merge`/`Revert`/`fixup!`/`squash!`) pass through. Tested on 30 cases
  and then confirmed to block a real commit.
- **Exec bit re-learned the hard way** (`6a367f2a`) — the new hook was committed
  as `100644` despite `git update-index --chmod=+x` beforehand, because
  `git commit -- <pathspec>` re-reads the worktree where `core.fileMode=false`
  makes the mode look unchanged. Same defect fixed for the other three hooks in
  `5caa4a14`, reintroduced within one session.
- **Website asset guard** (`5884f35c`) — `scripts/verify-website-assets.py` fails
  on base64 inside an `.svg`, on assets over a 100 KB budget, and on orphaned
  assets. The orphan check **disables itself** if any dynamic asset resolution
  (`import.meta.glob`, template-string paths) appears anywhere, because a zero
  filename grep only means "unused" while resolution is provably static.
  Registered in `gates.json`, run before `npm ci` in CI so a bad asset fails in
  ~1s instead of after a minute of dependency setup.

## API Write-Audit and Served-Store Selector

- **Write-audit hook on the shared router** (`637d5909`) — mutations through the
  local API are now recorded.
- **Store selector + API writes into the audit log** (`d2a5c6ce`) — desktop
  client surfaces which store an API write targeted.
- **Served-store selector in the Local API panel** (`2d18c224`) — the UI half,
  with the guide refreshed (`fa9780a8`).

## Documentation Accuracy

Six commits correcting statements that had drifted from the tree they describe.

- **The pre-commit gate inventory was wrong in all three instruction files**
  (`f93eed67`) — root `AGENTS.md` claimed 6 gates, `.agents/AGENTS.md` and
  `.prime/AGENTS.md` claimed 4 at "~1s"; the hook runs **8** at ~5–7s. The
  omissions mattered more than the counts: `.agents/` never mentioned the PG
  schema drift guard *or* the "never hand-edit `init.pg.sql`" rule, and
  `.prime/` had CI coverage exactly backwards — calling `lint-i18n`
  "informational" (it is a hard failure) and implying `bundle-parity` runs in CI
  (it runs nowhere but the local hook). Also corrected a false claim that a
  `pg-schema-drift` CI job exists: that job and `migration-column-types` were
  retired with `ci.yml` in `23c96330` and, unlike `i18n`, never restored.
- **Version lock synced in the three skills the bump script misses** (`24bd3db4`)
  — `bump-version.ps1` owns the three `AGENTS.md` files but not
  `.agents/skills/`, so two skills were instructing agents to hold 0.0.35 after
  the bump. Historical audit stamps recording *past* lock corrections were left
  intact.
- **Carousel plan corrected** (`ec37a029`) — said `client:load` (actual
  `client:idle`) and "5-slide mockup carousel" (actual: 1 mockup + 4
  placeholders). More importantly it never said where the video mounts;
  `SlideWindow`'s own doc comment already settles that the chrome stays live
  DOM, which means the mount box is ~1280×693, **not** the 1280×720 the plan
  encodes at — a mistake that would be baked into recorded footage and
  unfixable in CSS.
- **0.0.36 backlog opened** (`68ff2ec6`) — four verified findings carried out of
  PR #95's review rather than buried.

## Asset Hygiene

- **1,025,475 bytes of unreferenced SVG removed** (`6f40fc95`) — 7 files from
  `website/src/assets/`, including a 639 KB `logo-indonesia-map.svg` and a
  381 KB `footer-instagram.svg` that is not a vector at all but six base64
  rasters wearing an `<svg>` tag. Verified safe by proving every asset reference
  in `website/src` is a literal import — no `import.meta.glob`, no template
  paths — and by reading `Footer.astro`, which renders all social icons as
  inline `<svg>`.

## Test and Type Correctness

- **Analytics empty-states test made timezone-independent** (`cca9e5d0`) — it was
  not flaky. Three mocks used `mockResolvedValueOnce`, so a fourth unexpected
  call fell through to a default and the assertion depended on host timezone.
  This was misdiagnosed as flakiness in PR #95's description before the real
  cause was found.
- **Schema-surface pins updated for `webhook_endpoints`** (`14226407`).
- **Admin stats typedefs completed** (`9589a200`) — `AdminKpis` and `AdminStats`
  declared 8 and 9 members while `admin_stats.go:633-680` sends 29 and 12.

## Store-Zone Date Anchoring and Test Integrity

A chain that began with one timezone failure on PR #95 and kept widening. Each
link was found by sweeping for the class after fixing the previous instance.

- **Analytics ranges anchored to the store, not the host** (`42100cde`) —
  `isoToday(null)` fell through to the device calendar. The fallback is now
  `FALLBACK_STORE_TZ = 'UTC'`, which is not an invented policy: the schema
  declares `timezone TEXT NOT NULL DEFAULT 'UTC'` in both migrations, so a
  persisted store always has a zone and `null` only means "profile still
  loading". A test parses the migration and asserts the constant still matches,
  so the justification cannot rot.
- **Reports dashboard anchored the same way** (`264669fa`) — the primary
  reports entry point for managers (route `dashboard`, nav item, and the
  workspace-home reports card) computed its default 30-day window from the
  device calendar while querying store-scoped data. One missing mock handler was
  also making **23 of its 26 tests pass while rendering the app shell's error
  branch**.
- **Three more routed screens fixed** (`2dad3d37`) — `SalesReportScreen`,
  `CustomReportScreen`, `MenuEngineeringScreen`. These were already
  host-independent (`toISOString()` is UTC), so the defect is narrower: a
  `+07:00` store at 06:00 local is `23:00 UTC` the previous day, so "today"
  showed yesterday for the first seven hours of every business day.
  `monthAgo()`'s mixed host-local/UTC arithmetic was measured at 0/8760
  divergences in non-DST zones and 60/8760 in DST zones — a real but
  Indonesia-unreachable defect, fixed as a side effect.
- **Unmocked Tauri commands now fail loudly** (`e8c2157e`) — five test files
  ended their `invoke` mock chain with a silent `Promise.reject`. Added
  `test-utils/invokeCoverage.ts` to record unmatched commands and assert the set
  is empty, so a green test can no longer mean "the component rendered its error
  state".
- **Seven components moved to the `useWorkspace()` hook** (`eefd51ff`) — the
  global harness stubs the hooks but keeps the real context object, so anything
  reading `useContext(WorkspaceContext)` directly got an empty token in every
  test and skipped its token-gated effects. Several tests had baked that empty
  token into their expectations, e.g.
  `toHaveBeenCalledWith(10, '')` — asserting the defect as spec.

**Method note.** Every guard added in this release was verified non-vacuous by
sabotage: reintroduce the original defect and require the gate to fail. That
caught two of my own bugs — an asset guard that flagged all eight legitimate
assets as orphans, and a timezone test hardcoded to `+14:00` that was vacuous for
ten hours of every day because +14 shares a calendar date with UTC below 10:00.
The replacement picks `+14:00` or `-10:00` by current UTC hour; those windows are
complements, so one always discriminates.

## Rust and Dependency Hygiene

- Clippy warnings resolved in `oz-api` / `oz-cloud-server` (`9672cc7a`),
  needless `format!` references dropped (`130c7556`), JWT validation cache types
  named (`673aa575`), `reqwest` recorded in the cloud server's lock entry
  (`95cec6d4`).

---

## Known issues

**Four commits on this branch violate the conventional-commit rule the repo
claims to enforce.** `deleted`, `modified` and `updated` are not permitted
types, and one commit's message is a pasted `git status` block:

| Commit | Subject |
|---|---|
| `2eea3d07` | `deleted:    lighthouse-report.json` |
| `faa5dae0` | `modified:   .gitignore` |
| `5855c429` | `updated gitignore` |
| `84a71f3e` | `	new file:   lighthouse-report.json 	new file:   website/src/assets/…` |

This is tracked as **R36-04** in [`docs/plans/0.0.36-backlog.md`](../plans/0.0.36-backlog.md):
`AGENTS.md` states the format is "enforced", but there is no `commit-msg` hook
and nothing checks it. These four are the proof.

**Every backlog item this release named is now closed.** The four that were
listed as open when these notes were first written:

- **R36-01** 🔴 → closed (`42100cde`) — analytics range anchored to the store
  zone, with a `check-tz-invariance.py` gate that runs the suite under four host
  zones and requires an identical result.
- **R36-02** 🔴 → closed (`e8c2157e`) — `_scoped` mock gaps closed in all five
  affected test files, plus the loud-failure guard so a missed handler can no
  longer pass silently.
- **R36-03** 🟡 → closed (`5884f35c`) — the guard is implemented, including the
  self-disabling orphan check.
- **R36-04** 🟡 → closed (`9cf14190`) — `commit-msg` gate exists, and the four
  subjects above are exactly the evidence that motivated it.

Added and closed during the release: **R36-05** (`264669fa`), **R36-06**
(`2dad3d37`), **R36-07** (`eefd51ff`).

**Not fixed, flagged:** `load_topology_template` and `list_topology_templates`
require a session but check **no permission**, so any authenticated user can
read any branch's templates. The source comment documents this as intentional;
changing security posture is not something to slip into a gate fix.
