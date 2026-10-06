# Guard Location class 32 task program (STATICALLY CONFIRMED)
Verbatim move from ACTOR_RUNTIME.md during the 2026-08-24 cohesion split.

### Guard Location class 32 task program (STATICALLY CONFIRMED)

Class 32 descriptor `0x004C8958` starts at style `0x004C7BB8`; its next style
is `0x004C7C00`. Initializer `FUN_0040B5A0` copies the type-authored state word
at `+0x04` into the behavior's primary context, clears task slot 2, and creates
the slot-1 `FUN_00401F80/00401FB0` task. Its return convention is proven by the
generic constructor chain rather than inferred from the caller branch:

1. `FUN_00401020(out, owner, template)` allocates the wrapper and 0x34-byte
   task record. It writes the wrapper through `out` and returns zero only after
   successful initialization; allocation or initializer errors are nonzero.
2. `FUN_00406030` performs component setup only on that zero result.
3. `FUN_00405F80` then passes the newly written, non-null wrapper to
   `FUN_0040A7A0(entity, slot, wrapper)`. `FUN_0040A7A0` destroys the old slot
   occupant, stores the supplied wrapper, and writes its owner backpointer. It
   is an install/replace operation, not cleanup when the third argument is
   non-null.
4. Therefore `FUN_00401F80 == 0` means slot 1 is installed. Only this success
   path continues to `FUN_00402E20(entity, slot 0, 5000)`. Slot 0 and slot 1
   are the normal concurrent Guard tasks; slot 0 is not a failure fallback.

Rust `guard_location_owner` now implements exactly that bounded initializer
transaction. It copies the authored primary-context word before task mutation,
clears slot 2, fallibly prepares/replaces slot 1, and attempts the duration-5000
slot-0 companion only after slot-1 success. Preparation failure preserves that
phase's destination, commits every earlier write, and stops the remaining
phases. The initializer's fifth argument is the slot-1 constructor's filter
override at task state `+0x28`: zero performs no context write, `FFFFFFFF`
normalizes to same-type filter zero, and any other nonzero value replaces the
behavior search filter on an accepted acquisition gate. The nested detached
acquisition module retains that ordered write and the exact first-eligible
selector. Shared RNG ownership, live intrusive-list snapshots, behavior
callback lookup/storage, the common mover, component ownership, style
transitions, and live entity attachment remain adapter boundaries for generic
owners. Authenticated Level-1 and Intro2 Type-47 wrappers now supply those
boundaries for their captured cohorts.

The shared ownership contract is also closed. The outer wrapper is eight bytes,
but only `alive` at byte `+4` and `in_callback` at byte `+5` are stable; its
upper padding is allocator residue. A replacement is fully allocated and
initialized before the old slot occupant is destroyed, so initializer failure
preserves that destination while earlier ordered slot mutations remain
committed. The owner visits physical slots 0, 1, and 2 by reading each slot
fresh; a callback's mutation of a later slot is visible in the same pass. A
self-clear destroys inner state immediately but defers outer-wrapper reclaim
until callback unwind. The installed task backpointer targets the owner
callback pair at root `+0x10`, not byte offset `+4`. Rust
`actor_task_owner` implements this mutation/lifetime seam without interpreting
task callback addresses or inventing movement.

Class 46 Alien Hive uses that same pair: initializer `FUN_00425760` clears
slots 2 then 1 and `FUN_00425E10` `FUN_00401020`-publishes slot 0 with tick
`FUN_00425EA0`; death `FUN_00425790` repeats the clears and `FUN_00425F60`
publishes tick `FUN_004260F0`. Both 11-dword templates have a null constructor
and copy the component-table pointer into inner `+0x08`. Their task contact
callback `+0x18` is explicitly zero (`0x425E59` / `0x425FA9`); contact dispatch
returns null for either wrapper and still continues into the pair's physical
response. It must not attempt the common-mover descriptor callback. Sub-N / Sub-K /
infection / radial accumulators stay on that table, so the death realloc keeps
them. Type 67 Sub-K `[1, 0]` stores `FUN_0040a950(entity, 1)` at Sub-K
`+0` (authored byte 1 is a null second bind). Live and dying ticks write
that u16 into the model-callback word bank, which the port exposes as
`AnimVars.dynamic[1]`. Do not invent a hive1xa morph; the recovered
consumer is this word-bank write. The heterogeneous dispatcher treats this
family as a dedicated exact visit; `FUN_00425EA0` / `FUN_004260F0` still
run through the retained hive component owner.

