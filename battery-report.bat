@echo off
title battery report

set "REPORT=%USERPROFILE%\battery-report.html"

echo battery report
echo.

powercfg /batteryreport /output "%REPORT%"

if not exist "%REPORT%" (
    echo.
    echo Creating the battery report failed.
    pause
    exit /b
)

:: opens report in def browser
echo.
echo Opening report...
start "" "%REPORT%"

timeout /t 3 >nul
exit
