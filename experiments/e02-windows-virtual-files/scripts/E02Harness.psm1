<#
  E-02 harness safety primitives (Windows PowerShell 5.1).

  Rules enforced here:
  - A test root is used only if it is a dedicated, marker-identified directory on a fixed local
    drive, outside protected locations, with no reparse point anywhere in its path.
  - Every run gets a unique run id (timestamp + random suffix) and a marker file. Cleanup
    deletes only the run's expected synthetic files by exact name, then removes directories
    that are already empty. It never uses -Recurse, never follows or deletes through reparse
    points, and leaves (and reports) anything unexpected.
  - Processes are stopped only if they are experiment-owned: started by the harness, same PID,
    same start time, same image path.
#>
Set-StrictMode -Version 2

$script:TestRootMarker = '.onecopy-e02-testroot'
$script:RunMarker = '.onecopy-e02-run'
$script:RunIdPattern = '^A-0\d-\d{8}T\d{6}-[0-9a-f]{8}$'

function Test-IsReparsePoint {
    param([Parameter(Mandatory = $true)][string] $Path)
    $item = Get-Item -LiteralPath $Path -Force -ErrorAction Stop
    return [bool]($item.Attributes -band [IO.FileAttributes]::ReparsePoint)
}

function Assert-NoReparseInChain {
    <# Throws if the path or any existing ancestor is a reparse point (junction, symlink, mount). #>
    param([Parameter(Mandatory = $true)][string] $Path)
    $p = [IO.Path]::GetFullPath($Path)
    while ($p) {
        if ((Test-Path -LiteralPath $p) -and (Test-IsReparsePoint $p)) { throw "reparse point in path chain: $p" }
        $parent = [IO.Path]::GetDirectoryName($p)
        if (-not $parent -or $parent -eq $p) { break }
        $p = $parent
    }
}

