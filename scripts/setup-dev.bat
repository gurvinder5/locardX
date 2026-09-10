@echo off
REM ============================================================================
REM LocardX Development Environment Setup (Windows)
REM ============================================================================

echo [LocardX] Checking development prerequisites...

REM Check Rust / Cargo
where cargo >nul 2>nul
if %ERRORLEVEL% neq 0 (
    echo [WARNING] Cargo/Rust was not found in your PATH.
    echo Please install Rust via https://rustup.rs/
) else (
    echo [OK] Cargo detected.
    cargo --version
)

REM Check Node.js
where node >nul 2>nul
if %ERRORLEVEL% neq 0 (
    echo [WARNING] Node.js was not found in your PATH.
    echo Please install Node.js 18+ from https://nodejs.org/
) else (
    echo [OK] Node.js detected.
    node --version
)

REM Check npm
where npm >nul 2>nul
if %ERRORLEVEL% neq 0 (
    echo [WARNING] npm was not found in your PATH.
) else (
    echo [OK] npm detected.
    npm --version
)

REM Copy .env.example to .env if not exists
if not exist ".env" (
    if exist ".env.example" (
        echo [INFO] Copying .env.example to .env...
        copy .env.example .env
    )
)

echo [LocardX] Setup check complete.
