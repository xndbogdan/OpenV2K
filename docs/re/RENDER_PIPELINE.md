# V2000 Render Pipeline — Rasterization & Shading (ground truth)

How the original engine actually draws 3D model triangles/quads, decoded from
`V2000.EXE` (`Graph2D.c` + `DDCalls.c`, software and alternate consumers).
Source contracts and captured renderer selection must be distinguished when
matching the Rust port. See
[FORMAT_DOCUMENTATION.md §8](FORMAT_DOCUMENTATION.md) for the model command
stream that *feeds* this pipeline.

## TL;DR

V2000 ships both a 16-bit software DirectDraw consumer and an
alternate/Direct3D `Graph2D` table. The retained tick952 newant draw executes
the alternate table and source BeginScene/EndScene before a successful Flip;
its CPU map remains absent. This establishes shipped traversal, not physical
GPU acceleration or filtering state. See [same-draw backend custody](MODEL_DRAW_CUSTODY.md#same-draw-backend-discrimination).
The following properties describe the audited **software filler** path:

| Property | Audited software contract | Port today | Match? |
|----------|----------------|------------|--------|
| Texture mapping | **Affine** (NOT perspective-correct) | Affine projective varyings | ✓ |
| Texture filtering | **Point sampling** (1 texel/pixel) | NEAREST (sprites) | ✓ |
| **"Bilinear Filtering" menu toggle** | No setting read in the audited software filler path; alternate filtering state unretained | n/a | n/a |
| Shading | Separate unlit, uniformly lit and Gouraud face families | Authored normals and solid near/far composition; RGB565 Gouraud packing after affine interpolation | Implemented; matched visual acceptance open |
| Hidden surface | **Painter's algorithm** — depth-sorted queue, NO z-buffer | Geometric GL depth with explicit painter groups in selected paths | Partial; opaque geometry is not universally equivalent |
| Backface cull | Authored anchor/normal plane, strict negative camera dot | Exact world-space equivalent | ✓ |
| Color depth | 16 bpp RGB565 | Software renderer: RGB565 through the retail fill slots. OpenGL: RGBA8 composition | Software exact on receipts; OpenGL approximation |

## Colour depth and resolution

The retained [DirectDraw surface](#full-frame-directdraw-trace-acceptance-2026-07-27)
proves 1024×768×16 with RGB565 masks, rather than an 8-bit final framebuffer.
Indexed textures, their authored shade palettes, point sampling and quantized
lighting also contribute to the stepped look; reducing final colour depth alone
does not recreate the integer software rasterizer. The software renderer
([SOFTWARE_RASTER.md](SOFTWARE_RASTER.md)) is that rasterizer; it replaced the
OpenGL Classic Framebuffer option, which only quantized a finished RGBA8 frame
to RGB565 (removed 2026-10-07).

Display Resolution offers the original 640×480, 800×600 and 1024×768 presets.
The selected preset supplies the authored High layout and window size;
explicit Low retains its 320×240 internal canvas. Fullscreen uses SDL Desktop
mode and the actual desktop drawable, independently of the selected preset.
Older unsupported presets normalize to the largest original preset fitting
both dimensions, with 640×480 as the minimum window size.
High Native uses a port-owned readability scale above the original outputs.
With `H = min(drawable_height, drawable_width * 3/4)`, this scale stays at 1
through H=768, ramps linearly to 1.8 at H=1080, then becomes H/600. Ultrawide
width adds no zoom; narrow outputs bound growth.
`NativeCanvas` maps complete menu/story compositions through the selected
centred authored canvas, taking the smaller of that readability scale and the
complete-canvas fit. Text, sprite art, bars, clips and frontend prop anchors and
pixel focal lengths share the mapping, including after fullscreen transitions.
Its scale reaches 1.8 at 1080p and 3.6 at 4K when the selected canvas fits;
the 1024×768 tier is bounded to 1.40625 and 2.8125 respectively.

Gameplay uses the separate port-owned `NativeHud { anchor }` policy. The
orb/weapon/cargo group follows the drawable's bottom-left edge, the radar its
bottom-right edge, and the independent challenge trophy/clock its top-left
edge. Each group's sprites, clips and model projection share the same mapping;
authored edge insets and model pixel focal lengths grow with its uniform scale.
All three original presets retain their exact selected-tier pixels/positions.
Modern trophy models use normal 640-tier high-art pixel metrics for that lens,
keeping the larger 1024-tier projection from cropping the retained top anchor.
[HUD_RENDERING.md](HUD_RENDERING.md#time-trophy-and-countdown) owns this extension.
The HUD uses the same readability scale, down-fitting if necessary through
H=768. It reaches the 800×600 globe's 16% height density continuously: about
173 pixels at 1080p, 230 at 1440p and 346 at 4K. Corner groups do not fit their
unused full canvas, whose offset may legitimately be negative.
The M modal, entity icons, clipping and status panel use a separate centred
`FitAuthoredCanvas` mapping. High Native `WorldOverlay` messages use actual
viewport percentage anchors with the HUD glyph scale; wrapping is bounded by
the available drawable width, baseline adjustment and clipping. Matching
original displays keep their authored submission. Overlay51/results prompts
use the distinct `AuthoredCanvas` context so text and backdrop retain the same
bounded complete-canvas mapping.
The Frontier/Grolier logos and copyright row grow independently with the
readability scale (1.8 at 1080p, 3.6 at 4K), preserving their actual left,
centre and right bottom-edge anchors rather than a fitted canvas origin.
The selected resource tier remains independent of the desktop size. This is a
modern port-owned readability extension, not retail acceptance beyond the
original tiers. Matching 640×480, 800×600 and 1024×768 displays retain their
original geometry. Low and 4:3/Stretched retain their existing mapping;
4:3/Stretched enlarge branding with the complete logical viewport, and Classic
colour conversion is unchanged. High layout changes publish staged
font/projection/HUD snapshots live; artwork detail retains its launch-time boundary.
[MENU_SYSTEM.md](MENU_SYSTEM.md) owns the three-tier metric/spacing matrix,
shared authored-canvas submission and live layout-refresh contract.

`gl_backend/classic_color.rs` owns a **final RGB565 approximation**: the GL
renderer still composites into RGBA8, then preserves R5/G6/B5 high bits in each
logical pixel before the single bilinear enlargement. Presented-frame readback
applies the same idempotent conversion; current-scene readback preserves RGBA8.
Pause display changes redraw isolated world owners with frozen clocks, retain
their admitted particles, shield decisions and HUD/radar/text commands, and capture current-scene colour
before the menu overlay and swap. This restores full precision when Classic is
disabled and recomputes projection when switching to Native scaling without
advancing live simulation, render caches, effect RNG or audio. Authored RGB555 colours remain unchanged;
RGB565 green retains its sixth bit. A shader failure disables the feature with
the existing diagnostic instead of silently claiming a quantized display.
No global extra dither or palette reduction is invented. Renderer ownership
must replace this adapter when packed per-draw saturation/half-additive blends,
fog LUTs and their ordered dither are fully implemented. This approximation
does not count as matched retail raster acceptance. The explicit hidden-GL
test checks the actual final output across a quantization threshold, logical
readback at 1024×768, resize and Native-mode exclusion.

## Port live-world camera adapter

This is **port presentation policy**, not a retail rasterizer step. Retail never
mirrors overlay, terrain, or entity bytes. `FUN_0040ED10` sits the eye on the
craft's −Z side looking toward +Z. `FUN_0040F3A0` builds **right = up ×
forward**, so authored +X is screen-right. The software rasterizer consumes
that basis directly.

The port draws live worlds through OpenGL, which is the other convention
(right-handed, look down −Z, right = forward × up). The same look-at then puts
world +X on **screen-left**. `Camera.left_handed` only negates **view X** (and
`glFrontFace` to CW) so the picture matches retail. Free-fly stays on so a
chase detach does not remirror the world. Menu cameras stay right-handed; the
frontend applies `MENU_MODEL_LOCAL_X_BASIS` at the hierarchy root instead.

Do not "undo the world flip." There is no flipped map. Do not flip parsed UVs
or invent a 180° heading/Sub-I patch to repair one sprite. Screen-space
handedness belongs at the submitting scene basis
([FORMAT_DOCUMENTATION.md](FORMAT_DOCUMENTATION.md) implicit-UV note).

A native `FUN_0040F3A0` basis stored on the camera would still need a
conversion at the GL upload. Same pixels if the matrix is equivalent; fewer
missed consumers. It is a dedicated presentation refactor, not a drive-by
fidelity upgrade. Every live-world consumer of view X must honor the adapter
until then:

- `Camera::view_matrix` / `Camera::right`
- `glFrontFace` in the GL backend
- free-fly mouse yaw
- cylindrical camera-facing people (`man2`, `lev1sci2`, …): `Ry(atan2)`
  alone puts implicit sprite U on screen-left and 8-dir walk cycles moonwalk
  once heading matches motion. Reflect local +X at that submission when
  `left_handed` is set.

### Authored world projection

`FUN_0042B230` initializes the six display-context words from system-2
Section-0 as focal X/Y, reference width/height and screen centre X/Y.
`FUN_00453570` refreshes the two focals, bounds and centre X, retaining the
selected block's initialized centre Y. Its focal writes at `004535F9/00453620`
multiply by the session's 16.16 zoom at `+0x26C`. The session constructor
`FUN_0044FA30` (first callback in descriptor `004D0B10`) initializes that field
to `0x10000` at `0044FB0E`; the executable has no other direct writer. Default
zoom is therefore 1.0. The selected system tiers contain:

| Tier | Focal X/Y | Reference size | Centre X/Y |
|---:|---|---|---|
| 0 | 256 / 256 | 320 × 240 | 160 / 120 |
| 1 | 512 / 512 | 640 × 480 | 320 / 240 |
| 2 | 640 / 640 | 800 × 600 | 400 / 300 |
| 3 | 820 / 820 | 1024 × 768 | 512 / 384 |

At tier 1 the vertical lens is `2 atan(240/512) = 50.22967°`. The port's
former generic 60° world camera made objects only 81.19% as tall at the same
pose. `WorldProjection` now loads the selected system-2 values explicitly,
retains the principal point and focal-axis ratio, and reapplies the lens at
world construction, reset and viewport changes. Viewer defaults and separate
menu cameras retain their own policies. Native expands horizontal coverage
with the logical aspect; Stretched keeps a 4:3 world and stretches the final
image. At 16:9 those modes differ by one third in horizontal world span.

This lens correction does not change the chase target, terrain clearance or
spring. Compare pose, craft heading, Active Camera and scaling mode before
inferring a chase-distance error from screenshots: the authored chase distance
itself depends on the craft's lateral basis, and unrelated frames need not
share the same eye position.


The native scene screen stage uses the same `WorldProjection` allocation,
preserving the original six integer words rather than rounding the GL matrix.
`FUN_0042B870` explicitly installs that System-2 block at context `+4C..60`;
Intro2 `FUN_0042AF00` publishes the current context's six words to
`004FEED0..004FEEE4` before actor traversal. The latter does not itself load
System-2. `scene_projection_authority` admits `NativeScreenProjection` only
when a retained `NativeWorldViewport` accompanies the exact selected authored
logical dimensions. Free camera and the resize/widescreen logical adapter
retain their named compatibility routes. Claimed native scenes without raw
lens or view custody carry distinct `NativeLens`/`NativeView` boundaries.

`begin_scene` resets screen authority; buffered model commands own the display
authority they received at submission. A later actor, scene, or flush cannot
replace its lens or effect. The four-tier corpus regression requires the
actual overlays and checks their owning System-2 words despite a newer
unrelated pool. Screen authority contains focal lengths, centre, bounds and
the current projection effect. Native terrain fog, scan constructors and
global queue/raster ownership remain separate inputs; this publication does
not establish matched retail raster pixels or an arbitrary session zoom.

### Entity model visibility

`FUN_00411400` supplies the ordinary main-model window as well as the next
scheduler visit's detail bits. The tight test uses wrapping signed X/Z
differences in world axes: `-columns/2 <= dx <= columns/2`, `dz <= rows`,
and `dz >= 0 || dz >= rear`. In raw units,
`rear = wrapping((entity_y - camera_y) * (true_up_z_q31 >> 15)) >> 16`.
Camera Y remains a dword. There is no radial far cutoff or negative-Z box
limit, and model radius does not expand this outer window.

Normal gameplay and the world-load handoff reuse `RetailViewDetailContext`
and require its `Full` geometric result. The previous camera-yaw rectangle
could discard a complete building even with both world-axis differences
inside the authored limits. Canonical Main Base and factory regressions cover
diagonal views, camera height, inclusive boundaries and torus images. The
developer free camera explicitly retains its rotated compatibility window;
retail's chase target enforces a forward +Z separation and cannot represent
every arbitrary backward free-camera view.

This correction preserves existing actor eligibility and state publication.
It does not implement the complete `0x800` callback-admission gate: retail
skips the ordinary body callback when that bit is clear, then proceeds to its
separate attachment/effect suffix. Static terrain objects also retain their
existing traversal, described under steep-camera coverage below.

Attached bodies have a narrower, source-backed admission: `11720` still visits
their own live allocations, and `11400` does not test relation bit `1000`.
`18440` clears body bit `800` when the selected Sub-J slot's low policy byte
is zero; Type17's policy1 slot retains it, so its captured person remains an
ordinary model submission. Player cargo's policy0 slots hide the child through
that same bit. Both port world draw loops now admit attached bodies only when
`800` is proven set, while retaining the existing unparented admission and the
unresolved full body/effect gate above. See [Type17 Capture](TYPE17_CAPTURE.md).

The later model entry `FUN_00464E60` separately compares signed view Z with
the unsigned radius at model header `+0x08`. It rejects
`view_z - radius >= far`, and otherwise chooses the far constructor table
when `view_z + radius >= near`. Header bit `0x20` forces the near table.
`FUN_00467410` selects again for each child's own origin, radius and flags;
a near-only parent does not disable its children's fog. Command-stream LOD
branches are a separate interpreter boundary. `ModelTreeRenderer` applies
the far-radius rejection before static, linked and painter materialization,
so a rejected node emits no billboards, child instances or live Sub-H writes.
It reads the active scene's logical world fog planes; menu scene entry
suppresses inherited world planes, while explicit submission planes
override them, with frontend raw planes converted by `/100`. Disabled fog and
header `0x20` bypass that node's rejection. Children of admitted parents repeat
the test using their own origin, radius and header. Intrinsic diagnostics
without a scene view retain their unculled inspection path. The camera and
radius adapters still use the port's floating scene transforms.

## Named presentation adapters

The original projected-packet/queue system has software and
alternate/Direct3D consumers. OpenGL is the port's GPU backend; the
mismatches below are **named submission adapters**, not a generic DX7→GL
state table. Do not add a blanket axis, UV, or winding flip to "fix" one object.

Classify a live hairball against this list before changing a parser, a
model, or a heading. Each adapter already has an owner. Reopen that owner;
do not invent a second copy.

| Adapter | Retail | Port | Failure if missed | Do not |
|---|---|---|---|---|
| **View-X** | `FUN_0040F3A0` `right = up × forward`; authored +X is screen-right | `Camera.left_handed` negates view X and sets `glFrontFace` CW. Menu cameras stay right-handed. | World/cinematic left-right mirror; chase detach remirrors | Flip overlay bytes, reverse parsed UVs, 180° heading/Sub-I patch |
| **World lens** | System-2 focal lengths/centre and session 16.16 zoom | `WorldProjection` applies selected-tier lens to the logical viewport | World appears too distant; resize restores generic FOV | Fit a chase offset to unmatched screenshots |
| **Menu local-X** | Frontend model +X opposite the world basis | `MENU_MODEL_LOCAL_X_BASIS` at the two menu hierarchy roots | Save reads `200V`; Exit arrow on the wrong jamb; Klaus wing masks leave the joints | Reverse every quad's U |
| **Camera-facing local-X** | Cylindrical people; implicit sprite U tracks screen-right | Reflect local +X at that submission when `left_handed` | 8-dir cycles moonwalk once heading matches motion | Flip sprite UVs in the parser |
| **Dual vertex space** | Software near-table is already screen pixels + 8.8 depth | Inside a model draw: **raw-local**. Helpers (project, edge-quad select): **world**. `0x02`/`0x22` fill: **pixel ortho** (`Vertex3f(sx,sy,ortho_fill_z)`). Convert at every boundary. | Legs vanish (`projected ≈ 0`) or smear across the frame; headless fixtures still pass | Feed locals to a world projector; unproject fill pixels back into view space |
| **Torus draw image** | i16 overflow is the wrap | Owned native type-14 raw words use `viewport_origin_dword + (short)(endpoint_word - viewport_origin_word)` on all three axes before Q31 projection. Free/intrinsic compatibility retains `draw_origin + (short)(endpoint - actor_origin) / 256`; neither path uses absolute `i16/256` or a full i32 difference. Level-1 newants spawn at `+0x9A = 0x7F00` (world Z 127), so a +4 world foot wraps the stored word. | Hip-to-foot `0x02` lines span one world period (sky slivers, moving shadow, missing body) once heading rotates a foot across the signed-8.8 seam | Mix wrapped `[0,256)` with the unwrapped draw image in one primitive |
| **Sub-H / draw basis** | Type-14 callbacks run in the same object basis the body uses. `FUN_0046ECF0` then **bypasses** the model transform and projects that world point. | Selector world points feed the edge gate directly. Do not invert type-14 into raw-local and back through `O` for `0x02`/`0x22`. Newant authors six type-13 `0x22` ribbons whose source slots are type-14 primary feet; resolve that callback before terrain projection. The visible legs are twelve type-14 `0x22` plus fourteen `0x02`. | Type-13 blobs on the ground with no insect (newant); spider legs still work because their 40 edges are type-14 | Invert type-14 through the body matrix to "fix" one insect |
| **Edge width domain** | `FUN_004594C0` divides by 8.8 raw depth (world × 256) | Edge path keeps raw 8.8 end-to-end, `0x3F` near guard, `0x2000` cap | Legs 256× too wide, ribbons across the viewport | Divide by float world depth |
| **Implicit UVs** | Textured tri `(0,0)/(Umax,0)/(Umax,Vmax)`; quad full rectangle; mirror opcodes keep corner order | Parser stores authored UVs. Handedness lives in the scene basis above. | Purple hut-roof wedges; Klaus masks off the bone | Reverse UVs because one prop looked mirrored |
| **Overlay depth** | No z-buffer; painter overwrites coplanar terrain | `--overlay-depth-policy terrain-recede` is a window-Z bias | `player4` shadow / Targetter crosshair lose fragments on dry ground | World-Y nudge, global polygon offset, depth-test off |
| **World alias dependencies** | World tf12/tf13 callbacks run before generators and linked imports | Recursive slots carry raw-local coordinates, optional authoritative callback world points, semantic clip and surface provenance; intrinsic/collision decoding stays separate | Tall fence endpoint or dark dragon quads stretch from wing height to ground | Scale assets, clamp shadow length or project only final draw corners |
| **Model painter groups** | Signed camera-Z queue keys; sorted/unsorted nested lists | `MaterializedModel.painter_program` → tree queue; isolated `Painter` or validated `OpaqueSceneGroup` → `PainterGroup` depth | Handoff mattes hide Klaus's body; frontend body clips the emblem | Move vertices in Z, draw every black face first, or flatten descendant Z into the enclosing queue |
| **Klaus matte extent** | Logical aspect expands only outer linked supports in Native | `ModelTreeRootLinkPolicy::KlausMatte`, root slots 2–5 | World leaks at widescreen edges while Klaus is closed | Zoom the camera or stretch the body |

Intro2 may mix **live entity pose** (types whose visit already commits
`FUN_00401430`) with **presentation proxies** (`intro_actor_pose`) for
everyone else. Mixed ownership looks chaotic; it is not a GL convention
bug. Type-13 B6C0 now commits its normal Sub-G component, angle, velocity,
sound, and Entity animation-selector state. The current Intro2 presentation
controller may overwrite those selector outputs. The post-task DCA0/E870
basis refresh is live, with same-pass Aim using the retained pre-refresh
matrix. Normal Intro2 scheduler/environment/master-motion integration and
detailed Sub-K/Sub-L outputs are now live. It stays on the proxy until
selector presentation ownership and matched visual acceptance close.

The dual-space formulas, fill-space dump (`V2000_EDGE_FILL_DIAG`), and
ground-overlay policy are expanded below.

## 1. The deferred render queue (painter's algorithm)

The rasterizer itself is shared by menu and gameplay, but the callers do not
share an implicit live graphics state. Each frame installs a world/render
context and every queued primitive carries the material/shade information its
filler needs. The menu additionally has a distinct 3D world pass followed by a
UI pass before the queue is sorted and drained. A GPU port should therefore
share renderer implementation and assets while establishing an explicit
menu/world scene boundary; relying on persistent OpenGL state has no analogue
in the original and allows fog/blend/depth state to leak between modes.

Face handlers (`cmd_stream_handlers.c`) don't draw — they **append a primitive
record** to a per-frame queue at `ctx[0x15]`. Each record: `+0x00` next ptr,
`+0x04` **callback thunk** (`LAB_0047a7e0` tri / `LAB_0047a960` quad/billboard),
`+0x08` packed payload (screen-space corners + material/color). Push helpers:
`FUN_00459D10` (plain append), `FUN_0045B220` / `FUN_0045B280` (append **with a
depth sort key**, supplied by the 0x06/0x46/0xA6/0xC6 depth opcodes).

Flush is two stages in `bulk/game_logic.c`:
- **`FUN_00494930`** — bottom-up **merge sort** of the linked list by the depth
  key, back-to-front; equal keys compare the records' arena addresses.
- **`FUN_00494A50`** — **drain**: walk the sorted list, call each record's
  callback `(*record[1])(device, record+8)`. Each thunk dispatches into the
  **device fill-pointer table** (`device+0x1018..0x10C4`, set up in
  `FUN_00480D10`) — e.g. `call [eax+0x1080]` at 0x47A960.

**Port:** there is no z-buffer in the original. Hardware depth is not generally
equivalent even for opaque faces: Klaus's black cover polygons are physically
nearer than parts of his body but belong to deferred groups behind it. Ordinary
world submissions retain the existing geometric-depth path; Klaus's isolated
handoff explicitly uses the authored nested painter program below. Billboards
and translucency also require their recovered keys rather than geometric
depth alone.

### Original primitive queue controls

The promoted queue oracle
executes original `4948C0/4948F0/494930/494A50/494A80/494AB0/494B60`,
`459D10` and `45B220` instructions. Eight controls and the adjacent
twelve guards establish
signed descending keys, arena-address ties, repeated drain without record
consumption, atomic nested FIFO/sorted groups, first-nonzero propagation,
closing open scopes, payload alignment and overflow without an implicit flush.
The fill callback is an explicit inert payload-ID/return-value provider;
model, terrain, H, RNG, software fillers and the viewport allocator do not
execute in these controls.

Original `44F540` supplies the shared viewport `4D04E8` and arena request
`0x19000`; the guarded source allocator prefix gives the limit at
base+102388 and `4948C0` starts its cursor at base+24. The accepted tick952
empty sorted-root header corroborates those offsets. Its absolute allocation
address is evidence, not a runtime constant. The source constructor is
statically decoded; this does not execute its complete allocation/window chain.

This proof has no native world queue consumer in the port. Whole-body
submission buffering owns callback results and scene lens data, but does not
own primitive allocation offsets or interleave model, terrain and effect
records in one root. A model-only sort, first-endpoint GL Z or terrain bias
cannot replace the shared queue. Prepared producer keys/allocation order,
fractional terrain strips, nested dispatch and software fill remain integration
and pixel-acceptance work.

### Original model primitive receipt controls

The adjacent constructor oracle
and fifteen fail-closed guards
execute 87 original-PE controls: 56 faces, 11 groups, 12 billboards and eight
node-pass selector prefixes. Full opcodes matter. Newant302's reached
`83/84/A4/A7/A8` use near constructors `45A4E0/45DB30/463040/462070/464270`;
the fog constructors are `45A730/45DDE0/463330/4624C0/4647C0`.
Sorted face records use the first completed VIEW Z, with a separate mirrored
first corner; FIFO records have no individual key. Controls retain actual
palette/sprite packet bytes and allocation sizes. Near billboards allocate
24-byte payloads, fogged billboards 32; their key is the anchor VIEW Z.
Bit40 rejects a billboard before its operand callback, and a culled face
normal prevents corner/material callbacks.

The `464E60` prefix chooses the constructor table before the model loop:
queue-absent selects C pre-resolution; header flag20 selects near A. Otherwise
wrapped signed `originVIEWZ-u16(radius)>=far` rejects the node, and
`originVIEWZ+u16(radius)>=near` selects fog B, including equality. Prefix
controls stop before `46D4B0`; cache allocation and the model loop do not run.
Warm cache/normal rows, raw scene words and palette/sprite providers are
explicit controlled inputs. No cold vertex, H, emitter, RNG, terrain producer
or software filler executes, and these controls are not paired retail pixels.

These are packet-production proofs, not a shared world consumer. A native
receipt must retain full opcode, selected table, command-time cached corners,
raw key and source packet size. Fogged constructors additionally consume the
source byte17 fade and raw globals `4FEEE8/EC/F0`; the current six-word lens
alone cannot provide that authority. Existing float/final-register face,
billboard and group preparation cannot lend an exact native packet.

### Authored model painter groups

`FUN_00494AB0` inserts a group record in the enclosing list and opens a sorted
child list; `FUN_00494B60` opens an unsorted child list. `FUN_00494A80` restores
the enclosing list. Sorted lists use descending signed keys and arena-address ties;
unsorted lists retain command order. Instances expand inline in the current
scope, so parent-body-first traversal loses meaningful ordering.

Model opcode `0x15` opens a sorted group. `0x06`/`0x26` use the maximum/minimum
listed vertex Z and open unsorted groups. `0x46` (single vertex), `0x66`
(origin), `0x86` (fixed -1), `0xA6` (fixed signed dword), and `0xC6` (vertex plus
signed dword offset) also open **unsorted** groups; `0xE6` closes a group.
For C6, `FUN_00466BD0` takes the first operand as a signed vertex-slot reference
and computes `camera_z + u16(low) + s16(high) * 65536`. The offset changes the
queue key, never the vertex position. Resolve each key against the registers
at that command, before later register writes. The audited near/fog face
constructors `03/04/07/08/23/24/27/28/83/84/A4/A7/A8` key each source polygon
by its first completed VIEW Z when the enclosing scope is sorted. Mirrored
copies have separate keys, and quad triangulation stays atomic. Other face
families require their actual constructor audit.

Normal-tier Klaus head model 8 opens its palette-32 matte group at slot 16
plus 1500; jaw model 9 uses slot 4 plus 2250. Other C6 groups contain textured
body details, so neither a palette whitelist nor treating every C6 as a matte
is equivalent. `MaterializedModel.painter_program` preserves face ranges,
instance order and group operations independently of camera/presentation.
The handoff's opted-in linked traversal resolves keys in camera space,
sorts/drains the nested groups, and submits atomic polygons with
`ModelDepthPolicy::Painter`.
The isolated handoff uses `ModelTreePainterComposition::Isolated`; the GL
boundary suppresses depth testing/writes and restores incoming depth state.
Ordinary world geometry keeps `ModelDepthPolicy::Geometry`.


### Authenticated childless painter adapter

The existing opted-in isolated model painter now consumes command-time native
VIEW keys when the scene supplies `SceneProjectionAuthority::Native` and the
live Sub-H presentation supplies its current `NativeModelFrame`. It reuses
the ordinary live-node materializer, including authored normal admission and
H/E slot ownership, instead of preparing an intrinsic model before callback
submission. `ModelPainterDepthKey::Native` carries the first endpoint/anchor
Z, group extrema or current-node origin; C6 adds its signed offset with dword
wrapping. Missing native key inputs cannot borrow a float point.

The active [painter consumer](../../crates/v2k-game/src/model_tree/painter.rs)
drains prepared faces, edges and billboards in mixed key order. Quad triangle
ranges and nested groups remain atomic; ties retain earlier emission order.
The existing submission buffer owns these slices and edge packets, so drain
cannot revisit H/E, registers or terrain projection. Billboard submissions now
carry the same depth policy to the shared GL quad path; isolated Painter
suppresses depth testing and restores the incoming depth test, write mask,
function and range. Ordinary Geometry keeps its existing sprite policy.
Ordered model faces/edges and shared sprite quads use one explicit depth-state
snapshot. It queries the range as doubles, preserves test/mask/function, and
restores those fields after submission. Sprite fog still uses its own attribute
stack, but depth does not: the actual isolated GPU control found compatibility
`PushAttrib/PopAttrib` changed `[0.2,0.8]` to
`[0.20000000298023224,0.800000011920929]`. The exact-state regression keeps
nondefault and reversed ranges; accepting a tolerance would hide a state leak.
This is GL adapter state custody, not original software-raster equivalence.
OpaqueSceneGroup still rejects edge/billboard programs because their enclosing
scene colour/depth composition is not established by this adapter.

Before invoking live callbacks, a conservative instruction-path preflight
rejects any possible child instance, malformed/unknown instruction or native
13/14 distance branch. A reached-instance visitor and the native distance
input are separate owners. Diagnostics deduplicate by model and exact typed
boundary, so one unsupported model cannot suppress a different failure.
Explicit Free/resize compatibility retains float key preparation; claimed
native missing projection rejects before H/E. Real newant302 and childless
spider256 controls exercise the live consumer; child-bearing stag267 checks
that rejection precedes H writes. The newant control retains tick952 VIEW,
warm H endpoints and 69 recorded first-slot keys (35 face records/34 edges).
Group/offset/origin literals are independently grounded by the original-PE
constructor controls above, including empty extrema and signed overflow.

This recovers keys and an active model adapter. Face geometry and billboard
projection retain their existing presentation contracts. The raw queued-world
node selector, successful queue allocation, same-draw fog publication and
final-surface query are owned by
[MODEL_DRAW_CUSTODY.md](MODEL_DRAW_CUSTODY.md). Its current acceptance
boundary includes the source-supported admission/callback RNG changes and
remaining Intro2 scene comparisons. Ordinary gameplay and Intro2 retain Geometry
composition; numeric adapter controls and GL state checks do not establish
shared native scene ordering or retail pixel acceptance.

The frontend has a different enclosing contract. Model 1 branches on callback
selector 1 (`0x81`): zero opens `0x15` at slot 12, local `(0,0,2560)`;
nonzero opens it at slot 6, local `(0,-179,-5120)`. Each branch encloses the
complete model-2 instance and closes with `0xE6`. `FUN_00470840` reaches
`FUN_0040D320` → `FUN_0040A950(entity,1)`; C660 clears that word and C710 adds
private `+0x10`, the mouth morph. At idle the zero branch adds local `0xA00`
to Klaus's private view depth `0xA00`, giving outer key **`0x1400`**.
`FUN_0040F9C0` selects Klaus for the next ED10 camera update, which converts
private view XYZ into world coordinates and assigns the camera basis; the
world renderer reverses that transform. Do not add the camera's `0x800`
trailing distance again. Submenu-away view Z reaches `0x1400`, moving the
zero-branch group key to `0x1E00`.

`FUN_0042D030` queues the additive emblem at **`0xC00`** in the restored
sorted root queue. AA40's prop context aliases that same queue; row flags do
not create groups. Klaus's descendants drain atomically, so the former claim
that his nearer head/body should mask the emblem was incorrect. A 640×480
port GL diagnostic found **5,862 of 43,217** emblem pixels suppressed by the
old geometric depth path, across body, inner wings and tail. These are port
measurements, not a matched retail frame. Frontend model state 1 reaches no
palette-32 matte faces, so this clipping was not the handoff matte defect.

The frontend now requests `ModelTreePainterComposition::OpaqueSceneGroup`.
Before submission, traversal requires exactly one positive outer group and
checks all reached materials are `Masked`. The normal-tier Klaus hierarchy
uses only sprite flags `0x04`/`0x05`, with no additive or half-additive faces;
handoff state 0 additionally reaches opaque palette 32. Nested groups drain
in authored order, while `ModelDepthPolicy::PainterGroup` writes the outer
key as constant GL depth for every covered pixel. Keyed holes retain prior
depth; geometry, projection and fade coordinates remain unchanged. The GL
boundary restores depth range, function, test and write state afterward.
This is a bounded opaque-group adapter: ring submissions retain their current
geometric depth approximation, not a full retail shared-scene queue. The
isolated handoff is unchanged. Unsupported group shapes, translucent materials,
billboards or edges are rejected before visible submission. Code and port GL
clipping are corrected; matched retail visual acceptance remains pending.

### Klaus widescreen matte

Native presentation expands horizontal FOV. The normal-tier root model 1
exports mirrored outer support slots 2–5: the two authored negative-X corners
are `(-768,640,-1536)` and `(-768,-1280,-1536)`. Linked traversal passes them
through the body and head to head/jaw palette-32 cover polygons only.
`ModelTreeRootLinkPolicy::KlausMatte` scales their exported X coordinates by
`max(1, logical_width * 3 / (logical_height * 4))`. This is an explicit port
widescreen policy; source records, child attachments, mouth aperture, textured
body, camera and model size remain authored. Descendants use ordinary link
exports. FourThree/Stretched logical viewports remain 4:3, including Classic
Framebuffer, so their supports are unchanged.
The local corpus confirms identical Klaus models 1–12 (records, normals,
command words and render headers) across system tiers 0–3; this nonvisual
linkage policy therefore applies to every selected presentation tier.

## 2. Software rasterizer / DirectDraw surface

`FUN_004A9CE0` (`directx_wrappers.c`, tagged `…\Windows\DDCalls.c`) calls
**`IDirectDrawSurface::Lock`** (vtable +0x64) for a raw 16-bpp framebuffer
pointer; scanline fillers write pixels with the classic `push dx` trick (ESP
redirected to the destination scanline). The 16-bpp software table (mode
descriptor `+8` zero, `+4` = 16 bits per pixel) consists of the fill-slot handlers
`0x47AB20…0x480790`, the scan converter `0x472B20` with its helpers, and the
span-row routines `0x473690…0x47A540`; `0x481240…0x492A10` belongs to the
alternate/Direct3D table.
[SOFTWARE_RASTER.md](SOFTWARE_RASTER.md) is the full contract and the state
of its byte-exact port.

### Full-frame DirectDraw trace acceptance (2026-07-27)

The accepted local `v2000-intro-framecapture.trace` independently validates
this path in the retail 1024×768 presentation mode. Call `321664` selects
1024×768×16; call `321665` creates a system-memory primary flip chain with one
backbuffer. The locked surface has pitch 2048 and exact RGB565 masks
`R=0xF800`, `G=0x07E0`, `B=0x001F`. Presented frames use
`Lock` / CPU framebuffer write / `Unlock` / `Flip(DDFLIP_WAIT)`.

In that recorded epoch, no later sprite-composite, colour-key, palette,
filter, lighting, fog, or darkening operation appears between the final CPU
write and Flip. Despite its retail menu “Direct3D” label, that epoch contains
no per-primitive Direct3D draw calls: visible world, particle, model, HUD and
caption pixels are already resolved in the locked framebuffer. Its final
frames are visual oracles for their own recorded presentation, but cannot
recover individual software depth keys, material flags, camera matrices or
actor callbacks. This does not classify another recording: the tick952
indexed draw independently executes the shipped alternate/Direct3D table
and presents without that CPU Lock route ([backend custody](MODEL_DRAW_CUSTODY.md#same-draw-backend-discrimination)).

The trace's earlier 640×480 epoch, before call `321664`, is not a colour
oracle: its frames carry zero red throughout (white logos read cyan, the
yellow ring label green) while their green and blue match the same assets
exactly. Static screen elements of the 1024×768 epoch do compare exactly:
the frontend's ring label, copyright banner and logos (Flip `324003`) and the
Intro2 caption band (Flip `345026`) match the port's software frames pixel
for pixel.

The frontend's last per-frame DirectDraw black `COLORFILL` is call `325164`.
Intro2 then presents complete world frames without that external fill until
the black outro begins at call `356152`, immediately after Flip `356151`.
This is not evidence for an omitted world background: the CPU world pass still
writes the authored full-frame background described below. Yellow columns,
texture holes, lighting errors, and bad effect occlusion in the port must be
fixed in its scene/material/particle paths rather than hidden in DirectDraw
presentation.

The first village impact also settles one apparent resolution quirk. Its
billboards cross both legacy coordinates `x=640` and `y=480` and clip only at
the actual 1024×768 edges. One flame frame has an obvious square texture
boundary and the combined blast is intentionally enormous; reproducing that
retail look must not introduce a fictitious 640×480 scissor or anchor.

The same trace supplies final-pixel acceptance for several otherwise easy to
conflate policies:

- calls `327478..328904` show the village impact progressing through multiple
  animated camera-facing fire/smoke sprites, white flashes and authored grey
  billboard debris, with no remnant that waits for a camera pan. The PE
  descriptors for classes 16/30/93 select Section-3 frames699--702 and
  `43DC90 -> 43D410`, rather than the separate model-draw callbacks. Mode 3
  describes their update/collision path. `43D410` projects half-extents as
  `((sprite_dimension * effective_scale * frame_scale) >> 8) * focal /
  (depth_raw << 9)`; the port full world extent divides that numerator by
  65536, and its perspective half-extent multiplies by 128/depth_raw, giving
  the same size factor. Tier 1 and tier 3 retain identical pixels for these
  masked, fixed-row 28 sprites. Exact synchronized size/count/lifetime
  acceptance still needs matched births, RNG, camera and render cadence;
- calls `346264..346698` show a separate ground-aligned expanding magenta
  impact ring, while calls from `347008` show dragon fire as a chain of
  discrete flame sprites rather than one stretched beam;
- call `325556` shows opaque, textured, terrain-bounded trunks with separate
  foliage crowns; both trunks and crowns receive distance fog, while villagers
  remain animated camera-facing sprites;
- call `355006` shows a bounded dragon silhouette projected onto the terrain,
  including its articulated wing pose. It is neither a vertical connector nor
  a camera-facing extrusion; and
- calls `331818..334050` show the red infection footprint fixed to and
  conforming over terrain slopes and shores. Its irregular coverage is
  separate from the hive's round smoke/bubble billboards and dead vegetation.
  The stable footprint permits texture/palette/UV animation as an explanation
  for its apparent motion, but final pixels alone do not prove which mechanism
  retail uses.

The final images strongly support particles depth-testing against opaque world
geometry while retaining a transparent ordering policy among themselves, but
they cannot identify the exact queue keys. Keep that point classified as visual
evidence rather than recovered state.

### World background / sky

`FUN_00433130` copies six render-resource words into `0x4DB250`. Normal world
setup packs Section 13 `+0x54/+0x56` into its third dword. Main Base abort
`FUN_0042F1A0` retains request dword `+0xAC`, replaces that packed pair from
Section 13 `+0x58/+0x5A`, and copies `+0x4C`, `+0x88`, and `+0x8C` into the
remaining request words. `FUN_0042E3B0` zero-initializes the complete owner, so
the retained dword is statically closed as zero. Level 13 therefore changes
from palette/model `27/305` to `32/0`; all four display tiers author the same
pair, palette 32 is black, and model zero deliberately suppresses `sky1`. In
controller-offset order `[+A8,+AC,+B0/+B2,+B4,+B8,+BC]`, the exact abort
request is `[0x0042E860,0,0x20,0x650,0x15,8]`. These fields come from matched
retail/demo C and Section-13 bytes, not the accepted trace's mistakenly
labelled `+0x27C` controller window; the request owner is session `+0x25C`.

At the start of `FUN_0042F270`, before terrain and water, the low half indexes the system-level-2
master Section-7 palette through `DAT_004FE63C`. The selected RGB555 word is
queued as a complete-frame primitive using thunk `0x47A7C0`; the installed
16-bpp callback is `FUN_0047B9F0`, which fills every pixel with that word. This
is a solid authored background colour, not a stretched sprite or generated
gradient.

The high half is an optional global Section-8 model id. Only levels 13 and 14
set it (305/306 = `sky1`/`sky2`). `FUN_0042F270` submits two identity-oriented
copies: X follows half the camera X, Z follows the camera, and the second copy
is offset by `-0x8000` in 8.8 coordinates (−128 world units). This produces the
distant parallax sky geometry over the solid colour.

The port keeps the six-dword request under a per-level-load controller lease.
After the terrain-transform attempt, it atomically stores controller state 5
and submits the complete-frame clear, terminal fog colour, and optional sky model.
The unchanged terrain-base/depth words continue to drive the already-resident
terrain frame set and bounded scan footprint.

RGB555 components are expanded as `component << 3`, not normalized or
bit-replicated to the full 0..255 range. `FUN_004a84a0` masks each expanded
channel with `0xF8`, so the maximum authored component is 248. The matched
Level-1 background demonstrates the consequence directly: packed `0x0214`
presents as `#0080A0`; normalizing the same word produces the port's former
incorrect `#0083A4`. This policy applies to Section-3 shade palettes and direct
Section-7 colours alike.

### Model face rejection uses authored planes, not winding

`FUN_0046D5A0` reserves normal refs 0/1 as an implicit always-visible,
shade-table-zero pair. Stored normal record zero therefore starts at refs 2/3;
for any `r >= 2`, the record index is `(r >> 1) - 1`. The record's first word
names an authored vertex-slot anchor and its remaining words are the signed
normal. Odd mirror refs negate normal X and XOR 1 into the anchor slot, so the
anchor and normal remain the same mirrored plane.

`FUN_0046D3F0` resolves that plane before the primitive is projected and accepts
only a strictly negative camera-relative dot product. The GPU backend evaluates
the algebraically equivalent world-space form after applying model scale,
orientation, and translation to the anchor and orientation to the normal:

```
dot(world_normal, world_anchor - camera_position) < 0
```

A zero dot is rejected. `GL_CULL_FACE` remains disabled because shipped triangle
winding is not canonical; the explicit authored-plane policy is carried in
parallel with each body triangle instead. Full-corpus validation covers all 36
OVLs / 5,344 model entries / 124,100 canonical faces with zero unresolved planes.

### Authored mount rotations

Model opcode `0x5C` (`FUN_00466F80`) establishes a mount from the parent
basis, its signed axis permutation, and an optional packed-angle rotation.
Opcode `0x1C` (`FUN_004671D0`) applies another rotation to that existing
mount through the same `FUN_004671F0` helper. All three retail model dispatch
tables use this handler. Axis 1 rotates basis vectors 2/0, axis 2 rotates
0/1, and other values rotate 1/2; one packed turn is `0x10000`. The following
code-6 `0x0E` child copies the composed mount. Rust transposes the stored
basis vectors once into its child-to-parent matrix; the scene's handedness
adapter remains at the hierarchy root.

Normal-tier `1X2XX.OVL` model 8 `pteranasetheadklaus` contains
`5C 0 0 89; 1C 0 C7` before instancing jaw model 9 at slot 28,
`(0,-150,240)`. Its stream computes register 7 as
`max((callback[1] & 0xFFFF) >> 1, 0x4000) - 0x4000` (the following
exclusive `0x6000` cap is unreachable in that input domain). The jaw's X
angle is therefore callback 9's spin pose plus this authored opening: zero
through progress `0x8000`, 45 degrees at `0xC000`, and nearly 90 degrees at
`0xFFFF`. A fixed 33.75-degree jaw offset cannot represent this transition.
The root also uses `0x1C` for its -22.5-degree rest tilt; body/head/tail,
Options, and many world actors contain further composed mount rotations.
Do not replace a missing opcode with a scene-specific angle or UV change.

### Type-13 vertices use context-specific callbacks

The default type-13 callback family (`FUN_0046EB60` / `FUN_0046EBD0` /
`FUN_0046EC60`) first resolves the referenced source slot normally. It copies
the source's view X and Z, replaces view Y with `0xffff8001` (signed `-32767`
in the engine's 8.8 coordinates, therefore `-32767/256` world units), and
projects that point. The port's explicit `CameraFacing` mode implements this
default-family transform. It is neither a model-relative endpoint nor the
world's terrain-shadow policy.

World setup takes a different path. `FUN_00433FA0` installs
`FUN_004349C0` / `FUN_00435090` / `FUN_00435780` for type-13 corners. Given a
source world point `P`, those callbacks:

1. sample the initial surface `S0` at `P.x/P.z`;
2. compute `X' = P.x + q31(P.y - S0, slope_x)` and the corresponding `Z'`;
3. resample the selected surface once at `X'/Z'`; and
4. set the endpoint Y to that second sample.

The same call also swaps the three tf-12 alias entries for
`FUN_004340B0` / `FUN_004343A0` / `FUN_004346B0`. Those copy slot *a*'s
position, convert it to object-relative world offsets, replace Y with the plain
bilinear terrain sample at that world X/Z relative to the object origin, and
rotate back — no slopes, waves, or sea-band clip. Static terrain objects keep
their identity root orientation; this handler, not camera-facing yaw, is what
grounds tree-trunk bases and their ground quads.

The general/above-water branch samples `max(bilinear terrain, animated wave)`
at both positions. The deep-underwater branch samples the seabed only. When
terrain is at/below the flat sea and the source Y lies inclusively within
`sea +/- 0x96` raw units, the callback contributes clip bit `0x40`; the port
therefore rejects the complete face after resolving its corners. That interval
is a water-surface rejection band, **not** a maximum projection length.

`FUN_00433BD0` derives the two signed Q1.31 slopes from the Section-10 direction
as `-direction_x/direction_y` and `-direction_z/direction_y`, saturating each
magnitude at one. Level 1's `(-73,73,-73)` is first reduced to
`(-18,73,-18)`, yielding `(+0.25,+0.25)`. This vector is fixed world data; it
does not rotate with the camera. Camera pose affects only the ordinary final
projection of the resolved endpoint.

The port consequently keeps `CameraFacing`, `WorldSurface`, `Raw`, and
`Disabled` as explicit policies. Broad gameplay/Intro2 draws use
`WorldSurface` and fail closed if their live Section-10 terrain is unavailable;
default/menu contexts do not silently inherit it. Model parsing retains every
authored face—including all-type-13 faces—in the canonical face stream with
its material, UVs, corner normals, shading, and authored cull plane. World
projection preserves the submitted face's authored material/blend/depth
behavior and re-establishes `LEQUAL` for faces containing a world-projected
endpoint, including type11 imports carrying that provenance; it
does not turn them into a categorical read-only underlay. Only mixed faces in
the legacy/default `CameraFacing` compatibility mode retain that no-depth
exception. The old synthetic `shadow_triangles` split is not a retail
cast-shadow subsystem.

#### World alias dependencies

The tf-12 replacement above applies while resolving the slot graph, before
other generators consume the result. Canonical `fence1` (global model 492)
defines slot 4 as `(256,256,0)`, slot 6 as its tf-12 alias, and slot 8 as
the tf-6 completion `slot6 + slot2 - slot0`, with slot 2 `(0,128,0)`.
On flat terrain at the object's origin, slot 6 becomes `(256,0,0)` and
slot 8 becomes `(256,128,0)`. Intrinsic decoding instead gives slot 8
Y=384. Grounding only the final tf-12 draw corner therefore leaves one
end three times too tall. Ordinary fences `492..499`, spikes `512..526`,
and the pen's authored `532/540/542` variants share this dependency.

World materialization supplies `ModelVertexResolver` with the submitted
transform, terrain and retail tick. Both tf12 and tf13 callbacks sample the
resolved source in world coordinates and return raw-local coordinates before
generators, authored plane anchors, mount positions and linked exports consume
them. Static and linked draw paths both enter this contextual resolver when
the model needs either callback. Mirrored slots resolve and sample their own
reflected world X/Z; reflecting an already-grounded even slot gives the wrong
height on a slope. `ModelSurfaceResolution::ContextResolved` tells the backend
not to apply either callback a second time. Default/menu modes remain separate.

`ResolvedModelSlot` retains coordinates even when world tf13 contributes the
sea-band rejection (`ModelSlotClip::SurfaceBand`, retail clip40). Type11 copies
the full parent result through `LinkedModelSlots`; child-frame conversion changes
only coordinates. Ordinary arithmetic generators consume source XYZ and clear
inherited semantic clipping. Tf12 also clears it; tf13 computes fresh rejection
from the new source position. The tf1 screen midpoint instead retains either
source's clip40, alongside its existing
screen-midpoint dependency metadata. Faces retain all four original quad corners
for atomic rejection; edges and billboards also consult resolved slot admission.

Authenticated native draws also retain saved integer VIEW coordinates in the
same dependency walk. Spatial tf2/5/6/7/8/9 consume the completed native caches;
a second recursive evaluator cannot replace world or external callback results.
Missing native surface dependencies keep a diagnostic even when the affected
mesh is empty. Exact arithmetic, coverage bounds and complete-model controls
are owned by [spatial generator custody](MODEL_DRAW_CUSTODY.md#spatial-generator-coverage-and-failure-boundaries).

World tf13 additionally marks `ModelSurfaceOrigin::ViewPin`. Type11 copies it;
arithmetic generators, including tf1, clear it. This is explicit provenance for
the existing GL depth adapter, not an intrinsic asset property or a coordinate
test: a world triangle with any projected endpoint re-establishes `LEQUAL`, and
one with all projected endpoints receives the existing surface-decal policy.
Intrinsic submissions retain the authored-type test, while Raw, Disabled and
CameraFacing keep their distinct policies.

Model351 `grendrag` exposed the linked dependency in its wings.
Model352 `gwing1a` passes remap `[2,4,10,12]` to model353 `gwing2a`; parent
slots10/12 are type-13 endpoints. The child's first `0x84` quad, sprite1767,
uses slots `[8,10,14,12]`: slots8/12 import those endpoints through type11,
while slots10/14 are local type13. Model353 -> 354 repeats the same shape.
Previously imported corners stayed at wing height while local corners projected
onto the surface, producing long dark quads even in default-animation controls.
All four now consume the same callback dependency contract, while their authored
flags remain `[11,13,13,11]`. Real-asset tests cover both descendants, static and
linked entry points, translated/rotated/reflected frames, surface-band rejection
with source XYZ retained, and unchanged intrinsic controls. A normal-tier
production GL run through New Game, the full story and the first-world reveal
confirms that the long strips are gone at the 60/65-second dragon shots. Turret
rings and the 76-second factory wreck smoke remain visible; health/model event
ticks match the prior fixed-40ms run and no runtime step is blocked. Menu,
Klaus-cover, first-village and first-world reveal controls are byte-identical.
This establishes the port repair; matched retail whole-scene acceptance remains
in objective08.

Intrinsic decoding, collision and named emitter/origin queries intentionally
retain the XYZ-copy alias and supply no world presentation callback.
`FUN_0046AF20` installs scratch table `0x004D4A78`; its tf-12 entry
`0x004D4B20` selects `FUN_0046E9F0`, independently of the world render
table. The pen's collision midpoint remains Y=192 even where rendering
lowers the top to Y=128. The separately supported ordinary-fence convex
program is described in [FORMAT_DOCUMENTATION](FORMAT_DOCUMENTATION.md); neither this
render correction nor the user's corrected shooting observation changes
weapon damage admission.

#### Open ground-surface overlay depth gap

A port observation on 2026-08-09 reports position-dependent missing
fragments in both the `player4` ground shadow and the acquired Targetter's
terrain crosshair. Both remain visually complete over water. This is strongly
consistent with the known hidden-surface mismatch: retail queues primitives
through its painter sorter without a z-buffer, while the GL port depth-tests
these surface presentations against terrain as ordinary geometry.

Check source custody before diagnosing raster depth: missing native tf5 VIEW
midpoints can reject `player4`'s entire main footprint before submission. That
separate failure is repaired in [model draw custody](MODEL_DRAW_CUSTODY.md);
`overlay-always` cannot recover geometry that was never submitted.

The two paths reach that mismatch differently:

- `player4` owns six type-13 source records feeding eight authored footprint
  triangles. Their corners use the recovered bilinear world-surface callback,
  while the visible terrain is rasterized as hardware triangles. The surfaces
  can cross between sampled corners. Commit `40d5ba45` added `LEQUAL` plus
  `PolygonOffset(-1, -1)` for all-type-13 world-surface faces; that resolves
  small depth ties but does not guarantee visibility where the two geometric
  surfaces genuinely cross.
- Global model 236 is level-3 local model 223, `crosshair1`: a flat
  twelve-vertex/eight-triangle model with no type-13 records. The Targetter
  draw path places it directly at the recovered signed-8.8 terrain hit and
  applies `FUN_0044DF00`'s terrain-facing basis. That draw now opts into
  `ModelOverlayKind::TerrainSurface` so it shares the type-13 decal depth
  policy instead of remaining an ordinary world mesh that can lose fragments
  against terrain. Entity-locked and near-entity crosshairs stay untagged.

Treat this as a shared renderer-policy gap, not an asset-parser correction or
permission for a global height nudge. Retail evidence, not a guessed lift:

- the software rasterizer has **no z-buffer**. View-space Z is only the
  painter sort key (`FUN_00494930` stable merge-sort, then `FUN_00494A50`
  drain). A later primitive overwrites earlier pixels.
- world type-13 callbacks plant overlay corners **on** the sampled surface
  (`endpoint Y` is the second terrain/wave/seabed sample). They do not add a
  world-Y bias.
- `FUN_0044DF00` builds the Targetter terrain-facing basis from cell-height
  deltas at the recovered hit. It writes a 3×3; it does not raise the
  crosshair.
- therefore retail never "clips into" coplanar ground: the overlay paints
  after terrain at the same sort depth. Hills in front have nearer keys and
  still occlude.

The GL depth buffer is a strict occlusion improvement, but testing overlay
triangles against the hardware terrain mesh reintroduces a test the filler
never ran (bilinear type-13 / cell-hit Y ≠ terrain-triangle interpolation;
the Targetter mesh is a rigid `FUN_0044DF00` plane and extends below those
triangles). `PolygonOffset(1, 1)` is one depth-buffer ULP and does not cover
that cell-scale mismatch (~one Section-10 height byte is hundreds of ULPs at
typical gameplay depth). Live default is `terrain-recede`: the opaque terrain
fill recedes by `PolygonOffset(1, 512)` and classified overlays pull toward
the eye by `PolygonOffset(-1, -512)`. That is a window-Z bias, not a world-Y
lift. Foreground geometry and water stay unoffset. `--overlay-depth-policy
<terrain-recede|decal-offset|overlay-always>`
(`v2k_render::OverlayDepthPolicy`, GL backend only; presentation-only):

- `terrain-recede` (omitted) is the live policy described above.
- `decal-offset` recedes nothing on terrain and only pulls classified overlay
  faces. Use it if a large terrain recede makes hull or feet poke through.
- `overlay-always` is diagnosis only: overlay faces skip the depth test
  (foreground hills can no longer occlude them). Missing fragments under this
  policy would rule out depth as the cause.

Acceptance requires complete, stable `player4` shadow and terrain-crosshair
coverage across slopes and terrain-cell boundaries, unchanged water behavior,
and preserved occlusion by foreground terrain and world geometry. Validate
other authored type-13 footprints as a regression set. Do not use blanket
depth-test suppression, a global polygon offset, or a model/world Y offset;
`terrain-recede` is scoped to the opaque terrain fill precisely so it does not
become that forbidden global offset.

The Level-1 cargo weight illustrates the distinction between projected model
geometry and baked terrain lighting. Model 81 includes a type-13 quad textured
with the 2x2 black sprite 583; flag `0x0C` makes that footprint a fixed-row,
half-destination composite which disappears when attachment hides the model.
Independently, Section 10 authors its spawn vertex `(75,60)` at terrain shade
input 0 and its four direct neighbors at `[3,4,3,4]`. Interpolation leaves that
small ground-darkening mark after pickup. It is not a separate Section-9
object, retained attachment shadow, or reason to apply a global dark filter to
world models.

### Backend vertex-space contract — raw-local inside the draw, world for helpers, screen for 0x02/0x22 fill

The GL model-body path holds **two coordinate spaces**, and mixing them makes
geometry vanish silently instead of failing:

1. **Raw-local** is the currency *inside* `draw_tris_gl`. Authored Section-8
   vertices are raw-local, and every context callback result is re-expressed
   into it before submission: type-14 external-frame world points and
   camera-facing pins go through `world_point_as_raw_local`
   (`world - position` rotated onto the inverse orientation, divided by
   `scale = extra_scale / 100`). Faces reach world space only through the GL
   matrix stack: `world = position + O · (raw · scale)`, row-major
   `world[r] = position[r] + Σ_c O[r][c] · raw[c] · scale`.
2. **World** is the currency of everything computed *manually* against scene
   data: projection against `camera_position`/`camera_basis`, edge-quad
   selection. Those helpers take and return true world coordinates and never
   see the GL matrix.
3. **Screen** is the `0x02`/`0x22` fill: `FUN_0047EF10` / `FUN_00458c60` paint
   constructor pixels. Live GL installs a HUD-style ortho
   (`glOrtho(0, W, H, 0, -1, 1)`, Y-down) and submits `Vertex3f(sx, sy,
   ortho_fill_z)` with window Z matched to the 3D perspective pass
   (`ortho_fill_z = -ndc.z`). Inverting those pixels into view space and
   drawing 3D triangles/lines under the camera projection stretched ribbons
   (`0x22`) and produced full-width cables (`0x02`): a 3D segment between two
   different depths is not the constructed screen segment.

Any new consumer of resolved mesh vertices must state which space it expects
and convert explicitly at the call site with that formula. Feeding raw-local
vertices into a world-space projector leaves faces intact (they still ride the
GL stack) while the new feature drops everything behind the eye plane. The
failure signature is distinctive: headless tests pass because their fixtures
hand the helper real world coordinates; live, `endpoints_ok` counts stay high
while `projected ≈ 0` or clip drops consume every record, with no panic. The
2026-08-25 `V2000_EDGE_DIAG` capture showed exactly this across all creature
draws while spider/newant/stag legs were invisible.

### Model edge sprite quads (`0x02`/`0x22`) — near-pass construction recovered

Retail draws authored leg/edge streams as tapered screen-space quads, one
queued primitive per stream record. The near-pass constructor `FUN_00459000`
(opcode `0x22`, A-pass; B/far variants share the shape) is recovered:

1. Resolve both slot operands through the **near** transform table
   (`ctx+0x3C`). Slot entries already carry post-projection screen `x/y` as
   **i16 pixels** plus an **8.8 raw** depth dword. `FUN_0046CD90` sets clip
   bit `0x40` when that raw depth is `< 0x40`; `FUN_00459000` then drops the
   whole record.
2. `dx,dy` = endpoint screen delta; `len = FUN_00457730(dx*dx+dy*dy)+1`
   (integer square root, 0 for inputs `< 1`); direction unit
   `(dx<<15)/len, (dy<<15)/len` in Q15.
3. The first stream word resolves through the context `+0x1A` callback to a
   sprite-style record whose `+0x10/+0x12` u16 fields are the authored
   **start/end widths**. Tip shift along the edge is
   `t = (wEnd*len) / ((wStart-wEnd)*2)`, applied as `t*dir >> 15` on both
   quad ends: start corners move by `-t·dir` and end corners by `+t·dir`, so
   a tip thinner than the base extends the leg outward past its endpoints,
   while a wider tip pulls it inward. Equal widths divide by zero and drop
   the record.
4. `FUN_004594C0(widthOperand, midDepth)` projects the authored width into
   two integer screen half-width shorts: overflow-guarded
   `(value * DAT_004feed0|feed4) / depth` (operand order swaps when
   `|value>>12| > |depth|`), then both components halve while their OR
   reaches `0x2000`. For opcode `0x22`, `widthOperand` is the **raw stream
   short** at `param_2[1]` (`(int)(short)size`); unlike billboard size/angle
   this is not `FUN_00470840` packed-decoded. factory6 authors 45 — packed
   decode explodes that to 2048 and paints sky-high roof ribbons. Retail writes those two shorts only; Ghidra's
   `extraout_var` / `CONCAT22` around the call is 16-bit screen-coord
   reconstruction, not a shade byte. Live first-world draw now uses the same
   `with_sub_h_presentation` writeback as Intro2 (`6ECF0 → D350 → A9F0 →
   1D360`). A read-only `resolve_selector_world_points` copy leaves constructor
   flags 0, so `FUN_0041D0A0` never starts a phase: `12DA0` still translates
   the thorax while type-14 `0x22` legs stay glued to the rest pose. D360's
   `0x02`/`0x08` bits alias COPY_TARGET / WAIT_FOR_DEPS. Remaining insect /
   factory presentation leftovers: matched-retail whole-world gait timing,
   newant type-13 WorldSurface blobs versus type-14 legs, Gouraud
   `factory2pillar`, wreck model 225. Do not invent a Type17 whitelist or a
   second insect constructor, and do not packed-decode `0x22` size to keep
   factory roofs. The perpendicular corner offsets are
   `(-projX*dirY >> 15, +projY*dirX >> 15)` around each endpoint; average
   endpoint depth (8.8 raw) scales the width projection; the first endpoint
   depth keys the native sorted queue record. When that midpoint is
   `<= 0x3F` the width projection is skipped and the `perp == 0 → 1`
   hairline guards still produce a 2px ribbon.
5. Four corners are truncated to i16 and outcoded against `DAT_004feed8`
   (logical scene width) / `DAT_004feedc` (height). Per-axis bits are X: 1
   left, 2 inside (`(uint)(int)(short)x < width`), 4 right; Y: 8 top, `0x10`
   inside, `0x20` bottom. The four corner codes are OR'd and looked up in
   `DAT_004c5268` (VA `0x004C5268`, 256 bytes). **Non-zero queues** the
   primitive — fully inside (`0x12`) and straddling records submit;
   unanimous left/right/top/bottom drop; bit `0x40` is always reject and
   bit `0x80` duplicates the low 128. Software fill clips partial records.
   Survivors queue `{start z sort key, callback FUN_0047AA20, eight corner
   shorts in payload order A,C,D,B, resolved sprite pointer, zero}`.

The shared 0x02/0x22 endpoint projector retains ordinary `46CD90/46CEB0`
stabilization for native, intrinsic and Free projection. It latches signed
`abs(X)|abs(Y)` once, then arithmetic-halves both screen coordinates while
that control is above 8191. Underwater displacement follows the one cap;
all underwater compatibility paths retain it to match the GL vertex shader.
Completed pixel-ortho ribbons and hairlines bypass that shader projector,
with the scene setting restored afterward. Free retains its supplied float
camera/lens and never receives a manufactured Q31 viewport. Source-owned
world model endpoints use the raw path below.

Dry `Compatibility(LogicalViewportAdapter)` instead projects its edges with
the same uncapped float perspective as its GL model bodies. Applying the
retail cap directly in an enlarged logical viewport is not a source-pixel
resize: adjacent offscreen endpoints can cross the 8192-pixel threshold at
different times, then the tapered constructor extends one end back into the
viewport. A reproduced 3840x2160 Native Intro2 frame at 39.6 seconds had
sprite 680 endpoints near `(8377,3487)` and `(8095,3286)`; halving only the
first produced a detached sky ribbon beginning at `(3535,1494)`. Both direct
compatibility edges and native-receipt adapter fallbacks carry the explicit
scene authority through the same projector. Integer truncation, raw-depth
near admission, width construction, i16 overflow guards, outcode LUT and
pixel-ortho filling remain shared. This adapter change is not a change to
`NativeScreenProjection` or the original-resolution retail cap.
Matched GL frames also remove the stray strips during the hive approach at
15.0–15.4 seconds. Before/after simulation state remains identical; sampled
640x480 native-source intro and menu pixels remain identical as well.

The manual projector must reproduce the off-centre GL matrix as well as its
scale. For a top-origin logical viewport, retained terms
`[m00,m11,off_x,off_y]` become
`scale=(m00*W/2,m11*H/2)` and
`center=(W/2*(1-off_x), H/2*(1+off_y))`. The signs follow from the offsets
occupying the projection matrix's Z column while visible eye Z is negative.
Using the opposite signs detached `multipc`'s ribbons by twice the menu camera
offset (140 logical pixels vertically in the high-resolution layout), even
though the model body itself remained correctly placed.

`FUN_0047AA20` thunks to device slot `+0x1098`. `FUN_00480D10` installs
`FUN_0047EF10` there on the 16-bpp software table (`piVar3[2]==0`). That
handler is the port's textured-quad policy for 0x78 billboards and 0x83
faces: implicit UVs `(0,0)`, `(Umax,0)`, `(Umax,Vmax)`, `(0,Vmax)` on
payload A,C,D,B, which on stored corners A,B,C,D is A `(0,0)`, B `(0,1)`,
C `(1,0)`, D `(1,1)`, triangulated as A-C-D and A-D-B (`[[0,2,3],[0,3,1]]`),
with Section-3 blend flags `0x01` key / `0x04` row 28 / `0x08` half-additive
/ `0x10` additive.

The fogged B constructor `FUN_00459550` retains the two endpoint fade bytes
separately: start is copied to stored corners A/B and end to C/D (payload
A,C,D,B therefore carries start/end/end/start). It rejects the edge only when
**both** bytes are `0xFF`. `FUN_0047F060` interpolates those bytes across the
screen fill and applies the material-specific palette/fog composition
[described below](#material-specific-model-fog). For the frontend the terminal
color is Section-7 entry 55, pure red. The port
therefore transports the quantized endpoint values as explicit fog
coordinates in pixel ortho. The GL adapter uses midpoint window-Z, while
native sorted A22/B22 records use the first endpoint depth. Neither depth
can stand in for this lengthwise gradient. Native terrain queue keys also
retain family-specific scan-depth biases: ordinary ground `30430` adds
`0x200`, while water/infected `327C0` adds `0x180`. Both feed the same
viewport root queue (`431890` copies its pointer to terrain context +20).
Substituting the first endpoint as GL Z alone does not prove painter equivalence.
Frontend `ModelDepthFade` submissions retain the original plane integers and
the explicit 100-source-units-per-view conversion. Endpoint quantization then
uses retail's reciprocal-first form
`((z-near)*floor(0x1000000/(far-near)))>>16`; an ideal floating ratio is one
byte too bright at common band midpoints (`128` instead of retail `127`).
The palette B:02 handler `FUN_00458e20` carries the same two endpoint bytes
but has no both-`0xFF` rejection; a fully faded hairline still rasterizes in
the terminal color.

The port implements the construction and `DAT_004c5268` clip in
`v2k-render::edge_quads`. Live GL fill paints the constructor's screen-space
corners in a HUD-style pixel ortho with window Z matched to the 3D
perspective pass: implicit UVs, A-C-D / A-D-B split, Section-3 blend.
Feeding those corners back through the entity modelview as world triangles
stretched the ribbons; inverting them into view space and drawing under the
camera projection is still a 3D fill, not `FUN_0047EF10`. `V2000_EDGE_DIAG`
still accounts the selector. Palette-style `0x02` hairlines use A-pass
`FUN_00458c60`: two near-table endpoints, the same `DAT_004c5268` LUT on
their combined outcode, and `LAB_0047a740` which thunks to device `+0x1024`
(`LAB_0047b360` on the 16-bpp table). Live GL draws those as pixel-ortho
`GL_LINES`; 3D view-space lines produced full-width cables (a 3D segment
between two depths is not the constructed screen segment).
`V2000_EDGE_FILL_DIAG` / `V2000_EDGE_DIAG` helpers remain in
`edge_quads`; the live GL path no longer prints per-quad or per-vertex
fill dumps.
Unresolved sprite widths stay unsubmitted.
Type-14 selector resolution retains authoritative world endpoints.
For owned native draws, the `40D350` suffix rebases each callback WORD
around the inherited viewport dword before `6ECF0` projects it; see
[the exact native custody below](MODEL_DRAW_CUSTODY.md#authored-insect-body-and-leg-shadows).
Free/intrinsic compatibility still exports signed 8.8 endpoints as
`draw_origin + (short)(endpoint_raw - actor_origin_raw) / 256`, not as
absolute `i16 / 256`: world X/Z ≥ 128 overflow signed 8.8, and that
absolute f32 lands one world period from type-0 hips so hip-to-foot
`0x02` lines span the framebuffer.

Two live-only defects were found and fixed on 2026-08-25, both invisible to
headless tests whose fixtures already held the correct domains:

1. the call site fed raw-local vertices to the world-space selector (see the
   vertex-space contract section above), so every record died behind the eye
   plane; and
2. the width projection divided the `FUN_004594C0` size operand by the
   port's float world depth, while retail's near-table depth — and therefore
   this division — is 8.8 raw (world × 256). Legs projected 256× too wide and
   stretched across the frame. The port's depth channel is now raw 8.8
   end-to-end, with retail's `0x3F` raw near guard (degenerate 1px hairline)
   and the `0x2000` halving cap reproduced.

This change also closes the AABB "any corner outside drops" clip (retail's
LUT queues straddling records). Classic Framebuffer, external-frame, and
viewport were a misdiagnosis of those vertex-space and depth-domain bugs, not
a separate invisibility cause. Live fill now matches `FUN_0047EF10` /
`FUN_00458c60` as a 2D painter. Live Level-1 newant dumps submit the full
20-quad/14-line stream with insect-scale spans; those legs were visually
accepted. Spider/stag share the same path.

#### Native endpoint custody and controls

Command-time VIEW/clip packets, native screen projection, callback admission,
compatibility modes and focused controls are owned by
[shared model draw custody](MODEL_DRAW_CUSTODY.md#native-endpoint-custody-and-controls).

### Authored insect body and leg shadows

Authored newant/spider/stag shadow sources, native callback VIEW caches and
surface projection are owned by
[shared model draw custody](MODEL_DRAW_CUSTODY.md#authored-insect-body-and-leg-shadows).

### Section-3 textured-surface operations

The low byte at sprite record +0x04 controls independent coverage, shade, and
framebuffer policies used by ordinary model faces, billboards, particles, and
HUD layers:

- `0x01`: palette index zero is a transparent key. With it clear, zero is
  authored opaque surface colour.
- `0x04`: the unlit filler advances the palette pointer by `0x380`, selecting
  fixed row 28. Without the flag it retains the palette base (row 0 for
  4-bit texels). Uniformly lit and Gouraud fillers use Section-6 rows instead.
- `0x08`: composite `source + destination/2`. In GL this is `ONE,
  ONE_MINUS_SRC_ALPHA` with source alpha exactly `1/2`, not conventional alpha.
- `0x10`: additive `source + destination` (`ONE, ONE`).

The palette relocation at `004ABC81..004ABC8B` retains the original palette
offset. The flat filler `FUN_004782B0` and masked/additive siblings add only
the flag-controlled `0x380`; there is no implicit brightest-row bias. Direct
palettes with more than 16 colours retain the full-byte texel lookup. HUD,
radar and full-frame sprite consumers share this base/row-28 policy.

Palette-zero coverage and fixed-row selection are independent of a face's
lighting family. In particular, bright Hover/Main Base surfaces were caused
by decoding the `C3/C4/C7/C8` family as unlit; flag `0x04` then incorrectly
forced row 28 instead of the authored normal's Section-6 row. The row-0
correction alone does not explain those surfaces, whose sprites carry `0x04`.

The primitive supplies implicit UVs. `FUN_0047C600` assigns a textured
triangle `(0,0)`, `(Umax,0)`, `(Umax,Vmax)`, i.e. the upper/right half of the
sprite rectangle. `FUN_0047CBE0` is another triangle handler, not a quad
extension. The textured quad handlers `FUN_0047EF10`, `FUN_0047F320`, and
`FUN_0047F750` synthesize the ordinary full rectangle
`(0,0)`, `(Umax,0)`, `(Umax,Vmax)`, `(0,Vmax)`. Level-1 `pesnthut` uses
non-keyed sprite 1365 for all 12 roof faces: its unused half is authored
palette-zero magenta, while its upper/right half contains the thatch. Sampling
the opposite half therefore produces bright purple roof wedges rather than a
missing-resource fallback.

Mirror-family model opcodes submit A/B/C/D, then A^1/B^1/C^1/D^1, in the same
corner order and with the same implicit UVs. Do not reverse refs or texture
coordinates in the parser. The frontend's local-X presentation basis is
applied at menu hierarchy roots so directional Save/Exit art remains readable
while Klaus's keyed wing masks stay on their authored joints. Billboard,
terrain, water, and UI texture coordinates retain their own mappings.

### Steep-camera coverage and the former type-13 strips

A 2026-07-17 follow-up audit found no second shadow-length policy to port.
`FUN_0042F270` and `FUN_00431890` independently compute the same signed row
lead from camera basis word 12: add `0x58000000`, and on a negative result
multiply by `0x1c00`, shift by 31, then add `0x200`. This is the retail
behind-eye coverage rule used before both the terrain and water traversals.
The port evaluates that one policy for terrain, water, and static terrain
objects; it is intentionally discontinuous and must not be replaced by an
extra fixed rear buffer.

Only that signed lead and the authored 52-by-30 scan dimensions are literal
retail policy. The software loops walk an axis-aligned map rectangle outward
from raw camera X and from camera Z plus the lead. The GL port instead filters
a yaw-rotated horizontal rectangle, then expands its hidden rows/width through
the terminal fog plane and widescreen field of view. Those two envelope choices
are compatibility reconstructions, not additional facts recovered from the C.
If a post-fix pose still exposes clear colour, instrument that rotated/overscan
envelope; do not alter the proven pitch lead.

#### Native near row and bottom cap

Opaque terrain `2F980` walks X columns outward in both directions while
`2FCC0` builds an axis-aligned +Z row. For N=`DAT_004CAB74` projected
vertices, its first `30140` receives `floor(cameraZ + lead)` plus the raw
fraction f; interior records use whole-cell Z. The final `30140` receives
`base + (N-1)*0x100` with offset `f-0x100`, so the last point is
`cameraZ + lead + (N-2)*0x100`. The helper bilinearly samples the fractional
height; `30430` also interpolates the first/last strip's UVs. Full-cell
centre admission is not this boundary geometry.

Every first-row X pair also reaches `31970` before the detailed terrain
primitive. If `DAT_004C5268[(clip0 | clip1) | 0x20]` is nonzero, it queues
`[x0,screenHeight], [x1,screenHeight], [x1,y1], [x0,y0]` at signed key0,
with the first dword of system-2 Section-7 entry11 as its flat colour.
`47A960` dispatches device slot+1080, installed as the solid `47E150`
quad filler in the 16-bpp table. `2FCC0/30140` reproject terrain vertices
below the source near depth through `2FFF0` at rawZ0x40, retaining its
screen outcodes. This cap therefore requires the source row, active
projector, logical screen height, palette and enclosing queue; it is not
a raised terrain mesh or a shadow-only depth override.

A current Level13/spawn11 step33/view08 port witness exposes background
at pixel500,420 over reference cell181,126 (ray depth1.55173, rejected
cell-centre along1.78906). Its cameraZ128.28906 and two-cell lead put the
literal native first row atZ130.28906; the diagnostic view looks toward-Z.
The reference is constructor terrain rather than the live infection-moved
mesh. These facts establish an omitted GL coverage cell, not native cell
visibility or the cap's final pixel colour. The port's rotated envelope and
whole-cell centre test omit both the fractional strip and this cap.
Conservative cell intersection would be another compatibility envelope;
it does not recover `31970`. Recover the shared fractional-row/cap owner
with authenticated projection, palette and queue inputs before claiming
retail near-ground coverage. Do not fit a row lead, add a terrain/shadow
bias, or borrow a captured flat colour.

The bounded original-PE producer
`execute-native-terrain-strips.py`
now executes 22 controls, with 20 focused
`test-native-terrain-strips.py`
checks. It runs `2FCC0 -> 30140 -> 46D1E0/436440`, the dry `2FFF0`
near reprojection, source `33180` canonicalization, `30430` texture packets
and `31970` cap allocation. Three controls additionally execute the whole
`2F980 -> 31890/33530` scan from explicit retained process state. They
retain negative-X-first allocation, positive-X reset, clipped-row retirement,
the Q31 negative lead, native cap-before-strip order, first/last fixed UVs,
fog payload variants, and infected overlays. An explicit existing arena
cursor is preserved; ground keys consume the unsigned strip-depth word
plus `0x200`, not a signed first-corner depth or GL midpoint window-Z.

These are controlled complete Section10/sprite-metadata/viewport/light/
palette/arena inputs executing the verified retail PE. They are not a new
native frame capture or an implemented live terrain consumer. The native
infection refresh branch calls `457930`; the oracle rejects that call
without its process owner instead of replacing it. The current scene
bridge owns the six raw lens words, while raw viewport/fog/terrain state
still need publication to the terrain consumer. `2F270` background/sky/
static allocations, water, original filler coverage and the shared model/
particle/terrain queue remain separate owners. Do not convert the packet
proof into a cap-last overlay or claim the witness pixel has been repaired.

The long dark strips previously described as broken shadows were model faces
whose type-13 corners were routed through the default absolute-view-Y callback
instead of the world-surface callback family. Mixed and all-type-13 faces are
authored geometry, not a separate generated cast-shadow mesh. The world path
now preserves those faces and resolves each type-13 corner onto terrain, waves,
or seabed using the fixed Section-10 direction described above.

Visual acceptance still needs a new port capture paired with the same retail
camera pose. A clear-colour wedge at a screen edge reopens world-scan coverage;
a strip attached to one model reopens world callback selection or face data.
Neither case justifies a camera-relative shadow direction, a generic stretch
clamp, or another model-name exception.

## 3. Texture mapping = AFFINE (perspective-INCORRECT)

This section describes the audited software fill path; the alternate
consumer's interpolation/render state requires separate evidence.

Textured span filler **0x478D00** divides the U/V span deltas **once** per
scanline via a reciprocal-of-width table, then steps with constant per-pixel
adds — **no per-pixel `1/z`**:
```asm
shr eax,0x10 ; sub edx,edi               ; edx = span pixel width (screen space)
mov edx,[edx*4+0x4d4b98]                  ; recip(width) = 1/width (fixed point)
...                                        ; per-pixel step = (Uend-Ustart)*recip
mov eax,[ebp-0x1c]; add eax,[ebp-0x28]    ; U += dU/dx   (constant add)
mov ebx,[ebp-0x18]; add ebx,[ebp-0x4]     ; V += dV/dx
```
For this software span filler, view-space Z (vertex slot `+0x08`) feeds
the depth sort key rather than per-pixel correction, producing affine
"texture swim" on large/near polygons. Matching that software contract
requires affine interpolation (for example linear UV or `q=1`), rather than
GL's default perspective correction. This does not establish the observed
alternate/Direct3D consumer's interpolation state or change port defaults.

## 4. Filtering = POINT SAMPLING (no bilinear)

The audited software fillers fetch one texel per pixel —
`mov cl,[eax+ecx*1]` — without a four-tap average. This establishes point
sampling for those software tables, not alternate/Direct3D render state.
The shaded and fogged indexed fillers do dither, though: they add generator
noise to the palette shade row and the fog level per pixel (see
[SOFTWARE_RASTER.md](SOFTWARE_RASTER.md#span-rows-and-fillers)).

The source audit of the "Bilinear Filtering" setting global `0x4CB3F0`
(toggle handler `FUN_0043CCA0`) found its reference only in the menu handler;
the audited software fill path does not read it or implement a four-tap
filler. Keep that software contract point sampled. The shipped alternate
consumer is now independently observed, but its filtering, dither and
antialias state were not sampled. Neither the menu label nor software filler
findings establish those states or a global dead-toggle claim.

## 5. Unlit, uniformly lit and Gouraud face shading

- **Indexed textured-lit**: a Section-6 shade selects an authored texture
  palette row before the colour lookup. Uniformly lit faces retain one shade;
  Gouraud faces interpolate the corner shades. The far family additionally
  reduces that shade using the projected fade byte before lookup, as described
  under [material-specific fog](#material-specific-model-fog).
- **Untextured** (0x474220): true **per-channel RGB Gouraud** — setup
  `FUN_0047C150` initializes the Section-6 corner RGB accumulators; span
  `FUN_00474220` computes per-pixel gradients and packs the interpolated
  channels directly. `0047E150` is an unlit quad setup, not the Gouraud
  initializer.
- **Uniformly lit** faces carry one Section-6 value from their face normal;
  it is constant across the polygon. This family is distinct from unlit faces
  despite having no trailing corner-normal operands.

| Lighting family | Solid opcodes | Textured opcodes | Shade source |
|---|---|---|---|
| Unlit (`Flat`) | `03/04/07/08` | `83/84/87/88` | Section-7 colour or fixed sprite palette; header normal is for culling |
| Uniform (`FlatLit`) | `43/44/47/48` | `C3/C4/C7/C8` | One Section-6 dword from the header normal |
| Gouraud | `23/24/27/28` | `A3/A4/A7/A8` | Section-6 dword per trailing `s0..s3` normal |

For example, `C3`'s near handler `FUN_0045A9C0` queues thunk `0047A8E0`
and a normal-cache Section-6 dword at payload `+0x10`. The thunk reaches
`FUN_0047CBE0`; `0047CC37` copies that dword to raster context `+0x18`.
Opaque `FUN_004783E0` and masked `FUN_00478A20` then fetch
`palette + shade_byte(ctx+0x1B)*0x20 + raw_texel*2`, ignoring sprite flag
`0x04`. By contrast `83` queues `0047A8A0` without that shade payload.
Solid `43/44` use the same dword's RGB bytes. Mirrored primitives resolve the
header normal reference with XOR 1, just as their mirrored cull plane does.

The solid filler `FUN_00473C00` **adds**, rather than multiplies, a packed
light contribution to the relocated Section-7 word. The accepted DirectDraw
framebuffer uses RGB565. `FUN_004AA2E0` relocates RGB555 base `p` to
`((p & 0x7FE0) << 1) | (p & 0x1F)`; `FUN_004A8920` installs filler shifts
7/2/4. The light word is `((r & 0xF0) << 7) | ((g & 0xF0) << 2) |
((b & 0xF0) >> 4)`, followed by **u16 wrapping addition**. Thus Section-6
RGB 128 contributes RGB 64; a black base can still produce a lit gun part.
Retain packed carries (including green's sixth bit), then decode R5/G6/B5
into the high bits of RGB8 and apply any explicit scene tint. `FaceMaterial`
retains the original palette word so this operation never reconstructs source
bytes from a tinted float colour. Gouraud solids instead use the separate
[corner accumulator and span policy](#gouraud-solid-colour-composition).

Normal-tier `player4`, `college`, `factory2` and `lifter` contain these
uniformly lit surfaces. `college` is entirely in that family; the craft and
factory also retain genuinely unlit details. A global texture tint cannot
preserve this distinction.
Tree assets also differ: `bigtree3`'s sprite 1426 is uniformly lit, while
`bigtree1`/`bigtree2` sprites 1402/1406 and shared support sprite 1448 use the
unlit family. Do not darken all trunks to imitate a different tree or scene.

Span-filler tables: `0x4D55A0` (indexed texels) / `0x4D5900` (raw 16-bit
texels), via `PTR_PTR_004D5C60/64`. Sprite flag `0x02` selects the raw-word
texture family; these are not flat-versus-Gouraud or filtering tables.
Solid fillers occupy common slots in both tables.

### Model light table

The local menu-prop context (`FUN_0043AA40`) and ordinary world context
(`FUN_00433BD0`) construct the same 16-slot Section-6 lookup by resource id:

```
[0, 1, 2, 3, 4, 5, 6, 7, 0, 0, 0, 0, 0, 0, 0, 0]
```

Their light vectors differ, and every one is a VIEW-space vector. The local
contexts use `(0x49, 0x49, -0x49)`. The world context's vector is world
descriptor `+0x50/+0x54/+0x58`, which `FUN_0042EA30` fills from the Section-10
direction with X and Z divided by four, rounding toward zero: Level 1's
`(-73,73,-73)` becomes `(-18,73,-18)`. `FUN_00433BD0` copies the descriptor
to `DAT_004FEC40`, and `FUN_00433FA0` installs `DAT_004FEC90..98`,
unrotated, as the view context's `+0x3C..+0x44` every frame. The frontend
Klaus world descriptor `DAT_004CA838` supplies `(-100, 50, -50)` through the
same path; it must not be conflated with the local ring-prop context.

`FUN_00466160` forms the model-space light as the node's VIEW axes times the
context vector, and those axes are the camera rows times the model basis
(`FUN_00465870`). `FUN_0046D3F0` then selects the table with
`((Lx*nx + Ly*ny + Lz*nz) >> 19) & 0x0F`, so the dot is effectively taken
between the VIEW-space normal and the context vector: model lighting is fixed
to the camera, not to the world. Local contexts have identity camera rows, so
their VIEW frame is the context's own. A time-travel replay of a retail
Alpine session shows the world view context holding `(-18,73,-18)` across
frames whose camera rows differ. On the authored signed-16 unit-normal scale
the bin is `floor(dot(n_view, L) * 32767 / 2^19)`. Positive bins select
Section-6 entries 0..7; every negative bin maps back to entry 0 through slots
8..15. Each entry contains an RGB triplet used by the
untextured lit paths and a 0..31 Section-3 palette row used by the indexed
textured path.

`FUN_0046D5A0` initializes reserved normal-cache references 0/1 from the
current table's slot 0. A stored zero normal also resolves slot 0 through
`FUN_0046D3F0`. Preserve that zero direction through parser and model
orientation; substituting an upward normal incorrectly selects a brighter bin.

World model draws shift that table contextually. `FUN_004136C0` and
`FUN_004138F0` sample the signed scrolling 32×32 terrain-light byte at the
entity cell and, when terrain is active, subtract `FUN_004336E0`'s exact 0..8
underwater-darkness step. `FUN_0042F650` performs the corresponding darkness
shift for static terrain-object models. For each conceptual signed slot -8..7,
retail then selects `clamp(slot + shift, -8, 7)` from the base table. This
changes both uniformly lit and Gouraud Section-6 lookup, including slot 0;
only the explicit unlit families ignore it. The port carries the same signed
shift on each world `ModelMesh`, while
menu and viewer submissions retain zero.

### Terrain shade input

`FUN_00430140` establishes the terrain vertex's 0–7 input before the textured
filler interpolates it:

```
shade = clamp((terrain_type >> 5) + light_window[x][z]
              - underwater_darkness, 0, 7)
```

`light_window` is the signed camera-relative 32×32 byte buffer at `0x4FE820`.
The field-6 callback `530D0` begins the active world frame by clearing all
0x400 bytes. Presentation callback `53570` then calls
`FUN_004383F0(camera_x_raw, camera_z_raw + 0x0A00)`, using signed 8.8
truncation toward zero to place the window ten cells ahead of the camera.
The actual retail array `4D07A8` is `42AA40,53570,53410,53760,53A60,536A0,
53710,42AA80,42AA90,53AA0,42AAB0,0`. The identical retail EXEs retain SHA-256
`E9BE7A833612FBA3A5A5AB92A974ECE1A689E4B7E72409D9EE8331380573B4BA`.

`53760 ->11720 ->11400` first submits each actor's model and then drains that
actor's shot FIFO through `14870`. Ordinary actor `138F0` therefore samples
the pre-particle light field. Next, `53A60 ->3DCB0 ->3DCD0/3DDC0` traverses
the particle draw callbacks. New shots enter this same presentation at their
birth position and age; their first physical movement waits for the next
`40120`. `FUN_0043D410` selects the current frame, derives its positive
height-scaled or negative age-faded radius, and calls `FUN_004385E0`.
Only afterward does `536A0` dispatch terrain and static-object consumers,
including `2F650`, which see the accumulated particle light. The earlier
claim that all model consumers follow particle accumulation was too broad.

The port must evaluate the destructive particle gate once after the actor
FIFO suffix, accumulate into the existing positioned field without another
clear, and share those owned survivors with later sprite submission. A
second preparation would repeat copied D300/442240 samples and lifetime
decisions. Final-card `world+28A` suppresses both ordinary actor and particle
presentation through the native `53760/53A60` guards. `FUN_00441420` and
`FUN_00441610` provide the other signed saturated writers. Underwater
darkness uses the Section 10 words at terrain context `+0x36/+0x38`,
subtracting up to eight before the clamp. This is independent of the
low-three-bit corner material code and disproves the port's former
height-selected palette/LOD heuristic.

### Copied projectile draw records

Shared `442240` is authored by particle classes `52,53,68,69,87,89`; it is
not a Type47-specific effect. It calls D410 once on the live record, then six
times on a stack copy made before that first call. Normalize the signed
velocity words through `457730`, narrow its result to signed AX before IDIV,
and arithmetic-shift each signed Q12 product by12 after multiplying by25.
Zero speed uses `[25,0,0]`. The samples visit source offsets
`[0,-1,-2,-3,-4,-5,-8] * step`, with descriptor `+0A` scale divisors
`[1,2,2,4,4,8,8]`. Positions wrap as signed words; copies keep the same age,
frame, physical source slot and provenance, and create no particles or RNG
draws. Each hidden copy leaves the live allocation alone, and later copies
still run. All six authored descriptors have the hidden-retention flag2.
Class87's frame triples are `[840,0,256],[841,0,256],[842,0,256]`, so its
separate descriptor scale512 shrinks to256/128/64 while its light word stays0.

D410's size variation derives `((record_pointer >>5)&15)` from the actual
record pointer. The six backward copies therefore share a stack phase rather
than the live pool-slot phase. `PreparedParticle` records that address
authority, and `native_effective_draw_scale_raw` returns an explicit
`ParticleStackSizeJitterBoundary` when a copied descriptor has nonzero jitter
without a supplied captured address. The compatibility draw adapter then
uses only the proven unadjusted scale, which is not retail-matched size.
The six forward-D300 descriptors have zero jitter and need no stack phase;
backward class89 also has zero jitter. Earlier sampled particle-gate/enemy-AI
actor observations did not retain a442240/D410 entry ESP or stack-record
address; their render-context/pool pointers cannot supply it. Four later
[indexed original invocations](TYPE47_RUNTIME.md#model-and-shot-presentation)
now retain entry ESP001AFE30, copied address001AFE10 and the native caller
chain for class87 ticks902/946/2340 and sibling class68 tick1402. Their copied
phase is zero. Balanced source argument cleanup retains that local address
through the pool scan and its six applicable descriptor classes.

Those observations close address authority for these invocations, not the
absolute startup/thread/outer-context phase of every retail run. The
production default therefore retains its explicit copied-size compatibility
boundary; no captured phase, host stack address or fitted visual phase is
substituted. Controlled PE oracle addresses still prove arithmetic only for
a supplied phase. The scoped native query and its14 guards own the actual
observations, independently of the arithmetic oracle below.

The portable original-PE arithmetic oracle
executes only verified retail `442240` and `457730` bytes in private strict
memory; D410 is a passive argument sink, so this proves sample position,
scale, copy-before-live ordering and relative `entryESP-20h` address only.
Its focused tests
cover all six authored descriptors, signed-word wrapping, signed AX before
IDIV, sum-of-squares overflow, zero velocity, live deletion isolation and
fail-closed memory/control/step guards. Run both with `--exe retail/V2000.EXE`.
No retail process is launched. Controlled oracle ESP does not authenticate
the real run's phase: page-aligned OS stack allocation alone does not fix
the intervening CRT, render traversal and callback frames modulo512.

`FUN_0040FCB0` is a separate one-frame uniform writer. It obtains the current
wrapped 32x32 window from `FUN_00438790` and adds `0x10` to every signed byte,
saturating only at `0x7f`. `FUN_00453410` consumes session byte `+0x293` after
the camera-relative window has been established, applies the writer once, and
clears the byte before later particle and world consumers. The sole recovered
setter at `0x00456740` belongs to an unidentified resource-state callback
reached from `0x004078C0`; its exact resource/actor meaning is not yet known.
Do not attach this flash to generic player damage, explosions, or meteors until
that owner is captured.

Session byte `+0x295` is a separate viewport-sized sprite flash owned by
`FUN_00453410`, not another terrain-light writer. `FUN_00456750` starts or
restarts its one-based cursor. The exact retail/demo table is global sprites
`0x227, 0x228, 0x228, 0x228, 0x229, 0x22A, 0x22B, 0x22C`, followed by a null
sentinel; one successfully appended command advances one entry. Selected-tier
sprites are 2x2 fixed-row-28 materials: `0x227` is masked (`0x04`) and the
remaining ids are additive (`0x14`).

The mode pump calls simulation `493FA0` before presentation `493F50`.
Descriptor `4D0918` uses presentation array `4D07A8`, whose first callbacks
are `42AA40,53570,53410,53760`. A simulation setter can therefore supply
the first entry in that same frame. The command carries signed queue key
zero: positive-depth world effects and Klaus's enclosing model group drain
first, then this full-viewport material, with captions and HUD in later passes.
Intro2 stages after contact/follow and before actor presentation, then draws
above world/Klaus and below its overlays. The legacy Playing adapter still
stages before simulation, delaying its producers by one frame; correcting
that separate host boundary remains open. The sequence is presentation-only
and does not gate campaign progression.

The same routine owns a destructive pre-light presentation gate. A null frame
list deletes immediately. Projected centres with no outside/behind clip bits
continue even beyond the context far word, allowing the documented light-only
case; clipped centres apply the behind, far, and extreme-screen rejection, with
descriptor flag `0x02` retaining a hidden record. Sprite depth and expanded
screen-rectangle overlap are tested later. The port evaluates this gate once
per frame and shares its owned survivors between light accumulation and sprite
submission. Stable visible Intro2 and Level 1 both use the captured far word
`0x1800`; `0x1000` belongs only to the transient menu/pre-handoff context. Its
lifecycle/clip policy is exact; the current centre projection still uses the
port floating camera. The retained camera/model VIEW words do not by
themselves implement the particle centre's exact native projector.

Visible particle billboards enter that same stable painter queue with the
signed key `projected_depth_raw + *(i16 *)(descriptor + 0x12)`. Masked,
half-additive, and additive records are not split into material-first buckets;
priority/intrusive-list traversal survives only as the tie order for equal
keys. The port carries this raw key on `WorldSprite` and applies one stable
descending sort before changing per-record blend/depth-write state.

The canonical terrain sprites themselves contain normal 16-step palette
ramps. Representative 40×40 frames from the Medieval, Alpine, and Alien biome
sets all brighten monotonically from shade 0 through 15; they are not flat
colour masks driven solely by Section 7. The terrain emitter consumes inputs
0..7 from that authored ramp. The port now keeps the raw indexed texels and
per-frame palette rows in point-sampled textures; its GL 2.1 shader interpolates
that 0..7 shade across the polygon before the palette lookup. It also cancels
GL perspective correction with projective varyings, matching the software
filler's screen-linear UV and shade stepping. The old scalar luminance path is
retained only as a shader-unavailable fallback.

### Depth fade / fog byte

`FUN_00470A10(near, far)` stores the two 8.8 depth planes and computes
`0x1000000 / (far - near)`. Projection helper `FUN_0042FFF0` then writes each
terrain vertex's fade byte as 0 before `near`, 255 at/after `far`, and the
linear 0..255 interpolation between them. The terrain emitter selects its
fog-capable primitive filler whenever any corner has a nonzero fade byte.
This establishes a linear depth factor, not one universal RGB blend. Equal
`0x1E00, 0x1E00` local menu planes belong to `FUN_0043B130`'s screen-model
path, not to every menu model. For each flag-`0x02` prop-row callback,
`FUN_0043AA40` overwrites the local pair with
`near=min(world_near-0x900,0x200), far=0x200`; `FUN_0043B410` then adds only
that callback's model-root Z before the final `FUN_00470A10` call. Successive
ring roots do not accumulate. The same reset applies to the flags-`0x03`
submenu decoration after `optionsh`, so it does not inherit the panel's equal
pair or root shift. The panel itself remains the authored equal hard step at
`0x1E00+2900 = 0x2954` raw (105.8 in port view units).

The frontend Klaus/emblem scene evolves world context `+0x74/+0x78` in
`FUN_0042B5D0`, querying Klaus before C090 consumes the pending command.
`FUN_0042D210` is an integer state query despite its decompiled `bool`: state
1/3/4 returns 4/5/2; other states return 1 only with zero morph progress.
An exact result 1 evolves near/far toward `0xC00/0x1000`
(`near=min(near+dt_us/200,0xC00)`, `far=max(far-dt_us/1000,0x1000)`);
other results use `near=min(near+dt_us/200,0x1500)` and
`far=min(far+dt_us/200,0x1900)`. Klaus and the current emblem are queued from
that pair. `FUN_0042D030` then advances; `FUN_0042B870` queries the post-C090
state and, only for exact result 1, pins near to `0x500`/`0x800` on returned
frames 0/4. Otherwise those frames clamp near down to `0xF00`/`0x1200`.
Later rows consume the resulting near. Submenu command 4 selects state 1
(result 4), so the decoration becomes neutral at world near `0xB00` through
the unchanged AA40 formula; no submenu material override is needed.
Menu init `FUN_0042B230`
stores cumulative Section-7 entry 55 at world context `+0x7C`, and
`FUN_0043AA40` copies it to each local prop context. Model setup
`FUN_00465870` copies parent context dword `0x1F` into the child context;
fog-capable command handlers copy child dword `0x1D` into payload dword 6,
and `FUN_0047F060` passes that payload color to `FUN_0047CA20`. The filler
therefore blends the per-vertex fade bytes toward RGB555 `0x7C00` `(248,0,0)`.
Neither the planes nor this terminal color changes the Section-6 light table
or creates a red material/emissive tint. The generic gameplay initializer
`FUN_004515E0` writes `0x1E00, 0x2800` to context `+0x74/+0x78`, and
`FUN_0042AF00` can pass those words to `FUN_00470A10`. That is not, however,
the fog used by the software terrain pass in the recorded game.

The port now carries these transient frontend planes on each model-body
submission rather than changing renderer-global fog. Equal planes are an
authored hard step, not a divide-by-epsilon approximation in the shader.
Klaus uses the live pre-advance pair; flag-`0x02` props use independent
post-suffix, root-relative bands; and `optionsh` uses its fixed equal step. The
draw restores the scene fog enable, planes, and colour before later geometry,
leaving the separately sorted additive frontend billboard and 2-D overlays
untouched. Gameplay-pause overlays do not run frontend initializer
`FUN_0042B230`; the port therefore retains their neutral terminal color and a
capped non-pulsing prop-row band until that outer world context is recovered
independently.

Three 20-second passive captures from the supported retail executable
(`20260714-202113-fog-distance`, `20260714-202201-fog-over-water`, and
`20260714-202351-terrain-screen-edge`) consistently observe terrain scan
dimensions 32x21 and software-pass source words `0x0D00/0x1500`. During the
active draw, `FUN_00470A10` installs those exact 13..21-cell planes with scale
`0x2000 = 0x1000000 / (0x1500 - 0x0D00)`. Brief equal-plane 41.332-cell
samples occur between draws when the transform is reset to identity; they are
not scene fog. Those observations establish the captured world's 13..21
pair, rather than a universal eight-cell fog width or a fog scan cap.

**Authored world fog owner (`42E920` → `451710`, 2026-09-30):** the original
PE reads Section-13 `+0x88` as far distance and `+0x8C` as fog width. It shifts
both dwords left eight bits, then stores `far_raw-width_raw` at scene `+0x74`
and `far_raw` at `+0x78`. This is 32-bit wrapping arithmetic. `330D0`'s
30-row terrain scan cap is independent. `4515E0` first installs the generic
30..40 scene pair at `4516AD/4516BA`, then calls `451710`, whose successful
descriptor read replaces it; `44FEB0` also resets that pair during a separate
transition. The generic initialization does not override the loaded world's
selected authored planes.

The 152-descriptor/four-tier corpus has widths 1..29; 32 of the 36 ordinary
worlds 13..48 differ from the former eight-cell band. Level 2/Medaeval uses
16..22, world 15 uses 12..22, world 31 uses 24..29, and Arena4 uses 1..30.
The port's loaded scene, Main Base request and gameplay particle adapters
consume both authored values through `LevelFogPlanes`; no fabricated width
is supplied when the descriptor is absent. Main Base `42F1A0` copies these
same `+0x88/+0x8C` words to controller `+0xB8/+0xBC`, so the abort consumes
the same pair. Particle planes retain raw words directly instead of passing
through a float conversion. Frontend Klaus/menu transitions keep their
separately owned mutable pairs. Intro2's particle 12..24 path is unchanged;
its common loaded-scene setup uses descriptor 24/12.

`execute-native-world-fog.py`
replays the verified original descriptor reader, successful scene stores and
`470A10` setter on private controlled buffers. It accepts a strict canonical
Rust-parser corpus and includes zero/equal, negative-near, signed-bit,
wrapping and uncapped arithmetic controls; the
`guards` fail closed. This is
source/data/instruction proof, rather than a captured whole-world raster run.

The [same-draw active-fog receipt](MODEL_DRAW_CUSTODY.md#same-draw-active-fog)
authenticates the accepted tick 952 viewport pair and active publication only;
other model-local, identity-reset and Klaus-evolved pairs remain separately
owned. In the model-node context, `+0x74/+0x78` instead hold terminal colour
and the inherited viewport pointer. That scene pointer is distinct from the
immediate parent model context at child `+0xC0`, which `470410` follows for
selected tf11 cache imports through the child `+0x90` remap table. A native
fog/queue producer must preserve those owners and layouts independently.
The remaining sprite/raster boundary is described
[here](#sprite-fog-remains-a-separate-path).

The fade is not evaluated independently for every output pixel. The projection
helpers first quantize it into the vertex byte, and the software polygon path
then interpolates that value affinely in screen space. The port shader mirrors
that order with a projective varying. Its mesh coverage extends two hidden
cells past the terminal plane and expands laterally to the active horizontal
FOV; this preserves the recovered software planes while preventing widescreen
corners from exposing the rectangular scan boundary.

### Material-specific model fog

Retail's far textured fillers reduce the palette shade accumulator before
lookup, then compose that sampled colour with fog and destination. Applying
`mix(sampled_colour, fog_colour, fade)` before every blend adds excess cyan
to half-additive tree/fence ground faces and to additive materials.

The model-wide near/far constructor selection above is significant even
when every vertex of one face has a zero fade byte. B84 (`FUN_0045DDE0`)
still submits the far `FUN_0047F060` filler in that case. Far unlit faces
start at row 28 regardless of the near fill's sprite-`0x04` row-0/28 choice:
their 16.16 shade is `0x1C6400 - fade_byte * 0x1C00`. Far uniformly lit and
Gouraud faces use `(((255-fade_byte)*base_shade)+128)<<8`
(`FUN_0047F480` and siblings). These values interpolate affinely across the
polygon before palette lookup. Lighting family and mirror opcodes retain
their authored meaning.

Let `S` be the colour sampled from that faded palette row, `L` the fog-table
contribution, and `D` the existing framebuffer colour. Far indexed kernels
use the following component-level operations; the native implementation
performs packed-channel masking, saturation and ordered dither:

| Blend | Far operation | Retail filler |
|---|---|---|
| Opaque / keyed | `sat(S + L)` | `FUN_004786A0` / `FUN_00478D00`; shared LUT from `FUN_0047CA20` |
| Half-additive | `sat(2*S + L)/2 + D/2` | `FUN_00479B90` / `FUN_00479490` |
| Additive | `sat(S + D)`, no fog-colour term | `FUN_0047A540` / `FUN_0047A0B0` |

The half-additive destination coefficient stays one half; this is not an
opacity-to-zero fade. Near half-additive instead retains `S + D/2`
(`FUN_004796D0` / `FUN_00478F60`) without the far kernel's doubled-source
pre-saturation. At the terminal distance a black-source half-additive shadow
over an already fogged surface therefore converges to that same surface
colour, rather than becoming a brighter cyan patch.

The indexed GL path preserves this operation order, including palette lookup
after shade reduction and node-specific near/far selection. Floating RGB
composition still approximates native packed-channel arithmetic, fog-table
quantization and ordered dither. Direct 16-bit sprite texels (`0x02`) and
untextured RGB paths retain their separate fallback; they are not the
greater-than-16-colour indexed palettes. Whole-original-polygon all-`0xFF`
rejection remains open and cannot be replaced by dropping individual triangles.
The hierarchy's far-radius test runs before command execution; the existing
exact ribbon endpoint rejection is separate.

Controlled GL readbacks verify terminal opaque/half/additive output, keyed
zero, nonlinear palette rows, the all-zero-fade far constructor, and the
different near/far half-additive saturation rules. Elevated first-world
village and pen comparisons remove the reproduced cyan fragments. Paired
building views also retain the Main Base/factory inside the recovered scan
window where the previous rotated gate dropped their complete models.
These are port regressions; matched retail visual acceptance remains open.

→ **Port status:** `models.rs` preserves all three lighting families, the
uniform face normal and the Gouraud `s0..s3` corner normals. Model textures
retain raw indices plus all
32 palette rows; the GL model shader performs point-sampled, affine UV/shade
lookup and affine per-vertex fog. Uniform solid faces use the recovered packed
palette-plus-light operation; Gouraud solids pack their interpolated channel
accumulators in the fragment shader. Shader-unavailable textured fallback uses the
Section-6 RGB values for both lit families; unlit fallback textures keep their
authored row 0/28. It never reinstates the former made-up light curve.
`ModelMesh` also carries the active signed raw VIEW-space light vector: local
submissions default to `(73,73,-73)`, world trees install the level's reduced
Section-10 direction, and the frontend Klaus submission installs
`(-100,50,-50)`. Both backends dot it with VIEW-space normals, through the
same camera rows that place the vertices, and retain the raw magnitude when
applying `FUN_0046D3F0`'s signed-16-normal `>>19` binning. Before 2026-10-07
world models used `(73,73,-73)` against world-space normals, which lit Intro2's
tower walls about twice as bright as the retail trace.

## Open foundational mismatches

The following boundaries were checked against original handlers and canonical
assets during the 2026-09-07 audit. Lighting, view commands, sprite fog, near
rejection, terrain-cache and raw-word decoding repairs below are implemented;
the named acceptance limits remain open. Numeric and port GL
validation do not substitute for matched retail screenshots.

### Gouraud solid colour composition

The former `base_rgb * section6_rgb / 128` modulation is removed from
Gouraud solids. Near solid `23` runs `FUN_00460CC0 → LAB_0047A860 →
FUN_0047C150 → FUN_00474220`; the corresponding quad setup is `0047E930`.
Each Section-6 RGB byte `S` initializes an accumulator `(S << 16) + 0x8000`,
independent of the Section-7 base. A black base must not extinguish it.

Far solid `23/24` uses `0047C2F0/0047EB40` and the same span, with a different
initializer: `((B+S)*256 + (F-B-S)*(f+(f>>7))) << 9`. `B` and `F` are
base/fog display channels masked with `0xF8`, including green; `f` is the
corner fade byte. Even far `f=0` differs from the near setup. The span packs
after screen-linear interpolation: for each accumulator divided by 65536,
retain `floor(channel) & 0xFF0`, then form the wrapping word
`(R << 7) + (G << 2) + (B >> 4)` and decode RGB565. Channel carries are
intentional; independently clamping channels changes the result.

`gouraud_solid.rs` owns the two initializers and CPU packing reference.
The model shader interpolates unpacked channels through its affine varyings,
packs at the fragment, and does not apply fog a second time. Explicit scene
tint/emissive adapters apply afterward. Unlit, uniform, and textured families
retain their separate paths. Without the model shader, packed corner RGB is
the fallback and cannot reproduce per-fragment quantization; GL triangle
rasterization is not a bit-exact recreation of the integer retail scanline walker.

Canonical normal-tier models `50 pl4gunbarrel`, `51 pl4gatgun`,
`53 pl4thumpertop`, `56 pl4tubegun`, and `65 pl4biggatgun` contain Gouraud
solids with palette entry 32 equal to zero and nonzero corner normals.
`223 factory2pillar` and `283 pwrstat` also use the affected family. This is a
reachable material defect, not an unused fallback.

Focused tests cover black/nonblack bases, near/far endpoints, interpolation
thresholds and wrapping carries. Controlled real-GL samples exercise unequal
corner depths, varied colours and unchanged material families. Keep matched
retail/port comparisons of the named gun parts and structures in visual acceptance.

### View-dependent command selection

`models.rs` now interprets `0x0B/0x0C` using explicit `ModelViewSelection`.
Retail `FUN_00467050/00467090` chooses a block using
the normal-cache flag produced by `FUN_0046D3F0`. The condition controls
complete command streams, including ribbons, child instances and painter
groups; per-triangle backface culling cannot replace it.

A normal-tier census through all 53 available overlays found 16 such commands
across nine distinct layouts, among 1,301 unique model layouts. Selecting each
retail branch in cloned in-memory models demonstrates the emitted differences:

| Model | Intrinsic all-branches result | Either selected retail side |
|---|---|---|
| `271 bluebee2`, `272 grenbeet` | 36 ribbons | 24 ribbons |
| `351 grendrag` | Four wing-model-359 children; 30 triangles | Two wing children; 26 triangles |
| `147 waterscanner` | 50 triangles | 44 triangles |
| `148 scannerdish`, `149 deadscannerdish` | 66 / 64 triangles | 64 / 62 triangles |

The intrinsic all-branches mode remains explicit for asset inspection and
camera-independent consumers. Live static, linked and painter traversal selects
per-node view commands before geometry, children or painter operations are
emitted. Cached static roots rematerialize only when their decoded default
path actually reached a view command (or needs terrain aliases); opcode-like
operand words do not trigger it. `FUN_00466160` constructs the
raw local camera-relative origin using separately shifted Q31 products and
wrapping sums. `FUN_0046D3F0` uses mirrored normal/anchor parity and signed
wrapping i32 products; dot zero is backfacing. `0x0B` jumps on that flag,
`0x0C` on its inverse. Offsets are signed word counts relative to `pc+1`.
Children resolve their own context after their mount transform. The normal
predicate reads raw anchor words, not generated/grounded positions; implicit
normal refs 0/1 retain their preseeded false flag. Malformed live operands or
targets stop interpretation and increment `invalid_view_branches`; backward
loops retain the interpreter guard.

`ModelTreeView` carries the installed scene camera through gameplay, HUD,
menu and viewer calls. It adapts the current float world pose by undoing
the raw-to-world scale and transposing the node's orthogonal basis, including
local reflections. It does not recreate the separate Q31 rounding of retail's
camera/model transforms. Signed wrapping at the normal predicate is exact;
grazing-plane decisions can still differ at that presentation boundary.
Regression tests cover both sides and equality, mirrors, signed offsets,
wrapping arithmetic, register side effects, child mounts/transforms and all
three tree traversal paths while preserving intrinsic inspection.

The same complete normal-tier census found no reachable `0x13/0x14` distance
branches. Their frozen static interpretation is a dormant parser limitation,
not a justified explanation for current building pop-in. A candidate
`lifterweight` branch also produced no geometry difference at default
materialization and is not a demonstrated visible example.

### Sprite fog remains a separate path

Model billboards and particles now share the indexed material consumer with
model bodies. The former fixed-row RGBA upload, conventional masked RGB fog,
and unfogged additive attachment paths are removed for indexed sprites.

Retail B78/B8/F8 `FUN_00468520` rejects centre fade 255, copies the centre's
fade into all four corners, and queues `LAB_0047AA40 → FUN_0047F060`. That
uses the far palette/composition rules above. The Level-13 plus normal-tier
system-2/3/5 census found 117 additive and 37 masked textured billboards,
including powerup models
83–106 (sprites 612–617) and plasma attachments 46–48 (603/602/601). No
half-additive model billboards were found in that census. Preserve the
separate unfogged frontend emblem call when repairing world attachments.

The former `draw_world_fx` scaled additive RGB using Euclidean camera
distance. `FUN_0043D410` instead selects near/far using
projected centre depth and copies the projector's centre fade byte to all
four far-quad corners. Two particles at equal view depth therefore must not
fade differently merely because one lies toward the edge of the screen.
With the 13..21 band, offsets `(0,0,17)` and `(10,0,17)` both now select
centre byte 128; the former RGB factors were 0.5 and about 0.160 despite
identical view depth. The indexed lookup preserves nonlinear authored ramps.

Existing accepted evidence is sufficient to establish an independent Intro2
particle context: `20260722-022852-particle-gate-intro2.jsonl`, samples 1158
and 7045, retains parent and active near/far words `0x0C00/0x1800` together
(12..24 cells), while the scan has 24 rows. The port's generic far-minus-eight
choice would produce 16..24. The port carries these explicit draw planes
separately from lifecycle/overlap admission and sort keys. No repeat capture
is needed for this finding.

The submission decision matrix is now explicit:

| Caller | Near/far selection | Material/coverage policy |
|---|---|---|
| Textured model `78/B8/F8` | Owning node origin plus radius; header `0x20` forces near; each child reselects | Far rejects centre byte 255, repeats one byte on four corners; zero byte still uses the far row-28 accumulator |
| `FUN_0043D410` particles | Projected centre strictly before parent near is near; equality selects far | Late sprite rejection at parent far leaves prior lifecycle/light processing intact; stable sort key and mixed-material order remain unchanged |
| Frontend `FUN_0042D030` emblem | Always near `FUN_0047AA20` | Separate unfogged call; no inherited world or model fog |
| Flat model `68/E8` | Separate `A960/A980` solid fillers | Existing additive presentation retained; this indexed repair does not establish their far-colour parity |
| Direct RGB / shaderless fallback | Same centre and draw admission | Retains RGB approximation; additive attenuation adds no fog colour, shaderless indexed textures retain authored near row 0/28 |

Near indexed sprites use row 0 or 28 according to flag `0x04`; far sprites
always use `0x1C6400 - fade_byte * 0x1C00` before lookup and the existing
material-specific fog composition. Flag `0x01` still owns index-zero keying;
blend flag `0x10` takes precedence over `0x08`. The reciprocal is computed
before multiplication: `floor((z-near)*floor(0x1000000/(far-near))/65536)`.
Intro2's 12..24 midpoint therefore gives **127**, while 13..21 gives **128**.
`ModelBillboardDraw` carries the node radius and local fog policy; resolved
world-space quads share one GL submission phase without sharing traversal or
particle admission. It resets indexed texture units, shader mode, explicit fog
coordinates, blend, and depth state before the next draw. Native packed-channel
masks, fog-table quantization and spatial dither remain the model shader's
documented approximations. Matched retail-frame acceptance remains open.

Validation on 2026-09-07: 47 hidden-GL pixel checks cover nonlinear palette
rows, all three blend families, keyed zero, far rejection, equal-depth
off-axis sprites, and state after indexed/direct/solid/underwater/menu draws.
The normal-tier scene sweep retained 91 before/after pairs: 60 byte-identical
and 31 with localized fog changes. Menu, Display, Klaus and the initial
first-world view stayed identical; Intro2 and spider-pen composition remained
intact, and terminal-distance stray light pixels disappeared. Full repository
checks passed with both retail corpora present. These port comparisons do not
close matched retail-frame acceptance.

### Original-face near rejection

Model projectors `FUN_0046CD90` / `FUN_0046CEB0` and water projector
`FUN_004321E0` mark signed view Z below `0x40` raw with clip bit `0x40`. Constructors combine
all original corner flags and query `0x4C5268`; every table entry with that
bit rejects. Model triangle/quad constructors and water `FUN_004327C0`
therefore discard the complete original polygon when any corner is too near.

The port preserves each original polygon alongside its triangles, including
the otherwise hidden fourth corner of a degenerate quad. Both halves reject
together, and mirrored copies retain independent corner sets. Cached and live
materialization, painter slices and backend submission keep this provenance;
an unresolved original corner cannot leave a surviving half. This covers all
24 face opcodes in both constructor tables. Hardware clipping remains after
this admission step.

Near policy belongs to the submission, independently of model size, fog
planes or header `0x20` (which only forces the near fog constructor):

| Caller | Raw units per view unit | Admission |
|---|---:|---|
| World models and attachments | 256 | Reject the original face below raw Z 64 (0.25 cell) |
| Frontend props, Klaus and foreground HUD | 100 | Same raw threshold (0.64 view unit), including Klaus above the loaded world |
| Model billboards | Owning scene's explicit domain | Reject when the centre projector rejects |
| Water | 256 | Test all four animated surface corners before triangulation |
| Edge/line primitives | Owning scene's explicit domain | Reject either near endpoint; retain the edge core's existing 8.8 width/window-Z conversion |
| Diagnostic views | Camera | Retain existing GL camera clipping and diagnostic edge projection |

Admission uses full camera depth after the model and world-surface callbacks.
Type-14 selector outputs use their direct world positions, bypassing the model
basis as in retail. Type 1 (`FUN_0046DC00`) projects both source slots before
averaging screen coordinates: either source near bit rejects the midpoint,
even if the averaged 3-D position would pass. Its recursively mirrored source
references therefore remain projection dependencies. Type 5 is an ordinary
3-D midpoint and tests its own position. Missing/cyclic dependencies fail
closed. Formats retain this intrinsic distinction without selecting a scene.
Billboard-only nodes preserve their resolved anchors rather than replacing
them with the plain-point fallback. Extra projector source vertices do not
change named muzzle tips, diagnostic bounds or camera-facing classification:
those consumers measure submitted primitive references, retaining all points
only for nodes without primitives.

A canonical-parser census of PRELOAD plus the installed `1X*.OVL` corpus
materialized 1,358 entries at seven animation poses (9,506 evaluations).
Its 1,724 billboard submissions used centre types 0, 5, 8 and 9; none used
type 1 or contextual types 12/13/14. This samples animation values, not every
branch combination or linked-parent input. Contextual billboard anchors still
require their retail callbacks before projection; their existing intrinsic
presentation path is not established by this repair.

The float adapter rounds view depth into the selected raw units, matching the
existing CPU projector boundary; it does not establish retail Q31 transform
parity. Exact type-1 screen midpoint placement and its stored minimum source
depth for shading remain distinct from the repaired rejection dependency.
Model-radius rejection before complete child traversal remains the separate
boundary described above. Matched retail-frame acceptance remains open.

Validation on 2026-09-07: 46 hidden-GL controls cover raw 63/64 in both scene
domains, disabled fog, atomic and mirrored quads, type-1 versus type-5 sources,
pitched/transformed geometry, direct type-14 world positions, edge primitives,
billboards, and canonical/fallback water including straddling quads. Full
repository checks pass with 3,693 tests and both retail corpora present. Of
91 paired normal-tier scene frames, 90 are RGB-identical; the late Klaus
`0xC000` morph loses near-black polygons (maximum channel difference 8).
Menu, Display, other sampled Klaus poses, Intro2 and world scenes are unchanged.
Rendering that same late morph with `ModelNearClip::Camera` reproduces the
baseline byte-for-byte, isolating the difference to retail near admission.

### Terrain cache footprint precision

The former key floored camera X/Z and bucketed yaw, although cell selection
used the exact pose. With unchanged lighting and no infection it retained stale
boundary strips: for a +Z camera with two-cell lead, Z=20.1 and Z=20.9 shared a
key while cell Z22's centre crossed from depth 2.4 to 1.6. Fog-width changes
inside one rounded row bucket and changed frame scan dimensions were also absent
from that key. Zero terrain lights do not force a revision every frame.

`TerrainScan` now resolves one immutable `TerrainFootprint` from the exact
unwrapped X/Z, normalized horizontal heading, pitch lead, effective near/far
scan bounds and half-width. The cache compares that same value used to enumerate
cells; terrain and water share its Z-outer/X-inner selection order while retaining
their own frame dimensions (fallback 52×30). Camera setters retain the current
horizontal FOV without the former epsilon, and derived coverage replaces the
separate approximate setter invalidations. Irrelevant changes, such as FOV with
fog disabled or dimensions entirely covered by the same overscan, can still reuse
the mesh. Fractional movement can rebuild every frame; scratch allocations are
reused. Geometry stays unwrapped, and only sample indices wrap.

Light revisions, live terrain-type signatures and visible infection animation
retain their independent rebuild conditions. Heights, terrain headers, palettes
and frame UVs remain under the existing per-level explicit invalidation contract.
Retail `FUN_0042FCC0/00431D20` rebuild fractional first/last rows each draw. This
repair preserves the port's existing forced-rebuild selection, fog overscan and
per-cell geometry; it does not establish exact retail fractional-row geometry.
Acceptance compares incremental and forced builds at the identical final pose.

Validation on 2026-09-07 reproduced 15 stale-cache failures in 36 hidden-GL
comparisons with zero lights and no infection. After repair, all 40 controls
match fresh builds, including fractional moves/turns, negative and wrapped
coordinates, scan dimensions, fog width and tiny FOV changes. All 91 paired
menu/Klaus/Intro2/gameplay frames remain RGB-identical. These controls establish
cache consistency, not matched retail fractional-row rasterization.

### Raw-word sprite decoding

Sprite flag `0x02` selects little-endian RGB555 words, now decoded independently
of palette rows. Atlas X/width are bytes; pixel width is half the rectangle's
byte width. Flag `0x01` keys the converted zero word, so source bit 15 is ignored
before the zero test. Loader/converter provenance and the complete raw corpus
are in [Section 3](FORMAT_DOCUMENTATION.md#section-3-sprite-atlas).

The former byte-index interpretation doubled widths and corrupted colours.
All 30 level-selection thumbnails (3733–3762, every X51 tier) now decode at
32×24 low / 64×48 high, retaining opaque black. Copyright 1292 in `0X5XX.OVL`
now decodes at 155×14 with keyed transparency; tiers 1–3 retain their indexed
309×28 source. Raw 677/1319 retain their authored 1×1 / 4×8 dimensions.
`SpriteAtlas::dimensions` also supplies model/billboard aspect without assuming
indices. Raw textures keep direct RGBA uploads. This repairs intrinsic asset
decoding; exact packed RGB lighting/fog in raw retail fillers remains separate
from the indexed palette path (for example keyed RGB-Gouraud `004763F0`).

Validation checks 260 raw decodes against independent source-word RGB555/alpha
expectations across low/high tiers, ordinary/opaque passes and shade inputs.
All pass; the 66 sampled source rectangles are unchanged. Extracted images and
hidden-GL thumbnail/backdrop frames show the repaired pixels and proportions.
The normal indexed copyright and all 91 paired menu/Klaus/Intro2/gameplay
controls remain unchanged. Matched retail presentation acceptance remains open.

## Key addresses

| Thing | Address |
|---|---|
| Affine span reciprocal (1/width) | `0x4D4B98` |
| Light/shade ramp LUTs | `0x4FBDC8` / `0x4FBDCC` |
| 16bpp color masks | `0x4FBE10`, `0x4FBE24` |
| Span-filler tables (indexed / raw 16-bit texels) | `0x4D55A0` / `0x4D5900` (`PTR_PTR_004D5C60/64`) |
| Point-sampled affine textured filler | `0x478D00` |
| RGB-Gouraud untextured filler | `0x474220` |
| Device fill-pointer table setup | `FUN_00480D10` (device+0x1018..0x10C4) |
| Queue push (append / depth-key) | `FUN_00459D10` / `FUN_0045B220` / `FUN_0045B280` |
| Queue flush: merge-sort → drain | `FUN_00494930` → `FUN_00494A50` |
| Model authored-plane setup/test | `FUN_0046D5A0` / `FUN_0046D3F0` |
| DirectDraw surface lock | `FUN_004A9CE0` (IDirectDrawSurface::Lock vtbl+0x64) |
| Bilinear toggle (DEAD) global / handler | `0x4CB3F0` / `FUN_0043CCA0` |

Binary source tags: `C:\Coding\FGDK\Code\Windows\Graph2D.c` (rasterizer),
`…\DDCalls.c` (DirectDraw), `…\movie.c` (AVI). Imports: `DDRAW.dll` only.
