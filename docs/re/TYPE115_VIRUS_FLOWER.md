# Type115 virus flower

This document owns the infected sunflower's projectile path. Landscape cell
evolution and the separate hive emitter remain in
[VIRUS_SPREAD.md](VIRUS_SPREAD.md). The reusable Gun Turret task is documented
in [INTRO2_TYPE102.md](INTRO2_TYPE102.md).

## Identity and authored data

Intro2 spawn61 is Type115 at raw `[-16384,128,2816]`, with model slots
`[331 sunflwr,144 shadow,332 virusedsunflower,144 shadow]`. The accepted
[full-session observations](RETAIL_CAPTURE_2026-07-12.md) retain slot2/model332
and the same zero-velocity, zero-rotation pose through Intro2 time48.10s.
Its infected appearance is initial state, not a timed model substitution.
That evidence establishes pose/model identity; it does not establish firing
cadence or execution of any particle callback.

The canonical `1X3XX.OVL` Section12 row has section-relative offset`0xB87C`.
Its exact model words distinguish it from the hive and the moving stag:

| Field | Authored value |
|---|---|
| Model slots | `[331,144,332,144]` |
| Original capability dword (`+08`) | `0` |
| Default state policy (`+C0`) | `0x25025` |
| Components | E and L; no A/B/C/D/H/J/N |
| Weighted choices (`+118`) | `[(rule5,1,class29),(rule1,1,class0)]` |
| Rule / alternate class (`+11C/+124`) | `1 / 1` |
| E pointer | `0xB9A4` |
| L pointer | `0xB9C0` |

The exact 28-byte E payload is
`10000000 801A0600 0002A00F D007000A 00001A00 00000000 00000000`.
Its consumer-defined fields are:

| E field | Value |
|---|---|
| Projectile method | `16` |
| Random interval | `400000` microseconds |
| Spread / aim threshold | `512 / 4000` |
| Explicit shot speed | `2000` raw |
| Target axis tolerance | `2560` raw |
| Sound | `0` |
| Muzzle model slot | `26` |
| Alternate muzzle / stochastic mode / auxiliary command | all zero |
| Four variable bindings | all zero |

L bytes `01 02 40 1F A0 0F` bind selectors1/2 and retain words8000/4000.
Do not copy the defensive turret's L second word8000 or its four identical
model slots into this allocation.

## Behavior and firing ownership

Rule5 is `Mutated`, evaluator`004163E0`: it returns
`(entity.state_08 >> 13) & 1`. Rule1 returns1. `00425680` evaluates the
weights and consumes one shared RNG word unconditionally. With state0x2000
set, the two weights are1/1, so either class29 or class0 may be selected;
without that bit only class0 has weight. An infected model alone does not
authorize replacing weighted selection with an always-firing turret.

Class29's descriptor/style/initializer are `004C8928/004C8230/0040D190`.
D190 clears Secondary then Primary and installs Tertiary through4086D0,
whose private initializer/body/destructor are408DF0/408770/4086A0.
The existing shared gun task owns target selection, aim and tracking. Its
408DF0 gravity-lead word comes from44EA60(E.method): method16 uses jump
index4 and returns1, unlike the zero used by methods12/14. After retaining
the original capability and priority table, state0x2000 changes the live
capability to `(old & ~0x1004) | 8` and the constructor sets state0x100.

Class0 is a real alternative, descriptor/style/initializer
`004C8878/004C7468/0040C490`. C490 clears Secondary then Tertiary and calls
402800 for a Primary wait with lifetime9000ms. Its normal expiry/reselection
must remain available; selecting class0 is not an unsupported random outcome.

The gun task's425160 manual firing and4147A0 queue are shared with the
defensive turrets. E's400000us field participates in the stochastic gate and
catch-up cadence; it is not proof of one actual shot exactly every400ms.
Detailed muzzle resolution uses the active infected model and authored slot26.
The body remains at the authored transform while L changes model variables.
No emitter sound is requested by this row.

