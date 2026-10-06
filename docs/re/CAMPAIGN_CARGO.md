# V2000 Campaign Cargo Reconstruction

This document owns conventional cargo crossing a campaign world boundary.
[LOADING_TRANSITIONS.md](LOADING_TRANSITIONS.md) owns campaign routing and
controller preservation; [ACTOR_RUNTIME.md](ACTOR_RUNTIME.md) owns native
construction, task custody, and relation callbacks. The source below is the
local retail EXE, including routines omitted from the bulk decompiler index.
No new runtime capture is needed to establish these branches.

## Saved representation and termination

`FUN_00443260` clears the controller's raw unlocked cargo slots at `+0x19C`,
then walks direct Sub-J attachments in slot order. A resolved live, non-dying
actor contributes its type dword at `+0x58` plus its **unsigned word** at
`+0xB4` shifted left 16. Invalid or dying entries leave their cleared slot zero.
The final 99-dword copy starts at controller `+0x74`: cargo therefore appears
at saved-profile `+0x128`, or session `+0x160` when that profile starts at `+0x38`.
These entries contain no actor health, velocity, animation, task graph, or
other mutable actor state. Cargo under a Type 93 drop proxy is not on the
player's direct attachment list and is not serialized by this pass.

`FUN_00451C00` tests the **low 16-bit type**, first at `00451D1D` and again after
each entry at `00451E21`. The first zero type ends the list even if that
dword's high 16-bit stamp is nonzero or later slots contain nonzero types.
Do not compact holes or resume scanning after the terminator.

The switch at `00451D32..00451D3F` uses byte table `00451F38` and jump table
`00451F18`. The following decimal types advance to the next entry without
matching, constructing, or attaching anything:

| Excluded types | Retail case destination |
|---|---|
| 7, 8, 9, 78, 79, 86, 90, 91, 95, 116 | `00451E1B` |

These exact person types are deliberately excluded from conventional campaign
cargo reconstruction. Their ordinary-world native constructors do not authorize
carrying them across this boundary. Other nonzero types reach the generic
match/miss path; types greater than 116 reach it directly. This is an exact
switch policy, not a rule excluding every actor with person-like behavior.

## Eligible authored families

The canonical normal-tier Section-12/13 census of gameplay overlays 13 through
49 contains 21 types with the beam capability bit `0x1000`. Removing only the
ten switch exclusions leaves the following eleven types; each also has actual
authored births. Headers for these types are invariant across that corpus.
The class column gives authored weighted choices, not proof that a particular
choice executed. Counts establish reachability, not native runtime completion.

| Type | Default model | Authored class choices | Births | Gameplay overlays |
|---|---|---|---:|---|
| 49 | 266 `rover` | 68, 42 | 14 | 15, 21, 26, 31, 35, 39, 40 |
| 68 | 81 `weight` | 0 | 44 | 13, 14, 16–18, 20–23, 25, 27, 29–30, 33–34, 39, 43 |
| 74 | 253 `vaporise` | 59 | 3 | 21, 31, 36 |
| [83](PORTABLE_RADAR.md) | 79 `tinyscan` | 60 | 14 | 30, 39–40, 42, 46–47 |
| 92 | 171 `tulazred` | 29 | 3 | 26, 38–39 |
| 96 | 175 `tulazgre` | 29 | 2 | 26, 36 |
| 97 | 173 `tulazblu` | 29 | 8 | 31, 42, 46–47 |
| 100 | 152 `tucannon` | 29 | 1 | 26 |
| 107 | 24 `tanksmal` | 7, 29, 29 | 2 | 31 |
| 121 | 28 `tankbig` | 7, 29, 29 | 3 | 26, 31, 40 |
| 123 | 889 | 45, 6 | 11 | 49 |

Type 123 is the concrete counterexample to a broader person exclusion: its
capability word is `0x1804`, its choices are AttractAttention/Wander, and its
type exceeds the switch boundary. It reaches ordinary matching/reconstruction.
Type 52 has no beam capability and is absent from this list.

## World initialization order

`FUN_004515E0` calls `FUN_00451710` at `004516C8`. After successful
`493E40(3)`, the latter performs this order:

1. Reset session word `+0x2D4` to zero at `00451763`.
2. If `450390(session)` is false, call `451AF0` to create the player through
   `4438A0 -> 438080 -> 40D720 -> 4104B0`. Otherwise call `456C20` at
   `00451797` and discard its result: the skipped player still consumes one
   stamp. `450390` is true for signed phase `< 4`, or phase 4 with `+0x34 == 11`.
