# V2000 Actor Runtime

This document owns the two-layer behavior dispatch, actor task programs, common
mover, scheduler semantics, and process-shared RNG contracts formerly embedded
inside the virus section of [GAME_MECHANICS.md](GAME_MECHANICS.md). Virus-grid
mechanics remain there; current priorities and capture status remain in their
objective and runtime-RE ledgers.

Production family construction and lifecycle are tracked in
[cross-level gameplay runtime](CROSS_LEVEL_GAMEPLAY_RUNTIME.md), including
the [shared Type47 gunner](TYPE47_RUNTIME.md) and
[native terrain gate helpers](GATE_HELPERS.md). Shared behavior code and
per-instance state are distinct: every birth retains its own allocation,
components, task clocks and current behavior context.

## Dispatch architecture

1. **Per-entity-type vtable** at `g_resource_table[type*4] + 0x7c` — see the entity-vtable section in [GAME_MECHANICS.md](GAME_MECHANICS.md). It handles generic lifecycle, detailed/coarse update, presentation, death, and terrain contact. `g_resource_table = DAT_004fe650`.
2. **Per-instance behavior block** at `entity + 0xB8` — distinct from the type-vtable above. Holds per-instance behavior callbacks. Behavior context parameter at `entity + 0xC0`.

**Behavior block slot layout** (callbacks dispatched via the trampolines below):

