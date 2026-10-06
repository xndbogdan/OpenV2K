# Intro2 combat and projectile evidence

This document owns the comparison between retail's natural late-Intro2 combat
and the port's projectile execution. Native actor ownership remains in
[Type10 dragons](INTRO2_TYPE10.md), [Type92/102 turrets](INTRO2_TYPE102.md),
[native workers](INTRO2_TYPE8.md), and [Type66 structures](INTRO2_TYPE66.md).
Whole-scene acceptance remains in
objective 08.

## Natural combat outcomes

Three complete accepted passive recordings show the defensive turrets removed
and the factory becoming a wreck. Both dragons survive until the world unloads.
The factory camera command at 69 seconds does not itself cause destruction.

| Recording | Turret spawn53 removed/dying | Turret spawn54 removed/dying | Factory model225 / health0 |
|---|---|---|---|
| `20260722-022303-intro2-actor-ai.jsonl` | tick2738, line80445 | tick2971, line94342 | tick3583, line140136 |
| `20260717-032945-menu-intro2-level1.jsonl` | tick2764, line5001 | tick2924, line5288 | tick3479, line6459 |
| `20260712-203635-full-session.jsonl` | tick2792, line6351 | tick2947, line6703 | tick3500, line7957 |

The integer clock runs at 50 Hz, placing those factory terminal observations
around 69.6–71.7 seconds. Capture elapsed time includes menu/loading time and
must not be substituted for the scene clock. The two session samplers retain
state changes rather than every health write; their last live turret snapshot
can therefore precede the lethal change. Removal alone does not identify an
incoming projectile or the callback that caused it.

The actor-AI recording gives the most detailed joined evidence. Its globally
retained lifecycle records show:

- Factory spawn51, handle `04640001`, starts at health 1, repairs to 99999,
  then first falls to 87699 at tick 3249 / line 111012. Repeated attack losses
  interleave repair before the staged death's health10000000 sentinel first
  appears at tick3423 / line123840. Terminal entry writes health 0,
  model 210→225 and style `4C7468` at tick3583, 160 ticks later. The 69-second
  camera cut lies inside that source-owned 32×100ms destruction sequence.
- Turrets `04620001` and `04610001` each move from health 4000 to 0 and
  style `4C71E0` at their listed ticks. Their class49 effects are visible in
  particle records before the next sampled lifecycle record.
- Dragon spawn55, handle `04600001`, takes nine separate 800-point losses,
  from 32000 to 24800. Spawn56, `045F0001`, retains 32000 in this recording.
  Other recordings show different damage totals while preserving both actors.

These observations establish live combat rather than a timed replacement
model. They do not require identical shot counts or outcomes under a different
frame cadence and shared random-stream history.

## Source-owned projectile observations

The actor-AI capture's particle records retain physical slot, class, source
handle/type, raw position and velocity, age, flags, and the complete 32-byte
record. It observes 129 dragon class38 generation starts and 13 turret class55
starts. These are sampled birth/recycle signatures, not exact allocator-call
totals. The inspector detects a generation change when class changes or age
decreases; same-slot activity between samples can remain unobserved.

| Sample | Source / slot | Raw position | Raw velocity | Age |
|---|---|---|---|---|
| line76411, tick2620 | dragon56 `045F0001`, class38 slot189 | `[23980,1641,465]` | `[529,994,-1090]` | 0 |
| line78043, tick2681 | same slot's last snapshot before removal | `[24584,-258,-894]` | `[529,976,-1090]` | 60 |
| line78878, tick2700 | turret53 `04620001`, class55 slot197 | `[25644,513,-2590]` | `[-470,2758,4142]` | 0 |
| line83745, tick2765 | same slot's last snapshot before removal | `[25004,3873,2370]` | `[-470,2758,4142]` | 64 |