`FUN_00401FB0` consumes one RNG value on every callback. On the
`(low16 & 3) == 0` gate a nonzero private filter override is normalized (only
`-1` becomes zero), copied to primary context `+0x04`, and followed by
`FUN_00422C10(&candidate, entity, search_context)`. A zero private word skips
the write and retains the authored context. The selector walks the intrusive
list in order and returns the first eligible candidate inside its strict
wrapped-coordinate cube; it does not rank by distance. Retail `FUN_00422C10`
and exact-match demo `FUN_00422AE0` read candidate state only as
`state != 0 && (state & 0x5000) == 0`. Rust therefore carries a
`RetailStateWord` through `GuardLocationEntityRef`: unrelated unknown bits do
not block a proven predicate, while actual ambiguity remains an error before
later capability/RNG work. A found candidate is
passed to the behavior-owned indirect callback as `(entity, &candidate)`. A
zero callback result returns tagged task result `0x004BE1A8`; a nonzero result
propagates unchanged. No candidate or no callback returns zero. Generic
scheduler `FUN_00401120` consumes this `0x9C02` tagged result and continues; it
does not invoke a behavior-owner transition.

The duration-5000 slot-0 callback `FUN_00402EB0` always consumes one RNG word.
When `(low16 & 0x3F) == 0`, it consumes two more and selects signed-16-bit,
wrapping X/Z offsets `(low16 >> 5) - 1024` around immutable entity anchor
`+0x90`; Y is copied exactly from the anchor. It then calls the shared
seven-argument mover `FUN_00401430`. That mover owns component integration,
terrain/orientation correction, target lifetime, and completion policy, so the
retarget rule alone is not a faithful Guard locomotion implementation.

The ordinary type-9 class-6 initializer `FUN_0040AD10` clears slots 2 and 1,
then installs this callback in slot 0 through constructor `FUN_00402E20` with a
strictly-more-than-5000-ms lifetime. The task's 0x24-byte private allocation
starts its target at current entity words `+0x96`, tracked handle at the zero
sentinel, direction at `+1`, and reversal timer at zero. Successful task
construction consumes one shared RNG word through `FUN_00406070` to reset
Sub-A target speed to `base + (((low16 >> 8) * base) / 0xA00)` and direction
to `+1`. Per-frame lifetime accounting truncates `elapsed_us / 1000` before
adding it. A zero common-mover return maps to singleton `0x004BE140`, tag
`0x9C01`; ordinary static-point movement normally returns one.

The Rust `ordinary_type9_wander_owner` bridge now joins this oracle to
`actor_task_owner`. Its setup plan preserves clear slot 2, clear slot 1, then
fallible slot-0 installation: an allocation failure keeps the old primary,
commits both prior clears, consumes no constructor RNG, and leaves Sub-A
unchanged; successful post-allocation preparation consumes one draw, applies
the Sub-A reset, then publishes the new primary. Its phased tick advances
truncated elapsed time before the callback, performs the one-or-three-draw
retarget with `in_callback` set, unwinds the wrapper, and only then gives
tagged mover completion precedence over the strict `elapsed_ms > 5000`
timeout. Mover errors and unresolved returns stop after unwind with the
elapsed/retarget prefix committed; a surviving transition runs after unwind
and may replace the task without suppressing its result. Production attachment,
contact callbacks, the process-shared RNG owner, and the concrete behavior
transition remain external and fail closed, so this bridge does not yet
activate villagers.

The detached `ordinary_type9_wander_initializer` now closes class 6's earlier
pre-link publication boundary as well. It accepts only canonical choice 3
through the shared exact selected-entity/metadata/task/component preflight,
publishes the TypeDefault/null-target/zero-aux context and class-6 state policy,
and deliberately does not perform class 10's common-axis `+0x04` copy. It then
clears Tertiary and Secondary before the fallible Primary allocation. Failure
consumes no constructor word and enters the common outer fallback; success
consumes exactly one word, resets Sub-A direction/randomized speed, and only
then publishes the 5,000-ms Wander Primary. Both terminal paths move the exact
pending Sub-D/anchor custody and reject replay while leaving entity link,
physical basis, and wrapper bit `0x4` unresolved. This is not production
selection or live scheduler attachment.

