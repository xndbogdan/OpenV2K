# Shared insect static contact

This note owns the Type16/26/94 and G-flight Type13/15/87/10/57 extensions to
[`native_ground_actor/contact.rs`](../../crates/v2k-game/src/native_ground_actor/contact.rs),
alongside its existing Type53/122 adapters. It also owns their bounded live
Type66 contact through
[`native_actor_capture/pair.rs`](../../crates/v2k-game/src/native_actor_capture/pair.rs)
and the shared
[`native_actor_descriptor_contact.rs`](../../crates/v2k-game/src/native_actor_descriptor_contact.rs)
callback adapter. [Actor runtime](ACTOR_RUNTIME.md) owns construction and task
execution; [Type16](INTRO2_TYPE16.md) owns its cinematic first-query receipts.

The admitted native allocations are Type16 spawns 5/42, model257; Type26
spawns 10/25, model267, plus independently constructed ordinary-world births;
Type13 spawn0, model291; Type57 spawn1, model122; Type15 spawn44; Type87
spawn46; Type10 spawns 55/56, model351; and Type94 spawn43, model272. Each retains its own constructor
and completed task receipt. A current type label or task enum alone cannot
authorize body mutation.

## Recovered missing dispatch

The old Intro2 late dispatcher visited Type17/47/53/58/122 static contact,
but omitted Type16/26/94 and living Type13/15/87/10/57. An eligible native body
could intersect an authored Section10 model without running its callback or
physical response. Ordinary Playing likewise omitted Type26's static lane.
Mover obstacle avoidance and terrain/water contact did not replace that pass.

Native `11AD0` requires state8000, clear1000, zero+70, a nonzero active-model
radius and clear88000000 before the static scan. State10000 gates only the
earlier terrain/water lane. The scan uses the current Section10 models in
wrapped map cells and retains the deepest plane and cell through
`12CF0 -> A8B0 -> D920 -> 11760`. The adapter uses that exact oriented model
query and response; it adds no level offset or actor-radius approximation.

Section10 props and live Type66 structures use different collision lanes.
The tested tree models are452 `bigtree6` and768 `iclmang1`; these establish
shared prop contact, not building coverage. Intro2 also authors kind3/model410
`windmill` and kind10/model368 `castles`; the explicit building regression
uses both. Kind2/model142 `shieldup` and kind4/model140 `fuelcan` have solid
geometry but are pickups/props. Kind1's response is+1792, so it cannot supply
the negative-coefficient separating contact selected by these tests.

## Static callback and response

`A8B0 -> 01270` visits Primary, Secondary and Tertiary task+20 hooks in order.
Constructors `02B10`, `032A0`, `03B70`, `03650` and Chase `03360` install
`02CA0`. Defecate emitter, acquisition and Aim tasks retain their null hooks.
For these no-Sub-I A/D and G insects, `02CA0` reverses the private direction,
sets its timer to2500ms and consumes X then Z low-word RNG, each shifted6
minus512. `019C0` propagates direction to Sub-A or G+3C. G+38 target, rate,
modes, animation and K/L callbacks keep their chronology. The static callback
neither executes pair hook `02DA0` nor writes Sub-D yaw, ages a task, advances
an emitter or rebuilds the incoming body matrix.

| Family | Authored default C8 | D920 behavior before11760 |
|---|---:|---|
| Type16 |439| Generic D9B0 crush; admitted class4/5/7/9 style+1C is null |
| Type26 |439| Generic crush; class26 also invokes C89040000, then unconditional C690 |
| Intro2 Type94 |439| Generic crush; owned Follow/Search/Capture acquisition or pursuit has null style+1C |
| Type53 |39| No generic crush; admitted style+1C is null |
| Type122 |439| Generic crush; retains its own Capture relation and death owner |
| Living Type13/10/57 |8| No generic crush; Search/Move/fallback style+1C is null |
| Living Type15/87 | Authored initializer word | Own Search hook and null style+1C; actual alternate2 C470 death |

Type10's common-axis descriptor contains0xC85; that descriptor word is not
its collision policy. Constructor C8 is8, as for Type57/13. The adapter reads
the actual initializer metadata and retained native allocation.

D9B0's packet uses channels1/0, amounts40000/0, source-5 and owner0.
Type26 Furniture's task+20 is null. C890 runs after generic crush and calls
C690 even when filtering, duplicate registration or a changed static cell
prevents a second delivery. The replacement graph is registered before11760
rereads current motion and mass; the original plane survives. Native11760
separates on inward normal projection, computes impact from velocity delta
and mass, damages the current static cell before checked actor damage, and
publishes the actual family death when lethal.

