@echo off
title yt-dlp update

echo Updating yt-dlp...
echo.

yt-dlp -U

echo.
echo Done. Closing in 3 seconds...
timeout /t 3 >nul
exit
