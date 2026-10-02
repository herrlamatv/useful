@echo off
title yt-dlp + ffmpeg downloader

:: install folders (no admin req)
set "YTDLP_DIR=%LOCALAPPDATA%\yt-dlp"
set "FFMPEG_DIR=%LOCALAPPDATA%\ffmpeg"

echo yt-dlp + ffmpeg downloader
echo.

::yt-dlp
echo [yt-dlp]
where yt-dlp >nul 2>&1
if %errorlevel%==0 (
    echo yt-dlp is already installed.
    goto ffmpeg
)

if not exist "%YTDLP_DIR%" mkdir "%YTDLP_DIR%"
echo Downloading yt-dlp.exe to %YTDLP_DIR% ...
curl -L --fail -o "%YTDLP_DIR%\yt-dlp.exe" "https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp.exe"
if not exist "%YTDLP_DIR%\yt-dlp.exe" (
    echo Download of yt-dlp failed.
    goto ffmpeg
)
call :addpath "%YTDLP_DIR%"

::ffmpeg
:ffmpeg
echo.
echo [ffmpeg]
where ffmpeg >nul 2>&1
if %errorlevel%==0 (
    echo ffmpeg is already installed.
    goto done
)

:: winget ffmpeg (winget adds ffmpeg to path/sysvariables byitself)
where winget >nul 2>&1
if %errorlevel%==0 (
    echo Installing ffmpeg via winget...
    winget install -e --id Gyan.FFmpeg --accept-source-agreements --accept-package-agreements
    if not errorlevel 1 goto done
    echo winget install failed, trying manual download...
) else (
    echo winget not found, trying manual download...
)

:: fallback download zip from gyan.dev and add bin folder to path
set "FFZIP=%TEMP%\ffmpeg-release-essentials.zip"
set "FFTMP=%TEMP%\ffmpeg-extract"
curl -L --fail -o "%FFZIP%" "https://www.gyan.dev/ffmpeg/builds/ffmpeg-release-essentials.zip"
if not exist "%FFZIP%" (
    echo Download of ffmpeg failed.
    goto done
)
if exist "%FFTMP%" rmdir /s /q "%FFTMP%"
mkdir "%FFTMP%"
tar -xf "%FFZIP%" -C "%FFTMP%"

:: zip from https://www.gyan.dev/ffmpeg/builds/ffmpeg-release-essentials.zip contains the version
:: it looks like this: ffmpeg-9.0.2-essentials_build.zip
if exist "%FFMPEG_DIR%" rmdir /s /q "%FFMPEG_DIR%"
for /d %%D in ("%FFTMP%\ffmpeg-*") do move "%%D" "%FFMPEG_DIR%" >nul
rmdir /s /q "%FFTMP%"
del "%FFZIP%"

if not exist "%FFMPEG_DIR%\bin\ffmpeg.exe" (
    echo Extracting ffmpeg failed.
    goto done
)
call :addpath "%FFMPEG_DIR%\bin"

:done
echo.
echo Done.
echo Note: open a NEW terminal so the PATH changes are active.
pause
exit /b

:: adds a folder to the user path (only if it isnt alr inthere)
:: doesnt use setx cuz it cuts path at 1024 chars
:addpath
powershell -NoProfile -Command "$d='%~1'; $p=[Environment]::GetEnvironmentVariable('Path','User'); if(-not $p){$p=''}; if(($p -split ';') -contains $d){Write-Host 'Already in PATH:' $d} else {[Environment]::SetEnvironmentVariable('Path',(($p.TrimEnd(';')+';'+$d).TrimStart(';')),'User'); Write-Host 'Added to PATH:' $d}"
exit /b
