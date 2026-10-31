@echo off
title encsha build
cd /d "%~dp0"

echo Building encsha (release) ...
echo.
cargo build --release
if errorlevel 1 (
    echo.
    echo Build failed.
    pause
    exit /b
)

if not exist "output" mkdir "output"
copy /y "target\release\encsha.exe" "output\encsha.exe" >nul
copy /y "target\release\esh.exe" "output\esh.exe" >nul

echo.
echo Done: output\encsha.exe (window) and output\esh.exe (command line)
pause
