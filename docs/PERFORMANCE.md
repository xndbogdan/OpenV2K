# Performance

Compiler-build optimisation ported from OpenInfestation, measured on this
machine (Windows, NVIDIA, 640x480 hidden, 20 ms fixed step, 3000 frames,
median of three; `scripts/perf-bench.ps1`). Raw runs under
`.tmp/perf/<tag>/` (ignored); `summary.json` holds each tag's medians.

## Method

`scripts/perf-bench.ps1 -Tag <name>` copies the executable aside, then runs
levels 13, 20, 30 and 40 on the OpenGL and software renderers, three times
each, recording wall seconds and fps plus an end snapshot and an event log.
`-Baseline <old>` compares medians and checks the end snapshots are
byte-identical: compiler builds must not change results. Note the event
logs always differ run to run (frame lines carry the process random word),
even for the same binary, so snapshots are the fidelity check.

## The shipped build (2026-11-07)

Releases build with `--profile ship` (`Cargo.toml`: fat LTO, one codegen
unit) and every build targets x86-64-v2 (`.cargo/config.toml`). `release.ps1`
and `release-linux.sh` build `ship` (output under `target/ship/`). Neither
changes a result: Rust never contracts or reassociates floating point
unasked, and the end snapshots equal `--release` on every level and renderer.
`--release` stays the quick build for tests and probes.

`release-before` (plain `--release`, pre-x86-64-v2) against `ship-after`
(stage 1, the post-optimization shipped build) and `pgo-after` (stage 2,
profile-guided; blocked, see below):

| Level | Renderer | Pre (fps) | Stage 1 ship (fps) | Stage 2 PGO (fps) |
| --- | --- | --- | --- | --- |
| 13 | OpenGL | 221.1 | 222.5 (+0.6%) | 223.6 (+1.1%) |
| 20 | OpenGL | 220.7 | 222.0 (+0.6%) | 223.2 (+1.1%) * |
| 30 | OpenGL | 222.1 | 223.5 (+0.6%) | 224.3 (+1.0%) |
| 40 | OpenGL | 209.2 | 211.8 (+1.2%) | 218.4 (+4.4%) |
| 13 | Software | 315.9 | 324.6 (+2.8%) | 307.7 (-2.6%) |
| 20 | Software | 272.9 | 290.6 (+6.5%) | 274.9 (+0.7%) * |
| 30 | Software | 356.5 | 366.9 (+2.9%) | 353.0 (-1.0%) |
| 40 | Software | 231.6 | 247.3 (+6.8%) | 251.0 (+8.4%) |

\* Stage 2 diverges on level 20 (0.45% of pixels, both renderers); the other
seven cells are byte-identical across all three builds.

All end snapshots byte-identical. The ship binary is also smaller
(22.0 MB against 29.0 MB). Gains are smaller than OpenInfestation's (their
LTO 5–10%, x86-64-v2 2–4%): at 640x480 hidden the OpenGL runs sit on a flat
~4.5 ms a frame on every level but 40, so the frame is not all game code.

## Higher resolutions (2026-11-07)

`scripts/perf-bench.ps1` takes `-Width/-Height` (default 640x480).
`ship-1080p` vs `pgo-1080p`, 3000 frames, median of three:

| Level | Renderer | Ship (fps) | PGO (fps) |
| --- | --- | --- | --- |
| 13 | OpenGL | 221.3 | 222.6 (+0.6%) |
| 20 | OpenGL | 222.2 | 223.5 (+0.6%) |
| 30 | OpenGL | 222.3 | 223.5 (+0.5%) |
| 40 | OpenGL | 186.8 | 210.0 (+12.4%) |
| 13 | Software | 217.2 | 197.4 (-9.1%) |
| 20 | Software | 187.9 | 173.4 (-7.7%) |
| 30 | Software | 255.4 | 215.8 (-15.5%) |
| 40 | Software | 199.8 | 185.0 (-7.4%) |

