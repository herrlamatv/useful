# sysinfo (py version)
# hardware data comes from one pwrshell/cim call (json)

import ctypes
import json
import os
import re
import shutil
import sys
import subprocess
import winreg
from datetime import datetime

os.system("")
os.system("title sysinfo")

CYAN, GRAY, YELLOW, DARK, RESET = "\033[96m", "\033[37m", "\033[93m", "\033[90m", "\033[0m"

KB, MB, GB, TB = 1024, 1024**2, 1024**3, 1024**4


def section(name):
    print()
    print(f"{CYAN}xxxxxxxxxxxxxx {name} {'x' * max(0, 60 - len(name))}{RESET}")


def line(label, value):
    if value is None or str(value).strip() == "":
        value = "-"
    print(f"{GRAY}  {str(label):<22} {RESET}{value}")


def sub(title):
    print()
    print(f"{YELLOW}  {title}{RESET}")


def size(b):
    if not b:
        return "-"
    b = float(b)
    if b >= TB:
        return f"{b / TB:,.2f} TB"
    if b >= GB:
        return f"{b / GB:,.1f} GB"
    return f"{b / MB:,.0f} MB"


def as_list(x):
    # pwrshell turns single item arrays into plain objects
    if x is None:
        return []
    return [i for i in x if i is not None] if isinstance(x, list) else [x]


def date(s, fmt="%Y-%m-%d"):
    return datetime.fromisoformat(s).strftime(fmt) if s else None


def run(cmd):
    try:
        return subprocess.run(cmd, capture_output=True, text=True, encoding="oem", errors="ignore").stdout
    except OSError:
        return ""


def reg_value(path, name, root=winreg.HKEY_LOCAL_MACHINE):
    try:
        with winreg.OpenKey(root, path) as k:
            return winreg.QueryValueEx(k, name)[0]
    except OSError:
        return None