Type16/26/53/94/122 use common12. Type10/57 publish their real Class11 Tumble
owner. Type15/87 execute alternate2 C470: release sound11, clear all three
tasks and queue deferred removal. C470 consumes no G constructor draw and
installs no upward corpse impulse. Type13 executes its authored Class1
`10C10 -> BAC0 -> BAF0 -> radial -> A860 -> deferred removal` through the
shared terminal receipt. Its nonnull G allocation chooses ten class37
scatter particles and BAC0 has no BD20 ring suffix. Surface, static and
pair damage supply the full synchronous terminal frame; a genuine completed
receipt can authenticate the retained same-walk null-hook suffix. Class1
style4C7150 has zero solid/water/static/task hooks. Immediate static burn invokes shared427760's
effect callback rather than only setting the burned bit.

## Surface policy and validation boundary

Living39/439 has bit1 set, so D440 clears state10000. Natural Type26 terrain
intersections at the hive-camera handoff are therefore ineligible for the
generic solid/water lane; forcing separation would change retail policy.
Class12's reverse2015 mask restores10000. Actual Type16/26 common12 allocations
now join the shared current-model surface owner with Type17/58/122. Valid
water entry and dive policy remain distinct from static separation.

G surface and static phases share one11AD0 visit. Damage-induced Type10/57
Tumble runs only the static suffix, retaining the entry model without repeating
terrain/water classification. If C750 has completed, the completion style's
hooks are null and11760 still runs. A source-authenticated Type13 Class1
completion or Type15/87 C470
completion likewise retains its same-walk suffix and entry model. Changed
awake/pending flags or retired scheduler ownership do not cancel an already
admitted visit. Later visits still obey entry gates and cannot fabricate a
completion continuation.

Type94's existing native world phase retains its water Sub-C and six H feet.
Its static/pair adapters consume A/D hooks only, and do not replace that phase
with the flying surface kernel or change H's TerrainAndWater policy. Actual
spawn43 is admitted by its own constructor and completed scheduler lease;
matching model272 or metadata without that receipt is ineligible. The tests
check the six retained foot records, cursor, surface policy and enabled state
across living static and factory/hut response. Genuine common12 can disable
H without discarding its six records or water policy. Attached Capture2–5
continues to require its separately unowned relation callbacks.

## Live factory and hut body contact

The ordered pair walker previously admitted native captor17/122 pairs,
independent58 pairs and gunner/hive pairs. It now admits an authenticated
Type16/26/94/13/15/87/10/57 versus owned Type66 pair in either intrusive seat.
Intro2's real live hut is spawn36/model364 and factory is spawn51/model210.
Ordinary factories retain the same BaseFactory constructor/task receipt with
their authored model slots; an Intro2 module name does not restrict that owner
to the cinematic.

This uses native11AD0's oriented Section8 query and retained subject entry
model. It keeps12530 gates, both directional behavior callbacks, P/S/T
`A900 -> 02DA0` hooks, and12760 response and damage cap. The factory's fixed
body policy stays authored; the pass does not substitute static11760 response.

`02DA0` tests signed forward half-space before private/component reads.
An admitted front contact executes01A20: immediate Sub-D heading/roll reversal,
the1500ms private timer and A/F/G direction propagation. A/D families consume
conditional Sub-A draws, then X/Z retarget draws. G/D families consume only
X/Z draws. G+3C changes while oscillator, animation and K/L state remain
retained. Behind contacts are native no-ops. WorkingFactory's25BD0 and
Class11's04360 retain01020/05FF0's null task+18. Both bodies require their
actual completed owner before callbacks or physical writes; parked or stale
owners cannot be replaced by a type label or current graph shape.

Native12760's capped packet reaches each current participant through15040
checked damage. Type66 uses its progressive-death publisher and scheduler
owner. Insects use their own common12, Class11, C470 or synchronous Class1
publisher. The shared static error type preserves the full Class1 terminal
or radial error and retained prefix if completion fails. Intro2
and Playing use the same walker from each retail live-list cursor; an
unordered pair is visited from its earlier participant. Ordinary native
Type26/Type66 therefore share the admitted rule.

The actual dragon/factory and ordinary stag/factory probes also exposed
model-query opcode0x05. Native46AF20/46B6D0 recurse through composed child
attachment/basis and linked animation slots while retaining the callback
context. The shared model decoder owns that restoration for terrain, water,
model/model and ray queries. Unsupported geometry is reported, never treated
as an implicit contact miss.