3. When session `+0x278` is nonzero, call `42E570` at `004517AB`. Its authored
   Section 13 loop visits indices 0 through count - 1 via `42E440 -> 438080`.
   Completed-world death/removal policy runs **after** each birth, so those
   actors still consume stamps. `42EFB0` then removes duplicate exit markers;
   that cleanup creates no actors and consumes no additional stamp.
4. If `450390` is false, invoke `451C00` at `004517C9`, after destination
   authored actors exist. Successful restoration is followed by `443260`
   at `00451800`, refreshing the saved controller profile.

The `+0x278` gate matters: a load initializer may skip authored population
while still resetting the stamp counter and creating/reserving the player.
The initialization sequence is not an arbitrary successful-allocation census.

For native Begin Intro, the logical index is explicitly **38 (`0x26`)**.
New Game `42BBF0` records mode2; `42CFA0 -> 42CF40` passes it to `44F650`.
That function's non-4/6/7 branch sets its controller-index argument to `0x26`
and calls `42E3B0`, which stores it at controller `+0xC4`. The separate session
`+0x34` becomes1, the following gameplay world; it is not the current stamp
index. `44F8B0` stores phase2 at session `+0x296` and selects descriptor
`4D0918`. Consequently `451710` takes the skipped-player reservation branch:
stamp `0x9800` is discarded, and the first authored body receives `0x9801`.
`42E570` independently resolves the current overlay as `38 + 12 = 50`.
These are static constructor inputs, not an inference from the overlay number.
The chain is retained in game_logic.c,
menu_item_callbacks.c, and
menu_system.c.

## Entity stamp producer

`FUN_00456C20` calls `450C90(NULL)`, which reads the world controller's
`+0xC4` through `42EB00`. This is the logical world index, not the gameplay
overlay id or bounded trophy-array slot. Ordinary overlays 13, 14, 39, and 49
map to raw logical indices 1, 2, 27, and **37**. The current progress helpers
reject index 37 because their array bound is `< 37`; their `None` must not
become stamp world zero. The source getters impose no such bound. A null
session/controller source state returns zero through its own explicit branch.
At `00456C31..00456C46` the producer computes:

```text
old = session.counter_2d4_u16
value = (logical_world_index << 10) + zero_extend(old)
session.counter_2d4_u16 = wrapping_u16(old + 1)
```

`4104B0` calls this producer at `004104CA`, **before** body allocation
`4572B0`. Allocation success writes `value as u16` to entity `+0xB4` at
`004104F1`; failure still consumes the counter value. Component construction,
the type initializer, and intrusive publication happen later. Their failure
cleanup does not refund the stamp. Nested or later dynamic common-body births
consume the same counter in actual call order.

There is no 10-bit counter mask: the full previous u16 participates in the
sum, and both the counter update and stored stamp wrap at 16 bits. This is
separate from entity handles, port allocation leases, and the process Sub-D
seed counter. Session initialization `44FA30` explicitly zeros `+0x2D4` at
`0044FB4B`; initial session allocation `44F960` also clears the whole record.
The other explicit reset is the world initializer's `00451763` above.

The integration boundary is entry to `104B0`, not a successful tail append or
every outer `438080` request. Type 51 can fail an outer controller allocation
before `D720/104B0`, consuming no body stamp. Read-only plans and metadata
admission failures also do not establish that retail allocation was entered.

| Body path | Stamp attempt and retained prefix |
|---|---|
| Player | Create through `104B0`, or reserve through the explicit `451797` branch, once. Updating an existing player's arrival pose consumes nothing. |
| Authored actors, including Intro2 | Charge in the actual Section-13 constructor order before components and selector RNG. Preparing generic templates ahead of native initialization must not stamp future bodies after an earlier constructor failure. |
| Type 61 factory product | Charge before its singleton behavior selector, not at successful publication. |
| Type 8 factory/Main Base replacement | Charge before Sub-D/Sub-A construction and selector RNG, after the source caller's committed destruction/notification prefix where applicable. |
| Type 93 factory, player-drop, or missing-relation proxy | Read-only relation plans consume nothing; the actual local constructor consumes before its singleton selector. Carry one attempt receipt through later publication; remote requests that do not construct locally consume nothing. |
| Type 60 explosion/hard-water ring | Raw-body/component failure, post-selector failure, and successful task-allocation fallback all retain the entered body's stamp. Their RNG and publication outcomes remain different. |
| Campaign match/miss | A match consumes nothing. A miss consumes at zero-record construction, then overwrites only successful newborn `+0xB4`. |

