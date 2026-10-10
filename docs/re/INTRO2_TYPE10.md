# Native Intro2 dragons

This document owns the Type10 actors at authored spawns 55 and 56: native
construction, Search and Attack, fire emission, and Tumble To Death. Ordinary
Type5 rows share the same owner through a row profile (see the last section). Shared
task dispatch and component arithmetic remain in [ACTOR_RUNTIME.md](ACTOR_RUNTIME.md);
damage ownership remains in [ENTITY_DAMAGE_AND_DEATH.md](ENTITY_DAMAGE_AND_DEATH.md).
The factory's native damage and lifecycle are described in
[INTRO2_TYPE66.md](INTRO2_TYPE66.md).
Whole-scene validation belongs to objective 08.

## Authored data and construction

Both actors use model 351 in all four model slots, mass 100, capability 8,
health 32000, default flags 8, and the D/E/G/K/L topology. There is no
Sub-A/B/C/F/H/I/J/M/N/O allocation. Section12 supplies axis `[7680, 0xC85]`,
one Always choice of weight 1 for class7, and alternate class11. Every
constructor attachment, accepted-hit, generic-hit and death cue is null.

| Authored spawn | Raw XYZ | Heading/pitch/roll | Own Sub-D seed |
|---|---|---|---|
| 55 | `[0x7300, 0x0700, 0x0900]` | `[0xC000, 0, 0]` | `0x22` |
| 56 | `[0x5A00, 0x0600, 0x0800]` | `[0xC000, 0, 0]` | `0x23` |

`104B0 -> 09A80` allocates and clears the eleven-word animation bank, then
constructs G (`1B8C0`, one shared random word), D (`203D0`), K (`24450`),
L (`1BB80`), the common axis (`235A0`), and E (`24E30`, no random word).
The seven G pointers bind selectors `[1,2,3,4,5,8,9]`; K binds `[11,10]`
and L binds `[6,7]`. These are model351's actual bank bindings, independent
of Type13's allocation. G's randomized target starts at `700 + (word >> 8)`.
K/L allocations and their output words start at zero. E retains method10,
sound81, zero cadence, and null joint bindings. Descriptor E+12 is 44 and
is not a joint binding; E+18..1B are the binding bytes and are all zero.

`D4A0` does not ground either dragon: default bit0x20 is clear, so authored
Y and the full constructor Euler basis survive. The portable native birth
policy initializes the unwritten transient B2 mass word to zero exactly
once; later contributions survive behavior replacement.

`AC60 -> 425680` still consumes a selector word for the singleton list.
Class7 `B6C0` restores only common-axis +4, clears Tertiary, installs
Secondary `2050/2080`, then installs the 500-ms Primary `2B10/2BA0`.
Each successful `6030 -> 6070` suffix consumes one G word. Thus successful
birth consumes four words, while living C690 reselection consumes three.
Reselection preserves the allocated D/E/G/K/L state, target and auxiliary
context words, and the current axis radius; the new acquisition task uses
that retained radius.

Sub-D is `[48,1,0,512,256,0]`: signed steering divisor48, yaw-to-roll
coupling, no pitch steering, and classifier flags zero. `41F660` increments
the stagger byte and performs its periodic row clear, then jumps to
`41FC90` without reading cache origins. The own seeds follow the authored
prefix's successful D allocations; no Type13 first-query receipt or guessed
origin is used. The cache origin remains unresolved because this path never
consumes it.

## Living task and presentation chronology

The Section2 records enable both actors during 50000..50500 ms, select the
first dragon's camera during 61000..61500, the second during 65000..65500,
and the factory during 69000..69500. These are presentation-phase commands;
their state changes are consumed by the following actor visit. A native
owner never activates itself from elapsed time or moves along a presentation
spline.

The live owner retains `12DA0` timing, current task identities and behavior
context. Primary, Secondary, and Tertiary run in source slot order. C7D0/ADE0
replaces acquisition with Chase and Aim; the newborn Primary waits until
the next actor visit, while the current Tertiary is visited once in the
same pass. Invalid targets, strict lifetime expiration, state-bit suppression
and exact task-wrapper retirement retain their distinct source paths.
Committed partial callbacks remain pending and cannot replay their RNG or
task age on a later frame. The live GKL adapter retains explicit reached
writes on failure: target/L-direction prelude, D cache and angles, K/L
prewrites, and G animation/audio/attitude before a blocked B210 force.
It publishes no untouched component outputs or replacement body basis.

