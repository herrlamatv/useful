<#
.SYNOPSIS
    Downloader für den MaleCNS-Konnektom-Datensatz (Janelia FlyEM, v1.0, CC-BY).

.DESCRIPTION
    Wird normalerweise über install.bat gestartet. Lädt die gewählten Komponenten
    anonym über HTTPS von storage.googleapis.com, prüft jede Datei (MD5 bzw. CRC32C
    aus den GCS-Metadaten) und kann nach einem Abbruch einfach erneut gestartet werden.
    Lauffähig mit Windows PowerShell 5.1.

.PARAMETER Ziel
    Zielordner. Ohne Angabe wird gefragt (Standard: %USERPROFILE%\MaleCNS).
.PARAMETER Komponenten
    Auswahl ohne Menü, z. B. "A,B" oder "X" (= A bis D). E nur, wenn ausdrücklich genannt.
.PARAMETER Ja
    Sicherheitsabfrage vor dem Download überspringen.
.PARAMETER NurPruefen
    Nur Größen ermitteln und Übersicht anzeigen, nichts herunterladen.
.PARAMETER Parallel
    Anzahl gleichzeitiger Downloads für kleine Dateien (8 bis 16, Standard 12).
.PARAMETER Neupruefen
    Bereits vorhandene Dateien zusätzlich per Prüfsumme kontrollieren (langsam).
.PARAMETER Filter
    (Test) Nur Dateien, deren relativer Name auf dieses Muster passt, z. B. "body-annotations*".
.PARAMETER Limit
    (Test) Höchstens so viele Dateien pro Quelle.
.PARAMETER Bandbreite
    (Test) Bandbreite begrenzen, z. B. "500K" (wird an curl --limit-rate übergeben).
.PARAMETER EinzelnAbMB
    Dateien ab dieser Größe (MB) einzeln mit Fortschrittsanzeige und Fortsetzen laden (Standard 64).
#>
[CmdletBinding()]
param(
    [string]$Ziel,
    [string]$Komponenten,
    [switch]$Ja,
    [switch]$NurPruefen,
    [int]$Parallel = 12,
    [switch]$Neupruefen,
    [string]$Filter,
    [int]$Limit = 0,
    [string]$Bandbreite,
    [int]$EinzelnAbMB = 64
)

Set-StrictMode -Version 2.0
$ErrorActionPreference = 'Stop'
$ProgressPreference = 'Continue'
[Net.ServicePointManager]::SecurityProtocol = [Net.ServicePointManager]::SecurityProtocol -bor [Net.SecurityProtocolType]::Tls12

# ---------------------------------------------------------------------------
# Konstanten
# ---------------------------------------------------------------------------
$Script:Bucket   = 'flyem-male-cns'
$Script:ApiBase  = 'https://storage.googleapis.com/storage/v1/b/flyem-male-cns/o'
$Script:DlBase   = 'https://storage.googleapis.com/flyem-male-cns/'
$Script:DE       = [Globalization.CultureInfo]::GetCultureInfo('de-DE')
$Script:GrossAb  = [long]$EinzelnAbMB * 1MB   # ab dieser Größe einzeln mit Fortschrittsanzeige
$Script:BatchGr  = 500           # Dateien pro parallelem curl-Aufruf
$Script:MaxVers  = 3             # Versuche pro Datei
$Script:CacheTage = 7            # Dateilisten so lange wiederverwenden
$Script:Log      = $null
$Script:Curl     = $null
$Script:Gcloud   = $null

if ($Parallel -lt 8)  { $Parallel = 8 }
if ($Parallel -gt 16) { $Parallel = 16 }

$FC = 'v1.0/connectome-data/flat-connectome/'
$Script:Komps = [ordered]@{
    A = @{ Key = 'A'; Titel = 'Konnektom-Tabellen (Feather)'; Ordner = 'A_Konnektom-Tabellen'
           Hinweis = '7 Dateien, ca. 24 GB'
           Quellen = @(@{ Typ = 'Dateien'; Unterordner = ''; Prefix = $FC; Objekte = @(
                "${FC}body-annotations-male-cns-v1.0-minconf-0.5.feather",
                "${FC}body-neurotransmitters-male-cns-v1.0.feather",
                "${FC}body-stats-male-cns-v1.0-minconf-0.5.feather",
                "${FC}connectome-weights-male-cns-v1.0-minconf-0.5.feather",
                "${FC}syn-points-male-cns-v1.0-minconf-0.5.feather",
                "${FC}syn-partners-male-cns-v1.0-minconf-0.5.feather",
                "${FC}tbar-neurotransmitters-male-cns-v1.0.feather") }) }
    B = @{ Key = 'B'; Titel = 'Neuronen-Skelette (SWC)'; Ordner = 'B_Skelette-SWC'
           Hinweis = 'ca. 211.600 Dateien, ca. 9 GB'
           Quellen = @(@{ Typ = 'Prefix'; Unterordner = ''; Prefix = 'v1.0/segmentation/skeletons-malecns/skeletons-swc/' }) }
    C = @{ Key = 'C'; Titel = 'neuPrint-Datenbank (neo4j 4.4.16)'; Ordner = 'C_neo4j-Datenbank'
           Hinweis = 'ca. 1.000 Dateien, ca. 252 GB'
           Quellen = @(@{ Typ = 'Prefix'; Unterordner = ''; Prefix = 'v1.0/database/neo4j/' }) }
    D = @{ Key = 'D'; Titel = 'neuPrint-Input-CSVs'; Ordner = 'D_neuPrint-Inputs'
           Hinweis = 'ca. 101.000 Dateien, ca. 194 GB'
           Quellen = @(@{ Typ = 'Prefix'; Unterordner = ''; Prefix = 'v1.0/database/neuprint-inputs/' }) }
    E = @{ Key = 'E'; Titel = 'Weitere Skelett-Varianten (gespiegelt, Unisex-Template)'; Ordner = 'E_Skelett-Varianten'
           Hinweis = 'ca. 635.000 Dateien, ca. 39 GB - nur auf ausdrückliche Wahl'
           Quellen = @(
                @{ Typ = 'Prefix'; Unterordner = 'gespiegelt-swc';  Prefix = 'v1.0/segmentation/skeletons-malecns-mirrored/skeletons-swc/' },
                @{ Typ = 'Prefix'; Unterordner = 'unisex-template'; Prefix = 'v1.0/segmentation/skeletons-unisex-template/' }) }
}

# Nur Objekte unterhalb dieser Präfixe dürfen jemals geladen werden. Alles andere
# (EM-Bilddaten, Segmentierungsvolumen, Kernsegmentierung, ROI-Volumen …) ist
# ausgeschlossen – das sind Hunderte Terabyte.
$Script:Erlaubt = @(
    $FC,
    'v1.0/segmentation/skeletons-malecns/skeletons-swc/',
    'v1.0/database/neo4j/',
    'v1.0/database/neuprint-inputs/',
    'v1.0/segmentation/skeletons-malecns-mirrored/skeletons-swc/',
    'v1.0/segmentation/skeletons-unisex-template/'
)

