# Intro2 Type92/102 defensive gun turrets

This document owns the native allocation and dedicated Gun Turret task used by
Intro2 Type92 spawn52 and Type102 spawns53/54. The historical filename remains
stable for inbound links. The implementation is now
[`intro2_gun_turret`](../../crates/v2k-game/src/intro2_gun_turret.rs), with an
explicit [`Intro2GunTurretProfile`](../../crates/v2k-game/src/intro2_gun_turret/profile.rs)
for each immutable type row. General task custody, shared RNG, and callback-mode
rules remain in [ACTOR_RUNTIME.md](ACTOR_RUNTIME.md); damage entry ordering and
standard death remain in [ENTITY_DAMAGE_AND_DEATH.md](ENTITY_DAMAGE_AND_DEATH.md).
The attacking dragons are covered by [INTRO2_TYPE10.md](INTRO2_TYPE10.md).
The [Type115 virus flower](TYPE115_VIRUS_FLOWER.md) shares E/L and the class29
task, with its own mixed model slots, weighted class0 alternative, method16
gravity lead and class50 contacts. Its alternate class1 shares BAF0's blast
prefix with class49, then removes the source without constructing a ring;
the shared death owner retains that explicit terminal policy.

## Canonical allocation

Normal tier1 Intro2, level50, provides two immutable profiles. They share the
native constructor and class29 task; their model, weapon and damage fields are
not interchangeable:

| Field | Type92 | Type102 |
|---|---|---|
| Authored spawns / raw XZ | 52 at `[0x5800,0xE400]` | 53 at `[0x6400,0xF500]`; 54 at `[0x6800,0xEB00]` |
| Four model slots / header extents | global171 `tulazred`; `+8=125`, `+A=130` | global162; `+8=275`, `+A=253` |
| Health / capabilities | `5000 / 0x1044` | `4000 / 0x44` |
| Axis descriptor | `[5120,0x100B]` | `[5120,0x000B]` |
| Animation-bank words, type `+110` | 2 | 3 |
| Damage thresholds | `[0,2000,1800,200,200,200,0]` | `[0,8000,2100,200,200,200,0]` |
| E interval / speed override / axis tolerance | `400000µs / 4000 / 3840` | `300000µs / 5000 / 6400` |
| E muzzle `+12` / alternate `+14` | `18 / 0` | `46 / 48` |
| E binding bytes `+18..+1B` | `[0,0,0,0]` | `[0,3,0,0]` |

Both have spawn parameter1, zero authored Y/Euler words, mass100, default
policy `+C8=0x25025`, and E/L only: no A/B/C/D/F/G/H/I/J/K/M/N/O. Their initial
selection is singleton Always1, weight1, class29, with direct alternate49.
Damage Q8 multipliers are `[0,256,256,256,256,256,0]`; surface selectors,
lifetime and low-health effects are `[0,0]`, zero and `[0,0,0]`.

All constructor, accepted-hit, infected-presentation, generic-hit, target-warning,
and death sound fields are null. The E emission sound is independently `78`.
None of these actors has authored animation, model overrides, extra parameters, a
damage buffer, or a factory configuration. Native authentication binds the
actual allocation and authored index, independently of its later position or
behavior style. No Sub-D allocation or first-consumer capture is involved.

`09A80` zeroes the type's two- or three-word animation bank, allocates L through `1BB80`,
installs the common axis, then allocates E through `24E30`. L's descriptor is
`[1,2,0x40,0x1F,0x40,0x1F]`: yaw/pitch outputs bind selectors1/2 and both limits
are8000. E's four descriptor bytes map in the actual constructor
order `+18→runtime+08`, `+19→runtime+00`, `+1A→runtime+04`,
`+1B→runtime+0C`. Thus Type102's `[0,3,0,0]` binds its first joint to selector3,
with the other bindings null. Type92's four bytes are zero, so every E binding
is null and no selector3 word is allocated. Descriptor word `+12` is a model
muzzle slot in both profiles, not an animation binding.

The shared E fields are method14, spread512, aim threshold4000, sound78,
stochastic mode0, and auxiliary command0; the profile table owns interval,
speed, tolerance and muzzle differences. `24E30` zeroes all0x3C runtime bytes
and copies method/sound; the allocation initially has no cadence, manual budget,
selector, or extra basis adjustment. These component allocations and their
animation words survive task reselection.

