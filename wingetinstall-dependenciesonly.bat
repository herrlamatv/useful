@echo off

winget install --id Git.Git -e --accept-package-agreements --accept-source-agreements
winget install --id Microsoft.PowerShell -e --accept-package-agreements --accept-source-agreements

REM runtimes (vc++)
winget install --id Microsoft.VCRedist.2015+.x64 -e --accept-package-agreements --accept-source-agreements
winget install --id Microsoft.VCRedist.2015+.x86 -e --accept-package-agreements --accept-source-agreements
winget install --id abbodi1406.vcredist -e --accept-package-agreements --accept-source-agreements

REM c/c++
winget install --id Microsoft.VisualStudio.2022.BuildTools -e --accept-package-agreements --accept-source-agreements --override "--wait --passive --add Microsoft.VisualStudio.Workload.VCTools --includeRecommended"
winget install --id LLVM.LLVM -e --accept-package-agreements --accept-source-agreements
winget install --id BrechtSanders.WinLibs.POSIX.UCRT -e --accept-package-agreements --accept-source-agreements
winget install --id Kitware.CMake -e --accept-package-agreements --accept-source-agreements
winget install --id Ninja-build.Ninja -e --accept-package-agreements --accept-source-agreements
winget install --id JFrog.Conan -e --accept-package-agreements --accept-source-agreements
REM perl, nasm, protoc, openssl
winget install --id StrawberryPerl.StrawberryPerl -e --accept-package-agreements --accept-source-agreements
winget install --id NASM.NASM -e --accept-package-agreements --accept-source-agreements
winget install --id Google.Protobuf -e --accept-package-agreements --accept-source-agreements
winget install --id ShiningLight.OpenSSL.Dev -e --accept-package-agreements --accept-source-agreements
REM gpu
winget install --id Nvidia.CUDA -e --accept-package-agreements --accept-source-agreements
winget install --id KhronosGroup.VulkanSDK -e --accept-package-agreements --accept-source-agreements
REM zig
winget install --id zig.zig -e --accept-package-agreements --accept-source-agreements
REM rustcargo
winget install --id Rustlang.Rustup -e --accept-package-agreements --accept-source-agreements
REM csharp and .net
winget install --id Microsoft.DotNet.SDK.8 -e --accept-package-agreements --accept-source-agreements
winget install --id Microsoft.DotNet.SDK.9 -e --accept-package-agreements --accept-source-agreements
winget install --id Microsoft.DotNet.SDK.10 -e --accept-package-agreements --accept-source-agreements
winget install --id Microsoft.DotNet.Runtime.9 -e --accept-package-agreements --accept-source-agreements
winget install --id Microsoft.DotNet.SDK.Preview -e --accept-package-agreements --accept-source-agreements
winget install --id Microsoft.DotNet.Runtime.8 -e --accept-package-agreements --accept-source-agreements
winget install --id Microsoft.DotNet.Runtime.10 -e --accept-package-agreements --accept-source-agreements
winget install --id Microsoft.DotNet.dotnet-ef -e --accept-package-agreements --accept-source-agreements
REM golang
winget install --id GoLang.Go -e --accept-package-agreements --accept-source-agreements
REM py
winget install --id Python.PythonInstallManager -e --accept-package-agreements --accept-source-agreements
winget install --id Python.Launcher -e --accept-package-agreements --accept-source-agreements
::winget install --id Python.Python.2 -e --accept-package-agreements --accept-source-agreements
winget install --id Python.Python.3.9 -e --accept-package-agreements --accept-source-agreements
winget install --id Python.Python.3.12 -e --accept-package-agreements --accept-source-agreements
winget install --id Python.Python.3.13 -e --accept-package-agreements --accept-source-agreements
winget install --id Python.Python.3.14 -e --accept-package-agreements --accept-source-agreements
REM js (node + npm)
winget install --id OpenJS.NodeJS.LTS -e --accept-package-agreements --accept-source-agreements
winget install --id Schniz.fnm -e --accept-package-agreements --accept-source-agreements
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
winget install --id MongoDB.DatabaseTools -e --accept-package-agreements --accept-source-agreements

REM oth (tools)
winget install --id Gyan.FFmpeg -e --accept-package-agreements --accept-source-agreements
winget install --id yt-dlp.yt-dlp -e --accept-package-agreements --accept-source-agreements
winget install --id JohnMacFarlane.Pandoc -e --accept-package-agreements --accept-source-agreements
winget install --id ImageMagick.ImageMagick -e --accept-package-agreements --accept-source-agreements
winget install --id UB-Mannheim.TesseractOCR -e --accept-package-agreements --accept-source-agreements

pause
