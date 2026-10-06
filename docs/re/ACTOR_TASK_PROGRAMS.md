# Authored Actor Task Programs
Verbatim home of the recovered per-task program families, moved from ACTOR_RUNTIME.md during the 2026-08-24 cohesion split. Dispatch architecture and behavior selection remain in the runtime router.

### Type-9 Run Away and Attract Attention task programs (STATIC + RUNTIME CONFIRMED)

Run Away descriptor `0x004C88E0` starts at style `0x004C7618`. Initializer
`FUN_0040B6C0` clears slot 2, prepares the slot-1 acquisition task, then
prepares a concurrent 500-ms slot-0 `FUN_00402BA0` task; earlier mutations
survive a later preparation failure. Accepted acquisition stores the target
and advances to style `0x004C7660`. Its initializer clears slots 1 and 2,
then prepares one 2,000-ms slot-0 `FUN_00403F40` flee task. Successful
allocation first runs the shared generic suffix: one RNG word, Sub-A direction
1, then randomized base speed. The Run Away initializer subsequently
overwrites that target speed with signed `(descriptor base * 5) / 3` before
publication (Type-9 base 250 therefore becomes 416). Failed local allocation
consumes no constructor word and preserves the old Primary until the enclosing
selected-initializer fallback clears Secondary, Tertiary, then Primary. The
callback validates its target before sound or movement and retains the authored optional-sound gate before using the
`FUN_00423030` route predicate. First-world retail Type-9 stores sound 85 at
`+0x9E` and period zero at `+0xAC`, so this actor emits no flee cue and consumes
no sound RNG; the demo stores sound 87 with the same zero period. The matched
branch temporarily
reflects the target point as `2 * owner - target`, calls the common mover,
and restores target position and handle on both Boolean returns; the other
branch calls the mover with the original target. Their three tagged
singletons remain pointer-distinct, and strict 2,000-ms expiry is evaluated
only after callback-result precedence and wrapper unwind. A tagged callback
result wins over timeout; state bit `0x1000` suppresses both root selection and
timeout, and a clear gate invokes the generic root selector rather than forcing
Run Away. Style `0x004C76A8` is statically bounded but was not executed by
either focused capture.

Attract Attention descriptor `0x004C8998` starts at style `0x004C86B0`; both
retail and demo store initializer argument `+0x44 = 0x00000201`. Initializer
`FUN_0040BA40` first consumes one shared parity word. Odd parity prepares the
slot-1 candidate task while even parity clears that slot. Every successfully
prepared candidate, cue, or wander task then passes through
`FUN_00406030/FUN_00406070`, consuming one additional shared word, writing
Sub-A direction `+1`, and publishing the randomized descriptor-base speed
before the new destination wrapper is installed.

After the candidate phase it dispatches resource event `0x10`, prepares and
publishes the 1,000-ms slot-2 attention cue, optionally emits the Sub-I
descriptor `+0x04` positional sound (72 for Type 9) at full gain/rate, and then
sets only the Sub-I forced-stop byte. It finally prepares the 1,000-ms slot-0
shared-retarget task, performs that task's constructor RNG/Sub-A reset,
overwrites only the target-speed dword with literal 1, and publishes Primary.
Thus a successful initializer consumes three words on the even path or four on
the odd path; including the separately preceding behavior selector, production
totals are four and five. A failed preparation consumes no word for that phase,
preserves its old destination, stops all later actions, and retains every prior
clear, publication, RNG/component write, event, sound, and forced-stop effect.
The outer `FUN_0040C6B0` owner later maps that nonzero result to fallback
descriptor/style `0x004C8888/0x004C74F8` and clears slots 1, 2, then 0; the
detached initializer deliberately does not claim that live owner.

`FUN_004568B0` and demo `FUN_00456210` are exact instruction matches. They
forward event `0x10` and the callback's current millisecond timestamp
`(50-Hz tick * 1000) / 50` to the canonical resource slot only in gameplay
phase 5. The exact matched callee sets mask bit
16 once, maps the event through `DAT_004CAD80[16]` to global Section-2 resource
`0xF0`, and never refreshes a duplicate's slot or timestamp. Fresh production
now retains each already-committed request independently of actor lifetime and,
after selected Run Away/Go-To-Job/Wander adoption but before manager publication,
atomically
prevalidates and drains the complete batch through that owner. A malformed
later receipt leaves both manager custody and notification state untouched;
all valid duplicates are consumed while canonical dedup preserves the first
timestamp. The port admits this deferred callback only on its authenticated
fresh-New-Game post-intro handoff, corresponding to retail's phase-5 guard, and
resets its physical clock immediately after successful construction. The
load-only drain therefore explicitly rebases the retained request to the port's
new level-clock origin zero; this is not a claim that retail's absolute
construction timestamp is zero. No runtime capture is required for this drain.

Rust now executes this complete initial transaction behind a data-bearing
adapter and pins every allocation-failure boundary. Class 45's canonical
`BehaviorProgram` retains the exact `0x201` argument, so the executor cannot
accept an arbitrary candidate filter. The receipt-bound fresh-Type-9 adapter
now authenticates canonical choice 1 and the common pre-publication state,
without treating Player-Nearby's selector witness as a future target. It
publishes the selected prefix, delegates this exact transaction, and consumes
the result into either `AttractAttentionPublished` or the outer initializer-
fallback custody. Dedicated candidate and cue runtime families retain their
installing origin. Successful publication returns one non-duplicable initial-
graph owner bundle: even parity carries the exact cue owner, while odd parity
carries the exact candidate and cue owners together. Their captured task
identities prevent either later slot from being visited after an earlier-slot
replacement. The heterogeneous dispatcher remains fail-closed so it cannot
partially visit the shared local-wander task around either dedicated boundary.

This receipt bridge models only native fresh-context allocation's success path.
Retail `FUN_0040ABE0` and demo `FUN_0040ABF0` allocate the 0x1c-byte context
after selector RNG but before the style switch. Allocation failure propagates
unchanged, enters neither the selected initializer nor unnamed fallback,
destroys the provisional entity/component aggregate, unregisters its handle,
and skips basis finalization and live-list append. The selector receipt is
therefore consumed terminally. Rust's inline context cannot fail, so the
detached shared outer owner injects that one decision while linearly retaining
the still-unlinked entity and receipt. Exact preflight runs before the injected
allocator; success returns an opaque crate-private continuation, while either
preflight rejection or allocation failure drops both authority-bearing values
and exposes diagnostics only. It cannot enter branch fallback or retry. The
fresh production composer places this boundary between selector issuance and
exactly one initializer dispatch; no capture is needed.

Matched retail/demo C and the live Rust owner now close the complete accepted-
candidate-to-target-style transaction. The dedicated candidate exact visit
reads the actor's separate common-axis runtime (authored range `0xF00`),
applies the accepted one-in-four gate's filter overwrite `0x201` at its
`+0x04`, and selects the first eligible candidate
in intrusive order. Its non-forgeable handoff stores that handle before
`FUN_0040C6B0` publishes variant + 1 / style `0x004C86F8`; no selector RNG is
repeated inside the handoff. Target-style initializer retail `FUN_0040AF50` /
demo `FUN_0040AF60` then clears Secondary followed by Tertiary and fallibly
prepares the 5,000-ms shared target-route task from context `+0x08` and the
owner's current position. Successful preparation consumes exactly one generic
constructor word, resets Sub-A, overwrites target speed with signed
`(base * 4) / 3`, and publishes Primary last. The installed task uses the
distinct Attract-Attention target-route runtime family even though its stable
state shape and callback are shared with other target routes.

Preparation failure consumes no constructor RNG and preserves the old Primary
at the nested initializer boundary. The outer style-switch owner then retains
the already-written target/style, applies terminal unnamed fallback, and
clears Secondary, Tertiary, then Primary. On success the candidate wrapper has
self-retired during its own exact visit, so its stale tagged result is
discarded rather than visiting the newly published Primary in the same pass.
This boundary is statically closed and needs no new focused capture. Production
now binds the retained route to the exact scheduler, predicate, selected common
mover, root/class-6 transition, F70, and outer tail without revisiting the
newborn Primary in its publication frame. Candidate acquisition,
`FUN_00402BA0`, and the target-route callback remain shared task
programs rather than behavior-specific copies. The short July 30 Intro2 trace
did not reach an Attract Attention installation and cannot establish anything
about its later cinematic use.

The corresponding static path is now bounded independently of the unfinished
AI scheduler. Type 47's exact 28-byte Section-12 Sub-E selects projectile
method 30 `{0,3000,87,0}`, a 750,000-us stochastic interval, spread 100,
strict wrapped `+-0x600` X/Z target bounds, `+-16000` aim tolerance followed
by a forward-half-space gate, and positional sound 70. The common creator
consumes no shared RNG before its behavior/mode pre-gates, one cadence draw on
a rejected attempt, and cadence plus two spread draws on a successful attempt. Class 87
uses executable-backed sprite frames 840--842, not the 798--800 list owned by
classes 52/53. Its exact damage packet is channels `[2,6]`, raw amounts
`[1000,1000]`; target profiles perform the filtering. `FUN_00442420` converts
surface selector 6 to class 73 and selectors 0--5 or 8--12 to class 88 in
place; selector 7 and values above 12 return one without replacement, so the
enclosing dispatcher deletes the parent. If a water conversion reaches the
same traversal's ground tail, replacement callback `FUN_0043E3D0` converts
selectors 0--6/8/10 to class 73, makes one class-43/75 child attempt before
deleting on selectors 7/9, and deletes directly on selectors 11, 12, or above.
For the selector-7/9 request it copies parent particle `+0x1D` bit zero, but
loads the owner handle from current `DAT_004DCA00` rather than particle
`+0x18`; `FUN_00440A60` resolves the corresponding birth-time entity type.
Rust exposes the supported class-87 allocator-request slice and now consumes
its live collision outcomes at the common physical-scan F590 seam without
broadening the target claim. Entity hits with an unsuppressed birth-provenance
packet enter the existing audited Main-Base/Working-Factory
`FUN_0043F590 -> FUN_00410EB0` damage/audio bridge before parent deletion;
other entity types continue to fail closed. Runtime particle `+0x1D` bit zero
still suppresses only damage, and class 87's `FUN_0042E8E0` static hit remains
a true no-op after the parent is consumed. Routing note (2026-08-25): class 87
shares only the `FUN_0043F590` entity-hit slot with the class-1/3 primary
family — its `+0x20` packet is `0x004CC0F0` (class 1 is `0x004CBF70`, class 3
the adjacent `0x004CBF88`) and its `+0x24` static callback is `FUN_0042E8E0`
(primaries use `FUN_0043F800`) — so the port's primary collision gate routes
on that full pair. Gating on the entity-hit slot alone sent ant projectiles
into the class-1/3 water/terrain surface policy, which panicked; gating on
class 3's packet alone dropped default bullets. For captured fresh-Level-1 spawns
11/12/13, an authenticated already-published Guard-Pursuing/Aim lease now owns
shared cadence RNG, transient append-before-acknowledgement, sound 70, and a
separate exact-once class-87 drain. Production Guard acquisition, ADE0 same-pass Aim, later pursuing
Chase-then-Aim, the later live-list class-87 drain, and death
ownership are live; `FUN_00403490` live-target ignores constructor-unknown
`SURFACE_STATE_MASK` bits. Chase and Guard wander apply the
`V200003.run` full-reset first-query owner for Level-1 Type-47 seeds
`0x2B/0x2C/0x2D`.