These endpoint pairs prove movement. They do not reveal every intervening
gravity, contact, reflection, or surface-classification update; positive Y
velocity at both endpoints is not proof of the integration sign between them.
The native emitter contracts independently establish method10→class38 for
dragons and method14→class55, speed5000, for defensive turrets.

The same capture provides spatial and temporal joins with actual damage:

| Damage observation | Source-owned projectile's preceding snapshot |
|---|---|
| Turret53 health0 at tick2738 | line80356: dragon56 class38 slot29 ends at `[25464,235,-2894]`, age50, about163 raw units from the turret center |
| Turret54 health0 at tick2971 | line94278: dragon55 class38 slot12 ends at `[26739,328,-5329]`, age58, about131 raw units from the turret center |
| Factory's first health loss at tick3249 | line110992: dragon55 class38 slot156 ends at `[21763,-280,-6444]`, age48 |
| Factory's death sentinel at tick3423 | line123829: dragon56 class38 slot169 recycles at `[21473,-112,-6357]`, velocity `[-1102,-3877,773]`, age36 |

The joins support dragon-projectile damage as the natural trigger. A passive
slot disappearance/recycle does not directly identify the `F980/F780` call,
collision primitive or retained aiming target; those require source-backed
execution or a bounded callback query if a mismatch remains.
For recycle records, the outgoing projectile is in `previous`; reading only
the replacement `particle` loses the class38 identity at the lethal sample.

## Descriptor-owned execution

`40120` advances the signed position words with the incoming signed velocity:
`position += (velocity * (dt_us >> 5)) >> 15`, narrowing and wrapping each
component on every visit. The descriptor update then runs before entity and
static sweeps. Retaining fractional displacement between frames changes this
contract. Water classification and physical-slot visitation remain shared with
the other particles; callback allocations into later slots run in the same
pass, while earlier slots wait.

| Classes | Update / surface / cleanup | F590/F800 packet |
|---|---|---|
| 38, dragon fireball | `43F350` / `441B70` / null | `4CC048`: channels `[2,3]`, amounts `[500,6000]` |
| 49, method12 turret bolt | `42E8E0` no-op / `441C90` / null | `4CC078`: channels `[2,3]`, amounts `[6000,2000]` |
| 80, submerged method12 bolt | `43F030` / `441C90` / `442140` | same `4CC078` packet |
| 55, turret bolt | `42E8E0` no-op / `441C90` / null | `4CC0A8`: channels `[2,3]`, amounts `[3000,1000]` |
| 81, submerged turret bolt | `43F030` / `441C90` / `442140` | same `4CC0A8` packet |
| 56, method13 green plasma bolt | `42E8E0` no-op / `441C90` / null | `4CC090`: channels `[2,3]`, amounts `[4000,1500]` |
| 82, submerged green plasma bolt | `43F030` / `441C90` / `442140` | same `4CC090` packet |

The entity sweep `43F980` precedes the static sweep `43FF10`. F590 presents
and delivers at the integrated endpoint; FF10 instead publishes its refined
static probe before F800. Both preserve source type and owner from particle
birth in the packet's trailing words. F610 visual work precedes synchronous
damage, then `442B40` cleans up and frees the current physical record. The
particle suppression bit skips damage alone. These are the ordinary checked
hit paths, retaining target filtering, style callbacks and death ownership.
For example, a class38 packet filters to 12300 against the native Type66
profile; that value is an outcome of the profile, not a fixed factory hit.

`43F350` runs only while class38 is not fully submerged. Its do-while loop
attempts class40 through `4410B0`, applies gravity, then consumes three shared
random words in X/Y/Z order to move the next trail position. It runs at least
once, even for zero elapsed time. With `budget=5000000-64*count_scale_q16`,
the loop time quantum is `budget>>5`, the trail motion scale is `budget>>10`,
and the gravity multiplier is `0x300000` in the signed fixed-point product.
The class40 allocation is rejected at or below flat sea, but that rejection
does not remove the subsequent gravity or jitter draws.

