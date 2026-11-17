<# :
@echo off
title forza controller fix
rem fixes a PlayStation controller (DS4 / DualSense) that is plugged in but does not work in Forza
rem  1. reads the Steam controller log: did the USB connection drop in the last hour
rem  2. turns USB power saving off in every power plan (also the one pcmax switches to)
rem  3. turns "allow the computer to turn off this device" off for the controller + its USB port
rem  4. reconnects the controller (USB restart) so Windows + Steam pick it up again
rem  5. puts Forza back in front - Steam switches the controller to mouse mode when Forza is not in front
rem needs admin perms
rem "forza-controller-fix.bat check" = only check, change nothing, no admin needed

set "CF_MODE=%~1"
if /i "%CF_MODE%"=="check" goto run

net session >nul 2>&1
if errorlevel 1 (
    echo asking for admin perms..
    powershell -NoProfile -Command "Start-Process -FilePath '%~f0' -Verb RunAs"
    exit /b
)

:run
powershell -NoProfile -ExecutionPolicy Bypass -Command "iex ([IO.File]::ReadAllText('%~f0'))"
exit /b
#>

# ---------------- PowerShell part ----------------
$ErrorActionPreference = 'SilentlyContinue'
$checkOnly   = ($env:CF_MODE -eq 'check')
$interactive = -not [Console]::IsInputRedirected

function Head($t) { Write-Host ''; Write-Host $t -ForegroundColor Cyan }
function Ok($t)   { Write-Host "   [OK] $t" -ForegroundColor Green }
function Bad($t)  { Write-Host "   [!]  $t" -ForegroundColor Yellow }
function Info($t) { Write-Host "        $t" -ForegroundColor Gray }

$USB_SUB = '2a737441-1930-4402-8d77-b2bebba308a3'   # power plan: USB settings
$USB_SS  = '48e6b7a6-50f5-4782-a5d4-53bb8f07e226'   # power plan: USB selective suspend

Write-Host '=== Forza Controller-Fix ===' -ForegroundColor White
if ($checkOnly) { Write-Host '(nur prüfen - es wird nichts geändert)' -ForegroundColor Gray }

# ---- 1. Steam log -----------------------------------------------------------
Head '[1/5] Steam-Log: Verbindungsabbrüche in der letzten Stunde'
$steamPath = (Get-ItemProperty 'HKCU:\Software\Valve\Steam').SteamPath
if (-not $steamPath) { $steamPath = Join-Path ${env:ProgramFiles(x86)} 'Steam' }
$log = Join-Path $steamPath 'logs\controller.txt'
if (Test-Path -LiteralPath $log) {
    $since = (Get-Date).AddHours(-1)
    $inv   = [Globalization.CultureInfo]::InvariantCulture
    $drops = 0; $desk = 0
    foreach ($l in (Get-Content -LiteralPath $log -Tail 5000)) {
        if ($l -notmatch '^\[(\d{4}-\d\d-\d\d \d\d:\d\d:\d\d)\]') { continue }
        if ([datetime]::ParseExact($Matches[1], 'yyyy-MM-dd HH:mm:ss', $inv) -lt $since) { continue }
        if ($l -match 'hid_read failure') { $drops++ }
        if ($l -match 'Queueing activation for controller: \d+ app: 413080') { $desk++ }
    }
    if ($drops -gt 0) {
        Bad "$drops x ist die USB-Verbindung abgerissen (die Lampe bleibt trotzdem an, der Controller hat einen Akku)"
        Info 'Passiert das oft: anderes Kabel + USB-Port hinten am PC (kein Hub) probieren.'
        Info 'Die Micro-USB-Buchse am DS4 leiert gern aus - wackeln testen.'
    } else { Ok 'keine USB-Abbrüche' }
    if ($desk -gt 0) { Info "$desk x hat Steam den Controller auf Maus-Modus geschaltet (immer wenn Forza nicht vorne war)" }
} else { Info 'Steam-Log nicht gefunden - übersprungen' }

