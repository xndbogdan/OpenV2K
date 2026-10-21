# Type38, Type129, Type119 and Type32 ground shooters

This document owns the ten ordinary Type38 rows (worlds 42 and 43), the 25
Type129 rows (world 41), the three Type119 walkers (world 46), the three
Type32 walkers (world 32) and their shared native owner,
[`native_type38`](../../crates/v2k-game/src/native_type38.rs). Both run on the
shared ground host that also carries
[Type30](ALPINE_INSECTS.md#native-type30-owner). Static contact is in
[INSECT_STATIC_CONTACT.md](INSECT_STATIC_CONTACT.md). The class1 and class63
terminals are in [ENTITY_DAMAGE_AND_DEATH.md](ENTITY_DAMAGE_AND_DEATH.md).

## Section-12 rows

Type129's row is Type38's apart from four words:

| Field | Type38 | Type129 |
|---|---|---|
| Emitter sound | 82 | 0 |
| Target-warning sound | 87 | none |
| Axis `+04` (class7 candidate mask) | `0xC25` | `0xC01` |
| Alternate | class1 | class63 |

The shared fields:

| Field | Value |
|---|---|
| Model slots | 1131 ×4, four model variables |
| Mass / health / capability | 100 / 15000 / 8 |
| Damage thresholds | `[0, 10000, 5000, 10000, 200, 500, 0]` |
| Damage multipliers (Q8) | `[0, 128, 128, 128, 128, 512, 0]` |
| `+C0` default flags | `0x39` |
| Axis radius | 3840 |
| Choices / rule | Always ×1 class5, PlayerNearby ×5 class7; rule 1 |
| Components | A, B, C, D, E, H (eight records), K, L |
| Emitter | method 10, 600000 µs, spread 256, aim threshold 4000, speed 1500, tolerance 5120, `+12` 122 |
| Sounds | death 75, accepted hit 82 |
| Surface | selectors `[1, 0]`, 500 ms |

Descriptors:

- **A:** acceleration 1500, overspeed correction −3000, target speed 300.
- **B:** both rates 100000.
- **C:** clearance 150, lift range 75, strength `0x300000`, near boost 100,
  damping 200, surface mode 0.
- **D:** divisor 100, forward probe 512, lateral probe 256, classifier `0x13`,
  no yaw coupling or pitch steering.
- **H:** every record has phase and resolver `0x20000000` and axis mode 0. The
  vertex triples run `[4,34,42]` to `[23,41,49]` in two groups of four, each
  record depending on the other three of its group.

Both masks hold the player's `0x1` and `0x400/0x800`. Type38's also holds
`0x20` and `0x04`, so it can select the Type6 Main Base (`0x20`) and twenty
other types that Type129 ignores.

### Type119

Type119 (model 1161, three rows in world 46) has the same component shape and
classes, and its own data:

| Field | Type119 |
|---|---|
| Mass / health | 500 / 20000 |
| Damage thresholds | `[0, 10000, 5000, 25000, 200, 0, 0]` |
| Damage multipliers (Q8) | `[0, 0, 0, 256, 0, 0, 0]`: only channel 3 hurts it |
| `+C0` default flags | `0x439`: `D920` runs the generic crush |
| Axis | 3840, `+04` `0xC05` |
| Choices | Always ×1 class5, Always ×1 class7 |
| Emitter | method 10, 800000 µs, spread 256, aim threshold 8000, speed 1500, tolerance 3840, `+12` 48, sound 82 |
| Sounds | no death, hit or target-warning sound |
| Surface | selectors `[1, 0]`, 2000 ms |
| K / L | `[3,4]` / `[2,1,64,31,160,15]`, Type30's binding |

- **A:** acceleration 1500, correction −3000, target speed 350.
- **B:** both rates 10000.
- **C:** clearance 75, lift range 50, otherwise Type38's.
- **D:** divisor 64, probes 800/300, classifier `0x13`.
- **H:** four legs with phase rate `0x10000000`, vertex triples `[24,26,28]`,
  `[25,27,29]`, `[28,32,34]` and `[29,33,35]`, stepping in diagonal pairs
  (records 0/3 wait on 1/2 and the reverse). The descriptor's `+02` is sound
  90: when a record's phase completes, `41D120` draws one word and queues sound
  90 at the body's position, full gain, at rate `0xE000 + (word & 0xFFFF)/4`.

Both of its choices are Always, so its birth draws no PlayerNearby evidence.
Its alternate is class1, as Type38's.

### Type32

Type32 (model 1098, three rows in world 32) is Type119's row (components, H
legs and step sound, K/L, A/B/C/D, emitter, axis and `+C0`) apart from:

| Field | Type32 |
|---|---|
| Mass / health | 600 / 10000 |
| Damage thresholds | `[0, 10000, 10000, 10000, 200, 0, 0]` |
| Damage multipliers (Q8) | all zero: no channel reaches its health |
| Surface | selectors `[1, 0]`, 6000 ms |
| Choices | Always ×1 class5, Always ×1 class7, BeaconNearby ×5 class33 |

BeaconNearby (rule 11, `FUN_00416570`) is `164D0` with mask `0x100`: a beacon
within the strict axis. Class33 Follow Beacons runs the ground host's shared
`B740` acquiring pair and its follow. Only the Main Base abort's `10C10` or a
drowning reaches its class1 terminal.