`441B70` consumes class38 on surface selector 7. Selector 6 places an effect
at the response Y, choosing class43 at/below flat sea or class75 above it,
with incoming velocity `[0,200,0]`, then consumes the fireball. Other selectors
raise its center to response Y plus radius and, only for negative vertical
velocity, use `1-trunc(vy/4)` before queueing the ground program below.

`4410B0` keeps class49 above the signed flat sea word and substitutes80 at or
below it (`4410F3`); class55 independently substitutes81 (`441111`), and
class56 substitutes82 (`441102`). None of these bolt branches changes position
or velocity before40A60. Type97's method12 FIFO uses this constructor,
preserving its own source type/owner and6000/2000 packet
through F590 entity and F800 static impacts. An unsupported constructor input
must not stand in for an attempted particle allocation after accepting a method.

`441C90` selector 6 relinks class49 in place as80 (`441D90`) and class55 as81
(`441D38`), and class56 as82 (`441CDD`), preserving velocity and resetting age;
each existing submerged descriptor also resets age on this branch.
For other selectors the authored response table chooses the debris class,
advances the direction-table cursor for each attempt, then draws one shared
word for sound83 pitch `0xE000+(word>>2)` and consumes the bolt. Null response
classes still advance the cursor. `43F030` relinks a class80 that is no longer
fully submerged back to49 (`43F11A`), while81 returns to55 (`43F0C2`) and82
returns to56 (`43F067`), and resets age. A remaining submerged bolt older than five
ticks sets its pending-destruction bit; the next `40120` precheck performs
cleanup. `442140` attempts nine class42 bubbles at successive normalized
velocity steps. Allocator recycling and bulk reset do not invoke that cleanup.
The shared port binds these callbacks to all three explicit pairs 49/80,
55/81 and 56/82, retaining their distinct packets. Player selectors 12/13/14
reach the same `424650 -> 44E770 -> 4410B0` constructor in retail; the port's
queued player emission also selects the appropriate submerged descriptor
before its first particle visit. Previously selector 13 could fire green plasma
visually while its omitted 56/82 collision owner delivered no hit. Regressions
now acquire the actual inventory weapon, emit it through the player firing
path, sweep a model and assert its complete 4000/1500 packet and provenance in
air and water. Shared tests cover all three pairs' sea-equality birth,
water-contact and emergence relinks, delayed expiry and cleanup.

The descriptor's signed `+0x28` word is a **position-Y** bias in `440A60`,
applied before birth water classification. The separate `+0x2A` word biases
velocity Y. Keeping these fields distinct also corrects shared child effects;
the combat descriptors themselves have zero in both words.

### Copied-record streak presentation

Descriptor `+0x30 = 43D300` selects the same draw routine for classes
49/55/56/80/81/82. It copies the complete 32-byte record to the stack, draws
the original XYZ, then nine further samples at wrapping signed-word steps.
Age, animation frame and scale remain those of the original record.

The signed velocity square sum wraps to 32 bits; the `457730` square-root
return is narrowed to a signed word before division. Each Q12 normalized
quotient is also narrowed to a word, then the step is `(quotient*25)>>12`.
Zero speed selects a positive-X step of 25. This is distinct from the
Q31-normalization helper and its length adjustment.

D410's temporary visible/deletion bits affect only the stack copy. Rejected
samples do not free the live particle or run class81's cleanup, and do not
prevent later samples from being considered. Each surviving sample retains
the later sprite far-depth/rectangle admission. All six authored frame rows
have zero middle word and zero jitter divisor, so this routine adds neither
particle lights nor address-dependent scale jitter. Implementation ownership
is `world_fx/presentation.rs`; intrinsic sprites remain unchanged.

### Queued fireball ground program

`441B70` passes the pointer held at `4C9910` to `28720`. The actual program
at `4C98F8` contains exactly `[0,11,1]`, `[0,0,0]`; `4C9910` is separate
data after its terminator. It shares the static scheduler's cell key and FIFO,
including deduplication against ordinary damage/crater programs and the node
whose actions are currently being applied. Admission itself has no RNG draw,
damage filter or terrain write. Because particles follow the static pass,
their new programs first execute on the following static update.

