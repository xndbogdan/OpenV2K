# Type73 and Type109 splitting bugs

This document owns the Type73 bugs of worlds 18 (four) and 26 (two) and the
three Type109 bugs of world 26. Both rows join the Type31 owner,
[`native_type31`](../../crates/v2k-game/src/native_type31.rs), on the shared
ground host, beside the [Type31 slugs](TYPE31_VIRUS_SLUG.md). Their lethal
split is [`native_type31/split.rs`](../../crates/v2k-game/src/native_type31/split.rs).

## Section-12 rows

| Field | Type73 | Type109 |
|---|---|---|
| Model | 303 | 304 |
| Components | A, B, C, D | A, B, C, D, E |
| A acceleration / target speed | 1000 / 400 | 1000 / 800 |
| D classifier | `0x11` (steepness, object) | `0x15` (adds material) |
| E | none | method16, 1.5 s, spread 100, threshold 20000, tolerance 3072, sound 64 |
| `+C0` | `0x39` | `0x439` |
| Healthy sound | 92 | 93 |
| Choices | Always ×20 class6, UnderAttack ×100 class10, Always ×20 class4 | Always ×10 class6, UnderAttack ×100 class10, Always ×20 class4 |

Shared: health 1000, mass 100, capability 8, thresholds
`[0, 3000, 1000, 1000, 200, 0, 0]` with multipliers `[0, 256, 256, 256, 256,
0, 0]`, the healthy sound every half second, accepted-hit cue 93, Run Away
cue 86 every second, Defecate Virus's terrain task lifetime 64 ms, B rates
1000/1000, C clearance 50 with surface mode 1 (the bugs also stand on the
waved sea), D divisor 32 with probes 512/256, axis 3584/`0x3`, alternate
class2. Neither row selects class7, so Type109's E never fires.

## Birth and living graph

The authored entry is Type31's `104B0/09A80/D4A0`. `104B0`'s type switch
installs `FUN_00416820` as the instance damage modifier (`+44`) for types 73
and 109 only. `D4A0` copies the placed position to `+90`, the anchor class6's
`02EB0` wanders around. A fresh `+34` leaves UnderAttack false, so a birth
publishes Wander Near Location or Defecate Virus. A surviving hit stamps `+34`
and reselects, where UnderAttack weighs 100; Run Away's flee writes no H (the
bugs own none).

## The lethal split, `FUN_00416820`

`415040` and `414E10` call the modifier after the channel filter and pass its
result to `14E90`:

- `+68` (the scheduler's elapsed time) still zero: the body was born this
  frame and has not been scheduled. It returns 0, so the hit only stamps `+34`.
- Dying, or the filtered amount below buffer plus health: the amount passes
  unchanged.
- Otherwise the hit is lethal. When channel 3 of the raw packet carries more
  than four times the other channel, the particle is 90 and nothing is born.
  Otherwise the particle is 51 and a local body counts the Type73 and Type109
  bodies in the actor list. Two newborns of its own type follow; when the count
  plus two passes 25 only `25 − count` are born, and a source with capability
  bit 1 (the player) queues hint 24 (string `0xF8`). Each birth draws nine
  words through `438080`: X/Y/Z offsets of `((r >> 7) & 0x1FF) − 256`, X and Z
  velocities of `((r >> 6) & 0x3FF) − 512` and Y `(r >> 7) & 0x1FF`, the
  parent's heading plus `((r >> 4) & 0xFFF) − 2048`, then pitch and roll of
  `((r >> 4) & 0xFFF) − 2048`. The record keeps the parent's `0x01000000`
  objective bit and the null handle. A failed birth is disposed and the loop
  goes on.
- It then emits the particle at the body with its X/Z velocity (Y zero), zeroes
  the buffer and health and calls `10C10`, which runs class2's quiet death. It
  returns 0, so `14E90` meets the dying body and stops: a lethal split never
  queues the player-kill hint.

Particle hits, radials, static contact and actor pairs reach the split. The
Main Base abort calls `10C10` directly and births nothing. Held, with the
modifier still failing closed: a peer's body in a network game (its kind-9
damage send runs after the modifier), listed in
[MULTIPLAYER.md](MULTIPLAYER.md).

## Validation

[`native_type31/type73_tests.rs`](../../crates/v2k-game/src/native_type31/type73_tests.rs)
authenticates both rows, checks the nine births, runs both worlds' cohorts
for ten seconds under the scheduler and the late static walk, splits an aged
Type73 and Type109 into two owned newborns each (whose own lethal hit is then
absorbed), kills one with a channel-3 hit without births, and sends a hit bug
into Run Away. [The abort test](../../crates/v2k-game/src/main_base_abort_production/native/ground_split_tests.rs)
checks the quiet deaths in worlds 18 and 26.
