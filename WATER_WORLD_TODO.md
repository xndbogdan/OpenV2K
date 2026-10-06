# V2000 Port — Water + Round-World Environment

Status and remaining tasks for the sea rendering + toroidal ("round planet")
world features. Started 2026-07-02.

Ground-truth references:
[`FORMAT_DOCUMENTATION.md`](docs/re/FORMAT_DOCUMENTATION.md) §10,
[`GAME_MECHANICS.md`](docs/re/GAME_MECHANICS.md), and
[`RENDER_PIPELINE.md`](docs/re/RENDER_PIPELINE.md).

---

## Done (2026-07-02)

- **Coordinate-scale unification.** Terrain height is signed `i8 × HEIGHT_SCALE`
  (0.125), matching the engine's `(char)*32` over a 256-unit cell. Single space:
  1 cell = 1.0 unit. (`terrain.rs`, `gl_backend.rs`)
- **X-major terrain addressing.** All world sampling now matches
  `FUN_00445860`: `index = x * 256 + z`. This removes the old whole-world
  transpose from terrain rendering, collision, water, intro placement, and
  exported overview maps.
- **Per-level sea plane.** Height = Section 10 header[0] / 65536; rendered when
  `(header[0] >> 8) > -4096` (engine's water gate). `sea_level_world_y()` /
  `water_enabled()` in `terrain.rs`; drawn in `main.rs` Playing render.
- **Translucent water.** ~50% blend. The former invented fullscreen blue wash
  has been removed: the original changes underwater projection callbacks and
  applies its authored 0–8 darkness step to world palettes, not a colour tint.
- **Toroidal world wrap.** `v2k-core::world` (`wrap`, `delta`, `WORLD_CELLS`);
  seamless terrain sampling (wrapped `height_at`); wrapped player + camera;
  chase camera follows via shortest-path `delta`; terrain mesh rebuilt as a full
  256×256 (wrap row/col) and tiled around the camera; entities drawn at their
  nearest toroidal image (`main.rs` model loop).
- **Animated waves.** `terrain::water_surface_raw` owns submitted `FUN_00445920`
  heights (3 integer-frequency sines, depth-scaled, wet troughs terrain-clamped).
  Dry shoreline vertices remain at static sea height; the separate submerged
  bits and authored shoreline sprites determine coverage. The physics-oriented
  `wave_surface_raw` / `wave_surface_y` retain dry terrain as the ride surface
  and must not be reused for those rendered corners. Original-PE checks cover
  both wave modes and the Intro2 hut's wet centre/dry neighbours. Water is
  tessellated on a world-anchored lattice with dry quads culled. Vertices use
  `FUN_004321E0`'s slope-based 0–7 palette shade, shared scrolling dynamic
  lights, and camera-depth fade; the invented crest glint is removed.
- **No continuous ambient-wave sound.** The complete retail water-render chain
  `FUN_0042F270 -> FUN_00431A40 -> FUN_00431A60 -> FUN_00431D20` contains no
  sound enqueue. Water audio is event-driven: global sounds 26/27 belong to
  contact/downwash responses and global sound 17 belongs to hard whole-body
  entry. The menu's Ambient setting is the separate binary CD-music gate. Do
  not add a looping sea bed unless a future voice-catalog capture proves an
  independent world/entity owner outside the water pass.
- **Authentic bounded water scan.** `FUN_00431A60` consumes the same
  `DAT_004CAB70/74` dimensions as opaque terrain. The port now uses each
  level's Section 13 `+0x88` footprint for water too; the invented 80-cell
  near ocean, 32-cell coarse far tiles, and wave-damping seam are removed.
- **Ride surface.** `em.update` rides `max(terrain, wave)` so craft float on the
  sea on water levels. (`entity.rs`)
- **Common underwater buoyancy + weight sinking.** Both player movement styles
  now enter `FUN_0040E100`'s exact fixed-point common tail after their
  mode-specific callback: gravity, strict
  `depth < -100` gate, signed total-mass split at 151, depth-proportional light
  lift / constant heavy lift, and three-axis signed-word damping. The common
  sample is `max(integer-cell terrain, static sea)` rather than the animated
  wave. Carrying the type-68 mass therefore switches both the Sub-C ride target
  to seabed and the separate underwater force to the sinking regime. Hover and
  VTOL then share bit-gated `FUN_0044EC60` drag and `FUN_00412DA0` signed-8.8
  position integration. VTOL now ports the recovered center/±X/±Z animated
  ride probes, active-model underside clearance, manual/assist lift, fuel,
  near-surface correction, height/upward-velocity attenuation, and
  previous-basis projection for the captured normal Sub-G branch. The live
  Self Righting pitch recurrence is now exact as well: mode 0 is off, mode 1
  levels, and 2..15 seek their native signed-word nose-down targets after
  manual input. The temporary post-integration terrain/wave floor is removed,
  preserving retail water penetration on hard drops. The distinct oriented
  terrain path is now complete: an eligible type-46 player submits `player4`'s
  authored broad gate and eight moving spheres to the wrapped, signed,
  [retail cell-plane heightfield](docs/re/TERRAIN_COLLISION.md), retains the deepest contact, separates along its
  Q12 normal, removes only inward normal velocity, and feeds channel-1 impact
  through the hull damage profile. This closes the pitched-nose hole over dry
  terrain and makes the seabed solid without treating the animated water plane
  as collision geometry. Static recovery and the clean survey responses support
  the generic `FUN_004141D0`-style response, but the exact dispatch identity of
  type 46's vtable surface callback versus that default callback remains an
  RE-provenance caveat. Normal
  type-46 yaw/bank now also uses the exact shared Sub-D steering step followed
  by one A690 roll-damping pass. The alternate `Sub-G+0x42 == 0`
  terrain-derived attitude/rate arithmetic is now recovered as a detached,
  fail-closed plan: exact target smoothing, forward projected surface choice,
  velocity clamps, rate, and clearance hysteresis are regression-tested in
  `vtol.rs`. It is not connected to ordinary type-46 play because every
  accepted player trace remains in `+0x42 == 1`; the writer/owner which admits
  this alternate state and authored destruction presentation remain open.
  (`entity.rs`, `vtol.rs`,
  `FUN_0040E100`, `FUN_00445310`, `FUN_0041B210`, `FUN_00412870`,
  `FUN_004141D0`)
- **Whole-body water-entry classifier recovered.** `FUN_004129B0` uses the
  signed height byte of the current integer X/Z cell rather than bilinear
  terrain, applies the animated wave, preserves prior state on dry cells, and
  detects the strict active-model `+0x0A` sphere transition from fully above to
  intersecting. The edge itself is speed-independent. Its player response is
  connected: nearest-cell material selects the response; selector 7 returns
  before all effects; signed vertical velocity `>= -1000` makes four owned
  `FUN_00440DC0` allocation attempts at scale `0x1000`; and `< -1000` plays
  positional sound 17 and arithmetic-halves vertical velocity. Pool saturation
  can hide attempted births. Hard-entry operation `0x3C` now publishes a real
  entity-backed type-60 `ExplodingRing`: all four model overrides are `0x82`
  below `-1750` or `0x84` otherwise, at the displaced animated-water origin.
  Its shared constructor consumes one singleton selector word before sound 17
  and signed velocity damping. A successful class-48 Primary starts at control
  `0xFFFF`, joins the specialized scheduler after the current pass, advances
  the entity-owned presentation through retail control decay, and enters class
  2 plus deferred destruction after roughly 2.4 seconds. The moderate
  model-132 ring is independently visible in
  `20260719-181705-vtol-surface-effects.jsonl`; the earlier
  `20260712-203635-full-session.jsonl` closes the same entity family outside
  that guided run. Do not confuse this type-60 ring entity with particle
  classes 60..64 emitted by the class-19 ground-response probe.

  The classifier and common fallback now live in a pure entity-agnostic policy
  module. Type 46 commits the returned classifier state first, then executes a
  returned hard plan through the real actor constructor/owner or takes the
  particle fallback. Ordinary player behavior is unchanged; the previously
  fail-closed impossible/partial-state edge now follows retail's direct test of
  the prior fully-above bit. The generic non-player outer dispatch remains open:
  static `FUN_00411AD0` proves the ordering—solid-terrain response, whole-body
  classifier, optional type-record `+0x88` sound, then an exclusive choice
  between the type/style `+0x10` callback and `FUN_004141D0`—but the accepted
  corpus has no focused non-player crossing that identifies the live outer
  walker/type-vtable owner. This is separate from the class-19 motion probe and
  its eight class-14 water-downwash children, and from the special selector-6
  callback's single-particle attempt.
- **Class-19 terrain material selection corrected.** `FUN_0043E8A0` does not
  select an authored terrain triangle or consume the Section-10 attribute. It
  rounds the unsigned 8.8 X/Z words with wrapping `+0x80`, reads
  `terrain_type & 7` from that nearest cell, and maps the result through the
  active ground-response table. Its class-60..64 children inherit parent
  runtime byte `+0x1D` bit zero. The port now preserves both raw-word rules.
  The process-global `DAT_004F72CC` governor and its direct `>>13` consumer are
  now represented: the path emits eight attempts at full rate and can emit
  zero after a severe frame stall. The writer receives uncapped durations only
  in Intro2 and ordinary gameplay, excludes the post-Intro Klaus handoff, and
  survives level-effect teardown.
- **Retail software gameplay fog restored.** Passive captures show the active
  software world pass tying fog to the level-authored terrain footprint: it
  ends at the capped scan depth and begins eight cells earlier. The captured
  32x21 world installs exact `0x0D00/0x1500` (13..21-cell) planes, not the
  generic gameplay context's initialization words `0x1E00/0x2800`. Terrain
  uses explicit shader uniforms and reproduces the software path's per-vertex
  0..255 byte followed by affine screen-space interpolation. A two-cell
  fog-only GL overscan reaches beyond the terminal plane; its width also
  expands to the actual horizontal FOV so widescreen cannot expose the
  authored 4:3 scan rectangle at a corner. Visually verified against the
  original game by the user on 2026-07-14.
  Water resolves its palette texel first and approaches half the fog colour,
  which makes the retail `texel + dst/2` composite terminate at exactly the
  already-fogged background rather than becoming transparent over sharp land.
- **Marching-squares shoreline textures.** The GL water pass now resolves the
  five authored shoreline frames at `terrain_sprite_base + 125..129`, applies the
  original 16-entry shape/rotation table, preserves their raw indices and all
  32 palette rows, and maps slope shade 0..7 through Section 6 exactly. A
  brightest-shade RGBA texture remains only as a shader fallback. Partial
  shoreline shapes use index zero as their dry mask, while the full-water
  shape is solid: its index-zero pattern is retained as palette detail instead
  of becoming holes through to the seabed.
- **Gameplay model scale.** The original per-vertex handlers multiply signed
  model coordinates by the entity's 1.31 matrix and add them directly to its
  8.8 translation. Gameplay models and instance attachment offsets therefore
  use `raw / 256`; the port compensates for the menu renderer's older `/100`
  convention at gameplay call sites.
- **Authored bounded terrain mesh.** The GL backend now reads Section 13
  `+0x88` and reproduces `FUN_00433130`/`FUN_004330D0`: each level uses
  `min(2*floor(depth*4/5), 52)` columns by `min(depth, 30)` rows. It wraps only
  sample indices instead of drawing repeated copies of the full 256×256 world.
  Terrain and water share an exact selection footprint. Fractional camera
  movement, small turns, pitch lead and fog/FOV coverage participate in terrain
  cache validity; the fixed gameplay camera follows the original bearing. See
  [cache ownership](docs/re/RENDER_PIPELINE.md#terrain-cache-footprint-precision).
- **Pitch-dependent terrain-row origin.** `FUN_0042F270` and
  `FUN_00431890` derive the first scan row from camera basis word 12 rather
  than always starting two cells ahead. The port now reproduces the signed-Q31
  addition, both wrapping activation ranges, and the large upper-threshold
  discontinuity. Terrain, water, fog overscan, static objects, and the terrain
  cache key all consume the same recovered lead, keeping a bounded rear buffer
  when the camera tilts far enough to show the craft from below.
- **Authored opaque terrain tiles.** Section 13 `terrain_sprite_base` now
  resolves and uploads all 120 canonical transition sprites. The GL terrain
  mesh uses the executable's generated 625-entry frame/rotation lookup; an
  explicit opaque decode preserves palette index zero as terrain colour.
- **Persistent terrain infection overlay.** Terrain byte-2 bit `0x10` now
  submits the retail second terrain pass from fixed-row-28 sprites
  `terrain_sprite_base + 120..124`. The same executable marching-square table
  as shoreline selects the keyed partial shapes and opaque full tile, and a
  live type-byte signature invalidates the bounded mesh when infection changes
  under a stationary camera. The shared base and overlay vertices also apply
  `FUN_00433530`'s two-phase triangle-envelope X/Z displacement, preserving its
  16x16 selector layout and process-lifetime terrain-draw cadence. Selector
  realization remains deterministic until the port owns retail's complete
  process-global RNG call order.
- **Indexed Gouraud terrain rasterization.** Each vertex uses `(type >> 5)` and
  the exact Section 10 underwater darkness ramp. The GL 2.1 shader preserves
  the sprites' raw 8-bit texels, point-samples them, then performs the original
  per-pixel lookup in that frame's **32×16** palette. Section 6 maps the
  vertex's 0–7 result to rows `[7,10,14,17,21,24,28,31]`; that 0–31 shade
  value is then interpolated across the polygon. Projective varyings cancel
  GL's perspective correction so UV and shade interpolation remain screen-linear like the
  software span filler. A brightest-shade RGBA atlas remains as a fallback for
  drivers where the shader cannot be built.
- **Retail underwater projected-vertex refraction.** `FUN_00433FA0`'s strict
  below-sea gate now selects one explicit render-context policy for gameplay
  and Intro2. Models, terrain, water, Section-8 model billboards, and persistent
  world sprites all share the recovered post-perspective integer stage: signed
  offscreen SAR stabilization, top-origin phase, exact repeated-word sine
  multiply, and the asymmetric `{-2,-1,0,1}` displacement. The CPU builds the
  exact 64-entry table once per 50 Hz tick; GLSL never approximates it with
  `sin`. Depth, fog, affine UV/shade interpolation, and screen-space HUD or
  cinematic overlays remain unchanged, and every scene boundary resets the
  policy. The GL port's pre-effect perspective transform remains floating
  point, so this closes the recovered refraction effect rather than claiming a
  bit-identical replacement for retail's complete integer projector.
- **Frame-transient terrain-light window.** The X-major 32×32 signed field from
  `DAT_004FE820` is cleared for every presented world frame, then positioned
  from signed camera X and camera Z plus the retail ten-cell scan lead.
  `FUN_0043D410`'s implemented particle classes now contribute their exact
  positive-height or negative/age-faded radii through `FUN_004385E0` before
  terrain and model shading consume the table in gameplay and Intro2. The point
  and symmetric explosion writers (`FUN_00441610`/`FUN_00441420`) remain
  available for their still-unported callers.
- **Authored world background.** Section 13 `+0x54` now resolves through the
  system-level-2 master RGB555 palette and fills the world framebuffer before
  geometry, matching `FUN_0042F270` → `FUN_0047B9F0`. The optional `+0x56`
  `sky1`/`sky2` model used by the first two worlds is drawn twice with the
  original half-camera parallax and −128-unit repeat.
- **Retail gameplay camera side + handedness.** `FUN_0040ED10` puts the eye on
  the craft's −Z side looking toward +Z, while `FUN_0040F3A0` constructs right
  as `up × forward`, keeping authored +X on screen-right. The OpenGL chase
  camera now reproduces both facts without mirroring terrain/entity data.
  `Camera.left_handed` is that view-X adapter, not a flipped world; do not
  reopen it or camera-facing people local-X without reading
  [RENDER_PIPELINE.md](docs/re/RENDER_PIPELINE.md#port-live-world-camera-adapter).
  Active Camera distance uses the recovered `8 + A*(1-sin(heading))` upright
  formula. The shared 0xFA body-forward focus, minimum forward separation, and
  signed-word eye/focus springs are now implemented below. Active gameplay also
  uses the recovered terrain-aware eye target; the accepted 2026-07-30
  Minimum/Default/Maximum bundle confirms that Active Camera changes this
  horizontal distance without a separate vertical term.
- **Docs.** [`FORMAT_DOCUMENTATION.md`](docs/re/FORMAT_DOCUMENTATION.md)
  §10 header corrected (sea level, not
  "world_y_offset"; darkness-ramp fields, not "packed_flags"; signed height).

---

### Hard-water ring coordinate ownership

The location-dependent hard-landing ring failure was a port ownership bug.
Retail `FUN_004141D0` copies the three position words into its Type-60 request;
the class-48 callback `FUN_00406DC0` advances the model control independently of
which signed representation those words have. In the port, the entity decodes
X/Z as signed 8.8 while `WorldFx` presents the same wrapping words in unsigned
0..256 coordinates. For example, raw `0x8000` becomes entity X `-128` and
presentation X `128`.

The task admission check compared those floats directly. Any ring whose X or Z
word had bit 15 set therefore lost its owner with `PresentationStateMismatch`
on its first scheduled visit. Its model control remained at `0xFFFF`, so the
ring never expanded or reached its ordinary class-2 expiry. Sound 17 belongs
to the earlier impact response and could still play normally.

Admission now compares the exact retail position words, matching the existing
Type-60 Main Base abort check. It still rejects an actual changed position,
model, task, or control word. The original wave placement, impact thresholds,
model selection, and callback decay remain the authorities. Boundary tests
cover both hard-entry models across X/Z `0x7FFF`, `0x8000`, and `0xFFFF`, through
animation and deferred cleanup. A canonical-level OpenGL smoke also verifies
both models at all X/Z combinations of cells 64 and 192: the previously blank
ring expands, every lifecycle retires at frame 121 with 20 ms updates, and
signed/unsigned camera-relative images are byte-identical.

The earlier attribution of this report to `DAT_004F72CC`'s eight-frame particle
load governor was unsupported: that governor controls the separate soft-entry
particle burst, not the Type-60 hard-entry ring. The code and canonical level
data reproduce this failure without another retail capture. In-game acceptance
should revisit hard landings in the previously affected map regions.

## Remaining

### 1. Underwater projection and model darkness — implementation complete, visual acceptance pending
`FUN_004336E0` is a shared, tested 0–8 darkness helper. Terrain uses it in its
per-corner shade input, and world entity/static-object model draws now reproduce
`FUN_004136C0`/`FUN_004138F0`'s signed 16-slot Section-6 table shift from local
terrain light minus underwater darkness. This applies to the indexed Gouraud
model path while preserving retail's explicitly unlit flat handlers; menu and
viewer contexts remain unshifted. The three known `FUN_004336E0` callers are
the two entity-model paths (`FUN_004136C0`/`FUN_004138F0`) and the static
terrain-object path (`FUN_0042F650`). Terrain's per-corner shading implements
its own equivalent darkness reduction rather than calling this helper, so do
not invent an extra generic particle/sprite-darkening rule without another
call-site.

The refraction implementation follows the recovered `FUN_00433FA0` callback
contract described above and has deterministic fixed-point boundary tests plus
a real OpenGL 4.6/GLSL 1.20 shader-link smoke test. Acceptance still needs a
paired underwater retail/port recording: confirm the small whole-world wobble
affects opaque geometry and billboards together, begins only below the sea
plane, and does not leak into the HUD or the following menu frame.

### 2. Flood mechanic — runtime rising sea — medium (RE + behavior system)
The "Change Sea Level" AI behavior (`FUN_004013A0`) writes the Section 10 sea
word directly each tick; since the port re-reads `sea_level_world_y()` every
frame, mutating it makes rendering + physics flood automatically. The exact
integer transition primitive is now ported (`behavior::SeaLevelTransition`),
including the original half-speed rise/full-speed fall asymmetry. **Blocker:**
the activation path derives its target from an entity/model bottom and stores
the remaining delta in a behavior traversal record; it is not the factory's
Section-13 config blob. Connect it only with the native destruction/behavior
dispatcher. The name-table descriptor at 0x4C8980 is a pair of callback-record
pointers; its first pointer merely spells `HsL` in little-endian bytes and is
not an argument signature. Levels: Flood (21), WChannel (45). Do NOT fake the
trigger.

### 3. Non-player float behavior — large; blocked on common mover/type-vtable scheduler
✅ **Spawn placement corrected:** Section 13 stores signed 8.8 fixed X/Y in
`pos_data_1` and Z in the low half of `pos_data_2`. Explicit nonzero Y is
preserved for airborne/offset spawns; Y=0 actors and buildings are placed on
the terrain surface, reproducing the missing collision/behavior settling pass.
Type-52 `flag` records are hidden behavior/camera anchors, not scenery. X/Z no
longer collapse onto the origin after toroidal wrapping.

The remaining path is not a spawn-placement toggle. Section-12 Sub-C exists on
38 types, while its animated-wave byte is enabled on only 12; runtime mass
above 99 selects terrain even for those records. Retail invokes that same
Sub-C from several distinct update callbacks, with different component,
Sub-A/Sub-B, attitude, effect, and mass ordering. `FUN_0041F1C0` is recovered,
and `common_mover::sub_c` now retains its exact detached arithmetic while the
player path reuses it. Its generic reachability still belongs to the current
type-vtable callback and live task, not to Sub-C presence or a generic
“floats” flag. Applying it indiscriminately would animate static
mines/projectiles, pass the wrong mass, double-run controllers, and skip
collision/integration order.

Before enabling non-player ride behavior, close the type-vtable update map,
dynamic allocation paths, common mover/task scheduler, pitch/roll/controller
state, and the relevant callback-specific ordering. Then dispatch the authored
Sub-C from those proven callbacks using their exact zero-versus-attached-mass
argument and validate representative ground, wave, heavy-cargo, and static
types. The already-complete `EntityManager::from_level` placement path is not
the owner of this work.
- Files: entity behavior/task scheduler, callback dispatcher, common mover,
  Sub-C runtime.

### 4. Remaining authored terrain-light callers — bounded static programs live
The bounded mesh, recovered scan-depth-relative software fog, 120 canonical transition
sprites, exact 625-entry rotation lookup, base shade, underwater shade
reduction, indexed affine Gouraud palette lookup, and ordinary particle-light
presentation path are now in place.

The two static callers of `FUN_00441200` have now been separated. Model-program
opcode `0x8E` belongs to the Main Base/Working Factory staged progressive-death
path (`FUN_00419B50 -> FUN_00419C90 -> FUN_00419D90`) and is already live
through `StagedEffectInterpreter`, `BaseFactoryProgressionEvent::ModelEffectStage`,
and `emit_common_explosion_bundle_raw`. The other caller is
`FUN_004281A0`, the global timed static-terrain-object damage/effect scheduler.
Only action kinds 2/3/4/5 whose authored event byte is `0x1E` or `0x3A` use
this writer. The currently admitted kind-0 program contains event `18` and
correctly takes the separate `FUN_004410B0`-family path.

The extraction half is complete. `v2k_game::static_kind_catalog` losslessly
retains all 32 executable `0x2C` records, all 11 referenced damage profiles,
all seven terminated programs, all three radial payloads, the opcode-13 blob,
and kind 27's opaque auxiliary data at their retail VAs. Pointer closure,
program termination, corpus hashes, and the independently recovered collision
response words are regression-tested. Every program-bearing kind used by the
selected high-resolution worlds is therefore represented without borrowing
kind-0 behavior.

The admitted kinds 0, 1, 3, 8, 9, 10, 11, 27, 28, and 29 now use that catalog
directly rather than a parallel phase enum. Cursor/delay state is retained per
node, the original FIFO membership is captured at update start, and each node's
ordered effects are applied before the next current-static lookup. This closes
the previous whole-FIFO deferred-batch mismatch and keeps radial children out
of the current pass. `FUN_00427950`'s burned-state branch is also exact: a
non-kind-10 object still filters and may consume its chance draw, but cannot
deduplicate or restart once terrain bit `0x08` is set; an accepted already-
burned kind 10 clears only that bit and wrapping-increments the live Section-10
attribute.

Kinds 1 and 8 run the short effect-18/sound-90/burn program `0x004C9978`;
kind 28 reuses kind 0's exact program `0x004C9918` under its own profile and
chance gate. Kinds 3 and 11 run program `0x004C9A60`; kind 10 runs its
non-radial sibling `0x004C9AB8`. Their time-zero event 30 is connected to the
existing exact `FUN_00441200` common-explosion bundle with model header `+0x08`
retained as a separate source extent from collision radius `+0x0A`. At 500 ms they submit
effect 18 and set the burned bit; kinds 3/11 then dispatch radial template
`0x004C9878`, while kind 10 terminates. Shared-RNG ordering, current-slot
reacquisition, and the immediate kind-10 terrain transaction have focused
tests. Kind 9's opcode-13 direction-table scatter and kind 29's ordinary
class-79 event are live. Matched retail/demo FIFO, opcode-6, wrapped-distance,
and full-frame bodies close kind 27 too: its repeated effect-18/sound-90 lead-in
reaches an inclusive `0x1400` focus-spring check plus sound 62 at 2.0 seconds,
then a second check, burn, and its exact radial tail at 2.1 seconds. The first
visible full-frame command is staged on the following frame. All seven
extracted terminated programs are therefore behaviorally closed. The isolated
health-pack/radar run remains optional visual/audio acceptance, not an
admission gate.

Do not connect `FUN_00441200` to every animation or effect event. Remaining
unknown or future callers still require ownership of every opcode callee and
state transaction before becoming live. Preserve each event byte: the admitted
static path is specifically class `0x1E`, while a future scheduler event
`0x3A` must remain class `0x3A` rather than being collapsed into that helper.
Also do not treat `FUN_00465840` as a terrain transform: it is a
projectile/effect query wrapper around `FUN_00465870`.
- Files: static-object action catalog/scheduler, effect callers,
  `gl_backend.rs` terrain-light submission.

### 5. Chase-camera shared target + spring — retail behavior closed, port visual smoke pending

The eye side, horizontal handedness, base distance, and Active Camera heading
phase are recovered. The port now adds 0xFA raw units along the tracked
entity's stored forward basis (not its velocity), enforces
`focus.z - eye.z >= 0x100`, and advances the independent eye/focus states with
the exact `FUN_0040F740` signed-Q31 integer spring. Raw signed-word positions
preserve the torus seam automatically. Deterministic tests cover the tuning,
leash limits, first pose, and seam crossing.

Active gameplay now also follows `FUN_0044FFA0`'s terrain-aware branch through
`FUN_0040ED10`: the sea/terrain anchors, half-cell rearward ray, three-cell
clearance samples, Section-9 static-model collision radii, and object-kind
11/28 exclusions are ported without changing the shared spring. Menu and
cinematic callers retain their null-terrain path.

The accepted 2026-07-30 matched captures
`20260730-170241-active-camera-minimum`,
`20260730-170610-active-camera-default`, and
`20260730-170942-active-camera-maximum` validate settings 0/6/10 and the
recovered horizontal distance formula. Across more than 36,000 retained
samples, every resolved value of `FUN_0040ED10` parameters 3/4 is zero; there
is no hidden setting-dependent vertical lift. Duplicate/tick-stable eye-clearance
minimum/median values are 2/765 raw at Minimum, 450/777 at Default, and
330/806 at Maximum. Retail therefore approaches within 2 raw units (about
0.008 cell) of the water surface at Minimum. Do not add an unconditional Y
offset or infer monotonic vertical clearance from the setting.

The routes are comparable through the principal Tab handoff but diverge during
the later human-flown VTOL legs, so their late values are pose-normalized
retail envelopes rather than point-for-point setting order. The 2026-07-24
port report of occasional ground-edge clipping now requires only a matched
port-side visual smoke comparison at the same setting and pose. Reopen a
specific projection, rear-coverage, or terrain-context discrepancy if that
comparison fails; do not retune the closed retail target policy.

### 6. Wrap-aware AI distances — small (when combat lands)
When entity AI / projectiles are ported, distances/aim must use `world::delta`
(shortest-path across the seam), mirroring `FUN_00425370` / `FUN_00424650`.
Not needed until there is combat.
