@echo off
REM ============================================================================
REM LocardX Linux Build Script via Docker (Run from Windows)
REM Produces native Linux .deb and .AppImage packages without needing a Linux host.
REM ============================================================================

setlocal enabledelayedexpansion

echo =======================================================
echo [LocardX] Building Linux Desktop Packages (.deb / .AppImage) via Docker
echo =======================================================

where docker >nul 2>nul
if %ERRORLEVEL% neq 0 (
    echo [ERROR] Docker is not installed or not in PATH.
    echo Please install Docker Desktop for Windows or run the build inside WSL2 / Linux.
    exit /b 1
)

if not exist "dist\linux" mkdir "dist\linux"

echo [1/2] Building Linux container and compiling LocardX...
docker build -f Dockerfile.build-linux --target artifacts --output type=local,dest=dist/linux .

if %ERRORLEVEL% equ 0 (
    echo =======================================================
    echo [SUCCESS] Linux bundles generated successfully!
    echo Output directory: dist\linux\
    dir "dist\linux"
    echo =======================================================
) else (
    echo [ERROR] Docker build failed. Please check the logs above.
    exit /b %ERRORLEVEL%
)

endlocal
