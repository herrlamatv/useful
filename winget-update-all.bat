@echo off
title winget update all

:: list of installed versions (before + after update)
set "BEFORE=%TEMP%\winget-before.json"
set "AFTER=%TEMP%\winget-after.json"

echo winget update all
echo.

where winget >nul 2>&1
if not %errorlevel%==0 (
    echo winget not found.
    pause
    exit /b
)

echo Saving current versions...
winget export -o "%BEFORE%" --include-versions --accept-source-agreements >nul 2>&1

echo.
echo Updating all packages...
echo.
winget upgrade --all --include-unknown --accept-source-agreements --accept-package-agreements

echo.
echo Saving new versions...
winget export -o "%AFTER%" --include-versions --accept-source-agreements >nul 2>&1

:: compare both lists and show every package with their changed version
echo.
echo Updated:
powershell -NoProfile -Command "$a=(Get-Content '%BEFORE%' -Raw | ConvertFrom-Json).Sources.Packages; $b=(Get-Content '%AFTER%' -Raw | ConvertFrom-Json).Sources.Packages; $old=@{}; $a | ForEach-Object { $old[$_.PackageIdentifier]=$_.Version }; $n=0; $b | ForEach-Object { if($old.ContainsKey($_.PackageIdentifier) -and $old[$_.PackageIdentifier] -ne $_.Version){ Write-Host ('  {0}: {1} -> {2}' -f $_.PackageIdentifier, $old[$_.PackageIdentifier], $_.Version); $n++ } }; Write-Host ''; if($n -eq 0){ Write-Host '  Nothing was updated.' } else { Write-Host ('  {0} package(s) updated.' -f $n) }"
echo

del "%BEFORE%" "%AFTER%" >nul 2>&1

echo.
pause