`ordinary_type9_live` now supplies the next bounded composition: one
authenticated, already-published fresh-Level-1 Wander wrapper can advance
through lifetime accounting, shared-RNG retarget, the receipt-bound mover, and
callback unwind in either scheduler mode family. Mode zero executes D -> I ->
A -> B. Any nonzero signed scheduler word executes D -> A -> B, because matched
retail/demo `FUN_004018A0` suppresses Type 9's Sub-I dispatcher call while the
outer mover still performs D and the unconditional A/B tail. The restricted
path neither validates nor mutates the animation controller and reports its
preserved pre-frame output selector. The atomic mover commit covers Sub-D,
optional Sub-I, Sub-A, Wander private state, heading, and velocity; an
evidence block still leaves the earlier lifetime/retarget prefix committed and
returns that prefix explicitly, so callers cannot safely retry the consumed
frame. Tagged-zero and strict-timeout transitions are successful consumed-frame
outcomes carrying the mover return and post-unwind owner request, never
retryable errors; executing that owner request remains outside this seam.
The authenticated construction sidecar now supplies the immutable retail
`entity + 0x90` anchor. Matched retail/demo `FUN_0040D4A0` proves that Type 9's
`0x2F` profile takes the bit-`0x20` terrain branch, writes the exact two-stage
integer-bilinear height to current `+0x98`, and copies final `+0x96..+0x9A`
back to `+0x90..+0x94`; missing construction terrain remains unresolved rather
than falling back to authored Y. The live seam likewise snapshots
`Entity::physical_body_basis_q31` at callback entry. Either unresolved value
blocks before lifetime accounting or RNG, and the D/A/B commit leaves the
stored matrix unchanged for the later E870 rebuild. The caller now supplies
only terrain, global frame delta, tracked-target lookup, unique receipt
identity, the signed scheduler mode, and the same shared RNG stream used by
the scheduler. The exact detached planner, shared selected preflight, all four
selected-initializer adapters, detached native fresh-context abort custody,
four-way production RNG/single-issuance custody, construction composition, and
outer entity link/wrapper finalization are closed. All six retained entities
are post-wrapper storage and cannot re-enter the pre-publication adapters.
Authenticated selected Run Away, Go-To-Job, Wander, and multi-slot Attract
owners now leave their manager sidecars atomically in retained order for the
production scheduler. All four bind the exact heterogeneous dispatcher,
root-class-6 application, and direct F70 publication; each mover-bearing branch
uses the real common mover. Selected Wander preserves strict tag-
before-timeout precedence and post-unwind state-`0x1000` suppression;
Go-To-Job additionally pins the constructor-selected
target and authenticates the entity-local common-axis descriptor used by its
route predicate. Initial Attract retains exact same-pass Candidate/Cue ordering,
synchronous Target Route publication or zero-constructor-RNG fallback, and F70
publication. Only its unchanged `InitialGraph` custody is transferable to Main
Base; progressed Attract custody fails closed. The publication frame parks
Target Route through its outer tail; the following visit runs its dedicated
exact scheduler/predicate/common-mover/root path. Actual fresh owners park at
`CallbackMassUnavailable` before scheduler RNG because allocator-residue
`+0xB2` is not yet known. Selected Go-To-Job additionally retains weighted
class-54 frozen candidate/capacity evidence, terminal publication or fallback,
the post-Primary Secondary cursor, and F70 custody. Selected Attract retains the
matching weighted class-45 transaction and cursor/F70 custody. Wander ->
class-54 Go-To-Job, Go-To-Job -> class-10 Run Away, Go-To-Job -> class-6
Wander, Wander -> class-10 Run Away, Wander -> class-45 Attract Attention, and
Attract Attention -> class-54 Go-To-Job plus class-10 Run Away are now closed
across producers. The three Run Away crossings authenticate their canonical
lone-Primary or exact Attract initial/route predecessors and retained contexts,
no-redraw plans and frozen snapshots, exact two-phase success-only RNG/
fallback, original continuation, optional Fleeing suffix/fixed-`5/3` word,
final graph, F70, and PostBasisTail. Attract admits SharedRetarget Primary ->
Secondary, expired Cue -> terminal, and Target Route Primary -> Secondary;
progressed Main Base custody fails closed. Other cross-producer roots remain
static composition; selected Go-To-Job -> class-45 Attract Attention is the
closed for both the even graph and the odd Candidate/Target Route suffix and is not currently capture-gated. Alternate class 14 and
the generic lifecycle share one session-zero runtime oracle; fresh-birth
`+0xB2` remains separate.

Style-1 initializer `FUN_0040ADE0` first performs its optional type-state
`+0x9A` effect, then creates duration-5000 auxiliary task
`FUN_00402220/00402300` in slot 2 using type-state `+0x9C/+0xA8` and the
transition target. The port retains those two words on the type record
and copies them into `FUN_00402220` instead of inventing zeroes.
A Known nonzero `+0x9A` plays `FUN_0044F480` at entity `+0x96` with gain
and rate `0x10000` before slot-2; Known zero skips. Sibling initializer
`FUN_0040B340` proves that backend; ADE0 keeps Aim-then-clear-search-then-
Chase order, not 0B340's clear-1-and-2-then-`FUN_00404BE0` order.
Unresolved `+0x9A` fails closed. On successful installation it clears
slot 1 and installs
duration-5000 `FUN_00403360/00403490` in slot 0 with that same target.
`FUN_00403490` validates the referenced target, delegates movement to
`FUN_00401430`, and compares wrapped X/Z separation against the exact
`0x380`/`0x400` bands to update controller state or return tagged task results.

The shared Chase Target family is now closed as a detached Rust runtime.
Retail disassembly of `FUN_00403360` corrects the former RNG interpretation:
the constructor consumes no random word. After task/private allocation, an
authored Sub-A resets runtime dword `+0x00` from the signed descriptor word
`+0x04` as `(base * 4) / 3`, truncating toward zero. An authored Sub-F calls
`FUN_00424390(component, 1)`, which writes mode byte 1 and phase `0x1000`.
Both optional suffixes occur before task publication. The private 0x24-byte
record starts at the owner's current XYZ, retains the explicit target handle,
sets direction to `+1`, and starts its reversal timer at zero.

