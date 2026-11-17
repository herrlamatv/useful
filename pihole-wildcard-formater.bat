@echo off
setlocal EnableDelayedExpansion

set "IN=%~1"
if "%IN%"=="" set /p "IN=Pfad zur TXT-Datei: "
set "IN=%IN:"=%"
if not exist "%IN%" (
    echo Datei nicht gefunden: %IN%
    pause
    exit /b
)

for %%F in ("%IN%") do set "OUT=%%~dpnF-wildcard.txt"
if exist "%OUT%" del "%OUT%"

set "P=||"
set "C=^"

for /f "usebackq tokens=* delims=" %%L in ("%IN%") do (
    set "D=%%L"
    set "D=!D: =!"
    if "!D:~0,1!"=="#" (
        >>"%OUT%" echo(%%L
    ) else (
        if "!D:~0,2!"=="*." set "D=!D:~2!"
        if "!D:~0,2!"=="||" (
            >>"%OUT%" echo(!D!
        ) else (
            >>"%OUT%" echo(!P!!D!!C!
        )
    )
)

echo Fertig: %OUT%
pause