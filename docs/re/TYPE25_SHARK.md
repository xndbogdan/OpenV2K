# Type25 shark

This document owns the six authored Type25 rows (worlds 22, 23 and 34), their
Hive births and the class69 Shark Attack program. The sharks share the
[fish owner](FISH_RUNTIME.md) ([`shared_fish`](../../crates/v2k-game/src/shared_fish.rs))
and its B/D/F mover; the program is
[`shark_attack`](../../crates/v2k-game/src/shark_attack.rs).

## Section-12 row

| Field | Value |
|---|---|
| Model slots | 1238 ×4, 6 model variables |
| Mass / health / capability | 100 / 10000 / 8 |
| Damage thresholds | `[0, 6000, 2000, 0, 200, 0, 0]` |
| Damage multipliers (Q8) | `[0, 256, 256, 512, 128, 0, 0]` |
| `+C0` default flags | `0xC`: no `0x2000` environment bit |
| Axis | 7680 / `0x3` |
| Choices / rule | Always ×1 class6, PlayerNearby ×40 class69, Always ×6 class5; rule 1 |
| Alternate | class12 |
| Components | B, D, F; no emitter |
| Sounds | none (death, hits, detailed) |

- **B:** projection 1000000, correction 1000000.
- **D:** divisor 96, yaw coupled into roll, probes 1024/256, classifier `0x10`
  (Type18/28's steepness arm).
- **F:** clearance 500 + up to 1000, speed 1000, animation mode 0, selectors
  1-6.

## Births

Authored: world 22 spawns 12, 21 and 22; world 23 spawns 15 and 16; world 34
spawn 1. The constructor is the fish's: `104B0/09A80`, Sub-D, Sub-F's two or
three words, then `25680`'s single word. PlayerNearby (rule 6) scans the
earlier allocations, where an arriving player stands first.

Hive rows: world 34's Hive (spawn 0) births Type25 in the running session
(flags 1); the Hives of worlds 22 (spawn 28) and 33 (spawn 1) only after the
Main Base is lost (flags 3). A Hive birth takes the zeroed record through the
same constructor.

## Scheduling

Without `0x2000`, `FUN_0040E870` runs `A800` in mode 1 (Sub-F only) and the
whole tail, so a distant shark keeps hunting; `FUN_0040DCA0` never touches
motion bit `0x40000`, which birth set. `FUN_0040E100` skips gravity for
bit 4 and runs the wind/drag leaf for bit 8, as for the fish.

## Class69 Shark Attack

`FUN_0040D2D0` clears slots 1 and 2; `FUN_004056D0` builds the Primary
through `FUN_00401020` with no `06070` suffix and no lifetime.
`FUN_00405D50`'s 0x3C record: prey handle, phase, a microsecond timer,
an `FUN_004012E0` target record (null handle), and two speeds drawn as
`((word & 0x3F) + 0x80) × F+4 >> 8`, plus the current speed.

`FUN_00405760`, before `FUN_00401430` with the record's target:

- Phase 0 runs `FUN_00405950`: the nearest capability-bit-0 body by
  `FUN_00425370` distance becomes prey (phase 1); each nearer one draws a
  timer word (`<< 7`).
- The timer counts down by the callback time.
- Phases 1-2 run `FUN_004059E0`: phase 1 rings the prey at 1,400 units at
  speed `s1/2 + s0`; phase 2 swims 1,400 units onward from the body with a
  speed blended by distance, and past 0x15E0 attacks (phase 3, timer
  `(word + 0x2625A) × 64`). An expired timer selects phase 2 with
  `(word + 0xF424) × 64`; beyond 0x3139, or a vanished prey, ends the hunt.
- Phase 3 runs `FUN_00405C00`: straight at the prey, speed `s0` plus `s1`
  scaled by the forward projection over distance; a prey behind within
  1,400 units, or behind when the timer ends (new timer
  `(word + 0x3D09) × 128`), returns to phase 1.
- The goal height is capped at the sea less 0x80. Sub-F's `+3D` (follow the
  goal height) is set only while attacking; otherwise, when the body is more
  than 16 units off the goal height, `+34` becomes that height above the
  cell. `+38` takes the current speed.

