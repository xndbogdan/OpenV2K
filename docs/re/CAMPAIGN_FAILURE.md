# Campaign Failure and Casualty Limits

## Native Casualty Limit (`0042DD10` selector 0)

Section-13 campaign-record flag `0x04` tests **survivors**, using signed byte
`record+0x19` as the loss threshold. The census is controller `+0x1A8`
(capability `0x400`, scientists) plus `+0x1B0` (capability `0x800`, peasants).
`0042E210` adds every present Sub-M `+0x68` occupant count into the scientist
bucket before its live-entity gate. Ordinary capability matches require a
nonzero entity state and dying bit `0x4000` clear; a dying person stops
contributing before deferred deletion. Cargo and factory occupants therefore
must not be mistaken for deaths. Load-time native count `+0x170` is a results
statistic, not this predicate's baseline.

At `survivors == threshold + 1`, `0042DF54..0042DF74` sets controller `+0x1E8`
and queues direct string `0xC7` once per world load. At
`survivors <= threshold`, `0042DF91..0042DF9D` queues direct string `0xD4`;
the later flag-`0x80` branch calls the shared `00456960` abort. A frame may
skip the warning population entirely and still lose. Both comparisons are
signed; their population addition wraps as a 32-bit x86 dword.

All 21 authored casualty records are abort records: twenty use flags `0x84`,
and global world 13 uses `0x86`. Its extra flag `0x02` first requires peasants
at least signed byte `+0x18` (authored zero), before either warning or loss.
Other mixed-condition records must retain their preceding and following
predicates; population alone cannot authorize them. Campaign saved bit 0,
queried by `0042ED80`, or session abort byte `+0x28F`, queried by `00456CB0`,
skips abort records without resetting the warning latch. World load
`0042E570` clears that latch; per-frame `0042D9B0` invalidates census `+0xD0`
after ticking the independent trophy timer.

The NoCD05 trace loses **Castle**, global world 15/logical world 3. It emits
`0xC7` at tick `0x1741` with scientist/peasant counts `0/6`, then `0xD4` at
tick `0x1DCB` with `0/5`. Its record has flags `0x84`, threshold 5, and the
abort entry at `688029:B4F` returns to `0042E074`, then `00450335`.
Its three pre-abort Type9 deaths occur at `482A75:180C` through primary
particle damage (`00410F3D -> 004150D4 -> 00414F90`), `5AB2FD:1F74` through
the capture continuation (`0040D011`), and `688002:16B6` through the radial
burst (`004566FB -> 00414C1B -> 004150D4 -> 00414F90`). The warning follows
the second death at `5AB31F:49D`. This trace does not show drowning.
Alpine, global world 17, instead has threshold zero: warning at one survivor
and loss at zero. The trace's later attempted Alpine load is a distinct event.

The port's [casualty evaluator](../../crates/v2k-game/src/campaign_failure.rs)
and [authored-corpus tests](../../crates/v2k-game/tests/campaign_failure.rs)
retain these thresholds, message ordering and per-load latch. The caller
evaluates after actor/particle/contact work at the deferred campaign-selector
boundary. The selector retains authored record order: `42E099` returns on a
successful route before later casualty text or warning-latch writes; `42E06F`
executes the shared abort and `42E074` resumes the scan at the next record.
Its saved/abort/census values are entry snapshots, so a newly issued abort does
not retroactively change the remaining scan's skip predicate. While the entry
session-abort value is zero, a pending Main Base request (`+1F8`) is consumed
before the scan; `42DE56..42DE64` clears it before the saved-world branch,
suppresses its abort when the world is already saved, and otherwise aborts and
returns. An already-aborted entry instead tests the separate interior request
`+1F4`; when that is zero it enters the record scan with `+1F8` still pending
(`42DE17..42DE73`).
`42E300` identifies the resolved player by capability1, and `446440` tests the
stamp subtype/selectors/age without a separate hull-dying or abort gate.
Death remains with each actor's standard-death owner: Type9 E370
expiry runs relation release `00416750`, then `00410C10` and its class-14
publication; primary weapon damage reaches the same standard-death owner.
The casualty evaluator does not remove actors or manufacture a Main Base
terminal-origin receipt. The shared abort body retains callback-custody
validation and atomic rejection when an actor family is unsupported.
For a live native Main Base, the abort invokes the existing `00410C10 ->
00419750` publisher: health becomes 10,000,000, dying is cleared, a zero
Sub-M `+0x94` becomes 1, and the singleton selector republishes its task.
Its replacement owner first ticks on the next frame. Only the already-dying
Main Base which originated a terminal abort consumes the terminal-self receipt.

