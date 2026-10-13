@echo off

winget install --id Git.Git -e --accept-package-agreements --accept-source-agreements

REM np+++/ides
winget install --id Notepad++.Notepad++ -e --accept-package-agreements --accept-source-agreements
winget install --id JetBrains.IntelliJIDEA -e --accept-package-agreements --accept-source-agreements
winget install --id JetBrains.CLion -e --accept-package-agreements --accept-source-agreements
winget install --id JetBrains.PyCharm -e --accept-package-agreements --accept-source-agreements
winget install --id JetBrains.PhpStorm -e --accept-package-agreements --accept-source-agreements
winget install --id JetBrains.WebStorm -e --accept-package-agreements --accept-source-agreements

REM c/c++
winget install --id Microsoft.VisualStudio.2022.BuildTools -e --accept-package-agreements --accept-source-agreements --override "--wait --passive --add Microsoft.VisualStudio.Workload.VCTools --includeRecommended"
winget install --id LLVM.LLVM -e --accept-package-agreements --accept-source-agreements
winget install --id BrechtSanders.WinLibs.POSIX.UCRT -e --accept-package-agreements --accept-source-agreements
winget install --id Kitware.CMake -e --accept-package-agreements --accept-source-agreements
winget install --id Ninja-build.Ninja -e --accept-package-agreements --accept-source-agreements
REM rustcargo
winget install --id Rustlang.Rustup -e --accept-package-agreements --accept-source-agreements
REM csharp and .net
winget install --id Microsoft.DotNet.SDK.8 -e --accept-package-agreements --accept-source-agreements
winget install --id Microsoft.DotNet.SDK.9 -e --accept-package-agreements --accept-source-agreements
winget install --id Microsoft.DotNet.SDK.10 -e --accept-package-agreements --accept-source-agreements
winget install --id Microsoft.DotNet.Runtime.9 -e --accept-package-agreements --accept-source-agreeme
winget install --id Microsoft.DotNet.SDK.Preview -e --accept-package-agreements --accept-source-agreementsnts
winget install --id Microsoft.DotNet.Runtime.8 -e --accept-package-agreements --accept-source-agreements
winget install --id Microsoft.DotNet.Runtime.10 -e --accept-package-agreements --accept-source-agreements
REM golang
winget install --id GoLang.Go -e --accept-package-agreements --accept-source-agreements
REM py
winget install --id Python.PythonInstallManager -e --accept-package-agreements --accept-source-agreements
winget install --id Python.Launcher -e --accept-package-agreements --accept-source-agreements
winget install --id Python.Python.2 -e --accept-package-agreements --accept-source-agreements
winget install --id Python.Python.3.9 -e --accept-package-agreements --accept-source-agreements
winget install --id Python.Python.3.12 -e --accept-package-agreements --accept-source-agreements
winget install --id Python.Python.3.13 -e --accept-package-agreements --accept-source-agreements
winget install --id Python.Python.3.14 -e --accept-package-agreements --accept-source-agreements
REM js (node + npm)
winget install --id OpenJS.NodeJS.LTS -e --accept-package-agreements --accept-source-agreements
REM php
winget install --id PHP.PHP.8.3 -e --accept-package-agreements --accept-source-agreements
winget install --id PHP.PHP.8.5 -e --accept-package-agreements --accept-source-agreements
REM java eclips adoptium jdk
winget install --id EclipseAdoptium.Temurin.8.JDK -e --accept-package-agreements --accept-source-agreements
winget install --id EclipseAdoptium.Temurin.11.JDK -e --accept-package-agreements --accept-source-agreements
winget install --id EclipseAdoptium.Temurin.17.JDK -e --accept-package-agreements --accept-source-agreements
winget install --id EclipseAdoptium.Temurin.21.JDK -e --accept-package-agreements --accept-source-agreements
winget install --id EclipseAdoptium.Temurin.25.JDK -e --accept-package-agreements --accept-source-agreements
REM java amz corretto jdk
winget install --id Amazon.Corretto.8.JDK -e --accept-package-agreements --accept-source-agreements
winget install --id Amazon.Corretto.11.JDK -e --accept-package-agreements --accept-source-agreements
winget install --id Amazon.Corretto.17.JDK -e --accept-package-agreements --accept-source-agreements
winget install --id Amazon.Corretto.21.JDK -e --accept-package-agreements --accept-source-agreements
winget install --id Amazon.Corretto.25.JDK -e --accept-package-agreements --accept-source-agreements

REM db
winget install --id MariaDB.Server -e --accept-package-agreements --accept-source-agreements
winget install --id HeidiSQL.HeidiSQL -e --accept-package-agreements --accept-source-agreements
winget install --id Oracle.MySQL -e --accept-package-agreements --accept-source-agreements
winget install --id Oracle.MySQLWorkbench -e --accept-package-agreements --accept-source-agreements
winget install --id Oracle.MySQLShell -e --accept-package-agreements --accept-source-agreements
winget install --id MongoDB.Shell -e --accept-package-agreements --accept-source-agreements
winget install --id SQLite.SQLite -e --accept-package-agreements --accept-source-agreements
winget install --id DuckDB.cli -e --accept-package-agreements --accept-source-agreements

REM oth (tools)
winget install --id Gyan.FFmpeg -e --accept-package-agreements --accept-source-agreements
winget install --id PuTTY.PuTTY -e --accept-package-agreements --accept-source-agreements
winget install --id RARLab.WinRAR -e --accept-package-agreements --accept-source-agreements
winget install --id WinSCP.WinSCP -e --accept-package-agreements --accept-source-agreements
winget install --id Microsoft.PowerToys -e --accept-package-agreements --accept-source-agreements
REM oth (user)
winget install --id BlenderFoundation.Blender -e --accept-package-agreements --accept-source-agreements
winget install --id GIMP.GIMP.3 -e --accept-package-agreements --accept-source-agreements
winget install --id DuckDuckGo.DesktopBrowser -e --accept-package-agreements --accept-source-agreements
::winget install --id FACEITLTD.FACEITClient -e --accept-package-agreements --accept-source-agreements
::winget install --id FACEITLTD.FACEITAC -e --accept-package-agreements --accept-source-agreements
winget install --id Mozilla.Firefox.de -e --accept-package-agreements --accept-source-agreements
winget install --id TeamSpeakSystems.TeamSpeakClient -e --accept-package-agreements --accept-source-agreements

pause
