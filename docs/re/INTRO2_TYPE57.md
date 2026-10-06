# Intro2 Type57 bat

This document owns the native profile required by authored spawn1, runtime
entity2, Type57, model122 `bat`. Native construction, class7 Search-and-Attack, dormant B2 clear, method24 / particle68 / sound93 fire,
incoming Type13 class38 hits, alternate11 Tumble Out Of Sky, tumble contact, and terminal explosion
are implemented. Shared task and damage contracts remain in
[ACTOR_RUNTIME.md](ACTOR_RUNTIME.md) and
[ENTITY_DAMAGE_AND_DEATH.md](ENTITY_DAMAGE_AND_DEATH.md). Whole-scene acceptance
belongs to objective08.

## Authored profile

The canonical normal-tier cache (`1X3XX` followed by level50) supplies:

| Field | Type57 value |
|---|---|
| Spawn, position, Euler | spawn1; raw `[0xB400,0x0200,0x0600]`; `[0,0,0]` |
| Model slots | `[122;4]`, model name `bat`, extent240, collision radius330 |
| Mass, capability, health, default flags | `100`, `8`, `5000`, `8` |
| Damage thresholds, channels0..6 | `[0,10000,1000,0,0,200,0]` |
| Damage Q8 multipliers, channels0..6 | `[0,256,256,1024,0,64,0]` |
| Components | D/E/G/K/L; no A/B/C/F/H/I/J/M/N/O |
| Common axis | strict radius7680; filter word5 |
| Root selection | rule1, sole Always choice of weight2 for class7 |
| Alternate | class11 Tumble Out Of Sky |
| Surface and low-health effects | selectors `[0,0]`, lifetime0, effects `[0,0,0]` |
| Type-header sounds | constructor attachment, hit, infected, generic hit, death, warning and Search prelude/Aim cues are null |

Sub-D is `[64,1,0,512,256,0]`: signed steering divisor64, yaw-to-roll
coupling, no pitch steering, probes512/256, classifier flags0. These bytes
equal Type13's descriptor, while this allocation owns seed01. Do not import
Type13's runtime receipt or seed0.

