# Build determinism and the level-20 PGO divergence

This note owns the port's build-to-build reproducibility defect. It is not a
retail-fidelity claim: the plain `ship` build is the shipped binary, and its
output is deterministic. The defect was that one alternative code layout
(profile-guided optimisation) changed simulation state on level 20, which
meant the port had a layout-sensitive handoff that a future allocator, codegen
or platform change could reach in a shipped build.

The measurements and the surrounding optimization history are in
[PERFORMANCE.md](../PERFORMANCE.md#profile-guided-builds).

## Observed

- A plain MSVC `ship` build without profile data is byte-identical to the
  MinGW `ship` build on level 20, both renderers. The divergence was caused
  by the profile-guided step, not the toolchain or the C runtime.
- PGO changed 0.45% of level-20 pixels (a central bounding box) at 640x480 on
  both renderers. Every other tested level/renderer was byte-identical across
  the plain, MSVC-control and PGO builds.
- The divergence was resolution-dependent: the same PGO binary was clean at
  1920x1080 and 3840x2160, and only diverged at 640x480.
- Snapshots were identical at 500 and 1500 frames. The actor logs matched
  exactly through frame 2971. At about frame 2972 five actors (types 14, 16,
  94, 94 and 29) jumped at once by up to tens of units, with no spawn,
  disposal, weapon event or health change around the jump. The same frame's
  shared-stream RNG word read `82bfe7cb` vs `d227ccec`, then read equal again
  at frame 2972.
- A frame-by-frame trace of the particle pool put the first difference at
  frame 2949: slot 127 held a class-87 shot at `[-18995, 273, 264]` in the
  plain build and at `[0, 0, 0]` in the PGO build. Entity 14 (type 15) fires
  that shot in the same draw phase. Instrumenting the draw and drain showed
  the resolved origin, the (absent) muzzle and `time_offset_raw == 0` were
  identical and correct in both builds, yet the PGO materialization received
  `[0, 0, 0]`.
- Adding those traces changed code layout and removed the PGO divergence;
  both traced builds then agreed. That is the signature of a layout-sensitive
  copy or select in the shot drain, not of a value error in its inputs.

## Cleared

- The `Particle class68 draw 0x00442240` stack-address warning is the
  presentation-only `prepared_particle_draw_scale_raw` path
  (`crates/v2k-game/src/main.rs`, `world_fx.rs`); it is deterministic and
  cannot move actors.
- Scalar floating-point reassociation: the port does not enable fast-math and
  x86-64-v2 has no FMA contraction; Rust does not reassociate unasked.
- Wall-clock scheduling: a level-20 run under eight busy cores is
  byte-identical (snapshot and full event log) to the idle run, and the
  divergence reproduces deterministically per binary.
- MSVC versus MinGW: a current MSVC `ship` build without profile data matches
  the current MinGW `ship` build byte-for-byte (frame-2971 word `82bfe7cb`,
  same end-snapshot SHA-256), so the CRT and compiler are not the cause.

## Fix

`intro2_flyer_aim` and `ordinary_type47_live` now take a drained shot's
position from the resolved origin when `time_offset_raw == 0`. That is the
case where retail's reanchor applies no correction, so the drained word is
the source itself; reading the origin directly makes the no-correction case
explicit at the call site instead of carrying the helper's copy. The
velocity words are unchanged.

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

For a regression, compare the frame-2971/2972 event-log lines (actor
positions, the `rng` word) and the end snapshots between the plain and PGO
binaries, then trace the particle pool's first differing slot as above.

## Acceptance

- Level-20 snapshots are byte-identical between plain `ship` and PGO at
  640x480, 1920x1080 and 3840x2160, both renderers. **Met on 2026-11-17**:
  - 640x480: software `7F9A8D53F96C8BE44AB74AB91CDB9079E064CC94B56329C3E10658E3994B977A`,
    OpenGL `D435E515F1C4A4C126C05120AB4A903FF0801FD33077AAD714FECDF6D3C40264`,
    frame-2971 RNG `82bfe7cb`.
  - 1920x1080: software `19CA408E4B77BAB178D85083230AED7F97593E9107E1202F082BD87EE7C5754F`,
    OpenGL `5E393B8146199C2E3358CFF43F468FD60CB02756D5A57CB78CDF16DFD9DD9D3E`,
    frame-2971 RNG `82bfe7cb`.
  - 3840x2160: OpenGL `D47F26476E31065256C1AD2B4D7271355E34D839294BDB61BD2AFB4835FD865C`;
    software `19CA408E4B77BAB178D85083230AED7F97593E9107E1202F082BD87EE7C5754F`,
    frame-2971 RNG `d7db27cc` in both builds. The software raster's logical
    surface is capped at 1024x768 and upscaled, so its 4K snapshot is the
    same pixel surface as 1080p while the 4K frame's simulation (including
    the changed RNG word) still runs at the 4K window. The OpenGL pair is a
    true 3840x2160 pixel comparison.
- The fix keeps the drained velocity and the zero-offset position
  equivalent to the helper's result by construction.
- `scripts/build-pgo.ps1` remains a performance experiment; it does not
  become the release path.
