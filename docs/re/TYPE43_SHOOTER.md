# Type43 emitter-only shooters and Type35 meteor launchers

This document owns the twelve ordinary Type43 rows (worlds 42, 46 and 47), the
twelve Type35 meteor launchers (worlds 41, 42, 43, 46 and 47) and their shared
native owner, [`native_type43`](../../crates/v2k-game/src/native_type43.rs).
The meteors Type35 births are in [INTRO2_METEORS.md](INTRO2_METEORS.md#dynamic-meteors).
Shared task programs remain in [ACTOR_TASK_PROGRAMS.md](ACTOR_TASK_PROGRAMS.md).
Static contact is in [INSECT_STATIC_CONTACT.md](INSECT_STATIC_CONTACT.md). The
class1 terminal is in [ENTITY_DAMAGE_AND_DEATH.md](ENTITY_DAMAGE_AND_DEATH.md).

## Section-12 row

| Field | Value |
|---|---|
| Model slots | 1185 ×4, no model variables |
| Mass / health / capability | 1000 / 5000 / 8 |
| Damage thresholds | `[0, 2000, 200, 0, 200, 0, 0]` |
| Damage multipliers (Q8) | `[0, 256, 256, 512, 128, 0, 0]` |
| `+C0` default flags | `0x2003B` |
| Axis | 2560, candidate filter 1 |
| Choices / rule / alternate | Always ×1 class7, rule 3, alternate class1 |
| Components | Sub-E alone |
| Emitter | method 10, 300000 µs, spread 1024, aim threshold 65535, speed 1500, tolerance 7680, sound 68, `+12` 0 |

Every sound field and the surface selectors are zero. The type uses common
vtable `4C8A30`: detailed `DCA0`, coarse `E870`, primary hit `DAC0`.

## Birth

`104B0` allocates the body. `09A80` zeroes the model bank and allocates only
`24E30`'s Sub-E, which copies method 10 and sound 68 and draws no random word.
`D4A0` applies `+C0`:

- bit `0x20` grounds the body at the authored X/Z (bit `0x40` is absent);
- `D3C0` clears state `0x10000` (no terrain/water lane) and sets `0x08000000`.

`25680` then consumes one word for the singleton list. `AC60` publishes class7,
and `B6C0`:

- copies the type's axis `+04`;
- clears Tertiary;
- installs the slot-1 acquisition (`02050`) and the 500-ms slot-0 retarget
  (`02B10/02BA0`).

Neither `06070` suffix finds an H, G, F or A descriptor, so construction draws
no further word.

Class7's Search style `4C7A50` has `+34 = 0` and `+38 = 0x21080`. `EA10`
applies `+38` reversed, so it **clears `0x08000000` again** and sets motion
`0x40000` and pair collision `0x8000`. The living shooter is therefore not a
fixed body: `12DA0` integrates its velocity (impact reactions and wind), and
`11AD0` scans it for static contact and active pairs. Pursuit `4C7A98`
(`+38 = 0x80`) leaves those bits as Search set them.

## Callback

`12DA0` owns the scheduler prefix and the post-callback master motion. `DCA0`
and `E870` read `(C8 | style+34) & !style+38` once:

| Style | Effective flags |
|---|---|
| Search `4C7A50` | `0x3B` |
| Pursuit `4C7A98` | `0x2003B` |

Neither result has `0x1000/0x2000`, so `E870` never skips the walk. Neither has
`0x0800` (terrain material), `0x4000` (keep basis) or `0x40` (model origin).
`DCA0`'s own gates (`E640`, `413F70`, `DF70`) keep this entry word. `E100` and
`DF70` re-read the current style after the walk; an acquisition that switches
Search to Pursuit changes only `0x20000`, which neither reads.

Task walk (`0A800`):

- **Primary** is `02BA0` (Search) or Chase `03490` (Pursuit). Both call `01430`.
  With Sub-E alone, `01430` runs only the target prelude: no A, D, I, F, G, L or
  O descriptor exists, so it cannot reverse, retarget or write a component. A
  dying or inactive tracked target returns zero; otherwise the target may be
  extrapolated and the mover returns one. `02BA0`'s own retarget still consumes
  its words every visit. Chase's post-mover controller needs Sub-H (`40354E`),
  which Type43 lacks.
- **Secondary** is the class7 acquisition. An accepted target runs
  `C7D0/C6B0` to Pursuit, then `ADE0` installs Aim in Tertiary and Chase in
  Primary.
- **Tertiary** Aim (`02300`) fires method 10 through the native ballistic FIFO
  ([`intro2_native_ballistic_aim`](../../crates/v2k-game/src/intro2_native_ballistic_aim.rs),
  profile `Type43`). The aim threshold is above `0x7FFF`, so the error test is
  bypassed. Each shot is a class38 particle (class46 at or below the sea plane).

After the walk, in order:

1. Detailed sound (`DCA0` only).
2. `E640` terrain attitude. With no Sub-C, `E640` reads mode 0: terrain probes.
3. `413F70` body basis.
4. `E100` gravity. Its water response reads Sub-C `+0C`, which is absent.
   Drag `44EC60` follows (bit `0x08`).
5. `DF70` ground snap at the current X/Z.
6. `E370`, whose zero selectors only decay the surface timer.
7. `12DA0` master motion.

## Hits and death

- **Hit dispatch.** `DAC0` calls style `+28` directly. Both Search styles hold
  `C690`, so a hit reselects through `AC60/B6C0` with no `0x1000` suppression
  test; only the task wrapper's owner transition has one.
- **Damage.** `11030` applies the impact reaction and `15040` the checked damage.
- **Lethal hit.** Lethal damage publishes alternate class1. `BAC0` runs on the
  shared class49 terminal.
- **Death burst.** `BAF0`'s scatter takes the default branch at `40BB9C`: no
  A/B/N/G component, no capability `0x40`, and not type 49 or 112–115. That is
  ten class16 particles.
- **Corpse.** The terminal clears every task and stages removal. Until `14990`
  the corpse stays a radial, static and pair participant, through its Finished
  receipt.

Playing's particle visit, radial callbacks, static walk and pair lane all reach
this terminal with Playing's player hull. The Main Base abort's `10C10` takes
the same class1 death.

## Contacts

- **Static walk.** `resolve_insect_static_contact` admits Type43 with default
  flags `0x2003B`. `A8B0` calls the retarget or Chase Primary's `02CA0`. With no
  A or G it only reverses direction, installs the 2500-ms timer and retargets.
  The acquisition and Aim tasks keep `05FF0`'s null hook. `D920` finds no crush
  bit (`0x400`). `11760` then pushes the body out of the model.
- **Descriptor contact.** `02DA0` runs `01A20` with no A or D. It still
  installs the 1500-ms timer, negates direction and retargets with two words.
- **Pair damage.** Captor and Type58 pair lanes deliver pair damage through
  the class1 terminal.
- **Player pairs.** These remain the player adapter's open boundary. An
  ordinary birth has no closed pair identity, as for the other native ordinary
  actors.

The Playing walk now lends its player to the static walk's terminal death
(`resolve_insect_static_contact_with_playing`). That also covers the Type128
carriers' BC90.

## Type35

Type35 (12 rows, worlds 41/42/43/46/47) has the same Sub-E-only class7 shape
and runs on the same owner as row `Type35`, with these differences:

- **No model.** Its Section-12 model words are zero, so it has no collision
  radius: no particle hit, static contact or pair reaches it.
- **Mass 1.** `14AE0` gives no radial impulse below mass 2.
- **Never damaged.** Capability `0x2`, health 100000, thresholds
  `[0, 10000, 200, 0, 200, 0, 0]` and every multiplier zero, so a radial
  packet filters to nothing.
- **`+C0` `0x201A7`.** Bit `0x04` disables `E100`'s gravity; there is no `E640`
  (`0x10`) or drag (`0x08`). Search's effective word is `0x127` and Pursuit's
  `0x20127`. Bits `0x80` and `0x100` are read by none of `DCA0`, `E100` or
  `DF70`.
- **Axis 7680/1, rule 1, alternate class7.**

Its emitter fires method 11 (600000 µs, spread 65535, aim threshold 65535, no
speed override, tolerance 7680, no sound). The table row is `{0, 2400, 0, 34}`:
no particle class, actor type 34. `44EA60` gives method 11 the default gravity
lead, so Aim runs the shared transaction at table speed 2400 (profile `Type35`).
At drain, `11400` applies its time correction and `44E770` adds the source
velocity (leading word 0). The actor branch (`44E92B`) then builds a zeroed
`0x4C` record:

- type 34, the launcher's X/Z;
- Y + 300 (`44E976`);
- the vertical velocity floored at `0x960`;
- rotation words from the launcher's `+A2`/`+A6`.

`438080 -> D720` builds the meteor, and its `+60` is the launcher. The scheduler
registers each meteor's owner before its next pass.

**Retail lab.** In world 43, with the player written 2500 units from the
`(32256, -32256)` launcher, both corner launchers fired across the map wrap:

- the first meteors came 16 ticks after the move, then one every 10-30 ticks;
- each was born at its launcher's position plus 300 in Y, climbing;
- each died within 1-45 ticks of starting to move, through class1's BAF0 blast
  (`[0x200, 0x400]`, impulse 2000, `[1, 3]`/`[4000, 4000]`);
- the launchers never moved or changed state;
- firing stopped once the player left the 7680 axis.

**Death.** Only `10C10`, from the Main Base abort, kills a launcher. It sets
health 0 and the dying bit; type `+90` and the attached sound are null. `DB80`
then runs:

1. Search's `+2C`, which is null.
2. `AC40 -> AC60`. `16460` sees the dying bit, so `25660` republishes the
   class7 alternate without a word.

The launcher stays linked and keeps searching and firing. Every later `C690`
on the dying body takes the same `25660` path. World 41's abort now runs every
death.

## Validation

[`native_type43/tests.rs`](../../crates/v2k-game/src/native_type43/tests.rs)
covers:

- the twelve births and their graph;
- ten-second cohorts in all three worlds under the real scheduler and the late
  static walk;
- acquisition of a nearby player and method10 shots leaving the FIFO;
- lethal (class1) and nonlethal (`C690`) Playing hits;
- a static model contact and the `02DA0` descriptor contact.

[The abort test](../../crates/v2k-game/src/main_base_abort_production/native/type43_tests.rs)
checks class1 in worlds 42/46/47.
[`type35_tests.rs`](../../crates/v2k-game/src/native_type43/type35_tests.rs)
covers the twelve launcher births, the world-43 lab setup (both corner
launchers, births at launcher + 300 climbing at least `0x960`, landings that
explode), the rolling profile and `10C10`'s republished Search on a dying
launcher that keeps firing.
[Its abort test](../../crates/v2k-game/src/main_base_abort_production/native/type35_tests.rs)
checks the launchers in worlds 41-47 and a meteor in flight.
