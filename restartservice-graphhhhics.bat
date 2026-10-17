@echo off
title restart graphics driver
rem restarts the graphics driver, same as gui control shift b

echo restarting graphics driver ...

:: the batch basicly only starts ps
powershell -NoProfile -ExecutionPolicy Bypass -Command "$s=Get-Content -LiteralPath '%~f0' -Raw; iex $s.Substring($s.IndexOf('#PS'+'START'))"

echo Done - graphics driver restarted
timeout /t 2 /nobreak >nul
exit /b

#PSSTART
Add-Type -Namespace Win32 -Name Keys -MemberDefinition @'
[DllImport("user32.dll")]
public static extern void keybd_event(byte key, byte scan, uint flags, UIntPtr extra);
'@

$WIN, $CTRL, $SHIFT, $B = 0x5B, 0x11, 0x10, 0x42
$KEYUP = 2

# gui ctrl shift b
foreach ($k in $WIN, $CTRL, $SHIFT, $B) { [Win32.Keys]::keybd_event($k, 0, 0, [UIntPtr]::Zero) }
Start-Sleep -Milliseconds 100
foreach ($k in $B, $SHIFT, $CTRL, $WIN) { [Win32.Keys]::keybd_event($k, 0, $KEYUP, [UIntPtr]::Zero) }
