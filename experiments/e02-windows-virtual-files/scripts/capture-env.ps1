<#
.SYNOPSIS
  E-02 environment record (read-only). Never records user names, machine names or profile paths.
.PARAMETER Out
  Output JSON path.
.PARAMETER TestRoot
  Test root whose volume is described.
#>
param(
    [Parameter(Mandatory = $true)][string] $Out,
    [string] $TestRoot = 'D:\OneCopyE02'
)
$ErrorActionPreference = 'Stop'
$ws = Split-Path -Parent $PSScriptRoot

function TryGet([scriptblock] $Block) { try { & $Block } catch { $null } }

$os = Get-CimInstance Win32_OperatingSystem
$cv = Get-ItemProperty 'HKLM:\SOFTWARE\Microsoft\Windows NT\CurrentVersion'
$adv = TryGet { Get-ItemProperty 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Explorer\Advanced' }
$clip = TryGet { Get-ItemProperty 'HKCU:\Software\Microsoft\Clipboard' }
$mp = TryGet { Get-MpComputerStatus }
$drive = (Split-Path -Qualifier $TestRoot).TrimEnd(':')
$vol = TryGet { Get-Volume -DriveLetter $drive }
$cpu = TryGet { (Get-CimInstance Win32_Processor | Select-Object -First 1).Name.Trim() }
$ramGiB = TryGet { [math]::Round((Get-CimInstance Win32_ComputerSystem).TotalPhysicalMemory / 1GB, 1) }
$git = TryGet { (& git -C $ws rev-parse HEAD).Trim() }
$dirty = TryGet { [bool](& git -C $ws status --porcelain -- crates cases scripts Cargo.toml Cargo.lock) }
$rustc = TryGet { (& rustc --version).Trim() }
$lockHash = TryGet { (Get-FileHash -Algorithm SHA256 (Join-Path $ws 'Cargo.lock')).Hash.ToLower() }

function PropOrNull($obj, [string] $name) {
    if ($null -eq $obj) { return $null }
    $p = $obj.PSObject.Properties[$name]
    if ($null -eq $p) { return $null }
    return $p.Value
}

$record = [ordered]@{
    os_caption                   = $os.Caption
    os_version                   = $os.Version
    os_build                     = $os.BuildNumber
    os_ubr                       = PropOrNull $cv 'UBR'
    os_display_version           = PropOrNull $cv 'DisplayVersion'
    explorer_version             = (Get-Item "$env:WINDIR\explorer.exe").VersionInfo.ProductVersion
    explorer_separate_process    = PropOrNull $adv 'SeparateProcess'
    clipboard_history_enabled    = PropOrNull $clip 'EnableClipboardHistory'
    cloud_clipboard_enabled      = PropOrNull $clip 'EnableCloudClipboard'
    defender_product_version     = PropOrNull $mp 'AMProductVersion'
    defender_realtime_protection = PropOrNull $mp 'RealTimeProtectionEnabled'
    test_volume_filesystem       = PropOrNull $vol 'FileSystemType'
    test_volume_free_gib         = $(if ($vol) { [math]::Round($vol.SizeRemaining / 1GB, 1) } else { $null })
    cpu                          = $cpu
    ram_gib                      = $ramGiB
    experiment_commit            = $git
    experiment_tree_dirty        = $dirty
    rustc                        = $rustc
    cargo_lock_sha256            = $lockHash
}
$record | ConvertTo-Json -Depth 4 | Set-Content -Encoding UTF8 -Path $Out
