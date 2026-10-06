# V2000 HUD Rendering Pipeline

Verbatim move of former GAME_MECHANICS section 15 during the 2026-08-24 cohesion split.

## 15. HUD Rendering Pipeline (CONFIRMED)

### Architecture: Mixed 2D Compositor and Scene Objects

The persistent left status orb is an ordinary 2D sprite compositor in
`FUN_004292B0`; the six-slot weapon gauges and the selected resource/model use
FGDK scene objects. Gameplay messages use the authored sprite-font path. The
same HUD routine calls `FUN_004494A0` to rasterize the right terrain radar
directly into the framebuffer and `FUN_0044AFF0` to plot nearby entities; the
models 235..239 Targetter/crosshair path is a separate overlay. The older
all-3D interpretation conflated those subsystems.

Retail reloads every active OVL after a Display -> Resolution change
(`FUN_0044E1F0` -> `FUN_00493A40`). Level-3 variant 0 therefore supplies the
low orb at base `(7,165)` in 320x240, while variant 1 supplies the high orb at
`(20,350)` in 640x480; variants 2/3 retain the identical high sprite pixels
with 800x600/1024x768 anchors. Its recovered sprite order is global 500
(outer), 502 (fuel), 503 (hull), conditional digits or 504/505 status panels,
501 (inner), 506 (center shade), then 499 (final grid). Fuel uses a 58-pixel source with a
5-pixel cap and 43-pixel active region against controller fuel `0..200000`.
Hull uses an effective 59-pixel height, 5-pixel cap, and 44-pixel active region
against health `0..40000`; damage first draws the trailing visible value, then
decrements it by `max((visible-health)/6,800)`, while healing snaps upward.
Low bars flash from `(DAT_004DB1B8 >> 18) & 7`; that accumulator advances only
by the gameplay callback's 125,000-us-capped update delta.

These layers retain their Section-3 framebuffer operations and fixed shade:
all have flag `0x04` and therefore use palette row 28. Sprites 500/501/504/505
are masked (`0x05`), bars 502/503 and center reflection 506 are additive
(`0x15`), and final grid 499 uses `source + destination/2` (`0x0D`). In particular, black
texels in 506 add zero while its gray texels brighten the scene underneath;
drawing it with conventional alpha incorrectly creates the port's solid black
patch.

There is no hidden black backing draw under either globe. Sprites 500/501 carry
the left shell's authored opaque black pixels. The terrain radar's mask value
`0xF` deliberately leaves destination pixels untouched, and its entity pass
only plots markers. Any apparent shell holes must therefore be investigated as
material decode/sampling/layering errors rather than repaired with an invented
disc.

The retained `20260719-191529-radar-fullscreen-map` runtime trace also settles
the radar frame: it is a fixed north-up world view and never consumes player
yaw. Signed wrapping 8.8 deltas use +X toward screen-right and +Z toward
screen-up. Marker range is
`d = 2*isqrt((dx*dx >> 2) + (dz*dz >> 2))`. The `d < 0x4000` test selects the
normal `sin_q15[d >> 2]` projection and blinking marker variant; it is not a
range cull. At and beyond `0x4000`, retail substitutes saturated Q31 sine and
pins the marker toward the globe rim. The M screen likewise presents the fixed
full 256x256 world (X left-to-right, Z inverted vertically), with no
recentering, rotation, pan, zoom, or discovery mask.

The completed `20260719-220951-radar-resources.json` snapshot closes the
authored-resource side of this path. At the high-resolution tier the globe is
96x96. System-level-3 Section 14 records 0..2 contain, respectively, a 9,216
byte full-resolution shade grid (FNV-1a64 `050E7DC11506ECE9`), a 36,864 byte
48x48 grid of four signed `(x,z)` samples per cell
(`29CC61647BA2F525`), and a 2,304 byte 48x48 edge mask
(`0559F4559EB25648`). Radar palette entries originate as Section-7 RGB555;
retail expands them into its live RGB565 table by shifting the red and green
lanes left once without duplicating the new green low bit. The M-map layout is
`(x=2,y=7,w=464,h=464)` in the 640x480 virtual canvas. Each loaded entity type
selects its centered native-size map icon with the `u16` sprite selector at
type-record `+0x70`; zero means no icon.

