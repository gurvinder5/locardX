@echo off
REM ============================================================================
REM LocardX Build Script (Windows)
REM ============================================================================

echo [LocardX] Building backend and frontend...

where cargo >nul 2>nul
if %ERRORLEVEL% equ 0 (
    echo [INFO] Running cargo build...
    cargo build --workspace
) else (
    echo [WARNING] Cargo not available. Skipping Rust build.
)

if exist "frontend\package.json" (
    echo [INFO] Building frontend...
    cd frontend
    call npm run build
    cd ..
)

echo [LocardX] Build complete.
