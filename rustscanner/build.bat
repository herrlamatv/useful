@echo off
title rustscanner build
cd /d "%~dp0"

echo Building rustscanner (release) ...
echo.
cargo build --release
if errorlevel 1 (
    echo.
    echo Build failed.
    pause
    exit /b
)

if not exist "output" mkdir "output"
copy /y "target\release\rustscanner.exe" "output\rustscanner.exe" >nul

echo.
echo Done: output\rustscanner.exe
pause
