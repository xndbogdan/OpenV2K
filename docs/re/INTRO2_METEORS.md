# Intro2 Type34 meteors

This document owns the four authored Type34/model560 `grock` actors in Intro2.
The opening state machine and Section-2 event interpreter remain in
[LOADING_TRANSITIONS.md](LOADING_TRANSITIONS.md). Shared task-wrapper ownership
is described in [ACTOR_RUNTIME.md](ACTOR_RUNTIME.md); shared hit/death ownership
is in [ENTITY_DAMAGE_AND_DEATH.md](ENTITY_DAMAGE_AND_DEATH.md).

## Authored construction and activation

Normal-tier Intro2 is level50. Its four Section-13 records override every model
slot with 560. The cumulative Section-12 Type34 record has mass100, health1000,
capability flags0, effective/default flags `0x4008`, no optional components
A..O, no attached sound, no low-health effects, and no surface-lifetime effect.
Its sole weighted choice is rule1, multiplier1, class19 **Boulder Trailing Fire**.
The ordinary selector still consumes one process RNG word for this singleton.
Class19 descriptor `0x004C8900` selects style `0x004C7858` and initializer
`FUN_0040B910`; it is not Shoot Randomly.

| Spawn | Authored raw XYZ | Activation tick |
|---|---|---|
| 31 | `[-30976,6400,-28672]` | 150 |
| 33 | `[-29184,5120,-26112]` | 25 |
| 34 | `[32256,5120,-26112]` | 50 |
| 35 | `[-30208,5120,-26112]` | 75 |

`B910` first constructs Tertiary through `4069E0(slot2,0x28)`, clears Secondary,
then constructs Primary through `404580(slot0,5000)`. The Primary's
`401350 -> 4012E0` private initializer copies the **authored, pre-launch** XYZ,
sets direction dword `+10=1`, and clears movement words `+14/+16` and stationary
counter `+18`. Its common-mover initializer has no component work for Type34.

After all common constructions, `4519E0..451A1C` clears state `0x68000`, adds
raw offset `[-0x500,0,+0xA00]`, and writes velocity `[1500,-400,-3000]`.
The original `413F70` angle-zero constructor matrix is retained:

```text
lateral [0,0,-2147352576]
up      [0,2147352576,0]
forward [2147352576,0,0]
```

The activation events permanently OR `0x68000`. Time determines these authored
events, not the subsequent trajectory, rotation, trail cadence, or death.
Retail leaves heap word `+B2` unwritten at construction. The native Intro2
allocation initializes it once to zero; this is an explicit port birth policy,
not a claim that the retail allocator always returned zero.

## Common scheduler and rolling task

`FUN_00412DA0` owns subject gate `+70`, callback carry `+6C`, shared RNG waits,
mass `u16(typeMass+B2)` with zero promoted to1, post-callback B2 clearing, sound
follow, and final master integration. State `0x02000000` selects detailed
`DCA0` and bypasses random waits. Otherwise `E870` uses coarse mode1. Both
callbacks traverse Primary, Secondary, Tertiary before environment forces.
No 8-ms subdivision is part of this contract.

`FUN_00404690` compares all three current words to private previous XYZ.
An unchanged pose increments a wrapping signed counter; the eleventh stationary
visit clears `+14/+16` and returns singleton `004BE190` (tag `9C00`) before
damping. A changed pose resets the counter and replaces previous XYZ in both
modes. Only detailed mode mutates the matrix.

For `Q(a,b) = i32((i64(a)*i64(b)) >> 31)`, let `dx,dz` be wrapping signed-word
`previous-current` differences. Model-header extent `+08` is unsigned256:

```text
x = -Q((i32(dz)<<14)/extent, 0x7fffffff)
z =  Q((i32(dx)<<14)/extent, 0x7fffffff)
rX = Q(x,lateral.x) + Q(z,lateral.z)
rY = Q(x,up.x)      + Q(z,up.z)
rZ = Q(x,forward.x) + Q(z,forward.z)
```