| Block offset | Lifecycle hook | Trampoline |
|-------------:|---------------|------------|
| `+0x00` | Generic invoke (lifecycle entry) | `FUN_0040D760` (0x0040D760) |
| `+0x04` | Lifecycle event 2 | `FUN_0040D7A0` (0x0040D7A0) |
| `+0x08` | Relation-attach callback reached by `FUN_00416700` | `FUN_0040DBF0` (0x0040DBF0), through type-vtable `+0x44` |
| `+0x0C` | Relation-release callback reached by `FUN_00416750` | `FUN_0040DC50` (0x0040DC50), through type-vtable `+0x48` |
| `+0x10` | Bare-terrain callback before the generic solid response | `FUN_0040D7F0` (0x0040D7F0), through type-vtable `+0x0C` |
| `+0x14` | Surface/contact callback before the generic whole-body response | `FUN_0040D860` (0x0040D860), through type-vtable `+0x10` |
| `+0x18` | Active-entity pair contact | `FUN_0040D8D0` (0x0040D8D0), through type-vtable `+0x38` |
| `+0x1C` | Respawn/state callback | `FUN_0040D920` (0x0040D920) |
| `+0x20` | Infection hit | `FUN_0040DA00`, through type-vtable `+0x18` / `FUN_00411250` |
| `+0x24` | Cure hit | `FUN_0040DA60`, through type-vtable `+0x1C` / `FUN_00411320`; [Antidote contract](PLAYER_CRAFT.md#fire-both-modes) |
| `+0x28` | Primary-hit impact callback | `FUN_0040DAC0` (0x0040DAC0), through type-vtable `+0x14` |
| `+0x2C` | Death callback before the standard continuation | `FUN_0040DB80` (0x0040DB80) |

The ordinary style-only trampolines follow this pattern:

```c
entity_data = FUN_0043a580(entity_id);     // Resolve entity record
behavior_blk = *(int*)(entity_data + 0xb8);
if (behavior_blk && *(int*)(behavior_blk + SLOT_OFFSET) != 0) {
    (**(code**)(behavior_blk + SLOT_OFFSET))(
        entity_id,
        *(uint*)(entity_data + 0xc0),    // behavior context
        &call_param);
}
```

`FUN_0040D7F0` additionally re-resolves a surviving target and always calls
`FUN_004141D0(handle, contact_record, 0x7fffffff, 2, 1)` after its optional
style `+0x10` callback. `FUN_0040D860` similarly re-resolves and always calls
`FUN_004141D0(handle, contact_record, 0, 5, 0)` after its optional style
`+0x14` callback. `FUN_0040DAC0` has no such surface tail: it packages the
primary-hit impact, provenance, and direction arguments and dispatches only
style `+0x28`.

The shared late flying surface phase now admits native living13/10/57 and15/87
through their own allocation/task custody. Null Search/Move solid and water
hooks still run141D0; Type10/57 terrain-induced death retains its native Tumble
publication and subsequent changed water hook. Type15/87 instead retain their
authored class2 quiet death, sound11 release and immediate task retirement.
Type15 Hive children now enter that same family through an opaque native
allocation receipt, independently of Intro2 spawn44. Their populated zero
instance request retains the actual parent position/objective bit and shared
body ordinal; `104B0 -> 09A80` consumes Sub-G `1B8C0`'s word before the
process-owned `203D0` Sub-D allocation. `25680` then draws even for the sole
Always choice, followed by Secondary target acquisition and Primary retarget
`06070/B940`, for four constructor RNG words in order. The later child `+0x60`
source is permitted without manufacturing an attachment at `+0x80`.
Movement, aiming, particle hit and quiet death all authenticate that original
allocation; stale copied receipts fail before their callback/RNG prefix.
Authored state10000 exclusions,
underwater dives, and existing Tumble terminal response remain distinct.
The [Hive birth owner](HIVE_WRECK.md#authored-creature-births-and-failed-world-selection)
retains FIFO/ejection and mutation-sensitive same-pass scheduling. Source
evidence, controlled model regressions and remaining death/ordinary-birth
boundaries are owned by [Flying actor terrain and water contact](FLYING_SURFACE_CONTACT.md).

Relation release itself is now retained as a detached, receipt-bound
transaction in `v2k-game::entity_relation_release`. `FUN_00416750` caches the
type record, clears entity bits `0x20001000`, sets bit `0x800`, then applies
the type-record `+0xC0` mapping from `FUN_0040D3C0`: it copies that dword to
entity `+0xC8`, conditionally sets entity bits `0x00080000/0x08000000`, and
conditionally clears `0x00000800/0x00008000/0x00010000/0x00020000/0x00040000`.
It next writes `g_entity_db` to entity `+0x80`, samples the cached type-vtable
`+0x48`, and invokes a non-null callback with the original
`(entity, relation_argument)` pair. The callback result remains opaque and is
returned bit-for-bit; a missing entry or null callback returns zero. The Rust
transaction authenticates both the entity allocation and cached type-record
identity and deliberately performs no post-callback entity read, because the
callback may delete or rebind the entity and retail also returns immediately.
Dispatch or disposal of the returned opaque object belongs to each
`FUN_00416750` caller and is not folded into the release transaction.

The common type-vtable record at `0x004C8A30` stores these dispatchers. In
particular, `+0x0C = FUN_0040D7F0`, `+0x10 = FUN_0040D860`,
`+0x38 = FUN_0040D8D0`, and `+0x48 = FUN_0040DC50`. The separate per-behavior
style assigned to `entity+0xB8` holds the actual callbacks. The
2026-07-19 21-entity census confirms that current styles are heterogeneous even
within one type, so neither style identity nor callback policy may be frozen by
entity type.

Do not conflate behavior-style `+0x0C/+0x18` with component-state `+0x18`.
`FUN_0040A900`/`FUN_00401290` independently walks up to three component
wrappers and invokes each component state's `+0x18` contact callback with its
`+0x1C` context. The active-pair solver may therefore encounter both behavior
and component contact hooks. Ordinary entity scheduling instead reaches
type-vtable `+0x24/+0x28` plus component tick callbacks; it does not call
behavior-style `+0x18` as an ordinary entity update.

### Current Type-9 task ownership

The ordinary authored-world loader now publishes the shared Type9 constructor
through [`ordinary_type9_construction.rs`](../../crates/v2k-game/src/ordinary_type9_construction.rs).
It admits the canonical Section12 row independently of level, spawn index or a
captured allocation seed. `104B0 -> 09A80 -> D4A0 -> 25680 -> ABE0` retains
the actual authored parameter, Euler words, terrain domain/model slot and
grounded anchor. Sub-A consumes its `20450` word before the four-way selector;
all four selected initializer transactions remain available, including the
persistent player's class45 candidate. Later authored records cannot enter an
earlier constructor's candidate prefix. Linking and F70/bit4 follow complete
publication. The separate [Intro2 publisher](INTRO2_TYPE9.md) retains its exact
cohort and first-query evidence; it does not provide ordinary-world admission.

Each native receipt binds the manager's allocation generation and actor id to
the birth inputs and component allocation. Initial scheduler adoption validates
that lease before transferring the complete graph. Cargo uses the current
completed-visit lease, while checked damage, standard death and class14 reject
a stale native receipt even when the actor id matches another manager. Rejected
checked-damage admission leaves health, damage buffer, state and RNG unchanged;
genuine later death failures still retain their source-ordered damage prefix.
Native-world tests use levels14/15/25/39 for motion and death, plus real player
beam/Type93 release cycles in14/39. The wider migration boundary remains in
[Cross-level gameplay runtime](CROSS_LEVEL_GAMEPLAY_RUNTIME.md).

[Native Type17 capture](TYPE17_CAPTURE.md) transfers those same person owners
through the captor's C910/16700 and CAD0/443D10 release callbacks. The child
keeps its own Carried/Exploding task; scheduler custody can reside before or
after the current captor cursor. Shared checked damage reborrows the current
notification context for nested child work before its outer player-kill hint.

Live class45 resource text is dispatched at BA40's source callback before Cue
and Primary preparation, using the current actor-frame tick. Each production
frame receives an explicit notification sink from the scheduler; a constructor
failure retains already-issued text, and event16 dedup preserves its first
timestamp. Sound72 keeps its existing source ordering. Only authored load
construction retains deferred text receipts with load tick0; task-driven roots
and hit-driven C690 must not queue text for that load-only drain.

Sub-D receives the shared process owner's actual successful allocation seed,
including preceding unsupported or classifier-free actors. Native first use
explicitly anchors a cleared cache at the queried cell. This deterministic
`NativeFirstQueryReset` policy does not assert a retail heap origin or prove
that every retail allocation takes the full-reset branch. Known-residue and
captured-first-query fixtures retain their own distinct policies. The shared
outer tail continues to require its recovered mode-zero wind/drag contract.

Birth receipts authenticate the initial task graph, not every future graph.
After a root initializer publishes replacement tasks and its complete outer
suffix finishes, the heterogeneous scheduler transfers that exact publication
to its matching Wander, Go-To-Job, Run Away, or Attract owner. The current
authority retains allocation, context, component anchor, wrapper IDs, and task
family while allowing callback-owned private state and elapsed time to evolve.
It consumes no additional selector or constructor RNG and does not revisit the
new Primary in the same frame. A root selected during Primary still visits its
new Secondary/Tertiary through the remaining dispatcher cursor; Run Away can
therefore enter Attract or Go-To-Job without losing that callback suffix.
Incomplete callback/root/surface transactions still
retain their predecessor and cannot transfer or accept cargo callbacks.
An Attract Candidate may publish Target Route either in the root constructor's
remaining slots or on a later follow-up. Both paths consume the Candidate's
typed completion. After the outer tail completes, its exact route publication
transfers to the native Attract owner, whose existing C690/AC60 handler owns
invalid/zero returns and the strict five-second expiry. A host must not keep
authenticating the discarded Candidate graph or park every route expiry.

The former birth-only next-frame gate dropped every ordinary peasant after its
first task replacement (about two or five seconds), leaving walking animation
without movement. Cross-family selectors also preserve context target/auxiliary
words; a new Wander must not reinterpret an inherited Run Away context as a
fresh null-target/zero-auxiliary constructor. The sustained movement regression
checks all six peasants through repeated roots and three frame cadences, with
actual XZ motion long after the first expiry. Main Base abort's separate
initial-owner transaction remains a distinct boundary from normal conversion.

Fresh native peasants also initialize the transient callback-mass contribution
at entity `+0xB2` once, independently of first-scheduler flag publication.
Retail leaves this word as allocator residue; the accepted first-visit wait
captures clear it but do not prove that every RNG history waits. A first-visit
Continue previously retained `CallbackMassUnavailable` forever, freezing the
windmill peasant before any player approach. The explicit zero initialization
replaces undefined residue while preserving random scheduling and later mass
contributions; generic unresolved births remain evidence-gated. See the
[constructor evidence and policy](ACTOR_TASK_PROGRAMS.md#fresh-type-9-callback-mass)
and the RNG/cadence regression in
[`ordinary_type9_go_to_job_live.rs`](../../crates/v2k-game/tests/ordinary_type9_go_to_job_live.rs).

The later player active-pair pass is another writer between completed visits.
A positive ordinary Wander descriptor contact (`FUN_00402DA0`) adds `0x2000`
to heading and copies the task direction to Sub-A; its physical suffix may
also displace the actor and change velocity. These writes must cross an
authenticated scheduler boundary. Otherwise the next visit rejects the old
completed outer-tail snapshot with `OuterTailStateMismatch`, leaving a live
walking task that never moves again. The canonical isolated Level-1 peasant
(entity 17, authored spawn 15 near model 410 `windmill`) reproduces this after
an overlapping player contact. Nearby approach without contact does not.
The contact callback preserves the current body basis: the following actor
visit's original F70 phase owns its rebuild. This is a shared ownership issue,
not an authored exception to that peasant's Wander program.
The live pair commit authenticates every affected completed Wander visit
after body/descriptor preflight and consumes its old tail before publishing
contact writes. Rear-half-space misses also need this handoff when their
physical suffix changes the body. Parked transactions and missing adopted
owners reject the entire pair commit without consuming another actor's
custody. This boundary does not broaden the admitted descriptor-task families.

The late ordinary Type9 static pass also writes position and velocity through
`11AD0 -> 12CF0 -> A8B0 -> D920 -> 11760`, including separation when collision damage
is zero. Its walking-family check authenticates the actual completed
Wander/Go-To-Job/Run Away/Attract observation, then the existing external-writer
transfer consumes that observation before task hooks or the physical write. The task graph,
clocks, next transaction identity and previous body basis remain intact.
Checking only an owner ID and idle wrapper flags is insufficient: a completed
outer tail still records the preceding pose and velocity. A stationary native
world13 reproduction reached static geometry with all six reported peasants
(IDs11/12/16/17/18/24) and dropped each on a later mismatching visit; omitting
only the static pass caused no drops. No new retail capture was needed.
Missing, stale or unfinished custody blocks the writer, and a failure after a
committed response parks the reached prefix. The corpus-backed regression in
`tests/type9_static_contact.rs` retains the actual static pass across1024 ticks;
its corrected retail-data root prevents the previous silent fixture skip.
The later world15 repair-object contact uses the exact kind2 zero-damage
return before the actor packet; its profile and immediate-burn branch are
owned by [the shared static-damage catalog](ENTITY_STATIC_DAMAGE_PROGRAMS.md).
This avoids parking a valid walking graph merely because weak contact reached
a previously unadmitted static descriptor.

The ordinary Type9 pass now retains A8B0's Primary/Secondary/Tertiary `+0x20`
callbacks before physical response. Wander, shared retarget, Go-To-Job,
Run Away and the Class45 target route install `02CA0`; Candidate, acquisition
and Cue retain null hooks. Type9's Sub-I branch adds `0x2000` to the live heading
and consumes two shared RNG words for each current callback's wrapped X/Z
private target. It leaves task clocks, Sub-D, animation and physical basis
unchanged. The source matrix and acceptance controls are in
[Type9 static task contact](INTRO2_TYPE9.md#ordinary-static-task-contact).

Playing retains Type9's entry flags/model/+70 before its static prefix, then
visits only successor actor pairs with the current pose and callbacks. Lethal
static death preserves that same-pass admission; an unowned static suffix
stops only that subject. Ordinary and native Intro2 Type9 peers now join the same walk,
with actual carried and Class14 eligibility and synchronous lethal peer
publication. Source order and controlled regression scope remain in
[Type9 late actor pairs](INTRO2_TYPE9.md#ordinary-late-actor-pairs).

Native nonplayer spider contacts and native Type9 peer pairs use the separate shared
[`native_actor_descriptor_contact.rs`](../../crates/v2k-game/src/native_actor_descriptor_contact.rs)
owner for actual Type8/9/17/47/53 and existing Intro2 Type94 tasks. Each current task constructor determines
whether A900's +18 is null or02DA0; a whole-chain callback flag cannot erase an
opposing actor's steering callback. The wrapped signed forward projection runs
before private-state and descriptor reads. A negative projection is a complete
no-op. On an accepted projection, ABDI people add heading2000 and propagate
direction to Sub-A without RNG or animation changes; the no-Sub-I insect families
retain the descriptor's reversal, signed Sub-D step and X/Z random draws.
The current task clock, target ownership and incoming physical basis survive.
Both bodies lend completed custody even for null hooks and zero damage, and a
completed C910/D0B0 graph replacement is registered before that same pair's
component walk. A later failure retains the actual task owner and its linear
continuations in a contact-prefix wrapper; scheduler, hit, radial and adoption
entry points cannot replay or mutate that parked prefix. The existing player
bridge remains a separate admission boundary. Type53's shared authored birth
uses each allocation's own Sub-D seed and preceding live list; its fixed Intro2
first-query receipts belong only to the explicitly selected capture fixture.

Playing's static and ordinary-Hive radial passes are also external velocity
writers. `14AE0` may apply an impulse even when both damage amounts are zero;
that write previously left the selected Type9 owner's completed observation
stale. Playing now walks each allocation in source live-list order. For a
retained Type9 target it preflights that target, authenticates its affected
Wander/Go-To-Job/Run Away/Attract owner, and acknowledges only the planned
velocity before committing that entity and any player hull change. No task
time, selector RNG, steering, or matrix rebuild is replayed. A missing or
unfinished owner blocks that target without undoing earlier completed targets.
Fixed targets and unchanged velocities need no handoff. Native Types6/8/66
instead use their full radial damage/death callback and immediately retain its
replacement task before sampling the next live link. Type9's full ordinary
style-hit/reaction and callback-bearing radial-death migration remains separate;
an unsupported retained death still reports its exact boundary. The impulse-induced
freeze is covered by a regression. The user has confirmed that the windmill
peasant now moves normally; the reported intermittent startup failure is closed,
although its specific trigger was not isolated.
The historical report also included a frozen sprite surviving a registered kill;
the user confirmed that the villager issue no longer occurs.

The interactive runtime reports actual selected Type9 callback blocks and
lost owners independently per actor, task family, and failure kind, with the
authored spawn index and retail tick. Repeated parked callbacks do not flood
the log; normal retirement does not report a lost live owner. The historical
`PostBasisTailPending` success name is misleading: the live driver returns it
only after completing the outer tail. It must not consume an error-reporting
gate or conceal a later peasant failure. If movement regresses, use the first
current port `Blocked` or live `Dropped` report to identify the failed phase
before requesting additional retail evidence.

`FUN_00401430`'s tracked-target prelude reads target state `0x4000` and tests
the whole word for zero; it does not read every individual state bit. The
tracked snapshot therefore retains `RetailStateWord` evidence. Known nonzero
state with a known-clear `0x4000` can follow the target even when unrelated
bits are unresolved; uncertain live/dying classification remains blocked.
The first-world Main Base proves this distinction: `0x0e028805` under known
mask `0xff9fffff` leaves only surface bits `0x00600000` unknown, which must
not prevent an ordinary peasant's Go-To-Job movement.

### Peasant walking and attention animation

Attention acquisition reads the separate eight-byte common-axis runtime:
`FUN_00405FF0` stores the component array in task `+0x08`, and
`FUN_00401FB0` resolves that array's `+0x28`. `FUN_004235A0/004235D0`
allocate and initialize its range/filter from type `+0xC8/+0xCC`. The
one-in-four accepted gate writes filter `0x201` to this runtime's `+0x04`;
the authored Type-9 range remains `0xF00`. Behavior context target `+0x08`
and auxiliary `+0x0C` are separate and survive this write. Confusing those
records made a retained player handle of 1 become a one-unit search radius,
preventing reacquisition and repeatedly restarting the one-second help cue.
Candidate and Cue ownership now retain the correct component state across
root replacements; target-route predicates consume only the range. A new
Candidate in an expired route's Secondary suffix can allocate the next route
in that same visit. The original allows occasional consecutive help calls;
its five-second target-route lifetime is not a universal sound cooldown.

Type 9's Sub-I descriptor binds model callback selector 1 with stride 4.
`FUN_00420730` initializes a zeroed 20-byte controller with an 80-ms reload.
`FUN_00420520` subtracts `elapsed_us / 1000` with per-call truncation; only a
negative countdown advances one phase, reloads exactly 80, and discards
overshoot. Walking/help wrap four phases; special/death stops at phase 3.
`FUN_00420650` reads the unsigned entity heading at `+0xA2` directly. Retail's
chase eye stays on the fixed -Z compass bearing, so this is not a
camera-relative angle. `FUN_00413F70`'s forward axis agrees with the authored
`man2` body frames:

| Heading centre | World direction | Selector group | Artwork seen from retail's chase side |
|---:|---|---:|---|
| `0000` | +X | 24–27 | Right |
| `2000` | +X/+Z | 20–23 | Away/right |
| `4000` | +Z | 16–19 | Away |
| `6000` | -X/+Z | 12–15 | Away/left |
| `8000` | -X | 8–11 | Left |
| `A000` | -X/-Z | 4–7 | Toward/left |
| `C000` | -Z | 0–3 | Toward |
| `E000` | +X/-Z | 28–31 | Toward/right |

Each bin covers its centre ±`0x1000`, including the lower boundary. The
normal-tier model 558 uses sprites 1341–1356 with authored vertex-order
mirroring for those 32 walking selectors; sprite 1359 is a separate support.
Valid linked-handle or forced-stop state selects 32–35, the raised-hands
sprites 1357/1358. Special state takes precedence and selects death sprites
1360–1363 at 38–41. Model-only selectors 36/37 contain another authored
help pair but are not reached by this four-phase controller. Callback binding,
all frame branches, and body-basis directions are checked against the canonical
tier-1 assets in `peasant_animation_frames.rs`. The renderer keeps its existing
[camera-facing local-X policy](RENDER_PIPELINE.md#named-presentation-adapters).

The attention cue owns destructor `FUN_00402AC0`, which calls `FUN_00420830`:
clear the Sub-I linked handle, phase and forced-stop byte, while preserving
countdown, output and special state. `FUN_004010D0` invokes this when the inner
task is destroyed, including a clear while its wrapper is executing; deferred
wrapper reclamation must not defer the component reset. Replacement retires
the old cue before a new cue sets `FUN_00420870` forced-stop. Root changes,
candidate acceptance, carrying and death must all use that same retirement
hook. The next admitted Sub-I call publishes the walking or special selector;
the destructor itself does not synthesize a new output frame.

Presentation traversal `FUN_00411400` writes detail bits `0x06000000` after
simulation and before model culling. Ordinary selected Type-9 receipts now
receive that classification, so nearby villagers use the detailed callback
on the next visit. Coarse `FUN_00401430` deliberately skips Sub-I; both modes,
waiting visits and blocked visits retain animation custody and exclude the
compatibility fallback. Presentation waits for a selected actor's complete
visit; parked callback/outer-tail transactions retain their frozen mode.
Completed outer-tail receipts permit only the presentation-owned detail bits
to change, keeping pose, movement, lifecycle and `+0xB2` checks exact. Retained
Attract publication receipts synchronously acknowledge that classifier write
after validating the complete before/after snapshot; ordinary publication
checks remain strict. The former Type-17-only presentation bridge stranded
peasants in coarse mode, where help froze on the last walking frame. This is
separate from the missing cue destructor that retained forced-stop after a
return to walking.

### Ordinary Type-9 cargo callbacks

The ordinary peasant's initial null release callback does not mean pickup/drop
preserves its task graph. `FUN_00416700` first sets state `0x1000` and the
parent at `+0x80`, then reaches behavior-style `+0x08` through `FUN_0040DBF0`.
Classes 6/54 use `FUN_0040CD50` to select carrying variant 1; classes 10/45
use `FUN_0040CE70` to select variant 2. For capability `0x1804`, the shared
`FUN_0040CD70` path calls Sub-I `FUN_00420760` with the parent, clears
`0x8000`, and installs the carrying style through `FUN_0040C6B0`.
Its `FUN_0040ADB0` initializer deletes Tertiary and Secondary and installs a
real Primary None task (`FUN_00403230`), whose `FUN_00403250` tick only
advances Sub-I. It is not an empty task slot or a suspended Wander graph.
If Tertiary held an attention cue, its destructor runs after the preceding
`FUN_00420760` attach and clears the Sub-I linked handle again. The physical
parent relation remains attached; animation linkage must retain this actual
callback result rather than be inferred from that relation.

The carrying style's release callback is `FUN_0040CE90`: set `0x8000`,
release Sub-I with `FUN_00420830` if present, then call `FUN_0040AC60` with
the retained authored behavior-choice source. Type68 instead uses the separately
closed [D1C0 class0 release](CLASS0_RUNTIME.md#type68-attach-and-release): DC50
passes its context as the second argument, not the packaged parent handle.

A successful living player drop has **two** release/reselection boundaries.
`FUN_00443D30` creates Type 93, releases the child from the player through
`FUN_00416750`, then `FUN_00408F00` attaches it to the proxy and selects a
carrying style again. Later `FUN_00409030` copies the settled proxy position
to both child `+0x96` and anchor `+0x90`, enables master motion, and calls
`FUN_00416750` again. The first selector therefore sees the old anchor and
the second sees the landing anchor; their RNG consumption must not be merged
or deferred until the next actor callback. Sub-D allocation/cache state is
retained across these relation changes.

The live cargo phase constructs and attaches before the actor pass; Type93's
pose and settled-release callbacks run afterward. Type93 uses the authenticated
data constructor and consumes its own singleton selector word before the first
release selector. The carried None owner advances Sub-I
in both detailed and coarse callbacks; entry and post-cargo animation claims
prevent the neutral fallback from advancing it a second time. That fallback
also requires an actually neutral controller: a coarse Attract callback's
new forced-stop state remains owned by its task, as do linked/special modes.
Carrying effective flags are `(0x2f | 0x80) & ~2 = 0xad`: no gravity or DF70 ground snap,
but callback-mass-dependent drag, the basis rebuild, E370, B2, and master-motion
policy remain ordered. Deep-water E370 shares the timer, bubble allocation and
sound owner used by detached actors, including the source-proven carried expiry.

At expiry, `162B0` writes the surface timer before `16750` applies its fixed
relation-release prefix and calls carrying-style CE90. CE90's actual weighted
living selection and initializer run synchronously before `10C10` publishes
class14; their RNG, Sub-I release, temporary attention text/sound and task
destructors cannot be skipped because that living graph never receives a visit.
[lifecycle.rs](../../crates/v2k-game/src/ordinary_type9_carried_production/lifecycle.rs)
preflights the real intrusive-list candidates and job evidence before scheduler
RNG, then composes the existing cargo release transaction with generic standard
death, avoiding a second release prefix. It consumes the carried None receipt
and returns new class14 custody for the next scheduler visit.

E370 retains its entry model extent while rereading live state/position after
death for bubble and sound decisions. Exact expiry reaches the zero-percent
bubble gate; normal timer overshoot wraps the unsigned percentage and skips the
random-effect tail. The new dying bit suppresses sound106. The caller retains
its entry AD policy, then clears B2 and reloads the current master-motion flags,
matching `12DA0`; death does not introduce gravity or ground snap in this visit.
The released child remains in its parent's Sub-J row until the later `18640`
walk removes the dying entry without another release callback.

The [focused expiry controls](../../crates/v2k-game/src/ordinary_type9_carried_production/expiry_tests.rs)
cover detailed/coarse callbacks, exact expiry/overshoot, actual transient Wander
and Attract constructors, fresh surface RNG, current B2 mass, retained basis and
clocks, next-visit class14 custody, and native Level1 beam-drop Type93 row/body
cleanup. Unresolved release inputs or invalid live death cues still block before
scheduler or callback mutation; the death owner's unresolved-cue fallback to
authored sound35 remains admitted. These source-backed expiry controls extend
beyond NoCD03's observed 545ms carried window; they are not additional retail observations. Nonzero wind
remains an explicit unsupported branch; all five authored ordinary Type9 worlds
use mode0, drag3 and a zero vector.

Pickup `FUN_00443B50` calls Sub-J append `FUN_00418440` before
`FUN_00416700`. The player's zero slot-policy byte clears child state `0x800`,
and its nonzero outer policy sets `0x20000000`. These are legitimate writes
between actor visits, but they invalidate an unchanged completed-task receipt.
The port therefore preflights the append without mutation, consumes the exact
selected task custody, then commits the Sub-J row/policies, parent relation,
and behavior callback in retail order. It must not reauthenticate the old
receipt after the policy writes or weaken its state comparison. The live
regression collects a moving, presented Attract peasant and continues its
carried None task; unavailable attachment descriptors still fail before
either the row or child state changes.

### NoCD03 deep-water cargo

The user-supplied `<install>\V2000-nocd03.run` records ordinary Type9
`04AC0001` (allocation `03743700`, stamp `0x412`) collected by Type46
`04BE0001`, then dropped through Type93 `04970001`. The accepted timeline,
state, carried-surface, position-writer and exact-call oracles are named in the
capture ledger.

| Boundary | TTD position | Observed state |
|---|---|---|
| `443D30` drop | `30CBD7:1FC6` | Player XYZ `602A,FD92,9AB0`; child timer0, flags `26C21025`, parent `04BE0001` |
| `438080` Type93 request | `30CBD7:20B7` | Target `6025,F8CD,9ABB`, type93; vertical delta1221 raw units |
| `08F00` attachment complete | `30CBD9:1379` | Child flags `06C21025`, parent `04970001`, carrying style `004C8740`; proxy Y raised from `F8CD` to `F8CE` |
| `18640` child position writer | `30CBDC:9DE` | `41877D/418783` copy clamped proxy XYZ to child+96/+9A, after clearing velocity; no slot offset with bit800 clear |
| First deep `E370` | `30D2C5:141` | Child at target, timer0; callback91000us increments to91ms |
| Final carried `E370` | `30F3B5:873` | Timer472→545ms with73000us; still carrying |
| `409030` landing release | `30F3B6:1ECB` | Current/anchor XYZ both target, timer545ms retained; release clears parent and selects living style `004C86B0` |
| Detached expiry release/death | `32364B:C` / `32364B:B5` | Timer5010ms; `16750` leaves flags `06C68825`, then `10C10` sets health0, flags `06C64825`, style `004C70C0` |

The sea header is `FFFCB100` (raw sea Y `-847`); active model 558's header+08
extent is 165, so the deep threshold is `-888`. The immediate post-attach
callback still sees the child's old Y `-633` and leaves timer 0. The next seven
carried callbacks see seabed Y `-1843`, with whole-ms deltas
`91,79,79,78,72,73,73` and timer endpoints `91,170,249,327,399,472,545`.
No carried callback reaches the below-75-percent bubble gate or expiry.
The position watchpoint establishes the missing parent-side writer: the
first child E370 at `30CBDA:1C05` precedes Type93's `18640`. The port now
publishes the native proxy's zero-offset Sub-J row after its terrain clamp.
Player-drop proxies, appended after their existing cargo allocation, now run
in the later Materialiser phase after actor visits. Running them in the early
beam-command phase would move the first water tick and landing release ahead
of the child callback. The geometry-only test adapter remains independent.
Static `18640` also removes missing, dying or whole-state-zero child rows
without invoking their release callbacks. Once that row is empty, the later
`409030` expiry removes the proxy without releasing the stale sidecar child.
The component callback precedes Sub-J, so a row that first becomes stale on
the expiry visit still follows `18500`'s original callback admission.
Breakpoints on every `Random_Next` inside the first and last deep calls prove
zero surface draws, beyond their matching seed endpoints. The detached death
has exactly one draw through caller `406146`, seed `53222EF0→A807D1F3`;
later already-dying expiry calls recur before cleanup. This recording does
not establish nested expiry while attached, nor whole-scene RNG equivalence.

The port regression retains the actual carrying None task and replays all
seven nonterminal deltas. Static shared E370 logic also
owns later nonterminal bubble/sound decisions; these effects are not claimed
as observed inside this short attachment window.
The production [cargo regression](../../crates/v2k-game/tests/peasant_cargo_live.rs)
also collects a live peasant, drops it in this seabed area, and follows the
real Type93 release through class14 and deferred cleanup. Death before another
full five seconds after landing verifies that release preserves the carried
surface time without exposing the actor's private timer.
The Main Base approach matrix also retains its submerged western start at
world X/Z `68/60`, terrain raw Y `-1440`. With 20-ms callbacks, that peasant
walks toward the Base but reaches surface expiry at Y `-896`, still below the
`-888` threshold. This case now expects class14 before conversion; the other
23 approach/cadence cases still require successful delivery. These are port
regressions against the recovered timer and movement rules, not additional
observations from the retail recording.

The recorded executable has the same SHA256 and sole protection-branch byte
difference as [NoCD02](CLEANSING_VEHICLE.md#nocd02-replay-provenance).
`nocd03-identity.json` records verified image hashes, trace sizes, recording
session ID and replay lifetime. All five `.txt`/`.windbg.cmd` pairs and the
shared `nocd03-query.ps1` remain local evidence. Exact oracle invocation:

```powershell
& '<WinDbg>\cdb.exe' -sins -y '<workspace>\.tmp\ttd-worker-symbols' -logo '<workspace>\.tmp\nocd03-surface-oracle.txt' -z '<install>\V2000-nocd03.run' -c '$$><<workspace>\.tmp\nocd03-surface-oracle.windbg.cmd'
```

The other four queries change only the log/script stem to the ledger names.
They use read-only seeks, breakpoints, memory reads and debugger scratch
registers. Every transcript reaches its `NOCD03_*_COMPLETE` marker; the final
command-file diagnostic occurs afterward, and the launcher closes only its
owned debugger. Missing Windows DLL symbols do not invalidate game-memory
reads or the recorded call boundaries.

## Named Behaviors (CONFIRMED)

Behavior name table starts at **`0x004C8AA0`** (8-byte stride, each entry =
`{name_ptr, descriptor_ptr}`). `Defecate Virus` is entry 4 at `0x004C8AC0`;
`Change Sea Level` is entry 38 at `0x004C8BD0`. The second field is **not** an
alternate/empty string and is not an argument-signature pointer. It addresses
a 16-byte descriptor containing two pointers into callback/prototype records.
For `Change Sea Level`, descriptor `0x004C8980` is:

```text
{ 0x004C7348, 0, 0x004C85D8, 0 }
```

The first pointer's little-endian bytes are `48 73 4C 00`, which happen to look
like ASCII `"HsL"`; treating those bytes as a serialization signature is a
false lead. The surrounding entries prove the same field is a pointer (most
produce non-printable byte sequences), and both targets sit eight bytes into
larger callback records (`0x004C7340` begins with `FUN_0040C560`;
`0x004C85D0` begins with `FUN_0040B0B0`).

| Behavior name | String addr | Role |
|---------------|-------------|------|
| `"Defecate Virus"` | `0x004C9258` (table entry at `0x004C8AC0`) | Descriptor `0x004C88C8`, prototype `0x004C7E88`, callback `FUN_0040B9E0`; installs shared terrain-contact mode 5, which sets infection |
| `"Cleansing Landscape"` | `0x004C8FC0` (table entry at `0x004C8BF0`) | Descriptor `0x004C8988`, primary prototype `0x004C85D8`, callback `FUN_0040AD50`; installs shared terrain-contact mode 6, which clears infection. Player Antidote selector7 uses its separate F950 terrain-clear callback; selector29 retains F6E0/class78 firing admission while its41850 callback body remains unowned |
| `"Generate Creatures"` | `0x004C9014` | `FUN_004132d0`, the global type-61/pickup spawner documented in GAME_MECHANICS (anchor for table interpretation) |
| `"Capture People"` | `0x004C91E0` | Native-capture behavior (rescuable native) |
| `"Mutated"` / `"Normal"` / `"Killed"` | `0x004C8DE4` / `0x004C8DF4` / etc. | Plant lifecycle states (not behaviors but referenced near table) |
| 70+ other behavior names | `v2000_exe_strings.txt:820-902` | Complete behavior catalog: `Defender Drone`, `Capture People`, `Goto Target`, `Wander Near Location`, `Move About Aimlessly`, `Search And Attack Player`, `Tractor Beam`, `Megablast`, etc. |

The named behavior table and the terrain-grid controller are complementary
systems. Neither behavior name establishes a per-frame plant tick, and the
runtime trace below directly disproves the former claim that there is no global
terrain-grid evolution.

### Ordinary Level-1 actor survey (RUNTIME-VALIDATED 2026-07-24)

The retained friendly/non-shooting/shooting surveys
`20260724-041815-friendly-ai.jsonl`,
`20260724-041937-enemy-ai.jsonl`, and
`20260724-042053-enemy-ai.jsonl` validate the reachable style families without
inventing allegiance from the wrapper names. Type 9 `man2` visits Wander Near,
Run Away, and Attract Attention; type 17 `spider` visits Capture People, Run
Away, and Follow Beacons; type 47 `newant` visits Guard Location and Wander
Near. The captures close Follow Beacons variant 1 (`0x004C7B70`) and Attract
Attention variant 1 (`0x004C86F8`) as null-release/null-pair/null-death styles.
The type-47 run has sparse source-correlated particle classes 87/88/73, while
the type-17 run has no sustained equivalent projectile family.

Stable task snapshots bind style/tick pairs as follows: type 9 uses
`0x004C79C0/0x00402EB0`, `0x004C7660/0x00403F40`,
`0x004C86B0/0x00402BA0`, and `0x004C86F8/0x00403780`; type 17 uses
`0x004C8038/0x00403780`, `0x004C7660/0x00403F40`, and
`0x004C7B70/0x00403CE0`; type 47 uses
`0x004C7BB8/0x00402EB0`, `0x004C7C00/0x00403490`, and
`0x004C79C0/0x00402EB0`. Static call-site recovery shows every listed slot-0
tick reaches `FUN_00401430`. The captures therefore validate installed
topology but do not make any ordinary actor locomotion independently portable.

Retail `FUN_00403780` and demo `FUN_004037C0` are structurally identical. They
read the retained target from the shared `FUN_004012E0` private record and
return the first tagged singleton for a missing target, state zero, or state
bit `0x4000`, without calling either suffix. A live target reaches strict route
predicate `FUN_00423030`/demo `FUN_00422F00` and then always reaches common
mover `FUN_00401430`. Predicate nonzero plus mover nonzero returns null;
predicate nonzero plus mover zero returns the middle singleton; predicate zero
returns the third singleton after the mover regardless of its result. The
three retail objects at `0x004BE0B8/0x004BE0C0/0x004BE0C8` remain
pointer-distinct with common tag `0x9C01`. The callback and predicate consume no
process RNG directly; any draw belongs to the nested mover's own retarget path.

Rust now keeps that program in behavior-neutral `shared_target_route` state,
evaluator, lifetime, and post-unwind types. Go-To-Job delegates through a thin
behavior wrapper, while class-45 Attract Attention retains a separate linear
Primary owner. The Attract owner authenticates exact Level-1 type-9 metadata,
selected component custody, target style/context, task lease, immutable anchor,
Sub-A/Sub-I runtime, and callback-entry body basis before mutation. It advances
the strict `elapsed > 5000` lifetime before callback entry, stages mover writes,
commits them only after a resolved callback and surviving wrapper, gives a
tagged result precedence over timeout, and then applies the generic owner
state-bit-`0x1000` gate after unwind. Suppression retains the same linear owner;
an unresolved gate fails closed after the consumed prefix. The returned
transition receipt deliberately stops before the type-default/root Primary
reselection: retail's root callback is invoked with null and therefore must not
be modeled as a null call to argument-dereferencing `FUN_0040C690`. The
detached ordinary-Type-9 root planner now owns that live selector boundary. It
authenticates exact current type/model/metadata/context and the post-unwind
state bit, resolves Baddie -> Player -> Base proximity before one weighted-
selector word, or selects alternate class 14 with neither a list walk nor RNG
when bit `0x4000` is set. It reuses the existing context with TypeDefault choice
source while preserving target and auxiliary words. The production Attract
owner now enters the exact common scheduler and heterogeneous dispatcher with
fresh Primary/Secondary/Tertiary reads. Candidate consumes one gate word; an
accepted handoff self-retires, clears Cue, and synchronously publishes a
retained Target Route Primary whose stale result is discarded without
revisiting slot zero. Target allocation failure consumes no constructor RNG and
publishes the initializer fallback after clearing all three slots. A surviving
Cue receives the same-frame duration and expires only after unwind at strict
`elapsed > 1000`. A weighted
class-6 plan now continues in the retained Run Away, Go-To-Job, or selected-
Wander production owner without a selector redraw. Exact branch receipt, live
task, selected-custody, and predecessor/context authentication precede
replacement-context publication; target/auxiliary words survive and birth
provenance in `initial_behavior` is unchanged. The owner then performs the native Tertiary ->
Secondary -> fallible Primary transaction and retains the exact Wander or
initializer-fallback publication while parked at the same-frame outer suffix.
That retained owner consumes the same-frame direct F70 basis phase before
parking ahead of E100. Attract retains the same class-6/fallback root and F70
custody for Shared Retarget and Cue transitions; Candidate success parks the
exact Target Route owner after F70 without ticking it. After that frame's outer
tail completes, the following production visit enters this dedicated Primary
through the normal scheduler, authenticates the live target and actor-local
common-axis range, and runs the selected Type-9 common mover. A mismatched or
unresolved actor-local common-axis descriptor consumes the scheduler prefix
and route lifetime without calling the selected mover; post-entry adapter
failure parks `CallbackFailurePending` and cannot replay. An unresolved
selected-component immutable anchor is a mutation-free Target Route
preflight (`ImmutableAnchorUnavailable`): Attract has no production-level
anchor gate, so the visitor takes the route owner and enters preflight.
The outer scheduler prefix is already committed, the route owner is
retained, route lifetime does not age, and the next TargetRouteActive
visit retries the same Primary. It does not apply class-6. A Primary
wrapper that is not `{alive, !in_callback}` fails `route.validate()` and
drops `RootPublicationMismatch` before the scheduler prefix. That same
wrapper check lives inside Target Route publication, so
`Preflight(PublicationMismatch)` and `Preflight(TaskWrapperNotRunnable)`
are not reachable from this production visitor.
`Preflight(MetadataContractMismatch)` is not a reachable Target Route
production outcome: the visitor always supplies `type_runtime_metadata(9)`
or parks `RuntimeMetadataUnavailable`, and publication does not re-type
the actor. Reaching that preflight requires forging `entity_type` or
replacing the type-9 metadata row. An unresolved
Sub-A runtime is a mutation-free Target Route preflight: the production
visitor has already committed the outer scheduler prefix, but the route
owner is retained, route lifetime does not age, and the next
TargetRouteActive visit retries the same Primary. It does not apply
class-6. A still-known Sub-A runtime whose target-speed dword is unresolved
is the same preflight (`SubATargetSpeedUnresolved`) and also does not apply
class-6. An unresolved actor-animation runtime is the same preflight
(`ActorAnimationRuntimeUnavailable`) and also does not apply class-6. An
unresolved physical-body basis is the same preflight
(`PhysicalBodyBasisUnavailable`) and also does not apply class-6; the
published F70 tail must be consumed first, because that Complete
observation requires a Known basis. An
unresolved target-state word is the same consumed
target-validation failure and does not apply class-6. Continue and
state-`0x1000` suppression retain the same linear route owner; an unsuppressed
tag or strict `elapsed > 5000` expiry enters the existing no-redraw root/class-6
application, F70, and outer tail. A Continue expiry frame still commits the
selected mover before that apply. A suppressed `LifetimeExpired` Continue
still commits the selected mover and parks the same Target Route owner.
A live target outside the actor-local route range is the `ZeroPredicate`
tag and takes that same unsuppressed class-6 path after the selected mover.
The selected mover's prelude-Zero returns are dying or `state_flags == 0`,
already InvalidTarget before the mover, so `NonZeroPredicateAndZeroMover`
is not a reachable Target Route production outcome.
A dying or inactive target is that same
unsuppressed InvalidTarget tag and applies the existing class-6 root without
a mover visit.
Stable manager adoption now covers all of
those outcomes. Main Base restore/consume transfer remains limited to the
still-initial Attract graph; progressed Target Route, root-publication/retry,
and fallback custody fail closed. Selected Go-To-Job now also carries weighted
class-54 publication or terminal fallback through the original post-Primary
cursor and F70. Selected Attract now likewise carries one retained weighted
class-45 plan through its parity-ordered task/event/sound/animation transaction,
terminal fallback, original Secondary cursor, final graph authentication, and
F70. Preserved context target/auxiliary words and the actor-local common axis
bind any odd Candidate owner to the transition-time range/filter contract.
Matching selected-owner roots for classes 10, 54, and 45 are closed. Selected
Wander -> weighted Go-To-Job, selected Go-To-Job -> weighted Run Away, selected
Go-To-Job -> weighted Wander, selected Wander -> weighted Run Away, and selected
Wander -> weighted Attract Attention plus selected Attract Attention -> weighted
Go-To-Job and weighted Run Away, Go-To-Job -> Attract Attention, and Run Away
-> Attract Attention/Go-To-Job are ten closed cross-producer roots. The
three class-10 crossings authenticate their canonical TypeDefault/lone-Primary
or exact Attract initial/route predecessors, retained contexts and frozen
snapshots, exact two-phase success-only-RNG/fallback, and original cursor.
Attract admits SharedRetarget Primary -> Secondary, expired Cue -> terminal,
and Target Route Primary -> Secondary. Successful acquisition may
publish Fleeing with its own success-only generic-suffix/fixed-`5/3` word before
final authentication and F70/PostBasisTail; progressed Attract Main Base
custody stays fail closed. Go-To-Job -> class-45 Attract Attention owns both
the even transaction and odd Candidate/Target Route suffix. Run Away's
class-45/54 choices transfer their frozen plan and remaining dispatcher cursor
into that existing multi-family root continuation without a selector redraw.
Alternate class 14 and the generic Type-9 lifecycle share one
session-zero runtime oracle (`20260817-072421`). The six authenticated native
births use the separate [initial callback-mass policy](ACTOR_TASK_PROGRAMS.md#fresh-type-9-callback-mass).
Accepted trace `20260730-142219-villager-task-mover.txt` already corroborates
one route wrapper at lifetime `0x1388`, elapsed `0x1405`, successful mover
return, type-default choice source zero, and same-scheduler-call replacement by
class 6. That class is the observed weighted/data-driven result, not permission
to hardcode Wander Near as the root transition.

The focused Level-1 type-9 transcript
`runtime_re/captures/local/20260730-142219-villager-task-mover.txt` closes the
task/common-mover caller boundary for six villagers: all seven
`FUN_00401430` arguments, Wander Near, Run Away, and Attract Attention task
topology, the reflected-away and direct Run Away branches, and successful EAX
returns are observed. The companion
`20260730-143850-villager-task-mover.txt` covers only the first roughly
2--4 seconds of Intro2. It is positive evidence that early cinematic type-9
actors use the same machinery; it is not evidence that an unobserved behavior
is absent later in Intro2.

Those transcripts also correct two provenance labels. Behavior descriptor
`0x004C8868` is paired with string `0x004C918C`, **Exploding Person**;
**Trash Buildings** is descriptor `0x004C88E8`, paired with string
`0x004C917C`. Callback `FUN_00402BA0` is shared rather than
Attract-Attention-specific: call site `0x00402C87` was reached with live styles
`0x004C86B0` (Attract Attention), `0x004C7618` (initial Run Away), and
`0x004C70C0` (Exploding Person). Runtime interpretation must therefore use the
installed style/task context, not the callback address or call-site label
alone. In the early Intro2 sample, Exploding Person advances from style
`0x004C70C0` to `0x004C7108`; its 1000-ms task was observed crossing at
1125 ms under the intrusive 125-ms sampling cadence.

### Reef fish movement (RUNTIME-VALIDATED 2026-08-02)

The accepted corrected-filter trace
`runtime_re/captures/local/20260802-012632-fish-movement-Reef.jsonl` and its
protocol sidecar are a 30.5-second read-only aquatic observation against the
verified retail executable. The resource catalog retains presentation variant
1, aquatic resource overlay11 and gameplay overlay30 (Cistern). **Reef is the
capture scenario label, not proof that gameplay overlay23 was loaded.** Overlay11's
Section-8 global base1186 proves the local/global model mapping used by the
filter. Across all 1,492 stable actor samples the
selection is exactly 17 actors with the same model signature; 33 non-atomic
entity snapshots are explicit and are not trajectory samples:

| Canonical model | Local/global model | Retail type | Selected | Moved | Captured scope |
|-----------------|--------------------|-------------|---------:|------:|----------------|
| `lionfish` | 8 / 1194 | not observed | 0 | 0 | No presence or trajectory evidence |
| `pinkfish` | 10 / 1196 | 24 | 12 | 2 | Detailed 3-D motion plus coarse parked instances |
| `fatfish` | 11 / 1197 | 124 | 1 | 0 | Presence and coarse parked state only |
| `obfish` | 47 / 1233 | 22 | 4 | 4 | Detailed 3-D motion for every selected instance |

The six moving actors are `pinkfish` handles `0x048B0001`/`0x048A0001` and
`obfish` handles `0x048C0001`, `0x04890001`, `0x04880001`, and `0x04870001`.
All begin with detailed-update bit `0x02000000` and motion gate `0x00040000`
set. Their first stable position changes occur together at 2,400.4 ms, when the
captured pause stack becomes empty. The only earlier Enter edge is bracketed by
the retail pause screen at 1,780--1,880 ms; after unpause there is no captured
control edge until the F12 stop at 30,380 ms. Fish health, damage buffers,
anchors, model identities, and environment flags `0x200C` stay unchanged, and
no fish is born or unlinked.

Every detailed fish changes X, Y, and Z position, signed 8.8 velocity, body
basis, and rotation. The combined sampled velocity envelope is X
`[-750,666]`, Y `[-502,574]`, and Z `[-815,807]` raw 8.8 units. These are
one-run observations, not authored clamps or constants. The other ten
`pinkfish` and the one `fatfish` lack both detailed-update and motion-enable
bits for the whole trace; each retains exactly one position, zero velocity,
one rotation, and one body basis through all stable samples.

The detailed-to-coarse boundary is repeatable within this run. Five moving
fish clear `0x02000000` once, with their final position/velocity/basis/rotation
change in that same stable sample at 7,600.1, 9,020.3, 9,860.4, 13,820.5, or
13,900.6 ms. Boundary actor `0x04870001` toggles detailed/coarse several times
from 6,600.3 through 6,960.5 ms and advances in short bursts before its final
coarse state. For all six, `0x00040000` clears 60--140 ms after the final
detailed exit; pose and position then remain fixed while the last nonzero
velocity stays stored. This validates fish locomotion's coupling to the shared
detailed/coarse and motion-owned entity paths. It does not by itself identify
the range-test writer or prove the exact within-tick ordering.

The 610 deep snapshots per fish also bind installed behavior/task topology:
Move About Aimlessly style `0x004C7930` has slot-0 tick `0x00402BA0`;
Flocking styles `0x004C7DF8`/`0x004C7E40` retain
`0x00402BA0`+`0x00401FB0` and `0x00403780` respectively; and Wander Near
style `0x004C79C0` has `0x00402EB0`. The parked actors' task ages remain zero.
During detailed motion, two `obfish` change `0x004C7E40 -> 0x004C7930`, one
`obfish` changes `0x004C7930 -> 0x004C79C0`, and one `pinkfish` changes
`0x004C7930 -> 0x004C7E40`. These are live installed-state transitions, not
proof that the sampled callback addresses executed or proof of selector/RNG
order.

The trace is therefore a passive motion-envelope and scheduling-state oracle
for detailed `pinkfish` and `obfish`, plus a coarse-parking oracle for the
captured `pinkfish`/`fatfish`. It is not movement authority for absent
`lionfish` or the never-detailed `fatfish`, and it does not settle collision,
surface crossing, audio, effects, renderer order, or individual RNG draws.

The capture's selected families retain17 fish in Cistern30 (four Type22,
twelve Type24, one Type124); Reef23 authors29 in those three families
(fourteen Type22, fourteen Type24, one Type124). This filter omitted zebra
fish: Type23 adds three in Cistern and fourteen in Reef; Type62 adds all
three Level1 fish and occurs in14/18/36 as well. Both zebra types now use the
[same native owner](FISH_RUNTIME.md), with authored2004 instead of200C.
The original three selected types use B/D/F,
type+C0 `0x200C`, and no Sub-A/C. Their Always-weighted initial choices are
Type22 `Flocking3/Wander2/Aimless6`, Type24 `Flocking3/Wander2/Aimless4`, and
Type124 `Flocking10/Wander1`. Their former freeze came from missing native
construction/task/mover ownership. The [shared fish owner](FISH_RUNTIME.md)
now publishes that authored graph, including class13 `B640/AF50` acquisition
and reselection, and runs the B/D/F mover with the source detailed/coarse
policy. Coarse200C skips the complete task/outer callback and clears motion;
detailed fish resume it. Particle-hit/quiet-death ownership and remaining
combat boundaries are recorded in the fish runtime document.

Birth altitude is a separate resolved correction. D4A0's bit20 is clear, so
fish keep authored Y even when it is zero. The former generic zero-Y terrain
heuristic moved Reef23 spawn21 from `[28672,0,31232]` to Y1184. Shared
constructor placement now preserves its authored position. Conversely,
Cistern30 pinkfish spawns25/30 author Y3840 above the static sea3768; the
accepted trace retains them at `[2048,3840,-11520]` and `[-2304,3840,23552]`
with zero velocity throughout their coarse samples. Do not add an underwater
clamp. The corpus-backed
[`authored_fish_positions` regression](../../crates/v2k-game/tests/authored_fish_positions.rs)
covers both worlds and both altitude cases; the separate native fish tests own
swimming and coarse-parking regressions.

### Authored task programs (moved to dedicated owners)
The per-task program families moved verbatim during the cohesion split:

- Type-9 Run Away and Attract Attention, Search And Attack Target class 7,
  and Shared Aim And Fire slot-2 -> [ACTOR_TASK_PROGRAMS.md](ACTOR_TASK_PROGRAMS.md)
- Guard Location class 32 -> [GUARD_LOCATION_TASK.md](GUARD_LOCATION_TASK.md)
- Alpine Type30's native ABCDEHKL allocation, weighted5/7/26 graph, Method20
  FIFO and retained-bank Class12 -> [ALPINE_INSECTS.md](ALPINE_INSECTS.md#native-type30-owner)

Shared Sub-D steering `FUN_0041F660` rounds each signed Q31 basis product
before combining probe offsets. Left subtracts the lateral product at
`0041F740/0041F78D`; rear subtracts the half-forward product from position
at `0041F908/0041F936`. Half-forward first truncates signed
`direction * forward_distance / 2`; final coordinates wrap to 16 bits.
Negating a distance before Q31 multiplication changes fractional products by
one raw unit and can select the adjacent classifier cell at a world seam.

## Type-Authored Initial Behavior Selection (CONFIRMED)

Section 12 type-record offset `+0x118` is a pointer to a zero-terminated list
of 12-byte behavior choices, not a material list. Each choice is
`{ evaluator_rule, weight_multiplier, named_behavior_class }`. The loader
resolves the first field through the 8-byte-stride evaluator table at
`0x004C8CDC` and the third through the named behavior table at `0x004C8AA0`.
The proven evaluator ids are:

| Id | Evaluator | Callback |
|---:|-----------|----------|
| 1 | Always | `FUN_00425720` |
| 2 | Under Attack | `FUN_00416490` |
| 5 | Mutated | `FUN_004163E0` |
| 6 | Player Nearby | `FUN_00416550` |
| 7 | Baddie Nearby | `FUN_00416560` |
| 8 | Furniture Nearby | `FUN_00416650` |
| 9 | Buildings Nearby | `FUN_004166B0` |
| 10 | People Nearby | `FUN_00416590` |
| 11 | Beacon Nearby | `FUN_00416570` |
| 12 | Base Nearby | `FUN_004165B0` |
| 13 | Job Nearby | `FUN_004165C0` |

`FUN_00425680` evaluates and weights every candidate, caps only the upper
total at 0x7FFF, consumes a random word even for one deterministic candidate,
and chooses the first strict cumulative weight above the random threshold.
For the canonical Type9 row, the authored choices are Baddie
Nearby (`0x08`) x 10 -> class 10 Run Away, Player Nearby (`0x01`) x 3 -> class
45 Attract Attention, Base Nearby (`0x20`) x 200 -> class 54 Go To Job, and
Always x 1 -> class 6 Wander Near. The detached port planner authenticates the
admission receipt, exact type metadata, known capability word `0x1804`, null
recent relation and known birth anchor equal to current position before any
list walk or RNG. Captured Level1 fixtures retain their six spawn/seed pairs
and masked pre-wrapper state `0x06068801`; shared native construction instead
validates the receipt's actual authored pre-publication state and component
allocation. It completes
the Baddie, Player, and Base
first-eligible `FUN_00422C10` walks in authored order, consumes exactly one
caller-supplied random word, and returns a non-copyable receipt whose authority-
bearing fields are private. The planner itself still mutates nothing; the shared
preflight plus class-10, class-6, class-54, and class-45 adapters now consume
receipts through all four complete selected initializer/fallback boundaries.
The class-54 path retains the receipt's full ordered list
and admits only id-ordered, monotonic state/capability plus capacity refinement
before its nearest-target plan.
The reusable admission can issue more than one detached plan, but fresh
production now owns single issuance from the shared process RNG and consumes
one receipt through context allocation, branch dispatch, link, basis, and bit
`0x4`. No focused capture is required here. Selected Wander's class-6 self-root
is explicitly authenticated. Wander -> weighted Go-To-Job, Go-To-Job ->
weighted Run Away, Go-To-Job -> weighted Wander, Wander -> weighted Run Away,
Wander -> weighted Attract Attention, and Attract Attention -> weighted
Go-To-Job and weighted Run Away, Go-To-Job -> Attract Attention, and Run Away
-> Attract Attention/Go-To-Job are ten closed cross-producer roots. Attract ->
Run Away retains its three exact origin cursors, frozen no-redraw state, two-phase
success-only-RNG/fallback, optional same-pass Fleeing, final authentication,
F70/PostBasisTail, and fail-closed progressed Main Base custody. Go-To-Job ->
Attract owns both parity branches; Run Away -> Attract/Go-To-Job shares the
frozen-plan continuation described under current Type-9 task ownership.
Alternate class 14 and
generic lifecycle share one session-zero oracle; the native allocation policy
for fresh-birth `+0xB2` remains independent of the four-way task constructor
transaction.
`FUN_0040AC60` instead calls `FUN_00425660` when live entity state bit
`0x4000` is set; that direct path installs the alternate named behavior class
stored at Section-12 `+0x124`. Header `+0x11C` is an evaluator-rule reference
resolved through `0x004C8CDC`, not a weapon reference. Its later alternate-
path role is retained by the ordinary-Type-9 root planner as exact class 14.
The loader uses `+0x11C`'s nonzero presence to decide whether to resolve
`+0x124`.

For a clear `0x4000` bit, the same planner reuses the exact authored four-choice
list and shared evaluator walks against the current live owner/list snapshot;
it does not reuse the fresh-birth admission, anchor, or pre-publication-state
receipt. All three fallible walks complete before the one unconditional
selector draw. Existing-context `FUN_00438340 -> FUN_0040ABB0 -> FUN_0040C6B0`
supplies the replacement contract: preserve context target `+0x08` and
auxiliary `+0x0C`, replace the named program/style with TypeDefault choice
source, then let the selected initializer own its native publication/fallback
ordering. Exact retail/demo `FUN_00425680` and the accepted villager trace's
`choice_source=0` same-scheduler-call replacement close this planning boundary;
they do not authorize hardcoding the observed class 6 or delaying newborn
Attract Secondary/Tertiary tasks until another frame. Their detached same-pass
dispatcher admission is now closed.

For a weighted class-6 result, the retained Run Away, Go-To-Job, or selected-
Wander production owner consumes the plan linearly; it never redraws the
selector. Go-To-Job admission requires the canonical class-54 TypeDefault
context and its lone Primary task with Secondary/Tertiary empty; the production
dying-target path now exercises that exact predecessor through class-6
publication and F70 in the same tick. Before the constructor draw it
authenticates the exact branch post-unwind
receipt against the live task, selected
components, metadata/model/state, and predecessor/current context. The plan and
its authenticated task-wrapper snapshot remain in the same production owner
across an application retry or an unsupported selected branch. Class 6
publishes the planned TypeDefault context first, preserving predecessor target
`+0x08` and auxiliary `+0x0C`, while leaving `initial_behavior` as birth
provenance. Native `FUN_0040AD10` order then clears Tertiary, clears Secondary,
and fallibly prepares Primary. Allocation success consumes exactly one
constructor RNG word after allocation, applies the Sub-A reset before Primary
publication, and stores exact Wander task/publication custody in the production
owner. Allocation failure consumes zero constructor RNG, applies no Sub-A
reset, then publishes and retains the exact outer initializer-failure policy and
clears Secondary -> Tertiary -> Primary. Both paths then re-read the post-task
angles and publish the direct F70 basis under the pre-A800 `0x2F` latch before
parking ahead of E100; Main Base removal fail-closes while that root custody is
live.

For selected Run Away's weighted class-10 result, the exact retail chain is
`FUN_0040C690 -> FUN_0040AC60 -> FUN_00425680 -> FUN_00438340 ->
FUN_0040ABB0 -> FUN_0040C6B0`; demo maps those bodies to `FUN_0040C6A0`,
`FUN_0040AC70`, `FUN_00425550`, `FUN_00437D80`, `FUN_0040ABC0`, and
`FUN_0040C6C0`. Retail `FUN_0040B6C0` and demo `FUN_0040B6D0` have the same
two-phase structure, with the exact shared `FUN_00406030/00406070/0040A7A0`
bodies mapped to demo `FUN_004060A0/004060E0/0040A7B0`. The TypeDefault
replacement selects descriptor/style `0x004C88E0/0x004C7618` at index zero,
preserves predecessor context `+0x08/+0x0C`, and leaves `initial_behavior` as
birth provenance. B6C0 then copies only the type-authored common-axis `+0x04`
word and clears Tertiary. Phase 0 lazily allocates the Secondary acquisition
task through `FUN_00402050/00402080`; only allocation success runs the
one-word Sub-A-only suffix while the old Secondary remains installed, then
publishes the new Secondary. Phase 1 does the same for the fixed-500-ms Primary
retarget through `FUN_00402B10/00402BA0`, again publishing only after its
suffix. Each suffix sets direction `+1` and speed
`base + (((low16 >> 8) * base) / 0xA00)` from signed base 250, so full success
consumes exactly two constructor words and retains the second Sub-A result.

A phase-0 allocation failure consumes no constructor word; context, axis, and
the Tertiary clear remain while the old Secondary/Primary survive until the
outer fallback. A phase-1 failure retains the first publication, Sub-A result,
and constructor word while the old Primary survives until that fallback. Both
nonzero phase results immediately publish fallback descriptor/style
`0x004C8888/0x004C74F8`, apply policy `0x1280` (including clear mask
`0x00068000`), then `FUN_0040C4D0` clears Secondary -> Tertiary -> Primary
without rollback. Any pre-mutation application block returns the same retained
selector plan, transition-time actor common-axis value, and exact Sub-A
snapshot; a known axis cannot silently change, while an unresolved axis may
resolve before retry. After replacement begins, allocation
success or initializer fallback is terminal and cannot retry or redraw. Exact
acquiring-graph success resumes A800 after Primary and freshly reads Secondary
then Tertiary; an eligible acquisition may replace the intermediate graph with
fleeing in that same pass, while the newborn Primary is not revisited. That
handoff consumes its own success-only constructor word, applies the generic
Sub-A suffix, then performs Run Away's fixed `5/3` speed overwrite. The
authenticated final acquiring/fleeing graph and exact fallback custody then
continue through the receipt-bound post-task basis/F70 publication and park
before E100. Neither
path reads or clears `+0xB2`.

Selected Go-To-Job and selected Wander now adopt that same weighted class-10
transaction across producer boundaries. Go-To-Job admission requires its
canonical class-54 TypeDefault context, exact lone Go-To-Job Primary, and empty
Secondary/Tertiary. Wander admission requires the canonical class-6
TypeDefault descriptor/style, `WanderNearPublished`, its lone
`OrdinaryType9Wander` Primary, and empty Secondary/Tertiary while authenticating
and preserving the exact retained context `+0x08/+0x0C` values rather than
assuming fresh null/zero. Each owner binds the no-redraw plan to frozen entity/
target snapshots, actor-local common axis, exact Sub-A, and post-Primary cursor
before preserving the two lazy allocations, success-only suffix words,
phase-specific prefix, and terminal fallback described above. Both terminal
results resume Secondary without revisiting the newborn Primary; successful
acquisition may synchronously publish Fleeing through its own constructor word
and suffix. Final acquiring/fleeing or fallback custody is authenticated before
F70 and PostBasisTail. Matched retail/demo C, original Type-9 data, and the
accepted ledger close both boundaries without a capture; another broad task or
weighted-root trace remains forbidden.

Selected Go-To-Job's weighted class-6 Wander crossing is closed by the
production path present since `3d487267a` plus its allocation-failure
regression. It authenticates the canonical class-54 TypeDefault context, lone
Go-To-Job Primary, and empty Secondary/Tertiary, then carries the retained
no-redraw plan and frozen snapshots into the exact class-6 transaction. Both
terminal results resume at Secondary without revisiting the newborn Primary;
allocation fallback consumes zero constructor RNG. Final graph authentication,
F70, and PostBasisTail remain under linear custody. Matched retail/demo C and
the accepted ledger close this boundary without a capture.

Selected Go-To-Job's weighted class-54 result follows the same exact retail
outer chain `FUN_0040C690 -> FUN_0040AC60 -> FUN_00425680 -> FUN_00438340 ->
FUN_0040ABB0 -> FUN_0040C6B0`; demo maps those bodies to `FUN_0040C6A0`,
`FUN_0040AC70`, `FUN_00425550`, `FUN_00437D80`, `FUN_0040ABC0`, and
`FUN_0040C6C0`. Its selected initializer is retail `FUN_0040AF90` / demo
`FUN_0040AFA0`, its nearest-job scan is retail `FUN_004235F0` / demo
`FUN_004234C0`, and its shared constructor suffix is retail
`FUN_00406030/FUN_00406070` / demo `FUN_004060A0/FUN_004060E0`. Production
retains the selector's frozen manager-order snapshot and same-index capacity
refinement, uses the authenticated actor-local common-axis range, and preserves
the full axis while a pre-mutation retry also binds its exact Sub-A snapshot and
post-Primary cursor. The target scan precedes Secondary -> Tertiary -> fallible
Primary. Allocation success alone consumes one constructor word, runs the
generic suffix with direction `+1`, applies the fixed signed
`(250 * 4) / 3 = 333` overwrite, and publishes Primary. Failure consumes no
constructor word before the terminal outer fallback clears Secondary ->
Tertiary -> Primary without rollback. Either terminal publication resumes the
real dispatcher at Secondary without revisiting the newborn Primary,
authenticates the final graph, reaches F70, and makes progressed Main Base
transfer fail closed. Matched retail/demo C and the accepted capture ledger
close this boundary without a new capture.

Selected Attract Attention now carries that weighted class-54 transaction from
three exact predecessor receipts. SharedRetarget starts in the class-45 initial
Primary and resumes at Secondary; expired Cue starts in Tertiary and has a
terminal cursor. Both retain the parity-authored local-Wander Primary, optional
Candidate Secondary, Cue Tertiary graph, canonical style-0 context with exact
target/auxiliary words, and `AttractAttentionPublished`. Target Route starts in
its lone Primary, resumes at Secondary with S/T empty, and authenticates the
canonical style-1 context plus `AttractAttentionTargetRoutePublished`. Each
origin binds the no-redraw plan, frozen actor/target snapshots, actor-local
common axis, Sub-A/animation, and ordered candidate/capacity evidence before
the nearest-job scan and Secondary -> Tertiary -> fallible Primary transaction.
Allocation success alone consumes one constructor word; fallback consumes zero.
Every terminal result follows its retained cursor without revisiting the
newborn Primary, authenticates the final graph, and completes F70/PostBasisTail.
Matched retail/demo C, original Type-9 data, and the accepted ledger close all
three origins without a capture.

Selected Wander's weighted class-45 result follows those same exact retail/demo
outer selector chains and the original Type-9 data's Player x3 -> class-45
choice. Admission authenticates the canonical class-6 TypeDefault descriptor/
style, `WanderNearPublished`, its lone `OrdinaryType9Wander` Primary, and empty
Secondary/Tertiary while authenticating this production's exact `Known(None)` /
`Known(0)` context `+0x08/+0x0C`. The reusable `from_wander` root helper
preserves arbitrary resolved words; this production crossing does not admit
them. Before mutation it binds the frozen entity and target snapshots,
actor-local common axis, exact Sub-A and animation, and post-Primary cursor.
The retained plan is consumed without a redraw through the exact class-45
parity transaction: event `0x10` / resource `0xF0`, sound 72 / forced stop,
success-only constructor suffixes, and committed-prefix terminal fallback.
Both terminal results resume at Secondary without revisiting the newborn
Primary, authenticate the final graph, and complete F70/PostBasisTail. Only
successful class-45 publication runs Candidate/Cue in the resumed suffix and
may synchronously publish Target Route. The newborn Cue can age by at most
125 ms there, so strict `elapsed > 1000` cannot expire in the publication
visit. Selected Wander now completes the shared E100/DF70/E370/`+0xB2`
clear/master-motion tail before returning; the next production visit consumes
that completed custody once. The following visit re-enters the published
Attract graph: SharedRetarget uses the Attract forced-stop animation policy.
An expired SharedRetarget Primary applies a TypeDefault root from the Attract
predecessor and resumes at Secondary without revisiting the newborn Primary,
so an even Cue ages by the same callback delta. An expired Cue applies that
root terminally and does not visit the newborn graph. Attract-origin deferred
retry now parks without a Wander-shaped request, keeps the Cue/SharedRetarget
cursor, and continues through `from_attract` apply. A planless Cue-origin
retry re-plans after the blocking input is restored and does not visit the
newborn graph. A planless SharedRetarget-origin retry re-plans the same way,
resumes at Secondary, and ages the newborn Cue. An after-plan Cue-origin
class-10 retry parks authored-audio preflight with the frozen plan and no
Wander-shaped request, then applies `from_attract` without a selector redraw
and without the Fleeing suffix. A
SharedRetarget-origin after-plan class-10 retry keeps the Secondary cursor
and publishes Fleeing in the resumed suffix. After odd Candidate publishes
Target Route and the shared tail completes, the next Wander visit runs that
lone Primary through the selected mover; a Continue frame after the
route's elapsed crosses 5000 ms parks the unsuppressed `LifetimeExpired`
request as `TargetRouteTransitionPending` without applying a root.
Introducing state bit `0x1000` after that completed outer tail is an
observation mismatch and drops the owner; Attract can hold `0x1000` from
publication. A mismatched or unresolved actor-local common-axis
descriptor consumes the scheduler prefix and route lifetime without calling
the selected mover; post-entry adapter failure parks `CallbackFailurePending`
and cannot replay. An unresolved selected-component immutable anchor is a
production gate after that same prefix: the visitor parks
`CallbackFailurePending` / `ImmutableAnchorUnavailable` without taking the
route owner or entering Target Route preflight, so
`Preflight(ImmutableAnchorUnavailable)` is not reachable from this
production visitor. Route lifetime stays 0. The next visit is the parked
`CallbackFailurePending` replay and does not commit another prefix. A
Primary wrapper that is not `{alive, !in_callback}` fails
`route.validate()` and drops `RootPublicationMismatch` before the
scheduler prefix. That same wrapper check lives inside Target Route
publication, so `Preflight(PublicationMismatch)` and
`Preflight(TaskWrapperNotRunnable)` are not reachable from this
production visitor. `Preflight(MetadataContractMismatch)` is not a
reachable Target Route production outcome: the visitor always supplies
`type_runtime_metadata(9)` or parks `RuntimeMetadataUnavailable`, and
publication does not re-type the actor. Reaching that preflight requires
forging `entity_type` or replacing the type-9 metadata row. An unresolved Sub-A runtime is a mutation-free Target Route preflight: the
production visitor has already committed the outer scheduler prefix, but
the route owner is retained, route lifetime does not age, and the next
Active visit retries the same Primary. A still-known Sub-A runtime whose
target-speed dword is unresolved is the same preflight
(`SubATargetSpeedUnresolved`). An unresolved actor-animation runtime is
the same preflight (`ActorAnimationRuntimeUnavailable`). An unresolved
physical-body basis is the same preflight (`PhysicalBodyBasisUnavailable`);
the published F70 tail must be consumed first, because that Complete
observation requires a Known basis. An unresolved target-state word is the
same consumed target-validation failure. A missing, dying, or inactive target tags
InvalidTarget before the predicate or selected mover; the unsuppressed
tagged result parks `TargetRouteTransitionPending` without applying a root.
A live target outside the actor-local route range is the `ZeroPredicate`
tag; the selected mover still runs, then the unsuppressed result parks
`TargetRouteTransitionPending` without applying a root. The selected
mover's prelude-Zero returns are dying or `state_flags == 0`, already
InvalidTarget before the mover, so `NonZeroPredicateAndZeroMover` is not
a reachable Target Route production outcome.
Matched retail/demo C, original data, and the accepted ledger close this
cross-producer boundary without a capture.

Selected Wander can now carry a retained weighted class-54 plan across the
producer boundary into Go-To-Job. It preserves the selector's candidate and
capacity evidence, consumes no second selector word, follows exact Secondary ->
Tertiary -> fallible Primary or terminal fallback ordering, and completes F70
and PostBasisTail under the new owner. Selected Go-To-Job likewise carries its
retained weighted class-10 plan across the producer boundary into Run Away,
including the exact predecessor, frozen snapshots, two-phase success/fallback,
post-Primary continuation, synchronous fleeing handoff on success, final
authentication, and F70/PostBasisTail. Selected Wander now carries the same
class-10 transaction from its canonical class-6 descriptor/style and exact
retained context through the additional Fleeing suffix word when acquisition
succeeds. It also carries the retained weighted class-45 Attract Attention plan
through the exact parity transaction, original Secondary cursor, final graph,
and F70/PostBasisTail. Initial multi-slot Attract now
applies the same retained class-6/fallback and F70 boundary, then runs its
dedicated Target Route visit through the exact scheduler/predicate/common-mover/root and outer
tail on the following frame. Its matching selected-owner weighted class-45 root
is closed. Go-To-Job -> weighted class-6 Wander and Attract Attention ->
weighted class-54 Go-To-Job and class-10 Run Away are also closed across
producers. The class-10 crossing preserves SharedRetarget Primary -> Secondary,
expired Cue -> terminal, and Target Route Primary -> Secondary origins, the
frozen no-redraw plan/snapshots, exact two-phase allocation/RNG/fallback,
optional same-pass acquisition-to-Fleeing suffix and fixed `5/3` overwrite,
final authentication, F70/PostBasisTail, and fail-closed progressed Main Base
custody. Other cross-producer weighted roots remain static composition;
selected Go-To-Job -> weighted class-45 Attract Attention is admitted for both
the even transaction and the odd Candidate/Target Route suffix. After the
shared tail, the next Go-To-Job visit re-enters the published Attract graph:
SharedRetarget uses the Attract forced-stop animation policy. An expired
SharedRetarget Primary applies a TypeDefault root from the Attract
predecessor and resumes at Secondary, so an even Cue ages by the same
callback delta. An expired Cue applies that root terminally and does not
visit the newborn graph. Attract-origin deferred retry now parks without a
Go-To-Job-shaped request, keeps the Cue/SharedRetarget cursor, and continues
through `from_attract` apply. A planless Cue-origin retry re-plans after the
blocking input is restored and does not visit the newborn graph. A planless
SharedRetarget-origin retry re-plans the same way, resumes at Secondary, and
ages the newborn Cue. An after-plan Cue-origin class-10 retry parks
authored-audio preflight with the frozen plan and no Go-To-Job-shaped request,
then applies `from_attract` without a selector redraw and without the Fleeing
suffix. A SharedRetarget-origin after-plan class-10 retry keeps the Secondary
cursor and publishes Fleeing in the resumed suffix. After odd Candidate
publishes Target Route and the shared tail completes, the next Go-To-Job
visit runs that lone Primary through the selected mover; a Continue
frame after the route's elapsed crosses 5000 ms parks the unsuppressed
`LifetimeExpired` request as `TargetRouteTransitionPending` without applying
a root. Introducing state bit `0x1000` after that completed outer tail is
an observation mismatch and drops the owner; Attract can hold `0x1000`
from publication. A mismatched or unresolved actor-local common-axis descriptor
consumes the scheduler prefix and route lifetime without calling the selected
mover; post-entry adapter failure parks `CallbackFailurePending` and cannot
replay. An unresolved selected-component immutable anchor is a mutation-free
Target Route preflight (`ImmutableAnchorUnavailable`): Go-To-Job has no
production-level anchor gate, so the visitor takes the route owner and
enters preflight. The outer scheduler prefix is already committed, the
route owner is retained, route lifetime does not age, and the next Active
visit retries the same Primary. A Primary wrapper that is not
`{alive, !in_callback}` fails `route.validate()` and drops
`RootPublicationMismatch` before the scheduler prefix. That same wrapper
check lives inside Target Route publication, so
`Preflight(PublicationMismatch)` and `Preflight(TaskWrapperNotRunnable)`
are not reachable from this production visitor. `Preflight(MetadataContractMismatch)`
is not a reachable Target Route production outcome: the visitor always
supplies `type_runtime_metadata(9)` or parks `RuntimeMetadataUnavailable`,
and publication does not re-type the actor. Reaching that preflight
requires forging `entity_type` or replacing the type-9 metadata row. An unresolved Sub-A runtime is a mutation-free Target Route
preflight: the production visitor has already committed the outer scheduler
prefix, but the route owner is retained, route lifetime does not age, and
the next Active visit retries the same Primary. A still-known Sub-A
runtime whose target-speed dword is unresolved is the same preflight
(`SubATargetSpeedUnresolved`). An unresolved actor-animation runtime is
the same preflight (`ActorAnimationRuntimeUnavailable`). An unresolved
physical-body basis is the same preflight (`PhysicalBodyBasisUnavailable`);
the published F70 tail must be consumed first, because that Complete
observation requires a Known basis. An unresolved target-state
word is the same consumed target-validation failure. A missing, dying, or
inactive target tags
InvalidTarget before the predicate or selected mover; the unsuppressed
tagged result parks `TargetRouteTransitionPending` without applying a root.
A live target outside the actor-local route range is the `ZeroPredicate`
tag; the selected mover still runs, then the unsuppressed result parks
`TargetRouteTransitionPending` without applying a root. The selected
mover's prelude-Zero returns are dying or `state_flags == 0`, already
InvalidTarget before the mover, so `NonZeroPredicateAndZeroMover` is not
a reachable Target Route production outcome.
The published Target Route preflight and composition family is now closed
on Attract, Wander, and Go-To-Job. Run Away has no published Target Route
visitor. Specialized Wander/Go-To-Job now complete the shared F70→outer-tail
visit. Selected-owner E370 now consumes the
selected owner, adopts the inner class-14 SharedRetarget owner, and runs
the ordinary latched-`0x2F` E870 suffix. Later E370 visits apply
`FUN_00416750` then already-dying `FUN_00410C10` and continue. The
1,000-ms class-14 task reaches `FUN_0040C470`, stages shared deferred
destroy, still runs that suffix, and the manager sweep unlinks. That
path does not claim Main Base post-abort `0x00C64825`.
Session-zero `FUN_00456900(0xC6, 0)` is closed by `20260817-072421`. The fresh-birth
`+0xB2` policy and limits are documented under [current Type-9 task ownership](#current-type-9-task-ownership).
No new capture is justified.

### Intro2 trajectory variability

The [shared insect static-contact owner](INSECT_STATIC_CONTACT.md) retains
native Type16/26 construction and task custody alongside the existing
Type53/122 adapters. This closes the previously omitted Type16/26 building
pass in Intro2 and ordinary Type26 worlds. It preserves439 generic crushing,
Type26 Furniture's separate C890/C690 callback, the exact02CA0 task-only
retarget writes, and the current-model11760 physical tail. Living ground
insects intentionally lack terrain/water bit10000; an oriented model/terrain
intersection alone is not source permission to invoke141D0. Their actual
Class12 owners restore the permission and share the native surface pass.

Native [Type17](INTRO2_TYPE17.md) and [Type53](INTRO2_TYPE53.md) retain their
own births and Sub-D receipts while sharing the proven ABCDH phase and class9
pursuit callbacks. Their steering angle writes precede C/A/B; matrix rebuild
belongs to DCA0/E870 after all task visits. Both callback modes occur in the
accepted Intro2 Type17 evidence.

[Native Type58](INTRO2_TYPE58.md) retains its own A/B/C/D/E/H allocation while
sharing the proven Follow, Search/Aim and common12 phases. Class26 Trash
Furniture is shared with Type26: B7C0 clears Tertiary then Secondary, and
401D03 calls6030/6070 before publishing Primary. That successful suffix enables
H and consumes one A target-speed word; it is not a no-RNG constructor.
1DA0 scans using the live common axis, resolves previously unwritten target Y
on success, calls01430 even on a failed scan, and only then returns9C01.
Class4 Defecate Virus is a different B9E0 graph: its slot0 returns9C01 on zero
mover result, and its strict2000-ms timeout also selects style+00 C690 after
unwind. Both literal style4C7E88 masks+34/+38 are zero. Native Type58 class26
also owns bounded late C890 static damage, unconditional C690 and re-resolved
11760 response, interleaved with meteor contacts in intrusive actor order.
Full terrain/water/pair contact and other styles remain open;
the Type58 document retains validation limits and the source formulas. Native Type26
tasks do not depend on forcing its older captured class4 choice.

[Native Type94](INTRO2_TYPE94.md) owns its separate Follow/Search/Capture graph,
water Sub-C and method20 FIFO. Its own constructor-to-first-query receipt
authenticates Sub-D; equal Type58 descriptor bytes do not grant custody.
The zero E370 selectors decay the timer without a drowning lifecycle, including
through common12. Attached Capture and late active-pair contacts remain open.

Retail Intro2 actor paths are not a frame-identical script. Types 47, 26, and
13 run Guard Location, Defecate Virus, and Search And Attack Target through the
ordinary scheduler and the one process-global MSVC LCG at `0x004F7308`
(`state = state * 214013 + 2531011`, returning the high 16 bits). The PE owner
begins zero-initialized; observed run-to-run variation is therefore best
understood as history- and timing-sensitive interleaving of one deterministic
stream, not evidence for a separate time seed. Menu, effects, scheduler gates,
AI setup, and callbacks share the owner, and branch-dependent callbacks consume
different draw counts.

The closest-target helper itself consumes no RNG: it selects the strict nearest
eligible live candidate and preserves intrusive-list order for equal distances.
A dragon can nevertheless pursue a human in one run and a building in another
because earlier movement, task cadence, deaths, and list state have already
diverged by the time that deterministic search runs. The port must preserve one
process-lifetime RNG owner, conditional draw order/count, scheduler cadence,
candidate eligibility, and list/tie policy. It must not reset RNG at Intro2 or
per actor, bake one captured trajectory, or force a recorded dragon target.
Acceptance is based on authored spawns/models, valid task topology and targets,
retail timing/motion envelopes, live camera following, and the same functional
cinematic outcome—not frame-identical actor coordinates. Fixed-seed tests remain
appropriate for individual transaction and draw-count contracts.
