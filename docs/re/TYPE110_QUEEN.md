# Type110, the Queen

World 47 (Alien 3) authors one Type110 at `(17152, 0, -24832)`, heading
43508. Her row: model 1113 in all four slots, mass 1, capability 8, health
200, `+C0` `0x24027`, an Always choice of class67 with class67 as the
alternate, Sub-K alone with eight model words (payload `[8, 0]`), no sounds,
default axis. Her damage profile passes only channel 2 (threshold 5900,
multiplier 256).

The port runs her on the class0 host's owner ([CLASS0_RUNTIME.md](CLASS0_RUNTIME.md)):
the `104B0/D4A0` birth grounds her at `(17152, 2880, -24832)`, the same
`12DA0` prefix visits her, and her callback is class67's.

## Class67 and her record

`FUN_0040D210`, class67's initializer, clears S then T and stores a Primary
through `FUN_00401020` with no timeout or private data, tick `FUN_004222C0`.
Her record is not the task's: `FUN_00421B40` allocates it on the first visit
(which does nothing else) and hangs it off Sub-K `+3C`. It holds:

| Offset | Contents |
|---|---|
| `+00` | microseconds since death |
| `+04/+08` | Sub-K word 8 and its direction |
| `+0C/+10` | Sub-K word 7 and its direction |
| `+14` | the first gun: Sub-K words 6 and 5, limits `004C9408` (rates `0x8000`/`0x4000`, yaw `0x3000`, pitch `0x1000`..`-0x2000`), muzzle `+18`, draw count `+20`, range 5000, timer, period 300 ms, particle class 85, Q31 frame `+34` |
| `+6C` | the second gun: Sub-K words 2 and 1, limits `004C9438` (yaw `0x7400`, pitch `0`..`-0x4000`), range 3000, period 600 ms, class 68 |
| `+C4` | the neck: Sub-K words 4 and 3, limits `004C9420` (rate `0x6000`, pitch `0x800`..`-0x2000`) |
| `+DC` | the weak spot |
| `+E0..+F0` | the target's position, distance, lateral offset and handle |
| `+F4` | the Type68 she holds |

Because the record lives in Sub-K, `FUN_00410C10` (health 0, dying, no
death sound) taking class67 again through `FUN_0040DB80` keeps it: the dying
Queen carries on with the same record. The Main Base abort and a lethal hit
both do this.

## The living visit (`FUN_004222C0`)

Every visit zeroes her velocity. While she is alive (state nonzero, `0x4000`
clear):

1. `FUN_004226A0` scans the live list for a body with capability bit 0 or
   2 and pair state `0x8000`. A body within 20,000 (`FUN_00425370`'s
   largest-plus-half distance) scores 20,000 less its distance, the player
   30,000; the best becomes the target, with its position, distance and its
   offset along her lateral axis. The scan clears the handle and the lateral
   offset first but keeps the old position and distance when it finds none.
   Its 1.5-times rule for the current target compares against the best so
   far, so it never applies.
2. Both gun frames are rebuilt from the body basis: the second gun's turned
   `0x8000`, then four times by a quarter of its yaw and pitch, then pitch
   `0x1000`; the first gun's by the neck's yaw and pitch, then its own.
3. `FUN_00421E00` aims each gun. With a target and a drawn gun (count
   nonzero) it leads the target from the muzzle by the target's velocity over
   `distance * 1024 / range` (capped at `0x1000`, as Q31 `<< 17`), adds
   gravity `time^2 * 0x180000` (Q31) to Y, and takes the yaw and pitch errors
   as `FUN_00457F70` arcsines of `FUN_00457680` ratios of the lead on the
   frame's lateral and up rows (a target behind takes `+0x2000`, or
   `-0x2000`). Otherwise the demands are the joint's own angles. A side
   word overrides the yaw demand with `0x4000` toward its sign: the first gun
   gets the lateral offset when the neck and gun already point that way, the
   second its negation when it points away. `FUN_00421C60` steers the joint
   (frame microseconds `>> 5` times each demand, doubled against the current
   rate, then rates and angles clamped); the first gun's call steers the neck
   with half the demands too.
