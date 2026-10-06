# Native Intro2 hut and factory

This document owns Intro2 Type66 spawns 36 and 51, their Working Factory tasks,
checked damage, progressive destruction, and the hut's terrain crater. The
shared economy and Main Base contracts remain in [FACTORY_SYSTEM.md](FACTORY_SYSTEM.md).
The same native constructor/task machinery now accepts authored ordinary-world
factories; [AUTHORED_BASE_FACTORY_RUNTIME.md](AUTHORED_BASE_FACTORY_RUNTIME.md)
owns that scope and its pending validation. This page retains the scene-only
health1 policy and hut destruction template.
Whole-scene acceptance remains in objective 08.

## Authored allocation and task publication

Both structures are cumulative Type66, with mass 1000, capability `0x84`,
constructor health 99999, initializer flags `0x25027`, accepted-hit sound 7
and death sound 62. Their only component is Sub-M: the status word is 8 and
its six selector bytes are `[4, 3, 0, 2, 1, 0]`. There is no Sub-D allocation
or first-query receipt to infer. Instance `+44` and constructor sound
attachment `+8C` are source-proven null.

| Authored spawn | Initial raw XYZ | Euler words | Model slots | Sub-M template |
|---|---|---|---|---|
| 36, peasant hut | `[-28672, -256, -32768]` | `[0x4000, 0, 0]` | `[364, 365, 364, 365]` | control `0x102`, capacity 5, effect flags `0x2A`, crater radius 1280 / depth 16 |
| 51, factory | `[20992, -320, -5888]` | `[0x527D, 0, 0]` | `[210, 225, 210, 225]` | all 22 config dwords zero |

`104B0/09A80/18A90` retains the exact 88-byte Section13 config and initializes
production storage before `D4A0 -> AC60`. Sub-M starts with zero staff and
progress, no tracked pickup, and an idle progressive-death clock. The hut's
health-per-scientist threshold is 19999 and its repair rate is 333; the
zero-capacity factory's repair rate is 1666. Its zero production threshold is
legal while the current scientist count is zero. The shared production machine still
rejects an invalid zero divisor if a later staffed path would consume it.

`D4A0` bit `0x20` grounds the structures from actual terrain. Neither applies
the bit `0x40` model-radius addition or Sub-C clearance. `D3C0` preserves callback admission
while disabling master integration. Native birth explicitly initializes the
unwritten transient `+B2` mass contribution to zero once, matching the port's
documented deterministic policy for retail heap residue. Later callbacks own
the field. This does not authenticate a captured Level1 allocation.

The sole class choice is Always, with weight 1, selecting class 39 Working Factory,
descriptor `4C9708`, style `4C9558`. Even this singleton consumes one shared
AC60 RNG word. `257C0` clears Tertiary then Secondary; `25BD0` allocates through
`01020` and publishes a zero-lifetime Primary with callback `25C60`. It does
not invoke `06070` or consume component-reset RNG. Reselection retains the existing
Sub-M allocation and context payload, replacing the graph through the same
phase. An executing replaced Primary remains allocated until its visit
unwinds.

The later Intro2 post-load pass deliberately changes **current health to 1**.
In `451710`, after `2E570` finishes all constructors and the radar build,
the intrusive actor loop first primes Type0/Klaus (`451920`). For mode `<4`,
`4519BE..4519D7` separately clears activation for Type34/capability `0x08` actors,
then writes entity `+30 = 1` for Type66. This is not a constructor parameter:
the 99999 maximum, Sub-M cached health and repair thresholds remain intact.
There is no RNG draw or task replacement. The native post-load helper rejects
repeat application and actors whose factory callback has already started.

The accepted `20260712-203635-full-session.jsonl` independently records both
authored structures at health 1 before the first scene callback; the factory
subsequently repairs to 199 by tick 6, while the unstaffed hut remains vulnerable.
The same recording observes the hut's destroyed model 365 and terrain-derived
Y `-896` at tick 309. This explains why retaining constructor health 99999 hid
its natural death even when the meteor radial delivery already reached it;
no destruction timer or inferred collision damage substitutes for this write.

Implementation: [native.rs](../../crates/v2k-game/src/intro2_type66/native.rs)
and [native_tests.rs](../../crates/v2k-game/src/intro2_type66/native_tests.rs).

## Live callback and damage ownership

The source route is the ordinary `13500 -> 12DA0 -> DCA0/E870` wrapper,
followed by `25C60 -> 19010`; there is no special fixed-actor scheduler.
Native callbacks retain detailed/coarse timing, callback-mode sound gates,
the under-attack notification latch, factory transaction receipts, and exact
animation/status publication. The structure's effective bit `0x4000` skips the
Euler-to-matrix `13F70` write, allowing crater-generated terrain alignment to
remain authoritative.