## Level1 failed-world aftermath

Casualty loss and terminal Main Base destruction share `456960 -> 42F1A0`;
they can reach it with different existing actor states. The dispatcher performs
the synchronous `4170A0` actor sweep, then `433E30` terrain darkening, then
controller `+1F0 = 5` and the authored replacement frame. It does not enter a
separate insect-wave scheduler. The outer header first collects disposable
logical sounds, writes session `+28F = 1` / `+2BC = 0`, requests positional
global sound `0x3E`, and finally requests `456750`'s flash sequence. See the complete
[abort ordering](FACTORY_SYSTEM.md#reactor-mechanic).

The sweep tests each current actor in live-list order: capability `+0x64 & 0x11`
selects `1CF90`; otherwise Type111 is excluded and state `+0x08 & 0x10001000 == 0`
selects `10C10`. This skip mask is not the dying bit or the deferred-removal
bit. `10C10` independently returns before mutation for remote ownership, then
for an already-dying actor; it dispatches the actual type/style death program
rather than unlinking every ordinary actor immediately.

| Level1 category | Abort callback and retained aftermath |
|---|---|
| Type46 player | Capability `0x05` selects `1CF90`; authored `ABCDEGJO` has no Sub-N, so this is an exact no-op. The sweep does not reset hull, installed Targetter, weapons or ammunition. |
| Type67 Hive | Capability `0x18` selects `1CF90`. Retain health, current live/dying controller and tasks; write Sub-N `+0x50 = -3_000_000` microseconds. A retained marker clears its 3x3 terrain attributes. This does not kill or unlock the Hive immediately. |
| Live Type6 Main Base / Type66 factory | `10C10 -> 19750` revives health to 10,000,000, clears dying and starts/retains positive Sub-M `+0x94`; the real class41/39 task is reselected. Later `19B50` advances 100,000-us effect stages and reenters death at stage32. The same allocation selects its authored wreck slot (Level1 model225), clears staffing/required count and installs class0; it is not a replacement birth. |
| Already-terminal Main Base/factory | The generic already-dying return preserves its current wreck and does not restart progressive destruction. In a Main-Base-caused abort this is the origin's authenticated self revisit. |
| Type9 villagers | Fresh local death publishes alternate class14, Exploding Person; its 1,000-ms task has strict expiry and later stages deferred removal. Dying villagers already stopped contributing to the casualty census. Session-abort mode suppresses class14's optional normal-death C6 text, not its death program. |
| Type17 spiders / Type47 gunboats | Authored alternate class12, Common Dying, owns the corpse/motion/lifecycle continuation before removal. These are neither preserved living enemies nor immediate quiet deletion. |
| Type52 flags / Type62 fish / Type68 weight | Alternate class2 clears Primary, Secondary and Tertiary, then `10B70` stages deferred removal. Type68 additionally emits its authored death cue. |
| Type54 grock | Alternate class38 performs its burst and Change-Sea-Level continuation. Terrain transformation still follows the complete mutation-sensitive actor sweep. |
| Uncollected Type61 pickups | Spawn32 Targetter (selector `0x3C`), spawn33 trophy (selector `0x3F`, model138), and spawn34 flare-gun ammunition (packed `0xC802`: selector2/count200) all run alternate class49: burst, static/dynamic radial, task clear, Type60 ring attempt and deferred removal. Their world allocations disappear; already acquired inventory is independent. Factory-produced Type61 uses the same death family when reached. |
| Type60 rings appended by those explosions | The same forward sweep visits linked tails and runs alternate class2, removing their entity-backed ring and staging deletion. |
| Type111 exit markers / suppressed system actors | Type111 is explicitly excluded; Types0/1 are suppressed by the state mask. No generic destruction is inferred for them. |

The pickup/deferred-death details belong to [Type61](TYPE61_POWER_UP.md),
[Factory System](FACTORY_SYSTEM.md#reactor-mechanic), and the corresponding
actor death owners. Removing an uncollected flare pickup does not remove an
already installed flare descriptor. The failure header contains no inventory
transaction; later world/session teardown is a separate boundary.

The subsequent Hive callbacks retain their source order: infection, live
health/lock update, detailed class5 spit, then state1-only creature records.
Level1 Hive spawn24 has one record `[15,0,2000000,1000000,3,3,3]`: at most three
successful Type15 Wasp births, abort-only, with three tracked simultaneous
children and objective flag1. It has no ordinary-mode birth row and does not
select Type87 Deathwas. Row timers/counts are not reset by `1CF90`; birth
requires the Hive still to be in state1 after that callback's health prefix.
An already-unlocked state2 Hive remains vulnerable and emits no creature rows;
the abort does not relock it. Newborn capability `0x08` / objective bit `0x01000000`
actors enter `15120`'s hostile predicate even during temporary state `0x8000`
suppression. Killing the remaining objective actors permits the retained
two-second unlock grace; the lifetime production cap prevents replacement
births once three successes have been counted.
The wrapping jitter, constructor/ejection order, temporary pair suppression
and later unlock conditions are owned by [Hive wreck](HIVE_WRECK.md).

Failure is not a blanket health-damage disable. Retail `415040..415120`
requires a present target, state `0x8000` and a packet, then runs `255E0` and
`14E90`; neither checks session abort. `14E90` retains buffer writes but skips
health subtraction and lethal redispatch while dying. Thus a progressive Base
or factory with dying cleared still receives ordinary filtered damage; its
later dying wreck does not regain health damage. The surviving Hive retains
its health lock/unlock policy, and the player retains the normal hull owner.
These entry rules do not guarantee a particular weapon can overcome a target's
profile or lock, or that every attack/contact owner is implemented in the port.

The casualty-started Main Base can later reach stage32 and set `+1F8` through
`10C10 -> 456DB0 -> 42F160`, but an already-aborted `42DD10` entry bypasses
that pending request. It does not repeat the sweep and kill the newborn Wasps
through this route. `456960` itself has no abort-byte reentry guard; a distinct
direct caller must be audited separately before generalizing that suppression.

Failed-world Hive death cannot mark the world saved: `4567B0` requires session
`+0x28F == 0` before writing its completion timestamp and calling `42EE70`.
Its wreck-interior entry instead runs `456D10 -> 42F140`, raising controller
`+0x1F4`; the already-aborted selector returns current-world `+0xC4`
(`42DE20..42DE38`). `44FFA0` copies that output to session `+0x34` and sets
`+0x290` (`450344/450347`), establishing current-world retry routing rather
than the static marker's next-world route. The [loading authority](LOADING_TRANSITIONS.md#level-completion-flow)
owns the subsequent handoff; this static suffix does not accept visual warp,
reload timing or inventory persistence through that reload.

Static authority is the normal-tier Level1 Section12/13 corpus and the retail
callbacks in game_logic.c, with original-PE
`42DE17..42DE73`, `410C10..410D20`, `415040..415120` and `41C336..41C546`
for omitted/ambiguous C boundaries. The accepted Main Base destruction bundle
establishes that trigger's live cohort/sweep; NoCD05 establishes Castle's
casualty trigger. Neither alone accepts a complete Level1 casualty aftermath.
The capture ledger
owns those scopes. Exact fresh-Level1 abort cleanup and the
[Sub-N creature-row runtime](HIVE_WRECK.md#authored-creature-births-and-failed-world-selection)
are implemented, including native Wasp construction/task admission and
`1C830/1CA90` child ejection custody. Controlled production OpenGL checks use
normal-tier Level1, fixed40-ms callbacks, a living player and real source46
hits to trigger Main Base destruction or casualty loss. Both execute the exact
actor transaction, retain the original Main Base/factory allocations through
their staged wreck transition, remove world pickups while preserving a
collected weapon, and produce three objective Type15/model276 Wasps. These
native children now run the same late solid/water/static prefix as Intro2 in
the retained Playing world, before active pairs. The prior Playing dispatcher
omitted this family and allowed admitted Wasps to pass through solid terrain;
[source contact](FLYING_SURFACE_CONTACT.md) owns the shared response and retained
entry-model rules. The half-second source ejection exclusion remains unchanged.
After lethal player hits retire those native owners, the five-second grace timer
allows state2 and the vulnerable-Hive message. These controls validate the
running port's aftermath, not matched retail timing, interactive combat or
acceptance of the complete casualty scene. The subsequent controlled loops use
the existing projectile Hive death, wait for its wreck clock and enter the real
interior predicate. Both reach a distinct current-level retry, retain the
unsaved bit and collected weapon through the map, and rebuild fresh authored
buildings, pickups and Hive rows. The full controller/inventory/cargo checkpoint
controls live in [failed-world retry](../../crates/v2k-game/tests/failed_world_retry.rs).
`4567B0`'s abort/timestamp guard owns both the completion stamp and `2EE70`
campaign transaction; testing only the stamp had left an erroneous saved-world
write in the former main-loop glue. The same source guard now covers both.
Matched retail combat, aftermath timing and warp presentation remain open.

## Campaign-wide transition and actor-cleanup coverage

Casualty loss is selected from every loaded world's Section-13 records; Castle
is the matched NoCD05 example, not a level gate. The shared failure transition
is now exercised across all 38 authored OVLs13..50. OVL13..48 have the 36
playable campaign slots; OVL49/50 have no campaign control slot and no casualty
record. All 21 casualty worlds reach controller state5 and the authored abort
frame through either the exact actor transaction or the explicit bounded
world-state fallback.

The strict actor sweep still rejects atomically when a native callback owner
is unavailable. The shared [fallback policy](../../crates/v2k-game/src/main_base_abort_production/fallback.rs)
retains the earlier interactive behavior: known factories arm and queue their
authored death sound, terrain transformation consumes its own RNG, and the
controller enters state5 with the replacement sky/frame. A null or previously
applied terrain result does not suppress the controller handoff. The original
callback diagnostic stays visible. This does **not** implement the missing
actors' deaths, explosions, release callbacks or nested effects.

The [coverage regression](../../crates/v2k-game/tests/campaign_abort_coverage.rs)
constructs every campaign world with its actual authored actors and scheduler
adoptions. Exact rollback checks cover actors/tasks, terrain, controller and
notifications. An independently constructed control world runs the former
inline fallback to compare factories, sound, terrain output and the next shared
RNG sample. The fixture starts from fresh construction; it does not prove every
later runtime graph or predecessor history.

| Fresh-world exact sweep | Global OVLs |
|---|---|
| Complete | 13, 14, 15, 44 |
| No campaign controller or casualty record | 49, 50 |

The remaining first unsupported ordinary callbacks are listed below. Each is
the first blocker in the real authored live-list order; resolving it can expose
later missing families. These are implementation boundaries, not evidence that
retail skips those actors.

| First unsupported entity type | Global OVLs |
|---|---|
| 13 | 46, 47, 48 |
| 14 | 20 |
| 16 | 24 |
| 18 | 21, 22, 33, 36 |
| 25 | 23, 34 |
| 27 | 27, 35 |
| 31 | 16 |
| 35 | 41 |
| 40 | 17, 19, 32, 45 |
| 71 | 28 |
| 73 | 18 |
| 76 | 26 |
| 77 | 39 |
| 82 | 25, 38 |
| 83 | 30 |
| 102 | 37 |
| 103 | 29, 43 |
| 107 | 31 |
| 108 | 42 |
| 125 | 40 |

Thus exact fresh-world actor cleanup is complete for three of the 21 casualty
worlds (13/14/15); the other eighteen use the explicitly bounded transition.
The authored-threshold regression additionally verifies all 21 warning/loss
boundaries, saved/aborted suppression, per-load latch reset, scientist-plus-
peasant census, and the ordered selector's warning delivery.