All end snapshots byte-identical — including level 20 on both renderers.
The OpenGL frame is resolution-insensitive (same ~4.5 ms floor at
640x480, 1080p and 4K), so compiler gains carry over unchanged; the
software raster's CPU cost grows only weakly with pixels (level 13:
316 -> 217 fps for 6.75x pixels). The software path's PGO loss is
consistent across resolutions — the 1080p training profile is
GL/Vulkan-heavy (only three short software scenes), so PGO shapes the
binary for those paths at the software raster's expense.

At 3840x2160, OpenGL only (`ship-4k` vs `pgo-4k`): 218.3 -> 220.4 (+1.0%),
218.3 -> 219.5 (+0.5%), 221.3 -> 222.7 (+0.6%), 177.4 -> 185.0 (+4.3%),
all snapshots byte-identical. The GPU eats 4K for free; the frame stays
CPU-bound.

## Profile-guided builds (blocked: level 20 diverges)

`scripts/build-pgo.ps1` ports the Infestation PGO pipeline: an instrumented
`ship` build plays a silent training run (eight campaign levels on OpenGL
and the software raster, one with throttle, steering and fire on each), LLVM
merges the profile, and the `ship` build is made again from it. It builds
for MSVC, Rust's main Windows target (MinGW ships no profiler runtime),
with SDL2 bundled and the CRT statically linked; the main thread keeps its
8 MiB reserve. Rust's Linux toolchain has the runtime; the Linux counterpart
is not ported yet.

Stage 2 against stage 1 (the incremental value of PGO over ship): OpenGL
+0.5/+0.5/+0.4/+3.1%, software -5.2/-5.4/-3.8/+1.5% — no reliable gain at
640x480. Level 20 excluded from that sentence: stage 2 diverges there
(0.45% of pixels, central bbox, both renderers), while every other
level/renderer is byte-identical across all three builds. Each build is
self-consistent (three reps byte-identical), so this is systematic, not
flakiness.

A plain MSVC `ship` build without profile data (`target/pgo/msvc-control`)
is byte-identical to the MinGW builds on level 20, both renderers — so the
divergence comes from the profile-guided step, not the toolchain. Bisected
since: snapshots are identical at 500 and 1500 frames, and the actor logs
match exactly for the first 2971 frames; at frame ~2972 five actors (types
14, 16, 94, 94, 29) jump at once by up to tens of units while every other
actor matches, with no spawn/dispose/weapon event and no health change
around the jump. The same frame's shared-stream `rng` word also wobbles
(`82bfe7cb` vs `d227ccec` at frame 2971) and then reads equal again at 2972 —
a pure LCG advance cannot re-converge, so a conditional state assignment or
draw-count branch is implicated, firing in the same frame as the actor
jumps with the player state untouched throughout.

Cleared suspects, all verified in code: the run logs' `Particle class68
draw 0x00442240` stack-address warning comes from
`prepared_particle_draw_scale_raw` (`main.rs:14996`) via
`native_effective_draw_scale_raw` (`world_fx.rs:1701`), which is
deterministic (owned pool-VA math or an owned captured address, else the
explicit unadjusted-scale fallback) and presentation-only sprite scale — it
cannot move actors. Float perturbation is also out as a direct mechanism:
no FMA on x86-64-v2, no fast-math anywhere, so PGO cannot change a scalar
FP result. Wall-clock timing is out too: a level-20 run under eight busy
cores is byte-identical (snapshot and full event log) to the idle run.
Standing hypothesis: an uninitialized/padding read whose garbage follows
PGO's changed layout, feeding the shared-stream draw cadence or an
impulse/branch condition at frame ~2971. PGO is therefore **not** the
release path: `release.ps1` ships plain `ship`. `build-pgo.ps1` stays as
the reproducer.

The divergence is resolution-dependent: the same PGO binary is clean at
1920x1080 and 3840x2160 (below), diverging only at 640x480.