The port now parses these tables, rebuilds the shared 256x256 terrain raster
and packed coverage contribution, draws the north-up HUD globe and markers,
and uses the same raster for the staged nearest/bilinear M-map image and linked
entity icons. Two fidelity boundaries remain explicit. Partial globe masks
average authored terrain samples with the existing RGB565 destination pixel in
retail, so the current standalone RGBA texture can only approximate those edge
pixels; mask 0 and mask 15 remain exact. The M-map's right-side status text and
grid are no longer open as a blanket gap: Craft/Trophies and their live counts
are implemented, while Building/new-ship text remains gated on controller
`+0x1A6`. `FUN_0044A5C0` bakes the grid into the shared raster itself by
overwriting a cell with palette index `0x21` when
`(x & 31)==0 || ((z & 31)==0 && x!=0 && z!=0)`. The staged nearest and
interpolated M-map passes both sample that same raster. The independent
model-based Targetter overlay is handled separately.

The default selected item captured in the July 14 fire trace resolves to global
model 115 (`hudweap`) with sprite 518 as its 2-D fallback. Retail advances the
model's persistent spin by `(min(dt,125000)*58000)>>20`, evaluates
`FUN_0042A2E0(0x9000,0x0C00,spin)`, and draws it at level-3 layout point 12
before the orb sprites. The fallback uses point 11; point 13 supplies carousel
spacing. `FUN_0042A6F0` uses `abs(offsetY)*0x104+3000` through display width 640
and `(width-640)*0x10` above it. This is the selected overlay tier's logical
320/640/800/1024 width, never the physical host viewport or the port's final
Native/Stretched/4:3 transform. Feeding a 1920-pixel desktop width produced raw
depth 20480 and the tiny-weapon regression; normal variant 1 must use width 640
and raw depth 3000 at zero carousel offset. On selection changes,
`FUN_004292B0` seeds the signed carousel displacement from point 13's vertical
spacing, shortcuts last/first wraparound to one step, then approaches zero by
`1 + abs(offset)/4` per accepted HUD draw. It submits the adjacent occupied
descriptor before the selected one. The two `FUN_0042A570` sprite calls receive
`min(abs(offset)*3,100)` and its complement as centered size percentages
(raw zero is the full-size bypass); `FUN_0042A6F0`'s Section-8 model branch
intentionally ignores that size argument.

The selector-1 descriptor has byte `+0x0C = 7`, so bit zero takes the infinite-
ammo branch: no digits are drawn; `DAT_004CA6B0` advances
`0,16,32,...,96,100`; and masked sprites 504/505 reveal from their outer and
inner horizontal clip edges at layout point 7. The retained Level-1 trace and
the finite branch at `FUN_004292B0` prove selector 2 is different: it has no HUD
model, uses sprite 519, starts at 200 rounds, draws three leading-zero decimal
digits from sprites 508..517 at layout point 6, and retracts the complementary
panels by 16 per frame. The port now drives selector-1/2 presentation,
switching, exact compact-list carousel motion, digits, and ammunition from the
live descriptor inventory. Other weapon descriptors remain outside this
cutover.

### Main Render Pipeline (`FUN_004556a0`)

1. `FUN_004554e0` — clear stale HUD entity references
2. `FUN_0044f400(8)` — set render mode (clear bit 8 of `DAT_004f72d0`)
3. Background fill color render
4. `FUN_00451fb0` — update player name from rank table
5. `FUN_00443260` — HUD display object update
6. `FUN_0042eca0` — save control states
7. `FUN_00448e30` — scene graph walk + text render pass

### 6-Slot Weapon/HUD System

`FUN_0044f830` iterates slots 0-5, calling `FUN_00450fc0(slot, enabled)` based on `g_rng_state+0x1C4` bitmask (6 bits).

`FUN_00450fc0` per-slot control:
- **Enable:** Set bit 1, clear bit 3 in `DAT_004d0f20 + slot*0x1C`, call `FUN_0042a860(0x57)` to spawn HUD sprite
- **Disable:** Set bit 3, clear bit 1

Master visibility flag: `DAT_004d1154` mirrors the enable/disable pattern.