Type10's `01430` binds the shared G/K/L frame to its own descriptors and
runtime. Detailed mode dispatches G, K and L; restricted mode dispatches G
while retaining K/L outputs. D applies divisor48 rather than Type13's 64.
Force and target projections read the retained incoming Q31 basis. The
outer `DCA0/E870` tail later rebuilds the complete Euler basis with `13F70`,
applies `E100`, then commits master integration once. E370's selectors and
lifetime are zero; only its existing timer decay remains.

Chase's post-mover proximity gate at `40354E` reads type+C8+24, namely
the Sub-H descriptor at type+EC. It does not test Sub-G. The dragons have
no H and therefore skip this extra Sub-A speed/direction controller even
though they have G.

Model351's long dark wing quads came from a separate presentation dependency,
also reproduced in static default-animation controls. Models352 -> 353 -> 354
import parent type-13 endpoints through type11; previously these stayed at wing
height while adjacent local corners projected onto the surface. World callbacks
now run before linked exports, carrying projected coordinates, semantic clipping
and surface provenance into each child. The existing GL depth adapter follows
that provenance. Real wing-quad tests retain intrinsic controls and verify both
projected and rejected endpoints; the exact contract belongs to
[world alias dependencies](RENDER_PIPELINE.md#world-alias-dependencies).
The normal-tier production GL run confirms the dark strips are gone at the
60/65-second dragon shots, with turret rings and the factory wreck smoke still
visible. Combat health/model transitions match the prior fixed-40ms run.

## Fire and factory damage

Type10 uses the existing descriptor-driven Aim transaction with its own
allocation and graph authentication. E supplies method10, random interval
300000 us, spread128, aim threshold12000, speed override1500, target-axis
tolerance3840 and sound81. Cadence lives in the native E allocation and
survives behavior changes; the queued-shot owner retains delivery order.
Restricted Aim ages its wrapper without executing detailed emission. The
live source traversal drains method10 into class38, preserving the shared
stream, sound, and sea-boundary particle substitution. 40120 now runs that
descriptor's F350/F590/441B70 contract, including the 410B0 sea-boundary
class46 substitution whose +0x28 bias cancels the -100 input displacement.
Method10 adds no source velocity to the shot; its separate closing-speed
boost still applies.

The factory can be destroyed through native Type66 health/death processing;
there is no elapsed-time destruction proxy. Its authored constructor health is
preserved, followed by the source's separate post-load current-health1 write
documented in [the factory authority](INTRO2_TYPE66.md).
A controlled class38 sweep against the original factory model already applies
the filtered 12300 hit. Matched whole-scene turret/factory removal remains
the production-scene gate in
[Intro2 combat projectiles](INTRO2_COMBAT_PROJECTILES.md).
The [Type102 defensive turrets](INTRO2_TYPE102.md) have native targeting,
firing and death owners. The shared
[post-load activation rule](INTRO2_ACTIVATION.md) also gates the dragons until
their authored release.

## Living terrain and water contact

The late11AD0 walk now runs the shared solid/water phase for living native
Search dragons. Their style+10/+14 hooks are null, but D7F0/D860 still execute
generic141D0. The previous class11-only adapter returned Ineligible and left
living model penetration uncorrected. Both actual model351 allocations now
exercise flat/sloped ground and16.7/40/125ms downward-displacement controls,
with wet-terrain dives and resurfacing kept free of a sea-height clamp.
Ground damage uses the existing native standard-death/Tumble publication;
subsequent same-walk water contact rereads the changed C750 hook. Actors already
in Tumble retain their existing terminal resolver. Source gates, shared kernel,
custody and ordinary-birth/static-contact boundaries are in
[Flying actor terrain and water contact](FLYING_SURFACE_CONTACT.md).

## Death and remaining boundaries

Type10's alternate is class11, not the common class12 corpse. `10C10`
enters `C660/404360`: clear Secondary/Tertiary, consume the generic G
constructor suffix, set G+3F to 1 with `1B970`, and publish the zero-lifetime
Tumble Primary. `404460` owns its accumulated-age rotation. Its detailed
path calls `018A0` directly, with no target prelude or Sub-D step.

G's tumble path is exact: `41A6AA -> 41A9CB -> 41AA36` clears dwords
+20/+24 and calls B210 with both forces zero. It skips wing oscillation,
sound, terrain attitude, self-righting, and velocity projection. Existing
animation words remain visible; K/L still execute in the detailed direct
component pass using their retained state and basis. Other G override modes
remain explicit evidence boundaries.

Contact selects class11's C750/BAC0 terminal variant; elapsed time alone
does not invent that transition. Terminal particle/radial/deferred-removal
custody belongs to the native death owner and the ordered late contact
phase. Full Intro2 terrain/water/static/active-pair coverage remains a shared
world-walk requirement; class11 support does not close unrelated live
actors' contact callbacks. Attached or remote-controlled dragons and other
G controller modes are outside this native authored profile.

Constructor, shared steering, retained component, live-mode, Aim and death
tests accompany the implementation. Actual OpenGL walkthroughs exercise New
Game, the full story, final card and first-world handoff. The former Type13
Aim rejection of Type102 targets was an invented target-type restriction;
the source handle/state checks are retained in the corrected adapter.

Controlled lethal radial hits followed by production motion and terrain
contact exercise terminal self-radial damage, physical response and one-use
deferred removal, with one sound62 per impact. This validates the terminal
lifecycle independently of natural combat and exact explosion particle counts.
Reentrant turret blasts admit only sealed terminal receipts on the active
call stack; matching an old blast's origin/template cannot replay its prefix.

## Ordinary Type5 on the Type10 owner

The six authored Type5 records (worlds 42, 46 and 47) have Type10's
Section-12 shape: D/E/G/K/L, one Always class-7 choice, alternate class11,
method-10 fire with sound81, mass100, health32000, capability8, default flags
8, and the same K/L bindings and damage multipliers. They differ per row, so
`Type10Profile` carries each value and every reader authenticates the
allocation's own row:

| Field | Type10 | Type5 |
|---|---|---|
| Model | 351 | 1122 |
| Damage thresholds (channels 1/2) | 9000/2200 | 2000/1800 |
| Sub-D steering divisor | 48 | 64 (`FLYER_SUB_D`) |
| Sub-G +0E / +20 | 63 / 750 | 69 / 1500 |
| Emitter spread / aim threshold / speed override / axis tolerance / +12 | 128 / 12000 / 1500 / 3840 / 44 | 512 / 40000 / 0 / 5120 / 48 |
| Common axis | `[7680, 0xC85]` | `[6400, 7]` |

B6C0's 06070 randomized-target base is G+0C (700 for both rows), now read
from the row's payload. Each row's emitter is its own ballistic-aim source
profile; Type5 fires method 10 at the table speed.

An ordinary birth runs the same 09A80/AC60/B6C0 transaction at its authored
pose: G `1B8C0` (one word), the process Sub-D allocation (flags 0, so its
cache is never read), K/L zeroed by their constructors, E, the 425680 word,
and B6C0's two 06070 words. The receipt carries the manager's allocation
lease; Intro2's spawns 55/56 still use their own seeds and carry none. The
generic death, contact, hit and C750 terminal paths read the dying row's own
type record and attribute its blast and self-collision damage to that row.

E100 keeps drag bit 8, so worlds 46 (mode 1) and 47 (mode 2) take EC60's
current-wind branch through the shared effective-8 flyer environment (gravity,
then EC60 with the post-F70 basis); world 46's wind acts only below its sea.

In Playing, the late contact walk runs Intro2's own order for these rows: the
13/10/57 surface kernel, then the retained-model static suffix for a living
body, or the Tumble's own terrain/water/static walk once class11 is published.
The walk lends its player hull and lives, so C750's terminal finishes through
Playing's static and dynamic radial. Playing particle hits enter the Type10
hit wrapper, the radial routes the receipt natively, and the Main Base
abort's 10C10 publishes the Tumble (worlds 46/47 now stop at Type43 in the
abort matrix). Player pair identity stays unresolved, as for the other
ordinary flyers.

[Tests](../../crates/v2k-game/src/intro2_type10/ordinary_tests.rs) cover the six
births, five-second cohorts in all three worlds, world 47's wind, and a lethal
Playing hit whose Tumble lands and explodes through Playing's radial.
[Abort tests](../../crates/v2k-game/src/main_base_abort_production/native/type5_tests.rs)
cover the class11 publication once per body.
