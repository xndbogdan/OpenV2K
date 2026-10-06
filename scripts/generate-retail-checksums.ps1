[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [string]$ReferenceManifest,
    [string]$DataRoot = (Join-Path $PSScriptRoot '../retail'),
    [string]$Output = (Join-Path $PSScriptRoot '../crates/v2k-game/src/setup/retail-checksums.json')
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

# Publish fingerprints only after every required file agrees with a previously
# verified source/destination installation manifest. Original assets are read-only.
$manifest = Get-Content -LiteralPath $ReferenceManifest -Raw | ConvertFrom-Json
if ($manifest.version -ne 1 -or $manifest.files.Count -gt 1024) {
    throw 'Unsupported reference installation manifest'
}
$recorded = @{}
foreach ($entry in $manifest.files) {
    $key = $entry.path.ToLowerInvariant()
    if ($recorded.ContainsKey($key)) {
        throw "Duplicate reference path: $($entry.path)"
    }
    $recorded[$key] = $entry
}
$required = @('PRELOAD.DAT')
foreach ($tier in 0..3) {
    foreach ($level in 0..52) {
        $required += "Overlay/${tier}X${level}XX.OVL"
    }
}
$rows = @()
foreach ($relative in $required) {
    $entry = $recorded[$relative.ToLowerInvariant()]
    if ($null -eq $entry -or $entry.sha256 -notmatch '^[0-9a-fA-F]{64}$') {
        throw "Required reference fingerprint is missing or invalid: $relative"
    }
    $asset = Get-Item -LiteralPath (Join-Path $DataRoot $relative)
    $sha256 = (Get-FileHash -LiteralPath $asset.FullName -Algorithm SHA256).Hash.ToLowerInvariant()
    if ($asset.PSIsContainer -or $asset.Length -ne $entry.bytes -or $sha256 -ne $entry.sha256.ToLowerInvariant()) {
        throw "Canonical asset differs from the verified reference: $relative"
    }
    $rows += [ordered]@{ path = $relative; bytes = $asset.Length; sha256 = $sha256 }
}
$lines = @(
    '{',
    '  "version": 1,',
    '  "source": "Verified V2000.bin retail installation; required bytes also match the canonical port corpus",',
    '  "files": ['
)
for ($index = 0; $index -lt $rows.Count; $index++) {
    $suffix = if ($index + 1 -lt $rows.Count) { ',' } else { '' }
    $lines += '    ' + ($rows[$index] | ConvertTo-Json -Compress) + $suffix
}
$lines += '  ]', '}'
$destination = [System.IO.Path]::GetFullPath($Output)
[System.IO.File]::WriteAllText($destination, (($lines -join "`n") + "`n"), [System.Text.UTF8Encoding]::new($false))
Write-Output "Verified and generated $($rows.Count) required fingerprints: $destination"
