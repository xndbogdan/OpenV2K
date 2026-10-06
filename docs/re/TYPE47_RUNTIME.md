# Shared Type47 gunner runtime

This document owns native Type47 construction and lifecycle across ordinary
worlds and Intro2. Captured first-world and Intro2 entry points remain explicit
replay fixtures; their spawn indices and recorded Sub-D seeds are not production
construction parameters. [Cross-level runtime](CROSS_LEVEL_GAMEPLAY_RUNTIME.md)
owns the wider migration and Objective09
owns remaining work.

## Authored profile and construction

The canonical normal-tier corpus contains eight ordinary gunners: three in
overlay13, one in14, two in15 and two in31. Intro2 has three more in overlay50.
They share model302 in all four slots, mass100, health3000, capability8 and
A/B/C/D/E/H/J components. Their authored headings differ, including nonzero
angles outside the first world. All current gunner worlds use wind mode0 and
drag3; overlay31 disables waves. Current metadata and authored records remain
separate admission inputs.

`shared_type47::publish_authored_type47` owns the common
`104B0 -> 09A80 -> D4A0 -> AC60` birth. The current manager supplies its real
allocation lease; the process supplies the actual Sub-D allocation and RNG
stream. The body retains D720's current surface/model policy, authored Euler
words, the grounded immutable `+90` anchor and constructor state. `20450` consumes
its Sub-A draw before the weighted Guard/Wander selector and task constructor.
The two behavior rows retain weights 9/1; no extra captured constructor runs
before native publication.

