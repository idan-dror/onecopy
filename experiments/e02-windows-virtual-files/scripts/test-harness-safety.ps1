<#
.SYNOPSIS
  Focused negative tests for the E-02 harness safeguards.

.DESCRIPTION
  Non-desktop tests (default) never touch the clipboard or Explorer. They use a private
  sandbox under D:\OneCopyE02-selftest\<guid> containing junctions and sentinel files, and
  dummy child processes started by this script. No security settings are changed and no
  elevation is used.

  -IncludeDesktop adds tests that use the live clipboard (agent ownership, watchdog, single
  instance, durable record after an injected fault, Stage B1 clipboard release identity). Run
  them only in an agreed hands-off window.

.PARAMETER Out
  JSON summary path (default: reports\stage-b1\safety\harness-safety-<timestamp>.json).
#>
param([switch] $IncludeDesktop, [string] $Out = '')
$ErrorActionPreference = 'Stop'
Import-Module (Join-Path $PSScriptRoot 'E02Harness.psm1') -Force
$ws = Split-Path -Parent $PSScriptRoot
$bin = Join-Path $ws 'target\release'
$agentExe = Join-Path $bin 'e02-agent.exe'
$verifyExe = Join-Path $bin 'e02-verify.exe'
$psExe = Join-Path $env:SystemRoot 'System32\WindowsPowerShell\v1.0\powershell.exe'
if (-not $Out) { $Out = Join-Path $ws ("reports\stage-b1\safety\harness-safety-" + (Get-Date -Format 'yyyyMMddTHHmmss') + ".json") }

$results = New-Object System.Collections.ArrayList
function Test-Case([string] $Id, [string] $Name, [scriptblock] $Body) {
    try { & $Body; [void]$results.Add([ordered]@{ id = $Id; test = $Name; result = 'pass'; detail = $null }) ; Write-Host "PASS $Id $Name" }
    catch { [void]$results.Add([ordered]@{ id = $Id; test = $Name; result = 'fail'; detail = $_.Exception.Message }); Write-Host "FAIL $Id $Name :: $($_.Exception.Message)" }
}
function Assert-True($Condition, [string] $Message) { if (-not $Condition) { throw $Message } }
function Assert-Throws([scriptblock] $Body, [string] $Message) {
    $threw = $false; try { & $Body } catch { $threw = $true }
    if (-not $threw) { throw $Message }
}
function New-Junction([string] $Link, [string] $Target) {
    $null = New-Item -ItemType Junction -Path $Link -Value $Target
    Assert-True (Test-IsReparsePoint $Link) "junction was not created: $Link"
}

