@echo off
REM ============================================================================
REM  start-desktop.bat — launches the OZ-POS desktop client in dev mode.
REM
REM  Run from project root (or any directory). It will cd to the Rust crate,
REM  then `cargo tauri dev` which builds the Rust binary in debug profile,
REM  starts Vite on the devUrl defined in apps/desktop-client/tauri.conf.json,
REM  and bridges them via the Tauri Webview. UI edits under ui/src/*.tsx hot-
REM  reload via HMR; Rust edits under apps/desktop-client/src/ trigger a
REM  rebuild.
REM
REM  DO NOT REPLACE THE COMMAND WITHOUT READING THIS:
REM    - `cargo tauri dev` (current) = debug Rust + dev Vite + HMR. The
REM      process stays alive until the terminal window closes. Use this for
REM      iterating on code.
REM    - `cargo tauri build`         = release .exe + bundled installer with
REM      NSIS/MSI, no HMR, terminals close immediately on completion. Use
REM      `cargo tauri build --debug` if you want a debug .exe without
REM      leaving dev mode (still no HMR though).
REM    - DO NOT remove `pause`. The console host closes the window on its
REM      own when the script exits, hiding any startup error from you.
REM    - DO NOT change `cd /d "%~dp0apps\desktop-client"`. cargo locates
REM      Cargo.toml via CWD; without this, running the bat from project
REM      root fails with "could not find Cargo.toml".
REM    - `setlocal` / `endlocal` keep env-var changes scoped to this run.
REM      Do NOT add global `set` lines without bounding them between them.
REM    - `%~dp0` is the directory containing this bat; the `apps\desktop-
REM      client` suffix is resolved relative to that, so the bat works no
REM      matter which directory it is invoked from. Do NOT replace with an
REM      absolute path that ties it to one developer's machine layout.
REM
REM  PREREQS (you install these once, this bat does none of it):
REM    - Node.js + npm            (node / npm on PATH)
REM    - Rust toolchain           (rustup + stable)
REM    - Tauri CLI                (cargo install tauri-cli)
REM    - UI dependencies          (cd ui && npm install, once)
REM
REM  STALE-PORT WORKAROUND (Windows only — run once before retrying if a
REM  previous dev run crashed and port 1420 is still bound to a dead Vite):
REM      powershell -Command "Get-NetTCPConnection -LocalPort 1420 |
REM        ForEach-Object { Stop-Process -Id $_.OwningProcess -Force }"
REM ============================================================================
setlocal

REM cd /d into the desktop-client crate so cargo finds Cargo.toml.
REM `%~dp0` is this bat's own directory; `apps\desktop-client` is relative
REM to that, which keeps the bat independent of its invocation CWD.
cd /d "%~dp0apps\desktop-client"

echo Killing any stale process listening on port 1420...
powershell -Command "Get-NetTCPConnection -LocalPort 1420 -ErrorAction SilentlyContinue | ForEach-Object { Stop-Process -Id $_.OwningProcess -Force }"

cargo tauri dev

REM Keep the window open so any startup error from the line above
REM stays readable instead of scrolling off into a closed console.
pause

endlocal
