# Intro2 Type9 peasants

Intro2 constructs thirteen Type9 `man2` actors through the same retail
`004104B0 -> 0040D4A0` component and behavior path as ordinary Level1 peasants.
Their scene, authored parameters, candidate prefix, and Sub-D allocation state
are distinct. The native publisher is
[`intro2_type9.rs`](../../crates/v2k-game/src/intro2_type9.rs); shared behavior
and mover authority remains in [ACTOR_RUNTIME.md](ACTOR_RUNTIME.md) and
[ACTOR_TASK_PROGRAMS.md](ACTOR_TASK_PROGRAMS.md).

Ordinary authored worlds use the separate
[shared native constructor and allocation policy](ACTOR_RUNTIME.md#current-type-9-task-ownership).
That route retains actual process Sub-D allocations, supports all four root
choices including Player Nearby, and admits later-level motion, cargo and
death through manager-bound receipts. The exact Intro2 spawn/seed and first-
query conclusions below remain scoped to this cohort; they are not generic
constructor defaults.

Capture delivery retains the actual completed release task graph for later
contact until the deferred body sweep. This contact-only receipt is shared
with authenticated ordinary Type9 releases; it never lends another actor
visit. [TYPE17_CAPTURE.md](TYPE17_CAPTURE.md#released-type9-contact-after-delivery)
owns443D10/10B70 ordering, allocation/queue/task admission and regression evidence.

## Authored data and constructor

The normal-tier canonical Type9 Section-12 row is identical in Intro2 and
Level1: models558 in all four slots, mass10, capability0x1804, health1500,
accepted-hit sound95, death sound35, and no constructor attachment or generic
hit sound. Sub-A is `[1500,-3000,250]`, Sub-B `[10000,1000]`, Sub-D has divisor20,
zero yaw/roll coupling and pitch steering, probes128/64, classifier flags0x17,
and Sub-I is `[72,0,72,binding1,stride4]`. Initializer flags are0x2F and the
common-axis descriptor is `[0xF00,0x84]`. The alternate rule selects class14.

All thirteen Section-13 records have parameter1, zero rotation, no per-spawn
model override, and no authored animation/config extension. The exact birth
cohort and successful `004203D0` process seeds are:

| Spawn | X/Z cells | Sub-D seed |
| --- | --- | --- |
| 2 | 140,126 | 02 |
| 3 | 142,126 | 03 |
| 9 | 142,128 | 09 |
| 11 | 141,124 | 0B |
| 16 | 180,17 | 0F |
| 17 | 174,37 | 10 |
| 18 | 181,19 | 11 |
| 19 | 185,22 | 12 |
| 49 | 188,22 | 20 |
| 50 | 191,21 | 21 |
| 58 | 191,125 | 24 |
| 59 | 159,128 | 25 |
| 60 | 162,127 | 26 |

`00409A80` constructs Sub-A through `00420450` before the weighted behavior
selector. That component constructor consumes one shared RNG word and writes
`base + ((rngLow16 >> 8) * base) / 0xA00`, direction1, and scale100. Sub-D
initialization consumes no random word. `0040D4A0` applies Type9's terrain
snap and retains the resulting XYZ as the immutable task anchor at entity90.
The final `00413F70` wrapper writes the body basis and state bit4 after the
selected task initializer.

`00425680` evaluates the already-linked constructor prefix in retail order:
Baddie Nearby (mask8) ×10 → class10; Player Nearby (mask1) ×3 → class45;
Base Nearby (mask0x20) ×200 → class54; Always ×1 → class6. It then consumes one
unconditional selector word. Each nearby predicate uses the authored strict
axis limit0xF00 and shared `00422C10` candidate eligibility. Later records must
not influence an earlier birth. Intro2 has no persistent player allocation;
the native publisher rejects a nonzero Player Nearby result before consuming
RNG rather than omitting class45's resource-text and sound effects.

The selected class6, class10, and class54 initializers reuse their existing
ordinary Type9 task transactions. Class6 clears Tertiary, then Secondary, and
publishes one Wander Primary with one successful constructor word. Class10
copies the common-axis filter, clears Tertiary, and publishes acquiring
Secondary then Primary, with two successful constructor words. Class54 uses
its separate compatible-job scan and Secondary/Tertiary/Primary transaction,
including the one-word generic suffix followed by its fixed speed override.
Missing job-capacity evidence remains an explicit constructor rejection.

## Component and allocation custody

The native allocation receipt retains entity identity, authored spawn, final
anchor, and its own process seed. It survives later context/style changes and
allows native hit/death code to distinguish this allocation from generic
preview entities. Steering and classifier state remain in the shared
`OrdinaryType9PendingInitialSelection` data shape, transferred into selected
component custody after the complete initial task graph is installed.

Fresh native `+0xB2` is explicitly initialized to zero once, matching the
existing policy for allocator residue documented under
[fresh Type9 callback mass](ACTOR_TASK_PROGRAMS.md#fresh-type-9-callback-mass).
This does not authorize the Level1 first-scheduler state overwrite: Intro2
retains its own constructor and presentation-written state bits.

The native birth additionally retains the selected context and exact task
leases until the shared scheduler claims them once. The live class6/10/54
owners already implement `00412DA0`, detailed/coarse D/I/A/B callbacks,
task-root changes, F70, E100, DF70, E370 and master motion. They consume this
native graph through `OrdinaryType9CurrentTaskAuthority`; no Level1 initial
production sidecar is manufactured. Claimed Type9 animation is excluded from
the neutral actor-animation pass so detailed I runs once and coarse I stays
suppressed. Common collision scans remain a separate world phase.

The shared outer tail receives current mode-zero drag strength and the actual
callback B0 mass. `0044EC60` divides the low32 bits of elapsed×strength by
`B0 << 3`; B0 is the wrapping nonzero result of authored10 plus current B2.
Fixing the tail to mass10 would discard that live scheduler contribution.
The outer transaction requires every state bit except the two surface bits
`0x00600000`: native construction leaves those for `129B0`, and E100/DF70/E370
and master motion do not consume them. Committing the outer suffix preserves
both their values and their known/unknown mask instead of inventing a surface
classification. The native scheduler regression retains mass19 and unresolved
surface bits in both detailed and coarse visits.

## Shared carried surface expiry

Native ordinary Type9 carrying custody now completes underwater expiry through
the actual `162B0 -> 16750/CE90 -> 10C10` chain. Its temporary living selector
and initializer emit their real RNG and presentation before class14 replaces
the graph. The scheduler retains class14 for its next visit, and the late
parent Sub-J walk removes the released dying child. Exact/overshoot controls
exercise real Wander and Attract publication, then follow native Level1 beam
drop through Type93 row cleanup and deferred body removal. The full source
ordering and coverage limits belong to
[ordinary Type9 cargo callbacks](ACTOR_RUNTIME.md#ordinary-type-9-cargo-callbacks).
NoCD03 records only nonterminal carrying; this closure comes from source and
controlled native tests. The separate nonzero-wind boundary remains explicit.

## Ordinary static task contact

Ordinary authored-world peasants now retain the complete admitted `A8B0`
task-callback prefix at late static contact, before `D920/11760` separation and
static-before-actor damage. Source constructor writes at402E7B,402B6B,4032FB,
4036B3 and403E83 install `02CA0` for Wander, shared retarget, Attract local
wander, job/target route and fleeing. Candidate/acquisition construction through
405F80 and Cue preserve405FF0's null `+0x20` word at406019. Current slots are
read in Primary/Secondary/Tertiary order. The admitted completed Type9 graphs
have one retargeting Primary and, where present, null Candidate/Cue/acquisition
companions; they do not invent simultaneous retarget and route tasks.

The authenticated Type9 A/B/D/I topology selects02CA0's Sub-I branch: wrapping
heading `+0x2000`, then two shared random words for its private X/Z target around
the current position. It does not advance animation, change the task direction
or clock, rewrite Sub-D, or rebuild physical body basis. The existing completed
scheduler-authority transfer now precedes those writes as well as physical
response; incomplete or unsupported task custody stays an explicit block, and
a later failure retains the already committed prefix. This fixes the omitted
turn/retarget when Level1 peasants brush the spider-pen fences while preserving
the separately established separation and damage policy.

The native fence controls cover the callback/null matrix and ordered RNG,
wrapped private targets, retained basis/clocks and the following scheduler
visit. This ordinary-world owner does not broaden Intro2 contact dispatch or
claim complete matched retail containment.

All six living-style fence controls pass. The sustained authored Level1 control
records82 contacts, including35 soft separations, with all six native peasant
owners retained. Fixed and mixed-cadence production-main runs each complete
1024 first-world frames after the uninterrupted Intro2/Klaus handoff without
runtime blocks or dropped owners. Focused tests, the full V2000 repository gate
and the release build pass; matched retail containment remains separate.

### Ordinary late actor pairs

`FUN_00411AD0` retains subject entry flags, model and `+70` before the static
scan. Its deepest static result enters12CF0 before the intrusive next-node
walk enters12530. Playing now uses
[`resolve_ordinary_type9_late_contact`](../../crates/v2k-game/src/ordinary_type9_static_contact.rs)
for this complete peasant phase. Static turn/retarget and separation therefore
precede dynamic contact. A lethal fence hit retains the living entry admission
for that same suffix; geometry, relations, behavior and task slots remain live.
A blocked static suffix stops that subject's pair walk while later actors
proceed. Missing Type9 metadata is reported explicitly.

The shared [pair owner](../../crates/v2k-game/src/native_actor_capture/pair.rs)
also admits native Type9 peers through either ordinary allocation receipts or
the existing immutable Intro2 receipt. Level1 authors six peasants, including
adjacent records512 raw units apart; their native movement can overlap without
a spider participating. Receipt presence selects the lane, and both bodies
must lend authenticated completed custody before any callback or motion write.
All admitted living6/10/45/54 styles have null behavior `+38`; A900 still visits
both current Primary/Secondary/Tertiary chains in order. Their real02DA0/null
task matrix precedes12760 and the ordered14D30/15040 damage suffix. Actual
carried graphs are excluded by1000 in either seat. Class14's fresh entry is
ineligible after8000 clears; a same-pass retained entry uses its newly current
null behavior and current task callbacks. Lethal peer damage adopts the real
Class14 publication synchronously.

The [phase controls](../../crates/v2k-game/src/ordinary_type9_static_contact/late_tests.rs)
use native Level1 and Intro2 bodies, wrapped oriented geometry and authored pen fences.
They cover both callback/component directions, successor-only non-replay,
unchanged private clocks/basis, foreign or parked custody, carried exclusion,
real lethal peer damage and the next Class14 visit. A separate production seam
comparison includes a lethal static hit before the pair suffix. The six living
fence controls now enter this production phase. These controlled source tests
do not establish a matched retail scene trajectory or visual containment.

## Primary, static-route and infected hits

[`ordinary_type9_impact.rs`](../../crates/v2k-game/src/ordinary_type9_impact.rs)
owns native Type9 particle delivery in Intro2 and ordinary authored worlds
before the general fallback. It authenticates the
allocation and a completed scheduler visit, retaining the full six-word packet.
`00410EB0` stamps `+0x34` before `0040DAC0` dispatches current style `+0x28`;
then `00411030` applies impulse and `00415040` delivers checked damage. A
nonzero checked return on a surviving target requests accepted-hit sound95.
Type9 capability0x1804 excludes the separate capability8 class5 emission.
`00411180`, used by `442950` source classes52/68/85, instead stamps+34, requests
the living target's authored+80 cue before DAC0, applies eightfold25590 impulse,
then delivers15040 without inspecting its return. It has no primary capability8
suffix and suppression does not cancel its damage. The same native Type9
current-graph/death owner serves Intro2 and ordinary authored worlds. The bat's
class68 packet `[2,6]/[2000,2000]` filters to5600, killing a canonical1500-health
peasant through the existing Class14 publication. Class52/85 packets reach the
same owner in overlays13/14/15 regression cases. The source ordering, captured
retail peasant death and remaining attached-effect boundaries are documented in
[Bat projectiles and peasant damage](INTRO2_COMBAT_PROJECTILES.md#bat-projectiles-and-peasant-damage).
`00411250` instead sets0x2000 and requests the authored `+0x82` cue before
`0040DA00` dispatches style `+0x20`; it preserves `+0x34` and has no accepted-hit
sound suffix. All three particle paths supply a packet and therefore run11030,
including its three eligible RNG draws when the impact sum is zero. Exact
arithmetic and wrapper ordering remain in
[ENTITY_DAMAGE_AND_DEATH.md](ENTITY_DAMAGE_AND_DEATH.md).

The executable's style words require distinct dispatch:

| Current style | Primary `+0x28` | Infected `+0x20` |
| --- | --- | --- |
| Wander0 `4C79C0`, Go To Job0 `4C8788` | C690 | C690 |
| Wander1 `4C7A08`, Go To Job1 `4C87D0` | null | null |
| Run Away0/1 `4C7618`/`4C7660` | null | C690 |
| Run Away2 (carrying) `4C76A8` | null | null |
| Attract Attention0 `4C86B0` | null | null |
| Attract Attention1 `4C86F8` | C690 | C690 |
| Attract Attention2 (carrying) `4C8740` | null | null |
| Exploding Person0/1 `4C70C0`/`4C7108` | null | null |

The adapter reuses the shared impact reaction, checked damage, root selector,
and exact class6/10/45/54 constructor transactions. Each root application receives
the actual predecessor family. Living C690 reentry consumes one selector word
before constructor words. Its publication becomes current-task authority
synchronously, so a second hit can replace the newly published graph before
the next actor visit. Completed scheduler observations are consumed without
replaying movement, resetting task clocks, or applying cargo-release writes;
the next transaction identity is retained.

Class45's initial Candidate/Cue owners and acquired TargetRoute owner transfer
linearly through a completed external-hit boundary. A null style hook does not
justify skipping this transfer: infected model selection and impact reaction
can change state and velocity before the next task visit. The transferred
authority retains the actual graph without a stale constructor/outer-tail pose.
Hit-triggered class45 text uses the live notification tick and its sound is
queued once before checked damage; neither waits for a later world-load drain.
Carrying and class14 custody authorize only their null hooks, not root graph
replacement. An incomplete or foreign owner rejects the hit before its prefix.

Root-application gates admit live slots0 and2 only with all four canonical
models558. They preserve0x2000 and unrelated known or unresolved state bits.
The original0x4000 AC60 alternate selects class14 without RNG. At a completed
native hit boundary, standard death has already synchronously installed
class14, whose two hit slots are null. A dying selector paired with an old
living selected graph is therefore rejected as inconsistent custody; it is
not repaired by replaying standard death or copying a constructor receipt.
The shared live selector supports class45 when a real eligible player is present.
Intro2's authored construction still has its own player-free candidate prefix;
this does not replace that birth evidence with ordinary-world data.

The shared `publish_ordinary_type9_standard_death` supplies the class14 task,
death sound35, and session-zero C6 message. The scheduler adopts its receipt
before any later hit or actor visit, including a checked-damage error carrying
an already committed death publication. Primary accepted sound95 observes
the final dying bit and is absent after death; infected entry has no accepted
sound suffix. Initializer preflight errors preserve the wrapper's committed
prefix and the old graph. Native host allocation is infallible; these errors
do not follow partially committed constructor mutations.

Radial writers use the same completed-authority transfer before accepted
mutations, preserving the selected graph, task clocks, and next transaction
identity. During the actor walk, the transfer uses the actual pending or
already-visited owner storage and keeps its position relative to the cursor.
An unfinished transaction rejects the hit without discarding its old owner.
A later damage block retains the new authority and any committed impulse.
Shared static/meteor delivery adopts death receipts before the next blast.

The prior static-program adapter could change velocity while a completed
outer-tail observation retained the old value, causing the next actor visit
to drop with `OuterTailStateMismatch`. A deterministic port walkthrough
reproduced this on spawn2: at tick428 cell[138,127]'s blast changed velocity
`[168,0,361]` to `[-150,23,-961]` and health1500 to8 while retaining the Run Away
variant1 graph and age560ms. Tick430 discarded that owner. This is a port
custody failure, not evidence of retail task reselection. Walkthroughs must
report dropped owners as well as blocked callbacks.

Regression coverage in
[`shared_actor_impact/type9_tests.rs`](../../crates/v2k-game/src/shared_actor_impact/type9_tests.rs)
uses all36 later-world peasants from14/15/25/39 through the production loader
and shared hit router. It covers repeated primary/infected delivery, real
Candidate/Cue and acquired target graphs, carrying, reaction suppression and
buffering, and lethal class14 retirement. Scheduler tests retain task clocks
and transaction identities across completed transfers and reject an unfinished
target visit. These controls do not replace matched retail scene acceptance.
The production OpenGL walkthrough also follows a real Level14 peasant after
256 ordinary world visits: canonical primary damage publishes class14 and its
existing allocation retires47 ticks later at20ms per tick. Intro2, frontend,
first-world handoff and later-world factory draws remain regression controls.

## First-query ownership

The accepted constructor transcripts
`20260730-034232-sub-d-constructor-provenance.txt` and
`20260730-035135-sub-d-constructor-provenance.txt` establish the seeds above and
zero classifier rows. They also demonstrate why zero origins cannot be a
native constant: spawn59/seed25 has origin00,00 in the former run and40,2E in
the latter. Other actors' observations cannot supply this allocation's bytes.

The accepted read-only `V200001.run` walks on 2026-09-08 independently join all
thirteen births above to their first classifier use. The early cohort covers
spawns2/3/9/11/16–19; the late cohort covers49/50/58–60. Every allocation has a
balanced successful constructor/register/return, matching frame owner,
descriptor, allocation and thread, then `PRE -> full_reset -> WINDOW -> POST`.
The direct chain is `00401602 -> 0041F660 -> 0041F7A8 -> 0041FCB0`.

Each first query occurs at retail tick6 with elapsed125000 microseconds and
raw X/Z equal to its authored X/Z plus `[0x7F,0x40]`. In this recording all
origins are00,00 and all constructor rows are zero. Full-reset writes the
queried cell as origin, leaves delta00,00, returns class0, and fills row0 with1.
The observed frame counter is the constructor seed plus1: `0041F660` increments
`+0x3A` and clears a row every eighth visit before making any query. This is
frame maintenance, not a different allocator sequence.

The native publisher binds an Intro2-only pending-first-query receipt to each
exact spawn/seed pair. It retains zero constructor rows and an unresolved
origin until that first query, then performs the witnessed full-reset once;
later queries use normal cache-window and stagger behavior. The receipt moves
with shared component custody through task changes and class14 death. It
does not copy an observed heap origin, grant a Level1 startup overwrite, or
authorize another actor's allocation or callback cadence. The prior
Type13/26/47 replay excluded Type9 and is not this policy's authority.

Raw transcripts, exact replay commands and join-validation records are retained
under `captures/local/ttd/20260908-intro2-type9-first-query/`. Accepted artifact
names and capture policy remain owned by the
capture ledger.