## Focused evidence and remaining boundaries

`tests/insect_static_contact.rs` uses the actual high-tier Intro2 constructors
for all four Type16/stag births, six G births and the Type94 water actor, plus Type26 constructors
from two ordinary authored worlds. Real Section10 geometry supplies controlled
overlaps. Tests check missing-owner rejection without mutation, typed02CA0
state, retained task ages/basis/heading, authored crush and Furniture ordering,
native plane response and parked-prefix custody. G assertions retain the
oscillator/rate/modes/animation while checking the reverse byte. Native death
assertions distinguish graph destruction from living retarget. A separate
common12 publication proves restored Type16/26 surface eligibility while
living439 remains excluded.

`native_actor_capture/pair_insect_factory_tests.rs` uses the actual Intro2 hut
and factory for all nine admitted insect births, plus ordinary Type26/Type66
pairs. Tests preserve constructed intrusive order and verify missing-owner
rejection, exact model contact, fixed-body response, basis retention, six
ordered P/S/T callbacks, front/behind behavior and family RNG/private-state
effects. Unsupported geometry is collected with exact participant/model IDs
and still fails the test.

Separate lethal Type13 static and factory-pair cases keep the actual native
allocation and collision packet, with a controlled health1/buffer0 injury and source inward velocity selected through the exact authored filter and native damage-cap arithmetic. They check
finished Class1 custody, its real style and empty task graph, effects before
return, a single deferred removal, absence of the Class49 ring suffix, and
static re-hit suppression without replaying terminal publication.

Controlled OpenGL checks use authenticated Type26 stag/windmill and Type16
model257 `tarantu`/factory overlaps, then production `resolve_intro2_contacts`.
The windmill is the actual model410 at cell118/119; the factory is model210.
Baseline and current
libraries must render the identical input pose and report their own
post-dispatch position. The baseline remains at [30336,3,30592] and
[20800,-256,-5696]. Current static11760 moves the stag to [30631,46,30592];
current pair12760 moves Type16 to [20846,355,-5727]. The factory case explicitly
has no prior static contact. Both original and fixed overlap images match;
only the fixed post-dispatch images expose the supported separation.
This fixture proves the connected path; it does not
establish matched retail trajectories for the whole Intro.

Ordinary-world Type16 births now share this owner through their own 104B0
receipt ([Intro2 Type16](INTRO2_TYPE16.md)). Attached Capture16/94 transport,
ordinary G native construction and ordinary Type94 construction remain outside
this admission. Pairs with unrelated building/actor families
are outside the bounded Type66 extension; captor/58/gunner-hive pairs retain
their existing owners. Unknown graphs, parked/executing prefixes, unowned
static damage kinds and missing geometry fail closed. Existing Type10/57
Tumble callbacks retain their own Class11 terminal/radial behavior. These
boundaries must not be called retail acceptance solely because another family
passes.

The preliminary broad visual-fixture search encountered model556's collision
opcode `0x90` (144) at cell144/126. The sphere-query interpreter now owns it
as `FUN_00469C20`'s upright cylinder
([format](FORMAT_DOCUMENTATION.md)); world35's model658 uses the same
primitive. It was never silently converted to a miss or used for the supported
windmill fixture. The narrowed fixture was independently
selected through the real model410 geometry and production dispatcher.

The final natural Intro2 replay exposed a separate Type17/Type9 Capture
delivery lifetime gap at tick1072. Real child59 is released and queued for14990
removal before a later source-eligible pair in the same walk. The scheduler
now retains its exact completed release owner as contact-only custody, without
another task visit; actual allocation, queue, pending state and completed
inner graph remain mandatory. [TYPE17_CAPTURE.md](TYPE17_CAPTURE.md#released-type9-contact-after-delivery)
owns the source ordering and authored Intro2/ordinary regression cases.

The restored actual main loop with frontend seed137 and40000us frames
completes4302 ticks with zero contact issues (`.tmp/intro-integer-137.log`),
including the completed31-to59 contact and normal deferred removal of59.
The original complete route reported89 boundaries; the intermediate
restoration left this single lifetime failure. This closes the measured
Type9 case; other child families and ordinary unowned G constructors
retain their explicit source custody boundaries. A repeated10B70 accepts
no second state clear or deferred count increment on an already pending
allocation; the native repeated-delivery regression preserves its newly
completed child graph without advancing an ordinary task.
