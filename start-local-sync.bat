@echo off
REM ============================================================================
REM  start-local-sync.bat — launches the OZ-POS local sync server in Docker.
REM
REM  Run from project root (or double-click from Windows Explorer).
REM  Checks if Docker Desktop is installed and running; if stopped, attempts
REM  to auto-start Docker Desktop. Once ready, runs `docker compose up -d`
REM  to build and start the headless `pos-cloud-server` container on port 3099.
REM
REM  Usage (SQLite — default, no external DB container needed):
REM    start-local-sync.bat
REM
REM  Usage (PostgreSQL profile — launches pg container + server):
REM    start-local-sync.bat --pg
REM
REM  To stop the server at any time, double-click `stop-local-sync.bat` or run:
REM    docker compose down
REM ============================================================================
setlocal

cd /d "%~dp0"

echo [1/3] Checking Docker CLI availability...
where docker >nul 2>&1
if not errorlevel 1 goto check_daemon

echo.
echo ERROR: Docker command not found on PATH!
echo Please install Docker Desktop from https://www.docker.com/products/docker-desktop/
echo.
pause
exit /b 1

:check_daemon
echo [2/3] Checking Docker engine/daemon status...
docker info >nul 2>&1
if not errorlevel 1 goto docker_ready

echo Docker daemon is currently stopped.
if not exist "C:\Program Files\Docker\Docker\Docker Desktop.exe" goto docker_missing

echo Attempting to auto-start Docker Desktop...
start "" "C:\Program Files\Docker\Docker\Docker Desktop.exe"
echo Waiting for Docker Desktop engine to initialize (this may take 15-45 seconds)...

set /a attempts=0
:docker_wait_loop
ping 127.0.0.1 -n 6 >nul
docker info >nul 2>&1
if not errorlevel 1 goto docker_ready
set /a attempts+=1
if %attempts% gtr 15 goto docker_timeout
echo   Still waiting for Docker daemon... (%attempts%/15)
goto docker_wait_loop

:docker_timeout
echo.
echo ERROR: Timed out waiting for Docker Desktop to start.
echo Please check Docker Desktop manually from the Windows taskbar.
pause
exit /b 1

:docker_missing
echo.
echo ERROR: Docker daemon is not running and Docker Desktop executable could not be found.
echo Please open Docker Desktop manually and run this script again.
pause
exit /b 1

:docker_ready
echo Docker engine is running!

echo.
echo [3/3] Launching OZ-POS Cloud Sync Server via Docker Compose...
if "%~1"=="--pg" (
    echo Mode: PostgreSQL Database backend --profile pg
    docker compose --profile pg up -d --build
) else (
    echo Mode: Local SQLite Database backend default
    docker compose up -d --build
)

if errorlevel 1 goto compose_failed
goto compose_success

:compose_failed
echo.
echo ERROR: Docker Compose failed to build or launch the container.
echo Please check the error messages above.
pause
exit /b 1

:compose_success
echo.
echo ============================================================================
echo  OZ-POS Local Sync Server is running and ready!
echo.
echo  - API Endpoint:    http://localhost:3099
echo  - Health Check:    http://localhost:3099/api/v1/health
echo  - Live Logs:       docker compose logs -f pos-cloud-server
echo  - Stop Server:     Run stop-local-sync.bat or docker compose down
echo.
echo  To test cloud sync in your POS app, go to Settings -^> Sync / Multi-store
echo  and set the Sync Server URL to: http://localhost:3099
echo ============================================================================
echo.

pause
endlocal