Primary particle entry stamps `+34`, while infected `11250/F780` entry first
sets the infected model bit and preserves the old stamp. Both Working Factory
and its class 0 wreck style have separately verified null `+20/+28` hooks.
The wrappers retain `11030` reaction gates and shared `15040` checked damage.
Primary sound 7 requires a signed nonzero filtered result and a surviving
non-dying target; it is independent of buffered health loss. Capability `0x84`
has no capability `0x08` class 5 suffix. Infected entry has no primary sound suffix.
Lethal particle and radial delivery publish the native replacement owner
before the next live recipient/task visit.

The native bridge uses the existing receipt-bound factory production machine.
It admits the actual unstaffed templates, status/animation updates, model
destruction effects and terminal callback. Reached product/worker allocator
paths without their native construction host remain explicit blocks; this
does not widen the separate Level1 factory birth contract.

Implementation: [live.rs](../../crates/v2k-game/src/intro2_type66/live.rs),
[impact.rs](../../crates/v2k-game/src/intro2_type66/impact.rs), and
[death.rs](../../crates/v2k-game/src/intro2_type66/death.rs).

## Progressive and terminal destruction

The first `10C10 -> 19750` entry queues sound 62, then revives health to
10000000, clears the temporary dying bit and starts the clock at 1 if it was
idle. `DB80 -> AC60 -> 257C0` consumes the singleton selector and republishes
Working Factory. Existing positive progress and Sub-M production fields are
retained. A health-only revival would lose this task and RNG transition.

`19B50` advances 100-ms model-effect stages. Stage 1 emits threshold 25. The
hut's template bit `0x20` advances the following reached stage directly to 32;
the factory's zero flags use the normal 32-stage sequence. Terminal entry
commits clock `-1`, handles the tracked pickup, writes health 0 and selects the
appropriate destroyed model slot before the second sound 62 and `37390`.
The supplied template has its source type/handle fields patched at that point.

`37390` runs before scientist/capacity clear, animation resets and status
publication. Hut flags `0x2A` request presentation and a crater; factory flags 0
perform neither. Neither authored template has deferred bit `0x01`, radial
bit `0x04`, or Main Base-abort bit `0x10`. After the effects, the native class 0
style `4C7468` owns a 9000-ms Primary with a null callback. This is a real
retained task, not an empty task slot. Late blocks preserve their committed
graph, health, effect and terrain prefix; they cannot replay destruction.

The mode pump executes simulation `493FA0` before presentation `493F50`;
`4D07A8` reaches `53410` before actor submission. Intro2 therefore prepares
the requested viewport sprite after contact/follow in that same frame. Its
key zero draws above positive-depth world and Klaus geometry, below the later
captions/emblem. The closing descriptor has no `53410` consumer. The legacy
Playing adapter still stages before simulation; repairing that separate
one-frame delay remains open in [RENDER_PIPELINE.md](RENDER_PIPELINE.md).

## Crater, static programs, radar, and grounded actors

`36E00` visits X then Z offsets from negative to positive radius in steps of
256 raw units, wrapping both position words. It accepts only squared
distances strictly inside the outer circle. The original signed-height-byte
arithmetic uses inner radius `4r/6`, static/material radius `5r/6`, truncating
division, a quarter-depth term and final clamp `[-127, 127]`. A deeper valley
is not raised by the positive depression branch. Each affected cell clears
terrain bit `0x10`; this is a direct source write.

The inner static gate calls `27B20` synchronously. A null kind-program pointer
sets burned bit `0x08`; a supported program enters `28720` directly with live-cell
deduplication. It does not pass through damage filtering, chance RNG or the
already-burned hit gate. Unsupported programs retain the preceding changed
cells as an explicit partial result. `2EC30` selects replacement material 1
for world style 4, no replacement for style 5, and material 4 otherwise; the
upper three light bits are attenuated by squared distance.

After all terrain writes, the second complete cell pass invokes `4A890 ->
4A5C0`. This is the persistent radar terrain raster refresh, not a terrain
mesh normal calculation. Nonzero coverage nibbles consume one shared RNG
word even if their final palette index is hidden. Intro2 coverage is built
from the authored kind 8 footprints during the normal post-constructor
`4AFB0 -> 4AB20(0)` load phase. It cannot be replaced by a blanket zero-map
assumption: the hut lies within authored radius 50 coverage. Renderer terrain
and normal caches are separately invalidated from the committed mutation
report, including a later blocked suffix.

Finally `EB20` visits the live actor list. It requires
`state & 0x08009000 == 0x08008000`, strict wrapped XZ distance inside the crater,
and default ground-snap bit `0x20`. The high mask bit is **fixed**, so ordinary
moving peasants do not qualify. It resamples bilinear Y, adds the selected
signed model-header extent only for default bit `0x40`, then calls `16AC0`.
That basis uses forward differences along +X and +Z, Q12 normalized tangents,
their cross product, saturated Q31 expansion and `57960` normalization. It
does not change Euler words. State dirty bits `0x28` are set afterward.