`D4A0` default bit0x20 grounds before `AC60`; bit0x40 and Sub-C clearance are
absent. Canonical Type102 land cells yield native positions `[25600,192,-2816]`
and `[26624,288,-5376]`. Both profiles retain the body basis built by the
common constructor's `413F70` Euler-to-Q31 writer from the authored zero
angles. Native E/L publication does not rebuild that basis from heading or
substitute turret model yaw/pitch for it.
The port's explicit native policy supplies zero once for104B0's unwritten B2
heap residue, as for the other native cohorts; later contributions are retained.

## Ordinary native Type97

The singleton Class29 blue turret has eight canonical tier1 authored births:

| World | Spawn indices | Spawn param | Authored Euler differences |
| --- | --- | --- | --- |
| 31 | 41 | 0 | all zero |
| 42 | 17, 18, 19, 20 | 1 | heading32768 at18/19, others zero |
| 46 | 17, 18 | 1 | all zero |
| 47 | 21 | 0 | all zero |

All eight have zero initial damage buffer/model overrides and no animation or
configuration override. World46 entity19 is spawn18, with authored raw position
`[18944,0,4864]`. Its collision with native Type122 exposed the missing ordinary
constructor: the former generic body had an unresolved current behavior context,
so D780 correctly stopped before inventing a null callback.

Type97's invariant metadata is model173 `tulazblu`, mass100, capability1044,
health5000, default policy25025, E/L components, axis5120/filter100B, singleton
Always1/Class29 and alternate49. Its two model-variable words belong to L;
there is no A/B/C/D/H/I/J component. Native publication now retains the actual
authored allocation, Euler basis, pre-grounding surface classification and
post-D4A0 grounded position. AC60 consumes one selector word; E/L and D190
consume none. The current Class29 context and Tertiary wrapper are published
before the next authored body is exposed.

The ordinary receipt contains its issuing manager's allocation lease. Live,
Aim, drain and external contact ownership reject a foreign manager or replaced
body before mutations. The FIFO retains that same immutable construction origin,
so transplanting only a queue between equal public IDs cannot cross allocations.
Intro2's proven92/102 identities retain their existing
origin, while campaign zero-record/at-player-pose E/L reconstruction remains a
separate cargo origin; it does not authorize an ordinary living turret.

The shared pair adapter requires the completed current turret owner before
physical writes. Class29 style4C8230 has a null pair slot at+28;4086D0 explicitly
writes null task+18 at408721. The adapter therefore authenticates the actual
Tertiary GunTurret task before accepting that null contact. It does not install
a missing task or replace the behavior context with a null context.

Type97's E descriptor uses method12, interval400000us, spread1024,
threshold4000, explicit speed4000, tolerance3840, sound76, muzzle slot18, and no
alternate emitter or variable bindings. The executable row at4D0380 is
`{leading1,speed6000,particle49,entity0}`. The44EA60 selector byte for method12
chooses the zero-gravity-lead return, as for method14. Shared425160 now accepts
this authenticated descriptor and the FIFO drain uses its own method row and
explicit4000 speed, including the actual173 model hierarchy and retained body
basis for detailed muzzle callbacks. No asset/parser or presentation transform
changes are involved.
410B0 retains class49 above the flat sea plane and selects class80 at or below
it. Their update/surface callbacks and own4CC078 impact packet are covered in
[combat projectile ownership](INTRO2_COMBAT_PROJECTILES.md).

The two world46 births stand on terrain type94. D4A0 copies terrain bit10
into live state2000 before D190;408EA8 therefore changes their live capability
from1044 to48 at birth. The other six births stand on terrain82 and retain1044.
Later infection can reach the same capability edit through C690/D190.
41483C tests capability8 and state04000000 after a successful append, calls
1B9D0 (no Sub-G in this allocation), then4568B0(9). The scheduler now supplies
an explicit `Playing`/`NonGameplay` notification phase through the turret
callback and Aim frames. Playing queues resource event9/stringEA in the existing
session notification owner; deduplication retains its first timestamp across
catch-up shots and later frames. NonGameplay suppresses the hint without
changing append, cadence, RNG, selector or sound behavior.

