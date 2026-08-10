@echo off
REM ============================================================================
REM  run-hfuzz-overnight.bat — launches the honggfuzz overnight campaign.
REM
REM  Runs fuzz/hfuzz/run_overnight.sh (the "run while you sleep" fuzzing
REM  campaign) inside WSL, in a minimized console, so this .bat returns
REM  immediately and you can close it / go to sleep. Equivalent to:
REM
REM      cd fuzz/hfuzz && ./run_overnight.sh > /tmp/hfuzz-overnight.out 2>&1
REM
REM  Why a minimized console instead of `nohup ... &`? On WSL2, background
REM  processes are killed when the wsl.exe session that spawned them exits,
REM  so the campaign is run in the FOREGROUND of a minimized wsl.exe console
REM  (start /min). That keeps the WSL VM alive for the whole campaign and the
REM  .bat returns immediately. Keep the minimized window open; closing it
REM  stops the campaign. All output goes to /tmp/hfuzz-overnight.out.
REM
REM  Why WSL: honggfuzz does not build or run on native Windows — the whole
REM  fuzz/hfuzz/ crate is Linux/macOS/WSL-only (see fuzz/hfuzz/README.md).
REM
REM  PREREQS (once):
REM    - WSL2 with a distro that has Rust + `cargo honggfuzz` installed
REM      (Linux deps: build-essential binutils-dev libunwind-dev liblzma-dev)
REM    - This repo checked out on a Windows drive (the path is auto-converted
REM      to /mnt/<drive>/... via wslpath)
REM
REM  OVERRIDES (set in the environment before running):
REM    WSL_DISTRO        - WSL distro to use (default: the default distro)
REM    HFUZZ_RUN_TIME    - seconds per target (default 3600)
REM    HFUZZ_TARGETS     - space-separated targets (default: all)
REM    HFUZZ_THREADS     - fuzz threads (default: all cores)
REM    HFUZZ_REPORT_ROOT - campaign report root (default crash_reports/)
REM    HFUZZ_NO_NOTIFY=1 - skip desktop notifications (markers still written)
REM
REM  RESULTS:
REM    - campaign log:  /tmp/hfuzz-overnight.out (inside WSL)
REM    - crash reports: fuzz/hfuzz/crash_reports/<timestamp>/
REM    - next morning:  cd fuzz/hfuzz && ./triage_crashes.sh
REM
REM  DO NOT remove `pause`: the console closes on exit and hides errors.
REM ============================================================================
setlocal EnableDelayedExpansion

REM Repo root = one level above this bat's directory (scripts/).
set "ROOT=%~dp0.."

REM Optional distro override.
set "DISTRO_ARGS="
if defined WSL_DISTRO set "DISTRO_ARGS=-d %WSL_DISTRO%"

REM Convert the Windows repo path to a WSL path (/mnt/c/...).
for /f "delims=" %%P in ('wsl.exe %DISTRO_ARGS% wslpath "%ROOT%"') do set "WSL_ROOT=%%P"
if not defined WSL_ROOT (
    echo [ERROR] Could not resolve "%ROOT%" inside WSL.
    echo         Is WSL installed, and is the repo on a Windows drive?
    pause
    exit /b 1
)
echo [OK] WSL repo root: %WSL_ROOT%

REM Pass through HFUZZ_* overrides as shell exports.
set "EXPORTS="
for %%V in (HFUZZ_RUN_TIME HFUZZ_TARGETS HFUZZ_THREADS HFUZZ_WORKSPACE HFUZZ_REPORT_ROOT HFUZZ_NO_NOTIFY) do (
    if defined %%V (
        set "EXPORTS=!EXPORTS! export %%V='!%%V!';"
    )
)

REM Launch the campaign in a minimized WSL console; the console keeps WSL
REM alive for the whole run, and this bat returns immediately.
echo Launching overnight honggfuzz campaign...
echo   minimized console: keep it open until the campaign finishes
echo   campaign log (inside WSL): /tmp/hfuzz-overnight.out
start "OZ-POS hfuzz overnight" /min wsl.exe %DISTRO_ARGS% bash -lc "!EXPORTS! cd '!WSL_ROOT!/fuzz/hfuzz' && ./run_overnight.sh > /tmp/hfuzz-overnight.out 2>&1"

echo [OK] Launched. This window can be closed.
echo   watch progress from WSL:  tail -f /tmp/hfuzz-overnight.out
echo   next morning:             cd fuzz/hfuzz ^&^& ./triage_crashes.sh
pause
endlocal
