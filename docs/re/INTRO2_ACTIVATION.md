# Intro2 actor activation

This document owns the post-load activation gate and its release by authored
Section-2 commands. Actor constructors and callbacks remain in their native
subsystem documents; whole-scene acceptance remains in
objective 08.

## Post-load state, separate from construction

The retail `451710` load path finishes common construction and radar setup,
then walks the intrusive actor list. The Type0/Klaus branch primes its own
world-entry callback. For other actors, `4519BE` tests the signed world mode
byte at `+296`: modes below 4 enter the cinematic setup branch.

The executable establishes the following decision order:

| Source | Condition or write |
|---|---|
| `4519C7` | Type34 goes to the activation clear. |
| `4519CC` | Otherwise, test bit `0x08` in the **instance** capability byte at `+64`; a set bit goes to the same clear. |
| `4519D2..4519D7` | Otherwise, Type66 receives current health `+30 = 1`. |
| `4519E0..4519EC` | Write `state +08 &= 0xFFF97FFF`. |
| `4519EF..451A1C` | Only Type34 also receives the meteor launch offset and velocity. |

The clear removes exactly `0x68000`: checked-damage/pair admission `0x8000`,
common callback admission `0x20000`, and master motion admission `0x40000`.
It does not clear render bit `0x800`, unlink the actor, change its model slot,
reset task age, replace its behavior graph, or consume RNG. Constructors still
run in authored order, allocate their components and tasks, and consume their
normal selector/component words before this pass.

This is a runtime capability test, not a list of insect types or actors that
happen to have operation-2 records. Ordinary villagers, scientists, structures,
and turrets remain governed by their own capability words. In particular,
Type115's authored capability is 0, but its infected Gun Turret constructor
changes the live capability to 8; the accepted actor-AI recording confirms
that allocation must take this activation clear. The native flower owner
preserves that distinction: an initial class0 choice retains capability0,
while class29 changes it to8; see
[Type115 virus flower](TYPE115_VIRUS_FLOWER.md). The Type66 health write and
meteor launch fields have separate owners:
[INTRO2_TYPE66.md](INTRO2_TYPE66.md) and
[INTRO2_METEORS.md](INTRO2_METEORS.md).

## Release by the authored command stream

`52CB0/52790 -> 52270` evaluates Section-2 records during presentation.
Operation 2 ORs `0x68000`; operation 3 clears the same mask. Both preserve
visibility. Camera-selection operations address the same stable allocation
tags but do not perform either activation write. Tags `38914..38975` identify
Section-13 spawns `0..61`, independently of later live-list removals.

The normal-tier level50 strings contain these operation-2 windows. Times are
milliseconds on the cinematic clock, with inclusive endpoints.

| Window | Authored spawns |
|---|---|
| 500–2000 | 33, meteor |
| 1000–2000 | 34, meteor |
| 1500–2000 | 35, meteor |
| 3000–5000 | 31, meteor |
| 5500–6000 | 20, 19, 38, 5, 4 |
| 6500–7000 | 46, 21 |
| 15000–15500 | 43, 44, 41, 26, 24, 30, 42, 7, 8 |
| 22000–22500 | 25, 46, 61, 1, 10, 0, 6, 22, 40 |
| 50000–50500 | 55, 56, dragons |

The first insect group therefore starts at 5500 ms. This is an authored time
gate, not a callback from a meteor collision. The caption about creatures
following the meteors runs from 7000 to 15000 ms. A meteor's actual landing
still follows its live motion/contact implementation; it does not allocate
or activate the insect group.

The accepted full-session and actor-AI histories both retain spawns4/5/20/38
with their admission bits cleared at birth and set at tick275. Meteor31's
terminal state or unlink is observed at tick299, about480ms later. Thus the
existing retail evidence already places the first release before that final
village meteor impact; its screen visibility still depends on live pose and
camera framing rather than an impact-owned spawn.

The command writes become visible to the next world update. A dormant native
actor still receives the common scheduler prefix, including its normal
timers/waits and owned `+B2` clearing. The disabled callback and master-motion
gates suppress its task execution and translation. The same initial `0x8000`
clear makes an early meteor radial pass skip it before impulse or damage.

