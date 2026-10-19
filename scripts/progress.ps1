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
# Outputs: docs/progress/README.md, docs/progress/treemap.svg and the README.md
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
$sizes = @{}
foreach ($line in [IO.File]::ReadAllLines((Join-Path $root 'docs/progress/retail-functions.csv'), $utf8)) {
    if ($line -match '^\s*#' -or $line -match '^address,' -or -not $line.Trim()) { continue }
    $parts = $line.Split(',')
    $address = [Convert]::ToUInt32($parts[0].Substring(2), 16)
    $functions[$address] = $parts[2]
    $sizes[$address] = [int]$parts[1]
}
$game = @($functions.Keys | Where-Object { $functions[$_] -eq 'game' })

# ── Manual statuses ────────────────────────────────────────────────────────
$validStatuses = @('implemented', 'approximated', 'documented', 'not-needed')
$status = @{}
$statusNotes = @{}
$statusPath = Join-Path $root 'docs/progress/status.csv'
foreach ($line in [IO.File]::ReadAllLines($statusPath, $utf8)) {
    if ($line -match '^\s*#' -or $line -match '^address,' -or -not $line.Trim()) { continue }
    $parts = $line.Split(',', 3)
    $address = [Convert]::ToUInt32($parts[0].Trim().Substring(2), 16)
    $value = $parts[1].Trim()
    if ($validStatuses -notcontains $value) { throw "status.csv: unknown status '$value' for $($parts[0])" }
    if (-not $functions.ContainsKey($address)) { throw "status.csv: $($parts[0]) is not a retail function start" }
    $status[$address] = $value
    $statusNotes[$address] = if ($parts.Count -gt 2) { $parts[2].Trim() } else { '' }
}