The pair adapter's lethal damage continuation now owns the explicit Playing
world/player-hull context required by the turret's Class49 death: Playing
active-pair passes deliver lethal hits through `run_class49_standard_death`
with that context, while cinematic passes without it retain the fail-closed
`MissingPlayingContext` boundary with the committed health prefix preserved.
This does not change the separately owned primary/infected/radial hit and
Main Base abort paths below.

The canonical constructor, living E/L, detailed muzzle/FIFO, changed-metadata,
manager-generation and authored-infection/notification controls live in
[`authored_tests.rs`](../../crates/v2k-game/src/intro2_gun_turret/authored_tests.rs).
They complement the existing Intro2 turret and campaign-cargo controls.

## Ordinary native Type92, Type99 and Type102

The class-29 E/L family shares Type97's Section-12 shape: E and L only,
singleton Always1/Class29, alternate49, rule1, mass100, initializer state
`0x25025`, and no A..D, H, J, N or sound attachments. Rows differ only in data:

| Type | Model | Health | Capability | Model variables | E method / sound | Ordinary births |
|---|---|---|---|---|---|---|
| 92 | 171 | 5000 | `0x1044` | 2 | 14 / 78 | worlds 26, 38, 39 (3) |
| 97 | 173 | 5000 | `0x1044` | 2 | 12 / 76 | worlds 31, 42, 46, 47 (8) |
| 99 | 165 | 5000 | `0x44` | 3 | 12 / 76 | worlds 26, 32, 46, 48 (10) |
| 102 | 162 | 4000 | `0x44` | 3 | 14 / 78 | worlds 19, 25, 37, 39, 40 (9) |
| 96 | 175 | 5000 | `0x1044` | 2 | 13 / 77 | worlds 26, 36 (2), not yet ported |
| 100 | 152 | 32000 | `0x1044` | 3 | 19 / 7 | world 26 (1), not yet ported |
| 103 | 168 | 5000 | `0x44` | 3 | 13 / 77 | worlds 21, 25, 29, 31, 38, 40, 43 (15), not yet ported |

