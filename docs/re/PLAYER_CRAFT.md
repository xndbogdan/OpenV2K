# V2000 Player Craft: Modes, Fan, Gun Aim (ground truth)

RE'd 2026-07-02 (multi-agent workflow, adversarially verified). Supersedes the
hover-mode rows of [CONTROLS.md](CONTROLS.md) (see Corrections at the bottom).
VAs are ImageBase 0x400000. "Controller" = the control struct reached via
`g_rng_state+0x27c` (resolver `FUN_00443af0`, game_logic.c:34314).

## 1. Mode state machine (TAB)

There is **no animated physics transition** — TAB is a discrete
**movement-style swap**. The old "+0x198 fan/morph sub-state" note was wrong.

| Field | Meaning |
|---|---|
| `+0x86` | **movement style**: 0=hover, 1=fly (body 0/2); 4=hover, 3=fly (body 3); 2=special/amphibious (body 1). Init 0 (game_logic.c:33724); copied from vehicle-descriptor byte 18 by the 99-dword copy in `FUN_00443560` (:34055-34060); a change raises `+0x20d=1` (:34187) |
| `+0x198` | **vehicle body/config selector 0-3** (discrete; descriptor offset 0x124; setter EXE `0x447c70` also raises `+0x20d`). NOT a morph counter — never incremented |
| `+0x20d` | mode-change request flag |
| `+0x88` | fuel (int 0..200000) |

Sequence on TAB (`FUN_004442B0` → sets `+0x20d=1`, or `+0x70=1` if linked to
another entity):

1. `FUN_00446640` consumes `+0x20d` (game_logic.c:35838-35847) and calls the
   current mode-record's **morph handler** (`record+4`).
2. Morph handlers map body→new style via jump tables:
   **A `0x447970` (→fly)**: `{body 0→1, 1→2, 2→1, 3→3}`;
   **B `0x4479f0` (→hover)**: `{0→0, 1→2, 2→0, 3→4}`. Player = body 0, so
   **hover style 0 ↔ fly style 1**.
3. Handler calls `FUN_0040c6b0(entity, new_style)` (game_logic.c:3485-3490):
   selects mode record `0x4cd940 + style*0x48`, stores it at **`entity+0xb8`**
   (the per-entity behavior block), calls activation fn `record+0x40 = 0x4464f0`
   which rebuilds the behavior/render node. The swap is instant.
4. Integrator dispatch each frame (`FUN_00446640` :35785-35794) indexes the
   **static .data** table `DAT_004cd8d0` (`{reader, integrator}` pairs) by the
   mirrored style at `*(*param_1+0x28)`:
   styles 0/4 → hover `FUN_00444CF0`; 1/3 → fly `FUN_00445310`; 2 → `0x438050`.
   Reader is `FUN_004445E0` for all styles (bindings identical in both modes).
5. **Fuel empty in fly** → `FUN_00445310` presents positional global sound
   `0x31`, submits notification event `0x0B`, and sets `+0x20d=1`
   (game_logic.c:34986-34994) → auto-morph back to hover next consume. The
   earlier note that treated `0x0B` as the sound selector was wrong.

**Empty-fuel Hover→VTOL is an immediate callback round trip, not a separate
input gate.** User observation correctly established that pressing TAB with an
empty fuel bar leaves the craft in skimmer. Disassembly now closes the
mechanism: `FUN_004442B0` unconditionally raises `+0x20d`; `0x447970` commits
style 1; then the first `FUN_00445310` callback sees `+0x88 < 1`, plays
positional global sound `0x31`, submits deduplicated resource event `0x0B`, and
raises `+0x20d` for the return to Hover. `DAT_004CAD80[0x0B]` selects global
Section-2 string `0xEC`, authored as
`You cannot fly without any fuel`. Normal keyboard processing supplies one
callback request per TAB down edge. The port collapses the short-lived
VTOL→Hover presentation into `VehicleModeToggleOutcome::RefusedNoFuel`, while
preserving the exact sound and once-per-level resource notification. This path
is distinct from the throttled direct low-fuel string `0xE0`; direct string
`0xDE` belongs to a generic lethal-entity branch and is not a fuel message.

**Initial/refill lifecycle correction (user-verified 2026-07-16).** A new
player starts with controller fuel `+0x88 = 0`, so VTOL is unavailable until a
fuel powerup has been collected. Type `0x33` adds its authored raw value while
fuel is below 190001 and caps the tank at 200000. The Rust controller now starts
empty and exposes this exact acceptance/add/cap operation as a named pickup
stub; world contact/removal and refill are connected, as are the direct
positional pickup (`5`) and empty-VTOL (`0x31`) sounds. Accepted fuel pickups
now reproduce direct `(0xCD,0x114)` (`Extra Fuel`) plus deduplicated event
`0x0C` (`Try changing to flight mode now`); full tanks reproduce direct `0xDC`
without sound or removal. The strict 70-tick non-positional low-fuel warning is
wired with both global sound `0x31` and its replaceable direct text `0xE0`.
Fuel reaching zero during a powered callback does not switch immediately: the
following empty-fuel VTOL callback raises the return request, after which Hover
is selected and further VTOL attempts are refused until another refill.

**Runtime topology confirmation (2026-07-16).** The keyed
`skimmer-heli-mode-gear` trace captured both TAB transitions. Style 0→1 changed
the behavior pointer `0x004CD940→0x004CD988`, component state `+0x28` 0→1, and
effective environment flags `0x10018→0x8` in the same 50 Hz sample; the reverse
transition was equally immediate. Across all 2,750 samples the player remained
entity type 46, active slot 0, root model 41 (`player4`), with the same four
model slots `[41,67,41,67]` and no child entity. Matching skimmer and fly entity
snapshots also contained the same 39-handle topology. Mode-specific hull and
landing-gear presentation therefore belongs to `player4`'s conditional
Section-8 hierarchy, not a model-slot swap or a separately spawned entity.

**Mode-joint recovery (2026-07-17).** Type 46's authored Sub-O bytes are exactly
`05 06 07 08`. `FUN_00409A80` resolves these through `FUN_0040A950` into the
zero-initialized per-entity joint array; `FUN_0040D320` is the model callback
that returns the global 50-Hz tick for index 0 and the live u16 joint word for
every nonzero index. The style callbacks then supply the two mode targets:

- `FUN_00444CF0` (style-0 Hover) writes `{1,1}` to Sub-O runtime target words
  `+0x10/+0x14`;
- `FUN_00445310` (style-1 VTOL) writes `{0,0}`;
- their shared tail `FUN_00444F60` calls `FUN_00420A50`, which advances callback
  words 5 and 6 independently as
  `current += ((target*0xFFFF-current)*(dt_us<<12)) >> 31`, with the final
  halfword add wrapping exactly like retail.

At the normal 20-ms step, deployment begins `0, 2499, 4903, ...` and stalls at
`65509` (`0xFFE5`) because the positive signed shift truncates before 0xFFFF.
Retraction begins `65509, 63010, 60606, ...` and reaches exact zero. The retained
words are not reset on TAB. Real `0X3XX.OVL` materialization proves the split:
word 5 deploys the hull's side/gear panels (68 vertices / 104 body triangles at
zero; 80 / 120 when deployed) and moves the two sidepods from raw
`[+100,-20,-100]` / `[-100,-20,-100]` to approximately
`[+138,-48,-100]` / `[-140,-48,-100]`; word 6 rotates the authored fan,
shutter, and pod mount bases from their VTOL pose to the Hover pose, including
the `pl4enginesurround` quarter-turn. The renderer must therefore pass both
words into the entire linked hierarchy and must not add a second game-side fan
rotation.

Callback word 4 is independent of vehicle mode. At zero, PLAYER4 instances two
global-51 `pl4gatgun` children; setting it to one selects global-49
`pl4cannon`. The former provisional port table changed word 4 on TAB and thus
silently changed the equipped guns as well as reversing the gear endpoint.

The callback-zero clock drives the equipped plasma guns' travelling colour
glows. Phase, colour variants and presentation ownership are in
[PLASMA_WEAPON_PRESENTATION.md](PLASMA_WEAPON_PRESENTATION.md).

**Shared fan drive and audio recovery (2026-07-18).** Both vehicle callbacks
reach `FUN_00420A50`, which passes the signed controller throttle at `+0x288`
unchanged to `FUN_00420920`. The latter tests only zero versus nonzero. Space
and right Shift therefore produce identical fan behavior, while pressing both
cancels the controller channel to zero and follows the idle/release path. The
same zero-initialized 0x4C-byte Sub-O state drives fan rotation and sound:
runtime dword `+0x20` is the RPM/rate state and `+0x24` is the gain envelope.
At the normal 20-ms tick, powered RPM rises by `5000` to an excess cap of
`0x8000` (about 140 ms); release falls by `2500` per tick (about 280 ms). Gain
jumps to `0x10000` under power, then falls by `2500` per tick above `0x4000`
and by `312` below it until the `0x2000` idle floor (about 880 ms total).

Retail continuously updates two positional looping voices rather than
retriggering one-shots. Global 47 (`sound_047.wav`, direct PCM) uses rate
`state20*3/2`, hence 1.5x idle through 2.25x powered. Its creation call receives
the full envelope; steady updates use `envelope/6`. Global 31
(`sound_031.wav`) stays at 0.5x and uses gain `envelope/2 + 0x2000` for both
creation and updates. `sound_038.wav` is unrelated. The DirectSound path
changes frequency, volume, and pan only; there is no recovered EQ/filter—the
perceived filtering is the two-layer pitch/gain blend. The visual fan joint
uses the prior-tick RPM excess plus `0x6000`, so animation and audio must share
one retained fan-drive state.

The successful 200-Hz acceptance capture
`runtime_re/captures/local/20260718-045345-fan-audio-envelope.jsonl` independently
confirmed one unbroken looping/non-auto-GC voice for each PCM after startup.
Global 47 remained at 1.5x idle and 2.25x powered; global 31 remained exactly
0.5x. Space and right Shift reached the same powered plateaus, while both keys
produced a zero controller throttle and the complete release envelope. At the
observed 8-ms retail delta, attack reached maximum RPM in about 130–135 ms,
release reached idle pitch in about 265–270 ms, and the gain tail reached its
idle floor in about 875 ms (each boundary is quantized to one sampled frame).
Both live voices shared positional attenuation and pan. The port now replays
this delta-scaled integer state and mutates generational physical voices without
resetting their playback cursors while audible. Static `FUN_0044C970` evidence
also preserves retail's separate culling rule: outside the positional radius
the logical records survive but their physical buffers stop; re-entry restarts
the samples with the current steady-state gains.

## 2. Input channels (reader `FUN_004445E0`, motion vector `S+0x280`)

Reader decompiled fresh via `tools/ghidra_decompile_gunpitch.py` (function is
absent from the committed bulk decompile).

| Channel | Written by | Notes |
|---|---|---|
| `+0` turn | LEFT(−)/RIGHT(+), joystick X, mouse X (`+200` per mickey) | default Joystick **Relative** mode writes the duty-cycle-scaled keys and the joystick term directly; `FUN_00444B90` is only the Joystick Absolute-mode target-bearing helper. Mouse X is added in both modes |
| `+2` pitch | UP(+)/DOWN(−) authored amplitude **0xD80**, passed through the default Relative sensitivity/duty quantizer; S(+)/X(−) scale **0x900**, clamp ±0x900; S/X suppress only SPACE's separate powered coupling; joystick Y (forward +); mouse Y (`-200` per mickey towards the player) | one shared channel — in Hover positive depresses the gun; in VTOL it pitches nose-down. At default sensitivity 10 and 20 ms, a held arrow is ±2303 (`0x08FF`), confirmed live on 2026-07-16 |
| `+4` vertical | console pads only, ±0xA00 | no PC device binds it |
| `+8` throttle | SPACE / left mouse / joystick button 2 (+), RSHIFT(−), clamp 0x10000 | the three positive sources share both SPACE descriptors |
| `+C` fire | Enter / right mouse / joystick button 1 | foreground retail capture; all feed one held trigger |

