# Type17 capture and transport

This document owns the model256 spider's Class9 attachment, carrying tasks,
delivery, release and nested death cleanup. Construction and the ordinary
world-frame owner remain in [INTRO2_TYPE17.md](INTRO2_TYPE17.md); terrain and
fence contact remain in [SPIDER_BEHAVIOR.md](SPIDER_BEHAVIOR.md). The shared
Sub-J container stores ordered child identities. Its row callback policy belongs
to the captor and is distinct from player cargo.

Implementation ownership is shared with the independently authenticated
[Type122 profile](TYPE122_RUNTIME.md), through
[pair contact](../../crates/v2k-game/src/native_actor_capture/pair.rs),
[carrying tasks](../../crates/v2k-game/src/native_actor_capture/carry_tasks.rs),
[capture lifecycle](../../crates/v2k-game/src/native_actor_capture.rs) and
[scheduler child custody](../../crates/v2k-game/src/specialized_actor_task_production/capture.rs).
The Type17 modules retain used facade names; each captor keeps its own model,
components, selector, slot offset and allocation receipt. Child admission
remains native Type8/9; sharing the relation does not implement other children.

## Evidence and source boundaries

The accepted `20260724-042053-enemy-ai.jsonl` contains a paired successful
ordinary capture. At sample4448 / 22240.13ms, Type9 handle04B10001 changes to
Carried style004C76A8 with parent04A80001 and health1500. The same sample changes
Type17 handle04A80001 to Capture3 / 004C80C8. Sample4452 / 22260.52ms then changes
that captor to Capture4 / 004C8110. This proves the linked runtime transition;
it does not sample the entire callback transaction or every carrying variant.

The Intro2 survey `20260722-022303-intro2-actor-ai` records spawn30 /
handle04790001 entering Capture1 / 004C8038 at sample5072 / 25360.20ms, then
returning to Follow0 at 28490.42ms. Spawn4 remains Follow1. Its deep typed
selection was47/26/13, so that recording does not prove the Type17 target or a
successful attachment. The older `20260724-041937-enemy-ai.jsonl` likewise is
not the successful carrying oracle. Exact filenames and accepted status remain
in the capture ledger.

Direct retail executable disassembly closes the callback and task order below.
No new broad AI or death recording is needed to implement those branches. A
future matched scene comparison must distinguish an actual capture from a
pursuit-only observation.

## Contact and child ownership

`0040C910` runs through the current style's active-pair callback:

1. Reject an absent, inactive, dying or non-capability0C00 child.
2. `00418410` checks capacity. A full captor calls `00410C10` on the contacted
   child and returns null.
3. `00418440` appends one row `[0040CAD0, 00443D10, child]`; `00416700` then
   attaches the child to the captor at the returned slot descriptor plus2.
4. Apply conditional hint8 feedback, write actor-local axis filter10 and call
   destination search `00422C10`. Store a found target and select absolute
   variant2; a miss selects variant3.
5. Normal success returns tagA300 through `004BE328`.

These are sequential, fallible operations. A later failure retains its earlier
row/relation/behavior writes. A host admission failure is not a retail callback
error and must not be disposed as one. The native relation retains the actual
captor and child allocations alongside its fixed callback policy; another
manager's handle or a player-owned row cannot authorize the operation.

The active-pair walk begins at the subject's next intrusive-list node. Wrapped
eligibility and broadphase precede current oriented model geometry. A300
suppresses physical response while the opposite behavior callback and both
component chains still run. Each stage reads the current graph, including the
carrying graph that C910 has just installed. Proximity alone cannot call C910,
and an attachment must not stop the remaining pair callbacks.

The native lane owns person/destination pairs with a current C910/D0B0 spider
callback in either subject order. The persistent player keeps its existing
pair owner. General Follow/Run Away body contacts and unrelated actor pairs
remain separate work; this does not enable every pair callback at once.

The tag object's vtable4C5070 points to42E8E0, a single RET, so immediate4575A0
dispatch consumes no action or RNG. Hint8 belongs to C910's earlier conditional
feedback, not this effect object. Its4568B0 controller-phase5 gate is explicit
at the call site: ordinary gameplay can queue it, while Intro2 cannot consume
the session's deduplication bit.

The child's ordinary Type9 `16700 -> DBF0 -> CD50/CE70 -> CD70` transition
installs its real Carried None task. Release uses `16750 -> DC50 -> CE90` and
current type-default selection. Native worker callbacks retain their own Type8
implementation. Neither path applies player pickup cues, player cargo lists or
player release positioning. Child scheduler custody transfers synchronously
at the captor's current visit, including a child in the scheduler's pending or
already-visited portion.

## Carrying task matrix