A leg only starts a stride after `D360` resolves it, and `D360` is the model
draw's external-frame callback (`6ECF0 -> 0D350 -> 0A9F0`). A walker that is
never drawn keeps its legs at rest and plays no steps, in retail and in the
port. The common mover holds each step's sound with its drawn rate until the
ground host's visit leaves the Primary callback, then queues it before
anything later in the visit.

## Birth

The shared ground host's authored entry runs `104B0/09A80/D4A0`, as for Type30:

- `09A80` builds H, D, K, L, A and E. A's `20450` consumes one word.
- `D4A0` grounds the body at the authored X/Z (`0x20`). `+C0` has no `0x400`,
  so `D920` never runs the generic crush.
- `25680` draws the weighted root with one word. PlayerNearby counts a
  candidate within the 3840 axis: with one present, class7 weighs 5 against
  class5's 1; without, class5 is certain.
- `AC60` publishes the class. Each selected `06070` suffix consumes its own
  words.

**Model bank binding.** `09A80` binds K and L through `0A950` using the type's
own payload bytes (`409EF2` reads K's first selector from type `+0x30`; a zero
byte stores null). Model 1131 authors K `[4,3]` and L `[1,2]`, the reverse of
Type30's K `[3,4]` and L `[2,1]`; L's limits (8000 and 4000) are the same. K's
steering output therefore drives model variable 4, and L's lateral output
variable 1.
[`Intro2KlComponents`](../../crates/v2k-game/src/intro2_common_mover/kl_components.rs)
keeps each admitted model's selectors and admits only these two shapes.

## Living graph

- **Class5 Move About Aimlessly** keeps its 5000-ms retarget Primary.
  Completion and timeout reselect the root through `40C690`, as for Type30.
- **Class7 Search And Attack** installs the acquisition Secondary. `B6C0`
  copies only the row's axis `+04`. An accepted target runs `C7D0/C6B0` to
  Pursuit, which installs Chase and the method10 Aim.

Aim fires through the native ballistic FIFO
([`intro2_native_ballistic_aim`](../../crates/v2k-game/src/intro2_native_ballistic_aim.rs),
profiles `Type38`, `Type129`, `Type119` and `Type32`). Type129's shots are
silent.

## Hits and death

- **Hit dispatch.** `DAC0` calls style `+28`. A nonlethal hit runs `C690`,
  which reselects class5 or class7 and keeps every component.
- **Lethal hit.** Lethal damage publishes the row's alternate on the shared
  class49 terminal:
  - Type38 and Type119: `BAC0` clears every task and stages removal (corpse
    `4C7150`).
  - Type129: `BC90` keeps the living tasks and drops the Type61 authored in
    the carrier's `+88` at its position (corpse `4C7198`).
- **Death burst.** `BAF0`'s scatter takes the nonnull-A branch at `40BBB3`:
  ten class37 particles.

Playing's particle visit, radial callbacks, static walk and pair lanes reach
the terminal with Playing's player hull. The Main Base abort's `10C10` takes
the same class1 or class63 death.

## Contacts

- **Static walk.** Living styles `4C7930/4C7978`, Search and Pursuit call the
  retained Primary's `02CA0`; acquisition and Aim keep `05FF0`'s null hook. A
  finished corpse stays in the walk until `14990` through its Finished receipt:
  `4C7150`'s hooks are null, and `4C7198` still calls the retained Primary's
  `02CA0`.
- **Pair damage.** Captor and Type58 pair lanes deliver pair damage through the
  terminal.
- **Player pairs.** These remain the player adapter's open boundary, as for the
  other native ordinary actors.

## Held boundary

`E370` drowns a row after its surface time below the surface (500 ms for
Type38 and Type129, 2000 ms for Type119, 6000 ms for Type32). That death runs the
class1 or class63 terminal, which needs the static world and resources that the
surface step holds. The owner therefore fails closed at that step, as Type128
does. The fix is to split the surface step around its lifecycle callback.

## Validation

[`native_type38/tests.rs`](../../crates/v2k-game/src/native_type38/tests.rs)
covers:

- all 38 births, their rows, graphs and source profiles;
- ten-second cohorts in worlds 41, 42, 43, 46 and 32 under the real
  scheduler and the late static walk;
- Type32 absorbing a hit on every channel;
- Type119's step sound 90 at drawn rates, with the draw's `D360` writeback;
- method10 shots after a player comes near;
- lethal Playing hits: class1 for Type38 and Type119 (a channel-3 packet), class63
  with the Type61 drop for Type129, then the corpse's static walk;
- a nonlethal hit's `C690` reselection.

Elsewhere:

- [`kl_tests.rs`](../../crates/v2k-game/src/intro2_common_mover/kl_tests.rs)
  checks model 1131's bank binding;
- [the pair test](../../crates/v2k-game/src/native_actor_capture/pair_type38_tests.rs)
  runs a lethal pair delivery through each row's terminal, then pairs the
  finished corpse without replaying it;
- [the abort test](../../crates/v2k-game/src/main_base_abort_production/native/type38_tests.rs)
  checks the deaths in worlds 41, 42, 43, 46 and 32.

Headless 60-second runs of worlds 41, 42 and 43 completed with no blocked
owner. In world 42 a Type38 took thirty 500-point particle hits from other
shooters' fire, died by class1 and was swept.
