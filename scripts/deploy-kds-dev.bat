@echo off
setlocal
title KDS prototype - sync to dev/
cd /d "%~dp0.."

echo Syncing KDS prototype files to dev/ (the hosted copy)...
echo.

mkdir dev\kds-pwa 2>nul

copy /y kds-prototype.html dev\ >nul
copy /y manifest.json      dev\ >nul
copy /y sw.js              dev\ >nul
copy /y kds-pwa\icon-192.png dev\kds-pwa\ >nul
copy /y kds-pwa\icon-512.png dev\kds-pwa\ >nul

echo.
echo Done. dev/ is ready to deploy:
echo   - Cloudflare Pages Git build: root directory = repo root (dev/ is served at /dev/)
echo   - Direct Upload: upload the contents of the dev/ folder
echo.
pause
