@echo off
title sysinfo
mode con cols=120 lines=50 >nul 2>&1

:: starts pwrshell + runs the script inthere
powershell -NoProfile -ExecutionPolicy Bypass -Command "$s=Get-Content -LiteralPath '%~f0' -Raw; iex $s.Substring($s.IndexOf('#PS'+'START'))"

echo.
pause
exit /b

#PSSTART
$ErrorActionPreference = 'SilentlyContinue'
try { $Host.UI.RawUI.BufferSize = New-Object Management.Automation.Host.Size(120, 3000) } catch {}

function Section($name) {
    Write-Host ''
    Write-Host ('xxxxxxxxxxxxxx ' + $name + ' ' + ('x' * [Math]::Max(0, 60 - $name.Length))) -ForegroundColor Cyan
}
function Line($label, $value) {
    if ($null -eq $value -or "$value".Trim() -eq '') { $value = '-' }
    Write-Host ('  {0,-22} ' -f $label) -NoNewline -ForegroundColor Gray
    Write-Host $value
}
function Size($bytes) {
    if (-not $bytes) { return '-' }
    if ($bytes -ge 1TB) { return '{0:N2} TB' -f ($bytes / 1TB) }
    if ($bytes -ge 1GB) { return '{0:N1} GB' -f ($bytes / 1GB) }
    return '{0:N0} MB' -f ($bytes / 1MB)
}

Write-Host ''
Write-Host '  Collecting system info...' -ForegroundColor DarkGray

$cs   = Get-CimInstance Win32_ComputerSystem
$os   = Get-CimInstance Win32_OperatingSystem
$bios = Get-CimInstance Win32_BIOS
$mb   = Get-CimInstance Win32_BaseBoard
$cpus = Get-CimInstance Win32_Processor
$gpus = Get-CimInstance Win32_VideoController
$ram  = Get-CimInstance Win32_PhysicalMemory
$reg  = Get-ItemProperty 'HKLM:\SOFTWARE\Microsoft\Windows NT\CurrentVersion'

# sys
Section 'SYSTEM'
Line 'Computer name' $env:COMPUTERNAME
Line 'User' "$env:USERDOMAIN\$env:USERNAME"
Line 'Manufacturer' $cs.Manufacturer
Line 'Model' $cs.Model
Line 'System type' $cs.SystemType
Line 'Mainboard' ("{0} {1} (Rev. {2})" -f $mb.Manufacturer, $mb.Product, $mb.Version)
Line 'BIOS' ("{0} {1}" -f $bios.Manufacturer, $bios.SMBIOSBIOSVersion)
Line 'BIOS date' $(if ($bios.ReleaseDate) { $bios.ReleaseDate.ToString('yyyy-MM-dd') })
Line 'Boot mode' $env:firmware_type
$sb = try { if (Confirm-SecureBootUEFI -ErrorAction Stop) { 'On' } else { 'Off' } } catch { 'unknown (needs admin)' }
Line 'Secure Boot' $sb
$tpm = Get-CimInstance -Namespace root\cimv2\Security\MicrosoftTpm -ClassName Win32_Tpm
Line 'TPM' $(if ($tpm) { "Version " + ($tpm.SpecVersion -split ',')[0] } else { 'not found / needs admin' })

