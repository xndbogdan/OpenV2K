# Type43 emitter-only shooters

This document owns the twelve ordinary Type43 rows (worlds 42, 46 and 47) and
their native owner, [`native_type43`](../../crates/v2k-game/src/native_type43.rs).
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

Type35 (12 rows, worlds 41/42/43/46/47) has the same Sub-E-only class7 shape,
with these differences:

- model 0 and mass 1;
- capability `0x2`;
- `+C0` `0x201A7`;
- axis 7680/1;
- rule 1;
- alternate class7;
- zero damage multipliers, so it is never damaged.

Its emitter fires method 11. That projectile row has no particle class and
trails entity type 34, so `44E92B` builds a `0x4C` actor request instead of a
particle (`44E976` special-cases type 34): each shot is a Type34 meteor birth.
Type35 waits for that dynamic Type34 birth and its ordinary owner; until then
the Main Base abort stops at Type35 in world 46. World 47 now stops at Type110
(class67).

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
