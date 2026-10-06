[CmdletBinding()]
param(
    # Fail (exit 1) instead of writing when generated progress files are stale.
    [switch]$Check,
    # Print FUN_ citations that do not match any retail function start.
    [switch]$ListUnresolved
)

# Regenerates RE-progress numbers from function citations in the port and notes.
# Inputs: docs/progress/retail-functions.csv (complete retail function census)
# and docs/progress/status.csv (manual per-function status overrides).
# Outputs: docs/progress/README.md and the README.md
# block between the progress markers. Output is deterministic: no timestamps.

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
$root = Split-Path -Parent $PSScriptRoot
$inv = [Globalization.CultureInfo]::InvariantCulture
$utf8 = New-Object Text.UTF8Encoding($false)

# Compare and generate with LF line endings: Windows checkouts may convert to CRLF.
function Read-Text([string]$Path) { [IO.File]::ReadAllText($Path, $utf8).Replace("`r`n", "`n") }

# ── Retail function census ─────────────────────────────────────────────────
$functions = @{}
foreach ($line in [IO.File]::ReadAllLines((Join-Path $root 'docs/progress/retail-functions.csv'), $utf8)) {
    if ($line -match '^\s*#' -or $line -match '^address,' -or -not $line.Trim()) { continue }
    $parts = $line.Split(',')
    $functions[[Convert]::ToUInt32($parts[0].Substring(2), 16)] = $parts[2]
}
$game = @($functions.Keys | Where-Object { $functions[$_] -eq 'game' })

# ── Manual statuses ────────────────────────────────────────────────────────
$validStatuses = @('implemented', 'approximated', 'documented', 'not-needed')
$status = @{}
$statusPath = Join-Path $root 'docs/progress/status.csv'
foreach ($line in [IO.File]::ReadAllLines($statusPath, $utf8)) {
    if ($line -match '^\s*#' -or $line -match '^address,' -or -not $line.Trim()) { continue }
    $parts = $line.Split(',', 3)
    $address = [Convert]::ToUInt32($parts[0].Trim().Substring(2), 16)
    $value = $parts[1].Trim()
    if ($validStatuses -notcontains $value) { throw "status.csv: unknown status '$value' for $($parts[0])" }
    if (-not $functions.ContainsKey($address)) { throw "status.csv: $($parts[0]) is not a retail function start" }
    $status[$address] = $value
}

# ── Citations ──────────────────────────────────────────────────────────────
# FUN_0042D030, 0x0042D030 or a bare 0042D030; DAT_/LAB_ and longer hex are not
# function citations.
$citation = [regex]'(?<![0-9A-Za-z_])(FUN_|0x)?00(4[0-9A-Fa-f]{5})(?![0-9A-Fa-f])'
function Get-Citations([string[]]$Files) {
    $found = New-Object 'Collections.Generic.HashSet[uint32]'
    $unresolved = New-Object 'Collections.Generic.SortedSet[string]'
    foreach ($file in $Files) {
        foreach ($match in $citation.Matches((Read-Text $file))) {
            $address = [Convert]::ToUInt32('004' + $match.Groups[2].Value.Substring(1), 16)
            if ($functions.ContainsKey($address)) {
                [void]$found.Add($address)
            }
            elseif ($match.Groups[1].Value -eq 'FUN_') {
                [void]$unresolved.Add(('0x{0:X8}' -f $address) + "  " + $file.Substring($root.Length + 1))
            }
        }
    }
    return @{ Found = $found; Unresolved = $unresolved }
}
$codeFiles = @(Get-ChildItem (Join-Path $root 'crates') -Recurse -Filter *.rs | ForEach-Object FullName)
$docFiles = @(Get-ChildItem (Join-Path $root 'docs/re') -Filter *.md | ForEach-Object FullName)
$code = Get-Citations $codeFiles
$docs = Get-Citations $docFiles

$notNeeded = @($game | Where-Object { $status[$_] -eq 'not-needed' })
$scope = @($game | Where-Object { $status[$_] -ne 'not-needed' })
$codeCount = @($scope | Where-Object { $code.Found.Contains($_) }).Count
$anyCount = @($scope | Where-Object { $code.Found.Contains($_) -or $docs.Found.Contains($_) }).Count
$statusCounts = @{}
foreach ($name in $validStatuses) { $statusCounts[$name] = @($game | Where-Object { $status[$_] -eq $name }).Count }
$unresolved = @($code.Unresolved) + @($docs.Unresolved)

function Format-Count([int]$Value) { $Value.ToString('N0', $inv) }
function Format-Percent([int]$Part, [int]$Whole) {
    if ($Whole -eq 0) { return '0.0%' }
    (100.0 * $Part / $Whole).ToString('0.0', $inv) + '%'
}
$codePct = Format-Percent $codeCount $scope.Count
$anyPct = Format-Percent $anyCount $scope.Count

