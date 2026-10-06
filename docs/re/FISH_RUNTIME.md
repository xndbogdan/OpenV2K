# Native fish runtime

Types22,23,24,62,124 share B/D/F components and authored neutral swimming tasks.
Types23/62 are `zebrafsh`, with environment2004;22/24/124 use200C.
The aquatic resource catalog is overlay11; Reef is gameplay23 and Cistern30.
Zebra Type62 uses common model38 and occurs in13/14/18/36. Its former
constructor-only path classified it as furniture, omitted Sub-F construction
and never scheduled movement. Type23 was also absent from the fish type list.
The accepted capture named `fish-movement-Reef` actually loaded Cistern30.
Its observed moving/coarse populations and limitations remain in
[ACTOR_RUNTIME](ACTOR_RUNTIME.md#reef-fish-movement-runtime-validated-2026-08-02).

## Construction and model state

The [native owner](../../crates/v2k-game/src/shared_fish.rs) runs for each
authored instance, preserving its allocation generation, spawn identity,
four model slots, Euler angles and immutable +90 anchor. Neither the level
number nor a captured spawn/seed admits construction. D4A0 bit20 is clear:
authored Y0 and above-sea positions remain unchanged at birth.

Sub-D retains process allocation history. The
[Sub-F constructor](SWIMMING_COMPONENT.md) consumes its two or three words
before weighted selection. The six signed variable selectors bind the same
retained model-variable bank, including aliases. The renderer reads these
words through ordinary dynamic model callbacks; coarse updates preserve them.
Selector0 is separate:40D320 returns the low16 bits of the global50-Hz clock
there, while positive selectors use40A950's one-based bank. Fish presentation
must restore that clock after copying Sub-F's words. `zebrafsh` model38 reads
selector0 for its fin transforms; changing only the swimming word bank leaves
its fin geometry frozen. Coarse parking retains component words, not this
independent presentation clock.
The Section12 +110 count bounds construction bindings. A negative or
out-of-bank selector is an unsupported descriptor, not a new bank allocation.

## Authored task graph

| Type | Always-weighted initial choices |
| --- | --- |
| 22 | Flocking3, Wander2, Aimless6 |
| 23 | Flocking3, Wander2, Aimless5 |
| 24 | Flocking3, Wander2, Aimless4 |
| 62 | Wander5 |
| 124 | Flocking10, Wander1 |

The [task owner](../../crates/v2k-game/src/shared_fish_tasks.rs) composes
existing shared retarget, Wander, acquisition and target-route policies:

- Aimless `ACD0`: clear Secondary/Tertiary, Primary `402B10/402BA0`,5000ms.
- Wander `AD10`: clear Secondary/Tertiary, Primary `402DC0/402EB0`, retaining
  the immutable birth anchor for target selection.
- Flocking style4C7DF8 `B640`: restore authored common-axis +4, clear Tertiary,
  install same-type Secondary `401F80/401FB0` with unlimited lifetime, then
  Primary retarget5000ms.
- Acquisition `C7D0`: write the selected allocation handle into context+8,
  switch to style4C7E40. `AF50` clears Secondary/Tertiary and installs
  Primary `403650/403780`,5000ms, tracking that handle.
- Task completion returns through `C690/AC60` to the authored weighted list.
  The task-result suppression bit precedes the selector and its RNG draw.

Each successful task constructor runs Sub-F's exact406070 reset with no RNG.
The target-route suffix then sets follow mode1. Failed task allocation does
not run that suffix. Mutation-safe wrapper custody discards a retired task's
result; a new Primary installed by Secondary waits until the next actor pass.

## Detailed/coarse and mover order

The shared scheduler visits fish at their allocation's place in the live
list, alongside other families using the same process RNG. The presented
view-detail owner retains the existing shared range policy.
Type62's Sub-D descriptor uses divisor32, forward/lateral probes768/256 and
classifier20. Retail41FEB0 returns class6 when the signed static-sea minus
center-cell height is less than256 raw units, before the other terrain gates.
This is a single center-depth test, not an inverted four-corner water gate.
Other neutral fish use classifier0. Both paths retain Sub-D's cache cadence;
without Sub-A,41F660 uses probe direction1 even during task reversal.
Sub-D receives both callback time and the independent global simulation delta;
the latter must not be replaced by accumulated coarse callback time.

`12DA0` owns randomized coarse waits and callback elapsed time. PE40E8AF
tests effective environment bit2000 and jumps to40E9C0: clear motion40000,
skip A800 tasks, and return before basis/environment/surface callbacks. Stored
velocity, task clocks, pose and animation remain unchanged. Distant parked
fish are therefore expected retail behavior.

Detailed DCA0 enables motion for2000 without1000, visits Primary, its C690
replacement and then the current Secondary. `01430` uses the shared target
prelude, Sub-D, post-D Sub-F target/smoothed-turn writes, Sub-F, then Sub-B.
The components read the incoming body basis. The outer callback rebuilds13F70
after all task visits. Environment2004/200C suppresses gravity; only200C
enters E100's bit8-gated wind/drag callback. Sub-F's own damping applies to
both. Master motion follows the common callback suffix. The sea cap belongs to the detailed Sub-F callback,
never the constructor or coarse parking path.

Level1 authors three Type62 fish, spawns4/25/26. Spawns25/26 are near the
windmill and spawn4 near the Cistern route. Two author Y=-768 above static
sea=-847. Their first detailed Sub-F call caps them below sea; forcing their
birth altitude would hide the missing update and alter coarse retail behavior.

The [surface owner](../../crates/v2k-game/src/shared_fish/surface.rs) retains
E370's separate policies. Types22/24/124 use selectors[0,0]; zebra23/62 use
[0,1] with lifetime1000ms. PE40E3C8..40E62A first increments the zebra timer
only strictly above sea, then still executes the final saturating decay because
selector+72 is zero. Submerged/equal-sea cases decay twice. Ordinary fresh
timers therefore do not accumulate an invented dry-land lifetime. A reached
162B0 expiry preserves its timer prefix and remains explicitly blocked at
the separate synchronous relation-release/death continuation.

### Steady wind and current

`E100/44EC60` reads the basis just rebuilt by13F70. In steady mode1, force
applies only when `(sea < Y) == (Section13+80 != 0)`; otherwise ordinary drag
applies. Two bilinear terrain probes, half and one wind vector upwind, define
clearance capped by signed Section13+98. Negative clearance returns without
fallback drag. Roll/pitch receive Q31 projections before velocity approaches
the scaled vector using callback B0 mass. The angle writes do not rebuild the
basis again. Vector words come from the low16 bits of +9C/+A0/+A4; an all-zero
vector disables wind mode in44EB40.

The [environment leaf](../../crates/v2k-game/src/shared_fish/environment.rs)
matched original PE44EC60 for800 deterministic executable-emulation cases
(seed4460), covering medium gates, terrain shelter, signed vectors and word
overflow, and Q31 attitude changes. Three PE golden cases and explicit
sea/shelter regressions remain in its unit tests.

## Descriptor contact

The native task contact adapter retains `02DA0`'s forward-half-space gate and
`01A20/019C0`'s B/D/F branch. Fish have no Sub-A speed draw. An accepted contact
updates Sub-D reversal, consumes the two X/Z retarget draws, retains the1500ms
private reversal timer, and passes direction==-1 to Sub-F+3C through424380
(`4019E3..4019F7`). Pose, velocity, body basis, task lifetime and the remaining
Sub-F animation/controller state are not advanced by this descriptor callback.
The current native task owner must lend completed mutation custody, including
when its other task slots have null contact callbacks.

## Particle hits and quiet death

The [particle owner](../../crates/v2k-game/src/shared_fish/impact.rs) runs the
native primary10EB0, static-route11180, infected11250 and cured11320 wrappers.
Primary/static stamp entity+34 before DAC0; infected sets model bit2000 before
DA00, while cured clears it before DA60. Both preserve the primary stamp.
The ordinary impact dispatcher selects the retained fish receipt before its
other family switch; the old placement after that switch returned early and
never reached the fish adapter from Playing.
The four living styles have the same direct C690 callback in all three hit slots:

| Style | Address | Infected+20 / cured+24 / primary+28 |
| --- | --- | --- |
| Aimless | 4C7930 | 40C690 / 40C690 / 40C690 |
| Wander | 4C79C0 | 40C690 / 40C690 / 40C690 |
| Flocking acquisition | 4C7DF8 | 40C690 / 40C690 / 40C690 |
| Flocking target route | 4C7E40 | 40C690 / 40C690 / 40C690 |

These PE table words disprove the former no-RNG hit interpretation. C690
reselects from the actual weighted list and replaces the native task graph
before11030. Direct C690 does not inherit416410's task-result suppression
bit1000. Reselection resets Sub-F through the selected initializer; it retains
the native allocation and context target/auxiliary words.

The living state enables11030 (bit04000000 set,08000000 clear). Its
25590 channel1/2 sum drives the mass-scaled impulse; three RNG words update
heading, roll and pitch in source order, including when that sum is zero.
The existing shared reaction owner preserves signed narrowing and the stale
body basis until the next normal rebuild. Checked15040 then filters damage,
consumes any buffer and updates health. The authored fish have null modifier
and generic hit callbacks, zero hit/death cues and no capability8 particle
suffix. Filtered-zero packets still run the earlier reselection/reaction.
In particular, the real F780 channel6/2000 packet filters to zero for the
authored fish: it changes the selected behavior/model and angular reaction,
while retaining health. A primary channel1 packet above its threshold can kill.

PE4111C1..41123A establishes11180's separate ordering: stamp+34, request the
living type+80 cue before DAC0, apply11030 with the wrapping32-bit impact sum
multiplied by eight, then15040 with the original unscaled packet. It has no
10EB0 post-damage sound/capability suffix. Both Playing and shared dispatch
admit the current native fish receipt to this owner; class52/68/85 keep their
existing descriptor/packet checks. This is particle-to-entity delivery, not
admission of the fish's own physical static-model scan.

The Antidote class6 F7C0 delivery preserves004CBFE8's static six-dword packet:
channels[2,0], amounts[1000,0], zero source and owner.11320 requests type+84
after clearing2000 even on a dying target, runs the proven style+24 callback,
then11030 and15040. Quiet class2 retains its null cure callback and retired
task owner; a repeat cure does not revive it or replace its deferred removal.

Types22/23/24/62 select alternate class2, whose existing audited descriptor4C8870
and style4C7420 initialize through C470. The
[death owner](../../crates/v2k-game/src/shared_fish/death.rs) connects10C10's
health/dying prefix and DB80/AC60 alternate selection to that quiet terminal,
clears Primary/Secondary/Tertiary in source order, and stages deferred removal.
Scheduler custody retires synchronously; the world splice removes the body
later. A class2 terminal does not create a timed corpse or explosion.
Type124 selects class63 and still stops at `UnsupportedDeathProgram` after
the committed lethal health prefix. It cannot borrow22/24's alternate.
Section12 loader410090 installs vtable4C8A30, whose+08 word is40DB80;
the later438080 overrides affect67 and46/51, not fish. C470 calls A860's
slot0/1/2 retirement and10B70 marking before the later14990 sweep.

## Acceptance boundaries

Constructor, graph and native scheduler regressions live in
[shared_fish/tests.rs](../../crates/v2k-game/src/shared_fish/tests.rs) and
[task tests](../../crates/v2k-game/src/shared_fish_tasks/tests.rs).
The component's600-case executable oracle is recorded in
[SWIMMING_COMPONENT](SWIMMING_COMPONENT.md#port-and-verification).
Its arithmetic evidence is independent of the passive trajectory capture.

The [real-world death regression](../../crates/v2k-game/tests/shared_fish_death.rs)
exercises both particle entries across every authored22/23/24/62 in
13/14/18/22/23/30/34/36 after
live task visits, checks filtered infection followed by lethal primary hits,
verifies class2 custody and deferred removal, and continues
the surviving Type124 scheduler. Focused controls cover wrapper RNG/reaction,
allocation/pending-prefix rejection and the distinct class63 boundary.
Full combat and physical static contact remain separate from this particle
entry. An unsupported lifecycle entry
must remain an explicit development diagnostic. The accepted trace supplies
no detailed Type124 trajectory and no complete collision/death/audio comparison.

Main Base abort admits native22/23/24/62 through the same swimming allocation
and completed task owner, then runs the existing class2 publisher and retires
the scheduler owner before deferred removal. `170A0` reaches the ordinary
`10C10 -> DB80 -> AC60 -> C470` path for these capability-zero actors. The
[abort controls](../../crates/v2k-game/src/main_base_abort_production/native/type62_tests.rs)
cover moved bodies, foreign allocations, pending impact prefixes, missing
scheduler custody, repeated death and the later removal sweep. Fixed birth
X/Z cannot authenticate a living swimmer.

Playing's `14AE0` radial walk admits the same native allocations before any
impulse, retains full-radius `15040` and falloff `14E10` filtering, and calls
that class2 publisher synchronously before reading the next live-list link.
Surviving fish keep their current graph and accept the velocity change without
task reselection. Authentic completed class2 allocations remain buffer
addressable until `14990`; a retired living owner cannot authenticate a forged
corpse. A later callback block retains and parks its committed prefix.
Type124 reaches the explicit class63 blocker after lethal health, without
quiet deletion. The
[radial controls](../../crates/v2k-game/src/specialized_actor_task_production/playing_radial/fish_tests.rs)
exercise every aquatic cohort in overlays13/14/18/22/23/30/34/36 after actual
live task visits, plus falloff impulse, custody rejection and class63.

The earlier claim that fish never share worlds with captors was based on the
incomplete22/24/124 census. Type62 does overlap those families. C910 rejects
the capability-0 non-person before reading captor state. The player beam never
targets fish (`is_beam_collectible` requires capability `0x1000`), and the
player solid pass omits them (pair orientation/component contact stay
unresolved). Admitting fish contact needs the same style-table evidence as the
Type9 02CA0 matrix: which Flocking/Wander/Aimless task templates install a
`02DA0` component callback. C910's non-person rejection does not suppress the
separate physical response/damage tail: fish lethal pair damage still lacks
its class2 continuation in that caller and remains blocked. Radial damage
has the separate source-order owner above; physical pair admission remains
independent of that completed damage/death phase.
The [ordinary-world census](../../crates/v2k-game/tests/shared_fish_worlds.rs)
discovers neutral swimmers from Sub-F and their authored task choices rather
than repeating the production type list. Across normal-tier overlays13–49 it
finds114: three in13, three in14, four in18, eleven in22,43 in23, twenty in30,
nineteen in34 and eleven in36. The dedicated Level1 regression covers all
three actual fish, swimming/model animation, the first detailed sea cap and
coarse-to-detailed resumption. Oscillating wind requires its separate global
phase owner for200C actors;2004 zebra fish never enter that callback.