Class9 starts at004C7FF0 with stride48. Existing acquisition0 and pursuit1
remain the entry path; these variants extend the same actor and components.

| Variant / style | Initializer and task | Root / completion / pair |
|---|---|---|
| 2 / 004C8080 | AF10 clears Secondary then Tertiary and installs03650 with duration0, meaning an unlimited target route. | RootCF90; pairD0B0. |
| 3 / 004C80C8 | B780 clears Tertiary, allocates02190/021B0 search in Secondary, then02B10/500ms retarget in Primary, including both06070 suffixes. | RootC7B0 selects absolute5; successfulC7D0 advances to4. |
| 4 / 004C8110 | AFD0 clears Secondary then Tertiary and installs03B70/9000ms Following. | RootCF90; completionC790 selects absolute3. |
| 5 / 004C8158 | ACD0 clears Tertiary then Secondary and installs02B10/5000ms retarget. | Both root and completionCF90. |

The distinct021B0 search writes axis filter100 and calls22F10. It selects the
**minimum signed** target+88 value from initial65535; ties consume `low16 & 1`.
Follow Beacons'22E30 instead selects a maximum from initial0. Their shared data
shape does not make the selectors interchangeable. The known Type52 payload
supplies target+88; an unknown eligible target cannot be assigned a guessed
priority. Carry4 reuses the detached Following state/kernel, not the Class33
owner and its different context validation.

Executing task wrappers unwind before post-return callbacks. A newly published
Primary waits until the next actor visit; a replacement in a later physical
slot can run during the remaining current visit. Carrying movement retains the
current world's descriptor84 wave policy and existing Sub-D/basis ordering.

Carrying does not add the child's mass to this mover's Sub-C input:4017C9
explicitly pushes0 before41F1C0. The185C0 cargo-mass branch belongs to Sub-G,
which Type17 lacks. E640/E100's child-mass-dependent water mode is inactive
because the admitted Sub-C+0C is0;4EC60 drag uses the actor's own B0 mass.

The local single-player Sub-J pose phase18640 runs after DCA0/E870 and before
12DA0's master motion. It compacts missing, dying or zero-state child rows,
zeros each surviving child's velocity and copies the captor position. Child
bit800 enables the signed slot offset transformed by the captor's full Q31
basis. This updates the actual child used by presentation, not a visual copy.