`FUN_00403490` returns pointer-distinct tag-`0x9C01` singletons for an absent,
missing, inactive, or dying target (`0x004BE0D0`), an in-range zero mover
return (`0x004BE0D8`), and an out-of-range target (`0x004BE0E0`). The
out-of-range branch still calls `FUN_00401430` and ignores its Boolean result
before returning its singleton. Only an in-range nonzero mover return reaches
the optional proximity-controller writes. Retail looks up both entities again
after the mover, so those bands consume fresh post-mover positions rather than
the positions used by the earlier route predicate. They use wrapped absolute
X/Z separation only: both axes strictly below `0x380` copy the signed source
value to the primary control, write secondary `-1`, and set task direction to
`-1`; both strictly below `0x400` but not both inside the inner band write
primary 1; the outer case copies the source value. Missing optional
component/source pointers suppress those writes.

The detached scheduler advances truncated lifetime before callback, stages
task-private mutation until a resolved callback returns, and discards that
stage if the wrapper is cleared/replaced during callback unwind. Optional
controller writes remain an explicit live-adapter result. A surviving tagged
result transitions before the strict
`elapsed_ms > 5000` timeout; a continuing callback transitions on timeout only
after unwind. This family now participates in the central heterogeneous
dispatcher for both Search-and-Attack pursuit and Guard Location's pursuing
style. Captured Level-1 and Intro2 Type-47 owners now bind the live
entity/target/components and their cohort-specific shared mover. Generic and
type-13 bindings remain external. The normal, non-dying Intro2 C690
tag/timeout contract is evidence-closed and live with the selector-draw,
atomic-publication, and mutation rules below. Intro2 source-dying C690 remains
an explicit pre-publication unsupported branch because its class-12 receipt and
Common-Dying owner have not been authenticated for this cohort.

The accepted Intro2 behavior/pose capture
`runtime_re/captures/local/20260722-022303-intro2-actor-ai.jsonl` is complemented
by focused WinDbg transcript
`runtime_re/captures/local/20260727-235423-intro2-actor-task-mover.txt`.
The latter closes the private task/caller boundary: 132 observed style switches
each issue three ordered slot-install calls, including the Guard/Search
transition from slot-1 search plus slot-0 wander to slot-2 fire plus slot-0
chase. Its 671 valid `FUN_00401430` return samples all returned one and preserve
the seven arguments as `(task wrapper, entity handle, type-owned movement
state, controller context, target subrecord, elapsed microseconds, scheduler
mode)`. Stable movement-state pointers were `0x138200DC` for type 13,
`0x13821810` for type 26, and `0x13823EC0` for type 47; scheduler mode one was
seen only for types 13/26, while type 47 used mode zero.

The scheduler's literal zero is not C690's choice-source pointer.
`FUN_00401120` calls transition trampolines `FUN_0040D760` / `FUN_0040D7A0`
with `(entity, 0)`; they invoke the behavior callback as
`(entity, behavior_context, &local_zero)`. C690 ignores the third argument and
passes behavior-context word zero to `FUN_0040AC60`. Accepted Intro2 context
prefixes have that word zero, so the non-dying route uses TypeDefault `+0x118`
(Always x9 Guard / Always x1 Wander) and consumes exactly one selector RNG word.
Static code closes Chase `0x9C01` through `+0x00`, Aim `0x9C00` through
`+0x04`, both strict timeouts through `+0x00`, tag precedence, and the entity
bit-`0x1000` gate. Direct Aim `+0x04` and Chase timeout are static-only.

The accepted passive capture joins target `045D0001` becoming dying at
28.480 s to below-timeout owner `04900001` leaving Pursuit for Guard by
28.490 s; owner `048F0001` is already Guard at 28.470 s, so its nearby switch
remains unattributed. Static slot order makes the clean join Chase
invalid-target `0x9C01` / `+0x00`. The capture also keeps target `04650001`
live at health 1500 while owner `04910001` moves from
Pursuit ages 4968/4976 at 40.100 s to Guard ages 16/16 at 40.150 s, so Aim
crosses the 5,000-ms lifetime first. Focused tick `0x441` independently shows
Chase continuing at 4903/5000, Aim at 5017/5000, bit `0x1000` clear, and C6B0
variant-zero Guard publication. Other low-elapsed switches can also be impact
C690 and are not used as tag evidence. The live owner publishes the
selected graph atomically; after Primary Chase replacement the outer traversal
does not revisit the newborn Primary but freshly reads later Secondary, while
Tertiary Aim replacement has no later slot to visit.