# collect data (one pwrshell call)
PS = r"""
$ErrorActionPreference = 'SilentlyContinue'
[Console]::OutputEncoding = [Text.Encoding]::UTF8
$d = [ordered]@{}
$d.cs   = Get-CimInstance Win32_ComputerSystem | Select-Object Manufacturer, Model, SystemType, HypervisorPresent
$d.os   = Get-CimInstance Win32_OperatingSystem | Select-Object Caption, OSArchitecture, FreePhysicalMemory, TotalVisibleMemorySize,
            @{n='InstallDate'; e={ $_.InstallDate.ToString('s') }}, @{n='LastBoot'; e={ $_.LastBootUpTime.ToString('s') }}
$d.bios = Get-CimInstance Win32_BIOS | Select-Object Manufacturer, SMBIOSBIOSVersion, @{n='ReleaseDate'; e={ $_.ReleaseDate.ToString('s') }}
$d.mb   = Get-CimInstance Win32_BaseBoard | Select-Object Manufacturer, Product, Version
$d.cpu  = @(Get-CimInstance Win32_Processor | Select-Object Name, Manufacturer, SocketDesignation, NumberOfCores, NumberOfLogicalProcessors,
            MaxClockSpeed, LoadPercentage, L2CacheSize, L3CacheSize, VirtualizationFirmwareEnabled, ProcessorId)
$d.perf = Get-CimInstance Win32_PerfFormattedData_Counters_ProcessorInformation -Filter "Name='_Total'" | Select-Object PercentProcessorPerformance, PercentProcessorTime
$d.gpu  = @(Get-CimInstance Win32_VideoController | Select-Object Name, AdapterCompatibility, AdapterRAM, DriverVersion, Status,
            CurrentHorizontalResolution, CurrentVerticalResolution, CurrentRefreshRate, CurrentBitsPerPixel, @{n='DriverDate'; e={ $_.DriverDate.ToString('s') }})
$d.ram  = @(Get-CimInstance Win32_PhysicalMemory | Select-Object Manufacturer, PartNumber, Capacity, Speed, ConfiguredClockSpeed,
            SMBIOSMemoryType, ConfiguredVoltage, SerialNumber, DeviceLocator)
$d.slots = (Get-CimInstance Win32_PhysicalMemoryArray | Measure-Object MemoryDevices -Sum).Sum
$d.secureboot = try { [bool](Confirm-SecureBootUEFI -ErrorAction Stop) } catch { $null }
$d.tpm  = (Get-CimInstance -Namespace root\cimv2\Security\MicrosoftTpm -ClassName Win32_Tpm).SpecVersion
$d.lic  = (Get-CimInstance SoftwareLicensingProduct -Filter "ApplicationID='55c92734-d682-4d71-983e-d6ec3f16059f' AND PartialProductKey IS NOT NULL" | Select-Object -First 1).LicenseStatus
$d.hotfix = Get-CimInstance Win32_QuickFixEngineering | Sort-Object InstalledOn -Descending | Select-Object -First 1 HotFixID, @{n='InstalledOn'; e={ $_.InstalledOn.ToString('s') }}
$d.culture = (Get-Culture).Name
$d.tz = (Get-TimeZone).DisplayName
$d.psver = $PSVersionTable.PSVersion.ToString()
$d.disks = @(Get-PhysicalDisk | Sort-Object DeviceId | Select-Object FriendlyName, Size,
            @{n='BusType'; e={ "$($_.BusType)" }}, @{n='MediaType'; e={ "$($_.MediaType)" }}, @{n='Health'; e={ "$($_.HealthStatus)" }})
$d.vols = @(Get-CimInstance Win32_LogicalDisk -Filter 'DriveType=3' | Select-Object DeviceID, VolumeName, Size, FreeSpace, FileSystem)
$d.monitors = @(Get-CimInstance -Namespace root\wmi -ClassName WmiMonitorID | Select-Object UserFriendlyName, ManufacturerName, YearOfManufacture)
$d.net = @(Get-NetAdapter | Where-Object Status -eq 'Up' | ForEach-Object {
    $ip = Get-NetIPConfiguration -InterfaceIndex $_.ifIndex
    [pscustomobject]@{
        Name = $_.Name; Desc = $_.InterfaceDescription; Speed = $_.LinkSpeed; Mac = $_.MacAddress
        IPv4 = @($ip.IPv4Address.IPAddress); Gateway = @($ip.IPv4DefaultGateway.NextHop)
        DNS = @(($ip.DNSServer | Where-Object AddressFamily -eq 2).ServerAddresses)
    } })
$d.audio = @(Get-CimInstance Win32_SoundDevice | Select-Object Manufacturer, Name)
$d.battery = Get-CimInstance Win32_Battery | Select-Object EstimatedChargeRemaining, BatteryStatus
$d | ConvertTo-Json -Depth 5 -Compress
"""

print()
print(f"{DARK}  Collecting system info...{RESET}")
out = subprocess.run(["powershell", "-NoProfile", "-ExecutionPolicy", "Bypass", "-Command", PS],
                     capture_output=True, text=True, encoding="utf-8", errors="ignore").stdout
d = json.loads(out)

cs, osi, bios, mb = d["cs"] or {}, d["os"] or {}, d["bios"] or {}, d["mb"] or {}
winver = r"SOFTWARE\Microsoft\Windows NT\CurrentVersion"

