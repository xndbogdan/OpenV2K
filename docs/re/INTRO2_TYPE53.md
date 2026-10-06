# Shared Type53 actors

The model302 actor's shared constructor, task owner and hit binding live in
[`intro2_type53.rs`](../../crates/v2k-game/src/intro2_type53.rs). Ordinary and
native Intro2 loads retain their actual authored allocation and process Sub-D;
the explicit captured-replay adapter retains Intro2 spawns20,26,38,41 separately.
Shared task semantics remain in [ACTOR_RUNTIME.md](ACTOR_RUNTIME.md); the
whole-scene gate is Intro2 acceptance.

## Construction and current task

`104B0 -> D4A0 -> 381F0` builds A/B/C/D/H/J and the immutable task anchor before
linking the new actor. `425680` evaluates only the preceding live allocations:
Always x1/class33, Under Attack x20/class7, People Nearby x4/class9, and Player
Nearby x5/class7. Nearby scans use the authored common-axis range0xF00 and
filter0xC05. Sub-A construction consumes its own word before the selector;
each successfully prepared acquiring task applies its own 06070 suffix.
The selected B6C0 or B740 graph is published before the next spawn is admitted.

[`construction.rs`](../../crates/v2k-game/src/intro2_type53/construction.rs)
accepts the actual Section-13 row and linked prefix without a world/spawn list.
The prefix includes the persistent player where present. It retains the
caller's pre-D4A0 surface classification, terrain-dependent model-slot bit,
authored Euler words and damage buffer. H/D storage precedes20450's Sub-A word;
D4A0's bilinear terrain snap plus Sub-C clearance50 and immutable+90 anchor
precede the nearby predicates and weighted selector. Fresh+34=0 makes Under Attack false at every
construction tick. The acquiring task suffixes follow selection, and104B0
builds the body basis only after the initializer has run.

