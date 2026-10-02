@echo off
title winget show outdated

echo winget show outdated
echo.

where winget >nul 2>&1
if not %errorlevel%==0 (
    echo winget not found.
    pause
    exit /b
)

echo Outdated:
winget upgrade --include-unknown --accept-source-agreements
echo

echo.
pause