### Building Status Variable Publisher (`FUN_00419630`)

Writes current values into the six optional u16 entity/model variables bound
by Section-12 Sub-M. These are not HUD sprite pointers:

| Runtime pointer | Source | Published value |
|-----------------|--------|-----------------|
| +0x98 | +0x68 | low u16 count |
| +0x9C | +0x04 | low u16 capacity/limit |
| +0xA0 | +0x00 | low u16 control value |
| +0xA4 | +0x60 / +0x08 | progress ratio (0-0xFFFF) |
| +0xA8 | +0x64 / +0x0C | second progress ratio (0-0xFFFF) |
| +0xAC | +0x6C / +0x14 | recovery ratio (0-0xFFFF) |

When `+0x5C != 0`, both `+0xA4` and `+0xA8` instead publish the shared
`+0x5C / +0x10` ratio.

### Status Ratio Calculator (`FUN_00419710`)

Maps `(current / max)` → `0x0000-0xFFFF`:
- Returns `0xFFFF` when current ≥ max (full gauge)
- Returns `0` when current = 0 (empty)
- Handles large values by right-shifting both by 4 until max fits in 16 bits
- Formula: `(current * 0xFFFF) / max`

### Shield Display (`FUN_00456bd0`)

Implements **drain-only display policy** at `g_rng_state+0x2C8`:
- Only updates to a lower value (damage drains visually in real time)
- Can reset after 1,000,000 ticks since last update
- Creates smooth "damage flash" effect — bar falls but doesn't flicker upward on regen ticks

### Text Message System (`FUN_00456900`)

Single-slot message at `g_rng_state+0x2C0` (3 × int32: `[string_id, sub_param, timestamp]`).

**Guard:** Only fires when `g_rng_state+0x296 > 4` (gameplay phase).
**Priority:** Message 0xD9 cannot be overwritten except by `string_id == 0` (clear).
**No queue** — only one active message at a time; new messages overwrite.

**String ID mapping (confirmed from call sites):**

| ID | Event |
|----|-------|
| 0xC6 | Class-14 session-zero `People left: %d`. Authored prefix `<*,3000, 4,30, 2>`; `FUN_00452270` case 2 fills `%d` from `DAT_004fecdc` selector 3 (`FUN_0042E210` remaining capability `0x400` + `0x800`, including factory Sub-M `+0x68`). |
| 0xC9 | Item collected/teleported aboard |
| 0xCD | Parameterized message (`%s`; formatter 10 resolves `sub_param` as a global Section-2 string) |
| 0xCE | Entity destroyed / shield fully drained |
| 0xCF | Regeneration/attack failed |
| 0xD0 | Launch event |
| 0xD1 | Weapon fired / charge complete |
| 0xD2 | `The Hive is now vulnerable` (`FUN_0041BEB0` unlock) |
| 0xD3 | Level/mission event |
| 0xD6 | Damage event |
| 0xD9 | **Suppressed** (never displayed) |
| 0xDA | Player killed by beam-in entity |
| 0xDB | Vehicle destroyed (collision) |
| 0xDC | "Your fuel tanks are full" |
| 0xDD | "Your ship has no damage" |
| 0xDE | Generic lethal-entity branch (requires entity byte `+0x64 & 0x20`) |
| 0xDF | Low health warning. Authored `<*,3000,14, *,15>Your ship is critically damaged`: layout 14 shares the layout-4 box then `FUN_00470f80`-centers because the preset is `> 9`; reveal `*` becomes 1 (no type-on); formatter 15 is `FUN_00452270` case `0xF`, emptying the line while `|DAT_004FED60 / 40|` is even and forcing the blinking cursor off. |
| 0xE0 | "Fuel low" warning |

**Parameterized sub-values for 0xCD:**

| Sub-param | Event |
|-----------|-------|
| 0x10F | Teleporter activated |
| 0x110 | Shields |
| 0x111 | Pickup counter incremented |
| 0x112 | Beam power set |
| 0x113 | Turbo/hover upgrade |
| 0x114 | Fuel refill |

For fuel operation `0x33`, string `0xCD` is
`<*,3000, 4,30,10>%s`; sub-parameter `0x114` resolves to `Extra Fuel`.
An accepted pickup also queues event `0x0C`, which maps to string `0xED`
(`Try changing to flight mode now`). A full tank instead queues only direct
string `0xDC` (`Your fuel tanks are full`).

