@echo off
REM ============================================================================
REM LocardX Test Runner Script (Windows)
REM ============================================================================

echo [LocardX] Executing automated tests...

where cargo >nul 2>nul
if %ERRORLEVEL% equ 0 (
    echo [INFO] Running Rust workspace tests...
    cargo test --workspace
) else (
    echo [WARNING] Cargo not available. Skipping Rust tests.
)

if exist "frontend\package.json" (
    echo [INFO] Running frontend tests...
    cd frontend
    call npm test --if-present
    cd ..
)

echo [LocardX] Test execution finished.
