<#
.SYNOPSIS
Plays every world headless through the start harness and lists each one's
exit, last frame and notable log lines.

.DESCRIPTION
Each level (default 13..50: the campaign worlds, Intro1 and Intro2) starts
from its own arrival (`--start {"level": N}`), hidden at 640x480 with a
20 ms fixed step, for -Frames frames (3000 is a game minute). Output goes to
.tmp/level-sweep/<Tag>/: outNN.txt, eventsNN.jsonl, endNN.png and
summary.json, rewritten after each level. The executable is copied there
first, so cargo can rebuild during a sweep. -Baseline prints, per level,
the notable lines that are new (+) or gone (-) against an earlier sweep;
-DiffOnly compares without running.

.EXAMPLE
pwsh -NoProfile -File scripts/level-sweep.ps1 -Tag before
pwsh -NoProfile -File scripts/level-sweep.ps1 -Tag after -Baseline before
#>
[CmdletBinding()]
param(
    [string]$DataDir = 'retail',
    [int]$Frames = 3000,
    [string]$Tag = (Get-Date -Format 'yyyyMMdd-HHmmss'),
    [string]$Exe = 'target/release/v2k-game.exe',
    [ValidateSet('opengl', 'software')]
    [string]$Renderer = 'opengl',
    # Levels and ranges, such as '13..50' or '13,20,49..50'.
    [string]$Levels = '13..50',
    [string]$Baseline,
    [switch]$DiffOnly
)

$ErrorActionPreference = 'Stop'
$levelList = @($Levels -split ',' | ForEach-Object {
        if ($_ -match '^\s*(\d+)\.\.(\d+)\s*$') { [int]$Matches[1]..[int]$Matches[2] } else { [int]$_ }
    })
$root = Split-Path -Parent $PSScriptRoot
$sweeps = Join-Path $root '.tmp/level-sweep'
$out = Join-Path $sweeps $Tag
$notable = '(?i)panic|error|blocked|fail|unsupported|not ported|crash|missing|stopped|unowned'
# Numbers that differ between runs without meaning anything.
$noise = '\b\d{3,}\b'

function Get-Notable([string]$path) {
    if (-not (Test-Path -LiteralPath $path)) { return @() }
    @(Get-Content -LiteralPath $path | Where-Object { $_ -match $notable } |
        ForEach-Object { $_ -replace $noise, 'N' } | Sort-Object -Unique)
}

if (-not $DiffOnly) {
    New-Item -ItemType Directory -Force (Join-Path $out 'settings') | Out-Null
    $exeCopy = Join-Path $out 'v2k-game.exe'
    Copy-Item -LiteralPath (Join-Path $root $Exe) -Destination $exeCopy -Force
    $data = if ([IO.Path]::IsPathRooted($DataDir)) { $DataDir } else { Join-Path $root $DataDir }
    $summary = @()
    foreach ($level in $levelList) {
        $name = '{0:D2}' -f $level
        $state = Join-Path $out "start$name.json"
        Set-Content -LiteralPath $state -Value "{`"level`": $level}" -Encoding ascii
        $began = Get-Date
        $process = Start-Process -FilePath $exeCopy -PassThru -Wait -NoNewWindow `
            -RedirectStandardOutput (Join-Path $out "out$name.txt") `
            -RedirectStandardError (Join-Path $out "err$name.txt") -ArgumentList @(
            '--data-dir', "`"$data`"", '--settings', "`"$(Join-Path $out 'settings')`"",
            '--start', "`"$state`"", '--hidden', '--renderer', $Renderer,
            '--width', '640', '--height', '480', '--fixed-step-micros', '20000',
            '--frames', $Frames, '--snapshot', "`"$(Join-Path $out "end$name.png")`"",
            '--event-log', "`"$(Join-Path $out "events$name.jsonl")`"")
        $log = Join-Path $out "log$name.txt"
        Get-Content (Join-Path $out "out$name.txt"), (Join-Path $out "err$name.txt") |
            Set-Content -LiteralPath $log
        $events = Join-Path $out "events$name.jsonl"
        $last = Get-Content -LiteralPath $events -Tail 400 -ErrorAction SilentlyContinue |
            ForEach-Object { $_ | ConvertFrom-Json } | Where-Object { $_.ev -eq 'frame' } | Select-Object -Last 1
        # An idle player can die; a world that ends in the menu usually did.
        $death = Select-String -LiteralPath $events -Pattern '"ev":"player".*"health":0,' -List -ErrorAction SilentlyContinue
        $entry = [ordered]@{
            level   = $level
            exit    = $process.ExitCode
            seconds = [math]::Round(((Get-Date) - $began).TotalSeconds, 1)
            frame   = $last.frame
            state   = $last.state
            died    = if ($death) { ($death.Line | ConvertFrom-Json).frame } else { $null }
            notable = Get-Notable $log
        }
        $summary += [pscustomobject]$entry
        $summary | ConvertTo-Json -Depth 4 | Set-Content -LiteralPath (Join-Path $out 'summary.json')
        $died = if ($entry.died) { ", player died at frame $($entry.died)" } else { '' }
        Write-Host ('level {0}: exit {1}, frame {2} {3}{4}, {5} notable lines, {6} s' -f `
                $level, $entry.exit, $entry.frame, $entry.state, $died, $entry.notable.Count, $entry.seconds)
    }
}

if ($Baseline) {
    foreach ($level in $levelList) {
        $name = '{0:D2}' -f $level
        $old = Get-Notable (Join-Path $sweeps "$Baseline/log$name.txt")
        $new = Get-Notable (Join-Path $out "log$name.txt")
        $added = @($new | Where-Object { $old -notcontains $_ })
        $gone = @($old | Where-Object { $new -notcontains $_ })
        if ($added.Count -or $gone.Count) {
            Write-Host "level ${level}:"
            $added | ForEach-Object { Write-Host "  + $_" }
            $gone | ForEach-Object { Write-Host "  - $_" }
        }
    }
}
