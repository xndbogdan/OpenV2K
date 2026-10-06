[CmdletBinding()]
param()

$ErrorActionPreference = 'Stop'
$repositoryRoot = Split-Path -Parent $PSScriptRoot
$runName = 'retail-policy-' + [Guid]::NewGuid().ToString('N')
$testRoot = Join-Path (Join-Path $repositoryRoot '.tmp') $runName
$crateRoot = Join-Path $testRoot 'fixture'
$targetRoot = Join-Path $testRoot 'target'
$workingDirectory = Join-Path $testRoot 'working-directory'
$retailRoot = Join-Path $testRoot 'retail'
$demoRoot = Join-Path $testRoot 'demo'
$otherRetailRoot = Join-Path $testRoot 'other-retail'
$manifestPath = Join-Path $crateRoot 'Cargo.toml'
$utf8 = New-Object System.Text.UTF8Encoding($false)
$oldRetail = [Environment]::GetEnvironmentVariable('V2K_RETAIL_DIR', 'Process')
$oldDemo = [Environment]::GetEnvironmentVariable('V2K_DEMO_DIR', 'Process')

function Write-FixtureFile {
    param([string]$Path, [string]$Text)
    [IO.File]::WriteAllText($Path, $Text, $utf8)
}

function Add-TestCorpus {
    param([string]$Path)
    New-Item -ItemType Directory -Path $Path -Force | Out-Null
    # These are framework fixtures, deliberately not real retail-format bytes.
    Write-FixtureFile (Join-Path $Path 'PRELOAD.DAT') 'corpus policy fixture'
}

function Remove-TestCorpus {
    param([string]$Path)
    $absolutePath = [IO.Path]::GetFullPath($Path)
    $allowedPrefix = [IO.Path]::GetFullPath($testRoot) + [IO.Path]::DirectorySeparatorChar
    if (-not $absolutePath.StartsWith($allowedPrefix, [StringComparison]::OrdinalIgnoreCase)) {
        throw "Refusing to remove a corpus outside the test scratch directory: $absolutePath"
    }
    Remove-Item -LiteralPath $absolutePath -Recurse -Force
}

function Invoke-PolicyCase {
    param(
        [string]$Name,
        [int]$Passed,
        [int]$Failed,
        [int]$Ignored,
        [string[]]$ExpectedText = @(),
        [switch]$RequireFresh
    )

    $logPath = Join-Path $testRoot "$Name.log"
    $cargoArguments = @(
        'test', '--offline', '--color', 'never', '--manifest-path', $manifestPath,
        '--target-dir', $targetRoot
    )
    if ($RequireFresh) {
        $cargoArguments += '-vv'
    }
    $cargoArguments += @('--', '--test-threads=1')
    Push-Location $workingDirectory
    try {
        # Windows PowerShell represents redirected native stderr as error records.
        # Preserve Cargo's exit code so the intentional partial-corpus failure can
        # be asserted without treating normal compiler progress as a script error.
        $ErrorActionPreference = 'Continue'
        & cargo @cargoArguments *> $logPath
        $cargoExitCode = $LASTEXITCODE
    }
    finally {
        Pop-Location
        $ErrorActionPreference = 'Stop'
    }

    $output = Get-Content -LiteralPath $logPath -Raw
    $expectedExit = if ($Failed -eq 0) { 0 } else { 101 }
    $summary = "$Passed passed; $Failed failed; $Ignored ignored"
    if ($cargoExitCode -ne $expectedExit -or -not $output.Contains($summary)) {
        throw "Policy case '$Name' expected exit $expectedExit and '$summary'.`n$output"
    }
    foreach ($expected in $ExpectedText) {
        if (-not $output.Contains($expected)) {
            throw "Policy case '$Name' is missing '$expected'.`n$output"
        }
    }
    if ($RequireFresh) {
        if (
            -not $output.Contains('Fresh v2k-retail-policy-fixture') -or
            $output.Contains('Compiling v2k-retail-policy-fixture') -or
            $output -match '(?m)^.*Running.*rustc(?:\.exe)?.*--crate-name v2k_retail_policy_fixture(?:\s|$)'
        ) {
            throw "Policy case '$Name' rebuilt an unchanged fixture instead of keeping it Fresh.`n$output"
        }
    }
    Write-Host "PASS: $Name ($summary)" -ForegroundColor Green
}

