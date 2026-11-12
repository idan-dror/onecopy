<#
.SYNOPSIS
  Runs one E-02 Stage A case end to end and writes a sanitized run record.

.DESCRIPTION
  1. Starts a directory observer per destination and the provider agent.
  2. Opens an Explorer window per destination and pastes:
     - Driver Manual (D1): the operator presses Ctrl+V in the opened window (ground truth).
     - Driver ExplorerCommand (D2c): posts the folder-view Paste command to that window only
       (experiment-only automation, D-041; no global keystrokes).
  3. Waits until every expected file reaches its expected size, then stops the agent (exit mode
     "clear"), records the clipboard formats left behind, verifies digests and provenance,
     closes the Explorer windows and deletes the synthetic destination files.

  Running this REPLACES the current clipboard contents. Synthetic data only.

.EXAMPLE
  .\scripts\run-stage-a.ps1 -Case A-01 -Driver ExplorerCommand
#>
param(
    [Parameter(Mandatory = $true)][ValidatePattern('^A-0[1-6]$')][string] $Case,
    [ValidateSet('Manual', 'ExplorerCommand')][string] $Driver = 'ExplorerCommand',
    [ValidateRange(1, 4)][int] $Pastes = 1,
    [string] $TestRoot = 'D:\OneCopyE02',
    [int] $TimeoutSecs = 180,
    [switch] $KeepDestination
)
$ErrorActionPreference = 'Stop'
$ws = Split-Path -Parent $PSScriptRoot
$bin = Join-Path $ws 'target\release'
$agentExe = Join-Path $bin 'e02-agent.exe'
$verifyExe = Join-Path $bin 'e02-verify.exe'
$caseFile = Join-Path $ws "cases\$Case.toml"

