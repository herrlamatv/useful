@echo off

setlocal
title MaleCNS-Konnektom-Downloader

if not exist "%~dp0download.ps1" (
    echo download.ps1 not found
    echo make sure install.bat and download.ps1 are in the same folder.
    pause
    exit /b 1
)

"%SystemRoot%\System32\WindowsPowerShell\v1.0\powershell.exe" -NoProfile -ExecutionPolicy Bypass -File "%~dp0download.ps1" %*
set "RC=%ERRORLEVEL%"

echo.
if "%RC%"=="0" (
    echo Done.
) else (
    echo Stopped with code: %RC%. Details are in the logfile download-log.txt in the target dir
    echo After a canellation or error, you can simply restart the install.bat and it will continue
)
echo.
pause
exit /b %RC%