For the canonical hut crater, centre height changes from signed byte `-8`
to `-28`, giving raw Y `-896`. This derives the observed sinking from terrain
and current actor state. No destruction tick, stored Y delta or sampled
matrix remains in [opening.rs](../../crates/v2k-game/src/opening.rs).

### Retail crater and water comparison

The read-only `V200001.run` replay in
`20260919-intro2-hut-crater-ttd.txt` records `436E00` at `FB636:1E7`, tick
342, and its return at `FB641:13FB`. The caller is `437463`; arguments are
position `[-28672,-256,-32768]`, radius 1280, depth 16 and material-enable 1.
Section-10 sea dword remains `FFFCB100` (`-216832`, raw Y `-847`) on both
sides. The 11×11-vertex neighborhood, X 139..149 / Z 123..133, contains
the same local steep depression in retail: center vertex `[144,128]` changes from
`-8` to `-28`; its four direct neighbors finish at `-26,-26,-26,-25`.
Only the center vertex is below static sea level in that neighborhood.
The captured before/after heights are an independent regression in
`terrain_crater_tests.rs`. This covers all 69 vertices visited by the hut's
radius-1280 mutation; it validates that local height operation, not the entire
visible terrain or its presentation.

Water admission also agrees with the source. `4321E0` stores the strict
signed `terrain_y < sea_y` result at output byte `+7`, and `4327C0` combines
four such bits to select the shoreline sprite. `45920` changes the submitted
wave height, not that admission bit. One submerged vertex can contribute
four adjoining shoreline quads. The vertex count alone determines neither
their projected screen area nor the water admitted outside the sample.

A separate renderer mismatch was recovered during the Jev-assisted follow-up:
`445920` returns static sea Y for dry shoreline corners and when waves are
disabled. The port had reused the physics ride-surface helper, which instead
returns dry terrain Y. The explicit `water_surface_raw` presentation policy
now preserves the original sea-plane corners while leaving ride/contact
sampling unchanged. Twenty-four original-PE cases verify both wave modes,
wet displacement and dry/equality branches. The controlled 640×480 Intro2
9-second before/after differs at 450 pixels around the small pool; the grassy
foreground and overall composition remain unchanged. This repair therefore
does not close the broad-water discrepancy.

