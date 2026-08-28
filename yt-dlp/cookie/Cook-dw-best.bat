@echo off
title yt dw [Cook]

:: saving path (for the downloaded vids)
:: Example: "SAVE_DIR=C:\Users\YOURUSERNAME\Videos\Downloads"
::set "SAVE_DIR=C:\Users\<user>\Desktop\dw"
set "SAVE_DIR=%USERPROFILE%\Desktop\dw"

cd /d "%SAVE_DIR%"

echo [Cook] Edition (1080p)
echo Vid dw @ %SAVE_DIR%
echo Enter a link:
set /p URL=

echo.
echo Starting Download...
echo.

::working
yt-dlp -f "bestvideo+bestaudio/best" --merge-output-format mp4 --recode-video mp4 --cookies "cookies.txt" -o "%%(title)s - %%(uploader)s [%%(id)s].%%(ext)s" "%URL%"

echo.
echo Done.
echo You can close this tab
pause