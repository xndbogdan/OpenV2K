# Shared Type17 actors

The shared model256 spider constructor, tasks, movement and hits are owned by
[`intro2_type17.rs`](../../crates/v2k-game/src/intro2_type17.rs). The normal-tier
corpus authors seven ordinary Type17 actors: four in overlay13 and one each in
14,20,35. Intro2 adds spawns4/30. Native ordinary and Intro2 loads use actual
construction receipts; explicit captured replay remains separate. Shared task
semantics remain in [ACTOR_RUNTIME.md](ACTOR_RUNTIME.md); the whole-scene gate is
Intro2 acceptance.

## Construction and task ownership

104B0/09A80 constructs A/B/C/D/H/J, with eight Sub-H records, Sub-A base250,
mass100, health5000 and authored default39. D4A0 snaps the initial terrain
height and adds Sub-C clearance75 before copying the anchor and before
AC60/25680 evaluates the already-linked actor prefix. The four
authored choices are People Nearby x4/class9, Player Nearby x3/class10,
Under Attack x8/class10, Always x1/class33. The nearby range is strict wrapped
XYZ0xA00. Fresh last-hit tick0 makes Under Attack false at every possible
construction tick.

A successful birth consumes four shared random words in order: 20450's Sub-A
constructor, the weighted selector, Secondary06070 and Primary06070. The
shared constructor retains the Sub-D allocation already charged to the process
counter, rather than replacing it with a scene seed. Its frame owner/runtime
survive C690 and death. The receipt retains the actual manager allocation
generation, authored index, model slots and grounded anchor; a copied receipt
from another manager cannot authorize a live callback. Entity+B4 retains the
independent common-body stamp. B2 uses the explicit native zero-residue policy
and later follows the ordinary mass/animation owner.

