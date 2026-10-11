@echo off
title filescanner
mode con cols=120 lines=45 >nul 2>&1 <nul

:: starts pwrshell + runs the script below (paths go through env vars, so ' in a path cant break it)
set "FS_BAT=%~f0"
set "FS_DIR=%~dp0"
powershell -NoProfile -ExecutionPolicy Bypass -Command "$s=Get-Content -LiteralPath $env:FS_BAT -Raw; iex $s.Substring($s.IndexOf('#PS'+'START'))"

if errorlevel 1 pause
exit /b

#PSSTART
# settings (edit me)
$SkipFolders  = @('$Recycle.Bin', 'System Volume Information')   # folder names that never get entered
$ConsoleLimit = 300                                                # max results printed here (the .md always gets all)
$BarWidth     = 40

# presets
#   Mode:   files      = matching files
#           folders    = folders that contain matching files
#           foldername = folders whose name matches
#   Ask:    name+ext / foldername -> asks the user, otherwise the preset is used as it is
#   Ext:    extensions, MinSize: min file size, Folder + Parent: exact folder name, shows the folder above it
$Presets = @(
    @{ Name = 'Custom search (file name and/or extension)';      Mode = 'files';      Ask = 'name+ext' }
    @{ Name = 'Folders that contain ... (name and/or extension)'; Mode = 'folders';    Ask = 'name+ext' }
    @{ Name = 'Folders by name';                                 Mode = 'foldername'; Ask = 'foldername' }
    @{ Name = 'Web files';                                       Mode = 'files';      Ext = '.html .htm .css .js' }
    @{ Name = 'Web project folders (contain a .html)';           Mode = 'folders';    Ext = '.html .htm' }
    @{ Name = 'Images';                                          Mode = 'files';      Ext = '.jpg .jpeg .png .gif .webp .svg .bmp .ico .heic .tif .tiff' }
    @{ Name = 'Videos';                                          Mode = 'files';      Ext = '.mp4 .mkv .mov .avi .webm .wmv .flv .m4v' }
    @{ Name = 'Audio';                                           Mode = 'files';      Ext = '.mp3 .wav .flac .aac .ogg .m4a .opus .wma' }
    @{ Name = 'Documents';                                       Mode = 'files';      Ext = '.pdf .doc .docx .xls .xlsx .ppt .pptx .odt .ods .rtf .txt .md .csv' }
    @{ Name = 'Code / scripts';                                  Mode = 'files';      Ext = '.py .js .ts .jsx .tsx .java .c .cpp .h .cs .rs .go .php .rb .lua .bat .cmd .ps1 .sh' }
    @{ Name = 'Archives';                                        Mode = 'files';      Ext = '.zip .rar .7z .tar .gz .bz2 .xz .iso' }
    @{ Name = 'Programs / installers';                           Mode = 'files';      Ext = '.exe .msi .msix .appx' }
    @{ Name = 'Git repositories (folders with .git)';            Mode = 'foldername'; Folder = '.git'; Parent = $true }
    @{ Name = 'Big files (over 1 GB)';                           Mode = 'files';      MinSize = 1GB }
)
# 

$OutDir  = if ($env:FS_DIR) { $env:FS_DIR } else { (Get-Location).Path }
$CanDraw = -not [Console]::IsOutputRedirected
$CanKey  = -not [Console]::IsInputRedirected
$Fill    = [string][char]0x2588   # full block
$Empty   = [string][char]0x2592   # shaded block (like winget)
if ($CanDraw) { try { $Host.UI.RawUI.BufferSize = New-Object Management.Automation.Host.Size(120, 9999) } catch {} }

function Section($name) {
    Write-Host ''
    Write-Host ('xxxxxxxxxxxxxx ' + $name + ' ' + ('x' * [Math]::Max(0, 60 - $name.Length))) -ForegroundColor Cyan
}
function Line($label, $value) {
    Write-Host ('  {0,-12} ' -f $label) -NoNewline -ForegroundColor Gray
    Write-Host $value
}
function Size($bytes) {
    if ($null -eq $bytes) { return '-' }
    if ($bytes -ge 1TB) { return '{0:N2} TB' -f ($bytes / 1TB) }
    if ($bytes -ge 1GB) { return '{0:N1} GB' -f ($bytes / 1GB) }
    if ($bytes -ge 1MB) { return '{0:N1} MB' -f ($bytes / 1MB) }
    if ($bytes -ge 1KB) { return '{0:N0} KB' -f ($bytes / 1KB) }
    return "$bytes B"
}
function Fmt-Time($ts) {
    if ($ts.TotalHours -ge 1) { return '{0}:{1:00}:{2:00}' -f [int][Math]::Floor($ts.TotalHours), $ts.Minutes, $ts.Seconds }
    return '{0:00}:{1:00}' -f $ts.Minutes, $ts.Seconds
}
function Ask($prompt) {
    # piped input (e.g. "filescanner.bat < answers.txt"): Read-Host cant read that, Console.In can
    if ($CanKey) { $r = Read-Host $prompt } else { Write-Host ($prompt + ': ') -NoNewline; $r = [Console]::In.ReadLine(); Write-Host $r }
    if ($null -eq $r) { exit 0 }   # input closed
    return $r.Trim()
}
# cuts the middle out of long paths and pads to the width, so the line gets fully overwritten
function Fit([string]$s, [int]$w) {
    if ($w -lt 10) { $w = 10 }
    if ($s.Length -gt $w) {
        $head = [int][Math]::Floor(($w - 3) / 3)
        $s = $s.Substring(0, $head) + '...' + $s.Substring($s.Length - ($w - 3 - $head))
    }
    return $s.PadRight($w)
}
function ConWidth { try { [Math]::Max(40, [Console]::WindowWidth - 1) } catch { 119 } }
function Get-DirTime($path) { try { [IO.Directory]::GetLastWriteTime($path) } catch { $null } }

# plain text -> "contains", text with * or ? is used as wildcard
function To-Pattern([string]$text) {
    if (-not $text) { return '' }
    if ($text -match '[\*\?]') { return $text }
    return '*' + [Management.Automation.WildcardPattern]::Escape($text) + '*'
}
# "html, .css *.js" -> .html .css .js
function New-ExtSet([string]$text) {
    $set = [Collections.Generic.HashSet[string]]::new([StringComparer]::OrdinalIgnoreCase)
    foreach ($t in ($text -split '[\s,;]+')) {
        $t = $t.TrimStart('*')
        if (-not $t) { continue }
        if (-not $t.StartsWith('.')) { $t = '.' + $t }
        [void]$set.Add($t)
    }
    return ,$set
}
function Md-Code([string]$s) {
    $s = $s.Replace('|', '\|')
    if ($s.Contains('`')) { return '`` ' + $s + ' ``' }
    return '`' + $s + '`'
}
function Describe($q) {
    $parts = @()
    if ($q.NamePattern) { $parts += "name like '$($q.NamePattern)'" }
    if ($q.Ext.Count)   { $parts += 'extension ' + (@($q.Ext) -join ' ') }
    if ($q.MinSize)     { $parts += 'size >= ' + (Size $q.MinSize) }
    $s = $parts -join ', '
    switch ($q.Mode) {
        'folders'    { return 'folders containing files with ' + $s }
        'foldername' { if ($q.Parent) { return "folders that contain a folder with $s" } return 'folders with ' + $s }
        default      { return 'files with ' + $s }
    }
}

# status lines (path + turquoise bar)
function Start-Status {
    $script:StatusRow = -1
    if (-not $CanDraw) { return }
    try {
        Write-Host ''
        Write-Host ''
        $script:StatusRow = [Console]::CursorTop - 2
        [Console]::CursorVisible = $false
    } catch { $script:StatusRow = -1 }
}
# $pct < 0 = no bar (total unknown)
function Draw-Status([string]$line1, [double]$pct, [string]$line2) {
    if ($script:StatusRow -lt 0) { return }
    $w = ConWidth
    try {
        [Console]::SetCursorPosition(0, $script:StatusRow)
        Write-Host (Fit ('  ' + $line1) $w) -NoNewline -ForegroundColor Gray
        [Console]::SetCursorPosition(0, $script:StatusRow + 1)
        if ($pct -lt 0) {
            Write-Host (Fit ('  ' + $line2) $w) -NoNewline
        } else {
            $n = [int][Math]::Floor($BarWidth * [Math]::Min(100, $pct) / 100)
            Write-Host '  ' -NoNewline
            Write-Host ($Fill * $n) -NoNewline -ForegroundColor Cyan
            Write-Host ($Empty * ($BarWidth - $n)) -NoNewline -ForegroundColor DarkGray
            Write-Host (Fit (' {0,3}%  {1}' -f [int][Math]::Floor($pct), $line2) ($w - $BarWidth - 2)) -NoNewline
        }
    } catch {}
}
function Stop-Status {
    if ($script:StatusRow -lt 0) { return }
    try { [Console]::SetCursorPosition(0, $script:StatusRow + 2); [Console]::CursorVisible = $true } catch {}
}
function Test-Esc {
    if (-not $CanKey) { return $false }
    try { while ([Console]::KeyAvailable) { if ([Console]::ReadKey($true).Key -eq [ConsoleKey]::Escape) { return $true } } } catch {}
    return $false
}

# menus
function Read-Locations {
    Section 'WHERE'
    $drives = @([IO.DriveInfo]::GetDrives() | Where-Object { $_.IsReady })
    for ($i = 0; $i -lt $drives.Count; $i++) {
        $d = $drives[$i]
        $label = try { $d.VolumeLabel } catch { '' }
        if (-not $label) { $label = '-' }
        Write-Host ('  [{0}] ' -f ($i + 1)) -NoNewline -ForegroundColor Cyan
        Write-Host ('{0,-3} {1,-22} {2,-10} ' -f $d.Name.TrimEnd('\'), $label, $d.DriveType) -NoNewline
        Write-Host ('{0} free of {1}' -f (Size $d.AvailableFreeSpace), (Size $d.TotalSize)) -ForegroundColor DarkGray
    }
    Write-Host ''
    Write-Host '  Drives by number or letter, "all", or a folder path. Combine with a comma.     (q = quit)' -ForegroundColor DarkGray
    Write-Host '  e.g.   1   |   C D   |   all   |   C:\Users\me\Projects   |   D, C:\Users\me\Desktop' -ForegroundColor DarkGray

    while ($true) {
        $in = Ask '  Where'
        if ($in -eq 'q') { exit 0 }
        if (-not $in) { continue }
        $found = [Collections.Generic.List[string]]::new()
        $bad = @()
        if ($in -eq 'all') {
            foreach ($d in $drives) { $found.Add($d.RootDirectory.FullName) }
        } else {
            foreach ($part in ($in -split '[,;]')) {
                $p = $part.Trim().Trim('"').Trim()
                if (-not $p) { continue }
                if ($p -match '^(\d+|[a-z]:?)(\s+(\d+|[a-z]:?))*$') {
                    # list of drive numbers / letters
                    foreach ($t in ($p -split '\s+')) {
                        if ($t -match '^\d+$') {
                            if ([int]$t -ge 1 -and [int]$t -le $drives.Count) { $found.Add($drives[[int]$t - 1].RootDirectory.FullName) } else { $bad += $t }
                        } else {
                            $r = $t.Substring(0, 1).ToUpper() + ':\'
                            if ([IO.Directory]::Exists($r)) { $found.Add($r) } else { $bad += $t }
                        }
                    }
                } else {
                    $p = [Environment]::ExpandEnvironmentVariables($p)
                    if ([IO.Directory]::Exists($p)) {
                        $full = [IO.Path]::GetFullPath($p)
                        if ($full.Length -gt 3) { $full = $full.TrimEnd('\') }
                        $found.Add($full)
                    } else { $bad += $p }
                }
            }
        }
        foreach ($b in $bad) { Write-Host "  not found: $b" -ForegroundColor Yellow }
        if ($bad.Count -or -not $found.Count) { continue }

        # drop duplicates + folders that are inside another chosen folder
        $roots = [Collections.Generic.List[string]]::new()
        foreach ($f in ($found | Sort-Object Length)) {
            $inside = $false
            foreach ($r in $roots) {
                if (($f.TrimEnd('\') + '\').StartsWith($r.TrimEnd('\') + '\', [StringComparison]::OrdinalIgnoreCase)) { $inside = $true; break }
            }
            if (-not $inside) { $roots.Add($f) }
        }
        return ,$roots
    }
}

function Read-Query {
    Section 'WHAT'
    for ($i = 0; $i -lt $Presets.Count; $i++) {
        $p = $Presets[$i]
        if ($i -eq 3) { Write-Host '  ---- quick presets ----' -ForegroundColor DarkGray }
        $hint = if ($p.Ext) { $p.Ext } elseif ($p.Folder) { $p.Folder } else { '' }
        if ($hint.Length -gt 62) { $hint = $hint.Substring(0, 59) + '...' }
        Write-Host ('  [{0,2}] ' -f ($i + 1)) -NoNewline -ForegroundColor Cyan
        Write-Host ('{0,-50}' -f $p.Name) -NoNewline
        Write-Host $hint -ForegroundColor DarkGray
    }
    while ($true) {
        $sel = Ask '  Preset'
        if ($sel -match '^\d+$' -and [int]$sel -ge 1 -and [int]$sel -le $Presets.Count) { break }
    }
    $p = $Presets[[int]$sel - 1]
    $q = @{ Preset = $p.Name; Mode = $p.Mode; Ext = (New-ExtSet $p.Ext); NamePattern = [string]$p.Folder; MinSize = [long]$p.MinSize; Parent = [bool]$p.Parent }

    switch ($p.Ask) {
        'name+ext' {
            Write-Host ''
            Write-Host '  Fill in one or both. Plain text = name contains it, wildcards * ? work too.' -ForegroundColor DarkGray
            while ($true) {
                $n = Ask '  File name   (empty = any)'
                $e = Ask '  Extensions  (e.g. html css, empty = any)'
                if ($n -or $e) { break }
                Write-Host '  Enter a name or at least one extension.' -ForegroundColor Yellow
            }
            $q.NamePattern = To-Pattern $n
            $q.Ext = New-ExtSet $e
        }
        'foldername' {
            Write-Host ''
            Write-Host '  Plain text = folder name contains it, wildcards * ? work too.' -ForegroundColor DarkGray
            do { $n = Ask '  Folder name' } until ($n)
            $q.NamePattern = To-Pattern $n
        }
    }
    return $q
}

# phase 1: all folders (needed for a real %)
function Get-FolderList($roots) {
    $list  = [Collections.Generic.List[string]]::new()
    $stack = [Collections.Generic.Stack[IO.DirectoryInfo]]::new()
    $subs  = [Collections.Generic.List[IO.DirectoryInfo]]::new()
    $skip  = [Collections.Generic.HashSet[string]]::new([string[]]$SkipFolders, [StringComparer]::OrdinalIgnoreCase)
    for ($i = $roots.Count - 1; $i -ge 0; $i--) { $stack.Push([IO.DirectoryInfo]::new($roots[$i])) }

    $sw = [Diagnostics.Stopwatch]::StartNew(); $last = -1000; $spin = '|/-\'; $n = 0
    while ($stack.Count) {
        $d = $stack.Pop()
        $list.Add($d.FullName)
        $subs.Clear()
        try {
            foreach ($sub in $d.EnumerateDirectories()) {
                # junctions / symlinks (e.g. "Application Data") would loop or scan stuff twice, OneDrive folders are fine
                if ([int]$sub.Attributes -band 1024) {
                    $lt = $sub.LinkType
                    if ($lt -eq 'Junction' -or $lt -eq 'SymbolicLink') { continue }
                }
                if ($skip.Contains($sub.Name)) { continue }
                $subs.Add($sub)
            }
        } catch { $script:Skipped++ }
        for ($j = $subs.Count - 1; $j -ge 0; $j--) { $stack.Push($subs[$j]) }   # reversed -> alphabetical order

        if ($sw.ElapsedMilliseconds - $last -ge 80) {
            $last = $sw.ElapsedMilliseconds; $n++
            Draw-Status ('Indexing: ' + $d.FullName) -1 ('{0}  {1:N0} folders found   {2}' -f $spin[$n % 4], $list.Count, (Fmt-Time $sw.Elapsed))
            if (Test-Esc) { $script:Aborted = $true; break }
        }
    }
    Draw-Status 'Indexing: done' -1 ('{0:N0} folders found   {1}' -f $list.Count, (Fmt-Time $sw.Elapsed))
    return ,$list
}

# phase 2: scan every folder
function Invoke-Scan($folders, $q) {
    $hits    = [Collections.Generic.List[object]]::new()
    $total   = $folders.Count
    $extSet  = $q.Ext;         $useExt  = $extSet.Count -gt 0
    $pat     = $q.NamePattern; $useName = [bool]$pat
    $min     = [long]$q.MinSize
    $byName  = $q.Mode -eq 'foldername'
    $asFiles = $q.Mode -eq 'files'

    $sw = [Diagnostics.Stopwatch]::StartNew(); $last = -1000
    for ($i = 0; $i -lt $total; $i++) {
        $dir = $folders[$i]
        if ($byName) {
            $leaf = [IO.Path]::GetFileName($dir)
            if ($leaf -and $leaf -like $pat) {
                $target = if ($q.Parent) { [IO.Path]::GetDirectoryName($dir) } else { $dir }
                $hits.Add([pscustomobject]@{ Folder = $target; Files = $null; Examples = ''; Modified = (Get-DirTime $target) })
            }
        } else {
            $count = 0; $examples = ''
            try {
                foreach ($f in [IO.DirectoryInfo]::new($dir).EnumerateFiles()) {
                    if ($useExt -and -not $extSet.Contains($f.Extension)) { continue }
                    if ($useName -and $f.Name -notlike $pat) { continue }
                    if ($min -and $f.Length -lt $min) { continue }
                    if ($asFiles) {
                        $hits.Add([pscustomobject]@{ Name = $f.Name; Folder = $dir; Size = $f.Length; Modified = $f.LastWriteTime })
                    } else {
                        $count++
                        if ($count -le 3) { $examples = if ($examples) { $examples + ', ' + $f.Name } else { $f.Name } }
                    }
                }
            } catch {}
            if ($count) { $hits.Add([pscustomobject]@{ Folder = $dir; Files = $count; Examples = $examples; Modified = (Get-DirTime $dir) }) }
        }

        if ($sw.ElapsedMilliseconds - $last -ge 80 -or $i -eq $total - 1) {
            $last = $sw.ElapsedMilliseconds
            $done = $i + 1
            $left = if ($done -lt $total) { '~' + (Fmt-Time ([TimeSpan]::FromTicks([long]($sw.Elapsed.Ticks / $done * ($total - $done))))) + ' left' } else { 'done' }
            Draw-Status ('Scanning: ' + $dir) ($done * 100.0 / $total) ('{0:N0} / {1:N0} folders   {2:N0} hits   {3}   {4}' -f $done, $total, $hits.Count, (Fmt-Time $sw.Elapsed), $left)
            if (Test-Esc) { $script:Aborted = $true; break }
        }
    }
    return ,$hits
}

# output
function Show-Results($hits, $q) {
    Section ('RESULTS ({0:N0})' -f $hits.Count)
    if (-not $hits.Count) { Write-Host '  Nothing found.' -ForegroundColor Yellow; return }
    $lastFolder = $null; $n = 0
    foreach ($h in $hits) {
        if ($n -ge $ConsoleLimit) {
            Write-Host ''
            Write-Host ('  ... {0:N0} more - see the .md file for all results' -f ($hits.Count - $n)) -ForegroundColor Yellow
            break
        }
        $n++
        if ($q.Mode -eq 'files') {
            if ($h.Folder -ne $lastFolder) { Write-Host ''; Write-Host ('  ' + $h.Folder) -ForegroundColor DarkGray; $lastFolder = $h.Folder }
            Write-Host ('    {0,-56} ' -f $h.Name) -NoNewline -ForegroundColor Cyan
            Write-Host ('{0,10}   {1:yyyy-MM-dd HH:mm}' -f (Size $h.Size), $h.Modified) -ForegroundColor Gray
        } else {
            Write-Host ('  ' + $h.Folder) -NoNewline -ForegroundColor Cyan
            if ($h.Files) { Write-Host ('   {0} file(s): {1}' -f $h.Files, $h.Examples) -ForegroundColor DarkGray } else { Write-Host '' }
        }
    }
}

function Save-Markdown($hits, $q, $roots, $info) {
    $base = Join-Path $OutDir ('filescan_{0:yyyy-MM-dd_HH-mm-ss}' -f $info.Start)
    $file = $base + '.md'
    for ($k = 2; [IO.File]::Exists($file); $k++) { $file = "${base}_$k.md" }
    $L =[Collections.Generic.List[string]]::new()
    $L.Add('# Filescan')
    $L.Add('')
    $L.Add('| | |')
    $L.Add('|---|---|')
    $L.Add('| Date | {0:yyyy-MM-dd HH:mm:ss} |' -f $info.Start)
    $L.Add('| Locations | ' + ((@($roots) | ForEach-Object { Md-Code $_ }) -join ', ') + ' |')
    $L.Add('| Preset | ' + $q.Preset + ' |')
    $L.Add('| Search | ' + (Md-Code (Describe $q)) + ' |')
    $L.Add(('| Folders scanned | {0:N0} ({1:N0} not accessible) |' -f $info.Folders, $info.Skipped))
    $L.Add('| Duration | ' + (Fmt-Time $info.Duration) + ' |')
    $L.Add('| Results | {0:N0} |' -f $hits.Count)
    if ($info.Aborted) { $L.Add(''); $L.Add('> **Cancelled with ESC** - the results are incomplete.') }
    $L.Add('')
    $L.Add('## Results')
    $L.Add('')
    $i = 0
    if (-not $hits.Count) {
        $L.Add('Nothing found.')
    } elseif ($q.Mode -eq 'files') {
        $L.Add('| # | Name | Size | Modified | Folder |')
        $L.Add('|--:|---|--:|---|---|')
        foreach ($h in $hits) {
            $i++
            $L.Add(('| {0} | {1} | {2} | {3:yyyy-MM-dd HH:mm} | {4} |' -f $i, (Md-Code $h.Name), (Size $h.Size), $h.Modified, (Md-Code $h.Folder)))
        }
    } else {
        $L.Add('| # | Folder | Files | Examples | Modified |')
        $L.Add('|--:|---|--:|---|---|')
        foreach ($h in $hits) {
            $i++
            $files = if ($null -ne $h.Files) { '{0:N0}' -f $h.Files } else { '-' }
            $ex    = if ($h.Examples) { Md-Code $h.Examples } else { '' }
            $L.Add(('| {0} | {1} | {2} | {3} | {4:yyyy-MM-dd HH:mm} |' -f $i, (Md-Code $h.Folder), $files, $ex, $h.Modified))
        }
    }
    [IO.File]::WriteAllLines($file, $L, (New-Object Text.UTF8Encoding $false))
    return $file
}

# main
Write-Host ''
Write-Host '  FILESCANNER' -ForegroundColor Cyan
Write-Host '  find files by name / extension, or the folders that contain them' -ForegroundColor DarkGray

while ($true) {
    $roots = Read-Locations
    $q     = Read-Query

    Section 'SCAN'
    Line 'Where' ($roots -join ', ')
    Line 'What' (Describe $q)
    Write-Host '  ESC = cancel' -ForegroundColor DarkGray

    $script:Skipped = 0
    $script:Aborted = $false
    $start = Get-Date
    $clock = [Diagnostics.Stopwatch]::StartNew()

    Start-Status
    $folders = Get-FolderList $roots
    Stop-Status
    if ($script:Aborted) { Write-Host '  Cancelled.' -ForegroundColor Yellow; continue }

    Start-Status
    $hits = Invoke-Scan $folders $q
    Stop-Status
    $clock.Stop()

    Show-Results $hits $q
    $info = @{ Start = $start; Duration = $clock.Elapsed; Folders = $folders.Count; Skipped = $script:Skipped; Aborted = $script:Aborted }
    $file = Save-Markdown $hits $q $roots $info

    Section 'DONE'
    if ($script:Aborted) { Write-Host '  Cancelled with ESC - the results are incomplete.' -ForegroundColor Yellow }
    Line 'Results' ('{0:N0}' -f $hits.Count)
    Line 'Folders' ('{0:N0} scanned, {1:N0} not accessible' -f $folders.Count, $script:Skipped)
    Line 'Time' (Fmt-Time $clock.Elapsed)
    Line 'Saved to' $file

    :menu while ($true) {
        Write-Host ''
        Write-Host '  [1] new scan   [2] open .md   [3] show .md in folder   [0] exit' -ForegroundColor Cyan
        switch (Ask '  Choice') {
            '1' { break menu }
            '2' { Start-Process -FilePath $file }
            '3' { Start-Process explorer.exe ('/select,"' + $file + '"') }
            '0' { exit 0 }
        }
    }
}
