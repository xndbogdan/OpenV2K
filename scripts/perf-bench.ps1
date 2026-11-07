<#
.SYNOPSIS
Times headless gameplay runs for compiler-build comparison.

.DESCRIPTION
Runs each level hidden at 640x480 with a 20 ms fixed step for -Frames
frames, -Repeats times, and records wall seconds, fps and the end snapshot
under .tmp/perf/<Tag>/. The executable is copied there first, so cargo can
rebuild between runs. -Baseline compares a later tag against an earlier one
and checks the end snapshots are byte-identical (compiler builds must not
change results). Run one tag per build (release, ship, PGO).

.EXAMPLE
pwsh -NoProfile -File scripts/perf-bench.ps1 -Tag release-before
pwsh -NoProfile -File scripts/perf-bench.ps1 -Tag ship-after -Baseline release-before
#>
[CmdletBinding()]
param(
    [string]$DataDir = 'retail',
    [int]$Frames = 3000,
    [int]$Repeats = 3,
    [string]$Tag = (Get-Date -Format 'yyyyMMdd-HHmmss'),
    [string]$Exe = 'target/release/v2k-game.exe',
    [string]$Levels = '13,20,30,40',
    [string]$Renderers = 'opengl,software',
    [int]$Width = 640,
    [int]$Height = 480,
    [string]$Baseline
)

$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
$perf = Join-Path $root '.tmp/perf'
$out = Join-Path $perf $Tag
New-Item -ItemType Directory -Force (Join-Path $out 'settings') | Out-Null
$levelList = @($Levels -split ',' | ForEach-Object { [int]$_.Trim() })
$rendererList = @($Renderers -split ',' | ForEach-Object { $_.Trim() })

if (-not $Baseline) {
    $exeCopy = Join-Path $out 'v2k-game.exe'
    Copy-Item -LiteralPath (Join-Path $root $Exe) -Destination $exeCopy -Force
    $data = if ([IO.Path]::IsPathRooted($DataDir)) { $DataDir } else { Join-Path $root $DataDir }
    $summary = @()
    foreach ($renderer in $rendererList) {
        foreach ($level in $levelList) {
            $fpsRuns = @()
            for ($rep = 1; $rep -le $Repeats; $rep++) {
                $name = '{0:D2}_{1}_{2}' -f $level, $renderer, $rep
                $state = Join-Path $out "start$name.json"
                Set-Content -LiteralPath $state -Value "{`"level`": $level}" -Encoding ascii
                $began = Get-Date
                $process = Start-Process -FilePath $exeCopy -PassThru -Wait -NoNewWindow `
                    -RedirectStandardOutput (Join-Path $out "out$name.txt") `
                    -RedirectStandardError (Join-Path $out "err$name.txt") -ArgumentList @(
                    '--data-dir', "`"$data`"", '--settings', "`"$(Join-Path $out 'settings')`"",
                    '--start', "`"$state`"", '--hidden', '--renderer', $renderer,
                    '--width', "$Width", '--height', "$Height", '--fixed-step-micros', '20000',
                    '--frames', $Frames, '--snapshot', "`"$(Join-Path $out "end$name.png")`"",
                    '--event-log', "`"$(Join-Path $out "events$name.jsonl")`"")
                $seconds = ((Get-Date) - $began).TotalSeconds
                $fps = if ($process.ExitCode -eq 0) { [math]::Round($Frames / $seconds, 1) } else { 0 }
                $fpsRuns += $fps
                Write-Host ('level {0} {1} rep {2}: exit {3}, {4} s, {5} fps' -f `
                        $level, $renderer, $rep, $process.ExitCode, [math]::Round($seconds, 1), $fps)
            }
            $sorted = @($fpsRuns | Sort-Object)
            $median = $sorted[[math]::Floor($sorted.Count / 2)]
            $summary += [pscustomobject]@{ level = $level; renderer = $renderer; fps = $fpsRuns; median_fps = $median }
            $summary | ConvertTo-Json -Depth 4 | Set-Content -LiteralPath (Join-Path $out 'summary.json')
        }
    }
    Write-Host "`nmedian fps:"
    $summary | ForEach-Object { Write-Host ('  level {0} {1}: {2}' -f $_.level, $_.renderer, $_.median_fps) }
}
else {
    $old = Get-Content -Raw -LiteralPath (Join-Path $perf "$Baseline/summary.json") | ConvertFrom-Json
    $new = Get-Content -Raw -LiteralPath (Join-Path $out 'summary.json') | ConvertFrom-Json
    $mismatched = @()
    foreach ($row in $new) {
        $before = @($old | Where-Object { $_.level -eq $row.level -and $_.renderer -eq $row.renderer })[0]
        $gain = if ($before.median_fps -gt 0) {
            [math]::Round(100.0 * ($row.median_fps - $before.median_fps) / $before.median_fps, 1)
        } else { 0 }
        Write-Host ('level {0} {1}: {2} -> {3} fps ({4}{5}%)' -f `
                $row.level, $row.renderer, $before.median_fps, $row.median_fps,
            $(if ($gain -ge 0) { '+' } else { '' }), $gain)
        for ($rep = 1; $rep -le $Repeats; $rep++) {
            $name = '{0:D2}_{1}_{2}' -f $row.level, $row.renderer, $rep
            $a = Join-Path $perf "$Baseline/end$name.png"
            $b = Join-Path $out "end$name.png"
            if ((Test-Path -LiteralPath $a) -and (Test-Path -LiteralPath $b)) {
                $ha = (Get-FileHash -Algorithm SHA256 -LiteralPath $a).Hash
                $hb = (Get-FileHash -Algorithm SHA256 -LiteralPath $b).Hash
                if ($ha -ne $hb) { $mismatched += $name }
            }
        }
    }
    if ($mismatched.Count -gt 0) {
        Write-Warning ('snapshots differ: ' + ($mismatched -join ', '))
    }
    else {
        Write-Host 'all end snapshots byte-identical.' -ForegroundColor Green
    }
}