4. The gun's timer gains the frame's microseconds (`DAT_004D04E4`). Past
   its period, with the target within 5,000, it fires one particle of its
   class from the muzzle, owned by her: along the lead at the gun's range
   when both errors are under `0x400` and the lead is over 64 long, otherwise
   along the frame's forward row at the range, but only on a 1 in
   `period * 8 / frame` draw. A shot sets the timer to a random fraction of
   the period. With no target or an undrawn gun, retail compares her handle
   with 5,000, so that gun never fires. `FUN_00421D40`, an aiming line of
   class-51 particles, waits on `DAT_004DB0B8`, which nothing writes.
5. Sub-K word 8 waits at 0 until the target comes within 2,500, then falls
   by `dt >> 5` to `-0x3000` and rises by `dt >> 4` back past 0; word 7
   swings between 0 and 3,000 (`dt >> 7` down, `dt >> 6` up).
6. Both draw counts drop by one.
7. `FUN_004225C0` keeps the body she holds: when it is gone, a zero-record
   `FUN_00438080` Type68 (the same birth as a campaign-cargo weight), whose
   state then loses `0x68800` (callback, motion, pair and radial bits), whose
   `+A8` becomes model 286 and whose capability loses `0x1000` and gains 8.
   Each visit sets it on the first gun's muzzle with state `0x40000000`
   while that gun is drawn.

Last, the joints go back to the Sub-K words: 6 and 5 the first gun's negated
yaw and pitch, 4 and 3 the neck's, 2 and 1 a quarter of the second gun's
negated yaw and pitch. `FUN_0040D320` reads them in selectors 1..8 and the
50-Hz clock in selector 0.

## Drawing aims her guns

The draw counts and muzzles come from her model. Models 1117 and 1119, deep
in model 1113's hierarchy, each have one type-14 external-frame record,
selectors 0 and 1. `FUN_0040A9F0` reaches `FUN_00421AE0` for them (she has
no Sub-H, Sub-E or `+104`), which reads slot 0x1C (selector 0, the second
gun) or 0x1B (selector 1, the first) of the node being drawn through the
emitter's own `FUN_00424FA0` and stores the point as the muzzle, with a draw
count of 2. So a gun aims and fires only while it is drawn, and its muzzle
is where it was last drawn: until then `(0, 0, 0)`, where her Type68 then
waits. The port stamps the same points from the same draw (the actor
emitter presentation, with slots 0x1C and 0x1B). The transform-only pass
that gives undrawn actors their muzzles (`FUN_00413BE0`) runs only for an
actor with queued shots in its `+38` list; her guns fire directly, so it
never runs for her and her undrawn guns stay idle.

## Hits and the weak spot

Her style `0x004C8308` has one hit hook, `+28` `FUN_00421990`, which
`FUN_0040DAC0` calls on a primary hit before `FUN_00411030`: it zeroes her
velocity, and with the first gun drawn, a hit point within 256 of its muzzle
whose direction from the hit to the muzzle lies within 60 degrees of the
gun's up row (Q12 dot over `0x800000`) opens the weak spot. `FUN_004104B0`
gives her the instance damage modifier `FUN_00421910`: with a record, only a
hit whose packet owner (`+14`) has capability bit 0 (the player) passes
through an open weak spot, closing it; every other hit filters to 0 and
`FUN_00414E90` takes nothing. Before her first visit everything passes.
An accepted hit emits class 5 under her (capability 8). A lethal one takes
class67 again.

## The dying visit (`FUN_00422840`)

The target distance and offset clear. Then:

- past 30 seconds (`+00 > 29,999,999`) it calls `FUN_00456F20`, the move to
  the closing scene, and returns (below);
- it walks the live list, reading each successor after the call, and every
  other living body (state nonzero, `0x4000` clear) with capability 8 goes
  through `FUN_00410C10`;
- before 10 s the joints drift back inside their limits (`FUN_00422B40`)
  and sound 75 plays on a 1 in 8 draw; from 10 s they sag (pitch `-0x1000`)
  with sound 99 on a 1 in 12; word 7 swings between 1,000 and 6,000;
- from 15 s sound 99 on a 1 in 12, word 7 runs down to 10, and her body
  basis's up-Y loses `up * (dt << 7)` (Q31) while above `0x8000000`;
- each visit, on a draw under the visit's microseconds, four class-5 sparks
  leave the first gun's muzzle, their velocity a random walk from its
  forward row.

Retail never asks whether the world was lost: a Queen the Main Base abort
killed sweeps and moves on the same way.

### The sweep

