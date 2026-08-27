@echo off
title yt dw (best)

:: saving path (for the downloaded vids)
:: Example: "SAVE_DIR=C:\Users\YOURUSERNAME\Videos\Downloads"
::set "SAVE_DIR=C:\Users\<user>\Desktop\dw"
set "SAVE_DIR=%USERPROFILE%\Desktop\dw"

cd /d "%SAVE_DIR%"

echo Simple Edition (Best)
echo Vid dw @ %SAVE_DIR%
echo Enter a link:
set /p URL=

echo.
echo Starting Download...
echo.

yt-dlp -f "bestvideo+bestaudio/best" --merge-output-format mp4 --recode-video mp4 -o "%%(title)s - %%(uploader)s [%%(id)s].%%(ext)s" "%URL%"

echo.
echo Done.
echo You can close this tab
pause