# win
Section 'WINDOWS'
Line 'Edition' $os.Caption
Line 'Version' $reg.DisplayVersion
Line 'Build' ("{0}.{1}" -f $reg.CurrentBuild, $reg.UBR)
Line 'Architecture' $os.OSArchitecture
Line 'Language' (Get-Culture).Name
Line 'Timezone' (Get-TimeZone).DisplayName
Line 'Installed on' $os.InstallDate.ToString('yyyy-MM-dd HH:mm')
Line 'Last boot' $os.LastBootUpTime.ToString('yyyy-MM-dd HH:mm')
$up = (Get-Date) - $os.LastBootUpTime
Line 'Uptime' ("{0}d {1}h {2}m {3}s" -f $up.Days, $up.Hours, $up.Minutes, $up.Seconds)
$lic = Get-CimInstance SoftwareLicensingProduct -Filter "ApplicationID='55c92734-d682-4d71-983e-d6ec3f16059f' AND PartialProductKey IS NOT NULL" | Select-Object -First 1
Line 'Activated' $(if ($lic.LicenseStatus -eq 1) { 'Yes' } elseif ($lic) { 'No' } else { 'unknown' })
$hf = Get-CimInstance Win32_QuickFixEngineering | Sort-Object InstalledOn -Descending | Select-Object -First 1
Line 'Last update' $(if ($hf) { "{0} ({1})" -f $hf.HotFixID, $hf.InstalledOn.ToString('yyyy-MM-dd') })
Line 'PowerShell' $PSVersionTable.PSVersion.ToString()
Line 'Processes' (Get-Process).Count
Line 'Power plan' ((powercfg /getactivescheme) -replace '.*\((.*)\).*', '$1')

# cpu
foreach ($c in $cpus) {
    Section 'CPU'
    Line 'Model' $c.Name.Trim()
    Line 'Manufacturer' $c.Manufacturer
    Line 'Socket' $c.SocketDesignation
    Line 'Cores / Threads' ("{0} cores / {1} threads" -f $c.NumberOfCores, $c.NumberOfLogicalProcessors)
    Line 'Base clock' ("{0} MHz" -f $c.MaxClockSpeed)
    # perf data via CIM (Get-Counter names are translated on non-english Windows)
    $pi = Get-CimInstance Win32_PerfFormattedData_Counters_ProcessorInformation -Filter "Name='_Total'"
    if ($pi.PercentProcessorPerformance) { Line 'Current clock' ("~{0:N0} MHz" -f ($c.MaxClockSpeed * $pi.PercentProcessorPerformance / 100)) }
    Line 'Load' ("{0} %" -f $(if ($pi) { $pi.PercentProcessorTime } else { $c.LoadPercentage }))
    Line 'L2 cache' ("{0:N0} KB" -f $c.L2CacheSize)
    Line 'L3 cache' ("{0:N0} MB" -f ($c.L3CacheSize / 1024))
    Line 'Virtualization' $(if ($c.VirtualizationFirmwareEnabled) { 'Enabled' } elseif ($cs.HypervisorPresent) { 'Enabled (Hypervisor running)' } else { 'Disabled' })
    Line 'ID' $c.ProcessorId
}

# gpu
# real VRAM is in the registry (WMI AdapterRAM caps at 4 GB)
$gpuReg = Get-ChildItem 'HKLM:\SYSTEM\ControlSet001\Control\Class\{4d36e968-e325-11ce-bfc1-08002be10318}' |
    ForEach-Object { Get-ItemProperty $_.PSPath } | Where-Object { $_.DriverDesc }
foreach ($g in $gpus) {
    Section 'GPU'
    Line 'Model' $g.Name
    Line 'Vendor' $g.AdapterCompatibility
    $r = $gpuReg | Where-Object { $_.DriverDesc -eq $g.Name } | Select-Object -First 1
    $vram = $r.'HardwareInformation.qwMemorySize'
    if (-not $vram) { $vram = $r.'HardwareInformation.MemorySize'; if ($vram -is [byte[]]) { $vram = [BitConverter]::ToUInt32($vram, 0) } }
    if (-not $vram) { $vram = $g.AdapterRAM }
    Line 'VRAM' (Size $vram)
    Line 'Driver version' $g.DriverVersion
    Line 'Driver date' $(if ($g.DriverDate) { $g.DriverDate.ToString('yyyy-MM-dd') })
    if ($g.CurrentHorizontalResolution) {
        Line 'Resolution' ("{0} x {1} @ {2} Hz" -f $g.CurrentHorizontalResolution, $g.CurrentVerticalResolution, $g.CurrentRefreshRate)
        Line 'Color depth' ("{0} bit" -f $g.CurrentBitsPerPixel)
    }
    Line 'Status' $g.Status
}
# live nvidia data (the guys who pooped on us when AI became bigger)
$smi = Get-Command nvidia-smi -ErrorAction SilentlyContinue
if ($smi) {
    $q = & nvidia-smi --query-gpu=name,temperature.gpu,utilization.gpu,memory.used,memory.total,power.draw,power.limit,clocks.gr,clocks.mem,fan.speed,pcie.link.gen.current,pcie.link.width.current --format=csv,noheader,nounits
    foreach ($row in $q) {
        $v = $row -split ',\s*'
        Section ('NVIDIA LIVE (' + $v[0] + ')')
        Line 'Temperature' ("{0} C" -f $v[1])
        Line 'Usage' ("{0} %" -f $v[2])
        Line 'VRAM used' ("{0} / {1} MB" -f $v[3], $v[4])
        Line 'Power' ("{0} W / {1} W" -f $v[5], $v[6])
        Line 'Core clock' ("{0} MHz" -f $v[7])
        Line 'Memory clock' ("{0} MHz" -f $v[8])
        Line 'Fan' ("{0} %" -f $v[9])
        Line 'PCIe' ("Gen {0} x{1}" -f $v[10], $v[11])
    }
}