The port's [construction stamp owner](../../crates/v2k-game/src/entity/construction_stamp.rs)
represents the session word in the one loaded EntityManager. The ordinary
native load request supplies its raw logical index explicitly and initializes
the counter before player creation/reservation and authored construction.
`begin_common_body_attempt` consumes independently of successful publication;
the read-only `next_common_body_ordinal` exposes no allocation side effect.
Each Entity retains `construction_stamp_at_0xb4` separately from its allocation
lease. Generic snapshots retain unavailable lineage and stamp values; zero is
a valid known source value. The test-only setter establishes synthetic counter
lineage without retroactively authenticating existing fixture bodies.

The counter and body words follow the manager's Main Base transaction fork:
discarded speculative work cannot advance the accepted list's lineage.
Resource-cache loading and effect teardown are not reset owners. Task
replacement, removal and ordinary attachment preserve the existing stamp;
campaign restoration owns its separate saved-word overwrite after a real miss.

## Matching, birth, and attachment

At `00451D46`, restoration takes the packed dword's high half using **SAR by 16**
(`00451D4A`). It walks the current intrusive entity list from `414920`, first
comparing that signed 32-bit result against the **zero-extended u16** entity
`+0xB4` (`00451D5A..00451D63`), then comparing the low 16-bit type with entity
`+0x58`. The first match wins. No active/dead predicate is added to this scan.
Consequently a saved stamp `>=0x8000` cannot match: its sign-extended high
half differs from every zero-extended entity word. Preserve this asymmetry.

A match calls `443B30(controller, matched_handle)` at `00451D89` and consumes
no allocation stamp. A miss zeros the spawn record, sets its requested type
and the default allocation-handle argument `DAT_004DCA00`, then calls
`438080` at `00451DD1`. Its authored position, velocity, Euler angles, and
optional constructor parameters are zero; the arrival pose is not substituted
for that constructor input. Normal native construction must run first.
`00451DFC` then overwrites the newborn's word `+0xB4` with the saved high 16 bits
and calls `443B30` at `00451E0B`. The consumed counter is not rewound or set
from the old stamp.

`443B30` forwards the controller's real player handle to `443B50`. A match
and a newborn use the same complete attachment transaction below. An error
stops restoration at the source call, retaining its committed prefix. There
is no aggregate capacity preflight before the list: a miss constructs and
overwrites the newborn's stamp before attachment can return a full-slot error.
Earlier attached entries and that unattached newborn remain committed. Do not
replace an unsupported, dying, attached, or pending first match with a later
match or a fresh birth; those are callback-admission questions after identity
selection.

### Shared attachment transaction

`443B50` owns this order for both campaign restoration and live collection:

1. Call `18440` at `00443BA3`, passing row callbacks `443D30` and `443D10`.
   The slot helper checks capacity before duplicate identity, writes the row,
   clears child state `0x800` when the selected slot's first byte is zero,
   sets `0x20000000` when Sub-J `+0x0C` is nonzero, and increments the length.
2. Call `16700` at `00443BBF` with the child, actual parent handle, and authored
   slot offset. It writes child state `0x1000`, then parent `+0x80`, then calls
   the child's type-table `+0x7C/+0x44` callback. A callback error retains the
   published slot and relation prefix and skips the following success suffix.
3. After callback success, child capability `0x800` requests resource event 0
   at `00443BEE`; Type 68 requests resource event `0x15` at `00443C11`.
   These are the deduplicated `4568B0` notification owner, not direct text.
4. Clear child state `0x40000` at `00443C2C`. If the parent's capability mask `0x1`
   is set, also clear child `0x2000` at `00443C5D`, construct particle class
   `0x33` at the child's current raw position with zero velocity and the parent
   owner (`00443CBD`), then request sound 8 there with unity gain and rate
   (`00443CE8`). `4575A0` consumes the sound result; `443CF9` returns success.

Type 68's native class0 path reaches `DBF0`, whose style `4C7468/+0x08` is
null. Thus real attachment preserves its `Class0Timer` and existing task ages;
it does not install Type 8/9's carried animation task. The native constructor
receipt, exact current graph, and pending-prefix custody remain required by
the port adapter. [CLASS0_RUNTIME.md](CLASS0_RUNTIME.md) owns that behavior.
Neither direct parent-id assignment nor a copied task context reproduces the
slot/callback/success-suffix transaction.

