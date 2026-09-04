# Changelog — OZ-POS 0.0.36

**Release date:** 2026-09-04
**Commits since 0.0.35:** 66 (`d318e4ea..2d786e65`)
**Scale:** 123 files changed, +5,560 / −668

> The range is pinned to an explicit end commit rather than `HEAD`, because
> `d318e4ea..HEAD` is self-defeating in a changelog: the commit that corrects the
> count is itself inside the range it counts, so the number is wrong the moment
> it lands. That is how the previous revision came to say 37 for a range holding
> 38. Verify with `git rev-list --count d318e4ea..2d786e65`.

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

The final stretch is one continuous thread down that same hole. It began as
"four gates have no CI job" and ended with the repository's own manifest found
lying about **16 of 47 gates**, a drift checker that had been screaming 77 items
unread for a year, a documentation gate with a **fail-open that had been
structurally incapable of finding anything since September**, and a panic scanner
whose block-comment bug produced false positives *and* silently disabled itself
for an entire file. Net effect: `verify-ci-docs-drift` 77 → **0** and now
blocking in CI, `gates.json` able to express "this check does not run" at all,
seven checks that previously had no automated runner now running on every PR, and
`scripts/check.sh` — which had been red and is what `AGENTS.md` tells agents to
trust — completing again.

The recurring shape in every one of these: **a tool that reports the wrong thing
is worse than no tool, because it teaches its reader a specific, inverted
lesson.** Each fix was proven non-vacuous by sabotage before being trusted.