Products/sums wrap at the shown widths and division truncates toward zero.
All three angles are computed from the old matrix. Nonzero rotations execute
`457DD0(lateral,up,rZ)`, `457A90(up,forward,rX)`, then
`457C30(lateral,forward,-rY)`. Each old vector pair U,V becomes
`U'=Q(c,U)+Q(s,V)`, `V'=Q(c,V)+Q(-s,U)`. The sine lookup at `004D14D0` masks
angle bits `0x3FFC`, reflects the quarter-table index at bit4000, duplicates
the unsigned table word into both halves of a dword, then negates at bit8000.
Cosine uses angle+4000. The accumulated fixed-point scale/shear is retained;
normalizing the matrix changes retail geometry.

Nonterminal Primary visits approach signed `vx/vz` toward zero by `dt>>13`.
Its 5000-ms wrapper timeout is strict `elapsed>5000` after callback unwind.
Both the stationary tag and timeout reach style `40CEF0 -> 410C10` generic death.

Effective bit4000 suppresses DCA0/E870's normal Euler basis rebuild. E100 adds
gravity `vy -= Q(dt,0x300000)`, then applies the current level's no-wind drag.
Intro2's initial wind mode is0 and strength3, giving
`drag=dt*3/(mass*8)` and `v -= (drag*v)>>15`. The master suffix then stores
`p += (((dt>>5)*v)>>15)` as wrapping signed words. The captured 48-ms interval
is exact: position `[-32256,6400,-26112] -> [-32188,6378,-26249]`, velocity
`[1500,-400,-3000] -> [1487,-467,-2978]`.

## Live fire trail and explosion effects

Tertiary callback `406A70` emits only in detailed mode and only when the
wrapping signed sum `vx²+vy²+vz²` is strictly greater than `0x77A10` (700²).
It sees Primary's damped velocity and the pre-environment, pre-integration pose.
It calls `4061A0(entity,0x28,dt)`, then ORs entity byte+84 with0x20.
The accepted local meteor has its state high bit clear; the remote/high-bit
branch in12DA0 is not this actor's emitter owner.

4061A0 attempts at least one stationary class40 (`0x28`) particle with the
source handle. For each attempt, its local signed-word Y velocity subtracts43,
allocation runs, and the signed remaining delta subtracts30000. **Three RNG
draws then run even after the final attempt or a rejected allocation.** Each
next coordinate adds `((rng & 0xffff)>>11)-16` and
`Q(localVelocity,0x03A98000)` as wrapping words. Further attempts run only while
remaining delta is positive. Class40 has eight frames, rate0x18, scale0x0800,
jitter divisor0x20, priority2, radius0x28, lifetime15ticks, and no velocity bias
or update callback. No fixed lifetime, fitted onset, or predetermined attempt
count belongs in the live task.

Class1 death's `BAC0 -> BAF0` selects ten scatter attempts with both alternating
classes equal16 (`0x10`). Shared40950 runs frame-paced407D0 first: advance the
process direction cursor before each attempt and stop scatter on allocation
failure. Model extent256 controls the L1-normalized offsets
`[a,b,b]*256/(abs(a)+abs(b)+abs(c))`; Z deliberately duplicates B. Velocity is
`[a,b,c]*8>>4`. The independent surface tail then attempts class18 (`0x12`)
with velocity `[0,500,0]` strictly above the sea plane, or class45 then46 with
the second X shifted-64. Finally it consumes one RNG word for sound62 at rate
`0x10000 + (rngLow16>>3)`. All particles retain source type34 and handle.

Class16 uses global frames699..702, rate0x20, lifetime60ticks; class18 uses
873..886, rate0x10, lifetime54ticks. The underwater classes45/46 use790..792
and lifetime128ticks. Shared particle allocation, priority, recycling, physical
slot traversal, and particle-child gravity/trails remain WorldFx ownership.

## Contact and death boundary

### Infected particle delivery

The native class19 owner admits the descriptor-selected `F780 -> 11250`
infected entry with static packet `004CBFD0`:
`[6,0,2000,0,0,0]`. Its trailing source and owner words are literal zero;
particle birth provenance does not replace them. The adapter requires the
authored allocation, unchanged Primary/Tertiary wrappers and scheduler receipt,
class19/style `004C7858`, and the exact Type34 damage profile before mutation.
It rejects an executing or terminal wrapper rather than replacing its owner.