### Deduplicated Gameplay Resource Events (`FUN_004568b0`)

Notification slot at `g_rng_state+0x2C4` with **bitmask deduplication** (`FUN_00437f60`):
- Each event sets bit `1 << (event_id & 0x1F)` in `param_1[3]`
- If the bit is already set, that event is not queued again
- `DAT_004cad80` maps event IDs `0..24` to resource IDs `0xE1..0xF8`
- Guard: fires **only** when `phase == 5` exactly (stricter than text messages)

The consumer is `FUN_00452CB0`: after drawing the ordinary direct-text slot at
`+0x2C0`, it passes this resource slot to `FUN_00437EE0` with the blinking-cursor
flag set. `FUN_00437EE0` dereferences the global Section-2 string pointer and
calls the common authored-prefix/typewriter renderer `FUN_00452790`. These are
deduplicated **text resources**, not sounds. Direct world-sound calls remain
independent (`FUN_0044F450`/`FUN_0044F480`, including fuel/repair pickup id `5`,
empty-VTOL id `0x31`, cargo transfer id `8`, and cargo-full id `2`). The shared
type-on tick is a third concern: it dereferences physical global Section-11
slot 0 (`*DAT_004FE64C`), centered at half gain and native rate, with
`DAT_004F72E8`'s strict
`>2`-tick throttle. The nearby `0x2D` is a layout baseline, not a sound id.

**Notification event ID mapping:**

| ID | Event |
|----|-------|
| 0 | Neutral/UI |
| 2 | Damage taken |
| 3 | Hive destroyed (`0xE4`: "Fly down the hive to go to the next world") |
| 4 | Entity killed |
| 6-7 | Weapon pickup (variant) |
| 9 | Collision/new entity |
| 0xA | Hive still locked (`0xEB`: "The hive can only be destroyed once the alien creatures are dead") |
| 0xB | Empty-VTOL automatic return (`0xEC`: "You cannot fly without any fuel") |
| 0xC | Fuel collected |
| 0xD | Shield collected |
| 0xE | Hive unlocked (`0xEF`: "It is now possible to destroy the alien hive") |
| 0x10 | Attract Attention initializer hint (`0xF0`; authored meaning unasserted) |
| 0x11 | Cargo hold capacity hint (`0xF1`) |
| 0x13 | Teleport |
| 0x14 | Launch |
| 0x15 | Type-68 weight hint (`0xF5`) |
| 0x16 | Beam/vaporise |
| 0x17 | Vehicle destruction |

### Screen Coordinate System

HUD uses **Q16.16 fixed-point fractions** for positioning:
- `0x10000` = 1.0 = full screen dimension
- `0x8000` = 0.5 = half screen
- `0x20000` = 2.0

Entity screen positions at `+0x96/+0x98/+0x9A` (int16 world coords). HUD slot display params stored in `DAT_004d0f20` at stride `0x1C`.

Render resolution at `g_rng_state+0x27C` applies resolution-dependent scaling to HUD elements.

**Throttled warnings:**
- Fuel low: triggers when `fuel < 10000`, throttled at 70 ticks
  (`abs(DAT_004de854 - g_default_param) > 0x46`), then submits direct text
  `0xE0` and physical global sound slot `0xC4 / 4 = 49`.
- Low health: `FUN_00446640` uses the health captured at controller-callback
  entry, requires the controlled entity's type-flag bit 0, and triggers
  strictly below `0x1389`, throttled by
  `abs(DAT_004de858 - g_default_param) > 0x3C`; it submits direct text `0xDF`
  (`Your ship is critically damaged`) and physical global sound slot
  `0xB4 / 4 = 45` at centered Q16 gain `0x8000` (one half). There is no
  separate damage-magnitude branch: a large hit that leaves entry health
  `>= 0x1389` does not cue, and a same-tick solid contact that crosses the
  threshold becomes eligible on the next controlled-player callback because
  the snapshot is taken at entry. Both cadence words are process-global and
  survive level/player replacement in the Rust port. Direct `0xDF` is the
  layout-14 centered, case-`0xF` blinking `Your ship is critically damaged`
  line, not a type-on warning. This branch does not test the dying bit; the
  player controller remains bound throughout the captured three-second wreck
  dwell.