Ordinary 92/99/102 now take the same `104B0 -> D4A0/D190` birth, metadata
authentication (including each row's own model-variable count), class-29
owner, method-12/14 aim and FIFO drain, pair damage and class49 dispatch as
Type97. Ownership is decided by construction origin, not type number: Intro2's
92/102 identities keep their cinematic owners and campaign reconstruction stays
separate. Methods 13 and 19 have no live aim owner, so 96, 100 and 103 still
construct without a context.

## Ordinary native Type104 and Type115

Castle (global world15) authors anti-air Type104 at spawns1/49/50 and virus
flowers Type115 at51/52. Both use the ordinary E/L allocation request and retain
their actual manager lease and authored pose; an Intro2 spawn index is not an
admission key. The ordinary publisher authenticates each immutable profile and
rejects animation, configuration and model overrides. Type115 retains its
weighted class29/class0 selection and alternate1 policy.

Type104's Section12 row has model156 in all slots, two model-variable words,
mass100, health6000, capability44, default state25025, axis `[5120,0x100B]`,
singleton Always1/class29 and alternate49. Its damage thresholds are
`[0,6000,2100,200,2000,200,0]`, with Q8 multipliers
`[0,256,256,128,256,256,256]`. E uses method18, interval400000µs, spread512,
aim threshold4000, speed2000, tolerance2560, sound88, muzzle16, no alternate,
auxiliary command1 and four zero binding bytes. L is `[1,2,8000,8000]`.

The NoCD05 read-only allocation at `2E2352:1307` resolves Type104's vtable to
`4C8A30`: constructor `D4A0`, standard-death callback `DB80` and null type-hit
slot `+30`. Class29 style `4C8230` has null death cleanup; its initializer
`D190` clears slots1/0 before installing the shared turret task. Consequently
world loss must invoke the native alternate49/BD20 owner, not a quiet-death
substitute. [Campaign failure](CAMPAIGN_FAILURE.md) retains the shared abort
walk and completed-owner boundary.

The [Castle constructor tests](../../crates/v2k-game/src/intro2_gun_turret/castle_tests.rs)
check the three AA and two flower allocations, exact profile metadata, foreign
manager rejection, and the authored-row contract across all tier1 campaign
worlds. The ordinary receipt also prevents a coincident Intro2 index from
imposing Intro2's zero-Euler restriction on a different world.

## Initial graph and reentry

The class29 descriptor is `0x004C8928`, style `0x004C8230`, initializer
`0x0040D190`. Its style enable policy0x4027 combines with the default policy to
give effective0x25027. `AC60→425680` consumes one shared RNG word even for this
singleton list. None of the E/L or turret task constructors consumes a word.

`D190` clears Secondary, then Primary, and calls `4086D0(handle,Tertiary)`.
The wrapper uses initializer408DF0, body408770, destructor4086A0, and lifetime0.
The new wrapper/private allocation is prepared before A7A0 destroys the old
Tertiary; executing wrappers retain normal deferred outer reclamation. Successful
installation ORs state0x28. Destructor4086A0 only frees the private allocation and
clears the wrapper pointer; it does not restore capabilities or clear state0x100.

`408DF0` zeroes a0x2C private allocation and then publishes:

| Private offset | Initial value |
|---|---|
| `+00,+04` target / target age | zero |
| `+08,+0C` yaw / pitch | negated signed L selector1 / signed L selector2 |
| `+10,+14` selection timer / tracking-disabled | zero |
| `+18` gravity lead | `44EA60(E.method)`, zero for method14 |
| `+1C` original capabilities | current live `entity+64` before constructor edits |
| `+20` lost-target timer | 6000000µs |
| `+24` normal priorities | `4C7098` when captured capability8 is clear; otherwise `4C70A8` |
| `+28` infected priorities | `4C70A8` |

At `408EA0`, existing state0x2000 makes live capabilities
`(capabilities & ~0x1004) | 8`; then the constructor ORs state0x100. The private
original-capability word and priority table still describe the pre-edit value.
Native reentry preserves the behavior context's target/auxiliary and allocated
E/L state, consumes its own single selector word, and replaces only the graph.
The live caller owns task-result suppression and pending-prefix admission.

The static style matrix has `+00/+04=C690`, `+0C=D1C0`,
`+20/+24=C690`, and null `+28/+2C`. `D1C0` invokes16AC0 before AC60. Primary
and infected damage therefore have different callback entries; they cannot be
collapsed to the same impact helper. Alternate class49 is a distinct death
continuation, not Common-Dying class12.

## Activation and dedicated task

The turrets are active from their native allocation; Type102's Section2 records
only select the camera at50s and56s. The dragons receive EnableComponents at50s.
Native turret code must consume retained entity flags and actual delivered
callback time, without activating itself from the presentation clock.

The implemented gun task follows `408770/408BD0` target selection and tracking,
with manual emission through425160. It has no shared `03490/02300` Chase/Aim
graph. Selection preserves native intrusive target order, relation exclusions,
capability priorities and current-target hysteresis. Normal priorities are
`[0x20000,0x10000,0]`; infected priorities are `[0,0x30000,0x20000]`.
Constructor-captured capability8 also selects the infected table when the
current state0x2000 bit is clear. The distance used by425370 is
`max(abs(X),abs(Y),abs(Z)) + ((sum of the other two) >> 1)`; current-target
distance is penalized by `*6 >> 2`. The later normalized-angle calculation
independently uses457730's integer square root.

The signed selection timer uses strict `< callback_dt`; the unsigned lost-target
timer uses `<= callback_dt`. Expiry returns4BE250 before Aim or L output writes.
The wrapper retains its age and private prefix, unwinds the exact visit on
success/error/panic, and only a surviving wrapper may request reselection.
Native owner custody authenticates allocation, current context and all slots;
failed callbacks remain pending without repeating time, selection or RNG.

Both callback modes use12DA0 with the actual carried callback delta. The effective
0x25027 world tail retains the body basis and omits E640/F70 and E100 movement
forces, then performs DF70 grounding and the admitted zero-selector E370 timer
phase. Task yaw/pitch move L's model bindings, not the actor body matrix.

## Manual firing and model-owned muzzle drain

425160 advances E cadence using the global host delta, even when the task has
no firing direction. A null direction consumes no RNG. A real direction consumes
one stochastic word; acceptance does not add random spread words. The loop emits
while cadence is nonpositive, accumulating the descriptor's interval as each
command offset. The native `4252EA` branch toggles selectors only when E
descriptor `+14` is nonzero. Type102 therefore alternates0/1 at300000µs;
selector1 writes its native selector3 joint binding through24EE0. Type92 keeps
selector0 for every400000µs catch-up command and has no joint write because
both runtime joint pointers are null. Sound78 is submitted once after the loop.
E cadence, output words and the
allocation-owned FIFO survive task replacement.

Method14's4D02C0 row is `{leading=1, speed=5000, class=55, trailing=0}`.
The queued command retains the explicit descriptor speed override:4000 for
Type92 and5000 for Type102; it does not silently replace Type92's speed with
the method row's default.
11400 applies each command's source-velocity rewind and directional time advance;
4E770 then applies a positive closing-speed boost, without adding the source
velocity vector to the projectile. Each accepted command reaches the existing
40A60 descriptor constructor in FIFO order. Even rejected particle births consume
their command; a preflight failure leaves the source FIFO and RNG intact.

The Section12 loader installs callback pair4C8A28 (`40D320/40D350`) in both
types' `+78` fields. Dynamic selector 0 returns the global 50-Hz tick;
nonzero selectors use 40A950's retained bank. The clock drives the authored
plasma glows documented in
[PLASMA_WEAPON_PRESENTATION.md](PLASMA_WEAPON_PRESENTATION.md).
External-frame callbacks go through40A9F0. With native H absent
and E present, external indices select424F20. It resolves descriptor slot18
for Type92's only emitter, or slots46/48 for Type102, through424FA0 in the
**current nested model context**, then
stamps every queued command with that selector. The last reached callback wins.
40D350 wraps the resulting XYZ relative to the context origin in signed-word
coordinates. No menu/world reflection is part of this callback.

Type92 global171 embeds172 at authored root slot12, offset `[0,90,0]`.
Its muzzle remains a model-context callback using descriptor slot18; the actor
centre is not a substitute for a required detailed callback.

Canonical Type102 global162 embeds163 at its yaw mount;163 embeds two164 barrel
instances at pitch mounts, with register3 selecting the muzzle face. The actual
164 records42/44 are TF14 callback operands0/1, and46/48 contain muzzle points.
The C6 depth group also resolves44, including in the first barrel; the second
barrel later supplies selector1's final stamp. The intrinsic painter program now
retains raw group slot references alongside resolved depth keys, so this callback
cannot be inferred from coincident coordinates or delayed until queue sorting.
The decoder's existing depth/render behavior is unchanged. By contrast, opcode38
is4689D0 vertex-number diagnostics: normal4FEEF4=0 skips slot resolution entirely.
See [FORMAT_DOCUMENTATION.md](FORMAT_DOCUMENTATION.md) for the intrinsic format.

Offline4147A0 initializes every command's detail marker to1. After the host
publishes current presentation flags,11400 forces13BE0's model traversal only
when current0x02000000 lowers the marker threshold from2 to1. Without a model
callback the marker1 command explicitly uses the current actor position; this
branch does not require a model or body-basis read. Required detailed model
resolution failures remain explicit errors. The native drain uses the real
model hierarchy, mount registers and retained Q31 body basis. Intrinsic mount
rotation currently uses the formats API's floating-point trigonometry, so exact
671F0 table quantization and per-mount integer rounding remain a precision
boundary. This implementation does not claim bit-identical muzzle coordinates.

## Damage, class49 and synchronous terminal custody

Primary10EB0 writes the presentation-hit tick before its null class29+28 hook;
infected11250 publishes the model-bit prefix and dispatches C690 at+20 without
writing that tick. Both then perform shared11030 reaction and checked415040
damage using the full delivery record. Surviving infected reselection installs
the new owner synchronously and retains E/L/FIFO state. Only primary nonzero
filtered damage runs the accepted presentation suffix: the test is `!= 0`,
including negative filtered returns and positive damage fully absorbed by a
buffer. Accepted-hit sound is skipped while dying; the independent capability8
particle branch still reads current flags/model after standard death. A previously infected
turret has live capability0x48, so its later primary hit can emit capability8's
class5 suffix even when standard death has just completed.

Incoming Type13 Aim now retains an actually acquired turret target.
`402355..40236B` checks a resolved, non-dying handle without a target-type test;
`424650` reads that allocation's raw position and velocity for lead. The former
Type9-only host restriction was therefore unrelated to the source Aim consumer.
The same handle/dying and custody checks remain, with peasant controls retained;
this does not change the upstream class7 target-selection policy.

10C10 sets health0/dying, then directly selects alternate49 without a weighted
RNG draw. BD20 invokes BAF0 before clearing tasks. Native E/L has none of the
earlier A/B/N/G scatter branches, so capability0x40 selects sixteen class94/95
particles and the shared surface/sound tail. The radial template is
inner512/outer1024, impulse2000, channels `[1,3]`, amounts `[4000,4000]`;
logical owner/type come from416F90's relation resolution, falling back to self.
The shared implementation authenticates the source's own profile and reads its
actual type header and selected model extent. Type92 therefore retains its
health5000/damage thresholds, model171 burst extent and logical type92 identity;
sharing class29/49 does not route it through Type102's immutable row.

The terminal receipt is claimed once, runs static then dynamic radial damage
synchronously, and preserves later failure prefixes. Only after successful
radial completion does A860 clear slots. BD20 rereads current position and the
remote gate, constructs the native Type60 ring when local, and requests deferred
source removal. Ring allocation failure is consumed without replaying BAF0 or
radial damage. Nested terminal calls retain their real source allocations and
the scheduler adopts resulting owners at the same source phase.

The completed source remains a valid particle target until the deferred sweep.
`410B70` clears `0x60000` and sets `0x100000`, leaving `0x8000` intact;
`43FA3D..43FA51` tests `0x8000/0x800/0x1000`, without a deferred-removal
exclusion. A later physical particle can therefore hit the finished class49
allocation. Its current style `4C71E0` has null `+20/+28` hooks: primary still
writes the hit tick, while infected still sets `0x2000` and skips its sound
because the source is dying. Neither entry reruns the former class29 selector.
Both retain `11030 -> 415040`; filtering and buffer consumption precede the
dying-health no-op, and the primary capability8 presentation suffix remains
independent. No second blast, radial pass, ring or removal request is issued.
The port admits this path only with the finished terminal receipt's actual
manager allocation, class49 context, empty task slots and exact deferred state.
Issued, claimed, partially finished, foreign and parked receipts remain blocked.
Focused repeat-hit controls cover both Type92 and Type102; validation of this
correction is pending the shared test and rendered-scene gates.

The local Class49 tail retains Type60's default model243 in all four slots.
It publishes the same class48 `406DC0` lifetime callback used by hard-water
rings, while preserving the latter's model130/132 constructor overrides. Its
admission requires the retained constructor provenance and matching allocation,
task and presentation state. The callback decrements the native control word;
zero remains alive and a strictly negative result selects class2 teardown,
without callback RNG. See [FACTORY_SYSTEM.md](FACTORY_SYSTEM.md) for the shared
Type60 constructor, lifetime and publication authority.

## Ordinary Type97 damage and terminal lifetime

Fresh ordinary Type97 uses the same class29 callback slots with its own
model173, health5000 and damage row. Primary10EB0 keeps the current Tertiary
task and live capabilities, including `0x48` on already infected authored
births. Infected11250 runs C690 once, replaces that task and applies
`(capabilities & ~0x1004) | 8`, while preserving its allocated E/L state.
Channel6's authored multiplier is zero, so that infected delivery performs
reselection without health loss. A retained ordinary construction receipt must
match the current manager allocation before either entry writes its prefix.

Playing particle dispatch uses
[`PlayingActorImpactFrame`](../../crates/v2k-game/src/shared_actor_impact.rs)
to carry the actual mutable resource cache, static-damage scheduler and player
hull. The turret adapter chooses the existing Playing class49 continuation;
Intro2 retains its explicit cinematic continuation. A lethal hit therefore
finishes the static/dynamic blast, any nested native turret deaths, Type60 ring
and deferred-removal request synchronously. Re-hits require the completed
terminal receipt and cannot repeat the explosion; an accepted primary re-hit
still emits the independent class5 suffix when live capability8 is set.
Radial admission and terminal
publication independently check the ordinary allocation lease, so another
manager's otherwise matching Type97 cannot borrow this lifetime.

The Main Base abort enters this same class49 publisher through the abort's
world-effects owner, retaining its real Playing authorities and post-callback
successor. That enclosing abort is speculative: a later failure rolls back its
terrain, static scheduler, player hull, actors, tasks, RNG and notifications;
a successful sweep commits them together. A direct particle/radial failure
instead retains and parks its already committed terminal prefix.

The class29 hit-slot proof does not cover `411180`: particle classes52/68/85
remain blocked before timestamp, task or RNG writes. Nonlethal active-pair
checked damage retains the authenticated actor; lethal pair delivery completes
through the Playing Class49 owner with the explicit world/player context, and
fails closed as `MissingPlayingContext` without it, preserving the committed
health prefix. Neither case falls through an invented
primary callback or Common-Dying class12 publication. Focused controls use
real native births in worlds31/42/46/47 for primary/infected, terminal re-hit,
nested radial, abort and foreign-manager rejection, plus world46 Type122/Type97
Playing pair controls for lethal Class49 with ring/deferred removal and the
cinematic missing-context boundary.

## Passive retail Type92 destruction

The accepted `20260722-022303-intro2-actor-ai.jsonl` capture contains eleven
Type92 lifecycle records for handle `0x04630001`. Every record retains model171,
position `[22528,32,-7168]`, zero velocity and the common zero-Euler Q31 basis:
columns `[0,0,-2147352576]`, `[0,2147352576,0]`, `[2147352576,0,0]`. The
visible body stays fixed while the dedicated E/L model task owns aiming.

Birth at elapsed `9880.1475ms` (line1077) has health5000 and Gun Turret style
`0x004C8230`. The last sampled health5000 is at `67480.1729ms` (line91998).
At `76900.2216ms` (line114677), health is zero and style is `0x004C71E0`, the
class49 explosion continuation; `death_or_unlink` follows at `76910.2611ms`
(line114751). This establishes actual Type92 destruction during retail Intro2,
before handoff. The capture's deep filter selected types13/26/47, so it does
not establish a particular Type92 callback execution, exact death instant,
attacker or firing cadence. Capture scope and accepted filenames remain in
the capture ledger.

## Validation boundary

Native corpus and focused tests cover all three native profiles, component/RNG retention,
task timing and failure custody, targeting/firing, primary/infected delivery,
and class49/radial/ring continuation. Type102 controls retain both model barrels
with changed mount angles. Type92 controls distinguish its own allocation from
the Type102 profile, preserve the body across detailed/coarse callbacks, exercise
400ms single-muzzle catch-up with null joint bindings and model171→172 slot18,
and carry its own damage threshold through lethal class49 into the real Type60
lifetime. Two class55 births produce twenty D300 streak samples; presentation
sample count is not a second particle-birth count.

Drain controls cover source-order group stamps, graph replacement, offscreen
marker behavior, malformed queues, required-model failures and rejected births.
Formats tests preserve distinct raw slot references through dynamic branches
while keeping existing depth policies. Shared implementation changes preserve
the Type102 decision matrix; matching function addresses alone do not replace
the explicit profile differences above.

Current 640x480 OpenGL walkthroughs cover New Game, the full story, final black
card, closing Klaus and first-world reveal with actual descriptor-owned
projectiles. Fixed40ms/frontend100 and varied-frame/frontend250 runs both
destroy all three turrets and then the factory through natural combat, retain
both dragons and complete the Type60 tails. Neither Intro2 run reports a runtime
issue. The wider ordinary-world control status belongs to
Intro2 acceptance; Type57 hit behavior
under other process histories remains outside this turret owner. Exact comparison
ticks and remaining visual boundaries belong to
[Intro2 combat projectiles](INTRO2_COMBAT_PROJECTILES.md#production-scene-comparison).
These observations establish live causes and continuation, without claiming
retail-identical timing under different random-stream histories.

The shared full terrain/water/static/active-pair actor contact walk remains a
separate boundary. Multiplayer command-marker/network behavior and arbitrary
external-frame generators outside the authenticated E/L hierarchy are not
admitted by this offline native owner.
