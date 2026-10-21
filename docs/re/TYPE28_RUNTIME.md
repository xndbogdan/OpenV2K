# Type28 ground insects

This document owns the eleven ordinary Type28 rows and their native owner,
[`native_type28`](../../crates/v2k-game/src/native_type28.rs). Type28 shares
[Type18](TYPE18_RUNTIME.md)'s machinery on the ground host: rule 8, the Type26/58
Furniture policy, Type122's capture and class12 death. It has no emitter, and
its root replaces Defecate Virus and Search with Run Away.

## Cohort

| World | Rows |
|---|---|
| 21, 22, 23 | 1, 1, 2 |
| 30, 36 | 5, 2 |

Every spawn is plain: no model override, animation or configuration, and
parameter 1. In world 36 the two Type28s stand beside two Type18s.

## Section-12 row

| Field | Value |
|---|---|
| Model slots | 31 ×4, no model variables (header extent 0, collision radius 277) |
| Mass / health / capability | 100 / 5000 / 8 |
| Damage thresholds | `[0, 2000, 2000, 500, 200, 0, 0]` |
| Damage multipliers (Q8) | `[0, 256, 256, 128, 128, 0, 0]` |
| `+C0` default flags | `0x439` (Type122's policy, with drag) |
| Axis | 3840, candidate mask 5 |
| Components | A, B, C, D, H (eight records), J (one slot, `[0,30,100]`); no E |
| Sounds | death 62, accepted hit 6 |
| Surface | selectors `[0, 0]` |
| Alternate | class12 |

Sub-D has divisor 128, probes 512/256 and classifier `0x10` (steepness only).
Each H record depends on a partner across the body rather than within its
group: records 0..7 depend on 2, 3/7, 4, 5/1, 6, 7/3, 0 and 1/5.

The root, in authored order:

| Rule | Weight | Class |
|---|---|---|
| PeopleNearby | 9 | 9 Capture People |
| UnderAttack | 20 | 10 Run Away |
| FurnitureNearby | 5 | 26 Trash Furniture |
| Always | 3 | 33 Follow Beacons |

UnderAttack reads the last-hit tick at `+34`. A fresh birth's `+34` is zero, so
the rule is false at birth (tick < 250 and signed tick > 249 cannot both hold).
After a hit it weighs Run Away for 250 ticks.

## Birth and graph

Construction is Type18's without E, and it consumes the same words: A's
`20450`, the one `25680` selector word, then the class's `06070` suffix. Run
Away shares `B6C0` with Search and Capture, and copies axis `+04` as they do.
Its acquiring and fleeing styles `4C7618/4C7660` are admitted for Type28 in
the static walk. No class installs Aim, so the profile's Aim hook fails
closed.

## A carried person drowning

A person (Type86/95 and kin, or Type123) carried under water reaches its own
`E370` lifecycle. `162B0` calls `16750` with a null argument: it clears
`0x20001000` on the child, sets `0x800`, reapplies the type flags and nulls
the child's parent. `DC50` then runs the carried style's `CE90` with the
child's own context, so the release reselects exactly as a captor's release
does (enable pairs, Sub-I release, full reselection with its RNG and attract
receipts). `10C10` follows and class14 replaces that graph. Ordinary Type9's
carried expiry takes the same order.

`16750` never edits the captor's J row. Until the captor's next `18640`, the
row names a corpse with a null backlink and no relation bit. `18640` drops a
dying row callback-free; its re-attach branch runs only in multiplayer
(`DAT_004F741C`). A captor that pops the row first, through its carrying
root (`D040`), a delivery or its own death, pops the corpse: `16750` repeats
its fixed writes, and class14's null hook keeps the corpse's task. The row
check and each child family's release (people, Type123 and peasant corpses)
accept that self-released corpse in place of the backlink.

## Validation

[`native_type28/tests.rs`](../../crates/v2k-game/src/native_type28/tests.rs)
covers:

- all 11 births;
- ten-second cohorts in all five worlds under the real scheduler and the late
  static walk;
- a lethal hit's class12 death, then five seconds of the class12 world;
- a capture: once Type28 pursues a person, a body contact runs `C910` and the
  person attaches to Type28's J row;
- a minute of every world with the live-order pair walk, with no block. In
  world 36, person 20 drowns in Type18 captor 50's grip;
- that drowning in detail: below the sea, class14 replaces `CE90`'s graph,
  the captor's row drops the corpse on its next tick, and neither owner parks
  for the following five seconds.

[The abort test](../../crates/v2k-game/src/main_base_abort_production/native/type28_tests.rs)
checks class12 in every world, once per corpse.
