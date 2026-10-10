# V2000 Factory System

This document owns factory economy, native delivery, staffing, output selection,
Main Base systemic-abort ordering, and the reactor-adjacent paths formerly
embedded in [GAME_MECHANICS.md](GAME_MECHANICS.md). Current priorities and
capture requests remain in their dedicated objective and runtime-RE ledgers.

The [cross-level gameplay audit](CROSS_LEVEL_GAMEPLAY_RUNTIME.md) records the
later-world missing-context failure. The new
[authored Base/Factory authority](AUTHORED_BASE_FACTORY_RUNTIME.md) owns shared
class39/41 construction, task custody, actual templates/models, and remaining
worker limits. Persistent production/repair sound belongs to
[the factory audio authority](FACTORY_AUDIO.md).

Native staffing admits the actual Type7/8/79/90/91/116 worker profiles
through the shared `25850` capability-`0x400` callback. Contact, delivery,
descriptor suffix and deferred destruction retain each worker's own metadata
and completed allocation/task custody. The source's `02DA0` lookup uses the
worker's actual type record; it does not substitute the Type8 descriptor.
See [native delivery ownership](AUTHORED_BASE_FACTORY_RUNTIME.md#live-callbacks-and-production).
Main Base conversion authenticates native Type9/78/86/95/123 inputs and their
actual component slots. Dynamic Base/factory workers admit Type7/8/79/90/91/116
with their own profiles. Type7 retains its full four-choice root, capacity
predicate and query-free steering; all native worker profiles use shared wind/drag.
Type7 is a capability400 worker, never a capability800 Base input.

## Entity Types

| Type | Hex | Role |
|------|-----|------|
| 0x42 | 66 | **Factory** entity (has_config=True, 0x58-byte config block) |
| 0x06 | 6 | **Main Base** (`college` in Level 1), conversion/beam-in target |
| 0x2E | 46 | Player ship entity |
| 0x43 | 67 | Level-1 **Hive**, not the Main Base |
| 0x3D | 61 | Level-1 Power Up/contact-inventory callback family |
| 0x5D | 93 | Cargo-drop Materialiser proxy, not a factory timeout effect |

Two more Section-12 rows carry the same class-39 Working Factory shape (M
component, Always1/class39, alternate0, capability `0x84`, health99999):

| Type | Model slots | Initializer state | Ordinary births | Port |
|---|---|---|---|---|
| 66 | `210,225,210,225` | `0x25027` | 81, 28 worlds | native |
| 125 | `231,225,231,225` | `0x25027` | 4, worlds 37 and 40 | native, same owner |
| 82 | `210,210,210,210` | `0x21005` | 4, 4 worlds | no owner yet |

Type125 differs from Type66 only in its models (its authored spawns also
override slots 1/3 with model144), so it takes the same construction, task,
production, damage, progressive-death and Main Base abort paths; the port's
`is_working_factory_type` replaces the former type-66 checks. Type82's
initializer state lacks `0x4000`, `0x20` (the D4A0 type-default terrain snap)
and `0x2`, while the factory owner's live tick assumes `0x25027`; it stays an
explicit unsupported type until those bits' effects are recovered.

Factory appears in 29 levels. Section 8 models include `factory2`–`factory9`,
`factory2lift`, `factory3engine`, and `factory4door`. The older type-67 base
classification was disproven by the retained Level-1 entity list and pair
callbacks.

Intro2's native Type66 hut and factory use their own authored allocations,
Working Factory task, checked damage and progressive-death publishers. Their
distinct Sub-M templates and the hut's terrain/radar/grounded-pose destruction
tail are owned by [INTRO2_TYPE66.md](INTRO2_TYPE66.md); they do not reuse a
captured Level1 receipt or a timed presentation-model switch.

## Level-1 Type-61 Payload and Bounded Player Contact

[Native authored Type61 construction](TYPE61_POWER_UP.md) now retains each
ordinary world's pickup allocation and shared class49 death independently
from the captured Level-1 and factory-birth adapters described below.

`FUN_004104B0` copies a type-61 Power Up's Section-13 spawn dword `+0x1C` to
live entity `+0x88`. The three Level-1 instances at authored spawn indices
32/33/34 carry `0x0000003C` (Targetter), `0x0000003F` (trophy), and
`0x0000C802` (weapon selector 2, amount 200). `FUN_00425AF0` passes the low-byte
selector and the signed arithmetic dword shift by eight to the recipient's
inventory handler.

The port now integrates only the proven player/type-61 suffix of the active-
pair path. It visits candidates in retail live-list order, uses oriented
Section-8 collision and signed-8.8 toroidal deltas, stages inventory,
Targetter/Turbo capabilities, hull, and campaign progress transactionally, and
queues accepted ids
without mutating the list during the scan. Pending ids cannot be acquired again
in the same frame; the next `EntityManager::update` splices them while retaining
survivor order. The ordered result keeps accepted and rejected feedback in the
same live-list order. Selector 2 derives event 6/7 and direct-text arguments
from the installed descriptor; selector `0x3C` owns its first/duplicate sound
rates and acquisition messages; selector `0x3E` owns its first/duplicate
capability, sound, direct-text, persistence, and VTOL-boost contracts; selector
`0x3F` owns the exact zero-amount
hull/buffer restore, CLAIMED bit, wrapping trophy/extra-life counters, messages,
and sound 50 at fixed `1x`, or fixed `2x` on every fifth count. Level-1 overlay 13 is
campaign/control slot 1 because the
world loader resolves `overlay = slot + 12`; the trophy claim therefore updates
controller `+0xDC`. Already-claimed trophies still restore, sound, consume, and
do not count again.

This does not enable the general `FUN_00411AD0` solver, unrelated
behavior/component callbacks, physical pair response/damage, multiplayer
transport, or cross-save persistence of the 37 control words and counters.

## Factory State Machine (`entity_state_block + 0xB0`)

| State | Name | Behavior |
|-------|------|----------|
| 0 | PRODUCING | Timer at `+0x60` accumulates. When ≥ threshold `+0x08`: spawn powerup, → state 1 |
| 1 | DELIVERING | Timer at `+100` accumulates. When ≥ duration `+0x0C`: → state 2 |
| 2 | WAITING | If stock is nonzero, query the tracked pickup; once it is absent/dying, reset the timers and enter state 3. If stock is zero and staffing is zero, clear progress and enter state 4. Otherwise wait past the strict `0x3567E1` conversion gate, then attempt one worker/materialiser ejection and remain in state 2 for the next update. |
| 3 | COOLDOWN | Timer at `+0x5C` counts down. When 0 → state 0. Initial value from OVL: `config_0x5C * 1000 + 500` µs |
| 4 | IDLE_EMPTY | No natives. If `+0x68` (native count) becomes nonzero → state 2 |

## Production Speed

```c
if (capacity == 0) {
    rate = time_delta * base_speed;           // autonomous, no natives needed
} else if (native_count > 0) {
    rate = time_delta * base_speed * native_count;  // more natives = faster
} else {
    rate = 0;  // no natives = stopped
}
```

**Key fields in state block:**

| Offset | Field |
|--------|-------|
| +0x04 | Capacity (max natives) |
| +0x08 | Production threshold |
| +0x14 | Production timer limit (µs) |
| +0x58 | Remaining stock count |
| +0x60 | Production timer accumulator |
| +0x68 | Current native count |
| +0x6C | Production countdown |
| +0x7C | Production rate accumulator |
| +0x80 | Base production speed |
| +0x88 | Spawned powerup entity handle |
| +0x94 | Init sentinel (-1 = uninitialized) |
| +0xB0 | State (0-4) |

## Native Delivery Flow

1. Player type 46 beams a permitted native into its attachment list with C.
2. After release near Main Base type 6, the peasant's authored Base Nearby
   rule strongly favors Go To Job. Its ordinary mover walks toward the Base
   until it reaches the active-pair volume; the player need not drop it
   directly inside that volume.
3. Main Base callback `FUN_004258A0` accepts non-remote capability `0x800`,
   emits resource event 1, queues deferred destruction of the source, maps the
   live conversion selector to a replacement type, and attempts to spawn that
   replacement. Only spawn success installs the base relation and operation
   `0x33`; failure preserves the earlier event/destruction and returns null.
4. Working Factory/lifter callback `FUN_00425850` capacity-checks an arriving
   replacement, emits its authored effect, increments occupancy, and deferred-
   destroys the accepted actor.
5. `FUN_00419010` advances the factory production state and publishes the live
   scientist/light channels described below.

The release selector uses the same
[authored Type-9 weights](ACTOR_RUNTIME.md#type-authored-initial-behavior-selection-confirmed)
as subsequent task reselections. Base Nearby uses a strict wrapped range of
15 terrain cells on each axis, including height. The choice remains weighted,
so a brief alternate task is possible; it does not justify forcing conversion
or skipping the walk. The class-54 target scan below accepts Main Base without
consulting factory staffing capacity.

The Main Base walker preserves `FUN_00411AD0`'s contact-before-callback order:
active-pair eligibility and the oriented Section-8 model probe run before
requiring the candidate's behavior/component callback metadata. An unrelated
distant peasant with unresolved callback state is a missed pair, so it cannot
prevent a later live-list candidate from converting. A contacting unresolved
callback still stops before event 1 or deferred destruction. The normal-tier
`peasant_main_base_contact` regression exercises both cases with the authored
models, including the Type-8 replacement's recent Base relation and same-pass
tail visit. That `+0x60` relation is distinct from cargo attachment at `+0x80`;
the scientist remains free to walk.

The accepted Working Factory delivery branch is closed as a generic
receipt-bound transaction. A complete factory/scientist identity-and-version
snapshot is authenticated against the shared lifter callback planner before
the first side effect escapes. It then preserves retail's non-rollback order:
controller operation `0x33` at the scientist position, deduplicated HUD
resource event 5,
`FUN_00418C20(factory, 1)`, optional capacity countdown/text `0xCE`/HUD event
2, optional remote-scientist feedback `(1,8)`, deferred scientist destruction,
and the tagged `0xA300` callback result. Staffing commits only after its exact
receipt is acknowledged; a later unavailable presentation or destruction
boundary retains that earlier mutation. Receipts bind the caller-owned nonzero
transaction ID, both allocation identities and state versions, and the action
sequence, so stale or cross-pair completions cannot advance another delivery.

The `0xA300` result clears only the physical response in the surrounding
active-pair interpretation. The detached delivery machine still reports that
pair suffix as unclaimed. The live coordinator now owns the remaining
`FUN_00411AD0` visit after a successful Level-1 apply: class-54 style `+0x18`
is null, so candidate behavior is skipped; spawn-23 type 66 is component-free;
`FUN_00403650` writes scientist component-slot-0 `FUN_00402DA0` (slots 1/2
already cleared), and the type-8 Sub-I branch consumes no RNG; physical
response stays omitted because `0xA300` already cleared it. Queued destruction
does not stop that component visit. The specialized scheduler still owns the
one-shot apply when a named scientist is queued; proximity is not used. This
transaction also does not invent the scientist's route to the Factory; the
shared Go-To-Job/common-mover runtime remains the owner of travel. A bounded
live adapter now consumes that receipt for the exact fresh-New-Game Level-1
spawn 23/type-66/model-227/class-39 factory and a live Main-Base-born type-8
scientist whose Primary Go-To-Job task still targets it. It drives real
`EntityManager`, `WorldFx`, and `GameplayNotifications` state through
operation `0x33`, resource events 5/2, staffing, manager-owned deferred
removal, and the remaining pair visit. Remote delivery stays unadmitted.

The deterministic production half has two deliberately separate Rust
contracts. The pure observational replay retains all 22 dwords of the 0x58-byte
Section-13 template, reproduces `FUN_00418A90`'s health-derived initialization,
`FUN_00418C20`'s staffing clamp/countdown, and rolls an unresolved observed
frame back in full. Its returned trace remains non-executable.

The complementary full-health live frame is a detached, receipt-bound
transaction for phases 0--4 of `FUN_00419010`. It owns the exact zero-filled
0x4C-byte requests: allocation handle zero, type at `+0x08`, signed raw-8.8
position at `+0x0C/+0x0E/+0x10`, and only the pickup payload at `+0x20`.
Phase 2 copies X/Y and subtracts `0xFA` (250) raw units from coordinate index
2/Z: the fresh Level-1 factory `[0x5700,0xFD00,0x3A00]` issues
`[0x5700,0xFD00,0x3906]`. Type-8 construction terrain-snaps Y from the displaced
X/Z and retains `0xFD00`; type-93 uses the same terrain base and publishes its
proxy at `0xFD01`. Positional sound 8 keeps the original request position
`[0x5700,0xFD00,0x3906]`.
The working loop is a separate Sub-M voice configured by words20/21.
`18A90 ->44C830` allocates it; `18F60 ->44C8E0` selects1.0x operation or1.5x
repair without restarting an audible voice. The source matrix, constructor
emitter position, accepted sound31/44 observations and shared mixer lifetime
are retained in [FACTORY_AUDIO.md](FACTORY_AUDIO.md).
It preserves finite/unlimited stock, type-61 pickup spawn/presence, delivery,
cooldown, the strict signed `0x3567E1` conversion gate without a same-frame
recheck, the exact world-style output selector, type-93 materialiser link,
positional sound 8, and all non-rollback failure prefixes. In particular, phase
0 commits the threshold clamp, clears entity lifetime `+0x74`, and only then enters phase
1; a failed materialiser spawn retains the preceding scientist/capacity
decrements. Animation precedes status publication, and status precedes the
phase-0/1/3 dirty write.

Actions carry a private linear receipt bound to a caller transaction ID,
factory allocation identity/admission-version lease, and monotonic sequence.
Every issued action also carries the complete production/animation state that
retail mutated before that call; adapters must validate the lease and persist
this snapshot before executing the action. Protocol rejection returns the
receipt without advancing. A terminal external block guarantees that the
pending action performed no side effect and reports the durable
production/presentation prefix (or its recovery snapshot if that state write
was itself unavailable), so callers must retain it rather than replaying the
frame. A child lookup miss during an owner-link action is an acknowledged
no-op, and the ignored return from `FUN_00408F00` still advances to positional
sound 8, matching retail and demo.

`v2k-game::factory_production_owner` now closes the complete outer
`FUN_00419010` update as a second receipt-bound machine around that production
child. It carries the entry write `entity[+0x84] |= 4` in every durable state,
then preserves retail's branch priority: progressive death first; cached-health
reconciliation before damage handling; understaffed countdown/death; damaged
repair; otherwise full-health production. Reconciliation includes the observed
health-loss accumulator at `+0x70`, strict loss threshold, and retail's
apparent double scientist decrement. Above-capacity staffing is not rejected:
the exact countdown, direct-text, and HUD-resource sequence still runs. Repair
clamps staffing at capacity and publishes the corresponding countdown/text/HUD
sequence before the common animation/status tail.

The owner authenticates distinct caller-provided parent and child transaction
IDs and projects every child action through its own linear receipt. It copies
the child's complete detached state after every poll and resume, including
blocked prefixes and the exceptional outer state-commit failure, so a live
adapter cannot replay a partially completed production frame. Progressive
pickup destruction uses retail's deferred helper semantics: a zero or missing
handle is an acknowledged no-op. Terminal progressive mutations, animation,
status, and dirty publication remain in retail order.

Matched full/demo C fixes two otherwise easy-to-collapse terminal boundaries.
When one update crosses stages 31 and 32, runtime `+0x94` first holds the
positive `old + dt` value while the stage-31 selector-`0x10000` model effect is
synchronous; stage 32 writes `-1` only after that action returns and immediately
before pickup destruction. The later `InvokeProgressiveFactoryDeath` action is
issued while live health still contains `FUN_00419750`'s revived 10,000,000;
only acknowledgement of that external callback commits health zero, so the
action pre-state remains revived and the completion is dead. The detached
`FactoryProductionOwnerMachine` now preserves both contracts, covered by all 13
focused `factory_production_owner` integration tests. This static closure needs
no Type-66 capture.

`v2k-game::main_base_type66_production` now supplies the formerly missing
allocation-authenticated progressive owner for exact fresh-Level-1 spawn 23.
Its non-copyable custody binds the published Working Factory Primary task and
stable live-factory allocation while reading the current mutable factory
version on every frame. Admission authenticates the complete type/context,
model-slot, Sub-M/production, task-wrapper, sequence, effect-walk, destroyed
model, zero progressive config byte, live `+0x34` last-hit tick, and Power-Up
deferred-state contracts before changing wrapper elapsed, notification state,
progressive time, live state, or shared RNG.
Missing or malformed resources retain the same owner for retry; stale actor,
task, allocation, or sequence authority is consumed without executing a frame.

The callback preserves `FUN_00401120 -> FUN_00425C60 -> FUN_00419010` order in
the same Primary wrapper, including direct text `0xD3` (`Factory under attack`)
before the callback-private latch. Every accepted opcode-`0x8E` point consumes its
branch-local gate word (except threshold `0x10000`) followed immediately by the
18-word common explosion bundle. A crossed stage 31/32 frame publishes the
positive clock during the effect, writes `-1` before the optional Power-Up
defer, then enters generic death with revived health before selecting model
225, queuing fixed-rate sound 62, clearing staffing/capacity, resetting both
animation ranges, publishing zero status, and requesting the enclosing
presentation. Zero and missing pickup handles are no-ops; a clean type-61 is
queued once; an already-consistent pending entry is idempotent; unresolved or
inconsistent live state blocks before the wrapper. Terminal completion unwinds
but does not replace the surviving Primary and issues no successor authority.
Ten focused owner tests and the full `v2k-game` library suite cover these
boundaries. The owner is now the fifth detached specialized family. The
coordinator samples the live last-hit tick at the Type-66 visit, routes direct
`0xD3` through caller-owned gameplay notifications, consumes the concrete
shared `WorldFx`, and returns staged explosion lights without replay. Live
whole-sweep attachment remains engineering work, not evidence or another
capture request.

The real Level-1 fixture is payload `0x0001F412` (weapon selector `0x12`,
amount 500). `FUN_00445A90` finds that selector in `DAT_004CDC08` record 20
and installs the 24-byte master into the first empty inventory slot. Its
flag `0x01` grants infinite ammunition despite that stored amount; the live
40-ms firing, larger gun pair and class-2 damage path are described in
[PLAYER_CRAFT.md](PLAYER_CRAFT.md#fire-both-modes). The type-61 model 82
uses the payload's low five bits to select `powerup18`; its presentation clock
and sound-44 loop remain owned by the pickup, while type-93 materializers
belong to the separately ejected workers. The
0x4C spawn request is still the factory origin (pickup offset `[0,0,0]`);
capture `20260731-001224` is `[0x5700, 0xFD00, 0x3A00]`. That origin is the
`lifter` model origin, inside the building. The visible/contact site is updated later by the draw-owned
`0A9F0 ->198E0 ->199B0` Sub-M selector-zero callback. It resolves descriptor
word `+00` (slot8 for Type66) in the current nested model, returns that marker
as tf14 geometry, then writes tracked product `+88` with signed descriptor
world offsets and state bit20. The Level1 fork marker includes `[0,78,0]`
relative to its child origin. The same rule supports Level2 and the other
authored factory models; neither the name `lifterarmfork` nor product recent
relation `+60` selects its authority. `19010` keeps the birth request and
does not call cargo attachment `08F00`. See
[FACTORY_PRODUCT_MARKERS.md](FACTORY_PRODUCT_MARKERS.md) for machine-code
evidence, the model census, draw ownership, checks and remaining boundaries. Capacity 2, production
6,000,000 us, delivery 5,000,000 us, cooldown -1000, Section-12 health 99,999,
and one initial finite stock item. `factory_status_runtime` now closes the pure
six-word `FUN_00419630` projection into the retained Sub-M word bank. Its third
word is the zero-extended byte at runtime `+0x00` (`0x12` for Level 1's packed
descriptor `0x0001F412`), not the descriptor's low word. The projection
preserves the authored selector descriptor and independent progressive-death
clock. The exact Level-1 bridge owns live receipt, allocation, and
mutable-version authentication for its bounded prefix.

The live entity constructor now retains that complete deterministic
`FactoryProductionRuntime` inside the type-66 Sub-M state when, and only when,
both the Section-13 0x58-byte template and Section-12 initial health are known.
The real Level-1 fixture pins payload `0x0001F412`, capacity 2, the
6,000,000/5,000,000-microsecond manufacture/delivery intervals, cooldown
-1000, stock 1, cached health 99,999, health-per-scientist 49,999, repair rate
833, and phase 0. Type 6 and incomplete constructor evidence retain `None`.
Status projection and progressive death preserve this state byte-for-byte.
Generic factory identities retain construction-state custody only. The exact
fresh-Level-1 identity additionally owns a persistent allocation/version,
animation, lifetime, and entry-flag sidecar. Its explicit-arrival bridge runs
the full-health phase-0 owner through the first product, the complete
five-second phase-1 timer, the 176-frame finite-stock phase-2 wait, both
one-per-update class-1 worker ejections, and the following phase-4 transition.
It persists every pre-action snapshot before executing the corresponding live
list, spawn, owner-link, materialiser attachment, positional-sound, status,
animation, dirty-state, or HUD action. Damaged-repair and cached-health
frames now use the same live owner as full-health production.
Progressive-death frames, understaffed death, and the
nonzero-multiplayer phase-1 presentation branch remain outside this bridge.
Positive-stock pickup presence now queries the live spawned handle with
retail `FUN_0043a580`: a miss, dying bit `0x4000`, or zero `+0x08` starts
cooldown; an active crate stays in phase 2. Authored Level-1 cooldown is
still negative, so the first product remains finite unless remaining stock
is still positive. The unclaimed legacy progressive-death pass now
queues the tracked type-61 through `FUN_00410B70` at stage 32 and then runs
the terminal `FUN_00419750` suffix: production `+0x68/+0x04` clear, both
animation channels `FUN_0044C8E0(0,0,0x10000)`, and `FUN_00419630` Sub-M
republication. After that stage walk, `FUN_00419B50` still calls
`FUN_00418F60(..., 0)` whenever `+0xB4` is nonzero, so a staffed or
delivery pose idles on the first dying frame. Stage 32 with `param_3 == 0`
requests the shared session-`+0x295` flash through `FUN_00456750`. A zero
or missing pickup handle is still a no-op.

The focused activation transcript
`runtime_re/captures/local/20260731-001224-second-scientist-factory-activation-v2.txt`
closes that fixture's retail handoff and first product. Main Base source
`0x04AC0001` became type-8 scientist `0x04940001` at tick `0xBF5`; the same
scientist entered factory `0x04A40001` at tick `0xC7E`, committing staffing
`1 -> 2`. Two ticks later both pole sinks published current/required `2/2`.
Exactly 300 ticks (6.00 seconds) after intake, the factory spawned type-61
pickup `0x04930001` with payload `0x0001F412`, decremented stock `1 -> 0`, and
entered delivery. Exactly 250 more ticks (5.00 seconds) later, delivery ended
and phase 2 began.

`tests/factory_activation_capture.rs` remains the detached 300/250-tick and
finite-stock policy oracle. `tests/factory_activation_live.rs` loads the real
Level-1 corpus and Main Base replacement allocator, injects only the explicit
accepted-arrival position, authenticates the retained Go-To-Job target,
delivers two scientists, publishes `1/2` then `2/2`, rejects a stale parallel
preflight, and after 300 20-ms frames tail-appends and owner-links one real
type-61 product. It then drives the captured 250-frame phase-1 interval,
proves the exact 5,000,000-us clamp, animation `3 -> 1`, status/dirty tail, and
phase-2 entry. It advances exactly 176 pre-conversion 20-ms timer/status frames
from the production threshold to threshold plus 3,520,000 us, without
animation, dirty-flag writes, RNG, entity-list allocation/linkage, or conversion
actions. Frame 177 then performs the first exact transaction: HUD event `0x16`,
class-1 type-8 birth and factory relation, staffing/capacity decrement, type-93
birth and attachment, and positional sound 8. The next update performs the
second transaction and the following update enters phase 4 with zero progress.
It deliberately does not simulate scientist locomotion or dispatch the
nonzero-multiplayer phase-1 presentation branch. Live pickup presence,
cooldown, and repeat manufacture now use the spawned-handle lookup.

Stock policy is authored through the Section-13 cooldown dword at `+0x10`, not
through weapon or ammunition semantics. `FUN_00418A90` initializes one finite
item when that signed value is negative and `INT_MAX` stock otherwise.
Nonnegative-cooldown factories wait for the live pickup to disappear, count
down, and manufacture again. A finite factory instead remains in phase 2 after
stock reaches zero. Once production progress exceeds its threshold by
`0x3567E0` microseconds (the comparison is strict), it queues HUD resource
`0x16`, spawns the world-style-selected worker, owner-links it to the
factory, decrements both current staffing and capacity, then spawns and attaches
a type-93 materialiser and plays positional sound 8 at full gain and fixed
1.0 rate. The phase and over-threshold timer remain in place, so additional
workers are ejected on subsequent
50-Hz updates until staffing reaches zero; only then does the factory enter
phase 4. The focused transcript stopped before this branch, but matched
retail/demo C closes it exactly and the bounded fresh-Level-1 bridge now owns
both class-1 type-8/type-93 transactions and the phase-4 transition. Their
birth, selected constructor/task, publication, relations, decrements, and sound
are therefore closed without a focused ejection capture. A successful type-8
native birth consumes the Sub-A20450 word before its behavior-selector and
constructor-suffix words; class6 initially installs Wander Near, and
type-default bit0x20 terrain-snaps final Y through D4A0. The following08F00
attachment invokes DBF0/CD50/CD70 and replaces that task with the carrying
None Primary. Native materialisation, release/reselection and later motion
are owned by [Type8](INTRO2_TYPE8.md#carrying-and-materialiser-release), with
the process Sub-D allocation policy kept separate from captured heap history.

The exact active conversion selector/replacement is now closed by
`runtime_re/captures/local/20260724-040316-base-conversion-selector.txt`.
During a real type-9 peasant delivery, the post-call hook at `0x425914`
returned type 8 for world style 1 without changing the sampled shared RNG
endpoint. The callback copied raw position `[0x50BA, 0xFD02, 0x39EC]`, spawned
replacement handle `0x04970001`, linked it to Main Base handle `0x04B50001`,
and emitted the operation-`0x33` path. Do not restore the older direct type-67
delivery interpretation.

This conversion cannot be inserted as a proximity special case. In
`FUN_00411AD0`, the Main Base behavior is only one stage in an ordered pair
visit: subject behavior and immediate return interpretation, conditional
candidate behavior, subject component chain, candidate component chain, then
the physical/damage suffix if no `0xA300` result cleared it. The callback's null
return means type-9 `FUN_00402DA0` and the remaining suffix still run after the
source has been queued for destruction. Accepted WinDbg transcript
`runtime_re/captures/local/20260727-235038-base-conversion-pair-tail.txt`
closes the traversal ambiguity. The source saved successor `0x02CCE980`; spawn
appended type-8 replacement `0x02CBAD30` behind old tail `0x02CB5670`. When
traversal later reached that old tail it saved the tail's newly mutated `next`,
then visited the replacement in the same pair scan.
`BASE_PAIR_SCAN_END` reports `stage_mask=0x3F`,
`replacement_identity_valid=1`, and `replacement_cursor_seen=1`. Retail saves
`next` per candidate immediately before callbacks. It does not freeze a
pass-entry vector or stop at the entry-time tail. Therefore a new append is
seen if traversal has not yet saved its predecessor's old `next`; if the
source itself was the saved tail, the replacement would not be reached by that
visit.

The same accepted source dump records effective style frame entity `+0xB8 =
0x004C8788`, the class-54 `Go To Job` initial variant, whose pair-contact
callback is null. Retail therefore skips the conditional candidate-behavior
call for this conversion. The live adapter only skips that stage for a known
absent style or a known style with a null pair callback; unresolved or known
non-null callbacks fail closed before mutation.

Queued destruction also does not stop the remaining visit. Type-9 component
`FUN_00402DA0` passed its forward-half-space gate, entered descriptor effect
`FUN_00401A20`, and then the physical suffix changed flags
`0x06C68825 -> 0x06D08825` and raw velocity
`[0x000C,0x0000,0xFF68] -> [0xFFE7,0x001F,0xFF79]`. The component/effect
entry and exit RNG value was unchanged at `0x9CA563C8`. The accepted trace does
not partition the earlier advance from `0x0B480D77`, but static recovery now
closes the scientist policy: type 8 retains Section-12 words
`+0xC8 = 0x00000F00` and `+0xCC = 0x00000084`; `JobNearby` requires strict
per-axis wrapped separation below 15 units, gates choices in authored order,
and the shared-process RNG selects behavior class 54 versus 6 at 100:1. The
Rust static initializer now parses those fields, evaluates the strict range and
weighted choice, and retains the selected-behavior decision at the initializer
seam. It does not install a live scheduler/task graph.

The Rust mutation-safe list-pass executor now captures the general contract
proved by the accepted pair-tail transcript: save the current node's successor
before planning, close an immutable visit journal, apply every action in order
without rollback or short-circuiting, and run the suffix even after queued
destruction. An append behind an unvisited old tail is consequently visited in
the same pass; insertion behind a predecessor whose successor was already
saved cannot redirect that pass. The executor returns each action outcome for
audit and tests both append boundaries plus the non-rollback
notification -> queued-destroy -> failed-spawn -> independent-tail -> suffix
order.

The Main Base birth transaction runs after the particle pass and before the
independently closed player-subject pair subset. Native Bases authenticate their
own allocation; the captured Level-1 adapter retains its separate spawn6 gate.
Both reuse the active-pair eligibility/relation/broad-phase gates and oriented
Section-8 narrow phase. Native Type9/86/123 sources use completed task custody
and each actual component slot; the captured Type9 oracle stays separate.
Before its first visible action, the adapter closes the candidate-behavior
gate, selected worker metadata/initializer, `JobNearby` capacity inputs and
positive behavior set, plus the component/physical suffix. Apply
then emits event1, queues source destruction, constructs the worker components
and consumes its shared selector word, appends/relates the Type8/79/90/91/116 worker,
emits operation0x33,
and commits the retained component and no-damage physical suffix. Native
`02DA0` contacts use the stored forward matrix and actual private direction:
`01A20`'s Sub-I branch adds heading0x2000 and `019C0 ->204B0` writes that
direction, preserving private state, Sub-I, Sub-D and the incoming matrix.
Known null slots remain null, and the branch consumes no RNG. A
nonzero shared collision cap is also admitted when both local directional
`FUN_00415040 -> FUN_004255E0` filters return zero. The walking-arrival
regression reaches cap `3`; rejecting all nonzero caps previously blocked
conversion despite neither recipient taking damage. The zero filter returns
before modifier/hit callbacks, and the pair's channel words `[1, 0]` exclude
15040's remaining resource-message branch. The suffix retains the actual cap
in its outcome and leaves health unchanged; any nonzero filtered delivery or
unresolved/local-ownership failure still requires the separate damage path. A
successful class-54 or class-6 append consumes a separate constructor-suffix
word only after its task preparation succeeds; failed preparation consumes no
suffix word. These two selection/task words follow the genuine Sub-A20450
component-constructor word, making three words for a successful native Type8/79/90/91/116
birth. The
shared executor still preserves retail non-rollback ordering for an allocation
failure, while every currently modeled unresolved prerequisite fails before
the visit mutates.

The newborn scientist now retains its constructor-installed task graph on the
live port entity. Static recovery of the class-54 initializer shows a distinct
`Go To Job` sequence: select a target through `FUN_004235F0` first, clear task
slots 1 and 2, then install slot 0 through `FUN_00403650` with timeout 5000.
`FUN_004235F0` is not the
initializer's rule-13 `JobNearby`: it scans live-list order, rejects
self/inactive/dying candidates, and applies the owner's strict wrapped range.
An owner with capability bit `0x800` accepts a candidate with bit `0x20`
without consulting capacity; otherwise `FUN_00418EB0` requires a live
building component with current jobs below capacity and owner capability bit
`0x400`. Distance is the sum of each wrapped signed-16 axis delta squared and
shifted right by two. Only a strictly lower result replaces the current best,
so equal-distance ties retain the first list entry.
The target bit is byte-exact in both binaries: retail `0x0042368A` and demo
`0x0042355A` execute `test [candidate+0x64],0x20`. This corrects the earlier
`0x2000` interpretation, which made exact Type-9 owners (`0x1804`) miss every
normal Base target because they do not carry fallback owner bit `0x400`.

The Rust `go_to_job` module now preserves this selector and the subsequent
clear/clear/install-attempt order as a private, one-shot exact plan applied
through the shared three-slot task owner. The live Main Base birth adapter
builds the replacement locally, derives selector candidates from current
intrusive-list order, binds task owner and
Sub-A from that same unpublished entity, and reserves/publishes the entity ID
only after every fallible setup gate has succeeded. The allocation request
copies the source position, but type 8's default `+0xC0` has bit `0x20`, so
`FUN_0040D4A0` terrain-snaps the final published Y coordinate. This is not a
generic Main-Base `Y - 2` adjustment. Class 54 owns a real primary
`ActorTaskRuntime::GoToJob`. Class 6 is not taskless: `FUN_0040AD10` clears
slots 2 and 1, then installs a real slot-0 Wander Near task through
`FUN_00402E20 -> FUN_00406030/FUN_00406070`. The typed plans make malformed
slot indices and target/payload
disagreement unrepresentable. Selection still finishes before mutation; if
`FUN_00403650` allocation or initialization fails, the prior slot-0 task
survives while the already-completed slot-1 and slot-2 clears remain committed.
Its preparer supplies the selected handle plus the proven 5000-ms lifetime,
constructor `0x00403650`, initializer `0x00401350`, and tick `0x00403780`.

Static recovery closes the state consumed by that task, while
`20260727-235038-base-conversion-pair-tail.txt` independently corroborates its
live target handle, direction, and timer. Its captured target position is an
in-flight post-mover value, not initializer evidence. `FUN_00403650` reaches
the shared `FUN_00401350 -> FUN_004012E0` initializer used by Wander Near,
allocating a 0x24-byte private record with the controlled entity's current
position, the selected target handle, direction 1, and timer 0. A failed
selection is not an absent install: `FUN_004235F0` returns the zero-filled
`DAT_004DCA00` sentinel, retail still attempts the slot-0 install, and the
first callback returns the invalid-target singleton.

The successful constructor suffix is now statically closed for both selected
classes. Class 54 reaches `FUN_00406030/FUN_00406070` through
`FUN_00403650`; class 6 reaches the same suffix through
`FUN_0040AD10 -> FUN_00402E20`. Both paths run it only after task allocation and
initialization succeed. The exact first-world type-8 record has Sub-A and lacks
the generic Sub-H, Sub-G, and Sub-F branches. Its active Sub-A branch consumes
one shared RNG word, uses low-16 bits 8..15, writes direction 1, and writes the
intermediate target speed
`base + (((random_byte * base) / 0xA00))`. Retail
`0x004036CB..0x0040372B` (demo `0x0040370B..0x0040376B`) then follows
Section-12 Sub-A pointer `+0x08`, sign-extends descriptor word `+0x04`, and
the class-54 tail overwrites only live Sub-A runtime dword `+0x00` with signed
trunc-toward-zero `(base * 4) / 3`. The final Sub-F tail is absent for this
authored topology. Only after these generic and class-specific effects does
retail `0x00403761..0x00403770` publish the wrapper through `FUN_0040A7A0`.
Allocation/initializer failure jumps over the RNG draw, all component effects,
and publication, preserving the old primary while the earlier slot-1/slot-2
clears remain committed.

The Rust transaction derives selector state and constructor proof from one
`GoToJobOwner` snapshot. Its crate-private task-owner/Sub-A binder receives
both mutable references by destructuring the same replacement entity, while
apply also checks the snapshot identity before any clear. The identity check
is not treated as proof of reference provenance. Its corpus regression loads
normal-detail `1X3XX.OVL` local collision record 6 (cumulative type 8), proves
the authored A-without-H/G/F topology, and pins the recovered final Sub-A
runtime-dword-`+0x00` reset. Unit transaction oracles pin the lazy draw and
generic-write-before-fixed-overwrite order: dword `+0x04` becomes direction 1,
while drive-scale dword `+0x08` remains unchanged.

The same detached transaction also admits authenticated cumulative type 9.
Normal-detail local collision record 7
independently proves the required A-without-H/G/F topology and Sub-A base 250,
so the class-54 tail finishes at signed `(250 * 4) / 3 = 333`. Retail and demo
have the same prepare-success gate, generic one-draw write order, fixed
overwrite, and publication order. A missing target still publishes the task
with the zero sentinel; a failed preparation consumes no RNG or component
effect and preserves the old Primary after the earlier Secondary/Tertiary
clears. This closes the fresh-Level-1 type-9 Go-To-Job initializer. Fresh
production now consumes the selector and initializer transaction, retains its
exact target, and transfers a successful class-54 owner from the manager
sidecar into the live ordinary-Type-9 scheduler.

The outer Main Base conversion transaction preflights this one-shot class-54
plan whenever class 54 has positive effective weight. That happens before
event 1, source-destroy queuing, or the selector's shared-RNG draw. If the
constructor topology is unresolved, the live list, destroy queue, and RNG all
remain unchanged. On success, apply consumes the preflighted plan against the
same unpublished replacement; task-owner and Sub-A references cannot be
supplied from different entities.

The companion Rust `go_to_job_owner` module closes the behavior-neutral
task-family phases. It accounts the wrapping lifetime before callback, stages
the private record across callback entry, rejects missing/inactive/dying
targets, evaluates `FUN_00423030`, and exposes all seven semantic inputs of
`FUN_00401430` on either predicate branch. Its three retail results remain
pointer-distinct at `0x004BE0B8/0x004BE0C0/0x004BE0C8` even though all carry
tag `0x9C01`; tagged-result and strict `elapsed > 5000` decisions remain
separate post-unwind phases.

The ordinary fresh-Type-9 production adapter now binds that shared contract to
the real heterogeneous scheduler. It authenticates the constructor-retained
target and entity-local common-axis descriptor, performs live target lookup and
the exact common mover, preserves fresh later-slot reads and transition
suppression, retains root-class-6 selection across retry, publishes the direct
F70 basis and bit `0x4`, and then parks before E100. Main Base death transfers
that selected custody without probing or losing the branch. Stable-manager-
order adoption also includes birth-selected Wander: its exact dispatcher,
callback, and common mover retain tag-before-strict-timeout precedence, post-
unwind state-`0x1000` suppression, one root plan, class-6 application, F70, and
branch-neutral Main Base custody. Natural Type-8
scientist travel now has a live Go-To-Job visit through the shared D/I/A/B
kernel, a post-task `FUN_00413F70` rebuild from the live angle words, and
the common `FUN_00412DA0` master-motion suffix; E370 remains unclaimed. Conversion seed `0x38` and the D720/13F70 birth matrix are
owned. Factory receiving itself is scheduler-owned once a named scientist
is queued or an oriented factory/scientist pair contact is classified, and
that same owner now ticks the exact fresh-Level-1 production bridge. The
scheduler visit owns `FUN_00425C60` before `FUN_00419010`: it samples the
live `+0x34` last-hit tick through `FUN_00416490`, queues direct text `0xD3`
before the private latch, suppresses that text while Main Base abort is
active, and clears the latch when the under-attack window expires. Progressive-death frames stay with the Type-66 owner.

The deterministic common-mover core is no longer duplicated between the two
first-world actor records. Normal-detail Section-12 records prove that
cumulative type 8 (scientist) and type 9 (peasant) both use the exact
Sub-D -> Sub-I -> Sub-A -> Sub-B route. `v2k-game::common_mover::actor_abdi`
also admits native workers90/116 and people86/123 with their own complete
topology, verifies all four descriptors, retains distinct Sub-I records and
Sub-D policies, and commits the frame
atomically. The ordinary type-9 oracle now delegates to this shared kernel
without changing its public contract.

The explicit converted-worker replay retains the `V200002.run` first-query
owner seed38. Shared native Main Base and factory workers instead receive
their own successful process Sub-D allocation and native first-query policy,
then use the Type8 D/I/A/B kernel, post-task13F70 and12DA0. Neither the replay
seed nor Type9's captured first-consumer admission authenticates those births.
The complete native worker custody and world suffix are owned by
[INTRO2_TYPE8.md](INTRO2_TYPE8.md).

The conversion-born owner's completed class-54 callback now follows
`FUN_00401120` post-unwind ordering: tagged singleton before strict timeout,
state-`0x1000` suppression, then `FUN_0040AC60(NULL)` / `FUN_00425680`.
The exact Type8 root weights Job Nearby class 54 by 100 and class 6 by 1.
All selectable initializer inputs are checked before the selector RNG word;
the selected constructor consumes its separate word and publishes a new
Primary without ticking it again in that visit. Reselection preserves the
behavior context's target/auxiliary fields and Main Base birth provenance.
Class6 reads live entity+90, initially terrain-snapped at construction;
movement and ordinary task replacements do not recenter it. Factory08F00
and player attachment replace the original root with a carrying None task.
409030 resets both+90 and+96 to the materialiser position before16750/CE90
releases Sub-I and constructs a fresh selected root. Its selector sees the
landing anchor; the receipt's immutable birth anchor remains evidence only.
Unresolved post-unwind gates or initializer metadata retain a linear pending
transition and do not repeat the consumed callback, timer, movement or RNG.
An incomplete common-mover callback is parked separately; its partial work is
not replayed. Native Type8 class14 and E370 use the worker's own allocation
policy; replay-only worker histories remain explicitly separate.

## Converted Staff Output Selection (`FUN_0042eb70`)

**Section-13 world style determines the worker/output ejected after finite stock is
exhausted** (not the factory pickup):

| World Style (`+0x48`) | Converted Entity Type | Dynamic birth support |
|---------------|-------------------|------------------|
| 1 | 0x08 (8) | Native worker |
| 2 | 0x5B (91) | Native worker |
| 3 | 0x5A (90) | Native worker |
| 4 | 0x4F (79) | Native worker |
| 5 | 0x74 (116) | Native worker |
| 6 | 0x07 (7) | Native four-choice diver worker |

`FUN_0042EB70` calls `FUN_00450C90` for the current logical world/resource slot,
then reads `(*(DAT_004FE654 - 4 + slot*4)) + 0x48`: the current Section-13
`world_style`, not a player-craft field. It performs this lookup on each
phase-2 ejection attempt; the result is not selected or cached at factory
initialization. `FUN_004258A0` uses the same selector for Main Base conversion.
The native Type66 scheduler and Main Base conversion frame now forward the
actual loaded descriptor. Styles1–5 preserve the selected Type8/91/90/79/116
profile through construction, owner link, Type93 attachment and later release.
Style6 constructs Type7 through its own four-choice profile and allocation
receipt. The body phase is shared, while Job Nearby, Class45/10 constructors,
query-free Sub-D and cue106 remain explicit. Dynamic Class45 event16 is
delivered at the current tick before Type93 attachment; the constructor
provenance retains every actual suffix word. Native Type78/86/95 inputs use
their actual four-branch owner for conversion, including world32 steady wind.
Styles5/6 have no authored capability0x800 Base-input cohort. World42's
zero-capacity factory and world47's lack of a Base/factory do not justify
inventing conversion attempts merely because a worker profile exists.

## Factory Notifications

| String ID | Event | Call Site |
|-----------|-------|-----------|
| 0xD3 | "Factory under attack" | `FUN_00425c60` — queued before its private latch is set |
| 0xD0 | "Your factory is now working" | `FUN_00456820(0)` — first production cycle |
| 0xE6 | Deduplicated resource event 5 after accepted delivery | `FUN_00418D30`, after operation `0x33` and before staffing |
| 0xE3 | Deduplicated resource event 2 when capacity is reached | `FUN_00418C20` |
| 0xCE | Direct text on visible staffing clamp at capacity | `FUN_00418C20` — suppressed by the `INT_MAX` initialization sentinel |
| 0xCF | Understaffed countdown expired before the failure callback | `FUN_00419010` |
| 0xD1 | High-threshold notification | `FUN_00419010` — successful pickup spawn when `+0x08 > 4M` |

Output conversion separately queues deduplicated HUD resource event `0x16`
through `FUN_004568B0`. It is the beam/vaporise text resource, not a world
sound. Phase 1 also conditionally dispatches `FUN_004147A0` while
`DAT_004F741C` is nonzero; that branch clears the caller's status-publication
suppression before the common channel publication/dirty-bit tail.

## Scientist Indicator / Lifter Model Callback

Type 66's `FUN_0040D320` callback exposes five Section-8 channels: low 16 bits
of the 50-Hz tick, delivery/lifter ratio, production ratio, required scientist
count, and current scientist count. Required count is copied from config
`+0x04`; Level 1 contains 2. The `factpole` command stream emits one green
sprite-606 light per required position. During its blink-off clock phases only
the bottom `current_count` lights remain, so occupied positions are solid and
unfilled positions blink together. Channel 1 animates the lifter arm mount from
-22.5 degrees through upright to +22.5 degrees, but that mount does not belong
to the pole: model 227 executes opcode `0x0F` after its animated arm and weight
children and immediately before code-6 child model 179. Retail
`FUN_00466fc0` resets the current mount basis to Q1.31 identity.
`20260724-200841-factory-pole-render-basis.txt` confirms both the parent mount
and final `factpole` child basis are exact identity in the captured empty
factory state, even though register 1 is `0xF000`. The port therefore interprets
the authored reset rather than applying a type-specific pole override.
Accepted `20260724-041815-friendly-ai.jsonl` evidence independently includes a
phase-0 type-66 factory with delivery state `+0x64 = 0` and the channel-1 output
bound at `+0xA8`; `FUN_00419630` publishes that zero ratio.

## Reactor Mechanic

Reactors are related but distinct from factories. Model: `chernobylreactor`.

**Main Base/reactor abort** (`FUN_00456960`):
1. Run `FUN_0044C940`'s ordinary logical-sound garbage collection. It walks
   the live sound list and unlinks only nodes whose `+0x18` disposable one-shot
   flag is nonzero. That field is not a physical-playback completion flag:
   `FUN_0044C790` sets it after generic construction, and `FUN_0044C970`
   launches the request through `FUN_004958C0` as an auto-collected physical
   one-shot without storing that record at logical `+0x1C`. Persistent
   owner-held requests retain zero there. The sweep therefore does not stop
   already-playing mixer voices or persistent fan/gate loops.
2. Set session abort byte `+0x28F = 1` and reset session dword `+0x2BC = 0`.
3. Resolve the player and submit global positional sound `0x3E` at its
   signed-8.8 position
4. Enter `FUN_0042F1A0`'s systemic death/world-darkening dispatcher
5. Request the shared viewport-flash sequence through
   `FUN_00456750` (`session +0x295 = 1`)

The port maps the not-yet-submitted disposable subset to pending and ready
`WorldFx` positional sounds. Its first admitted Main Base header removes only
those requests before either session write; non-sound events retain order and
the shared RNG state is unchanged. A duplicate observation returns before the
sweep, while the null world-control branch still performs it and then attempts
the player-resolved resource `0x3E` request, matching the outer retail owner.
Existing `SoundManager` voices and persistent sound owners are deliberately
outside this operation.

`FUN_0042F1A0` first resolves the current world control slot. A null slot
suppresses its entire actor/factory/terrain/player body, while the preceding
session-abort write and positional sound remain committed. With a valid slot,
the actor/factory sweep and `FUN_00433E30` terrain transform complete
synchronously before the dispatcher returns; the port therefore performs the
terrain RNG draws before that frame's particle update.

Retail's actor sweep inside step 4 is now classified exactly in live-list order:
entity `+0x64 & 0x11 != 0` routes to `FUN_0041CF90`; otherwise type `0x6F`
is excluded and `+0x08 & 0x10001000 == 0` routes to generic authored death
`FUN_00410C10`. Unknown state fails closed. The accepted synchronized baseline
contains 39 actors: 34 ordinary-route actors across types 6, 9, 17, 47, 52, 54,
61, 62, 66, and 68; two alternate-route actors, the type-46 player and type-67
Hive; suppressed types 0 and 1; and excluded type 111. The capture establishes
live identity, list order, and route inputs rather than direct callback entry.
Generic death callbacks remain class-specific rather than a blanket live-list
splice. The port executes the fully recovered Working Factory branch and
terrain transform. The complete systemic body is now captured by a
non-replayable detached transaction: a known null world-control slot suppresses
every body action; otherwise it samples the intrusive successor only after each
actor callback, then orders terrain transformation, player state `+0x1F0 = 5`
plus the copied `+0xA8..+0xBC` request, and `FUN_00433130` submission. Receipts
bind every external action to one transaction and phase, while unresolved
post-callback state becomes a durable block retaining the acknowledged prefix.

`FUN_0041CF90`'s alternate route is closed by exact-matched retail/demo code.
It resolves the component table's Sub-N runtime slot `+0x34`; an absent Sub-N
returns without mutation. This makes the captured type-46 player route an exact
no-op because its authored component set is `ABCDEGJO`. Capability-mask types
51 and 88 likewise have no Sub-N; type 67 is the one such owner with Sub-N, and
its sole weighted behavior deterministically installs the Alien-Hive callback.
The generic topology constructor allocates and zero-fills the `0x5C` Sub-N
runtime from type-record `+0x108` **before** behavior selection; behavior does
not own that allocation. A spawn without animation therefore still owns Sub-N
with `+0x4C = 0`, `+0x50 = 0`, and a zero anchor. When animation is present,
`FUN_0041BC20` retains its exact six dwords, initializes the Sub-N anchor from
authored entity X/Y/Z and descriptor X/Z, then scans the same 3x3 neighborhood
in X-major order. The first nonzero Section-10 attribute whose Section-9
descriptor kind is 22 through 26 sets runtime `+0x4C = 1` and snaps the X/Z
anchor to that cell centre; otherwise the zero-filled flag stays clear.

For a live Sub-N, `FUN_0041CF90` always writes runtime
`+0x50 = -3,000,000`. Only a nonzero retained `+0x4C` enters the terrain path:
the callback re-resolves the descriptor's signed X/Z offsets, quantizes and
wraps the coordinates, and calls `FUN_00433860` for all nine cells in X-major
order. `FUN_00433860` clears each Section-10 cell's **attribute byte** while
preserving height and terrain type, then synchronously publishes `(x, z, old)`
even when the old attribute was already zero. The real Level-1 type-67 fixture
covers X cells 186--188 and Z cells 128--130. Its fifth cell `(187,129)` carries
attribute 255 and Section-9 kind 22, making it the first and only qualifier;
the retained constructor anchor is `[-17536,0,-32384]`, while the nine published
old attributes are `[0,0,0,0,255,0,0,0,0]`. The accepted capture retains this
Hive through the sweep, but its component-table window ends before slot
`+0x34`; static code and retained live custody close the callback without
another capture.

The first ordinary-death family is now statically closed and live as a bounded
callback adapter. Full-game `FUN_00410C10` and demo `FUN_00410BA0` agree on the
generic prefix: remote ownership returns zero, an existing `0x4000` dying bit
returns one, and a fresh local death writes health zero and `0x4000`, requests
the optional type `+0x90` cue, releases entity `+0x8C`, invokes the current
style death hook, and continues through the type's alternate behavior. Fresh
Level-1 types 52, 62, and 68 have alternate class 2 (`"Die Quietly"`), null
current-style death hooks, no managed-death capability, and a zero Section-12
`+0xB4`, proving the constructor-cleared `+0x8C` branch is a no-op. Their exact
Section-13 cohort is type 52 at indices 0, 1, 2, 3, 7, 21, and 27--31; type 62
at 4, 25, and 26; and type 68 at 5. All four retail X3 tiers retain the same
profiles and type-68 cue 62; both demo tiers retain the same control/data shape
but author cue 64.

Class-2 initializer `FUN_0040C470` calls the shared task clear
`FUN_0040A860` in Primary/Secondary/Tertiary order, then `FUN_00410B70` writes
`(state & 0xFFF9FFFF) | 0x00100000` and increments the deferred-sweep count.
Shared native Type52/68 now authenticate successful construction, the current
manager allocation and exact living class0 timer custody across ordinary
worlds. They commit health/dying state, Type68 audio, class2 publication, task
release and deferred staging, then sample the intrusive successor. Their
9,000-ms Primary is real, although the default callback-enable state leaves
its timer unvisited. Pending prefixes or stale receipts cannot fall through to
the old fresh-world adapter. [Class0 Runtime](CLASS0_RUNTIME.md) owns the
constructor, conditional timer and living weight-release contracts.

Native Type62 construction publishes the class-6 Wander Primary from actual
records. The earlier fresh-Level1 quiet adapter remains an explicit replay
boundary for old fixtures: exact spawn/type/model/component and class0/6
context checks admit locally releasable omissions without claiming that retail
had no task. The accepted run's 15 quiet actors out of34 ordinary actors,
class2 publication and following removals remain corroborating capture facts;
they do not restrict the native52/68 constructor to that cohort.

The authored Level-1 list fixes the remaining implementation order. After
quiet spawns 0--5, spawn 6 is the terminal type-6 Main Base revisited by the
later systemic abort sweep after its original generic-death callback has
raised controller `+0x1F8`; that revisit is therefore an authenticated already-
dying no-op. Quiet spawn 7 follows, then spawn 8/type 54
selects class 38, spawns 9--10/type 9 select class 14, spawns 11--13/type 47
select class 12, spawns 14--16 repeat type 9, and spawns 17--20/type 17 select
class 12. Quiet spawn 21, type-9 spawn 22, type-66 factory spawn 23, alternate
type-67 spawn 24, quiet spawns 25--31, and type-61 spawns 32--34 complete the
ordinary/alternate cohort. None of these initial callback selections needs a
new broad capture. A live heterogeneous dispatcher must nevertheless process
them in this order and sample each successor only after its callback; it may
not skip the earlier type-6/type-54/type-9 gates to reach type 47.

The enclosing frame order is also closed statically, and the systemic abort is
deferred relative to the actor-task sweep. Main Base stage 32 enters
`FUN_00410C10`; type 6's vtable `+0x08` is `FUN_0040DB80`, not
`FUN_00456960`. Its generic-death suffix reaches
`FUN_00456DB0 -> FUN_0042F160` and only raises controller flag `+0x1F8`.
Gameplay frame `FUN_0044FFA0` first calls `FUN_00413500`; later in the same
function it invokes campaign selector zero through `DAT_004FECDC`. The bound
`FUN_0042DD10` selector clears `+0x1F8` and then calls
`FUN_00456960 -> FUN_0042F1A0 -> FUN_004170A0`. There is no later
`FUN_00413500` before the frame function returns. Exact-matched German-demo
counterparts retain the same scheduler-before-selector topology.

The ordinary Type-9 tasks after Main Base consequently receive their final
predecessor visits and `FUN_0040E870` matrix rebuilds before class 14 is
published. New Exploding-Person tasks wait for the next frame. Sample 5560 is
the preceding tick's observation; sample 5564 is the first post-abort
observation and retains those final predecessor-produced matrices. The changes
at spawns 10, 14, and 22 are therefore old-task effects, not newborn class-14
visits. Production attachment must preserve the scheduler-then-campaign-abort
phase boundary. Mutation-sensitive successor sampling still applies inside the
later `FUN_004170A0` abort sweep, including its appended Type-60 tails.

The terminal type-6 seam is now callable as a bounded, non-mutating adapter. The
stage-32 progression commit issues a non-cloneable origin carrying the exact
Main Base allocation lease only after health zero, dying bit `0x4000`, and
model slot 1 are retained. `FUN_00456960` transports it to the systemic-body
gate; a null world-control slot drops it because `FUN_0042F1A0` performs no
actor sweep, while an admitted body retains it for the detached dispatcher.
When the first actor action names that same fresh-Level-1 spawn-6/type-6 lease,
the adapter re-resolves remote ownership before the dying bit, proves retail's
`AlreadyDyingNoOp`, performs no audio/RNG/task/relation/entity mutation,
consumes the origin, and samples the linked successor only after the callback
boundary. Receipt-bound composition then processes quiet spawn 7.

Spawn 8/type 54 remains the Level-1 authored grock; native ordinary construction
no longer uses that spawn index as the abort admission key. All four retail
tiers and both demo tiers agree on its tier-invariant type record: model 560 in
all four slots, mass 100, health 1,000, capability flags zero, initializer state
`0x4027`, class-0 style `0x004C7468`, alternate class-38 style `0x004C7348`,
null death hook/cue/constructor attachment, and no optional Section-12
components. Model 560's independent Section-8 header supplies extent 256. Its
exact fresh Section-13 `+0x1C` value, copied to
entity `+0x88`, is zero; the live adapter retains that as the named
derive-from-actor/model/sea source instead of treating an absent value as a
numeric mode. Other payload sources remain fail-closed.

Matched retail/demo code closes class 38 completely. Generic death commits
health zero and dying bit `0x4000`, selects class 38, then the initializer clears
entity bit `0x800`, makes the authored base of ten frame-paced class-16 scatter
attempts, reaches an exact solo kind-2 network no-op, emits the above/below-sea
surface branch, consumes one RNG word for sound 62, and clears Secondary then
Tertiary. With captured actor Y
`-1248`, model extent `256`, and Level-1 Section-10 header whose unsigned shift
then signed-word truncation gives `0xFFFCB100 >> 8 -> -847`, `FUN_00405040`
publishes Primary with exact delta
`((-1248 - 256) - (-847)) << 8 = -168192`. Allocation failure is not a
retryable block: the already-committed effects survive and the outer installer
publishes its unnamed `0x1280` fallback while clearing Secondary, Tertiary, then
Primary. The empty component topology makes the later `FUN_00406070` suffix an
exact no-op with no extra RNG.

The dedicated task owner retains `FUN_00401120`'s pre-callback whole-millisecond
counter separately from `FUN_00405160`'s integer `FUN_004013A0` terrain step.
Falling water uses the full rate-capped step, a strict sub-step remainder lands
exactly, and terminal completion queues `FUN_00410B70` deferred removal while
leaving the Primary wrapper owned until the later entity sweep. The accepted
first deep sample (`elapsed = 96`, remainder `-167022`) is consistent with this
start value and prefix. Linear receipts prevent replay, and the generic task
dispatcher explicitly rejects this terrain-owning family. Detached Main Base
composition acknowledges spawn 8; type-9 class 14 is the next family in exact
list order. No new Type-54 capture is required.

The six authored fresh-Level-1 type-9 actors are now closed by the next bounded
ordinary-death adapter. All four retail X3 tiers retain model 558, mass 10,
health 1,500, capability flags `0x1804`, initializer state `0x2F`, A/B/D/I
components, and alternate class 14; both demo tiers retain the same control and
component shape, with their separately authored sound values. The accepted
Main Base world-behavior track brackets spawns 9, 10, 14, 15, and 22 in
class-6 variant-zero style `0x004C79C0`, and spawn 16 in class-10 variant-one
style `0x004C7660`. Every predecessor has a null death hook. This corrects the
earlier port-side swap of spawns 16 and 22. This observation
authenticates the current context used by the callback without inventing the
earlier process-RNG-owned weighted selection that the port has not yet run.

Matched retail/demo `FUN_00410C10`/`FUN_00410BA0` and class-14 initializer
`FUN_0040C3A0` close the transaction. Fresh generic death writes health zero and
dying bit `0x4000`, requests retail sound 35, traverses the proven null
attachment/death-hook branches, and publishes Exploding Person style
`0x004C70C0`. The initializer clears Secondary then Tertiary, resets Sub-I's
linked handle to the zero sentinel, special mode to one, and phase to zero
without disturbing its other fields, then clears entity state bit `0x8000`.
The outer Main Base abort has already written session byte `+0x28F = 1`, so the
capability-`0x800` optional message `0xC6` branch is deterministically
suppressed for this owner.

`FUN_004032A0(entity, 0, 1000)` prepares the current-position, null-target
shared-retarget task before its shared initializer suffix. Allocation failure
therefore consumes no RNG and performs no Sub-A reset: the prior generic-death,
task-clear, Sub-I, and state mutations remain committed while the outer
installer publishes its unnamed `0x1280` fallback and clears Secondary,
Tertiary, then Primary. Success consumes exactly one shared RNG word in
`FUN_00406070`, writes direction `+1` and its randomized descriptor-base speed,
then overwrites that speed with literal one immediately before publishing
Primary. The bounded port's empty predecessor task slots are an explicit
equivalence for its still-dormant initial-selection owner, not a claim that
retail's observed Wander/Run-Away actors had no live task.

The dedicated non-copyable post-publication owner now holds that exact Primary
lease through the shared-retarget callback transaction. It accounts wrapping
whole milliseconds before callback, retains the branch-local retarget RNG
order and committed prefix on a later evidence block, and applies the common
D/I/A/B mover atomically. Matched retail/demo `FUN_00420520` closes class 14's
previously unowned Sub-I branch: special mode takes precedence over the
preserved forced-stop byte, advances the four-phase controller, and publishes
the Type-9 stride-4 special selectors 38--41 without target lookup or RNG.

Matched retail/demo code also closes the basis source and lifecycle. The
signed-Q31 lateral/up/forward matrix lives on the entity at
`+0x0C..+0x2C`; retail `FUN_0040D720 -> FUN_004104B0 -> FUN_00413F70` and demo
`FUN_0040D720 -> FUN_00410440 -> FUN_00413EE0` initialize it from the three
angle words. Type-9's D/A/B helpers read that retained matrix directly. A
Sub-D heading write does not rebuild it, so all mover phases consume the same
callback-entry basis; DCA0/E870 rebuilds the next basis only after task
dispatch. Rust now retains this physical matrix on the entity, snapshots it
after exact allocation/task validation, and no longer accepts a parallel
caller-supplied basis table. Because the bounded port deliberately leaves the
six villagers dormant before the abort, its existing capture bridge installs
the six exact sample-5564 post-predecessor/pre-class-14-callback matrices
together with the corrected predecessor contexts. This is a fresh-Level-1 bridge, not a
substitute for the ordinary AI path's future DCA0/E870 matrix updates.

After callback unwind, either a mapped mover transition or elapsed time
strictly greater than 1,000 ms selects class-14 variant-one style `0x004C7108`;
its `FUN_0040C470` initializer clears Primary, Secondary, and Tertiary and
queues deferred destruction. A known entity-state bit `0x1000` suppresses only
that frame's owner transition: custody returns to the running task and the
next manager pass executes a fresh callback. If the suppression/deferred state
is unresolved after the callback commits, the owner retains the exact pending
transition and retries only the terminal adapter, never elapsed accounting,
retarget RNG, Sub-I, or movement. The earlier accepted Intro2 sample
independently observes the style transition at 1,125 ms under its intrusive
125-ms cadence.

The Main Base path therefore owns exact entry, allocation failure,
post-publication task custody, special Sub-I, and the terminal style/deferred-
removal handoff from matched C and existing evidence; no new Main Base or
class-14 capture is required. The owner is now adopted as the fourth family in
the specialized Type-54/17/47/9 coordinator. The enclosing `FUN_0040E870`
terrain-attitude, surface-effect, master-motion, and world-loop suffix is still
outside this checkpoint; its acknowledged rebuild must update the retained
entity matrix before a later task visit. Only
`20260817-072421` closes the session-zero message-`0xC6` presentation path
for ordinary Level-1 type-9 capability `0x1804`. A non-fresh or non-Level-1
class-14 history remains an optional separate question.

Spawn 23/type 66 is now closed by a bounded Working Factory abort adapter.
Its authored Section-12 models `[210,225,210,225]` are checked separately from
the fresh Level-1 live override `[227,225,227,225]` / active model 227. The
exact row has mass 1,000, capabilities `0x84`, health 99,999, cue 62, null
constructor attachment, only Sub-M `[4,3,0,2,1,0]`, and one Always/class-39
choice. Accepted sample `5560` has state `0x0EC28805`, class-39 style
`0x004C9558`, idle progression `+0x94 = 0`, null T/S, and a Primary
`0x00425C60` task with private under-attack latch zero.

Matched retail/demo C fixes the synchronous order. Remote ownership returns
before even reading dying state; already-dying then returns before metadata,
context, component, task, audio, or RNG work. Fresh dispatch writes health
zero/dying, queues cue 62, then `FUN_00419750` revives health to 10,000,000,
clears dying, and changes the idle timer `0 -> 1`. The singleton selector still
evaluates its Always weight and consumes exactly one shared RNG word.
Reselection preserves context target/auxiliary words, then `FUN_004257C0`
clears T and S and prepares a replacement Primary. Success retains only the
stable factory-owner allocation identity and a fresh false notification latch;
it does not bind mutable `state_version` or duplicate the separately closed
production owner. Allocation failure preserves every prior effect, publishes
the unnamed fallback, and clears S, T, then P, ending at exact state
`0x0EC00805`.

The captured constructor publishes the initial task marker without replaying
historical RNG. The generic heterogeneous dispatcher rejects this dedicated
family in a pre-scan before an earlier task can mutate. Detached abort
composition now processes Type 66, executes alternate spawn 24's retained
Sub-N cleanup, processes quiet spawns 25--31, then runs Type-61 spawns 32--34
and every appended Type-60 tail before reaching terrain. This detached chain
starts at spawn 6 and processes 32 allocations. Production still uses the
older pre-sweep `arm_live_factories_for_main_base_abort` bridge because it does
not yet run the detached actor sweep; never run both paths in one abort, and
remove the bridge atomically with production sweep attachment. No focused
Type-66 abort recapture is required.

The final authored Type-61 suffix is also statically closed by matched
retail/demo code and live as a separate bounded composition. Section-13 spawns
32, 33, and 34 are the only admitted sources. They are respectively at
`[-29440,-512,22528]`, `[7424,-3072,-25344]`, and
`[22272,-1536,21248]`, with payloads `0x3C`, `0x3F`, and `0xC802`.
Spawns 32 and 34 use authored model 82 and its Section-8 extent 128; spawn 33's
four live model slots override that row with model 138 and extent 75. The
adapter authenticates the selected live model and consumes its extent rather
than hard-coding the Type-61 default.

Fresh generic death writes health zero and dying bit `0x4000`, releases the
live constructor sound attachment 44, traverses the null current-style death
hook, and installs direct alternate class 49 (`Explode With Ring`) without a
selector draw. `FUN_0040BAF0 -> FUN_00440950` first makes sixteen frame-paced,
failure-short-circuiting direction-table attempts, alternating classes 94/95
at velocity scale 8 and using the selected model extent. The direction cursor
advances before each attempt. The following kind-2 network request is an exact
solo no-op. Above the sea plane the independent surface tail attempts class 18
with velocity `[0,500,0]`; at or below it attempts classes 45 then 46, with the
second X position offset by `-0x40`. That tail runs even when scatter allocation
stops early, then consumes exactly one shared word for sound 62 at rate
`0x10000 + (word >> 3)`.

`FUN_00416F90` assigns both the burst and radial packet to a valid relation
`+0x60` owner; an absent or dangling relation falls back to the dying source.
The retained owner order is entity type then handle in the radial trailing
words. After the burst/sound word, `FUN_004566E0` applies static objects before
the live dynamic-entity pass using inner radius 512, outer radius 1024, impulse
2000, and channels `[1,3]` with amounts `[4000,4000]`. The implementation
preflights both domains before committing generic death, then preserves retail
commit order: class-49 publication, burst and solo no-op, static radial,
dynamic radial, Primary/Secondary/Tertiary clear, Type-60 construction, and
source deferred destruction. Thus class 49 consumes no selector word, its
sound word precedes both radial domains, and Type 60's constructor selector
word follows the complete radial transaction and source task clear.

Class 49 builds a zero-filled Type-60 request at the source position. The
Type-60 row uses model 243, health 1, sole Always class 48, alternate class 2,
and only Sub-K. Matched construction code proves three distinct failure
boundaries. Entity/component preparation rejection occurs before weighted
selection, consumes no selector word, and publishes no actor. Behavior-context
rejection occurs after the singleton selector's one mandatory RNG draw and
leaves no linked actor. If context construction succeeds but class 48's Primary
allocation fails, the nested initializer error is consumed by the behavior
installer: it publishes the unnamed fallback with bound output zero, and the
outer constructor still links a real Type-60/model-243 actor. Success instead
links class 48 with an `Exploding Ring` Primary and bound output `0xFFFF`.
Either linked form owns a matching entity-backed ring presentation; allocation
failure is non-fatal to the already-committed Type-61 death.

Tail append is observable to the same mutation-sensitive forward traversal.
The bounded suffix processes spawns 32--34 in authored order; after spawn 34,
the successor is the first Type-60 tail, and all linked tails are visited in
append order. Each authenticates either the named class-48/Primary/`0xFFFF`
shape or the fallback/no-Primary/zero shape, takes generic death's direct
class-2 alternate without a selector draw, clears Primary/Secondary/Tertiary,
removes its entity-backed ring, and queues deferred destruction. This closes
Type-61/Class-49 and Type-60/Class-48/fallback/Class-2 semantics without a new
runtime capture. The ordinary Working Factory constructor now feeds that same
path through explicit allocation provenance. Retail/demo `FUN_00419010`/
`FUN_00418FD0` build a zero-filled Type-61 request, and the shared constructor
still consumes one selector RNG word for the singleton Always/class-23 row.
The live factory bridge now takes that word from `WorldFx`, retains it with the
exact factory version and request, tail-appends the class-23/model-82 product,
then binds relation `+0x60` to the factory. Main Base admission accepts only
that receipt-backed allocation, authenticates newborn state `0x0E408805`,
payload/position/default extent 128, exact relation, and the source factory's
stable allocation identity. It never redraws the historical constructor word.
The product contributes one further class-49 callback and appended Type-60 in
the mutation-sensitive traversal; arbitrary unauthored Type-61 shapes remain
rejected.

The other live Type-60 caller is now closed without another capture. Retail
`FUN_004141D0` and demo `FUN_00414140` are exact-instruction matches with
1,478-byte bodies. On the hard-water branch they zero-fill the same `0x4C`-byte
request, write type `0x3C`, place it at `[entity.x, animated_surface_y,
entity.z]`, leave rotation, payload, and damage buffer zero, and override all
four model slots with 130 when pre-damping signed Y velocity is strictly below
`-1750`, or 132 otherwise. Retail `FUN_00438080` / demo `FUN_00437AC0` submits
that request through the shared constructor. The ring uses the displaced
animated-surface Y, while positional sound 17 uses the entering body's original
XYZ. The mandatory singleton class-48 selector word is therefore consumed
before that sound and the final signed arithmetic shift of Y velocity. The
three already-proven constructor failure boundaries remain distinct;
caller-owned sound and damping still run when no actor links.

Paired retail/demo task bodies `FUN_00406D20` / `FUN_00406D90` and
`FUN_00406DC0` / `FUN_00406E30` close the longer-lived actor path. A successful
Primary publishes bound output `0xFFFF`. Each later callback clamps the
effective unsigned control to `0x1000..0xD000`, subtracts
the 32-bit wrapping product `((elapsed >> 12) * effective) >> 7`, and keeps an
exact-zero result alive; only a negative result writes zero and returns the
terminal transition. The port
retains construction provenance separately from the selected model, stages the
new class-48 owner only after the current specialized pass, and mirrors every
task write into the one entity-backed ring presentation. Terminal class 2
removes that exact presentation and stages actor destruction. An existing
Type-60 water-entry actor also authenticates its 130/132 provenance and aged
task/presentation control during the atomic Main Base sweep, which retires its
scheduler custody in the same transaction.

Natural class-48 expiry is not generic death: class 2 removes the presentation,
clears the published tasks, and queues deferred actor destruction while
preserving health `1` and a clear generic-dying bit.

This complete lifetime also belongs to ordinary class-49 explosion tails,
including Intro2's destroyed gun turrets. Retail `FUN_0040BD20` clears the
source tasks after the radial burst, then passes its zero-filled Type-60
request to `FUN_00438080`; `FUN_00406D20` and `FUN_00406DC0` never branch on
that caller or the selected model. Production admission therefore resolves
all four model slots from the retained constructor policy: class 49 inherits
the authenticated row's model 243, while hard-water callers retain their
130/132 overrides. Both require the same exact allocation, Primary task,
callback sequence, context, position words and bound presentation control.
A default-model actor without constructor custody, or an explosion tail with
a substituted water model, remains rejected. The shared callback consumes no
additional RNG and does not replay the source explosion or radial damage.

Matched `FUN_004258A0`/`FUN_00425770` also closes the conversion-pending Type-9
overlap. Its shared pending bit `0x00100000` is outside the Main Base abort skip
mask `0x10001000`, so class 14 still publishes. The adapter admits only a
coherent `MainBaseConversion` queue owner, leaves that queue and bit untouched,
and does not create an actor-terminal owner; foreign or contradictory custody
fails before sound, RNG, or mutation. The next manager update removes the
source before class 14's strict one-second terminal can run.

One atomic production transaction now executes the persistent
type-46 player's exact no-Sub-N alternate route, authored spawns 0--34, and all
three appended Type-60 actors under the same receipt machine, reaching terrain
after 39 port-owned visits. The prefix quiet callbacks stage
IDs 1--6 in order and only spawn 5/type 68 publishes cue 62. The transaction
adopts six Type-9 leases, one linear Type-54 receipt, three Type-47 and four
Type-17 Common-Dying owners, and one Type-66 lease into the heterogeneous
scheduler without ticking them in the aborting frame; each Type-60 linking
receipt remains owned until its appended actor's same-sweep death. The
independent corpus-backed prefix and Hive regressions still authenticate the
real constructor/cache boundaries. Separately, a read-only
lease-authenticated advance admits exactly the
callback-free type-`0x6F` and state-mask branches, follows current live-list
order rather than numeric IDs, preserves capability-before-type precedence,
and rejects the unresolved ordinary-state route. Type `0x6F` remains excluded
without consulting state, as retail orders that gate first. The port manager
does not materialize retail's captured type-1/type-0/type-111 system
allocations. The port's 39 visits therefore are not retail's 39-node initial
census: retail appends and visits three further tails, for 42 visits. The
missing system allocations remain a census distinction rather than a port-live
actor owner. The transaction evaluates against forked actor/task/effect state
and a rollback-guarded current terrain. Only complete actor, terrain,
controller, and scheduler success replaces the live owners; a block discards
the speculative prefix and returns the canonical terminal origin. The baseline
fixture's additional constructor-backed Type-93 Materialiser raises its port
census from 39 to 40 visits. Those baseline fixtures
contain no hard-water ring and retain their exact counts. Each provenance-
authenticated live hard-water Type-60 contributes one additional entry visit;
its aged class-48 owner and presentation are retired transactionally by class
2. Factory-born Type-61 abort authenticates the producing Type66 allocation
identity in any ordinary world that constructed the product; overlay 13 spawn
23 is not the admission key. Authored Level-1 pickups still use the captured
spawn table. A stale factory allocation fails closed before class 49.
Attached cargo itself takes the callback-free route through its relation bit.
Quiet Type62 constructor/abort now authenticates the native class-6 Wander
receipt; remaining later-world abort blocks are tracked in
[Cross-level gameplay runtime](CROSS_LEVEL_GAMEPLAY_RUNTIME.md).
Native Type54 construction and Change-Sea-Level abort authenticate the class0
receipt rather than overlay-13 spawn 8.
Shared Type17/47 abort and factory-born Type61 class-49 already join that
sweep through allocation identity.

Type93 abort admission now follows the shared canonical constructor across
worlds. Successful allocation/publication mints an actual manager allocation
lease; player drops, factory output and Type17 missing-relation construction
retain that proof through the completed `08F00` link. The abort adapter checks
the proof, current Materialiser context/task, component topology and exact
Sub-J/backlink before mutation. Geometry-only proxy fixtures have an explicit
projection origin and cannot acquire this authority from a purpose tag or
matching model. A factory's later death does not invalidate its already-born
proxy's allocation proof.

The destruction path is independent of the child's normal release behavior:
`FUN_00408F00` installs `FUN_00409030` for release and `LAB_004090B0` for abnormal
teardown for every child. Class2 clears the proxy task and stages the proxy
while retaining Sub-J. Later component teardown clears the row and `090B0`
calls only `10B70(child)`; the deferred sweep rescans to remove a child which
preceded its proxy. It does not run normal release, root reselection or their
RNG/audio effects. This is established by `bulk/game_logic.c`'s `08F00` body
and the retail `090B0` instructions, without a new capture. The
[Type93 regressions](../../crates/v2k-game/src/entity/type93_abort_tests.rs)
exercise actual later-world Type8/9/68 player drops, factory output, incomplete
links and stale/projection proofs. Existing remote/already-dying no-op and
Type17 deferred-cascade controls remain in place.

The production-shaped terrain/static owner for that composition is now live.
It takes the old Type-67 attribute byte and clears it atomically on the actual
current-level Section-10 allocation, retaining every publication including
zero; the real Level-1 spawn-24 proof now exercises this owner and verifies the
authoritative centre cell changes from 255 to zero. A shared live-static
resolver distinguishes an empty attribute from missing terrain, Section-9, or
selected Section-8 model and reacquires the model selected by current
terrain-type bits. Each Type-61 static half preflights the complete 5x5 scan
without RNG, scheduler, or terrain mutation, rejects unresolved candidates or
unsupported in-range hits, then commits its prepared X-major hits through the
bound static scheduler and `WorldFx` RNG. Kind-10's immediate attribute/type
transition is applied before the next recorded hit. One stable terrain
snapshot supplies only the header/height/bit-0x10 inputs which these callbacks
cannot change. Its opaque custody is revalidated against the live level before
mutation; target
presence and model selection always use the live cache. The joined transaction
now holds this owner continuously across Type 67 and all three Type-61
callbacks. The Type-17 and Type-47 family schedulers delegate to exact one-
owner ticks. Their bounded composite now accepts a concrete mutable
`ResourceCache` and `WorldFx`, snapshots manager live order once, and draws
production randomness directly from that process-global effects owner. Before
taking its owner vector or visiting any actor, the pass requires Section 10
from the strict current `Level` cache layer plus Section 13 from that same
`LevelState`. A missing terrain or descriptor returns a pass-wide block with
every owner retained, no RNG consumed, and no manager, terrain, or effects
mutation. Section 9 remains the independently layered active biome table.

Each live-list visit takes only a short terrain borrow: mutable for Type 54 and
immutable for Types 9, 17, and 47. Type 54 mutates the authoritative terrain
inline at spawn 8 and ORs its before/after result into one pass-wide sea-level-
dirty signal. A boundary fixture starts sea level 46 just above Type 47's deep
threshold; Type 54 lowers it to 45 before the later Type-47 callback, which
consequently takes the non-deep timer path. Type-17 and Type-47 bubble requests
are no longer returned as a replay journal. Each is materialized immediately
through the same `WorldFx` and strict terrain collision context at its live-list
position, and the pass reports separate materialized and dropped counts. The
scripted RNG callback now exists only as a test seam; production owns the
actual `WorldFx` stream.

Type 9 is the fourth specialized family. Its exact six-actor Main Base owner
preflights the complete normal-scheduler, task, active-model, E870, and
surface-lifetime contract before shared RNG or actor mutation. Scheduler waits
publish only `+0x70/+0x6C/+0x68` and the retail `+0xB2 = 0` unwind. Continuing
visits pass the scheduler's carry-adjusted, 125,000-microsecond-capped callback
delta to both class 14 and E870 while retaining process-global elapsed as the
separate Sub-D clock. After the task it synchronously applies attitude, rebuilds
and publishes the Q31 body basis, applies E100 drag, snaps at old X/Z, runs E370
against the current live terrain/sea/water/tick, clears `+0xB2`, and only then
rereads state for master motion. A terminal class-14 callback still receives
the already-latched E870 suffix, but its cleared master-motion bits suppress
the final integration. The strict 1,000-ms task lifetime plus the scheduler cap
also proves its surface timer cannot exceed 1,125 ms, so this cohort cannot
reach the independently authored 5,000-ms lifecycle continuation.

The owner resolves exact model 558 in all four slots, active slot 1, and the
level-6 local-model-234 extent 165 by complete allocation lease rather than a
reusable entity id. Missing or mismatched basis/model/scheduler/terrain custody
fails before RNG. Same-family replacement returns the displaced non-copyable
owner, a cross-family conflict returns the rejected owner, and stale or absent
actors drop only after lease-aware diagnosis. The coordinator exposes exact
retained Type-9 allocation claims, and `EntityManager` accepts those complete
leases when advancing neutral Sub-I controllers. Direct composition proves the
claimed allocation is not advanced twice, an unclaimed allocation still
advances, and a stale lease with the same entity id but a different allocation
identity excludes nothing. The production world loop snapshots those claims,
runs the heterogeneous scheduler, and advances only unclaimed neutral Sub-I
owners. Static retail/demo C plus the accepted partial capture closed this
owner; no further Main Base Type-9 capture is required.

Type 66 is the fifth specialized family. Same-family replacement returns the
displaced owner and cross-family conflict returns the rejected owner. The
single live-order pass samples entity `+0x34` directly at the Type-66 visit,
uses caller-owned notifications plus the concrete `WorldFx`, returns terrain
lights in effect order, and retains blocked authority for exact retry; stale
and terminal custody are released. Its lease is not exposed as a Type-9
neutral-Sub-I claim; it is independently exposed as a full-allocation factory-
progression claim. A later legacy progressive-death pass can therefore exclude
the exact actor already advanced by the specialized callback without excluding
a same-ID actor from another allocation domain. The claim set is snapshotted
before the scheduler visit and retained through that frame, so an owner that
blocks or drops cannot fall through to legacy progression. The legacy factory
pass now receives those entry claims, and the later abort sweep deliberately
replaces the same actor's old Primary owner with its newborn Working Factory
authority. This adoption required no new capture.

Hard-water Type 60 is the sixth specialized family. The current specialized
pass precedes the player/contact walker, matching `FUN_0044FFA0`: a Type-54 sea
write and any earlier specialized-owner RNG are committed before whole-body
water classification and Type-60's selector draw. Construction then returns a
linear class-48 task lease which the main loop registers immediately; because
the pass is already complete, its first visit is next frame and the visible
birth state remains `0xFFFF`.
Later live-list visits authenticate the full actor/task allocation identity,
constructor provenance, selected 130/132 model, and matching entity-backed
presentation before advancing control. A block retains the same callback
sequence without consuming elapsed time twice; stale custody drops without
touching a same-id actor from another allocation. Terminal completion removes
the presentation, stages class-2 deferred destruction, and releases the owner.
The generic heterogeneous dispatcher continues to reject this dedicated family
before any earlier slot can mutate.

`TerrainCollisionContext::from_current_level_cache` applies the same strict
Section-10/same-Level Section-13 rule while retaining the active layered
Section-9 table. The existing Intro2 and gameplay physical-particle call sites
now use it, so they fail to a dry/unavailable context rather than pairing a
current section with unrelated PRELOAD or auxiliary fallback data.

The world loop calls the specialized composite in live-list order before
unclaimed neutral animation, particles, pairs, and Power-Up contacts. Type 9
reads the allocation-owned physical matrix directly, with sample 5564 closing
the bounded dormant-to-abort bridge; missing matrix custody remains a
fail-closed actor block. The later campaign-selector phase attempts the complete
systemic abort transaction atomically. A complete supported census commits it;
an unsupported progressed census rolls back and runs the old factory-only
bridge plus its terrain/controller suffix. Native Type8 and constructor-proven
Type93, native52/54/62/68 and factory-born Type61 now participate; remaining
unmigrated cargo constructors can still prevent general later-world abort
completion.
Their constructor and task custody must be closed before removing that
compatibility path. The outer
results/intermission consumer also remains; the bounded
E870/post-task owner is closed inside the specialized composite.

The independently recovered post-terrain controller/request suffix is live;
this does not claim that state 5 has a second, inferred gameplay meaning.
`FUN_0042E3B0` zero-fills the complete `0x1FC`-byte controller and explicitly initializes request dword
`+0xAC = 0`; no other writer was found in the owner family. Normal world setup
submits Section-13 `+0x54/+0x56`; its later runtime mode at `+0x1F0` is selected
from 1/3/4/5 and is therefore unresolved in the port before abort. Main Base
abort writes state `+0x1F0 = 5`,
retains `+0xAC`, replaces the pair with `+0x58/+0x5A`, copies `+0x4C`, `+0x88`,
and `+0x8C`, then submits all six dwords through `FUN_00433130`. Level 1's
exact abort request is `[0x0042E860, 0, 0x20, 0x650, 0x15, 8]`: palette 32 and
model zero replace normal palette 27 / model 305. The port retains this request
under a per-level-load controller lease and applies its colour to the complete
frame and terminal fog while model zero explicitly suppresses `sky1`. It
commits this suffix after the terrain-transform attempt even when no mutation
outcome exists, matching retail's ignored zero return.

The accepted world-behavior track gives a complete Type-17 abort bracket. Sample
`5560` at 27,800.512 ms / tick 1357 is the final pre-abort actor sample; with
system entries 0--3 excluded, intrusive list indices 21--24 are authored spawns
17--20. All four are model 256, health 5,000, state `0x01468825`, slot 0 only,
with null instance death hook and context auxiliary word zero:

| Spawn / list index | Actor pointer / handle | Predecessor | Context / Primary wrapper | Target / elapsed |
|---|---|---|---|---|
| 17 / 21 | `0x02A9A220` / `0x04450001` | class 9 Capture People variant 1, style `0x004C8038` | `0x02A9A5E8` / `0x02A8B920` (state `0x02A8B930`, tick `0x00403780`, pair `0x00402DA0`) | `0x04460001` / 2,748 ms |
| 18 / 22 | `0x02A9A6D0` / `0x04440001` | class 9 Capture People variant 1, style `0x004C8038` | `0x02A9AA98` / `0x02A8BBE0` (state `0x02A8BBF0`, tick `0x00403780`, pair `0x00402DA0`) | `0x04460001` / 1,612 ms |
| 19 / 23 | `0x02A9B1C0` / `0x04430001` | class 33 Follow Beacons variant 1, style `0x004C7B70` | `0x02A9B588` / `0x02A8A2D0` (state `0x02A8A350`, tick `0x00403CE0`, pair `0x00402DA0`) | `0x04550001` / 3,708 ms |
| 20 / 24 | `0x02A9B670` / `0x04420001` | class 9 Capture People variant 1, style `0x004C8038` | `0x02A9BA38` / `0x02A8B330` (state `0x02A8B340`, tick `0x00403780`, pair `0x00402DA0`) | `0x04460001` / 740 ms |

At sample `5564`, 27,820.100 ms, every row is health zero, state
`0x0147C825`, and class-12 style `0x004C7ED0`. Static metadata agrees: class-9
and class-33 variant 1 have null death hooks, while Capture People variants
2--5 use styles `0x004C8080`, `0x004C80C8`, `0x004C8110`, and `0x004C8158`
with hook `0x0040D040`. The Main Base adapter therefore admits only this exact
four-row census into the generic fresh standard-death publisher; it does not
infer class 10 or those Capture variants from the wider shared checked core.
Remote-owned and already-dying actors retain the generic no-op precedence, and
fresh mismatches fail before publication.

The atomic production transaction now applies its exact
alternate-callback adapter, the 15-actor class-2 ordinary callback, terminal
type-6 no-op, type-54 class-38 publisher/task owner, all six type-9 class-14
callbacks, the three exact type-47 class-12 publications, and the four exact
type-17 class-12 publications through authenticated live custody. The Type-47
and Type-17 abort adapters return each linear scheduler
owner and sample the successor after callback return. Live composition
continues through Type-47 spawns 11--13, Type-9 spawns 14--16, Type-17 spawns
17--20, quiet spawn 21, and Type-9 spawn 22, then reaches spawn 23/type 66;
then processes the exact Type-67, quiet, Type-61, and appended Type-60 tails.
Type 9's one-second shared-retarget owner and Type 66's progressive Working
Factory owner are adopted by the six-family specialized coordinator without
another capture. Hard-water Type-60 owners are likewise staged only after the
current pass. Main finishes the current actor-task sweep before consuming
the campaign abort flag, publishes specialized owners during the later
mutation-sensitive abort sweep, and retains every new owner for the next frame
only when that full transaction commits. Any callback block restores the prior
manager, scheduler, terrain, static-damage, hull, effect/RNG, and controller
owners before the compatibility body runs. No second task pass is run. Factory-born Type61 abort now uses that constructor receipt across ordinary
worlds. [Type68 release](CLASS0_RUNTIME.md#type68-attach-and-release)
is closed by the corrected DC50/D1C0 stack argument, and
[native Type8 class14](INTRO2_TYPE8.md#hits-and-class14-death) owns dynamic
worker death under its actual allocation. Neither requires another capture;
the remaining families retain their separate boundaries.
Presentation of the surrounding `+0x295` request is owned by `FUN_00456960`,
outside the systemic-body transaction. The port requests it after every newly
begun dispatch, including the null world-control path, and resets `+0x2BC`
before sound/dispatch. Its exact eight-frame selected-tier material sequence,
next-frame producer boundary, retry/restart state, and over-world/under-HUD
placement are live. The cursor is presentation-only and does not delay campaign
progression; the broader results/intermission owner remains open. The
synchronized retail authority is
`runtime_re/captures/local/20260724-201244-friendly-main-base-destruction/`.