A joined audit of the accepted passive capture establishes the Intro2 target
type without another run. Every nonzero Type-47 behavior target is one of
handles `045D0001`, `04650001`, `04660001`, or `04840001`; their lifecycle
births are all entity type 9 at raw positions `[-16640,-640,32000]`,
`[-16640,0,5376]`, `[-17408,128,5632]`, and `[-18176,128,5632]`. The
authenticated Intro2 wrapper therefore admits type 9, while the separate
Level-1 wrapper continues to require type 46.

The transcript also observes ten synchronous candidate-callback entries at
`0x0040D7A0`, but its candidate-return hook was disabled and it contains no
callback result samples. Static recovery independently closes the complete
`FUN_00401430` body and all eleven direct callees. Its exact component table is
F/H/D/A/I/J/G/K/L/null/axis/E/M/N/O; normal dispatcher priority is F, H, I, G,
followed independently by K and L, while the tail applies C lift, A propulsion,
B lateral correction, and the D terrain/orientation path under their authored
gates. Rust `common_mover::post_dispatch` now owns the exact ordinary C -> A ->
B tail plan: Sub-G suppresses all three phases, Sub-A skips on its zero runtime
gate, and an eligible unresolved Sub-A blocks the plan before mutation.
Ordinary type 9 consumes this shared A/B plan; its authored topology has no
Sub-C. The exact detached Sub-C phase is also shared now, but generic live
component binding, world sampling, and type/callback reachability remain
external.
Preserve the isolated Intro2 presentation proxy for the still-unbound actor
owners and moving-subject presentation work rather than encoding one captured
trajectory. Type-47 Guard acquisition and steady Chase-then-Aim are no longer
proxy-owned. The hooks patch temporary `INT3` instructions but never invoke
retail callbacks or explicitly assign retail game state.

Rust `common_mover::target_prelude` now retains the recovered
`FUN_00401430` target/reversal prefix as one pure, atomic plan. Target lookup
and inactive/dying rejection precede every mutation and RNG draw. With
direction `+1`, a live tracked target, and timer zero, it extrapolates each
signed 8.8 coordinate by
`((elapsed_us as signed i32) << 12) * velocity >> 31`, then commits the
wrapped signed-16 result. Direction mismatch is tested after extrapolation.
When Sub-I exists, mismatch consumes no RNG, adds `0x2000` to heading, and
propagates the authored direction. When Sub-I is absent, the prefix preserves
the retail speed-selection quirk: the clamp path consumes one speed word,
while the non-clamp path discards that first candidate and consumes a second;
X and Z retargeting then consume one word each. A near axis subtracts bits
8..15 from its retained coordinate, but a far axis replaces it with actor
position plus bits 6..15; only far X also subtracts `0x200`.

The 1500-ms reversal timer is installed by the no-I mismatch before the same
frame's `elapsed_us / 1000` decrement. Expiry resets direction to `+1` and,
when Sub-A has a descriptor, consumes exactly one word to reset target speed.
Sub-L receives the final target only after those stages. The later Sub-D
fan-out to F/K/L remains an explicit detached plan because it follows the
Sub-D component callback rather than this prefix. Ordinary type 9 proves
Sub-I, so its bounded adapter integrates only the zero-RNG I route plus tracked
extrapolation and timer expiry; no-I D/F/G/L writes remain unavailable to that
adapter until an exact topology owner supplies them. Tests cover every branch,
RNG count, signed/wrapping arithmetic, and the rule that unresolved evidence
cannot consume RNG or partially commit state.

Rust `common_mover::target_correction` now retains the exact detached
`FUN_00401BB0` phase and both calls made by `FUN_00401430`. The shared
primitive preserves unsigned X/Z subtraction followed by signed low-word
`abs(delta) < 0x0F00` tests, the retail Q31 normalization (including its
integer length plus one), the unconditional active Sub-G descriptor write, and
far-target quarter damping. The first caller starts its correction at zero,
subtracts half of the result from Sub-G word `+0x1E`, and clamps that word to
`[-4000, 4000]`. The second caller clears Sub-O dword `+0x10`, starts with
sentinel 1000, and restores the saved link only when unsigned
`abs(correction) < elapsed_us / 500` and Sub-G byte `+0x3C` is zero. Both
retail calls may therefore damp Sub-G independently in one frame. Entity
lookup, component allocation writes, and live common-mover ordering remain
explicit adapter boundaries.

Rust `common_mover::frame_machine` now composes the recovered mover as a
resumable detached transaction rather than a one-shot approximation. Its
adapter boundary order is target/reversal prelude -> Sub-D -> dependent
F/K/L writes -> primary O/G correction and effect -> Sub-N -> normal
F/H/I/G plus independent K/L dispatch -> C/A/B -> secondary O/G correction.
Every callback, effect, or committed write returns a fresh live snapshot
before the next gate is selected. Sub-D therefore executes before missing F/K
runtime can block; Sub-A's target-speed gate is read only after any Sub-C
write; and Sub-G suppresses C/A/B without inspecting unresolved Sub-A state.
Inactive or dying tracked targets return zero before topology or Sub-A
evidence is consumed.

