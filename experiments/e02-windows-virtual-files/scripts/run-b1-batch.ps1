<#
.SYNOPSIS
  Runs a list of Stage B1 cases sequentially (hands-off window only) and prints a summary.

.DESCRIPTION
  Each run is a separate run-stage-b.ps1 invocation with its own durable record.
  The batch stops early, without starting another run, if:
    - the file evidence\ABORT exists (create it to stop after the current run);
    - an explorer.exe process that existed before the batch disappeared (possible crash);
    - a run leaves this run's OneCopy content on the clipboard;
    - an e02-agent process is still alive after a run.

.EXAMPLE
  .\scripts\run-b1-batch.ps1 -Cases ERR-01,ERR-02 -Repeats 3
#>
param(
    [Parameter(Mandatory = $true)][string[]] $Cases,
    [ValidateRange(1, 5)][int] $Repeats = 1,
    [ValidateSet('Manual', 'ExplorerCommand')][string] $Driver = 'ExplorerCommand',
    [ValidateSet('Manual', 'Uia')][string] $DialogDriver = 'Uia',
    [string] $ReportDir = ''
)
$ErrorActionPreference = 'Stop'
# 'powershell -File' passes a comma-separated list as one string.
$Cases = @($Cases | ForEach-Object { $_ -split ',' } | ForEach-Object { $_.Trim() } | Where-Object { $_ })
$ws = Split-Path -Parent $PSScriptRoot
if (-not $ReportDir) { $ReportDir = Join-Path $ws 'reports\stage-b1' }
$abort = Join-Path $ws 'evidence\ABORT'
# Long-lived explorer.exe processes only (folder launches start short-lived ones).
$explorerBefore = @(Get-Process explorer -ErrorAction SilentlyContinue | Where-Object { $_.StartTime -lt (Get-Date).AddSeconds(-120) } | ForEach-Object { "$($_.Id)@$($_.StartTime.ToString('o'))" })
$rows = @()
$stopReason = $null
:outer for ($r = 1; $r -le $Repeats; $r++) {
    foreach ($c in $Cases) {
        if (Test-Path $abort) { $stopReason = 'ABORT file present'; break outer }
        $before = @(Get-ChildItem $ReportDir -Filter "$c-*.json" -ErrorAction SilentlyContinue | ForEach-Object { $_.FullName })
        $t0 = Get-Date
        Write-Host "=== $c  repeat $r/$Repeats  $(Get-Date -Format 'HH:mm:ss')"
        & (Join-Path $PSScriptRoot 'run-stage-b.ps1') -Case $c -Driver $Driver -DialogDriver $DialogDriver -ReportDir $ReportDir | Out-Host
        $new = @(Get-ChildItem $ReportDir -Filter "$c-*.json" | Where-Object { $before -notcontains $_.FullName } | Sort-Object LastWriteTime)
        $rec = $null; if ($new.Count) { $rec = Get-Content -Raw $new[-1].FullName | ConvertFrom-Json }
        $row = [ordered]@{
            case = $c; repeat = $r; seconds = [math]::Round(((Get-Date) - $t0).TotalSeconds)
            run_id = $(if ($rec) { $rec.run_id } else { $null }); result = $(if ($rec) { $rec.result } else { 'no-record' })
            report = $(if ($rec) { $rec.classification.target_report } else { $null }); class = $(if ($rec) { $rec.classification.outcome_class } else { $null })
            expect_met = $(if ($rec) { $rec.classification.expectation_met } else { $null }); clipboard_clean = $(if ($rec) { $rec.clipboard.clean_after } else { $null })
        }
        $rows += [pscustomobject]$row
        $explorerNow = @(Get-Process explorer -ErrorAction SilentlyContinue | ForEach-Object { "$($_.Id)@$($_.StartTime.ToString('o'))" })
        if (@($explorerBefore | Where-Object { $explorerNow -notcontains $_ }).Count -gt 0) { $stopReason = 'an explorer.exe process disappeared'; break outer }
        if ($rec -and -not $rec.clipboard.clean_after) { $stopReason = "run $($rec.run_id) left OneCopy content on the clipboard"; break outer }
        if (@(Get-Process e02-agent -ErrorAction SilentlyContinue).Count -gt 0) { $stopReason = 'an e02-agent process is still running'; break outer }
    }
}
$rows | Format-Table -AutoSize | Out-Host
if ($stopReason) { Write-Host "BATCH STOPPED: $stopReason" } else { Write-Host 'batch complete' }