[CONTROLS.md](CONTROLS.md#mouse-joystick-and-pointer) records the mouse and
joystick bindings, the reader's order and the Absolute-mode arithmetic.

The powered pitch coupling lives in the fly integrator: it is gated on the
dedicated positive-thrust binding, no manual S/X this frame, and the
auto-stabilize option `S+0x20C > 1`; cap 0x600, gain body-pitch/6. The two July
16 acceleration traces recorded self-righting value 10, while the accepted
July 30 submerged-launch traces recorded value 9 and the executable's pristine
frontend default is 1. These settings are not interchangeable calibration
evidence: at the captured 8 ms/default-sensitivity cadence, sustained Up+Space
settles at pitch 10469 in mode 10 but 9995 in mode 9 (about 2.6 degrees
shallower). Mode 1 is a different regime again: the `> 1` gate suppresses
SPACE's opposing-sixth term, so sustained Up+Space can pass pitch `0x4000`,
turn body-up Y negative, and project powered lift downward. Terrain impact and
death after that inversion are therefore compatible with retail C, not
evidence for a missing global pitch clamp. A 2026-08-09 retail replay with Self
Righting 0 reproduced the reported sustained Up+Space inversion and ground
crash, closing the apparent regression as a settings-root mismatch. Always
match the retail and port Self Righting values before diagnosing vertical-lift
feel. The former port `config.json` settings-root mismatch remains the
cause of that observation. Current [settings persistence](SAVE_AND_SETTINGS.md#port-storage-and-migration)
imports those preferences and separates the native DWORDs from modern display
options; always compare the effective Self Righting value. The demo provides
an independent control: its input reader, `FUN_0041A650` recurrence,
`FUN_00444D00` player caller, `FUN_0041AFE0` lift/ceiling path, pristine mode-1
default, terrain response, and collision-damage chain are instruction-matched
to their retail counterparts apart from relocated addresses and controller
layout. It supplies no alternate clamp or impact protection.

## 3. What each integrator does with the channels

Angles are 16-bit, full circle = 0x10000.

### HOVER `FUN_00444CF0` (skimmer)

- **pitch channel → GUN BARREL joint only** (`gunpitch_decomp.c:481-489`):
  `*barrel -= channel/2`, clamp **[−0x800, +0x3000]** (−11.25°..+67.5°).
  The step is applied once per callback and is not scaled by the frame delta
  (`0x00444D73`: truncating division, signed word addition, then the clamp),
  so a mouse aims by the same angle at any frame rate. The port used to scale
  it by the 50 Hz tick and now matches retail.
  UP/S are positive channel inputs and therefore **depress** the gun; DOWN/X
  are negative and **elevate** it. `FUN_00424650` reduces to
  `projectile_y = sin(barrel)` at level attitude, so this is projectile aim as
  well as joint animation. The key labels are counterintuitive but conclusive
  from binding records `0x4C2CC8/0x4C2CF0`, slot map `0x4CDBD8/0x4CDBE0`, and
  reader instructions `0x4446AC..0x444760`.
  The barrel pointer `weapon_state+8` is initialised (game_logic.c:2848-2853)
  via `FUN_0040A950` (:3335-3344) to an entry of the entity's **sub-model joint
  array** `*(*(model_state+0x4c)+8) + (idx−1)*2` — a genuinely separate
  variable from body pitch `+0xA4`. The body does **not** pitch and does
  **not** translate from this channel in hover.
- **throttle channel → forward propulsion** (`FUN_0041e9e0`, motion slot +8)
  — SPACE is what propels the skimmer. Altitude is held by the hover-height
  controller `FUN_0041F1C0` (slot 0x10; reads pos/height + terrain lookups —
  it is the altitude PD, **not** a forward-thrust source).
- Spin node: hover writes `[[ctrl+8]+4] = motion[0]*0x34 / physics[0x14]`
  (EXE 0x444d18-3c) — **turn-channel-proportional** node rotation (steering
  vane/prop yaw). Fly writes the identical +8 node (game_logic.c:34968-34970).

#### Hover steering and planar drive (resolved 2026-07-13)

The static defaults are Joystick mode **Relative** (`settings+0x08=1`),
Absolute Mode **Half** (`+0x28=0`), sensitivity 10 (`+0x2C`). A fully held
left/right or up/down binding writes
`floor(floor(sensitivity*dt_us/15)*0xD80/dt_us)`: 2303 (`0x08FF`) at
20,000 us and the default sensitivity 10. The 2026-07-16 keyed capture measured
stable pitch values of exactly +2303/-2303; the port previously bypassed this
quantizer for arrows and therefore aimed/tilted about 50% too quickly.
Type 46's Sub-D divisor is 28, giving the exact wrapping yaw path:

```text
turn_rate = trunc_zero(motion.turn * 52 / 28)
yaw_step  = i16((turn_rate * (dt_us >> 2)) >> 15)
heading   = wrapping_i16(heading - yaw_step)
```

At 20 ms, RIGHT changes heading by -652 and LEFT by +653 raw angle units. The
one-unit asymmetry is the x86 arithmetic right shift. `FUN_00444B90` instead
builds a target bearing from two analogue axes and is not entered by the
default Relative keyboard path. The port implements it for the Joystick
Absolute setting, where it also takes the arrow keys.

**Renderer-basis correction (2026-07-14):** `FUN_00413F70` stores contiguous
lateral/up/forward vectors. Transposing that legacy layout into the port's
row-major `world = M·local` convention gives `Ry(PI/2-heading)`, not the old
camera-era `Ry(heading+PI)` compensation. Thus the captured first-world heading
`0x4000` is identity/forward +Z; LEFT's increasing heading turns the nose toward
world/screen −X and RIGHT's decreasing heading toward +X. Draw, hover force,
primary muzzle, cargo placement, and model collision now share this one basis.

Type 46's identical A/B/D payloads in all four `?X3XX.OVL` variants are
Sub-A `i16 {1400,-300,3000}`, Sub-B `i32 {0,500}`, and Sub-D
`1C0000000100000000000000`. `FUN_00420450` persists a per-spawn forward target
of `3000..3298` raw. `FUN_0041EB50` removes only velocity projected onto the
body-right basis (9 raw units at 20 ms). While SPACE/RSHIFT is held,
`FUN_0041E9E0` uses only the throttle sign, accelerates toward the signed target
with coefficient 1400, and uses -300 after overspeeding. Releasing throttle
does not call propulsion: the skimmer coasts without a Sub-F or the generic
`FUN_004236D0` drag/cap path.

That absence does not disable common environment physics. The 2026-07-14
`skimmer-start-stop-turn` live trace recovered effective environment flags
`0x10018`; bit 3 makes `FUN_0040E100` call `FUN_0044EC60`. In the captured first
world its runtime wind mode/vector are zero, Section-13 drag strength is 3, and
player self-mass is 100. The no-wind fallback applies, per signed-i16 velocity
axis, `v -= (((dt_us * drag_strength) / (mass << 3)) * v) >> 15)`. At 20 ms the
factor is 75; the observed release sample matches that fixed-point damping.

#### Authored wind and calm water

The first-world zero-wind sample is not a player rule. Both Hover
(`0x10018`) and VTOL (`0x8`) enter the same `44EC60` owner. `44EB40` copies
Section-13 `+9C/+A0/+A4` into signed-word authored/current wind vectors,
`+94` into drag strength, `+98` into the signed height cap, `+90` into mode,
and `+80 != 0` into the sea-side selector. A zero vector forces mode and
height cap to zero. Nonzero mode uses the current vector; mode 2 alone
updates it through `44EBD0`.

`44EBD0` adds the active frame delta to wrapping process-global
`DAT_004D04C8`, then scales each authored signed word by
`retail_sine_q15(phase >> 7) >> 15`. It runs once before world actor
callbacks, never once per actor. `44EB40` does not reset this phase on a
level change. The executable initializes this .data word to `0x12345678`
(retail PE file offset `0xCECC8`, bytes `78 56 34 12`); starting at zero
would shift every mode-2 world relative to retail. The port retains it beside the process RNG in `WorldFx`,
preserves it through effect teardown and transactional forks, and installs
the current vector in the manager before player/actor updates. Mode 0/1,
pause, and menu frames do not advance the gust phase; wind consumes no RNG.

For nonzero wind, `44EC60` compares `(static_sea_y < entity_y)` with `+80`.
On the other side it retains the ordinary subtraction drag. On the selected
side it probes bilinear terrain half and one wind-vector upstream, subtracts
the higher probe from entity Y, and caps that clearance at `+98`. A negative
clearance returns before all wind/drag writes. Otherwise each relative-wind
word is `(vector * (clearance * 32) >> 15) - velocity`, narrowed with retail
wrapping arithmetic. Its projections through the freshly rebuilt F70
lateral/forward basis add signed roll/pitch deltas with coefficients
`-0x4000`/`0x1000`; linear velocity receives the factor-scaled relative wind.
The angle words change without a second basis rebuild in that frame. The
player returns those words to `PlayerCraft`, retaining the original F70
basis through solid/static/actor collision, camera, vector effects, weapon
submission, and rendering. The following mode callback's force packet also
reads that retained prior matrix; it must not reconstruct it from EC60's
newer Euler words. The next F70 boundary publishes those words normally.

Normal-tier authored controls include steady wind in worlds 22
`[1000,0,0]`, 32 `[-300,0,-300]`, 45 `[0,0,32000]`, and 46
`[1000,1000,1000]`; worlds 33 `[0,0,20000]` and 47 `[2000,0,0]` use
mode 2. Native-player regressions exercise both styles against these actual
descriptors, with controlled terrain to separate wind from terrain forces,
plus the 33→46→47 retained-phase sequence.

Section-13 `+84` is the distinct wave-animation gate, not the presence of a
rendered sea plane or Sub-C's water-target flag. `41F470` (Hover lift),
`45920` (VTOL/terrain-attitude probes), and `129B0` (whole-body contact)
retain the static sea when the gate is zero; ride/attitude probes still take
the greater of terrain and sea. Cistern/world 30 authors `+84=0` and raw
sea Y 3768. The player therefore has static-water lift and classification
there, while retaining the independent Sub-C cargo and buoyancy policies.
The canonical Cistern regression verifies both movement styles are
independent of the wave clock and that entry occurs at the static sea.

The 2026-07-16 `skimmer-controls-keyed` capture closes the remaining apparent
forward/reverse friction mystery. Held SPACE accelerated by +26 raw velocity
units per 20 ms tick while held RSHIFT accelerated by -27. The common signed
right-shift drag also removes one extra raw unit on one sign at particular
velocities (for example -1923→-1918 exactly matches the formula above). Turning
preserved most planar momentum, so the visible sliding and slightly icier
reverse coast are authentic consequences of retail fixed-point arithmetic,
not evidence for a missing second friction coefficient. The port keeps that
signed asymmetry deliberately.

The live order is heading → lift → lateral correction → propulsion, all using
the previous frame's Q31 body basis; `FUN_00413F70` rebuilds the basis only
after the behavior callback. Common gravity and the bit-gated environment
wind/drag callback follow, then `FUN_00412DA0` integrates signed 8.8 position words with
`pos += ((dt_us >> 5) * velocity) >> 15`, wrapping as `i16`.
`FUN_0044FE20` caps the behavior delta at 125,000 us before these operations.
This is the specialized player `FUN_00444CF0` path: direct disassembly calls
Sub-C at `0x444DE0`, Sub-B at `0x444EF1`, and Sub-A at `0x444F30`. Generic
entity callbacks do call A before B, but they are not the player's Hover path.

#### Submerged VTOL-to-Hover launch ordering (resolved 2026-07-30)

The matched guided captures
`20260730-182504-submerged-mode-launch-up.jsonl` and
`20260730-182610-submerged-mode-launch-down.jsonl` close the apparent
underwater "launch impulse." There is no transition-only force. TAB changes
the live behavior pointer from VTOL `0x004CD988` to Hover `0x004CD940` and
effective environment flags from `0x00000008` to `0x00010018`, while retaining
the entity pose and the same component root/table. The first Hover C/B/A force
pass therefore consumes the extreme VTOL body basis once and produces the
launch in the corresponding direction.

The following common owner phase runs `FUN_0040E640` /
`FUN_0041EC70` through the freshly rebuilt current basis. Its effective
`0x10000` bit clamps pitch and roll to +/-`0x1800`: the UP capture records
pitch `19220 -> 6144` (roll `0 -> -16`), while DOWN records
`-17166 -> -6144` (roll `0 -> 33`) on the next 8-ms update. Later values track
the sampled wave/terrain gradient rather than easing geometrically toward zero.
The port therefore keeps TAB as a pure style swap, applies the first Hover
force through the retained basis, then shares the exact common
terrain-attitude planner before the next force frame. VTOL remains unrestricted
by this Hover-only effective flag.

The port implements the fully-held Relative keyboard path for turn and pitch,
including live sensitivity, the mouse and joystick terms, the Joystick
Absolute-mode mapping, and the exact local `Random_Next` recurrence.
Event-timestamped partial-frame key duty cycles remain a bounded input
fidelity gap.

Player SubA allocation now consumes `20450` from the same process RNG as actor
construction, after the player's SubD allocation and before33BD0 terrain gate
helpers and Section13 actors. Native save and campaign loads follow this order;
world teardown does not reseed the stream. The retained SubA target is passed
to PlayerCraft without a second draw. The portable compatibility load explicitly
constructs its player component after its separate preview reconstruction;
it is not a native constructor-order claim.

Native force, collision and presentation consumers retain the exact F70 Q31
basis. EC60 changes Euler words after that rebuild; it does not rebuild again.
The updated words feed the next frame while the current frame retains the
matrix. Authored calm-water surface selection and the integer wave helper now
share this policy rather than depending on the renderer's water toggle.

#### Hover lift, waves, cargo mass, and common buoyancy (resolved 2026-07-16)

The persistent player is type 46, cumulative Section-12 entry 44. Its exact
Sub-C controller bytes are
`96 00 7D 00 00 00 90 00 64 00 C8 00 01 01 00 00`: base clearance 150,
lift range 125, strength `0x00900000`, near-boost range 100, damping range
200, wave flag 1, body-offset flag 1. All distance values are signed-8.8 raw
units (divide by 256 for port world units).

`FUN_0041F1C0` computes the gap above its sampled surface after subtracting
the base clearance. Below the lift range it adds a range-scaled, near-surface
boosted impulse (capped at 200) along the craft's body-up axis. Within the
damping range it removes velocity directed into the surface, but preserves
upward velocity. On the wave path, only penetration of the underlying static
terrain is position-corrected. The hull therefore lags moving water and can
glide visibly above a falling crest; a live gap such as 228 is not a constant.

`FUN_004185C0` sums attached entities' `u16 +0xB0` mass. Sub-C samples waves
only while attached mass is at most 99; above that it targets terrain/seabed.
Type 68 `weight` has mass 200. Common environment callback `FUN_0040E100`
also changes underwater buoyancy at total mass 151: player mass 100 receives
depth-proportional lift, while player plus weight mass 300 sinks. This proves
both halves of the weight mechanic independently of visual inference.

The common branch is now ported after both player movement styles. Hover first
runs its type-specific C/B/A controller; VTOL first runs the recovered
specialized lift callback and the normal type-46 attitude callback. Both then enter `FUN_0040E100`, followed by
bit-gated `FUN_0044EC60` drag and `FUN_00412DA0` signed-word integration.
Environment flag value 4 disables both gravity and the underwater branch.
Otherwise it first applies `vy -= (dt_us * 0x300000) >> 31`. Its surface is not
the animated wave or the bilinear hover probe: it reads the signed
terrain-height byte at the entity's integer X/Z cell, multiplies it by 32, and
takes the greater of that and the static Section-10 sea word. Sub-C byte 12
must be nonzero and the strict signed depth `entity_y - surface_y` must be below
`-100` raw units.

Total mass is the wrapping `u32` sum of the entity's `u16 +0xB0` mass and its
Sub-J attachments, then compared as signed. For total mass below 151 and
post-gravity `vy < 0x600`, depth lift is
`vy -= ((dt_us << 13) * (4 * depth)) >> 31`; the negative depth therefore
produces upward force proportional to immersion. Mass 151 or greater receives
only `vy += ((dt_us << 13) * 300) >> 31`. At 20 ms that restores 22 raw units
after gravity removed 29, so a stationary weighted craft still accelerates
downward. Both regimes then damp every signed velocity word with
`v -= ((dt_us << 10) * v) >> 31`; the arithmetic right shift intentionally
rounds negative components differently. VTOL no longer duplicates this with
the port's old float gravity, 0.97 planar friction, vertical damping/speed cap,
or float position integration. Its authored Sub-G manual lift, height assist,
fuel burn, attenuation, and previous-basis projection are now ported as a
separate fixed-point phase. The normal player's pitch, yaw, and bank recurrence
is exact; the alternate terrain-target attitude branch remains provisional.
The temporary post-integration terrain/wave floor has been removed because
retail force response deliberately permits brief water
penetration after a high drop.

#### Beam attachment and weight interaction (resolved 2026-07-13)

The beam controls are plain **C** (collect) and **D** (drop) while LShift is
up. Raw binding records put C/D in `FUN_00471DE0`'s required-down bytes and
LShift in its required-up bytes; the former `C+LShift` interpretation had the
polarity backwards. The callbacks operate on the player's signed byte `+0x20E`:
an idle C edge stores +80 and an idle D edge stores -80, repeating the same
direction does not refresh the timer, and the opposite direction cancels it.
`FUN_00446640` consumes whole milliseconds (`dt_us / 1000`) and dispatches only
when a step strictly crosses the remaining magnitude. Exact equality subtracts
to zero without firing. The port retains the 80-ms signed timer and whole-ms
countdown, but treats equality as expiry: its deliberate 16-ms frame limiter
otherwise creates the pathological sequence `80,64,48,32,16,0` and loses every
beam command, whereas retail's variable callback cadence ordinarily crosses the
endpoint.

Collection first requires carrier capability bit 1. The player's Section-12
`id_field` at `+0x08` is 5, and its Sub-J payload authors five attachment slots.
That count is the maximum, not the fresh-game capacity. `FUN_00443560` copies
the profile block into controller `+0x74`; controller byte `+0x199` therefore
comes from executable data byte `0x004D0C55`, whose initial value is exactly 1.
`FUN_00418620` exposes `min(Sub-J maximum, +0x199)` slots, and the type-`0x3A`
"Larger Cargo Capacity" pickup in `FUN_00445A90` can raise `+0x199` later.
`FUN_00446C80` accepts candidates whose corresponding `id_field` bit `0x1000`
was copied to runtime `+0x64`, excluding already attached entities. Type 68 has
`id_field=0x1040`. It compares independently quantized, wrapping signed-8.8
deltas with strict limits `|dx|,|dz| < 0x400` and `|dy| < 0x800`, then chooses
the smallest `dx²+dz²` in entity-list order. Distance zero doubles as the
"nothing selected" sentinel, so a later eligible candidate may replace an
exact-overlap candidate. A successful attach enters the ordered Sub-J list,
hides the entity from independent world presentation/collision, and contributes
its live unsigned `+0xB0` mass to `FUN_004185C0`'s wrapping 32-bit sum. Drop is
LIFO.

The distance envelope belongs only to collection. `FUN_00446D30` passes the
player's Sub-J to `FUN_00418500`, which pops its last row and invokes the
`FUN_00443D30` callback installed by `FUN_00443B50`. None of those drop-path
consumers checks the collection distances. The former observation-based drop
gate was incorrect and is removed. [NoCD03](ACTOR_RUNTIME.md#nocd03-deep-water-cargo)
confirms a successful seabed drop and underwater Materialiser attachment;
its vertical delta is only 1221 raw units, so the absence of the `0x800` drop
limit follows from the complete static callback chain, not that sample alone.

The port implements that data-driven collection, timing, capacity,
visibility/collision exclusion, mass sum, LIFO removal, and the wave-to-seabed
hover switch. Its current partial drop presentation follows
`FUN_00443D30`'s target and timing. The
landing point starts at `carrier - body_up_xz*200`; when the carrier is less
than 400 raw units above that terrain sample, it additionally subtracts
`body_forward_xz*400`. Every multiply uses the signed Q31 basis, horizontal
word arithmetic wraps, and Y comes from the retail signed-byte integer
bilinear terrain sampler. Retail spawns a real type-93 entity there with
identity orientation. If the subsequent release callback returns zero,
`FUN_00408F00` copies the cargo's selected model into proxy slots 0/2, attaches
and hides the real cargo, then raises the proxy by exactly one signed-8.8 Y
unit. Positional global sound 8 then plays at the unraised target. A nonzero
callback return exits before both attachment and sound.
The corresponding `FUN_00416700`/`FUN_00416750` relation-membership bit
`0x1000` is now projected at collection, campaign restore, proxy transfer, and
stable release. This keeps carried cargo on Main Base abort's callback-free
route without treating the host backlink as a universal source of retail
state. Ordinary fresh Level-1 Type-9 cargo also owns the actual carrying-style
attach, None task, and both release/reselection callbacks described in
[Actor Runtime](ACTOR_RUNTIME.md#ordinary-type-9-cargo-callbacks). The player
cargo phase runs those synchronously before actor ticking; settling updates
the peasant's retained movement anchor, and the selected live owner continues
without adopting a forged fresh-birth receipt.
Static retail/demo comparison also proves that the borrowed model comes from
the cargo's live `0x2000/0x4000` selector bits. Rust now uses that selector and
refuses an unresolved slot without popping cargo or allocating a proxy, rather
than trusting the stale `model_index` compatibility view.

The player Type68 release is statically closed and uses its shared native
class0 owner. DC50 passes the behavior context as the second callback argument;
the packaged parent is the third. D1C0 reads context[0], terrain-aligns the
weight and reselects through the TypeDefault list, consuming one living
singleton RNG word and replacing Primary. The null attach hook preserves that
timer. Normal Type93 settlement copies release XYZ to both current+96 and
anchor+90 before the second release/reselection. The exact stack proof and
task-custody boundaries are in [Class0 Runtime](CLASS0_RUNTIME.md#type68-attach-and-release).

Type 93 is the Section-12 Materialiser behavior (L3 local 91/global 93,
material index 30). Its component stores the pre-raise position and a 32-bit
movement counter. `FUN_004091B0` runs after entity age is incremented but before
the terrain clamp. On a changed position, an even counter increments and emits
particle class `0x33`; it resets age only while the new counter is below 16. An
odd counter increments and zeroes velocity only when age is strictly greater
than 100,000 microseconds. The common tail releases only at age strictly
greater than 500,000 microseconds. Consequently the initial one-unit raise
emits one class-`0x33` particle, the same update clamps the proxy back to the
terrain, and a normal stationary drop hands the cargo back after the later
strict half-second crossing. Release copies the proxy position into both cargo
position histories, restores independent visibility, detaches it, and removes
the proxy in the same simulation pass, so no frame renders both. Class `0x33`
uses seven Section-3 frames `794,795,796,797,796,795,794`, lifetime 31,
animation rate `0x10`, radius 20, and draw scale `0x0800`. Successful collection
uses the same particle and global sound 8 at the cargo position. Type 68 then
emits deduplicated resource event `0x15`; executable table `DAT_004CAD80` maps
it to global Section-2 string `0xF5`, whose authored text is `This should make
your ship heavier`. A full currently unlocked attachment list follows the
separate failure branch:
positional global sound 2, replaceable direct-text slot `0xDA` (`Your cargo bay
is full`), and once-per-level resource event `0x11` -> global string `0xF1`
(`There is only limited space in your cargo hold`). `FUN_00452CB0` draws the
direct slot first and resource slot second. Both use the yellow Section-4 font
and their authored three-second/30-ms type-on prefixes; layouts 4 and 1 resolve
to percentage triples `(x=25, baseline=78, width=60)` and `(25,87,50)`. Only
the resource slot adds the 200-ms blinking `" _"` cursor. Their type-on sound
shares `DAT_004F72E8`'s strict three-tick throttle and physical global
Section-11 slot 0 (`*DAT_004FE64C`; `0x2D` in the routine is a layout
baseline). The port now implements this complete feedback contract without
conflating either HUD resource event with the independent positional sounds.

While the child remains attached, native Type93 also runs `FUN_00418640`:
its one zero-offset Sub-J row copies the clamped proxy position to the live
child and clears velocity. The [NoCD03 writer oracle](ACTOR_RUNTIME.md#nocd03-deep-water-cargo)
proves this occurs after the earlier child callback. Player beam commands run
early, but their tail-appended Materialisers now advance in the existing
post-actor phase, including pose publication and eventual release.

`FUN_004292B0` presents the selected weapon before the orb layers. Selector 1
uses spinning global model 115 (`hudweap`) with sprite 518 as its fallback and
the infinite-ammunition 504/505 status panels. The Level-1 selector-2 record
has no HUD model: it uses sprite 519, finite ammunition, and three leading-zero
decimal digits from sprites 508..517. `FUN_0042A6F0` computes the model depth
from the selected overlay tier's logical 320/640/800/1024 width, not the final
host viewport or the port's Native/Stretched/4:3 output transform. Variant 1
therefore takes the width-640 branch and raw depth 3000 at zero carousel offset;
passing a 1920-pixel host viewport here was the cause of the port's tiny-weapon
regression.

The selected resource is one entry in a compact occupied-descriptor carousel.
When its compact index changes, `FUN_004292B0` seeds `DAT_004DB1BC` from level-3
point 13's signed vertical spacing, with explicit one-step shortcuts for
last-to-zero and zero-to-last wraparound. Each accepted HUD draw then moves that
offset toward zero by `1 + abs(offset)/4`. While it is nonzero, retail submits
the adjacent entry first at `offset +/- spacing`, then the selected entry at
`offset`. The corresponding fourth arguments to `FUN_0042A570` are
`min(abs(offset)*3,100)` and its complement; nonzero values resize a fallback
sprite around its center, while zero is the full-size bypass. The Section-8
model branch ignores this size argument. The port now preserves that asymmetry
and the neighbor-before-selected ordering.

`FUN_004292B0` also presents the attachment list itself below the left orb.
For each currently unlocked slot it draws global sprite 533 (6x6, flags
`0x05`, masked at flat palette row 28). Variant-zero point 14 `(37,72)` is the
row origin relative to base `(7,165)` and point 15 `(10,0)` is its spacing. For
slot `i` of `n`, the model anchor is
`base + origin + (spacing.x*i - spacing.x*n/2, 0)` and the marker top-left is
`anchor - (sprite.width/2, sprite.height)`. An occupied entry reads the attached
entity's type at runtime `+0x58`, then bypasses live/Section-13 model overrides
and resolves that type record's primary Section-12 model at `+0x0C`. The model
uses raw depth 4000. Rigid 3D cargo uses
`FUN_0042A2E0(cargo_spin,0,0)`, where the independent
`DAT_004DB1A8` accumulator advances by
`(min(dt,125000)*20000)>>20`. The C loop supplies that common matrix for every
occupied entry, but user-verified retail presentation disproves the earlier
inference that planar people visibly spin: structurally camera-facing actors
remain upright and face-on while retaining callback `0x42A520`'s four-phase
walk/mirror animation. The marker/model row is submitted before the selected
weapon and orb layers. The port implements this full empty/occupied presentation
and the separate one-unlocked/five-authored capacity state.
Live play now dispatches retail type `0x3A` through the bounded type-61 contact
transaction. The raw controller byte at `+0x199` remains separate from the
Sub-J-clamped live capacity, so later authored comparisons retain the original
truncation behavior.

### FLY `FUN_00445310` (VTOL/heli)
Moved verbatim to [VTOL_FLIGHT.md](VTOL_FLIGHT.md) during the cohesion split.

### Fire (both modes)

The player controller owns 32 fixed 24-byte descriptors at `+0x2A8`.
`FUN_00445A90` accepts weapon selectors below `0x32`, scans
`DAT_004CDC08`..`DAT_004CDE48` for the matching 24-byte master, takes the first
slot that is empty or already holds that selector, copies that master when
empty, and stores `max(previous,incoming)` under the routine's signed
comparisons. Level-1 factory payload `0x0001F412` is selector `0x12` with stored
amount 500 (table record 20). Its descriptor grants infinite ammunition: 500
controls acquisition/auto-selection, not a finite shot budget. It auto-selects only when the incoming amount is strictly greater
than the previous amount. A valid duplicate is nevertheless accepted and its
Power Up consumed even when the amount does not increase. Manual next/previous
`FUN_004440D0` uses `441D0`/`44260` to walk occupied slots but skips a finite
zero count (`444129..444145`); the infinite flag accepts a stored zero count.
Neither this path nor exhaustion clears the descriptor selector. It remains
available for same-slot reacquisition and native save/restore. The HUD snapshot
`FUN_00443260` copies contiguous selector/count pairs, including zero counts;
`4292B0` draws the selected weapon and temporary carousel neighbor. An exhausted
weapon therefore leaves the selected HUD through the automatic switch and
cannot return through manual cycling, while its outgoing transition may finish.

Automatic post-shot exhaustion instead follows `FUN_00445290`'s selector graph
at executable VA `0x004CDE48`: `0x12→0x01, 0x0E→0x12, 0x0D→0x0E,
0x0C→0x0D, 0x0A→0x16, 0x13→0x0A, 0x1D→0x07, 0x1C→0x08`, with an unlisted
selector defaulting to `0x0C`. Missing and empty finite candidates are skipped
until an owned nonzero-finite or infinite slot is found. Selector 2 therefore
normally returns through `0x02→0x0C→0x0D→0x0E→0x12→0x01` to the infinite
default gun.

`motion+0xC` → `FUN_00444F60` → weapon handler `FUN_00444FA0` (:34755) →
shot creation `FUN_00424650` (:18187) with the craft as shooter. These shots
in the particle family are **not ordinary 0xCC entities and have no Section-8
projectile model**. Selectors 3/4/5 take the separate modeled entity branch
described below.
`FUN_004147A0` allocates a transient 0x2C intrusive record on the shooter's
`+0x38/+0x3C/+0x40` list; `FUN_00411400` submits it through
`FUN_00414870` → `FUN_0044E770` and frees it in the same update. This is why a
retail main-entity lifecycle capture shows no spawn or despawn when firing.

The default descriptor is weapon id 1 (`DAT_004CDC08`, record 0). Its cadence
byte is 8, giving an exact **8 × 20 ms = 160 ms** repeat interval, and its
sound word is global id **88**. A controlled retail capture confirmed both
Enter and right mouse invoke this same primary: a tap fires once immediately;
a hold repeats at exactly ticks `N, N+8, N+16, ...` (6.25 events/s); release
does not create another event. Every event resolves global sound 88 to physical
blob 7 (`sec11_3XX_000`, 48,944 PCM bytes) at fixed multiplier `0x19999`
(1.6× / 35,279 Hz). The descriptor's auxiliary-command flag causes one normal
class-1 record followed by a distinct class-15 record with zero direction/time
fields; its origin is refreshed from the shooter before dispatch, so it is not
a second muzzle shot. The sound is played once for the whole event. Projectile
class 15 uses raw speed 1 and particle class 32, not the bullet's class-1
particle.

`FUN_00444FA0` copies the selected descriptor dword into weapon-component
`+0x10`. `FUN_0044EA60` classifies that selector: `5`, `0xC`, `0xD`, `0xE`,
`0xF`, `0x11`, `0x15`, and `0x1F` are straight-ray; every other selector is
ballistic. Targetter consumes that switch plus `DAT_004D02C0` speed. Firing
emits recovered `FUN_0043F590`, `FUN_0043F6E0`, `FUN_0043F780`, and
`FUN_0043F7C0` `DAT_004D02C0` rows, including factory selector `0x12`
and Antidote selector7.

The factory machine gun's master at `0x004CDDE8` contains callback 15,
cadence 2 ticks (**40 ms / 25 shots per second**), decay 5, flags `0x07`,
sound 88 and HUD model 116. `DAT_004D02C0 + 0x12 * 16` (`0x004D03E0`)
is `[0,4000,2,0]`: ballistic speed 4000 and particle class 2. Its descriptor
at `0x004CC1A0` matches class 1 except for draw scale `0x400` instead of
`0x300`. It therefore shares `FUN_0043EF30` flight/underwater damping,
`FUN_0043DEB0` surface response, `FUN_0043F590` entity damage, packet
`0x004CBF70`, and `FUN_0043F800` static damage. The port now admits that
whole path. `FUN_00424650` emits one alternating shot per cadence event,
not a simultaneous volley or player-aim random spread. Its class-15 auxiliary
is unchanged. Sound is submitted once after the callback's catch-up loop,
even when several shots are due in one update; the port submits the same batch.

The retained 2026-07-20 pickup/fire trace closes the other Level-1 primary
without generalizing beyond it. Selector 2 installs descriptor
`02000000C800000004020F0504003C000000070204010000`: callback selector 4,
finite count 200, cadence byte 15 (**300 ms**), sound 60, and HUD sprite 519
with no 3-D HUD model. It emits particle/projectile class 3 (sprite 704, raw
speed 2000, lifetime 255) followed by the same class-15 companion. Selector 1
and selector 2 share one persistent A/B side-gun phase across weapon changes,
including visits to an unfireable selected slot.
The port commits one finite round only for an emitted selector-2 event; zero
rounds suppress the projectile, audio, and phase advance while the held cadence
continues. The event consuming the last round is still emitted, then the port
applies the recovered successor graph and rebinds the player weapon component
to the selected usable descriptor. A restored selected finite-zero descriptor
rejects firing without invoking this post-shot path; manual cycling skips it.
Selectors `1/0x12/0x1B` author the infinite flag, independently of their stored
acquisition count. A separate `444FA0` branch relocates a newly selected
descriptor to the occupied tail only for positive signed trigger-duration
`motion+0xC` and absent resolved pending replacement (`controller+0x6C`), when
the selected slot differs from `controller+0x5A8`. `445310`/`444F60` pass that
trigger word separately from frame elapsed; elapsed alone does not enable the
relocation. The firing visit seeds the infinite working count with 888, then
writes the post-`24650` count back: 888 with no shot, 887 after one emission.
Those ordering/count writes are not yet owned
by the port. `46640` additionally regenerates retained selector `0x1A` by
`elapsed_micros >> 11`, capped at 999, under state bit `0x200000`; that distinct
controller branch remains open. Removing zero-count descriptors would lose
these native reacquisition, persistence and regeneration semantics.

Retail observation closes selector 2's underwater presentation boundary.
`FUN_00424650` has no underwater firing guard: it attempts the selected class-3
record and class-15 companion first, commits the finite-ammunition counters
after both allocation calls, and submits sound 60 after the firing loop. At or
below the sea plane the player therefore still loses one round and hears the
firing cue, but no flare or muzzle particle survives visibly. The port condenses
that immediate class-3 underwater lifecycle to no queued particle without
suppressing the firing event, ammunition commit, or sound. This is
selector-2-specific; the class-1 machine-gun projectile retains its independently
recovered underwater flight.

PLAYER4 callback word 4 is also part of that descriptor contract. Selector 1
stores callback value 0 and materializes the two `pl4gatgun` children; selector
2 stores callback value 4 and materializes two `pl4tubegun` children instead.
Their authored +Z tips are raw `[+148,24,180]` and `[-148,24,180]` relative to
the hull. The port resolves the alternating A/B presentation against the active
pair, so changing to the Level-1 flare/tube gun no longer collapses its visible
stream back to the entity center. Factory selector `0x12` stores callback 15
and selects two `pl4biggatgun` children (global model 65 in `1X3XX.OVL`),
whose raw hull-relative tips are `[+136,20,140]` and `[-136,20,140]`.
Plasma selectors `0x0E/0x0D/0x0C` store callbacks 6/7/8 and instance two
`pl4plasmared` / `pl4plasmagreen` / `pl4plasmablue` children at
`[+140,24,0]` / `[-140,24,0]`. Their +Z tips resolve to
`[+140,24,120]` / `[-140,24,120]`. Each plasma mesh keeps the same type-14
A/B muzzle callbacks as `pl4gatgun` (operands 0/1). Descriptor `+0x14` is
nonzero (`0xFE/0xFD/0xFC`) and `FUN_00424650`'s extra-volley branch is only
selectors 10/22/26, so plasma alternates once per event like the machine guns.
All six pairs receive the shared barrel-elevation transform. Other
callback-word-4 branches remain unresolved gun-mount policies;
their meshes alone do not prove alternating rather than simultaneous discharge.
Player shots whose `DAT_004D02C0` class uses `FUN_0043F590`, `FUN_0043F6E0`,
`FUN_0043F780`, or `FUN_0043F7C0` now fire from the live descriptor plus that
table. Selector 8 emits class 5 (speed
2000, cadence 80 ms, sound 64). `FUN_0043F780` entity hits skip F610, skip
`FUN_00411250` when `+0x1D` bit 0 is set, and otherwise deliver packet
`0x004CBFD0` (`[6,0] / [2000,0]`) then set state bit `0x2000`. `FUN_00425590`
ignores channel 6 so `FUN_00411030` sees zero. Type-46 now applies that
zero-impact `FUN_00411030` before `FUN_00415040` (three RNG samples and
angular `-0x0400`; bit `0x80000000` fail-closes). Then `FUN_00415040` uses the
existing type-47/Base/Factory owners plus type-46 empty-link
`FUN_004484A0` (channel 6 filters to 4000) and Type-9's matching
channel-6 4000 against 1500 health, which is lethal `FUN_00410C10`
class 14. Types 6/8/17/47/66/67 author channel-6 multiplier 0. Nonempty cargo/type-63 links
stay fail-closed. Type `+0x82` plays through
`FUN_0044F450` at entity `+0x96` with scale `0x10000` when dying bit
`0x4000` was clear and the authored word is nonzero (first-world type 17
is 92; types 6/9/46/47/66/67 are zero). Retained native actor owners dispatch
vtable `+0x18` / `FUN_0040DA00` through their audited style `+0x20` words;
unowned target callbacks stay fail-closed. Mode 3 then runs `FUN_0043FF10` and `FUN_0043F920`: `+0x1D`
bit 0 skips the write, otherwise `FUN_00427DE0` is `FUN_00433720` of the
`FUN_00427410` tile-center 8.8 words at probe `+0x1c/+0x20` with flag 1.
The extra cdecl `0x7D0` / `g_default_param` arguments are unused. F920 does
not call F610; `FUN_0043FF10` still recycles the parent.
Selector7's terrain-clearing binding is now statically established by the
locally retained `V2000-nocd.exe` and the exact `DAT_004D02C0`/particle tables:
selector7 selects class6, whose surface slot is `FUN_0043E1A0`, entity slot
is `FUN_0043F7C0`, and static slot is `FUN_0043F950`. `43E1A8` pushes zero
into shared `FUN_0043E1C0`; `43F972` calls `FUN_00427E00`, which forwards
the refined tile-center words to `FUN_00433720(x,z,0)`. This proves the
clear-bit policy without asserting retail firing acceptance. `43F7EC` calls
`FUN_00411320` with static packet `0x004CBFE8` (`[2,0]/[1000,0]`, both
provenance dwords zero). That wrapper clears entity state `0x2000`, submits
type `+0x84` sound unconditionally when nonzero, invokes vtable `+0x1C`
(common `FUN_0040DA60`, current style `+0x24`), then runs `FUN_00411030` and
`FUN_00415040` for a nonnull packet. Production selector7 now emits class6
at its authored speed2000/80-ms cadence, finite inventory and sound64, with
no auxiliary command. Entity and static hits never emit F610; suppression
skips11320/the clear write but still consumes the particle. Native actor
owners retain separate cure dispatch: Run Away+24 invokesC690, attached
Capture People4C8080/80C8/8110/8158 invokesD040 (infection+20 is null), and
audited completion/death leaves have null hooks. Cure preserves the primary
timestamp and skips10EB0's accepted-hit sound/class5 suffix; its channel2
packet still passes each target's authored damage filter. Unsupported target
allocations/styles and unowned D040 continuations report explicit boundaries.
The local player uses the common type vtable: `FUN_00410090` installs
`0x004C8A30`, and `FUN_00438080` copies it to `0x004DC6E0` for types46/51,
overriding only update slot+20 with421590. Its +18/+1C remain DA00/DA60.
All seven Player Control frames `4CD940 + 0x48*n` have null +20/+24 hooks;
the local hit wrapper therefore owns model selection, the optional +82/+84
cue,11030, and direct15040 with the actual hull and empty484A0 attachment
list. The null hooks themselves change no controller, gun, inventory, or HUD fields;
a lethal checked tail separately enters the native dying constructor below.
`4CDAA0` is style4's +40 activation word, while447AF0 appears at style5
`4CDAB8/4CDABC` (+10/+14 contact). It validates the controller, tests the
signed Q12 normal/velocity dot product, requests haptics and a throttled wreck
burst, and writes Y velocity1000; it is not an infected/cured callback.
Normal-tier Type46 authors model slots `[41,67,41,67]`, so changing2000 still
selects the authored slot even though both living slots display model41.
Unresolved styles, remote impact/damage, nonempty controller links, and
filtered-zero source46 selector23/DB feedback remain explicit preflight boundaries. Class19 meteor
cure delivery remains unowned and does not fall through to generic damage.
A normal-tier optimized production
smoke grants selector7 through inventory and fires its actual gun callback:
nine shots reach class6 sprite submission on27 frames, including a captured
Level1 frame. The full frontend/Intro2/Level1 run retains19 captures and1024
Playing frames with no runtime issues. This does not establish matched retail
firing acceptance. Selector29 retains its existing F6E0/class78 firing profile;
its41850 callback body remains a separate boundary.


`FUN_0043F6E0` copies the integrated endpoint, calls F610, always
dispatches `FUN_00410EB0` (no `+0x1D` bit-0 skip) with packet `0x004CBFA0`
(`[2,0] / [10000,0]`) when that dword is present, then plays sound 90 through
`FUN_0044F450` / `FUN_004575A0` with scale `(rng16 >> 3) + 0x10000`. Null
`+0x20` rows skip `FUN_00410EB0` instead of following retail's null deref.
Solid-static `FUN_0043FF10` writes the refined probe, then `FUN_0043F890`
runs F610. Class 4 then runs `FUN_00441850`'s sound-90 prologue and
`FUN_00441A50` (class-1 debris, count `(0x13880 >> 10) * scale >> 16` min 1,
velocity table X / `abs(Y >> 1)` / Z, no allocator abort) and F890 sound 62.
Class 54 then runs that prologue and `FUN_004410B0` class `0x27` (packed
table dword `* 2` plus independent `Z * 2`; unsuppressed count 1; suppressed
`DAT_004F72CC >> 12`; at or below sea, class `0x2E` and Y-100) and F890
sound 62. Class-4 `FUN_004566E0` template `0x004CD818`, class-54
`0x004CD7F8`, and class-77/78 `FUN_00456710` stay fail-closed.
`FUN_004292B0` uses the descriptor `+0x10` high word as the HUD fallback
sprite without a 518/519 whitelist.

The 2026-07-14 high-frequency fire capture also recovered the descriptor's
side-gun selector. Fifteen stable events alternate variants
`0,1,0,1,...` once per actual 160-ms shot. The phase survives trigger release
and re-press and an Enter→right-mouse handoff; it is weapon state, not
per-input state. Variant 0 pulses one runtime weapon joint to 65535 and variant
1 the other, then stores the opposite selector for the next event. The authored
`player4` hierarchy supplies the corresponding visible geometry: two direct
`pl4gatgun` children at raw attachment positions `[+120,24,0]` and
`[-120,24,0]`. Their model-local +Z extreme-centroid is raw `[16,2,140]`, so
the level-pose visible tips resolve through the authored mirrored transforms to
`[+136,22,140]` and `[-136,22,140]` relative to the hull. These are presentation
origins only; they must not replace the center-origin transient records below.

The complete pointer map closes those pulse words as PLAYER4 callback words
**3 and 2**, respectively. Weapon runtime `+0x04` points at component
`root+0x34` (joint/callback 3), so selector variant 0 / port channel A pulses
word 3. Runtime `+0x00` points at `root+0x32` (joint/callback 2), so variant 1 /
channel B pulses word 2. `FUN_00424E70` subtracts
`descriptor[0x0B] * (elapsed_us >> 4)` from both words with a zero floor. Both
proven descriptors store rate 5; the observed 160-ms gap therefore leaves the
previous side at `65535 - 50000 = 15535` when the other side is pulsed. The
port retains and publishes this exact state after successful shots only; an
empty-ammunition cadence attempt neither pulses a joint nor advances A/B.

Projectile class 1 (`DAT_004D02C0`, 0x10-byte stride) supplies raw speed 4000,
particle class 1, and ordinary entity type 0. Particle class 1 is record 1 in
the exact 96 × 0x34-byte array at `DAT_004CC138` (ending at `0x004CD4B8`) and
renders global sprite **705**: a 5×5
indexed/shaded yellow-orange luminous dot, scale `0x0300`, lifetime 200 retail
ticks, update/collision mode 3, flags 5, radius 40. It belongs in the
lightweight particle/projectile path, never `EntityManager`.

Free-fire direction = **body Euler** `+0xA2/+0xA4/+0xA6` via basis
`FUN_00413D40` (:18393-18396), combined with the live barrel joint `g` as
`direction = body_up*sin(g) + body_forward*cos(g)`. Thus the hover barrel does
deflect the projectile; it is not visual-only, and both visible `pl4gatgun`
children share that same elevation. Both class-1 and class-15 simulation
records start at the shooter's exact entity center (`+0x96/+0x98/+0x9A`)—the
dispatch has no model mount/muzzle-vertex offset. The presentation layer may
retain the selected authored gun-tip displacement while collision follows the
center-origin trajectory. Class 1 adds the full shooter velocity to its raw
4000 forward velocity. Class 15 resolves to particle class 32: nine frames
`843,844,845,846,847,848,870,871,872`, scale `0x0300`, lifetime 18 ticks,
with a +35 raw Y offset. It is the orange flash → grey smoke effect, emitted
only while the shooter center is strictly above the water plane; its velocity
is refreshed from the shooter after each generic integration step. The
auto-aim branch (`weapon_state[9]==0`) still uses target lead.

#### Class-1 collision and impact dispatch (retail)

After `FUN_00440120` integrates a particle and runs its class update callback,
class 1's collision mode 3 follows a strict, short-circuiting order:

1. swept entity/model collision (`FUN_0043F980`);
2. swept solid tile-model collision (`FUN_0043FF10`);
3. water-state transition callback; then
4. endpoint terrain-height callback.

A hit/deleting callback stops the remaining stages. The entity sweep probes
the displacement midpoint with radius `max(segment_length/2, 40)`. Candidate
entities use their active Section-8 model's unsigned 8.8 collision radius at
header `+0x0A`; zero excludes that model. The shooter is excluded only through
projectile age 50, so an older shot may hit its owner. Candidates retain active
list order and the first authored collision-program hit wins, rather than the
nearest analytic contact. The `extra_count * 4` bytes after each model header
form that collision program; they are separate from the visible face stream.
The sweep repeatedly halves the successful interval, tests the earlier half
before the later half, and stops at the projectile's raw radius 40. If neither
refined half reproduces a coarse hit, retail cancels it. The target may change
between successful probes, but effects and damage still use the integrated
endpoint.
The class-1 callback applies damage type 2, amount 2000, at the integrated
endpoint. `FUN_0043F610` then uses the strict split `sea_y < impact_y`:

For a kind-0 static target, amount 2000 exactly equals the channel-2 threshold
and therefore filters to zero under retail's strict-greater comparison. The
port revalidates the deferred mutable cell before submitting that proven no-op;
ordinary primary bullets do not ignite trees and consume no shared RNG there.

| Region | Class | Sprite sequence | Rate / draw scale | Life / radius | Initial raw velocity |
|--------|------:|-----------------|--------------------|---------------|----------------------|
| Strictly above sea | 34 | 23-entry sequence using 895..905 | `0x20` / `0x0400` | 80 / 20 | `[0,80,0]` |
| At or below sea | 42 | 787 | `0x10` / `0x0100` | 64 / 80 | `[0,400,0]` |

Class 42 rises with underwater drag and remains a bubble only while fully
submerged. Class 34 uses the same recovered 23-frame sprite sequence as the
meteor trail but has its own lifetime, scale, and callback.

Surface response selectors are level data, not hard-coded terrain colors.
`FUN_0042EA30` copies Section-13 dwords `+0xA8..+0xC4` into the eight-entry
ground selector map and repeats `+0xA8` across the water map; unless authored
ground selector 0 is 7, it replaces that entry with `+0xAC`. At collision,
the low three material bits from the nearest terrain cell
(`(u16(coord)+0x80)>>8`, X-major) index the appropriate map.
`FUN_0043DEB0` resolves the first-world selector subset as follows; every class
in this table has lifetime 30 and radius 20:

| Selector | Particle class | Sprite frames | Rate / draw scale | Projectile result |
|---------:|---------------:|---------------|--------------------|-------------------|
| 0 | 7 | 716..723 | `0x10` / `0x0200` | four-particle burst; delete |
| 1 | 8 | 706..715 | `0x20` / `0x0200` | four-particle burst; delete |
| 2 | 9 | 945..952 | `0x20` / `0x0200` | four-particle burst; delete |
| 3 | 10 | 887..894 | `0x20` / `0x0200` | four-particle burst; delete |
| 4 | 7 | 716..723 | `0x10` / `0x0200` | four-particle burst; delete |
| 5 | 11 | 724..731 | `0x20` / `0x0200` | four-particle burst; delete |
| 6 | 13 | 732..741 | `0x10` / `0x0300` | one stationary particle; continue |
| 7 | 12 | 824..833 | `0x40` / `0x0C00` | one stationary particle; delete |

The table at executable address `0x004CD718` does not stop there. Later authored
worlds use the same ordinary four-particle/delete path with these additional
selector mappings:

| Selector | Particle class |
|---------:|---------------:|
| 8 | 59 |
| 9 | 10 |
| 10 | 71 |
| 11 | 70 |
| 12 | 72 |

Selectors 0..5 and 8..12 play positional global sound 83 at full volume and rate
`0xE000 + ((rng & 0xFFFF) >> 2)` (`0xE000..0x11FFF`) at the projectile
endpoint. Selectors 6 and 7 first call `FUN_0043E060` at that endpoint: RNG bit
`0x1000` chooses sound 26 when set or 27 when clear, again at full volume, with
rate `0xC000 + ((rng & 0xFFFF) >> 2)` (`0xC000..0xFFFF`). Their stationary
particle uses the supplied surface Y.

The port now preserves and interprets the first-world collision-program subset
(`0x88/0x8E/0x8F/0x95/0x05`) and target convex plane groups (`0x89..0x8D`),
and performs the signed-8.8 midpoint/refinement
sweep for live player 41, weight 81, Main Base 286, and lifter factory 227
(including child 179). Supported-program misses fall through; malformed or
later unsupported programs deliberately retain the conservative header-sphere
fallback. Remaining fidelity work is the axial/transform opcode set,
bit-exact Q31 transforms and animated fixed-point slot generators, retail live
eligibility flags/active model-slot changes, and saturated-pool allocation
ordering.

The separate static tile-model test (`FUN_0043FF10` / `FUN_004276D0`) is also
ported. It performs the initial midpoint test before water and heightfield
handling, then scans a phase-anchored lattice whose signed offsets start at
exactly `-radius`, advance by `0x100`, and visit X outside Z. Thus the normal
class-1 radius 44 (40 plus the scanner's fixed 4) visits one candidate cell,
not an aligned 2x2 neighborhood. Each candidate uses the unsigned high bytes
of the wrapped X/Z probe, rejects Section-10 attribute zero, resolves that
attribute through the active Section-9 table, selects model slot
`(terrain_type >> 3) & 3`, and rejects a zero model collision radius. Its model
center is `((cell << 8) | 0x7F)` in X/Z and the truncating signed average of the
four wrapped corner heights in Y. The exact Section-8 collision program runs
in the identity basis with only the live masked 50-Hz tick exposed on animation
channel zero.

Static refinement retains retail's unusual branches: an earlier-half miss
keeps that earlier probe and continues, an earlier hit followed by a later miss
cancels the whole coarse collision, and two hits retain the later probe and
descriptor. Segment length is narrowed to signed 16-bit before the signed
half/max operation, including its overflow behavior. A successful callback
writes the refined position into the projectile, emits the same above/below-sea
class-34/42 burst used for entity hits, dispatches the global damage template
at `0x004CBF70`, and deletes the projectile. Its filter view is channels
`[2,0]`, amounts `[2000,0]`; the following provenance words are the source
entity type and owner handle. `FUN_00427950` resolves the current live static
kind from the collision record's cell-centre X/Z. The port mirrors the inline
F800 boundary: after F610 it re-reads suppression, resolves the current cell
through mutations committed by earlier physical slots, submits the exact
packet, and only then frees the parent; no stale collision-snapshot equality
gate is retained. Primary classes 1 and 3 use identical-value records
`0x004CBF70` and `0x004CBF88`, both `[2,0] / [2000,0]`. All ten admitted
static kinds may receive them. Only unburned kind 9 (severity 2000) and kind 27
(severity 3000) are positive, and both are deterministic. The seven admitted
ballistic classes use `0x004CC000`, `[1,0] / [2500,0]`, and filter to zero for
all ten kinds. Thus none of these exact packets consumes RNG or reaches kind
10's immediate mutation. Retrospective impact arrays are never replayed; class
87 retains its separate static no-op, and unsupported descriptor families do
not gain generic admission.

<a id="grenades-depth-charges-and-rockets-retained-source-and-local-adapter"></a>

### Grenades, depth charges and rockets: native entity owners

The accepted Alpine faststart recordings establish Type42 rocket kills in
faststart02 and a Type59 grenade explosion in faststart01. The bounded reverse
launch receipt proves selector3 Grenades and descriptor callback4 for that shot;
the shared Type59 body also serves selector4 Depth Charges. The accepted
evidence, forward replay coverage and remaining target actor
custody are owned by [ALPINE_INSECTS.md](ALPINE_INSECTS.md).

The weapon master descriptors and `DAT_004D02C0` settle the usable loadouts:

| Selector | Callback | Body/model | Launch speed raw | Cadence | Firing alias → PCM |
|---:|---:|---|---:|---|---|
| 3 Grenades | 4 | Type59 / 128 | 2000 | 50 ticks / 1 s | 61 → 7, rate32768, variance6553 |
| 4 Depth Charges | 9 | Type59 / 128 | 1000 | 50 ticks / 1 s | 88 → 7, rate104857, variance0 |
| 5 Rockets | 5 | Type42 / 240 | 1 | 50 ticks / 1 s | 79 → 19, rate65536, variance6553 |

Thus the rocket firing PCM is **sound_019**. Submit alias79 rather than direct
PCM19 so the authored rate variation survives. All three descriptors are finite
ammo and retain auxiliary flag4. `424650` requests the descriptor cue once after
the catch-up loop, and emits each separate class15 companion. The shared weapon
scheduler and inventory own cadence, alternate joint pulses and last-round
selection; body delivery is explicit rather than interpreting particle class0
as a hidden behavioral mode.

`44E770` narrows grenade/depth-charge velocity to signed words after
`q31(direction,speed)+shooter_velocity`. Rockets use speed1 plus the nonnegative
forward projection of shooter velocity, with no additive source vector. The
targetter preview's1024 override is not the real launch speed. Callback9 aims
`forward*sin(barrel)-up*cos(barrel)`; neutral depth charges therefore point down.
The trigonometric lookup duplicates the positive quarter-table word into both
dword halves before quadrant negation. Source state bit31 rejects remote births;
the sea-plane early gate belongs to selector31, so these three may launch below
water. 14870 copies the transient position and the shooter's current Euler words;
barrel direction affects velocity separately. Source relation+60 retains the
current shooter handle. Ordinary entity velocity and position remain signed
raw words through the common `(dt_us>>5)*velocity>>15` integration, with the125ms frame clamp.

The launch origin belongs to the reached model draw callback. Type46 has no
SubH; PLAYER4's op5C Parent16/48 already rotates axis0 with operand129 dynamic1.
The native barrel value is published to that authored variable rather than
adding a float gun rotation. Standalone SubE executes the reached op14 mounted
plain slots18/20 with their actual `NativeModelFrame` and stamps the selected
A1/B0 callback. If no callback is requested, flag1 keeps the source-center
fallback observed for the accepted rocket receipt. A reached unowned frame or
generated slot suppresses that birth and logs the boundary.

Catch-up retains411400's signed `timeOffset>>5` source-velocity rewind followed
by the table-speed direction advance; zero offset preserves the exact draw
position. The body consumes current Euler and velocity after that draw. Births
publish at the player's intrusive draw seat, and the refreshed successor admits
new tail actors to their later draw with age0. The fan rotation remains a
presentation-only hook and does not authorize launch transforms.

Queued transients also drain if the source starts dying in that frame while
its allocation remains linked.14870/44E770 test source bit31 for remote ownership;
the dying bit4000 adds no launch gate. After the current draw, the drain reads
the linked source's current Euler and velocity.

**Free camera currently suppresses native weapon births:** it supplies no
`NativeWorldViewport`, so the draw-origin owner is unadmitted and queued births
are skipped with a logged boundary. The no-callback source-center fallback
requires an admitted native draw; it cannot replace the missing viewport.

Successful Type42 construction consumes four shared words in order: SubA
constructor, singleton weighted behavior selector, `401F80` SubA reset, then
`403840` final reset. `4069E0` installs the trail without another reset. The
initial×3/2 target write is overwritten; final target is randomized from authored
base10000. Type59 consumes its one singleton selector word. Class22 retains
C→A→B force order, Type42 SubA4000/-4000, SubB100000/100000 and SubC clearance75,
range75, gain3145728, near100, damping200, terrain/center sampling. Type42 flags12
skip gravity; Type59 flags8 apply gravity (`q31(dt_us,0x300000)`) and class35 X/Z
friction `dt_us>>13`. Mode-zero authored wind drag uses the existing common
environment helper and mass10. Both primary constructors supply2000ms; the
strict timeout is elapsed whole milliseconds>2000, and mine stillness is>10
unchanged XYZ callback visits.

Type42's radial template is inner512/outer1024; Type59 is512/2048. Both retain
impulse2000 and packet channels[2,3], amounts[14000,12000]. The source relation
fills trailing source-type/owner words; damage goes through the existing static
and playing dynamic radial owners. `410C10` first emits authored+90 cue62 at
fixed rate, then `40BAF0` emits its randomized62 after scatter. Present A/B make
Type42 use ten class37 attempts; componentless Type59 with capability40 uses
sixteen alternating94/95 attempts. Both use the current model extent and shared
density/allocation policy. Rocket alternate class1 has no ring; Type59 alternate
class49 normally constructs Type60 after the radial blast.

Production constructs actual entities through
[104B0/D720](../../crates/v2k-game/src/entity/entity_weapon_construction.rs)
and retains the allocation and exact three task slots in
[the native weapon owner](../../crates/v2k-game/src/native_entity_weapons/mod.rs).
It publishes all known absent optional components, the source relation, raw
motion/Euler words and authored model/health/collision data. Class22 prepares
acquisition, trail and flight in native order; class35 clears the other slots
before publishing its rolling Primary. An actor ID, model or type alone cannot
admit damage or contacts.

The [native visit](../../crates/v2k-game/src/native_entity_weapons/production.rs)
runs at its actual position in the intrusive 13500 pass. It preserves callback
elapsed/RNG prefixes, fresh later-slot reads and wrapper unwind ordering.
Class22 acquisition calls CEF0 synchronously inside the callback; class35's
stillness tag and both strict timeouts dispatch after the wrapper unwinds. The
shared basis rebuild, authored gravity/wind/underwater motion and full master
motion follow the tasks. Neither Type42 nor Type59 has a nonzero E370 surface
selector, so this path does not enter that independent timer/bubble/cue suffix.
Rocket trail406A70 runs only for detailed reason0, requires the signed wrapping
velocity square sum to exceed `700*700`, and attempts at least one allocation.
Each attempt
consumes its three displacement words after allocation, including rejected and
final attempts; each displaced point independently selects class31 or42 from
the sea word. Trail attribution is the rocket allocation, distinct from the
explosion's logical owner.

[Native terrain, whole-water and static contacts](../../crates/v2k-game/src/native_entity_weapons/contact.rs)
and the shared active pair host use authored collision programs and the current
retained physical matrix. They preserve the opposite actor's callbacks,
component hooks and post-terminal physical suffix. Solid class22 contact calls
CEF0; class35 has no style terrain/water hook and retains the shared solid-normal
and water response. Dynamic source grace is the strict750ms recent-relation
window; particle owner grace remains its independent50tick window.

[Particle hits](../../crates/v2k-game/src/native_entity_weapons/impact.rs)
authenticate completed live task custody, or a completed class1/49 continuation
for a later particle before deferred removal. The current style has null
primary/infected/cured hooks; each wrapper retains its own timestamp/model-bit,
cue and impulse order.11180 remains distinct: it queues the living accepted-hit
cue before checked damage and uses eight times the11030 impulse. The checked
hit and chain blast run actual Playing hull damage and synchronous class1/49
with the retained scheduler/static world. Type60 is a real constructor-issued
ring owner; newly appended rings receive their later visit in the same pass.
A blocked committed hit or terminal remains parked, so its effects and RNG
cannot replay as a fresh callback.

One approximation remains explicit: **native allocation residue policy** starts
entity+B2's transient mass contribution at zero for these births.104B0 calls a
nonzeroing allocator, and these templates/D720 do not write B2 before the first
callback; a captured zero does not establish a constructor invariant. The
four accepted motion-input receipts
observe B2 zero for the grenade and first rocket, but Curly retains`0xF71E`
and Louse`0x035F` from publication through first mover entry. Their first task
entries already have unsigned callback masses`0xF728` and`0x0369`, matching
`4130F6`'s wrapping authored-mass10 plus B2 write. Both completed first visits
retain B2 zero; their next callbacks restore mass10. These observations prove
the nonzero word reaches its mass consumer. The accepted Curly write watch
finds`0xF71E` already present at CRT malloc return, with no B2 writes before
the first mass read and one executed`413157` clear afterward. The live type
definition requests212 bytes; snapshots retain the208-byte common prefix.
This proves untouched returned storage for that actor, without identifying its
earlier heap occupant or establishing a portable allocation policy. The same
explicit host allocation policy is used for fresh native Type9; native
animation/task writers and13500's clear own subsequent contributions.
Follow-up owner: this section's replaceable allocation policy; broader retail
heap behavior remains unowned. A
[compiled native arithmetic control](../../crates/v2k-game/src/native_entity_weapons/recorded_motion_tests.rs)
matches all39 recorded motion visits from these retained inputs;
it excludes RNG target choice, trail/candidate completeness and this allocation
policy. The policy keeps an immediate callback defined and does **not** count
as retail acceptance.

These implementations and focused corpus-backed tests close the former local
sweep/fuse/body adapter. Matched audiovisual acceptance remains separate.
Attached bit1000 bodies, multiplayer dispatch and Guided Missile class28 need
their own owners. [Alpine insect custody](ALPINE_INSECTS.md) owns the separate
Type30/40/56 constructors, task graphs and synchronous hit/death paths; the
weapon's arithmetic controls do not establish their complete retail scene
acceptance.

### Player contact damage and death (retail traces, 2026-07-17)

The two controlled 100-Hz collision captures close the player hull/death
lifecycle without inferring it from the bar animation:

- `20260717-042514-collision-health.jsonl` records a single lethal tree impact.
  At **14800.425 ms / entity tick 696**, hull health changes `40000 -> 0`, the
  dying bit `0x4000` is set, and the active type-46 model changes from slot 0,
  global model 41 (`player4`), to slot 1, global model 67 (`play4ded`), in the
  same stable sample. The player disappears at **17810.270 ms**, 3009.846 ms
  later at this sampling rate.
- `20260717-042601-collision-health.jsonl` records repeated destructive tree
  contacts. The authoritative entity-health sequence is
  `40000 -> 36772 -> 28954 -> 25130 -> 21389 -> 9145 -> 4188 -> 4176 -> 4104
  -> 0`, at elapsed times `15050.035, 18330.337, 21080.397, 23130.418,
  26390.167, 38980.120, 40230.396, 45580.297, 61620.509 ms`. The lethal sample
  again switches directly to model 67; the entity disappears at
  **64630.125 ms**, 3009.616 ms later. Both observations agree with the
  authored `0xBB8`-ms `"Dying bounce:"` control state and the strict
  `elapsed > 3000` expiry already used by the port.

The successful persistent local craft constructor publishes class24 for fresh,
native level and campaign entries through the same owned controller/component
boundary. Source4438A0 passes the actual Type46 type+118 choice override through
438080/4381F0 to0AC60; publication authenticates the authored primary model-slot
agreement and audited player pair interface. Missing/mismatched metadata or a
foreign selected descriptor retains its unresolved context rather than acquiring
Player Control from model identity alone.

The local production death entry authenticates Type46's rule1/class25 alternate,
its current Player Control context, clean/infected dying model slot1/3, empty
cargo, null logical sound `+8C`, and the independently retained null pending
replacement handle at controller `+6C`. PE `447280` then calls `475F0`
synchronously, publishes direct text `CA` when `+194` has extra lives or sticky
`D9` otherwise, seeds signed contact latch `+208` with `rng16>>13 + tick`, clears
task slots2 then1, zeroes control7, and replaces slot0 with the `BB8`/`4476F0`
wrapper. The host's `PlayerDeathLifecycle` owns that wrapper and begins charging
elapsed time on the following controlled visit, before common physics. Its
primary-weapon reset is local adapter cleanup. Dying style masks `+34/+38` are
zero, so living controls and Sub-O fan retunes stop;447280 has no fan-loop stop
call. Terrain/water447AF0 owns strict signed latch comparison, contact burst
and `rng16>>11 + tick` reseed; static447C90 has no burst/latch/Y1000 suffix.
Pending replacement446D70, non-null logical attachment release, nonempty cargo,
and the capability20 post-death objective continuation report named preflight
blocks. The retained wreck observations and source cadence are distinguished in
[the existing wreck-effect note](#player-wreck-bursts-and-sample-timing-retail-trace-2026-07-22).

The native player contact phase now preserves `11AD0`'s entry model, scan gates
and animation inputs across terrain, water and static suffixes. A lethal
`448280` material packet enters `447280` before the retained `141D0` physical
tail; lethal physical damage finishes the same constructor before water/static
look up the current style. `447AF0` performs feedback, strict signed `+208`
comparison, synchronous `475F0`, `rng16>>11 + tick` reseed and finally live
`VY=1000`. The physical tail measures its velocity delta after that write.
`129B0` authors water normal Y `0x7fff`; `447AF0` still consumes the exact
short words with its `>>12` dot product. Its `475F0` extent is half the fresh
active model's unsigned `+08` radius. Water type `+88` sound precedes the hook,
including selector7's later common-response suppression. Static type `+8A`
sound and pickup precede `447C90`; kind9 enters existing `427B20 ->28720`
destruction before `11760`, with no wreck burst, RNG or `VY=1000` write. The
callback matrix is Player Control frames0..4 `448280/NULL/447C90`, frame5
`447AF0/447AF0/447C90`, and frame6 all NULL; alternate class25 authenticates
exactly frame5. The current force-feedback backend remains unowned: the port
reports each reached command and retains its exact strength. Native Level1
controls exercise real player-model terrain geometry, material death ordering,
water throttle/response ordering and both living/dying static kind9 contacts.

`FUN_00443440` mirrors entity `+0x30` to controller `+0x8C`. On the lethal
sample the mirror still contains the preceding health, then becomes zero by
the next 10-ms sample. The independently stored visible bar value at
`0x004DB1C8` follows the recovered `/6`, minimum-800 trailing step while the
player is alive, including a one-step undershoot followed by an upward snap.
It does **not** animate to zero during the dying dwell: it remains at the last
pre-death value (`40000` in the one-shot capture, `4104` in the repeated-hit
capture). This freeze is retail behavior, not missing samples.

The repeated-contact capture also crosses the independently recovered warning
threshold at entity tick 1929 (`9145 -> 4188`). `FUN_00446640` snapshots health
at controller-callback entry, requires it to be strictly below `0x1389`, and
repeats only when the absolute shared-tick distance from process-global
`DAT_004DE858` is strictly greater than `0x3C`. The port retains that cadence
across level/player resets and submits direct text `0xDF` plus physical global
sound slot `0xB4 / 4 = 45` at centered Q16 gain `0x8000` (one half). Because
solid contact occurs later in the world update, a newly low hull becomes
eligible on the following controller tick.
The warning branch does not test the dying bit. Samples 6162/6163 and 6223 of
the repeated-contact trace retain the matching controller binding at dying
ticks 3061 and 3091, so cadence-eligible warnings continue during the wreck
dwell until the player allocation is removed.

The adjacent low-hull plume is **not** part of that warning cadence. After the
same two controlled callbacks succeed, type-flag bit 0 is present, and the
entry-snapshotted health is below `0x1389`, every callback consumes two global
RNG words and attempts one particle allocation. Its signed-8.8 origin is
`entity position + [jitter_x, 0x14, jitter_z] - body_forward * 0x5A`, where
each jitter is `((rand16 >> 9) & 0x7F) - 0x40` and body-forward is the exact
Q31 basis. Strictly above the sea plane it emits class 20 with input velocity
`[0,0x50,0]`: sprite frames beginning at 0x37F, 23 frames, rate 8, draw scale
`0x0200`, radius `0x14`, and lifetime `0xAA` ticks. At or below the plane,
`FUN_00441670` consumes a third RNG word for another X jitter and substitutes
class 42 with input velocity `[0,0x190,0]`; its descriptor contributes another
`+0x190` Y bias, sprite 0x313, draw scale `0x0100`, radius `0x50`, and lifetime
`0x40` ticks. The source handle is retained and the allocator's success is
ignored. Retail then ORs entity `+0x84` bit 3; the generic bit-31 consumer is a
separate inactive player path, so this latch must not manufacture a second
emitter. The Rust port now reproduces this recurring plume synchronously before
the later solid-contact pass, including RNG consumption when its 200-slot pool
is already full.

The generic solid-contact call graph also rules out several tempting invented
effects. `FUN_00411760` constructs a channel-1 self-damage packet and calls
`FUN_00415040`; positive filtered damage reaches `FUN_00414E90`, and lethal
damage reaches `FUN_00410C10`. Type 46's exact Section-12 header has zero in
all three sound fields used by this chain (`+0x8A` collision, `+0x98` damage,
`+0x90` death), so this player-side path plays no authored sound. The captured
player's type flags are 5, so neither the filtered-zero collision branch
requiring byte `+0x64 & 8` (event `0x17`, text `0xDB`) nor the generic death
branch requiring `+0x64 & 0x20` (text `0xDE`) applies. Its runtime flags also
lack `0x01000000`, excluding the source-type-46 death event 4. Tree-side burn,
destruction, and audio remain a separate static-object response.

<a id="delayed-player-wreck-burst-retail-trace-2026-07-22"></a>

### Player-wreck bursts and sample timing (retail trace, 2026-07-22)

`runtime_re/captures/local/20260722-022105-player-wreck-effects.jsonl` and its
protocol sidecar retain the 19.152-second run: 9,576 main samples, 25,734
particle events and 3,799 audio events. Entity context is sampled at 100 Hz,
while the 200 Hz particle sampler retains distinct stable pool states. The last
sampled living entity is 14640.5527 ms / tick 1979; the first sampled dead entity is
14650.5026 ms / tick 1980 (`health=0`, bit `0x4000`, model 41→67). That timestamp
is an observation bound, not the exact health write or initializer time.

The retained age-zero player-owned groups are:

| Elapsed ms / retail tick | Class 16 / class 18 | Observed allocation |
|---|---|---|
| 14644.0648 / 1980 | 10 / 1 | Eleven recycled slots, before the first sampled dead entity |
| 14764.458 / 1986 | 10 / 1 | Eleven recycled slots, about 114 ms after that entity sample |
| 16654.083 / 2080 | 10 / 1 | Eleven births into an empty pool |

The earlier analysis omitted both recycled groups and inferred a universal
2000-ms timer from the last group. The retained events disprove that inference.
PE `FUN_00447280` calls `FUN_004475F0` at `0044743C` during local Dying-bounce
initialization, then seeds controller+208 with `(rand16>>13)+tick` and publishes
the 3000-ms `0xBB8` dwell. `FUN_004476F0` advances saturated control 7 by `dt/40`;
it has no 2000-ms burst test. Terrain/water `FUN_00447AF0` calls `FUN_004475F0`
on contact only while controller+208 is
strictly below the current signed retail tick, reseeds with `(rand16>>11)+tick`,
and sets signed 8.8 VY=1000. Passive particle groups do not identify the exact
caller; source establishes the initializer and contact policies. Their timing
must never become captured constants in the port.

Reproduce the bounded event census from the repository root:

```powershell
python runtime_re/scripts/summarize-player-wreck-effects.py `
  runtime_re/captures/local/20260722-022105-player-wreck-effects.jsonl `
  --output .tmp/player-wreck-effects-summary.json
```

Static `FUN_004475F0` and the trace agree exactly. The helper reads the active
wreck model's `+0x08` radius (2200 raw in this run), halves it to 1100, and
calls `FUN_00440950`. That function pre-increments the shared 100-vector cursor
for each of at most ten class-16 attempts and stops that loop on the first
allocator rejection. The last captured group uses cursor indices 9 through 18; it
is session state, not a player-specific constant. The normalized position
offsets preserve `FUN_004407D0`'s visible bug: normalized Y is stored into both
Y and Z, while velocity still uses the independent Z direction. The independent
surface tail then attempts class 18 with raw velocity `[0,500,0]` strictly
above sea, or classes 45 then 46 at/below sea. Every accepted record retains
the player handle. The last group's empty pool accepted all eleven; later class-31
births are class 16's ordinary ballistic-trail callback and must not be spawned
as another authored burst.

After particle allocation, `FUN_00440950` consumes one shared RNG word and
submits positional global sound 62 at wrapper rate
`0x10000 + (random16 >> 3)`. `FUN_004475F0` immediately submits the same sound
again at fixed rate `0x10000`. The latter two observed groups each have two
PCM-correlated starts at the same sample; effective mixer rates include resource
and 3D processing. PCM matching alone does not identify a caller or establish
that every start was observed. Type 46's zero Section-12 death-sound field remains
correct: these two source submissions belong to `FUN_004475F0`, separately from
generic damage. The eight-class-9 burst at 14444.590 ms is a separate earlier collision
response. `WorldFx` retains shared cursor/RNG, allocator, surface and duplicate
audio order; the player lifecycle must use source initialization/contact owners
rather than the superseded automatic 2000-ms effect.

## 4. Player model = runtime-assembled hierarchy (0X3XX.OVL Section 8)

- **Hull `player4`** entry #28: 66 verts (type_flags {0:51, 5:4, **8:5**,
  13:6}), 1137 cmd words. The stream contains **45 op-0x0E inline instances,
  26 op-0x5C mount-frame ops, 45 op-0xE6 state ops, and 19 op-0x2B/0x2C
  conditional jumps** keyed on dynamic animation variables (operandA
  0x84/0x85 = callback idx **4/5**; compare constants
  {0,1,2,6,7,8,0xa,0xb, packed 0x400a/0x800a/0xc00a}). Word 4 selects the
  weapon/loadout branches; word 5 selects and morphs the mode-specific hull
  panels. Word 6 separately supplies the authored rotating mount bases.
- The **5 tf-8 lerp verts** (a=0x85 → idx5, effective `t = value/0x10000`) are the
  hover↔VTOL geometry morph in the hull.
- Siblings in the same OVL: `pl4engine` #30 (19v, static), `pl4enginesurround`
  #29 (28v, own 0x5C + 0x0E of `pl4tubegun`), weapon meshes `pl4cannon`/
  `pl4gatgun`/`pl4fmissile`/`pl4gunbarrel`…, `shipgyro` #231 (2v, one textured
  billboard op 0xB8, sprite id 595, **static** size 0x800F/angle 0xA000),
  `shipaura` #232 (26v, 16 tf-8 verts on register #0 — animated glow),
  `play4ded` #54 (wreck, tf-8 idx7).

### Sub-model machinery (new Section 8 semantics)

- **op 0x0E = inline instance** — `FUN_00467410` (game_logic.c:46660):
  snapshots parent render ctx into a child ctx, builds child orientation via
  `FUN_004676c0` (:46761), recursively renders (`FUN_00464e60` :46724),
  saves/restores the transform globals around the child (:46717-46731).
  Orientation code (`code&7`):
  - `== 6` → **copy full 3×3 mount basis from `ctx+0xC4`** (:46768-46778);
  - else → `FUN_00467730` (:46791-46856): **axis permutation** (low 3 bits) +
    per-axis **negation** (bits 0x08/0x10/0x20) of the parent matrix — the
    octahedral 90°-step reorientation set. player4 uses codes 6, 0x10, 0x30…
- **op 0x3C = vertex mount frame** — `FUN_00466cd0` (:46394): resolves 3
  parent vertex slots, builds an orthonormal basis (normalized edge + cross
  products) and writes it to **`ctx+0xC4`** (dword 0x31; :46445-46458) — the
  same slot code-6 instances read. If the mount triangle uses tf-8 morph
  verts, mounted parts physically swing with the morph.
- **op 0x5C = oriented mount frame** — `FUN_00466f80` first calls
  `FUN_00467730` to permute/negate the parent basis using operand 0, then
  optionally calls `FUN_004671f0` with operand 1 as the axis and operand 2 as
  a packed/dynamic angle. Klaus uses this form for its 180° root turn and its
  mirrored/animated wings; its operands are not vertex slots.
- **Dynamic operands** — `FUN_00470700` (:51009) / `FUN_00470840`
  (fgdk_engine.c:1194): operand bits 7+6 → **register file `ctx+0x94`**
  (64×u16 per-entity animation variables, e.g. shipaura reg #0); bit 7 only →
  **callback `(*ctx+0x58)(ctx+0x60, ctx, idx)`**. `FUN_004138F0` obtains the
  per-entity callback pair from the active type record at `+0x78`: pair dword
  0 is copied into draw-request dword 15 and pair dword 1 into dword 16.
  `FUN_00465870` then propagates those values to render context `+0x58/+0x5C`
  (with the separate external-attachment pair at `+0x60/+0x64`). The old
  claim that `DAT_004fec90/94/98` supplied this callback was wrong: those are
  the signed global render-direction values consumed by `FUN_00433BD0` and
  copied to render/model state `+0x3C/+0x40/+0x44` by `FUN_00433FA0`. The
  concrete callback idx→value map remains runtime evidence; do not infer
  idx4/idx5 or idx7 meanings merely from the model stream's use of them.
- Entity orientation → matrix: `FUN_00464f90` (:45354-45434) concatenates the
  model 3×3 with the frame matrix into `ctx+0x18`, wires `ctx+0x58/0x60`;
  `FUN_00464e60` (:45286) loads it into the transform globals; `FUN_00465390`
  (:45456) is the top-level entity draw (translation = entity − camera).
- **Fan spin**: no stream-level dynamic billboard angle on the player
  (shipgyro's operands are static); the integrators write per-frame node
  rotations (hover/fly `+8` node ∝ turn; fly rotor node ∝ throttle, see §3).
  Exact node→sub-model routing is runtime-installed; the port should drive a
  spin angle ∝ throttle (both modes; user-visible behavior) and calibrate.

### Doc corrections

- The old "0x081/0x082/0x084 = sub-object begin/end" claim is **wrong** —
  those were 9-bit face-command bases from the superseded 2026-03 pass. In the
  validated 8-bit opcode set, 0x84 is a QUAD face op (mirror 0x88), 0x86 a
  1-word state op; 0x81/0x82 never occur.
- CONTROLS.md hover-mode corrections: UP/DOWN in hover aim the **gun barrel**
  (not fwd/rev thrust — that reading is only true in fly, via tilt); hover
  forward propulsion is the **SPACE throttle** channel. "Guns locked in heli"
  is TRUE for aim (barrel forced to 0), false only for the trigger.
- `+0x194..+0x19c` are pickup/inventory fields (not morph state): +0x194
  reward accumulator, +0x196 active sub-model/weapon slot, +0x197 capability
  bits (bit0 Targetter, bit1 turbo → 1.5× + blur), +0x199 attached-entity count,
  +0x19c attachment refs.
  The port retains both acquired bits across ordinary campaign world
  replacement and clears them at the frontend session reset. Targetter's first
  pickup installs its recovered scan/overlay program. Turbo's first pickup
  drives the exact VTOL force/height policy; its model-side blur presentation
  remains unresolved.

## 5. Port implementation notes

Player = body 0: style 0 (hover) ↔ 1 (fly). Sequence: input → channels
(scales above) → per-mode integrator (§3) → anim state → draw.

Needed pieces (see workstream breakdown in the session log):
1. `models.rs`: parameterize interpretation with live anim variables
   (register file + callback values) so op-0x2B/0x2C jumps, tf-8 lerps, and
   billboards evaluate per frame; with all-zero vars output must stay
   byte-identical (validate_sec8 harness). Implement op-0x3C vertex mount
   basis, op-0x5C oriented mount basis, and op-0x0E child orientation codes
   (permute/negate table + code-6 basis).
2. Renderer: matrix-capable draw (pitch/roll, per-child 3×3), recursive
   instance draw.
3. `v2k-game`: type-46 Hover now decodes A/B/C/D and runs the recovered
   fixed-point steering, lateral correction, propulsion, and lift. Both modes
   share the recovered gravity/underwater, environment-drag, and signed-word
   position tail. VTOL now also runs recovered fixed-point manual lift,
   height-assist, fuel, ceiling attenuation, previous-basis projection, and
    normal type-46 pitch/yaw/bank recurrence. Its oriented authored-sphere
    solid-terrain/seabed response is now wired; the alternate terrain-target
    attitude state and exact contact-presentation callback remain provisional.
    Feed idx4/idx5
   + morph t (the port currently ramps idx5
   0↔0x8000, i.e. t 0↔0.5, on mode change); the read-only
   `capture-hover-model-callback.ps1` trace follows the type-record `+0x78`
   pair so retail callback values and branch-state coupling can be calibrated
   without invoking unknown game code.