try {
    New-Item -ItemType Directory -Path (Join-Path $crateRoot 'src'), $workingDirectory -Force | Out-Null
    $supportPath = (Join-Path $repositoryRoot 'crates/v2k-test-support').Replace('\', '/')
    Write-FixtureFile $manifestPath @"
[package]
name = "v2k-retail-policy-fixture"
version = "0.0.0"
edition = "2021"
publish = false

[workspace]

[dev-dependencies]
v2k-test-support = { path = "$supportPath" }

[build-dependencies]
v2k-test-support = { path = "$supportPath" }
"@
    Write-FixtureFile (Join-Path $crateRoot 'build.rs') @'
fn main() {
    v2k_test_support::configure();
}
'@
    Write-FixtureFile (Join-Path $crateRoot 'src/lib.rs') @'
#[cfg(test)]
mod tests {
    #[test]
    fn synthetic_always_runs() {
        assert_eq!(2 + 2, 4);
    }

    #[v2k_test_support::retail_test]
    fn retail_requires_fixture() {
        let path = v2k_test_support::retail_dir().join("PRELOAD.DAT");
        assert_eq!(std::fs::read_to_string(path).unwrap(), "corpus policy fixture");
    }

    #[v2k_test_support::demo_test]
    fn demo_requires_fixture() {
        let path = v2k_test_support::demo_dir().join("PRELOAD.DAT");
        assert_eq!(std::fs::read_to_string(path).unwrap(), "corpus policy fixture");
    }
}
'@

    $env:V2K_RETAIL_DIR = $retailRoot
    $env:V2K_DEMO_DIR = $demoRoot
    Invoke-PolicyCase -Name 'absent-both' -Passed 1 -Failed 0 -Ignored 2 -ExpectedText @(
        'retail_requires_fixture ... ignored, retail game missing',
        'demo_requires_fixture ... ignored, demo game missing'
    )

    # Empty directories and tracked setup notes are not an installation.
    New-Item -ItemType Directory -Path $retailRoot, $demoRoot | Out-Null
    Invoke-PolicyCase -Name 'empty-both' -Passed 1 -Failed 0 -Ignored 2
    foreach ($corpusRoot in @($retailRoot, $demoRoot)) {
        Write-FixtureFile (Join-Path $corpusRoot 'README.md') 'Copy private game files here.'
        Write-FixtureFile (Join-Path $corpusRoot '.gitignore') '*'
    }
    Invoke-PolicyCase -Name 'placeholders-only' -Passed 1 -Failed 0 -Ignored 2
    # Cargo treats a nonexistent watched path as perpetually dirty. Existing
    # setup-note directories must avoid recompiling the real game on every run.
    Invoke-PolicyCase -Name 'placeholders-unchanged' -Passed 1 -Failed 0 -Ignored 2 -RequireFresh

    # Keep the override unchanged: any payload must enable strict checks even
    # when the install is incomplete or malformed. No cargo clean is used.
    New-Item -ItemType Directory -Path (Join-Path $retailRoot 'Overlay') | Out-Null
    Invoke-PolicyCase -Name 'partial-retail-fails' -Passed 1 -Failed 1 -Ignored 1
    Write-FixtureFile (Join-Path $retailRoot 'PRELOAD.DAT') 'broken fixture'
    Invoke-PolicyCase -Name 'malformed-retail-fails' -Passed 1 -Failed 1 -Ignored 1

    Add-TestCorpus $retailRoot
    Invoke-PolicyCase -Name 'retail-ready' -Passed 2 -Failed 0 -Ignored 1

    Add-TestCorpus $demoRoot
    Invoke-PolicyCase -Name 'both-ready' -Passed 3 -Failed 0 -Ignored 0
    Invoke-PolicyCase -Name 'both-ready-unchanged' -Passed 3 -Failed 0 -Ignored 0 -RequireFresh

    # Removal also refreshes the markers without cargo clean or an env change.
    Remove-TestCorpus $retailRoot
    Invoke-PolicyCase -Name 'retail-removed' -Passed 2 -Failed 0 -Ignored 1

    Add-TestCorpus $retailRoot
    $env:V2K_RETAIL_DIR = ".tmp/$runName/retail"
    $env:V2K_DEMO_DIR = ".tmp/$runName/demo"
    Invoke-PolicyCase -Name 'relative-to-repository' -Passed 3 -Failed 0 -Ignored 0

    $env:V2K_RETAIL_DIR = $otherRetailRoot
    Invoke-PolicyCase -Name 'override-switched-to-absent' -Passed 2 -Failed 0 -Ignored 1

    Add-TestCorpus $otherRetailRoot
    Invoke-PolicyCase -Name 'override-switched-ready' -Passed 3 -Failed 0 -Ignored 0

    Remove-TestCorpus $demoRoot
    Invoke-PolicyCase -Name 'demo-removed' -Passed 2 -Failed 0 -Ignored 1

    Write-Host "Private-corpus policy verified; fixture and logs: $testRoot"
}
finally {
    [Environment]::SetEnvironmentVariable('V2K_RETAIL_DIR', $oldRetail, 'Process')
    [Environment]::SetEnvironmentVariable('V2K_DEMO_DIR', $oldDemo, 'Process')
}
