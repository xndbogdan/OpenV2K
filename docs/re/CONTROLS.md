# V2000 In-Game Control Scheme (ground truth)

Reverse-engineered from `V2000.EXE` + `decompiled/bulk/game_logic.c` (multi-agent
workflow, 2026-07-01). Every gameplay key resolves through a **static data-driven
binding table**, not per-scancode `if` branches. VAs are ImageBase 0x400000.

> **2026-07-02 correction** (see [PLAYER_CRAFT.md](PLAYER_CRAFT.md) for the full
> RE): the hover column below originally over-generalized the fly-mode reading.
> In HOVER, the pitch channel (UP/DOWN + S/X) drives a **separate gun-barrel
> elevation joint** (`weapon_state+8`, clamp −0x800..+0x3000) — the body does
> not pitch or translate from it; forward propulsion is the **SPACE throttle**
> channel. In FLY the barrel is decayed to 0 every frame (**aim locked**, trigger
> still works) and the pitch channel becomes body-pitch rate → tilt-to-fly.
> The keymap table below has been fixed accordingly.

## Pipeline

DirectInput poll `0x004AC6D0` (GetDeviceState `0x004AC869`) → 256-byte DIK buffer
`0x004FED80` → callback `FUN_00472560` builds key-down bitmask `0x004FBD78`
(LEVEL-only, no edge logic) → binding evaluator `FUN_00471DE0` (`game_logic.c:52193`)
walks the table and writes channel values/hold-times into the control struct `S` →
reader **`FUN_004445E0`** (all modes) collapses channels into the motion vector at
`S+0x280` → per-frame update **`FUN_00446640`** (`:35658`) applies it via the
mode-selected integrator (`DAT_004CD8D0[style]`).

**Binding table @ `0x004C2B38`** (29 × 20-byte records). `FUN_00471DE0`
interprets bytes `+4..+7` as keys that must be down and bytes `+0x0C..+0x0F`
as keys that must be up; the latter field was previously mislabeled as a
modifier. Descriptors @ `0x4CD838` (8-byte `{callback, flag}`; `flag==6 &
callback==NULL` = analog/held axis, `callback!=NULL` = discrete press). Slot map @
`0x004CDBA0`.

**Bindings are IDENTICAL in both modes** — hover vs VTOL differ only in the
integrator that interprets the same motion vector, not in the keys.

## Keymap — KEY → HOVERCRAFT | VTOL/HELI