The user's `sound_037.wav` observation near the flower camera is the separate
Type67 Hive's visible-actor cue: logical99 aliases PCM37 at Q16 rate`0x6666`
(approximately0.4x). The flower's normal/alternate/low-health sound selectors
are all zero. Keep that cue on the Hive's shared detailed-update owner;
neither a flower shot nor the camera command triggers it. The
[Hive audio contract](VIRUS_SPREAD.md#hive-visible-actor-audio) owns the source
and passive-capture bounds.

`DAT_004D02C0[16]` is `[leading=0,speed=2000,particle_class=50,trailing=0]`.
The projectile is therefore class50, not a direct class5 hive spit. Its
descriptor at `004CC138 + 50*0x34` retains:

| Descriptor callback | Address |
|---|---|
| Update | `0043F260` |
| Surface response | `00441EA0` |
| Entity contact | `00441FA0` |
| Static contact | `00442070` |
| Cleanup | null |
| Draw | `0043DC90` |

Its collision mode is3, and its packet pointer is004CC0D8. The callbacks
below do not pass that packet to an entity or static damage routine.

## Class50 contact contracts

All three callbacks prepare class5 children at the class50 particle's current
`+08/+0A/+0C` position. Child owner comes from the current global
`DAT_004DCA00`, **not** the parent's stored owner. Child allocator mode copies
only parent`+1D & 1`. When this bit is set, the ordinary spawn branch is
suppressed; the parent is still consumed. No callback requests sound or
consumes the shared random stream.

The spawn count is signed `DAT_004F72CC >> 13`, replaced by1 when the result
is zero: eight attempts at the ordinary0x10000 pace. Each attempt first
increments shared direction cursor`DAT_004DE840`, selects
`DAT_004CD4B8[cursor % 100]`, and then calls440A60. A rejected fixed-pool
allocation does not stop the loop or undo the cursor step. No additional RNG
draw selects those directions.

| Callback | Branch and effect |
|---|---|
| 441EA0 surface, selector6 | Signed C division of parent X/Z velocity by4 and Y by2; preserve position, return0 and keep parent. This branch precedes suppression testing. |
| 441EA0 surface, every other selector | Emit child velocities `[dir.x,abs(dir.y >> 2),dir.z]`; return1 and consume parent. |
| 441FA0 entity | Emit `[dir.x >> 1,abs(dir.y >> 2),dir.z >> 1]`; return1 and consume parent, without invoking entity damage/infection. |
| 442070 static | Same half-XZ directions as entity contact, then call442B40 to unlink/free the parent directly. No direct static infection call. |

Direction shifts are signed arithmetic shifts. The surface selector6 divisions
truncate toward zero, which differs for negative odd words. Surface contact
does not snap the parent to the terrain/water Y. Entity emission uses the
integrated endpoint. Static sweep `43FF10` instead writes its refined hit
position into `+08/+0A/+0C` before calling442070, so static children inherit
that refined position.

The emitted class5 particles subsequently execute their own normal
`440120 -> 43E180 -> 43E1C0 -> 433720(x,z,1)` terrain path. That later write
sets terrain byte2 bit0x10 and drives the persistent infection overlay.
Class50 does not directly paint an infection radius when it hits. Class5
solid-static `43F920 -> 427DE0 -> 433720` is a separate subsequent contact.

## Accepted runtime evidence

The already accepted `20260722-022303-intro2-actor-ai.jsonl` supplies direct
flower execution evidence; no new recording is needed to identify the owner:

| JSONL line / elapsed time | Observation |
|---|---|
| 1086 / 9880.1475ms | Type115 handle045A0001 is born with style004C8230, model332/slot2, state0F40292D, live capability8, and the authored pose. |
| 44035 / 31880.2419ms | State changes to0946A92D: the0x68000 activation mask is set. |
| 44771 / 33345.1116ms, tick1173 | Class50 birth, slot7, source handle045A0001/type115, position`[-16409,357,2848]`, velocity`[-918,1092,1401]`. |
| 50508 / 42425.211ms, tick1627 | Last observed class50 birth, slot179, at the actor center`[-16384,128,2816]`; its death record follows at42435.3081ms. |
| 50533 / 42450.247ms | The actor changes to class0 style004C7468 while retaining model332 and its pose. |

The source115 filter finds17 class50 birth records and2 recycle records
entering class50, plus19 corresponding death records. These are passive
sample transitions, not a census of all allocation attempts. In particular,
the actor-center last birth is compatible with11400's coarse muzzle branch;
it does not disprove the detailed model muzzle or authorize using the actor
center for every shot. The sampler does not prove which contact callback
consumed each projectile, every class5 descendant, exact RNG order, or a
one-to-one infection write for each class50 disappearance.

The capability distinction matters to Intro2 activation. Authored Type115
capability is0, but the observed infected class29 constructor has already
changed its **live** capability to8. The post-load4519CC test must therefore
clear0x68000 for this allocation before the command enables it. Treating the
authored0 as the constructor's final capability would skip this gate.

## Port boundary and focused follow-through

The port retains a native Type115 allocation through
[`opening.rs`](../../crates/v2k-game/src/opening.rs) and the shared
[`profile.rs`](../../crates/v2k-game/src/intro2_gun_turret/profile.rs)
authenticates its mixed model slots, weighted class29/class0 choices,
alternate class1 and E/L payloads. Both living task graphs retain E cadence
through reselection. Method16 supplies gravity-aware tracking and drains the
real active-model slot26 muzzle into class50; the normal Intro2 capability
pass and authored command control activation. An initial class0 selection
retains capability0 until a later class29 constructor changes it to8.

[`virus_projectile.rs`](../../crates/v2k-game/src/world_fx/virus_projectile.rs)
owns class50's descriptor-authorized motion and three contacts in the shared
[physical traversal](../../crates/v2k-game/src/world_fx/traversal.rs).
Children are allocated before the parent is freed, preserving physical pool
order. Their later class5 callbacks publish into the live terrain allocation.
The current-emitter context is shared with class87 replacement; it does not
borrow the parent flower's owner handle.

Focused regressions cover both selector outcomes, strict class0 expiry and
coarse suppression, method16/gravity state, active-model muzzle publication,
contact direction asymmetry, selector6, suppression, current owner, unchanged
RNG, rejected allocations and the class50-to-class5-to-live-terrain chain.
Incoming Type115 primary/infected hits now retain the live style's callback
matrix: class29 has C690 at infected+20 and null primary+28, while idle class0
has both null. Fresh death selects alternate class1/style4C7150 directly.
`40BAC0` runs `40BAF0`, clears all actor tasks through A860, then requests
deferred removal through10B70. Unlike class49/BD20, it constructs no Type60
ring. Its signed type112..115 BAF0 branch emits ten class37 scatter particles;
the same source then applies the authored radial template before task clearing.
The shared [native explosion owner](../../crates/v2k-game/src/class49_death.rs)
names the Class1/Class49 suffix policy explicitly and retains allocation-bound
terminal custody, preventing repeated particles or nested blasts from replaying
the first-death prefix. Ordinary authored E/L publication uses the actual
spawn transform and manager receipt, including Castle's flowers51/52; it does
not inherit Intro2 spawn61's fixed-pose evidence.

Two normal-tier optimized release runs use the natural frontend RNG, authored
activation, camera, target selection and model muzzle through44s. With40ms
frames and100 frontend ticks, the flower initially chooses class0, reselects
class29 at39.80s and produces seven observed newborn class50 particles from
40.68s. With250 frontend ticks it initially chooses class29, acquires native
Type9 spawn50 at22.04s and produces50 observed newborns from24.52s. The25s
capture visibly shows three pink balls leaving the flower; neither run reports
an unresolved runtime step. The different waits are weighted outcomes, not a
forced shot schedule. Nearby class5/grid observations also include hive
emissions and are not individually attributed to the flower; the focused
physical-traversal regression supplies the parent-to-grid proof.

Type67 spawn24 is the distinct hive: its retained1BEB0 component emits
class5 directly from an attachment at50ms cadence in Intro2. Type26 is the
moving stag/model267 with class4 `Defecate Virus`, which directly emits class5
or makes coarse cell writes. Neither is the Type115 sunflower implementation.
Existing hive/Defecate captures cannot prove execution of flower class50
contacts. Check the accepted capture ledger
before requesting any further recording.

The contact contracts above were verified against the retail executable by
read-only disassembly of441EA0..44213C,442B40..442B98,4163E0..416401,
408DF0..408EBA,40D190..40D1DF,40C490..40C4C0 and44EA60's jump table.
For example, from the repository root:

```powershell
objdump -d -Mintel --start-address=0x441ea0 --stop-address=0x44213d v2000/V2000-nocd.exe
```

These static proofs do not establish a particular run's weighted choice,
target, muzzle pose, shot cadence, allocation acceptance, or timing of the
subsequent infection write.
