@echo off
REM ============================================================================
REM LocardX - Safe Local Launcher (Windows)
REM Launches LocardX in Safe Local Preview Mode
REM ============================================================================
title LocardX - Safe Local Environment

echo ============================================================================
echo   LocardX - Forensic Recovery and Secure Sanitization Platform
echo   Running in SAFE DEVELOPMENT PREVIEW MODE
echo ============================================================================
echo [SAFETY] Host drives (C:, OS boot partitions) are protected.
echo [SAFETY] Device operations default to isolated mock/virtual datasets.
echo ============================================================================
echo.

REM Verify Node.js
where node >nul 2>nul
if %ERRORLEVEL% neq 0 (
    echo [ERROR] Node.js was not found in your PATH.
    echo Please install Node.js 18+ from https://nodejs.org/
    pause
    exit /b 1
)

REM Verify .env exists
if not exist ".env" (
    if exist ".env.example" (
        echo [INFO] Creating .env from .env.example...
        copy .env.example .env >nul
    )
)

REM Check if frontend node_modules is installed
if not exist "frontend\node_modules" (
    echo [INFO] Installing frontend dependencies (one-time setup)...
    cd frontend
    call npm install
    cd ..
)

echo [INFO] Starting LocardX local server on http://localhost:1420 ...
echo [INFO] Press Ctrl+C in this terminal window to stop the server.
echo.

REM Launch browser after a short delay
start "" http://localhost:1420

REM Start the dev server
cd frontend
call npm run dev
cd ..
