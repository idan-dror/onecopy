<#
.SYNOPSIS
  Runs one E-02 Stage A case end to end and ALWAYS writes a durable run record.

.DESCRIPTION
  1. Validates the marker-identified test root and creates a unique run layout
     (<root>\dest\<run-id>\paste-target-N, with a run marker).
  2. Snapshots existing Explorer windows (handles and tab counts only), then starts the directory
     observers and the provider agent. All are experiment-owned processes that watch this
     script's PID and exit if it disappears.
  3. For each paste target: opens Explorer at it, records whether the window is new or a tab in a
     pre-existing (user) window, then pastes:
       - Driver Manual (D1): the OPERATOR presses Ctrl+V in the window titled "paste-target-N".
         Nothing is simulated.
       - Driver ExplorerCommand (D2c): posts the folder-view Paste command to that window only
         (experiment-only automation, D-041).
  4. Always (finally): releases the clipboard via the stop file (the agent clears it only if its
     object is still current), stops only experiment-owned processes, records clipboard formats
     (read-only), verifies digests, closes the window only if it is experiment-owned with no
     other tabs, writes the record, and deletes the run's synthetic files only after a pass.

  Running this REPLACES the current clipboard contents. Synthetic data only.

.EXAMPLE
  .\scripts\run-stage-a.ps1 -Case A-01 -Driver Manual
#>
param(
    [Parameter(Mandatory = $true)][ValidatePattern('^A-0[1-6]$')][string] $Case,
    [ValidateSet('Manual', 'ExplorerCommand')][string] $Driver = 'ExplorerCommand',
    [ValidateRange(1, 4)][int] $Pastes = 1,
    [string] $TestRoot = 'D:\OneCopyE02',
    [int] $TimeoutSecs = 0,
    [switch] $KeepDestination,
    # Records go here (default reports\stage-a). Self-tests redirect them to evidence\.
    [string] $ReportDir = '',
    # Testing aid for harness self-tests only: throw at a named stage.
    [ValidateSet('none', 'after-layout', 'after-agent-ready', 'after-paste')][string] $InjectHarnessFault = 'none'
)
$ErrorActionPreference = 'Stop'
Import-Module (Join-Path $PSScriptRoot 'E02Harness.psm1') -Force
$ws = Split-Path -Parent $PSScriptRoot
$bin = Join-Path $ws 'target\release'
$agentExe = Join-Path $bin 'e02-agent.exe'
$verifyExe = Join-Path $bin 'e02-verify.exe'
$caseFile = Join-Path $ws "cases\$Case.toml"
if (-not $ReportDir) { $ReportDir = Join-Path $ws 'reports\stage-a' }
if ($TimeoutSecs -le 0) { $TimeoutSecs = $(if ($Driver -eq 'Manual') { 180 } else { 60 }) }

function Invoke-Native {
    <# Runs a native command without letting stderr abort the script; returns output and exit code. #>
    param([string] $Exe, [string[]] $Arguments)
    $prev = $ErrorActionPreference; $ErrorActionPreference = 'Continue'
    try { $out = & $Exe @Arguments 2>&1 | Out-String; $code = $LASTEXITCODE } finally { $ErrorActionPreference = $prev }
    return [pscustomobject]@{ Output = $out.Trim(); ExitCode = $code }
}
function Invoke-Fault([string] $Stage) { if ($InjectHarnessFault -eq $Stage) { throw "injected harness fault at $Stage" } }

$state = [ordered]@{ stage = 'init'; exception = $null; failed_stage = $null }
$layout = $null; $runDir = $null; $agent = $null; $watchers = @(); $expected = @()
$completion = @(); $driverErrors = @(); $windows = @(); $preSnap = $null
$runId = $null