The proposed null-camera-context explanation is disproven too. Intro2's
active `44FFA0` and closing `50C00` both pass the terrain pointer to `ED10`;
the accepted camera-entry trace independently records nonzero parameter 2.
At the crater callback the replay's tracked raw position is
`[-29184,0,-32768]`, with sprung eye `[-29184,590,30834]`. The eye is at
terrain X142 / Z120.445, about 2.555 cells before the sample's nearest Z123
edge; the intervening foreground was not sampled. These globals were read
at crater callback entry, without a matched presented frame or its view and
projection matrices. The terrain ray must remain enabled. Null-context
frontend calls are a different path; see
[the camera contract](LOADING_TRANSITIONS.md#phase-tick-fun_004503c0).

`27B20` likewise retains its existing inner-radius policy. In this replay the
foreground objects at Z123 remain outside the strict five-cell crater circle;
their survival after this first call does not establish a missed burn. It also
does not establish that the terrain remains unchanged for the rest of Caption 2.

The user's 2026-09-19 retail screenshot during Caption 2 positively shows broad
foreground turquoise water together with steep sandy back/side slopes, shoreline
rubble and insects on the rear ridge. The earlier description of retail as
having no steep bowl was incorrect. Before the burn-callback repair, the port's
9-second captures showed a small water patch and a grassy foreground ridge. Retail's
supplied image is 1905×1069 and those port captures are 640×480; the caption
alone does not match the exact camera or simulation state.

The later replay identifies the missing foreground mutation. At tick 436,
`427899` returns from a second `436E00` call, radius 2048/depth 20, centred at
raw `[-29056,-224,31872]` (cell `[142,124]` plus half a cell). This is the
[kind-27 static burn callback](ENTITY_STATIC_DAMAGE_PROGRAMS.md#burn-callback-and-secondary-crater),
which the port's opcode-10 burned-bit write had omitted. It is not another
Type66 hut terminal call. The 19×19 before/after sample changes 164 heights;
the foreground centre drops from `-7` to `-32`, below unchanged sea `-847`,
and submerged vertices rise from 23 to 44. The earlier 121-height hut oracle
was correct but stopped before this distinct callback.

The follow-up also rules out a compensating camera-height change. The port's
tracked proxy reaches retail `[-29184,-386,-32768]`; executing original `ED10`
on the first-crater terrain produces the same high requested eye Y348 as the
port. At late TTD tick605, after the second crater, `ED10` and `36C30` use the
same nonnull terrain pointer and the eye target is Y−59. The older passive
capture's sprung Y−116 has different frame durations/deadzone settling, so
its numerical ticks and eye endpoints must not be used as a replacement path.
Apply the missing terrain callback and let the shared terrain-aware camera
react naturally. Matched viewport, lighting and complete static-effect
presentation remain separate visual checks.

The repaired release render at 9–14 seconds restores the broad foreground
water and removes that grassy ridge. At the same 40-ms frame duration, the
late eye settles at raw Y−127 instead of +290; no camera adjustment is applied.
The independent secondary-crater regression matches all 361 captured heights,
retains same-call deduplication and the following radial's pre-crater origin,
and verifies the active class-93 scatter/surface suffix.

Implementation: [terrain_crater.rs](../../crates/v2k-game/src/terrain_crater.rs),
[static_damage.rs](../../crates/v2k-game/src/static_damage.rs), and
[terminal_effects.rs](../../crates/v2k-game/src/intro2_type66/terminal_effects.rs).

## Validation and remaining scope

The full V2000 repository gate covers the game unit tests,
the workspace integration suites, retail-data doctor, formatting and local
Markdown links. The meteor integration fixture retains the native Type66
owners and the same post-constructor radar stream.
Focused tests cover both exact births, singleton RNG, invalid-admission
atomicity, executing task retirement, progressive/death prefixes, integer
crater profiles and toroidal boundaries, direct static insertion, and the
canonical hut's terrain-derived Y/basis with a colocated dynamic-peasant
negative control. The presentation regression rejects clock-driven wrecks.
Captured frontend spawn 51 retains authored slots `[210, 225, 210, 225]`,
Euler `0x527D`, D4A0-grounded Y, initializer bit `0x4000` (live E870 skips
`13F70`), and live-pose submission. Model 210 `factory6` authors five
opcode-`0x22` sprite-608 roof ribbons between type-3 crenellation slots,
gated by `0x2C` on Sub-M production progress. The unstaffed zero-threshold
template publishes `u16::MAX` into that word, so the ribbons submit; they
are horizontal roof segments. `FUN_00459000` feeds the raw size short 45
into `FUN_004594C0`. Packed `FUN_00470840` decode of that 45 became 2048 and
extruded the ribbons as sky-spanning white verticals; the raw-size contract
keeps them factory-scale. Matched-retail factory hierarchy appearance,
Gouraud `factory2pillar` lighting, and wreck model 225 remain renderer
comparisons.

An uninterrupted OpenGL run at 40 ms per frame, after 100 frontend ticks,
completes the story and first-world handoff with zero unresolved reports.
The meteor's radial damage now kills the hut naturally: its terminal flash
begins at tick 308 and its retained wreck reaches raw Y `-896`, consistent
with the accepted capture's destroyed hut by tick 309. New Game, world,
caption, black-card and handoff captures were visually checked.
The [retail crater comparison](#retail-crater-and-water-comparison) verifies
both terrain mutations against independent height oracles. Restoring the
static burn callback removes the foreground ridge and restores the broad
water footprint while retaining the steep rear slopes seen in retail.
Matched viewport, lighting and effect timing remain visual acceptance work in
objective 08; the height matches and
uninterrupted run do not establish whole-scene pixel parity.

At 125 ms per frame after 137 frontend ticks, the hut again reaches that
wreck state and submits its flash at tick 312. That run reports five
`TargetSurvivorPolicy` boundaries from infected particle hits on Type15
spawn 44; it is not a clean whole-scene pass. A separate controlled lethal-hit
OpenGL exercise verifies both terminal clocks, destroyed models 365/225,
and the hut flash above geometry and below captions. It exposes one other
unresolved particle hit on Type92 spawn 52. Neither run reports a Type66
owner or lifecycle block; their unrelated hit boundaries remain visible.

Natural factory destruction remains a matched-scene acceptance requirement.
Class38 now delivers its F590 packet through the shared particle sweep; a
controlled hit on spawn 51 applies the filtered 12300 loss from health
99999. The [native Type10 dragons](INTRO2_TYPE10.md) and
[Type102 defensive turrets](INTRO2_TYPE102.md) have their own combat
owners. The [shared post-load gate](INTRO2_ACTIVATION.md) must also retain
each insect's authored release before comparing the complete attack. Retail observation
identifies dragon attacks as the destruction trigger; the Section-2 record
at 69 seconds only selects the factory camera. See
[Intro2 combat projectiles](INTRO2_COMBAT_PROJECTILES.md).

This checkpoint does not complete Intro2's other nonnative cohorts, whole
`11AD0` contact/pair scan, Capture People attachment/carry/release, or
unimplemented native product/worker allocation. These remain separate
scene requirements rather than reasons to reinstate timed structure proxies.