# ── Citations ──────────────────────────────────────────────────────────────
# FUN_0042D030, 0x0042D030 or a bare 0042D030; DAT_/LAB_ and longer hex are not
# function citations.
$citation = [regex]'(?<![0-9A-Za-z_])(FUN_|0x)?00(4[0-9A-Fa-f]{5})(?![0-9A-Fa-f])'
function Get-Citations([string[]]$Files) {
    $found = New-Object 'Collections.Generic.HashSet[uint32]'
    $unresolved = New-Object 'Collections.Generic.SortedSet[string]'
    # address -> file name -> occurrences, for the treemap's owners
    $byAddress = @{}
    foreach ($file in $Files) {
        $name = [IO.Path]::GetFileNameWithoutExtension($file)
        foreach ($match in $citation.Matches((Read-Text $file))) {
            $address = [Convert]::ToUInt32('004' + $match.Groups[2].Value.Substring(1), 16)
            if ($functions.ContainsKey($address)) {
                [void]$found.Add($address)
                if (-not $byAddress.ContainsKey($address)) { $byAddress[$address] = @{} }
                $byAddress[$address][$name] = 1 + [int]$byAddress[$address][$name]
            }
            elseif ($match.Groups[1].Value -eq 'FUN_') {
                [void]$unresolved.Add(('0x{0:X8}' -f $address) + "  " + $file.Substring($root.Length + 1))
            }
        }
    }
    return @{ Found = $found; Unresolved = $unresolved; ByAddress = $byAddress }
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

# ── Treemap ────────────────────────────────────────────────────────────────
# Each game function belongs to the RE note that cites it most often (ties:
# the note citing fewer functions, then the name); catalog notes own nothing,
# and a function only they cite takes its nearest owned neighbour's note.
# Within a note's rectangle, the functions' bytes split by state.
$catalogs = @('SMALL_FUNCTIONS', 'INFESTATION_CROSS_REFERENCE', 'PORT_FIDELITY_GAPS')
$noteTotals = @{}
foreach ($entry in $docs.ByAddress.Values) {
    foreach ($note in $entry.Keys) { $noteTotals[$note] = 1 + [int]$noteTotals[$note] }
}
$sortedGame = @($game | Sort-Object)
$owner = @{}
$adopted = New-Object 'Collections.Generic.HashSet[uint32]'
foreach ($address in $sortedGame) {
    if (-not $docs.ByAddress.ContainsKey($address)) { continue }
    $entry = $docs.ByAddress[$address]
    $best = @($entry.Keys | Where-Object { $catalogs -notcontains $_ } |
        Sort-Object @{ Expression = { $entry[$_] }; Descending = $true }, @{ Expression = { $noteTotals[$_] } }, @{ Expression = { $_ } })
    if ($best.Count -gt 0) { $owner[$address] = $best[0] }
}
$previous = $null
foreach ($address in $sortedGame) {
    if ($owner.ContainsKey($address)) { $previous = $owner[$address] } elseif ($previous) { $owner[$address] = $previous; [void]$adopted.Add($address) }
}
$next = $null
foreach ($address in ($sortedGame | Sort-Object -Descending)) {
    if ($owner.ContainsKey($address)) { $next = $owner[$address] } elseif ($next) { $owner[$address] = $next; [void]$adopted.Add($address) }
}
function Get-State([uint32]$Address) {
    if ($status[$Address] -eq 'not-needed') { 'omitted' }
    elseif ($code.Found.Contains($Address)) { 'code' }
    elseif ($docs.Found.Contains($Address)) { 'notes' }
    else { 'none' }
}
$states = [ordered]@{ code = '#2da44e'; notes = '#d4a72c'; none = '#cf222e'; omitted = '#8c959f' }
$groups = @{}
foreach ($address in $sortedGame) {
    $name = $owner[$address]
    if (-not $groups.ContainsKey($name)) { $groups[$name] = @{ Bytes = @{}; Functions = @{}; Adopted = 0 } }
    $state = Get-State $address
    if ($adopted.Contains($address) -and ($state -eq 'notes' -or $state -eq 'none')) { $groups[$name].Adopted++ }
    $groups[$name].Bytes[$state] = $sizes[$address] + [int]$groups[$name].Bytes[$state]
    $groups[$name].Functions[$state] = 1 + [int]$groups[$name].Functions[$state]
}
function Format-Group([string]$Note) {
    $text = ($Note -replace '_RUNTIME$', '').Replace('_', ' ').ToLowerInvariant()
    $text.Substring(0, 1).ToUpperInvariant() + $text.Substring(1)
}

# Squarified layout (Bruls, Huizing and van Wijk): rows along the short side
# while they keep the cells nearest square. Items carry Value, largest first.
function Get-Worst($Row, [double]$Sum, [double]$Side) {
    $max = ($Row | Measure-Object Area -Maximum).Maximum
    $min = ($Row | Measure-Object Area -Minimum).Minimum
    [Math]::Max($Side * $Side * $max / ($Sum * $Sum), $Sum * $Sum / ($Side * $Side * $min))
}
function Get-Squarified([object[]]$Items, [double]$X, [double]$Y, [double]$W, [double]$H) {
    $total = ($Items | Measure-Object Value -Sum).Sum
    if ($total -le 0 -or $W -le 0 -or $H -le 0) { return @() }
    $cells = @($Items | ForEach-Object { [pscustomobject]@{ Item = $_; Area = $_.Value * $W * $H / $total } })
    $rects = New-Object 'Collections.Generic.List[object]'
    $i = 0
    while ($i -lt $cells.Count) {
        $side = [Math]::Min($W, $H)
        $row = @($cells[$i]); $sum = $cells[$i].Area
        $worst = Get-Worst $row $sum $side
        $j = $i + 1
        while ($j -lt $cells.Count) {
            $wider = Get-Worst ($row + $cells[$j]) ($sum + $cells[$j].Area) $side
            if ($wider -gt $worst) { break }
            $row += $cells[$j]; $sum += $cells[$j].Area; $worst = $wider; $j++
        }
        $thickness = $sum / $side
        $offset = 0.0
        foreach ($cell in $row) {
            $length = $cell.Area / $thickness
            if ($W -ge $H) { $rects.Add([pscustomobject]@{ Item = $cell.Item; X = $X; Y = $Y + $offset; W = $thickness; H = $length }) }
            else { $rects.Add([pscustomobject]@{ Item = $cell.Item; X = $X + $offset; Y = $Y; W = $length; H = $thickness }) }
            $offset += $length
        }
        if ($W -ge $H) { $X += $thickness; $W -= $thickness } else { $Y += $thickness; $H -= $thickness }
        $i = $j
    }
    return $rects.ToArray()
}
function Format-Number([double]$Value) { [Math]::Round($Value, 1).ToString('0.#', $inv) }
function Format-Xml([string]$Text) { [Security.SecurityElement]::Escape($Text) }

$mapWidth = 1200; $mapHeight = 720; $mapTop = 30
$groupItems = @($groups.Keys | ForEach-Object {
        [pscustomobject]@{ Name = $_; Value = ($groups[$_].Bytes.Values | Measure-Object -Sum).Sum }
    } | Sort-Object @{ Expression = { $_.Value }; Descending = $true }, Name)
$svg = New-Object Text.StringBuilder
[void]$svg.Append("<svg xmlns=`"http://www.w3.org/2000/svg`" width=`"$mapWidth`" height=`"$mapHeight`" viewBox=`"0 0 $mapWidth $mapHeight`" font-family=`"Segoe UI, Helvetica, Arial, sans-serif`" font-size=`"11`">`n")
[void]$svg.Append("<rect width=`"$mapWidth`" height=`"$mapHeight`" fill=`"#ffffff`"/>`n")
$legend = [ordered]@{ code = 'referenced by port code'; notes = 'by RE notes only'; none = 'not referenced'; omitted = 'omitted (status.csv not-needed)' }
$legendX = 4
foreach ($state in $legend.Keys) {
    [void]$svg.Append("<rect x=`"$legendX`" y=`"8`" width=`"14`" height=`"14`" fill=`"$($states[$state])`"/>")
    [void]$svg.Append("<text x=`"$($legendX + 19)`" y=`"19`" fill=`"#1f2328`">$($legend[$state])</text>`n")
    $legendX += 40 + 6 * $legend[$state].Length
}
[void]$svg.Append("<text x=`"$($mapWidth - 4)`" y=`"19`" fill=`"#59636e`" text-anchor=`"end`">tiles sized by function bytes, grouped by owning RE note</text>`n")
foreach ($group in (Get-Squarified $groupItems 0 $mapTop $mapWidth ($mapHeight - $mapTop))) {
    $name = $group.Item.Name
    $data = $groups[$name]
    $inScope = (@('code', 'notes', 'none') | ForEach-Object { [int]$data.Functions[$_] } | Measure-Object -Sum).Sum
    $share = if ($inScope -gt 0) { Format-Percent ([int]$data.Functions['code']) $inScope } else { 'omitted' }
    $title = Format-Xml ("$(Format-Group $name) ($name.md): " + (($states.Keys | Where-Object { $data.Functions[$_] } |
                ForEach-Object { "$($data.Functions[$_]) $_" }) -join ', '))
    [void]$svg.Append("<g><title>$title</title>`n")
    $parts = @($states.Keys | Where-Object { $data.Bytes[$_] } | ForEach-Object { [pscustomobject]@{ Name = $_; Value = $data.Bytes[$_] } })
    foreach ($part in (Get-Squarified $parts ($group.X + 1) ($group.Y + 1) ($group.W - 2) ($group.H - 2))) {
        [void]$svg.Append("<rect x=`"$(Format-Number $part.X)`" y=`"$(Format-Number $part.Y)`" width=`"$(Format-Number $part.W)`" height=`"$(Format-Number $part.H)`" fill=`"$($states[$part.Item.Name])`"/>`n")
    }
    $chars = [Math]::Floor(($group.W - 6) / 6.2)
    if ($group.H -ge 15 -and $chars -ge 4) {
        $label = "$(Format-Group $name) $share"
        if ($label.Length -gt $chars) { $label = $label.Substring(0, $chars - 1) + [char]0x2026 }
        [void]$svg.Append("<text x=`"$(Format-Number ($group.X + 3))`" y=`"$(Format-Number ($group.Y + 13))`" fill=`"#1f2328`">$(Format-Xml $label)</text>`n")
    }
    [void]$svg.Append("</g>`n")
}
[void]$svg.Append("</svg>`n")

# The groups with the most functions the code doesn't reference yet.
$gaps = @($groups.Keys | ForEach-Object {
        [pscustomobject]@{ Name = $_; Left = [int]$groups[$_].Functions['notes'] + [int]$groups[$_].Functions['none']; Adopted = $groups[$_].Adopted; Code = [int]$groups[$_].Functions['code'] }
    } | Where-Object { $_.Left -gt 0 } | Sort-Object @{ Expression = { $_.Left }; Descending = $true }, Name | Select-Object -First 10)
$gapRows = ($gaps | ForEach-Object {
        "| [$(Format-Group $_.Name)](../re/$($_.Name).md) | $(Format-Count $_.Left) | $(Format-Count $_.Adopted) | $(Format-Count $_.Code) |"
    }) -join "`n"
$omittedReasons = @($notNeeded | Group-Object { $statusNotes[$_] } | Sort-Object @{ Expression = { $_.Count }; Descending = $true }, Name)
$omittedRows = if ($omittedReasons.Count -gt 0) {
    ($omittedReasons | ForEach-Object { "| $($_.Name) | $(Format-Count $_.Count) |" }) -join "`n"
} else { '| (none) | 0 |' }

# ── Generated outputs ──────────────────────────────────────────────────────
# A static shields.io badge with the numbers in its URL: it needs no fetch from
# the repository, so it also renders while the repository is private.
function Format-BadgePart([string]$Text) { [uri]::EscapeDataString($Text).Replace('-', '--') }
$badgeUrl = 'https://img.shields.io/badge/' + (Format-BadgePart 'RE coverage') + '-' +
    (Format-BadgePart "$codePct code / $anyPct notes") + '-orange'
$block = @"
<!-- progress:start -->
[![RE coverage]($badgeUrl)](docs/progress/README.md)

Of the **$(Format-Count $scope.Count)** game functions in retail ``V2000.EXE`` the port
covers ([$(Format-Count $notNeeded.Count) omitted](docs/progress/README.md#omitted)),
**$(Format-Count $codeCount) ($codePct)** are referenced by the port's code and
**$(Format-Count $anyCount) ($anyPct)** by its code or RE notes.
[How this is measured](docs/progress/README.md).

[![Map of the retail functions by RE note and state](docs/progress/treemap.svg)](docs/progress/README.md#map)
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

## Map

![Treemap of retail V2000.EXE's game functions by owning RE note and state](treemap.svg)

Each rectangle is the RE note that cites its functions most often, sized by
their bytes and split by state; its label gives the share of its functions the
port's code references. A function only catalog notes cite (small functions,
the Infestation cross-reference, the fidelity-gap list) joins its nearest
neighbour's note.

Where the code's references are thinnest:

| Note | Not referenced by code | Of them, its neighbours | Referenced |
|---|---:|---:|---:|
$gapRows

## Omitted

Functions marked ``not-needed`` in [``status.csv``](status.csv) leave the
denominator:

| Reason | Functions |
|---|---:|
$omittedRows

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
  hook. ``scripts/check.ps1`` runs ``scripts/progress.ps1 -Check`` and fails if they are stale.
"@

$readmePath = Join-Path $root 'README.md'
$readme = Read-Text $readmePath
$markers = [regex]'(?s)<!-- progress:start -->.*?<!-- progress:end -->'
if (-not $markers.IsMatch($readme)) { throw 'README.md is missing the progress markers' }
$newReadme = $markers.Replace($readme, { param($m) $block.Replace("`r`n", "`n") }, 1)

$outputs = [ordered]@{
    'docs/progress/README.md' = $report.Replace("`r`n", "`n") + "`n"
    'docs/progress/treemap.svg' = $svg.ToString()
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