`11250` sets model bit2000, observes null type+82 sound, and reaches `DA00`'s
style+20 hook, which is null. It does not stamp+34 or execute the primary
accepted-hit suffix. All four model slots are560. Channel6 contributes zero
impact, but enabled, unsuppressed `11030` still consumes three shared RNG words
and subtracts0400 from each wrapping Euler word. Velocity and the accumulated
physical matrix remain unchanged. The optional network callback is a retained
post-reaction boundary. Disabled or suppressed reaction skips those inputs and
draws.

`15040` then filters channel6 to zero with Type34's multiplier0. Capability0
does not request filtered-zero player feedback, so buffer, health, task ages,
private rolling/trail state and scheduler receipt remain unchanged. Repeated
infected hits keep that same receipt. This closes the observed class5 delivery;
primary packets, nonzero particle damage and their explosion/death continuation
remain outside this adapter. Focused regressions cover the native allocation,
repeated zero-impact RNG, preflight rejection and retained failure prefixes;
all eight focused delivery regressions pass with the complete V2000 workspace
suite. The [combined scene checks](INTRO2_TYPE94.md#validation) complete both
walkthroughs without unresolved meteor hits or dropped task owners.

### Terrain and termination

The global11AD0 scan follows the complete12DA0 mover pass. Its admitted solid
path requires state8000 set,1000 clear, subject gate70 zero, nonzero active
model collision radius, no remote/fixed flags88000000, and state10000 set.
12870 submits the retained oriented model560 collision program; its header
collision radius+0A is284. D7F0 finds class19's null terrain callback and enters
141D0 with responseMAX, scale2, mode1.

141D0 sets state800000 and separates XYZ by `(normalQ12*penetration)>>12`.
For inward velocity it emits the ground-material effect at bilinear terrain
height **before** velocity correction. Response is `speed+Q(speed,MAX)`, effect
scale twice that value, and correction `normalQ12*(response>>1)>>12`.
Channel1 damage is `((sum(wrapping_i16(oldV-newV)²)>>7)*mass)>>10`.
Type34's threshold2000 and multiplier256 filter this packet before common
buffer/health handling.

Lethal damage sets health0 and state4000. Type+90 and attached sound are null.
`DB80 -> AC40/AC60 -> 25660` takes alternate rule3/class1 without a weighted
RNG selection. BAF0 performs its effects prefix, then4566E0 delivers the
authored radial template: inner512, outer1024, impulse2000, channels[1,3],
amounts[4000,4000], source type34/self handle. Only afterwards BAC0 clears all
three slots through A860 and calls10B70: clear60000, set100000, enqueue for the
later live-list sweep. A task-triggered death still completes the source
DCA0/E870 environment and post-callback suffix; cleared master-enable prevents
position integration. Four model slots all remain560.

The native owner separates the synchronous effects prefix, authenticated radial
receipt, and exactly-once finalizer. The terrain adapter does not claim the
following129B0 water callback, static-model scan, or active-pair scan. Their
broader Intro2 walker remains an explicit integration boundary. The accepted
tracked impact is above sea: rawY95 versus sea-1129.

## Evidence and acceptance

Static retail functions above, the normal-tier canonical Rust asset decoders,
and accepted captures `20260712-213639-intro-meteor.jsonl`,
`20260712-222040-intro-meteor.jsonl`, `20260712-203635-full-session.jsonl`, and
`20260716-165213-intro2-particles.jsonl` establish the contract. Accepted capture
names and query policy are owned by the
capture ledger.

Sampled detailed-mode onsets108/132/159/180, impact direction windows25..34,
21..30,22..31,12..21, and observed birth/recycle counts86/110/90/291 describe
those recordings. They are not production inputs. The tracked final pose at
tick298 was `[-28860,95,32396]` with velocity `[290,1436,1740]`, deleted at299;
the independent full-session capture shifted the impact/deletion one tick.
This variation is expected from shared callback timing and pool/RNG history.
Retail-matched full-sequence visual acceptance remains open.