All three actor types install model callback pair
`{FUN_0040D320,FUN_0040D350}`. Selector zero is the wrapping 50-Hz clock.
For nonzero selector N, `FUN_0040A950` returns word N-1 from the live
component-array reached through entity `+0x4C/+0x08`. Real-model tests prove
`man2` consumes selector 1 and `spider` consumes none of selectors 1--63.
The peasant writer uses its exact Sub-I state rather than a synthetic model
clock. The eight direction groups, help/carrying and death frames, cue
destructor, and detailed/coarse update ownership are documented in
[peasant animation](ACTOR_RUNTIME.md#peasant-walking-and-attention-animation).
The forced-stop branch zeroes velocity before the common Sub-A/Sub-B tail;
the special branch takes precedence and consumes no RNG. Recurring sound
timing remains owned by the attention task.
Attract Attention statically uses
Sub-I `+0x04` (global sound 72, PCM alias 15), a 1000-ms cue and local-wander
task, and a separate 5000-ms accepted-target route. There is no blanket help
cooldown: selecting class 45 again executes its initializer and sound again.
Retail `FUN_00438050` is an unconditional zero-return leaf;
the demo uses the structurally identical shifted leaf `FUN_00402B40`. The
generic owner first adds `elapsed_us / 1000` with per-call truncation and u32
wrapping, then enters and unwinds the leaf. Exactly 1000 ms remains active;
only `elapsed > 1000` can call the type-default/root-first selector, and only
when the live post-unwind state bit `0x1000` is clear. A set bit retains the
expired cue for a later retry. The exact Rust Tertiary owner now preserves
these rules, retains the behavior target/auxiliary words independently from
the candidate's mutable common-axis filter, rejects changed
Primary/Secondary/Tertiary leases before advancing time, and returns a typed
root request instead of hard-coding the observed next behavior. The cue leaf
consumes no RNG, so matched C closes it without another capture. The friendly
trace independently shows the initial one-second transition and later
five-second route recurrences; sound starts at 30.950 and 31.950 seconds also
prove that a one-second repeat can occur. At 21.000 s it replaces the task wrapper while
style `0x004C86F8` remains installed: task identity/lifetime is not derivable
from style address alone. Sound 72 varies pitch, volume, and pan across the
retained events, so its PCM-15 alias is not a fixed playback packet.

The authored `spider` mesh (global model 256, local model 243 in `1X3XX.OVL`)
contains 16 type-14 external-frame vertex records as well as 17 type-13
view-dependent records. `FUN_0046ECF0` sends each type-14 record's authored
`(a,b,c)` through `FUN_0040D350` and `FUN_0040A9F0`. For the spider,
`FUN_0040A9F0`'s Sub-H branch maps those 16 selectors onto eight paired,
entity-owned 0x3c-byte component records before `FUN_0041D360` resolves each
output point. The component state is reached through
`entity +0x4C -> +0x0C -> +0x04`. The absence of a consumed model-clock
selector therefore does not make the legs static and does not authorize a
clock oscillator. The exact detached `FUN_0041D0A0`/`FUN_0041D120` writer is
implemented: all records update once per call, the first-record cursor rotates,
signed phase arithmetic and dependency gates are preserved, completion cues
remain ordered, and the paired 16-selector binding is tested against the real
spider data. `FUN_0041D360` is also represented by an exact detached
fixed-point transaction covering conditional C/A/B resolution, cache updates,
terrain/external-surface constraint search, animated sine arch, and all six
secondary-axis modes. D360 is a model-presentation callback, reached through
`6ECF0 -> 0D350 -> 0A9F0`, not an `01430` or `12DA0` mover suffix. Constructor
`FUN_0041D2A0` still zeros flags; bit `0x02`/`0x08` from D360 are
`COPY_TARGET`/`WAIT_FOR_DEPS` for `FUN_0041D0A0`, which is how a stride
starts. Intro2 Type-26 and Type-47 now retain this live writeback at model
submission. The selected command branch and `45A4E0` authored-normal test
precede selector invocation; only requested original face corners, edges,
billboards and screen-midpoint projector dependencies resolve endpoints.
Near rejection follows the callback. Missing models, skipped actor submissions
and the final black card do not precommit the other Sub-H records. The root
actor's source slots, toroidal origin and physical basis remain the callback
context through child models. Live D360 must pass the
`FUN_00413F70` Q31 words into `FUN_0041DF20` unshifted: DF20 and the
secondary basis already narrow with `>> 0x13`. An extra `>> 12` collapses
the up-axis to 0, so the constraint search never leaves rest C and the
knee cross-product is zero (one hive newant plants trailing feet while
another is mid-stride). Ordinary gameplay and Intro2 now share reached,
draw-owned Sub-H callbacks; movers invalidate caches and each admitted model
command resolves only its dependencies. Actor model callbacks precede that
actor's FIFO drain, and the single particle presentation pass follows the
complete manager-order actor visit. The [render owner](MODEL_DRAW_CUSTODY.md#authored-insect-body-and-leg-shadows)
owns VIEW-cache, raw normal-gate and surface projection custody;
[Type47](TYPE47_RUNTIME.md#authored-draw-and-firing-origin) owns its muzzle
stamp and current queue epoch. `464E60` far-radius rejection runs at the hierarchy boundary
before materialization and D360, including descendants and billboards;
header `0x20` bypasses only its owning node. Type-17 rebuilds `FUN_00413F70`
after Sub-D yaw.

DF20 narrows each interval difference to a signed WORD before dividing by2
toward zero, at both its first midpoint (`41DFCC/41DFE6/41DFFE`) and repeated
midpoint (`41E2A4/41E2BA/41E2D2`). Keeping the full signed32-bit difference
instead takes a false route across a65536-unit seam. The shared helper now
retains the native narrowing for every actor's terrain-constraint search.
The bounded original-PE oracle
executes the complete `41DF20..41E309` routine, including all four inline
terrain probes, with explicit signed center/Q31 axis/excess and the full real
world13 terrain bytes. Three ordinary newant moving-input controls independently
produce `[-18868,-576,-32768]`, `[-18366,-368,-32768]` and
`[-18679,-505,-32747]`, matching every repaired probe; the former port result
instead reached Z near-18400. These are controlled arithmetic comparisons,
not recorded retail actor poses. The oracle permits surface policy0 only,
has no callback stubs, rejects missing/borrowed memory and unaudited calls,
and limits execution to5000 instructions. Ten fail-closed guards and11 focused
tests cover both X/Z seam signs, negative odd division, zero-excess Y/Z seams,
read-only inputs and loop bounds. Policy1's wave path remains outside this
oracle; the shared runtime's separate wave/surface controls are unchanged.

### Search And Attack Target class 7 task program (STATICALLY CONFIRMED)

Named-behavior entry `0x004C8AD8` (`"Search And Attack Target"` at
`0x004C920C`) points to the two-dword descriptor at `0x004C88A8`:
`{ style_base = 0x004C7A50, 0 }`. The style base contains three contiguous
0x48-byte variants. This corrects the former interpretation of
`0x004C7D68` as another class-7 style; that address is the following behavior
descriptor's base.

| Variant | Address | Non-zero program fields |
|---:|---:|---|
| 0 | `0x004C7A50` | `+00 FUN_0040C690`, `+04 FUN_0040C7D0`, `+08 FUN_0040CE70`, `+20/+24/+28 FUN_0040C690`, `+38 0x00021080`, `+40 FUN_0040B6C0` |
| 1 | `0x004C7A98` | `+00/+04 FUN_0040C690`, `+08 FUN_0040CE70`, `+20/+24/+28 FUN_0040C690`, `+38 0x00000080`, `+40 FUN_0040ADE0` |
| 2 | `0x004C7AE0` | `+0C FUN_0040CE90`, `+34 0x00000080`, `+38 0x00000002`, `+40 FUN_0040ADB0` |

`FUN_0040C6B0(entity, behavior_context, variant)` installs
`style_base + variant * 0x48` and records the variant in program state `+0x18`.
On successful search, `FUN_0040C7D0` copies the returned target handle into
behavior context `+0x08`, computes variant + 1, and re-enters
`FUN_0040C6B0`; C6B0 performs the actual `+0x18` variant write before style
publication.

Variant-0 initializer `FUN_0040B6C0` copies the type-authored word at type
subrecord `+0x04` into component state, clears task slot 2, and installs
`FUN_00402050/00402080` in slot 1. The raw `FUN_0040B6C0` branch follows the
same proven constructor convention as the Guard and Defecate programs: zero
means the slot-1 task was published successfully. Only that success path then
installs duration-500 `FUN_00402B10/00402BA0` in slot 0. Search and wander are
the normal concurrent variant-0 program, not a primary attempt plus failure
fallback; a nonzero slot-1 construction result returns without replacing slot
0. Both successful constructors run the same shared suffix before publication:
retail phase 0 follows
`FUN_00402050 -> FUN_00405F80 -> FUN_00406030 -> FUN_00401020 -> FUN_00406070 -> FUN_0040A7A0(slot 1)`,
while phase 1 follows
`FUN_00402B10 -> FUN_00406030 -> FUN_00401020 -> FUN_00406070 -> FUN_0040A7A0(slot 0)`.
Demo `0x0040B6D0` has the same structure through its shifted shared allocator,
`0x004060E0` suffix, and `0x0040A7B0` publication owner.

The port now represents both shared task families in the heterogeneous
three-slot dispatcher without duplicating acquisition state. Its shared B6C0
preparer validates the exact phase/slot/constructor/tick/lifetime contract,
retains the class-specific style-`+0x44` filter override, and for type 17
authenticates A/B/C/D/H/J topology and Sub-A base 250 for either phase. Each
successful allocation retains its own `FUN_00406070` suffix as
ordered Sub-H enable, one shared RNG draw, Sub-A direction 1, and randomized
target-speed effects which must run while the old destination task remains
installed and before replacement. A phase-0 allocation failure consumes no
constructor RNG; a phase-1 allocation failure leaves the first suffix, its draw,
and the published secondary committed. The enclosing C6B0/C6C0 owner then
installs the special fallback and clears slots 1 -> 2 -> 0 without rolling back
those component effects. Type 17's exact eight-record Sub-H runtime is
Entity-attached, and the live publisher exposes the ordered zero/one/two
successful suffix receipts across failure and success. No capture is required
for this closed transaction.

The shared constructor boundary is no longer mislabeled as type-17-only. It
now derives one of two explicit profiles from the component graph:
Sub-A-only, or Sub-H followed by Sub-A; any Sub-G/Sub-F branch remains
unsupported. Existing type-17 live callers retain their H -> RNG -> A order.
Normal-tier retail cumulative type 9 independently proves A/B/D/I, common axis
`0x0F00/0x0084`, and signed Sub-A base 250. Its detached Run Away acquiring
composition therefore clears Tertiary, prepares and suffixes Secondary, then
prepares and suffixes the 500-ms Primary with exactly two constructor words and
no Sub-H effect. Phase-0 allocation failure consumes no word and preserves the
old Secondary/Primary; phase-1 failure retains the first word/effects and new
Secondary while preserving the old Primary. The provenance-bound shared
selected-initializer preflight now authenticates known capability word
`0x1804`, masked pre-wrapper state `0x06068801` (surface bits exempt), null
relation, known birth anchor/current position, cached type policy, zero
rotation, exact metadata/components, and empty task/custody state before any
completed branch publishes. Run Away publishes the canonical
TypeDefault/null-target/zero-aux context, copies only common-axis `+0x04`, then
executes that T -> S -> P transaction. Either allocation error enters the
outer unnamed fallback, clears state bits `0x00068000` and slots S -> T -> P,
and preserves any earlier axis/Sub-A/RNG prefix. Success and fallback both move
the dormant Sub-D/anchor state into explicit terminal custody and reject replay.
Class 6 uses the same preflight and terminal custody without copying the axis:
it clears Tertiary then Secondary, and its fallible Primary consumes zero words
on failure or exactly one post-allocation word before Sub-A reset and Wander
publication on success. Class 54 also preserves the axis. The linear selector
receipt now owns the complete ordered candidate snapshot traversed by
Base-Nearby; the selected adapter accepts only same-index, id-ordered evidence
whose state/capability values monotonically refine that retained entry, plus
class-54-only capacity evidence. Caller evidence therefore cannot change list
membership, order, id, or position, forget known state/capability facts, or
contradict them before `FUN_004235F0` planning. An unresolved later-consumed
field rejects the entire refinement bundle and returns the base receipt without
mutation, allowing a consistent fuller bundle to retry. Choice 2 proves a
Base-Nearby witness, and the corrected Type-9 `0x800` / candidate `0x20` fast
pair makes a nonzero target mandatory for this exact history. A successful
plan publishes the common prefix, clears Secondary then Tertiary, prepares the
Primary, consumes one constructor word, writes Sub-A direction/randomized
speed, overwrites target speed with signed `(250 * 4) / 3 = 333`, and only then
publishes Primary. Allocation failure consumes no constructor word before the
outer fallback and terminal component-custody transfer. Class 45 authenticates
choice 1 without binding its Player-Nearby weight witness to later candidate
acquisition, preserves the axis, and composes the parity-selected Secondary,
event, Tertiary cue/sound/forced-stop, and Primary local-wander phases. Every
successful preparation consumes one constructor word; any failure keeps all
earlier component and external effects before outer fallback. Even/odd full
success consumes three/four initializer words. Fresh production now consumes
these adapter results in the native outer order: live-list tail append, all
nine Q31 basis writes, then bit `0x4`. Context-allocation failure remains a
terminal pre-link abort; initializer allocation failure instead links the
unnamed fallback. A successful class-6 publication also mints one non-duplicable
exact-Primary owner while basis and bit `0x4` are still deliberately pending.
Its live preflight admits only the finalized fresh-Level-1 Type-9 entity,
canonical Wander context, unchanged task lease, selected component custody,
known body basis, and known-set bit `0x4`. It then runs the existing exact
D/I/A/B transaction for scheduler mode zero or D/A/B for every nonzero mode,
committing Sub-D, optional Sub-I, Sub-A/B, animation, heading, velocity, and
task-private state directly in selected custody. No temporary pending copy is
created. Pre-callback rejection returns the owner; callback-entry failure
consumes it; continuation and a live `0x1000` suppression return it; a clear
gate emits a typed type-default/root-first transition and consumes it.

Selected Run Away, Go-To-Job, and Wander now have corresponding production
owners. Level load atomically extracts those authenticated branches from the
manager in stable sidecar order after validating every current allocation
lease. All three adapters authenticate normal local/callback-enabled owner
custody before the common scheduler prefix, then resolve each callback-only
dependency at its exact retail consumer. Scheduler waits therefore skip task
and mover dependencies; selected Wander commits its retarget RNG prefix before
the common mover consumes terrain, component, Sub-A, Sub-I, and physical-basis
dependencies. Its topology is bound by the initializer-minted live owner;
runtime metadata is first consumed later by root planning/application. Run Away
preserves `FUN_00412DA0` wait/carry/cap policy, fresh
Primary/Secondary/Tertiary reads, its synchronous acquiring-to-fleeing handoff,
target/audio order, and reflected/direct common-mover custody.

Go-To-Job additionally authenticates the constructor-retained target against
the live Primary and the entity-local `+0x28` common-axis descriptor against
the type metadata. Its Primary then uses the same common scheduler and exact
heterogeneous dispatcher: target validity precedes `FUN_00423030`, every live
target reaches `FUN_00401430` exactly once, and the three pointer-distinct
singletons retain their predicate/mover meanings. Generic lifetime advances
before callback, staged private state commits only if the wrapper survives,
tagged results precede strict `elapsed > 5000`, and state bit `0x1000`
suppresses the root callback without weakening post-callback custody.

Selected Wander enters the same exact dispatcher through its initializer-
minted live owner, performs the receipt-bound D/I/A/B common mover, and retains
strict tagged-result-before-timeout precedence plus post-unwind state-`0x1000`
suppression. Its transition receipt uses the same concrete root planner and
class-6 application without redrawing the retained selector plan. Production
now explicitly authenticates that class-6 self-root and the weighted class-54
Go-To-Job cross-producer result.

Every successful unwind receipt-binds the post-task angle words and pre-F70
matrix, overwrites all nine Q31 basis words, then publishes state bit `0x4`.
Root retries retain and authenticate those inputs together with the original
capped callback delta and pre-A800 `0x2F` latch; a retained selector plan is
never redrawn. Parked custody authenticates the exact F70 result and completion
bit before refusing replay. Selected Attract, Wander, Go-To-Job, and Run Away continue from
that F70 publication through the shared non-lifecycle outer tail in the same
actor-list visit. The
unconditional `+0xB2` clear belongs to that later outer callback tail; an
unresolved contribution still fails closed at the callback-mass read. The
native birth policy below supplies a defined initial value. A Run Away frame
whose outer tail is complete may transfer to Main Base
death handling. A generic-death no-op restores the exact completed-tail state
and `+0xB2` authentication snapshot, so the next frame validates that boundary
without repeating its effects. Pending F70, outer-tail, lifecycle, or root
transactions remain non-transferable. Run Away's retained root authority now receipt-binds the exact
Sub-A snapshot with the no-redraw plan before mutation and applies weighted
class 10 through its two-phase initializer, exact fallback, and F70 parking.
Go-To-Job's retained root authority now likewise binds the selector's frozen
manager-order/capacity evidence, full actor-local common axis, exact Sub-A
snapshot, and post-Primary cursor before applying weighted class 54 without a
redraw. Success or fallback is terminal, resumes at Secondary without revisiting
the newborn Primary, authenticates the final graph, and reaches F70. Matching
selected-owner roots for classes 10, 54, and 45 are closed. Selected Wander ->
class-54 Go-To-Job, selected Go-To-Job -> class-10 Run Away, selected Go-To-Job
-> class-6 Wander, selected Wander -> class-10 Run Away, and selected Wander ->
class-45 Attract Attention plus selected Attract Attention -> class-54 Go-To-Job
and class-10 Run Away, Go-To-Job -> Attract Attention, and Run Away ->
Attract Attention/Go-To-Job are ten closed cross-producer roots. The class-10
crossings authenticate their canonical lone-Primary or exact Attract initial/
route predecessors, exact retained contexts, no-redraw plans and frozen
snapshots, two-phase success-only RNG/fallback, original cursor, optional
Fleeing suffix/fixed-`5/3` word, final graph, and F70/PostBasisTail. Attract's
three cursors are SharedRetarget Primary -> Secondary, expired Cue -> terminal,
and Target Route Primary -> Secondary; progressed Main Base custody fails
closed. Go-To-Job -> class-45 Attract Attention owns the even transaction and
odd Candidate/Target Route suffix. Run Away's class-45/54 choices retain the
selector plan, predecessor components, and post-task frame across a transfer
to the existing multi-family root continuation. Its remaining Secondary cursor
visits newly published Candidate/Cue tasks before F70; it does not replay the
prefix or Primary. Completed publications become current scheduler authority,
as described in [Actor Runtime](ACTOR_RUNTIME.md#current-type-9-task-ownership).
Alternate class 14 and the generic lifecycle share the session-zero oracle.

<a id="fresh-type-9-callback-mass"></a>

Fresh construction must support either first scheduler branch. Retail
`FUN_004104b0` in `bulk_clean/game_logic.c:5154-5400` allocates the Type-9
`0xCC`-byte body through nonzeroing `Mem_Alloc`, writes `+0xB0` and clears
`+0xB6`, but never writes `+0xB2`; the demo body matches. Diagnostic
`20260817-060254-type9-callback-mass-b2.txt` shows the six later
`spawn_word3=0` births at first `FUN_00412DA0` entry with flags
`0x00468805` (callback set, wait-disable clear), not `0x06478801`.
Allocation `+0xB2` matched first-entry `+0xB2` and was not universally
zero (`0000/f59c/02ab/0000/ffff/0000`). Independent consume-site traces
`20260817-061455` and `20260817-062155` observe a same-visit `WAIT_CLEAR`
at `0x00412EBB`, with no `MASS_READ` at `0x004130F6`, for all six ordinary
births. These are observed RNG histories, not a guaranteed first wait:
wait-disable being clear permits the random wait test, whose threshold can
also select Continue. Both coarse and detailed callbacks then read `+0xB2`
before invoking the type callback. Leaving it unresolved permanently parked
the windmill peasant in `CallbackMassUnavailable` on frame zero for ten of
64 tested shared-RNG starting offsets, without player contact.

Native fresh construction explicitly initializes this transient contribution
to `Known(0)`. This deterministic allocation policy applies to the shared
authored-world Type9 constructor as well as the six authenticated Level1
fixtures; it is not a claim that the retail allocator clears the word. Only
the explicit Level1 fixture path applies its independent, one-shot first-
scheduler flags `0x00468805`. Shared native worlds and Intro2 retain their own
constructor/presentation state; B2 is never a first-visit sentinel. The original
random wait/continue decisions and RNG consumption remain unchanged.
Unpublished/imported generic fixtures retain unresolved values, and later
nonzero contributions (including `FUN_004425D0`'s particle mass addition)
survive until the scheduler's original clear sites. The live
regression varies initial RNG history and frame cadence, publishes camera
detail, and checks sustained windmill motion with the player left at spawn.

Main Base death temporarily decomposes any
selected linear scheduler owner into the manager sidecar for existing Type-9
death authentication, restores the same branch after remote/already-dying
no-ops or a preflight rejection, and consumes it before a new Exploding Person
owner is registered.

The outer post-task basis rebuild now completes before any selected owner can
be ticked again. An admitted initial multi-slot Attract frame, and selected
Wander, Go-To-Job, or Run Away after F70, continue through the shared
non-lifecycle outer tail in the same actor-list visit. Retail
`FUN_00412DA0`, demo `FUN_00412D10`, and the exact-matched helpers establish the
order: E100 applies ordinary `0x2F` drag, DF70 snaps Y from the old X/Z terrain
sample and zeroes vertical velocity, then E370 commits its timer and chooses
either lifecycle or random effects. The random branch plans and materializes
its optional bubble before drawing the independent sound gate. The outer caller then clears `+0xB2`
unconditionally before re-reading live master state and integrating motion.
The adapter retains the originating retail tick across bubble acknowledgement,
authenticates the exact completed state, and never revisits a newly published
Target Route Primary in that frame.

E370 lifecycle remains linear rather than guessed: both Attract styles
`0x004C86B0` and `0x004C86F8` have statically audited null release/death hooks,
but those null hooks do not erase the fixed `FUN_00416750` release prefix or
following `FUN_00410C10` standard-death call. Standard death calls Type-9
vtable `+0x08` `FUN_0040DB80`, which invokes optional style hook `+0x2C` and
then entity callback `+0xBC` `FUN_0040AC60`; the dying bit forces direct
class 14. Static ordering is closed. The accepted Main Base trace has session
byte `+0x28F = 1` and therefore suppresses message `0xC6`.
`20260817-072421` closes the session-zero branch: capability `0x1804`
requests `FUN_00456900(0xC6, 0)` then `FUN_004032A0(entity, 0, 1000)`.
Selected-owner E370 now reaches that initializer through the generic Type-9
standard-death publisher, consumes the selected owner after class 14, and
adopts the inner class-14 SharedRetarget owner and runs the ordinary
latched-`0x2F` E870 suffix. Later E370 visits apply `FUN_00416750` then
already-dying `FUN_00410C10` and continue. The 1,000-ms class-14 task
reaches `FUN_0040C470`, stages shared deferred destroy, still runs that
suffix, and the manager sweep unlinks. Class-14 visits now run the shared
`FUN_00412DA0` prefix; INSTALL `0x06C64825` wait-disable skips the random
gates. Specialized Wander/Go-To-Job now complete the shared F70→outer-tail
visit once the exact fixtures carry Known `+0x48` and surface-classifier
bits.
The accepted
`20260730-142219-villager-task-mover.txt` transcript plus matched retail/demo C
and original data close the non-lifecycle tail without new evidence.

The accepted two-process first-visit wait traces and the unaccepted diagnostic
`20260817-060254` have distinct evidence status; neither establishes a general
zeroed constructor invariant. The [native initialization policy](#fresh-type-9-callback-mass)
is limited to the six authenticated births.

The production dispatcher closes the newborn Attract slot schedule without
waiting for a new capture. A Primary transition that installs
Candidate Secondary and Cue Tertiary exposes both through later-slot re-reads
in the same pass. Candidate runs under generic callback unwind after one gate
RNG word; acceptance may replace its own wrapper, clear Tertiary, and install a
target-route Primary, so its stale result is ignored and neither the cleared Cue
nor the already-passed Primary runs. A pending Candidate leaves the Cue visible;
the zero leaf then advances by the dispatcher's same frame duration and invokes
its post-unwind root boundary only at strict wrapping `elapsed > 1000`. The
adapter retains the synchronous Target Route owner or initializer fallback,
publishes F70, and parks the result under stable manager custody. On the next
production visit, completed outer-tail custody is consumed exactly once before
the dedicated Target Route callback enters the normal scheduler; its staged
mover writes, singleton/lifetime precedence, suppression, root retry, F70, and
outer tail remain linear. Main Base transfer remains limited to the unchanged
initial P/S/T graph; progressed Attract owners fail closed. Retained root-plan
retries bind the exact task states, context, selected component,
`initial_behavior`, and Sub-A snapshot and reuse the plan without a selector
redraw. Wander -> weighted Go-To-Job, Go-To-Job -> weighted Run Away,
Go-To-Job -> weighted Wander, Wander -> weighted Run Away, and Wander ->
weighted Attract Attention plus Attract Attention -> weighted Go-To-Job and
weighted Run Away, Go-To-Job -> Attract Attention, and Run Away -> Attract
Attention/Go-To-Job are ten closed cross-producer roots. Attract -> Run Away retains
SharedRetarget Primary -> Secondary, expired Cue -> terminal, and Target Route
Primary -> Secondary origins, frozen plan/snapshots, its two-phase transaction
or fallback, optional same-pass Fleeing, final F70/PostBasisTail, and fail-
closed progressed Main Base custody. Go-To-Job -> Attract owns both parity
branches; Run Away -> Attract/Go-To-Job shares the frozen-plan continuation
described above. Alternate class 14 and
generic lifecycle share the narrow session-zero oracle, independently of the
fresh-birth `+0xB2` initialization policy above.

`FUN_00402080` delegates acquisition to `FUN_00422CD0`. That search walks the
global intrusive entity list, rejects the owner, zero-state and `0x4000`
dying/deferred candidates, and candidates suppressed by the symmetric recent
relation, then applies the type/mask filter stored in component state. State
bit `0x1000` is not an exclusion in this helper. Mask zero requires the same
entity type; a nonzero mask replaces that test and instead requires any matching
candidate capability bit. It measures wrapped signed-16 XYZ distance (each
squared term shifted right by 2) and selects the strictly closest candidate
inside the squared radius; equal-distance ties retain intrusive-list order.
Radius zero is the helper's unbounded mode and no match returns success without
writing the caller's output handle. The same zero sentinel is retained after a
zero-distance match, so the following accepted candidate replaces that match
regardless of distance before ordinary closest-distance comparison resumes.
Final return status is independent of output mutation: a nonzero radius returns
tagged result `0x004BE8C0` whenever the final best distance still equals the
original wrapping `radius * radius` sentinel. Normally that means no match, but
a wrapping-square/quantized-zero edge can write a handle and still return the
tag. Target selection consumes no RNG. `FUN_00423030`, used later by the chase
task, is a separate strict axis-aligned range test: all three wrapped absolute
coordinate deltas must be less than the configured radius; radius zero again
bypasses the bounds, while a negative nonzero radius always fails.

The surrounding slot-1 callback contract is closed too. `FUN_00402050` is a
thin callback-owned constructor: it has no duration or timeout. At each
`FUN_00402080` tick, constructor override zero preserves the type-authored
filter, an ordinary nonzero value replaces it, and `0xFFFFFFFF` normalizes to
mask zero before `FUN_00422CD0`. The selector's `0x004BE8C0` no-target tag is
consumed and returns zero without invoking the behavior callback. A successful
selection with no behavior callback also returns zero. If the callback exists,
its zero return maps to singleton `0x004BE1B0` (generic-owner tag `0x9C02`);
every nonzero result propagates unchanged. Retail's unbounded
success-without-output edge retains stale stack storage, so portable code fails
closed instead of inventing a handle. The behavior handoff is synchronous and
may replace slot 1 and publish slot 2 while the callback is unwinding; stale
output from the retired wrapper is discarded and the newly published later
slot is still visited in the same owner pass.

The accepted Intro2 capture
`runtime_re/captures/local/20260722-022303-intro2-actor-ai.jsonl` validates the
program in live retail state. Ptersect handle `0x04970001` (type 13, model 291)
remains in variant 0 through approximately 31,900 ms with slot-1 tick
`0x00402080` and an already-present slot-0 `0x00402BA0`. It then selects target
handle `0x047F0001`, switches to variant 1, removes slot 1, and installs
slot-0 chase tick `0x00403490` plus slot-2 aim/fire tick `0x00402300`. Both
new tasks carry the exact 5,000-ms duration.

Variant-1 initializer `FUN_0040ADE0` is therefore the normal attack setup (not
`FUN_0040AEA0`). It optionally plays the type-subrecord `+0x9A` sound through
`FUN_0044F480` at entity `+0x96` when that word is nonzero, creates
the duration-5000 slot-2 task using type values `+0x9C/+0xA8`, clears slot 1,
and creates the duration-5000 slot-0 task with the same target. Slot 2 feeds
the type-authored weapon descriptor and mutable emitter state into generic
projectile creator `FUN_00424650`; cadence, spread, optional firing sound and
alternating-barrel state may consume the shared RNG. Slot 0 validates the
target, applies `FUN_00423030` and the `0x380`/`0x400` X/Z proximity bands, but
delegates actual locomotion to `FUN_00401430`. A failed slot-2 install leaves
slot 1 untouched and skips chase; a later chase failure leaves slot 2 alive
after slot 1 has been cleared. Variant 2 clears slots 2 and 1 and installs
`FUN_00403230/00403250` in slot 0; failure leaves both cleared. Its external
event entry/task semantics remain unresolved. Its `FUN_0040CE90` completion
marks `0x8000`, finishes the component route, and reselects behavior.

### Shared Aim And Fire slot-2 task (STATICALLY CONFIRMED)

`FUN_00402220(owner, slot, lifetime_ms, target, optional_sound, sound_period)`
constructs the shared Aim And Fire task used by class-7 pursuit. It installs
tick `FUN_00402300` and destructor `FUN_00407120` through the common
`FUN_00401350` task initializer. That initializer owns the 0x24-byte
allocation (`push 0x24; call FUN_004572b0`) and hands the fresh record to the
shared private-state constructor `FUN_004012E0`, which stores the owner's
construction-time XYZ at `+0x00/+0x02/+0x04`, target handle at `+0x08`, zero
at `+0x0C`, direction
`+1` at `+0x10`, reversal timer zero at `+0x14`, and another zero dword at
`+0x18`; `FUN_00402220` then writes the optional sound resource and signed
period at `+0x1C/+0x20`. Only successful allocation reaches the constructor
suffix: if normalized Section-12 Sub-F (type-state base `+0x1C`) exists,
`FUN_00424390(component, 1)` selects mode byte 1 and phase `0x1000`, then the
task is published in the requested slot. Allocation failure returns nonzero,
does not run Sub-F, and preserves the previous destination task.

The generic scheduler advances the task's truncated millisecond lifetime before
calling `FUN_00402300`. A nonzero scheduler mode suppresses the complete
callback and consumes no RNG. Otherwise a missing target, a target whose state
flags are zero, or one carrying `0x4000` returns singleton `0x004BE158` with
generic-owner tag `0x9C00`. That tag requests the style's `+0x04` transition
after wrapper unwind, subject to the generic owner's callback and entity-state
gates; it does not guarantee that a transition runs. The tagged invalid-target
result takes precedence over an already-expired lifetime.

For a valid target, a nonzero optional sound and signed period of at least
`0x400` consume exactly one shared-RNG value before any emitter work. Retail
computes unsigned
`q = ((elapsed_us << 6) wrapping in 32 bits) / (period >> 10)` and submits the
positional sound at owner `+0x96` with frequency and volume multipliers both
`0x00010000` exactly when `q >= random.low16`. This sound can play even when
there is no weapon emitter. A null normalized Section-12 Sub-E (type-state base
`+0x18`) then returns zero. Otherwise the callback invokes generic emitter
`FUN_00424650(owner, owner, target, null, sub_e_descriptor,
task_component_runtime+0x2C, elapsed_us)`. Its zero result falls through to the
generic scheduler; every nonzero result propagates before lifetime handling.
The 5,000-ms task expires only when accumulated elapsed time is strictly greater
than 5,000 ms, and expiry merely requests the owner's ordinary timeout
transition after its own gate.

The detached `v2k-game::aim_and_fire` shell preserves this allocation suffix,
private state, invalid-target tag, optional-sound RNG gate and ordering, typed
seven-argument emitter request, nonzero propagation, and tag/timeout precedence.
The companion `v2k-game::generic_projectile_emitter` phase machine now closes
the complete `FUN_00424650` control transaction. It preserves entry cadence and
shared-RNG consumption, target/manual mode-zero exits, same-allocation cached
source refreshes at each raw dereference, fresh handle lookup inside each
transient append, target lead/gravity mutation across catch-up shots, per-shot
manual-angle reads, basis -> optional joint -> final-vector order, primary and
auxiliary append failure propagation, alternating selector/counter writes, and
fresh positional-sound lookup. Invalid divide, dereference, or nonterminating
states become explicit durable evidence blocks rather than guessed behavior.
The ordinary type-47 firing core remains bounded to two authenticated cohorts,
not interchangeable with generic owners: Level-1 spawns 11/12/13 target type
46, while Intro2 spawns 6/7/8 target the captured type-9 allocations. Both use
the exact Sub-E payload and shared Aim/emitter transaction; separate wrappers
own graph, target-type, Sub-D, and scheduler custody. Generic live adapters
remain external. The accepted Intro2 type-13 5,000-ms slot-2 topology remains
a separate method-10 / sound-75 / speed-2400 adapter and does not authorize
this type-47 core.

The normal-tier first-world data and focused enemy-AI capture also close a
narrow live-provenance boundary. Authored Section-13 spawns 11, 12, and 13 are
the exact type-47/model-302 cohort, and their Section-12 record carries the
exact Sub-E descriptor and A/B/C/D/E/H/J topology retained by
`ordinary_type47_live`. Fresh-New-Game construction stores that provenance,
mutable emitter state, and a transient FIFO; replay/load-compatible construction
and every type/model/index/descriptor mismatch retain nothing.
`type47_initial_behavior_live` now consumes one selector word and publishes
the authored Always x9 Guard Location or Always x1 Wander Near graph for all
three spawns before link. Guard copies type-authored common-axis `+0x04`,
clears Tertiary, and installs Secondary acquisition plus the 5,000-ms Primary
wander companion; Wander clears Tertiary/Secondary and installs only that
Primary. Each successful allocation runs the shared Sub-H-then-Sub-A
`FUN_00406070` suffix. Style `+0x44` is the EXE-proven zero word for both
programs, so the acquisition override performs no filter write. The cold
Level-1 stream selects Guard for all three births. `+0xB2` stays unresolved.
Authored spawns 11/12/13 bind the synchronous live coordinator with an
already-published exact class-32 variant-1 Pursuing context, player target,
Primary Chase, and Tertiary Aim lease. Successful Guard/Wander graphs now
enter the specialized scheduler at Level load. Guard acquisition consumes
one shared RNG word and walks `FUN_00422C10` with the authored common-axis
pair. The EXE class-32 table at `0x004C7BB8` has `+0x04 FUN_0040C7D0` and
`+0x44` zero; variant 1 at `0x004C7C00` has `+0x40 FUN_0040ADE0`. A found
candidate therefore stores the target at `+0x08`, publishes variant 1, and
installs Tertiary Aim / no Secondary / Primary Chase. A later pursuing
visit runs slot-0 `FUN_00403490` before slot-2 Aim: lifetime, live target
validation, and `FUN_00423030` against the authored common-axis pair.
`FUN_00403490` tags InvalidTarget on lookup failure, dying bit `0x4000`, or
a zero `+0x08` word; constructor-unknown `SURFACE_STATE_MASK` bits are not
that test, so a known nonzero remainder is live. Published Level-1 Guard
graphs now queue class 87 through same-pass ADE0 Aim and a later pursuing
Aim visit, then drain in live-list order.
Type 47 uses scheduler mode 0. Both authenticated cohorts start the shared
`FUN_00401430` frame machine on the authored A/B/C/D/E/H/J route
(normal dispatch selects Sub-H, not Type-9 D/I/A/B). Level-1 Type-47 applies
the `V200003.run` full-reset owner for seeds `0x2B/0x2C/0x2D`; Intro2 applies
its separate entity-owned full-reset pair for seeds `0x06/0x07/0x08`.
Both then run shared `FUN_0041F660` yaw for `classifier_flags` `0x13`
(steepness, water, then object, divisor 64), the six-record Sub-H writer,
and the authored C -> A -> B tail. Accepted constructor transcripts
`20260730-034232` / `20260730-035135` join Level-1 spawns 11/12/13
(`b700/7f00`, `be00/8500`, `be00/7d00`) to process seeds
`0x2B/0x2C/0x2D`. Section-12 Sub-D is divisor 64, probes `0x200/0x100`,
`classifier_flags` `0x13` — not Type-9 `0x17` or seeds `0x29..0x35`.
Those seeds do not authorize Type-9's first-query full reset. Variant-1
`+0x00` and `+0x04` are both `FUN_0040C690`. Aim tag `0x9C00` asks
`FUN_00401120` for `+0x04(entity, 0)`; Chase tag `0x9C01` asks for
`+0x00(entity, 0)`. Those calls target `FUN_0040D7A0` / `FUN_0040D760`,
not C690 directly. Each trampoline copies the literal second argument into a
local and invokes the behavior slot as `(entity, behavior_context, &local)`.
C690 ignores the third argument, reads word zero of the live context, and calls
`FUN_0040AC60(entity, *behavior_context)`. Accepted Intro2 context prefixes
have word zero, so AC60 legitimately falls back to TypeDefault `+0x118`
Always x9 Guard / Always x1 Wander. The literal scheduler zero is neither
dereferenced nor a null-second-argument failure. The same C690 contract is
reached by impact trampoline `FUN_0040DAC0`.

Static scheduler order closes Aim tag `0x9C00` through `+0x04`, Chase tag
`0x9C01` through `+0x00`, both strict `elapsed > lifetime` routes through
`+0x00`, tag precedence over timeout, and state-bit-`0x1000` suppression.
Direct Aim `+0x04` and Chase timeout are not capture-observed. Accepted Intro2
captures instead join owner `04900001`'s Chase invalid-target `0x9C01` to
Guard publication at 28.480--28.490 s while its Chase/Aim tasks are below
timeout; owner `048F0001` is already Guard at 28.470 s and remains
unattributed. The capture also joins Aim timeout to Guard with a live target
at 40.100--40.150 s. Focused tick `0x441` independently
shows Chase continuing at 4903/5000, Aim expired at 5017/5000, bit `0x1000`
clear, and C6B0 variant-zero Guard publication. Low-elapsed style switches are
not classified as scheduler transitions because impact C690 has the same
result surface.

The live owners consume exactly one selector RNG word, publish the
`FUN_00438340` Guard/Wander graph atomically, and continue mutation-safely.
If Primary Chase publishes a replacement, the outer traversal does not revisit
the newborn Primary but freshly reads later Secondary; a Tertiary Aim
publication has no later slot to visit. Intro2 source-dying bit `0x4000` must
block before selector RNG or behavior-graph mutation because this cohort has
no authenticated class-12 receipt/Common-Dying owner; do not reuse Level-1's
coarse owner. Authenticated Level-1 scheduler and impact C690 paths instead
select TypeDefault `+0x124` class 12 for a dying owner. Dying-bit AC60 is
`FUN_00425660` →
`FUN_00438340(entity, 0, *+0x124)` then `FUN_0040C620`; it does not
zero health or queue sound 75. A live Type-47 primary-hit owner stamps
`+0x34` then publishes the selected initial Guard or Wander graph, or
that class-12 install. The class-12 receipt retires the live
Guard/Wander/Pursuing scheduler and enters Common-Dying production
without the generic death prefix. Accepted pair census
`20260718-024449` / `20260718-024518` samples all three Level-1
type-47/model-302 bodies with instance `+0x44` and type-hit slots
zero; component-contact hooks stay unresolved. A surviving
`FUN_00415040` visit after C690 applies that null-modifier arithmetic
and does not invent a Base/Factory survivor policy. A live Type-47 hit applies `FUN_00411030` after C690 from the
particle's live Q15 velocity words; bit `0x80000000` fail-closes
instead of inventing `FUN_00469200`. A nonzero 15040 return then
queues type `+0x80` (cue 92) when the target is not dying. Lethal 15040
enters the recovered Type-47 `FUN_00410C10` standard-death publisher,
which sets dying first so the suffix skips that cue. Capability bit
`0x8` then emits class 5 at scale `0x0800` through shared
`FUN_00440DC0`, subtracting model 302 header `+0x08` extent 182 from
entity Z. Dying does not skip that branch. An already-dying 15040 still
returns the filtered amount after buffer-only `FUN_00414E90`; it does
not invent a second 10C10 prefix.
It
does not invent Pursuing ADE0. Missing optional
proximity-controller pointers suppress those writes. Guard visits slot-0 `FUN_00402EB0` before slot-1 acquisition. The
companion uses the shared one-or-three-word retarget. Fresh Type-47
construction retains entity `+0x90` from the shared `FUN_004104B0`
copy plus `FUN_0040D4A0`: type-default `+0xC0` `0x2039` has bit
`0x20`, so Y is `FUN_00445860(x,z)` before the post-snap current
position is copied into `+0x90..+0x94`. This is not Type-9's `0x2F`
outer latch. Missing construction terrain stays unresolved. Its mover is the Type-47 `FUN_00401430` bind and fails closed
at Sub-D. After A800, live Guard/Wander run E870/DCA0 `FUN_0040E100`
for that `0x2039` word (gravity and mode-zero drag; bit `0x02` clear so
no `FUN_0040DF70`) before `FUN_00412DA0`. A successful ADE0 handoff visits the newly published Tertiary
Aim in the same owner pass; the already-visited Primary is not re-run
as Chase that pass. Aim-and-Fire also ticks on later pursuing visits when the
spawn-11/12 bind already authenticates.

`v2k-game::aim_and_fire_transaction` now closes the synchronous parent/child
composition itself. A caller supplies distinct parent and child identities and
an exact owner/task lease. The machine retains the already-committed scheduler
prefix, consumes the optional sound RNG and sound before constructing the
generic-emitter child, maps all seven arguments through the typed request, and
preserves child zero, nonzero, and durable-block outcomes before owner unwind.
Rejected or cross-child receipts return both linear receipts without advancing.
This remains detached infrastructure for generic owners. Cohort-specific
type-47 wrappers compose it synchronously for Level-1 spawns 11/12/13 and
Intro2 spawns 6/7/8, revalidate the task lease before every action, commit
cadence and append before acknowledgement, queue sound 70, and later drain
each queued request exactly once into class 87. No receipt escapes. The
coordinator still ticks only after an authenticated pursuing bind.

The shared heterogeneous actor dispatcher now owns this task family's generic
scheduler composition as well. It accepts only Search And Attack's authenticated
pursuing phase zero (slot 2, constructor `0x00402220`, tick `0x00402300`,
lifetime 5,000 ms), retains the successful-construction Sub-F suffix until the
caller applies it before publication, and commits truncated elapsed accounting
before callback entry. Scheduler mode nonzero suppresses the callback but not
that elapsed commit. After unwind, a surviving tag-`0x9C00` result attempts
owner `+0x04` before the strict lifetime route at owner `+0x00`. An installed
and state-admitted `+0x04` callback exits that path even when it returns zero;
only an absent or entity-state-suppressed callback falls through to `+0x00` when
the committed lifetime is already expired. A nonzero emitter result propagates
before either transition. If the emitter clears or replaces its own wrapper,
its result or adapter error is discarded and later task slots are read freshly.
A selected optional sound remains committed when later emitter work fails.

The lossless class-7 program state, exact nearest-target search, target
handoff, and 5,000-ms task setup are safe Rust implementation boundaries.
The pure `v2k-game::search_attack` checkpoint encodes those selector, handoff,
and setup plans: its request preserves intrusive order and fails closed when a
consumed live field is unresolved, while its setup phases make constructor
failure ordering explicit. The `search_attack_owner` bridge now applies those
authenticated phases through the shared mutation-safe three-slot owner.
Acquiring clears slot 2, attempts slot 1, then attempts slot 0; pursuing
attempts slot 2, clears slot 1, then attempts slot 0; external event clears
slots 2 and 1 before attempting slot 0. A failed allocation preserves the
destination task and all earlier mutations remain committed. This still does
not attach the owner to an entity. The shared slot-0 Chase Target callback now
has an exact detached dispatch path. The callback-owned slot-1 acquisition
family now has one as well, including mutation-safe handoff replacement and
generic-scheduler tag consumption. Its concrete live entity/component snapshot now binds
`FUN_00422CD0` to the manager live list. An authenticated class-7
variant-0 context stores the target at `+0x08` and publishes
`FUN_0040C6B0(variant + 1)` style `0x004C7A98`. Variant-1
`FUN_0040ADE0` then optionally plays type `+0x9A` through `FUN_0044F480`
at entity `+0x96`, installs slot-2 Aim-and-Fire from type `+0x9C/+0xA8`,
clears slot 1, and installs slot-0 Chase. Unresolved prelude or Aim
audio fails closed. A successful ADE0 handoff visits the newly published
Tertiary Aim in that same owner pass; Primary Chase is not revisited that
pass. A later slot-0 visit advances Chase lifetime, validates the live
target, and applies `FUN_00423030` against the actor-local common-axis
limit. First-world type 13 authors `+0xC8/+0xCC` `0x1900` / `0x0C05`;
Chase binds that recovered pair from the entity copy or the type record.
First-world type 13 also authors Sub-D divisor 64, couple-yaw 1, probes
`0x200`/`0x100`, and `classifier_flags = 0`. Chase binds that payload
and the detached common-mover prefix now consumes it. Retail checks byte
`+0x0A` only after advancing Sub-D `+0x3A`; zero then branches around every
`FUN_0041FCB0` call and continues to shared yaw integration. It is not an
unresolved Type-9 first-query reset or Type-47's `0x13`. First-world type 13
also authors exact Sub-G/K/L payloads (`K = [11, 10]`, `L = [6, 7, 0x40, 0x1F,
0x40, 0x1F]`; retail Sub-G word `+0x0E` is 69, demo 71). Chase binds those
bytes without decoding invented field names.
First-world type 13's `+0x118` list is Always x1 class 5
(`Move About Aimlessly`) then Always x3 class 7 (`Search And Attack
Target`). `+0x11C` is 3 and `+0x124` is class 1; that alternate class is
retained as the authored dword. This list is why class 7 is type 13's
dominant initial program. A detached `FUN_00425680` plan now consumes one
word against that Always x1 / Always x3 pair: `r16 < 0x4000` selects class
5, otherwise class 7. A class-7 result publishes C6B0 style `0x004C7A50`
and variant-0 `FUN_0040B6C0`: copy type-authored common-axis `+0x04`,
clear slot 2, install `FUN_00402050/00402080` in slot 1 with radius
`0x1900`, filter `0x0C05`, and style `+0x44` zero, then duration-500
`FUN_00402B10/00402BA0` in slot 0. Each successful phase then runs type
13's `FUN_00406070` Sub-G branch: `FUN_0041B970(..., 0)`,
`FUN_0041B940(..., 0)` (one shared word, target = signed descriptor
`+0x0C` + high random byte; type 13 authors 200), `FUN_0041B980(..., 0)`
(copy descriptor `+0x00` = 0), and `FUN_00424380(..., 0)`. ADE0 remains
the post-acquisition variant. Each successful suffix writes the recovered
live Sub-G words (`+0x3F=0`, `+0x38` from `+0x0C` plus the high random
byte, `+0x40=0`, `+0x24=0`, `+0x20` from descriptor `+0x00`, `+0x3C=0`).
A later live `FUN_00402080` visit may drive that published B6C0 graph through
recovered C7D0/ADE0. Later Chase visits now commit Type-13 `FUN_00401430`;
later Aim runs Type-13 `FUN_00424650` (method 10, sound 75, speed 2400).
ADE0 same-pass Aim uses that adapter once and skips generic without_emitter.
A later live-list drain materializes class 38, not class 87. A detached Type-13 `FUN_00401430` transaction
authenticates the D/E/G/K/L topology and payloads, runs the target prelude,
classifier-free Sub-D yaw with coupled heading/roll, the prelude's L target
write, and the post-Sub-D K smoothing plus L exact write. Restricted scheduler
mode 1 skips the K/L callbacks and dispatches Sub-G. Detailed mode 0 continues
from G through `FUN_004243B0` K and `FUN_0041BA00` L. Both modes still commit
the target and post-Sub-D prelude writes.

`FUN_00409A80` binds K outputs `+0/+4` through `FUN_0040A950` to signed
animation selectors 11/10, and L outputs to selectors 6/7. K consumes the low
signed word of its smoothed `+0x0C` input and the post-G vertical velocity.
The pointed-to words begin at zero because 09A80 allocates and clears the full
`type +0x110`-word animation bank before component binding; zeroed K/L
allocations alone would only prove their pointer bytes. K's
`/2` and `/4` truncation precedes the signed-word store and then the
`0x1800`/`0x1200` clamps. L calls shared `FUN_0041E700` with the retained body
matrix, including the vertical projection which Sub-D does not consume.
The behind-target full-turn override changes only the lateral result. L tests
the whole `+0x10` dword for zero, then uses its signed low word on the nonzero
drive branch; the zero branch follows target lateral projection. Its vertical
output resets on absolute target-Y word `+0x0A == 0`, not relative height.
L applies signed division toward zero and stores before its authored
`0x1F40` limits. All four words persist across task replacement and detailed
visits; Restricted visits preserve them without running K/L.

The normal `FUN_0041A690` Sub-G branch is now exact through ordered
`FUN_0041AA60`/`FUN_0041ABB0`, `FUN_0041AC40`/`FUN_0041B020`, and non-player
`FUN_0041B210`. Type 13 binds all seven selector bytes `[1,2,3,4,5,8,9]`:
selectors 1--5 publish the authored oscillator records, selector 8 publishes
zero, and selector 9 publishes `+0x30` before its decay. The inclusive
phase-0 band `0x61..0x9F` latches `+0x3E` and emits retail sound 69 at the
entity position on the rising edge. A690 applies the `+0x1C/+0x1E/+0x3C`
target/pitch path; AC40 projects `0x300` along the retained forward X/Z basis,
samples terrain or waves at the center and projected point, and clamps the
derived rate to the authored 50..130 range. With Type-13's selector-1 binding,
`+0x20` is descriptor `+0x00` outside that phase band and
`100 * rate^2 / 90^2` inside it; the null-binding quarter-force branch is not
used. B210 projects the resulting force along retained body-up with self plus
attached-cargo mass. The complete 0x44-byte runtime and seven entity-bank
outputs persist to the next frame. The D/K/L allocation is retained on the
entity too, so the C7D0/ADE0 task replacement cannot discard its evolved
state before the pursuit callback is attached.

TTD `V200001.run` authenticates constructor seed `0`, origin `[0,0]`, cleared
rows, and `FRAME_NO_CLASSIFIER`, corroborating the static zero-flags branch;
there is no Type-13 first query to capture. Captured Intro2 production
publishes spawn 0's class-7 B6C0 graph and ticks slot-0 `FUN_00402BA0` then
slot-1 `FUN_00402080` from the specialized scheduler. Slot 0 now retains the
whole prefix/Sub-G result transactionally: it commits task state, persistent
component runtime, angles, velocity, and optional sound only after the selected
G/K/L callbacks succeed, then resumes `FUN_00401430` to return literal 1.
The shared post-unwind resolver gives the tagged zero result precedence over
strict `elapsed > 500` expiry, then routes either transition through the live
class-7 variant-0 `+0x00` C690/AC60 owner adapter. The outer state-`0x1000`
gate precedes metadata, current-context, and RNG access. Suppression preserves
the existing graph and freshly visits Secondary in the same pass. Otherwise
the authenticated TypeDefault root consumes one selector word: below `0x4000`
publishes class 5 and `0x4000` or above publishes class 7. Reselection uses the
existing behavior-context allocation, preserving target `+0x08`, auxiliary
`+0x0C`, and `initial_behavior`; class 5 consumes exactly one Sub-G suffix word
and class 7 exactly two. A new class-7 graph freshly visits Secondary without
revisiting newborn Primary. Class-5 publication retains the specialized owner
on its 5,000-ms Primary; later class-5 visits commit the same `FUN_00401430`
normal Sub-G transaction without a Secondary. A blocked plan/publication
retains linear custody without replaying
the callback or a committed selector. The dying `+0x124` class-1 branch fails
closed before selector RNG or graph mutation. A common-mover evidence block
likewise stops before slot 1 and retains an entered-callback failure receipt
for observation without replay.
Static recovery plus the accepted mode/constructor evidence is sufficient; no
new capture is needed for this normal callback. Exactness is relative to the
pre-call basis supplied to A690/B210. Production now closes the post-task
`FUN_00413F70` boundary shared by DCA0/E870. Type 13's Section-12 `+0xC0`
copies `0x8` to live `+0xC8`; the unrelated type-record `+0xC8` is axis data.
Class-5 variant 0 has style `+0x34/+0x38 = 0/0`, class-7 variant 0 has
`0/0x21080`, and class-7 variant 1 has `0/0x80`. Each therefore latches
effective flags `0x8` before `A800`, skipping E640's `0x10` gate and taking
F70's clear-`0x4000` branch. Authored `+0x94/+0x96/+0xA0` are zero, so the
intervening low-health emitters do not run. After the entire completed task
traversal, the owner rebuilds all nine Q31 words from the post-task angle
words and publishes state bit `0x4`. An explicit effective `0x4000` preserves
the existing matrix and bit; unresolved flags or unported E640 stop before
callback/RNG admission. The original flag latch survives pending C690 work,
including completion without a new frame. Blocked mover, acquisition, Aim,
or C690 work never publishes the suffix.

`FUN_00424650`'s `FUN_0041E4D0` aim-error and `FUN_0041E930` forward-half-space
tests read cached entity lateral/forward columns, not a matrix regenerated
from angles inside Aim. The Type-13 adapter now follows those reads; same-pass
Aim sees the pre-F70 basis, and the next mover/Aim visit sees the published
matrix. The bounded [common world owner](#intro2-type-13-common-world-scheduling-static--runtime-confirmed)
now encloses these tasks with scheduler timing, E100 gravity/drag, and master
motion. Type 13 authors surface `+0x72/+0x73/+0x74 = 0/0/0` and mass
100; those values do not authorize another type's outer owner. The seven G
and four K/L selector outputs are committed into the Entity animation-variable bank, but
Intro2 presents that live bank with the physical body basis and position.
Matched visual acceptance remains pending. The same recording
closes type-26 and Intro2 Type-47 first queries as full-reset. Type-26 spawn 25
now retains the pending
reset (seed `0x15`) and the detached `FUN_00401430` bind applies it and
shared `FUN_0041F660` steering for `classifier_flags` `0x12` and the
authored terrain-only C -> A -> B tail. The spawn-25 Defecate Virus
Primary visit binds that mover and commits the applied pose and `FUN_00412DA0`.
Visible model submission invokes `FUN_0041D360` on the selected endpoints of
the six-record Sub-H. Restricted mode 1
does not invoke `FUN_0041D0A0`. The explicit captured fixture
adopts that Primary into the specialized scheduler
and then visits the mode-5 Tertiary `FUN_00402850` terrain callback.
Generic `FUN_0040D720` writes constructor `FUN_00413F70` from the three
Section-13 angle words and does not borrow Type-9's delayed bit-`0x4`
suffix. Captured Intro2 Detailed `FUN_00402850` uses that constructor
forward column and each slot's Section-8 header `+0x08`; a missing model
record stays fail-closed. Emitted class-5 carriers take the later
`FUN_00440120` / `FUN_0043E180` ordinary-surface tail and set terrain
byte-2 bit `0x10`. A later `FUN_00411400` Coarse write clears
`0x02000000`, so the next Tertiary visit is coarse `FUN_00402850`: two
RNG words, no particle, one direct `FUN_00433720(...,1)` cell. Primary tagged
`0x9C01` precedes strict `elapsed > 2000`; both dispatch through owner `+00`,
after the wrapper unwinds and
`FUN_00416410` applies the state-`0x1000` suppression gate. Exact executable
prototype `0x004C7E88` contains `C690` at both `+00` and the separate
`0x9C00` route's `+04`. The older Type26
captured resolver's `CallbackAbsent` result and native world's discarded
transition are implementation limitations, not null retail callbacks.
Native Type16 binds the real C690 result; see
[INTRO2_TYPE16.md](INTRO2_TYPE16.md#native-construction-and-styles).
Intro2 Type-47 seeds
`0x06/0x07/0x08` apply that pending owner from the detached Guard-anchor
`FUN_00401430` bind, shared `0x13` yaw, six-record Sub-H, and C -> A ->
B. Level-1 `0x2B/0x2C/0x2D` apply the separate `V200003.run` owner from
Chase and Guard wander. Native Intro2 production first applies Sub-A
`FUN_00420450`: one shared word writes
`base + ((low16 >> 8) * base) / 0xA00`, direction 1, and drive scale 100.
`FUN_00425680` then consumes one selector for Always x9 class-32 Guard or
Always x1 class-6 Wander; each installed task applies its own 06070 suffix.
Guard therefore consumes four birth words and Wander three. Both retain the
pending first-query Sub-D owner and enter the live scheduler; Guard visits
Primary `FUN_00402EB0` and freshly read Secondary acquisition, while Wander
has only Primary. Explicit `CapturedGuard` fixtures replay only the two
observed task suffixes and do not define production RNG order.
An accepted Type-9 target runs the shared
C7D0/ADE0 handoff, skips the newborn Primary Chase, and visits the newborn
Tertiary Aim in the same pass. Later passes visit Chase then Aim and drain
queued shots into class87 in presentation after particle traversal; the new
particles first move on the next simulation update. The joined passive and
focused evidence above closes normal Intro2 C690 tag/timeout inputs and
results; no new capture is required for that route. The Rust application is
live with the stated selector-draw, atomic-publication, and fresh-later-slot
boundary. Source-dying AC60 now transfers the authenticated allocation to
the shared class-12 Common-Dying owner.
Type-13 Sub-G birth constructor `FUN_0041B8C0` allocates 0x44 bytes,
zero-fills, copies descriptor bytes `+0x30/+0x3C/+0x48/+0x54/+0x60`
(`[0, 0, 0, 230, 210]`) into runtime `+0x28..+0x2C`, consumes one word
for `+0x38 = 200 + (low16 >> 8)`, then writes `+0x1C = 0x5000`,
`+0x1E = 0`, and `+0x41 = 1`. Later `FUN_00406070` overwrites `+0x38`
without clearing those birth fields. Metadata-backed type-13 construction
applies that constructor with one caller-supplied word before behavior
publication. Detached metadata birth and native Intro2 both sequence that
constructor, `FUN_00425680`, and B6C0/ACD0 from one caller RNG stream:
class 5 consumes three words and class 7 four. Explicit `CapturedClass7`
fixtures retain the observed graph without replaying its selector.
A class-5 result publishes C6B0 style `0x004C7930` and `FUN_0040ACD0`:
`FUN_0040A7A0` clears slot 2 then slot 1, then `FUN_00402B10(entity, 0,
5000)` runs `FUN_00406030`/`FUN_00406070` and publishes the 5,000-ms
`FUN_00402BA0` Primary. EXE style `+0x44` is zero. Type 13 does not copy
common-axis `+0x04`. This class-5 publication is reachable at native birth and
through the live C690/AC60 root and has no later slot to visit in that owner pass. The
specialized owner then retains that 5,000-ms Primary, and later class-5
`FUN_00401430` visits use the same live scheduler ownership. Captured Intro2
B6C0 commits lifetime and its RNG-owned retarget prefix before entering the
mover, then atomically commits the complete normal A690/AA60/AC40/B210 result
and returns one. A
continuing result reaches same-pass acquisition; a transition applies the root,
where suppression or a successful class-7 result also reaches freshly read
Secondary and a blocked result stops traversal.
Same-pass Aim can advance
lifetime, validate the live target, and play recovered `+0x9C` audio. A
successful ADE0 handoff now retains the pursuing graph, and later Chase
visits commit the same Type-13 `FUN_00401430`. Pursuing admission joins the
Known TypeDefault context target to both Chase and Aim and requires Aim's
authored direction 1, timer 0, null optional sound, and zero period. A partial
state word with known value zero does not become an inactive target, and a
blocked Chase stops before Tertiary Aim. Chase tag/timeout enters variant-1
`+0x00` C690/AC60. Aim invalid-target tag enters `+0x04`; Aim timeout enters
`+0x00`, and a suppressed due tagged call falls through from `+0x04` to
`+0x00`. A parked Plan binds the committed dispatch prefix: Primary for a
Chase call, Primary plus Tertiary for an Aim call. Later slots remain fresh
reads, the suppression gate stays ahead of exact predecessor-context
validation, and Publication additionally binds the context frozen after
selector success. An initial Aim-origin block preserves the already-committed
Aim/acquisition receipt for its caller. Tertiary Aim C690 has no later slot and does not ADE0 a newborn
Secondary. Later Aim runs Type-13 `FUN_00424650` with method 10 / sound 75 /
speed 2400. Its positive source-velocity closing boost remains 32-bit through
direction scaling and truncates only at the final i16 components. ADE0
same-pass Aim uses that adapter
once and skips generic without_emitter. A later live-list drain
materializes class 38, not class 87.
First-world type 13 authors Sub-E
method 10, interval 600,000 us, spread 128, aim 20,000, axis 0x0F00,
and sound 75. ADE0 observes that Known payload and runs the Type-13
emitter adapter on the same pass. Later pursuing visits reuse it.
Known-null Sub-E returns zero; unresolved Sub-E
fail-closes. Slot-2 Aim And Fire participates in the same detached dispatcher
with its exact
pre-callback/callback/post-unwind lifecycle; its live
target/component/sound/RNG and generic-emitter world adapters remain external.
Ptersect's captured B6C0 wander callback now owns the exact seven-argument
`FUN_00401430` normal Sub-G transaction with type-owned movement, terrain,
collision, control, and persistent component state. The accepted focused
transcript closes its caller arguments and task transition order, while static
recovery closes the normal mover's internal decision matrix; do not request a
new Sub-G capture. The callback consumes its retained pre-call basis and the
completed owner pass publishes the next basis before its common world suffix
as described below. Intro2 now submits the retained live pose, body basis,
active model, and selector bank directly; the separate Ptersect animation
oscillator and captured-origin displacement are removed. Matched presentation
acceptance remains open; unsupported dying, relation, and wind branches remain
explicit gates.

An entered Type-13 SharedRetarget or Chase mover failure retains a durable
callback-failure owner. SharedRetarget has already committed lifetime and the
near-axis two-word or far-axis one/three-word retarget prefix; Chase has already
committed lifetime. The authored D/E/G/K/L mover consumes no additional RNG
because its random target-correction routes require absent Sub-A. Failed mover
component, rotation, velocity, and audio writes remain unpublished. Retained
production-owner visits authenticate the exact Primary task identity/state, runnable
wrapper, and behavior context, then report `CallbackFailurePending` without
re-entering the callback or later slots, even if the evidence has since been
repaired. Changed custody is rejected. Pre-entry Chase failures, such as an
unresolved target state, remain mutation-free and retryable. This preserves
`FUN_00402BA0`'s target-prefix-before-`FUN_00401430` ordering without replaying
already-consumed process RNG.

### Intro2 Type-13 common world scheduling (STATIC + RUNTIME CONFIRMED)

`intro2_type13_live/world.rs` encloses the captured spawn-0, model-291 Type-13
task owner with local `FUN_00412DA0` and the DCA0/E870 environment suffix.
Admission is limited to the authored mass-100, initial-flags-8 profile, detached
from an attachment, with no Sub-C/I/J, and with null live sound-follow `+0x8C`.
The live relation-owner bit `0x1000`
must be known clear: 12DA0 tests its original state snapshot before the type
callback and exempts only type 111. A missing `+0x80` owner invokes `180F0`,
which can construct type 93; `attached_to == None` does not make that path a
no-op. Remote ownership, dying behavior, alternate effective flags, wind, and
unresolved required inputs remain outside this owner.

The accepted `20260722-022303-intro2-actor-ai.jsonl` gives spawn 0's birth
`+0xB2 = 5754` decimal (`0x167A`) at line 1025, a callback-disabled pass clearing it at line
1308, and the later flags `0x00068000` activation around 22 seconds at line
44028. The allocator residue is not a constructor zero. The common owner
therefore runs during the dormant period: `0x20000` clear skips callback mass,
tasks, and environment work, then clears B2 before the current-state motion
tail. Callback waits also clear B2. An enabled continuing callback computes
`B0 = wrapping_u16(authored_mass + B2)`, replacing a zero result with one;
unknown B2 at this consuming boundary blocks rather than inventing mass.

The shared prefix preserves subject `+0x70` then callback `+0x6C` RNG order,
coarse wait thresholds, the 125,000-us cap and carry, `+0xB6`'s one-us override,
and wrapping `+0x68` time accounting. Detailed state `0x02000000` selects DCA0
and task mode 0, so `01430/018A0` dispatches G, K, L. Coarse E870 passes mode 1
and dispatches G alone. `02080` acquisition does not suppress its search in
mode 1; `02300` Aim does suppress all target, emitter, matrix, sound, and RNG
reads, while its generic wrapper still advances truncated milliseconds and
resolves strict 5,000-ms expiry after unwind. A new ADE0 Aim visit receives the
same enclosing mode as the preceding slots.

Tasks use the adjusted callback delta; Sub-D's yaw integration separately uses
global `DAT_004D04E4`, and Sub-G's terrain/wave probes retain the retail tick.
A suspended C690 pass retains both deltas, retail tick, mode, model
extent, mass, and committed source state. Resumption never reruns the prefix
or mass calculation, replaces the frozen timing with a later frame, or
revisits an already-committed slot. Entered callback failures remain observation
only. Pre-entry failures retain their bounded retry boundary.

After the complete task traversal, the pre-A800 flag latch owns F70 as above.
E100 then rereads current live C8 and style masks; the supported class-5 and
class-7 profiles still yield `0x8`. DCA0's effective `0x2000` master-bit priming,
E640 `0x10`, ground-snap bit `0x2`, and tile-contact bit `0x800` branches are
absent in this profile. Authored low-health `+0x94/+0x96/+0xA0 = 0` contributes
no effect. E100 applies gravity and mode-0 drag, with no Sub-C buoyancy. Intro2
Section-13 raw `+0x90/+0x94/+0x9C/+0xA0/+0xA4 = 0/3/0/0/0` selects wind mode
zero and drag strength three. The E370 surface selectors `+0x72/+0x73` and
lifetime dword `+0x74` are zero, but its entity `+0x48` timer still saturating-decays by
`callback_delta / 1000` unless state `0x20000000` suppresses E370.

Finally 12DA0 clears B2, then rereads current state for master motion. Motion
requires `0x40000`, a zero callback result, and clear `0x08000000`; the
`0x00880000` pair selects the existing approach-zero step. Position advances
in X/Z/Y order by `((delta >> 5) * velocity) >> 15`, with signed-word wrap and
state `0x20`. The callback result is the outer DCA0/E870 result, not `01430`'s
internal literal one. Intro2 draw submission and camera targeting consume this
live pose. `Entity::presentation_anim_vars` publishes the Sub-G and Sub-K/L
outputs from the same retained runtime; drawing does not advance another
component clock. The remaining complete-scene acceptance is tracked in
Intro2 acceptance.

### Native Type-26 construction, hit and surface ownership (STATIC + RUNTIME CONFIRMED)

Type26 is the mandible-bearing `stag` model267, with authored health5000, mass400,
capability8, and default C8=0x0439. `V2000-nocd05.run` opens Castle OVL15 at
`2CBB78:1F61` and repeatedly reaches `10EB0` with type0x1A; the later OVL17
open belongs to the failed Slot02 reload. This trace establishes the requested
actor identity, not an Alpine Type30/40 behavior family.

Ordinary authored construction now publishes the same weighted class26 Trash
Furniture, class33 Follow Beacons, and class4 Defecate Virus graph for all eight
campaign Type26 births: six in OVL15 and two in OVL18. `104B0 -> 09A80 -> D4A0`
publishes common state and terrain grounding; the shared native component path
consumes the current process Sub-D allocation counter. Sub-A consumes its own
constructor RNG word before AC60's weighted selection; conditional Furniture
Nearby has multiplier2, and Always contributes4/3 to Follow/Defecate. The selected
initializer consumes its own suffix words. No captured selector or Intro2 seed
is replayed for these births.

The ordinary birth receipt authenticates the entity id, authored index, and
manager generation. It survives movement, reselection and death; live pose is
not an allocation identity. The global load path already adopts Type26 owners
without a level gate. The two Intro2 allocations (spawns10/25) retain their
explicit captured Sub-D first-query receipts, and the detached captured class4
fixture remains separate from production selection. Ordinary Sub-D follows the
existing deterministic native first-query policy, not a claim about unknown
retail allocator-origin residue.

Common vtable `004C8A30` sends primary `+14` to `0040DAC0` and infected `+18`
to `0040DA00`, with a null generic-hit `+30`. Executable style rows `004C7E88`,
`004C7738`, `004C7B28`, and `004C7B70` individually contain `0040C690` at both
infected `+20` and primary `+28`. Class12 `004C7ED0` and initializer-failure
`004C74F8` contain nulls at both offsets; all six have null style-death `+2C`.
Impact entry calls C690 directly, bypassing the `00416410` task-result-only
`0x1000` gate. Its new live task graph transfers to the scheduler before impulse
and checked damage. Unresolved constructor inputs preserve their committed
context/task prefix as pending; they do not imply a retail initializer failure or
authorize an invented fallback. Trash Furniture's constructor Y remains
allocator residue until a successful first scan, including after hit reselection.

`00410EB0` stamps `+34` before DAC0, applies `00411030`, then runs `00415040`.
Its signed nonzero return is tested before buffer absorption: a surviving target
may play type `+80`, and capability 8 emits class5 at scale `0x800`, with Z reduced
by the current state-selected model header `+08`. Dying suppresses that cue but
still permits the class5 suffix. Infected `00411250` instead commits its model-bit
and optional type `+82` cue before DA00; it neither stamps `+34` nor executes
the primary suffix. Class5/F780 retains `[6,0,2000,0,0,0]` delivery provenance.
Type26's authored channel-1 threshold is 6000 and channel-6 multiplier is zero,
so the 2500 fragment and F780 both filter to zero after their live callbacks.
The class1 channel-2/2000 packet exceeds its 1800 threshold and returns 200.

Lethal checked damage publishes the native common class12 owner synchronously.
The same owner now receives E370's 30,000-ms surface expiry after the living task traversal:
`162B0` timer write, `16750` release/default restoration, standard death, then
the remaining bubble/sound and master-motion suffix. Default C8 stays `0x0439`;
class12's effective mask is `0x0428`. Its new Primary starts next pass. A later
blocked suffix retains the committed death receipt without replaying living work.

The ordinary-world regressions enumerate all campaign overlays, construct each
Type26 through the real authored path, run detailed native mover frames, and
exercise primary hit, blood emission, Class12 transfer and repeated corpse hits
for OVL15/18. A separate cross-manager receipt test rejects a foreign allocation
before a hit timestamp, health write or particle emission. Existing native
Type26 tests retain weighted-constructor RNG and detailed/coarse Defecate
infection coverage. These checks do not establish complete visual parity for
Castle: the shared furniture scan's unresolved constructor target-Y edge,
uncaptured allocator residue, and unimplemented other insect families retain
their explicit boundaries.
