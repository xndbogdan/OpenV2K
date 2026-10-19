# Type39 Runtime

Type39 is a B/D/E/G/K flyer (model 287, health 4000, mass 100, capability 8)
that wanders and attacks. Its Section-12 record follows Type108's, with these
differences:

- the Sub-G flight table's speed cap at `+0x20` is 1500 (Type108's is 750),
  and its selector-one wing amplitude at `+0x2E` is `0x4000` (`0x6000`);
- Sub-K `[2, 3]`: `24450` binds its two words to model variables 2 and 3;
- the Sub-E emitter takes method 20 (class 52, as the deathwasp's) with a 2 s
  random interval, axis tolerance 2560, speed override 512 and sound 105, and
  the constructor attaches no sound 11;
- the common axis is radius 3840 with filter word 1;
- DCA0's healthy sound is 71 (`+0x94`), with period 500 000 µs (`+0xA4`);
- the second damage threshold is 8000 (Type108's is 4000);
- alternate class 1, `Explode`, where Type108 dies quietly through class 2.

World 31 authors two. Hive rows give birth to it in worlds 19, 25, 27 and 38,
and failed-world rows in worlds 16, 17, 19, 32 and 38. In the retail lab (an
idle player from each world's default arrival), world 19's Hive gives birth to
one at ticks 1502, 2505 and 3508, after which its live cap of three holds, and
world 38's at 1002, 1753 and 2506. The port's first births land a tick
earlier, on 1501 and 1001.

## Behavior

Its two choices take rule Always, in this order: class 5 `Move About
Aimlessly` (weight 1) and class 7 `Search And Attack Target` (5). They run as
Type108's do: `ACD0` installs the 5000-ms wander alone, `B6C0` the acquiring
pair, and `C690` reselects. Every task constructor reaches `406070`, whose
Sub-G branch draws one RNG word.

Sub-K (`FUN_004243B0`) runs after Sub-G in `018A0`'s detailed dispatch. Its
first word follows the prelude's smoothed steering word and its second the
vertical velocity Sub-G has just written; the coarse dispatch skips K and the
words hold. They drive only the model.

DCA0's post-task sound gate runs in each detailed visit of a visible Type39:
it draws one RNG word and plays sound 71 with chance `(elapsed × 64 / 488 + 1)
/ 65536`, about 4% a 20-ms tick. Coarse visits have no gate.

## Death

Alternate class 1 (`BAC0`, then `BAF0`), as Type14's: a lethal hit or the
session abort explodes the Type39. `BAF0`'s non-null Sub-B takes the `40BBB3`
scatter (ten class-37 particles) before the radial blast. The corpse keeps
only its Finished receipt until the deferred sweep.

## Port

The native flyer owner (`intro2_flyers_live`) carries Type39: its Sub-G
table, Sub-K state and publication, emitter profile and choices. The flyer
world runs DCA0's sound gate for every flyer type, and Type39 shares Type14's
class-1 routes: a Playing hit, the abort's class-1 dispatch and the corpse's
custody.