The supported profile is model302 in all four slots, mass100, health3000,
capability8, A/B/C/D/H/J, six authored Sub-H records and one policy1 Sub-J slot
at local `[0,0,120]`. Sub-A has acceleration1500, correction-3000 and base400;
the constructor uses the actual initialized Sub-D pair, not a captured seed.
Different model/component profiles or animation/configuration payloads fail
explicitly. Native construction uses the shared empty-cache and zero-B2 residue
policies described in [cross-level construction](CROSS_LEVEL_GAMEPLAY_RUNTIME.md#native-ordinary-world-construction);
these policies do not claim arbitrary retail heap residue was observed.

The construction receipt distinguishes `Native` manager generation from
`CapturedIntro2`, retaining the authored index, model slots and grounded anchor.
Live, hit, descriptor-contact and Class12 entry authenticate the issuing
manager as well as the current graph. A transplanted native receipt cannot
become a captured replay merely because its numeric spawn index matches.
The runtime lease retains allocation, current context and all three task IDs.
Completed transitions refresh that lease. A failed frame retains its current
committed graph as pending and cannot replay elapsed-time or RNG effects.
Primary shared retarget, Secondary target acquisition and Follow Beacons run
in the actual slot order. Successful beacon acquisition clears its executing
Secondary and publishes variant1/Primary through AFD0; the old wrapper's
non-survival at callback unwind is the expected success case.

The shared Following phase calls the type-specific common mover before reading
the target again, then evaluates 423030 with the actor's controller range and
tests wrapped X/Z differences strictly below0x300. The null reached callback
leaves a successful visit continuing. Task expiry is strictly greater than9000ms
after callback unwind. C690 may replace the graph, and the remaining dispatcher
cursor visits its new Secondary in that same world frame; it does not visit a
new Primary twice. Type26 shares this task phase while retaining its own mover,
component metadata and first-query receipt.

## Capture People pursuit

Class9 acquisition owns the executing Secondary through `C7D0 -> C6B0`:
the selected target is stored at context+8 and style `004C8038` is published
before `AEE0` runs. Its `416360` warning unconditionally stamps the target's
entity+34 with the current retail tick, then plays the unsigned type-record
+92 cue only when the target is not dying and that selector is nonzero.
This is independent of health damage and of the accepted-hit sound at+80.

`AF50` clears Secondary then Tertiary, prepares the `403650` Primary, and only
after successful allocation/private initialization applies the shared06070
Sub-H enable and Sub-A direction/RNG reset. The final signed `base*4/3` speed
is533 for Type53, overwriting the randomized speed before publication. Its
absent Sub-F (type+C8+1C at EXE40374A) skips the optional424390 suffix. Failed preparation retains
target feedback and the clears before C6B0 installs its initializer fallback.
The old executing Secondary unwinds after its retirement; the new Primary
has age zero and first runs on the next actor pass.

[`capture_pursuit.rs`](../../crates/v2k-game/src/intro2_type53/capture_pursuit.rs)
binds the distinct Capture task family to shared403780. Target validity and
the strict wrapped423030 route test precede the Type53 mover. A zero route
still runs the mover, then returns its own tagged singleton. Invalid targets
skip the mover. Tagged returns and expiry strictly beyond5000ms invoke C690
after callback unwind; any newly published Secondary runs in the remaining
same-pass slot order. Target/mover prefixes remain committed on a later block.
Static EXE authority and the four accepted first-query joins suffice for this
phase; no Capture-specific trajectory or new runtime oracle is assumed.

## Search pursuit and the absent-emitter Aim task

Class7's C7D0/C6B0 handoff publishes style `4C7A98` before ADE0. EA10
applies its literal `+34=0/+38=80h` policy before the initializer: reversed
D440 sets entity bit `8000h`, preserving every unrelated bit. This state
write remains committed if ADE0 later fails. ADE0 then installs Tertiary
Aim, clears Secondary and installs Primary Chase. Type53's null `+9A` cue,
absent Sub-F and signed Sub-A reset `400*4/3=533` consume no constructor RNG.

The shared [live Chase wrapper](../../crates/v2k-game/src/chase_target/live_primary.rs)
retains each actor's own mover and D state. `03490` validates the target before
reading the live common axis or invoking `01430`. Missing, zero-state and
dying targets return tag `9C01` without moving. A valid out-of-range target
still runs the mover before returning its different `9C01` singleton; an
in-range zero mover return has a third singleton with that same tag. Both
detailed and coarse Chase visits run the mover. EXE `40354E` tests
type+C8+24, the Sub-H pointer at type+EC, rather than Sub-G. Type53 has H
and A, so a successful in-range mover re-resolves both positions and applies
the wrapped X/Z controller: both distances below0x380 restore signed A-base
speed400 and set A/task direction to-1; both below0x400 otherwise write speed1;
an outer distance restores400 without changing either direction. Out-of-range
or zero-mover tagged returns skip this phase. Sub-F's constructor hook is a
different field. Staged
target/cache writes survive a later failure, and the exact wrapper unwinds
before transitions or panic propagation. Retired wrappers cannot reselect.

The [native Type53 Aim host](../../crates/v2k-game/src/intro2_type53/aim.rs)
runs `02300` even though normalized Sub-E is null. `01120` first adds
`floor(dt_us/1000)` to task age. A nonzero mode then skips all target,
emitter, sound, matrix and RNG reads. Detailed mode checks target existence,
nonzero state and `4000h` before the emitter-null return. Type53's authored
Aim cue and period are both zero, so valid visits consume no RNG and create
no sound or projectile. This is an executed task callback, not an invented
weapon or omitted scheduler visit.

An invalid detailed target returns `4BE158/9C00` before checking the strict
`age > 5000 ms` expiry. D7A0/current style `+04` and D760/`+00` both call
C690 for this style. Each uses 16410's entity bit `1000h` suppression; entity
bit `80h` is not that gate. Suppressed `+04` falls through to the expired
timeout check, which repeats the same gate. After Primary reselection the
cursor rereads Secondary and Tertiary: a new Primary waits until next pass,
while a newly acquired Aim runs once in the current pass with its actual
callback mode. The acquisition helper no longer also ticks that newborn Aim.

## Common world callback

12DA0 retains subject/callback carries, random waits, detailed/coarse selection,
mass `u16(typeMass+B2)` with zero promoted to1, B2 clearing and master motion.
Native birth applies the explicit zero-B2 residue policy once; later callback
contributions are not overwritten. Detailed DCA0 alone runs the post-task sound gate before E640/F70/
E100. The shared sound policy reads health as signed dword and period at A4/B0
with the original signed period test, unsigned wrapping `dt<<6` division and
inclusive random comparison. Positional admission and resource aliases consume
their own RNG later in the audio pass.

The shared Type17/53 ABCDH phase preserves the incoming body matrix through
Sub-D steering and C/A/B; 20360 changes only angles. DCA0/E870 rebuild13F70
after the complete task walk. See [the native Type17 ordering audit](INTRO2_TYPE17.md#matrix-and-world-frame-ordering).

The native Type53 tail owns effective flags0x39, terrain attitude, body basis,
gravity/drag and the outer motion suffix. Its task callback does not run11AD0;
static and active-pair contact belong to the later world scan. D360 external
geometry is written during actual visible model submission, after view/normal
and whole-hierarchy far admission, rather than at mover completion.

The living E370 tail now commits its underwater timer before random effects.
At less than75% lifetime remaining it consumes the bubble gate, the five payload
words only on a successful gate, materializes class42 synchronously, and then
evaluates the independent sound106 gate. All currently admitted Type53 styles
have null release callbacks. At expiry,162B0 writes the timer, runs16750's
authored-default/state/parent reset, and invokes10C10 to publish native class12.
The new task starts on the next actor pass because the living Primary cursor
has already passed. A later emission or master-motion failure retains that
death receipt without replaying the retired living graph.

E370 caches model header+08 before the lifecycle callback; bubble position,
forward basis, type and state are read afterward. The now-dying state suppresses
the sound gate without suppressing the bubble gate. Exact expiry has remaining
percent zero; an overshoot preserves the original unsigned wrapping division
and can skip the random suffix. Type26 uses the same surface owner with its own
model267 extent,30,000-ms lifetime and authored0439 default; Type53's lifetime
is2,000ms. Type26's native class12 retains mass400, Sub-A base200, absent Sub-E/Sub-J
and effective flags0428. Detailed class12 visits execute DCA0's post-task sound
phase before F70: current health selects A0 below half health or visible94/96
at/above half. Dying4000 does not suppress that phase; E870 omits it entirely.

## Particle hits and checked damage

Primary particles enter10EB0: stamp entity34, type-vtable14/DAC0 to style28,
11030 impulse, then415040. Infected class5 enters11250: set2000 and its authored
82 cue, type-vtable18/DA00 to style20, then11030 and415040. It does not stamp34
and has no primary accepted-hit suffix. The static class5 descriptor at4CBFD0
is `[6,0,2000,0,0,0]`, so its source/owner words are zero independently of the
particle birth owner.

The admitted styles at4C7A50/4C7A98/4C7AE0,4C7FF0/4C8038,
4C7B28/4C7B70 and4C7ED0 have individually verified matching20/28 callbacks.
Non-null C690 reselects immediately, including without the task wrapper's16410
suppression gate. The scheduler adopts that replacement graph before the next
hit or world visit. Capture variants2..5 retain their separate D040 boundary.

[`live_actor_checked_damage.rs`](../../crates/v2k-game/src/live_actor_checked_damage.rs)
shares the local offline415040/414E10 to14E90 phase with live radial delivery.
It preserves the full six-dword packet, modifier boundary, remote gate, positive
buffer write, dying gate, generic sound, type hit callback, health subtraction
and standard-death call in order. Filtered-zero feedback requires capability8,
source46, and `(channel0 != 1 || channel1 != 0)`; amount0 is not that second
predicate. Radial falloff's414E10 entry omits that checked-only feedback.

Lethal native Type53 hits publish the existing class12 owner synchronously.
A later feedback boundary retains the completed death receipt separately from
the original10C10 return. Only primary nonzero415040 returns execute the80 cue
and capability8/class5 suffix, including its current-model extent subtraction
and dying-state sound gate. Main Base abort reuses this same
`publish_intro2_common_standard_death` owner with Type53 allocation and
scheduler custody, so ordinary worlds 14/15 no longer block on this family.

## Contact integration

The shared native-spider incident scan now admits ordinary Type53 counterparts,
including overlay14's authored spawn35, through the same construction receipt
and completed scheduler owner used by live callbacks and hits.
[`native_actor_descriptor_contact.rs`](../../crates/v2k-game/src/native_actor_descriptor_contact.rs)
classifies each current Primary/Secondary/Tertiary task's actual null or02DA0
hook, preserving typed private state, source half-space/RNG order and incoming
basis. Physical response uses direct15040 and the shared Class12 death owner;
it does not replay projectile callbacks, a mover or task time. Foreign and
parked prefixes block before mutation.

The owned Type53 late11A80/11AD0 scan now runs in ordinary worlds and native
Intro2 through [`intro2_type53/contact.rs`](../../crates/v2k-game/src/intro2_type53/contact.rs),
following the shared Type17 static-contact owner: deepest-static scan with the
entry model, the eight-style allowlist, read-only scheduler custody, the02CA0
private-state commit for retarget/pursuit/Following/Chase Primaries, physical
response, and ordered static/actor damage into the shared class12 publisher.
The Chase installer `0x403360` stores02CA0 at task `+0x20` (site `0x4033C3`)
for the `0x403490` Chase behavior and its 423030/01430/`0x380`/`0x400`
controller, so a coexisting Tertiary Aim keeps its own records while only the
Primary's shared private state is committed. Class12's null hook runs the
same response without task, Sub-A or RNG writes. This counterpart integration
does not implement C910 attachment, carry variants2--5, the bare-terrain/water
surface response, or the occupied Sub-J callbacks.

## First-query evidence

The task, mover, hit and static phases now live in the shared
[native ground executor](../../crates/v2k-game/src/native_ground_actor.rs),
with separate sealed Type53 and [Type122](TYPE122_RUNTIME.md) profiles.
Type53 retains its no-emitter Aim and pursuit-only capture boundary; Type122's
E/J transport and crushing policy are not Type53 admission rules.

The accepted `V200001.run` joins cover the four captured Intro2 births. The initial walk
closes spawn20/seed13 and spawn38/seed18. A separate bounded address-read query
closes spawn26/seed16 and spawn41/seed1A through their own successful constructor,
live handle/allocation/descriptor, `00401602 -> 0041F660 -> 0041F7A8`, full-reset
branch and classifier return. Each frame receives125000 microseconds; its seed
increments once, the result is class0 and row0 becomes1.

| Spawn / constructor seed | First frame tick | First query X/Z | Resulting origin |
|---|---|---|---|
| 20 / 13 | 011E | 9300/8D00 | 93,8D |
| 26 / 16 | 02FF | C200/1501 | C2,15 |
| 38 / 18 | 011E | 9100/8D00 | 91,8D |
| 41 / 1A | 02F9 | 9500/6901 | 95,69 |

These are allocation-specific replay first-query receipts, not sampled movement
or fixed callback timing. The captured adapter's origin remains unresolved until
its own first query consumes that receipt; subsequent fills and cadence use the
shared implementation. Authored native construction instead retains the process
allocation and named empty-cache policy above. Constructor4203D0 clears rows but
leaves origin bytes unwritten, so neither constructor flags nor another captured
birth can authorize a replay reset.
The accepted ledger
owns retained filenames and complete validation joins.

## Validation

The existing Search controls cover the four captured births in detailed and coarse modes,
same-pass newborn Aim and next-pass Chase, strict wrapper lifetime, target
validation order, callback suppression and zero absent-emitter RNG/audio.
The shared-construction controls cover every ordinary Type53 allocation in
overlays13--49 and native Intro2 after earlier process history. Ordinary
particle/radial controls preserve current task custody through primary/infected
hits, exact surviving damage, lethal Class12 publication and the next scheduler
visit. Static-contact controls cover authored-pose Miss custody with zero RNG,
non-owner Ineligible, synthetic fence overlaps with the exact02CA0
direction/timer/retarget/RNG prefix for both the acquiring retarget and a real
driven Chase/Aim handoff (Tertiary Aim untouched), and a lethal
corpse applying the null-hook path without republishing class12. Parked living and dying owners reject later mutation. The full V2000
repository check passes 4,319 tests, including 3,234 game library tests; the
[combined scene checks](INTRO2_TYPE94.md#validation) do not establish matched
retail trajectories for every ordinary Type53 actor.
Fixed40ms/frontend100 GL completes125 captures without runtime issues. The
varied-frame/frontend250 run completes124 captures, including the same ordinary
actor controls, but records a separate [Type13 incoming-hit boundary](INTRO2_COMBAT_PROJECTILES.md#production-scene-comparison).

## Remaining work

Class9 pursuit, class7 Chase/absent-emitter Aim and ordinary C690 reentry are
live, and the owned11AD0 static scan now covers retarget/pursuit/Following/
Chase Primaries plus the Class12 null-hook path. Actual C910 attachment,
carry/destination/release variants2..5, the bare-terrain/water surface
response, and matched retail scene acceptance remain open. C910 needs
distinct Sub-J releaseCAD0 and destructive-consume443D10 row policies and
native task/relation custody. Living0x39 clears the surface `0x10000` prefix
like living Type1739, so the surface remainder concerns Class12 corpses.
Its A300 response must suppress
physical response while preserving the opposite style and402DA0 component
callbacks; existing player-cargo behavior does not supply this transaction.
The [spider authority](SPIDER_BEHAVIOR.md#next-bounded-implementation-order)
retains the carry and nested-cleanup boundaries. These paths stay explicit;
pursuit alone does not claim that Type53 can capture or carry a person.
