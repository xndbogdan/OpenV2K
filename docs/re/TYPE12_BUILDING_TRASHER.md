# Type12 and Type127 building-trashers

This document owns the eight ordinary Type12 rows (worlds 31 and 35), the 25
Type127 rows (world 27) and their shared native owner,
[`native_type12`](../../crates/v2k-game/src/native_type12.rs). Both run on the
shared ground host. Static contact is in
[INSECT_STATIC_CONTACT.md](INSECT_STATIC_CONTACT.md); the class1 and class63
terminals are in [ENTITY_DAMAGE_AND_DEATH.md](ENTITY_DAMAGE_AND_DEATH.md).

## Section-12 rows

Type127's row is Type12's apart from five words:

| Field | Type12 | Type127 |
|---|---|---|
| Axis radius / `+04` (class7 candidate mask) | 3584 / `0x5` | 4096 / `0x1` |
| Terrain-contact task lifetime | 67 ms | 0 |
| Generic hit sound | 87 | none |
| Search prelude sound | 75 | none |
| Alternate | class1 | class63 |

The shared fields:

| Field | Value |
|---|---|
| Model slots | 684 ×4, 82 model variables |
| Mass / health / capability | 200 / 10000 / 8 |
| Damage thresholds | `[0, 2000, 2000, 2000, 200, 0, 0]` |
| Damage multipliers (Q8) | `[0, 256, 256, 0, 128, 0, 0]` |
| `+C0` default flags | `0x39` (no crush) |
| Choices / rule | Always ×2 class33, PlayerNearby ×8 class7, PeopleNearby ×3 class9, BuildingsNearby ×1 class15; rule 1 |
| Components | A, B, C, D, E, H (eight records), K, L; no J |
| Emitter | method 24, 200000 µs, spread 256, aim threshold 4000, speed 1500, tolerance 3072, `+12` 56, sound 82 |
| Sounds | death 75, accepted hit 6 |

Descriptors:

- **A:** acceleration 3000, overspeed correction −3000, target speed 400.
- **B:** projection 1000000, correction 100000.
- **C:** clearance 75, lift range 75, strength `0x300000`, near boost 100,
  damping 200, surface mode 0.
- **D:** Type47's: divisor 64, probes 512/256, classifier `0x13`.
- **H:** phase rate `0x30000000`, resolver `0x20000000`, axis mode 0. Record
  `i` binds vertices `[110+i, 118+i, 126+i]`.
- **K/L:** K `[81, 82]`, L `[2, 1, 244, 1, 120, 5]`.

## Birth

The authored entry runs `104B0/09A80/D4A0`:

- `FUN_00409A80` builds H, D, K, L, A and E. The variable bank is the type's
  `+0x110` words, 82 here, zeroed; `FUN_0040A950` binds selector `n` to word
  `n-1`. K writes words 80 and 81, beyond the 63 a model's six-bit operand can
  read, so its outputs never reach the drawing; L writes words 1 and 0.
- A's `20450` consumes one word; `D4A0` grounds the body.
- `25680` draws the root with one word after the rules. PlayerNearby and
  PeopleNearby (mask `0xC00`) scan the live candidates; BuildingsNearby
  (`FUN_004166B0`) asks `FUN_004230C0` for a kind-3 object within the full
  strict axis. None draws a word.

## Living graph

- **Class33 Follow Beacons**, **class7 Search** (method24 Aim) and **class9
  Capture** run their shared programs.
- **Capture without a J.** With no Sub-J, `C910`'s capacity test reads full,
  so a contact in pursuit (`4C8038`) calls the person's `10C10`: the person
  dies and nothing is carried, as Type40's capture does.
- **Class15 Trash Buildings.** Its style `4C76F0` holds initializer
  `FUN_0040B7C0` with argument 3, the same initializer Trash Furniture's
  style `4C7738` holds with argument -1. `FUN_00401CA0` builds the 2000-ms
  furniture task with kind filter 3: it scans the full axis for a kind-3
  object, walks to it, and its style's `FUN_0040C890` deals any touched
  static object a channel-1 40000 packet, then reselects the whole root.

## Hits and death

- A nonlethal hit runs `C690`, which reselects the root.
- Lethal damage runs the row's alternate on the shared class49 terminal:
  Type12's `BAC0` clears every task (corpse `4C7150`); Type127's `BC90`
  keeps them and drops the Type61 authored in its `+88` (corpse `4C7198`).
  `BAF0`'s scatter takes the nonnull-A branch: ten class37 particles.
- The Main Base abort's `10C10` takes the same deaths. Worlds 27 and 35 now
  run every actor's death when lost.

`E370` drowning fails closed, as for Type38: its terminal needs the static
world the surface step holds.

## Validation

[`native_type12/tests.rs`](../../crates/v2k-game/src/native_type12/tests.rs)
covers the 33 births, their rows, bank size, graphs and source profiles;
ten-second cohorts in worlds 27, 31 and 35 under the real scheduler and the
late static walk; lethal class1/class63 hits and the corpse's static walk;
and rule 9 against the task's own scan.
[The abort test](../../crates/v2k-game/src/main_base_abort_production/native/type12_tests.rs)
checks both deaths.

Retail lab runs of worlds 31 and 35 (`compare.py`, idle player, a minute):
in world 31 neither retail nor the port lost the world; one port run lost it
to the casualty row after a Type12 in Capture pursuit touched a Type95, a
branch retail's run did not reach because its random choices parted. In
world 35 both lose the world (retail at tick 1163, the port at 1907); both
abort walks then run the eight Type3 splits and three Type60 rings.