# --- Sandbox (owned by this script; removed with a reparse-aware walk at the end) -------------
$sandboxParent = 'D:\OneCopyE02-selftest'
$sandbox = Join-Path $sandboxParent ([guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Force -Path $sandbox | Out-Null
Set-Content -LiteralPath (Join-Path $sandbox '.onecopy-e02-selftest') -Value 'E-02 harness self-test sandbox' -Encoding ASCII
$sentinelDir = Join-Path $sandbox 'sentinel'
New-Item -ItemType Directory -Path $sentinelDir | Out-Null
$sentinelFile = Join-Path $sentinelDir 'keep-me.txt'
Set-Content -LiteralPath $sentinelFile -Value 'sentinel: must survive every test' -Encoding ASCII
function Assert-SentinelIntact { Assert-True ((Test-Path -LiteralPath $sentinelFile) -and ((Get-Content -LiteralPath $sentinelFile) -eq 'sentinel: must survive every test')) 'sentinel file was modified or deleted' }

# --- Filesystem safeguards -------------------------------------------------------------------
Test-Case 'FS-01' 'new test root is created with a marker' {
    $r = Assert-SafeTestRoot -Path (Join-Path $sandbox 'root-new') -Create
    Assert-True (Test-Path -LiteralPath (Join-Path $r '.onecopy-e02-testroot')) 'marker missing'
}
Test-Case 'FS-02' 'existing non-empty directory without marker is refused and untouched' {
    $d = Join-Path $sandbox 'user-owned'; New-Item -ItemType Directory -Path $d | Out-Null
    Set-Content -LiteralPath (Join-Path $d 'user-file.txt') -Value 'user data' -Encoding ASCII
    Assert-Throws { Assert-SafeTestRoot -Path $d -Create } 'unmarked non-empty directory was accepted'
    Assert-True ((Get-Content -LiteralPath (Join-Path $d 'user-file.txt')) -eq 'user data') 'user file changed'
    Assert-True (-not (Test-Path -LiteralPath (Join-Path $d '.onecopy-e02-testroot'))) 'marker was written into a user directory'
}
Test-Case 'FS-03' 'drive roots and protected locations are refused' {
    Assert-Throws { Assert-SafeTestRoot -Path 'D:\' -Create } 'drive root accepted'
    Assert-Throws { Assert-SafeTestRoot -Path (Join-Path $env:USERPROFILE 'OneCopyE02-should-not-exist') } 'profile path accepted'
    Assert-Throws { Assert-SafeTestRoot -Path (Join-Path $env:SystemRoot 'Temp\OneCopyE02-should-not-exist') } 'Windows path accepted'
    Assert-Throws { Assert-SafeTestRoot -Path (Split-Path $env:USERPROFILE -Parent) } 'ancestor of the profile accepted'
    Assert-True (-not (Test-Path -LiteralPath (Join-Path $env:USERPROFILE 'OneCopyE02-should-not-exist'))) 'directory was created in the profile'
}
Test-Case 'FS-04' 'a test root that is a junction (or under one) is refused' {
    $j = Join-Path $sandbox 'root-junction'; New-Junction $j $sentinelDir
    Assert-Throws { Assert-SafeTestRoot -Path $j -Create } 'junction test root accepted'
    Assert-Throws { Assert-SafeTestRoot -Path (Join-Path $j 'child') -Create } 'path under a junction accepted'
    Assert-SentinelIntact
}
Test-Case 'FS-05' 'run ids are unique and cleanup removes only expected files of a passing run' {
    $root = Assert-SafeTestRoot -Path (Join-Path $sandbox 'root-05') -Create
    $a = New-RunLayout -TestRoot $root -Case 'A-01' -Pastes 2
    $b = New-RunLayout -TestRoot $root -Case 'A-01' -Pastes 1
    Assert-True ($a.RunId -ne $b.RunId) 'run ids collided'
    foreach ($t in $a.Targets) { Set-Content -LiteralPath (Join-Path $t 'e02-A-01-f0.bin') -Value 'x' -Encoding ASCII }
    $r = Remove-RunDestination -TestRoot $root -RunId $a.RunId -ExpectedFileNames @('e02-A-01-f0.bin') -Pastes 2
    Assert-True (-not $r.refused) "refused: $($r.refused)"
    Assert-True ($r.deleted_files -eq 2) "deleted $($r.deleted_files)"
    Assert-True (-not (Test-Path -LiteralPath $a.RunDest)) 'run destination not removed'
    Assert-True (Test-Path -LiteralPath $b.RunDest) 'another run was touched'
}
Test-Case 'FS-06' 'unexpected files are left in place and reported' {
    $root = Assert-SafeTestRoot -Path (Join-Path $sandbox 'root-06') -Create
    $a = New-RunLayout -TestRoot $root -Case 'A-01'
    Set-Content -LiteralPath (Join-Path $a.Targets[0] 'e02-A-01-f0.bin') -Value 'x' -Encoding ASCII
    Set-Content -LiteralPath (Join-Path $a.Targets[0] 'unexpected.txt') -Value 'keep' -Encoding ASCII
    $r = Remove-RunDestination -TestRoot $root -RunId $a.RunId -ExpectedFileNames @('e02-A-01-f0.bin')
    Assert-True ($r.deleted_files -eq 1) 'expected file not deleted'
    Assert-True (Test-Path -LiteralPath (Join-Path $a.Targets[0] 'unexpected.txt')) 'unexpected file deleted'
    Assert-True (($r.left_entries -join ';') -match 'unexpected\.txt') 'unexpected file not reported'
}
Test-Case 'FS-07' 'a junction named like an expected file is not followed or deleted' {
    $root = Assert-SafeTestRoot -Path (Join-Path $sandbox 'root-07') -Create
    $a = New-RunLayout -TestRoot $root -Case 'A-01'
    New-Junction (Join-Path $a.Targets[0] 'e02-A-01-f0.bin') $sentinelDir
    $r = Remove-RunDestination -TestRoot $root -RunId $a.RunId -ExpectedFileNames @('e02-A-01-f0.bin')
    Assert-True ($r.deleted_files -eq 0) 'something was deleted through the junction'
    Assert-True (Test-IsReparsePoint (Join-Path $a.Targets[0] 'e02-A-01-f0.bin')) 'junction removed'
    Assert-True (($r.left_entries -join ';') -match 'not a regular file') 'junction not reported'
    Assert-SentinelIntact
}
Test-Case 'FS-08' 'a paste target replaced by a junction is not entered' {
    $root = Assert-SafeTestRoot -Path (Join-Path $sandbox 'root-08') -Create
    $a = New-RunLayout -TestRoot $root -Case 'A-01'
    Remove-Item -LiteralPath $a.Targets[0] -Force   # empty directory created by the layout
    $copy = Join-Path $sentinelDir 'e02-A-01-f0.bin'; Set-Content -LiteralPath $copy -Value 'sentinel copy' -Encoding ASCII
    New-Junction $a.Targets[0] $sentinelDir
    $r = Remove-RunDestination -TestRoot $root -RunId $a.RunId -ExpectedFileNames @('e02-A-01-f0.bin')
    Assert-True (Test-Path -LiteralPath $copy) 'file deleted through a junctioned paste target'
    Assert-True (($r.left_entries -join ';') -match 'reparse point') 'junctioned target not reported'
    Remove-Item -LiteralPath $copy -Force
    Assert-SentinelIntact
}
Test-Case 'FS-09' 'a run folder without a marker (pre-existing) is refused' {
    $root = Assert-SafeTestRoot -Path (Join-Path $sandbox 'root-09') -Create
    $fake = 'A-01-20990101T000000-deadbeef'
    $d = Join-Path (Join-Path $root 'dest') $fake; New-Item -ItemType Directory -Force -Path (Join-Path $d 'paste-target-1') | Out-Null
    Set-Content -LiteralPath (Join-Path $d 'paste-target-1\e02-A-01-f0.bin') -Value 'user' -Encoding ASCII
    $r = Remove-RunDestination -TestRoot $root -RunId $fake -ExpectedFileNames @('e02-A-01-f0.bin')
    Assert-True ([bool]$r.refused) 'unmarked run folder was cleaned'
    Assert-True (Test-Path -LiteralPath (Join-Path $d 'paste-target-1\e02-A-01-f0.bin')) 'file deleted'
}
Test-Case 'FS-10' 'a run marker that does not match is refused' {
    $root = Assert-SafeTestRoot -Path (Join-Path $sandbox 'root-10') -Create
    $a = New-RunLayout -TestRoot $root -Case 'A-01'
    Set-Content -LiteralPath (Join-Path $a.RunDest '.onecopy-e02-run') -Value 'A-01-20990101T000000-00000000' -Encoding ASCII
    Set-Content -LiteralPath (Join-Path $a.Targets[0] 'e02-A-01-f0.bin') -Value 'x' -Encoding ASCII
    $r = Remove-RunDestination -TestRoot $root -RunId $a.RunId -ExpectedFileNames @('e02-A-01-f0.bin')
    Assert-True ([bool]$r.refused -and $r.deleted_files -eq 0) 'mismatched marker accepted'
}
Test-Case 'FS-11' 'malformed run ids and expected names are refused' {
    $root = Assert-SafeTestRoot -Path (Join-Path $sandbox 'root-11') -Create
    foreach ($bad in @('..\..\x', 'A-01', 'A-01-20261112T000000', 'A-01-20261112T000000-GGGGGGGG', '')) {
        if (-not $bad) { continue }
        $r = Remove-RunDestination -TestRoot $root -RunId $bad -ExpectedFileNames @('e02-A-01-f0.bin')
        Assert-True ([bool]$r.refused) "run id accepted: $bad"
    }
    $a = New-RunLayout -TestRoot $root -Case 'A-01'
    foreach ($badName in @('..', '..\x.bin', 'sub\x.bin', 'C:x.bin')) {
        $r = Remove-RunDestination -TestRoot $root -RunId $a.RunId -ExpectedFileNames @($badName)
        Assert-True ([bool]$r.refused) "expected name accepted: $badName"
    }
}
Test-Case 'FS-12' 'cleanup refuses a test root whose marker disappeared' {
    $root = Assert-SafeTestRoot -Path (Join-Path $sandbox 'root-12') -Create
    $a = New-RunLayout -TestRoot $root -Case 'A-01'
    Set-Content -LiteralPath (Join-Path $a.Targets[0] 'e02-A-01-f0.bin') -Value 'x' -Encoding ASCII
    Remove-Item -LiteralPath (Join-Path $root '.onecopy-e02-testroot') -Force
    $r = Remove-RunDestination -TestRoot $root -RunId $a.RunId -ExpectedFileNames @('e02-A-01-f0.bin')
    Assert-True ([bool]$r.refused -and (Test-Path -LiteralPath (Join-Path $a.Targets[0] 'e02-A-01-f0.bin'))) 'cleanup proceeded without a root marker'
}
Test-Case 'FS-13' 'a run destination that is a junction is refused' {
    $root = Assert-SafeTestRoot -Path (Join-Path $sandbox 'root-13') -Create
    $fake = 'A-01-20990101T000000-cafebabe'
    $destParent = Join-Path $root 'dest'; New-Item -ItemType Directory -Force -Path $destParent | Out-Null
    Set-Content -LiteralPath (Join-Path $sentinelDir '.onecopy-e02-run') -Value $fake -Encoding ASCII
    New-Junction (Join-Path $destParent $fake) $sentinelDir
    $r = Remove-RunDestination -TestRoot $root -RunId $fake -ExpectedFileNames @('keep-me.txt')
    Assert-True ([bool]$r.refused) 'junctioned run destination accepted'
    Remove-Item -LiteralPath (Join-Path $sentinelDir '.onecopy-e02-run') -Force
    Assert-SentinelIntact
}

# --- Process safeguards ---------------------------------------------------------------------
function Start-Dummy { Start-OwnedProcess -FilePath $psExe -ArgumentList @('-NoProfile', '-NonInteractive', '-Command', 'Start-Sleep -Seconds 120') }
Test-Case 'PR-01' 'a process whose identity does not match is not stopped' {
    $d = Start-Dummy
    try {
        $fake = [pscustomobject]@{ Process = $d.Process; Id = $d.Id; StartTime = $d.StartTime.AddSeconds(-5); Path = $d.Path }
        $r = Stop-OwnedProcess -Owned $fake
        Assert-True ($r -eq 'identity-mismatch-not-stopped') "returned $r"
        Assert-True (-not $d.Process.HasExited) 'process was stopped despite identity mismatch'
    } finally { $null = Stop-OwnedProcess -Owned $d }
}
Test-Case 'PR-02' 'an owned process is stopped; an exited one is reported' {
    $d = Start-Dummy
    Assert-True ((Stop-OwnedProcess -Owned $d) -eq 'stopped') 'owned process not stopped'
    Assert-True ((Stop-OwnedProcess -Owned $d) -eq 'exited') 'exited process not reported'
    Assert-True ((Stop-OwnedProcess -Owned $null) -eq 'none') 'null owned not handled'
}
Test-Case 'PR-03' 'the directory observer exits when its parent harness process exits' {
    $parent = Start-Dummy
    $obsDir = Join-Path $sandbox 'observer'; New-Item -ItemType Directory -Path $obsDir | Out-Null
    $w = Start-OwnedProcess -FilePath $verifyExe -ArgumentList @('watch', '--dest', "`"$obsDir`"", '--out', "`"$(Join-Path $sandbox 'obs.jsonl')`"",
        '--stop-file', "`"$(Join-Path $sandbox 'never')`"", '--timeout-secs', '120', '--parent-pid', [string]$parent.Id)
    try {
        Start-Sleep -Milliseconds 500
        Assert-True (-not $w.Process.HasExited) 'observer exited too early'
        $null = Stop-OwnedProcess -Owned $parent
        Assert-True ($w.Process.WaitForExit(5000)) 'observer did not exit after its parent'
        Assert-True ((Get-Content (Join-Path $sandbox 'obs.jsonl') -Raw) -match 'parent_exited') 'parent exit not recorded'
    } finally { $null = Stop-OwnedProcess -Owned $w; $null = Stop-OwnedProcess -Owned $parent }
}
Test-Case 'PR-04' 'an injected failure before the agent starts still yields a durable inconclusive record' {
    $rep = Join-Path $ws 'evidence\selftest-reports'
    $before = @(Get-ChildItem $rep -Filter 'A-01-*.json' -ErrorAction SilentlyContinue).Count
    $out = & (Join-Path $PSScriptRoot 'run-stage-a.ps1') -Case A-01 -InjectHarnessFault after-layout -ReportDir $rep -TestRoot (Join-Path $sandbox 'root-pr04') *>&1 | Out-String
    $recs = @(Get-ChildItem $rep -Filter 'A-01-*.json' | Sort-Object LastWriteTime)
    Assert-True ($recs.Count -eq $before + 1) "no new record. output: $out"
    $rec = Get-Content -Raw $recs[-1].FullName | ConvertFrom-Json
    Assert-True ($rec.result -eq 'inconclusive') "result $($rec.result)"
    Assert-True ($rec.harness.exception -match 'injected harness fault at after-layout') 'exception not recorded'
    Assert-True ($rec.harness.post.agent_stop -eq 'none') 'an agent was started'
    Assert-True ($rec.destination_kept) 'destination should be kept on non-pass'
}

# --- Stage B1 safeguards (no clipboard, no Explorer) -----------------------------------------
Test-Case 'NB-01' 'write-synthetic never overwrites an existing file' {
    $ErrorActionPreference = 'Continue'   # the refusals below write to stderr by design
    $f = Join-Path $sandbox 'existing-original.bin'
    Set-Content -LiteralPath $f -Value 'pre-existing content' -Encoding ASCII
    $null = & $verifyExe write-synthetic --seed 1 --index 0 --size 4096 --out $f 2>&1
    Assert-True ($LASTEXITCODE -ne 0) 'write-synthetic reported success over an existing file'
    Assert-True ((Get-Content -LiteralPath $f) -eq 'pre-existing content') 'existing file was modified'
    Assert-SentinelIntact
}
Test-Case 'NB-02' 'unbounded (Stage B2) and unsafe cases are refused before the clipboard is touched' {
    $ErrorActionPreference = 'Continue'   # the refusals below write to stderr by design
    $big = Join-Path $sandbox 'big.toml'
    Set-Content -LiteralPath $big -Encoding ASCII -Value "case_id = `"BIG-01`"`ndescription = `"x`"`nseed = 1`n[[files]]`nname = `"a.bin`"`nsize = 1073741824`n"
    $null = & $verifyExe case-files --case $big 2>&1
    Assert-True ($LASTEXITCODE -ne 0) '1 GiB case accepted without large = true'
    $flush = Join-Path $sandbox 'flush.toml'
    Set-Content -LiteralPath $flush -Encoding ASCII -Value "case_id = `"FL-01`"`ndescription = `"x`"`nseed = 1`n[harness]`nexit_mode = `"flush`"`n[[files]]`nname = `"a.bin`"`nsize = 134217728`n"
    $null = & $verifyExe case-files --case $flush 2>&1
    Assert-True ($LASTEXITCODE -ne 0) 'flush of a 128 MiB case accepted'
    $sfalse = Join-Path $sandbox 'sfalse.toml'
    Set-Content -LiteralPath $sfalse -Encoding ASCII -Value "case_id = `"SF-01`"`ndescription = `"x`"`nseed = 1`n[[files]]`nname = `"a.bin`"`nsize = 10`n[[files.faults]]`nkind = `"error`"`nat_offset = 1`nhresult = 1`n"
    $null = & $verifyExe case-files --case $sfalse 2>&1
    Assert-True ($LASTEXITCODE -ne 0) 'an injected success code (S_FALSE) was accepted as a fault'
    # The provider refuses the unbounded case before it takes the clipboard (no ready file).
    $rd = Join-Path $sandbox 'agent-big'
    $a = Start-OwnedProcess -FilePath $agentExe -ArgumentList @('--case', "`"$big`"", '--run-dir', "`"$rd`"", '--run-id', 'selftest-nb02', '--hold-secs', '5')
    Assert-True ($a.Process.WaitForExit(10000)) 'agent did not exit'
    Assert-True ($a.Process.ExitCode -ne 0) 'agent accepted the unbounded case'
    Assert-True (-not (Test-Path (Join-Path $rd 'ready'))) 'agent took the clipboard'
}
Test-Case 'NB-03' 'dialog-act refuses a window that is not a live Explorer dialog of this run' {
    $out = & $verifyExe dialog-act --hwnd 1 --ids 2 --tokens 'paste-target-1' --test-root $sandbox 2>&1 | Out-String
    $r = $out | ConvertFrom-Json
    Assert-True ($null -eq $r.invoked -and $r.refused) "dialog-act did not refuse: $out"
    $self = (Get-Process -Id $PID).MainWindowHandle.ToInt64()
    if ($self -ne 0) {
        $r2 = (& $verifyExe dialog-act --hwnd $self --ids 2 --tokens 'paste-target-1' --test-root $sandbox 2>&1 | Out-String) | ConvertFrom-Json
        Assert-True ($null -eq $r2.invoked) 'dialog-act invoked a button in a non-Explorer window'
    }
}
Test-Case 'NB-04' 'Stage B1 run ids are accepted and validated by cleanup' {
    $root = Assert-SafeTestRoot -Path (Join-Path $sandbox 'root-nb04') -Create
    $a = New-RunLayout -TestRoot $root -Case 'CANCEL-01b' -Pastes 2
    Assert-True ($a.RunId -match '^CANCEL-01b-\d{8}T\d{6}-[0-9a-f]{8}$') "unexpected run id $($a.RunId)"
    foreach ($t in $a.Targets) { Set-Content -LiteralPath (Join-Path $t 'e02-CANCEL-01b-f0.bin') -Value 'x' -Encoding ASCII }
    Set-Content -LiteralPath (Join-Path $a.Targets[0] 'e02-CANCEL-01b-f0 (2).bin') -Value 'renamed copy' -Encoding ASCII
    $r = Remove-RunDestination -TestRoot $root -RunId $a.RunId -ExpectedFileNames @('e02-CANCEL-01b-f0.bin') -Pastes 2
    Assert-True ($r.deleted_files -eq 2) "deleted $($r.deleted_files)"
    Assert-True (Test-Path -LiteralPath (Join-Path $a.Targets[0] 'e02-CANCEL-01b-f0 (2).bin')) 'an unexpected name was deleted'
    foreach ($bad in @('cancel-01b-20261113T000000-00000000', 'CANCEL-01B-20261113T000000-00000000', 'ERR-1-20261113T000000-00000000')) {
        Assert-True ([bool](Remove-RunDestination -TestRoot $root -RunId $bad -ExpectedFileNames @('x.bin')).refused) "run id accepted: $bad"
    }
}
Test-Case 'PR-05' 'Stage B1 runner: an injected failure before the provider starts yields a durable inconclusive record' {
    $rep = Join-Path $ws 'evidence\selftest-reports'
    $before = @(Get-ChildItem $rep -Filter 'ERR-01-*.json' -ErrorAction SilentlyContinue).Count
    $out = & (Join-Path $PSScriptRoot 'run-stage-b.ps1') -Case ERR-01 -InjectHarnessFault after-layout -ReportDir $rep -TestRoot (Join-Path $sandbox 'root-pr05') *>&1 | Out-String
    $recs = @(Get-ChildItem $rep -Filter 'ERR-01-*.json' | Sort-Object LastWriteTime)
    Assert-True ($recs.Count -eq $before + 1) "no new record. output: $out"
    $rec = Get-Content -Raw $recs[-1].FullName | ConvertFrom-Json
    Assert-True ($rec.result -eq 'inconclusive') "result $($rec.result)"
    Assert-True ($rec.harness.post.agent_stop -eq 'none') 'a provider was started'
    Assert-True ($rec.destination_kept) 'destination should be kept on an inconclusive run'
}

# --- Desktop safeguards (clipboard; hands-off window only) ------------------------------------
if ($IncludeDesktop) {
    $caseFile = Join-Path $ws 'cases\A-01.toml'
    function Start-TestAgent([string] $Tag, [int] $ParentPid = 0) {
        $runDir = Join-Path $ws ("evidence\selftest-agent-$Tag-" + [guid]::NewGuid().ToString('N').Substring(0, 8))
        New-Item -ItemType Directory -Force -Path $runDir | Out-Null
        $agentArgs = @('--case', "`"$caseFile`"", '--run-dir', "`"$runDir`"", '--run-id', "selftest-$Tag", '--hold-secs', '60', '--exit-mode', 'clear')
        if ($ParentPid -gt 0) { $agentArgs += @('--parent-pid', [string]$ParentPid) }
        $a = Start-OwnedProcess -FilePath $agentExe -ArgumentList $agentArgs
        $deadline = (Get-Date).AddSeconds(15)
        while (-not (Test-Path (Join-Path $runDir 'ready'))) {
            if ($a.Process.HasExited -or (Get-Date) -gt $deadline) { throw "test agent not ready (exit $($a.Process.ExitCode))" }
            Start-Sleep -Milliseconds 100
        }
        return [pscustomobject]@{ Owned = $a; RunDir = $runDir }
    }
    function Get-ClipFormats { (& $verifyExe clipboard-formats | Out-String | ConvertFrom-Json) }

    Test-Case 'DT-01' 'after losing ownership the agent does not clear another application''s clipboard' {
        $t = Start-TestAgent 'dt01'
        try {
            $text = 'e02-selftest-' + [guid]::NewGuid().ToString('N')
            Set-Clipboard -Value $text     # another "application" takes the clipboard
            Start-Sleep -Milliseconds 500
            New-Item -ItemType File -Path (Join-Path $t.RunDir 'stop') | Out-Null
            Assert-True ($t.Owned.Process.WaitForExit(10000)) 'agent did not exit'
            Assert-True ((Get-Clipboard -Format Text -Raw).Trim() -eq $text) 'other application clipboard content was cleared or changed'
            $exit = Get-Content (Join-Path $t.RunDir 'events.jsonl') | ForEach-Object { $_ | ConvertFrom-Json } | Where-Object { $_.event -eq 'exit' }
            Assert-True ($exit.ours -eq $false -and $exit.cleared -eq $false) 'agent claimed or cleared a clipboard it no longer owned'
            $sum = Get-Content -Raw (Join-Path $t.RunDir 'summary.json') | ConvertFrom-Json
            Assert-True ($null -ne $sum.clipboard_lost_ms) 'loss of ownership not recorded'
        } finally { $null = Stop-OwnedProcess -Owned $t.Owned }
    }
    Test-Case 'DT-02' 'the agent exits and releases its own clipboard when the harness disappears' {
        $parent = Start-Dummy
        $t = Start-TestAgent 'dt02' $parent.Id
        try {
            Assert-True ((Get-ClipFormats).has_onecopy_marker) 'agent did not own the clipboard'
            $null = Stop-OwnedProcess -Owned $parent
            Assert-True ($t.Owned.Process.WaitForExit(5000)) 'agent kept running after its parent exited'
            Assert-True (-not (Get-ClipFormats).has_onecopy_marker) 'clipboard still offers OneCopy formats'
            $stop = Get-Content (Join-Path $t.RunDir 'events.jsonl') | ForEach-Object { $_ | ConvertFrom-Json } | Where-Object { $_.event -eq 'stop_requested' }
            Assert-True ($stop.parent_exited -eq $true) 'stop reason not recorded as parent exit'
        } finally { $null = Stop-OwnedProcess -Owned $t.Owned; $null = Stop-OwnedProcess -Owned $parent }
    }
    Test-Case 'DT-03' 'a second agent refuses to start while one is running' {
        $t = Start-TestAgent 'dt03'
        try {
            $runDir2 = Join-Path $ws ("evidence\selftest-agent-dt03b-" + [guid]::NewGuid().ToString('N').Substring(0, 8))
            New-Item -ItemType Directory -Force -Path $runDir2 | Out-Null
            $b = Start-OwnedProcess -FilePath $agentExe -ArgumentList @('--case', "`"$caseFile`"", '--run-dir', "`"$runDir2`"", '--run-id', 'selftest-dt03b', '--hold-secs', '30')
            Assert-True ($b.Process.WaitForExit(10000)) 'second agent did not exit'
            Assert-True ($b.Process.ExitCode -ne 0) 'second agent exited successfully'
            Assert-True (-not (Test-Path (Join-Path $runDir2 'ready'))) 'second agent took the clipboard'
            Assert-True (-not $t.Owned.Process.HasExited) 'first agent was disturbed'
            Assert-True ((Get-ClipFormats).has_onecopy_marker) 'first agent lost the clipboard'
        } finally {
            New-Item -ItemType File -Force -Path (Join-Path $t.RunDir 'stop') | Out-Null
            $null = Stop-OwnedProcess -Owned $t.Owned -GraceMs 10000
        }
    }
    Test-Case 'DT-04' 'an injected failure after the agent took the clipboard yields a durable record and a released clipboard' {
        $rep = Join-Path $ws 'evidence\selftest-reports'
        $before = @(Get-ChildItem $rep -Filter 'A-01-*.json' -ErrorAction SilentlyContinue).Count
        $null = & (Join-Path $PSScriptRoot 'run-stage-a.ps1') -Case A-01 -InjectHarnessFault after-agent-ready -ReportDir $rep -TestRoot (Join-Path $sandbox 'root-dt04') *>&1 | Out-String
        $recs = @(Get-ChildItem $rep -Filter 'A-01-*.json' | Sort-Object LastWriteTime)
        Assert-True ($recs.Count -eq $before + 1) 'no new record'
        $rec = Get-Content -Raw $recs[-1].FullName | ConvertFrom-Json
        Assert-True ($rec.result -eq 'inconclusive') "result $($rec.result)"
        Assert-True ($rec.harness.failed_stage -eq 'start-processes') "failed stage $($rec.harness.failed_stage)"
        Assert-True ($rec.agent_exit_code -eq 0) "agent exit $($rec.agent_exit_code)"
        Assert-True (-not $rec.clipboard_after_exit.has_onecopy_marker) 'clipboard not released'
        Assert-True (@(Get-Process e02-agent -ErrorAction SilentlyContinue).Count -eq 0) 'an agent is still running'
    }
    Test-Case 'DT-05' 'clipboard-release never empties another application''s clipboard' {
        $text = 'e02-selftest-' + [guid]::NewGuid().ToString('N')
        Set-Clipboard -Value $text
        $r = (& $verifyExe clipboard-release --tag 'ERR-01-20261113T000000-00000000' --expect-sequence 1 | Out-String) | ConvertFrom-Json
        Assert-True (-not $r.cleared -and $null -eq $r.identified_as_this_run) 'foreign clipboard identified as ours'
        Assert-True ((Get-Clipboard -Format Text -Raw).Trim() -eq $text) 'foreign clipboard content changed'
    }
    Test-Case 'DT-06' 'clipboard-release removes only content tagged with this run' {
        $tag = 'LIFE-03-20261113T000000-' + [guid]::NewGuid().ToString('N').Substring(0, 8)
        $set = (& $verifyExe clipboard-replace --tag $tag | Out-String) | ConvertFrom-Json
        Assert-True ($set.replaced) 'replacement helper failed'
        $other = (& $verifyExe clipboard-release --tag 'LIFE-03-20261113T000000-ffffffff' | Out-String) | ConvertFrom-Json
        Assert-True (-not $other.cleared) 'content of another run was cleared'
        Assert-True ((Get-Clipboard -Format Text -Raw).Trim() -eq 'E-02 synthetic clipboard replacement') 'replacement text missing'
        $mine = (& $verifyExe clipboard-release --tag $tag | Out-String) | ConvertFrom-Json
        Assert-True ($mine.cleared -and $mine.identified_as_this_run -eq 'replacement_tag') 'own tagged content not released'
        Assert-True (-not (Get-Clipboard -Format Text -Raw)) 'clipboard not empty after release'
    }
}

# --- Sandbox removal: owned by this script, walked without following reparse points ------------
function Remove-SandboxTree([string] $Dir) {
    foreach ($child in @(Get-ChildItem -LiteralPath $Dir -Force)) {
        if ($child.Attributes -band [IO.FileAttributes]::ReparsePoint) {
            if ($child.PSIsContainer) { [IO.Directory]::Delete($child.FullName) } else { [IO.File]::Delete($child.FullName) }   # removes the link only
        } elseif ($child.PSIsContainer) { Remove-SandboxTree $child.FullName }
        else { Remove-Item -LiteralPath $child.FullName -Force }
    }
    Remove-Item -LiteralPath $Dir -Force
}
Assert-SentinelIntact
if (Test-Path -LiteralPath (Join-Path $sandbox '.onecopy-e02-selftest')) { Remove-SandboxTree $sandbox }
if (@(Get-ChildItem -LiteralPath $sandboxParent -Force).Count -eq 0) { Remove-Item -LiteralPath $sandboxParent -Force }

$summary = [ordered]@{
    generated     = (Get-Date -Format 's')
    include_desktop = [bool]$IncludeDesktop
    total         = $results.Count
    passed        = @($results | Where-Object { $_.result -eq 'pass' }).Count
    failed        = @($results | Where-Object { $_.result -eq 'fail' }).Count
    tests         = $results
}
Write-DurableJson -Object $summary -Path $Out
Write-Host "harness safety: $($summary.passed)/$($summary.total) passed  ($Out)"
if ($summary.failed -gt 0) { exit 1 }
