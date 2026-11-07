[CmdletBinding()]
param(
    # The retail installation the training run plays (see AGENTS.md).
    [string]$DataRoot = 'retail',
    # Rust's MinGW toolchain ships no profiler runtime; MSVC's, Rust's main
    # Windows target, does.
    [string]$Toolchain = '1.95-x86_64-pc-windows-msvc'
)

# Builds the shipped Windows game with profile-guided optimisation: an
# instrumented `ship` build plays a silent training run (eight campaign
# levels on OpenGL and the software raster, one with throttle, steering and
# fire), its profile is merged, and the `ship` build is made again from it,
# the C runtime linked in (no Visual C++ redistributable). Replays unchanged
# (docs/PERFORMANCE.md). Needs Visual Studio's C++ build tools and CMake.
# Prints the game's path last.

$ErrorActionPreference = 'Stop'
$repositoryRoot = Split-Path -Parent $PSScriptRoot
$target = 'x86_64-pc-windows-msvc'

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
    $work = Join-Path $repositoryRoot 'target/pgo'
    $data = Join-Path $work 'data'
    $runs = Join-Path $work 'runs'
    foreach ($dir in $data, $runs) {
        if (Test-Path $dir) { Remove-Item -Recurse -Force $dir }
        New-Item -ItemType Directory -Force $dir | Out-Null
    }
    Invoke-Checked 'toolchain' 'rustup' @('toolchain', 'install', $Toolchain, '--profile', 'minimal')
    Invoke-Checked 'llvm-tools' 'rustup' @('component', 'add', 'llvm-tools-preview', '--toolchain', $Toolchain)
    $sysroot = (& rustup run $Toolchain rustc --print sysroot).Trim()
    $profdata = Join-Path $sysroot "lib/rustlib/$target/bin/llvm-profdata.exe"

    # MinGW's C standard flag (release.ps1) means nothing to MSVC's compiler.
    $env:CFLAGS = $null
    $build = @("+$Toolchain", 'build', '--profile', 'ship', '-p', 'v2k-game', '--target', $target)
    # The C runtime linked in, so the game needs no Visual C++ redistributable.
    # The main thread keeps its 8 MiB reserve (.cargo/config.toml).
    $flags = '-C target-cpu=x86-64-v2 -C target-feature=+crt-static -C link-arg=/STACK:8388608'
    $env:CARGO_TARGET_DIR = Join-Path $work 'generate'
    $env:RUSTFLAGS = "$flags -Cprofile-generate=$data"
    Invoke-Checked 'instrumented build' 'cargo' $build
    $instrumented = Join-Path $env:CARGO_TARGET_DIR "$target/ship/v2k-game.exe"

    # Throttle on, steer, fire in bursts: exercises flight, weapons and HUD.
    [IO.File]::WriteAllText((Join-Path $runs 'drive.json'),
        '[{"frame":40,"key":"Space","pressed":true},{"frame":60,"key":"Left","pressed":true},{"frame":200,"key":"Return","pressed":true},{"frame":260,"key":"Return","pressed":false},{"frame":400,"key":"Left","pressed":false},{"frame":400,"key":"Right","pressed":true},{"frame":500,"key":"Return","pressed":true},{"frame":560,"key":"Return","pressed":false},{"frame":700,"key":"Right","pressed":false},{"frame":1250,"key":"Space","pressed":false}]')
    $levels = @(13, 18, 24, 30, 35, 40, 45, 50)
    function Invoke-Training {
        param($Level, [string]$Renderer, [int]$Frames, [string]$Keys)
        $start = Join-Path $runs 'start.json'
        [IO.File]::WriteAllText($start, "{`"level`":$Level}")
        $arguments = @('--data-dir', $DataRoot, '--start', $start,
            '--settings', (Join-Path $runs 'settings'),
            '--hidden', '--renderer', $Renderer,
            '--width', '1920', '--height', '1080', '--fixed-step-micros', '16000',
            '--frames', "$Frames")
        if ($Keys) { $arguments += @('--input-replay', (Join-Path $runs "$Keys.json")) }
        # Piped, so PowerShell waits for the GUI program.
        & $instrumented @arguments 2>&1 | Out-Null
    }
    Write-Host "`n== training run" -ForegroundColor Cyan
    foreach ($level in $levels) {
        Invoke-Training $level 'opengl' 900 ''
        Invoke-Training $level 'software' 900 ''
    }
    Invoke-Training 20 'opengl' 1300 'drive'
    Invoke-Training 30 'software' 1300 'drive'
    $raw = @(Get-ChildItem $data -Filter '*.profraw')
    if ($raw.Count -eq 0) {
        throw 'The training run left no profile; is the retail installation at -DataRoot?'
    }
    $merged = Join-Path $work 'merged.profdata'
    Invoke-Checked 'merge profile' $profdata (@('merge', '-o', $merged) + $raw.FullName)

    $env:CARGO_TARGET_DIR = Join-Path $work 'use'
    $env:RUSTFLAGS = "$flags -Cprofile-use=$merged"
    Invoke-Checked 'optimised build' 'cargo' $build
    Join-Path $env:CARGO_TARGET_DIR "$target/ship/v2k-game.exe"
}
finally {
    Remove-Item Env:CARGO_TARGET_DIR, Env:RUSTFLAGS -ErrorAction SilentlyContinue
    Pop-Location
}