# ---- 2. USB power saving in every power plan --------------------------------
Head '[2/5] USB-Energiesparen in allen Energieplänen'
$plans = @(powercfg /list | ForEach-Object {
    if ($_ -match '([0-9a-fA-F]{8}-[0-9a-fA-F-]{27})\s+\((.+?)\)') { [pscustomobject]@{ G = $Matches[1]; N = $Matches[2] } }
})
foreach ($p in $plans) {
    # last two "0x..." lines of the query are the AC and DC value (works in every Windows language)
    $vals = @(powercfg /query $p.G $USB_SUB $USB_SS | Select-String ':\s*0x([0-9a-fA-F]+)\s*$' |
              ForEach-Object { [Convert]::ToInt32($_.Matches[0].Groups[1].Value, 16) })
    $on = ($vals -contains 1)
    if ($checkOnly) {
        if ($on) { Bad "$($p.N): an  (sollte aus sein)" } else { Ok "$($p.N): aus" }
        continue
    }
    powercfg /setacvalueindex $p.G $USB_SUB $USB_SS 0 | Out-Null
    powercfg /setdcvalueindex $p.G $USB_SUB $USB_SS 0 | Out-Null
    if ($on) { Ok "$($p.N): war an -> jetzt aus" } else { Ok "$($p.N): aus" }
}
if (-not $checkOnly) { powercfg /setactive SCHEME_CURRENT | Out-Null }

# ---- 3. find the controller -------------------------------------------------
Head '[3/5] Controller suchen'
$padNames = @{ '05C4' = 'DualShock 4 (v1)'; '09CC' = 'DualShock 4 (v2)'; '0BA0' = 'DualShock 4 Funkadapter'; '0CE6' = 'DualSense (PS5)'; '0DF2' = 'DualSense Edge' }
function PadName($id) {
    if ($id -match 'PID[_&]([0-9A-Fa-f]{4})') { $n = $padNames[$Matches[1].ToUpper()]; if ($n) { return $n } }
    'PlayStation-Controller'
}
$all  = @(Get-PnpDevice -PresentOnly)
$usb  = @($all | Where-Object { $_.InstanceId -match '^USB\\VID_054C&PID_[0-9A-F]{4}\\' })
$bt   = @($all | Where-Object { $_.InstanceId -match '^HID\\\{00001124-' -and $_.InstanceId -match '054C' })
$targets = @()
foreach ($d in $usb) { Ok "$(PadName $d.InstanceId) per USB (Status: $($d.Status))"; $targets += $d.InstanceId }
if (-not $usb) { foreach ($d in $bt) { Ok "$(PadName $d.InstanceId) per Bluetooth (Status: $($d.Status))"; $targets += $d.InstanceId } }
if (-not $targets) {
    Bad 'Kein PlayStation-Controller gefunden!'
    Info 'Kabel abziehen, 5 Sekunden warten, wieder anstecken - am besten hinten am PC, nicht am Hub.'
    Info 'Anderes Kabel testen: viele Handy-Kabel können nur laden, keine Daten.'
}