# ---------------------------------------------------------------------------
# C#-Hilfsklasse: Prüfsummen (MD5, CRC32C wie GCS), Verzeichnis-Index
# ---------------------------------------------------------------------------
if (-not ('McnsUtil' -as [type])) {
    Add-Type -TypeDefinition @'
using System;
using System.IO;
using System.Collections.Generic;
using System.Security.Cryptography;

public class GcsObj {
    public string Name;   // Objektname im Bucket
    public string Rel;    // relativer Pfad im Komponentenordner, mit '/'
    public string Local;  // absoluter lokaler Pfad
    public string Url;
    public long   Size;
    public string Md5;    // base64, kann leer sein (Composite-Objekte)
    public string Crc;    // base64, big endian
}

public static class McnsUtil {
    static uint[][] T;
    static McnsUtil() {
        T = new uint[8][];
        for (int k = 0; k < 8; k++) T[k] = new uint[256];
        for (uint i = 0; i < 256; i++) {
            uint c = i;
            for (int j = 0; j < 8; j++) c = ((c & 1) != 0) ? ((c >> 1) ^ 0x82F63B78u) : (c >> 1);
            T[0][i] = c;
        }
        for (int i = 0; i < 256; i++)
            for (int k = 1; k < 8; k++)
                T[k][i] = (T[k - 1][i] >> 8) ^ T[0][T[k - 1][i] & 0xFF];
    }

    public static string Crc32cB64(string path) {
        uint crc = 0xFFFFFFFFu;
        byte[] buf = new byte[1 << 20];
        using (FileStream fs = new FileStream(path, FileMode.Open, FileAccess.Read, FileShare.Read, 1 << 16, FileOptions.SequentialScan)) {
            int n;
            while ((n = fs.Read(buf, 0, buf.Length)) > 0) {
                int i = 0;
                while (n - i >= 8) {
                    uint a = crc ^ (uint)(buf[i] | (buf[i + 1] << 8) | (buf[i + 2] << 16) | (buf[i + 3] << 24));
                    uint b = (uint)(buf[i + 4] | (buf[i + 5] << 8) | (buf[i + 6] << 16) | (buf[i + 7] << 24));
                    crc = T[7][a & 0xFF] ^ T[6][(a >> 8) & 0xFF] ^ T[5][(a >> 16) & 0xFF] ^ T[4][a >> 24]
                        ^ T[3][b & 0xFF] ^ T[2][(b >> 8) & 0xFF] ^ T[1][(b >> 16) & 0xFF] ^ T[0][b >> 24];
                    i += 8;
                }
                for (; i < n; i++) crc = T[0][(crc ^ buf[i]) & 0xFF] ^ (crc >> 8);
            }
        }
        crc ^= 0xFFFFFFFFu;
        return Convert.ToBase64String(new byte[] { (byte)(crc >> 24), (byte)(crc >> 16), (byte)(crc >> 8), (byte)crc });
    }

    public static string Md5B64(string path) {
        using (MD5 md5 = MD5.Create())
        using (FileStream fs = new FileStream(path, FileMode.Open, FileAccess.Read, FileShare.Read, 1 << 16, FileOptions.SequentialScan)) {
            return Convert.ToBase64String(md5.ComputeHash(fs));
        }
    }

    // Ergebnis: "ok-md5", "ok-crc32c", "ok-groesse", "fehlt", "groesse", "md5", "crc32c"
    public static string Verify(string path, long size, string md5, string crc) {
        FileInfo fi = new FileInfo(path);
        if (!fi.Exists) return "fehlt";
        if (fi.Length != size) return "groesse";
        if (!String.IsNullOrEmpty(md5)) {
            string h = null;
            try { h = Md5B64(path); } catch (InvalidOperationException) { h = null; }  // FIPS-Modus
            if (h != null) return (h == md5) ? "ok-md5" : "md5";
        }
        if (!String.IsNullOrEmpty(crc)) return (Crc32cB64(path) == crc) ? "ok-crc32c" : "crc32c";
        return "ok-groesse";
    }

    public static void Commit(string part, string final) {
        if (File.Exists(final)) File.Delete(final);
        File.Move(part, final);
    }

    // relativer Pfad ('/') -> Größe, ohne .part-Dateien und Steuerdateien
    public static Dictionary<string, long> IndexDir(string root) {
        Dictionary<string, long> d = new Dictionary<string, long>(StringComparer.OrdinalIgnoreCase);
        if (!Directory.Exists(root)) return d;
        string r = Path.GetFullPath(root).TrimEnd('\\') + "\\";
        foreach (string f in Directory.EnumerateFiles(r, "*", SearchOption.AllDirectories)) {
            if (f.EndsWith(".part", StringComparison.OrdinalIgnoreCase)) continue;
            if (f.EndsWith(".curl-batch.cfg", StringComparison.OrdinalIgnoreCase)) continue;
            d[f.Substring(r.Length).Replace('\\', '/')] = new FileInfo(f).Length;
        }
        return d;
    }

    public static long PartBytes(string root) {
        long s = 0;
        if (!Directory.Exists(root)) return 0;
        foreach (string f in Directory.EnumerateFiles(root, "*.part", SearchOption.AllDirectories)) s += new FileInfo(f).Length;
        return s;
    }
}
'@
}

# ---------------------------------------------------------------------------
# Ausgabe und Log
# ---------------------------------------------------------------------------
function Write-Log {
    param([string]$Text, [string]$Art = 'info', [switch]$NurDatei)
    $zeit = (Get-Date).ToString('yyyy-MM-dd HH:mm:ss')
    if ($Script:Log) {
        try { $Script:Log.WriteLine("$zeit [$Art] $Text") } catch { }
    }
    if ($NurDatei) { return }
    switch ($Art) {
        'fehler'  { Write-Host $Text -ForegroundColor Red }
        'warnung' { Write-Host $Text -ForegroundColor Yellow }
        'ok'      { Write-Host $Text -ForegroundColor Green }
        'titel'   { Write-Host ''; Write-Host $Text -ForegroundColor Cyan }
        default   { Write-Host $Text }
    }
}

function Open-Log([string]$Pfad) {
    # Die offene Logdatei dient zugleich als Sperre gegen einen zweiten gleichzeitigen Start.
    try {
        $fs = New-Object IO.FileStream($Pfad, [IO.FileMode]::Append, [IO.FileAccess]::Write, [IO.FileShare]::Read)
    } catch {
        throw (New-Fehler 'laeuft' "Der Downloader läuft für diesen Zielordner offenbar bereits in einem anderen Fenster (Logdatei ist gesperrt). Bitte zuerst das andere Fenster schließen.")
    }
    $Script:Log = New-Object IO.StreamWriter($fs, (New-Object Text.UTF8Encoding $false))
    $Script:Log.AutoFlush = $true
    $Script:Log.WriteLine('')
    Write-Log ('===== Start ' + (Get-Date).ToString('dd.MM.yyyy HH:mm:ss') + ' =====') -NurDatei
}

function Format-Bytes([double]$b) {
    $u = @('Bytes', 'KB', 'MB', 'GB', 'TB')
    $i = 0
    while ($b -ge 1024 -and $i -lt 4) { $b = $b / 1024; $i++ }
    if ($i -eq 0) { return [string]::Format($Script:DE, '{0:N0} Bytes', $b) }
    return [string]::Format($Script:DE, '{0:N1} {1}', $b, $u[$i])
}
function Format-Zahl([double]$n) { return [string]::Format($Script:DE, '{0:N0}', $n) }

function Read-JaNein([string]$Frage, [bool]$Standard = $false) {
    $hinweis = '[j/N]'
    if ($Standard) { $hinweis = '[J/n]' }
    while ($true) {
        $a = Read-Host "$Frage $hinweis"
        if ($null -eq $a) { return $Standard }
        $a = $a.Trim().ToLower()
        if ($a -eq '') { return $Standard }
        if (@('j', 'ja', 'y', 'yes') -contains $a) { return $true }
        if (@('n', 'nein', 'no') -contains $a) { return $false }
        Write-Host 'Bitte "j" für Ja oder "n" für Nein eingeben.'
    }
}

function New-Fehler([string]$Art, [string]$Text) {
    $ex = New-Object System.Exception $Text
    $ex.Data['Art'] = $Art
    return $ex
}
function Get-FehlerArt($err) {
    $e = $err
    if ($e -is [System.Management.Automation.ErrorRecord]) { $e = $e.Exception }
    while ($e) {
        if ($e.Data -and $e.Data.Contains('Art')) { return [string]$e.Data['Art'] }
        $e = $e.InnerException
    }
    return ''
}