# random acces mem
Section 'RAM'
$memTypes = @{ 20 = 'DDR'; 21 = 'DDR2'; 24 = 'DDR3'; 26 = 'DDR4'; 34 = 'DDR5'; 35 = 'LPDDR5' }
# JEDEC vendor codes -> brand
$vendors = @{ '859B' = 'Corsair'; '029E' = 'Corsair'; '9E' = 'Corsair'; '04CD' = 'G.Skill'; 'CD04' = 'G.Skill';
              '0198' = 'Kingston'; '9801' = 'Kingston'; '00CE' = 'Samsung'; 'CE00' = 'Samsung'; '80CE' = 'Samsung';
              '00AD' = 'SK Hynix'; 'AD00' = 'SK Hynix'; '80AD' = 'SK Hynix'; '002C' = 'Micron'; '2C00' = 'Micron'; '802C' = 'Micron';
              '859B0000' = 'Corsair'; '04EF' = 'TeamGroup'; 'EF04' = 'TeamGroup'; '0A45' = 'Patriot'; '8551' = 'Crucial'; '0983' = 'ADATA'; '04CB' = 'ADATA' }
$totalMem = ($ram | Measure-Object Capacity -Sum).Sum
$slots = (Get-CimInstance Win32_PhysicalMemoryArray | Measure-Object MemoryDevices -Sum).Sum
$free = $os.FreePhysicalMemory * 1KB
Line 'Total' (Size $totalMem)
Line 'In use' ("{0} of {1} ({2:N0} %)" -f (Size ($os.TotalVisibleMemorySize * 1KB - $free)), (Size ($os.TotalVisibleMemorySize * 1KB)), (100 - $free / ($os.TotalVisibleMemorySize * 1KB) * 100))
Line 'Slots used' ("{0} of {1}" -f @($ram).Count, $slots)
$i = 0
foreach ($m in $ram) {
    $i++
    $part = "$($m.PartNumber)".Trim()
    $brand = "$($m.Manufacturer)".Trim()
    if ($vendors.ContainsKey($brand.ToUpper())) { $brand = $vendors[$brand.ToUpper()] }
    # guess brand from the part number
    if ($brand -match '^[0-9A-F]+$|^Unknown$|^$') {
        switch -Regex ($part) {
            '^CM'          { $brand = 'Corsair' }
            '^F[345]-'     { $brand = 'G.Skill' }
            '^(KF|KHX|KVR)' { $brand = 'Kingston' }
            '^(BL|CT)'     { $brand = 'Crucial' }
            '^(TF|TD|FF)'  { $brand = 'TeamGroup' }
            '^PV'          { $brand = 'Patriot' }
            '^(AX|AD)'     { $brand = 'ADATA' }
            '^M[34]'       { $brand = 'Samsung' }
            '^HMA|^HMCG'   { $brand = 'SK Hynix' }
        }
    }
    # CL is not stored in WMI, but most part numbers contain it (e.g. CMK32GX5M2B6000C30 / ...6000Z30 (EXPO) -> CL30)
    $cl = if ($part -match '\d{3,4}(?:CL|C|J|Z)(\d{2})') { 'CL' + $matches[1] + ' (from part number)' } else { 'unknown' }
    Write-Host ''
    Write-Host ("  Stick {0} ({1})" -f $i, $m.DeviceLocator) -ForegroundColor Yellow
    Line 'Brand' $brand
    Line 'Part number' $part
    Line 'Size' (Size $m.Capacity)
    Line 'Type' $memTypes[[int]$m.SMBIOSMemoryType]
    Line 'Speed' ("{0} MT/s (rated {1} MT/s)" -f $m.ConfiguredClockSpeed, $m.Speed)
    Line 'Timing' $cl
    Line 'Voltage' $(if ($m.ConfiguredVoltage) { "{0:N2} V" -f ($m.ConfiguredVoltage / 1000) })
    Line 'Serial' $m.SerialNumber
}

