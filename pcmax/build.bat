@echo off
title pcmax build
cd /d "%~dp0"

echo Building pcmax (release) ...
echo.
cargo build --release
if errorlevel 1 (
    echo.
    echo Build failed.
    pause
    exit /b
)

if not exist "output" mkdir "output"
copy /y "target\release\pcmax.exe" "output\pcmax.exe" >nul
copy /y "target\release\pcm.exe" "output\pcm.exe" >nul

echo.
echo Done: output\pcmax.exe (window + tray) and output\pcm.exe (command line)
pause