# --- Safety checks -------------------------------------------------------------------------
foreach ($exe in @($agentExe, $verifyExe)) {
    if (-not (Test-Path $exe)) { throw "Missing $exe. Run: cargo build --release --locked" }
}
$rootFull = [IO.Path]::GetFullPath($TestRoot).TrimEnd('\')
if ($rootFull.Length -le 3) { throw 'TestRoot must not be a drive root.' }
if ($env:USERPROFILE -and $rootFull.StartsWith($env:USERPROFILE, [StringComparison]::OrdinalIgnoreCase)) {
    throw 'TestRoot must not be inside the user profile.'
}

# --- Run layout ----------------------------------------------------------------------------
$runId = "$Case-" + (Get-Date -Format 'yyyyMMddTHHmmss')
$runDir = Join-Path $ws "evidence\$runId"
New-Item -ItemType Directory -Force -Path $runDir | Out-Null
$stopFile = Join-Path $runDir 'stop'
$expected = (& $verifyExe case-files --case $caseFile | Out-String | ConvertFrom-Json).files

$dests = @()
for ($i = 1; $i -le $Pastes; $i++) {
    $d = Join-Path $rootFull "dest\$runId\p$i"
    New-Item -ItemType Directory -Force -Path $d | Out-Null
    $dests += $d
}
Write-Host "run $runId  case $Case  driver $Driver  pastes $Pastes"

# --- Start observers and the agent ---------------------------------------------------------
$watchers = @()
foreach ($d in $dests) {
    $leaf = Split-Path $d -Leaf
    $watchers += Start-Process -FilePath $verifyExe -PassThru -WindowStyle Hidden -ArgumentList @(
        'watch', '--dest', "`"$d`"", '--out', "`"$(Join-Path $runDir "observer-$leaf.jsonl")`"",
        '--stop-file', "`"$stopFile`"", '--timeout-secs', [string]($TimeoutSecs + 120))
}
$agent = Start-Process -FilePath $agentExe -PassThru -WindowStyle Hidden -ArgumentList @(
    '--case', "`"$caseFile`"", '--run-dir', "`"$runDir`"", '--run-id', $runId,
    '--hold-secs', [string]($TimeoutSecs + 60), '--exit-mode', 'clear')
$null = $agent.Handle  # cache the handle so ExitCode is available later (PowerShell 5.1)

$readyDeadline = (Get-Date).AddSeconds(20)
while (-not (Test-Path (Join-Path $runDir 'ready'))) {
    if ($agent.HasExited) { throw "agent exited early with code $($agent.ExitCode)" }
    if ((Get-Date) -gt $readyDeadline) { throw 'agent did not become ready' }
    Start-Sleep -Milliseconds 100
}

# --- Paste into each destination ----------------------------------------------------------
function Test-Complete([string] $dest) {
    foreach ($f in $expected) {
        $p = Join-Path $dest $f.name
        if (-not (Test-Path -LiteralPath $p)) { return $false }
        if ((Get-Item -LiteralPath $p).Length -ne [int64]$f.size) { return $false }
    }
    return $true
}

$completion = @()
foreach ($d in $dests) {
    $start = Get-Date
    Start-Process -FilePath 'explorer.exe' -ArgumentList "`"$d`""
    if ($Driver -eq 'ExplorerCommand') {
        $driverOut = & $verifyExe explorer-paste --dest $d 2>&1 | Out-String
        Write-Host "driver: $($driverOut.Trim())"
    } else {
        Write-Host "D1: press Ctrl+V in the Explorer window showing $d"
    }
    $done = $false
    $deadline = $start.AddSeconds($TimeoutSecs)
    while ((Get-Date) -lt $deadline) {
        if (Test-Complete $d) { $done = $true; break }
        Start-Sleep -Milliseconds 200
    }
    $completion += [ordered]@{
        destination = "p$($completion.Count + 1)"
        completed   = $done
        seconds     = [math]::Round(((Get-Date) - $start).TotalSeconds, 2)
    }
    Start-Sleep -Seconds 3  # let the target finish and release its streams
}

# --- Stop, inspect, verify ----------------------------------------------------------------
New-Item -ItemType File -Force -Path $stopFile | Out-Null
if (-not $agent.WaitForExit(30000)) { Stop-Process -Id $agent.Id -Force; throw 'agent did not exit' }
foreach ($w in $watchers) { $null = $w.WaitForExit(10000) }
$clipAfter = & $verifyExe clipboard-formats | Out-String | ConvertFrom-Json

$verifications = @()
for ($i = 0; $i -lt $dests.Count; $i++) {
    $out = Join-Path $runDir "verify-p$($i + 1).json"
    & $verifyExe verify --case $caseFile --dest $dests[$i] --out $out | Out-Null
    $verifications += (Get-Content -Raw $out | ConvertFrom-Json)
    & $verifyExe explorer-close --dest $dests[$i] 2>&1 | Out-Null
}

$observers = [ordered]@{}
foreach ($d in $dests) {
    $leaf = Split-Path $d -Leaf
    $lines = @(Get-Content (Join-Path $runDir "observer-$leaf.jsonl") -ErrorAction SilentlyContinue | ForEach-Object { $_ | ConvertFrom-Json })
    $observers[$leaf] = [ordered]@{
        events     = $lines.Count
        names_seen = @($lines | ForEach-Object { $_.name } | Sort-Object -Unique)
        removals   = @($lines | Where-Object { $_.change -eq 'removed' }).Count
    }
}

& (Join-Path $PSScriptRoot 'capture-env.ps1') -Out (Join-Path $runDir 'env.json') -TestRoot $rootFull
$envRecord = Get-Content -Raw (Join-Path $runDir 'env.json') | ConvertFrom-Json
$summary = Get-Content -Raw (Join-Path $runDir 'summary.json') | ConvertFrom-Json

$allMatch = -not ($verifications | Where-Object { -not $_.all_files_match })
$allCompleted = -not ($completion | Where-Object { -not $_.completed })
$clipClean = -not $clipAfter.has_onecopy_marker
$result = if ($allMatch -and $allCompleted -and $clipClean -and $agent.ExitCode -eq 0) { 'pass' } else { 'fail' }

$record = [ordered]@{
    experiment_id        = 'E-02'
    stage                = 'A'
    case_id              = $Case
    run_id               = $runId
    driver               = $(if ($Driver -eq 'Manual') { 'D1' } else { 'D2c' })
    driver_calibrated    = $(if ($Driver -eq 'Manual') { $true } else { $false })
    target               = 'G-01'
    pastes               = $Pastes
    env                  = $envRecord
    completion           = $completion
    agent_exit_code      = $agent.ExitCode
    agent_summary        = $summary
    destinations         = $verifications
    observer             = $observers
    clipboard_after_exit = $clipAfter
    result               = $result
}
$reportDir = Join-Path $ws 'reports\stage-a'
New-Item -ItemType Directory -Force -Path $reportDir | Out-Null
$record | ConvertTo-Json -Depth 12 | Set-Content -Encoding UTF8 -Path (Join-Path $reportDir "$runId.json")

if (-not $KeepDestination) {
    $runDest = Join-Path $rootFull "dest\$runId"
    if ($runDest.StartsWith($rootFull + '\dest\', [StringComparison]::OrdinalIgnoreCase)) {
        Remove-Item -LiteralPath $runDest -Recurse -Force
    }
}
Write-Host "result: $result  (record: reports\stage-a\$runId.json)"
