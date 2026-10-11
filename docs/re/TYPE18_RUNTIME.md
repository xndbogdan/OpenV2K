# Type18 ground insects

This document owns the nineteen ordinary Type18 rows and their native owner,
[`native_type18`](../../crates/v2k-game/src/native_type18.rs). They run on the
shared ground host that also carries [Type122](TYPE122_RUNTIME.md). Every class
in their root reuses an existing task program. Static contact is in
[INSECT_STATIC_CONTACT.md](INSECT_STATIC_CONTACT.md). Capture and carrying are
in [TYPE17_CAPTURE.md](TYPE17_CAPTURE.md).

## Cohort

| World | Rows |
|---|---|
| 16, 21, 22, 23 | 2, 1, 1, 2 |
| 30, 33, 34, 36 | 3, 4, 3, 3 |

No row has a model override, animation or configuration. Every spawn has
parameter 1. Some carry authored pitch and roll.

## Section-12 row

| Field | Value |
|---|---|
| Model slots | 35 ×4, no model variables |
| Mass / health / capability | 100 / 8000 / 8 |
| Damage thresholds | `[0, 2000, 2500, 500, 200, 0, 0]` |
| Damage multipliers (Q8) | `[0, 256, 256, 128, 128, 0, 0]` |
| `+C0` default flags | `0x431` |
| Axis | 4352, candidate mask `0x25` |
| Components | A, B, C, D, E, H (eight records), J (one slot, `[0,20,350]`) |
| Emitter | method 20, 500000 µs, spread 100, aim threshold 30000, tolerance 3072, sound 69, `+12` 156 |
| Sounds | death 75, accepted hit 87; Aim sound 82 with period 0 |
| Surface | selectors `[0, 0]` |
| Alternate | class12 |

The root, in authored order:

| Rule | Weight | Class |
|---|---|---|
| Always | 1 | 4 Defecate Virus |
| PeopleNearby | 8 | 9 Capture People |
| PlayerNearby | 20 | 7 Search And Attack |
| Always | 3 | 33 Follow Beacons |
| FurnitureNearby | 1 | 26 Trash Furniture |

Three words set this row apart from the hosted profiles:

- **`+C0` 0x431 has no drag bit 8.** `E100` tests bit 4 for gravity, then
  `40E354` tests bit 8 before `44EC60`. Type18 therefore falls under gravity
  but takes neither wind nor drag. The ground host now gates `44EC60` on the
  effective bit.
- **Sub-D classifier `0x10`** runs `41FEB0`'s steepness arm alone: no water
  or object avoidance.
- **Aim sound 82 has period 0.** `AimAndFire` draws and plays its optional
  sound only from period `0x400`, so this cue never sounds and draws no word.

The mask `0x25` selects the player (`0x1`), the Type6 Main Base (`0x20`) and
other `0x04` actors.

## Birth

The ground host's authored entry runs `104B0/09A80/D4A0`, as for Type122:

- `09A80` builds H and D before A; A's `20450` consumes one word.
- `D4A0` grounds the body at the authored X/Z.
- `25680` draws the root with one word, after evaluating the three rules.
  None of them draws a word:
  - PeopleNearby selects capability `0xC00` within the strict axis;
  - PlayerNearby selects capability `0x1` within it;
  - FurnitureNearby scans terrain objects within a quarter of the current axis
    (1088), with no kind filter.
- `AC60` runs the class initializer: Defecate `B9E0`, Search and Capture
  `B6C0` (which copies axis `+04`), Follow Beacons `B740`, or Furniture `B7C0`.
  Each `06070` suffix draws its own words.

Furniture's target height uses the Type26/58 source policy. A successful scan
writes the retail height. A failed scan leaves the allocator's word, which the
owner refuses to invent: that visit fails closed. Rule 8 offers Furniture only
when an object is in range, so the first scan finds one.

## Living graph

Every class runs its existing ground-host program:

- class 4 wanders, and its terrain task infects cells (`NativeGroundResources`
  is mutable for Type18);
- class 9 acquires a person and pursues it; the pair callback `C910` attaches
  it to J, and the carry styles run until delivery or release;
- class 7 acquires, then Chase and method20 Aim;
- class 33 follows beacons;
- class 26 trashes furniture.

Every reselection (`C690`) evaluates the three rules again. The ground host's
`reselect` now reads FurnitureNearby from the resident world. Capture's
release, cleanup and delivery callbacks lend that world through
`CaptureContext::resources`.

## Hits and death

- **Hits.** Particle hits reselect through `C690`, as for Type122.
- **Lethal damage.** Type122's captor death runs: a carried person is released
  or killed, then class12 publishes.
- **Class12 world.** Its effective word is `C8 & !0x2015`. For Type18 that is
  `0x420`, not Type122's `0x428`: the corpse falls without drag. The class12
  world now admits `0x420`, and gates its drag on bit 8 too.
- **Main Base abort.** `10C10` takes the same class12 death.

## Contacts

- **Static walk.** `D920` runs `D9B0`'s generic crush (`0x400`) before the
  style hook. Furniture's `C890` delivers its packet, then reselects with
  rule 8 read against the cell it just damaged.
- **Pairs.** The captor lane admits Type18 for capture (`C910`), delivery
  (`D0B0`) and pair damage.
- **Class12 corpse.** The corpse takes the terrain and water contact through
  `native_actor_surface_contact`.

## Held boundary

A carried four-choice person that drowns fails closed in its own `E370`,
which admits no `CE90` release hook. It then blocks its captor's next
release. [TYPE28_RUNTIME.md](TYPE28_RUNTIME.md#held-boundary-a-carried-person-drowning)
owns that chain, which every captor shares.

## Validation

[`native_type18/tests.rs`](../../crates/v2k-game/src/native_type18/tests.rs)
covers:

- all 19 births and their graphs;
- ten-second cohorts in all eight worlds under the real scheduler and the late
  static walk;
- a lethal hit's class12 death, then five seconds of the `0x420` class12 world;
- method20 shots after a player comes near;
- world 21's capture of a Type86 person during a minute of carrying and
  pair walks.

[The abort test](../../crates/v2k-game/src/main_base_abort_production/native/type18_tests.rs)
checks class12 in every world, once per corpse.