[`construction.rs`](../../crates/v2k-game/src/intro2_type17/construction.rs)
accepts the current metadata and authored row without a world or spawn list.
It preserves the caller's pre-D4A0 surface classification, integer terrain
grounding, constructor model-slot selection, authored angles and damage buffer.
The preceding linked prefix includes the persistent player where present.
The supported profile remains model256/A/B/C/D/H/J; different model overrides,
animation/configuration payloads and component descriptors fail explicitly.
Native first classification uses the shared empty-cache policy described in
[cross-level construction](CROSS_LEVEL_GAMEPLAY_RUNTIME.md#native-ordinary-world-construction),
not the replay coordinates below.

Capture and Run Away use B6C0's acquisition plus 500-ms retarget graph; Follow
uses B740's distinct beacon acquisition plus the same retarget duration. B6C0
resets only actor-local axis+04; B740 preserves both axis words. Target
acquisition and successful handoff update the current graph before the
executing wrapper unwinds. A new Primary starts on the next actor pass; a
replacement Secondary can run in the remaining current pass. Blocked prefixes
retain their current graph and cannot replay elapsed time or RNG.

Capture's C7D0/C6B0/AF50 publication and 403780 pursuit are shared with Type53
through an explicit allocation profile in
[`intro2_capture_pursuit.rs`](../../crates/v2k-game/src/intro2_capture_pursuit.rs).
Target warning416360 stamps the target and evaluates its type+92 cue before
the Primary allocation. The generic06070 suffix precedes the signed
`base*4/3` overwrite:333 for Type17,533 for Type53. Each actor keeps its own
metadata, mover and Sub-D custody.

## Matrix and world-frame ordering

The common A/B/C/D/H phase has one source order: target prelude, Sub-D20360,
018A0 component dispatch, Sub-C1F1C0, Sub-A1E9E0, Sub-B1EB50. Sub-D changes
angle words and steering runtime; it does not rebuild entity0C..2C. C/A/B must
use all nine incoming Q31 matrix words, including nonzero pitch and roll.
Normal mode0 admits Sub-H; restricted mode1 skips H and still reaches C/A/B.

The actual13F70 rebuild is outside the task callback: DCA0 at40DE9B or E870
at40E8EB, after the complete A800 task walk and optional E640 terrain attitude,
before E100 gravity/drag. The native shared
[`intro2_common_mover.rs`](../../crates/v2k-game/src/intro2_common_mover.rs)
preserves this boundary for both Type17 and Type53. The older Level1 Type17
mover's immediate rebuild belongs to the separate bounded replay adapter.
Native first-world actors use this same shared outer owner as later worlds.

The outer owner retains detailed/coarse scheduler carries, random waits,
mass `u16(typeMass+B2)` with zero promoted to1, B2 clearing, surface lifetime
and master motion. Presentation and the camera consume this live position,
physical body basis and animation state.

## Late static contact

The shared living owner reaches authored static/fence contact in the later
`44FFA0 -> 11A80 -> 11AD0` pass, after physical particles. The
[`contact.rs`](../../crates/v2k-game/src/intro2_type17/contact.rs) adapter uses
the current Section-10 model, active model slot, animation and physical body
basis. It retains the selected plane across `12CF0 -> A8B0/01270 -> D920 ->
11760`; the task hook does not rebuild the basis or run another mover visit.

The four Primary constructors `02B10`, `03650`, `03B70` and `03E20` all install
`02CA0` at task+20. Thus Capture/Run Away/Follow acquisition, pursuit, Following
and Run Away share the private-state response. Their acquisition Secondary
tasks leave that hook null. The no-Sub-I Type17 branch reverses direction,
sets the 2500-ms reversal timer, propagates Sub-A and draws target X then Z.
Typed commits retain each task's elapsed time, target/route and audio state.

Capture0/1, Run Away0/1 and Follow0/1 have null style+1C and zero flag masks;
effective39 excludes the crush branch. The common response delivers nonzero
collision damage to the static cell before the self-attributed actor. Lethal
damage publishes the shared Class12 owner without visiting it in this pass.
Actual completed allocation/task custody is required before mutation; a
failure after task, physical or damage writes parks the retained prefix and
cannot replay its RNG. A geometric miss consumes no task work.

Class12 style `4C7ED0` remains eligible for static contact. Its `404120`
constructor retains `05FF0`'s null task+20, so the contact leaves the exact
Common-Dying task, elapsed time, Sub-A and RNG unchanged before physical
response. The `2015` disable mask still excludes crush. Current completed
Class12 custody is required, and any later failed prefix is retained under that
owner. The world20 corpus control covers a real dying spider against an
authored fuel object, including immediate static burn before actor delivery.

The [spider contact authority](SPIDER_BEHAVIOR.md#following-static-contact-ownership)
retains the detailed hook arithmetic, fence policy and controlled corpus
regressions. The subsequent sections in that authority own the separate
[bare-terrain/water](SPIDER_BEHAVIOR.md#late-bare-terrain-and-water-contact) and
[active-pair](SPIDER_BEHAVIOR.md#native-active-actor-pairs) phases. Native
[Capture transport](TYPE17_CAPTURE.md) retains the C910/D0B0 callbacks,
carrying variants2--5 and child custody.

## Late terrain and actor pairs

Living C8 policy `39` clears body `10000`, so its source11AD0 visit skips
12870/129B0 while still scanning static objects and actors. Class12's `2015`
disable mask re-enables that body bit. The native
[`native_actor_surface_contact.rs`](../../crates/v2k-game/src/native_actor_surface_contact.rs) owner now
runs its oriented solid query, D7F0/default141D0 response, then live-position
water classification and D860/default141D0 response. It retains the entry
model and current wave descriptor84, emits the source scatter or hard-entry
Type60/sound suffix, and preserves the existing Common-Dying task clock.
Forcing the flag on an unsupported living style does not authorize this path.
The same phase now serves Type58 through an explicit family policy, retaining
the spider's own model, cue27 and capture-specific death cleanup.

The native [`pair.rs`](../../crates/v2k-game/src/intro2_type17/pair.rs) lane
classifies every non-player pair incident to a native spider in intrusive
subject/next order, including Follow, Run Away and Class12. It keeps the pass's
entry eligibility/model while rereading current poses, relations and candidate
state. Subject sounds precede both directional behavior calls and the fresh
Primary/Secondary/Tertiary component walks. Capture acceptance and PowerUp's
unsupported-recipient A300 both suppress physical response only; their
42E8E0 dispatch is a no-op and the remaining callbacks still run.

[`native_actor_descriptor_contact.rs`](../../crates/v2k-game/src/native_actor_descriptor_contact.rs)
owns the exact per-task 02DA0 callback for native8/9/17/47/53 and the existing
Intro2 Type94 owners. It retains the signed half-space gate, each actual
private task record, Sub-I versus no-Sub-I
policy, shared RNG and incoming basis. Both physical bodies require current
custody even with null component hooks or zero damage. The shared response
uses current unsigned self masses and ordered14D30 cap, then direct15040
deliveries; it does not replay a projectile stamp, mover, clock or constructor.
Unknown counterpart task/body/death owners remain explicit reached blocks.
In particular, generic surviving Hive67 damage does not close its lethal owner.
Ordinary Type53 counterparts now use their own [shared construction and
lifecycle](INTRO2_TYPE53.md), including the actor reached by the overlay14
spider scan. They retain actual process Sub-D and manager/task custody;
Intro2 Type94 support remains bounded to its existing constructor owner.

## Hit and death entry differences

Primary10EB0 stamps entity34 before DAC0/style28; infected11250 sets2000 and
plays type82 before DA00/style20, preserving entity34. Both then execute11030
and415040. The exact executable words differ by style:

| Style | Infected +20 | Primary +28 | Death +2C |
|---|---|---|---|
| Capture0/1, Follow0/1 | C690 | C690 | null |
| Capture2..5 | null | D040 | D040 |
| Run Away0/1 | C690 | null | null |
| Run Away2, Class12, initializer fallback | null | null | null |

C690 consumes the current list, hit tick and shared RNG immediately. Its
replacement graph is transferred to the scheduler before the next hit. A
pending unfinished actor prefix rejects either hit entry before any timestamp,
model, sound or RNG write, so an impact cannot revive that partial frame. A
filtered-zero class5 virus packet still executes the preceding callback and
11030 reaction, including its three random words when reaction is enabled.
It must not inherit the particle's birth provenance: F780 uses its static
six-dword descriptor with zero source/owner words.

Checked lethal hits and dynamic radial death publish native Class12 with the
same A/B/C/D/H/J allocation, model256 extent315, mass100, Sub-A base250 and
death sound94. The null Class12 target does not consume Sub-D. E370's
2000-ms living surface lifetime also transfers directly to this death owner;
its new Primary is not visited again in the same actor pass. Primary accepted
hits alone perform the type80/capability8 suffix with the current model extent.
The shared ordinary/Intro2 particle entry also owns14E90's local player-kill
event4: source46, a successful10C10 return, and the post-death01000000 bit.
It queues the existing session-deduplicated hint before returning to10EB0's
suffix. The separate filtered-zero player selector23/soundDB boundary remains
explicit; a notification sink does not silently authorize that branch.

Live visits, hits, Class12 and Main Base abort authenticate the same allocation
generation and current task custody. The abort's170A0 callback uses
10C10/DB80/AC60/C620 and registers the shared Common-Dying owner; it does not
require captured spawns17..20 or their old selected styles. Completed current
tasks are required before replacement, and pending/executing prefixes block
without replay. Remote and already-dying callbacks preserve the existing graph,
elapsed time and RNG; a foreign native receipt is rejected before either no-op.
The constructor/lifecycle regression matrix covers all seven ordinary births,
the two native Intro2 births, varied process history, and later-world abort
publication/custody.

## Shared-runtime validation

The normal-tier constructor census covers ordinary13--49, all seven authored
spiders and both Intro2 births after earlier process history. Lifecycle controls
exercise movement, primary/radial damage, Class12 retirement, abort custody,
living/dying static contact and capture-related child ownership. The new
[surface controls](../../crates/v2k-game/src/native_actor_surface_contact/type17_tests.rs) and
[physical-pair controls](../../crates/v2k-game/src/intro2_type17/pair_physical_tests.rs)
exercise actual native allocations, source-selected Follow/Run Away, both
intrusive orders, zero/lethal person and gunner contact, Class12 and parked
custody. The full V2000 repository check passes 4,319 tests, including 3,234
game library tests. These controlled phases do not establish sustained retail trajectories. The
[transport authority](TYPE17_CAPTURE.md) retains the source matrix and its
separate matched-retail boundaries. The whole-scene gate includes the complete
frontend/Intro2/first-world transition alongside ordinary gameplay controls.
The fixed40ms GL run completes125 captures without runtime issues, including
actual Class12 dry/water contact and sound17 in ordinary14 and Intro2. The
varied-frame/frontend250 run completes124 captures and the same actor controls;
its separate [Type13 incoming-hit boundary](INTRO2_COMBAT_PROJECTILES.md#production-scene-comparison)
keeps whole-scene acceptance open.

## Existing first-query evidence

The explicit captured-replay adapter retains these accepted V200002 joins for
the two Intro2 births independently:

| Spawn / seed | Handle / allocation | Tick / elapsed microseconds | Query X/Z | Mode |
|---|---|---|---|---|
| 4 / 04 | 04F80001 / 03774A70 | 011E / 121000 | 8D00 / FFFF8E00 | 0 |
| 30 / 17 | 04DE0001 / 0377A2E0 | 02FD / 125000 | C2FF / 8100 | 1 |

Each constructor/register/return/frame/query chain retains descriptor10A96044,
its own allocation and thread457C. The first query takes the full-reset branch,
returns0 and writes row0=1. These authorize each replay first-query reset, not
fixed movement, timing or a copied trajectory. The
accepted ledger
owns the retained transcript filename; no new capture is required.

## Remaining boundaries

Player-as-candidate Type17 pairs now enter the player adapter and apply native
no-Sub-I `02DA0`. Player-as-subject uses that same adapter walk: authenticated
Type17 component contact applies the descriptor plan, and unauthenticated Type17
fails closed as `Type17DescriptorContact` with actor/type/spawn diagnostics.
Unknown counterpart tasks and physical-body
custody stay explicit. Lethal Type17 `DeathDispatchRequired` on the player
adapter remains deferred. Hive67 lethal destinations now run the dying initializer,
slot-0 `FUN_004260F0` and `FUN_00440950` burst with dying-model extent and sea
level; nested `FUN_004566E0` stays the projectile/abort host. Shared Type53 construction and component contact do not
close that family's carry2--5/D040 or complete11AD0 scan. The external writer/entry
for Run Away2 and missing-parent recovery are not authorized by the current native
graph; an externally entered Run Away2 graph fails closed as `UnsupportedRunAway2`.
Sustained pen containment and matched retail scene acceptance remain open. The
[spider authority](SPIDER_BEHAVIOR.md#next-bounded-implementation-order)
retains these contact gates. [Capture transport](TYPE17_CAPTURE.md) owns the
implemented C910/carry2--5/release/delivery/D040 lifecycle.