# sys 
section("SYSTEM")
line("Computer name", os.environ.get("COMPUTERNAME"))
line("User", f"{os.environ.get('USERDOMAIN')}\\{os.environ.get('USERNAME')}")
line("Manufacturer", cs.get("Manufacturer"))
line("Model", cs.get("Model"))
line("System type", cs.get("SystemType"))
line("Mainboard", f"{mb.get('Manufacturer')} {mb.get('Product')} (Rev. {mb.get('Version')})")
line("BIOS", f"{bios.get('Manufacturer')} {bios.get('SMBIOSBIOSVersion')}")
line("BIOS date", date(bios.get("ReleaseDate")))
fw = ctypes.c_uint(0)
ctypes.windll.kernel32.GetFirmwareType(ctypes.byref(fw))
line("Boot mode", {1: "Legacy", 2: "UEFI"}.get(fw.value))
sb = d["secureboot"]
line("Secure Boot", "unknown (needs admin)" if sb is None else ("On" if sb else "Off"))
line("TPM", f"Version {d['tpm'].split(',')[0]}" if d["tpm"] else "not found / needs admin")

# win
section("WINDOWS")
line("Edition", osi.get("Caption"))
line("Version", reg_value(winver, "DisplayVersion"))
line("Build", f"{reg_value(winver, 'CurrentBuild')}.{reg_value(winver, 'UBR')}")
line("Architecture", osi.get("OSArchitecture"))
line("Language", d["culture"])
line("Timezone", d["tz"])
line("Installed on", date(osi.get("InstallDate"), "%Y-%m-%d %H:%M"))
line("Last boot", date(osi.get("LastBoot"), "%Y-%m-%d %H:%M"))
if osi.get("LastBoot"):
    up = datetime.now() - datetime.fromisoformat(osi["LastBoot"])
    h, rest = divmod(up.seconds, 3600)
    line("Uptime", f"{up.days}d {h}h {rest // 60}m {rest % 60}s")
lic = d["lic"]
line("Activated", "Yes" if lic == 1 else ("No" if lic is not None else "unknown"))
hf = d["hotfix"]
line("Last update", f"{hf['HotFixID']} ({date(hf['InstalledOn'])})" if hf else None)
line("PowerShell", d["psver"])
line("Python", f"{sys.version.split()[0]}")
line("Processes", len([l for l in run(["tasklist", "/fo", "csv", "/nh"]).splitlines() if l.strip()]))
plan = re.search(r"\((.*)\)", run(["powercfg", "/getactivescheme"]))
line("Power plan", plan.group(1) if plan else None)

# cpu
perf = d["perf"] or {}
for c in as_list(d["cpu"]):
    section("CPU")
    line("Model", (c.get("Name") or "").strip())
    line("Manufacturer", c.get("Manufacturer"))
    line("Socket", c.get("SocketDesignation"))
    line("Cores / Threads", f"{c.get('NumberOfCores')} cores / {c.get('NumberOfLogicalProcessors')} threads")
    line("Base clock", f"{c.get('MaxClockSpeed')} MHz")
    if perf.get("PercentProcessorPerformance"):
        line("Current clock", f"~{c['MaxClockSpeed'] * perf['PercentProcessorPerformance'] / 100:,.0f} MHz")
    load = perf.get("PercentProcessorTime", c.get("LoadPercentage"))
    line("Load", f"{load} %")
    line("L2 cache", f"{c.get('L2CacheSize') or 0:,} KB")
    line("L3 cache", f"{(c.get('L3CacheSize') or 0) / 1024:,.0f} MB")
    if c.get("VirtualizationFirmwareEnabled"):
        virt = "Enabled"
    elif cs.get("HypervisorPresent"):
        virt = "Enabled (Hypervisor running)"
    else:
        virt = "Disabled"
    line("Virtualization", virt)
    line("ID", c.get("ProcessorId"))


# gpu
# real VRAM is in the registry (WMI AdapterRAM caps at 4 GB)
def gpu_vram_from_registry():
    result = {}
    base = r"SYSTEM\ControlSet001\Control\Class\{4d36e968-e325-11ce-bfc1-08002be10318}"
    try:
        with winreg.OpenKey(winreg.HKEY_LOCAL_MACHINE, base) as k:
            i = 0
            while True:
                try:
                    subkey = winreg.EnumKey(k, i)
                except OSError:
                    break
                i += 1
                desc = reg_value(f"{base}\\{subkey}", "DriverDesc")
                if not desc or desc in result:
                    continue
                mem = reg_value(f"{base}\\{subkey}", "HardwareInformation.qwMemorySize")
                if not mem:
                    mem = reg_value(f"{base}\\{subkey}", "HardwareInformation.MemorySize")
                    if isinstance(mem, bytes):
                        mem = int.from_bytes(mem[:4], "little")
                result[desc] = mem
    except OSError:
        pass
    return result