| Area | Commits | Character |
|---|---|---|
| Website performance & correctness | 8 | Lighthouse follow-ups: caching, CSS deferral, island waterfall |
| Documentation accuracy | 18 | instruction files, plans, backlog, release notes, guide |
| Local gate + CI enforcement | 17 | path routing, rewritten `pre-push`, `commit-msg` gate, exec bits, asset guard, **CI truthfulness made blocking** |
| Test integrity (mocks, harness, tz) | 4 | unmocked IPC commands now fail loudly; empty-token assertions corrected |
| Store-zone date anchoring | 3 | report windows anchored to the store's calendar, not the device's |
| Rust / dependency hygiene | 4 | clippy, refactor, lockfile entry |
| API write-audit + served-store selector | 3 | new audit hook, store picker end-to-end |
| Panic-gate scanner fix | 2 | a gate that produced false positives **and** silently skipped a whole file |
| Non-conforming messages | 4 | see [Known issues](#known-issues) |
| Asset hygiene · Release · Admin | 1 each | 1.0 MB dead SVG removed; version bump; stats typedef |

*Counts derived by partitioning `d318e4ea..2d786e65` so every commit lands in
exactly one bucket and the total equals the range count — not by hand-tallying.
The partition summed to 37 against a range that actually held 38 until that
revision; the script now asserts the sum equals `git rev-list` and reports any
commit matching no rule instead of dropping it silently.*

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

### Making CI truthfulness enforceable (R36-08 → R36-10)

The thread that started as "four gates have no CI job" turned out to be the
manifest itself lying about 16 of 47 gates. `verify-ci-docs-drift.py` had been
reporting **77** drift items continuously and nobody read them.

- **fmt, clippy, typecheck and lint put into CI** (`7fbac607`) — none had *any*
  automated runner, only an opt-in hook. Each verified green locally before being
  wired in, because adding a gate that already fails turns every unrelated PR red.
- **The drift checker had no word for "retired"** (`9112e128`, `65ad42f5`) — it
  globbed only `*.yml`, so all 11 workflows retired to `.bak` were reported as
  "MISSING WORKFLOW FILES", a category name that was false as stated. Taught it
  that `<x>.yml.bak` is retired, and that a matrix or gate row naming a retired
  workflow in its own column is recording history. **The exemption is provably
  ungameable**: a row claiming a *live* workflow that lacks the job is still an
  error, so nobody can silence a finding by pointing at a workflow that does not
  contain it. Drift 67 → 36 → 29.
- **The inventory was genuinely wrong too** — my "it's only a tooling gap"
  hypothesis was half right. `## Workflow inventory` described all 11 dead
  workflows in present tense and **omitted `dev-ci.yml` entirely**: the canonical
  CI dashboard listed 11 dead workflows and not one live one. Given a Status
  column, plus two new blocking checks (`UNLABELLED RETIRED`,
  `UNDOCUMENTED LIVE WORKFLOWS`) that each encode a mistake that actually happened
  here.
- **`gates.json` made truthful** (`47aa1290`) — 11 gates repointed to a live job
  that genuinely runs them, 16 retired, 2 kept `required` with the dead pointer
  removed. A new `retired` status was needed because the vocabulary could express
  "blocks", "reports" and "blocks on push" but not "does not run", which is why
  13 fictional gates could sit marked `required`. **A `retired` gate must carry no
  `ci` block**, so the status cannot be used as a mute button.
- **A correction to my own analysis, worth more than the rest:** a first pass
  classified 16 gates as running "nowhere" by searching `check.sh` and `pre-push`
  for label strings. Reading what `dev-ci.yml` actually executes showed
  `cargo nextest run --workspace --all-features` carries **no `--exclude`**, while
  `check.sh`'s own step excludes `oz-pos-app` — so three gates were enforced by CI
  *more broadly than the manifest admitted*. Retiring them would have swapped one
  inaccuracy for another and hidden real coverage. Substring matching on gate IDs
  also produced four false **positives** (`audit` matched "Audits production
  unwrap"; `coverage` matched an unrelated gate; `fuzz` matched a routing regex;
  `lighthouse` matched nothing). **Classify a gate by the command it runs, never
  by whether its name appears in a script.**
- **A fail-open the checker had been suffering since September** — the "jobs that
  exist but are undocumented" check computed `ci_jobs - documented`, and `ci_jobs`
  was only populated when a file literally named `ci.yml` existed. Since
  `23c96330` retired it, that has been the empty set: the check could never find
  anything, *and it was informational anyway*. Repointed at every live job it
  immediately found **four** undocumented ones (`website`, `cargo-nextest`,
  `northflank-deploy`, `static-gates`). Now blocking.
- **`static-gates` job added** (`13f2a1dc`) — six checks that existed only in the
  manual, unhooked `check.sh`, now including Go `fmt`/`vet`/`test -short`, which
  `AGENTS.md` had been calling local-only. Not path-gated: ~5s of Python/shell
  spanning four languages, so routing them risks the silent-skip failure the
  router test exists to catch.
- **The docs-drift gate is now blocking** (`237b16be`) — the condition R36-10 set
  for flipping it was "count reaches 0", and it did. `continue-on-error` removed;
  the step summary kept, because a bare exit code tells a reader nothing about
  *which* contract broke, and unreadable output is why this rotted for a year.
- **Process lesson recorded in R36-10 itself:** for two rounds I treated "77
  pre-existing drift items" as a *baseline* and reported each change as "adds no
  drift". A large, steady failure count is not a baseline, it is an unread alarm.
  Six of those 77 were the exact reports that made up the issue. **When a gate
  fails with a big number, check whether the number is the finding before using it
  as a control.**

**Net: drift 77 → 0, and the checker is more discriminating at each step rather
than more permissive.** Every new path was proven non-vacuous by sabotage — six
mutations, each caught, baseline restored byte-exact each time.

## Panic Gate: a Tool That Lied Both Ways (R36-12)

Found while deciding whether `panic-inventory` could join `static-gates`. It
could not: it *fails* today. That single observation turned out to mean
`scripts/check.sh` — the runner `AGENTS.md` tells every agent to use before
declaring a change verified — **cannot currently complete**.

- **The scanner did not track block comments across lines** (`48aa000f`) —
  `strip_comment` handled `/*` by breaking out of the line, correct only for a
  single-line block. For the multi-line audit stamp at the top of every crate
  root, only line 1 was recognised as a comment; lines 2..n-1 were scanned as
  code. **6 of 18 findings were stamp prose**, every one at line 4 of a file whose
  stamp says it is clean — a file failing the cleanliness gate because of the
  sentence documenting that it passes. 9 production files carry
  `unwrap()`/`expect()` in their stamp prose.
- **The same bug had a false-negative half, and it is the serious one.**
  Unstripped stamp prose also feeds the skip-context detector.
  `apps/cloud-server/src/redirect.rs` line 9 reads *"findings: 4 unsafe blocks in
  `#[cfg(test)]` only"*, so the scanner opened a `cfg(test)` skip and **stopped
  checking the rest of that file entirely** — a real production `.unwrap()` went
  unreported for its whole life. Proven with a synthetic pair differing only in
  that phrase. The lesson an agent would have learned from the broken tool is
  exactly inverted: *writing about your tests in a stamp disables the audit of
  your production code.*
- **The gate fought `cargo fmt`.** The invariant marker had to sit on the same or
  immediately-preceding *line*, so a multi-line comment block or a fmt-wrapped
  builder chain counted as undocumented — and the pre-commit hook runs
  `cargo fmt --all`, which rewraps the line and silently re-breaks a gate the code
  had satisfied. **A rule the formatter can violate on its own is not a rule.**
  The lookback now walks to the start of the enclosing statement and accepts a
  marker anywhere in the comment block above it; blank lines still break the
  block. Verified across six placements, each reporting exactly 1 finding so none
  could "pass" because the scanner failed to read the file. The next commit proved
  it for real: the hook ran `cargo fmt`, rewrapped the code, and the gate stayed
  green.
- **Triage: 2 fixed, 11 documented** (`8033a3e3`) — `migrate_sqlite_to_pg.rs`
  panicked on a condition its own doc comment calls *expected* (PG/SQLite column
  drift), aborting a migration without naming the column; both sites now return
  `Err(String)` like the rest of the file. Four header unwraps became
  `HeaderValue::from_static`, infallible **by signature** — the panic removed
  rather than justified. The `mock.rs` mutex `expect`s were documented rather than
  converted to `into_inner()`: that is a lock-semantics change to a shipped
  driver, and this task was to make the gate honest, not to refactor hardware code
  underneath it.

**130 production unwrap/expect calls, all documented; `check.sh` completes; and
the gate now runs in CI** (`2d786e65`) — closing R36-10's last loose end.


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
(`2dad3d37`), **R36-07** (`eefd51ff`), **R36-08** (`7fbac607`), **R36-09**
(`2a35f346`), **R36-10** (`9112e128` → `47aa1290` → `237b16be`), **R36-12**
(`48aa000f` → `8033a3e3` → `2d786e65`).

**One item is still open, and it blocks shipping.** **R36-11** 🔴 — every
`tags:`-triggered workflow (`release.yml`, `android.yml`, `ios.yml`) was retired
to `.bak` by `23c96330` and nothing replaced them, so **pushing a `v*` tag today
triggers no workflow and produces no installers, updater manifest, or
provenance**. `docs/releases/checklist.md` and `release-process.md` now say so at
the top rather than describing automation that does not run. This is a product
decision about release shape (desktop + two mobile targets + code signing +
updater manifests + provenance), not a mechanical restore, and it should not be
attempted as a drive-by.

**Not fixed, flagged:** `load_topology_template` and `list_topology_templates`
require a session but check **no permission**, so any authenticated user can
read any branch's templates. The source comment documents this as intentional;
changing security posture is not something to slip into a gate fix.

**Also not fixed, flagged by R36-12:** the panic gate's rule is that a
recoverable `unwrap()` needs a `// SAFETY:` / `// INVARIANT:` comment. Eleven such
comments were added this release and all eleven were checked by reading the
guard or the value's provenance — but the gate accepts the *marker*, not the
*argument*. A confidently-worded wrong invariant satisfies it. Reviewing those
comments for truth is worth doing independently of this release.