# ---------------------------------------------------------------------------
# GCS JSON API (anonym)
# ---------------------------------------------------------------------------
function Test-RequesterPaysText([string]$t) {
    return ($t -match '(?i)requester pays|userProjectMissing|user project|billing project')
}

function Invoke-GcsApi([string]$Url) {
    $letzter = $null
    for ($v = 1; $v -le 5; $v++) {
        $req = [System.Net.HttpWebRequest]::Create($Url)
        $req.UserAgent = 'MaleCNS-Downloader/1.0'
        $req.Timeout = 60000
        $req.ReadWriteTimeout = 120000
        $req.AutomaticDecompression = [System.Net.DecompressionMethods]::GZip -bor [System.Net.DecompressionMethods]::Deflate
        try {
            $resp = $req.GetResponse()
            try {
                $sr = New-Object IO.StreamReader($resp.GetResponseStream(), [Text.Encoding]::UTF8)
                $txt = $sr.ReadToEnd()
            } finally { $resp.Close() }
            return [pscustomobject]@{ Ok = $true; Status = 200; Json = ($txt | ConvertFrom-Json); Body = '' }
        } catch {
            $ex = $_.Exception
            while ($ex -and -not ($ex -is [System.Net.WebException])) { $ex = $ex.InnerException }
            $status = 0; $body = $_.Exception.Message
            if ($ex -and $ex.Response) {
                $status = [int]$ex.Response.StatusCode
                try { $body = (New-Object IO.StreamReader($ex.Response.GetResponseStream())).ReadToEnd() } catch { }
                $ex.Response.Close()
            }
            $letzter = [pscustomobject]@{ Ok = $false; Status = $status; Json = $null; Body = $body }
            # Nur Netzwerk- und Serverfehler wiederholen
            if ($status -ne 0 -and $status -ne 429 -and $status -lt 500) { return $letzter }
            Start-Sleep -Seconds ([Math]::Min(30, 2 * $v * $v))
        }
    }
    return $letzter
}

function Throw-ApiFehler($r, [string]$Was) {
    if (Test-RequesterPaysText $r.Body) { throw (New-Fehler 'requesterpays' "Der Bucket verlangt 'Requester Pays' ($Was).") }
    if ($r.Status -eq 401 -or $r.Status -eq 403) { throw (New-Fehler 'auth' "Anonymer Zugriff verweigert (HTTP $($r.Status)) bei $Was.") }
    if ($r.Status -eq 404) { throw (New-Fehler 'fehlt' "Nicht gefunden (HTTP 404): $Was") }
    if ($r.Status -eq 0) { throw (New-Fehler 'netz' "Keine Verbindung zu storage.googleapis.com ($Was). Bitte Internetverbindung prüfen.") }
    throw (New-Fehler 'netz' "Serverfehler HTTP $($r.Status) bei $Was.")
}

function Get-ObjUrl([string]$Name) {
    return $Script:DlBase + [uri]::EscapeDataString($Name).Replace('%2F', '/')
}

function Test-NameGueltig([string]$Rel) {
    if ($Rel -match '[<>:"\\|?*\x00-\x1F]') { return $false }
    foreach ($seg in ($Rel -split '/')) {
        if ($seg -eq '' -or $seg -eq '.' -or $seg -eq '..') { return $false }
        if ($seg.EndsWith('.') -or $seg.EndsWith(' ')) { return $false }
    }
    return $true
}

function Assert-Erlaubt([string]$Name) {
    foreach ($p in $Script:Erlaubt) { if ($Name.StartsWith($p, [StringComparison]::Ordinal)) { return } }
    throw (New-Fehler 'sicherheit' "Sicherheitsabbruch: Das Objekt '$Name' liegt außerhalb der erlaubten Ordner und wird nicht geladen.")
}