### Load-time notification policy

`4568B0` accepts a resource event only when session `+0x296 == 5`; direct
`456900` requests require a phase greater than 4. First-world handoff writes
phase 5 at `00453E67` (except raw world `+0x34 == 0x25`), then clears
`+0x28E` at `00453E74` before dispatching the game descriptor. Campaign warp
`456D10` clears `+0x28E` at `00456D34` and preserves the current phase.

After successful `451C00` restoration, the ordinary `+0x28E == 0` branch of
`451710` clears both visible notification slots with `437F30(slot, 0, 0, 0)`
at `0045189C` and `004518AE`. That helper writes only the string, parameter
and timestamp words; it does **not** clear resource-slot `+0x0C`, the seen-event
mask. Restoration events 0/`0x15`, and authored `BA40` construction hints, can
therefore consume deduplication bits without appearing in the loaded world's
HUD. The later suffix selects music, primes Klaus/camera, copies the world
name through `451FB0`, and sets graphics/environment state; it seeds no further
notification. The `+0x28E != 0` respawn branch instead calls `438040` at
`00451885` to rebase only the direct slot's timestamp, retaining both messages
and the resource mask.

The mask has session-selection lifetime, not per-world lifetime: `44F650`
calls `437FF0` at `0044F7E1` through frontend `42CF40`, before `44F8B0` enters
the selected session. Ordinary campaign warp does not enter that reset owner.
The port therefore preserves the mask across `CampaignWarp`, and clears only
visible slots after native restoration and the deferred authored-hint drain.
New session setup uses `GameplayNotifications::reset_session`.

Load callbacks use the existing clock. `451740..451750` marks the normal
load's session `+0x274` reset flag and clears `+0x270`; the following `4FE20`
visit consumes that flag and calls `428AD0` to reset `DAT_004FED60`. Keeping
restoration before the port's post-load clock reset preserves this order;
rebasing a restoration hint for display would bypass the source slot clear.

## Port ownership and remaining boundaries

The [identity owner](../../crates/v2k-game/src/entity/campaign_cargo_identity.rs)
now serializes each valid direct row with the full wrapping type-plus-stamp
addition. It retains cleared unlocked slots and invalid-row holes. An unavailable
fixture stamp preserves its known type as `UnresolvedStamp`; it never becomes
a guessed zero. Matching uses the complete allocation in intrusive order with
the signed high-half comparison above. An unresolved earlier candidate of the
same type blocks selection; a known different type cannot match. A negative
saved high half proves a miss independently of every candidate stamp.

The [restore owner](../../crates/v2k-game/src/entity/campaign_cargo.rs) applies
first-zero/type exclusions, then consumes each identity sequentially. Type68
matches retain the actual allocation, pose and task. A miss enters the shared
[zero-record constructor](../../crates/v2k-game/src/entity/class0_actor/zero_record.rs),
including terrain-at-zero grounding, temporary body stamp, singleton RNG and
native task publication. Saved B4 is overwritten only after construction, before
the shared [attachment owner](../../crates/v2k-game/src/entity/cargo_attachment.rs).
That owner supplies collection notification requests, state writes and deferred
class33/sound8 presentation for both Playing and Loading; Playing does not
emit a second copy from `BeamOutcome`. Loading applies the subsequent visible
slot clear above while preserving the requests' deduplication effects.

Repeated identities still select the same first allocation. Capacity precedes
duplicate admission; a successful repeated attach preserves the class0 timer
but still reaches its relation/null callback and success suffix. A dying,
attached, unsupported or pending first match is never replaced with another
candidate or a fresh birth. Dying bodies without living task custody remain a
callback boundary, with slot/relation writes retained if that phase was entered.
Foreign allocation and already pending host-prefix rejection happen before
entry; these are evidence/custody blocks, not invented retail callback results.