# ---- 4. device power management off + reconnect -----------------------------
Head '[4/5] Energiesparen am Controller + USB-Port aus, Controller neu verbinden'
if ($checkOnly)          { Info 'übersprungen (nur prüfen)' }
elseif (-not $targets)   { Info 'übersprungen (kein Controller gefunden)' }
else {
    # controller + the USB port / hub it hangs on
    $ids = @()
    foreach ($id in $targets) {
        $ids += $id
        $parent = (Get-PnpDeviceProperty -InstanceId $id -KeyName DEVPKEY_Device_Parent).Data
        if ($parent) { $ids += $parent }
    }
    # = the "allow the computer to turn off this device to save power" tick in Device Manager
    $changed = 0
    foreach ($m in @(Get-CimInstance -Namespace root\wmi -ClassName MSPower_DeviceEnable)) {
        foreach ($id in $ids) {
            if ($m.Enable -and $m.InstanceName.StartsWith($id, [StringComparison]::OrdinalIgnoreCase)) {
                Set-CimInstance -InputObject $m -Property @{ Enable = $false }
                $changed++
            }
        }
    }
    if ($changed -gt 0) { Ok "'Computer kann das Gerät ausschalten' bei $changed Gerät(en) abgeschaltet" }
    else                { Ok "'Computer kann das Gerät ausschalten' ist schon aus" }

    foreach ($id in $targets) {
        pnputil /restart-device "$id" | Out-Null
        switch ($LASTEXITCODE) {
            0       { Ok 'Controller wird neu verbunden ...' }
            3010    { Bad 'Windows will einen Neustart, damit es greift' }
            default { Bad "Neu verbinden ging nicht (Code $LASTEXITCODE) - Kabel einmal ab- und anstecken" }
        }
    }
    $back = $false
    for ($i = 0; $i -lt 20 -and -not $back; $i++) {
        Start-Sleep -Milliseconds 500
        $back = -not ($targets | Where-Object { (Get-PnpDevice -InstanceId $_).Status -ne 'OK' })
    }
    if ($back) { Start-Sleep -Seconds 2; Ok 'Controller ist wieder da' }
    else       { Bad 'Controller meldet sich nicht zurück - Kabel ab- und wieder anstecken' }
}

# ---- tips -------------------------------------------------------------------
Head 'Wichtig'
Info 'Forza beim Spielen vorne lassen: sobald ein anderes Fenster vorne ist (YouTube, Discord, Alt+Tab),'
Info 'steuert der Controller über Steam die Maus statt Forza.'
Info 'Hilft alles nicht: Steam > Rechtsklick Forza > Eigenschaften > Controller >'
Info '"Steam Input deaktivieren" einstellen (oder umgekehrt aktivieren) und Forza neu starten.'

# ---- 5. Forza back in front -------------------------------------------------
Head '[5/5] Forza wieder nach vorne holen'
$fz = Get-Process | Where-Object { $_.ProcessName -match '^forzahorizon\d' -and $_.MainWindowHandle -ne [IntPtr]::Zero } | Select-Object -First 1
if (-not $fz) {
    Info 'Forza läuft gerade nicht.'
} elseif ($checkOnly) {
    Info "$($fz.ProcessName) läuft - im Prüf-Modus wird es nicht nach vorne geholt."
} else {
    Add-Type -TypeDefinition @'
using System; using System.Runtime.InteropServices;
public static class CfFg {
  [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr h);
  [DllImport("user32.dll")] public static extern bool ShowWindow(IntPtr h, int cmd);
  [DllImport("user32.dll")] public static extern bool IsIconic(IntPtr h);
  [DllImport("user32.dll")] public static extern void keybd_event(byte vk, byte scan, uint flags, UIntPtr extra);
}
'@
    Write-Host '        Forza kommt in 5 Sekunden nach vorne (Taste = sofort) ...' -ForegroundColor Gray
    for ($i = 0; $i -lt 50; $i++) {
        if ($interactive -and [Console]::KeyAvailable) { [void][Console]::ReadKey($true); break }
        Start-Sleep -Milliseconds 100
    }
    $h = $fz.MainWindowHandle
    if ([CfFg]::IsIconic($h)) { [void][CfFg]::ShowWindow($h, 9) }   # 9 = restore
    [CfFg]::keybd_event(0x12, 0, 0, [UIntPtr]::Zero)                # tap ALT so Windows allows the focus change
    [CfFg]::keybd_event(0x12, 0, 2, [UIntPtr]::Zero)
    if ([CfFg]::SetForegroundWindow($h)) { Ok "$($fz.ProcessName) ist wieder vorne - viel Spaß!"; Start-Sleep -Seconds 2; return }
    Bad 'Konnte Forza nicht nach vorne holen - einfach selbst anklicken oder Alt+Tab.'
}

if ($interactive) { Write-Host ''; Read-Host 'Enter zum Schließen' | Out-Null }