The result tag `9C00` reaches style `4C7D20`'s null `+04`, so the hunt never
ends by itself. Record words `[10]`/`[11]` (a sound and its period) lie in
the target record's untouched zeros: the shark never plays that sound.
Style `4C7D20` holds `C690` at `+10`, `+20`, `+24` and `+28`.

### The bite

`FUN_00405E10` is the Primary's A900 hook. A touched body behind the shark
(`FUN_0041E930`) ends an attack (phase 2, `(word + 0xF424) × 64`). Touching
its prey ahead sets the timer to 100 ms and delivers a checked packet
through `FUN_00415040`: channel 1, 8,000, kind 25, scaled by the global
frame delta `>> 2` over 25,000 (1,600 a 20-ms frame). Below the sea it adds
`(delta × 50) >> 20` class-45 bubbles (at least one) at the prey, moving with
the shark.

The native pair lane owns player-shark pairs in both directions. The player's
style callback `FUN_00447D70` sees capability 8 (no `0xC00`), so it only
feeds the pad its relative-speed vibration: the root of the two velocities'
squared difference (`+9C..+A0`), as for weapons.

## Death

A lethal hit runs `10C10`, `DB80` (null `+2C`) and alternate class12, whose
`FUN_0040C620` clears slots 1 and 2 and builds `FUN_00404120`'s 9,000-ms task:
`06070` resets Sub-F and the body gains vertical velocity 500. The class12
style clears `0x2015`, so the effective word is 8 and `E100` applies
gravity. `FUN_00404220`:

- On a coarse visit it takes `4BE170` (tag `9C01`): the terminal stages
  removal at once.
- On a detailed visit it flips the body (`FUN_0041EC70` with surface mode 0,
  as there is no Sub-C), runs `01430` with a null target (Sub-F's dying arm,
  then Sub-B) and damps X and Z.

The fish owner keeps ticking the dying shark under its fresh receipt. The
Main Base abort's `10C10` takes the same death: worlds 23 and 34 now reach
their Type37 and Type117 rows.

## Not run

- Shark pairs with bodies other than the player stay outside the native pair
  lane, as every fish pair does: a fish touching the shark neither breaks an
  attack nor pushes it.
- No fish reaches the `11AD0` terrain walk, so the shark's bare-terrain
  `C690` (`+10`) never runs.

## Validation

[`shark_tests`](../../crates/v2k-game/src/shared_fish/shark_tests.rs) cover
the births and rows, a hunt that circles and attacks a nearby player, the
bite through the hook and through the player's pair lane, class12's flip,
fall and coarse terminal, and world 34's Hive birth;
[`shark_attack/tests`](../../crates/v2k-game/src/shark_attack/tests.rs) the
record arithmetic. The abort matrix moves worlds 23 and 34 to Type37 and
Type117.

The player's modifier `FUN_004484A0` walks the controller's `+220` owned
list. The world-22 arrival carries a Type68 ballast in the body's Sub-J
cargo, which that list never holds; the bite (and every other checked hit on
the player there) now reaches the hull instead of stopping at the ballast.

Retail lab run of world 22 (`skip_world.py 22`, idle player): bites
(`415040` from `00405EC4`, kind 25, 8,000) on the player start about 30 s
into the world, 1,518 passes after its load, and take 400 health a frame:
the idle player dies 108 frames later. The port's sharks wander out of
step with retail's random words and did not meet the player from that
arrival within three minutes; started 3,000 units from the world-22
shark, the port's shark rings, attacks and bites at the same 400 a frame,
killing the player in 100 frames. World 34's Hive births its Type25s at
port ticks 501, 1002, 1503 and 2004 against retail's 502, 1003, 1504 and
2009.
