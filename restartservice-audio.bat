@echo off
title restart audio service
rem restarts the win audio service (audiosrv)
rem needs admin perms

net session >nul 2>&1
if errorlevel 1 (
    echo asking for admin perms..
    powershell -NoProfile -Command "Start-Process -FilePath '%~f0' -Verb RunAs"
    exit /b
)

echo Stop audio service ...
net stop audiosrv /y >nul 2>&1

timeout /t 2 /nobreak >nul

echo Start audio service ...
net start audiosrv >nul 2>&1

sc query audiosrv | find /i "RUNNING" >nul
if errorlevel 1 (
    echo audio service did NOT start, check services.msc
    pause
    exit /b
)

echo Done - audio service restarted
timeout /t 2 /nobreak >nul
