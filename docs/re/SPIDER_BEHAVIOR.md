# First-world spider behavior

This document owns the first-world Type17 hunting/contact boundary (`spider`,
global model256, authored spawns17--20). Native loads now use the
[shared Type17 constructor and lifecycle](INTRO2_TYPE17.md), including direct
first-world loads, later worlds and Intro2. The older bounded Follow/static-
contact adapter remains separate evidence. Native capture and transport use
[the shared Class9 lifecycle](TYPE17_CAPTURE.md).

Shared mechanics remain in [ACTOR_RUNTIME](ACTOR_RUNTIME.md),
[ACTOR_TASK_PROGRAMS](ACTOR_TASK_PROGRAMS.md), and
[ENTITY_DAMAGE_AND_DEATH](ENTITY_DAMAGE_AND_DEATH.md). This document owns the
Type-17 integration boundary, not another copy of those programs.

## Implemented versus missing

| Stage | Current port boundary | Source / original program |
|---|---|---|
| Fresh birth and weighted selection | Shared native construction retains actual process Sub-D and manager allocation, then four RNG words: Sub-A, selector, Secondary06070, Primary06070. Behavior is selected from the linked prefix. | [intro2_type17/construction.rs](../../crates/v2k-game/src/intro2_type17/construction.rs), [type17_impact_reselection.rs](../../crates/v2k-game/src/type17_impact_reselection.rs); `004104B0 -> 004381F0 -> 0040AC60 -> 00425680`, then `0040C6B0`. |
| Capture / Run Away acquisition | Shared live owner runs the Secondary search and parallel 500-ms Primary retarget, retaining task replacement/unwind custody. | [intro2_type17/behavior.rs](../../crates/v2k-game/src/intro2_type17/behavior.rs), [intro2_type17/live.rs](../../crates/v2k-game/src/intro2_type17/live.rs); `0040B6C0`. |
| Follow acquisition | Shared live owner runs ranked search, Primary retarget/mover and synchronous variant-one handoff. | [intro2_type17/behavior.rs](../../crates/v2k-game/src/intro2_type17/behavior.rs), [type17_follow_beacons_live.rs](../../crates/v2k-game/src/type17_follow_beacons_live.rs); `00402120`, `00422E30`, `0040C7D0`. |
| Follow movement | Shared A/B/C/D/H owner retains incoming body basis through the mover, then rebuilds it in the outer callback before environment/master motion. | [intro2_type17/mover.rs](../../crates/v2k-game/src/intro2_type17/mover.rs), [intro2_type17/world.rs](../../crates/v2k-game/src/intro2_type17/world.rs); `00403CE0 -> 00401430`, then `00412DA0` / `0041D360`. |
| Follow completion / reselection | Shared live Primary consumes post-unwind results and strict 9,000-ms expiry, then reselects without retaining a stale Following graph. | [follow_beacons/live_primary.rs](../../crates/v2k-game/src/follow_beacons/live_primary.rs), [intro2_type17/live.rs](../../crates/v2k-game/src/intro2_type17/live.rs). |
| Run Away movement | Native variant-one 2,000-ms task and result/reselection execute with Type17 components. Externally entered variant2 remains unsupported. | [intro2_type17/run_away.rs](../../crates/v2k-game/src/intro2_type17/run_away.rs); `0040B3F0`, `00403E20`, `00403F40`. |
| Wooden-pen contact | Shared native late11A80/11AD0 contact runs the actual retarget, pursuit, Following or Run Away hook, physical response and static-then-actor damage. Peasant-side entry is now enforced by the ordinary Type9 static-contact owner; sustained spider-side containment remains a scene acceptance gate. | [intro2_type17/contact.rs](../../crates/v2k-game/src/intro2_type17/contact.rs), shared [static_contact.rs](../../crates/v2k-game/src/static_contact.rs), [ordinary_type9_static_contact.rs](../../crates/v2k-game/src/ordinary_type9_static_contact.rs). |
| Late bare terrain / water | Living39 is source-ineligible; native Class12 re-enables body10000 and owns oriented solid response followed by current-wave water crossing, scatter or hard-entry Type60. | [native_actor_surface_contact.rs](../../crates/v2k-game/src/native_actor_surface_contact.rs); `00412870`, `004129B0`, `0040D7F0`, `0040D860`, `004141D0`. |
| Native actor body pairs | Every non-player pair incident to a native spider is classified in intrusive order. Current callbacks, native8/9/17/47/53 and existing Intro2 Type94 component hooks run under actual custody before shared physical/checked damage; unsupported counterparts stop explicitly. | [intro2_type17/pair.rs](../../crates/v2k-game/src/intro2_type17/pair.rs), [native_actor_descriptor_contact.rs](../../crates/v2k-game/src/native_actor_descriptor_contact.rs); `00411AD0`, `0040D8D0`, `0040A900`. |
| Native pursuit / capture contact | Class9 variant-one pursuit and oriented C910 contact retain the actual child/captor allocation in either intrusive order, including A300 and fresh task callbacks. | [intro2_capture_pursuit.rs](../../crates/v2k-game/src/intro2_capture_pursuit.rs), [entity_pair_callbacks.rs](../../crates/v2k-game/src/entity_pair_callbacks.rs); `0040C910`. |
| Carry / deliver / release / kill | Native variants2--5 retain captor-owned Sub-J callbacks, child Carried tasks, full-basis pose, delivery and release/kill. [Source order](TYPE17_CAPTURE.md) is shared across ordinary worlds and Intro2. | [sub_j_attachment.rs](../../crates/v2k-game/src/sub_j_attachment.rs); `0040C910`, `0040D0B0`, `0040CF90`, `0040D040`. |
| Spider hit / death | Shared primary/infected style dispatch, reaction, Class12 and Main Base abort retain actual allocation generation and completed task custody. Capture2--5/D040 performs child cleanup before the current death continuation; direct death and ordinary primary hits retain different C620 publication counts. | [intro2_type17/impact.rs](../../crates/v2k-game/src/intro2_type17/impact.rs), [intro2_common_dying.rs](../../crates/v2k-game/src/intro2_common_dying.rs), [main_base_abort_production/native.rs](../../crates/v2k-game/src/main_base_abort_production/native.rs). |

