# V2000 Virus Spread System

## 18. Virus Spread System (PARTIALLY RUNTIME-VALIDATED)

### Project Codename

Game's internal name was **"virus2"**, confirmed via PDB-leak source path `c:\Coding\virus2\code\common\objtable.c` at `v2000_exe_strings.txt:927`. The shipped title "V2000" / "Virus 2000" matches. `objtable.c` is the generic object/entity slot allocator (assertion strings at v2000_exe_strings.txt:925–934 include "Unable to find a free slot for an object", "Owner num %d and num tables %d", "slot %d table %d max slot %d") — not virus-specific, but a strong indicator that the project was authored around the virus mechanic.

### Architecture (CONFIRMED — two cooperating layers)

V2000 has both a **terrain infection grid** and **named entity behaviors**. A
type-67 hive component emits class-5 particles which infect terrain cells on
surface contact; the same component owns a second, lower-frequency evolution
path that can set or clear grid cells and enqueue cardinal propagation. Named
behaviors are also real terrain writers: `"Defecate Virus"` installs the shared
terrain-contact component in set mode 5, while `"Cleansing Landscape"` installs
it in clear mode 6. The accepted Intro2 actor capture binds `Defecate Virus` to
type-26 handle `0x047E0001`; other actor/level assignments remain open.
`Cleansing Landscape` is bound to the native Type49/model266 rover by passive
and exact TTD evidence; its construction, movement, cargo and damage owners
are implemented. [Cleansing Vehicle](CLEANSING_VEHICLE.md) owns that contract.
Selector7's independent player cure-weapon binding and supported production
owners are established by retained executable/descriptor evidence in
[the player firing contract](PLAYER_CRAFT.md#fire-both-modes); matched retail
firing remains open. These named behaviors are not evidence that the global
grid is absent. Each actor carries a
mutable behavior-style assignment. Its ordinary
scheduler work and its active-pair contact hook travel through distinct
type-vtable entries; behavior style `+0x18` must not be described as a per-frame
tick.

General behavior dispatch, the named-behavior table, ordinary actor surveys,
task programs, common mover, Sub-D provenance, authored selection, and
process-shared RNG policy now live in
[ACTOR_RUNTIME.md](ACTOR_RUNTIME.md). The virus-specific writers remain here:
`Defecate Virus` installs terrain-contact mode 5, while `Cleansing Landscape`
installs mode 6. Their proven actor bindings and unresolved player-weapon
bindings are retained in the actor document and capture ledger.

### Intro flower (NATIVE FIRING AND CONTACTS IMPLEMENTED)

The infected sunflower is **Type115, Intro2 spawn61/model332**. Its Gun Turret
component fires method16/class50. Ordinary class50 ground contact emits
class5 droplets; those children infect terrain through the shared surface
tail. The accepted actor-AI recording confirms flower-owned class50 births
and its later switch from Gun Turret to class0. The native flower allocation,
both weighted living tasks and all three class50 contacts are implemented. Exact
weighted choices, emitter data, callback rules and coverage limits belong to
[Type115 virus flower](TYPE115_VIRUS_FLOWER.md). Neither the Type67 hive nor
Type26's Defecate Virus is a substitute for this actor.

### Hive visible-actor audio

The sound heard around the Intro2 Hive/flower views belongs to the Hive,
Type67, rather than the [Type115 sunflower](TYPE115_VIRUS_FLOWER.md). The
normal-tier Section12 record has sound selectors`[99,0,0]` at`+94/+96/+A0`,
signed periods`[500000,0]` at`+A4/+B0`, and authored health2000. Logical99
aliases direct PCM37 (`sound_037.wav`,111688 PCM bytes) at fixed frequency
`0x6666/0x10000` and full gain, without pitch variance. This makes the sample
approximately2.5 times longer than playing its WAV at the native rate.

`0040DCA0` runs its sound gate after the task cursor, hence after the Hive's
`25EA0 -> 1BEB0` Sub-K/infection/radial work. At or above half authored health,
entity state`+08` bit`800` must be set. It draws one shared low16 RNG word and
requests99 when `((dt_us << 6) / (500000 >> 10)) >= rngLow16`. Below half
health the zero`+A0` selector is silent and consumes no sound RNG. Coarse
`0040E870` has no equivalent cue. This is a positional one-shot chance per
detailed callback; it is neither a persistent Hive loop nor a flower-shot
sound nor a camera-timestamp event. Ordinary-world Hives share this policy.

The accepted passive `20260717-032945-menu-intro2-level1.jsonl` observes
PCM37-sized voices at fixed Q16 rate26214. The reproducible summary command
from the repository root is:

```powershell
.\.venv\Scripts\python.exe runtime_re/scripts/summarize-intro-hive-audio.py runtime_re/captures/local/20260717-032945-menu-intro2-level1.jsonl --output .tmp/intro-hive-audio-summary.json
```

The passive sample identifies PCM size/rate and positional gain/pan, not the
retail caller or every audio admission. Canonical Rust resource tests separately
prove Type67 is the sole authored selector37/99 owner in the loaded pool and
that Type115's presentation and emitter sound fields are zero. Source proof is
`objdump -d -Mintel --start-address=0x40dca0 --stop-address=0x40df70 v2000/V2000-nocd.exe`;
the relevant gate spans`40DD2E..40DE7D`.

The port's retained Hive pass now queues this gate after its component work,
through the same process RNG and ordinary fixed positional-sound drain. The
policy remains data driven and respects enabled/detailed/visible admission.
Regressions in
[`intro_flower_audio.rs`](../../crates/v2k-game/tests/intro_flower_audio.rs)
cover alias/canonical ownership, inclusive chance, silent/coarse/disabled
branches, entity-center origin and infection/radial-before-cue RNG ordering.
The retained Hive component adapter still receives its existing frame delta
and presentation-derived detailed/coarse classification; this repair does not
claim globally matched scheduler/RNG history or a matched audible whole-scene
recording.

### Insect blood and corpse hits (SOURCE-PROVEN)

The primary-hit wrapper `FUN_00410EB0` has an infectious-blood suffix separate
from the accepted-hit sound. After checked damage `FUN_00415040` returns
nonzero, capability byte `entity+0x64` bit `0x08` requests class 5 at scale
`0x0800` through `FUN_00440DC0`. It uses the post-hit model selected by state
bits `0x2000/0x4000`, including a newly selected corpse model, and copies the
owner's sign bit. The complete hit/callback contract belongs to
[Entity Damage and Death](ENTITY_DAMAGE_AND_DEATH.md#primary-hit-wrapper-fun_00410eb0-statically-confirmed).

Dying state suppresses the hit sound, **not this blood suffix**. Checked damage
can return nonzero for an already dying target without subtracting health or
restarting death, so a further admitted shot can make the corpse bleed again.
There is no blood-emission RNG gate at this suffix: `FUN_00440DC0` normally
attempts two allocations at scale `0x0800`, scales that count through frame
pacing, and advances the shared direction cursor even if the particle pool is
full. Collision admission, checked-damage return, capability, pool capacity,
and the later surface contact therefore matter to the observed intermittent
result. This does not establish an independent random corpse-bleeding timer.

These droplets use the same class-5 `FUN_0043E180` surface tail as hive
emissions. An ordinary contact sets bit `0x10` in the wrapped terrain cell and
consumes the particle. Selector 6 instead damps velocity and keeps it alive;
the copied owner sign suppresses infection writes when set. Existing infection
in a cell does not produce another bit change. The persistent ground mark is
the terrain overlay described below, not the transient blood sprite.

The native Type17 and Type47 hit owners retain the living, lethal, and corpse
suffixes. The fixed-target/Hive primary-hit adapter still has an explicit
capability-8 suffix boundary; that limitation must not be generalized to native
insect corpses. Accepted Type17 recordings prove living/lethal class-5 births
but did not sample the resulting terrain grid, so they are not an end-to-end
runtime proof of blood landing.

### Live particle-to-terrain publication

Playing and Intro2 now publish particle terrain writes into the current
Section-10 allocation between physical particle slots. The mutable traversal
host applies each ordered batch after releasing its terrain borrow, before
an external callback or the next slot, and invalidates terrain presentation
when a byte changes. Committed writes leave the collision overlay and remain
only as history in the returned outcome. A later factory crater can clear an
earlier infection, and a still later particle can infect the cell again;
replaying the historical journal at frame end would destroy that order.
Read-only/borrowed particle adapters retain the existing deferred journal.

This closes ordinary gameplay's discarded particle-infection writes, which
previously allowed blood or hive carriers to disappear without changing the
live grid. The native Type17 regression reproduces that failure before the
repair and follows living, lethal and corpse hits through class5 retirement,
live bit-`0x10` changes and presentation invalidation in both world hosts.
A separate ordered-callback regression covers infection, synchronous clearing
and optional reinfection in one traversal. Existing class5/class6 controls
retain set/clear, suppression, selector6 and wrapped-cell behavior. Type115's
separate class50 regression follows a parent burst through its class5 children
and the same live terrain publication.

### Hive Emission and Terrain-Grid Evolution (RUNTIME-VALIDATED)

The accepted Level-1 trace is
`runtime_re/captures/local/20260722-022643-hive-virus.jsonl`, retained with its
`.protocol.json` sidecar. Over 53.545 seconds, type-67 handle `0x043A0001`
produces 463 stable class-5 births at essentially one every 80 ms. The authored
component header is `[80000,5000,5,5,1]`; its source is raw hive position
`[-17920,-256,-32768]` plus attachment `[384,400,384]`, yielding
`[-17536,144,-32384]` before the three recovered RNG perturbations. Emission
begins only when the hive enters the detailed-update region, not at a fixed
level timer.

Class 5's `FUN_0043E180` surface tail dispatches its material response through
`FUN_0043E1C0`; every ordinary terrain/water selector reaches
`FUN_00433720(x,z,1)`. That helper sets terrain byte 2 bit `0x10`, updates the
exact infected-cell counter, and notifies registered callbacks. The trace's
counter rises from 95 to 139 and always matches a full-grid recount: 47 cells
are set and three clear. Thirty-seven sets follow a class-5 removal in the same
wrapped cell within the preceding roughly 110-ms terrain-sample interval,
validating the direct particle-impact path. Ten sets lack that same-window
correlation, but pool sampling is not atomic, so those unmatched sets alone do
not establish which path produced them. All three clears do prove the separate
evolution layer because the class-5 callback can only set infection.

Retail visibly presents a landed hive emission as a blood-like infection mark
on the ground; the mark then participates in the perceived spread from the
hive. Static rendering closes its presentation owner. `FUN_0042FCC0` and
`FUN_00430140` read terrain bit `0x10` directly and displace each infected
terrain vertex in X/Z through the 16 signed offsets prepared by
`FUN_00433530`. `FUN_00430430` builds a four-corner infection mask and submits
a second terrain pass from the five Section-3 shapes at
`terrain_sprite_base + 120..124`, using the exact `DAT_004CACC8/004CACCC`
marching-square frame/rotation table also used by shoreline water. Frames 0..3
are zero-keyed and all five select fixed palette row 28; frame 4 is the opaque
all-corners tile. The ordinary terrain texture remains beneath partial masks.
`FUN_00433720`'s callbacks are change notifications for secondary consumers,
not the owner of the ground mark. The GL terrain path now reproduces the
fixed-shade overlay, live terrain-grid invalidation, and `FUN_00433530`'s
two-phase triangle-envelope X/Z displacement on the shared base and overlay
vertices. It retains the 16x16 selector layout and zero-amplitude refresh
boundary, with a deterministic selector realization until the port owns
retail's single process-global RNG call order. The overlay uses a depth-read-only
raster bias to prevent backend coplanar flicker without becoming an occluder.
Do not replace this terrain pass with a permanent generic particle decal.

`FUN_0041BEB0` copies six authored header words into controller words 7..12.
Header word 0 is the radial-emission interval. Word 1 is a second strict
accumulator which calls
`FUN_00436960(crossings, word2, word3, state+2,
controller_state_is_one)`. That function
samples a random wrapped cell, counts infected cells in its 3x3 neighborhood,
clears a sparse selected infection, may seed a dense neighboring cell, and can
enqueue a bounded propagation item. `FUN_00436750` advances those queued items
one cardinal cell every 350,000 us. Immediately before each accepted infection
write it calls the positional-audio path with global sound 64, full gain, and
fixed 1.0 playback rate; the first argument is a sound id, not a visual
runtime-object type. Both boundaries are strict: equality does not cross. Word
1 and the retained accumulator are signed 32-bit values, and a non-positive
authored interval disables this branch. Queued tails advance only from the
same word-1 crossing branch, and a tail stops when its current anchor is
cleared or its next cell is below the signed sea-level word.
`FUN_00415120` supplies the evolution gate: at least one live, non-dying entity
with capability bit `0x08` and state bit `0x01000000` must remain. Multiplayer
clients skip only `FUN_00436960`'s natural selection; existing queued tails
still advance. Sparse clearing is independent of controller state; only the
dense random-walk branch requires state 1.

`infection_evolution.rs` retains this exact header-driven terrain executor in
the live authored-hive component. Intro2 and gameplay visit each component in
live-list order, execute infection before radial class-5 emission, and share
the one process-global `WorldFx` RNG stream. Ordered bit-`0x10` set/clear writes
are replayed into the current Level Section-10 allocation before the particle
surface pass, invalidating cached terrain geometry only when a byte changes.
Unresolved objective state or terrain fails the infection branch closed while
leaving the independently ordered radial callback active. Tests pin toroidal
neighborhood counting, the four/five/six-cell decision boundary, signed strict
interval/remainder handling, random-walk draw order and run limit, FIFO
350,000-us tail catch-up, sound-before-write ordering, anchor cancellation,
signed underwater termination, and live cache mutation order. A long Level-26
trace remains the acceptance oracle for that later world's authored envelope.
The three observed clears belong to this autonomous evolution layer and are not
proof of a player cure. A distinct internal cleansing mechanism is statically
proven below, but it should not be exposed as player functionality unless retail
first demonstrates its binding.

### Authored terrain-contact set and clear behaviors (CONFIRMED)

`"Defecate Virus"` resolves through table entry `0x004C8AC0`, descriptor
`0x004C88C8`, and prototype `0x004C7E88` to `FUN_0040B9E0`. The initializer
first clears component/task slot 1 with `FUN_0040A7A0(entity,1,0)`. It resolves
the live entity's type index at `+0x58`, reads the type-record word at `+0xA2`,
and calls `FUN_00402820(entity,2,type_A2,0,5)`. The wrapper packs the last two
arguments as `0x00050000` and constructs this slot-2 state:

| State offset | Class-4 value / role |
|-------------:|----------------------|
| `+0x0C` | type-record `+0xA2` word (67 on captured Intro2 type 26) |
| `+0x10` | tick callback `FUN_00402850` |
| `+0x18/+0x1C` | no pair callback / zero context |
| `+0x28` | packed low payload 0, high signed mode 5 (`0x00050000`) |
| `+0x2C` | generic elapsed-millisecond accumulator |
| `+0x30` | common transition-table pointer |

The constructor polarity is important. `FUN_00401020` returns zero only after
successfully allocating/copying the 0x34-byte task state and publishing its
wrapper through the out-pointer; `FUN_00405F80` then installs that wrapper with
`FUN_0040A7A0`. Consequently, `FUN_00402820 == 0` is slot-2 success. Only that
success path continues to `FUN_004032A0(entity,0,2000)`, which installs the
duration-2000 slot-0 `FUN_00402BA0` task with pair callback `FUN_00402DA0` and
auxiliary callback `FUN_00402CA0`. It is the normal concurrent movement
companion, not an allocation-failure fallback. A nonzero slot-2 result skips
the slot-0 installation. The mutation order is deliberately not transactional:
slot 1 remains cleared on either failure, successful slot-2 installation
remains committed if slot-0 preparation later fails, and the old destination
task is preserved whenever its replacement cannot be prepared.

Successful slot-0 preparation also has one exact component side effect before
the new task is published. If Section-12 Sub-A is authored, common
component-table slot 3 receives literal `1` in target-speed runtime dword
`+0x00`; direction multiplier `+0x04` and drive scale `+0x08` are preserved.
An entity without authored Sub-A skips the write and still installs the task.
Preparation failure happens before either this write or slot replacement.

`FUN_00402850` receives the common scheduler's detailed/coarse mode directly:

- It first suppresses all work while live entity state bit `0x1000` is set.
- **Detailed mode 0** consumes exactly one process-global RNG word on every
  callback and emits only when `random16 < (elapsed_us >> 2)`. A passing call
  selects model slot `+0xA8/+0xAA/+0xAC/+0xAE` from state bits
  `0x2000/0x4000`, reads that model's unsigned extent word at `+0x08`, and
  subtracts `signed_high32(2 * forward_Q31_axis * extent)` from each signed-8.8
  position word; Y then receives an additional `+100`. The three inputs are
  the live entity rotation matrix's forward column at `+0x24/+0x28/+0x2C`,
  not velocity (stored as signed words at `+0x9C/+0x9E/+0xA0`). It submits
  source class 5, scale `0x800`, owner-sign from live entity flags
  `(entity+0x08)>>31`, and the entity handle to
  `FUN_00440DC0`. That helper attempts
  `max(1, ((0x800 >> 10) * DAT_004F72CC) >> 16)` particles (two at normal
  global scale), pre-increments the shared 100-vector direction cursor for
  every attempt, and preserves those cursor advances even when the fixed pool
  rejects an allocation. Packed low payload zero means this class-4 component
  requests no positional sound; the distinct type `+0xA2` state word does not
  become that sound argument.
- **Coarse mode 1** consumes exactly two RNG words with no chance gate. It
  chooses wrapping X and Z independently as
  `actor_axis + (random16 >> 7) - 256`, then calls
  `FUN_00433720(x,z,1)` directly. Thus an off-detail actor can seed one nearby
  terrain cell without creating a visible carrier particle. The callback
  selects this branch for any nonzero scheduler mode; mode 1 is the ordinary
  coarse caller observed here.

For each detailed allocation attempt, the direction-table vector becomes
`[x>>1, abs(y>>2), z>>1]` with no RNG draw. Class 5's descriptor has zero
spawn-velocity bias at `+0x2A`; its signed `-100` at `+0x12` is a render sort
bias, not a Y-velocity adjustment. The copied owner-sign becomes particle bit
`+0x1D & 1`: a bit-31-clear owner infects contacted terrain, while a
bit-31-set owner still emits a transient carrier but suppresses that impact
mutation.

The duration-2000 companion's `FUN_00402BA0` is not self-contained movement.
It chooses a new target when either wrapped target-axis delta is below `0x300`
or a one-in-64 RNG gate passes; two further words select X/Z offsets in
`[-4096,+4095]` around the actor and target Y is copied from the actor. It then
delegates to the shared seven-argument mover `FUN_00401430`. A zero mover
return maps to the pointer-distinct tagged singleton `0x004BE138` (`0x9C01`),
not ordinary type-9 wander's `0x004BE140` singleton. The accepted
`20260722-022303-intro2-actor-ai.jsonl` trace independently binds type-26
handle `0x047E0001` to style `0x004C7E88`, an empty slot 1, the exact slot-2
state above, and the duration-2000 slot-0 task. The focused
`20260727-235423-intro2-actor-task-mover.txt` transcript additionally binds
type 26 to movement-state pointer `0x13821810`, scheduler mode one, and the
complete mover call/return boundary. It still does not expose the mover's
internal terrain, integration, target-lifetime, or completion decisions, so
the class-4 mover body remains external to the detached runtime.

The Rust port now retains this bounded behavior across its live publication and
runtime seams. Canonical Section-12 metadata keeps the unsigned `+0xA2` word
(67 in retail, 69 in the demo; the behavior-choice table is identical);
the normal frontend `BeginIntro` route authenticates the exact Intro2 descriptor,
type-26 record, and authored spawn 25 before replaying the captured
choice-2/class-4 success. It consumes no selector RNG, publishes the
duration-2000 Primary Wander / empty Secondary / mode-5 Tertiary graph, and
uses the retained `+0xA2 = 67` value rather than a replacement literal. Direct
`--level 50` and other construction histories deliberately remain unresolved,
and any captured-route authentication or setup failure blocks the level load.

`defecate_virus_owner` preserves exact clear/prepare/Sub-A-reset/publish
ordering, `defecate_virus` owns detailed/coarse RNG and the slot-0 callback
prefix, and `WorldFx::emit_defecate_virus_particle_raw` owns pacing-scaled
direction-cursor allocation attempts. The central heterogeneous dispatcher
ticks both the slot-0 wander and slot-2 terrain task through the generic
scheduler's exact elapsed-before-callback, self-clear dominance,
tagged-result-before-timeout, strict-expiry, and fresh-later-slot phases. An
unresolved Primary mover stops that pass before Tertiary, so the terrain task is
not advanced independently. The slot-0 planner commits its elapsed and retarget
prefix before the mover, stages only mover-owned mutations until the external
mover returns a proven zero/nonzero result, applies mover completion before
strict `elapsed_ms > 2000` expiry, and fails closed otherwise. Captured Intro2
spawn 25 now runs that Primary `FUN_00401430` bind and Detailed `FUN_00402850`
from the constructor `FUN_00413F70` heading-`0x4000` matrix and each slot's
Section-8 header `+0x08`. Emitted class-5 carriers take the later
`FUN_00440120` / `FUN_0043E180` ordinary-surface tail and set terrain
byte-2 bit `0x10`. A later `FUN_00411400` Coarse write clears
`0x02000000`, so the next Tertiary visit is coarse `FUN_00402850`.
Type 13's captured B6C0 owner now commits the complete normal Sub-G callback
and post-task body-basis refresh. Normal Intro2 scheduling, activation,
environment forces, and master motion are live; presentation acceptance and
selector ownership remain open, so presentation stays
proxied. Captured fresh-Level-1 Type-47
is live, while replay-level Type-47 and generic non-Intro2 histories remain
open.

Detailed class-5 particles are transient carriers. On an ordinary surface,
the recovered class-5 tail sets terrain byte-2 bit `0x10` through
`FUN_00433720(...,1)` and removes the particle. The user-observed blood-like
ground mark is persistent terrain infection/presentation, not a permanent
particle decal. This class-4 actor path is also distinct from type 67's already
implemented authored radial hive emitter and from `FUN_00436960`/
`FUN_00436750`'s controller-owned neighborhood and queued cell-to-cell spread.

`"Cleansing Landscape"` resolves through table entry `0x004C8BF0`, descriptor
`0x004C8988`, and primary prototype `0x004C85D8` to `FUN_0040AD50`; it installs
the same component with mode 6. The shared terrain callback sends mode 6 to
`FUN_00433720(...,0)`, establishing an internal clear path. The analyzed
`20260731-185548-cleansing-landscape-vehicle` passive run now supplies the
runtime half of that static contract. Type-49 handle `0x04B30003`/model 266
begins parented in Stationary style `0x004C74B0`; the drop transition enters
carrying style `0x004C8620` under a type-93/model-266 proxy, which then
disappears as type 49 unparents into canonical style
`0x004C85D8`. At that boundary the actor publishes slot 2 with callback
`FUN_00402850` and private mode dword `0x00060000`, slot 0 with callback
`FUN_00403040`, and an empty slot 1. The run contains 181 source-owned class-6
carrier births and 14 duplicate-validated infection-bit clears following the
vehicle over roughly 1.7--3.9 world units. The separate `20260731-184134` run
is a negative control: the same presentation remains attached, produces no
class-6 stream, and clears no terrain.

Particle descriptor class 6 is the mode-6 sibling of class 5: it retains the
same collision mode and gravity family but dispatches `FUN_0043E1A0`, whose
ordinary terminal response clears rather than sets bit `0x10`. The Rust shared
terrain-contact planner therefore retains the identical detailed chance,
origin, pacing, and coarse-cell RNG contract while selecting class 5/set for
mode 5 and class 6/clear for mode 6. `WorldFx` derives that policy from the two
authored descriptor callback addresses and returns ordered set/clear writes, so
opposite same-cell contacts in one traversal preserve retail order.

The [native vehicle owner](CLEANSING_VEHICLE.md) implements the source-proven
constructor, infection-seeking movement, detailed/coarse clear tasks, separate
cargo releases, and class49 hit/death path. The accepted NoCD02 TTD now joins
the cargo/AD50 transition and proves the exact detailed clear caller: class6
particle4DE080, owned by Type49 handle04940001, reaches33720 from43E21B at
`460588:985`; terrain cell `[143,63]` changes `DD0054` to `DD0044`. Its fatal
hit is separately attributed to player46/class55, not Newant fire.

Neither the passive vehicle run nor that TTD establishes player cure-weapon
acceptance. Retail's Weapons cheat proves that selectors7 (`Antidote`) and29
(`Antidote Bomb`) enter normal inventory. Separate retained executable and
descriptor evidence now proves selector7's class6 surface/static terrain-clear
binding; [the player firing contract](PLAYER_CRAFT.md#fire-both-modes) records
its exact callbacks, production cure-hit owners and explicit target/style
boundaries. Matched retail Antidote firing acceptance remains open. Selector29's
impact binding and matched player firing acceptance remain open for the
prepared focused trace.

### Visual Representation (CONFIRMED — unchanged)

Model substitution system with 40+ `virused*` plant model variants:
- `virusedsunflower`, `virusedchines1`, `virusedmangrove`, `virusedfern1`, `virusedlilly1`, `virusedstump`, `virusedmangtre1`, `virusedmanghse`, `virusedcicad`, `virusedbrome1`, `virusedferntre1`, `virusedbranch1`, `virusedflower2`, etc.
- Separate infected and dying-infected model slots include `virused*` and
  `deadvirused*` variants (e.g., `deadvirusedchines1`). A dead model does not
  by itself prove that an actor can no longer emit infection: the accepted-hit
  suffix above can still release class5 from a corpse. The landscape percentage
  counts infected terrain cells, independently of an actor's selected model.
- Seedpod projectile model: `seedpod1`
- The representative list above is retained here. The retired bulk
  `v2000_strings.txt` dump is no longer checked in; re-read the OVL string
  sections through `v2k-formats` when a complete current census is needed.

### Message System (CONFIRMED)

The virus failure / warning / end-of-level messages are queued through the standard text notification system documented in §15:

- **Single-slot message queue** at `g_rng_state + 0x2C0`. Enqueue via `FUN_00437f30(queue_ptr, message_id, value, expiration_time)`. Sticky message id `0xD9` cannot be overridden.
- **Fire function**: `FUN_00456900(message_id, value)` at `0x00456900` — public API used throughout the code.
- **Deduplicated resource-text queue** at `g_rng_state + 0x2C4`, enqueue via `FUN_00437f60`, fire via `FUN_004568b0`. Event IDs index `DAT_004cad80` (small int -> global Section-2 string id, `0xE1..0xF8` range).
- **HUD render entry** at `FUN_004556a0`. The render side reads the queue slot, resolves `message_id → string`, and displays.

The text message IDs in code (e.g., the `FUN_00456900(0xc9, 0)` call in case 0x39 seedpod pickup) are NOT byte offsets — they are global Section-2 string IDs. Resource-event IDs are the separate small `0..24` domain translated by `DAT_004cad80`. Both string families are resolved by the HUD render path; positional/global Section-11 sound IDs are independent.

### Virus Message Strings — Resolved to Section 2 Byte Offsets

All three virus-trigger messages live in **`0X3XX.OVL` Section 2** (system level 3, the menu/system OVL that holds shared HUD strings):

| Section-2 byte offset | Seq index | String |
|----------------------:|----------:|--------|
| `0x3EB` (1003) | #73 | `' <2500,10000,*,15,51,85,30,5>The landscape was %d%% virused.'` |
| `0x56A` (1386) | #82 | `'\t\t<*,3000, 4, *, *>Warning: Virus level critical'` |
| `0x71B` (1819) | #92 | `'\t<*,3000, 4,30, *>The virus went critical. World lost.'` |
| `0xDCE` (3534) | #121 | `'\t\t<*,3000, 1,30, *>If Virus covers too much of the landscape, the world will be lost'` (briefing tip) |
| `0x10E7` (4327) | #145 | `'Virus'` (weapon name) |
| `0x10ED` (4333) | #146 | `'Antidote'` |
| `0x10F6` (4342) | #147 | `'Seedpods'` |
| `0x11A9` (4521) | #161 | `'Virus Bomb'` |
| `0x11B4` (4532) | #162 | `'Antidote Bomb'` |

The leading `<delay,duration,?,x_pos,y_pos,?,?,color>` parameter prefix is embedded in the string itself — the HUD render strips and applies it.

### Seedpod Pickup (CONFIRMED — unchanged)

`game_logic.c` case `0x39` at line 35465:
- Increments `vehicle_state+0x195` (capped at 4 seedpods)
- Spawns entity type `0x41` (65) — the seedpod projectile
- Fires text event `0xC9` (201 decimal) — the seedpod pickup notification

### Per-Level Configuration (CORRECTED 2026-08-10)

VSpread level (`0X26XX.OVL`) Section-13 base parameters have outliers, but the
previous interpretation mixed two unrelated systems:

- `+0x5C = 600` is the proven time-trophy deadline in seconds. Castle's 300
  and Cistern's 180 are the same controller field; none is a virus threshold.
- `+0x8C = 10` (typical 4–8) remains a possible spread radius or interval, but
  it must be established at its retail consumer rather than joined to `+0x5C`.
- All four display/resource variants retain identical parameters, so these are
  authored world rules rather than display-tier differences.

### Open virus boundaries

`FUN_004366F0` computes the global display percentage as
`100 - floor(0x06400000 / (infected_count*99 + 0x00100000))`; the accepted
count of 95 therefore yields 1%. Overlay 51 now snapshots that transform from
the current level's Section-10 bit-`0x10` census when the results card opens.
Remaining actor bindings, Antidote selectors, and the natural Level-26 envelope
are implementation/evidence work rather than mechanics-reference content. They
are tracked in `OBJECTIVE.md`,
PARTIAL_EVIDENCE_FOLLOWUPS.md, and
the capture ledger.

---

