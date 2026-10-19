# Type14 Runtime

Type14 is a B/D/G flyer (model 277, health 4000, mass 100, capability 8) that
never shoots: it authors no Sub-E emitter and no attached sound 11. Its
Section-12 record otherwise follows the wasp (Type15), with four differences:

- Sub-D steering divisor 128 (the wasp's is 64; the probes match);
- the Sub-G flight table's speed cap at `+0x20` is 400 (the wasp's is 500),
  and its wing-beat sound at `+0x0E` is 71 (the wasp's is silent);
- the common axis is radius 6400 with filter word `0x801`;
- alternate class 1, `Explode`, where the wasps die quietly through class 2.

Hive rows in worlds 18, 21, 24, 26, 28 and 37 give birth to it (world 18's at
20 s, then every 30 s; the port's first lands on retail's tick 1001), and
worlds 20, 26 and 37 each author two. In the retail lab, world 20's authored
pair flies continuously between heights of about 330 and 1870.

## Behavior

Its three choices all take rule Always: class 4 `Defecate Virus` (weight 1),
class 5 `Move About Aimlessly` (4) and class 13 `Flocking` (2).

- **Defecate Virus** (`B9E0`) clears slot 1, installs `402820`'s virus task in
  slot 2 with the type's `+0xA2` lifetime, then `4032A0`'s 2000-ms wander
  (`402BA0`) in slot 0. Type14's lifetime is zero, which disables the
  timeout, so it drops virus until the wander ends and `C690` reselects.
  `4032A0`'s target-speed reset needs a Sub-A descriptor (type `+0xD0`), which
  Type14 lacks. In detailed updates `402850` drops a class-5 carrier below the
  body, with chance `elapsed / 4` over a 16-bit word; in coarse updates it
  infects a terrain cell near the body directly.
- **Move About Aimlessly** (`ACD0`) clears slots 2 and 1 and installs the
  5000-ms wander alone, as for Type108.
- **Flocking** (`B640`) copies the authored axis word `+4` into the actor,
  clears slot 2, then installs `401F80`'s endless search in slot 1 and
  `402B10`'s 5000-ms wander in slot 0. The search's constructor override is
  -1, which normalizes to filter 0, "same entity type", on its first accepted
  one-in-four gate, so Type14s flock with other Type14s. A found one switches
  the style from `0x004C7DF8` to `0x004C7E40` (`C7D0`); `AF50` clears slots
  1 and 2 and installs `403650`'s route tracking it.

Every task constructor reaches `406070`, whose Sub-G branch draws one RNG word:
two for `B9E0` and `B640`, one for `ACD0` and `AF50`. Type14 has neither Sub-A
nor Sub-F, so `403650`'s speed and follow-mode suffix does not apply.

All four styles (`0x004C7E88`, `0x004C7930`, `0x004C7DF8`, `0x004C7E40`) send
infected, cured and primary hits to `C690`, mask no effective flags (`+0x38`),
have null contact hooks (`+0x30..+0x3C`) and a null death cleanup (`+0x2C`).

## Death

Alternate class 1 (`BAC0`, then `BAF0`): a lethal hit or the session abort
explodes the Type14. `BAF0`'s non-null Sub-B takes the `40BBB3` scatter (ten
class-37 particles) before the radial blast, and class 1 leaves no Type60
ring. The corpse keeps only its Finished receipt until the deferred sweep.

## Port

The native flyer owner (`intro2_flyers_live`) carries Type14: per-type
topology, Sub-D and Sub-G, the three graphs, and the mover's wing-beat sound,
which the owner now queues for every flyer. A Playing hit lends the class-1
terminal its static world and player, and the abort admits Type14 to its
class-1 dispatch. The world-less shared hit entry refuses a Type14 death
rather than borrowing the quiet one.
