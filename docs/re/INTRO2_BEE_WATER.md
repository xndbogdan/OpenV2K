# Intro2 hive-shot ground burial and later water-entry response

The user's ground-burial report concerns the earlier hive camera shot, not
the later “All the time spreading their lethal virus” water sequence. Water
dips and resurfacing conform to retail and must remain permitted. Native
Type15/spawn44 is the wasp at the hive; Type87/spawn46 owns the later dive.

## Hive-shot reproduction and shared ground response

The production Intro2 baseline at commit31049dbf, with333 native frontend
updates and16,667-us frames, reproduces243 solid intersections for
Type15/spawn44, with maximum penetration719 raw units. The20-second OpenGL
frame shows the hive caption “Travelling between the worlds through their
hives”; the wasp is buried in that shot. The baseline is built from that
commit's source and its own libraries and includes Type57 scheduler adoption.

An earlier isolated harness without that adoption reproduces204 intersections
between scene ticks950 and1185. Its tick1060 position is
`[-14449,-1823,31443]`, velocity `[501,-165,-532]`, state`07478825`, and
terrain penetration880 raw units, normalQ12 `[1438,3835,0]`. That actual pose
remains a focused regression fixture; it is not the final full-owner baseline.
Other tested frontend entry/cadence pairs100/40ms,137/40ms,250/mixed and
1000/125ms do not reproduce the same flight. RNG/cadence variation explains
why a single run missed it.

The existing bilinear height and retail415160 averaged cell-plane collider
identify the intersection. The omitted mechanism is11AD0's late contact
dispatch: Type15 never entered the already-owned Type87 solid/water path.
Movement, terrain height, camera and sea level require no fitted correction.
The same [flying surface owner](FLYING_SURFACE_CONTACT.md) now admits the
source-supported living13/10/57 and15/87 families, retaining their individual
custody and death callbacks. Focused tests retain the actual hive pose and
exercise both15/87 on flat and sloped terrain over16.7/40/125ms displacement
cases. Wet controls preserve crossing below sea and later resurfacing.

