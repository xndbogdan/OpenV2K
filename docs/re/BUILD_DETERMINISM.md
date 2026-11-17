# Build determinism and the level-20 PGO divergence

This note owns the port's build-to-build reproducibility defect. It is not a
retail-fidelity claim: the plain `ship` build is the shipped binary, and its
output is deterministic. The defect is that one alternative code layout
(profile-guided optimisation) changes simulation state on level 20, which means
the port still has a layout-sensitive read that a future allocator, codegen or
platform change could reach in a shipped build.

The measurements and the surrounding optimization history are in
[PERFORMANCE.md](../PERFORMANCE.md#profile-guided-builds-blocked-level-20-diverges).

## Observed

- A plain MSVC `ship` build without profile data is byte-identical to the
  MinGW `ship` build on level 20, both renderers. The divergence is caused by
  the profile-guided step, not the toolchain or the C runtime.
- PGO changes 0.45% of level-20 pixels (a central bounding box) at 640x480 on
  both renderers. Every other tested level/renderer is byte-identical across
  the plain, MSVC-control and PGO builds.
- The divergence is resolution-dependent: the same PGO binary is clean at
  1920x1080 and 3840x2160, and only diverges at 640x480.
- Snapshots are identical at 500 and 1500 frames. The actor logs match
  exactly through frame 2971. At about frame 2972 five actors (types 14, 16,
  94, 94 and 29) jump at once by up to tens of units, with no spawn,
  disposal, weapon event or health change around the jump. The same frame's
  shared-stream RNG word reads `82bfe7cb` vs `d227ccec`, then reads equal
  again at frame 2972. A pure LCG advance cannot re-converge, so the
  hypothesis is a conditional assignment or draw-count branch, not a
  different RNG draw count alone.

## Cleared

- The `Particle class68 draw 0x00442240` stack-address warning is the
  presentation-only `prepared_particle_draw_scale_raw` path
  (`crates/v2k-game/src/main.rs`, `world_fx.rs`); it is deterministic and
  cannot move actors.
- Scalar floating-point reassociation: the port does not enable fast-math and
  x86-64-v2 has no FMA contraction; Rust does not reassociate unasked.
- Wall-clock scheduling: a level-20 run under eight busy cores is
  byte-identical (snapshot and full event log) to the idle run.

## Standing hypothesis

An uninitialized or padding byte is read, and PGO's changed code layout gives
it a different value. That value feeds the shared-stream draw cadence or an
impulse/branch condition at frame ~2971. This is a hypothesis, not a proven
cause.

## Probe

Reproduce with the current `ship` and PGO binaries at 640x480, level 20,
fixed 20 ms steps, 3000 frames, recording the event log and a snapshot:

```powershell
cargo build --profile ship -p v2k-game
pwsh -NoProfile -File scripts/build-pgo.ps1
pwsh -NoProfile -File scripts/perf-bench.ps1 -Tag pgo-level20 `
  -Exe target/pgo/use/x86_64-pc-windows-msvc/ship/v2k-game.exe `
  -Levels 20 -Renderers opengl,software -Frames 3000 -Repeats 1
```

Then:

1. Compare the frame-2971/2972 event-log lines (actor positions, the `rng`
   word, draw counts) between the plain and PGO binaries.
2. Run the plain `ship` build with a poisoning allocator (a feature-gated
   fill of fresh allocations with a non-zero pattern) and diff the same
   snapshot. A value-dependent difference points at the uninitialized read;
   an identical frame rules out the simple poison hypothesis.
3. If poisoning does not reproduce it, bisect the first divergent read by
   instrumenting the frame-2971 draw cadence and the five actors' task
   inputs, not by adding a level-20 special case.

## Acceptance

- Level-20 snapshots are byte-identical between plain `ship` and PGO at
  640x480, 1920x1080 and 3840x2160, both renderers.
- The plain `ship` build is byte-identical under two different poison fills.
- `scripts/build-pgo.ps1` may then be described as a performance experiment;
  it does not become the release path without this acceptance.
