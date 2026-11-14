<#
.SYNOPSIS
  Runs one E-02 Stage B1 fault case end to end and ALWAYS writes a durable run record.

.DESCRIPTION
  Stage B1 = fault handling and correctness on bounded synthetic files (<= 256 MiB per case).
  Large-file cases (Stage B2) are refused.

  1. Validates the marker-identified test root and creates a unique run layout (Stage A safety
     layer). Optionally pre-creates synthetic "original" files at the destination (ERR-08).
  2. Starts experiment-owned observers: destination watchers, the read-only Explorer dialog
     watcher and, if the case asks for it, the clipboard latency probe (STREAM-08). All watch
     this script's PID and exit if it disappears.
  3. Starts the provider with the case's fault plan and exit mode, then pastes:
       - Driver ExplorerCommand (D2c): folder-view Paste command posted to the run's own window.
       - Driver Manual (D1): the OPERATOR presses Ctrl+V. Nothing is simulated.
  4. While the copy runs, applies the case's harness actions:
       - kill the provider by PID and identity after a provider event (PROC-02);
       - replace the clipboard with tagged synthetic text after a provider event (LIFE-03);
       - close the run's own window after a provider event (CANCEL-04);
       - answer Explorer dialogs: DialogDriver Uia invokes one named button in a dialog that
         mentions this run (experiment-only automation, D-041); DialogDriver Manual leaves it
         to the operator.
  5. Always (finally): stop file, stop only owned processes, release the clipboard only if it
     holds this run's tagged content, verify every destination (complete, partial prefix,
     wrong content, original kept), close only the run's own window, classify the outcome
     (spec §9.1, O1-O5), write the record, and delete only this run's expected file names.

  Running this REPLACES the current clipboard contents. Synthetic data only.
#>
param(
    [Parameter(Mandatory = $true)][ValidatePattern('^[A-Z]{1,6}-\d{2}[a-z0-9]{0,4}$')][string] $Case,
    [ValidateSet('Manual', 'ExplorerCommand')][string] $Driver = 'ExplorerCommand',
    [ValidateSet('Manual', 'Uia')][string] $DialogDriver = 'Uia',
    [string] $TestRoot = 'D:\OneCopyE02',
    [switch] $KeepDestination,
    [string] $ReportDir = '',
    [ValidateSet('none', 'after-layout', 'after-agent-ready', 'after-paste')][string] $InjectHarnessFault = 'none'
)
$ErrorActionPreference = 'Stop'
Import-Module (Join-Path $PSScriptRoot 'E02Harness.psm1') -Force
$ws = Split-Path -Parent $PSScriptRoot
$bin = Join-Path $ws 'target\release'
$agentExe = Join-Path $bin 'e02-agent.exe'
$verifyExe = Join-Path $bin 'e02-verify.exe'
$caseFile = Join-Path $ws "cases\$Case.toml"
if (-not $ReportDir) { $ReportDir = Join-Path $ws 'reports\stage-b1' }

# Dialog actions map to Win32 control ids (UIA automation ids), so the UI language does not
# matter: 1 = IDOK, 2 = IDCANCEL, 4 = IDRETRY, 5 = IDIGNORE, 10 = IDTRYAGAIN, 11 = IDCONTINUE.
# "cancel" on an error box that only offers OK dismisses it (the operation has already failed).
# Observed on this build: the progress dialog's Cancel is "CancelButton"; the copy error box is
# a task dialog whose OK is "CommandButton_1" (CommandButton_<Win32 id>).
$ActionIds = @{
    cancel    = @('CancelButton', 'btnCancel', 'CommandButton_2', '2', 'CommandButton_1', '1', 'Close')
    try_again = @('CommandButton_10', 'CommandButton_4', '10', '4')
    skip      = @('CommandButton_5', 'CommandButton_11', '5', '11')
    # Conflict dialog ("Copy File"): tile 1 = copy and replace, tile 2 = don't copy; both "tileMain".
    replace   = @('tileMain#1', 'CommandButton_6', '6')
    keep_both = @('CommandButton_7', '7')
}

function Invoke-Native {
    param([string] $Exe, [string[]] $Arguments)
    $prev = $ErrorActionPreference; $ErrorActionPreference = 'Continue'
    try { $out = & $Exe @Arguments 2>&1 | Out-String; $code = $LASTEXITCODE } finally { $ErrorActionPreference = $prev }
    return [pscustomobject]@{ Output = $out.Trim(); ExitCode = $code }
}
# True if a dialog offering these automation ids has a button for the action ('id#n' compares by id).
function Test-OffersAction($Ids, [string] $Action) {
    $wanted = @($ActionIds[$Action] | ForEach-Object { ($_ -split '#')[0] })
    return [bool](@($Ids | Where-Object { $wanted -contains $_ }).Count)
}
function Invoke-Fault([string] $Stage) { if ($InjectHarnessFault -eq $Stage) { throw "injected harness fault at $Stage" } }

