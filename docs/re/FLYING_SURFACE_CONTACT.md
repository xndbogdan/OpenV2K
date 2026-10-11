# Flying actor terrain and water contact

This note owns the shared late `11AD0` solid/water phase for native living
Types13/10/57 and15/87. The bee/wasp visual comparison and recorded trajectories
remain in [Intro2 bee water](INTRO2_BEE_WATER.md). Family movement, firing and
death remain in their existing actor notes.

## Source admission and dispatch

`FUN_00411AD0` first requires state8000, state1000 clear, subject+70 zero and
a nonzero active-model collision radius. Its terrain/water branch additionally
requires88000000 clear and10000 set. It retains the entry model across callbacks.
`12870` finds solid model contact; the type-vtable+0C trampoline then invokes
the current style's+10 hook and re-resolves the handle before the generic
`141D0(handle,contact,0x7fffffff,2,1)` response. Whole-body `129B0` classification
uses the resulting current position and retained entry radius. A water-entry
edge requests type+88, invokes type-vtable+10/current style+14 and, if the handle
survives, runs `141D0(handle,contact,0,5,0)`.

The common vtable4C8A30 uses D7F0/D860 for these two trampolines. The authenticated
retail executable (SHA256 `e9be7a833612fba3a5a5ab92a974ece1a689e4b7e72409d9ee8331380573b4ba`)
has these style words, corroborating the null-hook branch in the decompiled
`bulk/game_logic.c` D7F0/D860 consumers:

| Current style | Solid+10 | Water+14 |
| --- | --- | --- |
| Search acquiring4C7A50 / pursuing4C7A98 / completion4C7AE0 | null | null |
| Move About Aimlessly4C7930 / completion4C7978 | null | null |
| Initializer fallback4C74F8 / completed Class1 Explode4C7150 | null | null |
| Completed class63 Auto Pilot4C7198 | null | null |
| Falling Tumble4C7F60 | C750 | C750 |

The native frontend constructor now binds104B0's authored-center surface bits
for every Intro2 body with its actual construction tick and descriptor+84 wave
policy, before09A80 can change position or components. The former four-spawn
dry-cell binding omitted13/57; their unknown old-above bit became a water-contact
block when a later route entered a wet cell. Clockless captured construction
resolves only admitted13/10/57/15/87 dry cells and retains its wet-clock boundary;
generic level construction does not borrow the native frontend receipt.

A null hook still permits the generic response. The prior Type10/57 contact
adapter deliberately admitted only class11 Tumble and returned Ineligible for
living Search; Type13 had no late surface dispatcher. Living15 also missed the
already-owned flyer surface phase. These omissions are distinct from authored
terrain exclusions: default policy39/439 clears10000 for ordinary living
Type17/26/58/122 cohorts. Geometric terrain overlap alone cannot override that
policy. Their Class12 reverse policy can restore the branch after death.

## Shared implementation and regression scope

[`flying_surface_contact.rs`](../../crates/v2k-game/src/flying_surface_contact.rs)
owns the common source phases, while
[`native_flying_surface_contact.rs`](../../crates/v2k-game/src/native_flying_surface_contact.rs)
authenticates Types13/10/57 and
[`intro2_flyer_contacts.rs`](../../crates/v2k-game/src/intro2_flyer_contacts.rs)
authenticates15/87. They share active oriented model/animation geometry,
signed-word separation, inward-speed scatter, velocity correction, channel1
collision damage, current-pose water classification and water response. No sea
height clamp or family-independent ground-height offset is added.

The complete15/87 prefix also owns the subsequent static visit, retaining the
entry model before synchronous surface damage can publish quiet death. Both
Intro2 and the ordinary Playing loop call this shared phase before active pairs.
A blocked surface leaves static unvisited; an ineligible surface still permits
static's independent source admission; either blocked phase stops later pairs.
Native Hive children authenticate their allocation and completed task receipts
through the existing flyer owner. Their source1C830/1CA90 ejection clears8000
for half a second and remains ineligible during that interval. The failed-Level1
ground-burial defect was a missing Playing dispatch: the generic actor surface
and insect-static adapters omit15/87, although their native Hive custody and
surface response were already implemented.

Collision damage uses the shared checked-damage path. Native10/57 lethals
publish their existing class11 Tumble owner. The subsequent water hook rereads
the changed style and uses the existing C750 terminal callback when required;
the generic water tail follows that synchronous callback. The dispatcher does
not replay a living surface visit through the Tumble resolver. Actors already
in Tumble retain their prior terrain/water/static resolver.

Native Type13 instead authors alternate1, Class1 Explode, at style4C7150.
Its source-proven null predecessor death hooks enter10C10/DB80/AC60 without a
weighted selector draw. The shared [explosion owner](../../crates/v2k-game/src/class49_death.rs)
keeps its native D/E/G/K/L allocation and current Search or Aimless graph;
G is present, so BAF0 takes40BB52→40BBB3 and emits ten class37 particles.
The authored model header+08 supplies scatter extent and Section12+50..+6C
supplies radial damage and logical-owner provenance. The complete static then
dynamic radial scan finishes synchronously before A860 clears slots and10B70
stages removal. BAC0 creates no Type60 ring. Incoming primary/infected and11180
hits, solid terrain, G static contact and admitted Type66 pairs all lend the
same complete terminal context. A late radial failure retains the actual
Class1 receipt and original tasks in parked custody; later visits cannot
replay its effects, RNG or damage. A completed allocation lends exact receipt
custody to the remainder of that same retained-model contact walk.