The native load adopter retains whichever supported graph the actual constructor
selected for each spider. A cold-run spawn18 Follow result describes one RNG
history, not a rule for the other spiders. Explicit replay fixtures preserve
their captured allocation and first-query policy separately.

Canonical normal-tier Level-1 data places all four spiders inside the authored
pen: spawn 17 at wrapped X/Z `(157,155)`, 18 at `(154,148)`, 19 at `(154,157)`,
and 20 at `(157,148)`. The 40 kind-9 `spikes*` fence cells trace its outline
within X `150..161`, Z `145..161`; on each spawn's row its X lies strictly
between the left and right fence cells. Both ordinary models `512..522` and
the mirrored `532/540/542` side participate. No spawn relocation is needed.

## Original hunting and capture program

The authored choices are People Nearby x4 -> Capture People, Player Nearby
x3 -> Run Away, Under Attack x8 -> Run Away, and Always x1 -> Follow Beacons.
Nearby selection uses the strict wrapped XYZ candidate walk; the actor
common-axis range is `0x0A00`. Capture acquisition overrides the capability
filter with `0x0C00`; Follow uses `0x0100`. A fence breach is not a separate
behavior selector here: normal movement/contact must enforce intact geometry.

The original `00411AD0` pass requires state `0x8000` set, `0x1000` clear,
and subject `+0x70 == 0`. After bare terrain/water handling, static contact
calls `00412CF0` first and the type's `+0x8A` sound when applicable, then
`00427E20` dispatches the static-kind contact action. `0040A8B0` dispatches each task's
`00401270` static-contact hook before re-resolving the actor and invoking
type-vtable `+0x34` or default `00411760`. This owns physical response and
damage; replacing it with a pen-shaped position clamp would omit the program.

### Following static-contact ownership

The shared native owner ends12DA0 after master motion. The later
`44FFA0 -> 11A80 -> 11AD0` pass runs static contact after physical particles,
using the current physical body basis without another rebuild or mover visit.
The earlier bounded Following replay adapter retains its separate callback
placement before Sub-H geometry. Both scans read current Section-10 attributes and
terrain-type model selector, wrap signed 8.8 coordinates, and retain the
deepest contact in retail X-outer/Z-inner order. State `0x88000000` must be
clear; static objects do **not** require the separate bare-terrain `0x10000`
enable bit. Unrelated unresolved surface bits do not disable the static scan.