In world 47 the sweep's victims are the two Type2s, Type5s, Type13s and
Hives (Type67, capability `0x18`), the three Type33s, four Type43s and the
Type68 she holds; all die on her first dying visit, after which the sweep
finds nothing. Each victim's `10C10` runs through that body's own death
owner, the same per-type owners as the Main Base abort walk's ordinary
route, with the playing world's player and scheduler: the Type13s, Type43s
and Type2s explode through class1, the Type5s take class11 Tumble, the
Type33s class12, and the held Type68 (capability `0x48` after
`FUN_004225C0`) dies quietly through class2.

The abort walk never kills a Hive (its capability bit `0x10` sends it to the
alternate cleanup), so the sweep is the Hive's only `10C10`: health 0 and the
dying bit (its active model moves to its pair's dying slot), the type's death
sound and the `+8C` sound record released, then the live style's
`FUN_00425790`: task slots 2 and 1 cleared and `FUN_00425F60`'s dying slot 0
(tick `FUN_004260F0`), then the shared `FUN_00440950` burst at the Hive with
the dying model's extent and `FUN_004566E0`'s radial (`0x55F0` to each other
Hive in range). `10C10` does not change the Hive's style.

The port runs the visit with every scheduler owner reunited, as a weapon's
blast does, and repartitions them against the live list afterwards. A
victim whose owner refuses its death stops her visit with a typed, logged
stop naming that victim.

### The move to the closing scene (`FUN_00456F20`)

`FUN_00456F20` sets session `+290` (leave this world at the next gameplay
frame), calls `FUN_00456750` (`+295`, the full-frame flash), switches cheats
1 (Weapons Menu) and 0 (Skip to World) on through `FUN_00450FC0`, and sets
the session's logical world `+34` to `0x25` (overlay 49) and its phase
`+296` to 3. Outside a network game, which world 47 never is,
`FUN_00450FC0` sets the cheat's bit in session `+1C4`, plays sound `0x57`
through `FUN_0042A860`, and shows the cheat's Cheats row
(`4D0F20 + id * 0x1C`: flags lose 8, gain 2) and the pause menu's Cheats
row (`4D1154`). The port resolves both sounds in the visit's RNG order and
applies the rest after the pass: the flash, the cheat word and the rows the
pause menus show.

Overlay 49 in phase 3 is the closing scene: its Section-2 records caption
"WELL DONE!", "You've completed V2000 by killing the Alien Queen." and the
rest, move the camera over its actors, and at 290 s command `0x40`
(`FUN_00452770`), in phase 3, sets the next world to 1 and leaves; the player
then arrives in world 13 in ordinary play (phase 5) with both cheats on. The
port plays it: see [CLOSING_SCENE.md](CLOSING_SCENE.md). With no player in
the world when `FUN_00456F20` runs (the port cannot know where `FUN_00453C20`
would leave the camera proxy), the port logs `Closing scene held` and the
world runs on.

## Evidence

A retail lab run of world 47 from Skip to World, with her record dumped at
every visit: her first visit reads `0x09468805`, 200 health and the position
above, and allocates the record; from the next she targets the Type97 at
`(28928, -2656, -29952)`, 17,104 away and -12,730 to her side, with frames
the port reproduces word for word. Undrawn, her guns hold still and her
Type68 sits at `(0, 0, 0)`; her timers run on coarse visits by the frame
time only. With the player placed 3,500 from her, the guns are drawn from
the next tick, aim and fire (three second-gun shots in a run from the east);
consecutive records there match the port's visit exactly, frames, joints and
timers, given the player's logged velocity. Before the port had her, world
47's Main Base loss stopped at her; it now completes.

A tampered lab run of world 47 from Skip to World calls `FUN_00410C10` on
her at tick 842. Her first dying visit, at tick 845, kills every living
capability-8 body: class1 bursts for the Type13s, Type43s and Type2s, the
Hives' radials hitting each other, the Type33s, Type5s and her Type68 dying.
At tick 2345, 1,500 ticks later, `FUN_00450FC0` switches cheats 1 and 0 on,
`+34` reads `0x25` and `+296` 3, and the next pass loads overlay 49, whose
captions and credits run until it leaves for world 13 about 291 s later.
The port's world 47 reproduces the victims and calls `FUN_00456F20` on the
same tick after her first dying visit. The closing scene follows in
[CLOSING_SCENE.md](CLOSING_SCENE.md).