| Key (DIK) | Type | Hovercraft | VTOL/Heli |
|---|---|---|---|
| **SPACE** 0x39 | held | throttle → **forward propulsion** (fan spin-up; no fuel) | powered **ascent** (burns fuel) |
| **RSHIFT** 0x36 | held | throttle **down** / brake (same-direction fan spin-up) | descend (same-direction fan spin-up) |
| **UP** 0xC8 | held | **gun depression** (aim down; body unaffected) | body nose-down → **forward** |
| **DOWN** 0xD0 | held | **gun elevation** (aim up) | body nose-up → back |
| **LEFT** 0xCB / `,` | held | **steer/yaw left** (direct) | bank-turn left (yaw+roll) |
| **RIGHT** 0xCD / `.` | held | **steer/yaw right** (direct) | bank-turn right (yaw+roll) |
| **S** 0x1F | held | gun depression, finer 0x900 scale | body nose-down; suppresses SPACE's separate powered-pitch coupling, not Self Righting |
| **X** 0x2D | held | gun elevation, finer 0x900 scale | body nose-up; suppresses SPACE's separate powered-pitch coupling, not Self Righting |
| **ENTER / right mouse / joystick button 1** | held\* | **fire primary weapon** | fire (**NOT locked**) |
| **A** 0x1E | press | special-weapon deploy (cap 4; cb 0x00444400) | same |
| **ENTER+LALT** | press | confirm / special-deploy | same |
| **E** 0x12 | press | next target / rotate selection list | same |
| **B/PgDn/LShift/L** | press | next weapon | same |
| **V/PgUp/CapsLock/K** | press | previous weapon | same |
| **D** (LShift up) | press | beam-out / drop (`entity+0x20E=−80`) | same |
| **C** (LShift up) | press | beam-in / collect (`entity+0x20E=+80`) | same |
| **TAB** 0x0F | press | **Hover↔VTOL toggle** (cb 0x004442B0) | toggle back |
| **Mouse X / joystick X** | analog | steer (see [mouse and joystick](#mouse-joystick-and-pointer)) | bank-turn |
| **Mouse Y / joystick Y** | analog | gun aim: pulling back elevates | body pitch: pushing forward drops the nose |
| **Left mouse / joystick button 2** | held | same as SPACE | same as SPACE |
| **Mouse wheel** | per frame | towards you: next weapon; away: previous | same |
| (console pads only) | held | VTOL vertical ±0xA00; no PC device binds it | same |

SPACE and RSHIFT contribute `+0x10000` and `-0x10000` to one signed throttle
channel, but fan speed uses that channel's magnitude. Either key therefore
accelerates the fan in the same rotational direction; holding both cancels to
the idle fan rate.

The weapon/target labels above were corrected from accepted retail evidence on
2026-07-20. Callback `0x00444180` walks controller descriptor slots forward;
`0x00444210` walks them backward. Their binding groups are respectively
B/PageDown/Left Shift/L and V/PageUp/Caps Lock/K. Callback `0x004442E0`, bound
to E, instead rotates the separate 12-byte selection list through
`FUN_00418980`. A 1-kHz pickup/fire capture independently observed four
Left-Shift weapon changes as active-slot transitions followed by complete
descriptor swaps. The older action-name interpretation had these families
reversed even though its raw key grouping was correct.

\* A foreground retail capture observes Enter and right mouse feeding the same
held trigger. This supersedes the earlier Ctrl inference from the isolated
input-reader decompile; Ctrl does not fire in the captured retail configuration.

The beam bindings deliberately **exclude** LShift. Their raw records require
DIK D (`0x20`) or C (`0x2E`) at `+4` and place DIK LShift (`0x2A`) in the
must-be-up field at `+0x0C`. A discrete edge therefore occurs when C/D is
pressed while LShift is up, or when LShift is released while C/D remains held;
holding LShift suppresses the binding. The older `C+LShift` / `D+LShift`
reading inverted this field's meaning.

**Motion vector `S+0x280`:** +0x00 turn/heading; +0x02 pitch channel (UP/DOWN
scale 0xD80 and S/X scale 0x900 sum here) — routed per mode:
hover → gun-barrel joint, fly → body-pitch rate; +0x04 vertical (VTOL); +0x08
throttle (hover forward propulsion / fly ascent); +0x0C fire. VTOL's bounded
SPACE coupling and subsequent Self Righting recurrence are separate fly phases.

**Retail VTOL attitude observation (2026-07-17):** SPACE supplies lift but does
not level the body. In the captured Self Righting mode 10, the normal-player
`FUN_0041A690` recurrence seeks raw pitch 6028/6029 with no thrust. SPACE's
bounded `-pitch/6` coupling balances that seek at about 1854 (10.2° nose-down),
so the craft eventually drifts forward; sustained UP balances near 10469
(57.5° nose-down), while sustained DOWN balances near -7572 (41.6° nose-up).
Holding an approximately level attitude therefore requires brief UP/DOWN
corrections chosen by eye, not a fixed programmable cadence. After backward
travel, releasing pitch while SPACE remains held returns toward the 10.2°
forward equilibrium. Modes are materially different: 0 disables pitch
correction and 1 damps toward level, while 2..15 seek progressively steeper
nose-down targets. Runtime traces retain key edges and the full body basis so operator
corrections can be separated from lift/ceiling response.

LEFT/RIGHT use the same type-46 Sub-D signed-word step in both modes. VTOL also
subtracts that step from roll before its near-surface force correction, after
which `FUN_0041A690` applies one `roll -= ((dt>>10)*roll)>>8` damping pass. The
trimmed July 17 trace replays all 53 clean sustained steering transitions
exactly; short partial-key frames remain a duty-cycle sampling limitation.

The pitch-channel signs are statically conclusive: binding records `0x4C2CC8`
(UP) and `0x4C2CF0` (DOWN) map through slots `+0x50/+0x54`; the reader adds UP
and subtracts DOWN. Hover then performs `barrel -= channel/2`. Consequently
UP/S depress the gun and DOWN/X elevate it. Positive barrel angles are world-up;
the asymmetric clamp allows only 11.25° depression but 67.5° elevation.

**NOT bound in the 29-row craft table (definitive):** weapon keys 1–6
(menu-only, 0x4C05F0); zoom; HUD toggle; self-right keypress (it's an options
auto-stabilize setting → `S+0x20C`); boost; horn; secondary-fire.

**Fullscreen map is bound separately:** gameplay also registers UI descriptor
`0x004C2AD8`, whose keyboard row at `0x004C26D0` binds DIK **M** (`0x32`) to
`FUN_00456610`. A press with `session+0x296 > 4` and `DAT_004F741C == 0`
switches to map descriptor `0x004D0A68`. The map's own table at `0x004C4610`
binds M (along with its cancel/action aliases) to `FUN_00456650`, returning to
gameplay descriptor `0x004D0950`. No arrow, keypad plus/minus, pan, or zoom
binding exists in the map keyboard or joystick tables.

## Mouse, joystick and pointer

FGDK gives each input device a type, and the type selects that device's row
in the active binding set. `FUN_004292A0` always returns 0 on the PC, so play
uses craft set `0x004C4120`: slot 0 is the keyboard table above, slot 1 the
mouse and slot 2 the joystick. Slots 6 and 7 hold console-pad tables that no
PC device uses. Menu sets bind only the keyboard and those console slots, so
the mouse and joystick do nothing outside the craft.

**Mouse.** A DirectInput `GUID_SysMouse` device with relative axes, read in
the background (the game never sets a cooperative level). Binding table
`0x004C2DE0`:

| Input | Reader effect | Hovercraft | VTOL |
|---|---|---|---|
| X | turn `+= 200·Δx` (mickeys, right positive) | steer | bank-turn |
| Y | pitch `+= -200·Δy` (towards the player positive) | pull back: elevate gun; push: depress | push: nose down, forward; pull: nose up |
| Wheel | one weapon step per reader call when the total changed | towards the player: B's step (`0x004441D0`); away: V's (`0x00444260`) | same |
| Button 1 (left) | both SPACE descriptors (`0x4CD838` throttle, `0x4CD848` positive-thrust gate) | thrust | ascent and powered-pitch gate |
| Button 2 (right) | fire descriptor `0x4CD850`, shared with Enter | fire | fire |

The analog binding scales each axis by -400 and Q31 one half. The reader adds
`previous - current` of X and `current - previous` of Y as signed words, so
one fast frame can wrap: 164 mickeys exceed `0x7FFF`. The wheel consumer holds
the negated `lZ` total. The reader steps forward when it rose and backward
when it fell, never more than once per call.

**Joystick.** WinMM: `FUN_004AC110` probes up to 16 devices with
`joyGetPosEx`/`joyGetDevCapsA`, and `FUN_004AC2A0` polls them every 20 ms with
`JOY_RETURNALL | JOY_RETURNCENTERED`. The type-2 adapter row of `0x004CA448`
subtracts `0x8000` from each axis. Binding table `0x004C2E90`:

| Input | Value | Effect |
|---|---|---|
| X | `(x - 0x8000) >> 3`, then the dead zone | turn, right positive |
| Y | `(0x8000 - y) >> 3`, then the dead zone | pitch, forward positive: depress gun or nose down |
| Button 1 | fire descriptor | fire |
| Button 2 | both SPACE descriptors | thrust |

The reader's dead zone (`0x0044463F..0x00444663`) maps `v < -2000` to
`v + 2000`, `-2000 <= v < 2000` to 0 and larger values to `v - 2000`. Full
deflection is therefore about ±2096 words, close to a held arrow's 2303. Z, R,
U, V, the POV hat and buttons 3 onward are unbound.

**Reader order** (`FUN_004445E0`): clear turn and pitch; read the two joystick
terms; compute the arrow terms; in Relative mode add arrows plus joystick to
pitch and turn, in Absolute mode pass those sums and settings `+0x28` to
`FUN_00444B90`; then mouse Y, the S/X term, mouse X, the console-only vertical
terms, the wheel step, throttle and fire. Every channel update is a signed
16-bit addition.

**Absolute mode** (Joystick setting Absolute, `FUN_00444B90`):
`magnitude = isqrt(pitch² + turn²)` (`FUN_00457730`) and
`bearing = asin(pitch / magnitude)` (`FUN_00457F70`), mirrored to
`-0x8000 - bearing` for leftward input. The arcsine table at `0x004D34D0` is
exactly `floor(asin(i/1024) / 3.14159 × 32768)` for `i = 0..1023`. The bearing
is a heading word: 0 faces world +X and `0x4000` world +Z, whichever way the
craft faces. Ground styles 0 and 4 subtract `(bearing - heading) ×
2·magnitude >> 15` from turn. Flying styles 1 and 3 do the same, but with
Absolute Mode enabled ("Full") a bearing more than a quarter turn off the nose
uses `±0x7FFF - offset` and a negated magnitude, and pitch becomes
`((2·magnitude - body_pitch) × (0x2000 - |turn|)) >> 15 + trunc(body_pitch/4)`.
Style 2 ignores the request. The arrow keys feed the same sums, so Absolute
mode makes them compass controls too. The saturated ratio
`(pitch ^ magnitude) | 0x7FFFFFFF` is -1 for straight-back input, which
therefore aims at bearing 0, or `-0x8000` with any leftward component, rather
than `-0x4000`.

**Pointer.** The window procedure `FUN_00495A00` answers `WM_SETCURSOR` with
`SetCursor(NULL)` only while the game is focused and full-screen
(`0x004FEE8C`, written by the display-mode routines `FUN_004A8C30` and
`FUN_004A8D90`), in menus and in play alike. Retail never confines the
pointer. DirectInput keeps reporting relative motion after it reaches the
screen edge.

### Port behaviour

- In focused play the port uses SDL relative mouse mode. It hides the pointer
  and keeps it in the window so motion keeps arriving at the edge, where a
  windowed retail game shows a free pointer. Menus leave the pointer free:
  hidden over a full-screen game as in retail, visible in a window. Losing
  focus, the F12 debug window and the tilde console free it at once.
- SDL's raw relative motion stands in for DirectInput mickeys. The reader
  terms, wheel step and button bindings are exact. Modern mice report many
  more counts per inch than 1999 mice, so the same gain feels much faster, and
  a fast flick can wrap the word just as it would in retail.
- Pads with an SDL controller mapping are numbered the way Windows' legacy
  joystick API numbers an XInput pad: left stick on X/Y, the south button (A)
  is button 1 and fires, the east button (B) is button 2 and thrusts. Other
  joysticks use their first two axes and their own button order. The port
  reads the first attached device every frame instead of on WinMM's 20 ms
  timer.
- Not reproduced: the reader turns the change in each mouse consumer's running
  total into motion. Retail can therefore apply movement made during a pause
  in the first frame afterwards, and a new craft controller's first comparison
  depends on stored values whose initialization was not traced. The port drops
  motion outside play and starts each frame's deltas at zero.
- The Hover barrel moves by `-(pitch / 2)` once per callback, as in retail,
  instead of scaling by the 50 Hz tick. Keyboard aim speed follows the frame
  rate as it does in retail, and mouse aim is the same at any frame rate.

## Mode toggle + fuel

- Mode vars in the controller struct (`FUN_00443af0`): `+0x86` movement style
  (hover {0,4} / fly {1,3} / special 2); `+0x198` = discrete vehicle-body
  selector 0-3 (player=0), NOT a morph counter; **`+0x20d` = mode-change
  flag** (the toggle trigger); the mirrored style `*(*behavior+0x28)` indexes
  the integrator table. Full state machine: [PLAYER_CRAFT.md](PLAYER_CRAFT.md).
- **TAB** (`0x004442B0`) is the only keyboard writer of `+0x20d`. If linked to
  another entity it drives the convert/possess timer (`+0x70`) instead; else sets
  `+0x20d=1`. Consumed in `FUN_00446640` (`:35838`) → swaps hover integrator
  `FUN_00444CF0` ↔ fly `FUN_00445310`.
- **Fuel `+0x88`** (int 0..200000): burned **only in fly mode** by `FUN_00445A40`
  (`burn = |thrust|·(dt<<11)>>31`); warn `<10000`; **empty `<1` → forces `+0x20d=1`
  back to hover**; pickup type `0x33` is accepted below 190001 and caps the
  result at 200000.
- **An empty-fuel TAB appears refused, but retail implements it as an immediate
  VTOL callback round trip.** `FUN_004442B0` raises `+0x20d` unconditionally
  and `0x447970` commits the fly style. The first `FUN_00445310` call sees
  `fuel < 1`, plays positional sound `0x31`, submits once-per-level resource
  event `0x0B` (table-selected Section-2 string `0xEC`,
  `You cannot fly without any fuel`), and requests Hover again. The port
  collapses the short-lived mode round trip into an explicit refused outcome while
  preserving that feedback. Keyboard down-edge filtering suppresses repeats.
  Direct string `0xDE` is a generic lethal-entity notification, not this fuel
  path; throttled low fuel remains direct string `0xE0`.
- **Initial/refill lifecycle (user-verified 2026-07-16):** the player begins
  with zero fuel and cannot enter VTOL. Type-`0x33` fuel pickups fill the tank;
  reaching zero in VTOL requests the automatic skimmer return on the following
  empty-fuel callback and blocks another switch until refilled. The Rust
  controller now implements the empty start, integer burn, delayed return, and
  exact pickup threshold/cap. Section-10 kind-4 contact, the physical rebound,
  the refill, and removal of the consumed cell are wired too; the 2026-07-17
  retail capture independently observed the same one-sample `0 -> 100000`
  refill and object disappearance. The direct positional pickup (`5`) and
  empty-VTOL (`0x31`) Section-11 cues are wired, as is the strict 70-tick
  non-positional low-fuel warning cadence using the same `0x31` sample.
  Full-tank and low-fuel HUD messages and the separate deduplicated gameplay-
  notification slot remain to be connected.

## Physics / feel constants (from `FUN_004445E0` + settings `0x4CB3D8`)

| Quantity | Value | Where |
|---|---|---|
| Pitch-channel scale (UP/DOWN) | 0xD80 (3456) | 0x4446AC/0x444706 |
| Sensitivity | 10 (default) | settings+0x2C |
| Joystick mode | 1=Relative default; 0=Absolute | settings+0x08 |
| Absolute Mode | 0=Half default; 1=Full (only consulted in Joystick Absolute mode) | settings+0x28 |
| Pitch-rate cap | 0x900 (2304) | S/X |
| Throttle clamp | 0x10000 | SPACE/RSHIFT |
| VTOL vertical rate | ±0xA00 (2560) | console pads only |
| Mouse axis gain | ±200 words per mickey, wrapping | slot-1 binding `0x004C2DE0` |
| Joystick axis | `(raw - 0x8000) >> 3`, dead zone ±2000 | slot-2 binding `0x004C2E90`, reader `0x0044463F` |
| Powered pitch coupling | cap 0x600, gain `-pitch/6`; dedicated SPACE binding active, S/X inactive, Self Righting >1. UP/DOWN and RSHIFT may coexist | FUN_00445310 |
| Default keyboard turn | held key ≈±0x8FF; `step=((turn*52/28)*(dt>>2))>>15`, heading subtracts it; VTOL roll subtracts it before one A690 damping pass | FUN_004445E0 → FUN_00420360 |
| Absolute joystick helper | `motion.turn -= (target−heading)·magnitude·2>>15`; Full flying also writes pitch | FUN_00444B90 |
| Body-pitch bound | no global ±0x1800 clamp in normal VTOL; its mode recurrence is exact. Roles of those constants in alternate branches remain open | FUN_0041A690 |
| Hover-height PD (fly) | P=24/1000, D=5/1000, window altErr∈[−299,999] | FUN_00445310 |

**Entity-sourced:** type 46's cumulative Section-12 record supplies Sub-A
`{1400,-300,3000}`, Sub-B `{0,500}`, Sub-C lift data, and Sub-D divisor 28.
Its constructor randomizes the persistent target to 3000..3298. There is no
Sub-F generic drag, forward-friction factor, or arbitrary Hover speed cap. This
is distinct from common `FUN_0044EC60` environment damping: effective flag bit 3
is set, and zero-wind first-world play uses Section-13 strength 3 and player mass
100 (factor 75 at a 20 ms tick).

## Verdict on prior recollection

| Claim | Verdict |
|---|---|
| SPACE = thrust(hover)/ascend(heli) | ✅ throttle channel: hover **forward propulsion**, fly ascent |
| LEFT/RIGHT = steer/turn | ✅ |
| ENTER = fire | ✅ foreground retail capture confirms Enter and right mouse |
| UP/DOWN = gun-elevation | ✅ in hover (barrel joint `weapon_state+8`); in fly the same channel = body tilt. *(2026-07-02: earlier ❌ verdict retracted — it read only the fly integrator)* |
| TAB = fuel-gated mode toggle | ✅ (user-verified 2026-07-03): empty tank REFUSES Hover→VTOL; VTOL→Hover always allowed; VTOL also drains fuel. Earlier "doesn't gate press" verdict retracted — code location of the gate still open |
| guns locked in heli | ✅ for **aim** (fly integrator forces barrel → 0 every frame); ❌ only for the trigger — fire works in both modes |

## Port status (updated 2026-10-09)

`v2k-game` in-game controls remapped to this scheme: LEFT/RIGHT steer (model yaws),
UP/DOWN aim in Hover or pitch the body in VTOL, SPACE/RSHIFT throttle, TAB Hover↔VTOL (VTOL burns
`fuel`, empty→Hover), with Enter/right mouse/joystick button 1 driving the recovered primary-fire
scheduler, class-1 particle, class-32 above-water flash/smoke, and global sound
88. Dev free-fly camera uses F11; Backquote/tilde toggles the Windows text
console, which starts hidden for a standalone game launch. Type-46 Hover steering,
planar drive/lateral correction, lift ordering, and signed-word position
integration now use the authored A/B/C/D data and recovered fixed-point path.
VTOL manual/assist lift, signed fuel burn, five-probe ceiling attenuation,
previous-basis projection, common underwater response, and the Space-versus-
RShift powered-pitch asymmetry are also ported from the July 16 traces. The
normal type-46 pitch, yaw, and bank recurrence is exact against the July 17
trace. Mouse, wheel and joystick input, the Joystick Absolute mode and the
per-callback Hover barrel step are ported (see
[mouse and joystick](#mouse-joystick-and-pointer)). Partial-frame key duty
cycles, the shared RNG stream, alternate terrain-target attitude state,
runtime turbo/session policy, S/X pitch render, and solid-ground
impact/destruction remain follow-ups.