Opcode11 calls `37100`: for a positive amount and nonzero high three light
bits, subtract the amount, clamp at zero, and preserve the low five bits.
Neither a static descriptor nor its model/height is required. `281A0` supplies
X=`cellX*256+128`, Z=`cellZ*256`; `4A890` refresh uses that exact position,
including the additional material-rounding half-cell in `4A5C0`.

The write belongs only to the current level's terrain allocation. It marks
terrain presentation dirty and refreshes the persistent radar immediately,
consuming one shared random word only when the original coverage map covers
that cell. A no-change branch performs neither refresh nor RNG. A later
radar-resource failure retains the terrain write and dirty request, with an
explicit diagnostic rather than replaying the consumed opcode. The dirty
request follows load/unload and Main Base transaction rollback. Shared static
program authority remains in
[ENTITY_STATIC_DAMAGE_PROGRAMS.md](ENTITY_STATIC_DAMAGE_PROGRAMS.md).

### Production scene comparison

Focused source tests cover the callback, sweep, draw-copy and queue contracts,
including the Type66-filtered 12300 hit. Native Type92/102 owners now retain
their distinct weapon, model and damage profiles through class49 death. Its
Type60 tail uses the shared class48 task with constructor-owned default model243
and natural class2 removal. Native workers retain their own allocation/Sub-D
state through live hits and alternate14 death; these are not fixed-actor
survivor-policy exceptions. Their previously missing constructor random draws
also belong to the shared stream, so the scene comparison uses the complete
current construction order.

The reference comparison before shared Type53 construction uses two
uninterrupted normal-tier OpenGL runs through New Game, story, final
black card and first-world reveal. Both kill all three turrets, run the Type60
rings to removal, kill/remove the three workers, preserve both dragons and
turn the factory into model225/health0 through natural projectile damage.
The 69-second camera command causes none of those health/model changes.

| Port frame/stream entry | Type102 spawn53/54 death ticks | Type92 spawn52 death | Factory first hit / sentinel / wreck | Runtime issues |
|---|---|---|---|---|
| 40ms, frontend100 | 2772 / 2950 | 3202 | 3164 / 3548 / 3708 | 0 |
| Varied delta, frontend250 | 2819 / 3004 | 3238 | 3540 / 3615 / 3781 | 1, Type57 hit below |