The protocol cannot silently replay retail work. Commit acknowledgements name
their exact phase, a mismatched acknowledgement leaves the pending action
unchanged, and an evidence failure after a callback/effect moves the machine
to a durable blocked state. The machine itself is deliberately non-`Copy` and
non-`Clone`. Concrete component callbacks, live allocation writes, target and
entity lookup, the primary correction effect, and terrain/wave sampling remain
adapter-owned; this closes ordering, not live actor attachment.

For ordinary type 9, Sub-D bytes are exactly
`{ divisor=20, yaw_roll=0, pitch=0, forward_probe=128,
lateral_probe=64, classifier=0x17, reserved=0 }`. Sub-D performs target
steering plus terrain/occupancy avoidance; it does not attach the actor to the
ground. Within `FUN_00401430`, type 9 executes D avoidance, commits the
resulting heading, then runs I animation, A propulsion, and B lateral damping.
The bounded Rust adapter obtains A/B admission and ordering from the shared
post-dispatch policy instead of maintaining a second type-9-specific gate.
The enclosing normal callback `FUN_0040E870` latches effective flags before
entering `FUN_0040A800`. Ordinary type 9's original-data profile is `0x2F` and
its selected Run Away masks leave that value unchanged, so bit `0x10` is clear:
E870 skips `FUN_0040E640`, preserves the post-task pitch/roll words, and rebuilds
the Q31 body basis directly through `FUN_00413F70`. It then applies environment
drag through `FUN_0040E100`, performs `FUN_0040DF70`'s ground snap at the
**old** X/Z, and later crosses `FUN_0040E370`'s surface timer/RNG/particle
phase. Paths whose effective flags do set `0x10` use `FUN_0040E640`: gain
`0x100` through clearance `0x100` inclusive and `0x40` from `0x101`, with
three `FUN_0041EC70` probes and optional flag-`0x10000` pitch/roll clamping.
That shared helper remains valid but is not part of ordinary Type 9's `0x2F`
outer callback. First-world type 9 has Section-12 bytes
`+0x72=1,+0x73=0` and duration `+0x74=5000` in all three high-resolution
tiers, so `FUN_0040E370` is not a no-op even though every statically recovered
return path returns zero. It saturating-counts entity timer `+0x48` down
outside the strict deep-water band, but counts it up below
`flat_sea_y - model_header_extent(+0x08)/4`. During the last 74 percent it
uses one shared-RNG modulo gate for a class-42 bubble; a successful packet
uses three positive seven-bit position jitters followed by unsigned nine-bit
X/Z velocity arguments, after which the allocator adds class 42's `+0x190`
Y bias. An independent one-in-16 gate requests fixed positional sound 106.
The miss/hit paths consume exactly two/seven shared RNG words when sound is
eligible. Only after the callback returns does
master owner `FUN_00412DA0` integrate X/Z/Y. Snapping after integration samples
a different terrain cell at boundaries and is not retail behavior. The
captured first-world profile uses effective flags `0x2F`, mass 10, drag
strength 3, no model-origin Y offset, and therefore a mode-zero drag factor of
750 at a 20-ms effective owner step. That step is the carry-adjusted/capped
value computed by `FUN_00412DA0`, not necessarily the outer frame delta. Rust
`common_mover::sub_d`, `common_mover::type9`,
`common_mover::type9_attitude`, and `common_mover::type9_tail` retain the exact
bounded phases separately. The attitude API keeps `FUN_0040E640` available for
admitted callers and exposes `FUN_00413F70` independently; the ordinary owner
uses only the direct rebuild. The tail API exposes drag and old-XZ snap as
independent helpers.
`common_mover::type9_surface` now retains the ordinary live underwater timer,
RNG, packet, and sound branches plus a narrow class-42 `WorldFx` materializer.
It deliberately returns a lifecycle-continuation request at the exact expiry
boundary: retail synchronously invokes `FUN_00416750` and `FUN_00410C10`, then
continues from their mutated live record. Selected-owner outer tail now
supplies that pair through `ordinary_type9_standard_death` when the selected
style's release hook is null and the actor is unattached. The
receipt-bound `common_mover::type9_owner` transaction now composes the complete
detached post-task suffix in retail order: direct basis rebuild, drag, old-X/Z
ground snap, one-shot surface planning, particle/sound effects,
the synchronous lifecycle pair when required, a fresh surviving-entity
snapshot, and master motion. Shared RNG is not consumed until the preceding
state commits have been acknowledged, every effect returns a fresh state before
the next gate, and a missing post-lifecycle allocation blocks durably without
reissuing either lifecycle call. Terminal blocks retain the action kind and an
explicit execution bit: adapter failures before an action record `false`,
whereas a missing allocation after the synchronous lifecycle pair records
`true`, so an owner cannot mistake that observation failure for permission to
replay disposal. This detached transaction owns ordering and persistence
without inventing component, entity, RNG, effect, or lifecycle adapters. The
exact Main Base adapter in `main_base_type9_actor_production` now supplies those
bounded live owners, composes the transaction after the class-14 callback in
the specialized live-list pass, and proves the 5,000-ms lifecycle action
unreachable before the task's strict terminal frame. Broader Type-9 cohorts
remain outside that exact adapter. The
type-9 frame oracle stages D -> heading subtraction -> I -> A -> B atomically.
A and B receive the entity's pre-D Q31 basis because retail changes heading
without rebuilding that basis inside the mover. Exact Section-12 component
presence is retained alongside the descriptors. Rust now owns
`FUN_004018A0`'s central route planner: mode zero chooses F else H else I else
G and then independently K and L; nonzero mode chooses F else G and skips the
other branches. First-world type 9 proves A/B/D/I present and
C/F/G/H/K/L absent, so its admission gate requires the shared planner to
select Sub-I alone rather than duplicating that priority ladder. A
compatibility record or a record with a pre-empting/supplementary dispatcher
component therefore cannot enter the oracle. Sub-I's existing Rust controller
already matches its 80-ms, stride-4, eight-direction contract. The exact
type-9 direction-mismatch branch is retained too: when task direction differs
from Sub-A, retail's I-present `FUN_00401A20` path adds wrapping `0x2000` to
heading, propagates direction to Sub-A, consumes no RNG, and continues through
D -> I -> A -> B without rebuilding the pre-write basis. Tracked-target
validation/extrapolation and active reversal-timer expiry remain separate
prelude seams.

