@echo off
title rm temp

echo rm temp (clears the %%temp%% folder)
echo.

:: safety check: if TEMP is empty, "%TEMP%\*" would become "\*" (the root of the drive)
if "%TEMP%"=="" goto notemp
if not exist "%TEMP%\" goto notemp

echo Folder: %TEMP%

:: size before (in bytes)
for /f %%S in ('powershell -NoProfile -Command "[long](Get-ChildItem -LiteralPath $env:TEMP -Recurse -Force -File -ErrorAction SilentlyContinue | Measure-Object Length -Sum).Sum"') do set "BEFORE=%%S"

echo Deleting...

:: files (locked files from running programs get skipped)
del /f /s /q "%TEMP%\*" >nul 2>&1
:: subfolders (the temp folder itself stays)
for /d %%D in ("%TEMP%\*") do rmdir /s /q "%%D" >nul 2>&1

:: size after (in bytes)
for /f %%S in ('powershell -NoProfile -Command "[long](Get-ChildItem -LiteralPath $env:TEMP -Recurse -Force -File -ErrorAction SilentlyContinue | Measure-Object Length -Sum).Sum"') do set "AFTER=%%S"

echo.
echo Result
powershell -NoProfile -Command "function f($b){ if($b -ge 1GB){'{0:N2} GB' -f ($b/1GB)} else {'{0:N1} MB' -f ($b/1MB)} }; Write-Host ('  Before: ' + (f %BEFORE%)); Write-Host ('  Freed:  ' + (f (%BEFORE% - %AFTER%))); Write-Host ('  Left:   ' + (f %AFTER%) + '  (locked by running programs)')"
echo

echo.
pause
exit /b

:notemp
echo TEMP folder not found, nothing was deleted.
pause
exit /b
