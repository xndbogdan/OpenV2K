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

## Held boundary: a carried person drowning

A four-choice person (Type86/95 and kin) carried under water reaches its
`E370` lifecycle. In retail that runs `16750`. It clears `0x20001000` on the
child, sets `0x800`, reapplies the type flags and nulls the child's parent.
It then calls the carried style's release hook `CE90` (enable pairs, then
reselect), and `10C10` follows. `16750` never edits the captor's J list.

The four-choice owner's `E370` admits only a null release hook. It therefore
fails closed and parks, with its prefix committed. The parked person then
blocks its captor's next release ("capture four-choice actor pending
callback"), and the captor parks too. Pairs that touch the parked captor fail
closed in turn ("completed pair owner").

This chain predates Type28: Type122 and Type18 captors carrying four-choice
people into deep water reach it too. World 36 reaches it in its first minute.
Closing it needs the captor's response to a child that released itself
(`18640`'s J update and the carry tasks); ordinary Type9's carried expiry is
the model.

## Validation

[`native_type28/tests.rs`](../../crates/v2k-game/src/native_type28/tests.rs)
covers:

- all 11 births;
- ten-second cohorts in all five worlds under the real scheduler and the late
  static walk;
- a lethal hit's class12 death, then five seconds of the class12 world;
- a capture: once Type28 pursues a person, a body contact runs `C910` and the
  person attaches to Type28's J row;
- a minute of every world with the live-order pair walk. This asserts no block
  except the held drowning chain above. In world 36, person 20 drowns and
  its Type18 captor 50 holds; either captor family may carry it.

[The abort test](../../crates/v2k-game/src/main_base_abort_production/native/type28_tests.rs)
checks class12 in every world, once per corpse.
