@echo off
setlocal
title install composer

set "PHPID=PHP.PHP.8.5"
set "COMPOSERDIR=%LOCALAPPDATA%\ComposerSetup\bin"

:: install php, if its alr, it will skip install
winget install --id %PHPID% -e --accept-package-agreements --accept-source-agreements

:: find php.exe of that version directly (PATH of this window isnt updated by winget + another php could be first in PATH)
set "PHPDIR="
for /d %%D in ("%LOCALAPPDATA%\Microsoft\WinGet\Packages\%PHPID%_*") do if exist "%%D\php.exe" set "PHPDIR=%%D"
if not defined PHPDIR for /d %%D in ("%ProgramFiles%\WinGet\Packages\%PHPID%_*") do if exist "%%D\php.exe" set "PHPDIR=%%D"
if not defined PHPDIR (
    echo.
    echo php.exe of %PHPID% not found, winget install failed.
    goto end
)
set "PHP=%PHPDIR%\php.exe"
echo.
echo Using php: %PHP%

:: winget php comes without php.ini, createit + enable the extensions composer needs
if not exist "%PHPDIR%\php.ini" copy /y "%PHPDIR%\php.ini-development" "%PHPDIR%\php.ini" >nul
powershell -NoProfile -Command "$f=Join-Path $env:PHPDIR 'php.ini'; $q=[char]34; $c=[IO.File]::ReadAllText($f); $c=$c -replace '(?m)^;extension_dir = \x22ext\x22', ('extension_dir = '+$q+(Join-Path $env:PHPDIR 'ext')+$q); $c=$c -replace '(?m)^;extension=(openssl|curl|mbstring|zip|fileinfo)\b', 'extension=$1'; [IO.File]::WriteAllText($f,$c)"

"%PHP%" -r "exit(extension_loaded('openssl') ? 0 : 1);"
if not %errorlevel%==0 (
    echo.
    echo php cant load the openssl extension, the php install seems broken.
    echo Fix: winget uninstall --id %PHPID% -e and run this script again.
    goto end
)

:: dw composer
echo.
echo Downloading composer installer...
set "SETUP=%TEMP%\composer-setup.php"
"%PHP%" -r "exit(copy('https://getcomposer.org/installer', $argv[1]) ? 0 : 1);" "%SETUP%"
if not %errorlevel%==0 (
    echo Download failed.
    goto end
)
"%PHP%" -r "if (hash_file('sha384', $argv[1]) !== trim(file_get_contents('https://composer.github.io/installer.sig'))) { echo 'Installer corrupt'.PHP_EOL; unlink($argv[1]); exit(1); } echo 'Installer verified'.PHP_EOL;" "%SETUP%"
if not %errorlevel%==0 goto end

if not exist "%COMPOSERDIR%" mkdir "%COMPOSERDIR%"
"%PHP%" "%SETUP%" --install-dir="%COMPOSERDIR%" --filename=composer.phar
set "ERR=%errorlevel%"
del "%SETUP%" >nul 2>&1
if not %ERR%==0 goto end

:: composer.bat so "composer" works as command, uses the php from above
>"%COMPOSERDIR%\composer.bat" echo @"%PHP%" "%%~dp0composer.phar" %%*

:: add composer + global composer packages (composer global require ...) to user PATH
set "ADD1=%COMPOSERDIR%"
set "ADD2=%APPDATA%\Composer\vendor\bin"
powershell -NoProfile -Command "$k=[Microsoft.Win32.Registry]::CurrentUser.OpenSubKey('Environment',$true); $p=[string]$k.GetValue('Path','',[Microsoft.Win32.RegistryValueOptions]::DoNotExpandEnvironmentNames); $old=$p; foreach($d in @($env:ADD1,$env:ADD2)){ if((($p -split ';') | ForEach-Object { $_.TrimEnd('\') }) -notcontains $d.TrimEnd('\')){ $p=($p.TrimEnd(';')+';'+$d).TrimStart(';'); Write-Host ('Added to PATH: '+$d) } }; if($p -ne $old){ $k.SetValue('Path',$p,[Microsoft.Win32.RegistryValueKind]::ExpandString); [Environment]::SetEnvironmentVariable('COMPOSER_SETUP_TMP','1','User'); [Environment]::SetEnvironmentVariable('COMPOSER_SETUP_TMP',$null,'User') }; $k.Close()"
set "PATH=%PATH%;%ADD1%;%ADD2%"

echo.
call "%COMPOSERDIR%\composer.bat" --version
echo.
echo Done. Open a new terminal and composer works everywhere.

:end
endlocal
echo.
pause