The Primary constructors `00402B10` (retarget), `00403650` (pursuit),
`00403B70` (Following) and `00403E20` (Run Away) all install `00402CA0` at task
state `+0x20`. Their exact writes are `00402B6B`, `004036B3`, `00403BDB` and
`00403E83`. The acquisition Secondary constructors `00402050`/`00402100`
retain the null hook from `00405FF0`. Following's `+0x18` is the distinct
`00402DA0` pair hook. `00402CA0` tests the common table's Sub-I pointer. Its
present branch adds `0x2000` to entity heading `+0xA2`. Type 17 has no I/F/G,
so its actual branch negates WanderPrivate `+0x10`, sets `+0x14` to 2,500 ms,
and propagates the direction to Sub-A through `004019C0`. It then consumes
exactly two shared random words: target X and Z become the corresponding
actor position plus `((low16 >> 6) - 0x200)`, wrapping at 16 bits. Target Y,
tracked handle, elapsed lifetime, route and audio state remain unchanged.
The shared adapter commits only the private record through each actual task
type; it does not replace a live task with a Following fixture. Unlike `00401A20`,
this hook does not invoke the Sub-D immediate-reversal writer. The detached
implementation and both branches are in
[follow_beacons/static_contact.rs](../../crates/v2k-game/src/follow_beacons/static_contact.rs).

The Section-12 loader installs common vtable `004C8A30` at type `+0x7C`
(`bulk/ovl_handlers.c`); its `+0x34` points to `0040D920`. Capture0/1
(`004C7FF0`/`004C8038`), Run Away0/1 (`004C7618`/`004C7660`) and Follow0/1
(`004C7B28`/`004C7B70`) have null `+0x1C` and zero set/clear masks. Type-17 policy `0x39`
therefore skips `0040D9B0`'s `0x400` crush branch. Authored capabilities `8`
exclude player-only kind-contact actions, and type `+0x8A` is zero in both
sound sites. The owner re-resolves the actor before `00411760` response.

Impact is the shared signed-word velocity-delta/mass formula. A nonzero
impact delivers `[1,0,impact,0,17,self_handle]` first to `00427950` on the
current static cell, then directly to `00415040` on the actor. Fence kind 9
uses response `-256`, channel-1 threshold 4,000, multiplier 512, and chance
gate 1,000. The spider uses threshold 2,000 and multiplier 256. Static chance
RNG precedes actor damage and any death-constructor RNG; the specialized pass
and later destruction advance share the same `StaticDamageScheduler`.

[intro2_type17/contact.rs](../../crates/v2k-game/src/intro2_type17/contact.rs)
uses the shared checked-delivery boundary: buffer, dying gate, generic hit,
health, then standard-death/Class12 publication. Self attribution
does not enter projectile hit-stamping, impact reselection, reaction, or
player feedback. A new Class12 owner first ticks on the next pass. Native
contact requires current manager allocation and completed task custody before
mutating either actor or RNG. Missing resources, unsupported static damage
programs or unresolved reached callbacks retain the committed prefix and park
that owner; a repeated call cannot replay the hook. A geometric miss leaves
task state and RNG untouched. The later terrain and active-pair owners below
retain these same physical-basis and task-custody boundaries.

Class12's active style `004C7ED0` has null+1C, and `00404120` retains the null
task+20 from `00405FF0` through allocation and publication. The corpse still
passes the static subject gate; its `2015` disable mask does not clear8000 or
enable crush. The shared adapter authenticates its completed Common-Dying owner,
skips task/Sub-A/RNG writes, and runs physical/static-before-actor response while
retaining that owner's task clock. A later failure parks the surviving Class12
prefix. The world20 corpus regression exercises this null-hook branch against
an actual authored kind4 object.

The older replay boundary in
[type17_collision_damage.rs](../../crates/v2k-game/src/type17_collision_damage.rs)
preflights its complete death suffix before entering it: unavailable evidence
retains lethal health subtraction but stops before health-zero/dying-bit/sound
writes. This remains a replay evidence boundary, not an emulation of an
original failed Sub-H callback.

The older normal-tier corpus tests in
[type17_static_contact.rs](../../crates/v2k-game/tests/type17_static_contact.rs)
exercise real model 518 and mirrored models 532/542, exact task RNG, inward
response, a `532 -> 533` authored breach, both damage recipients, and next-pass
Common-Dying custody. Shared native tests in
[contact_tests.rs](../../crates/v2k-game/src/intro2_type17/contact_tests.rs)
use actual authored construction and task handoffs to cover those fences,
acquisition/pursuit/Run Away private-state commits, preserved basis and clocks,
Class12 publication and parked partial failure. A breached cell loses its own
collision while intact neighboring fences remain eligible. Controlled contact
tests do not establish sustained containment across all four spiders.

The complete Class9 style matrix, C910 attachment transaction, carrying search,
D0B0 delivery, CF90 release/kill and D040 nested cleanup now belong to
[Type17 capture and transport](TYPE17_CAPTURE.md). The pair dispatcher below
selects those callbacks from the current graph, independently of whether the
same pair ultimately receives physical response.