try {
    foreach ($exe in @($agentExe, $verifyExe)) { if (-not (Test-Path $exe)) { throw "Missing $exe. Run: cargo build --release --locked" } }
    $state.stage = 'layout'
    $rootFull = Assert-SafeTestRoot -Path $TestRoot -Create
    $layout = New-RunLayout -TestRoot $rootFull -Case $Case -Pastes $Pastes
    $runId = $layout.RunId
    $runDir = Join-Path $ws "evidence\$runId"
    New-Item -ItemType Directory -Force -Path $runDir | Out-Null
    $stopFile = Join-Path $runDir 'stop'
    $expected = @((Invoke-Native $verifyExe @('case-files', '--case', $caseFile)).Output | ConvertFrom-Json).files
    $preSnap = Join-Path $runDir 'explorer-pre.json'
    $snap = Invoke-Native $verifyExe @('explorer-snapshot', '--out', $preSnap)
    if ($snap.ExitCode -ne 0) { throw "explorer snapshot failed: $($snap.Output)" }
    Write-Host "run $runId  case $Case  driver $Driver  pastes $Pastes"
    Invoke-Fault 'after-layout'

    $state.stage = 'start-processes'
    foreach ($t in $layout.Targets) {
        $leaf = Split-Path $t -Leaf
        $watchers += Start-OwnedProcess -FilePath $verifyExe -ArgumentList @(
            'watch', '--dest', "`"$t`"", '--out', "`"$(Join-Path $runDir "observer-$leaf.jsonl")`"",
            '--stop-file', "`"$stopFile`"", '--timeout-secs', [string]($TimeoutSecs * $Pastes + 120), '--parent-pid', [string]$PID)
    }
    $agent = Start-OwnedProcess -FilePath $agentExe -ArgumentList @(
        '--case', "`"$caseFile`"", '--run-dir', "`"$runDir`"", '--run-id', $runId,
        '--hold-secs', [string]($TimeoutSecs * $Pastes + 60), '--exit-mode', 'clear', '--parent-pid', [string]$PID)
    $readyDeadline = (Get-Date).AddSeconds(20)
    while (-not (Test-Path (Join-Path $runDir 'ready'))) {
        if ($agent.Process.HasExited) { throw "agent exited before becoming ready (exit code $($agent.Process.ExitCode))" }
        if ((Get-Date) -gt $readyDeadline) { throw 'agent did not become ready' }
        Start-Sleep -Milliseconds 100
    }
    Invoke-Fault 'after-agent-ready'

    $state.stage = 'paste'
    $n = 0
    foreach ($t in $layout.Targets) {
        $n++
        $start = Get-Date
        $driverError = $null
        Start-Process -FilePath 'explorer.exe' -ArgumentList "`"$t`""
        $found = (Invoke-Native $verifyExe @('explorer-find', '--dest', $t, '--pre-snapshot', $preSnap)).Output | ConvertFrom-Json
        $windows += [ordered]@{
            destination     = "paste-target-$n"
            found           = [bool]$found.found
            top_hwnd        = $(if ($found.found) { $found.top_hwnd } else { $null })
            top_preexisting = $(if ($found.found) { $found.top_preexisting } else { $null })
            tabs_on_top     = $(if ($found.found) { $found.tabs_on_top } else { $null })
            close_decision  = $null
        }
        if (-not $found.found) {
            $driverError = 'Explorer window for the paste target was not found'
        } elseif ($Driver -eq 'ExplorerCommand') {
            $r = Invoke-Native $verifyExe @('explorer-paste', '--dest', $t, '--expected-top', [string]$found.top_hwnd)
            Write-Host "driver: $($r.Output)"
            if ($r.ExitCode -ne 0) { $driverError = $r.Output }
        } else {
            Write-Host ''
            Write-Host "  >>> OPERATOR: click inside the Explorer window titled 'paste-target-$n' and press Ctrl+V once."
            Write-Host "  >>> Then keep hands off until this run reports a result (timeout $TimeoutSecs s)."
            Write-Host ''
        }
        if ($driverError) { $driverErrors += $driverError }
        $done = $false
        if (-not $driverError) {
            $deadline = $start.AddSeconds($TimeoutSecs)
            while ((Get-Date) -lt $deadline) {
                $all = $true
                foreach ($f in $expected) {
                    $p = Join-Path $t $f.name
                    if (-not (Test-Path -LiteralPath $p) -or (Get-Item -LiteralPath $p).Length -ne [int64]$f.size) { $all = $false; break }
                }
                if ($all) { $done = $true; break }
                Start-Sleep -Milliseconds 200
            }
        }
        $completion += [ordered]@{
            destination  = "paste-target-$n"
            completed    = $done
            seconds      = [math]::Round(((Get-Date) - $start).TotalSeconds, 2)
            driver_error = $driverError
        }
        Start-Sleep -Seconds 3  # let the target finish and release its streams
    }
    Invoke-Fault 'after-paste'
    $state.stage = 'finished-pastes'
} catch {
    $state.exception = $_.Exception.Message
    $state.failed_stage = $state.stage
    Write-Host "harness error at stage '$($state.stage)': $($state.exception)"
} finally {
    $post = [ordered]@{}
    # 1. Release the clipboard and stop experiment-owned processes (never by name).
    try { if ($runDir) { New-Item -ItemType File -Force -Path (Join-Path $runDir 'stop') | Out-Null } } catch { $post.stop_file_error = $_.Exception.Message }
    $post.agent_stop = Stop-OwnedProcess -Owned $agent -GraceMs 30000
    $post.watcher_stops = @($watchers | ForEach-Object { Stop-OwnedProcess -Owned $_ -GraceMs 10000 })
    $agentExit = $(if ($agent) { $agent.Process.ExitCode } else { $null })

    # 2. Read-only inspection, verification and owned-window cleanup.
    $clipAfter = $null; $verifications = @(); $summary = $null; $envRecord = $null; $observers = [ordered]@{}
    if ($runDir) {
        try { $clipAfter = (Invoke-Native $verifyExe @('clipboard-formats')).Output | ConvertFrom-Json } catch { $post.clipboard_error = $_.Exception.Message }
        $i = 0
        $targets = @(); if ($layout) { $targets = @($layout.Targets) }
        foreach ($t in $targets) {
            $i++
            try {
                $out = Join-Path $runDir "verify-p$i.json"
                $null = Invoke-Native $verifyExe @('verify', '--case', $caseFile, '--dest', $t, '--out', $out)
                $verifications += (Get-Content -Raw $out | ConvertFrom-Json)
            } catch { $post["verify_error_p$i"] = $_.Exception.Message }
            if ($i -le $windows.Count -and $windows[$i - 1].found) {
                $c = Invoke-Native $verifyExe @('explorer-close', '--dest', $t, '--pre-snapshot', $preSnap, '--expected-top', [string]$windows[$i - 1].top_hwnd)
                try { $windows[$i - 1].close_decision = ($c.Output | ConvertFrom-Json).decision } catch { $windows[$i - 1].close_decision = "error: $($c.Output)" }
            }
            $leaf = Split-Path $t -Leaf
            $lines = @(Get-Content (Join-Path $runDir "observer-$leaf.jsonl") -ErrorAction SilentlyContinue | ForEach-Object { $_ | ConvertFrom-Json })
            $observers[$leaf] = [ordered]@{
                events                 = $lines.Count
                names_seen             = @($lines | Where-Object { $_.PSObject.Properties['name'] } | ForEach-Object { $_.name } | Sort-Object -Unique)
                removals               = @($lines | Where-Object { $_.change -eq 'removed' }).Count
                stopped_by_parent_exit = [bool](@($lines | Where-Object { $_.change -eq 'stopped' }).Count)
            }
        }
        try {
            & (Join-Path $PSScriptRoot 'capture-env.ps1') -Out (Join-Path $runDir 'env.json') -TestRoot $TestRoot
            $envRecord = Get-Content -Raw (Join-Path $runDir 'env.json') | ConvertFrom-Json
        } catch { $post.env_error = $_.Exception.Message }
        $summaryFile = Join-Path $runDir 'summary.json'
        if (Test-Path $summaryFile) { $summary = Get-Content -Raw $summaryFile | ConvertFrom-Json }
    }

    # 3. Classification. Harness errors and interference mean the paste outcome was not observed.
    $interference = @()
    if ($summary -and $summary.clipboard_lost_ms) { $interference += "clipboard replaced by another process at $($summary.clipboard_lost_ms) ms" }
    for ($k = 0; $k -lt $completion.Count; $k++) {
        $w = $null; if ($k -lt $windows.Count) { $w = $windows[$k] }
        if (-not $completion[$k].completed -and $w -and $w.close_decision -eq 'not_found') { $interference += "test window for paste-target-$($k + 1) closed before completion" }
    }
    $allMatch = ($verifications.Count -gt 0) -and -not ($verifications | Where-Object { -not $_.all_files_match })
    $allCompleted = ($completion.Count -eq $Pastes) -and -not ($completion | Where-Object { -not $_.completed })
    $clipClean = [bool]$clipAfter -and -not $clipAfter.has_onecopy_marker
    $result = 'fail'
    if ($state.exception -or $driverErrors.Count -gt 0 -or $interference.Count -gt 0) { $result = 'inconclusive' }
    elseif ($allMatch -and $allCompleted -and $clipClean -and $agentExit -eq 0) { $result = 'pass' }

    # 4. Destination cleanup: only after a pass (otherwise kept for diagnosis), and only safely.
    $cleanup = $null
    if ($layout -and $result -eq 'pass' -and -not $KeepDestination) {
        $cleanup = Remove-RunDestination -TestRoot $layout.TestRoot -RunId $layout.RunId -ExpectedFileNames @($expected | ForEach-Object { $_.name }) -Pastes $Pastes
    }

    # 5. Durable record, always.
    $recordId = $runId; if (-not $recordId) { $recordId = "$Case-unstarted-" + (Get-Date -Format 'yyyyMMddTHHmmss') }
    $record = [ordered]@{
        experiment_id        = 'E-02'
        stage                = 'A'
        case_id              = $Case
        run_id               = $recordId
        driver               = $(if ($Driver -eq 'Manual') { 'D1' } else { 'D2c' })
        target               = 'G-01'
        pastes               = $Pastes
        injected_fault       = $InjectHarnessFault
        env                  = $envRecord
        completion           = $completion
        explorer_windows     = $windows
        agent_exit_code      = $agentExit
        agent_summary        = $summary
        destinations         = $verifications
        observer             = $observers
        clipboard_after_exit = $clipAfter
        harness              = [ordered]@{ failed_stage = $state.failed_stage; exception = $state.exception; driver_errors = $driverErrors; post = $post }
        interference         = $interference
        destination_cleanup  = $cleanup
        destination_kept     = [bool]($layout -and -not $cleanup)
        result               = $result
    }
    $recordPath = Join-Path $ReportDir "$recordId.json"
    try { Write-DurableJson -Object $record -Path $recordPath } catch { Write-Host "FAILED to write record: $($_.Exception.Message)" }
    Write-Host "result: $result  (record: $recordPath)"
}