**Recurring low-health plume:**
- This is adjacent to the warning in `FUN_00446640` but has no cadence gate or
  50-Hz latch. Every eligible controlled-player callback below the same strict
  `0x1389` entry-health threshold consumes two RNG words and attempts an
  allocation, including while the fixed 200-record pool is full.
- Signed-8.8 origin is `position + [jitter_x,0x14,jitter_z] -
  body_forward_q31 * 0x5A`, with each jitter
  `((rand16 >> 9) & 0x7F) - 0x40`. The exact player body basis is required;
  renderer float matrices are not an equivalent source for the Q31 narrowing.
- Strictly above the raw sea plane, `FUN_00441670` selects class 20 with input
  velocity `[0,0x50,0]`. At or below it, a third RNG word adds another X
  jitter and class 42 receives `[0,0x190,0]`; the class descriptor adds its own
  `+0x190` Y bias. The source entity handle is retained.
- After the attempted allocation retail ORs entity `+0x84` bit 3. Its only
  identified consumer is under a separate generic entity-flags bit-31 branch
  that the controlled type-46 player does not enter, so it is not evidence for
  a second player emission path.

### Targetter Is Not a Minimap

The right-side sphere is the independent Targetter subsystem, not a map
projection. Its draw callback composes dynamic global
Section-8 models 235 (`target`) / 239 (`targetgreen`) with crosshair models
236..238 according to live acquisition flags and the selected target entity's
model/pose. Those flags come from distance, visibility/hit, and aim-convergence
tests; drawing a static sphere would not reproduce it. Config word
`DAT_004D0148` gates only the selected-entity ring block at `0044DB93`; the
result-bit crosshair block at `0044DCA2` still runs. "Portable Radar" and
"Mini Radar" remain pickup item names, not evidence of a persistent minimap.

The first accepted selector-`0x3C` player/type-61 contact sets the retained
Targetter capability, queues direct `0xCD/0x10F`, resource event `0x13`,
positional sound 5, and deferred pickup removal. A duplicate remains live and
plays sound 5 at full gain with fixed playback rate `0xAAAA`. The capability
survives ordinary in-memory world replacement. The port now also reproduces
`FUN_0044CF30`'s active-program installation at controller `+0x22C`, ordered
candidate gates, authored collision exact/near selection, current/prior timers,
signed-word terrain convergence, and models 235..239. Exact/near probes and
the selected-entity ring consume `entity_pair_to_world` (pair-census Euler
or published Q31 body matrix). Using pair-census orientation alone silently
dropped Type-17/47 locks — the terrain crosshair still ran, so the red
`target` / green `targetgreen` ring never appeared around insects. Unresolved
both ways stays fail-closed. `DAT_004D0148` / settings Targetter still gates
only the ring, not the crosshair. `FUN_0044CFE0` really
zeros the distance/step fields on pickup; the first `FUN_0044D0F0` update turns
the zero step into `0x14` before continuing convergence. Both currently
recovered player weapon profiles use the ballistic family; unknown projectile
methods remain unsupported instead of silently acquiring a guessed trajectory.

Selector `0x3E` is the adjacent Turbo capability transaction in
`FUN_00445A90`. Its first accepted contact sets controller `+0x197` bit 1,
queues direct text `0xCD/0x113`, plays positional sound 5 at native rate, and
defers pickup removal. A duplicate leaves the pickup live and plays sound 5 at
fixed rate `0xAAAA`, with no text. The campaign-owned bit survives ordinary
in-memory world replacement and clears on frontend session reset. Ordinary
VTOL `FUN_00445310` consumes it by setting Sub-G `+0x3D`, multiplying manual
lift by 3/2, and raising the height-attenuation threshold from `0x800` to
`0xC00`. The model callback also exposes that runtime flag, but no visual blur
effect is claimed in the port until its presentation consumer is proven.

### Time trophy and countdown

