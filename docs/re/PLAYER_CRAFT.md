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
   **A `0x00447970` (→fly)**: `{body 0→1, 1→2, 2→1, 3→3}`;
   **B `0x004479F0` (→hover)**: `{0→0, 1→2, 2→0, 3→4}`. Player = body 0, so
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
mechanism: `FUN_004442B0` unconditionally raises `+0x20d`; `0x00447970` commits
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

Slot 2 is the infected craft, slot 3 its wreck. A warp releases the old body,
and `FUN_004438A0` constructs the player at the controller snapshot's arrival,
so `104B0` selects slot 2 when that cell's terrain type has bit `0x10`, as
world 43's arrival does. Both live slots hold model 41, so the player's pair
walk resolves the infected craft as it does the clean one.

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
| `+0` turn | LEFT(−)/RIGHT(+), joystick X, mouse X (`+200` per Sensitivity-scaled mickey, [CONTROLS](CONTROLS.md#mouse-joystick-and-pointer)) | default Joystick **Relative** mode writes the duty-cycle-scaled keys and the joystick term directly; `FUN_00444B90` is only the Joystick Absolute-mode target-bearing helper. Mouse X is added in both modes |
| `+2` pitch | UP(+)/DOWN(−) authored amplitude **0xD80**, passed through the default Relative sensitivity/duty quantizer; S(+)/X(−) scale **0x900**, clamp ±0x900; S/X suppress only SPACE's separate powered coupling; joystick Y (forward +); mouse Y (`-200` per Sensitivity-scaled mickey towards the player) | one shared channel — in Hover positive depresses the gun; in VTOL it pitches nose-down. At default sensitivity 10 and 20 ms, a held arrow is ±2303 (`0x08FF`), confirmed live on 2026-07-16 |
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
an independent control: its input reader, `DEMO_0041A650` recurrence,
`DEMO_00444D00` player caller, `DEMO_0041AFE0` lift/ceiling path, pristine mode-1
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
updates it through `0044EBD0`.

`0044EBD0` adds the active frame delta to wrapping process-global
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
when a step strictly crosses the remaining magnitude (`0x4466EE`/`0x446720`:
`cmp remaining, step; jle` keeps counting down). Exact equality subtracts to
zero without firing. This is retail's high-frame-rate C/D failure: its frame
pump (`FUN_00494070`) runs whole-millisecond frames of at least 8 ms, so a fast
machine runs exactly 8-ms frames, `80,72,...,8,0` lands on zero and every beam
command is lost. Slower, irregular cadences ordinarily cross the endpoint.

The port keeps the 80-ms signed timer and the whole-ms countdown. The port fix
`beam_commands` ([launcher Options](../../LAUNCHER.md#options-and-command-line-operation),
`--beam-commands`) is on by default and also fires when a step lands exactly on
zero; turning it off restores retail's strict crossing.

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
comparisons. `FUN_00448420` returns the slot holding a given selector, or
null after all 32. Level-1 factory payload `0x0001F412` is selector `0x12` with stored
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
runs F610. Classes 4 and 54 then run `FUN_00441850`: sound 90, then
`FUN_004566E0` with the shell's source type and owner in the template's last
words (class 4 `0x004CD818`: radii 255/1023, impulse 1000,
`[3,2]/[2000,4000]`; class 54 `0x004CD7F8`: the same with `[3,0]/[8000,0]`).
Class 4 then runs `FUN_00441A50` (class-1 debris, count
`(0x13880 >> 10) * scale >> 16` min 1, velocity table X / `abs(Y >> 1)` / Z,
no allocator abort); class 54 runs `FUN_004410B0` class `0x27`
`DAT_004F72CC >> 12` times whatever `+0x1D` says (zero takes one; packed
table dword `* 2` plus independent `Z * 2`; at or below sea, class `0x2E` and
Y-100). F890 ends with sound 62. Class-77/78 `FUN_00456710` stays
fail-closed.
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
`403840` final reset. `004069E0` installs the trail without another reset. The
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
fixed rate, then `40BAF0` emits its randomized62 after scatter. Neither has Sub-F,
H, N or G (Type42 has A, B and C) and both have capability40, so each uses
sixteen alternating94/95 attempts, as the retail lab counts. Both use the current model extent and shared
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
Attached bit1000 bodies and multiplayer dispatch need
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
zero, so living controls and Sub-O fan retunes stop; `00447280` has no fan-loop stop
call. Terrain/water `00447AF0` owns strict signed latch comparison, contact burst
and `rng16>>11 + tick` reseed; static `00447C90` has no burst/latch/Y1000 suffix.
Pending replacement `00446D70`, non-null logical attachment release, nonempty cargo,
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
sound and pickup precede `00447C90`; kind9 enters existing `427B20 ->28720`
destruction before `11760`, with no wreck burst, RNG or `VY=1000` write. The
callback matrix is Player Control frames0..4 `448280/NULL/00447C90`, frame5
`447AF0/447AF0/00447C90`, and frame6 all NULL; alternate class25 authenticates
exactly frame5. Each reached feedback command submits its exact strength to
the pad driver ([console vibration](CONTROLS.md#console-vibration)). Native Level1
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

A particle that a hit leaves attached to the player (`FUN_004425D0`) adds its
mass, `(256 - age) / 4`, to the player's `+B2` on each update and deals its
damage through the attached-particle packet. `FUN_00412DA0` zeroes `+B2` after
every controlled-player callback, as for any scheduled actor, so retail's
player reads 63, 62, 61 while one particle ages and zero otherwise. These are
the repeated hits of kind -5 that follow a Type76, Type87, Type108, Type122
or Type38 shot on an idle player: 255 hits a particle, of 22 points each in
worlds 24, 31, 42 and 46 and of 10 in world 21.
The port's player starts with `+B2` zero and the controlled-player tick zeroes
it after the craft's callback; before, the unresolved word retired every
particle attached to the player.

Static-route particles (classes 52, 68 and 85, `FUN_00442950`) reach the
player through `FUN_00411180`, not `FUN_00410EB0`. It stamps `+34`, requests
type 46's accepted-hit cue (`+80`) while the player is alive and before
damage, calls the type's `+14` trampoline (`FUN_0040DAC0`, whose style `+28`
hook is null in every Player Control frame), strikes `FUN_00411030` with eight
times the packet's impact sum, then runs checked damage `FUN_00415040`. A
Type122 or Type57 class68 shot therefore deals 5800 to the player
(`[2,6]/[2000,2000]` against type 46's profile), and the class84 particle that
`FUN_00442950` then attaches adds the 255 kind -5 ticks above. Retail's idle
player in world 24 takes its first such hit 510 ticks after the arrival. The
port used to hand these hits to the shared actor owners, which reject the
player, so none of them landed.

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

## 6. Controller, death and mind-control routines (STATIC)

The controller is the player's `0x5AC`-byte record (`+68` its body, `+74` the
99-dword profile block, `+22C` the Targetter program).

### The controller

| Routine | Does |
| --- | --- |
| `FUN_00442ED0` | creates the local player's controller (0x5AC bytes, from `FUN_0044FA30`): zeroed, no body yet, the Self Righting byte `+20C` = 15 and `+200` the tick; adds its reference-counted records from the tables at `004CDB58` (`FUN_00471510`, some also linked with `FUN_004715E0`) and `004CDBA0` (`FUN_00471280`), restores the profile block (`FUN_00443560`) and links it at the tail of the controller list (`004DE850`); a failure releases what it made, frees it and raises the error |
| `FUN_00443110` | creates a remote player's controller: the same record without the reference-counted records, restored from a profile block (`FUN_00443560`) and linked at the list's tail; the network birth of a Type51 craft (`FUN_00438080`) passes a zeroed profile |
| `FUN_004431C0` | frees a controller: unlinks it, stages its live body (`+68`) for deferred destruction (`FUN_00410B70`) and runs `FUN_0044CF00` on `+22C`, drops each record of both tables, then frees it |
| `FUN_00443AC0` | returns a controller's body handle (`+68`) when that body is live, else error `0x8801` |
| `FUN_00444330` | sets (argument 0) or clears (nonzero) checked-damage admission `0x8000` on the controller's body, and through `FUN_0040EAB0` on each live attached entity of its list `+220` that lacks `0x4000` |
| `FUN_004382E0` | removes a body (from the frontend init `FUN_0042B230`): a Type51 craft first frees its controller (`+88`, `FUN_004431C0`), then the body is flagged for the deferred sweep (`FUN_0040D750`) |
| `FUN_00446530` | installs the Player Control task (from the style initializer `FUN_004464F0`): without a controller (error `0x8802`) does nothing; otherwise stores the movement style in controller `+86`, builds the task around the per-frame update `FUN_00446640` (`FUN_00401020`, `FUN_0040A7A0`), and on a craft whose description has `+20` set stores 1 in its first record's `+42` (`FUN_0041B9B0`) |
| `FUN_00447F70` | applies the controller's staged start to a newly created player body (from `FUN_00451AF0`): empties the list `+220`, then by the flags at `+290` copies the position (bit 2, `+294` to `+96`), velocity (bit 4, `+29A` to `+9C`) and heading, pitch and roll (bit 8, `+2A0` to `+A2`), sets full health 40000 (bit 1), and rebuilds the basis (`FUN_00413F70`) |
| `FUN_00446BD0` | the beam-in collection: walks the live entity list, letting `FUN_00446C80` keep the nearest eligible candidate (an error from it ends the walk), then attaches the chosen one to the player (`FUN_00443B50`) |
| `FUN_00421590` | the player craft's draw callback: the default draw `FUN_004138F0`; in a network game, a remote craft gets its name tag from the slot table `004FE7A0` drawn above it (`FUN_00470A70`, queued through `FUN_00421840`/`FUN_004218A0`); then the lock reticle `FUN_00421230` while a lock runs |
| `FUN_00424480` | the Targetter's launch preview (from `FUN_0044D0F0`): builds the player's launch request for its emitter (the method class from `FUN_0044EA60`, pitched by the weapon's elevation word unless method 5) and passes it to `FUN_0044E770`; error `0xA200` without an emitter |

### Passive Player

| Routine | Does |
| --- | --- |
| `FUN_00446F30` | installs the Passive Player (class 66) task around its per-frame update `FUN_00446FC0` (`FUN_00401020`, `FUN_0040A7A0`) |
| `FUN_00446FC0` | the Passive Player's per-frame update, a craft with no controls: spins the model's two fan channels toward full when its description's `+44` is set; applies lift with the cargo mass (`FUN_004185C0`, `FUN_0041F1C0`) and, on a detailed body, its effect draw (`FUN_00440E80`); applies drag (`FUN_0041EB50`); and below 20000 health on a detailed body emits smoke behind it at random (`FUN_00441670`, a 1-in-max(2, health >> 13) chance), setting `+84` bit 8 |

### Death

| Routine | Does |
| --- | --- |
| `FUN_004478E0` | Player Death's second style initializer: without a controller (error `0x8802`) only stages the body for deferred destruction; otherwise releases its task slots (`FUN_0040A860`), runs the death of each entity on the controller's list `+220` (`FUN_00410C10`), empties the list (`FUN_00414AA0`) and stages the body (`FUN_00410B70`) |
| `FUN_00447590` | drops all of a dying player's cargo: pops the body's Sub-J rows one by one (`FUN_00418500`) until none is left |
| `FUN_00447750` | places one of a dying network player's Type61 power-ups (operation and amount in `+20`) at a random point around the body (x and z within 0x200, y up to 0x1FF above), kept between 0x80 above the ground and 0x800 above the higher of ground and sea, and creates it through the network birth (`FUN_00438080`) |
| `FUN_00450980` | the player's death, from the gameplay frame (`FUN_0044FFA0`) once the body is gone: clears the message queue (`FUN_00437F30`), takes the dying controller's snapshot (`FUN_00443260`), restores the checkpoint state into the session (`FUN_00448E50`, in an error scope) and the world's control words (`FUN_0042ECC0`, then bit 4 with `FUN_0042EEE0`), keeps and re-applies the cheats (`+1C4`, `FUN_0044F830`), sets health 40000 (`+50`), carries one life fewer (`+158`, not below zero) and the snapshot's other fields over (all 64 dwords at `+54` in single play, two in network play), and refills the cargo list `+160` from the snapshot without types 7-9, 78, 79, 86, 90, 91, 95 and 116, zeroing every second slot after them up to the capacity `+15D` |

### Mind control

| Routine | Does |
| --- | --- |
| `FUN_00421800` | starts the lock search (mode 1) when idle or more than 1 s into the current mode; reached from the projectile launch `FUN_0044E770` |
| `FUN_00420C60` | advances the player's mind-control lock (the Primary's record): searching (mode 1) for up to 1.5 s through `FUN_00420E00`, locked (mode 2) while `FUN_00421070` keeps the target in its cone, and after 1 s locked taking control of it through `FUN_004480E0`; it also spins the reticle angle (`+48`) and grows the reticle word `+44` up to `0x600` |
| `FUN_00420E00` | the lock's search: the closest live, admitted, solid body with capability bit 3 within 4/3 of the range and inside a narrow forward cone (each lateral offset under half the forward distance), scored by distance plus twice the offsets |
| `FUN_00421070` | keeps a lock while the target lives, lies within four times the range, and stays ahead (forward distance over 0x80 with both lateral offsets no larger) |
| `FUN_00421230` | draws the lock reticle (draw callback `FUN_00421570`) along the line from the craft to a locked target (mode 2), storing that distance in `+44` |
| `FUN_004480E0` | the mind-control takeover (from `FUN_00420C60`): gives the target Type46's behavior source (`+D0`) and the controller (`+88`), gives the old body Type88's, points the controller at the target (`+68`, the old body at `+6C`), reinstalls both behaviors (`FUN_0040AC60`), clears the target's fan and record flags (`FUN_0041B9B0`, `FUN_00424F10`), records the body in the session (`FUN_00456770`), and holds controller `+70` at 60000 (500 when the install failed) |

| `FUN_00446D70` | ends a takeover (controller `+70` reaching zero in `FUN_00446640`, or its old body dying): the old body (`+6C`) gets Type46's behavior source and the controller back, the target loses both, and the two handles swap back; when both behaviors reinstall, a released Type98 only moves the session's view to the craft, any other target leaves a Type98 MindBall at it (`FUN_00409950`) that flies to the craft and hands the view back when it ends (`FUN_00409420`); a failed reinstall swaps the handles back and retries in 500 ms |
| `FUN_00448760` | a Hive swallowing the possessed body sets controller `+70` to 1, so control returns on the next Player Control frame |

The Possession Ray is weapon selector `0x1B` (Cheats -> Weapons code
`0x3E71B`): infinite, 400 ms cadence, no sound, no auxiliary command. Its
`DAT_004D02C0` row `{0, 1000, 0, 0}` names neither a particle nor a body, so
`FUN_0044E770` only starts the lock (`FUN_00421800`) on the shooter. The lock
record is the shooter body's component `+0x38`: mode `+34`, mode time `+38`,
target `+3C`, locked time `+40`, reticle size `+44` and angle `+48`.

**Port status: held.** The takeover makes the target's own body run Player
Control (`FUN_00446640`, with the integrator its movement style selects from
`0x4CD8D0`) over that body's type components, and the craft run Passive
Player. The port's Player Control drives only the player craft, so the lock,
the takeover, the release and the MindBall stay unported together: a lock
without its takeover would show a reticle that never ends in one. Holding the trigger with selector `0x1B` selected logs one
"Possession Ray held" stop per run and fires nothing; the A/B gun phase is
kept as for any unresolved selector.

### Special weapons

What each of the remaining Cheats -> Weapons rows is, from the master
descriptors, `DAT_004D02C0`, the Section-12 types and their behavior
programs ([class table](ACTOR_TASK_PROGRAMS.md#task-programs-of-the-other-classes-static)).
A row's code is the power-up word: operation in the low byte, amount above.

| Weapon | Row | Firing | Body and program | Port |
| --- | --- | --- | --- | --- |
| Possession Ray | `0x3E71B` | selector `0x1B`, infinite, 400 ms; row `{0, 1000, 0, 0}` | no body: `FUN_0044E770` starts the [lock](#mind-control) | held |
| Guided Missiles | `0x3E711` | selector `0x11`, 999, 1 s, sound 79; row `{1, 1, 0, 45}` | Type45 (model 240), class 28 | ported |
| Proximity Mines | `0xA09` (10) | selector 9, 1 s, sound 60; row `{0, 1000, 0, 48}`; `FUN_00424650` aims selectors 4 and 9 alike, so a mine is dropped as a depth charge is | Type48 (model 241), class 34, death class 49 | ported |
| Virus Bomb, Antidote Bomb | `0x3E71C`, `0x3E71D` | selectors `0x1C`/`0x1D`, 400 ms; particles 77/78 at 2000 | no body; see below | ported |
| Megablast | `0x539` | operation `0x39`; deployed by A and `/` held together | Type65 (model 243), class 55 | ported |
| Defender Ball | `0x3F38` | operation `0x38`: a Type63 born at the player into the hold | Type63 (model 244), class 50 | ported; its pair hook held |
| Autobot | `0x4038` | operation `0x38` with Type64 | Type64 (model 248), class 53 Defender Drone, death class 11 | ported |

**Proximity Mines.** Class 34's first style (`0x4C8548`, initializer
`FUN_0040B080`: Secondary, then Tertiary cleared, then `FUN_00406B00`'s
lifetime-0 Primary, which also zeroes Sub-K variable 2) runs the falling
Primary (`FUN_00406BA0`): the heading turns by the elapsed microseconds over
16, the pitch becomes (3 × pitch + q31(elapsed << 11, `0x1000` − pitch)) / 4,
Sub-K variable 1 grows by the elapsed time over 4 and variable 2 is cleared;
once the mine is
within 300 of the higher of its cell's ground and the sea (the clearance at
most 299) it returns tag
`0x9C00`, whose style slot `+04` (`FUN_0040C750`) switches to the second
style (`0x4C8590`). Its initializer `FUN_0040B0B0` arms the mine: Secondary
becomes the shared target search (`FUN_00401F80(entity, 1, 0, 0)`, one visit
in four, the type's 0x300 range and capability mask 8 from its `+C8`, never
the mine's shooter) and
Primary the landed program (`FUN_00406F20`: the same pitch step toward 0,
variable 1 by the elapsed time over 16, variable 2 (read unsigned) plus
q31(elapsed << 11, `0x4000` − variable 2), then the type's Sub-C lift with
no attached mass, on the waved sea since its surface byte is 1, and Sub-B
drag). The task loop rereads each slot, so the new search runs in the same
pass as the arming. Neither initializer draws a random word: Type48 has no
Sub-A for `FUN_00406070` to reset. A
found target runs the style's slot `+04`, `FUN_0040CEF0`, the mine's death,
as any pair contact (`+18`) or static contact (`+1C`) does in either style;
`+10`, `+14`, `+28` and `+2C` are null in both, so ground and water take
the shared response a grenade's do. The class-49 death scatters sixteen
particles of classes 94 and 95: `FUN_0040BAF0` tests the F, H, N and G
blocks (component block words 0, 1, `+34` and `+18`, from type `+E4`,
`+EC`, `+108` and `+E8`), Type48 has only B, C and K, and its capability
word has bit `0x40`. Model variables 1 and 2 are the two Sub-K words.

The port runs these as the native weapon owner's `ProximityMine` kind
(`native_entity_weapons::mine`): the falling and armed Primaries, the arming
style switch, the search, the class-49 death and the two model variables.
Retail lab (Level 1, master code, F9, fifteen B presses, one mine at tick
959 over the sea): the mine sinks 20 units in its first tick, then 1 a
tick on the lift, and its pair contact with the player kills it at tick 997,
the first tick past the 750-ms shooter grace, through `FUN_0040BD20` with a
Type60 ring. The port, started from retail's player state 270 ticks before
the shot with the same keys, drops its mine at frame 269: it sinks 19, then 1
a tick, and dies on the player 38 ticks after its birth with a Type60 ring,
as retail's does (the two players' idle drift differs by a few units a
tick, so the positions are not identical).

**Guided Missiles.** Row `0x11` is `{1, 1, 0, 45}`, the rocket's shape:
speed 1 plus the shooter's closing speed along the barrel. Class 28's
initializer `FUN_0040D160` clears Secondary, publishes the rocket's class-31
trail in Tertiary (`FUN_004069E0(entity, 2, 0x1F)`) and the lifetime-0
Primary `FUN_00407F20`, whose record `FUN_00408610` holds the target, the
flight time and the speed (the `FUN_00457730` length of the birth velocity,
narrowed to a word); `FUN_004086A0` frees it with the task. Neither draws a
random word. Style `0x4C81E8`'s `+34` sets `0x4000`, so the common update
never rebuilds the matrix from the Euler words: the tick steers the matrix
itself. Its `+10`, `+18` and `+1C` are `FUN_0040CEF0`, the rest null, and the
alternate is class 1. Each visit of `FUN_00407FB0`:

- once the shooter relation's age (`+68`) passes 15 s, runs `FUN_00410C10`
  and stops;
- rolls the matrix by the elapsed microseconds over 8 (`FUN_00457DD0`,
  lateral and up rows);
- without a live target clears the flight time and searches
  (`FUN_004083F0`): among live bodies with capability `0x9`, state
  `0x8000` without `0x4000`, not itself, under 4,500 by `FUN_00425370`'s
  distance and with no recent relation either way (`FUN_00411A20`), those
  more than `0x200` ahead with lateral and vertical offsets each smaller than
  that; the lowest forward distance plus twice both offsets wins;
- with one, adds the elapsed time to the flight time, aims 500 above it when
  it is farther than the speed, and takes the inverse-sine angles of the
  lateral and vertical offsets over the distance (a target behind gives
  ±`0x2000` past them), each capped at the elapsed time over 32; the larger
  turn is whole and the other halved, pitch through `FUN_00457A90` (up and
  forward rows), yaw through `FUN_00457C30` (lateral and forward);
- below a quarter of Sub-A's base speed adds q31(Sub-A word 0, elapsed << 11)
  to the speed, capped there; the velocity is the speed along the forward
  row, plus an eighth of it along the up row while untargeted or farther than
  half the speed.

Sub-B and Sub-D are never called. The port runs these as the native weapon
owner's `GuidedMissile` kind (`native_entity_weapons::guided`). Retail lab
(Level 1, master code, F9, ten B presses, one missile at tick 929): it
flies 19, 54, 90, 125, 160, 195 and 226 units along Z in successive
five-tick spans, then 230, wobbling with the roll, and dies at tick 983
through `FUN_0040BAC0`. The port from retail's player state with the same
keys fires at frame 239: 24, 59, 94, 130, 166, 201, 228, then 230. Its
player had drifted to the opposite pitch (306 against retail's −295), so
its missile sinks where retail's rises and meets the ground 13 ticks
sooner.

**Virus Bomb and Antidote Bomb.** Classes 77 and 78 hold `FUN_00441850` in
their descriptor's `+2C`, which the particle free `FUN_00442B40` calls for
every record it frees: at the end of the bomb's 50-tick life, and after a
ground, entity or static hit. A solid static hit (`FUN_0043FF10`) runs it
twice: its `+24` callback `FUN_0043F890` calls `+2C` itself after the F610
burst, then plays sound 62, and the free that follows calls it again. The
bulk reset `FUN_00441B10` calls no callback. The bombs' case of
`FUN_00441850` plays sound 90 (one random word for its rate), runs
`FUN_00456710` at the particle with radius 500, infecting (77) or curing
(78), then the debris (`FUN_00441A50`, class 5 or 6, 40000, the null owner
`DAT_004DCA00` and the particle's `+1D` bit 0).

`FUN_00456710` is `FUN_00437630` then `FUN_00414C50`:

- `FUN_00437630` (nothing without a terrain table) steps X outer and Z inner
  from −500 by 256 up to 500 (offsets −500, −244, 12 and 268, so −500 never
  fits), wrapping the words, and calls `FUN_00433720` at a sample point when
  its horizontal offset squared is below 500² and that plus the square of
  the height above the cell's ground (the signed height byte << 5) is too.
  Each write runs the infection listeners at that moment (the radar's
  redraw, the static object's carriers), as every infection write does.
- `FUN_00414C50` walks the live list and, for each body whose state masked
  by `0xB800` is `0x8800` (clean, Virus) or `0xA800` (infected, Antidote)
  and whose position is within 500 by the wrapped word differences, calls
  the infect (`FUN_00411250`) or cure (`FUN_00411320`) wrapper with no
  packet, the body's position `+96` and velocity `+9C`; it reads the next
  body after the call. With no packet the wrapper sets or clears bit
  `0x2000`, plays the type's `+0x82`/`+0x84` cue (infect only while not
  dying) and runs the type's `+0x18`/`+0x1C` hook (the current style's
  `+20`/`+24`) with a zero packet sum, but skips `FUN_00411030` and
  `FUN_00415040`.

Retail lab (Level 1, master code, F9, 21 weapon presses, the Virus Bomb
held for 60 ticks from tick 663): three bombs ran the tail from the free,
two at the end of their life (ticks 715 and 735) and one at tick 739. Each
drew its sound word, then wrote 7, 8 and 9 cells (the radar drew once for
each changed cell: 7, 3 and 9; the second bomb's other five were the
first's), then left its class-5 debris; no body was in range. The port's
retail test computes the same 24 writes in the same order from the three
origins over world 13 and changes the same cells.

The port runs it whole. The particle pool keeps each shell or bomb record it frees
during a traversal slot, and the slot's end runs the tail for each (the
static hit runs it first itself, then sound 62, then the free's). The
Playing host runs `FUN_00456710`: the cells through the infection write and
its listeners, then the live list, each eligible body as a no-packet hit
(`ParticleEntityImpact::no_packet_hit`, the body's own position and
velocity) through the owner a class-5 or class-6 hit takes: the cleansing
vehicle, the player's model switch (Player Control's style hooks are null),
or the shared actor owners. Each owner sets or clears the bit, plays the
cue and runs its style hook as its packet hit does, then returns before
the impact reaction and the checked damage. A body in range whose state is
unresolved, a peer's body, a body without a port owner or an owner's
logged stop ends the walk with a "Bomb infection radial blocked" line; the
cells and bodies before it stay done and the debris follows. A host
without the radial (Intro2) logs "Bomb tail held" and still plays the
sounds and the debris.

**Flight.** A bomb leaves as any particle weapon does: `FUN_00424650`
aims the body's up and forward rows by the barrel angle, and
`FUN_0044E770` gives it the row's speed (2000) along that direction plus
the craft's velocity. `FUN_00440A60` then adds the class's `+0x2A`
vertical bias, +50 for classes 4, 54, 77 and 78. The particle is mode 3:
`FUN_00440120` integrates its words, `FUN_0043F260` takes the gravity word
(29 a tick at 20 ms, half while fully submerged), the entity sweep and the
static sweep run, then the water entry and the terrain test each call
`+0x18`, `FUN_0043E300`, with the material's selector and the surface word
(the wave, or the ground: `FUN_0043DB60`'s bilinear height while the record's
`+1D` bit `0x10` says it is on screen, else the cell's). `FUN_0043E300`
bursts the class `DAT_004CD788[selector]` (`FUN_00440DC0`, scale 10000, the
null owner), plays sound 90 and, while both horizontal velocity words are
below `0x4B1`, returns one so the record is freed (and its `+2C` runs);
otherwise it puts Y on the surface and halves and reverses the vertical
word. A bomb or shell moving faster than that skips along the sea and
slides over the ground until its life ends.

The `FUN_00441A50` debris of the bombs and of class 4 takes the null owner,
and its record's source-type byte is never written: it is the stack residue
under the call, the third byte of the particle's address plus 8 (`0x4D`)
after class 4's `FUN_004566E0`, and the third byte of the class (0) that
`FUN_00456710` saved.

Retail lab (Level 1, master code, F9, the Virus Bomb fired at tick 664):
the launch record holds direction (0, −0.140, 0.990), the craft's velocity
(0, −1, −218) and speed 2000; the bomb is born with (0, −232, 1762), falls
29 a tick, meets the sea after 17 ticks and bounces (362), settles at 9 or
10 a tick, rides the beach up and ends its life at (19850, −303, 16072),
where the tail runs. A port retail test from the same birth over world 13
matches every tick's vertical word, Y and Z, and the tail's origin. The
class-4 and class-54 shells share this flight and `FUN_0043E300`, and run
`FUN_00441850` at every free as the bombs do (twice at a static hit).

A second lab run moved the player to (20480, −596, 13710) before the shot,
so the first bomb's life ended at (20615, −768, 15664) beside Level 1's
Type6 at (20480, −768, 15360) (state `0x0EC2C805`): `FUN_00414C50` infected
that body alone through `FUN_00411250` with no packet. The radial drew nine
random words, one radar redraw for each of its nine changed cells, and the
body none. The port's retail test runs the radial from that origin over
world 13 and infects the same body alone, changes nine cells and leaves the
random stream where retail's was.

The port computes the launch direction and velocity from the body basis and
the barrel in floating point and rounds them to words, where retail floors
its Q31 products, so a launch word can differ from retail's by one.

**Megablast.** Operation `0x39` adds its amount to controller `+195` (at most
4) and births one Type65 in mode 0 at the player, owned by it (`+60`).
`FUN_004443B0`, the held-fire descriptor's callback, runs `FUN_00444400`
while A and `/` are held together: with a count, it takes one, plays sound 79
at a random rate up to an eighth higher, and births three Type65 in mode 1
(the birth record's word `+20`; the pickup's is 0) at the player's position,
velocity and attitude, the
second and third with pitch `+0x4000`, the third also roll `+0x4000`, then
queues text `0xC9`. Class 55's initializer `FUN_0040B140` clears Secondary
and Tertiary and builds `FUN_00407800`'s Primary with the body's owner and `+88` word:
Sub-K variable 2 holds that mode, variable 1 starts at `0xFFFF` in mode 1 and 0 in mode 0. Each visit of
`FUN_004078C0` puts the ball on its owner with zero velocity, turns pitch by
the elapsed microseconds over 64 and roll over 32, and sets state `0xA0`.
Any mode but 1 charges variable 1 by the elapsed time over 8 and dies (tag
`0x9C00`, `FUN_0040CEF0`) once it would pass `0xFFFF`. Mode 1 drains it by
(elapsed >> 12) × clamp(variable 1, `0x5000`, `0xD000`) >> 7, and while
draining requests the one-frame terrain flash (`FUN_00456740`, session
`+293`) when a random word falls below a quarter of (`0xFFFF` less variable
1). When it would go below zero the ball takes its owner and the owner's
attached bodies out of checked damage (`FUN_00444330` for the player,
`FUN_0040EAB0` otherwise), runs the radial `FUN_004566E0` at itself with
radii `0x800`/`0x1000`, impulse 2000 and packet `[4,3]/[25000,25000]` over
the owner's type and handle, flashes the screen (`FUN_00456750`), restores
the admission and dies. Retail lab (Level 1, every cheat, F9 then A and `/`
at tick 1010): three Type65 births that tick and three radials from
`00407AE6` at tick 1080.

The port runs it whole. Type65 is a row of the class0 constructor (model
243, mass 1, capability `0x40`, health 1, `+C0` `0x21084`, Sub-K `[1, 2]`
with two model words, alternate class2), so its visits take the shared
scheduler prefix and its model reads the charge in selector 1 and the mode
in selector 2. Controller `+195` is saved at state `+145`. A pickup's
count is raised in the contact pass and its Type65, sound 5 and text follow
the contact's own feedback; a press of A and `/` (or the pad's chord) runs
the deploy before the cheat keys, as the craft callbacks do. The blast's
radial is the playing world's (static cells, then the live list), between
the shield and its restore; the shield covers the player and its hold
(`+220`). The pod's death is `FUN_00410C10` into class2 (sound 62, tasks
cleared, removal staged). The terrain flash adds `0x10` to every cell of
the next presented frame's light window, after its clear and anchor, as
`FUN_00453410` does. With a test deploy the three pods blast 70 visits after
their birth frame, as in the lab. A blast whose owner is not the player
(retail would read a null record) stops the pod's visit with a logged
block.

**Defender Ball and Autobot.** Operation `0x38` is ported (see
[CHEATS](CHEATS.md#port-status)). Neither type has capability `0x1000`, so
both join the controller's `+220` list. Both are ported. What their owners do:

- Type63 (model 244, health 1, no components, alternate class 2): class 50's
  initializer `FUN_0040B0F0` clears Tertiary then Secondary and builds the
  Primary (`FUN_00407070`, tick `FUN_00407150`), then draws one shared random
  word into the pitch. Each visit puts the ball on its owner (`+60`) with zero
  velocity, adds the elapsed microseconds >> 7 to the pitch and
  (>> 5) + (>> 4) to the roll. Its pair hook `FUN_0040C800` gives any touching
  body but another Type63 a checked channel-1, 40,000 packet attributed to the
  ball.
- Type64 (model 248, health 3000, Sub-D and Sub-E, alternate class 11): class
  53's `FUN_0040B200` clears Tertiary then Secondary and builds `FUN_00407280`.
  Each visit zeroes the velocity; with a target from `FUN_00422CD0` (the
  closest admitted body other than itself by the class record's range and
  capability mask) its goal is the owner's position less 300 times the unit
  vector from the target, its pitch decays by a quarter, and it steers through
  Sub-D (`FUN_0041F660`) and fires its emitter (`FUN_00424650`) at the target;
  without one it eases each Euler word a quarter of the way to the owner's and
  takes as its goal the owner's position plus 150 and 350 times words of
  the owner's basis rows (`+0C..+20`). It then
  closes on the goal by q31(elapsed << 13, goal − position) per axis and damps
  its roll by (elapsed >> 10) × roll >> 8.
- The player's damage modifier `FUN_004484A0` walks the `+220` list after its
  feedback: the first live (nonzero state, not dying) Type63 takes the whole
  packet (`FUN_00415040` with no ratio) and the craft takes none (its
  `FUN_00414E90` runs with 0); otherwise every live body in the list takes the
  packet too, with state bit `0x8000` set for the call and cleared after it
  when it was clear before and the body still lives, and the craft takes it as
  well. A dead or dying body is skipped. Player death (`FUN_00447280`) never
  reads the list; only the controller's destructor `FUN_004478E0` kills its
  bodies. Retail lab (Level 1, F9, firing every weapon): at tick 1668 the
  player's own blast reaches the craft first in the live list; the ball takes
  19,516 and dies, the craft takes 0, and the dying ball's own radial visit
  then only meets its buffer. At tick 1787 the Autobot takes the walk's packet
  through its own filter before the craft takes its own.

The port runs the Defender Ball whole on the class0 constructor (model 244,
mass 10, capability `0x40`, health 1, initializer state `0x404`, axis
`{0, 8}`, alternate class2, death cue 62): class50's Primary, the pitch draw,
the visit, and the class2 death (`FUN_00410C10`: health 0, dying, cue 62,
tasks cleared, removal staged) from the hold walk, a particle hit or a radial.
The walk runs in every player hit that goes through the checked-damage owner
(shots, particles, radials) and in a collision on the craft. Without a live
ball, each live Autobot in the list takes the packet under a temporary
`0x8000`, and the craft then takes its own hit in full; a live hold body
without a port owner stops the craft's hit with a logged block naming its
type.

The port builds the Autobot on the class0 constructor too (model 248, mass
10, capability `0x40`, health 3000, initializer state `0x84`, axis
`{3072, 0x18}`, alternate class11, death cue 62), with the two components
`FUN_00409A80` gives it: Sub-D (`FUN_004203D0`, which spends one process
stagger seed; divisor 16, yaw coupled into roll, pitch steering on, no
terrain classes) and Sub-E (method 1, 200 ms, spread 1, aim threshold 3000,
axis tolerance 1536, sound 88, the method-15 auxiliary command). Neither
component, the selector nor class53's initializer reads another random
word. The search is the class-7 selector (`FUN_00422CD0`) over the live list
with the component table's `+28` pair, a copy of the type's range 3072 and
capability mask `0x18`; it reads no random word and writes nothing, so the
port runs it ahead of the visit. The no-target visit is whole: the velocity
cleared, each Euler word `(3 × own + owner's) / 4` (truncating), the goal
(each product truncated to a word before the sum), the per-axis close and the
roll damping, which also runs when the owner does not resolve. Retail lab
(Level 1, master code, F9 at tick 810, `wpn4-hold`): the Autobot is born at
the player and holds formation until tick 1787 with nothing in range; the
port's formation step reproduces its position at ticks 812 to 817, 1700,
1701, 1786 and 1787 exactly from the player's logged position and Euler
words (the player's matrix is `FUN_00413F70` of them).

The target branch is whole too. The goal is the owner's position less 300
times `FUN_00457960`'s unit vector from the target to the Autobot (the
difference of the two position words taken as integers, not wrapped), each
product truncated to a word. The pitch eases a quarter toward level, then
Sub-D (`FUN_0041F660`) runs without terrain classes: its stagger byte, then
`FUN_0041E700` on the Autobot's own matrix (the one the class0 host's
`FUN_00413F70` built from its Euler words after the previous visit), the
yaw rate (`FUN_00420260`) from the lateral projection and the pitch rate
(`FUN_004202E0`: damped as the yaw rate is, then the vertical projection
over the divisor added, twice when it opposes the rate, clamped to
±`0x20000` over the divisor) from the vertical one, both by the visit's
elapsed time. `FUN_00420360` turns them into steps by the frame's elapsed
time (`DAT_004D04E4`): the yaw step comes off the heading and, coupled,
the roll, the pitch step off the pitch. Sub-E then fires straight from the
visit (`FUN_00424650(self, owner, target, null, ...)`, the emitter every
Aim and Fire task calls): its runtime is `FUN_00424E30`'s zeroed record
with method 1 and sound 88, built at birth with no random word; one
cadence draw a visit admits a shot one time in ten (200 ms over the 20-ms
visit), the axis tolerance 1536 and the aim threshold 3000 gate it, and
each shot is a method-1 command followed by the method-15 auxiliary. The
command's `+24` owner word is the owner, so `FUN_00414870` launches each
shot owned by the player under the Autobot's type byte: class 1 at
method 1's 4000 plus the Autobot's velocity, and the auxiliary's class 32
(dropped at or below the sea), from the selector's muzzle stamp (the
transform-only pass) or the centre. A nonzero return from the append
would end the visit early in retail; the port's append always succeeds.

Retail lab (world 20 by Skip to World, F9 at tick 955, the actor walk read
with the Euler words and matrix): the Autobot holds formation until tick
1079, when the search finds a Type15 about 6,000 away, and leaves
formation to sit about 300 from the player toward it. From its logged
pose, matrix and target, the port's turn reproduces the Euler words of all
17 visits from tick 1079 to 1095 exactly, chaining the Sub-D rates from
zero, and each logged matrix is `FUN_00413F70` of the visit's Euler words;
its close reproduces every position from the owner's position (not
logged at the visit: the Defender Ball's next position, which copies it,
or the player's next, one of which matches at every tick). Every visit
makes the cadence draw (`00424711`) and none fires: the target is beyond
the axis tolerance. A port probe of the same keys (world 20 from retail's
arrival pose, F9 at frame 162) leaves formation 68 frames after F9 (retail
124 ticks: the Type15 flyers' paths differ, the port's random words not
being in lockstep) and then keeps about 300 from the player toward its
target with no stop.

**The Autobot's death.** A living Autobot's state lacks `0x8000`, so only
the player's walk (above) reaches it, unless a Megablast's restore has set
the bit (its class53 policy lacks `0x80`); then particles, radials and pairs
do too, as does the Main Base abort's `FUN_00410C10`. Every lethal entry
takes `FUN_00410C10`: health 0, dying, sound 62, then its alternate class11
(`FUN_0040C660`): Secondary then Tertiary cleared and `FUN_00404360`'s
Primary (tick `FUN_00404460`, no timeout). Type10's class11 also resets its
Sub-G with a random word; the Autobot has no Sub-A, G or H for
`FUN_00406070` to reset, so its death reads no random word. Class11's
policy sets `0x8000` and `0x10000` and clears the rest of the type's `0x84`,
so its effective flags are 0.

The class0 host keeps visiting it. A detailed visit spins it
(`FUN_00404460`: the multiplier is 3 less the task's elapsed seconds, at
least 1; roll by it times `dt >> 7`, heading by it times `dt >> 8`, pitch
by it times their sum); `FUN_004018A0` finds none of its components. With
effective flags 0, `FUN_0040E100` applies gravity, `q31(dt, 0x300000)` off
the vertical velocity (29 a tick at 20 ms), and nothing damps it. Its
contacts are Type10's class11 ones, shared: `FUN_00411AD0`'s terrain,
water and static phases, each through class11's `FUN_0040C750` into the
completion style (`FUN_0040BAC0`): the `FUN_0040BAF0` burst (no F, H, N or G
block and capability `0x40`: sixteen particles of classes 94 and 95), the
type's radial (inner `0x200`, outer `0x400`, impulse 2000, `{[1,3],
[4000,4000]}`, attributed to itself), the tasks cleared and the removal
staged, then the contact's own response. Its static gates read capability
bit 1 (`FUN_00427E20`) and effective `0x400` (`FUN_0040D920`); the Autobot
has neither.

Retail lab (Level 1, master code, F9, `wpn4-hold`): at tick 1787 (world
clock 1435) a radial reaches the craft with `{[2,3], [3025,2593]}`; the walk
hands the Autobot that packet, which its profile filters to 7211, and it
dies into class11 before the craft takes its own 8011. It starts its fall
moving at `(1561, 631, 402)`. From its pose at its first tumble
visit (tick 1788), the port's class0 host reproduces its position, velocity
and Euler words at every visit to tick 1846; that tick's contact pass enters
the sea (world clock 1494), runs the terminal, and the hard entry leaves
its vertical velocity at -540, as retail's tick 1847 shows, with a Type60
ring.

A second lab run (the same keys to F9, then `FUN_00410C10` called on the
Autobot at tick 897) shows the falling body in the pair walk: at tick 914
it passes the Defender Ball, whose hook gives it a checked `{1, 40000}`
(34000 after its filter, harmless to a dying body) and whose contact
response stops its fall; it ends in the sea at tick 942, a soft entry that
leaves its velocity. The port's pair walk admits it (the ball's hook needs
its class0 owner, which holds the tumble); that pass was not compared tick
by tick. A port probe of `wpn4-hold`'s keys (Level 1 from retail's player
state, the keys 400 frames after the arrival) loses the ball at frame
1380, then the Autobot at 1442, which tumbles to the sea and is removed 37
frames later with no stop (the two worlds' blasts differ: their random
words are not in lockstep).

A body that has finished its terminal keeps its allocation until the
removal sweep; a particle hit on it in that window stops with a logged
block ("completed actor custody").

The ball's pair hook `FUN_0040C800` runs in the late pair walk (see
[ACTOR_RUNTIME](ACTOR_RUNTIME.md#the-ports-walk)): another Type63 returns
`0x004BE320`'s A300, which cancels the response; any other touching body
takes a checked `{1, 40000}` attributed to the ball (its type and handle),
then the visit goes on. A Main Base (Type6) takes it through its own
`FUN_00415040` entry, whose lethal hit is its `FUN_00410C10` revival; a
Level 1 run had logged that pair as held ("Active pair Type6 x Type63
stopped before its visit"), and a test now walks it with no stop. The ball never meets the craft it sits on: retail's
pair probe finds no overlap at their shared centre, and a retail lab run
(Level 1, F9: the ball lived ticks 811 to 1669 on the player) logs no
`FUN_00415040` from the hook; the port's probe agrees. A collision on the
craft while a live ball is in the hold runs the modifier inside the craft's
pair hit: the ball takes the collision packet (its source the other body)
and the craft none, the ball's death preceding the craft's hit sound. A
second craft collision in the same pass after the ball's hit stays
unresolved (a typed block of the pass). `FUN_004484A0`'s closing cue is ported for
every hit it modifies: with no shield buffer (`+50`) and the craft's health
(`+30`) less the modifier's returned damage (zero when a ball took it) in
5,001..30,000, and the clock more than 120 ticks from the last cue (its own
process-global latch `DAT_004DE85C`, from zero), it plays sample `0x2D`
through `FUN_004958C0` at half gain, apart from `FUN_00446640`'s warning and
its latch.

**Power-ups carrying operation `0x38`.** Worlds 20 and 24 each author a
Defender Ball (`0x3F38`) and an Autobot (`0x4038`) power-up, the only `0x38`
payloads in worlds 13 to 49. `FUN_00445A90` takes the power-up (returns 1)
with no cue or text, unless the `+220` list refuses the body (a second live
Type64, or 15 live bodies): then it still builds the body, removes it
(`FUN_00410B70`) and leaves the power-up. The port decides that at the
contact, staging the pass's own births, and builds the bodies after the pass
in contact order, the refused ones too. A type with no port constructor at
the player, or one with capability `0x1000` (the cargo branch, which no
power-up carries), leaves the power-up with a logged stop; both of these
worlds' payloads now have one.

**Seedpods.** Selector `0x10` (world 20's power-up `0x6410`, or the master
descriptor's callback 11, as the bombs) fires `DAT_004D02C0` row
`{0, 2000, 50, 0}`: the class50 carrier at speed 2000, cadence and sound 88
from its master descriptor. Its contacts (`FUN_00441EA0` surface,
`FUN_00441FA0` entity, `FUN_00442070` static) burst into class5 virus
particles; no body is built (Type65 is the Megablast's pod). The port fires
it through the particle path its virus carrier already runs.

**Lightning Shield.** No Section-12 type selects class 52, as a behavior or as
its alternate, so `FUN_0040B1B0`, `FUN_004076E0` and `FUN_00407770` never run.

The port fires nothing for a selector without a firing profile and logs one
stop per selector and run while its trigger is held: "Possession Ray held"
for `0x1B`, "Weapon selector ... held" for the others.

### Wind on trail particles

| Routine | Does |
| --- | --- |
| `FUN_0044F0C0` | the wind owner's particle form, for class-31 trails (`FUN_0043F290` passes mass 5): on the selected side of the sea, pulls the velocity toward the current wind vector scaled by the capped clearance above the upstream terrain, as `FUN_0044EC60` does, then applies the subtraction drag |