vram_reg = gpu_vram_from_registry()
for g in as_list(d["gpu"]):
    section("GPU")
    line("Model", g.get("Name"))
    line("Vendor", g.get("AdapterCompatibility"))
    line("VRAM", size(vram_reg.get(g.get("Name")) or g.get("AdapterRAM")))
    line("Driver version", g.get("DriverVersion"))
    line("Driver date", date(g.get("DriverDate")))
    if g.get("CurrentHorizontalResolution"):
        line("Resolution", f"{g['CurrentHorizontalResolution']} x {g['CurrentVerticalResolution']} @ {g['CurrentRefreshRate']} Hz")
        line("Color depth", f"{g['CurrentBitsPerPixel']} bit")
    line("Status", g.get("Status"))

# live nvidia data (the guys who pooped on us when AI became bigger)
if shutil.which("nvidia-smi"):
    q = run(["nvidia-smi",
             "--query-gpu=name,temperature.gpu,utilization.gpu,memory.used,memory.total,power.draw,power.limit,"
             "clocks.gr,clocks.mem,fan.speed,pcie.link.gen.current,pcie.link.width.current",
             "--format=csv,noheader,nounits"])
    for row in q.strip().splitlines():
        v = [x.strip() for x in row.split(",")]
        if len(v) < 12:
            continue
        section(f"NVIDIA LIVE ({v[0]})")
        line("Temperature", f"{v[1]} C")
        line("Usage", f"{v[2]} %")
        line("VRAM used", f"{v[3]} / {v[4]} MB")
        line("Power", f"{v[5]} W / {v[6]} W")
        line("Core clock", f"{v[7]} MHz")
        line("Memory clock", f"{v[8]} MHz")
        line("Fan", f"{v[9]} %")
        line("PCIe", f"Gen {v[10]} x{v[11]}")

# random acces mem
section("RAM")
MEM_TYPES = {20: "DDR", 21: "DDR2", 24: "DDR3", 26: "DDR4", 34: "DDR5", 35: "LPDDR5"}
# JEDEC vendor codes -> brand
VENDORS = {"859B": "Corsair", "029E": "Corsair", "9E": "Corsair", "04CD": "G.Skill", "CD04": "G.Skill",
           "0198": "Kingston", "9801": "Kingston", "00CE": "Samsung", "CE00": "Samsung", "80CE": "Samsung",
           "00AD": "SK Hynix", "AD00": "SK Hynix", "80AD": "SK Hynix", "002C": "Micron", "2C00": "Micron", "802C": "Micron",
           "859B0000": "Corsair", "04EF": "TeamGroup", "EF04": "TeamGroup", "0A45": "Patriot", "8551": "Crucial",
           "0983": "ADATA", "04CB": "ADATA"}
# brand from the part number prefix
PART_PREFIX = [(r"^CM", "Corsair"), (r"^F[345]-", "G.Skill"), (r"^(KF|KHX|KVR)", "Kingston"), (r"^(BL|CT)", "Crucial"),
               (r"^(TF|TD|FF)", "TeamGroup"), (r"^PV", "Patriot"), (r"^(AX|AD)", "ADATA"), (r"^M[34]", "Samsung"),
               (r"^HMA|^HMCG", "SK Hynix")]