The final natural333/16,667-us before/after run reaches the complete story.
Type15 geometry probes fall from243 intersections/max719 raw to two
intersections/max1 raw after the source's integer plane separation. No fitted
clearance or sea clamp is added. The20-second OpenGL images were visually
inspected. The final333 and137 fixed-frame runs and777 varied-frame control all
complete the story with zero runtime reports. The137 same-walk release now
retains the real retired Type9 task graph under its allocation generation and
pending-removal receipt; it does not resume ordinary child task visits. See
[the delivered-child owner](TYPE17_CAPTURE.md#released-type9-contact-after-delivery).
The active Type26 class68 hit exposed by333 now uses its owned shared11180
route, including real C690 and Class12 rather than borrowing Type13's callbacks.
The later accepted water observations below remain valid; they do not describe
the user's ground-burial moment. The controlled Type87 descent/resurface cases
continue below sea without clamping. These rendered old-port regressions do
not establish a newly matched retail cinematic recording.

## Later legitimate water sequence

The bee in the “All the time spreading their lethal virus” shot is
**Type87, authored spawn46, model270 `deathwas`**. Type26/spawn10 is the next
camera subject, not the actor performing the observed dive. This corrects the
initial Type26 hypothesis; it does not change either actor's movement policy.

## Authored shot and accepted observation

Normal-tier `1X50XX.OVL` Section2 selects spawn46 (authored handle38960) in the
33,000–33,500ms command window, then spawn10/Type26 (38924) at 36,000–36,500ms.
Selection persists between commands, and the camera follows through its retained
spring state. The caption runs 35,000–41,000ms. The 39,000ms camera command selects
spawn1/Type57 (38915); model122 is not entity Type122.

The accepted passive capture `20260722-022303-intro2-actor-ai.jsonl` joins the
Type87 birth at `[-18176,1024,4864]` to handle`04690001`, Search style`004C7A50`,
and active model270. Its global lifecycle observations retain the following
crossing. Coordinates and VY are signed retail8.8 words; times are the sampled
50Hz world tick divided by 50, not exact callback timestamps.

| Capture line | Tick / scene seconds | Position raw X,Y,Z | VY raw | Retained surface state |
| --- | --- | --- | --- | --- |
| 50742 | 1650 / 33.00 | −954,740,3664 | 685 | fully above |
| 52008 | 1774 / 35.48 | 623,−830,1920 | −875 | intersecting; above bit cleared |
| 52282 | 1806 / 36.12 | 1025,−1107,1479 | −16 | fully below |
| 52580 | 1834 / 36.68 | 1451,−852,1209 | 974 | intersecting |
| 52782 | 1848 / 36.96 | 1686,−451,1097 | 1760 | fully above |

Lines 52004–52007 independently record **four class 13 particle births** at
tick 1774, each with source handle`04690001`, source type87, and position
`[623,−1024,1920]`. These are the ordinary water-entry scatter selected by
`004141D0 → 00440DC0`, response selector6. The sampled VY−875 selects the gentle
branch; no Type60 hard-entry constructor is needed for this observed event.
Resurfacing is visible in the retained actor state, but these records do not
prove a second emission at exit.

The same capture's Type26/spawn10 and25 stay fully above water during this
caption interval. The separate accepted
`20260717-032945-menu-intro2-level1.jsonl` has a different Type87 route: its own
handle`04CE0001` is at`[-1617,175,18453]` at tick 1651 and
`[275,−549,18887]` at tick 1801, with no sampled crossing. A dive in every run is
therefore not an acceptance invariant. Native target, RNG and cadence must be
matched before comparing trajectories. Both files are indexed by the
accepted-capture ledger.
Passive snapshots establish allocation, pose, state and emitted particles;
they do not establish individual RNG draws or unsampled callback ordering.

## Source contact and response order

Retail `00411A80` first calls`00417130`, then visits the live intrusive list
through`00411AD0`. This is the late contact phase, after the actor mover and
particle control pass. For each admitted actor:

1. Require state`0x8000`, clear`0x1000`, `+70 == 0`, and a nonzero active-model
   collision radius at header`+0A`. Fixed/remote state`0x88000000` excludes the
   shared terrain/water branch. Terrain/water state`0x10000` enables it.
2. Run solid contact`00412870` and its response first. Resolve the surviving
   actor again before classifying its current water pose.
3. `004129B0` commits the two surface bits before returning an entry contact.
   Only then play the nonzero type-record`+88` cue and invoke type vtable`+10`,
   or direct fallback`004141D0` if that callback is null.
4. The common vtable at`004C8A30` has`+10 = 0040D860`. This callback invokes
   **style`+14`**, resolves the actor again, then calls`004141D0` if it survives.
   The common solid callback`0040D7F0` instead uses style`+10`.

Type87 Search styles`004C7A50/004C7A98` have null style`+14`, so they reach the
common water fallback. Type87 type-record`+88` is zero; its attached-loop
selector`+B4 = 11` is a separate buzz owner. Type26's`+88 = 91` must not be
copied onto Type87. The Type26 Follow/Trash/Defecate styles also have null water
hooks, but that does not make Type26 the observed bee.

`004129B0` first compares static sea against the signed height byte of the
current integer terrain cell, multiplied by32. A dry cell preserves both old
surface bits. A wet cell uses the current three-sine wave surface when waves
are enabled, clamped to that terrain height; disabled waves use static sea.
For signed centerY, unsigned active collision radius R and surface W:

- `centerY + R < W`: fully below, bit`0x200000`.
- `W < centerY − R`: fully above, bit`0x400000`.
- Otherwise: intersecting, neither bit; equality is intersection.

An entry is **old above bit set and new above bit clear**. Exit updates the
bits but does not produce another contact response. The response material uses
the independently rounded nearest terrain cell `(raw + 0x80) >> 8`, wrapped
in X/Z, not the integer cell used by the wet gate.

`004141D0` reads the Section13 water-response selector for that material.
Selector 7 returns before effects or damping. Otherwise it resamples the water
height after the callback. VY≥−1000 submits`00440DC0` at that surface, scale
`0x1000`, preserving the actor handle and state sign. The selector→class table
is`[7,8,9,10,7,11,13]`; allocation count follows the shared particle governor.
This branch has no direct water-entry sound or velocity damping.

VY<−1000 constructs Type60 operation`0x3C`, using model132, or model130 only
when VY<−1750. Constructor/error handling precedes positional sound17; signed
`VY >> 1` follows. Its constructor RNG and new actor ownership are separate
from the gentle scatter. The user's sound recollection remains an audio
attribution check: neither Type26 cue91 nor hard-entry sound17 is justified
for the four class 13 births observed here. Nearby projectile effects and the
bee's attached loop have distinct sound owners.

## Native contact owner and remaining acceptance

Native Type87 birth retains `104B0`'s pre-component surface comparison. At its
authored `[-18176,1024,4864]` position the integer-cell floor is96, above sea−847,
so the constructor's wave branch is unreachable and the initial above bit is
exactly`0x400000`. This compares the authored centerY without collision radius;
it is distinct from the later `129B0` sphere test. Native flyer/dragon births
bind this clock-independent dry-cell result before component publication.
Wet constructor cells retain their unresolved clock requirement, and later
reselection does not repeat or replace the birth comparison.

`intro2_flyer_contacts` runs Type87's late terrain/water phase against its
native Sub-D/E/G allocation and completed scheduler owner. It retains the
entry model's collision radius across the solid response, then reads the
surviving actor's current pose and crossing bits. Surface bits commit before
cue/style/fallback dispatch. A failed committed prefix parks that actual
owner, preventing a later contact or task visit from replaying its effects.

The adapter uses `whole_body_surface` and the provenance-bearing
`WholeBodyContactScatter` request. Its `440DC0` allocation advances the shared
direction cursor and initializes each particle's terrain/water cache at birth,
retaining the bee's source type and handle. Reached hard impacts use the real
Type60 constructor and immediate task adoption; sound17 and signed VY damping
remain after the constructor even when allocation is rejected. Solid contact
retains its physical response, effect allocation and checked damage before
water classification. Type15/87's authored lethal alternate is class2 Die
Quietly: cue+90, release attached sound11, clear all three tasks and queue
deferred removal. Its allocation survives the same source walk; no Class12
corpse, launch velocity or Sub-G constructor draw may be substituted.

This contact phase supplies the missing entry-effect mechanism. It does not
establish why a particular port run takes a different flight path. Living G
static-building contact and the bounded actor-pair response against owned
Type66 structures now use their authenticated family direction/death custody
in [Insect static contact](INSECT_STATIC_CONTACT.md). Other active-pair families
retain their explicit boundaries.

The shared [terrain sphere callback](TERRAIN_COLLISION.md) now follows retail
`415160`'s averaged Q12 cell plane and perimeter edges. The former independent
triangle tests missed model270 at `[623,-3559,1920]` on the original Level50
terrain. Running the original callback instructions with the same gate/detail
spheres gives normal `[247,3903,-1216]`, penetration572. The native contact
regression now separates that buried bee through the existing solid response.
This is an isolated reproduced contact defect, not proof that the observed
cinematic flight reaches that pose; natural route and water/audio acceptance
remain as described below.

Focused tests cover gentle entry particles, hard-entry construction, sound
and damping order, and retained owner state. Full-story OpenGL walkthroughs
with fixed and varied frame durations produce no bee-contact boundary, but
both sampled routes remain above water during this camera interval. They
therefore do not close the matched retail dive, splash or depth comparison.

Normal-tier Intro2's canonical static sea is−847 raw. Model270 has draw extent
270 and collision radius198; these fields are not interchangeable. The
accepted entry particles sit on the animated surface at−1024, demonstrating
that the instantaneous water plane can be below static sea. Compare water
depth using the same raw X/Z, draw tick, terrain, camera pose and presentation
tier before changing a water offset. A screenshot alone does not distinguish
wave phase, route, camera spring or terrain depth.

The reusable surface formulas are documented with the
[player whole-body owner](PLAYER_CRAFT.md); camera and phase ownership belongs
to [loading transitions](LOADING_TRANSITIONS.md), and water submission to the
[render pipeline](RENDER_PIPELINE.md). Remaining acceptance is a matched
Type87 interval proving route/Y, classifier transitions, four-way entry
particles and their audio context, while retaining legitimate run variation.
