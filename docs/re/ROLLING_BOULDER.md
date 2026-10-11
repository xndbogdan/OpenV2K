# Rolling Boulder class20 (Type3/Type27)

`v2k-game::rolling_boulder` owns the authored Type3 and Type27 boulders:
native construction, both class20 styles and their tasks, the late terrain,
water and static contacts, hits, player pairs, and both deaths, including
Type27's split into two Type3 boulders. Other pairs and the Main Base abort
are still explicit boundaries (see [Port status](#port-status)).

## Identity

| Type | Model | Mass | Health | Damage thresholds | Alternate class | Births |
|---|---:|---:|---:|---|---:|---|
| 3 | 648 | 100 | 5000 | `[0,8000,4000,10000,200,0,0]` | 1 | worlds 31/35: 3/2 |
| 27 | 647 | 200 | 10000 | `[0,10000,4000,10000,200,0,0]` | 18 (Split And Explode) | worlds 27/31/35: 6/7/12 |

Both rows have capability `0x2000`, no components, sounds, effects or model
variables, and initializer state `0x4060` (bit20 terrain snap, bit40 model
lift). Their only choice is Always/1/class20. Damage multipliers are
`[0,256,256,0,128,0,0]`. `104B0 -> 09A80 -> D4A0` places the boulder at the
bilinear terrain height plus the active model's header `+08`, and the single
weighted choice consumes one shared RNG word.

## Styles and tasks

The class table entry `0x004C8B44` holds descriptor `0x004C8908`, style table
`0x004C78A0`. Both `0x48`-byte styles were read at runtime from the
faststart04 recording:

| Style | `+04` (tag 9C00) | `+10`/`+14`/`+1C` | `+18` pair | `+28` impact | `+34`/`+38` | Initializer | Primary |
|---|---|---|---|---|---|---|---|
| 0 rolling `4C78A0` | `40C750` -> style1 | null | null | null | `0`/`1005` | `40B950` | `404580(slot0, 0)` |
| 1 resting `4C78E8` | `40C730` -> style0 | null | `40C730` | `40C730` | `1005`/`0` | `40B9B0` | `404B40(slot0, 0)` |

Both initializers first clear Tertiary and Secondary through `40A7A0`.
`40EA10` applies `+34` through `40D440` and `+38` reversed. Policy bit1 maps to
state `0x10000` and bit1000 to `0x40000`, so rolling sets terrain/water
admission and master motion, and resting clears both. The effective
environment word `(C8 | +34) & ~+38` is `0x4060` while rolling (E100 gravity)
and `0x5065` while resting (bit4 suppresses gravity). Neither sets bit8 (4EC60
drag), bit2 (DF70 ground snap) or bit400 (D9B0 crush).

Both Primaries are built through `05FF0`, which zeroes `+00`, `+04` and
`+14..+24` and stores the callback at `+10`. `404580` then writes wrapper
`401350` at `+00` and destructor `407120` at `+24`; `404B40` goes through
`05F80`, which adds neither. Neither writes the static hook `+20`. The callback
`404690` is the rolling body task the Intro2 meteors also use (they give it a
5000 ms lifetime). It counts visits with an unchanged position and returns
singleton `0x004BE190` (tag 9C00) after more than ten. The resting callback
`404B60` returns zero on a coarse visit (state `0x02000000` clear). Otherwise
it takes the integer length of the velocity, narrowed to 16 bits. Above 100 it
returns singleton `0x004BE198` (tag 9C00); otherwise it zeroes the velocity.
Both boulder lifetimes are zero, which never expires in `401120`.
`416410` (state `0x1000`) gates the 9C00 transition.

So a rolling boulder falls and rolls until it holds still for eleven visits,
then rests. A detailed resting visit, a pair contact (`+18`) or a hit (`+28`)
returns a moving boulder to rolling.

## Contacts

Late `11AD0` admits a body with state `0x8000`, clear `0x1000`, zero `+70`, a
nonzero model radius and clear `0x88000000`. Terrain and water also need
`0x10000`, which only the rolling style sets. Both styles' `+10`/`+14` hooks
are null, so `D7F0`/`D860` reach the bare `141D0` solid and water responses.
Both type records carry zero `+86`/`+88` cues.

The static scan does not need `0x10000`, so both styles reach
`12CF0 -> A8B0 -> D920 -> 11760`. The task `+20` hooks and style `+1C` are
null and neither effective policy has bit400. `11760` reflects and separates
only a body moving into the plane, then sends the impact to the static cell
before checked actor damage. Ghidra shows that it reads velocity, position,
the remote bit, mass and the type's `+8A` cue, and never the capability word.

In all 30 authored placements the first terrain contact lifts the boulder by
28 (Type27) or 18 (Type3) raw units: the collision geometry reaches below the
`D4A0` header-`+08` lift. Once its visits are detailed, every boulder rests
within 13–20 visits, with collision damage of at most 1. No recording yet
covers worlds 27/31/35.

## Hits

Both rows keep the shared type vtable `4C8A30`: `DAC0`/`DA00`/`DA60` at
`+14..+1C` and a null `+30` generic-hit slot (runtime read). Rolling style0
has null `+20/+24/+28`. Resting style1 has `+28` = `40C730`, so a primary hit
wakes a resting boulder, while infected and cured hits reach null slots.
`10EB0` stamps `+34` before the callback; the `11250`/`11320` model-switch
prefix stands in for it on infected and cured hits. `11030` then pushes and
jolts the body (the birth state sets enable bit `0x04000000`) with three RNG
words, and `415040` filters the packet through the type's thresholds.

## Deaths

`10C10` zeroes health and sets the dying bit. Style `+2C` is null in both
styles, so AC60 selects the alternate class directly, with no RNG word.

- **Type3, class1 Explode** (`4C7150`, initializer `BAC0`). BAF0 finds no
  A/B/N/G and no capability `0x40`, and the type is neither 49 nor 112..115.
  So the `40BB9C` default scatters 10 class-16 particles. Its radial uses the
  type's `+50..+67` template: inner 512, outer 1024, impulse 2000, and 4000 on
  channels 1 and 3. All three task slots then clear and removal is deferred.
  That radial needs the player hull, so a hit runs it in the Playing scene.
- **Type27, class18 Split And Explode** (`4C73D8`, initializer `40C080`). The
  `440950` burst tests A/B/N/G and picks class `0x10` with count 2
  (`40C0B3..40C0F5`), drawing one RNG word. After the task clear, the
  Type27->Type3/count2 row builds two children from the zeroed birth record.
  Each child takes nine launch words (position offsets, velocity, Euler), then
  AC60's choice word. D4A0 grounds it at its launch X/Z plus the model lift,
  then D720 runs. Each child joins the scheduler before the next launch.

Both terminal styles are all-zero apart from their `+40` initializer (runtime
reads), so the linked corpse keeps null-hook contacts and hits until the sweep.

## Port status

Implemented:
- construction, both tasks and the 9C00 style switches;
- terrain, water and static contacts;
- primary, infected and cured hits;
- player pairs: the pass admits boulders through their D720/13F70 body basis
  (the rolling task turns it, so no authored pitch/roll applies). The response
  pushes them, and a resting boulder's style1 `+18` (`40C730`) reinstalls
  rolling style0 inside the walk, before 411760's damage;
- both deaths from hits and from Playing or cinematic blasts;
- a Type27 split from a lethal collision;
- corpse contacts and hits.

Radial blasts and attached particles reach a boulder only through its
completed owner or its finished terminal.

Remaining boundaries, each held rather than approximated:

- A Type3 killed by a terrain or static collision holds at "Rolling Boulder
  class1 radial needs the player hull", because the late contact frame does
  not carry the Playing hull. That needs over 13000 raw collision damage,
  which has not been observed.
- Lethal player pairs. A lethal pair packet holds at `DeathDispatchRequired`.
  Type3's class1 radial needs mutable resources this pass does not carry, and a
  Type27 split appends children that retail's continuing pair walk would visit
  in the same frame.
- Other pairs. Boulder pairs with actors other than the player, and the Hive
  ram (`UnsupportedHiveImpactCounterpart`, [Hive wreck](HIVE_WRECK.md)).
- The Main Base abort transaction, which still stops at Type27 in worlds
  27 and 35.