`common_mover::type9_transaction` is the detached persistence boundary around
that atomic frame oracle. Construction evaluates the complete mover on copied
state and retains no borrowed terrain or entity data. An applied frame issues
exactly one linear receipt bound to a caller transaction ID, the controlled
entity, and the exact `ActorTaskVisit`; its action carries complete before/after
component, Wander, heading, and velocity snapshots. The adapter must replace
that state atomically and echo the exact committed snapshot. A mismatched
transaction, lease, sequence, or echo returns the still-live receipt and leaves
the action awaiting; an adapter block consumes it into a durable terminal
state. Evidence blocks and retail's intentional zero return issue no action.
This transaction deliberately does not duplicate Wander retarget, lifetime,
tag `0x9C01`, timeout, or task-unwind ownership from the heterogeneous
dispatcher, and it does not authorize live type-9 attachment.

Sub-D's 0x3c-byte allocation also owns process-order-sensitive state. Its
constructor copies process-global byte `DAT_004DB0B0` into runtime `+0x3A` and
wrapping-increments the global once for every successful Sub-D allocation;
each frame then increments the per-allocation copy before staggered row
invalidation. A missing Sub-D consumes neither increment, while an unresolved
descriptor/allocation must poison rather than skip the construction owner.
Constructor `FUN_004203D0` clears the eight classifier rows but leaves cache
origin bytes `+0x38/+0x39` uninitialized. Zero rows make the first returned
class independent from those bytes, but the resulting window placement is not:
allocator garbage can place the first cell at a different edge and alter later
fills. Generic execution therefore cannot treat those bytes as authored
coordinates or silently initialize them.

The detached generic constructor/cache foundation is now exact without
claiming a live allocator. `common_mover::sub_d::construct_generic_sub_d`
retains the constructor-untouched allocator bytes at `+0x00`,
`+0x38/+0x39`, and `+0x3B`; zeroes `+0x04/+0x08/+0x10/+0x14` and all eight
rows at `+0x18..+0x37`; sign-extends descriptor byte `+0x04` into runtime
`+0x0C`; writes zero descriptor words `+0x06/+0x08` back as
`0x0200/0x0100`; then copies the old process byte to `+0x3A` before wrapping
the process owner. Its failed-allocation input leaves the detached descriptor,
runtime slot, and counter unchanged; live allocator slot publication remains
outside that transaction.

`GenericSubDClassifierCache` likewise runs a complete frame against a copy,
including the stagger increment/row invalidation and every requested
classifier lookup, and publishes only after all external classifications
succeed. It retains the exact full reset, positive/negative X shifts,
negative-Z row copy, and the literal positive-Z origin-only shift. A missing
origin or external classification therefore cannot partially advance the
stagger byte or move the window. This closes the generic arithmetic and
mutation contract. It does not supply allocator residue, inject the
process-lifetime counter, authenticate a live allocation, or attach the cache
to a scheduler/entity owner.

Retail has already made menu and Intro2 allocations before ordinary Level-1
villagers exist, so starting this counter when `EntityManager` is created is
not an exact substitute. Live activation must obtain the retained
process-lifetime owner or a capture-proven seed instead of assigning each
villager zero. The accepted pair-runtime census
`runtime_re/captures/local/20260719-140359-pair-runtime-census/` and conversion
tail `runtime_re/captures/local/20260727-235038-base-conversion-pair-tail.txt`
expose live Type-9 Sub-D pointers but do not close construction provenance.