function New-GcsObj([string]$Name, [long]$Size, [string]$Md5, [string]$Crc, [string]$Rel, [string]$KompDir) {
    # Unter Windows unzulässige Zeichen (z. B. ':' in "male-cns:v1.0.json") lokal durch '_' ersetzen
    if ($Rel -match '[<>:"|?*]') {
        $neu = $Rel -replace '[<>:"|?*]', '_'
        Write-Log "  Hinweis: '$Name' wird lokal als '$neu' gespeichert (Zeichen unter Windows unzulässig)." -NurDatei
        $Rel = $neu
    }
    $o = New-Object GcsObj
    $o.Name = $Name; $o.Size = $Size; $o.Md5 = $Md5; $o.Crc = $Crc; $o.Rel = $Rel
    $o.Local = Join-Path $KompDir ($Rel -replace '/', '\')
    $o.Url = Get-ObjUrl $Name
    return $o
}

function Get-Prop($obj, [string]$Name) {
    $p = $obj.PSObject.Properties[$Name]
    if ($p) { return [string]$p.Value }
    return ''
}

# Liste aller Objekte einer Quelle (mit Paging über nextPageToken, Cache im Verwaltungsordner)
function Get-QuellenObjekte($Komp, $Quelle, [string]$KompDir) {
    $liste = New-Object 'System.Collections.Generic.List[GcsObj]'
    $sub = [string]$Quelle.Unterordner
    $vorsatz = ''
    if ($sub) { $vorsatz = $sub + '/' }

    if ($Quelle.Typ -eq 'Dateien') {
        foreach ($name in $Quelle.Objekte) {
            Assert-Erlaubt $name
            $r = Invoke-GcsApi ($Script:ApiBase + '/' + [uri]::EscapeDataString($name) + '?fields=name,size,md5Hash,crc32c')
            if (-not $r.Ok) { Throw-ApiFehler $r $name }
            $rel = $vorsatz + $name.Substring($name.LastIndexOf('/') + 1)
            $liste.Add((New-GcsObj $name ([long]$r.Json.size) (Get-Prop $r.Json 'md5Hash') (Get-Prop $r.Json 'crc32c') $rel $KompDir))
        }
        return , $liste
    }

    $prefix = [string]$Quelle.Prefix
    Assert-Erlaubt $prefix
    $cache = Join-Path $Script:VerwDir ('liste-' + ($prefix.TrimEnd('/') -replace '[^A-Za-z0-9.-]', '_') + '.tsv')
    $frueh = ($Limit -gt 0 -and -not $Filter)

    if (-not $frueh -and (Test-Path -LiteralPath $cache)) {
        $alter = (Get-Date) - (Get-Item -LiteralPath $cache).LastWriteTime
        if ($alter.TotalDays -lt $Script:CacheTage) {
            $zeilen = [IO.File]::ReadAllLines($cache)
            if ($zeilen.Length -ge 1 -and $zeilen[0] -eq ("#mcns1`t" + $prefix)) {
                for ($i = 1; $i -lt $zeilen.Length; $i++) {
                    $f = $zeilen[$i].Split("`t")
                    if ($f.Length -lt 4) { continue }
                    $liste.Add((New-GcsObj $f[0] ([long]$f[1]) $f[2] $f[3] ($vorsatz + $f[0].Substring($prefix.Length)) $KompDir))
                }
                Write-Log ("  {0}: Dateiliste aus Zwischenspeicher ({1} Dateien)" -f $prefix, (Format-Zahl $liste.Count)) -NurDatei
                return , $liste
            }
        }
    }

    $token = $null; $seite = 0
    do {
        $url = $Script:ApiBase + '?prefix=' + [uri]::EscapeDataString($prefix) + '&maxResults=1000&fields=' +
               [uri]::EscapeDataString('items(name,size,md5Hash,crc32c),nextPageToken')
        if ($token) { $url += '&pageToken=' + [uri]::EscapeDataString($token) }
        $r = Invoke-GcsApi $url
        if (-not $r.Ok) { Write-Progress -Id 1 -Activity 'x' -Completed; Throw-ApiFehler $r "Auflisten von $prefix" }
        $items = $r.Json.PSObject.Properties['items']
        if ($items) {
            foreach ($it in $items.Value) {
                $name = [string]$it.name
                if ($name.EndsWith('/')) { continue }       # Ordner-Platzhalter
                Assert-Erlaubt $name
                $liste.Add((New-GcsObj $name ([long]$it.size) (Get-Prop $it 'md5Hash') (Get-Prop $it 'crc32c') ($vorsatz + $name.Substring($prefix.Length)) $KompDir))
            }
        }
        $t = $r.Json.PSObject.Properties['nextPageToken']
        $token = $null
        if ($t) { $token = [string]$t.Value }
        $seite++
        Write-Progress -Id 1 -Activity "Ermittle Dateiliste für Komponente $($Komp.Key)" -Status ("{0} Dateien gefunden" -f (Format-Zahl $liste.Count))
        if ($frueh -and $liste.Count -ge $Limit) { break }
    } while ($token)
    Write-Progress -Id 1 -Activity "Dateiliste" -Completed

    if (-not $frueh) {
        $sb = New-Object Text.StringBuilder
        [void]$sb.Append("#mcns1`t" + $prefix + "`n")
        foreach ($o in $liste) { [void]$sb.Append($o.Name + "`t" + $o.Size + "`t" + $o.Md5 + "`t" + $o.Crc + "`n") }
        [IO.File]::WriteAllText($cache, $sb.ToString(), (New-Object Text.UTF8Encoding $false))
    }
    return , $liste
}

# ---------------------------------------------------------------------------
# Google Cloud CLI (nur als Rückfallebene)
# ---------------------------------------------------------------------------
function Show-RequesterPays {
    Write-Host ''
    Write-Host '============================================================' -ForegroundColor Red
    Write-Host ' Abbruch: Der Bucket ist auf "Requester Pays" umgestellt.'     -ForegroundColor Red
    Write-Host '============================================================' -ForegroundColor Red
    Write-Host @'

Was bedeutet das?
  Normalerweise zahlt der Anbieter (Janelia) die Übertragungskosten. Bei
  "Requester Pays" zahlt stattdessen derjenige, der die Daten abruft. Google
  erlaubt den Download dann nur mit einem Google-Cloud-Projekt, für das ein
  Rechnungskonto (Kreditkarte) hinterlegt ist.

Was würde es kosten?
  - Datenübertragung ins Internet: grob 0,08 bis 0,12 US-Dollar pro GB
    (je nach Region und Menge; aktuelle Preise:
    https://cloud.google.com/storage/pricing#network-egress).
    Beispiel: Komponenten A bis D (rund 480 GB) etwa 40 bis 60 US-Dollar,
    nur Komponente A (rund 24 GB) etwa 2 bis 3 US-Dollar.
  - Dazu kleine Gebühren pro Dateiabruf (Bruchteile eines Cents je 1000
    Dateien) - bei Hunderttausenden Skelett-Dateien einige Cent.

Dieses Programm verwendet absichtlich KEIN Abrechnungsprojekt.
Wenn Sie die Kosten tragen möchten, können Sie selbst z. B. mit
  gcloud storage cp -r --billing-project=IHR-PROJEKT gs://flyem-male-cns/<pfad> <ziel>
laden. Alternativ bei den Anbietern nachfragen (https://male-cns.janelia.org).
'@
    Write-Log 'Abbruch wegen Requester Pays.' 'fehler' -NurDatei
}

function Find-Gcloud {
    $c = Get-Command 'gcloud.cmd' -ErrorAction SilentlyContinue
    if ($c) { return $c.Source }
    $kand = @(
        (Join-Path $env:LOCALAPPDATA 'Google\Cloud SDK\google-cloud-sdk\bin\gcloud.cmd'),
        (Join-Path ${env:ProgramFiles(x86)} 'Google\Cloud SDK\google-cloud-sdk\bin\gcloud.cmd'),
        (Join-Path $env:ProgramFiles 'Google\Cloud SDK\google-cloud-sdk\bin\gcloud.cmd'))
    foreach ($k in $kand) { if ($k -and (Test-Path -LiteralPath $k)) { return $k } }
    return $null
}

function Invoke-Gcloud([string[]]$Argumente, [switch]$Live) {
    $zeilen = New-Object 'System.Collections.Generic.List[string]'
    $alt = $ErrorActionPreference
    $ErrorActionPreference = 'Continue'
    try {
        & $Script:Gcloud @Argumente 2>&1 | ForEach-Object {
            $s = "$_"
            $zeilen.Add($s)
            if ($Live) { Write-Host $s }
        }
        $code = $LASTEXITCODE
    } finally { $ErrorActionPreference = $alt }
    $txt = $zeilen -join "`n"
    Write-Log ("gcloud " + ($Argumente -join ' ') + " -> Exitcode $code") -NurDatei
    if (Test-RequesterPaysText $txt) { throw (New-Fehler 'requesterpays' 'Requester Pays (gcloud)') }
    return [pscustomobject]@{ Code = $code; Text = $txt }
}

# Stellt sicher, dass gcloud installiert und angemeldet ist. Gibt $true/$false zurück.
function Initialize-Gcloud {
    if ($Script:Gcloud) { return $true }
    $g = Find-Gcloud
    if (-not $g) {
        Write-Log 'Die Google Cloud CLI (gcloud) ist nicht installiert.' 'warnung'
        $winget = Get-Command 'winget.exe' -ErrorAction SilentlyContinue
        if (-not $winget) {
            Write-Host 'winget ist auf diesem PC nicht verfügbar. Bitte die Google Cloud CLI von'
            Write-Host '  https://cloud.google.com/sdk/docs/install'
            Write-Host 'installieren und danach install.bat erneut starten.'
            return $false
        }
        if (-not (Read-JaNein 'Soll die Google Cloud CLI jetzt mit "winget install Google.CloudSDK" installiert werden?' $true)) { return $false }
        Write-Log 'Installiere Google Cloud CLI über winget …'
        & $winget.Source install --id Google.CloudSDK -e --accept-source-agreements --accept-package-agreements | Out-Host
        $g = Find-Gcloud
        if (-not $g) {
            Write-Log 'gcloud wurde nicht gefunden. Bitte dieses Fenster schließen und install.bat neu starten.' 'warnung'
            return $false
        }
    }
    $Script:Gcloud = $g
    Write-Log "Google Cloud CLI gefunden: $g" -NurDatei
    $r = Invoke-Gcloud @('auth', 'list', '--filter=status:ACTIVE', '--format=value(account)')
    if (-not ($r.Text -match '@')) {
        Write-Host 'Für den Zugriff ist eine Anmeldung mit einem Google-Konto nötig. Gleich öffnet sich der Browser.'
        & $Script:Gcloud auth login | Out-Host
        if ($LASTEXITCODE -ne 0) { Write-Log 'Die Anmeldung bei Google ist fehlgeschlagen oder wurde abgebrochen.' 'fehler'; $Script:Gcloud = $null; return $false }
    }
    return $true
}

function Get-GcloudGroesse($Komp) {
    $summe = [long]0
    foreach ($q in $Komp.Quellen) {
        $ziele = @()
        if ($q.Typ -eq 'Dateien') { $ziele = $q.Objekte | ForEach-Object { "gs://$($Script:Bucket)/$_" } }
        else { $ziele = @("gs://$($Script:Bucket)/$($q.Prefix)") }
        $r = Invoke-Gcloud (@('storage', 'du', '-s') + $ziele)
        if ($r.Code -ne 0) { throw (New-Fehler 'gcloud' "gcloud konnte die Größe nicht ermitteln:`n$($r.Text)") }
        foreach ($z in ($r.Text -split "`n")) { if ($z -match '^\s*(\d+)\s+gs://') { $summe += [long]$Matches[1] } }
    }
    return $summe
}

function Invoke-GcloudDownload($Komp, [string]$KompDir) {
    $ok = $true
    foreach ($q in $Komp.Quellen) {
        $dest = $KompDir
        if ($q.Unterordner) { $dest = Join-Path $KompDir $q.Unterordner }
        New-Item -ItemType Directory -Force -Path $dest | Out-Null
        if ($q.Typ -eq 'Dateien') {
            $quellen = $q.Objekte | ForEach-Object { "gs://$($Script:Bucket)/$_" }
            $a = @('storage', 'cp', '--no-clobber') + $quellen + @($dest)
        } else {
            $a = @('storage', 'cp', '-r', '--no-clobber', "gs://$($Script:Bucket)/$($q.Prefix)*", $dest)
        }
        Write-Log ("gcloud lädt {0} …" -f ($q.Prefix)) 'info'
        $r = Invoke-Gcloud $a -Live
        if ($r.Code -ne 0) { $ok = $false; Write-Log "gcloud meldet einen Fehler (Exitcode $($r.Code)). Details im Log." 'fehler' }
    }
    return $ok
}

# ---------------------------------------------------------------------------
# curl
# ---------------------------------------------------------------------------
function Find-Curl {
    $kand = @((Join-Path $env:SystemRoot 'System32\curl.exe'))
    $c = Get-Command 'curl.exe' -ErrorAction SilentlyContinue
    if ($c) { $kand += $c.Source }
    foreach ($k in $kand) {
        if (-not (Test-Path -LiteralPath $k)) { continue }
        $v = (& $k --version 2>$null | Select-Object -First 1)
        if ($v -match 'curl (\d+)\.(\d+)') {
            $ver = [version]("{0}.{1}" -f $Matches[1], $Matches[2])
            if ($ver -ge [version]'7.66') { return $k }
        }
    }
    return $null
}

# Eine große Datei laden (mit Fortsetzen). Rückgabe: Status-String
function Save-GrosseDatei([GcsObj]$o, [string]$KompDir) {
    $part = $o.Local + '.part'
    $relPart = $o.Rel + '.part'
    for ($versuch = 1; $versuch -le $Script:MaxVers; $versuch++) {
        $stillstand = 0
        while ($true) {
            $hat = [long]0
            if (Test-Path -LiteralPath $part) { $hat = (Get-Item -LiteralPath $part).Length }
            if ($hat -gt $o.Size) { Remove-Item -LiteralPath $part -Force; $hat = 0 }
            if ($hat -eq $o.Size) { break }
            if ($hat -gt 0) { Write-Log ("  Setze Download fort bei {0} von {1}" -f (Format-Bytes $hat), (Format-Bytes $o.Size)) }
            $a = @('-L', '--fail', '--create-dirs', '--connect-timeout', '30', '--speed-limit', '1024', '--speed-time', '120',
                   '--retry', '3', '--retry-delay', '5', '-w', '%{response_code}', '-o', $relPart)
            if ($hat -gt 0) { $a += @('-C', '-') }
            if ($Bandbreite) { $a += @('--limit-rate', $Bandbreite) }
            $a += $o.Url
            $out = & $Script:Curl @a
            $exit = $LASTEXITCODE
            Write-Host ''
            $http = [string]($out | Select-Object -Last 1)
            if ($http -eq '401' -or $http -eq '403') { throw (New-Fehler 'auth' "HTTP $http beim Laden von $($o.Name)") }
            if ($exit -ne 0) { Write-Log ("  Übertragung unterbrochen (curl-Code {0}, HTTP {1})." -f $exit, $http) 'warnung' }
            $neu = [long]0
            if (Test-Path -LiteralPath $part) { $neu = (Get-Item -LiteralPath $part).Length }
            if ($neu -le $hat) { $stillstand++ } else { $stillstand = 0 }
            if ($stillstand -ge 3) { break }
            if ($exit -ne 0) { Start-Sleep -Seconds 5 }
        }
        if (-not (Test-Path -LiteralPath $part)) { Write-Log "  Datei konnte nicht geladen werden (Versuch $versuch von $($Script:MaxVers))." 'warnung'; continue }
        Write-Host '  Prüfe Datei (bei großen Dateien kann das einige Minuten dauern) …'
        $res = [McnsUtil]::Verify($part, $o.Size, $o.Md5, $o.Crc)
        if ($res.StartsWith('ok')) {
            [McnsUtil]::Commit($part, $o.Local)
            Write-Log ("  OK, Prüfung: {0}" -f $res.Substring(3)) 'ok'
            return $res
        }
        Write-Log ("  Prüfung fehlgeschlagen ({0}), lade neu (Versuch {1} von {2})." -f $res, $versuch, $Script:MaxVers) 'warnung'
        if ($res -ne 'groesse') { Remove-Item -LiteralPath $part -Force -ErrorAction SilentlyContinue }
    }
    return 'fehler'
}

# Viele kleine Dateien parallel laden. Gibt Liste der fehlgeschlagenen Objekte zurück.
function Save-KleineDateien($Objekte, [string]$KompDir, $Stat, [string]$Titel) {
    $offen = New-Object 'System.Collections.Generic.List[GcsObj]'
    foreach ($o in $Objekte) { $offen.Add($o) }
    $gesamt = $offen.Count
    $gesamtBytes = [long]0; foreach ($o in $offen) { $gesamtBytes += $o.Size }
    $fertig = 0; $fertigBytes = [long]0
    $cfg = Join-Path $KompDir '.curl-batch.cfg'
    $utf8 = New-Object Text.UTF8Encoding $false

    for ($versuch = 1; $versuch -le $Script:MaxVers -and $offen.Count -gt 0; $versuch++) {
        if ($versuch -gt 1) { Write-Log ("  Versuch {0}: {1} Dateien werden erneut geladen." -f $versuch, $offen.Count) 'warnung'; Start-Sleep -Seconds 5 }
        $fehl = New-Object 'System.Collections.Generic.List[GcsObj]'
        for ($s = 0; $s -lt $offen.Count; $s += $Script:BatchGr) {
            $ende = [Math]::Min($s + $Script:BatchGr, $offen.Count) - 1
            $zeilen = New-Object 'System.Collections.Generic.List[string]'
            foreach ($z in @('fail', 'location', 'create-dirs', 'silent', 'show-error', 'retry = 3', 'retry-delay = 3', 'connect-timeout = 30')) { $zeilen.Add($z) }
            if ($Bandbreite) { $zeilen.Add("limit-rate = $Bandbreite") }
            for ($i = $s; $i -le $ende; $i++) {
                $o = $offen[$i]
                $zeilen.Add('url = "' + $o.Url + '"')
                $zeilen.Add('output = "' + $o.Rel + '.part"')
            }
            [IO.File]::WriteAllLines($cfg, $zeilen, $utf8)
            $pct = 0
            if ($gesamtBytes -gt 0) { $pct = [int](100 * $fertigBytes / $gesamtBytes) }
            Write-Progress -Id 2 -Activity $Titel -Status ("{0} / {1} Dateien, {2} / {3}" -f (Format-Zahl $fertig), (Format-Zahl $gesamt), (Format-Bytes $fertigBytes), (Format-Bytes $gesamtBytes)) -PercentComplete ([Math]::Min(100, $pct))

            $out = & $Script:Curl --parallel --parallel-immediate --parallel-max $Parallel -K '.curl-batch.cfg' -w '%{response_code}\n'
            $codes = @($out | ForEach-Object { "$_".Trim() })
            if (($codes -contains '401') -or ($codes -contains '403')) {
                Remove-Item -LiteralPath $cfg -Force -ErrorAction SilentlyContinue
                throw (New-Fehler 'auth' 'HTTP 401/403 beim Laden von Skelett-/Einzeldateien')
            }
            for ($i = $s; $i -le $ende; $i++) {
                $o = $offen[$i]
                $part = $o.Local + '.part'
                $res = [McnsUtil]::Verify($part, $o.Size, $o.Md5, $o.Crc)
                if ($res.StartsWith('ok')) {
                    [McnsUtil]::Commit($part, $o.Local)
                    $fertig++; $fertigBytes += $o.Size
                    $Stat.Geladen++; $Stat.GeladenBytes += $o.Size
                } else {
                    if ($res -ne 'fehlt') { Remove-Item -LiteralPath $part -Force -ErrorAction SilentlyContinue }
                    Write-Log ("  Fehler bei {0}: {1}" -f $o.Rel, $res) 'warnung' -NurDatei
                    $fehl.Add($o)
                }
            }
            Write-Log ("  {0}: {1} / {2} Dateien fertig" -f $Titel, $fertig, $gesamt) -NurDatei
        }
        $offen = $fehl
    }
    Remove-Item -LiteralPath $cfg -Force -ErrorAction SilentlyContinue
    Write-Progress -Id 2 -Activity $Titel -Completed
    return , $offen
}

# ---------------------------------------------------------------------------
# Planung (Größen ermitteln) und Download pro Komponente
# ---------------------------------------------------------------------------
function Get-Plan($Komp) {
    $kompDir = Join-Path $Script:ZielDir $Komp.Ordner
    $plan = [pscustomobject]@{
        Komp = $Komp; Dir = $kompDir; Modus = 'https'
        Todo = (New-Object 'System.Collections.Generic.List[GcsObj]')
        Anzahl = 0; Gesamt = [long]0; VorhandenAnz = 0; VorhandenBytes = [long]0; TodoBytes = [long]0
        MaxPfad = 0; MaxDatei = [long]0; Ungueltig = 0
    }
    Write-Log ("Ermittle Größe von {0} – {1} …" -f $Komp.Key, $Komp.Titel)
    $index = [McnsUtil]::IndexDir($kompDir)
    try {
        foreach ($q in $Komp.Quellen) {
            $objs = Get-QuellenObjekte $Komp $q $kompDir
            $n = 0
            foreach ($o in $objs) {
                if ($Filter -and -not ($o.Rel -like $Filter -or $o.Rel.Substring($o.Rel.LastIndexOf('/') + 1) -like $Filter)) { continue }
                if ($Limit -gt 0 -and $n -ge $Limit) { break }
                $n++
                if (-not (Test-NameGueltig $o.Rel)) { $plan.Ungueltig++; Write-Log "  Übersprungen (Dateiname unter Windows unzulässig): $($o.Name)" 'warnung' -NurDatei; continue }
                $plan.Anzahl++; $plan.Gesamt += $o.Size
                if ($o.Size -gt $plan.MaxDatei) { $plan.MaxDatei = $o.Size }
                if ($o.Local.Length -gt $plan.MaxPfad) { $plan.MaxPfad = $o.Local.Length }
                $vorhanden = $false
                if ($index.ContainsKey($o.Rel) -and $index[$o.Rel] -eq $o.Size) {
                    $vorhanden = $true
                    if ($Neupruefen -and -not ([McnsUtil]::Verify($o.Local, $o.Size, $o.Md5, $o.Crc)).StartsWith('ok')) {
                        Write-Log "  Vorhandene Datei ist beschädigt und wird neu geladen: $($o.Rel)" 'warnung'
                        $vorhanden = $false
                    }
                }
                if ($vorhanden) { $plan.VorhandenAnz++; $plan.VorhandenBytes += $o.Size }
                else { $plan.Todo.Add($o); $plan.TodoBytes += $o.Size }
            }
        }
    } catch {
        $art = Get-FehlerArt $_
        if ($art -ne 'auth') { throw }
        Write-Log ("Anonymer Zugriff auf Komponente {0} wurde verweigert ({1})." -f $Komp.Key, $_.Exception.Message) 'warnung'
        if (-not (Read-JaNein 'Stattdessen die Google Cloud CLI mit Google-Konto verwenden?' $true)) { return $null }
        if (-not (Initialize-Gcloud)) { return $null }
        $plan.Modus = 'gcloud'
        $plan.Gesamt = Get-GcloudGroesse $Komp
        $lokal = [long]0; foreach ($v in $index.Values) { $lokal += $v }
        $plan.VorhandenBytes = [Math]::Min($lokal, $plan.Gesamt)
        $plan.TodoBytes = $plan.Gesamt - $plan.VorhandenBytes
        $plan.Anzahl = -1
    }
    return $plan
}

function Invoke-Komponente($Plan, $Stat) {
    $k = $Plan.Komp
    Write-Log ("Komponente {0} – {1}" -f $k.Key, $k.Titel) 'titel'
    New-Item -ItemType Directory -Force -Path $Plan.Dir | Out-Null
    $Stat.Uebersprungen = $Plan.VorhandenAnz

    if ($Plan.Modus -eq 'gcloud') {
        if (Invoke-GcloudDownload $k $Plan.Dir) { $Stat.Hinweis = 'über gcloud geladen (gcloud prüft die Prüfsummen selbst)' }
        else { $Stat.Hinweis = 'gcloud meldete Fehler, bitte Log prüfen und erneut starten'; $Stat.Fehlgeschlagen.Add("$($k.Key): gcloud-Fehler") }
        return
    }
    if ($Plan.Todo.Count -eq 0) { Write-Log '  Alle Dateien sind bereits vollständig vorhanden.' 'ok'; return }

    $klein = New-Object 'System.Collections.Generic.List[GcsObj]'
    $gross = New-Object 'System.Collections.Generic.List[GcsObj]'
    foreach ($o in $Plan.Todo) { if ($o.Size -ge $Script:GrossAb) { $gross.Add($o) } else { $klein.Add($o) } }

    Push-Location -LiteralPath $Plan.Dir
    try {
        try {
            if ($klein.Count -gt 0) {
                Write-Log ("  {0} kleinere Dateien ({1}), {2} gleichzeitig …" -f (Format-Zahl $klein.Count), (Format-Bytes (($klein | Measure-Object Size -Sum).Sum)), $Parallel)
                $rest = Save-KleineDateien $klein $Plan.Dir $Stat ("Komponente $($k.Key)")
                foreach ($o in $rest) { $Stat.Fehlgeschlagen.Add("$($k.Key): $($o.Name)") }
                Write-Log ("  Kleinere Dateien fertig: {0} geladen, {1} fehlgeschlagen." -f (Format-Zahl ($klein.Count - $rest.Count)), $rest.Count)
            }
            $nr = 0
            foreach ($o in $gross) {
                $nr++
                Write-Log ("  [{0}/{1}] {2} ({3})" -f $nr, $gross.Count, $o.Rel, (Format-Bytes $o.Size))
                $res = Save-GrosseDatei $o $Plan.Dir
                if ($res.StartsWith('ok')) { $Stat.Geladen++; $Stat.GeladenBytes += $o.Size }
                else { $Stat.Fehlgeschlagen.Add("$($k.Key): $($o.Name)"); Write-Log "  Endgültig fehlgeschlagen: $($o.Rel)" 'fehler' }
            }
        } finally { Pop-Location }
    } catch {
        if ((Get-FehlerArt $_) -ne 'auth') { throw }
        Write-Log ("Der Server verweigert den anonymen Download ({0})." -f $_.Exception.Message) 'warnung'
        if ((Read-JaNein 'Stattdessen die Google Cloud CLI mit Google-Konto verwenden?' $true) -and (Initialize-Gcloud)) {
            $Plan.Modus = 'gcloud'
            Invoke-Komponente $Plan $Stat
        } else {
            $Stat.Fehlgeschlagen.Add("$($k.Key): Zugriff verweigert")
        }
    }
}

# ---------------------------------------------------------------------------
# Python
# ---------------------------------------------------------------------------
function Find-Python {
    $kand = @(@{ Exe = 'py.exe'; Arg = @('-3') }, @{ Exe = 'python.exe'; Arg = @() }, @{ Exe = 'python3.exe'; Arg = @() })
    foreach ($k in $kand) {
        $c = Get-Command $k.Exe -ErrorAction SilentlyContinue
        if (-not $c) { continue }
        $alt = $ErrorActionPreference; $ErrorActionPreference = 'Continue'
        try { $v = & $c.Source @($k.Arg) -c 'import sys; print(sys.version.split()[0])' 2>$null } catch { $v = $null } finally { $ErrorActionPreference = $alt }
        if ($LASTEXITCODE -eq 0 -and "$v" -match '^3\.') { return @{ Exe = $c.Source; Arg = $k.Arg; Version = "$v".Trim() } }
    }
    return $null
}

function Install-PythonPakete($Py) {
    Write-Log ("Installiere Python-Pakete mit Python {0} …" -f $Py.Version) 'titel'
    $alt = $ErrorActionPreference; $ErrorActionPreference = 'Continue'
    try { & $Py.Exe @($Py.Arg) -m pip install pandas pyarrow neuprint-python navis | Out-Host } finally { $ErrorActionPreference = $alt }
    if ($LASTEXITCODE -eq 0) { Write-Log 'Python-Pakete wurden installiert.' 'ok' }
    else { Write-Log 'Die Installation der Python-Pakete ist fehlgeschlagen (siehe Ausgabe oben).' 'fehler' }
}

# ---------------------------------------------------------------------------
# Menü
# ---------------------------------------------------------------------------
function Read-Auswahl($Py) {
    while ($true) {
        Write-Host ''
        Write-Host 'Welche Komponenten sollen heruntergeladen werden?' -ForegroundColor Cyan
        foreach ($k in $Script:Komps.Values) { Write-Host ("  [{0}] {1}  ({2})" -f $k.Key, $k.Titel, $k.Hinweis) }
        Write-Host  '  [X] Alles (A bis D, ca. 480 GB)'
        if ($Py) { Write-Host ("  [P] Python-Pakete zum Auswerten installieren (pandas, pyarrow, neuprint-python, navis; Python {0})" -f $Py.Version) }
        Write-Host  '  [Q] Beenden'
        $ein = Read-Host 'Auswahl (mehrere mit Komma, z. B. A,B)'
        if ($null -eq $ein) { return @() }
        $r = ConvertTo-Auswahl $ein
        if ($r.Fehler) { Write-Host $r.Fehler -ForegroundColor Yellow; continue }
        if ($r.Quit) { return @() }
        if ($r.Python) {
            if ($Py) { Install-PythonPakete $Py } else { Write-Host 'Python wurde nicht gefunden.' -ForegroundColor Yellow }
            if ($r.Liste.Count -eq 0) { continue }
        }
        if ($r.Liste.Count -gt 0) { return [string[]]$r.Liste.ToArray() }
    }
}

function ConvertTo-Auswahl([string]$Text) {
    $keys = New-Object 'System.Collections.Generic.List[string]'
    $res = @{ Liste = $keys; Python = $false; Quit = $false; Fehler = '' }
    foreach ($t in ($Text.ToUpper() -split '[\s,;]+')) {
        if ($t -eq '') { continue }
        switch ($t) {
            'Q'     { $res.Quit = $true }
            'P'     { $res.Python = $true }
            'X'     { foreach ($x in 'A', 'B', 'C', 'D') { if (-not $keys.Contains($x)) { $keys.Add($x) } } }
            'ALLES' { foreach ($x in 'A', 'B', 'C', 'D') { if (-not $keys.Contains($x)) { $keys.Add($x) } } }
            default {
                if ($Script:Komps.Contains($t)) { if (-not $keys.Contains($t)) { $keys.Add($t) } }
                else { $res.Fehler = "Unbekannte Auswahl: '$t'. Bitte Buchstaben aus dem Menü verwenden." }
            }
        }
    }
    $sortiert = @($keys | Sort-Object)
    $keys.Clear(); foreach ($x in $sortiert) { $keys.Add($x) }
    return $res
}

function Read-Zielordner {
    $standard = Join-Path $env:USERPROFILE 'MaleCNS'
    while ($true) {
        $ein = Read-Host "Zielordner [$standard]"
        if ($null -eq $ein -or $ein.Trim() -eq '') { $ein = $standard }
        $ein = [Environment]::ExpandEnvironmentVariables($ein.Trim().Trim('"'))
        try {
            $voll = [IO.Path]::GetFullPath($ein)
            New-Item -ItemType Directory -Force -Path $voll | Out-Null
            return $voll
        } catch {
            Write-Host "Der Ordner '$ein' kann nicht verwendet werden: $($_.Exception.Message)" -ForegroundColor Yellow
        }
    }
}

# ---------------------------------------------------------------------------
# Hauptprogramm
# ---------------------------------------------------------------------------
function Main {
    Write-Host ''
    Write-Host '==================================================================' -ForegroundColor Cyan
    Write-Host ' MaleCNS-Konnektom-Downloader  (männliche Fruchtfliege, Version v1.0)' -ForegroundColor Cyan
    Write-Host ' Janelia FlyEM - Lizenz CC-BY 4.0 - https://male-cns.janelia.org' -ForegroundColor Cyan
    Write-Host '==================================================================' -ForegroundColor Cyan

    # Zielordner
    if ($Ziel) {
        $Script:ZielDir = [IO.Path]::GetFullPath([Environment]::ExpandEnvironmentVariables($Ziel.Trim('"')))
        New-Item -ItemType Directory -Force -Path $Script:ZielDir | Out-Null
    } else {
        $Script:ZielDir = Read-Zielordner
    }
    $Script:VerwDir = Join-Path $Script:ZielDir '_verwaltung'
    New-Item -ItemType Directory -Force -Path $Script:VerwDir | Out-Null
    $logPfad = Join-Path $Script:ZielDir 'download-log.txt'
    Open-Log $logPfad
    Write-Log "Zielordner: $($Script:ZielDir)"
    Write-Log "Logdatei:   $logPfad"

    $Script:Curl = Find-Curl
    if (-not $Script:Curl) {
        Write-Log 'curl.exe (ab Version 7.66) wurde nicht gefunden. Es ist ab Windows 10 (Version 1803) enthalten; bitte Windows aktualisieren.' 'fehler'
        return 1
    }
    Write-Log "curl: $($Script:Curl)" -NurDatei

    $py = Find-Python

    # Auswahl
    if ($Komponenten) {
        $r = ConvertTo-Auswahl $Komponenten
        if ($r.Fehler) { Write-Log $r.Fehler 'fehler'; return 1 }
        if ($r.Python) { if ($py) { Install-PythonPakete $py } else { Write-Log 'Python wurde nicht gefunden.' 'warnung' } }
        $keys = @($r.Liste)
    } else {
        $keys = @(Read-Auswahl $py)
    }
    if (-not $keys -or $keys.Count -eq 0) { Write-Log 'Nichts ausgewählt, Programm beendet.'; return 0 }
    Write-Log ('Auswahl: ' + ($keys -join ', ')) -NurDatei
    if ($Filter -or $Limit -gt 0) { Write-Log ("TESTMODUS: Filter='{0}', Limit={1}" -f $Filter, $Limit) 'warnung' }

    # Größen ermitteln
    Write-Log 'Ermittle Größen (das kann bei den Skelett-Ordnern einige Minuten dauern) …' 'titel'
    $plaene = New-Object System.Collections.ArrayList
    foreach ($key in $keys) {
        $p = Get-Plan $Script:Komps[$key]
        if ($p) { [void]$plaene.Add($p) } else { Write-Log "Komponente $key wird übersprungen." 'warnung' }
    }
    if ($plaene.Count -eq 0) { Write-Log 'Keine Komponente kann geladen werden.' 'fehler'; return 1 }

    # Übersicht
    Write-Log 'Übersicht' 'titel'
    $zeile = '  {0,-2} {1,-48} {2,10} {3,11} {4,11} {5,11}'
    Write-Log ($zeile -f '', 'Komponente', 'Dateien', 'Gesamt', 'vorhanden', 'noch laden')
    $summe = [long]0; $summeTodo = [long]0; $maxDatei = [long]0; $maxPfad = 0
    foreach ($p in $plaene) {
        $anz = 'unbekannt'
        if ($p.Anzahl -ge 0) { $anz = Format-Zahl $p.Anzahl }
        $titel = $p.Komp.Titel
        if ($titel.Length -gt 48) { $titel = $titel.Substring(0, 47) + '…' }
        Write-Log ($zeile -f $p.Komp.Key, $titel, $anz, (Format-Bytes $p.Gesamt), (Format-Bytes $p.VorhandenBytes), (Format-Bytes $p.TodoBytes))
        if ($p.Modus -eq 'gcloud') { Write-Log '     (wird über die Google Cloud CLI geladen)' }
        if ($p.Ungueltig -gt 0) { Write-Log ("     {0} Dateien mit unter Windows unzulässigem Namen werden ausgelassen (siehe Log)." -f $p.Ungueltig) 'warnung' }
        $summe += $p.Gesamt; $summeTodo += $p.TodoBytes
        if ($p.MaxDatei -gt $maxDatei) { $maxDatei = $p.MaxDatei }
        if ($p.MaxPfad -gt $maxPfad) { $maxPfad = $p.MaxPfad }
    }
    Write-Log ($zeile -f '', 'Summe', '', (Format-Bytes $summe), (Format-Bytes ($summe - $summeTodo)), (Format-Bytes $summeTodo))

    # Speicherplatz
    $frei = $null; $format = ''
    try {
        $drive = New-Object IO.DriveInfo([IO.Path]::GetPathRoot($Script:ZielDir))
        $frei = $drive.AvailableFreeSpace; $format = $drive.DriveFormat
    } catch { }
    $platzOk = $true
    if ($null -ne $frei) {
        Write-Log ("  Freier Speicher auf {0}: {1}" -f [IO.Path]::GetPathRoot($Script:ZielDir), (Format-Bytes $frei))
        $reserve = [long]1GB
        if ($summeTodo + $reserve -gt $frei) {
            Write-Log ("  NICHT GENUG PLATZ: Es fehlen {0} (inkl. 1 GB Reserve). Bitte weniger Komponenten wählen oder ein anderes Laufwerk nehmen." -f (Format-Bytes ($summeTodo + $reserve - $frei))) 'fehler'
            $platzOk = $false
        }
        if ($format -eq 'FAT32' -and $maxDatei -ge 4GB) {
            Write-Log '  Das Ziellaufwerk ist FAT32 und kann keine Dateien über 4 GB speichern. Bitte ein NTFS- oder exFAT-Laufwerk verwenden.' 'fehler'
            $platzOk = $false
        }
    } else {
        Write-Log '  Der freie Speicherplatz auf dem Ziel konnte nicht ermittelt werden (z. B. Netzlaufwerk). Bitte selbst prüfen.' 'warnung'
    }
    if ($maxPfad -gt 259) {
        Write-Log ("  Hinweis: Einige Dateipfade werden {0} Zeichen lang. Falls Fehler auftreten, einen kürzeren Zielordner wählen (z. B. D:\MaleCNS)." -f $maxPfad) 'warnung'
    }
    if ($NurPruefen) { Write-Log 'Nur Prüfung gewünscht, es wird nichts heruntergeladen.'; return 0 }
    if (-not $platzOk) { return 1 }
    if ($summeTodo -eq 0 -and -not ($plaene | Where-Object { $_.Modus -eq 'gcloud' })) {
        Write-Log 'Alles ist bereits vollständig vorhanden. Nichts zu tun.' 'ok'
        return 0
    }

    if (-not $Ja) {
        Write-Host ''
        if (-not (Read-JaNein ("Jetzt {0} herunterladen?" -f (Format-Bytes $summeTodo)) $false)) { Write-Log 'Vom Benutzer abgebrochen.'; return 0 }
    }

    # Download
    $stats = New-Object System.Collections.ArrayList
    $start = Get-Date
    foreach ($p in $plaene) {
        $st = [pscustomobject]@{ Key = $p.Komp.Key; Geladen = 0; GeladenBytes = [long]0; Uebersprungen = 0
                                 Fehlgeschlagen = (New-Object 'System.Collections.Generic.List[string]'); Hinweis = '' }
        [void]$stats.Add($st)
        $null = Invoke-Komponente $p $st
    }

    # Zusammenfassung
    $dauer = (Get-Date) - $start
    Write-Log 'Zusammenfassung' 'titel'
    $alleFehler = New-Object 'System.Collections.Generic.List[string]'
    foreach ($st in $stats) {
        $txt = "  {0}: geladen {1} Dateien ({2}), übersprungen {3} (bereits vorhanden), fehlgeschlagen {4}" -f `
            $st.Key, (Format-Zahl $st.Geladen), (Format-Bytes $st.GeladenBytes), (Format-Zahl $st.Uebersprungen), $st.Fehlgeschlagen.Count
        $art = 'ok'
        if ($st.Fehlgeschlagen.Count -gt 0) { $art = 'warnung' }
        Write-Log $txt $art
        if ($st.Hinweis) { Write-Log "     $($st.Hinweis)" }
        foreach ($f in $st.Fehlgeschlagen) { $alleFehler.Add($f) }
    }
    Write-Log ("  Dauer: {0:hh\:mm\:ss}" -f $dauer)
    $fehlerDatei = Join-Path $Script:ZielDir 'fehlgeschlagen.txt'
    if ($alleFehler.Count -gt 0) {
        [IO.File]::WriteAllLines($fehlerDatei, $alleFehler, (New-Object Text.UTF8Encoding $false))
        Write-Log ("  {0} Dateien sind fehlgeschlagen (Liste: {1}). Einfach install.bat erneut starten, dann werden nur diese nachgeladen." -f $alleFehler.Count, $fehlerDatei) 'warnung'
        return 2
    }
    if (Test-Path -LiteralPath $fehlerDatei) { Remove-Item -LiteralPath $fehlerDatei -Force }
    Write-Log '  Alle Dateien wurden erfolgreich geladen und geprüft.' 'ok'
    return 0
}

$Script:Fertig = $false
$exitCode = 1
try {
    $exitCode = [int](@(Main)[-1])
    $Script:Fertig = $true
} catch {
    $art = Get-FehlerArt $_
    if ($art -eq 'requesterpays') {
        Show-RequesterPays
        $exitCode = 3
    } else {
        Write-Host ''
        $meldung = $_.Exception.Message
        if ($art -eq '') { $meldung = "Unerwarteter Fehler: $meldung" }
        Write-Log $meldung 'fehler'
        Write-Log 'Bereits geladene Dateien bleiben erhalten. Nach Behebung des Problems install.bat einfach erneut starten.' 'warnung'
        Write-Log ("Technische Details: " + $_.ScriptStackTrace) 'fehler' -NurDatei
        $exitCode = 1
    }
    $Script:Fertig = $true
} finally {
    if (-not $Script:Fertig) {
        Write-Log 'Abgebrochen. Beim nächsten Start von install.bat wird an dieser Stelle fortgesetzt.' 'warnung'
    }
    if ($Script:Log) { try { $Script:Log.Close() } catch { } }
}
exit $exitCode