function Test-PathWithin {
    param([string] $Child, [string] $Parent)
    if (-not $Child -or -not $Parent) { return $false }
    $c = [IO.Path]::GetFullPath($Child).TrimEnd('\') + '\'
    $p = [IO.Path]::GetFullPath($Parent).TrimEnd('\') + '\'
    return $c.StartsWith($p, [StringComparison]::OrdinalIgnoreCase)
}

function Assert-SafeTestRoot {
    <#
      Returns the full path of a validated test root. With -Create, a missing root (or an
      existing EMPTY directory) is initialized with the marker. An existing non-empty directory
      without the marker is never adopted.
    #>
    param([Parameter(Mandatory = $true)][string] $Path, [switch] $Create)
    $full = [IO.Path]::GetFullPath($Path).TrimEnd('\')
    if ($full.Length -le 3) { throw 'test root must not be a drive root' }
    $protected = @($env:USERPROFILE, $env:SystemRoot, $env:ProgramFiles, ${env:ProgramFiles(x86)}, $env:ProgramData, $env:PUBLIC) |
        Where-Object { $_ }
    foreach ($prot in $protected) {
        if ((Test-PathWithin $full $prot) -or (Test-PathWithin $prot $full)) { throw 'test root overlaps a protected location' }
    }
    $drive = New-Object IO.DriveInfo ([IO.Path]::GetPathRoot($full))
    if ($drive.DriveType -ne [IO.DriveType]::Fixed) { throw 'test root must be on a fixed local drive' }
    Assert-NoReparseInChain $full
    $marker = Join-Path $full $script:TestRootMarker
    if (Test-Path -LiteralPath $full) {
        if (-not (Test-Path -LiteralPath $full -PathType Container)) { throw 'test root path is not a directory' }
        if (-not (Test-Path -LiteralPath $marker -PathType Leaf)) {
            $entries = @(Get-ChildItem -LiteralPath $full -Force)
            if ($entries.Count -gt 0) { throw 'existing directory is not an E-02 test root (no marker); refusing to use it' }
            if (-not $Create) { throw 'test root has no marker' }
        }
    } elseif (-not $Create) {
        throw 'test root does not exist'
    } else {
        New-Item -ItemType Directory -Path $full | Out-Null
    }
    if (-not (Test-Path -LiteralPath $marker)) {
        Set-Content -LiteralPath $marker -Encoding ASCII -Value 'OneCopy E-02 experiment test root. Only harness-created run folders under dest\ are ever removed.'
    }
    return $full
}

function New-RunLayout {
    param(
        [Parameter(Mandatory = $true)][string] $TestRoot,
        [Parameter(Mandatory = $true)][ValidatePattern('^A-0\d$')][string] $Case,
        [ValidateRange(1, 4)][int] $Pastes = 1
    )
    $root = Assert-SafeTestRoot -Path $TestRoot
    $suffix = [guid]::NewGuid().ToString('N').Substring(0, 8)
    $runId = "$Case-" + (Get-Date -Format 'yyyyMMddTHHmmss') + "-$suffix"
    $destParent = Join-Path $root 'dest'
    if (-not (Test-Path -LiteralPath $destParent)) { New-Item -ItemType Directory -Path $destParent | Out-Null }
    Assert-NoReparseInChain $destParent
    $runDest = Join-Path $destParent $runId
    if (Test-Path -LiteralPath $runDest) { throw 'run destination already exists' }
    New-Item -ItemType Directory -Path $runDest | Out-Null
    Set-Content -LiteralPath (Join-Path $runDest $script:RunMarker) -Value $runId -Encoding ASCII
    $targets = @()
    for ($i = 1; $i -le $Pastes; $i++) {
        $t = Join-Path $runDest "paste-target-$i"
        New-Item -ItemType Directory -Path $t | Out-Null
        $targets += $t
    }
    return [pscustomobject]@{ RunId = $runId; TestRoot = $root; RunDest = $runDest; Targets = $targets }
}

function Remove-RunDestination {
    <#
      Deletes only the expected synthetic files of one run, then removes directories that are
      empty. Returns what was deleted and what was left (never throws; a refusal is reported).
    #>
    param(
        [Parameter(Mandatory = $true)][string] $TestRoot,
        [Parameter(Mandatory = $true)][string] $RunId,
        [Parameter(Mandatory = $true)][AllowEmptyCollection()][string[]] $ExpectedFileNames,
        [ValidateRange(1, 4)][int] $Pastes = 1
    )
    $result = [ordered]@{ deleted_files = 0; removed_dirs = 0; left_entries = @(); refused = $null }
    try {
        if ($RunId -notmatch $script:RunIdPattern) { throw 'run id has an unexpected form' }
        foreach ($name in $ExpectedFileNames) {
            if ($name -match '[\\/:]' -or $name -match '^\.+$' -or -not $name) { throw "unexpected expected-file name: $name" }
        }
        $root = Assert-SafeTestRoot -Path $TestRoot
        $runDest = Join-Path (Join-Path $root 'dest') $RunId
        if (-not (Test-Path -LiteralPath $runDest -PathType Container)) { throw 'run destination missing' }
        Assert-NoReparseInChain $runDest
        $marker = Join-Path $runDest $script:RunMarker
        if (-not (Test-Path -LiteralPath $marker -PathType Leaf)) { throw 'run marker missing' }
        if ((Get-Content -LiteralPath $marker -TotalCount 1) -ne $RunId) { throw 'run marker does not match the run id' }

        for ($i = 1; $i -le $Pastes; $i++) {
            $label = "paste-target-$i"
            $t = Join-Path $runDest $label
            if (-not (Test-Path -LiteralPath $t)) { continue }
            if (Test-IsReparsePoint $t) { $result.left_entries += "$label (reparse point, not entered)"; continue }
            foreach ($name in $ExpectedFileNames) {
                $f = Join-Path $t $name
                if (-not (Test-Path -LiteralPath $f)) { continue }
                $item = Get-Item -LiteralPath $f -Force
                $isReparse = [bool]($item.Attributes -band [IO.FileAttributes]::ReparsePoint)
                if ($item.PSIsContainer -or $isReparse) { $result.left_entries += "$label\$name (not a regular file)"; continue }
                Assert-NoReparseInChain $f   # re-check immediately before deleting
                Remove-Item -LiteralPath $f -Force
                $result.deleted_files++
            }
            $remaining = @(Get-ChildItem -LiteralPath $t -Force)
            if ($remaining.Count -eq 0) {
                Remove-Item -LiteralPath $t -Force    # empty directory; no -Recurse
                $result.removed_dirs++
            } else {
                foreach ($r in $remaining) { $result.left_entries += "$label\$($r.Name)" }
            }
        }
        $rest = @(Get-ChildItem -LiteralPath $runDest -Force | Where-Object { $_.Name -ne $script:RunMarker })
        if ($rest.Count -eq 0) {
            Remove-Item -LiteralPath $marker -Force
            Remove-Item -LiteralPath $runDest -Force
            $result.removed_dirs++
        } else {
            foreach ($r in $rest) {
                if ($r.Name -notmatch '^paste-target-\d+$' -or -not ($result.left_entries -match "^$([regex]::Escape($r.Name))")) {
                    $result.left_entries += $r.Name
                }
            }
        }
    } catch {
        $result.refused = $_.Exception.Message
    }
    return [pscustomobject]$result
}

function Start-OwnedProcess {
    param([Parameter(Mandatory = $true)][string] $FilePath, [string[]] $ArgumentList = @())
    $path = (Resolve-Path -LiteralPath $FilePath).Path
    $p = Start-Process -FilePath $path -ArgumentList $ArgumentList -PassThru -WindowStyle Hidden
    $null = $p.Handle    # holding a handle keeps the PID from being reused and makes ExitCode reliable
    return [pscustomobject]@{ Process = $p; Id = $p.Id; StartTime = $p.StartTime; Path = $path }
}

function Stop-OwnedProcess {
    <# Returns: none | exited | stopped | identity-mismatch-not-stopped #>
    param($Owned, [int] $GraceMs = 0)
    if ($null -eq $Owned) { return 'none' }
    $p = $Owned.Process
    if ($GraceMs -gt 0 -and $p.WaitForExit($GraceMs)) { return 'exited' }
    if ($p.HasExited) { return 'exited' }
    $live = Get-Process -Id $Owned.Id -ErrorAction SilentlyContinue
    if (-not $live) { return 'exited' }
    $samePath = (-not $live.Path) -or ($live.Path -eq $Owned.Path)
    if ($live.StartTime -ne $Owned.StartTime -or -not $samePath) { return 'identity-mismatch-not-stopped' }
    Stop-Process -Id $Owned.Id -Force
    $null = $p.WaitForExit(5000)
    return 'stopped'
}

function Write-DurableJson {
    <# Writes JSON via a temporary file and an atomic rename, so a record is never half-written. #>
    param([Parameter(Mandatory = $true)] $Object, [Parameter(Mandatory = $true)][string] $Path)
    $dir = Split-Path -Parent $Path
    if (-not (Test-Path -LiteralPath $dir)) { New-Item -ItemType Directory -Force -Path $dir | Out-Null }
    $tmp = "$Path.tmp"
    $Object | ConvertTo-Json -Depth 12 | Set-Content -Encoding UTF8 -LiteralPath $tmp
    Move-Item -LiteralPath $tmp -Destination $Path -Force
}

Export-ModuleMember -Function Test-IsReparsePoint, Assert-NoReparseInChain, Test-PathWithin, Assert-SafeTestRoot,
    New-RunLayout, Remove-RunDestination, Start-OwnedProcess, Stop-OwnedProcess, Write-DurableJson
