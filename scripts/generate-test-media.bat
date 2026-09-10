@echo off
REM ============================================================================
REM LocardX Synthetic Test Media Generator (Windows)
REM Creates a safe, virtual disk file for sanitization and carving tests
REM ============================================================================

set TARGET_DIR=test-data\filesystem-images
if not exist "%TARGET_DIR%" mkdir "%TARGET_DIR%"

echo [LocardX] Generating synthetic test disk artifact...
echo [LocardX] Test media target: %TARGET_DIR%\synthetic_test_disk.bin

REM Generate 1MB binary file filled with zeroes using fsutil if available
where fsutil >nul 2>nul
if %ERRORLEVEL% equ 0 (
    fsutil file createnew "%TARGET_DIR%\synthetic_test_disk.bin" 1048576
    echo [OK] Created 1MB synthetic test media.
) else (
    echo [INFO] fsutil not available or requires administrator privileges.
    echo Please create test images manually or run inside an elevated session.
)