`FUN_004539D0` obtains controller `+0x1F0` and `+0x1EC / 1000` from
`FUN_0042F100`. `FUN_004292B0` at `00429509` renders the trophy and clock
only when state equals 1. Both disappear immediately when the timer expires,
the world is completed, or a world failure enters results state 5. The active
`0:00` half-second from the timer's `+500` bias still draws both.

The trophy uses global model 138 at raw depth 2000, orientation
`FUN_0042A2E0(2 * cargo_spin,0,0)`, and the same `42A520` model callback as
cargo. Its screen anchor is global Section-1 point 40; the `%1d:%02d` clock
uses global font 0 at point 41. These are system-3 local points 27/28 and are
absolute screen positions, not offsets from the status orb:

| Tier | Trophy centre | Clock baseline |
|---|---|---|
| 0 | (30,15) | (45,12) |
| 1 | (60,30) | (90,25) |
| 2 | (75,38) | (113,32) |
| 3 | (60,30) | (90,25) |

The native HUD copies global Section-0 words 0..3 into its model context,
then replaces the two projection-origin words with the trophy point. System2
owns global Section-0 base zero; its high-tier focal lengths are 512, 640 and
820. The trophybody's raw Y extent is 0..114 at depth 2000, so the tier-3
lens can extend its top above the retained y=30 origin. The callback at
`42A520` supplies only animation channels; it adds no anchor or scale offset.

Above the original output sizes, High Native uses normal 640-tier art pixel
metrics for this trophy's model lens before applying the shared HUD readability
scale. Its selected-tier model/text anchors remain together; at 4K the normal
model and glyph pixels grow by 3.6. Original outputs, Low and fitted modes
retain their existing selected-canvas projection. This is a port presentation
policy for modern outputs, not a claim of retail raster acceptance.

The port's `gameplay_hud::time_trophy` owns this presentation and submits it
before cargo, weapons and the status orb. It consumes the existing time
controller, so pausing, completion and expiry cannot race a second clock.
The shared 42A520 cargo/trophy policy must traverse the linked hierarchy:
global 138 (`trophy`) has two vertices and no faces; its child 139
(`trophybody`) supplies 52 vertices and 91 triangles. A zero recursion budget
silently draws only the empty root even though model138 is resident. The HUD
linked submission now uses the ordinary bounded hierarchy traversal, retaining
the authored child transform and material lookup.
This top-left HUD trophy is independent from a hidden Type61 pickup, whose
amount-zero selector `0x3F` restores hull/shield on actual contact. Neither
enters the cargo HUD. Type61 has capability `0x40`, so the beam candidate gate
at `FUN_00446C80` rejects it for lacking `0x1000`; `FUN_00445A90` case `0x3F`
applies the reward without appending a Sub-J attachment. Cargo models come
only from the actual ordered Sub-J rows through `FUN_00443260`, using each
entity type's primary model rather than its live model override. A renderer
fixture that directly supplies model 138 as a cargo slot does not represent
retail pickup or cargo behavior. The
[controller contract](LOADING_TRANSITIONS.md#per-level-time-trophy-countdown-static--port-live-2026-08-10)
owns timing, campaign awards and NoCD05's bounded active-timer evidence.
The explicitly requested physical-pickup clock stop adds a shared tick/HUD
gate to the port's timer; [Player shield](PLAYER_SHIELD.md#requested-pickup-clock-policy)
records its user-directed status and the static retail distinction.

### Key Globals

| Global | Address | Content |
|--------|---------|---------|
| `g_rng_state+0x2C0` | dynamic | Text notification: {string_id, sub_param, timestamp} |
| `g_rng_state+0x2C4` | dynamic | Notification event: {resource_id, 0, timestamp, bitmask} |
| `g_rng_state+0x2C8` | dynamic | Shield display value (drain-only) |
| `g_rng_state+0x2CC` | dynamic | Shield display update timer |
| `g_rng_state+0x1C4` | dynamic | 6-bit active weapon slot bitmask |
| `DAT_004d0f20` | `0x4D0F20` | 6 HUD slot records (stride 0x1C) with render flags |
| `DAT_004d1154` | `0x4D1154` | Master HUD visibility flag |
| `DAT_004cad80` | `0x4CAD80` | Notification event ID → resource ID table |
| `DAT_004fe64c` | `0x4FE64C` | Global resource/sprite table |

---

