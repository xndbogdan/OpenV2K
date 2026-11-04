# Shared class-0 flags and weights

This owns ordinary Type52/54/68/72/85/106 and Intro2 Type52 construction, living timer/task custody, Type68
cargo attachment/release, Type54/72/85 Change-Sea-Level abort and lethal hits, and generic quiet death,
and the [rows outside class0](#rows-outside-class0-type50-type89-and-type118) on the same constructor:
Type50's Die When Moved, Type89's flood and Type118's Geyser. The production owner is
[`entity/class0_actor`](../../crates/v2k-game/src/entity/class0_actor.rs).
[Actor Runtime](ACTOR_RUNTIME.md) owns the shared scheduler primitives;
[Factory System](FACTORY_SYSTEM.md#reactor-mechanic) owns the enclosing Main
Base abort transaction. Objective09
remains open for remaining eligible cargo constructors other than Type68/92/96/97/100.

## Canonical records and constructor

Normal-tier ordinary overlays13–49 author 157 Type52 flags, 44 Type68
weights, one Type54 grock, world 17's three Type72s, world 43's Type85 and one
Type106 in each of worlds 37, 38 and 40. Intro2 additionally authors twelve Type52 instances through the same
constructor and task owner. All these authored
52/68 Euler words and model overrides are zero; Type54's Level-1 record is also
zero-Euler with zero `+0x88`; the Type72s face heading `0x4000` with `+0x88 = 0xB4`; Type85 is zero-Euler with
`+0x88 = 0x1B0`. Params are normally zero;
Type68 level25 spawns64/92 use1, as do the twelve Intro2 flags. The flag's
priority at spawn+1C is independent from spawn+10's parameter.

| Field | Type52 | Type54 | Type68 | Type72 | Type85 | Type106 |
|---|---|---|---|---|---|---|
| Default model slots | `[145;4]`, flag | `[560;4]`, grock | `[81;4]`, weight | `[1104;4]` | `[140;4]` | `[0;4]`, none |
| Selected model header+08 |200|256|110|272|150|-|
| Mass / health |1 / 1|100 / 1000|200 / 50000|1 / 1000|1 / 500|1 / 1|
| Capability |`0100`|0|`1040`|0|`0002`|`4000`|
| Type+C0 policy |`73E7`|`4027`|`27224`|`20027`|`24127`|`7385`|
| Root / alternate |Always `(1,1,0)` / class2|Always `(1,1,0)` / class38|Always `(1,1,0)` / class2|Always `(1,1,0)` / class38|Always `(1,1,0)` / class38|Always `(1,1,0)` / class2|
| Optional components A–O |none|none|none|none|none|none|
| Constructor sound / death sound |none / none|none / none|none /62|none / none|none / none|none / none|

Type85's behavior rule reference is 3; every other row's is 1.

Type72's `+C0` runs the scheduler callback (`0x20000`) and drops the retained
basis (`0x4000`), so a detailed visit's DCA0 rebuilds the basis through
`413F70` before DF70. A coarse visit takes E870's `0x2000` branch instead:
retail's world-17 Type72s read `0x08468805` at their first visit and
`0x08428805` after their first coarse callback (motion `0x40000` cleared, no
DF70 grounding bit). Type85's `+C0` runs the callback and keeps the basis:
retail's world-43 Type85 reads `0x0E468005` at its first visit, grounded at Y
2112, and `0x0EC68005` after its first detailed callback (DF70's grounded
`0x800000`). Type106's `+C0` has neither the callback enable nor grounding
(`0x20`): retail's Type106s keep `0x06400005` and their authored Y 0, and the
port's visits only age the scheduler and run master motion.

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

## Hits

[`class0_impact`](../../crates/v2k-game/src/class0_impact.rs) owns Playing's
particle hits on native class0 bodies. Class0's style `4C7468` has null
`+20/+24/+28` and the rows keep vtable `4C8A30`, whose `+30` is null, so a hit
is `10EB0`'s stamp (or `11250/11320`'s model prefix), `11030`'s jolt and
`15040 -> 14E90`, with no reselection. `11030` moves only a detailed body
(`0x04000000`) that `0x08000000` doesn't hold: Level 1's grock (state
`0x00428805`) can be jolted, with its three spin words; Type72 and Type85,
holding `0x08000000`, never are. Flags, weights, Type106 and gates have zero
multipliers, so a shot only stamps them.

A lethal hit on Type54, Type72 or Type85 runs `10C10`'s class38 Change Sea
Level, the same transaction as the Main Base abort's: the class0 owner
retires, the burst runs and the sea-level task takes the delta. A tampered
retail run (the lab's direct `00414E90` call of 2000 damage on Level 1's grock,
handle `04B30001`, at pass 1300) reads state `0x0042C005` and health 0 after
the hit, and the sea word `0xFFFCB100` falls to `0xFFFA2000` (delta `-168192`)
in 690 paced updates, through pass 1989. The port matches each of these.

**Type85's death.** `FUN_0040BDC0` gives Type85 (`0x55`) its own branch in
place of `FUN_00440950`'s burst. It queues template `004C89E8` through
`FUN_00437390` (flags `0x0B`: deferred, flash, crater; a ring 0xA00 across,
10 height units high, over 1,000,000 us; a 0x1900 crater 10 deep), starts the
full-frame flash (`FUN_00456750`), and fires 25 shots through `FUN_0044E770`
from a 5x5 grid of 0x400 steps around the body (Z rows, X columns), each of
method 11 (a Type34 meteor) at speed 2000 with no shooter and zero Euler
words. Every shot draws five shared words: its X and Z direction (each word
shifted to the top half of a Q31 direction, Y up at full scale), then the X
offset `(word >> 4) % 255`, the Y drop `(word >> 6) & 0x3FF` and the Z offset;
`FUN_0044E770` then lifts it 300 and floors its climb at 0x960, and the
meteor's constructor draws one more word. S, T and the sea task follow as for
the other rows. In the Main Base abort the port's walk reaches the meteors
behind Type85, as it does other births appended during the walk.

`FUN_004374B0` runs the queued ripples each frame after the static programs
and the radar's sea redraw. `FUN_004371B0` advances a ripple's clock by the
frame's microseconds (the period wraps it to zero), takes the ring's radius
`((t >> 10) * r) / (period >> 10)` and its height (rising over the first
quarter of the period, full to three quarters, then the last quarter's rise
again from zero), and for every cell within the radius adds the difference
between this frame's lift and the last frame's, where a cell's lift is the
height less `(|d^2 - ring^2| - 0x40000) >> 16`, never below zero. A cell whose
height fell takes `FUN_00427B20`'s static hit. When the clock wraps, the last
frame takes the ground back to where it began and the record runs its flash
and crater (`FUN_00436E00` with the world's crater floor) and is freed;
`FUN_00437470` drops the queue with the world. A tampered retail run in
world 43 (a direct 2000-damage `00414E90` hit on Type85, handle `04900001`)
births the 25 meteors at the positions the port computes from the same
random state, leaves the stream where the port does, and its row of heights
through Type85's cell matches the port's after every frame checked, through
the crater.

## Rows outside class0: Type50, Type89, Type118, Type36, Type74 and Type110

Six rows share the class0 constructor (`104B0/D4A0`, AC60's singleton
draw, the `+88` sea word) but not its living program. One owner runs all
their visits, through the same `12DA0` prefix and DCA0/E870 tail.

| Field | Type50 | Type89 | Type118 | Type36 | Type74 | Type110 |
|---|---|---|---|---|---|---|
| Worlds | 16, 40 | 21 (two) | 40 (two) | 42, 43, 46, 47 | 21, 31, 36 | 47 |
| Default model slots | `[647;4]` | `[0;4]` | `[676;4]` | `[0;4]` | `[253;4]` | `[1113;4]` |
| Mass / health | 100 / 1000 | 1000 / 1000 | 1000 / 1000 | 1000 / 1000 | 10 / 50000 | 1 / 200 |
| Type+C0 | `4067` | `21A7` | `20027` | `201A7` | `26025` | `24027` |
| Root / alternate | Always `(1,1,36)`, rule3 / class38 | Always `(1,1,38)` / class1 | Always `(1,1,0)` / class65 | Always `(1,1,16)` / class16 | Always `(1,1,59)` / class1 | Always `(1,1,67)` / class67 |
| Authored `+88` | world16 -1068, world40 0 | 3249 | 1000 | unread | unread | unread |

**Type50, Die When Moved.** `FUN_0040B980` clears T then S and
`FUN_00404A20` publishes a Primary without timeout; `FUN_00401350/FUN_004012E0`
copy the body's position into its record. `FUN_00404AA0` acts only on DCA0's
detailed visits (`FUN_0040E870` passes 1): it compares the record with `+96`,
stores the current position and calls `FUN_00410C10` once
`dx*dx + dy*dy + dz*dz` (wrapping 32-bit) exceeds 0x100, a move past 16 units.
The alternate class38 then changes the sea: world 40's zero word takes the
derived delta down to the plug's bottom, world 16's -1068 is taken as is.
Retail's world-40 Type50 reads `(23040, -3840, 24576)` and `0x00468805` at its
first visit.

**Type89, the flood.** Class38 runs from birth: `FUN_0040BDC0` clears `0x800`,
`FUN_00440950` bursts at the grounded body (model 0's header+08 is 0), S and T
clear and `FUN_00405040` publishes the Primary with `3249 << 8`. With `0x800`
clear, `FUN_00411400` never reclassifies the body, so the constructor's detail
bits keep every visit detailed and `FUN_00405160` steps the sea each 20-ms
tick: two rising half steps of 122 raw. Retail's world 21 reads the sea at
-1035008 on arrival and -1027932 29 ticks later, 24400 every 100 ticks, until
its casualty loss 1774 ticks in; the abort explodes both Type89s through
class1 (`FUN_0040BAC0`), which ends the rise. The port's sea matches tick for
tick. The loss itself is chance: four more retail runs differing only in menu
timing lost between 1051 and 1745 ticks in, the first native drowning between
818 and 872, as the people's wandering and the Type18 hunting them decide the
second casualty. A finished task stages the body's removal (`FUN_00410B70`).

**Type118, the Geyser.** It lives in class0 until `FUN_00410C10` selects its
direct alternate class65: `FUN_0040C500` clears T then S and `FUN_00407BC0`
publishes a Primary without timeout whose word is `+88 << 8` (or the body's
height over the sea, with no model extent). Style `0x004C7540` has no
callbacks or policy words, so unlike class0 its effective `0x20027` lacks
`0x2000` and coarse visits run E870's walk too. `FUN_00407CA0`:

- a detailed visit sprays class-13 particles, one per started 30 ms of
  callback time, each drawing X, Y and Z velocity steps and a Y position step
  (four RNG words) that accumulate over the visit; class 13 has a no-op update
  and collision mode 0, so it flies straight for 30 ticks;
- every visit then runs `FUN_00407DF0` over the live list: a body above the
  geyser, within `dy/2 + 50` of its axis and under 1200 units up, gains
  `((2000 - dy) * 4 * (dt << 13)) >> 31` on its Y velocity word (the player
  too; a selected Type9 acknowledges the write as it does a radial impulse's);
- `FUN_004013A0` finally moves the sea while the word lasts. The geyser never
  removes its body.

Retail's world-40 Type118s read `(25856, -3296, 23552)` and
`(10752, -2848, 30208)` with `0x08468805` at their first visit.

**Type36, Generate Flares.** Class16 runs from birth: `FUN_0040B8B0` clears T
then S and `FUN_00406300` publishes a Primary with no timeout or private data.
Its style `0x004C77C8` has no policy words, so coarse visits run E870's walk
too, and `FUN_00406390` ignores the visit's detail argument and returns 0.
Every visit draws twice around the player (`FUN_00456F90`): X gains
`((r >> 3) & 0x1FFF) - 0x1000`, Z gains `((r >> 4) & 0xFFF) - 0x400`. A point
whose nearest cell's material has ground response selector 7
(`FUN_0042EA30`), or whose containing cell lies under the sea, gets a still
class-47 particle owned by the generator, on the higher of the ground and the
waved sea (the sea itself with waves off). `FUN_00410C10` takes the direct
alternate, class16 again: a dead Type36 keeps generating. Retail's world-43
Type36 reads `(-32512, 2112, -31744)` and `0x0E460005` at its first visit.

Flares (classes 47 and 48) sweep entities only (descriptor mode 4, through
`FUN_0043F980`) and drift on the shared RNG (`0x0043F450`). A touch runs
`FUN_0043F660`: the F610 burst, then `FUN_00410D30` with packet `0x004CC060`
(channel 2 500, channel 3 6000) at a ratio of `elapsed >> 2` over 0xFFFF,
four times the amount on Type119, with 410D30's presentation (the `+0x34`
throttle, the sound unless dying, the capability-8 class-5 puff). It returns
0, so the flare lives on. Class 47 has radius 600 and 43 ticks of life,
class 48 radius 100 and 20 ticks. A 20-ms touch costs the player 938, as in
retail's world 43, whose player took 69 such hits from 247 ticks in.

**Type74, the Remote Beamer.** Unlike the others it carries Sub-K (one
model word, payload `[1, 0]`), death cue 62, capability `0x1224` and an axis
of 512/8. `FUN_0040C530` clears T then S and `FUN_00405420` publishes a
Primary with no timeout: while the body is not held (`0x1000`),
`FUN_004054C0` adds the visit's microseconds over 16 to Sub-K word 1,
saturating at `0xFFFF`, which it then keeps; a held body's word drops to 0,
and the destructor `FUN_004054A0` zeroes it with the task. Its `+C0` has
`0x2000`, so coarse visits skip the walk and the next detailed one charges
the time since the last. The model reads that word in selector 1 and, as
`FUN_00410090` gives every row the `{40D320, 40D350}` pair, the 50-Hz clock
in selector 0. Style `0x004C72B8` has a null attach (`+08`), `FUN_0040D1C0`
as its release (`+0C`), so a carried beamer attaches and releases as
Type68's weight does and a drop reselects class59 with a fresh word, and
`FUN_0040CAF0`
as its pair callback (`+18`). A lethal hit or the Main Base abort explodes it
through class1. Retail's world-21 Type74 reads `(-14080, 3008, 32512)`,
50000 health and `0x08468805` at its first visit.

`FUN_0040CAF0` is the beam. Against a body whose live capability has
`0xC00` (a person) it takes the beamer's `+60` relation as the receiver, or
else the first Type6 in the live list (never the list's last node, which
the walk does not test), whose `+60` it then sets to its own handle. With no
receiver it returns null and the pair bounces as usual. Otherwise it
constructs a Type93 at the receiver's X, Z minus 0x100, on the terrain
there, plays sound 8 at the person, attaches the person to the Type93
through `FUN_00408F00`, plays sound 8 at the Type93 and returns the `0xA300`
object `0x004BE330`, whose vtable entry is the same RET as capture's: the
pair's physical response is cancelled, while the person's behavior callback
and both task chains still run (the beamer's Primary keeps `05FF0`'s null
`+18`). `08F00` attaches without a release, so an already attached person
stops with a logged block. `08F00`'s bit `0x800` and `0x40000` clears now
apply to every cargo, people included, as its instructions do.

The Type93 is born 256 units from the Base's centre, inside its body. The
Base's own 11AD0 pass meets it as a successor: `258A0` returns at once for a
body without capability `0x800`, and the Materialiser style `4C82C0` and
both Primaries (`25CD0`, `090C0`) have null pair hooks, so only 12760 runs
and the fixed Base pushes the Type93 out (`main_base_conversion_live`, after
the conversions of the same pass). Each push is a movement for `091B0`, which
keeps resetting the settle timer, so the person stays held longer than at a
drop on open ground.

A tampered retail run of world 21 (Skip to World, then the person
`04870001` written onto the beamer `048F0001` for ten passes) beams it at
tick 846: a Type93 at `(-14592, 3008, -31744)`, the Type6's `+60` set to
`04860001`, the person's state `0x00C68825 -> 0x00C21025` and its parent
the Type93. The Base pushes the Type93 to Z -31931 (Y 3228), -32052,
-32090, -32098 and -32100; the person is let go there at tick 943 (Y 3009),
walks, and is converted at the Base at tick 1197. The port's [beam test](../../crates/v2k-game/src/native_actor_capture/pair_remote_beam_tests.rs) repeats
the beam and the pushes (-31932, -32054, -32092, -32100, -32101) and the
release where the Type93 settled; the Base's visits come at RNG-driven
intervals in both, so the release tick is not compared. In an untampered
retail run of world 21 from the Skip to World arrival to its casualty loss
1,774 ticks in, no Type93 was born, so no beam happened there.

**Type110, the Queen.** Class67 runs from birth and is her alternate too;
her eight Sub-K words carry her joints. Her guns, her held Type68, her weak
spot and her dying are in [TYPE110_QUEEN.md](TYPE110_QUEEN.md).

Hits reach all six through `class0_impact`: classes 36, 38, 16 and 59 have
null hit callbacks, like class0; class67's `+28` is the Queen's
`FUN_00421990`. A lethal hit, like the Main Base abort, runs
class38 for Type50, class1 for Type89 and Type74, the Geyser for Type118,
class16 for Type36 and class67 for Type110. Regression coverage is in
`entity/class0_actor/{sea_level_tests,flare_tests,remote_beamer_tests,queen_tests}.rs`,
`queen/tests.rs`,
`die_when_moved.rs`, `geyser.rs`, `generate_flares.rs` and
`flare_damage.rs`.

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

The hold's class0 bodies, the Megablast pod (Type65, class55) and the
Defender Ball (Type63, class50), have capability `0x40` and no
`0x10001000` state, so the abort's walk gives each `10C10` as well: the same
remote/dying entries, cue 62, alternate class2, tasks cleared and removal
staged. A pod inside its own visit (its blast) has no adoptable graph and
stops the abort with a typed block.

Regression coverage lives in `tests/shared_world_construction.rs` and
`entity/class0_actor/{tests,live/tests,cargo_tests,later_world_tests}.rs` and
`class0_impact/tests.rs`, with abort controls in
`entity/class0_abort_tests.rs`. It distinguishes corpus publication from
controlled callback-enabled timers, fractional grounding, model/angle/parameter
variation, wave policy, RNG ordering, pending and stale custody, and release.
Quiet Type62 constructor/abort is owned with the Type62 native receipt.
Remaining eligible cargo constructors other than Type68/92/96/97/100, and matched-retail
Intro2 acceptance, stay open. Factory-born
Type61 abort is owned with the factory constructor receipt. Native Type54 abort
is in [Factory System](FACTORY_SYSTEM.md); Type72 and Type85 take the same class38
path, Type85 with its ripple and meteors in place of the burst. `405040` derives the sea delta only from a zero `+0x88`; a nonzero word is
taken as is, so Type72's delta is `0xB4 << 8` and Type85's `0x1B0 << 8`. Type106
dies quietly through class2, as Type52 does.
