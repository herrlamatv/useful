@echo off
title specific-task restarter
rem shows every running task + service (num on left)
rem type num to restart. name to search or "cancel" to quit
rem if something cant be restarted, the script tells you why + asks if you want to stop it instead

set "SELF=%~f0"
:: the bat only starts ps, which reads this file + runs everything below the marker
powershell -NoProfile -ExecutionPolicy Bypass -Command "$s=Get-Content -LiteralPath $env:SELF -Raw; iex $s.Substring($s.IndexOf('#PS'+'START'))"
exit /b

#PSSTART
# ===== shared part (also used by the 2nd ps that runs as admin) =====

$IsAdmin = ([Security.Principal.WindowsPrincipal][Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)
$MySession = (Get-Process -Id $PID).SessionId
# critical Windows processes: stopping them crashes Windows (bluescreen) or logs you out
$Protected = 'System', 'Registry', 'Memory Compression', 'Secure System', 'smss.exe', 'csrss.exe', 'wininit.exe', 'services.exe', 'lsass.exe', 'LsaIso.exe', 'winlogon.exe', 'svchost.exe', 'fontdrvhost.exe'
# never touch this script itself (+ its console)
$Skip = @($PID) + @(Get-CimInstance Win32_Process -Filter "ParentProcessId=$PID" | ForEach-Object { [int]$_.ProcessId })

function Say([string]$Text, [string]$Color = 'Gray') {
    # the admin ps has no visible window, so it writes into a log file that the normal window shows
    if ($LogFile) { Add-Content -LiteralPath $LogFile -Value "$Color|$Text" -Encoding UTF8 }
    else { Write-Host "  $Text" -ForegroundColor $Color }
}

function Get-ErrorText($Err) {
    $ex = $Err.Exception
    while ($ex.InnerException) { $ex = $ex.InnerException }
    $ex.Message
}

# waits (max $Ticks x 250ms) until none of the processes is running anymore
function Wait-Gone([int[]]$Ids, [int]$Ticks) {
    for ($i = 0; $i -lt $Ticks; $i++) {
        if (-not (Get-Process -Id $Ids -ErrorAction SilentlyContinue)) { return }
        Start-Sleep -Milliseconds 250
    }
}

# closes the processes: first politely (like clicking X), then by force. Returns the IDs that are still running.
function Stop-Pids([int[]]$Ids, [bool]$Polite) {
    if ($Polite) {
        # not explorer: closing its desktop window opens the "shut down Windows" dialog
        $windows = @(Get-Process -Id $Ids -ErrorAction SilentlyContinue | Where-Object { $_.MainWindowHandle -ne 0 -and $_.ProcessName -ne 'explorer' })
        foreach ($p in $windows) { try { [void]$p.CloseMainWindow() } catch {} }
        if ($windows) { Wait-Gone $Ids 8 }
    }
    Stop-Process -Id $Ids -Force -ErrorAction SilentlyContinue
    Wait-Gone $Ids 8
    @(Get-Process -Id $Ids -ErrorAction SilentlyContinue | ForEach-Object { $_.Id })
}

# Store apps (in ...\WindowsApps) can't be started with their .exe, they need their app ID
function Get-Aumid([string]$Path) {
    try {
        $pkg = Get-AppxPackage -ErrorAction Stop | Where-Object { $_.InstallLocation -and $Path.StartsWith($_.InstallLocation + '\', [StringComparison]::OrdinalIgnoreCase) } | Select-Object -First 1
        if (-not $pkg) { return $null }
        $rel = $Path.Substring($pkg.InstallLocation.Length + 1)
        $apps = @((Get-AppxPackageManifest -Package $pkg).Package.Applications.Application)
        $app = $apps | Where-Object { $_.Executable -eq $rel } | Select-Object -First 1
        if (-not $app -and $apps.Count -eq 1) { $app = $apps[0] }
        if ($app) { return "$($pkg.PackageFamilyName)!$($app.Id)" }
    } catch {}
    $null
}

# the arguments the program was started with (everything after the .exe in its command line)
function Get-StartArgs([string]$Line, [string]$Path) {
    if (-not $Line) { return '' }
    $Line = $Line.Trim()
    if ($Line.StartsWith('"')) {
        $end = $Line.IndexOf('"', 1)
        if ($end -lt 0) { return '' }
        return $Line.Substring($end + 1).Trim()
    }
    if ($Line.StartsWith($Path, [StringComparison]::OrdinalIgnoreCase)) { return $Line.Substring($Path.Length).Trim() }
    $space = $Line.IndexOf(' ')
    if ($space -lt 0) { return '' }
    $Line.Substring($space + 1).Trim()
}

# finds every process of a program and works out how to start it again
function Get-Plan([string]$Name) {
    $procs = @(Get-CimInstance Win32_Process)
    $byId = @{}
    foreach ($p in $procs) { $byId[[int]$p.ProcessId] = $p }
    $mine = @($procs | Where-Object { $_.Name -eq $Name -and $Skip -notcontains [int]$_.ProcessId })
    $plan = [pscustomobject]@{ Ids = @($mine | ForEach-Object { [int]$_.ProcessId }); Launch = @(); Problem = $null; NeedAdmin = $false; Services = @() }
    if (-not $mine) { return $plan }
    $plan.Services = @(Get-CimInstance Win32_Service -Filter "State='Running'" | Where-Object { $plan.Ids -contains [int]$_.ProcessId })

    foreach ($p in $mine) {
        $parent = $byId[[int]$p.ParentProcessId]
        # process IDs get reused, so the parent only counts if it was started before this process
        if ($parent -and $parent.CreationDate -gt $p.CreationDate) { $parent = $null }
        # a child process of the same program gets started again by its main process
        if ($parent -and $parent.Name -eq $Name) { continue }

        $path = $p.ExecutablePath
        if ([int]$p.SessionId -ne $MySession) {
            $plan.Problem = 'it runs in the background for Windows or a service (not as your program).'
            break
        }
        if (-not $path) {
            $plan.Problem = 'no access to it, it runs as admin or system.'
            $plan.NeedAdmin = $true
            break
        }
        if ($path.StartsWith("$env:windir\SystemApps\", [StringComparison]::OrdinalIgnoreCase)) {
            # Windows parts like start menu / search: Windows starts them again by itself
            $plan.Launch += [pscustomobject]@{ Kind = 'auto' }
        }
        elseif ($path -like '*\WindowsApps\*') {
            $aumid = Get-Aumid $path
            if (-not $aumid) { $plan.Problem = "it's a Store app that can't be started again from here."; break }
            $plan.Launch += [pscustomobject]@{ Kind = 'app'; Aumid = $aumid }
        }
        elseif ($path.StartsWith("$env:windir\", [StringComparison]::OrdinalIgnoreCase) -and $parent -and $parent.Name -ne 'explorer.exe' -and $Name -ne 'explorer.exe') {
            # Windows' own helper processes (started by Windows or another program, not by you)
            $plan.Launch += [pscustomobject]@{ Kind = 'auto' }
        }
        else {
            $plan.Launch += [pscustomobject]@{ Kind = 'exe'; File = $path; Args = (Get-StartArgs $p.CommandLine $path) }
        }
    }
    $plan
}

# starts the program again after it was closed
function Start-Program([string]$Name, [int[]]$OldIds, $Launch) {
    $base = [IO.Path]::GetFileNameWithoutExtension($Name)
    # some programs come back by themselves (explorer, Windows parts, apps with a watchdog)
    for ($i = 0; $i -lt 8; $i++) {
        Start-Sleep -Milliseconds 250
        if (Get-Process | Where-Object { $_.ProcessName -eq $base -and $OldIds -notcontains $_.Id }) {
            Say "$Name restarted (it came back by itself)." Green
            return $true
        }
    }
    $started = 0
    foreach ($L in @($Launch)) {
        if ($L.Kind -eq 'auto') { continue }
        try {
            if ($L.Kind -eq 'app') {
                # explorer starts the Store app (also as normal user, even if this runs as admin)
                $si = New-Object Diagnostics.ProcessStartInfo 'explorer.exe', "shell:AppsFolder\$($L.Aumid)"
            } else {
                $si = New-Object Diagnostics.ProcessStartInfo $L.File, ([string]$L.Args)
                $si.WorkingDirectory = Split-Path $L.File
            }
            $si.UseShellExecute = $true
            [void][Diagnostics.Process]::Start($si)
            $started++
        } catch {
            Say "Could not start $Name again: $(Get-ErrorText $_)" Red
            return $false
        }
    }
    if ($started) { Say "$Name restarted." Green }
    else { Say "$Name was closed. Windows (or the program that opened it) starts it again by itself when it's needed." Yellow }
    $true
}

# all running services that need this service (deepest first = the order to stop them)
function Get-RunningDependents($Svc) {
    foreach ($d in $Svc.DependentServices) {
        if ($d.Status -ne 'Stopped') { Get-RunningDependents $d; $d }
    }
}

# stops a service + the services that need it, returns those so they can be started again
function Stop-Svc($Svc) {
    $seen = @{}
    $deps = @(foreach ($d in @(Get-RunningDependents $Svc)) { if (-not $seen[$d.Name]) { $seen[$d.Name] = $true; $d } })
    foreach ($s in @($deps) + @($Svc)) {
        $s.Refresh()
        if ($s.Status -eq 'Stopped') { continue }
        Say "  stopping $($s.DisplayName) ..." DarkGray
        if ($s.Status -ne 'StopPending') { $s.Stop() }
        $s.WaitForStatus('Stopped', [TimeSpan]::FromSeconds(30))
    }
    $deps
}

function Restart-Svc([string]$Name) {
    try {
        $svc = Get-Service -Name $Name -ErrorAction Stop
        Say "Restarting service '$($svc.DisplayName)' ..." Cyan
        $deps = @(Stop-Svc $svc)
        Say "  starting $($svc.DisplayName) ..." DarkGray
        $svc.Refresh()
        if ($svc.Status -ne 'Running') { $svc.Start() }
        $svc.WaitForStatus('Running', [TimeSpan]::FromSeconds(30))
        # start the services that needed it again (in start order)
        [array]::Reverse($deps)
        foreach ($d in $deps) {
            try {
                $d.Refresh()
                if ($d.Status -eq 'Running') { continue }
                Say "  starting $($d.DisplayName) ..." DarkGray
                $d.Start()
                $d.WaitForStatus('Running', [TimeSpan]::FromSeconds(30))
            } catch { Say "  $($d.DisplayName) could not be started again: $(Get-ErrorText $_)" Yellow }
        }
        Say "Service '$($svc.DisplayName)' restarted." Green
        'ok'
    } catch {
        Say "Could not restart it: $(Get-ErrorText $_)" Red
        'fail'
    }
}

# one action. Runs directly, or in the admin ps (then $J comes from the normal window)
function Invoke-Action($J) {
    switch ($J.Do) {
        'restart' {
            $plan = Get-Plan $J.Name
            if (-not $plan.Ids) { Say "$($J.Name) is not running anymore." Yellow; return 'gone' }
            if ($plan.Problem) { Say "$($J.Name) can't be restarted: $($plan.Problem)" Yellow; return 'problem' }
            Say "Restarting $($J.Name) ..." Cyan
            if (Stop-Pids $plan.Ids $true) { Say "Could not close $($J.Name), Windows protects it." Red; return 'fail' }
            if (Start-Program $J.Name $plan.Ids $plan.Launch) { return 'ok' }
            return 'fail'
        }
        'finish' {
            # the normal window already closed what it could, this closes the rest + starts it again
            if (Stop-Pids $J.Left $false) { Say "Could not close $($J.Name), Windows protects it." Red; return 'fail' }
            if (Start-Program $J.Name $J.Ids $J.Launch) { return 'ok' }
            return 'fail'
        }
        'stop' {
            if (Stop-Pids $J.Ids ([bool]$J.Polite)) { Say "Could not stop $($J.Name), Windows protects it." Red; return 'fail' }
            Say "$($J.Name) stopped." Green
            return 'ok'
        }
        'svc-restart' { return (Restart-Svc $J.Name) }
        'svc-stop' {
            try {
                $svc = Get-Service -Name $J.Name -ErrorAction Stop
                [void](Stop-Svc $svc)
                Say "Service '$($svc.DisplayName)' stopped." Green
                return 'ok'
            } catch { Say "Could not stop it: $(Get-ErrorText $_)" Red; return 'fail' }
        }
    }
}

# --- this is the admin ps: do the one action and quit ---
if ($Job) {
    $J = [Text.Encoding]::UTF8.GetString([Convert]::FromBase64String($Job)) | ConvertFrom-Json
    $Skip += @($J.Skip)
    try { $r = Invoke-Action $J } catch { Say "Error: $(Get-ErrorText $_)" Red; $r = 'fail' }
    Add-Content -LiteralPath $LogFile -Value "RESULT|$r" -Encoding UTF8
    return
}

# ===== main (the normal window) =====

# the cmd window that runs this .bat (+ its console) is not shown in the list either
$me = Get-CimInstance Win32_Process -Filter "ProcessId=$PID"
$Skip += [int]$me.ParentProcessId
$Skip += @(Get-CimInstance Win32_Process -Filter "ParentProcessId=$($me.ParentProcessId)" | ForEach-Object { [int]$_.ProcessId })

# runs an action in a 2nd ps with admin perms (UAC) and shows what it did
function Invoke-Elevated($J) {
    $J.Skip = $Skip
    $log = Join-Path $env:TEMP ('specific-task-' + [guid]::NewGuid().ToString('N') + '.log')
    $data = [Convert]::ToBase64String([Text.Encoding]::UTF8.GetBytes(($J | ConvertTo-Json -Depth 5 -Compress)))
    $boot = "`$Job='$data'; `$LogFile='$($log.Replace("'", "''"))'; `$s=Get-Content -LiteralPath '$($env:SELF.Replace("'", "''"))' -Raw; iex `$s.Substring(`$s.IndexOf('#PS'+'START'))"
    $enc = [Convert]::ToBase64String([Text.Encoding]::Unicode.GetBytes($boot))
    Say 'Asking for admin rights (UAC) ...' Yellow
    try {
        $p = Start-Process powershell.exe -Verb RunAs -WindowStyle Hidden -PassThru -ErrorAction Stop -ArgumentList "-NoProfile -ExecutionPolicy Bypass -EncodedCommand $enc"
    } catch {
        Say 'No admin rights given, cancelled.' Red
        return $null
    }
    $p.WaitForExit()
    $result = 'fail'
    if (Test-Path -LiteralPath $log) {
        foreach ($line in (Get-Content -LiteralPath $log -Encoding UTF8)) {
            $color, $text = $line -split '\|', 2
            if ($color -eq 'RESULT') { $result = $text } else { Say $text $color }
        }
        Remove-Item -LiteralPath $log -Force -ErrorAction SilentlyContinue
    } else {
        Say 'The admin part did not run.' Red
    }
    $result
}

# runs an action that always needs admin rights (services)
function Invoke-AsAdmin($J) {
    if ($IsAdmin) { Invoke-Action $J } else { Invoke-Elevated $J }
}

function Ask([string]$Question) {
    while ($true) {
        $a = "$(Read-Host "  $Question (y/n)")".Trim().ToLower()
        if ($a -in 'y', 'yes', 'j', 'ja') { return $true }
        if ($a -in 'n', 'no', 'nein', 'cancel', '') { return $false }
    }
}

function Invoke-Program($e) {
    $name = $e.Name
    if ($Protected -contains $name) {
        Say "$name can't be restarted or stopped: it's a critical part of Windows." Red
        Say 'Stopping it would crash Windows (bluescreen) or log you out.' DarkGray
        return
    }
    $plan = Get-Plan $name
    if (-not $plan.Ids) { Say "$name is not running anymore." Yellow; return }
    $count = "$($plan.Ids.Count) process" + $(if ($plan.Ids.Count -ne 1) { 'es' })

    if ($plan.Problem -and $plan.NeedAdmin -and -not $IsAdmin) {
        Say "$name runs as admin, so admin rights are needed to restart it." Yellow
        if ((Invoke-Elevated @{ Do = 'restart'; Name = $name }) -ne 'problem') { return }
    }
    elseif ($plan.Problem) {
        Say "$name can't be restarted: $($plan.Problem)" Yellow
    }
    else {
        Say "Restarting $name ($count) ..." Cyan
        $left = Stop-Pids $plan.Ids $true
        if (-not $left) { [void](Start-Program $name $plan.Ids $plan.Launch); return }
        if ($IsAdmin) { Say "Could not close $name, Windows protects it." Red; return }
        Say 'Some of its processes need admin rights to be closed.' Yellow
        if ($null -eq (Invoke-Elevated @{ Do = 'finish'; Name = $name; Ids = $plan.Ids; Left = @($left); Launch = $plan.Launch })) {
            Say "$name is only partly closed now." Yellow
        }
        return
    }

    # it can't be restarted -> ask if it should be stopped instead
    foreach ($s in $plan.Services) {
        $svc = $list | Where-Object { $_.Type -eq 'svc' -and $_.Name -eq $s.Name } | Select-Object -First 1
        if ($svc) { Say "Tip: it belongs to the service '$($s.DisplayName)' (number $($svc.Num)), you can restart that one instead." DarkGray }
    }
    if (-not (Ask "Do you want to stop $name instead? ($count)")) { Say 'Cancelled, nothing was changed.' DarkGray; return }
    $left = Stop-Pids $plan.Ids $true
    if (-not $left) { Say "$name stopped." Green; return }
    if ($IsAdmin) { Say "Could not stop $name, Windows protects it." Red; return }
    Say 'Admin rights are needed to stop it.' Yellow
    [void](Invoke-Elevated @{ Do = 'stop'; Name = $name; Ids = @($left); Polite = $false })
}

function Invoke-Service($e) {
    $svc = Get-CimInstance Win32_Service -Filter "Name='$($e.Name.Replace('\', '\\').Replace("'", "\'"))'"
    if (-not $svc -or $svc.State -ne 'Running') { Say "Service '$($e.Display)' is not running anymore." Yellow; return }
    if (-not $svc.AcceptStop) {
        Say "Service '$($svc.DisplayName)' can't be restarted or stopped." Red
        Say "Windows doesn't allow stopping it (it's needed to keep Windows running)." DarkGray
        return
    }
    if ($svc.StartMode -eq 'Disabled') {
        Say "Service '$($svc.DisplayName)' can't be restarted: it's disabled, so it would not start again." Yellow
    } else {
        if ((Invoke-AsAdmin @{ Do = 'svc-restart'; Name = $svc.Name }) -ne 'fail') { return }
        if ((Get-Service -Name $svc.Name).Status -ne 'Running') { Say 'The service is stopped now.' Yellow; return }
        Say "Service '$($svc.DisplayName)' is still running, but it can't be restarted." Yellow
    }
    if (-not (Ask "Do you want to stop the service '$($svc.DisplayName)' instead?")) { Say 'Cancelled, nothing was changed.' DarkGray; return }
    [void](Invoke-AsAdmin @{ Do = 'svc-stop'; Name = $svc.Name })
}

# every running program (grouped by name, like the task manager) + every running service
function Get-List {
    $windows = @{}
    foreach ($p in Get-Process) { if ($p.MainWindowTitle) { $windows[$p.Id] = $p.MainWindowTitle } }
    $rows = foreach ($p in Get-CimInstance Win32_Process) {
        $id = [int]$p.ProcessId
        if ($id -eq 0 -or $Skip -contains $id) { continue }
        [pscustomobject]@{ Name = $p.Name; Session = [int]$p.SessionId; Path = [string]$p.ExecutablePath; Title = $windows[$id]; Mem = [double]$p.WorkingSetSize }
    }
    $list = @()
    foreach ($g in ($rows | Group-Object Name | Sort-Object Name)) {
        $r = $g.Group
        $mine = @($r | Where-Object { $_.Session -eq $MySession })
        if ($Protected -contains $g.Name) { $sec = 'Windows' }
        elseif ($mine | Where-Object { $_.Title }) { $sec = 'App' }
        elseif ($mine | Where-Object { $_.Path -and -not $_.Path.StartsWith("$env:windir\", [StringComparison]::OrdinalIgnoreCase) }) { $sec = 'Background' }
        else { $sec = 'Windows' }
        $info = $(if ($Protected -contains $g.Name) { '[protected]' } else { ($r | Where-Object { $_.Title } | Select-Object -First 1).Title })
        $list += [pscustomobject]@{ Type = 'proc'; Section = $sec; Num = 0; Name = $g.Name; Count = $g.Count; Mem = ($r | Measure-Object Mem -Sum).Sum; Info = $info }
    }
    foreach ($s in (Get-CimInstance Win32_Service -Filter "State='Running'" | Sort-Object DisplayName)) {
        $info = $(if (-not $s.AcceptStop) { '[protected]' } else { '' })
        $list += [pscustomobject]@{ Type = 'svc'; Section = 'Service'; Num = 0; Name = $s.Name; Display = $s.DisplayName; Info = $info }
    }
    # apps get the small numbers, they are shown at the bottom (right above the input)
    $n = 0
    foreach ($sec in 'App', 'Background', 'Windows', 'Service') {
        foreach ($e in $list) { if ($e.Section -eq $sec) { $n++; $e.Num = $n } }
    }
    $list
}

function Fit([string]$Text, [int]$Max) {
    if ($Max -lt 4) { return '' }
    if ($Text.Length -le $Max) { return $Text }
    $Text.Substring(0, $Max - 3) + '...'
}

function Format-Mem([double]$Bytes) {
    if ($Bytes -ge 1GB) { '{0:N1} GB' -f ($Bytes / 1GB) } else { '{0:N0} MB' -f ($Bytes / 1MB) }
}

function Write-Entry($e, [int]$Width) {
    Write-Host ('{0,5}  ' -f $e.Num) -ForegroundColor Cyan -NoNewline
    if ($e.Type -eq 'svc') {
        Write-Host ('{0,-48}  ' -f (Fit $e.Display 48)) -NoNewline
        Write-Host (Fit "$($e.Name)  $($e.Info)" ($Width - 57)) -ForegroundColor DarkGray
    } else {
        $count = $(if ($e.Count -gt 1) { "x$($e.Count)" } else { '' })
        Write-Host ('{0,-32}  ' -f (Fit $e.Name 32)) -NoNewline
        Write-Host ('{0,4} {1,9}  ' -f $count, (Format-Mem $e.Mem)) -ForegroundColor DarkGray -NoNewline
        Write-Host (Fit $e.Info ($Width - 57)) -ForegroundColor DarkGray
    }
}

function Show-List($List, [string]$Filter) {
    $width = 119
    try { $width = $Host.UI.RawUI.WindowSize.Width - 1 } catch {}
    $titles = @{ Service = 'Services (running)'; Windows = 'Windows / system processes'; Background = 'Background processes'; App = 'Apps (with a window)' }
    $found = 0
    foreach ($sec in 'Service', 'Windows', 'Background', 'App') {
        $items = @($List | Where-Object { $_.Section -eq $sec })
        if ($Filter) { $items = @($items | Where-Object { "$($_.Name) $($_.Display) $($_.Info)".IndexOf($Filter, [StringComparison]::OrdinalIgnoreCase) -ge 0 }) }
        if (-not $items) { continue }
        Write-Host ''
        Write-Host "  -- $($titles[$sec]) --" -ForegroundColor Yellow
        foreach ($e in $items) { Write-Entry $e $width }
        $found += $items.Count
    }
    if ($Filter -and -not $found) { Write-Host ''; Write-Host "  Nothing found for '$Filter'." -ForegroundColor Yellow }
}

$list = $null
$filter = ''
$note = ''
while ($true) {
    if (-not $list) { Write-Host '  Loading ...' -ForegroundColor DarkGray; $list = @(Get-List) }
    Clear-Host
    Show-List $list $filter
    Write-Host ''
    if ($note) { Say $note Yellow; $note = '' }
    Write-Host '  number = restart it   |   name = search   |   Enter = full list (refresh)   |   cancel = quit' -ForegroundColor DarkGray
    $in = "$(Read-Host '  Number')".Trim()
    if ($in -in 'cancel', 'exit', 'quit') { break }
    if (-not $in) { $filter = ''; $list = $null; continue }
    if ($in -notmatch '^\d{1,6}$') { $filter = $in; continue }
    $e = $list | Where-Object { $_.Num -eq [int]$in } | Select-Object -First 1
    if (-not $e) { $note = "There is no number $in."; continue }
    Write-Host ''
    if ($e.Type -eq 'svc') { Invoke-Service $e } else { Invoke-Program $e }
    Write-Host ''
    [void](Read-Host '  Press Enter to go back to the list')
    $filter = ''
    $list = $null
}
