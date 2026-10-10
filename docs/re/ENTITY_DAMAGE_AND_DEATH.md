# V2000 Entity Damage and Death Runtime

This document owns the detailed primary-hit, cure-hit, impact-reaction, standard-death,
and Common-Dying contracts formerly embedded in
[GAME_MECHANICS.md](GAME_MECHANICS.md). Current implementation priority stays
in OBJECTIVE.md; capture status stays in the runtime RE
ledger.

Shared retail damage/death is not level-specific. Production still restricts
several constructor and task owners to Level1/Intro2, so matching actors in
other worlds can lack the callbacks or death context required by these
transactions. The [cross-level audit](CROSS_LEVEL_GAMEPLAY_RUNTIME.md) records
the reproduced Type9 and factory failures and their shared-runtime migration.

## Primary-hit wrapper `FUN_00410EB0` (STATICALLY CONFIRMED)

Native Type9 actors in Intro2 and ordinary authored worlds use
[`ordinary_type9_impact.rs`](../../crates/v2k-game/src/ordinary_type9_impact.rs)
for the full primary/infected wrapper. Shared allocation and completed task
custody precede its first write. The owner transfers all four living behavior
families, retains null carrying/corpse hooks, and publishes replacement/death
owners synchronously. Class45 initial attention has null hit hooks; its acquired
target style invokes C690 for both entries. Its hit-triggered resource cue uses
the current tick before checked damage. See the
[Type9 style matrix](INTRO2_TYPE9.md#primary-static-route-and-infected-hits).

The wrapper resolves and caches the target allocation pointer, then resolves
the same handle again to cache its initial type-record pointer. It sums the two
packet amounts in packet order only where the corresponding channel is exactly
1 or 2, using signed 32-bit wrapping. After the unconditional cached-target
`+0x34 = tick` write it reads the cached type-vtable `+0x14` callback and, when
present, calls it before either independently resolving helper. In the common
vtable this callback is `FUN_0040DAC0`, which optionally invokes current
behavior-style slot `+0x28`.

`FUN_00411030` and `FUN_00415040` each resolve the handle independently. Exact
zero from checked delivery ends the wrapper; every nonzero signed bit pattern
enters the suffix even if nested callbacks performed no health loss. That
suffix never re-resolves the outer pointers: it reads current state,
capability, position, and model slots through the cached target allocation and
accepted-hit sound `+0x80` through the cached initial type record. In-place
callback mutations are therefore observed, while handle rebinding or a changed
type index is not. Retail relies on an allocation-lifetime invariant and can
read stale memory after callback-driven deletion. The detached Rust transaction
instead samples and validates the cached allocation/type identities after
checked damage fully unwinds and samples them again after accepted-hit sound
submission, making that safety divergence explicit without holding a Rust
borrow across callbacks.

Accepted nonzero delivery requests positional `+0x80` sound only when the
cached target's final state is not dying. Independently, cached target byte
`+0x64` bit 3 selects one of model words `+0xA8/+0xAA/+0xAC/+0xAE` from
current `0x2000/0x4000` state and emits class 5 at scale `0x0800`; this branch
still runs on a dying target. Fresh Level-1 Type-47 now applies the detached `FUN_00411030` owner
after C690 and before 15040, using the particle's live signed velocity
words. F780 type-46 applies that same owner with `FUN_00425590` channel-6
sum 0 before 15040. F780 Type-9 `FUN_00415040` filters channel 6 to 4000
and reuses the recovered class-14 `FUN_00410C10` publisher. General F590/F6E0
deliveries similarly route Type-46 player impact reaction and checked damage as
well as Type-9 peasant `FUN_00415040` before Base/Factory damage fallback. Original state bit `0x80000000` fail-closes rather than
submitting `FUN_00469200`.

The bounded fixed Base/Factory/Hive adapter carries the full six-word
`DamageDeliveryRecord` and explicit `EntityHitEntry`. Primary entry stamps
`+0x34` before the checked eligibility/filter gates, including a later blocked
delivery; infected entry preserves the old stamp and returns no accepted-hit
presentation. Its caller first runs `11250`'s infected-model/`+0x82` cue prefix.
EXE styles `4C9480` (Main Base), `4C9558` (Working Factory), and `4C94C8`
(live Hive) have null `+0x20/+0x28` slots, and the existing admitted fixed or
reaction-suppressed state skips `11030`. The filter now uses the actual source
word and both channel words for zero-feedback admission. Intro2 spawn24,
Type67, therefore accepts F780's `[6,0,2000,0,0,0]` as a filtered-out delivery
after its model prefix, with no health, timestamp, feedback, or RNG change.
An accepted lethal Hive hit selects slot 3 and retains `0x4000`, so primary
entry suppresses its `+0x80` cue while retaining the earlier timestamp write.
This does not generalize the survivor whitelist or complete Type9's primary
wrapper. Nonzero primary capability-8 class5 emission remains unimplemented in
the fixed/Hive adapter; infected entry has no such suffix in the original.

[Native Intro2 Type94](INTRO2_TYPE94.md) uses the complete primary/infected
wrapper around its current style callbacks, impact reaction and checked damage.
Its zero-filter channel6 delivery still runs DA00/C690 before15040; it is not
admitted by extending the fixed survivor policy. Native common12 retains its
water components and transfers scheduler custody before the next hit.

Native Type15/87 use
[`intro2_flyer_impact.rs`](../../crates/v2k-game/src/intro2_flyer_impact.rs)
for these wrappers on their authenticated native Intro2 allocation or
[Type15 Hive-child receipt](HIVE_WRECK.md#authored-creature-births-and-failed-world-selection)
and completed Search task graph. Corrected model callbacks and process RNG exposed
infected-turret class5 hits on the bee in the final seed137 replay; the generic
fixed-actor survivor policy had no mobile C690 owner. The two authored damage
profiles both give channel6 multiplier0. Nevertheless, 11250's infected-model
prefix, DA00/C690 replacement and11030 reaction run before15040 filters it to
zero. The hit consumes the actual selector, two6030 initializer draws and three
reaction draws; it does not stamp+34 or borrow primary sound/class5 emission.
Primary nonzero damage retains10EB0's suffix and publishes alternate2/C470 quiet
death synchronously, retiring Search custody while keeping the allocation for
the deferred splice. Focused regressions cover both real native births, zero
filter, primary nonlethal/lethal, retired quiet custody and unreplayed blocked
prefixes. The shared particle router uses this same owner and the caller's
actual notification context for source46's lethal objective hint. Other
ordinary15/87 births and11180 static-route delivery remain explicit boundaries. Neither public type
matching nor a manufactured metadata profile establishes their custody.

The port's phase machine stops separately at the
tick write, style dispatch, impact reaction and optional network request,
checked delivery, both cached-pointer samples, sound, and capability emission.
A caller-supplied transaction identity and private monotonic action receipt bind
each completion to its one issued boundary, so the machine neither reissues an
outstanding action nor accepts a stale/cross-transaction completion. The
external adapter remains responsible for journaling and deduplicating actual
callback, RNG, mutation, and presentation execution by that receipt. Transaction
identities must be unique across all simultaneously live machines. A rejected
phase, completion kind, transaction, or sequence returns the still-valid receipt
to the adapter; only an accepted completion (including an accepted durable
block) consumes it. This lets a misrouted receipt return to its source without
replaying the already journaled external action.

The detached request pre-resolves the common type record's immutable `+0x14`
callback address while retaining the type-record identity. Retail physically
dereferences that slot after the tick write; because the authored type tables
are immutable and the intervening operation is only a scalar tick store, this
is an explicit observational equivalence rather than a claim that the adapter
performs the metadata read at the same instruction boundary.

Native Intro2 births use their own allocation and full primary/infected
wrappers in [INTRO2_TYPE17.md](INTRO2_TYPE17.md), including the differing
Run Away and Capture style20/28 words. The Level1 facade below retains its
separate birth and same-tick primary-hit admission.

`v2k-game::type17_impact_reselection` now closes the normal type-17
`FUN_0040C690 -> FUN_0040AC60` path as an authenticated detached planner. It
requires the fresh-Level-1 model-256 metadata and exact four authored choices:
People Nearby x4 -> Capture People, Player Nearby x3 -> Run Away, Under Attack
x8 -> Run Away, and Always x1 -> Follow Beacons. The two nearby evaluators
reuse the exact first-eligible `FUN_00422C10` walk with capability masks
`0x0C00` and `0x0001`; the Under Attack evaluator sees the tick just stamped at
`+0x34`, so it is false during ticks 0..249 and true for the hit from tick 250
onward. All fallible list evidence is resolved before the selector consumes
its single process-RNG word.

`v2k-game::follow_beacons` now closes class-33 variant zero as a detached
task-owner program. Matched retail/demo code identifies initializer
`FUN_0040B740/0040B750`, the distinct acquisition constructor/tick
`FUN_00402100/00402120` (`00402130/00402150` in the demo), ranked selector
`FUN_00422E30/00422D00`, and synchronous target handoff
`FUN_0040C7D0/0040C7E0`. The initializer clears slot 2, prepares and publishes
the callback-owned acquisition in slot 1, then prepares the shared
`FUN_00402B10/00402BA0` 500-ms retarget task in slot 0 only after phase zero
succeeds. Both successful allocations enter the now behavior-neutral,
type-17-authenticated `FUN_00406070` suffix: enable Sub-H, consume one shared
RNG word, write Sub-A direction 1, then write
`base + (((low16 >> 8) * base) / 0x0A00)` before publication. At
`FUN_0040B740`'s initializer-local return boundary, phase-zero failure consumes
no constructor word and preserves old slots 1/0; phase-one failure preserves
phase zero's effects, draw, and published slot 1 while retaining the old slot
0. The outer `FUN_0040C6B0` caller subsequently consumes that nonzero failure,
publishes its fallback, and clears slots 1 -> 2 -> 0 without rolling the
component effects back.

Every `FUN_00402120` tick first overwrites descriptor `+0x04` with capability
mask `0x100`. The detached task retains its folded descriptor state, while
`v2k-game::type17_follow_beacons_live` now authenticates the real per-actor
copy reached through task `+0x08 -> +0x28` and synchronizes both policy dwords
before selection. The ranked selector rejects self, zero/dying state,
recent relations, missing capability, and candidates outside the strict wrapped XYZ
range, then ranks the signed entity/node word at `+0x88`. A strictly greater
score replaces the best without RNG; an equal score consumes one shared word
and replaces only on odd low16 parity. Eligible zero-score ties therefore draw
and may write the output handle, but final best zero still returns tagged
no-positive result `0x004BE8C8`; negative scores neither replace nor draw.
Only a positive best reaches current style `+0x04`. Variant zero's handoff
stores the target at behavior-context `+0x08`, advances the style, and
re-enters `FUN_0040C6B0`. The heterogeneous dispatcher preserves that
synchronous replacement: a retired slot-1 wrapper cannot propagate its stale
`0x9C02` singleton, nonzero result, or adapter error, and the newly installed
slot-0 task is not revisited in the same 0 -> 1 -> 2 traversal.

Variant one is now closed as a distinct detached family from matched
retail/demo code. `FUN_0040AFD0/0040AFE0` clears slots 1 then 2 before asking
`FUN_00403B70/00403BB0` to prepare a 9,000-ms primary task with the target
copied from behavior-context `+0x08`. Allocation/private initialization,
generic type-17 Sub-H -> one RNG -> Sub-A direction/random target effects, and
the final signed `(base * 4) / 3` Sub-A overwrite all precede publication. A
failure therefore leaves the old primary installed after the two clears and
commits none of those constructor effects. The semantic 0x24-byte private
record begins at the owner's construction-time XYZ, retains the copied target
at `+0x08`, direction 1, and zero reversal timer; padding `+0x06..+0x07` and
allocator residue `+0x20` are deliberately not fabricated.

`FUN_00403CE0/00403D20` cannot reuse Chase Target because it calls the common
mover first. A successful mover's private-record mutations survive subsequent
target-evidence, route, or position failures. It then freshly validates the
saved target, calls `FUN_00423030/00422F00` with the actor-lived route record
reached through controller/component record `+0x28`, rereads owner/target
positions, and
requires wrapped signed-16 X and Z deltas to be strictly below `0x300` (Y is
not part of this final proximity test). Mover zero, invalid target, reached
callback zero, and route rejection return pointer-distinct retail singletons
`0x004BE118`, `0x004BE120`, `0x004BE128`, and `0x004BE130` with tags `0x9C01`,
`0x9C01`, `0x9C00`, and `0x9C01`. A nonzero reached-callback result propagates
unchanged from `403CE0` into the generic scheduler, not automatically out of
the actor owner. A surviving wrapper reads result-object `+0x04`: `0x9C00`
selects style `+0x04`, `0x9C01` selects style `+0x00`, and `0x9C02` is consumed;
all other tags propagate the original object. Owner callback admission uses
state bit `0x1000`, with the exact absent/suppressed fallthrough distinctions
retained before the strict unsigned `elapsed > 9000` check. The scheduler
advances truncated milliseconds before callback entry and discards the result,
adapter error, and timeout if synchronous style replacement retires the current
wrapper.

The bounded live type-17 Follow binding now retains both storage sources.
Entity construction stores type 52's Section-13 spawn dword `+0x1C` as a
signed, type-specific priority rather than aliasing type 61's unrelated
payload. It also retains the actor component/controller record's 8-byte local
common-axis copy. Type 17 admits only the authored
`{ strict=0x0A00, policy=3 }` value or its proven post-callback Follow/Capture
states with policy `0x100`/`0x0C00`; metadata and other actors are not mutated.
Impact-selected class 33 publishes variant zero and both generic
constructor suffixes on the live owner. Its callback lease snapshots live
entities in intrusive order, commits policy `0x100`, performs signed/tie-aware
selection, and on a positive score synchronously publishes the target,
variant-one context, and Primary task. The variant-one generic suffix precedes
the signed `(base * 4) / 3` overwrite, while preparation failure preserves the
prefix/target and enters the exact outer fallback. The resulting owner now drives mover-first `FUN_00403CE0` on the
authenticated type-17 A/B/C/D/H/J route: live target validation,
`FUN_00423030` through controller `+0x28`, and the `0x300` X/Z reached test.
The common mover applies the `V200002.run` first `FUN_0041FCB0` full-reset
for spawn-18 seed `0x32`. Authored Sub-D flags `0x13` match Type-47's
bytes and do not authorize Type-9's reset. Fresh-Level-1 production
adopts the published spawn-18 variant-zero graph and ticks that
acquire/handoff path into this first following frame. The live visit
still fail-closes when the tracked target or unpublished birth Q31
basis is missing. No focused capture is required.

Audited Run Away styles remain a null-callback boundary, Capture People
variants 2..5 retain their distinct `FUN_0040D040` cleanup boundary, and state
bit `0x4000` returns the alternate class-12 choice without evaluators or RNG.
The planner returns the selected behavior and the exact shared B6C0 acquiring
setup request without mutating an actor. Capture People class 9 and Run Away
class 10 use the same target-acquisition and duration-owned retarget topology:
slot 1 is callback-owned; slot 0 expires only when elapsed time is strictly
greater than 500 ms; a `0x004BE138` / `0x9C01` mover result precedes that
timeout; and target RNG writes commit before the staged common-mover suffix.
The real type-17 record has components A/B/C/D/H/J and Sub-A base 250, so each
successful slot-1 and slot-0 allocation independently runs `FUN_00406070`:
enable Sub-H, consume one additional shared RNG word, then write Sub-A
direction 1 and target speed
`250 + (((low16 >> 8) * 250) / 0x0A00)` before that task is published. Full
setup therefore consumes two constructor words, and phase 1 leaves the final
Sub-A result. Neither prepared Rust task can be released without exposing its
own ordered effects.

`v2k-game::type17_impact_live` binds that detached proof to one live Entity
without accepting a replayable selection receipt. Before the planner can enter
the shared `WorldFx` RNG, it authenticates the current named type-default
context, the wrapper's same-tick live `+0x34` hit stamp, active model and all
four model-256 slots, exact A/B/C/D/H/J metadata, Sub-A base 250, the
eight-record Sub-H descriptor, and matching live Sub-A and Sub-H allocations.
Missing or stale evidence is therefore a zero-draw, zero-mutation error.
Capture People class-9 variant zero now publishes through the same bounded
`FUN_0040B6C0` owner as Run Away, while variants 2--5 retain their separate
capture-gated cleanup. Follow Beacons returns its distinct live variant-zero
publication. All three consume two constructor words on full setup. A selected
Capture People or Run Away first publishes its class-specific descriptor/style
context while preserving target and auxiliary words. B6C0 then restores the
actor-local common-axis `+0x04` word from the type record before clearing slot
2. Phase 0 allocates its acquisition task into
scratch, applies Sub-H enable -> one shared RNG draw -> Sub-A direction ->
Sub-A target while the old slot-1 task remains installed, and only then
publishes slot 1. Phase 1 repeats that complete suffix while the old slot-0
task remains installed, then publishes the retarget task into slot 0. The
phase-zero constructor separately retains initial style `+0x44`: Capture
People uses `0x0C00` and Run Away uses zero. Thus Capture starts from the
type-authored filter 3, carries override `0x0C00` in the task, and applies that
override when its acquisition callback runs. The pre-clear actor write survives
either allocator failure and the enclosing fallback.

`FUN_0040B6C0` itself is deliberately not an atomic transaction. At its return
boundary, a failed slot-1 allocation has cleared slot 2 while preserving the
old slots 1/0 and consumes no constructor word. A failed slot-0 allocation has
additionally committed phase 0's Sub-H/Sub-A effects, its one constructor word,
and the new slot-1 search while preserving the old slot 0. Full-game
`FUN_0040C6B0` and demo `FUN_0040C6C0` prove that this is only the nested
initializer-local checkpoint, not the outer final state. Either nonzero return
immediately installs special fallback descriptor `0x004C8888`, style
`0x004C74F8`, and raw context index 0, clears live state bits `0x00068000`
through the normal `+0x34` policy `0x1280`, then fallback initializer
`FUN_0040C4D0` clears slots 1, 2, and 0 in that order. The fallback does not
roll back any component effects which ran before a later failure.

The live Rust Entity authority now retains one behavior-context value rather
than a style alone: named descriptor identity, choice-list-source evidence,
target handle `+0x08`, auxiliary word `+0x0C`, raw style-table index `+0x10`,
and the active audited style. The special fallback remains an unnamed identity
with descriptor/style `0x004C8888`/`0x004C74F8`; it is not fabricated as class
0. Class 12 is separately catalogued at descriptor `0x004C88D0` with style
table `0x004C7ED0` as an audited alternate program. Retail reaches it through
the type record's Section-12 `+0x124` alternate slot when state bit `0x4000`
is set; it is absent from the retained authored `+0x118` weighted-choice
lists. Descriptor replacement preserves the target and auxiliary words. The
type-17 live publisher now consumes this authority for both bounded B6C0
transitions; other behavior owners still must publish their own selected or
fallback result. Constructor-time behavior selection remains a separate,
fallible phase rather than permission to expose the selected context early.

The exact fresh-New-Game Level-1 type-17 birth owner is now live. Retail follows
`FUN_004104B0 -> FUN_0040D4A0 -> FUN_004381F0 -> FUN_0040AC60 ->
FUN_00425680`, then installs and initializes the selected descriptor through
`FUN_00438340 -> FUN_0040ABE0/FUN_0040ABB0 -> FUN_0040C6B0`. The matched demo
constructor `FUN_00410440`, class-9/10 initializer `FUN_0040B6D0`, class-33
initializer `FUN_0040B750`, and outer `FUN_0040C6C0` preserve the same ordering.
The allocation is not linked until the selected initializer or its terminal
fallback has completed, so spawn 17 sees only the earlier live prefix and each
later spider additionally sees its already-published predecessors. The null
relation sentinel remains distinct from the persistent player's port handle
zero.

Fresh `+0x34` is zero, making Under Attack false at every possible construction
tick: below tick 250 the signed startup guard rejects it, and from tick 250 the
strict 250-tick window rejects `current_tick - 0`. Each full-success birth
therefore consumes exactly one unconditional selector word and the two
phase-local initializer words, all from process-global `WorldFx`, before list
publication. A selected task-initializer failure retains the proven prefix and
publishes the unnamed fallback; unsupported/direct-level construction remains
unresolved.
The natural captures corroborate the non-hardcoded result: spawns 17--20 are
Capture, Follow, Follow, Capture in `20260712-203635` and Follow, Capture,
Follow, Capture in `20260717-032945`, with state `0x07468805` for every actor.
Those different cohorts reflect earlier process-global RNG history; the port
consumes the correct local stream order and does not assign a behavior by spawn
index. Earlier Level-1 initializers and other process-global RNG consumers are
not all live-bound yet, so a cold port stream is not expected to reproduce
either captured cohort absolutely. Retail can also propagate `FUN_0040ABE0`'s
context `Mem_Alloc` failure after the selector draw and before linkage; the port
stores `BehaviorContextRuntime` inline and therefore has no corresponding
fallible production allocation. Both are explicit integration limits, not new
capture requests. No new birth capture is required.

Full-game class-23 `FUN_004257F0` and demo `FUN_004256C0` prove Power Up is the
sole reachable
initializer with fixed writes and an unconditional zero return. Its fresh
context therefore knows the type-default source, null target, zero auxiliary
word, raw index 0, and selected style directly. Every other reachable
initializer has an allocator/task/component error path. Separately, the exact
fresh-New-Game post-Intro evidence seeds the observed player class-24 and
Level-1 Main Base class-41 selected contexts; generic/direct-level construction
keeps both unresolved.

The exact type-17 eight-record Sub-H allocation is attached to Entity storage
with enabled state 1, cursor zero, and zeroed records. The same constructor seam
now admits only fresh post-Intro Level-1 type-47 spawns 11--13: model 302, the
exact A/B/C/D/E/H/J topology, signed Sub-C `+0x0C` value zero, and all six
authored Sub-H rows must be present before their enabled zeroed runtime is retained.
Generic/replay construction and every mismatched type-47 profile remain
unresolved. Other Sub-H owners remain unresolved because the detached runtime
does not yet retain the outer `+0x0C` word which their Sub-C descriptor may
enable. Modeled B6C0 phase-allocation failure now reaches
the proven outer terminal result rather than becoming an evidence error: the
selected context is replaced by the unnamed fallback, state mask `0x00068000`
is cleared, and slots 1 -> 2 -> 0 are removed. Capture-People variants 2--5
materialization/cleanup plus concrete common-mover/owner adapters remain
outside this checkpoint. Primary-impact scan placement and the bounded Follow variant-one
publication/handoff are complete. Static full-game/demo writer tracing also
closes the live Follow storage provenance. The exact authored spawn-17--20,
type-17/model-256 records now retain the census-proven slot-0 descriptor-contact
callback, absent slots 1/2, and null instance modifier/type-hit callbacks;
ordinary pre-Common-Dying pair orientation remains unresolved rather than
inferred from model or spawn data. The later authenticated death scheduler's
post-task matrix publication is a narrow consumer exception, not authority for
that general pair path.
Matched retail `FUN_004104B0` / demo `FUN_00410440` plus the exact Level-1
terrain and sea values also close their constructor surface domain. The wave
branch is unreachable at all four cells, each authored Y is strictly above the
static sea surface, and only the fresh post-Intro route now resolves
`0x00200000/0x00400000` to fully-above. Their exact pre-task state is
`0x07468805`; generic/direct starts retain unknown surface bits.
The ordinary null-hook impact/damage/death composition is now a bounded live
owner for the naturally published exact fresh-Level-1 context, not a
focused-capture blocker. The
candidate walk no longer requires another actor's complete state dword:
retail `FUN_00422C10` and exact-match demo `FUN_00422AE0` read only
`state != 0 && (state & 0x5000) == 0`. `GuardLocationEntityRef` now retains the
actual `RetailStateWord`; known exclusion bits or exact zero reject, a known
nonzero bit plus `0x5000` known clear admits, and genuine ambiguity still stops
before capability reads or selector RNG.

## Cure-hit wrapper `FUN_00411320` (STATICALLY CONFIRMED)

The [Antidote firing contract](PLAYER_CRAFT.md#fire-both-modes) owns selector7's
F7C0 packet and terrain binding.11320 clears model bit2000, requests the current
type's +84 cue even for a dying target, then calls type-vtable+1C before11030
and15040. It never stamps+34 or emits primary-hit sound/class5 feedback.
Common DA60 dispatches current style+24 independently from infection's+20;
attached Capture styles use D040 for cure while their infection hook is null.
Production admits audited native owners and the local player's authenticated
null Player Control hooks. The [player firing contract](PLAYER_CRAFT.md#fire-both-modes)
records the common-vtable loader and the correction:447AF0 belongs to contact
style slots+10/+14, not cure+1C. The direct player checked request carries the
complete six-word delivery and explicit unsigned numerator/denominator through
the unchanged four-word255E0 filter arithmetic, then uses the actual hull.
That entry never stamps+34 or dispatches primary/style/11030 effects; the
infected/cured wrapper owns its preceding prefix and impulse separately.
The explicit Checked/Unchecked policy distinguishes15040 from14E10: both
retain255E0 and the four filter words; only Checked owns bit8000 and source46
filtered-zero feedback. Neither direct entry stamps+34 or runs11030. Lethal
local player damage synchronously publishes health0/DYING, the death cue,
authored class25/model1/3, immediate475F0, CA/D9 text, signed208 latch seed and
ordered task cleanup. Its actual resource, FX, hull, notification and extra-life
custody travels in one frame; [the player death contract](PLAYER_CRAFT.md#player-contact-damage-and-death-retail-traces-2026-07-17)
owns constructor/contact cadence and the BB8 wrapper. Source46 lethal resource4
feedback follows successful native death. Remote owners, unresolved fields,
nonempty484A0 cargo, pending craft replacement, logical sound release and the
filtered-zero selector23/DB continuation report named preflight blocks.

## Primary-hit impact reaction `FUN_00411030` (STATICALLY CONFIRMED)

`FUN_00411030` is the common impulse/jolt phase reached after the optional
current-style impact callback and before checked health mutation. It applies
nothing unless entity state word `+0x08` contains enable bit `0x04000000`; set
bit `0x08000000` suppresses it even when enabled. Both suppression paths return
before reading mass, consuming RNG, or mutating the entity. An eligible call
reads unsigned mass word `+0xB0`. Retail divides `0x00400000` by that mass,
narrows the quotient into the signed wrapping multiply with the caller's signed
impact, and arithmetic-shifts the product right 16:

```text
scale = ((0x00400000 / unsigned_mass) * signed_impact wrapping i32) >> 16
```

Mass zero would take retail's integer divide fault. The detached port reports
it as an atomic explicit error before RNG or mutation rather than inventing a
fallback mass.

The caller supplies the particle's three live signed velocity words directly;
they are not normalized, reconstructed from the launch direction, or adjusted
relative to the shooter. `FUN_0043F590` passes `particle + 0x0E` after that
frame's class update, so same-frame gravity and underwater drag are already
present. `v2k-game::world_fx::ParticleEntityImpact` now retains those exact
post-update `[i16; 3]` words with each accepted entity hit. The bounded type-17
owner consumes them at F590; generic outer impact ownership remains external.

Retail consumes those words as signed Q15 direction. It updates signed-8.8
linear velocity words `+0x9C/+0x9E/+0xA0` in X/Y/Z order by wrapping each
existing word with `(direction_axis * scale >> 15)` narrowed to 16 bits. It
then consumes exactly three 16-bit samples from the process-shared RNG—even
when signed impact and scale are zero. Consumption order is heading, roll,
pitch. Contiguous storage is heading, pitch, roll at
`+0xA2/+0xA4/+0xA6`; the RNG/working order deliberately visits the last two in
the opposite order. Each angular delta is:

```text
angular_delta = (((random16 >> 5) * scale) >> 15) - 0x0400
```

The multiplication/shift and final signed-16 addition follow x86 wrapping
semantics. Consequently a zero-impact eligible call leaves linear velocity
unchanged but still consumes three samples and subtracts `0x0400` from every
angular word. Raw assembly derives all three linear and all three angular
deltas—and consumes all three RNG samples—before mutating the entity. It then
commits velocity X/Y/Z followed by angular heading/roll/pitch; this final angular
write order is not contiguous address order.

After those commits, original state bit `0x80000000` requests
`FUN_00469200(1, 10, &entity_handle)`. The request is absent on either
suppression path and on the portable zero-mass error, and it is based on the
entry state word rather than any later mutation. `v2k-game::impact_reaction`
retains this exact detached state gate, mass/scale arithmetic,
heading-roll-pitch RNG contract, six wrapping commits, and typed post-commit
network request. It does not resolve an entity, invoke the preceding style
callback, own the shared RNG, subtract health, select a dying behavior, or
authorize types 17/47 in the live damage path.

## Standard death audio before class49

Native source profiles in the shared
[explosion owner](../../crates/v2k-game/src/class49_death.rs) distinguish
the alternate's terminal policy. `40BAC0` (class1/style4C7150) executes
BAF0, A860 and10B70; `40BD20` (class49/style4C71E0) executes the same
blast/clear prefix, then conditionally constructs a Type60 ring before10B70.
Type115 flowers and native Intro2 Type13 use Class1; defensive turrets, the cleansing vehicle and
ordinary Type61 pickups use Class49. This is shared source-backed structure,
not a substitution of class49 for a flower's authored alternate.

BAF0's ten-scatter default becomes sixteen classes94/95 only when A/B/N/G
are all absent and capability40 is set. Type115's signed type112..115 branch
uses ten class37 particles; the cleansing vehicle reaches that same particle
class through its present A/B branch. Native Type13 reaches40BBB3 through
its present G branch at40BB52, also retaining ten class37 attempts. Its actual
Search/Aimless predecessor+28 cleanup hooks are null; the Class1 style4C7150
has null solid, water and static hooks. Each retains its own damage template,
current-style death cleanup and source allocation. Radial delivery completes
synchronously before A860; nested flower death therefore completes without a
ring before the outer turret's class49 ring is appended.

`FUN_00410C10` first zeros health and sets dying, then requests the type header's
nonzero `+0x90` sound at fixed gain/rate `0x10000`, before the type death callback.
The shared [class49 owner](../../crates/v2k-game/src/class49_death.rs) retains
this prefix separately from `40BAF0 -> 440950`'s later randomized sound62.
The fixed cue consumes no shared RNG; the scatter suffix still draws its one
rate word. Both requests enter the immediate audio list in source order, so a
later pending-event flush cannot reverse them. Remote and repeated death
entries emit neither request again.

After that cue,10C10 stops the constructor sound attachment at+8C before the
type death callback. Ordinary Type61's null-cleanup class23/style4C96C0 owns
sound44, so its native standard-death entry authenticates that attachment,
clears it and then enters direct alternate49. It has no actor task or synthetic
scheduler owner. The native receipt remains independent from the older bounded
Level1 and factory-born Type61 death adapters.
If a pickup was accepted earlier in the same frame, its existing Type61
deferred queue still owns removal. The shared death receipt retains that
typed owner through blast/ring completion;10B70 then adds no second queue
entry. Contradictory pending state/queue custody blocks before the first
death mutation.

Currently admitted turret and rover records have a zero `+0x90` selector.
Type83 authors sound62, requiring two distinct requests once its native
lifecycle is admitted. A nonzero parsed-data fixture tests the common prefix;
it does not establish native Type83 admission.

**Shared underwater scatter sound position.** In
game_logic.c, `440950` passes its local position to
`440A20`, which subtracts `0x40` from X for the second underwater particle.
The subsequent `44F450` sound62 request reads that modified position.
[WorldFx](../../crates/v2k-game/src/world_fx.rs)'s
`emit_scatter_surface_sound_tail_raw` now retains that wrapping signed-word
mutation for the randomized sound even when particle allocation fails. The
above-sea branch keeps the input position. The player-only `4475F0` fixed-rate
second cue independently reads the original entity position. Boundary tests
cover sea equality, signed X wrap, a saturated pool, cue order and the single
rate RNG draw. This shared helper also precedes Alpine's class18 split births;
the recorded above-sea Alpine death does not establish underwater acceptance.

## Auto Pilot class63: explode and drop the authored power-up (STATICALLY CONFIRMED)

The class table at `0x004C8AA4` holds 8-byte `{descriptor, name}` pairs; class63's
entry `0x004C8C9C` names **"Auto Pilot"** with descriptor `0x004C8828`
`{style table 0x004C7198, style index 0}`. `40ABB0 -> 40C6B0` installs that style
and calls the `0x48`-byte style entry's `+40` initializer with its `+44` argument.
At runtime (faststart04 recording) entry `0x004C7198` has every callback slot null
and initializer `0x0040BC90`, argument zero. Neither Ghidra's default analysis nor
the bulk decompilation defines that function; WinDbg disassembly gives:

1. `43A580` resolves the actor, then `40BAF0` runs the shared scatter/radial death
   (the same BAF0 that class1 and class49 use).
2. A zeroed `0x4C`-byte construction record receives `DAT_004DCA00` at `+00`,
   type `0x3D` (61, Power Up) at `+08`, the actor's position words `+96/+98/+9A`
   at `+0C..+11`, and the actor's own word `+88` at `+20`; `438080` builds it and
   `4575A0` dispatches the result.
3. `410B70` requests the actor's deferred removal.

Unlike class49's `40BD20` it neither calls `A860` nor tests the remote bit, and it
leaves a Type61 rather than a Type60 ring. Entity `+88` is the Section-13 spawn's
extra word, which Type61 itself reads as its packed power-up payload. Every
alternate-63 type authors such payloads (`0x3F`, `0xC35035`, `0x43A`, `0x1F40C`
and so on): Type71 (25 births), 80 (3), 81 (3), 117 (2), 124 (3), 126 (4), 127 (25),
128 (22) and 129 (25). Destroying one of these carriers therefore explodes it
and drops the power-up it was authored with.

The port has no class63 program yet; Type124 fish stop at it explicitly
([FISH_RUNTIME](FISH_RUNTIME.md)). Implementing it needs the BAF0 terminal
owner for each family plus a dynamic Type61 birth from a packed payload, which
factory production already performs for its own pickups.

One evidence gap blocks a faithful terminal. Because `40BC90` never calls
`A860`, the dying carrier keeps its task slots until removal. `413500` walks
the whole live list (`414920` returns `DAT_004DB090` unfiltered) and sweeps
deferred removals only at the end of its walk (`414990`). So a carrier killed
outside the walk (by a particle in `440120` or a contact in `411A80`) is visited
once more by the next frame's walk. `412DA0` has no dying test; its type
callbacks still run while state bit `0x20000` is set. Whether installing
class63 clears that bit, and so whether that last visit moves or acts, is not
settled statically. A recording of a carrier's death (any world with Type71,
124 or 127..129) would close it; the port's finished-terminal rule (empty task
slots) must not be assumed for class63 until then.

## Rolling Boulder class20 (STATICALLY CONFIRMED, not ported)

Class20 (table entry `0x004C8B44`, descriptor `0x004C8908`, style table
`0x004C78A0`) is the behaviour of Type3 (5 births) and Type27 (25 births),
which have no components. Its two style entries were read at runtime:

| Style | Callbacks | Initializer | Primary task |
|---|---|---|---|
| 0 (`4C78A0`) | `+04`: `40C750` | `40B950` | `404580(slot0, 0)`, the rolling body task the Intro2 meteors build with 5000 |
| 1 (`4C78E8`) | `+04`, `+18`, `+28`: `40C730` | `40B9B0` | `404B40(slot0, 0)` |

Both initializers first clear Tertiary and Secondary through `40A7A0`.
`40C750` and `40C730` call `40C6B0` with style index 1 and 0, so the boulder
switches between the two. `404B40` builds a single-callback task (`404B60`):
while `sqrt(vx²+vy²+vz²) > 100` it keeps running, otherwise it zeroes the
velocity and settles. The port implements `404580` for meteors but not
`404B60` or the switches; Type3 dies through class1 and Type27 through
class18 (Split And Explode).

## Flip Over And Die class-12 task shell (STATICALLY CONFIRMED, RUNTIME VALIDATED)

Canonical type records for 17 and 47 both author alternate behavior class 12,
`"Flip Over And Die"`. Its `0x004C7ED0` initializer `FUN_0040C620` clears task
slot 1, then slot 2, then attempts to install the 9,000-ms slot-0
`FUN_00404220` task through `FUN_00404120`. A failed preparation does not
publish a primary task, and the two earlier auxiliary clears are not rolled
back. The death transition sets low model-selector bit `0x4000` without
clearing high bit `0x2000`; accepted Level-1 actors were observed in model slot
1, but that observation is not a general selector proof.

`FUN_00404120` first constructs the generic fixed-duration 0x34-byte task, with
no task-private constructor or destructor, and calls shared setup
`FUN_00406030`. Setup failure occurs before owner velocity, constructor
suffixes, or slot publication. On success, `FUN_00406030` runs the
descriptor-gated `FUN_00406070` component effects in this fixed branch order:

1. A present descriptor `+0x24` target sets runtime dword `+0x08` to 1.
2. A present `+0x20` target first clears runtime byte `+0x3F`, then consumes
   one process-shared RNG sample. It writes runtime `+0x38` as
   `(low16 >> 8) + signed word at that descriptor's +0x0C`, clears dwords
   `+0x24`, clears bytes `+0x40/+0x3C`, and copies descriptor dword `+0x00`
   to runtime `+0x20`.
3. A present `+0x1C` target clears bytes `+0x3C/+0x3D` and writes `0xE000` to
   dword `+0x24`.
4. A present `+0x08` target consumes the next process-shared RNG sample—or the
   first sample if the `+0x20` branch was absent—sets target dword `+0x04` to
   1, and writes:

```text
target[+0x00] =
    signed_base
    + trunc_toward_zero(((low16 >> 8) * signed_base) / 0x0A00)
```

Thus construction consumes zero, one, or two RNG samples according to the
actually present descriptor branches; it must not pre-draw a fixed pair. After
shared setup, the class-12 suffix writes raw signed-8.8 owner velocity Y
`+0x9E` to 500, applies the inverse ready-state writes for the present targets
(`+0x08` target dword `+0x00 = 1`, `+0x24` runtime dword `+0x08 = 0`, and
`+0x20` runtime byte `+0x3F = 1`), then publishes the requested slot. The
effects and suffix are ordered side effects, not a declarative final-state
merge.

`FUN_00404220` returns singleton `0x004BE170` (tag `0x9C01`) immediately after
its ordinary task/type/component context resolution when the scheduler-mode
argument is nonzero. That path performs no terrain effect, common movement, or
damping; the tag is a request for the generic style-0 transition path, not
proof that a live transition succeeded.

In ordinary mode, the callback derives a signed effect byte from descriptor
`+0x10 -> +0x0C`, defaulting to zero when the pointer is absent. An attached
cargo/component mass query through runtime `+0x14` suppresses that byte to zero
when the returned mass is strictly greater than 150. It then always calls:

```text
FUN_0041EC70(owner, 0x100, signed_effect_byte, 1, elapsed_us)
FUN_00401430(task, owner, type_runtime, component_runtime,
             null_target, elapsed_us, 0)
```

The common mover's return value is ignored. After it returns, retail performs
fresh owner lookups and damps only signed-8.8 linear velocity X/Z
`+0x9C/+0xA0`; Y `+0x9E` is untouched. For each damped axis, the exact wrapping
update is:

```text
delta = (i64(i32(elapsed_us << 11)) * i64(i32(velocity_axis))) >> 31
velocity_axis = wrapping_i16(velocity_axis - delta)
```

The callback consumes no RNG and returns null in ordinary mode. Before invoking
it, the generic scheduler adds `floor(elapsed_us / 1000)` to the task's
millisecond counter. After callback unwind, a present tagged-result transition
has precedence over the timeout check; with the ordinary null result, the task
completes only when the already-updated counter is strictly greater than 9,000.
Completion enters `0x004C7F18`, clears all tasks, and marks deferred
destruction.

`v2k-game::common_dying` retains this ordered initializer, conditional setup
effects/RNG cadence, constructor suffix, callback plan, damping arithmetic, and
post-unwind scheduler contract as a detached exact shell. Its runtime state
participates in the central heterogeneous actor-task dispatcher, which commits
the elapsed prefix before callback, gives a surviving tagged result precedence
over the strict timeout, discards callback errors after self-clear/replacement,
and re-reads later slots freshly.

`v2k-game::actor_standard_death_live` closes the exact fresh-Level-1 type-17
publication boundary at `FUN_0040DB80`. It does not trust the public callback
payload by shape alone: the complete action plus receipt transaction/sequence
must match the private outstanding DeathCallback state of the real nested
`PrimaryHitCheckedDamageCoordinator` child, including callback address and
target. A forged phase, action variant, callback, target, transaction, or
sequence returns the original issued action intact, leaves the child waiting,
and performs no entity or RNG mutation.

After authentication, the publisher rejects an already queued deferred splice
and resolves the target afresh through `EntityManager`, matching `FUN_0040DB80`'s
post-hook existence boundary. The admitted type-17 styles have a null `+0x2C`
death hook. Before mutation it proves the authored fresh-Level-1 spawn, active
type-17/model-256 allocation and metadata, exact health zero, selector low bit
set/high bit clear, no deferred-destroy bit, a type-default class 9/10/33 or
exact initializer-fallback context with null death policy, alternate class 12,
the A/B/C/D/H/J topology, common-axis descriptor, Sub-A base 250, and matching
eight-record descriptor/live Sub-H plus live Sub-A. It also prepares the exact
class-12 task and constructor suffix before publication.

The committed order is selected class-12 context/state policy, clear slot 1,
clear slot 2, prepare the replacement primary, enable Sub-H, consume one shared
`WorldFx` RNG word for the type-17 Sub-A target, write Sub-A direction/target,
write raw velocity Y = 500 while preserving X/Z, restore the suffix ready
states, and publish slot 0 last. The resulting visit is immediately wrapped as
the synchronous `LevelOneType17CommonDyingOwner`, then the same checked-damage
receipt is acknowledged. A modeled primary-preparation failure occurs after
the two auxiliary clears, consumes no RNG, preserves the old primary until the
outer initializer-failure result replaces the context, applies policy `0x1280`,
and clears slots 1 -> 2 -> 0. Full-game `FUN_0040DB80`/`FUN_0040C620`/
`FUN_00404120` and the structurally matching demo chain define this ordering;
no additional retail capture is needed for class-12 publication itself.

Native Type47 now carries its actual construction receipt through the shared
particle, radial, Class12 and abort owners in [TYPE47_RUNTIME.md](TYPE47_RUNTIME.md).
The captured first-world publication and coarse owner described below remain
replay controls; their three spawn identities no longer restrict production
gunner damage or retirement in other worlds or native Intro2.

`v2k-game::ordinary_type47_death_live` closes the sibling captured post-prefix
publication for exact fresh-Level-1 type-47/model-302 spawns 11--13. The
boundary begins only after the generic death prefix has committed health zero,
selected the low model bit without clearing the captured high selector, and
requested retail death sound 75. Accepted-hit presentation remains owned by
the preceding hit pipeline rather than being relabeled as part of this suffix.
The publisher authenticates the transition-relevant retail profile:
local/nondeferred state, null current-style death hook, Guard-v0 or Wander-v0
context, the captured Wander primary plus Guard's optional acquisition
secondary, alternate class 12, initializer/default flags `0x2039`, common axis
`(1792, 5)`, exact A/B/C/D/E/H/J topology, Sub-A
`{1500,-3000,300}` with constructor drive scale 100, signed Sub-C effect zero,
all six authored Sub-H descriptor rows, and a matching six-record live runtime.

The committed suffix selects class-12 style `0x004C7ED0`, applies its
`0x10000` state bit, clears secondary then tertiary, consumes exactly one
shared `WorldFx` word for Sub-A target `300..329`, preserves velocity X/Z and
the high model selector while writing raw Y = 500, restores Sub-H disabled,
and publishes primary `0x00404220` last. This reproduces captured states
`0x0143C805` for spawns 11/12 and `0x0143E805` for spawn 13. A final primary
preparation failure retains the already-selected class/state and both
auxiliary clears, preserves the old primary, and consumes no RNG or component
effects. The post-prefix API deliberately returns only a receipt without
scheduler methods.

The sibling `publish_fresh_level_one_type47_standard_death` entry now owns the
exact preceding generic helper too. Stable fresh-Level-1 cohort/metadata
custody is authenticated before classification, but remote ownership wins over
the dying bit and both remote/already-dying exits skip task/component readiness
and make no writes, sound requests, or RNG draws. Fresh local dispatch accepts
any current health evidence because retail overwrites it. It preflights the
complete class-12 transaction before the first write, then commits health zero,
ORs only selector bit `0x4000`, queues fixed-rate logical sound 75 at the
pre-callback raw position, takes the statically proven null `+0xB4` release
branch, and runs the prepared suffix. Re-entry sees the dying bit and cannot
repeat sound or RNG. The request is for logical sound 75; the accepted abort
audio timeline did not start its PCM-18 alias, so backend audibility is not an
acceptance assertion.

The next coarse-owner profile is also statically closed across all four retail
X3 tiers and both demo X3 tiers. Section-12 bytes `+0x72/+0x73` are `1/0`,
`+0x74..+0x77` is 2,000 ms, and model 302's header extent `+0x08` is 182
(distinct from collision radius 140). Sub-J descriptor
`01 00 01 00 00 00 00 00 00 00` authors one policy-1 zero-offset slot;
retail `FUN_00418330` and demo `FUN_004182F0` construct live length zero,
capacity one, outer policy zero, and one zeroed row. Initial flags `0x2039`
with class-12 clear mask `0x2015` produce effective flags `0x28`. The accepted
abort states keep relation bit `0x1000` clear and parent null through
publication, so terminal variant 1 is unsuppressed.

The same census samples instance `+0x44` and type-hit slots as zero on
those three bodies. `v2k-game::type47_checked_damage` therefore applies
surviving `FUN_00415040` null-modifier arithmetic after C690 without
adding Type-47 to the Base/Factory projectile-survivor whitelist.
A nonzero 15040 return then queues type `+0x80` (Level-1 cue 92) when
entity `+0x08` is not dying. Lethal 15040 calls the recovered Type-47
`FUN_00410C10` standard-death publisher rather than inventing a second
prefix; 10C10 sets dying first, so the accepted-hit cue is skipped.
Capability `+0x64` bit `8` is recovered as `0x8` and emits class 5 at
scale `0x0800` through the shared `FUN_00440DC0` owner. Slot selection
uses current `0x2000`/`0x4000` model words; Z subtracts model 302
header `+0x08` extent 182. That branch still runs after lethal 10C10.
An already-dying 15040 still returns the filtered amount after
`FUN_00414E90` consumes only the pre-health buffer; health and 10C10
stay untouched, and the suffix skips `+0x80` because dying is already
set.
The live Type-47 hit request now distinguishes primary10EB0 from infected11250.
11250 sets2000 and queues the authored82 cue before type-vtable18/DA00 dispatches
current style20; it never stamps34 and never executes the primary80/class5
suffix. Native class32 styles4C7BB8/4C7C00 and class6 style4C79C0 have C690 at
both20/28; class6 style4C7A08 and class12 variants0/1 have null callbacks at both
slots. Null callbacks preserve the graph while the independently resolved
impulse and checked helpers continue. C690 replacement is immediately retained
by the scheduler rather than invalidating the next frame's task lease.

F780 forwards the immutable descriptor packet at4CBFD0:
`[6,0,2000,0,0,0]`. Its trailing source/owner words are zero independently of
particle birth ownership. The canonical Type-47 profile filters this packet
to zero after model/behavior/impulse effects. Capability8 alone does not enter
filtered-zero feedback:415040 additionally requires source46 and
`(channel0 != 1 || channel1 != 0)`, not an amount0 comparison. A meteor's
class16 packet, by contrast, supplies channel1/2500 and admits500 damage.
The explicit entry and full delivery record preserve both outcomes.

Intro2 Types9,17,26 and53 use the same checked phase with their own behavior/death
custody; see the [peasant](INTRO2_TYPE9.md) and
[spider](INTRO2_TYPE53.md) hit contracts and
[Type26 task authority](ACTOR_TASK_PROGRAMS.md). Native Type26 class12 retains
its model267, mass400, Sub-A base200, ABCDH topology and authored default0439;
effective0428 bypasses the same terrain-attitude/ground-snap gates as28.
Its absent Sub-J is distinct from Types47/53's authenticated empty Sub-J.
Live radial delivery
shares buffer/sound/health execution while retaining its distinct outer range,
impulse and checked-versus-unchecked entry. A completed death publication is
retained even if a later player-feedback boundary cannot continue.

Playing's static and ordinary-Hive radial calls now supply the actual player
hull, shared FX/RNG stream and scheduler through `PlayingRadialFrame`. The
coordinator samples each live-list successor after that target finishes.
Native Types6/8/66 enter the existing live `14AE0 -> 15040/14E10 -> 10C10`
owners with their own allocation and completed-task custody. First building
death therefore runs `19750` and republishes `25730/257C0`, including its
selector word, rather than merely changing health and the progressive clock.
Worker death similarly publishes its class14 graph. The replacement is
retained before a subsequent target can encounter it; no actor task is ticked
again during the radial pass. An actual player allocation elsewhere in the
manager does not reject these native callbacks.

The player radial route lends the actual hull, resources, FX, notifications and
campaign extra lives to the synchronous shared player helper. Full-radius15040
uses Checked; falloff14E10 uses Unchecked, and both retain255E0. The outer radial
owner commits its zero/nonzero impulse before this entry; an already dying
player consumes only its buffer. Complete packet provenance remains retained.
Falloff14E10 passes DAT_F7378 as14E90's fourth argument; that word remains
unauthenticated for future owner-dependent consumers, which are preflight
blocked on this null-modifier/null-hit local-death path. Retained actor routes
keep their existing exact filter/modifier/impulse planner for each target.
Selected Type9 velocity changes preserve completed mover custody, but this does
not broaden ordinary Type9 style-hit, reaction or callback-bearing radial death.
Unsupported families and pending native task prefixes remain explicit blocks.
A later block retains earlier targets and queued sounds/RNG; the report must
be consumed once, not retried as an entire blast. Corpus regressions exercise
native building/worker death alongside a real player hull, one constructor RNG
word, retained task identity, and a later missing-owner boundary.

`v2k-game::type47_common_dying_production` now adopts only those three
publication receipts and visits them in current manager live-list order.
Coarse `FUN_00412DA0` retains branch-local `+0x70` then `+0x6C` RNG, exact
wait/carry/125,000-us cap and unit-delta behavior, wrapping `mass 100 + B2`
with zero promotion, and both retail `+0xB2` clears. Mode 1 returns the class-12
transition tag before effect, mover, or strict timeout; the unsuppressed owner
publishes variant 1, clears all three task slots, and stages deferred removal.
The same callback then still executes entry-snapshot flags `0x28` through
`FUN_00413F70`, E100, the complete 2,000-ms E370 timer/random/lifecycle
profile, the authenticated empty Sub-J update, and the final inactive
master-motion reread. Bubble misses consume one post-scheduler word and hits
six; type 47 preserves `0x4000`, so the later sound gate consumes none. Exact
terminal states are `0x0143C805 -> 0x0151C805` and
`0x0143E805 -> 0x0151E805`. Live construction publishes `0x07068805` (both
`FUN_00411400` bits), and 10C10 yields `0x0707c805`. Bit `0x02000000` makes
`FUN_00412DA0` skip `+0x70/+0x6C` waits; class 12 still runs this E870 owner.
A former coarse-only gate left health-0 newants emitting class-5 smoke with no
Flip Over And Die. This captured owner retains E870; current native Type47
uses the shared detailed/coarse Class12 owner described in
[TYPE47_RUNTIME.md](TYPE47_RUNTIME.md#hits-death-and-abort). The entity remains
live until the manager's later sweep. No new broad death capture is needed.

`v2k-game::type17_primary_hit_live` now owns the bounded synchronous prefix. The
F590 handler offers a complete checked-delivery record to it before the
Base/Factory bridge. For one of the four exact authored allocations with an
already-published exact context, it journals every
issued receipt before executing the effect and retires it only after the owning
machine accepts the completion. The nonlethal test composes reselection,
reaction, 1,800 damage, accepted-hit sound 84, and the cached class-5 suffix.
The lethal test continues through the checked callback into class-12
publication, selector-4 player-kill notification, death sound 94, and class 5.
A later block preserves the committed prefix and cannot replay it as a fresh
transaction. Fresh construction now supplies exact state `0x07468805` plus the
selected/fallback behavior context and task graph, so the natural actors no
longer reject solely at the former birth-publication gate.

`v2k-game::common_dying_live` and the bounded production Type-17 scheduler now
close the deterministic detailed DCA0 boundary for the same four authored
fresh-Level-1 type-17/model-256 actors. The retained receipt still authenticates
health zero,
model slot 1, exact active style `0x004C7ED0`, empty auxiliary slots, a runnable
primary wrapper, and no pending deferred mark on every visit. State bit
`0x02000000` selects the outer owner: set enters detailed `FUN_0040DCA0`, while
clear enters coarse `FUN_0040E870`.

The class-12 value `0x2015` is not an effective flag word. Retail style
`0x004C7ED0` stores `+0x34 = 0` as its set mask and `+0x38 = 0x2015` as its
clear mask. Type 17's Section-12 initializer flags `+0xC0 = 0x39` are copied to
live context `+0xC8`, so both DCA0 and E870 derive the exact effective flags as
`(0x39 | 0) & ~0x2015 = 0x28`. Bit `0x2000` is therefore clear. E870 does call
`FUN_0040A800` in mode 1: the scheduler first increments elapsed time, the
Common-Dying callback returns its variant-1 tag without effect, mover, or
damping, and that tagged result takes precedence over timeout and requests the
variant-1 transition/removal before E870 evaluates its remaining suffix.

Retail and German-demo `FUN_0040DCA0` are behaviorally identical. Both begin at
`0x0040DCA0`; retail returns at `0x0040DF67` and demo at `0x0040DF66`, with the
one-byte delta confined to equivalent tile-index code generation. Both compute
effective flags, optionally prime the master-motion bit, dispatch
`FUN_0040A800` / mode-0 `FUN_0040A810`, visit the low-health emitter, optionally
run E640, rebuild the basis when flag `0x4000` is clear, run E100, optionally run
DF70, optionally dispatch flag-`0x800` class-6 tile contact, and always finish
with E370. Type 17's effective `0x28` skips master-bit priming, E640, DF70, and
tile contact. Its Section-12 `+0xA0 = 0` makes the low-health branch return
before cadence division, RNG, or sound, leaving basis rebuild -> E100 -> E370
as the reachable post-task suffix.

The matrix and its two consumers are closed by matched static code, not a new
capture. Retail `FUN_00413F70` and demo `FUN_00413EE0` build all nine signed-Q31
dwords at entity `+0x0C..+0x2C` from angle words `+0xA2/+0xA4/+0xA6`.
Retail/demo default draw `FUN_004138F0` / `FUN_00413860` copies all nine dwords
into the detailed submission, while the particle sweep
`FUN_0043F980` / `FUN_0043F230` reaches the narrow model test
`FUN_0043FCE0` / `FUN_0043F590`, which copies the same complete matrix for
collision. The port therefore retains the post-DCA0/E870 matrix in entity
custody and transposes its three world-space columns into the row-major draw and
particle/model-collision convention.

The callback step is exact rather than an assumed raw-frame delta. Production
plans retained `+0x70/+0x6C/+0x68/+0xB6` state on a clone. Detailed state bit
`0x02000000` bypasses both randomized waits and therefore consumes no scheduler
RNG. A coarse visit uses the exact normal prefix: its conditional threshold
draws use the process-shared low-16 stream in `+0x70`-before-`+0x6C` order, and
a wait returns before the callback. A wait commits the planned gate/accumulator state, clears
`+0xB2`, leaves `+0xB0` and task age unchanged, and retains the receipt. A
continuation commits the wrapping `+0x68` advance and passes the carry-adjusted
step capped at 125,000 microseconds, retaining any remainder in `+0x6C`.

The scheduler visits receipts in manager live-list order and draws through the
same `WorldFx` owner used by later world effects. This proves the exact
branch-local draw count and `+0x70`-before-`+0x6C` order; it does not claim a
globally identical retail RNG history while other entity scheduler families
remain detached. Production conservatively preflights every supported callback
and suffix before touching that stream. An unsupported suffix therefore blocks
without learning whether retail would have waited on that particular frame.

Before every admitted normal callback, retail writes entity `+0xB0` as the
wrapping 16-bit sum of the Type-17 authored base mass 100 and entity `+0xB2`,
promoting a zero result to one. DCA0 and E870 use that callback-time mass in
E100. A scheduler wait clears `+0xB2` without rewriting `+0xB0`; a callback
clears `+0xB2` after return. The authenticated lethal F590 publication occurs
after the same entity's normal scheduler visit in the later particle pass, and
all accepted Type-17 samples retain `+0xB2 = 0` / mass 100, so publication can
promote that field to exact zero without claiming constructor provenance.

The production adapter admits the normal local subject: remote bit
`0x80000000` clear and callback-enable bit `0x00020000` set. Relation bit
`0x1000` must be known, but no longer has to be clear. Its pre-callback owner
reconciliation is live: an authoritative local-parent Sub-J row retains the
relation, an absent row synchronously applies Type 17's null-hook relation
release, and a remote parent copies its three position words and sets `0x20`.
A null or unresolved parent now enters the separately preflighted
`FUN_004180F0` Type-93 transaction. Exact retail/demo code and authenticated
Type-93 metadata close its terrain target, one-word singleton selection,
tail-appended allocation, release/reattach state mapping, true one-row Sub-J
runtime, borrowed active model, and same-pass first callback. The dying Type-17
child retains `0x4000`, so the proxy's following `FUN_00418640` removes that row
without invoking either callback. On the child's next admitted visit,
`FUN_00412DA0 -> FUN_00418A00` finds the existing parent with no membership and
releases through its own null-hook `FUN_00416750`. That release sets `0x800`
but does not restore master-motion bit `0x40000`: only `FUN_00409030` owns that
write. The empty proxy subsequently expires without a child release event.
The selector word is consumed only after the scheduler continues; a coarse
wait leaves both RNG and the stale relation untouched. Type 17's immutable
Sub-J descriptor is authenticated as count 1, reserved byte zero, policy word
`1`, and local offset `[0, 10, 110]`; its entity-owned ordered runtime has
capacity one and outer policy `0`.

For an admitted clear-detail visit, mode 1 commits wrapper age and returns
singleton `0x004BE170` with tag `0x9C01` without effect lookup, common-mover
dispatch, or X/Z damping. When the relation prelude cleared `0x1000`, the
allowed transition publishes style
`0x004C7F18`, clears slots 0, 1, and 2, and stages `FUN_00410B70`. E870 then
rebuilds and stores all nine Q31 body-basis dwords, publishes state bit `0x4`,
applies `FUN_0040E100`
gravity followed by Level-1 no-wind drag strength 3 at the computed callback
mass, and runs E370's timer/random-effects/lifecycle path. The transition has
cleared master-motion bit `0x00040000`, so final
position integration is skipped before the same-tick manager sweep.
If the local parent still contains the subject (or the parent is remote),
`0x1000` survives: the mode-1 request is suppressed after unwind, the receipt
and task remain live, and E870, the attachment tail, and outer master motion
still execute.

The solo direct-attachment tail is now live. Attached-child mass consumed by
the callback's common mover is summed from the original ordered rows before the
callback; the later updater pairs surviving rows with authored slots by ordinal,
stably compacts missing, exact-zero, and `0x4000` children, and reuses a vacated
descriptor ordinal for the shifted row.
A valid child's velocity is zeroed and its position starts at the owner's
post-callback, pre-master-motion position. Child bit `0x800` adds the authored
offset through the owner's rebuilt Q31 basis with wrapping signed-word writes.
`DAT_004F741C` gates backlink repair and wrong-backlink rejection, so this
authenticated single-player adapter deliberately ignores `attached_to` during
the direct tail. The tail invokes neither of the two row callbacks. Those
callbacks belong to LIFO pop/destruction, while multiplayer repair/network
publication remains a separate owner/transport scope. No capture is justified
for the implemented relation, direct-update, or missing-parent Type-93
contract.
The authenticated Type-17 E370 owner is otherwise complete
through expiry. Each deep visit adds `floor(elapsed_us / 1000)` to timer `+0x48`
with wrapping arithmetic. At exactly 2,000 ms the synchronous
`FUN_00416750 -> FUN_00410C10` pair runs before the zero-percent random window;
a normal overshoot runs the same pair, computes the original unsigned wrapped
percentage, and returns before RNG. A surviving deep actor repeats the pair on
later overshoot visits because retail does not reset the timer.

Type 17's release transaction writes entity `+0xC8 = 0x39`, clears relation
`+0x80` to the zero global sentinel, and maps the state word as
`(before & 0xDFFE_EFFF) | 0x00000800`. The type-vtable `+0x48` trampoline is
invoked, but both admitted Common-Dying styles have a null nested `+0x0C`
release callback. Preserved model bit `0x4000` then makes `FUN_00410C10` return
one without health, sound, auxiliary-object, death-callback, or deletion side
effects. This is a synchronous release, not the later deferred Common-Dying
sweep.

Before expiry, and again at the exact zero-percent boundary, a failed bubble
gate consumes exactly one shared RNG word. A hit consumes six total: the gate,
positive seven-bit X/Y/Z position jitters, then unsigned nine-bit X/Z velocity
arguments. The request position adds the post-`FUN_00413F70` Q31 forward column
times active model extent 315, shifted by 31, and the three jitters to the live
signed-8.8 position with wrapping signed-16 writes. It queues class 42 with
velocity `[vx, 0, vz]` and the live Type-17 owner identity. Authenticated model
slot 1 supplies state bit `0x4000`, so the later sound branch is suppressed
without consuming its gate word.

Coarse scheduler draws occur before these E370 words on the same shared
`WorldFx` stream; detailed visits have no scheduler draw and proceed directly
to E370. Ordered class-42 requests are materialized before the physical-
particle traversal. Matched retail/demo C closes this control flow; the demo's
different sound literal is unreachable for the authenticated Type-17 owner.
Neither the random branch nor the lifecycle needs a targeted capture. Detailed
DCA0 and terminal E870 share the bounded E100/E370 planner, so an admitted
detailed frame commits exact `+0x48`, lifecycle storage, and state evidence
instead of invalidating it. The relation reconciliation and solo direct-
attachment continuation are live; no scheduler-arithmetic, classifier, DCA0,
or bounded E870 capture is required. For a detailed visit,
the adapter also preflights the exact A/B/C/D/H/J topology, A/B/C/H
descriptors, attached-mass sum, live Sub-A, disabled eight-record Sub-H,
terrain, and task lease. A failure is therefore non-mutating; a callback error
after entry retains its consumed elapsed prefix and cannot be replayed as a
fresh frame.

At the entity-manager boundary before physical particles, authenticated
receipts are visited in current retail live-list order. Clear-detail receipts
use the bounded E870 preflight above; detailed receipts use the DCA0 prefix. For
a detailed-admitted receipt, the direct `FUN_0041EC70` call uses gain `0x100`
and parameter 4 equal to one:
pitch keeps the ordinary slope
expression, roll uses the reversed expression, and an upright body forces a
nonzero roll drive to at least `0x28`. Its corrected pitch/roll do not rebuild
the basis before `FUN_00401430`; the null-target mover receives the pre-effect
basis and runs disabled Sub-H -> Sub-C -> Sub-A -> Sub-B. Type 17's authored
signed Sub-C selector is zero, so both the attitude probes and Sub-C use terrain
rather than waves. Disabled Sub-H clears only transient record bits, emits no
cues, and consumes no shared RNG. The task callback applies wrapping X/Z
damping to the mover velocity with Y untouched, then the live seam atomically
commits that final damped velocity together with position, pitch/roll, and
Sub-H state before wrapper unwind.

DCA0 continues through its post-task suffix even after the wrapper requests a
terminal transition. The bounded owner preflights E370 before mutation, then
commits `FUN_00413F70`'s complete post-attitude Q31 matrix followed by persistent
state bit `0x4`, E100's final velocity, and the exact E370 timer/lifecycle
transaction. The enclosing owner then runs the
entry-gated direct-attachment tail, clears entity `+0xB2`, and rereads master
state in retail order. Every
statically recovered supported E370 return is zero, so the callback-result gate
passes. Continuing detailed frames retain state bit `0x00040000`, do not match
the captured zero-approach mask `0x00880000`, and, when the final velocity is
nonzero, integrate it with the exact callback step and publish state bit
`0x20`. An allowed terminal transition clears `0x00040000`, so that frame
preserves the completed DCA0 suffix but skips master-position integration.

Exactly 9,000 admitted detailed milliseconds survives. A detailed frame
reaching 9,001 ms still invokes the effect and mover and commits velocity before
the post-unwind owner transition. Static `FUN_00416410` then inspects state bit
`0x1000`, where a set bit would suppress that transition. The same entry-
snapshot bit already selects `FUN_00412DA0`'s relation/owner branch before the
callback. The bounded adapter now plans and commits that prelude: a missing
local-parent membership releases the relation and admits the due transition,
while an authoritative membership or remote parent retains `0x1000` and the
post-unwind gate returns `TransitionSuppressed`. The latter still commits DCA0,
the direct-attachment tail, and outer master motion while retaining the task
receipt. A missing parent instead materialises and attaches the authoritative
Type-93 proxy, so relation bit `0x1000` remains set and suppresses that terminal
transition on this post-unwind check. The later proxy visit retires the dying
child row; the next child's prelude releases the now-absent membership before
its transition check. An allowed transition publishes variant
1/style `0x004C7F18`, applies its reversed
`+0x38 = 0x2011` policy (setting `0x00010000`), clears task slots 0, 1, and 2 in
order, then applies `FUN_00410B70`'s
`(state & 0xFFF9FFFF) | 0x00100000` deferred mark. The manager records the same
accepted identity/count. `FUN_00413500` then runs
`FUN_00414990 -> FUN_004149F0` later in that same manager tick, before the
physical-particle snapshot. The terminal mask clears motion-enable bit
`0x00040000`, so the already-completed task and DCA0 suffix survive but the
final master-position integration is skipped on that frame. A lethal F590
publication happens later in the particle traversal and therefore remains at
age zero until the next scheduler invocation. This normal deferred sweep does not
reinterpret an earlier mark from another caller as timeout completion.

The matched focused type-17 captures
`20260731-181510-entity-death-effects-type17.jsonl`,
`20260731-181555-entity-death-effects-type17.jsonl`, and
`20260731-181640-entity-death-effects-type17.jsonl` dynamically validate this
shell and separate three stages which must not be collapsed into one death
timer:

- the clean victim `0x04AA0001` takes three accepted 1,800-point hits,
  `5000 -> 3200 -> 1400 -> 0`. The lethal sample changes state
  `0x07468825 -> 0x07C7C825`, selects model slot 1 without changing global
  model 256, installs style `0x004C7ED0` and task `0x00404220`, clears the two
  auxiliary task slots, and starts with raw signed-8.8 velocity
  `[-108, +500, -278]`. The newly set `0x00814000` bits are the existing
  contact-response, terrain/water-collision, and low model-selector bits, not
  a new opaque death flag;
- before each damage commit the clean victim leaves Follow Beacons variant 1
  (`0x004C7B70`) through the impact callback, briefly enters Run Away initial
  (`0x004C7618`, task `0x00402BA0`), then settles roughly 30 ms later into Run
  Away variant 1 (`0x004C7660`, task `0x00403F40`). This dynamically proves
  the pre-damage `FUN_0040C690` behavior reselection is mandatory; a generic
  null-callback damage adapter cannot reproduce type 17 by merely subtracting
  health;
- accepted nonlethal and lethal contacts each emit a class-5/flags-`0x3A` hit
  presentation. The class-7 surface stream does not begin at the lethal
  sample. It begins about 865 ms later when the tumbling body reaches the
  surface, then attempts roughly once per active callback. This belongs to the
  class-12 terrain-effect call and must remain surface-gated rather than a
  constructor-time death burst;
- the two nonlethal accepted-hit voices resolve to authored global sound 84
  (alias of direct PCM 23), while the lethal sample resolves to authored global
  sound 94 (alias of direct PCM 30). Their captured playback rates fall inside
  only those aliases' authored multiplier/variance windows; the competing
  aliases sharing each PCM cannot produce the observations. Section-12 type-17
  selectors independently store accepted-hit 84, death 94, and no generic-hit
  sound. `v2k-formats::real_ovl` pins that static/runtime join;
- the continuation capture starts with the same victim at task age 7,001 ms.
  It later clears inner/full-detail bit `0x04000000`, then outer detail bit
  `0x02000000`; fully-above-surface bit `0x00400000` remains set. The victim
  disappears near task age 7,835 ms without sampled style `0x004C7F18` or
  deferred-destroy bit, and before strict `>9000` completion. Static dispatch
  now explains that boundary: the next accepted E870 visit increments elapsed,
  invokes `FUN_0040A800` in mode 1, and consumes the task's variant-1 tag before
  the timeout test. The same-tick transition, deferred mark, and manager sweep
  can all fall between passive samples. Its already-born class-7 particles
  remain alive for roughly another 595 ms, proving effect lifetime is
  independent of source linkage.

Five earlier route-trace actors follow the approximately 9,000-ms completion
path, while handles `0x04A90001` and `0x04AE0001` disappear after approximately
3,100/4,100 ms and the focused victim above disappears after approximately
7,860 ms. Static C proves the observed `0x04000000` and `0x02000000` view-detail
writers are classifiers rather than direct unlink predicates, but clearing the latter
selects E870 and its mode-1 Common-Dying transition. Every removal still passes
through `FUN_00410B70`'s transient `0x00100000` mark followed by
`FUN_00414990 -> FUN_004149F0`; the passive sampler can miss that complete
same-tick transition and sweep. The focused 7.86-second removal is therefore no
longer capture-gated, and the prepared 6.5-second exact-victim caller trace is
obsolete. The two other short-lived route actors lack equivalent selector
history, so their earlier mark ownership remains a separate question.
The authored task timeout is not an unconditional entity lifetime and must not
be used as one. The focused runtime evidence also does
not bypass the outer transaction: type-17 pre-impact callbacks may reselect a
behavior or clean up Capture People state before impact reaction and checked
damage.

Rust retains the broader standard null-death ordering as an authenticated
detached composition. `v2k-game::actor_standard_death` re-plans the original
lethal-hit request internally, rejects Capture's nested death path and
nonlethal/unknown styles before mutation, then applies
`plan_common_dying_setup` unchanged. Native production separately uses
[Type17 Capture cleanup](TYPE17_CAPTURE.md#delivery-release-and-nested-cleanup):
D040 releases its actual child before C690, and DB80 rereads the current outer
continuation. Direct death can publish C620 twice; primary-hit cleanup first
reselects while alive. Radial, static collision, E370 expiry and Main Base abort
share that child/task/notification context. The bounded live publisher above owns
class-12 construction/publication for the exact fresh-Level-1 type-17 null-hook
route. Primary-impact scan placement and synchronous composition of that inline
event through impact reaction, checked damage, and the publisher are complete
for the exact fresh published context; no new generic type-17 null-hook death
capture is required. Production birth-time shared-RNG selection and
selected/fallback task publication are complete. The bounded detailed task
prefix, full Q31 basis publication/E100/E370 timer, random-effects, and Type-17
lifecycle suffix, outer master motion, and terminal sweep are production-bound.
E870's
unsuppressed mode-1 transition followed by E100 and the complete authenticated
E370 branch remains a bounded callback path for an exact clear-detail snapshot.
Detailed DCA0 now preserves exact `+0x48` and lifecycle-storage evidence. The
presentation phase now publishes the exact `FUN_00411400` three-tier result for
every retained Type-17 Common-Dying
receipt, so its broader-detail bit naturally selects the next scheduler visit.
The bounded coarse common-scheduler prefix is production-wired through shared
`WorldFx`, with exact conditional branch-local draw order, waits, carry, and
callback mass. Local/remote relation reconciliation and the solo ordered
direct-attachment update are live in their exact callback ordering. The exact-
matched missing-parent Type-93 materialiser, callback-free dying-row retirement,
and next-child-visit relation release are live too; the resulting empty proxy
expires without `409030`. Multiplayer backlink/network repair remains outside
this adapter.
Authenticated Common-Dying Type-17 presentation and particle/model collision
now consume the retained post-DCA0/E870 full Q31 basis. The latter projection
refreshes its live membership, center, model, radius, and matrix after damaging
F590 handlers. Unauthenticated or broader non-Type-17 actors retain the
yaw-derived fallback; class-32 owner velocity remains separate. General callback binding
and the two short-lived route actors without selector history keep the scope
`PARTIAL` before the trace-defined gate may expose the hive.