ram = as_list(d["ram"])
total_vis = (osi.get("TotalVisibleMemorySize") or 0) * KB
free = (osi.get("FreePhysicalMemory") or 0) * KB
line("Total", size(sum(int(m.get("Capacity") or 0) for m in ram)))
if total_vis:
    line("In use", f"{size(total_vis - free)} of {size(total_vis)} ({100 - free / total_vis * 100:.0f} %)")
line("Slots used", f"{len(ram)} of {d['slots']}")
for i, m in enumerate(ram, 1):
    part = (m.get("PartNumber") or "").strip()
    brand = (m.get("Manufacturer") or "").strip()
    brand = VENDORS.get(brand.upper(), brand)
    if re.match(r"^[0-9A-F]+$|^Unknown$|^$", brand):
        for pattern, name in PART_PREFIX:
            if re.match(pattern, part):
                brand = name
                break
    # CL is not stored in WMI, but most part numbers contain it (e.g. CMK32GX5M2B6000C30 / ...6000Z30 (EXPO) -> CL30)
    cl = re.search(r"\d{3,4}(?:CL|C|J|Z)(\d{2})", part)
    sub(f"Stick {i} ({m.get('DeviceLocator')})")
    line("Brand", brand)
    line("Part number", part)
    line("Size", size(m.get("Capacity")))
    line("Type", MEM_TYPES.get(m.get("SMBIOSMemoryType")))
    line("Speed", f"{m.get('ConfiguredClockSpeed')} MT/s (rated {m.get('Speed')} MT/s)")
    line("Timing", f"CL{cl.group(1)} (from part number)" if cl else "unknown")
    line("Voltage", f"{m['ConfiguredVoltage'] / 1000:.2f} V" if m.get("ConfiguredVoltage") else None)
    line("Serial", m.get("SerialNumber"))

# disks
section("DRIVES")
for dk in as_list(d["disks"]):
    line(dk.get("FriendlyName"), f"{size(dk.get('Size'))}, {dk.get('BusType')} {dk.get('MediaType')}, Health: {dk.get('Health')}")

section("PARTITIONS")
for v in as_list(d["vols"]):
    total, vfree = v.get("Size") or 0, v.get("FreeSpace") or 0
    pct = 100 - vfree / total * 100 if total else 0
    bar = ("#" * int(pct / 5)).ljust(20, ".")
    line(f"{v.get('DeviceID')} {v.get('VolumeName') or ''}",
         f"[{bar}] {size(vfree)} free of {size(total)} ({pct:.0f} % used, {v.get('FileSystem')})")

# monitors
section("MONITORS")
for mon in as_list(d["monitors"]):
    name = "".join(chr(c) for c in as_list(mon.get("UserFriendlyName")) if c)
    man = "".join(chr(c) for c in as_list(mon.get("ManufacturerName")) if c)
    line(name or "Monitor", f"Manufacturer: {man}, built {mon.get('YearOfManufacture')}")

# net
section("NETWORK")
for n in as_list(d["net"]):
    sub(n.get("Name"))
    line("Adapter", n.get("Desc"))
    line("Speed", n.get("Speed"))
    line("MAC", n.get("Mac"))
    line("IPv4", ", ".join(as_list(n.get("IPv4"))))
    line("Gateway", ", ".join(as_list(n.get("Gateway"))))
    line("DNS", ", ".join(as_list(n.get("DNS"))))
ssid = re.search(r"^\s+SSID\s+:\s*(.+)$", run(["netsh", "wlan", "show", "interfaces"]), re.M)
if ssid:
    line("WiFi SSID", ssid.group(1).strip())

# audio
section("AUDIO")
for a in as_list(d["audio"]):
    line(a.get("Manufacturer"), a.get("Name"))

# battery
bat = d["battery"]
if bat:
    section("BATTERY")
    line("Charge", f"{bat.get('EstimatedChargeRemaining')} %")
    line("Status", "Charging / plugged in" if bat.get("BatteryStatus") == 2 else "On battery")

print()
input("Press Enter to close...")
