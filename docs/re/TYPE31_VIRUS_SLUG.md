# Type31 virus slugs and their Type105 split children

This document owns the three authored Type31 rows (two in world 16, one in
world 31), the Type105 rows their class18 split births, and their shared
native owner, [`native_type31`](../../crates/v2k-game/src/native_type31.rs).
Both rows run on the shared ground host. Static contact is in
[INSECT_STATIC_CONTACT.md](INSECT_STATIC_CONTACT.md); the class18 kernel is in
[ALPINE_INSECTS.md](ALPINE_INSECTS.md), whose Type40 split it shares.

## Section-12 rows

| Field | Type31 | Type105 |
|---|---|---|
| Model / health | 279 / 8000 | 280 / 3000 |
| Damage thresholds | `[0, 4000, 200, 0, 200, 0, 0]` | `[0, 3000, 1500, 500, 200, 0, 0]` |
| Components | A, B, C, D, E, H (fourteen legs) | A, B, C, D, E |
| A acceleration / target speed | 2500 / 200 | 2600 / 300 |
| D classifier | `0x06` (water, material) | `0x04` (material) |
| E interval | 1,000,000 µs | 600,000 µs |
| Axis | 2560 / `0x5` | 2048 / `0x5` |
| `+C0` | `0x439` | `0x39` |
| Choices | UnderAttack ×8 class10, Always ×1 class4, Always ×2 class5 | UnderAttack ×8 class10, Always ×1 class4 |
| Alternate | class18 | class2 |

Shared: mass 100, capability 8, damage multipliers `[0, 256, 256, 512, 128, 0,
0]`, B `1000000/1000000`, C clearance 50 with surface mode 0, D divisor 128 with
probes 512/256, the method30 emitter (spread 256, threshold 4000, speed 1500,
tolerance 2560, sound 105), healthy sound 54 every four seconds, and no other
sounds. Type31's fourteen H records take their first two vertices from
`52..65` and `80..93`; the third runs `66..75`, repeats 74 and 75, then ends
with 78 and 79. Record 8's resolver flags are `0x04000000`, the others
`0x40000000`.

## Birth

The authored entry runs `104B0/09A80/D4A0`: H, D and E are allocated, A's
`20450` draws a word and `25680` draws the selector. A fresh `+34` of zero
leaves UnderAttack false (`16490` needs the tick both below 250 and, signed,
above 249), so a birth publishes Defecate Virus or Move About Aimlessly.
Neither row selects class7, so the 09A80 emitter never fires.

## Living graph

Defecate Virus4, Move About Aimlessly5 and Run Away10 run the ground host's
shared programs. The shared `06070` constructor suffix writes H only for a type
that owns one, so Type105's suffix is A alone.

## Death

- **Type31**: `10C10` runs `DB80` and AC60 selects class18 directly (no
  selector word). `FUN_0040C080` emits the class37 burst (the type owns A and
  B), clears the tasks and, below the 80-allocation limit, births eight Type105s
  through `438080`, each from nine shared words. The children are appended
  behind the cursor and owned at once; the parent is marked for `14990`.
- **Type105**: class2's `C470` clears the tasks and marks the body.

The Main Base abort's `10C10` takes the same deaths, and its visit limit counts
each Type31's eight children. Hits, radials, static contact and pair damage
reach the same publishers.

## Validation

[`native_type31/tests.rs`](../../crates/v2k-game/src/native_type31/tests.rs)
authenticates both rows, checks the three births, runs both worlds' cohorts for
ten seconds under the real scheduler and late static walk, and splits a Type31
into eight owned Type105s, one of which then dies quietly.
[The abort test](../../crates/v2k-game/src/main_base_abort_production/native/ground_split_tests.rs)
checks both deaths in worlds 16 and 31.
