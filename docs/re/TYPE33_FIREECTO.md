# Type33 fireectos

This document owns the ten ordinary Type33 rows (three in world 42, four in
43, three in 47). They share the [Type30 owner](ALPINE_INSECTS.md#native-type30-owner)
([`native_type30`](../../crates/v2k-game/src/native_type30.rs)) as its second
row and run on the shared ground host. Guard Location's tasks are in
[GUARD_LOCATION_TASK.md](GUARD_LOCATION_TASK.md); class12 is in
[ENTITY_DAMAGE_AND_DEATH.md](ENTITY_DAMAGE_AND_DEATH.md).

## Section-12 row

Type33 has Type30's A/B/C/D/E/H/K/L shape, mass, capability and class12
alternate. The row's own values:

| Field | Value |
|---|---|
| Model slots | 1171 ×4, 4 model variables |
| Health / `+C0` | 6000 / `0x31` |
| Damage thresholds | `[0, 8000, 3000, 0, 200, 0, 0]` |
| Damage multipliers (Q8) | `[0, 256, 256, 64, 128, 128, 0]` |
| Axis / `+04` | 3072 / 5 |
| Choices / rule | Always ×1 class6, Always ×5 class32; rule 3 |
| Emitter | method 24, 400000 µs, spread 256, threshold 12000, speed 1500, tolerance 3072, sound 105, `+12` 48 |
| Sounds | accepted hit 109, death 75 |

Components:

- **A:** acceleration 1500, overspeed −3000, target speed 300.
- **B:** projection 10000, correction 1000.
- **C:** clearance 220, lift range 55, strength `0x60000`, near boost 100,
  damping 200, surface mode 1.
- **D:** divisor 100, probes 512/256, classifier flags 0: Type38's no-pitch
  `1F660` path with the classifier skipped, as the flyers' is.
- **H:** four records, phase rate `0x10000000`, resolver `0x20000000`, vertices
  `[44, 46, 48]`, `[45, 47, 49]`, `[34, 36, 38]`, `[35, 37, 39]`; each pair
  depends on its twin.
- **K/L:** K `[4, 3]`, L `[1, 2, 64, 31, 64, 31]`.

## Birth and selection

The authored entry runs `104B0/09A80/D4A0`. `+C0`'s bit `0x20` grounds the
body: `D4A0` writes terrain height plus C's clearance 220 and copies that pose
to the immutable `+90` anchor. A's `20450` draws one word. `25680` draws one
word over the two Always choices, so a sixth of births wander and the rest
guard.

## Living graph

- **Class6 Wander Near Location.** `FUN_0040AD10` clears slots 2 and 1 and
  builds `FUN_00402E20`'s 5000-ms wander, through `06070`'s Sub-H/Sub-A
  suffix. `FUN_00402EB0` draws one word and, one time in 64, two more for a
  target within ±1024 of `+90` on X and Z, keeping its Y. Its style
  `4C79C0` clears `0x21000` from the effective flags, which `0x31` lacks.
- **Class32 Guard Location.** `FUN_0040B5A0` copies axis `+04`, clears slot
  2, then builds `01F80`'s search and the same 5000-ms wander. A candidate
  found by `01FB0` runs `C7D0` into style `4C7C00` and Search's `ADE0` Chase
  and method24 Aim.
- **Hooks.** All three styles hold `C690` at `+20/+24/+28` and null contact,
  water and death hooks. `02E20` installs `02B10`'s `02CA0`/`02DA0`, so the
  wander takes the shared static and pair redirects.
- **Sub-C surface mode 1.** C's lift samples the waved sea over water, and so
  do `E640`'s attitude probes. `E100`'s water response, which reads C `+0C`,
  compares the body with the solid-or-sea cell height. Sub-H feet stand on
  terrain or water.

## Hits and death

A nonlethal hit runs `C690`. A lethal one, a radial or the Main Base abort's
`10C10` takes class12 (`Flip Over And Die`); its retained K/L mover samples
the waved sea at the current tick.

## Validation

[Type33 tests](../../crates/v2k-game/src/native_type30/type33_tests.rs)
cover the ten births, twenty-second cohorts under the scheduler with the
surface and static walks, a guard taking the player into Chase and Aim, and
a lethal hit's class12.
[The abort test](../../crates/v2k-game/src/main_base_abort_production/native/ground_split_tests.rs)
checks class12 in all three worlds; world 43's Main Base loss now gets past its
fireectos to a Type36.

Retail lab runs (`compare.py`, idle player after Skip to World) of worlds 42
and 47: each fireecto's track stays within a few dozen units of retail's for
the first 80 to 120 ticks, its hover height included, before random
retargets part them. Retail loses world 42 at tick 438 and world 43 at 462;
the port loses neither in the window.
