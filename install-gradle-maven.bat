@echo off
setlocal
title install gradle + maven

:: both get installed here, running it again: update to latest version
set "ROOT=%LOCALAPPDATA%\Programs"
set "GRADLEDIR=%ROOT%\gradle"
set "MAVENDIR=%ROOT%\maven"
set "MAVENREPO=https://repo.maven.apache.org/maven2/org/apache/maven/apache-maven"
set "DL=%TEMP%\gradle-maven-install"

:: install jdk if not there
where java >nul 2>&1
if %errorlevel%==0 goto java_ok
echo java not found, installing Temurin JDK 21...
winget install --id EclipseAdoptium.Temurin.21.JDK -e --accept-package-agreements --accept-source-agreements
:: reload PATH so this window finds the new java
for /f "delims=" %%P in ('powershell -NoProfile -Command "[Environment]::GetEnvironmentVariable('Path','Machine')+';'+[Environment]::GetEnvironmentVariable('Path','User')"') do set "PATH=%%P"
:java_ok

if exist "%DL%" rmdir /s /q "%DL%"
mkdir "%DL%"
if not exist "%ROOT%" mkdir "%ROOT%"

:: gradle, latest version + sha256 from services.gradle.org
echo.
echo Getting latest gradle version...
curl -fsSL -o "%DL%\gradle.json" https://services.gradle.org/versions/current
if not %errorlevel%==0 (
    echo Could not reach services.gradle.org.
    goto end
)
set "GVER="
for /f "tokens=1,2" %%A in ('powershell -NoProfile -Command "$j=Get-Content -Raw '%DL%\gradle.json' | ConvertFrom-Json; $j.version+' '+$j.checksum"') do (
    set "GVER=%%A"
    set "GSHA=%%B"
)
if not defined GVER (
    echo Could not read the gradle version.
    goto end
)
echo Downloading gradle %GVER%...
curl -fL -# -o "%DL%\gradle.zip" https://services.gradle.org/distributions/gradle-%GVER%-bin.zip
if not %errorlevel%==0 (
    echo Download failed.
    goto end
)
powershell -NoProfile -Command "$s=[IO.File]::OpenRead('%DL%\gradle.zip'); $h=[BitConverter]::ToString([Security.Cryptography.SHA256]::Create().ComputeHash($s)).Replace('-',''); $s.Close(); if($h -ne '%GSHA%'){ exit 1 }"
if not %errorlevel%==0 (
    echo Checksum wrong, download is corrupt.
    goto end
)
echo Checksum ok, extracting...
:: stop running gradle daemons first, they lock the files of the old version
if exist "%GRADLEDIR%\bin\gradle.bat" call "%GRADLEDIR%\bin\gradle.bat" --stop >nul 2>&1
call :replace "%DL%\gradle.zip" "gradle-%GVER%" "%GRADLEDIR%" || goto end

:: maven, latest stable version (full v) + sha512 from maven central
echo.
echo Getting latest maven version...
curl -fsSL -o "%DL%\maven.xml" %MAVENREPO%/maven-metadata.xml
if not %errorlevel%==0 (
    echo Could not reach repo.maven.apache.org.
    goto end
)
set "MVER="
for /f "delims=" %%V in ('powershell -NoProfile -Command "([xml](Get-Content -Raw '%DL%\maven.xml')).metadata.versioning.versions.version | Where-Object { $_ -match '^\d+\.\d+\.\d+$' } | Sort-Object { [version]$_ } | Select-Object -Last 1"') do set "MVER=%%V"
if not defined MVER (
    echo Could not read the maven version.
    goto end
)
echo Downloading maven %MVER%...
curl -fL -# -o "%DL%\maven.zip" %MAVENREPO%/%MVER%/apache-maven-%MVER%-bin.zip
if not %errorlevel%==0 (
    echo Download failed.
    goto end
)
curl -fsSL -o "%DL%\maven.zip.sha512" %MAVENREPO%/%MVER%/apache-maven-%MVER%-bin.zip.sha512
powershell -NoProfile -Command "$want=([IO.File]::ReadAllText('%DL%\maven.zip.sha512').Trim() -split '\s+')[0]; $s=[IO.File]::OpenRead('%DL%\maven.zip'); $h=[BitConverter]::ToString([Security.Cryptography.SHA512]::Create().ComputeHash($s)).Replace('-',''); $s.Close(); if($h -ne $want){ exit 1 }"
if not %errorlevel%==0 (
    echo Checksum wrong, download is corrupt.
    goto end
)
echo Checksum ok, extracting...
call :replace "%DL%\maven.zip" "apache-maven-%MVER%" "%MAVENDIR%" || goto end

:: add them to path + set gradle home & maven home
set "ADD1=%GRADLEDIR%\bin"
set "ADD2=%MAVENDIR%\bin"
powershell -NoProfile -Command "$k=[Microsoft.Win32.Registry]::CurrentUser.OpenSubKey('Environment',$true); $p=[string]$k.GetValue('Path','',[Microsoft.Win32.RegistryValueOptions]::DoNotExpandEnvironmentNames); $old=$p; foreach($d in @($env:ADD1,$env:ADD2)){ if((($p -split ';') | ForEach-Object { $_.TrimEnd('\') }) -notcontains $d.TrimEnd('\')){ $p=($p.TrimEnd(';')+';'+$d).TrimStart(';'); Write-Host ('Added to PATH: '+$d) } }; if($p -ne $old){ $k.SetValue('Path',$p,[Microsoft.Win32.RegistryValueKind]::ExpandString) }; $k.Close(); [Environment]::SetEnvironmentVariable('GRADLE_HOME',$env:GRADLEDIR,'User'); [Environment]::SetEnvironmentVariable('MAVEN_HOME',$env:MAVENDIR,'User')"
set "PATH=%PATH%;%ADD1%;%ADD2%"

echo.
call "%GRADLEDIR%\bin\gradle.bat" --version
echo.
call "%MAVENDIR%\bin\mvn.cmd" -v
echo.
echo Done. Open a new terminal and gradle + mvn work everywhere.

:end
if exist "%DL%" rmdir /s /q "%DL%"
endlocal
echo.
pause
exit /b

:replace
:: %1 = zip, %2 = folder inside the zip, %3 = install folder (old v gets replaced)
if exist "%ROOT%\%~2" rmdir /s /q "%ROOT%\%~2"
tar -xf "%~1" -C "%ROOT%"
if not exist "%ROOT%\%~2" (
    echo Extracting %~nx1 failed.
    exit /b 1
)
if exist "%~3" rmdir /s /q "%~3"
if exist "%~3" (
    echo Could not delete the old %~3, close everything that uses it and try again.
    rmdir /s /q "%ROOT%\%~2"
    exit /b 1
)
ren "%ROOT%\%~2" "%~nx3"
exit /b 0
