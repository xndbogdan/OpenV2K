# Type41 Runtime

Type41 is a D/G flyer (model 1291, health 2000, mass 50, capability 8): it
has neither Sub-B nor an emitter. World 22 authors one, world 33 two and
world 34 four. Its Section-12 record:

- C8 `0x8`: no `0x4` no-gravity bit, so `E100` applies gravity and, through
  bit 8, `44EC60`'s drag and wind;
- Sub-D divisor 100 with yaw coupled into roll and the wasp's 512/256 probes;
- Sub-G: no first force, second force 50, clearance 40 (`+0x0C`), wing-beat
  sound 26 (`+0x0E`), rates 30/130, speed cap 1000 (`+0x20`), selector one
  bound to model variable 1 and surface mode 1 (`+0x2B`);
- the common axis is radius 512 with filter word 0 (the same type);
- choices Defecate Virus x1 (class4), Retarget x3 (class5), Wander x1
  (class6), all Always, and UnderAttack x4 Run Away (class10); rule 3;
- alternate class12, `Flip Over And Die`; no death, hit or attachment sound.

## Behavior

Every task constructor reaches `406070`, whose Sub-G branch draws one RNG
word.

- Class4 is Type14's `B9E0`: the 2000-ms wander and, with Type41's lifetime
  of zero, the endless virus task in slot 2.
- Class5 is the shared `ACD0`: the 5000-ms retarget alone.
- Class6 `FUN_0040AD10` clears slots 2 and 1, then installs `FUN_00402E20`'s
  5000-ms Wander Near Location around the immutable `+90` anchor, the birth
  position. Its style `4C79C0` clears `0x21000`.
- Class10's `4C7618` runs `FUN_0040B6C0`: the 500-ms retarget and
  `FUN_00402050`'s search. With filter word 0 the search takes another
  Type41 within the axis; `FUN_0040C7D0` hands it to `4C7660`, whose
  `FUN_0040B3F0` installs `FUN_00403E20`'s 2000-ms flight.
- UnderAttack (rule 2) holds while a hit stamped `+34` lies within the last
  250 ticks, once the clock has passed 249. A primary hit stamps `+34` before
  the style's hook, so a hit Type41 can flee at once. A newborn never does.
- Hits: `4C7930`, `4C79C0` and `4C7E88` hold `C690` in all three hit slots;
  both Run Away styles hold it for infected and cured hits only.

## Death

`10C10`, then `DB80` (every living style's `+2C` is null) and alternate
class12. `FUN_0040C620` clears slots 1 and 2; `FUN_00404120` builds the
9000-ms task: `406070`'s Sub-G branch (one word), vertical velocity 500, then
`1B970(G, 1)`, which stops Sub-G's flight controller. `FUN_00404220`:

- on a coarse visit it takes `4BE170` (tag `9C01`): `C470` stages removal;
- on a detailed visit it flips the body (`1EC70`, mode 0 without Sub-C),
  runs `01430` with a null target, which skips the prelude and Sub-D and
  leaves Sub-G only its zero force words, then damps X and Z.

The class12 styles clear `0x2015` and `0x2011`, so the corpse keeps gravity
and drag. The Main Base abort takes the same death.

## Port

The native flyer owner (`intro2_flyers_live`) carries Type41: its row, the
Wander and Run Away graphs, UnderAttack in `C690`, and class12, whose graph
the same owner keeps ticking until the terminal. The flyer world now applies
`44EC60`'s current wind in wind modes 1 and 2, as the Type10 owner does.

Retail lab, Skip to World with an idle player, steps on moving ticks (raw
units): world 34's four Type41s moved on 1,002-1,092 of 2,927 ticks (median
53.0-61.4), the port's on 1,012-1,201 of 2,928 (46.8-60.9); world 33's two
(wind mode 2) on 1,004 and 1,019 (65.8, 62.7), the port's on 1,027 and 1,035
(63.5, 61.6); world 22's on 604 of 1,761 (60.0), the port's on 635 of 1,760
(68.5). In both games one world-34 Type41 dipped far below the others
(retail to Y -2,115, the port to -3,278). None was hit.
