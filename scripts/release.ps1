[CmdletBinding()]
param(
    # Create the GitHub Release from the packages. Without it the packages are
    # only built, under target/dist/v<version>.
    [switch]$Publish,
    # Build only the Windows package, for example when Docker isn't available.
    [switch]$SkipLinux
)

# Builds the release packages that the Release workflow used to build on GitHub
# Actions: OpenV2K-v<version>-windows-x64.zip, OpenV2K-v<version>-linux-x64.tar.gz
# and SHA256SUMS.txt. The Linux package is built in an ubuntu:22.04 Docker
# container (scripts/release-linux.sh). Run it on `main` after the milestone
# merge; versions with a suffix (0.2.0-rc1) are published as pre-releases.

$ErrorActionPreference = 'Stop'
$repositoryRoot = Split-Path -Parent $PSScriptRoot

function Invoke-Checked {
    param([string]$Name, [string]$Program, [string[]]$Arguments)
    Write-Host "`n== $Name" -ForegroundColor Cyan
    & $Program @Arguments
    if ($LASTEXITCODE -ne 0) {
        throw "$Name failed with exit code $LASTEXITCODE"
    }
}

Push-Location $repositoryRoot
try {
    $manifest = Get-Content -Raw 'crates/v2k-game/Cargo.toml'
    if ($manifest -notmatch '(?m)^version = "([^"]+)"') {
        throw 'No version in crates/v2k-game/Cargo.toml.'
    }
    $version = $Matches[1]
    $tag = "v$version"
    $commit = (git rev-parse HEAD).Trim()

    if ($Publish) {
        if (git status --porcelain) {
            throw 'The working tree has changes; publish from a clean checkout of main.'
        }
        Invoke-Checked 'fetch' 'git' @('fetch', '--quiet', '--tags', 'origin', 'main')
        if ($commit -ne (git rev-parse origin/main).Trim()) {
            throw 'HEAD is not origin/main; publish from the merged milestone.'
        }
        git rev-parse -q --verify "refs/tags/$tag" > $null
        if ($LASTEXITCODE -eq 0) {
            throw "$tag is already released; bump crates/v2k-game's version to publish."
        }
        if ($SkipLinux) {
            throw 'A published release needs the Linux package; drop -SkipLinux.'
        }
    }

    $out = Join-Path $repositoryRoot "target/dist/$tag"
    if (Test-Path $out) { Remove-Item -Recurse -Force $out }
    New-Item -ItemType Directory -Force $out | Out-Null

    # cargo-about 0.9 builds its CLI only with the `cli` feature.
    if (-not (Get-Command cargo-about -ErrorAction SilentlyContinue)) {
        Invoke-Checked 'install cargo-about' 'cargo' @(
            'install', 'cargo-about', '--locked', '--version', '0.9.2', '--features', 'cli'
        )
    }
    $licenses = Join-Path $out 'THIRD_PARTY_LICENSES.html'
    Invoke-Checked 'third-party licenses' 'cargo' @(
        'about', 'generate', '--fail', '-o', $licenses, 'about.hbs'
    )

    # Windows: the MinGW-w64 GNU toolchain, as for local builds. MSYS2's GCC 15
    # defaults to C23, which SDL2's bundled sources reject.
    $env:CFLAGS = '-std=gnu17'
    Invoke-Checked 'build (Windows)' 'cargo' @(
        'build', '--release', '-p', 'v2k-game'
    )
    $windowsName = "OpenV2K-$tag-windows-x64"
    $windowsDir = Join-Path $out $windowsName
    New-Item -ItemType Directory $windowsDir | Out-Null
    Copy-Item README.md, LAUNCHER.md, LICENSE-MIT, LICENSE-APACHE, $licenses $windowsDir
    Copy-Item 'target/release/v2k-game.exe' (Join-Path $windowsDir 'OpenV2K.exe')
    Push-Location $out
    try {
        Invoke-Checked 'package (Windows)' (Join-Path $env:SystemRoot 'System32/tar.exe') @('-a', '-c', '-f', "$windowsName.zip", $windowsName)
    }
    finally {
        Pop-Location
    }
    Remove-Item -Recurse -Force $windowsDir

    if (-not $SkipLinux) {
        # Named volumes keep the toolchain and build cache between releases.
        Invoke-Checked 'build and package (Linux)' 'docker' @(
            'run', '--rm',
            '-v', "${repositoryRoot}:/src",
            '-v', 'openv2k-release-cargo:/root/.cargo',
            '-v', 'openv2k-release-target:/target',
            '-e', 'CARGO_TARGET_DIR=/target',
            '-e', "RELEASE_NAME=OpenV2K-$tag-linux-x64",
            '-e', "RELEASE_OUT=/src/target/dist/$tag",
            'ubuntu:22.04',
            'bash', '-c', "tr -d '\r' < /src/scripts/release-linux.sh | bash"
        )
    }

    $packages = Get-ChildItem $out -File | Where-Object Name -like 'OpenV2K-*' | Sort-Object Name
    $sums = $packages | ForEach-Object {
        "$((Get-FileHash -Algorithm SHA256 $_.FullName).Hash.ToLowerInvariant())  $($_.Name)"
    }
    [IO.File]::WriteAllText((Join-Path $out 'SHA256SUMS.txt'), (($sums -join "`n") + "`n"))
    Remove-Item $licenses

    if ($Publish) {
        $arguments = @('release', 'create', $tag, '--target', $commit, '--title', $tag, '--generate-notes')
        if ($version.Contains('-')) { $arguments += '--prerelease' }
        $arguments += @($packages.FullName) + (Join-Path $out 'SHA256SUMS.txt')
        Invoke-Checked "publish $tag" 'gh' $arguments
    }

    Write-Host "`nRelease packages for $tag are in $out." -ForegroundColor Green
}
finally {
    Pop-Location
}