Particle model collision has its own admission at `43F980`: the executable
at `43FA3D..43FA51` requires `0x8000` and then accepts either a set `0x800`
or a clear `0x1000`. This check precedes model lookup, broad-radius testing,
and the model sweep. Both the Intro2 and Playing collision projections use
the shared predicate in
[entity_collision_state.rs](../../crates/v2k-game/src/entity_collision_state.rs).
A known clear `0x8000` rejects a dormant actor without requiring unrelated
state bits to be known. Unresolved admission is reported explicitly; it does
not create a collision candidate. Intro2 retains each unresolved entity ID
once, in first-observed live-list order, across inline projection refreshes.
The gate leaves admitted actors' primary/infected hit wrappers and survivor
policy checks intact. In particular, enabling Type77 does not establish its
currently unimplemented survivor behavior.

## Port ownership and validation

### Birth clearance before the dormant interval

The shared `D4A0` placement runs before activation is cleared. `40D5A0` reads
the terrain height when policy bit `0x20` is set; `40D62E` optionally adds
the active model's header `+08` radius for policy bit `0x40`. Independently of
that model flag, `40D650..40D691` tests the type's Sub-C descriptor pointer at
`+0xD8` and adds its signed first word (`base_clearance_raw`). Only then does
`40D6BB..40D6CA` copy the final pose to the immutable `+90` anchor. A clear
bit `0x20`, or absent terrain, skips this complete placement branch.

Both `20260712-203635-full-session.jsonl` and
`20260722-022303-intro2-actor-ai.jsonl` retain the same first-cohort births:
Type17 spawn4 Y `11 = -64 + 75`, Type16 spawn5 Y `491 = 416 + 75`, and
Type53 spawns20/38 Y `242 = 192 + 50` / `178 = 128 + 50`. The port formerly
omitted that clearance, leaving dormant bodies and immutable anchors too low
until the mover first ran. The shared constructor placement now supplies the
clearance to native and replay-selected insects, including Type47/58/94.
The [corpus regression](../../crates/v2k-game/tests/intro2_activation.rs)
checks both constructor entries before any actor tick.
This repairs initial placement; it does not establish matched visual arrival
or authorize movement for dormant Type77. Type122 now has a separate
[native constructor](TYPE122_RUNTIME.md) with its own allocation evidence.

[EntityManager::disable_authored_behavior_components](../../crates/v2k-game/src/entity.rs)
applies the source Type34/instance-capability predicate once after loading the
Intro2 world. Its retained hive-emitter enable value follows the same gate.
[Intro2Commands](../../crates/v2k-game/src/intro2_commands.rs) releases each
allocation through the canonical strings. Native actor owners consume those
state bits; no actor-specific start time is added to their callbacks.

The previous post-load adapter cleared only Type13 spawn0 and the hive's
separate enable boolean. Other native insect constructors retained their
enabled callback, motion and damage state. That omission accounts for a
concrete path to premature movement and susceptibility to the early meteor
blast. It does not establish that insects should be hidden before activation.

Focused corpus regressions in
[intro2_activation.rs](../../crates/v2k-game/tests/intro2_activation.rs)
cover retained task/model/pose state, instance-capability versus type lookup,
the canonical release windows, a native Type17's pre-release scheduler visits,
and a Type16's rejection/admission of the same meteor radial request. All four
activation regressions and the focused F980 collision controls pass within the
4012-test V2000 workspace suite. Complete fixed and varied-frame OpenGL
walkthroughs also pass without unresolved collision candidates or the dormant
Type77 meteor-particle hit reproduced before the F980 repair. Run conditions
and combat limits are recorded in [INTRO2_TYPE102.md](INTRO2_TYPE102.md#validation-boundary).
Matched cinematic comparison remains separate.

The existing accepted full-session and actor-AI recordings remain available
through the capture ledger. No additional
retail recording or timing receipt was needed for this state-mask repair.
Exact visual arrival relative to the meteor landing, the bee's underwater
excursion, and remaining scene-only presentation paths still require the
whole-scene acceptance comparison; this repair does not close those claims.
In particular, Type77 spawn45 has capability8 and no enable record. The old
`intro_actor_motion_position` flying group no longer translates it; presentation
uses the committed authored pose through `intro2_uses_live_actor_pose`. Bounce
or restitution is not a substitute enable. Its unimplemented survivor policy
after a genuine particle hit remains an explicit boundary.

The same live-pose gate now covers the twelve Type52 class-0 camera/beacon
anchors, Type67 spawn24 once its authored radial emitter is present, and
Type115 spawn61 which is born on slot 2 (`virusedsunflower`) rather than a
clock-driven model switch. **Type122 spawn21** (model274) now uses its own
native A/B/C/D/E/H/J receipt and live task pose. Its capability8 and operation-2
enable at6500–7000 ms retain the same command gate; no synthetic6.5s trajectory
runs for a native allocation. Explicit compatibility/replay bodies without
that receipt retain their separate presentation fallback.