The 104-byte Sub-G payload equals [Type10's](INTRO2_TYPE10.md) except for
word `+0x0E` sound70 instead of63 and dword `+0x20` maximum linear velocity550
instead of750. It retains force words0/100/0, target-clearance base700, rate
range50..130, dword `+0x18=300`, pitch limit16384, surface mode0, and the
seven bindings `[1,2,3,4,5,8,9]`. Sub-K is `[11,10]`; Sub-L is
`[6,7,0x40,0x1F,0x40,0x1F]`. The eleven-word animation bank belongs to
model122's hierarchy, including its mirrored model123 wings and model126
child. Matching selector numbers do not authorize using another model's bank.

Sub-E supplies method24, random interval600000us, spread256, aim threshold16000,
speed override0, target-axis tolerance7680, sound93, raw word `+0x12=42`, and
zero alternate/gate/auxiliary/binding bytes. The raw word42 is not a joint
binding. Method24's executable row at `004D02C0 + 24*0x10` is
`{leading0,speed2000,particle68,entity0}`.

## Constructor and dormant scheduling

`104B0 -> 09A80` allocates and clears the animation bank, then constructs
G (`1B8C0`, one shared random word), D (`203D0`), K (`24450`), L (`1BB80`),
the common axis (`235A0`), and E (`24E30`, no random word). K/L state and
their bank outputs start at zero. G's target starts at `700 + (word >> 8)`;
its five phase bytes, angle `0x5000`, zero auxiliary angle and self-righting
mode1 come from the common constructor. E starts with method24/sound93 and
zero cadence/control words and null bindings.

Default bit0x20 is clear, so D4A0 preserves authored Y and the zero-Euler
constructor basis. `AC60 -> 425680` consumes a selector word even for this
singleton choice. Class7 B6C0 copies only the type's common-axis `+4`, clears
Tertiary, and installs Secondary `2050/2080` before the 500-ms Primary
`2B10/2BA0`. Each successful `6030 -> 6070` suffix resets G mode, draws one
G target word, and resets its force/direction controls before publication.
Successful birth therefore consumes four words; living C690 reselection
consumes three. It retains the current axis radius, target/auxiliary context,
component allocations, animation state and E cadence. Allocation failures
retain their reached prefixes and enter the existing initializer fallback.

The accepted-capture ledger
retains two Sub-D constructor transcripts:
`20260730-034232-sub-d-constructor-provenance.txt` and
`20260730-035135-sub-d-constructor-provenance.txt`. Each independently joins Type57
(hex type39), position `B400,0200,0600`, handle `04FB0001`, and process-counter
`01 -> 02` to runtime seed01. Since descriptor classifier flags are zero,
`41F660` performs stagger maintenance then jumps to `41FC90` without consuming
allocator cache origins. No first-classifier query or copied origin is needed.

Type57 does **not** have proven constructor-zero B2. Its own accepted
`20260722-022303-intro2-actor-ai.jsonl` records the following history:

| Capture line | State and timing | B2 / callback mass |
|---|---|---|
| 1026 | birth handle `04960001`, flags `07410805`, original pose | B2 `0x1577`, stored mass100 |
| 1309 | tick4, flags `01410805`, age85000us, unchanged pose | B2 zero |
| 44029 | tick1100, first sampled enabled flags `01478805`, original pose | B2 zero, mass100 |
| 44061 | tick1102, flags `01478825`, Search pursuing style `4C7A98`, Y499/VY−138 | B2 zero, mass100 |

Post-load capability8 clears callback/motion/damage mask `0x68000`.
The authored operation2 at22000..22500ms releases spawn1; camera selection
at39000..39500ms is separate. See [INTRO2_ACTIVATION.md](INTRO2_ACTIVATION.md).
`12DA0` reads mass+B2 only inside callback gate0x20000. Both its wait path
and ordinary disabled-callback exit clear B2. A native dormant owner can
therefore preserve unresolved birth residue until this source-owned clear,
without inventing constructor-zero or blocking a callback which cannot run.
The passive samples do not identify the exact first mass-read instruction,
but do establish this allocation's preactivation clear and sampled live mass.

## Living task, weapon and impact ownership

Class7 acquisition uses radius7680 and capability mask5 over the current
intrusive list, with the shared wrapped-distance, recent-relation, state and
dying gates. C7D0/ADE0 publishes Chase Primary and Aim Tertiary against the
same accepted target. The newborn Primary waits for the next visit; the
remaining dispatcher cursor visits the new Tertiary once in the current pass.
Source `40354E` tests Sub-H, which is absent, so it skips the extra
proximity-triggered Sub-A controller even though G exists.

`01430` uses the shared G/K/L mover with Type57's own descriptors, retained
incoming basis and component state. Detailed mode runs G/K/L; restricted mode
runs G while retaining K/L outputs. The outer DCA0/E870 tail then publishes
F70's complete Euler basis, applies E100 with effective flags8, runs E370's
timer-only path, clears B2 and performs master integration once. Preserve
target/L prewrites, D cache/angle writes and G animation/audio/attitude writes
when a later phase blocks; a parked owner must not replay their RNG.

The shared Aim transaction can use an authenticated Type57 profile and its
native E cadence. Method24 inherits source velocity at the later11400/4E770
drain, unlike method10. 410B0 has no class68 substitution in either sea branch,
so it reaches the ordinary descriptor creator, retaining FIFO order,
owner/type, suppression and raw Y bias.

Class68 is not an F590 bullet. Its descriptor has F260 gravity,
`442420` surface, `442950` entity hit, packet pointer `004CC108`, null static
response (`42E8E0`), and presentation `442240`. `442950` first calls
`411180`, then F610's hit effect, then, when suppression is clear, attaches
class84 to the target. The sibling source-class52 branch chooses class83;
other members choose class86. The attached offset is impact minus target
position; capability1 targets get zero Y and signed-truncated three-quarter
XZ offsets.

`411180` differs from10EB0: stamp+34, alive target+80 sound, type-vtable+14
callback (DAC0/current style+28), `11030` with eight times the channel1/2
amount returned by25590, then15040. It has no10EB0 capability8 post-hit
class5 suffix. Reusing the primary or infected wrapper would change order
and force. This static route is in `bulk/game_logic.c` at411180 and442950.
The model sweep now admits the 442950 entity-hit family (classes 52/68/85)
with their descriptor-selected packets (`004CC0F0`/`004CC108`/`004CC048`;
class68's `[2,6]/[2000,2000]` row recovered by a static executable read
cross-validated against seven documented packet rows), consuming 52/68 static
contact through the `42E8E0` no-op and delivering `F800` for 85. The 411180
damage prefix (stamp, pre-callback `+0x80` sound, DAC0, 11030 at eightfold
force, result-blind 15040) is owned for native Type9/26 in Intro2 and their
owned ordinary authored worlds, Type13 in Intro2, and
[native fish](FISH_RUNTIME.md#particle-hits-and-quiet-death). The prior Type13-only Intro2 router
rejected peasants; the shared native Type9 owner now delivers class68's5600
filtered hit and publishes the native Class14 death graph. Retail's accepted
actor-AI sample links a class84 birth to a peasant health1500-to0 transition at
tick2058. Details and reproducible evidence are in
[Bat projectiles and peasant damage](INTRO2_COMBAT_PROJECTILES.md#bat-projectiles-and-peasant-damage).
Other targets fail closed instead of entering F590-style owners. The attached allocation
(83/84/86 by source class, owner handle, capability-gated offset) and its
`FUN_004425D0` follow/kill lifecycle retain global owner handles through death
until unlink, including the direct and native Q31 basis-rotated branches.
Damage precedes F610; suppression gates the attachment but not11180.
Class68's `442420` surface dispatch now follows the shared descriptor owner
described below. The complete83/84/86 attached callback now owns B2, follow,
`4417E0`, marking, scaled direct15040, ratio/age and cascade order on native
living owners as well as the native postdeath Type9 class84 zero-return path.
Damage/death run synchronously before the suffix rereads the current physical
particle and the original authenticated allocation's post-callback velocity.
Unowned callbacks, absent/rebound cached allocations and erased free-slot
memory remain named per-particle committed-prefix blockers. The
[attached callback contract](INTRO2_COMBAT_PROJECTILES.md#attached-callback-and-direct-checked-damage)
owns that distinction. Full
firing acceptance must include them, rather than stopping at successful
particle allocation.

### Projectile surface replacement

The original executable's `442420..4425CD` switches source class before
material response: classes52/53 select74, classes68/69 select76, and class87
selects88. Selector6 replaces any admitted source with73; selectors0..5 and
8..12 select its source-specific replacement; selector7, out-of-range
selectors and unsupported source classes return1 for deletion. Successful
replacement keeps the physical slot and provenance, writes the response Y,
relinks its draw group, and clears age and all velocity words without allocation
or RNG. The production surface owner now admits this descriptor family rather
than only the Type47 class87 shot.

`40120` preserves an important asymmetry: a water replacement continues the
originating mode3 visit's ground tail with the replacement descriptor's
`43E3D0` callback, without rerunning update/entity/static sweeps. Its selectors
0..6/8/10 replace with73; selectors7/9 attempt a `441770` child, then delete
the parent even if allocation fails; selectors11/12/out-of-range delete.
The child retains the current emitter's identity and parent suppression bit.
All four replacement descriptors are collision mode0, so later visits skip
contacts entirely. Ground-first conversion therefore does not call E3D0 a
second time. These rules come from the original PE jump tables at442570,
442588,4425AC,4425C0,43E4C4,43E4E0 and4407B4; they are independent of a
captured scene or seed.

Focused controls cover all five source classes, selector boundaries, physical
slot/group/provenance preservation, same-visit water/ground order, subsequent
mode0 visits, and child suppression/allocation failure. Entity/static sweep
and attached damage/cascade ownership remain separate; this surface repair
does not certify complete class68 or matched retail scene acceptance.

The normal-tier 640x480 production-main comparison completes the whole intro,
Klaus closing/opening and1024 first-world frames at fixed40ms/frontend100 and
mixed16,667/33,000/40,000/125,000us/frontend250. Both retain18 required scene
readbacks and report no runtime blocks or dropped owners. Fixed early frames
through40seconds are unchanged; the recovered surface path changes later
shared-RNG actor trajectories and combat timing, while all three turrets and
the factory still retire and both dragons survive. These are port regression
controls, not matched retail acceptance. The full V2000 repository gate and
release build pass alongside the focused surface tests.

Incoming primary hits use10EB0/DAC0 and infected hits11250/DA00. Both class7
living styles (`4C7A50/4C7A98`) have C690 at+20/+28; completion/fallback and
class11 styles have null hit slots. Preserve actual allocation/current graph
custody through reselection, impact and damage. The varied-run class38 packet
`[2,3]/[500,6000]` filters to24000: channel2 is below threshold1000 and
channel3 contributes `6000*1024 >> 8`. It requires death from authored
health5000; the current fixed-target rejection applies none of that prefix.

## Living terrain and water contact

Living Search bat styles have null+10/+14 hooks. D7F0/D860 nevertheless apply
generic141D0 after those hooks; the former Tumble-only contact adapter skipped
this response for the living bat. The shared surface kernel now uses the real
model122 geometry, native task custody and checked collision-damage path. Flat
and sloped downward-displacement controls cover16.7/40/125ms, while underwater
descent and resurfacing remain unclamped. Ground lethals publish the existing
native class11 Tumble; subsequent water contact rereads the changed C750 hook.
Already-falling bats keep their existing terminal resolver. The shared source
contract and remaining ordinary-birth/static/pair boundaries are owned by
[Flying actor terrain and water contact](FLYING_SURFACE_CONTACT.md).

## Alternate11 and implementation boundary

Type57 shares Type10's source alternate11 path:10C10 writes health0/DYING
after the remote/already-dying gates, then C660/404360 clears Secondary and
Tertiary, runs the successful common G suffix, sets G+3F=1 via1B970 and
publishes the zero-lifetime Tumble Primary. There is one constructor G word,
no selector draw and no common12 vertical launch. Null Type57 sounds do not
authorize moving mutable component/graph validation ahead of the health/DYING
prefix when a later phase can block.

404460 retains elapsed age; detailed mode rotates by
`max(1,3-signed(age_ms)/1000)` times the source dt shifts, then calls018A0
directly. There is no target prelude or Sub-D step. Tumble G clears its two
forces and skips ordinary wing/audio/terrain attitude; K/L use retained state.
Late terrain/water/static contact selects C750/BAC0 terminal. A timer or a
guessed ground height must not trigger it. Its type-header radial values are
inner512, outer1024, impulse2000, packet `[1,3]/[4000,4000]`; provenance is
Type57 and this handle. BAF0 sees G and emits the common class37 scatter,
followed by40950's sea/audio tail, radial continuation and one-use deferred
removal. The existing Type10 terminal helper hardcodes source10 and must gain
an explicit source-bearing request before being shared.

The smallest justified implementation is a native DEGKL Search/alternate11
family with explicit Type10 and Type57 profiles, retaining separate authored
birth identities and data. Reuse the existing GKL frame, class7 acquisition/
Chase, typed ballistic Aim transaction, impact phases and class11 contact
algorithm; avoid duplicating all of `intro2_type10` or widening its type gate
while retaining model351, seed22/23 or method10 assumptions. The decision
matrix must preserve Type10's two rotated births, divisor48, mask0xC85,
health32000, method10/class38 and source-velocity exclusion, versus Type57's
one zero-Euler birth, divisor64, mask5, health5000, method24/class68 and
source-velocity inheritance. Type13's additional class5 selection remains
outside that family.

Focused acceptance must cover own four-word birth order/seed, dormant B2 clear,
repeated class7 replacement and cadence retention, detailed/coarse mover output
with Type57's sound/velocity data, incoming lethal class38,
contact-selected terminal/provenance, and method24/442950/411180/attached84
allocation/follow ordering with complete direct scaled15040/death and current
particle/original-allocation custody, with unowned target callbacks and missing
record memory held explicit. Then replay both
fixed and varied frontend/frame histories through the complete intro and first-world handoff.
Existing source and accepted captures suffice for the recovered constructor/
mover/class11 profile; no new retail recording was needed. Native birth,
dormant B2 clear, class7 live motion, method24/particle68/sound93 fire,
incoming lethal class38 hits, and alternate11 tumble/terminal are implemented,
as are the 442950 sweep, the 411180 Type9/13/26 damage prefix, the attached84
allocation/follow lifecycle and the descriptor-owned 442420 surface replacement.
The complete disabled-damage83/84/86 callback, including B2, `4417E0` and cascade,
is implemented. Enabled attached damage and cached entity-record custody remain open.
