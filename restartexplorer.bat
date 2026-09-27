@echo off
rem Restarts Windows Explorer (taskbar, desktop, tray).
rem Note: opened File Explorer folder windows will be closed.

echo Stopping explorer.exe ...
taskkill /f /im explorer.exe >nul 2>&1

timeout /t 3 /nobreak >nul

tasklist /fi "imagename eq explorer.exe" | find /i "explorer.exe" >nul
if errorlevel 1 (
    echo Starting explorer.exe ...
    start "" explorer.exe
)

echo Done - taskbar (and more) reloaded.
timeout /t 2 /nobreak >nul
