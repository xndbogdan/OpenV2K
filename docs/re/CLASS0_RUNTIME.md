# Shared class-0 flags and weights

This owns ordinary Type52/54/68 and Intro2 Type52 construction, living timer/task custody, Type68
cargo attachment/release, Type54 Change-Sea-Level abort and generic quiet death. The production owner is
[`entity/class0_actor`](../../crates/v2k-game/src/entity/class0_actor.rs).
[Actor Runtime](ACTOR_RUNTIME.md) owns the shared scheduler primitives;
[Factory System](FACTORY_SYSTEM.md#reactor-mechanic) owns the enclosing Main
Base abort transaction. Objective09
remains open for remaining eligible cargo constructors other than Type68/92/96/97/100.

## Canonical records and constructor

Normal-tier ordinary overlays13–49 author 157 Type52 flags, 44 Type68
weights and one Type54 grock. Intro2 additionally authors twelve Type52 instances through the same
constructor and task owner. All these authored
52/68 Euler words and model overrides are zero; Type54's Level-1 record is also
zero-Euler with zero `+0x88`. Params are normally zero;
Type68 level25 spawns64/92 use1, as do the twelve Intro2 flags. The flag's
priority at spawn+1C is independent from spawn+10's parameter.

| Field | Type52 | Type54 | Type68 |
|---|---|---|---|
| Default model slots | `[145;4]`, flag | `[560;4]`, grock | `[81;4]`, weight |
| Selected model header+08 |200|256|110|
| Mass / health |1 / 1|100 / 1000|200 / 50000|
| Capability |`0100`|0|`1040`|
| Type+C0 policy |`73E7`|`4027`|`27224`|
| Root / alternate |Always `(1,1,0)` / class2|Always `(1,1,0)` / class38|Always `(1,1,0)` / class2|
| Optional components A–O |none|none|none|
| Constructor sound / death sound |none / none|none / none|none /62|

The full metadata profile is authenticated before publication; model overrides,
position, angles and parameter remain authored instance inputs. These profiles
allocate no Sub-D and consume no Sub-A/06070 component word. The successful
AC60 singleton selector still consumes one shared RNG word. No captured level,
spawn index or seed authorizes a native allocation.

`104B0` copies the authored position/angles, applies spawn-parameter state and
classifies the **authored Y** against sea/wave before D4A0 grounding. The
coarse cell is unsigned X/Z high bytes; its terrain-type bit10 selects slot2
instead of slot0. The attribute byte does not select this model domain. All
four per-spawn overrides survive, including inactive slots1/3.

`D4A0` first copies type+C0 to entity+C8. Bit20 calls `445860` with current X/Z;
this is signed-byte integer bilinear terrain, including toroidal neighbors and
arithmetic-shift rounding. It does not sample sea or waves. Nested bit40 adds
the **selected active model's header+08** with wrapping word addition. Thus
Type52 adds its selected extent; Type54 and Type68 do not add256/110. Collision header+0A
is not this input. `40D6B2..40D6CA` then copies current XYZ into entity+90.
The successful D720/13F70 wrapper retains the authored Euler-derived basis.

The native constructor requires the selected header08 lookup only for bit40,
before consuming the singleton selector. It retains manager allocation,
type/model slots and post-grounding+90 in a private receipt. Unwritten transient
B2 uses an explicit native zero policy; this is not a claimed retail104B0
writer or evidence about arbitrary allocator residue.

## Intro2 publication

Both native and explicitly captured Intro2 entry points execute the shared
class0 constructor at each authored flag position. The captured birth-selection
options retain only their named Type13/47/26 graphs; they do not omit Type52
construction. The shared preparation authenticates metadata, resources and the
actual manager allocation before publication. Ordinary construction appends its
new actor; Intro2 publishes into its existing authored slot and registers the
same private receipt without duplicating or reordering the live list.

The twelve flag indices are14,22,23,27,28,29,32,37,39,47,48,57. Each uses slot0
model145/header08=200, with no components or Sub-D allocation. Its successful
AC60 birth consumes one real singleton selector word in that order. Later
actors can therefore select differently from the old incomplete constructor;
preserving an earlier scene's total RNG endpoint is not an acceptance oracle.

All twelve authored Y values are0, and every coarse cell is above Intro2's
sea=-847. The dry-only104B0 resolver proves surface00400000 independently of
the clock and fails closed if a changed input requires waves. D4A0 grounds
their Y values to-152,8,72,200,328,-344,8,456,-280,72,8,-248 respectively,
then copies+90. The selected model extent is a required live cache resource.

The main load boundary adopts their class0 owners before the first cinematic
scheduler pass. Flags retain their disabled callback and invisible800 gates;
their timer is not artificially advanced. Camera commands and beacon consumers
read the grounded live positions directly. Intro2 presentation submits those
anchors through `intro2_uses_live_actor_pose`; they remain invisible
(`intro_actor_visible` is false) and are not `intro_actor_pose` subjects. `tests/intro2_class0.rs` covers both
entry policies, manager/task custody, first actor visits, the initial camera
anchor and missing-header failure before its selector with the reached RNG
prefix retained. Whole-scene retail comparison remains a separate requirement.

## Authored wave policy

`DAT_004FECE4` is independent from water-plane visibility. `42EA30` copies the
0xA8-byte default block at4CAAA0 into the world controller; its final dword
4CAB44 is zero. `42EA74..42EA7E` sets controller+A4 to1 when the current
Section13 descriptor+84 is nonzero. `433BD0` copies42 dwords into4FEC40, so
controller+A4 becomes4FECE4. `42E570 -> 42EA30` runs before authored births.

Constructor waves therefore use `descriptor[+84] != 0` and the actual retail
clock. Only when enabled and coarse cell height is below sea does104B0 call
the wave formula. Authored Y below/equal/above the chosen surface sets
`00200000`/zero/`00400000`; equality is strict and later grounding does not
reclassify it.

Normal-tier waves are enabled in13,14,15,16,18,19,21,22,24,25,27,28,32,33,36,
44,48,50. Worlds17,20,23,29,30,31,34,38,40,41,43 render water while disabling
waves. The ordinary52/68 census reaches31/7 wet coarse cells respectively;
Type68 in23,30,34 reaches the disabled-wave branch. A visible sea plane cannot
substitute for this field.

## Living timer and outer visit

`C490` clears Secondary then Tertiary and calls `02800(entity,0,9000)`.
`05F80` installs a Primary with a null callback and no private payload.
Its wrapper adds `callback_elapsed_us / 1000` with per-call truncation;
timeout is **strictly elapsed >9000ms**, after callback unwind. A surviving
expired wrapper reselects through A800/C690 only when state1000 is clear.
Living AC60 consumes a fresh singleton selector word and C490 replaces Primary.
The factory wreck's already-dying class0 reselection is a separate no-draw path.

Timer existence does not imply advancement on every frame. Canonical52/68
start with scheduler callback-enable bit20000 clear. `12DA0` still owns its
branch-local random waits, scheduler clocks, attachment relation checks,
post-callback B2 clear and gated master motion. Only an enabled callback reaches
DCA0/E870: the detailed/coarse branch and effective policy determine whether
the task wrapper is visited. In particular, coarse effective-bit2000 skips the
task branch; detailed effective3000==2000 sets master-motion state before it.

Presentation publishes `11400` view detail under the exact live owner before
the next scheduler visit. Type68 retains runtime bit800, so the camera can
classify it even with callback-enable bit20000 clear. Type52's authored gate
clears800 and bypasses this classification. Far/near results therefore change
the next12DA0 subject/callback wait decisions and branch-local RNG consumption;
they do not enable the class0 callback or advance its timer by themselves.

The owner preserves the exact task lease and rereads the source state domains.
The supported callback suffix retains detailed sound eligibility, the canonical
environment skips, bit2 ground snap with optional bit40 extent, zero-selector
surface-timer decay, B2 clearing and final motion ordering. Missing-parent or
membership repair and unsupported environment/descriptor inputs remain explicit
errors; attachment is not a reason to suppress an otherwise reached callback.
An error after scheduler/task mutation retains a pending-prefix receipt.
Fresh adoption, external cargo transfer and abort cannot erase that prefix or
repeat elapsed/RNG by borrowing an earlier graph.

## Type68 attach and release

Class0 style4C7468 has a null attach hook. `16700/DBF0` publishes membership
and parent without replacing its existing timer or consuming selector RNG.
Type52 has no corresponding collectible capability.

Campaign Type68 misses now enter the same constructor through an explicit
zero-record input in
[`class0_actor/zero_record.rs`](../../crates/v2k-game/src/entity/class0_actor/zero_record.rs).
Position, Euler words and parameters start at zero; current terrain supplies
grounding and+90. The body consumes its temporary construction stamp before
the singleton selector and publishes real manager/task custody. Only afterward
does the campaign caller overwrite B4 with the saved word. A matched weight
keeps its allocation, current pose and timer ages. Neither path copies a saved
task graph or substitutes the player's arrival pose for constructor input.

Both campaign and live collection use the shared
[`443B50` attachment owner](../../crates/v2k-game/src/entity/cargo_attachment.rs):
slot admission, relation/null callback, then resource notice, state writes and
transfer presentation. Capacity is checked before duplicate identity. Repeated
attachment can preserve the same timer and still execute the callback/suffix;
a dying first match is retained as the selected identity but does not acquire
a valid living owner. Unavailable callback custody stops with its actual
committed prefix. A foreign allocation or already pending host prefix is
rejected before entry. [Campaign cargo](CAMPAIGN_CARGO.md) owns matching,
ordinary-load player surface/clamp and the exactly-one style6 ballast birth.
The restore/attachment integration passes the focused and runtime controls in
[campaign cargo validation](CAMPAIGN_CARGO.md#validation).

The old Type68 release capture request arose from a misidentified stack
argument. `40DC76` loads entity+C0 into EAX; `40DC84` pushes the address of
the packaged parent as the **third** argument, `40DC85` pushes the context as
the **second**, and `40DC87` invokes style+0C. D1C0's `40D1CB` read of
stack+10 accounts for saved ESI and the outstanding call argument: it reads
that second argument. `40D1D2` supplies context[0] to AC60. This is the
TypeDefault choice source, not a packed player handle used as a pointer.

`16750` restores relation/default state; D1C0 terrain-aligns the physical
columns through16AC0, sets28 while retaining Euler words, then invokes living
C690/AC60. One singleton RNG word selects class0 and publishes a new Primary.
Player drop releases once before attaching to its new Type93. Normal proxy
settlement `409030` copies the proxy position into child+96 **and+90**, enables
master motion and invokes release again, with its own selector word and task.
Null attachment between those releases preserves the newly selected timer.
Task replacement is transferred to the scheduler under the same allocation.

## Quiet death and remaining boundaries

Native52/68 abort admission authenticates their construction receipt and
completed current class0 task custody. `10C10` retains remote/already-dying
precedence, writes health0/dying, requests Type68 cue62, traverses null sound
attachment/style-death hooks and selects alternate class2 without a living
selector draw. C470 clears tasks, writes the10B70 deferred state and stages
removal. The mutation-sensitive abort reads the successor only after callback
return; its enclosing transaction retains rollback on any later-family block.
Attached cargo retains the separate callback-free relation route. Type93's
abnormal teardown marks its child without invoking this normal release path.

Regression coverage lives in `tests/shared_world_construction.rs` and
`entity/class0_actor/{tests,live/tests,cargo_tests}.rs`, with abort controls in
`entity/class0_abort_tests.rs`. It distinguishes corpus publication from
controlled callback-enabled timers, fractional grounding, model/angle/parameter
variation, wave policy, RNG ordering, pending and stale custody, and release.
Quiet Type62 constructor/abort is owned with the Type62 native receipt.
Remaining eligible cargo constructors other than Type68/92/96/97/100, and matched-retail
Intro2 acceptance, stay open. Factory-born
Type61 abort is owned with the factory constructor receipt. Native Type54 abort
is in [Factory System](FACTORY_SYSTEM.md).