Completed Type13 custody authenticates the exact allocation and currently
published task graph, with no pending task/world/basis prefix. Its cached
Normal/Restricted mover dispatch mode is not a graph identity. A fresh adopter
starts in Normal mode, so comparing the entire adopter with a completed
Restricted owner wrongly blocked the independent late11AD0 phase. The custody
check now compares allocation, graph and completion while preserving11AD0's
own subject+70 and state gates.

Ordinary Type13 births reach the same phases in Playing. The late contact walk
sends type13 through the shared flyer helper. That helper keeps this 13/10/57
surface kernel for Type13, then the retained-model static suffix (or static's
own admission when the surface is ineligible). Playing's walk lends these
phases its player hull and lives (the `_with_playing` entries), so a lethal
surface or static contact finishes Class1 through Playing's static and dynamic
radial. Intro2's walk lends none and keeps the cinematic radial, which cannot
visit a live player. Ordinary Type5 rows on the Type10 owner take the same
walk, and their Tumble's C750 terminal (terrain, water or static) runs
Playing's radial with the lent player. Intro2-only Type57's Tumble water
terminal still owns only its cinematic radial and fails closed if a Playing
player reaches it.

Native15/87 instead author alternate2, `Die Quietly`, whose zero-policy style
is4C7420 and initializer isC470. Their actual Section12 rows in1X3XX retain
mass100, sound11 and B/D/E/G with A/H absent. `10C10` writes health0/dying,
requests the authored+90 cue and releases+8C before DB80/AC60 selects that
alternate. C470 clears slots0/1/2 and queues the deferred splice. It creates
no corpse task, changes no velocity, consumes no G/A constructor RNG and
preserves Sub-G state. The shared flying death adapter carries the completed
allocation publication into scheduler retirement. The current contact walk
can still resolve its handle and perform the generic physical tail before
the central deferred-removal phase.

Before/after regressions retain the real Type10/57 allocations and demonstrate
the old Tumble resolver's living Ineligible result, followed by the shared
surface correction. All four native13/10/57 model allocations cover27 controlled
flat/sloped contacts each, with downward displacements spanning16.7/40/125ms
frame durations. Tests check actual model penetration after separation,
velocity response and native death custody. Wet-terrain controls descend below
sea level and resurface without clamping, retaining one entry effect per crossing.
The15/87 source-pose and natural-scene checks are owned by the bee-water note.
The [native Hive Playing regression](../../crates/v2k-game/src/specialized_actor_task_production/hive/tests.rs)
uses actual Level1 birth/ejection/task receipts across two runtime RNG prefixes
and16.7/40/125ms callbacks. It preserves1CA90's500000us timer independently from
the flight task's per-callback truncated milliseconds. Controlled downward
poses on unmodified dry Level1 ground show the former generic dispatcher leaving
an overlap unchanged; the shared phase separates the real oriented model to at
most3raw residual, then visits static. Lethal contact also finishes native quiet
death before that retained-model suffix. Type15's retail selector slots alias
model276; the changed dying selector does not create another corpse model.

Controlled full-main failed-world checks retain natural Wasps for28seconds
under Main Base loss, casualty loss and a real near-Hive player target. Each
produces three children; the new phase has no blocked outcomes, preserves the
ejection gate and resolves reached static-building contacts. The no-contact
Main Base control retains identical actor traces and pixels. None of those
bounded natural runs reproduces solid-ground burial, so they establish the
connected phase and scene smoke, not the reported trajectory or retail acceptance.
The shared Intro2 extraction also retains identical opening38-second frame
traces and five bee-scene images in a matched before/after port control.
A further2,592 visits cover all six native model allocations through actual
family movers and the production late-contact dispatcher: three runtime RNG
offsets, three frame durations and48 iterations per combination. A dormant
12DA0 visit first clears allocator-owned B2 through its real source write;
a bounded downward impulse every eight frames then stresses solid contact while
subsequent positions and orientations come from the family mover. Tests check
real model residual penetration (at most3 raw units), movement, contact dispatch
and absence of parked prefixes. A separate completed Restricted Type13 visit
reproduces the old custody rejection and verifies its solid response. These
controlled runtime RNG offsets do not establish a cinematic constructor stream
or require a dive in every native scene run.
Additional Class1 regressions exercise both real Search and Aimless predecessor
graphs, primary and11180 lethal hits, complete solid death before water/static
tails, and a genuine late-radial failure with unreplayed effects. The prefix
unit retains tasks until radial completion, verifies authored provenance and
G state, and rejects duplicate claims/finishes. No corpse graph or ring is
substituted for Class1.
These tests establish the bounded source phases; scene fidelity still requires
the corresponding original-game visual comparison.

## Remaining boundaries

Nonzero solid+86 cues need the source inward-speed gain; these admitted rows
have zero solid cues.

The13/10/57/15/87 birth/custody adapters authenticate their existing Intro2 allocations.
Native Type15 Hive children also retain the
[complete allocation/task birth receipt](HIVE_WRECK.md#authored-creature-births-and-failed-world-selection)
and use the same surface/quiet-death phase. Other ordinary authored-world births
and complete mover/task custody for those families remain unowned. The common
kernel contains no level or spawn offsets, but a public type match alone cannot
manufacture their component/task custody. Living13/10/57/15/87 static-building
dispatch now uses the shared geometry, G component direction and native death
adapters in [Insect static contact](INSECT_STATIC_CONTACT.md); active-pair
dispatch beyond already-owned family paths remains a separate boundary.
Source-ineligible terrain overlap is preserved, including living
Type26. None of those boundaries is closed by passing surface regression tests.