Presentation is independent of relation bit1000. The11720 live traversal calls
11400 for the child itself;11400 admits its main-model callback through bit800,
without inspecting its parent. The later entity+38 effect suffix is not Sub-J
child rendering.18440 only clears800 when the selected slot's low policy byte
is0. Canonical Type17's one slot has policy1 and offset[0,10,110], so a captured
Type8/9 keeps its own model visible. Player policy0 slots instead clear800 in
the shared append transaction. The port's gameplay and Intro2 model submissions
therefore include attached children with proven800; the broader unparented
body/effect admission remains the boundary documented in
[Render Pipeline](RENDER_PIPELINE.md#entity-model-visibility).

## Delivery, release and nested cleanup

The reset235D0 restores **both** axis words. The callbacks pop exactly one row;
the authored capacity1 is not permission to replace the operation with a
container-wide clear.

| Callback | Ordered effect and continuation |
|---|---|
| D0B0 | Reset axis; capability10 contact calls418590 once. Decrement the row before its443D10 callback. An error returns beforeC690. |
| 443D10 | Call16750(child, child), then only on zero return call10B70(child) for deferred retirement. This is not10C10 standard death. |
| CAD0 | Call16750(child, captor). |
| CF90 | Reset axis, draw one random word, call418500 once. Dispose its callback error and continue. If `low16 & 3 == 0`, call10C10 on its output child or sentinel. Always re-enterC690. |
| D040 | Reset axis, call418500 once. An error returns beforeC690; zero return entersC690. |

Successful443D10 delivery retires the released child's ordinary scheduler
visit when10B70 marks deferred destruction. Its queued allocation survives
until the normal14990 sweep. Ordinary release without10B70 keeps the living
owner eligible for its next task visit.

### Released Type9 contact after delivery

Retail443D10 calls16750(child, child) and only on zero return invokes10B70.
The latter, only while0x100000 is clear, clears0x60000, sets0x100000
and increments the deferred count. Repeated calls on an already pending
allocation leave its state and count intact. It does not clear the newly
published P/S/T tasks, free their private state, or remove the intrusive body.
The later14990 sweep owns those removals. Neither11AD0's geometric eligibility
nor A900's component dispatch uses callback-enable0x20000 as a substitute
for its own gates.

The40ms Intro2 replay exposed the missing lifetime distinction at tick1072.
Entity59 is the actual Type9 child58/model558, carried by Type17 entity31.
Earlier Type58 entity25 contacts31; D0B0/443D10 releases59 through16750/CE90
and publishes RunAway tasks before10B70. The former port deleted the owned
release graph immediately; the later source-eligible31-to59 pair then reported
`BodyCustody { entity_id: 59 }` before14990.

The restored actual main-loop replay with frontend seed137 and fixed
40000us frames completes4302 ticks with zero contact issues
(`.tmp/intro-integer-137.log`). The later31-to59 contact completes and
allocation59 is removed by its ordinary deferred sweep. The original
baseline reported89 boundaries over the same complete route; the
intermediate restoration left this one same-walk custody failure.

The scheduler now retains the exact completed Type9 publication inside
[`delivered_type9_contact`](../../crates/v2k-game/src/specialized_actor_task_production/delivered_type9_contact.rs).
That linear receipt preserves the allocation lease and underlying live owner
in the same Scheduler/Pending/Retained storage position. It lends no task
visit. Contact mutation requires the actual deferred queue, known pending bit,
matching manager allocation and the existing inner graph's completed boundary.
Authorized C910 attachment, C690 hit replacement and Class14 death publication
retain the retirement proof. A parked source prefix, changed graph, missing
allocation or absent pending bit remains blocked. No graph is reconstructed
from a model, behavior label or task bytes.

The same receipt path admits actual ordinary Type9 release publications;
an owned Class14 null release preserves its genuine corpse graph. The focused
tests reproduce the old owner deletion against identical authored Intro2
spider30/child58 and ordinary Level2 allocations, exercise later oriented pair
dispatch, and assert no task age, position or RNG advance before removal.
Repeated native attachment/delivery compares the genuine child16750
release with and without the second10B70, preserving independent
callback state, one deferred queue entry and one linear contact receipt.
Other delivered child families retain their existing source custody boundaries;
this Type9 publication proof is not borrowed for them.

Empty delivery is a separate unresolved boundary. Only418500 explicitly returns
BE5B0 for count0. Executable418590..4185BA instead decrements count
unconditionally, then reads the destroy callback at `rows + count*12 + 4`;
D0B0 calls it directly at40D11D. If18640 already compacted the only child,
that reads `rows-8`, and a nonnull callback also consumes the handle at
`rows-4`.18330 allocates these rows separately through4572B0; those preceding
bytes are outside its initialized row storage. Native delivery therefore
retains the235D0 prefix and reports this boundary, without inventing BE5B0,
C690 or a successful collision continuation.

Type9 CE90 ignores the packaged DC50 relation argument, so CAD0's captor and
443D10's child-self argument reach the same living release selector. A carried
Class14 corpse instead has a null release hook: clear its relation while
preserving the exact corpse task and age. Release planning must not reject a
corpse simply because the living Carried None owner is no longer present.
If18640 compacts that dying child's row first, the child's next12DA0 visit
detects absent membership in its still-live parent and runs16750/null release
before continuing with its retained callback-entry flags. Native Type9 and
Type8 corpse tasks keep their age and private state through this repair.

`DB80` calls the current style+2C, disposes that result, checks that the actor
survived, then **rereads entity+BC/+C0** for the continuation. It does not freeze
the pre-cleanup root/context. This creates an observable death asymmetry:

- Direct10C10 death commits parent health0/dying before D040 releases its child.
  Successful D040 enters dyingC690 and publishes C620 once. OuterDB80 rereads
  the new AC40 continuation and publishes C620 again. The second Primary
  survives; both constructor RNG draws and task clears belong to the program.
- A normal primary hit invokes DAC0/style28/D040 while the parent is still
  alive. Successful cleanup enters livingC690. A subsequent lethal checked
  packet then usually reaches a null death hook and publishes C620 once.
  DAC0 disposes a retail D040 error and still continues the hit.

The released child's selector must see the parent's already-committed dying
state on the direct-death path. Planning candidates before that prefix would
change its behavior and RNG. A final lethal-primary snapshot cannot prove the
double publication on direct standard death. These source results retire the
old nested-cleanup capture gate.

The same cleanup context is threaded through primary/infected hits, checked
static contact, Playing/Intro2 radial death, living E370 expiry and Main Base
abort. Aborts fork and commit notification state with the existing speculative
manager/task/effect transaction, so a later unsupported actor cannot leak a
released child's hint into the live session.

The Hive67 destination's generic surviving15040 damage path is available; its
lethal10C10/Sub-K/radial continuation still needs its complete native owner.
Missing-parent180F0 repair, player contact and unsupported counterpart owners
remain separate boundaries. The [shared spider contact](INTRO2_TYPE17.md) now
also runs Follow/Run Away/Class12 nonplayer pairs through current native task
callbacks. Controlled port capture/release scenes do not establish the retail
intro's natural target choice or capture timing.