# Incremental reader for a JSONL file another process is appending to.
function New-TailReader([string] $Path) { return [pscustomobject]@{ Path = $Path; Pos = [int64]0; Partial = '' } }
function Read-NewJson($Reader) {
    if (-not (Test-Path -LiteralPath $Reader.Path)) { return @() }
    $text = ''
    try {
        $fs = [IO.File]::Open($Reader.Path, [IO.FileMode]::Open, [IO.FileAccess]::Read, [IO.FileShare]'ReadWrite,Delete')
        try {
            $null = $fs.Seek($Reader.Pos, [IO.SeekOrigin]::Begin)
            $buf = New-Object byte[] ([int]([math]::Min(16MB, $fs.Length - $Reader.Pos)))
            $n = $fs.Read($buf, 0, $buf.Length)
            $Reader.Pos += $n
            $text = [Text.Encoding]::UTF8.GetString($buf, 0, $n)
        } finally { $fs.Dispose() }
    } catch { return @() }
    $lines = ($Reader.Partial + $text) -split "`n"
    $Reader.Partial = $lines[-1]
    $out = @()
    for ($i = 0; $i -lt $lines.Count - 1; $i++) {
        $l = $lines[$i].Trim()
        if ($l) { try { $out += ($l | ConvertFrom-Json) } catch { } }
    }
    return $out
}
function Read-DialogState([string] $Path) {
    try { return (Get-Content -Raw -LiteralPath $Path -ErrorAction Stop | ConvertFrom-Json) } catch { return $null }
}
# Crash detection watches the processes that matter: the shell (owner of Shell_TrayWnd) and the
# process hosting each of the run's own Explorer windows. Launching a folder also starts a
# short-lived explorer.exe that exits on its own; it is not counted.
Add-Type -Namespace E02 -Name Win -MemberDefinition @'
[DllImport("user32.dll", CharSet = CharSet.Unicode)] public static extern IntPtr FindWindow(string cls, string title);
[DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr hwnd, out uint pid);
'@
function Get-WindowProcess([int64] $Hwnd, [string] $Role) {
    $procId = [uint32]0
    $null = [E02.Win]::GetWindowThreadProcessId([IntPtr]$Hwnd, [ref]$procId)
    $p = $null; if ($procId) { $p = Get-Process -Id $procId -ErrorAction SilentlyContinue }
    if (-not $p) { return $null }
    return [ordered]@{ role = $Role; id = $p.Id; name = $p.ProcessName; start = $p.StartTime.ToString('o') }
}
function Get-ExplorerProcesses([int64[]] $Hwnds = @()) {
    $out = @()
    $shell = Get-WindowProcess ([E02.Win]::FindWindow('Shell_TrayWnd', $null).ToInt64()) 'shell'
    if ($shell) { $out += $shell }
    foreach ($h in $Hwnds) { $w = Get-WindowProcess $h 'run_window_host'; if ($w) { $out += $w } }
    return @($out)
}

$clock = [Diagnostics.Stopwatch]::StartNew()
$timeline = New-Object System.Collections.ArrayList
function Add-Timeline([string] $Source, [string] $Event, $Detail = $null) {
    [void]$timeline.Add([ordered]@{ t_ms = [math]::Round($clock.Elapsed.TotalMilliseconds); source = $Source; event = $Event; detail = $Detail })
}

$state = [ordered]@{ stage = 'init'; exception = $null; failed_stage = $null }
$layout = $null; $runDir = $null; $dialogState = $null; $ownerTops = @(); $tokens = @(); $rootFull = $null; $agent = $null; $watchers = @(); $dialogWatch = $null; $probe = $null; $helpers = @()
$expected = @(); $caseInfo = $null; $runId = $null; $windows = @(); $preSnap = $null; $driverErrors = @()
$agentEvents = New-Object System.Collections.ArrayList
$dialogEvents = New-Object System.Collections.ArrayList
$actions = New-Object System.Collections.ArrayList
$setSequence = $null; $explorerBefore = Get-ExplorerProcesses
$progress = [ordered]@{ completed = $false; timed_out = $false; seconds = $null; reason = $null }