# disks
Section 'DRIVES'
foreach ($d in (Get-PhysicalDisk | Sort-Object DeviceId)) {
    Line $d.FriendlyName ("{0}, {1} {2}, Health: {3}" -f (Size $d.Size), $d.BusType, $d.MediaType, $d.HealthStatus)
}
Section 'PARTITIONS'
foreach ($v in (Get-CimInstance Win32_LogicalDisk -Filter 'DriveType=3')) {
    $pct = if ($v.Size) { 100 - $v.FreeSpace / $v.Size * 100 } else { 0 }
    $bar = ('#' * [int]($pct / 5)).PadRight(20, '.')
    Line ("{0} {1}" -f $v.DeviceID, $v.VolumeName) ("[{0}] {1} free of {2} ({3:N0} % used, {4})" -f $bar, (Size $v.FreeSpace), (Size $v.Size), $pct, $v.FileSystem)
}

# monitors
Section 'MONITORS'
foreach ($mon in (Get-CimInstance -Namespace root\wmi -ClassName WmiMonitorID)) {
    $name = -join ($mon.UserFriendlyName | Where-Object { $_ } | ForEach-Object { [char]$_ })
    $man  = -join ($mon.ManufacturerName | Where-Object { $_ } | ForEach-Object { [char]$_ })
    Line $(if ($name) { $name } else { 'Monitor' }) ("Manufacturer: {0}, built {1}" -f $man, $mon.YearOfManufacture)
}

# net
Section 'NETWORK'
foreach ($n in (Get-NetAdapter | Where-Object Status -eq 'Up')) {
    $ip = Get-NetIPConfiguration -InterfaceIndex $n.ifIndex
    Write-Host ''
    Write-Host ("  {0}" -f $n.Name) -ForegroundColor Yellow
    Line 'Adapter' $n.InterfaceDescription
    Line 'Speed' $n.LinkSpeed
    Line 'MAC' $n.MacAddress
    Line 'IPv4' ($ip.IPv4Address.IPAddress -join ', ')
    Line 'Gateway' ($ip.IPv4DefaultGateway.NextHop -join ', ')
    Line 'DNS' (($ip.DNSServer | Where-Object AddressFamily -eq 2).ServerAddresses -join ', ')
}
$wifi = netsh wlan show interfaces | Select-String '^\s+SSID\s+:' | Select-Object -First 1
if ($wifi) { Line 'WiFi SSID' (($wifi -split ':', 2)[1].Trim()) }

# audio
Section 'AUDIO'
foreach ($a in (Get-CimInstance Win32_SoundDevice)) { Line $a.Manufacturer $a.Name }

# battery
$bat = Get-CimInstance Win32_Battery
if ($bat) {
    Section 'BATTERY'
    Line 'Charge' ("{0} %" -f $bat.EstimatedChargeRemaining)
    Line 'Status' $(if ($bat.BatteryStatus -eq 2) { 'Charging / plugged in' } else { 'On battery' })
}

Write-Host ''
