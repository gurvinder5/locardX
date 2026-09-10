@echo off
REM ============================================================================
REM LocardX Linting & Formatting Script (Windows)
REM ============================================================================

echo [LocardX] Running lint and format checks...

where cargo >nul 2>nul
if %ERRORLEVEL% equ 0 (
    echo [INFO] Checking Rust formatting...
    cargo fmt --all -- --check
    echo [INFO] Running Clippy...
    cargo clippy --workspace --all-targets -- -D warnings
) else (
    echo [WARNING] Cargo not available. Skipping Rust checks.
)

if exist "frontend\package.json" (
    echo [INFO] Checking frontend linting...
    cd frontend
    call npm run lint
    cd ..
)

echo [LocardX] Lint checks finished.
