@echo off
setlocal

cd /d "%~dp0apps\desktop-client"
cargo tauri dev
pause
endlocal