Type92/96/97/100 class-29 E/L match reuses the destination authored identity; a
miss runs the recovered D190 constructor at the zero-record pose with that
type's Section-12 model/emitter/damage, then the same `443B50` membership
attach. Type123 people join that path through their native Always-weighted
root: a miss runs the shared BA40/AD10 birth grounded at the zero-record pose
with the divisor-32 Sub-D, Always-only choices and cues56/95/74, retains any
BA40 receipt for the canonical load drain, and attaches through the
16700/DBF0/CE70 carrying adapter with the player as parent; the beam can drop
them back through CE90 reselection. Type 68's null style-`+08` path is unchanged.
The [Type49 rover](CLEANSING_VEHICLE.md) now supplies its native class68/42
constructor and attachment lifecycle. Other eligible constructors
(74, 83, 107, 121) remain unsupported explicitly. The port does not yet own
vaporise59, tinyscan60, or tank class-7/29 (D190 is the singleton turret, not a
tank), so 451C00 must not invent them. The session+184 auxiliary list keeps its
own controller-operation reconstruction, which has no evidenced Type123 entry.
The [Type83 Portable Radar source matrix](PORTABLE_RADAR.md) recovers C560,
its deployment/destructor task, and the ordered coverage/carry/death/abort
contracts. Native task custody and shared radar ownership remain unimplemented;
this evidence does not admit its constructor or cargo restore.
Source sequential errors retain earlier attachments, the newborn and consumed stamp
where applicable. Main Loading stops, unloads the failed destination and returns
to the frontend instead of continuing with partial generic cargo or uncertain
future construction history. The validation below covers this admitted scope;
it does not certify every eligible family.

Ordinary native loads run the following source-closed phases even for empty
profiles. Intro2 retains its separate skipped-player/load policy:

- Before the list, `00451C36..00451D06` rebuilds player surface flags through
  `445920` at the current tick, then raises raw Y to at least
  `41DE60(terrain, X, Z, 0) + 0x200`. The signed surface comparison sets
  `0x200000` below, `0x400000` above, neither on equality. These flags describe
  the position before the Y raise and are not recomputed afterward.
- After it, `00451E2F..00451EDC` checks `42EB50(controller) == 6`
  (world-record `+0x48`, not the logical world index). With a player Sub-J and
  attached mass from `4185C0` below 200, it constructs one zero-record Type68
  ballast through `438080` and attaches it through `443B30`. This is exactly
  one birth, including when the conventional list is empty, with no saved
  stamp overwrite. `4185C0` wrapping-sums resolved direct rows' unsigned
  `+0xB0` masses without a dying filter or recursion; the threshold comparison
  is signed. Construction precedes the slot-capacity error, retaining the
  newborn and consumed stamp if attachment fails.

The auxiliary phase is implemented for recovered constructors:

- Finally, `00451EDE..00451F0A` walks the separate session `+0x184` dword list
  until a whole zero entry, calling `413600(player, (entry << 8) + 0x38)` for
  each value. `43260` builds it from the controller's separate `+0x220` owned
  list into `+0x1C0` (`004433B0..00443415`), compacting live/non-dying types and
  writing no stamps. The profile copy relocates that to session `+0x184`.
  Controller operation `0x38` constructs at the player's current position,
  velocity, and Euler angles, then assigns owner `+0x60`; it is separate from
  zero-record conventional cargo. Its non-cargo branch manages the auxiliary
  owned list, including Type 64 uniqueness and a 15-object limit, while its
  cargo branch can use `443B50`. The final `451C00` loop does not inspect the
  operation return before advancing. Recovered Type52/68/92 bodies are
  admitted; an unrecovered type continues the walk without fabricating success.

## Validation

The [native cargo integration tests](../../crates/v2k-game/tests/campaign_cargo_native.rs)
cover authored reuse, zero-record construction, saved-stamp overwrite, signed
high-half misses, first-match/error prefixes, slot capacity, Type92/96/97/100
match/miss, Type123 miss/drop/rematch with CE70 attach and CE90 living release,
auxiliary `0x38` reconstruction, the player surface
prefix and style6 ballast across all 37 ordinary normal-tier worlds. A real
13-to-14 weight transfer drops, settles and can be collected again through the
same native owner. Unit controls cover identity holes and wrapping, failed-birth
counter/RNG consumption, shared callback/particle ordering, hint persistence,
and selected Type9 adoption after class0 load ownership. Existing stamp tests
retain the skipped-player reservation and dynamic-birth controls.

`repo-check.ps1 -Mode Full -Scope V2000` passes all 4,214 tests plus corpus,
linker, format, script and documentation checks; the dev game build also passes.
Hidden OpenGL runs cover both fixed and varied frame schedules through the
intro/frontend/later-world controls, plus three 20-ms cargo views per run:
restored HUD, live drop and settled release. Both runs produce 59 frames and
zero unresolved runtime reports. Of the 56 previous controls, 53 remain byte
identical in each schedule, including every Intro2 frame. Three first-world
handoff/reveal frames reflect the recovered player ground-clearance prefix;
visual inspection confirms that change and the restored weight's HUD/model
transitions. These are port regression controls, not matched-retail acceptance.