### Late bare-terrain and water contact

11AD0 requires body8000, no1000 attachment, subject70 zero and a nonzero
active-model collision radius. Its solid/water prefix additionally requires
88000000 clear and10000 set. Living Type17 C8=`39` clears10000; Class12's
`2015` disable mask re-enables it. This is why a native corpse needs the
late terrain owner although the same living spider skips that prefix.
An externally changed living flag or unsupported Run Away2 graph is not a
substitute for the actual source entry.

[native_actor_surface_contact.rs](../../crates/v2k-game/src/native_actor_surface_contact.rs) retains the
entry model for12870 and129B0. The oriented solid query precedes
D7F0/default141D0: contact bit/separation, optional gain cue and material
scatter, velocity response, then direct checked damage. Water rereads the
surviving pose and crossing bits, uses the current descriptor84 wave policy,
and queues type88 before D860/default141D0. Class12's style hooks are null.
Its response may scatter particles or construct/register a real hard-entry
Type60 ring; the source sound17/vertical-velocity suffix survives allocation
failure. Neither phase re-runs the mover, advances the Common-Dying clock or
replays C620. Missing metadata or a later publication failure retains the
committed prefix under its current owner.

### Native active actor pairs

The native pair lane owns every non-player pair with at least one shared
Type17 instance, rather than only capability0C10 C910/D0B0 contacts. The
caller visits subjects in intrusive order after their surface/static phases.
Each scan retains entry eligibility/model while querying current poses,
relations and candidate flags. Eligibility is independent of behavior class;
Follow, Run Away and Class12 use their real current callbacks.

Subject type8E/8C sounds precede D8D0 subject then candidate. Main Base258A0,
lifter25850 and Hive259F0 return null against the spider's capability8 before
their dependent work. PowerUp25AF0 instead returns4BEAA8 (tagA300): like
Capture's accepted tag, its4C5070[0] dispatch reaches the single RET42E8E0.
The tag cancels physical response, not the opposite behavior or either A900
walk. Retail non-A300 callback errors retain the physical suffix before return;
an unavailable host callback is an explicit evidence block, not that source
return and not an invented null result.

A900 rereads Primary, Secondary and Tertiary after preceding callbacks.
[native_actor_descriptor_contact.rs](../../crates/v2k-game/src/native_actor_descriptor_contact.rs)
classifies each actual task constructor's null or02DA0 hook for native8/9/17/47/53
and the existing Intro2 Type94 allocations. Their current constructor receipts
and scheduler owners are required. [Shared Type53 construction](INTRO2_TYPE53.md)
now supplies ordinary actors' own process Sub-D, allocation and selected graph;
Type94 remains bounded to its existing Intro2 constructor.
The signed forward-half-space test precedes private/descriptor reads. The
Sub-I branch changes heading and direction without advancing animation;
the no-Sub-I branch retains its reversal and RNG policy. Typed writes preserve
lifetimes, routes, audio and incoming body basis. Native47 refreshes completed
mutation custody on its retained owner, preserving its task counters.

Both native physical bodies require completed allocation/task custody before
sounds or mutations, even for null hooks and zero damage. Shared response uses
unsigned self masses at+B0, separation12760 and ordered14D30 damage capping.
It calls15040 for subject then candidate, gating each delivery by the opposite
source's current remote sign. It does not enter projectile11030/DAC0 or stamp
a hit tick. The local implementation leaves remote transport and unsupported
counterpart task/body/death owners explicit. Generic Hive67 survival still does not
authorize that lethal suffix; lethal contact now does.
The overlay14 spider/Type53 contact now uses that ordinary Type53 owner instead
of stopping at a missing constructor/context. This closes the reached
counterpart dependency, not Type53's own carry2--5/D040 or complete11AD0 scan.

Pairs without a native spider are skipped by the lane (`pair_spiders`
gate), except native-Type47 versus bound-hive pairs, which the lane now owns
in either intrusive direction with the exact 11AD0 ordering. HiveRadial
live/dying tasks are classified as null-contact owners (their 25E10/25F60
templates zero +18 explicitly), so the shared component walk closes: the
gunner's 02DA0/null hooks run under actual custody and the hive side is a
no-op. Both sides' behavior-style +0x18 policies are read live at contact
time (None/Continue on the gunner side, Hive-null against capability 8);
anything unaudited stays an explicit `UnsupportedBehavior` block. Other
non-spider pairs still need their contact-time styles and component owners
recovered before admission.