# ── Generated outputs ──────────────────────────────────────────────────────
# A static shields.io badge with the numbers in its URL: it needs no fetch from
# the repository, so it also renders while the repository is private.
function Format-BadgePart([string]$Text) { [uri]::EscapeDataString($Text).Replace('-', '--') }
$badgeUrl = 'https://img.shields.io/badge/' + (Format-BadgePart 'RE coverage') + '-' +
    (Format-BadgePart "$codePct code / $anyPct notes") + '-orange'
$block = @"
<!-- progress:start -->
[![RE coverage]($badgeUrl)](docs/progress/README.md)

Of the **$(Format-Count $scope.Count)** game functions in retail ``V2000.EXE``,
**$(Format-Count $codeCount) ($codePct)** are referenced by the port's code and
**$(Format-Count $anyCount) ($anyPct)** by its code or RE notes.
[How this is measured](docs/progress/README.md).
<!-- progress:end -->
"@

$report = @"
# Reverse-engineering progress

> Generated by ``scripts/progress.ps1``. Do not edit by hand.

OpenV2K is a reimplementation, not a matching decompilation. Nothing is
recompiled into the original binary, so there's no byte-exact "percent
decompiled". Instead, this report counts which retail functions the port's code
and RE notes cite by address (for example ``FUN_0042D030``).

| Measure | Functions | Share |
|---|---:|---:|
| Game functions in retail ``V2000.EXE`` | $(Format-Count $game.Count) | |
| Marked not needed (excluded below) | $(Format-Count $notNeeded.Count) | |
| **Referenced by port code** | **$(Format-Count $codeCount)** | **$codePct** |
| Referenced by code or RE notes | $(Format-Count $anyCount) | $anyPct |
| Library functions (CRT, DirectX stubs; not counted) | $(Format-Count ($functions.Count - $game.Count)) | |

Manual statuses from [``status.csv``](status.csv):

| Status | Functions |
|---|---:|
| implemented | $(Format-Count $statusCounts['implemented']) |
| approximated | $(Format-Count $statusCounts['approximated']) |
| documented | $(Format-Count $statusCounts['documented']) |
| not-needed | $(Format-Count $statusCounts['not-needed']) |

$(Format-Count $unresolved.Count) ``FUN_`` citations don't match a retail function
start. Run ``scripts/progress.ps1 -ListUnresolved`` to list them.

## Method

- [``retail-functions.csv``](retail-functions.csv) lists every function start in
  the retail executable. It comes from a Ghidra census that combines
  auto-analysis with seeding from callback tables, already-cited addresses and
  uncovered code gaps. Functions cover 94% of ``.text``, and most of the rest is
  padding. Everything from the first linker-placed library import stub
  (``0x004AC5A0``) onward counts as library code.
- A function counts as *referenced* when a Rust source file under ``crates/``
  (for code) or a note under ``docs/re/`` cites its start address. A citation
  means the port or the notes deal with that function. It doesn't prove that
  every behavior of the function is reproduced.
- ``status.csv`` records judgments the citation count can't make:
  ``implemented``, ``approximated``, ``documented`` and ``not-needed``.
  ``not-needed`` removes a function from the denominator, for example
  DirectPlay networking or debug-only paths.
- The numbers are regenerated on every commit by the ``.githooks/pre-commit``
  hook. CI runs ``scripts/progress.ps1 -Check`` and fails if they are stale.
"@

$readmePath = Join-Path $root 'README.md'
$readme = Read-Text $readmePath
$markers = [regex]'(?s)<!-- progress:start -->.*?<!-- progress:end -->'
if (-not $markers.IsMatch($readme)) { throw 'README.md is missing the progress markers' }
$newReadme = $markers.Replace($readme, { param($m) $block.Replace("`r`n", "`n") }, 1)

$outputs = [ordered]@{
    'docs/progress/README.md' = $report.Replace("`r`n", "`n") + "`n"
    'README.md' = $newReadme
}

if ($ListUnresolved) { $unresolved | ForEach-Object { Write-Output $_ } }

$stale = @()
foreach ($relative in $outputs.Keys) {
    $path = Join-Path $root $relative
    $current = if (Test-Path -LiteralPath $path) { Read-Text $path } else { '' }
    if ($current -ne $outputs[$relative]) {
        $stale += $relative
        if (-not $Check) { [IO.File]::WriteAllText($path, $outputs[$relative], $utf8) }
    }
}
if ($Check) {
    if ($stale.Count -gt 0) {
        Write-Error ("Progress numbers are stale: " + ($stale -join ', ') + ". Run scripts/progress.ps1 and commit the result.")
        exit 1
    }
    Write-Output "Progress up to date: code $codePct, code or notes $anyPct."
}
else {
    # One changed path per line, consumed by the pre-commit hook.
    $stale | ForEach-Object { Write-Output $_ }
}
