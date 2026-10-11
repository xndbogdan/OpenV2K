# Type76 and Type77 ground insects

This document owns the six ordinary Type76 rows, the eight Type77 rows and
their shared native owner,
[`native_type76`](../../crates/v2k-game/src/native_type76.rs). Both run on the
ground host that carries [Type18](TYPE18_RUNTIME.md) and
[Type28](TYPE28_RUNTIME.md). Neither captures; both carry an emitter and take
class12. Each class in their roots is an existing shared program.

## Cohort

| World | Type76 | Type77 |
|---|---|---|
| 26 | 2 | 1 |
| 31 | 4 | 0 |
| 37, 39, 40 | 0 | 2, 3, 2 |

Every spawn is plain: no model override, animation or configuration, and
parameter 1. Only world 40 has terrain below its sea, so only its Type77s can
drown. Worlds 26, 37 and 39 park the sea at the dry marker (`-6144`). World
31's sea (`-2180`) lies below its lowest terrain corner (`-2144`).

## Section-12 rows

| Field | Type76 | Type77 |
|---|---|---|
| Model slots | 269 ×4 | 271 ×4 |
| Mass / health / capability | 100 / 7000 / 8 | 100 / 10000 / 8 |
| Damage thresholds | `[0, 4000, 5000, 0, 200, 0, 0]` | `[0, 4000, 4000, 0, 200, 0, 0]` |
| Damage multipliers (Q8) | `[0, 256, 256, 1024, 0, 0, 0]` | `[0, 256, 256, 128, 128, 0, 0]` |
| `+C0` default flags | `0x39` (no generic crush) | `0x439` |
| Axis | 3584, candidate mask 7 | 3584, candidate mask `0x105` |
| Sub-A | 1500 / -3000 / 450 | 1500 / -3000 / 300 |
| Emitter | method 24, 150000 µs, tolerance 5120, `+12` 114 | method 30, 300000 µs, tolerance 2560, `+12` 168 |
| Detailed sound | none | healthy cue 80, period 2 s |
| Defecate terrain task | none | 67 ms |
| Alternate | class12 | class12 |

Shared by both rows:

- components A, B, C, D, E and H (six records); no F, G, J, K or L;
- emitter spread 256, aim threshold 8000 and sound 70, with no bindings;
- sounds: death 75 and accepted hit 84;
- surface selectors `[1, 0]` with a 2000 ms lifetime;
- Sub-D `INTRO2_TYPE58_SUB_D` (divisor 96, probes 512/200, classifier `0x13`);
- Sub-C clearance 50, range 75, strength `0x300000`.

The six H records resolve with flags `0x20000000`. Their phase rate is
`0x30000000` for Type76 and `0x20000000` for Type77. Both rows pair their legs
the same way: records 0..5 depend on `{1,2,3,4}`, `{0,2,3,5}`, `{0,1,4,5}`,
`{0,1,4,5}`, `{0,2,3,5}` and `{1,2,3,4}`. Only their vertex references differ.
Type77's `low_health_effect_words` `[80, 0, 0]` are its detailed-sound cue,
already played by DCA0's planner.

The roots, in authored order:

| Rule | Type76 | Type77 | Class |
|---|---|---|---|
| Always | 1 | 10 | 33 Follow Beacons |
| FurnitureNearby | 5 | 5 | 26 Trash Furniture |
| UnderAttack | 10 | 20 | 7 Search |
| Always | — | 1 | 4 Defecate Virus |
| Always | — | 2 | 5 Move About Aimlessly |

A fresh birth's `+34` is zero, so UnderAttack is false at birth. After a hit it
weighs Search for 250 ticks. FurnitureNearby is rule 8: an object within a
quarter of the strict axis.

## Birth and graph

Construction is Type18's without J. `09A80` builds H and D before A. A's
`20450` consumes its own word, then the selector word, then the class's
`06070` suffix.

Move About Aimlessly (`ACD0`) is new to the shared root publisher. It clears
the tertiary slot, then the secondary, then installs `402B10(owner, 0, 5000)`
as the primary and runs `06070` once. Search (class 7) acquires, then runs
Chase and the row's Aim. Each row's emitter is its own ballistic profile
(`TYP76` / `TYP77` in the FIFO), and shots drain through the native projectile
path like Type18's.

## Death

A lethal hit, a radial, a static or pair contact, the abort and `E370`
drowning all take class12. `E370`'s 2000 ms lifecycle lands on the 100th
20 ms visit below `sea - extent/4`.

## Validation

[`native_type76/tests.rs`](../../crates/v2k-game/src/native_type76/tests.rs)
covers:

- all 14 births;
- ten-second cohorts in all five worlds under the real scheduler and the late
  static walk;
- a minute of every world with the live-order pair walk, without a block;
- a lethal hit's class12 death;
- world 40's Type77 drowning on a deep seabed after exactly 2000 ms;
- a hit Type76 (world 31) and Type77 (world 39) firing at a nearby player
  through the native FIFO.

[The abort test](../../crates/v2k-game/src/main_base_abort_production/native/type76_tests.rs)
checks class12 in every world, once per corpse.