Focused [pair controls](../../crates/v2k-game/src/intro2_type17/pair_physical_tests.rs)
use actual native allocations and weighted roots for both intrusive orders,
zero/lethal person and gunner contacts, Class12, A300 and parked-corpse refusal.
[Surface controls](../../crates/v2k-game/src/native_actor_surface_contact/type17_tests.rs)
exercise the separate source flag and response branches. The full V2000
repository check passes 4,319 tests; these controls do not prove sustained
retail behavior.
The fixed-timing GL control completes125 captures without runtime issues;
the varied-frame control completes124 captures and the same native controls
but retains the separate [Type13 incoming-hit gap](INTRO2_COMBAT_PROJECTILES.md#production-scene-comparison).

## Next bounded implementation order

1. Verify sustained first-world spider-side pen containment and native
   Intro2 trajectories with the complete late surface/static/pair ordering.
   Peasant entry and ordered02CA0 task turn/retarget are implemented through
   the ordinary Type9 static-contact owner's audited living-style matrix.
2. Complete spider/player contact: the player adapter now admits Type17
   candidates with live pair-to-world orientation and applies native no-Sub-I
   `02DA0` through the same descriptor planner as pair.rs. Unauthenticated Type17
   component contact fails closed as `Type17DescriptorContact` with actor/type/spawn
   diagnostics. Lethal player-pair packets to a native Type17 stage a
   commit-time `CandidateDeathDispatch` and complete through the shared
   capture-path 15040 and standard death (motion first, capture death second,
   staged pre-damage collision never clobbering it); the planner stays pure,
   RNG is drawn once, and lethal packets to any other family preserve the
   fail-closed `Core(DeathDispatchRequired)` boundary. A lethal subject packet
   stages symmetrically and re-enters the player's own checked-damage terminal
   at commit time — subject direction first — with hull/model/sound ownership,
   so the pass no longer grants ram immunity. Hive67 lethal contact now owns the dying initializer/Sub-K suffix.
   Type61 abort, pair, live radial and capture-destination lethals emit
   `FUN_00440950` with dying-model extent and sea level; nested `FUN_004566E0`
   stays the projectile/abort host.
3. Recover the external writer and actual relation/task entry for Run Away2,
   plus missing-parent recovery. An externally entered Run Away2 graph is now a
   named `UnsupportedRunAway2` fail-closed error rather than a silent Graph miss.
   Static analysis has identified the writer chain in the decompiled C: every
   weighted selection flows through `FUN_00438340`, which dispatches through the
   entity `+0xCC` external-event link (`FUN_0040ABE0` allocate with the
   `DAT_004DCA00` static-point fallback, `FUN_0040ABB0` rewrite) into the
   `FUN_0040C6B0` variant switch and the `FUN_0040ADB0` initializer, installing
   the `FUN_00403230` None task. What remains unproven is every trigger: the
   port's own selection never yields variant 2, `FUN_00447AC0` (variant advance)
   has no observed caller, and a sweep of the accepted intro2-actor-ai,
   enemy-ai (which does show variant-1 fleeing) and friendly-ai captures finds
   zero `0x004C76A8` sightings. It stays fail-closed pending a targeted capture
   of an actual variant-2 entry.
   Static analysis has identified the writer chain in the decompiled C: every
   weighted selection flows through `FUN_00438340`, which dispatches through the
   entity `+0xCC` external-event link (`FUN_0040ABE0` allocate with the
   `DAT_004DCA00` static-point fallback, `FUN_0040ABB0` rewrite) into the
   `FUN_0040C6B0` variant switch and the `FUN_0040ADB0` initializer, installing
   the `FUN_00403230` None task. What remains unproven is every trigger: the
   port's own selection never yields variant 2, `FUN_00447AC0` (variant advance)
   has no observed caller, and a sweep of the accepted intro2-actor-ai,
   enemy-ai (which does show variant-1 fleeing) and friendly-ai captures finds
   zero `0x004C76A8` sightings. It stays fail-closed pending a targeted capture
   of an actual variant-2 entry.

The successful occupied-row capture and nested cleanup program are no longer
capture-gated. [Type17 capture evidence](TYPE17_CAPTURE.md#evidence-and-source-boundaries)
separates the accepted ordinary attachment from Intro2 pursuit-only samples;
`V200002.run` retains the existing first Sub-D query evidence. Matched scene
acceptance must still compare actual trajectories, contacts and timing, without
substituting a controlled port fixture for a retail observation.
