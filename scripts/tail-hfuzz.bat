@echo off
REM ============================================================================
REM  tail-hfuzz.bat - watch the running honggfuzz overnight campaign log.
REM
REM  Shows the live tail of /tmp/hfuzz-overnight.out (the campaign's console
REM  log inside WSL) via scripts/tail-campaign.sh. The window follows new
REM  lines as they are written; close the window (or Ctrl+C) to stop
REM  following. Safe to open any time - it only reads the log.
REM
REM  Usage: double-click, or run from anywhere (%~dp0-relative).
REM  Override: WSL_DISTRO=<name>  (only if you started the campaign with a
REM            non-default distro - must match)
REM ============================================================================
setlocal EnableDelayedExpansion

REM Optional distro override.
set "DISTRO_ARGS="
if defined WSL_DISTRO set "DISTRO_ARGS=-d %WSL_DISTRO%"

REM Repo root = one level above this bat's directory (scripts/).
set "ROOT=%~dp0.."

REM Convert the Windows repo path to a WSL path (/mnt/c/...).
for /f "delims=" %%P in ('wsl.exe %DISTRO_ARGS% wslpath "%ROOT%"') do set "WSL_ROOT=%%P"
if not defined WSL_ROOT (
    echo [ERROR] Could not resolve "%ROOT%" inside WSL.
    echo         Is WSL installed, and is the repo on a Windows drive?
    pause
    exit /b 1
)

wsl.exe %DISTRO_ARGS% bash -lc "cd '%WSL_ROOT%' && bash ./scripts/tail-campaign.sh"
pause
endlocal
