[CmdletBinding()]
param(
    [ValidateSet('Quick', 'Full')]
    [string]$Mode = 'Full',
    # Fail instead of ignoring retail-backed tests when no retail install is configured.
    [switch]$RequireRetailData,
    [switch]$Clippy
)

$ErrorActionPreference = 'Stop'
$repositoryRoot = Split-Path -Parent $PSScriptRoot

function Invoke-Checked {
    param([string]$Name, [string]$Program, [string[]]$Arguments)
    Write-Host "`n== $Name" -ForegroundColor Cyan
    Push-Location $repositoryRoot
    try {
        & $Program @Arguments
        if ($LASTEXITCODE -ne 0) {
            throw "$Name failed with exit code $LASTEXITCODE"
        }
    }
    finally {
        Pop-Location
    }
}

if ($RequireRetailData) {
    $retail = if ($env:V2K_RETAIL_DIR) { Join-Path $repositoryRoot $env:V2K_RETAIL_DIR } else { Join-Path $repositoryRoot 'retail' }
    if ([IO.Path]::IsPathRooted($env:V2K_RETAIL_DIR)) { $retail = $env:V2K_RETAIL_DIR }
    foreach ($required in @('PRELOAD.DAT', 'Overlay')) {
        if (-not (Test-Path -LiteralPath (Join-Path $retail $required))) {
            throw "Retail data required but '$required' is missing under $retail (set V2K_RETAIL_DIR)."
        }
    }
}

Invoke-Checked 'rustfmt' 'cargo' @('fmt', '--all', '--', '--check')
if ($Mode -eq 'Quick') {
    Invoke-Checked 'check' 'cargo' @('check', '--workspace', '--all-targets')
}
else {
    Invoke-Checked 'optional retail test policy' 'powershell' @(
        '-NoProfile', '-ExecutionPolicy', 'Bypass',
        '-File', (Join-Path $PSScriptRoot 'test-retail-policy.ps1')
    )
    Invoke-Checked 'tests' 'cargo' @('test', '--workspace')
}
if ($Clippy) {
    Invoke-Checked 'clippy' 'cargo' @('clippy', '--workspace', '--all-targets')
}

Write-Host "`nOpenV2K check passed ($Mode)." -ForegroundColor Green