The focused fresh-process captures
`runtime_re/captures/local/20260730-034232-sub-d-constructor-provenance.txt`
and
`runtime_re/captures/local/20260730-035135-sub-d-constructor-provenance.txt`
close the constructor owner and counter sequence. Each contains 56 balanced
entry/allocation/commit/return transactions with the same semantic
owner/type/handle order, no overlap or failed allocation, and all eight cache
rows zero after every commit. Both begin at process counter zero, consume every
pre-increment value `0x00..0x37`, leave the global at `0x38`, and prove that
runtime `+0x3A` receives the old counter exactly once per successful
allocation. The scene tick resets at the Intro2 -> Level-1 handoff while this
counter continues, so a Level-1-local zero owner is not faithful.

Matched owners and allocation ordinals nevertheless receive different
`+0x38/+0x39` bytes in the two runs. The constructor preserves allocator
residue; the bytes are neither authored state nor repeatable constants. For the
captured New Game -> Level-1 route, the six ordinary type-9 allocations have
seeds `0x29,0x2A,0x2E,0x2F,0x30,0x35`. The constructor transcripts stop before
the first `FUN_0041FCB0` consumer invocation, so they do not authorize an origin
policy on their own.

The accepted focused transcript
`runtime_re/captures/local/20260730-064237-sub-d-first-consumer.txt` closes that
boundary for fresh-New-Game ordinary first-world type 9. It joins six
constructor registrations to six first consumers; registration ordinals 2 and
3 are consumed in reverse order. All six calls enter through return address
`0x0041F7A8`, and all six select the full-reset path even though three origins
contain nonzero allocator residue. Each reset replaces `+0x38/+0x39` with the
query cell, yields effective delta zero, returns class zero, advances the
per-allocation seed once, and changes the zero row set to
`[1,0,0,0,0,0,0,0]`. A fresh ordinary type-9 owner may therefore represent
this route as pending a mandatory first-query full reset instead of requiring a
fabricated origin.

That permission is deliberately narrow. Static analysis still proves that a
near-band supplied origin follows a shift path and produces a different cache
window. The detached generic cache can now replay any concrete supplied state,
but Load Game, alternate allocation histories, other Sub-D owners, and live
owners still need their actual origin/process owner or must fail closed; the
fresh-route result is not a generic zero-origin initialization.

`V200002.run` closes the same full-reset caller (`0x0041F7A8` /
`0x00401602`) for type-17 seeds `0x04/0x17/0x31..0x34` and type-8 seeds
`0x0C/0x0D/0x0E/0x38`. Fresh Level-1 type-17 spawns 17--20 now retain
that pending owner; Follow Beacons applies spawn-18 seed `0x32`. Main
Base cargo-conversion scientists retain seed `0x38` and now take a live
Go-To-Job visit through the shared D/I/A/B kernel and the common
`FUN_00412DA0` master-motion suffix. Factory-ejected
type-8 and Intro2 type-8 construction stay fail-closed. Level-1 Type-47
`0x2B/0x2C/0x2D` now apply the `V200003.run` first query; `V200002.run`
only constructed that trio without a `FRAME`/`PRE`. Conversion Type-8 now rebuilds `FUN_00413F70` from post-task angle words
after a successful Go-To-Job callback. First-world Type-8 authors the same
Section-12 `+0x72=1,+0x73=0` pair as Type-9; Type-8 E370 remains unclaimed.

The port now retains this proven constructor/first-use boundary on the exact
six fresh-Level-1 authored entities: spawn indices
`[9,10,14,15,16,22]` receive Sub-D seeds
`[0x29,0x2A,0x2E,0x2F,0x30,0x35]` and a pending mandatory first-query reset.
The admission predicate is shared with their audited pair-collision state so
model slot, model, rotation, world, and authored-index provenance cannot drift.
The same sidecar now retains the deterministic zero steering rate and last-yaw
step written by `FUN_004203D0`; unlike the cache origin and stagger byte, these
words need no process-history inference. Fresh construction now consumes this
custody before link: one shared-RNG selector/context/initializer transaction
publishes the selected or fallback graph, appends the entity, writes the Q31
basis, sets bit `0x4`, and retains one linear manager owner. It does not yet
adopt those branch owners into the live frame scheduler.

A bounded live persistence seam now exists for the later point at which those
missing owners are genuinely supplied. It authenticates the same exact
fresh-Level-1 admission receipt plus the Section-12 A/B/D/I topology, binds one
surviving ordinary-Wander task visit through the controlled entity itself, and
compares the complete D/I/A/B, Wander, heading, and velocity before-state before
performing one atomic replacement. A wrong entity/slot/task lease, stale state,
missing component, or unresolved Sub-A target speed rejects without mutation.
This legacy seam does not install a task or synthesize behavior, RNG, basis, or
surface input. Fresh selected Wander uses its separate production-adopted
linear owner and exact finalized entity state.