The fixed run's sentinel-to-wreck interval is 160 ticks; the varied run takes
166 ticks through the same task's per-visit timing. The outcome matches the
retail combat sequence, while exact shot counts/timestamps remain sensitive
to frame cadence and the shared random history. A subsequent fixed-40ms GL
comparison retains these same event ticks, turret rings and factory wreck smoke
after repairing the dragon wing's linked type13 projection defect in
[the renderer authority](RENDER_PIPELINE.md#world-alias-dependencies). The long
dark wing strips are gone; whole-scene matched retail acceptance remains open.

The varied run's tick2201 hit by Type13 sourceid1 (class38) against runtimeid2,
authored **spawn1/Type57/model122**, is now natively admitted: its class7 SearchAndAttack
styles `4C7A50/4C7A98` dispatch C690 at primary+28 and infected+20, followed by ordinary
impact reaction and checked damage. Filtered damage (24000) exceeds health (5000),
entering alternate11 `C660/404360`, initializing Sub-G with `1B970` tumble, and
publishing `Intro2Type57Tumble`. Contact resolves to the `C750/BAC0` terminal explosion
with sound 40950 and class 37 scatter, authenticating radial death.

The hive's wreck-ring draw remains separate: it allocates no Type60 and
requires a dead marker-backed hive. Neither of these Intro2 runs enters that
death state. Whole-scene acceptance remains open for Type57 and the rendering,
bee-water, activation and final-card audio comparisons in objective08.

A prior varied-frame port comparison exposed class30's channel1/2500 fragment
at tick1473 reaching
**Type13 spawn0/runtimeid1**, with raw velocity `[254,-471,508]`. Its delivery
is now natively dispatched via `apply_intro2_type13_particle_hit` (vtable4C8A30
DAC0/DA00): infected bit or primary presentation tick, style C690 reselection,
Euler impact reaction, and checked damage with primary capability emission, without
widening the fixed-actor whitelist. Lethal Type13 now enters the authored
Class1 BAC0 terminal through the shared [explosion owner](../../crates/v2k-game/src/class49_death.rs):
its G allocation selects ten class37 particles, static/dynamic radial damage
finishes before task clearing and deferred removal, and no Type60 ring is
created. Primary/infected/cured hits and the distinct11180 static-route entry
supply the complete live terminal frame. A failed radial retains the real
receipt and parks the owning prefix; a completed receipt permits later
same-frame hits without replaying death. Controlled actual-model regressions
verify both lethal hit entries and native terrain/static continuation.
Release visual regression now covers the complete normal-tier frontend,
Intro2, closing/opening iris and1024 Level1 frames. Fixed40ms/frontend100 and
varied16.667/33/40/125ms/frontend250 runs each retain18 captures with no runtime
issues. All18 pictures and actor/particle-count/camera traces match their
respective preceding-checkpoint controls exactly. The native hit and attached
callback source tests own branch coverage; these scene controls do not certify
matched retail cinematic acceptance.

## Bat projectiles and peasant damage

The `FUN_00442950` entity-hit family (classes 52/68/85) enters the model
sweep with descriptor-selected packets: `004CC0F0` (shared with class 87),
`004CC108` (`[2,6]/[2000,2000]`, recovered by static executable read and
cross-validated against seven documented rows), and `004CC048` (shared with
dragon class 38). Static contact consumes 52/68 through the `42E8E0` no-op and
delivers `F800` for 85. `FUN_00441180` delivery (stamp, pre-callback `+0x80`
sound, DAC0/C690, 11030 at eightfold 25590 force, result-blind 15040) runs for
native Type9 and Type26 in Intro2 and their owned ordinary authored worlds,
Type13 in Intro2, and [native fish](FISH_RUNTIME.md#particle-hits-and-quiet-death)
through their authenticated task/allocation owner.
The previous Intro2 router sent this family exclusively to Type13 and rejected
peasants before damage delivery. The shared native Type9 owner now applies
the distinct `11180` prefix, rather than reusing the ordinary `10EB0` wrapper.
Class68's packet filters to5600 against canonical Type9's health1500, publishing
the existing Class14 peasant death graph. Its living sound95 precedes death
sound35. The sibling class52/85 packets filter to2600/12100 through the same
owner; focused tests exercise all three in ordinary overlays13/14/15.

The final333-entry Intro2 replay also reaches Type26 spawn10/runtimeid11 at
tick2428 with a Type57 class68 shot. The old exclusive static-route dispatcher
rejects it before11180, although that native family already owns4C8A30's DAC0
and its C690/Class12 paths. The same Type26 owner now distinguishes11180 from
its primary/infected wrappers: alive+80 precedes DAC0,11030 receives8 * 25590,
and result-blind15040 has no post-hit cue or capability8 class5 suffix. The
active resource-cache record supplies cue and damage fields; local OVL row
numbers are not global entity ids. Its actual52/68/85 filters are0/200/4000.
Eightfold force applies to25590's raw sum before11030's mass quotient and
Q16/Q15 shifts. The actual mass400 control and2000 channel1/2 sum produce
primary delta79 and11180 delta639 along direction8192; multiplying the
already quantized79 by8 would incorrectly produce632.
Completed native task custody is required, including actual retained Class12;
a blocked C690 graph cannot lend another hit prefix. This generalizes only
through the existing genuine ordinary Type26 constructor/custody receipts.

`442950` runs `11180` even when particle suppression is set. The shared physical
slot traversal then executes F610 and, only if suppression is clear, allocates
the attached class83/84/86. The attachment reads the cached target record's
post-callback position and capability; admitted native wrappers retain that
allocation through the suffix. Capability1 uses zero Y and signed truncation toward
zero of three-quarter X/Z offsets. Damage callbacks and F610 can recycle the
particle record, so the suffix rereads the current record as retail does.
`425D0` follows the global owner handle until unlink, including dying actors
excluded from particle collision; using the filtered collision list previously
removed class84 immediately upon a peasant's death. Live rotated follow retains
the native Q31 basis, per-term signed-word narrowing and wrapping position sum.
A missing global handle sets particle byte+1D bit80; collision mode0 leaves the
marked record until the next visit's pre-update removal gate.

The accepted `20260722-022303-intro2-actor-ai.jsonl` contains thirty sampled
class68 generation starts from the Type57 bat handle04960001. At tick2058,
class84 first appears attached to Type9 handle04870001; that same sample changes
the target's health1500 to0 and stamps+34 with2058. Its authored anchor is
`[-19456,0,4352]` (spawn16, cells180/17). The attached particle remains linked
until the target's later unlink. The reproducible read-only join is
`summarize-intro2-bat-combat.py`.
These passive samples identify the emitter family and death result; they do
not identify the exact collision primitive or sampled caller instruction.

The exact-model regression collides a Type57 class68 shot with a native Intro2
`man2` model, checks damage/death and cue order, and retains the attached effect.
Shared traversal tests cover suppression, callback/F610/attachment ordering,
all three source classes and negative capability1 offsets. Successful focused
tests do not replace uninterrupted visual Intro2 comparison.

The shared descriptor surface owner now retains `442420`'s source-specific
replacement and same-visit E3D0 ground tail; subsequent mode0 visits intentionally
skip contacts. The [Type57 surface contract](INTRO2_TYPE57.md#projectile-surface-replacement)
owns its complete decision matrix. Type57 method24 is the known
emitter of this family (class68); 52/85 emitters are unaudited. Other target
families explicitly reject the unowned `11180` route rather than entering
their F590-style owners. Immediate target-record unlink/recycling by an unowned
wrapper would require cached physical entity-record custody across11180/F610;
the live-handle adapter does not invent that memory history. Those boundaries
limit full projectile-family fidelity.

### Attached callback and direct checked damage

The PE at `4425D0..442918` owns one callback for all three attached descriptors,
83/84/86, including direct scaled checked damage on native living actors. Both
Intro2 and Playing use their mutable world, task/death custody and real B2
writer. Playing also lends the actual player hull and synchronous death effects.
The actual postdeath Type9 class84 route above remains a genuine zero return:
Class14 clears bit8000 while retaining the global handle, and15040 at
41505D..415063 exits before profile lookup or any damage/death callback.
The separate442950 full-handle pose/capability
lookup still owns its earlier attachment-allocation offset; a collision-only
projection cannot authorize the later425D0 mutation.

After40120 increments age,425D0 stamps the descriptor packet's source/owner,
adds `(256 - particle_age_byte) / 4` to the owner's signed-word B2 with wrapping,
then performs direct or native Q31 follow. Only the rotated branch consumes one
shared RNG word, even at zero frame delta or under impact suppression. It calls
4417E0 when `low16_rng * 2 < elapsed_micros`. That helper compares signed raw Y
to the authored flat sea word, independently of displaced waves and the water
enablement gate: class31 above, class42 at-or-below. It supplies zero velocity,
current process emitter provenance and the parent's suppression bit to40A60;
the descriptor's birth biases and allocator rejection remain authoritative.
An unavailable static sea plane explicitly blocks a requested effect rather
than guessing class31.

All three attached descriptors have a nonzero model word at+04.40A60 at440D5C
therefore sets particle+1D bit40, and40120 at4401E4 skips generic integration:
the +E/+10/+12 words remain follow offsets, not an independently moving velocity.
Water classification still occurs before425D0's follow at the retained position.

The attached birth's initial source-type byte1C remains unresolved in the older
allocation request.425D0's first transient packet stamp copies that byte, but
4417E0 does not consume the packet and there is no callback before425D0 replaces
both provenance words at4427FF/44280F. Thus it cannot affect this clear-bit
callback result. Keep the missing birth byte explicit; completing provenance
for a future consumer requires a genuine target-type lookup, not an invented
type or captured constant. The same evidence bound applies to enabled damage.

The callback next marks bit80 when the classification passed in by40120 was
fully underwater. This classification precedes follow; the mark does not skip
the remaining phases.425D0 then overwrites packet source with signed-5 and
owner with the current process emitter, and requests15040 with denominator
descriptor lifetime255. A positive target+50 **pre-health buffer** selects
numerator `lifetime - current_age_byte`; otherwise it selects the elapsed-tick
low word. Target health is+30, not this branch field.255E0 at4255E0 masks the
numerator to16 bits, multiplies filtered damage with signed32 wrapping and
divides by the unsigned16 denominator. The live core applies that ratio after
the authored filter, before the optional+44 modifier and14E90's buffer, dying,
generic-hit cue, type-vtable+30, health and10C10 phases. This entry never calls
11180/10EB0, style C690,11030 or radial impulse/falloff, and never stamps+34.
The six-dword packets remain the descriptor rows4CC0F0/4CC108/4CC048; the
request replaces only their provenance words with signed-5/current emitter.
Source-5 cannot enter either filtered-zero or kill player-feedback branch.
Native allocation/task custody precedes actual mutation, and completed death
publications are adopted synchronously. Playing class49 callbacks lend their
actual player hull, campaign extra lives and notification slot to the nested
radial visit. Its player15040/14E10 continuation synchronously completes the
authored class25 initializer, immediate475F0 burst, CA/D9 notification and
controller208 RNG seed before the attached callback resumes. Full-radius
radial entry retains Checked15040; falloff retains Unchecked14E10, including
255E0 filtering and the original packet provenance. Their outer admission,
optional word impulse, already-dying buffer-only path and cue order remain
radial policies. Direct attached player entry carries its explicit ratio
through Checked15040 and never acquires radial falloff or impulse. Missing
player hull/lives in a cinematic or detached context is an explicit boundary.
14E10 preserves packet+10 but passes process DAT_004F7378 as14E90's fourth
argument. The admitted empty player modifier/null-hit/local-death path does
not consume that argument; the complete packet remains intact, without
inventing the process emitter. A future owner-dependent modifier or network
continuation must authenticate that global rather than substitute packet+14.
The clear admission gate performs none of that filtering. A positive pre-call
buffer still writes age=lifetime after15040 returns, even on zero, regardless
of what that call did to the buffer. Expiry waits for the next visit.

Finally, if suppression is clear and
`((u32(age_byte) - elapsed_ticks) ^ u32(age_byte)) & 0xFFFFFFF0` is nonzero,
425D0 consumes a further shared RNG word. An odd word attempts one cascade:
83→53,84→69,otherwise→39. The source-local descriptor stays cached across damage,
but442855..4428C2 rereads the original physical particle slot's current age,
suppression, class, position and+14 child owner, including a recycled record.
The inherited velocity instead comes from the original cached EDI allocation
AFTER callbacks: X/Z divide signed velocity by2 toward zero, and Y uses `min(VY,0)`.
That allocation must still match its authentic lease; dying/deferred actors
remain addressable until14990.40A60 independently resolves the current+14
owner's birth type, so a replaced particle cannot inherit EDI's old handle.
40A60 then applies the authored Y-velocity bias: -50 for53/69, zero for39.
The subtraction is a wrapping dword, not a previous age byte. Allocation may
preempt a lower-priority physical victim, but these children cannot recycle
the attached parent:31/42 have priority2,53/39 priority4,69 priority5, while
83/84/86 have priority6. No stale parent write follows the attempt. Earlier
child slots wait until the next traversal; later slots
receive the current pass. An allocation failure still consumed the RNG gate.

Unowned modifier, generic-hit, death, remote, player linkage or task-custody
paths retain a named checked-damage diagnostic and their exact committed prefix.
Unresolved admission is `CheckedDamageAdmissionUnavailable`; unavailable
mutable owner/B2/basis/buffer/static sea remains explicit. A reached cascade
cannot use an unlinked or rebound cached allocation: `CachedOwnerAllocationMissing`
and `CachedOwnerAllocationRebound` block that particle instead of freezing
pre-follow velocity. A physically freed particle is
`PhysicalSlotFreedDuringDamage`: retail442B40 preserves most free-record bytes,
but the port's Option slot does not retain that memory image, so it cannot
reconstruct or pretend to execute the freed suffix. Each block logs and retires
only this particle; B2/follow/damage/death effects already committed never replay.
These are per-feature boundaries, not a whole-world stop.

The96-row PE descriptor table's priority6 classes are83/84/86/91/92. Current
owned death callbacks emit lower-priority scatter/bursts (including the native
player's class16/18 initializer burst) or publish tasks;
they do not execute442950 attachment creation or442DB0 weapon emission during
this direct15040 call. The controlled reentrant replacement test uses the real
intrusive allocator with an isolated priority6 class91 record, then a separate
cleanup opens the cascade allocation. It proves current-record custody and
cached-descriptor age clamping, not native class91 death emission or that
model particle's unowned birth/draw path.

Focused source controls cover the three-class packet/cascade matrix, age/B2
ordering and wrap, rotated-only RNG, flat-sea equality with water disabled,
pre-follow underwater marking, buffer clamp despite disabled damage,
suppression, failed allocation, parent protection/lower-priority victim order,
next-visit removal and
blocked-prefix retirement. Enabled controls use all three authored packets on
actual native Type9 allocations; all three ratio-zero packets preserve the real
Type26 buffer/graph while completing the age write. An actual native Type26 class12 death changes
Y velocity from-203 to500 before a class86→39 cascade; the child inherits0,
making a stale velocity snapshot fail. Unlink/rebind and physically freed
records retain exact-prefix diagnostics. Actual class68 model-hit regressions
publish both living and dying Type9 class84 attachments and execute them through
both production hosts; B2 remains available to the next actor callback's mass
read. A native turret's attached damage also completes its class49 radial
death and a real player's immediate wreck burst/CA notification before return.
These source-backed callbacks do not establish matched retail cinematic
acceptance or ownership of every target's remaining callback.

## Evidence limits and comparison requirements

The completed actor-AI protocol samples the full particle pool and global
lifecycle, but its deep task/context filter includes only Types13/26/47.
Type10 and Type102 installed styles, physical poses and source-owned shots
are visible; their target-context memory and exact Aim callback invocation
are not. Do not infer a unique target handle solely from an installed style
or a projectile direction. The accepted task/mover debugger survey also does
not establish unperturbed firing cadence.

`20260716-165213-intro2-particles.jsonl` ends at tick1303. It remains authority
for early meteor/hive effects, but cannot answer late dragon/turret combat.
All named files are retained through the
accepted-capture ledger.
The existing `<install>\V200001.run` contains the complete Intro2 if a
specific unresolved target, materializer or hit callback needs a narrow
read-only query. Process-local handles from a passive recording must never be
transferred into that replay without authenticating its own allocations.

Compare launch position/direction, class-specific motion and lifetime,
collision admission, source-owned impact delivery and terminal ordering
before attributing missing natural combat to target selection. Source-proven
unit tests and controlled terminal fixtures establish their own bounded
contracts; acceptance still requires the uninterrupted production scene.