`NativeType47Construction` retains the issuing allocation, authored record,
model slots, anchor and actual Sub-D seed independently of mutable task and
death state. The native Sub-D frame/runtime pair stays on the entity. An empty,
unpositioned cache and zero transient `+B2` are explicit native policies for
bytes not initialized by the retail constructor; they are not claims about
every original allocator residue. See [process construction](CROSS_LEVEL_GAMEPLAY_RUNTIME.md#process-owned-construction-evidence).

The supported descriptor contract is checked before behavior publication.
Unsupported component/model/animation/configuration profiles fail explicitly;
an arbitrary spawn index is not such a failure. Ordinary loads and native
Intro2 use this owner, while compatibility snapshots and captured replay keep
their separate contracts.

## Movement, targeting and firing

The shared Type47 world owner retains `12DA0`, detailed `DCA0` and coarse
`E870` ordering around class32 Guard and class6 Wander, acquired Chase and
Aim And Fire tasks. A completed callback may replace a graph; a partial
callback must remain parked so the next frame cannot repeat task ages,
target acquisition, RNG or already committed movement.
Initial Guard/Wander Primary expiry is strict: five seconds itself retains the
task; the next callback enters the current style's `+00` C690. A replacement
Primary waits for the next visit while newly published later slots can execute
in the same pass. Guard variant0's `+04` remains the distinct C7D0 handoff.

Native movement uses the common component executor with the actual Sub-D pair.
Target prelude, Sub-D and Sub-A writes survive a later component failure in
source order. C/A/B consume the incoming physical body basis; the later body
rebuild must not be moved before them. E870's `0x2000` branch deliberately skips
the mover and environment/surface suffix, whereas DCA0 restores its movement
gate and completes the suffix after newly published task slots.

The native Sub-E emitter authenticates its real descriptor and process seed.
It resolves the current live target instead of requiring the captured
first-world player or Intro2 peasant type. Method30/class87 firing retains
per-instance emitter transaction state and the existing projectile callbacks.
Current task custody must not be reconstructed by resetting that transaction
counter when a later hit arrives. Each world supplies its own drag, wind and
wave policy; nonzero wind remains an explicit unsupported branch.

## Authored draw and firing origin

The normal-tier `newant` model302 has 78 records and a 706-word command
stream. All four Type47 model slots select it. The complete stream has no
view-selection, state-selection or child-model commands: it authors 123
triangles, 20 sprite edges and 14 palette hairlines. Guard, Wander, acquired
Chase/Aim, hit reaction and Class12 death therefore share the authored mesh;
their current physical pose, Sub-H records and command-clock registers supply
the changing presentation. Model selection is not a supported explanation
for a camera-dependent extra mesh.

`40A9F0` consumes external selector ranges in source order: twice the Sub-H
record count, then the nonzero Sub-E slot words at descriptor `+12/+14`, then
present Sub-M and live Sub-G reservations. Type47's six H records own 0..11.
Its Sub-E words are 150/0, so selector12 is emitter0, and the terminal
relation/self fallback begins at13. Model302 references selector12 in the
first triangle (sprite668, normal0, slots58/59/152). The actual sprite is
2x2 with zero RGBA throughout; the reached command still resolves its
corners and invokes the emitter callback. Its transparency does not explain
visible black lines. The antenna and legs have their own authored sprite680
ribbons and palette hairlines, and six sprite986 shadows depend on H
endpoints through type13 aliases. Normal admission precedes live selector
resolution; resolving every selector before this gate changes source cache
and RNG order.

`424F20 -> 424FA0` resolves the current emitter source slot in the current
VIEW frame, inversely transforms it with the native viewport, narrows the
three coordinates to signed words, and stamps every queued command for that
emitter. This stamp clears the command's center-origin fallback. Source
`11400` draws the actor before draining its FIFO; source time/velocity
correction and particle creation then use the stamped muzzle. The shared
Type47 runtime keeps each queued command's optional draw origin, validates
the entire FIFO before stamping, and preserves append/drain epochs: commands
appended after a draw remain unstamped until a later reached draw. This
restores the authored origin without a model-specific positional offset.

The external wrapper `40D350` rebases every returned endpoint WORD around
the native viewport origin before `6ECF0` projects it: signed16(endpoint WORD
minus viewport-origin WORD), then add the full viewport-origin dword.
This applies to X/Y/Z and selects the nearest viewport image. A direct
signed-word subtraction can turn a foot crossing32767 into a roughly65536-raw
unit leg or shadow span. The shadow callback consumes the resulting cached
VIEW point, performs its own per-term Q31 inverse and forward transforms, and
preserves its native clip byte. The semantic world point remains separate
from this quantized cache. Original-machine suffix controls independently
pin the wrapped delta, nonidentity quantization and `4349C0` output; they do
not execute or replace the preceding live D360 writer.

The authored antenna tail also uses the shared model register interpreter.
Its packed shift operand11 decodes12, so the wrapping50-Hz clock visits16
phases. `466870`/`466900` duplicate the literal quarter-table word into Q31,
negate the whole dword for the negative half-cycle, multiply the unsigned
amplitude and extract product bits31..46. The shared fixed-math helper now
retains this exact DD/ED result: the positive peak plus the authored0x8000
is65535, where the former floating sine wrapped tozero. This repairs a
separate source-backed animation discrepancy; it is not proof that antenna
phase caused the reported line artifact.

The live emitter provider authenticates the root model and a plain source
slot against its native actor frame and viewport. This admits model302's
plain slot150 `[0,14,-10,168]`. Child-node emitters and generated source slots
remain explicit boundaries until the caller supplies current node-frame,
register and cache custody. The pure callback kernel accepts authenticated
current-frame inputs rather than treating a root frame as a child frame.
The terminal fallback retains actual recent-relation handle resolution,
signed-word wrapping and three shared RNG draws, including relation mode's
zero displacement multiplier. An unknown relation handle blocks terminal
selector zero before RNG; it is not replaced with the AI target or a captured
position.

The original-PE arithmetic oracle executes `424FA0` and its called plain
vertex instructions with actual model302 slot data. Its three controlled
frame cases agree with the Rust callback's dwords and signed words, including
both torus seams. These are independent machine-code arithmetic controls;
the paired camera fixture and actor poses are not a matched live Type47
callback or pixel capture. Class87's `442240` presents the live particle
first, then six backward samples from one copied stack record. Its exact
address-dependent size jitter requires that original stack address. The
earlier sampled particle/AI captures do not retain it; the indexed native
query below now observes it for specific original invocations.

## Model and shot presentation

Native `53760 ->11720 ->11400` presents each model before draining that
actor's FIFO. The later `53A60` particle traversal therefore sees newborn
class87 at birth position in the same frame, while its first40120 movement
waits for the following simulation pass. See the corrected actor/particle/
terrain ordering in [rendering](RENDER_PIPELINE.md#terrain-shade-input).

Class87 uses the shared442240 seven-sample backward draw callback, rather
than one ordinary sprite. The source scale512 is distinct from the frame
triple's size word256; the three frame light words are zero. The shared
callback, six applicable descriptor classes, integer offsets and shrinking
scales are owned in [copied draw records](RENDER_PIPELINE.md#copied-projectile-draw-records).
The read-only indexed stack query
observes accepted `V200001` at class87 ticks902,946,2340 and sibling class68
tick1402. All four enter442240 at ESP001AFE30 and copy the record to001AFE10,
so copied phase `(pointer >>5)&15` is zero and first copied source scale256
is unchanged. The live pool phases15,9,1,5 instead produce effective scales
752,656,528,592 from source512 and divisor32. The original return words
authenticate `493F6B ->53A60 ->43DCB0 ->43DCD0 ->442240`; balanced source
argument cleanup proves a fixed ESP across this pool scan and its six
descriptor classes. The query validates loaded index, callback, descriptor,
thread/tick, exact copied address and scale order;14 guard tests reject
malformed or borrowed observations. See the compact query proof, transcript
and original-PE caller disassembly retained with the newant evidence.

These original invocations close their local address authority. They do not
establish an absolute startup/thread/outer-context stack phase for every
retail execution. The runtime therefore keeps the explicit copied-size
compatibility boundary and unadjusted source scale, without installing
captured phase zero as a default. Class87 can increase width by up to
46.875% from address jitter; the source position and signed-word seam
handling are independent. Matched retail widths beyond these invocations
remain unverified. Do not fit a phase to the port screenshots.

## Hits, death and abort

`apply_shared_type47_particle_hit` is the native particle entry. It authenticates
the completed manager/task boundary before primary10EB0 or infected11250
prefixes, resolves the current style's C690/null hook, applies11030 reaction and
checked damage, then retains the replacement owner and appropriate feedback.
Primary and infected sound, timestamp and suffix behavior remain distinct.
See [damage and death](ENTITY_DAMAGE_AND_DEATH.md) for the packet arithmetic.
A present but invalid native receipt blocks; it cannot fall through to a legacy
spawn adapter.

Lethal hits, live radial damage and Main Base abort use the same native
construction custody through standard10C10/Class12 publication. Shared Class12
ticks retain allocation generation and current task leases through terminal
deferred removal. An already-dying hit must not restart the death clock. Native
Type47 is excluded from the old first-world scheduler adopter, so the actor
cannot receive two independent task visits.

Static contact runs for ordinary and Intro2 newants through the shared
`type47_static_contact` owner: deepest-static scan, physical response, and
ordered static/actor damage on nonzero impact under read-only scheduler
custody, with per-world standard death. The 02CA0 task-hook retarget stays
open pending the Type47 style-hook matrix. Live-actor pair contact against
the bound hive runs through the shared native pair lane
(`native_actor_capture/pair.rs`): authored spawn 11 rests ~158 raw units
outside the hive narrow solid with a +-1024 Guard retarget, so retail's
mass-weighted separation is the only thing keeping wanderers out of the
fixed hive body. The lane admits native-Type47 versus bound-hive pairs in
either intrusive direction with the exact 11AD0 ordering (null Hive/None
behavior hooks, shared 02DA0/null component walk, oriented narrow phase,
12760 response, ordered 14D30 damage); other non-Type17 counterparts remain
explicit boundaries. See the
[spider behavior audit](SPIDER_BEHAVIOR.md#native-active-actor-pairs) for the
remaining admission requirements. Corpus regression
`tests/type47_hive_contact.rs` pins admission, birth miss, overlap
separation with an untouched hive, and the surviving hostile.

## One original model302 draw fixture

The read-only newant draw query
observes accepted indexed `V200001`, starting at `17BC1E:1716`, through the
next Type47/model302 draw at tick952. It retains the real actor04F50001,
raw position `[-17501,-551,31307]`, velocity, Euler words, physical Q31 body,
VIEW viewport, draw packet, native context, active terrain bytes, wave policy,
descriptor and Sub-H records before/after the same draw. It captures all156
completed near records with their VIEW/screen/clip bytes and88 reached authored
commands before464F78 releases the cache. The queue context uses sorted mode1.
These recorded inputs belong to a paired fixture, not a constructor default.

The actual dispatch table has separate A/B/C vertex families. This draw reaches
A13 at435090 for six foot-shadow sources112/114/116/118/120/122, C14 at46ECF0
for their primary H selectors0/2/4/1/3/5, A14 at46EE70 for secondary selectors
6..11, and selector12 at slot152 before command traversal. Watching only C13
at4349C0 would incorrectly classify the projected shadows as absent. The
pending method30 shot queue is retained; its source selector12 endpoint can be
compared with the completed cache without inventing a firing pose.

Constructor source09A80 authenticates direct Sub-D allocation at component+8
through4203D0 and direct Sub-A at component+0C through420450. The query's raw
`D_OUTER`/`E_OUTER` block labels preserve these locations; the latter is not
Sub-E. Their60/12 owned bytes are distinguished from four adjacent dump bytes.
The genuine Sub-E allocation at component+2C through424E30 is not separately
dumped. Source40D320 selector0 returns tick WORD4FED60 directly; uninitialized
register words are not callback-owned animation values.

Sixteen query guard tests reject mixed owner/thread/tick/context, borrowed
allocations, unreadable/noncontiguous memory, a normal pool mistaken for the
command stream, invalid vertex family/dispatch and command injection. The
completed original transcript is parsed by that same guarded source. Original
trace/index and executable remain unchanged. The retained full terrain is
captured process data, not a generated source asset. Paired numeric draw output,
sorted raster visibility and final retail pixels are separate checks; this
fixture alone does not close those boundaries.

The paired Sub-H comparison exposed a draw-custody error even though the
submitted native frame matched: the previous callback resolved A/B/C through
the physical body basis directly. `41E310` instead reads the current resolved
VIEW slot and inverse-projects it through the retained viewport, preserving
each independently shifted Q31 product. Plain odd-X slots mirror the shifted
X product, rather than negating the local coordinate before multiplication.
The native draw adapter now uses the shared `NativeModelFrame` plain-slot
resolver followed by `NativeWorldViewport::view_point_to_world`; physical
body axes remain the input of DF20 and D360's secondary basis calculation.

The tick952 regression retains all six H before/after records and the sixteen
terrain height bytes reached by this draw, with probes outside those captured
base cells rejected. It checks all live cache fields, the authentic primary
selector order and twenty-two terrain calls. The separate twelve-vertex test
checks captured VIEW words and the inverse-projected A/C points. This explains
the earlier one- or two-word foot differences and record3's fifteen-word knee
Y difference: the incorrect anchor changed its primary chord length and Q12
normalization. No secondary formula adjustment is fitted to the capture.

This is a shared correction for authenticated native root draws. Free-camera
and intrinsic presentation retain their existing body-space compatibility
path. A child frame alone does not authenticate child-local H vertex refs:
the presentation still retains the owning actor's root record table. The
model302 fixture and current root spider/stag controls do not establish a
child-local Sub-H record-table owner. Sorted queue admission and GL pixel
occlusion remain separate from these paired integer callback results.


The [authenticated childless painter adapter](RENDER_PIPELINE.md#authenticated-childless-painter-adapter)
now provides an explicitly opted-in current-frame consumer for model302's
prepared face and edge keys. Its warm tick952 control uses the real authored
model/H/E descriptors, retained VIEW frame, H endpoints and 69 recorded keys;
the submission buffer drains mixed primitives without repeating callbacks.
The same admission reaches childless spider256 and rejects every possible
stag267 child path before H/E. This adapter preserves Free compatibility and
the ordinary Geometry path. It does not establish the shared terrain/model
queue, native fog/polygon packets or final retail raster visibility.

## Moving camera seam regression

A continuous near-hive control uses genuine ordinary-world13 Type47 spawn11,
model302, its native task graph, and the gameplay RetailChase camera. Only the
diagnostic player moves, along three paths relative to the authored ant origin;
the insect trajectory and targeting are not scripted. Three controlled process
RNG prefixes (0/37/137 draws before world construction) produce nine runs of
360 native20ms ticks. The actual-main Full classifier admits H/E/model work,
and each frame performs one destructive full-world draw. The runs contain
2,659 targeting frames and28 focused queued shots; player/ant signed-Z seam
crossings total21/18. Prefixes are not claims about observed NewGame seeds.

This control reproduced the reported long black legs: path0/prefix37/tick188
shows segments rising into the sky beside the hive. The live H cache traces
the failure to the shared DF20 constraint search. Its former midpoint used
the full32-bit difference between signed endpoint words. Native41DF20 instead
narrows the difference to signed16 before division by2, truncates toward zero,
then wraps the sum. Bisecting across32767 formerly took the long arc, sending
an actual constraint target toZ=-18434 rather than-32768. The shared midpoint
now follows that source rule across X/Y/Z, with positive/negative seam and odd
delta tests plus three required real-world13 terrain/model fixtures. The
independent [DF20 machine controls](ACTOR_TASK_PROGRAMS.md) execute the original
body with full authored terrain, rather than replacing the midpoint with port
arithmetic.

The corrected nine runs retain identical recorded ant/player pose, physical
basis, task/health/context, queued-shot counts and float/native camera inputs
on all3,240 ticks. Every run completes without runtime reports. The valid
retained H endpoint maximum nearest-viewport span falls13,501 to397 raw units;
frames above2,000 fall60 tozero. The tick188 full-frame pair, moving seam/firing
sheet and three route sheets were viewed: the reproduced tall segments are
gone. All834 corrected captures retain their original640x480 RGB pixels;
metadata records every draw. A read-only cache-snapshot repeat yields the same
actor/camera/firing states without extra selector resolution or callback replay.

The saved sequences are port before/after controls, not synchronized retail
framebuffers. Valid cached endpoints include retained values; their presence
does not prove all six foot shadows are eligible in every draw. Physical
projectiles advance against terrain, but diagnostic target/actor damage is
omitted to preserve the moving target. Hit/death and full gameplay acceptance
remain the separate controls below. These boundaries do not weaken the
reproduced geometry failure, source narrowing proof, or state-aligned fix.

## Native command endpoint validation

The shared [endpoint owner](MODEL_DRAW_CUSTODY.md#native-endpoint-custody-and-controls)
now retains reached command-time VIEW/clip packets through materialization,
model-tree submission and `OwnedBody` buffering. Native packets currently
come from authenticated H/M presentation contexts; ordinary Fixed/Raw
producers remain explicit compatibility. Free and resized logical viewports
also keep their named compatibility paths, rather than deriving native lens
words from the GL matrix.

The captured tick952 model302 draw matches all 34 reached edges and 68 endpoint
occurrences, including the antenna's earlier-register cosine endpoint. Twelve
dry/wet line/ribbon GL controls match completed-pixel references; the
fractional 212-pixel compatibility mismatch becomes zero. A buffered native
line retains its producer lens after the consuming scene changes to
MissingLens. This validates owned integer screen projection and callback
custody; it does not establish sorted world visibility or native raster pixels.

The fresh two-route moving comparison keeps the exact infection-clock prefix
and all 720 focused actor/player/camera/task/count records, including3 queued
shots and 566 targeting frames. Integer projection changes 121 of 182 captured
RGB images; all captures were reviewed in sheets, supplemented by full-frame
checks. No tall seam-spanning legs were seen in that captured coverage.
Intro2 and Level2 controls also complete without runtime/native edge reports;
the latter's three delivery images and exact marker positions remain unchanged.

`SourceClipped` drops an authenticated A22/B22 first-endpoint rejection before
final remapping can visit the second callback. A claimed-native `MissingNative`
command also stops remapping, while its separate `ModelNativeEdgeFailure`
receipt retains the model, command and source slots for the live model-tree
consumer before any deferred body buffer. That failure does not authenticate
the missing input or invent an endpoint. Explicit compatibility scenes retain
their legacy geometry and reached callbacks. The fully owned captured draw did
not exercise this failure branch. Child-local H tables, projected-midpoint
dependencies, prepared world queue integration and synchronized retail
framebuffers remain open.

## Validation and limits

The constructor census covers all 37 ordinary overlays and a later-world to
Intro2 load with retained process history. Focused regressions cover authored
rotation, selector RNG order, missing components, committed mover prefixes,
stale allocation/task custody, damage publication and deferred Class12 removal.
The later-world integration control uses real gunners in14/15/31, runs the
shared scheduler past the initial five-second task timeout, then delivers
canonical primary packets through the native hit entry.

The draw regression census covers all eight authored ordinary newants in
worlds13/14/15/31. The archived before control has1,152 normal-tier GL frames;
restored Native and Free viewport controls each have1,404 frames in117 complete
twelve-view pose groups. Recorded descriptors/tasks distinguish Guard,
Named Wander where reached, chasing/aiming, a surviving1,200-health hit,
Class12 death and actual deferred removal. Every image step has pose/state
metadata; matching raw-pose comparison rows show the restored authored foot
shadows. Sixteen additional GL frames cover actual spider/stag shadow models.

Twelve fresh single-view firing runs add36 birth frames. They retain the full
authored world and the real muzzle/queue/particle path; other ticks use view0.
The first reached shot submits seven age-zero sprites in every view. This
avoids interpreting later alternate views after a destructive particle pass as
independent shots. Sprite admission does not establish visibility after world
occlusion, exact copied-record width, or a synchronized retail framebuffer.
The source-owned native draw and remaining address/camera boundaries above
remain distinct from passing numeric and port-rendered controls.
These controls do not establish complete active-pair contact or full
later-world gameplay.
Other unmigrated families can still block a whole Main Base abort. Campaign
restoration and nonzero-wind behavior remain separate evidence boundaries.
Filtered-zero player feedback through selector23/soundDB retains the existing
shared damage boundary. Canonical player primary damage is 1,800 for this
profile; infected F780 filters to zero with its own zero source words.
