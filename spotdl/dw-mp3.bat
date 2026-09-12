@echo off
title spotdl dw (mp3 320k)

:: saving path (for the downloaded songs)
:: Example: "SAVE_DIR=C:\Users\YOURUSERNAME\Music\Downloads"
::set "SAVE_DIR=C:\Users\<user>\Desktop\spotdl"
set "SAVE_DIR=%USERPROFILE%\Desktop\spotdl"

if not exist "%SAVE_DIR%" mkdir "%SAVE_DIR%"
cd /d "%SAVE_DIR%"

echo Simple Edition (mp3 320k)
echo Music dw @ %SAVE_DIR%
echo Enter a link (song/album/playlist/artist):
set /p URL=

echo.
echo Starting Download...
echo.

spotdl download "%URL%" --format mp3 --bitrate 320k --threads 4 --output "{artists} - {title}.{output-ext}"

echo.
echo Done.
echo You can close this tab
pause