try {
    foreach ($exe in @($agentExe, $verifyExe)) { if (-not (Test-Path $exe)) { throw "Missing $exe. Run: cargo build --release --locked" } }
    if (-not (Test-Path -LiteralPath $caseFile)) { throw "no case file cases\$Case.toml" }
    $caseInfo = (Invoke-Native $verifyExe @('case-files', '--case', $caseFile)).Output | ConvertFrom-Json
    if (-not $caseInfo.files) { throw 'case file did not load' }
    if ($caseInfo.large) { throw 'large-file cases are Stage B2 and are not run by this script' }
    $h = $caseInfo.harness
    $expected = @($caseInfo.files)
    $state.stage = 'layout'
    $rootFull = Assert-SafeTestRoot -Path $TestRoot -Create
    $layout = New-RunLayout -TestRoot $rootFull -Case $Case -Pastes ([int]$h.pastes)
    $runId = $layout.RunId
    $runDir = Join-Path $ws "evidence\$runId"
    New-Item -ItemType Directory -Force -Path $runDir | Out-Null
    $stopFile = Join-Path $runDir 'stop'
    if ($null -ne $h.pre_existing_seed) {
        foreach ($t in $layout.Targets) {
            for ($i = 0; $i -lt $expected.Count; $i++) {
                $r = Invoke-Native $verifyExe @('write-synthetic', '--seed', [string]$h.pre_existing_seed, '--index', [string]$i,
                    '--size', [string]$expected[$i].size, '--out', (Join-Path $t $expected[$i].name))
                if ($r.ExitCode -ne 0) { throw "could not pre-create original: $($r.Output)" }
            }
        }
        Add-Timeline 'harness' 'originals_created' @{ seed = $h.pre_existing_seed }
    }
    $preSnap = Join-Path $runDir 'explorer-pre.json'
    $snap = Invoke-Native $verifyExe @('explorer-snapshot', '--out', $preSnap)
    if ($snap.ExitCode -ne 0) { throw "explorer snapshot failed: $($snap.Output)" }
    Write-Host "run $runId  case $Case  driver $Driver  dialogs $DialogDriver  threading $($caseInfo.threading)"
    if ($h.operator_note) { Write-Host "  note: $($h.operator_note)" }
    Invoke-Fault 'after-layout'

    $state.stage = 'start-processes'
    $timeout = [int]$h.timeout_secs
    $tokens = @($expected | ForEach-Object { $_.name }) + @(1..([int]$h.pastes) | ForEach-Object { "paste-target-$_" })
    foreach ($t in $layout.Targets) {
        $leaf = Split-Path $t -Leaf
        $watchers += Start-OwnedProcess -FilePath $verifyExe -ArgumentList @(
            'watch', '--dest', "`"$t`"", '--out', "`"$(Join-Path $runDir "observer-$leaf.jsonl")`"",
            '--stop-file', "`"$stopFile`"", '--timeout-secs', [string]($timeout + 180), '--parent-pid', [string]$PID)
    }
    $dialogLog = Join-Path $runDir 'dialogs.jsonl'; $dialogState = Join-Path $runDir 'dialogs-state.json'
    $ownersFile = Join-Path $runDir 'owners.json'
    Set-Content -LiteralPath $ownersFile -Value '[]' -Encoding ASCII
    $dialogWatch = Start-OwnedProcess -FilePath $verifyExe -ArgumentList @(
        'dialog-watch', '--out', "`"$dialogLog`"", '--state', "`"$dialogState`"", '--stop-file', "`"$stopFile`"",
        '--owners-file', "`"$ownersFile`"",
        '--tokens', "`"$($tokens -join ',')`"", '--test-root', "`"$rootFull`"", '--timeout-secs', [string]($timeout + 180),
        '--parent-pid', [string]$PID)
    if ($h.probe_clipboard) {
        $probe = Start-OwnedProcess -FilePath $verifyExe -ArgumentList @(
            'clipboard-probe', '--out', "`"$(Join-Path $runDir 'clipboard-probe.jsonl')`"", '--stop-file', "`"$stopFile`"",
            '--timeout-secs', [string]($timeout + 120), '--parent-pid', [string]$PID)
    }
    $agent = Start-OwnedProcess -FilePath $agentExe -ArgumentList @(
        '--case', "`"$caseFile`"", '--run-dir', "`"$runDir`"", '--run-id', $runId,
        '--hold-secs', [string]($timeout + 120), '--exit-mode', $h.exit_mode, '--parent-pid', [string]$PID)
    $events = New-TailReader (Join-Path $runDir 'events.jsonl')
    $dialogTail = New-TailReader $dialogLog
    $readyDeadline = (Get-Date).AddSeconds(20)
    while (-not (Test-Path (Join-Path $runDir 'ready'))) {
        if ($agent.Process.HasExited) { throw "agent exited before becoming ready (exit code $($agent.Process.ExitCode))" }
        if ((Get-Date) -gt $readyDeadline) { throw 'agent did not become ready' }
        Start-Sleep -Milliseconds 100
    }
    foreach ($e in (Read-NewJson $events)) { [void]$agentEvents.Add($e); if ($e.event -eq 'set_clipboard') { $setSequence = $e.sequence } }
    Add-Timeline 'agent' 'ready' @{ sequence = $setSequence }
    Invoke-Fault 'after-agent-ready'

    if ($h.kill_before_paste) {
        $k = Stop-OwnedProcess -Owned $agent
        Add-Timeline 'harness' 'agent_killed_before_paste' $k
        Start-Sleep -Milliseconds 500
    }

    # --- Paste ---------------------------------------------------------------------------------
    $state.stage = 'paste'
    $n = 0
    foreach ($t in $layout.Targets) {
        $n++
        Start-Process -FilePath 'explorer.exe' -ArgumentList "`"$t`""
        $found = (Invoke-Native $verifyExe @('explorer-find', '--dest', $t, '--pre-snapshot', $preSnap)).Output | ConvertFrom-Json
        $windows += [ordered]@{
            destination = "paste-target-$n"; found = [bool]$found.found
            top_hwnd = $(if ($found.found) { $found.top_hwnd } else { $null })
            top_preexisting = $(if ($found.found) { $found.top_preexisting } else { $null })
            tabs_on_top = $(if ($found.found) { $found.tabs_on_top } else { $null }); close_decision = $null
        }
        if (-not $found.found) { $driverErrors += "window for paste-target-$n not found" }
    }
    $pasteTargets = @()
    for ($k = 0; $k -lt $layout.Targets.Count; $k++) { if ($windows[$k].found) { $pasteTargets += $k } }
    # The dialog watcher attributes dialogs whose owner chain leads to these windows to this run.
    $ownerTops = @($windows | Where-Object { $_.found } | ForEach-Object { [int64]$_.top_hwnd })
    Set-Content -LiteralPath $ownersFile -Value ('[' + ($ownerTops -join ',') + ']') -Encoding ASCII
    $explorerBefore = Get-ExplorerProcesses $ownerTops
    function Invoke-Paste([int] $Index) {
        $t = $layout.Targets[$Index]
        if ($Driver -eq 'ExplorerCommand') {
            $r = Invoke-Native $verifyExe @('explorer-paste', '--dest', $t, '--expected-top', [string]$windows[$Index].top_hwnd)
            Add-Timeline 'driver' 'paste' @{ target = "paste-target-$($Index + 1)"; exit = $r.ExitCode }
            if ($r.ExitCode -ne 0) { $script:driverErrors += $r.Output }
        } else {
            Write-Host "  >>> OPERATOR: click inside the Explorer window titled 'paste-target-$($Index + 1)' and press Ctrl+V once."
            Add-Timeline 'driver' 'operator_prompted' @{ target = "paste-target-$($Index + 1)" }
        }
    }

    # --- Monitor -------------------------------------------------------------------------------
    $pendingActions = New-Object System.Collections.ArrayList
    foreach ($a in @($h.dialog_actions)) { if ($a) { [void]$pendingActions.Add([string]$a) } }
    $created = 0; $released = 0; $agentGone = $false; $agentGoneAt = $null; $errorAt = $null; $lastChange = $clock.Elapsed.TotalSeconds
    $triggerTimes = @{}; $done = @{}; $dialogFirstSeen = @{}
    $errorSignal = $false
    # Groups of target indexes pasted together. Built without $( ), which would unroll the nesting.
    if ($h.concurrent) { $sequenceOrder = @(,@($pasteTargets)) } else { $sequenceOrder = @($pasteTargets | ForEach-Object { ,@($_) }) }
    $deadline = $clock.Elapsed.TotalSeconds + $timeout
    foreach ($group in $sequenceOrder) {
        foreach ($idx in $group) { Invoke-Paste $idx }
        $groupStart = $clock.Elapsed.TotalSeconds
        $createdAtStart = $created
        $quietSince = $null
        while ($true) {
            $now = $clock.Elapsed.TotalSeconds
            foreach ($e in (Read-NewJson $events)) {
                [void]$agentEvents.Add($e)
                switch ($e.event) {
                    'stream_create' { $created++; $lastChange = $now }
                    'stream_release' { $released++; $lastChange = $now }
                    'fault' {
                        Add-Timeline 'agent' $e.action @{ offset = $e.offset; stream_id = $e.stream_id }
                        if (-not $triggerTimes.ContainsKey($e.action)) { $triggerTimes[$e.action] = $now }
                        if (@('error_returned', 'eof_returned', 'read_timeout', 'self_kill', 'byte_budget_exceeded') -contains $e.action) { $errorSignal = $true; if ($null -eq $errorAt) { $errorAt = $now } }
                    }
                    'set_data' { Add-Timeline 'agent' 'set_data' @{ format = $e.format; dword = $e.dword } }
                }
            }
            foreach ($d in (Read-NewJson $dialogTail)) {
                [void]$dialogEvents.Add($d)
                if ($d.event -ne 'start') { Add-Timeline 'dialog' $d.event @{ hwnd = $d.hwnd; ours = $d.ours; title = $d.title } }
            }
            if (-not $agentGone -and $agent.Process.HasExited) {
                $agentGone = $true; $agentGoneAt = $now; $errorSignal = $true
                Add-Timeline 'agent' 'process_exited' @{ exit_code = $agent.Process.ExitCode }
            }
            # Harness actions triggered by provider events.
            if ($h.kill_after_event -and -not $done.kill -and $triggerTimes.ContainsKey($h.kill_after_event) -and
                ($now - $triggerTimes[$h.kill_after_event]) * 1000 -ge [int]$h.kill_delay_ms) {
                $done.kill = $true
                Add-Timeline 'harness' 'kill_agent' (Stop-OwnedProcess -Owned $agent)
            }
            if ($h.replace_clipboard_after_event -and -not $done.replace -and $triggerTimes.ContainsKey($h.replace_clipboard_after_event) -and
                ($now - $triggerTimes[$h.replace_clipboard_after_event]) * 1000 -ge [int]$h.replace_delay_ms) {
                $done.replace = $true
                $helpers += Start-OwnedProcess -FilePath $verifyExe -ArgumentList @('clipboard-replace', '--tag', $runId) -StdOut (Join-Path $runDir 'clipboard-replace.json')
                Add-Timeline 'harness' 'clipboard_replace_started'
            }
            if ($h.close_window_after_event -and -not $done.close -and $triggerTimes.ContainsKey($h.close_window_after_event)) {
                $done.close = $true
                $c = Invoke-Native $verifyExe @('explorer-close', '--dest', $layout.Targets[$group[0]], '--pre-snapshot', $preSnap,
                    '--expected-top', [string]$windows[$group[0]].top_hwnd)
                Add-Timeline 'harness' 'close_window' $c.Output
            }
            # Dialogs.
            $ds = Read-DialogState $dialogState
            $ours = @(); $openDialogs = @()
            if ($ds -and $ds.dialogs) { $openDialogs = @($ds.dialogs); $ours = @($ds.dialogs | Where-Object { $_.ours }) }
            foreach ($dlg in $openDialogs) { if (-not $dialogFirstSeen.ContainsKey([string]$dlg.hwnd)) { $dialogFirstSeen[[string]$dlg.hwnd] = $now } }
            if ($DialogDriver -eq 'Uia' -and $pendingActions.Count -gt 0 -and $ours.Count -gt 0) {
                $next = $pendingActions[0]
                # If an error/decision dialog of ours has been open for 5 s without offering the
                # requested button, the target does not offer that choice: record it and move on.
                $stuck = @($ours | Where-Object { -not $_.progress_bar -and ($now - $dialogFirstSeen[[string]$_.hwnd]) -ge 5 -and
                    -not (Test-OffersAction @($_.button_ids) $next) })
                if ($stuck.Count -gt 0 -and $next -ne 'cancel') {
                    [void]$actions.Add([ordered]@{ t_ms = [math]::Round($clock.Elapsed.TotalMilliseconds); action = $next; dialog_class = $stuck[0].class
                        dialog_progress = $false; result = [ordered]@{ invoked = $null; refused = 'not offered by the target dialog'; offered_ids = @($stuck[0].button_ids) }; raw_exit = $null })
                    Add-Timeline 'harness' "dialog_$($next)_not_offered" @{ offered_ids = @($stuck[0].button_ids) }
                    $pendingActions.RemoveAt(0)
                    if ($pendingActions.Count -eq 0) { [void]$pendingActions.Add('cancel') }
                    $next = $pendingActions[0]
                }
                foreach ($dlg in $ours) {
                    $ids = @($dlg.button_ids)
                    # A dialog without a progress bar is an error or decision dialog.
                    $isErrorDialog = -not $dlg.progress_bar
                    $allowed = $false
                    if ($next -eq 'cancel') { $allowed = ($h.expect -eq 'cancel') -or $isErrorDialog }
                    else { $allowed = $isErrorDialog }
                    $hasButton = Test-OffersAction $ids $next
                    if ($allowed -and $hasButton) {
                        $r = Invoke-Native $verifyExe @('dialog-act', '--hwnd', [string]$dlg.hwnd, '--ids', ($ActionIds[$next] -join ','),
                            '--tokens', ($tokens -join ','), '--owners', ($ownerTops -join ','), '--test-root', $rootFull)
                        $res = $null; try { $res = $r.Output | ConvertFrom-Json } catch { }
                        [void]$actions.Add([ordered]@{ t_ms = [math]::Round($clock.Elapsed.TotalMilliseconds); action = $next; dialog_class = $dlg.class
                            dialog_progress = [bool]$dlg.progress_bar; result = $res; raw_exit = $r.ExitCode })
                        Add-Timeline 'harness' "dialog_$next" $res
                        if ($res -and $res.invoked) { $pendingActions.RemoveAt(0) }
                        Start-Sleep -Milliseconds 700
                        break
                    }
                }
            }
            # Completion.
            $streamsSettled = ($created -gt $createdAtStart) -and ($created -eq $released)
            $nothingHappened = ($created -eq $createdAtStart) -and (($now - $groupStart) -gt 20)
            $settled = ($openDialogs.Count -eq 0) -and ($agentGone -or $streamsSettled -or $nothingHappened)
            if ($settled) {
                if ($null -eq $quietSince) { $quietSince = $now }
                # Give Explorer time to show an error dialog after a fault or a provider exit.
                $needQuiet = 3
                if ($errorSignal) { $needQuiet = 8 }
                if ($agentGone -and ($now - $agentGoneAt) -lt 15) { $needQuiet = [math]::Max($needQuiet, 15 - ($now - $agentGoneAt)) }
                if (($now - $quietSince) -ge $needQuiet -and ($now - $groupStart) -ge 3) { break }
            } else { $quietSince = $null }
            if ($now -gt $deadline) { $progress.timed_out = $true; Add-Timeline 'harness' 'timeout' @{ open_dialogs = $openDialogs.Count; created = $created; released = $released }; break }
            Start-Sleep -Milliseconds 200
        }
        if ($progress.timed_out) { break }
    }
    $progress.completed = -not $progress.timed_out
    $progress.seconds = [math]::Round($clock.Elapsed.TotalSeconds, 2)
    $progress.pending_dialog_actions = @($pendingActions)
    Invoke-Fault 'after-paste'
    $state.stage = 'finished-pastes'
} catch {
    $state.exception = $_.Exception.Message
    $state.failed_stage = $state.stage
    Write-Host "harness error at stage '$($state.stage)': $($state.exception)"
} finally {
    $post = [ordered]@{}
    try { if ($runDir) { New-Item -ItemType File -Force -Path (Join-Path $runDir 'stop') | Out-Null } } catch { $post.stop_file_error = $_.Exception.Message }
    $post.agent_stop = Stop-OwnedProcess -Owned $agent -GraceMs 30000
    $post.watcher_stops = @($watchers | ForEach-Object { Stop-OwnedProcess -Owned $_ -GraceMs 10000 })
    $post.dialog_watch_stop = Stop-OwnedProcess -Owned $dialogWatch -GraceMs 10000
    $post.probe_stop = Stop-OwnedProcess -Owned $probe -GraceMs 10000
    $post.helper_stops = @($helpers | ForEach-Object { Stop-OwnedProcess -Owned $_ -GraceMs 30000 })
    $agentExit = $(if ($agent) { $agent.Process.ExitCode } else { $null })
    # Leave no dialog of this run on the operator's screen. Only dialogs the watcher attributed to
    # this run are touched, and dialog-act re-validates each one before invoking anything.
    $cleanupDialogs = @()
    if ($runDir -and $dialogState -and (Test-Path -LiteralPath $dialogState) -and $DialogDriver -eq 'Uia') {
        $ds = Read-DialogState $dialogState
        foreach ($dlg in @($(if ($ds) { $ds.dialogs } else { @() }) | Where-Object { $_.ours })) {
            $r = Invoke-Native $verifyExe @('dialog-act', '--hwnd', [string]$dlg.hwnd, '--ids', ($ActionIds['cancel'] -join ','),
                '--tokens', ($tokens -join ','), '--owners', ($ownerTops -join ','), '--test-root', $rootFull)
            $res = $null; try { $res = $r.Output | ConvertFrom-Json } catch { }
            $cleanupDialogs += [ordered]@{ class = $dlg.class; progress_bar = $dlg.progress_bar; result = $res }
        }
    }
    $post.cleanup_dialogs = $cleanupDialogs
    if ($runDir) {
        try { foreach ($e in (Read-NewJson $events)) { [void]$agentEvents.Add($e) } } catch { }
        try { foreach ($d in (Read-NewJson $dialogTail)) { [void]$dialogEvents.Add($d) } } catch { }
    }

    $clipBefore = $null; $clipRelease = $null; $clipAfter = $null; $verifications = @(); $summary = $null; $envRecord = $null
    $observers = [ordered]@{}; $replace = $null; $probeStats = $null
    if ($runDir) {
        try { $clipBefore = (Invoke-Native $verifyExe @('clipboard-formats')).Output | ConvertFrom-Json } catch { $post.clipboard_error = $_.Exception.Message }
        # Only this run's tagged content is ever removed (W-02): replacement tag or provider marker.
        $relArgs = @('clipboard-release', '--tag', [string]$runId)
        if ($null -ne $setSequence) { $relArgs += @('--expect-sequence', [string]$setSequence) }
        try { $clipRelease = (Invoke-Native $verifyExe $relArgs).Output | ConvertFrom-Json } catch { $post.clipboard_release_error = $_.Exception.Message }
        try { $clipAfter = (Invoke-Native $verifyExe @('clipboard-formats')).Output | ConvertFrom-Json } catch { }
        $rp = Join-Path $runDir 'clipboard-replace.json'
        if (Test-Path $rp) { try { $replace = Get-Content -Raw $rp | ConvertFrom-Json } catch { } }
        $pp = Join-Path $runDir 'clipboard-probe.jsonl'
        if (Test-Path $pp) {
            $samples = @(Get-Content $pp | ForEach-Object { $_ | ConvertFrom-Json })
            $gets = @($samples | Where-Object { $null -ne $_.ole_get_ms } | ForEach-Object { [double]$_.ole_get_ms })
            $enums = @($samples | ForEach-Object { [double]$_.enum_ms })
            $probeStats = [ordered]@{
                samples = $samples.Count
                enum_ms_max = $(if ($enums.Count) { ($enums | Measure-Object -Maximum).Maximum } else { $null })
                ole_get_samples = $gets.Count
                ole_get_ms_max = $(if ($gets.Count) { ($gets | Measure-Object -Maximum).Maximum } else { $null })
                ole_get_ms_median = $(if ($gets.Count) { ($gets | Sort-Object)[[int][math]::Floor($gets.Count / 2)] } else { $null })
                ole_get_over_1s = @($gets | Where-Object { $_ -gt 1000 }).Count
            }
        }
        $i = 0
        $targets = @(); if ($layout) { $targets = @($layout.Targets) }
        foreach ($t in $targets) {
            $i++
            try {
                $out = Join-Path $runDir "verify-p$i.json"
                $vArgs = @('verify', '--case', $caseFile, '--dest', $t, '--out', $out)
                if ($caseInfo -and $null -ne $caseInfo.harness.pre_existing_seed) { $vArgs += @('--pre-existing-seed', [string]$caseInfo.harness.pre_existing_seed) }
                $null = Invoke-Native $verifyExe $vArgs
                $verifications += (Get-Content -Raw $out | ConvertFrom-Json)
            } catch { $post["verify_error_p$i"] = $_.Exception.Message }
            if ($i -le $windows.Count -and $windows[$i - 1].found) {
                $c = Invoke-Native $verifyExe @('explorer-close', '--dest', $t, '--pre-snapshot', $preSnap, '--expected-top', [string]$windows[$i - 1].top_hwnd)
                try { $windows[$i - 1].close_decision = ($c.Output | ConvertFrom-Json).decision } catch { $windows[$i - 1].close_decision = "error: $($c.Output)" }
            }
            $leaf = Split-Path $t -Leaf
            $lines = @(Get-Content (Join-Path $runDir "observer-$leaf.jsonl") -ErrorAction SilentlyContinue | ForEach-Object { $_ | ConvertFrom-Json })
            $observers[$leaf] = [ordered]@{
                events = $lines.Count
                names_seen = @($lines | Where-Object { $_.PSObject.Properties['name'] } | ForEach-Object { $_.name } | Sort-Object -Unique)
                removals = @($lines | Where-Object { $_.change -eq 'removed' } | ForEach-Object { $_.name })
                max_sizes = @($lines | Where-Object { $_.PSObject.Properties['size'] -and $null -ne $_.size } | Group-Object name | ForEach-Object {
                    [ordered]@{ name = $_.Name; max_size = ($_.Group | ForEach-Object { [int64]$_.size } | Measure-Object -Maximum).Maximum } })
            }
        }
        try {
            & (Join-Path $PSScriptRoot 'capture-env.ps1') -Out (Join-Path $runDir 'env.json') -TestRoot $TestRoot
            $envRecord = Get-Content -Raw (Join-Path $runDir 'env.json') | ConvertFrom-Json
        } catch { $post.env_error = $_.Exception.Message }
        $summaryFile = Join-Path $runDir 'summary.json'
        if (Test-Path $summaryFile) { $summary = Get-Content -Raw $summaryFile | ConvertFrom-Json }
    }
    $explorerAfter = @(Get-Process -ErrorAction SilentlyContinue | ForEach-Object { try { [ordered]@{ id = $_.Id; start = $_.StartTime.ToString('o') } } catch { } })
    $explorerLost = @($explorerBefore | Where-Object { $b = $_; -not ($explorerAfter | Where-Object { $_.id -eq $b.id -and $_.start -eq $b.start }) })

    # --- Classification (spec §9.1) -------------------------------------------------------------
    # Every Explorer dialog that appeared during the run is counted (hands-off window); its
    # attribution (owner chain or token) is recorded separately.
    $appeared = @($dialogEvents | Where-Object { $_.event -eq 'appear' })
    $ourDialogs = @($appeared | Where-Object { $_.ours })
    $errorDialogs = @($appeared | Where-Object { -not $_.progress_bar })
    $progressDialogs = @($appeared | Where-Object { $_.progress_bar })
    $errorDialogSeen = $errorDialogs.Count -gt 0
    $errorTexts = @($errorDialogs | ForEach-Object { @($_.names) } | Sort-Object -Unique)
    $cancelInvoked = [bool](@($actions | Where-Object { $_.action -eq 'cancel' -and $_.result -and $_.result.invoked -and $_.dialog_progress }).Count)
    $streamsCreated = @($agentEvents | Where-Object { $_.event -eq 'stream_create' }).Count
    $releases = @($agentEvents | Where-Object { $_.event -eq 'stream_release' })
    $allEof = ($releases.Count -gt 0) -and -not ($releases | Where-Object { -not $_.eof_read })
    $faultEvents = @($agentEvents | Where-Object { $_.event -eq 'fault' } | ForEach-Object { $_.action } | Sort-Object -Unique)
    $setData = @($agentEvents | Where-Object { $_.event -eq 'set_data' -and $_.format -ne 'DropDescription' } | ForEach-Object { [ordered]@{ format = $_.format; dword = $_.dword } })
    $performedEffect = @($setData | Where-Object { $_.format -eq 'Performed DropEffect' })
    $targetReport = 'success'
    if ($progress.timed_out) { $targetReport = 'hang_or_timeout' }
    elseif ($cancelInvoked) { $targetReport = 'cancelled' }
    elseif ($errorDialogSeen) { $targetReport = 'failure' }
    elseif ($streamsCreated -eq 0) { $targetReport = 'no_operation' }
    elseif ($DialogDriver -eq 'Manual' -and $caseInfo.harness.expect -eq 'cancel' -and $progressDialogs.Count -gt 0 -and -not $allEof) { $targetReport = 'cancelled' }
    elseif (-not $allEof -and $performedEffect.Count -eq 0) { $targetReport = 'silent_stop' }
    $severity = @{ 'O5' = 7; 'O4' = 6; 'O3' = 5; 'O2' = 4; 'O1' = 3; 'C' = 2; 'S' = 1; 'N' = 0 }
    $perFile = @(); $worst = 'N'; $vi = 0; $silentNoOp = 0
    foreach ($v in $verifications) {
        $vi++
        # A destination where Explorer created nothing while other pastes succeeded: silently dropped.
        $leafKey = "paste-target-$vi"
        $inactive = ($verifications.Count -gt 1) -and $observers.Contains($leafKey) -and (@($observers[$leafKey].names_seen).Count -eq 0)
        if ($inactive) { $silentNoOp++ }
        foreach ($f in @($v.files)) {
            $cls = 'N'
            if ($inactive -and $targetReport -eq 'success') { $cls = 'N' }
            elseif ($explorerLost.Count -gt 0 -or $targetReport -eq 'hang_or_timeout') { $cls = 'O5' }
            elseif ($f.state -eq 'full_size_wrong_content') { $cls = 'O3' }
            elseif ($targetReport -eq 'success') { $cls = $(if ($f.state -eq 'complete_correct') { 'S' } else { 'O4' }) }
            elseif ($targetReport -eq 'no_operation') { $cls = $(if ($f.state -eq 'absent' -or $f.state -eq 'original_preserved') { 'N' } else { 'O4' }) }
            elseif ($targetReport -eq 'silent_stop') {
                # No error shown and no success signal: a remaining file looks like a finished copy.
                $cls = $(if ($f.state -eq 'absent' -or $f.state -eq 'original_preserved') { 'O1' } elseif ($f.state -eq 'complete_correct') { 'S' } else { 'O4' })
            }
            else {
                switch ($f.state) {
                    'absent' { $cls = 'O1' }
                    'original_preserved' { $cls = 'O1' }
                    'complete_correct' { $cls = 'C' }
                    default { $cls = 'O2' }
                }
            }
            if ($f.mistakable_as_complete -and $f.state -ne 'complete_correct' -and $f.state -ne 'original_preserved' -and $cls -eq 'O2') { $cls = 'O3' }
            $perFile += [ordered]@{ target = "paste-target-$vi"; name = $f.name; state = $f.state; size = $f.size; mistakable = $f.mistakable_as_complete; class = $cls }
            if ($severity[$cls] -gt $severity[$worst]) { $worst = $cls }
        }
    }
    $interference = @()
    if ($summary -and $summary.clipboard_lost_ms -and -not $caseInfo.harness.replace_clipboard_after_event) {
        $interference += "clipboard replaced by another process at $($summary.clipboard_lost_ms) ms"
    }
    $closedEarly = @($windows | Where-Object { $_.close_decision -eq 'not_found' })
    if ($closedEarly.Count -gt 0 -and -not ($caseInfo -and $caseInfo.harness.close_window_after_event)) { $interference += 'a test window was closed before the run ended' }
    $unanswered = @($cleanupDialogs | Where-Object { -not $_.progress_bar -and -not ($_.result -and $_.result.invoked) })
    if ($progress.timed_out -and $unanswered.Count -gt 0) { $interference += 'a dialog of this run offered no button the harness could answer; timeout is a harness limitation' }
    $result = 'completed'
    if ($state.exception -or $driverErrors.Count -gt 0 -or $interference.Count -gt 0) { $result = 'inconclusive' }
    $expectMet = $null
    if ($caseInfo -and $result -eq 'completed') {
        switch ($caseInfo.harness.expect) {
            'success' { $expectMet = ($targetReport -eq 'success' -and $worst -eq 'S') }
            'failure' { $expectMet = ($targetReport -eq 'failure') }
            'cancel' { $expectMet = ($targetReport -eq 'cancelled') }
            default { $expectMet = $null }
        }
    }
    $clipClean = [bool]$clipAfter -and -not $clipAfter.has_onecopy_marker -and -not (@($clipAfter.formats) -contains 'OneCopyE02Replacement')

    # --- Cleanup: this run's expected names only, after a completed run --------------------------
    $cleanup = $null
    if ($layout -and $result -eq 'completed' -and -not $KeepDestination) {
        $cleanup = Remove-RunDestination -TestRoot $layout.TestRoot -RunId $layout.RunId -ExpectedFileNames @($expected | ForEach-Object { $_.name }) -Pastes ([int]$caseInfo.harness.pastes)
    }

    $recordId = $runId; if (-not $recordId) { $recordId = "$Case-unstarted-" + (Get-Date -Format 'yyyyMMddTHHmmss') }
    $record = [ordered]@{
        experiment_id = 'E-02'; stage = 'B1'; case_id = $Case; run_id = $recordId
        description = $(if ($caseInfo) { $caseInfo.description } else { $null })
        driver = $(if ($Driver -eq 'Manual') { 'D1' } else { 'D2c' })
        dialog_driver = $(if ($DialogDriver -eq 'Manual') { 'D1' } else { 'D2u' })
        target = 'G-01'; threading = $(if ($caseInfo) { $caseInfo.threading } else { $null })
        injected_harness_fault = $InjectHarnessFault
        case = $caseInfo; env = $envRecord
        progress = $progress; timeline = $timeline
        explorer_windows = $windows
        explorer_processes = [ordered]@{ watched = $explorerBefore; lost = $explorerLost }
        dialogs = [ordered]@{ appeared = $appeared.Count; attributed = $ourDialogs.Count; error_dialog_seen = $errorDialogSeen
            progress_dialog_seen = ($progressDialogs.Count -gt 0); error_texts = $errorTexts; events = @($dialogEvents | Where-Object { $_.event -ne 'start' }) }
        dialog_actions = $actions
        agent_exit_code = $agentExit; agent_summary = $summary
        agent_fault_events = $faultEvents; set_data = $setData; performed_drop_effect = @($performedEffect | ForEach-Object { $_.dword })
        streams = [ordered]@{ created = $streamsCreated; released = $releases.Count; all_reached_eof = $allEof }
        destinations = $verifications; observer = $observers
        clipboard = [ordered]@{ set_sequence = $setSequence; before_release = $clipBefore; release = $clipRelease; after = $clipAfter; clean_after = $clipClean; replacement = $replace; probe = $probeStats }
        classification = [ordered]@{ target_report = $targetReport; per_file = $perFile; outcome_class = $worst; expectation_met = $expectMet; silent_no_op_targets = $silentNoOp }
        harness = [ordered]@{ failed_stage = $state.failed_stage; exception = $state.exception; driver_errors = $driverErrors; post = $post }
        interference = $interference
        destination_cleanup = $cleanup; destination_kept = [bool]($layout -and -not $cleanup)
        result = $result
    }
    $recordPath = Join-Path $ReportDir "$recordId.json"
    try { Write-DurableJson -Object $record -Path $recordPath } catch { Write-Host "FAILED to write record: $($_.Exception.Message)" }
    Write-Host "result: $result  report: $targetReport  class: $worst  expectation_met: $expectMet  clipboard_clean: $clipClean"
    Write-Host "record: $recordPath"
}
