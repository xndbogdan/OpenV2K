# V2000 Game Mechanics Reference

Cross-system reference for V2000 gameplay, derived from decompiled source,
original data, and accepted runtime evidence. Detailed actor/task mechanics,
damage/death runtime, and factory economy live in
[ACTOR_RUNTIME.md](ACTOR_RUNTIME.md),
[ENTITY_DAMAGE_AND_DEATH.md](ENTITY_DAMAGE_AND_DEATH.md), and
[FACTORY_SYSTEM.md](FACTORY_SYSTEM.md); this file keeps shared mechanics and
stable routing rather than checkpoint history.

**Status legend:** CONFIRMED = verified in code/data, INFERRED = strong evidence, UNKNOWN = needs investigation

---

## 1. Vehicle / Player State

### Per-World Campaign / Time-Trophy State (CONFIRMED)

Player-controller offset `+0x1F0` stores the per-world time-trophy state, not a
vehicle flight mode:

| Value | State | Transition |
|-------|-------|------------|
| 0 | Dormant/zero-filled | Before the post-load initializer owns the field |
| 1 | Active countdown | Fresh world with a nonzero Section-13 `+0x5C` deadline |
| 2 | Expired | Remaining milliseconds reached the current frame delta; sound 2 |
| 3 | World saved outside active window | Campaign-completion bit `0x1` set without an active timer award |
| 4 | Time trophy secured | Active completion or zero-deadline initialization; bit `0x8` |
| 5 | Results/abort active | Existing results byte at load or post-terrain Main Base abort commit |

Retail `FUN_0042EE70` / demo `FUN_0042E8B0` is the campaign "save world"
handler, not weapon fire. It sets control-slot bit `0x1`; state 1 becomes state
4 and calls `FUN_00456820(1)` to claim independent time-trophy bit `0x8`, while
every other state becomes 3. Selector-`0x3F` hidden pickups use separate bit
`0x2`. Their amount-zero collection restores hull to 40000 and shield buffer
to 100000. A newly secured time award separately dispatches packed operation
`0x13F` (selector `0x3F`, amount 1), which counts one trophy and grants every
fifth trophy's extra life without those restoration writes. The active HUD
trophy and clock are shown only in state 1 and disappear together on expiry.
See the [complete countdown contract](LOADING_TRANSITIONS.md#per-level-time-trophy-countdown-static--port-live-2026-08-10).

The port also implements the explicitly requested physical-pickup clock stop
as a separate, labelled policy. Its static evidence bounds and the authored
red aura, shield damage and persistence are owned by
[Player shield](PLAYER_SHIELD.md).

Additional state flags:
- `+0x1F4`: Campaign-warp request flag. `FUN_0042f140` sets it only from
  `FUN_00456d10` when an authored exit has accepted player contact; it is not a
  force-land command.
- `+0x1F8`: Pending campaign-selector/Main-Base flag set by `FUN_0042F160` and
  consumed/cleared by `FUN_0042DD10`; it is not a can-fly capability.

The EXE strings below describe the separate vehicle-control modes and must not
be mapped onto controller `+0x1F0`:
1. **Hovering** — ground-level movement
2. **Standard Flying** — normal flight
3. **Flying Brick** — heavy/weighted flight (e.g. carrying weight in cargo)
4. **Complex Flying** — advanced flight

### Retail hover/wave controller (confirmed 2026-07-12)

Player type 46 uses Section-12 Sub-C values base/range/near/damping
`150/125/100/200`, strength `0x00900000`, with wave and body-offset flags set.
`FUN_0041F1C0` applies lift to velocity rather than assigning a fixed Y, and
only hard-corrects static-terrain penetration. This produces the observed
temporary glide above a falling wave crest.

Attached cargo mass is summed by `FUN_004185C0`. The hover target changes from
waves to terrain/seabed above attached mass 99. Player self-mass is 100 and the
type-68 weight is 200. `FUN_0040E100` changes underwater buoyancy at total mass
151, so the player alone floats while player+weight (300) sinks. This is the
concrete implementation behind the "Flying Brick"/weight behavior.

Rust `common_mover::sub_c` now owns the exact detached `FUN_0041F1C0`
arithmetic. It keeps the optional 200-unit body-up sample offset, the signed
`mass < 100` wave-selection test, distinct terrain/wave penetration correction,
wrapping low-32 lift numerators and signed divisions, upper-only impulse cap
200, and post-lift removal of velocity directed into the selected surface.
The live player path delegates to this shared phase. Terrain/wave lookup and
generic actor reachability remain caller-owned; in particular, the player
world adapter retains its pre-existing dry-level `sea_level.is_some()` gate
rather than claiming that sampler availability is part of the retail
descriptor/mass predicate.

### Entity Runtime State — Resource Cache Record (CONFIRMED)

Accessed via `ResourceCache_Lookup(entity_handle)`:

```
+0x08  uint32 Entity flags
              bit 0x0004  = orientation dirty
              bit 0x0020  = position changed
              bit 0x4000  = DYING
              bit 0x8000  = visible
              bit 0x100000 = marked for deferred destruction
              bit 0x800000 = grounded/snapped
              bit 0x880000 = drag-active pair
+0x0C  int32[9]  3x3 rotation matrix (fixed-point 1.31 format)
                  rows at +0x0C/+0x10/+0x14, +0x18/+0x1C/+0x20, +0x24/+0x28/+0x2C
                  The +0x24/+0x28/+0x2C column is the "forward" direction used for thrust
+0x4C  ptr    Entity data pointer (contains control structures)
+0x58  int    Entity type index (into g_resource_table)
+0x96  int16  X position (world coordinates)
+0x98  int16  Y position (height)
+0x9A  int16  Z position (world coordinates)
+0x9C  int16  X velocity
+0x9E  int16  Y velocity (vertical)
+0xA0  int16  Z velocity
+0xA2  int16  Heading angle (yaw, 0-0xFFFF = 0-360°, full uint16 range)
+0xA4  int16  Pitch angle (branch-dependent signed angle word; captured type-46 VTOL exceeds +0x4800)
+0xA6  int16  Roll angle (branch-dependent signed angle word)
+0xA8  uint16[4]  Active model slots selected by state bits 0x2000/0x4000
+0xB0  uint16 Normal-callback mass: wrapping type-record base mass + entity +0xB2, zero promoted to 1
+0xB2  uint16 Animation/frame offset; allocator-indeterminate at construction, cleared by normal scheduler waits/callback unwind
+0xB4  uint32 (from resource +0x4c)
+0xB8  uint32 (from resource +0x88)
+0xBC  uint32 (from resource +0x8c)
```

### Entity Direct Structure (not via resource cache)

```
+0xC4  int32     Current control slot index (0-36)
+0xD0  int32     Secondary/comparison slot index
+0xD4  int32     Target/render slot index
+0xD8  uint32[37] Control state bitfield array (37 slots, see §3 Controls)
+0x1EC int32     Signed time-trophy remaining milliseconds
+0x1F0 uint32    Time-trophy/results state (0..5; see above)
+0x1F4 uint32    Campaign-warp request flag
+0x1F8 uint32    Pending campaign-selector/Main-Base flag
```

### Fuel System (CONFIRMED)

Runtime fuel belongs to **vehicle state `+0x88`**; drain, refuel, HUD, save,
and campaign replacement use that owner. Player-controller `+0x1EC` is the
independent time-trophy countdown and must never seed or display fuel.

`FUN_0042F100` reads the time-trophy controller for presentation, not the fuel
system:
```c
*param_1 = *(uint32_t *)(controller + 0x1f0); // time-trophy/results state
*param_2 = *(int *)(controller + 0x1ec) / 1000; // signed whole seconds
```

`FUN_0042E570` initializes that timer from the **level descriptor's** Section-13
`+0x5C` deadline. It is not an entity-type fuel capacity. User verification on
retail (2026-07-16) independently confirms that a new player begins with
vehicle-state fuel zero and must collect type-`0x33` fuel before VTOL can be
selected. Entity `+0xA4` remains the pitch angle word, not either resource.

**Drain formula** (`FUN_00445a40`, game_logic.c line 35139):
```c
drain = abs(thrust * 16) * (delta_time * 2048) >> 31
// = abs(thrust) * delta_time / 1048576
if (fuel < drain) { fuel = 0; } else { fuel -= drain; }
```
Where thrust = `(base_speed * 19 / 65536) + terrain_slope_factor`:
```c
terrain_slope_factor = ((1000 - clamp(height, 0, 1000)) * 24) / 1000 - (weight * 5) / 1000
```
Near ground: more fuel consumed. High altitude: less from terrain factor but weight still drains.

**Refuel** (pickup type 0x33, game_logic.c line 35294):
```c
if (fuel < 190001) {
    fuel += pickup_value;          // from OVL entity data
    fuel = min(fuel, 200000);      // max fuel cap
} else {
    show_message("Your fuel tanks are full");
    reject_pickup();
}
```

Section-10 object kinds 2, 4, and 7 use packed controller operations `0x37`,
`0x33`, and `0x35` respectively. `FUN_00427E20` invokes them only while the
object's terrain-type state bits 3-4 are clear, and the callback runs before
the contact response/damage packet:

- Repair (`0x37`) accepts only when health `< 0x9471` (38001), adds the authored
  amount 20000 capped at 40000, plays positional sound 5, and queues resource
  event `0x0D` -> string `0xEE`. Otherwise it retains the pickup and emits
  direct text `0xDD` (`Your ship has no damage`).
- Fuel (`0x33`) uses the threshold/cap shown above, plays positional sound 5,
  emits parameterized `0xCD`/`0x114`, and queues resource event `0x0C`.
- Shield (`0x35`) adds the authored amount 100 to entity `+0x50` capped at
  100000, emits parameterized `0xCD`/`0x110` (`Shields`), and has no pickup
  sound or resource event.

An accepted pickup clears the live Section-10 attribute byte; rejected fuel or
repair leaves the same world object available for a later contact.

**Low fuel warning** (game_logic.c line 35120):
- Threshold: fuel < 10000 raw (= 10 display units)
- Throttled: one warning every 70+ game ticks
- Shows the `0xE0` "Fuel low" message and submits the four-word
  `[0x10000, 0x10000, 0, 1]` parameters with global sound `0x31`. This uses
  the managed non-positional voice path (`FUN_004958C0`), while exhaustion
  below uses the positional wrapper around the same Section-11 sample.

**Fuel exhaustion** (game_logic.c line 34988):
- When a VTOL callback begins with fuel < 1, its powered branch is skipped
- Sets out-of-fuel flag at vehicle_state `+0x20D`
- Plays positional Section-11 sound `0x31` at the player's live position and
  submits session-deduplicated gameplay-notification event `0x0B`;
  `DAT_004CAD80[0x0B]` selects global Section-2 string `0xEC`,
  `You cannot fly without any fuel`
- Common post-mode physics still runs; the pending flag then selects Hover
- An empty-fuel TAB therefore briefly selects VTOL and reaches this same
  callback before returning to Hover; there is no separate input-side fuel
  gate or separate refusal notification

**Key values:**

| Parameter | Raw Value | Display Value |
|-----------|-----------|---------------|
| Max fuel | 200,000 | 200 |
| Low warning | 10,000 | 10 |
| Pickup reject | 190,001 | 190 |
| Exhaustion | < 1 | 0 |

**Vehicle state struct layout** (accessed via `g_rng_state + 0x27c`):

| Offset | Type | Field |
|--------|------|-------|
| +0x68 | uint32 | Entity ID |
| +0x88 | int32 | **Fuel level (runtime)** |
| +0x194 | char | Pickup counter |
| +0x197 | byte | Capability flags (bit 0=Targetter, bit 1=Turbo) |
| +0x20C | byte | Self-righting mode value |
| +0x20D | byte | Out-of-fuel flag |
| +0x20E | char | Invulnerability timer / beam timer |
| +0x220 | ptr | Linked list of attached entities |

### Shield / Damage System (CONFIRMED)

The player shield is entity `+0x50`, spent before hull health and retained in
native saves and campaign arrivals; its red `shipaura` is model245. See
[Player shield](PLAYER_SHIELD.md). The component fields and `56BD0` value below
belong to building Sub-M status/notification, not that player buffer.

**Health structure** accessed via `entity_resource+0x4C → +0x0C`:

| Offset | Type | Field |
|--------|------|-------|
| +0x04 | int32 | Max health |
| +0x08 | int32 | Charge threshold (weapon state) |
| +0x0C | int32 | Cooldown limit |
| +0x14 | int32 | Regeneration rate |
| +0x58 | int32 | Weapon count |
| +0x68 | int32 | Current health |
| +0x6C | int32 | Regeneration counter |
| +0x80 | int32 | Damage rate multiplier |
| +0x84 | int32 | Regen-from-damage factor |
| +0x88 | int32 | Target entity handle |
| +0x94 | int32 | Progressive damage accumulator |
| +0xB0 | int32 | Weapon state machine (0-4) |

**Damage application** (`FUN_00418c20`, game_logic.c line 10338):
```c
health = health + param_2;  // param_2 is signed: negative = damage
if (health < max_health) {
    // If has regeneration and threshold not reached: heal counter
} else {
    health = max_health;  // clamp
    FUN_00456bd0(0);      // update shield display to 0
    FUN_00456900(0xce, 0); // "entity destroyed" notification
}
```

Known damage values:
- `-1`: timed damage (per tick)
- `0`: reset/check
- `0x7FFFFFFF`: instant kill

**Shield display** (`FUN_00456bd0`, game_logic.c line 43632): Updates `g_rng_state+0x2C8`. Only overwrites if new value is lower (shield draining visual effect) or 1,000,000+ ticks since last update.

**Progressive damage** (`FUN_00419b50`, game_logic.c line 11050): Accumulates via `+0x94`. Every 100,000 units = 1 damage level. At level 32, entity is destroyed.

### Vehicle Type Mapping (CONFIRMED)

`FUN_0042eb70` (game_logic.c line 23164):

| Vehicle Type | Entity Type | Description |
|-------------|-------------|-------------|
| 1 | 8 | Standard hovercraft |
| 2 | 91 (0x5B) | Vehicle variant 2 |
| 3 | 90 (0x5A) | Vehicle variant 3 |
| 4 | 79 (0x4F) | Vehicle variant 4 |
| 5 | 116 (0x74) | Vehicle variant 5 |
| 6 | 7 | Vehicle variant 6 |

### Player HUD Display (CONFIRMED from strings + code)

- Lives
- Rank (14 tiers, computed from active control slots + base score)
- Trophies collected
- Megablasts available
- Shield Strength (at `g_rng_state+0x2C8`)
- Fuel level (vehicle-state runtime `+0x88`)
- Damage accumulation
- Worlds Visited / Worlds Rescued
- Cargo status (people/items vs capacity)
- Weapon selection and finite ammunition from the controller's 24-byte
  inventory descriptors. The six runtime pointers at `+0x98..+0xAC` belong to
  the separate building status component, not the player weapon inventory.
- Targetter (toggleable aiming reticle)
- Active Camera (toggleable)

---

## 2. Movement Physics

### Movement Pipeline (CONFIRMED)

The component-level movement phases are:

```
heading (+0xA2) ──→ FUN_00413f70 ──→ 3x3 rotation matrix (+0x0C..+0x2C)
pitch (+0xA4)       (Euler→Matrix)     via g_state_table sine lookups
roll (+0xA6)

                                         ↓

FUN_004236d0 ──→ thrust along matrix forward column (+0x24/+0x28/+0x2C)
(Generic component) velocity += forward × thrust_scaled
                    velocity.y += lift_force
                    pitch/roll damped toward zero
                    velocity *= drag

                                         ↓

FUN_00412da0 ──→ position += (dt_scaled × velocity) >> 15
(Entity Tick)       flags |= 0x20 (position changed)

Additional forces applied by:
  FUN_0040e100 — gravity: vel.y -= 0x300000 × dt >> 31
  FUN_0040df70 — ground snap: pos.y = terrain_height, vel.y = 0
  FUN_0044ec60 — wind forces: velocity nudged toward global wind vector
  FUN_00411760 — collision bounce: velocity reflection + restitution
  FUN_0041eb50 — aerodynamic drag: per-axis damping along local frame
```

This is a dispatch inventory, not one unconditional player chain. Captured
normal type-46 VTOL selects its Sub-D → `FUN_0041A690` attitude path and does
not additionally dispatch `FUN_004236D0`; applying both damping handlers would
contradict every clean mode-10 transition.

### Euler-to-Matrix Conversion (CONFIRMED — `FUN_00413f70`)

game_logic.c line 7678. Reads heading/pitch/roll from entity, looks up sin/cos from
`g_state_table` using quadrant folding:
- `& 0x3FFC` — index into quarter-period table (byte offset, so /2 for entry index)
- Test `& 0x4000` — if set, use complement entry (cosine from same table)
- Test `& 0x8000` — if set, negate result

Writes 9 rotation matrix elements to entity +0x0C through +0x2C.

The port now reproduces this as integer arithmetic rather than composing an
`f32` renderer matrix and quantizing it afterwards. Each Q15 product narrows to
a signed word at the same point as retail, then is stored in the high word of
the Q31 body-basis component. The two 2026-07-16 keyed skimmer/control-mode
traces validate all nine output words for all 5,000 captured entity samples
with zero mismatches.

### Player Physics Master (CONFIRMED — `FUN_004236d0`)

game_logic.c line 17391. The main per-frame vehicle physics update:

**Steering dispatch** based on mode byte:
- Mode 0: `FUN_00423890` — Ground/hover steering (uses g_state_table directly with heading)
- Mode 1: `FUN_00423ba0` — Air steering (simple)
- Mode 2: `FUN_00423c80` — Air steering (advanced)

**Thrust** (param_4[0xb] = thrust magnitude):
```c
thrust_scaled = (thrust * (dt >> 5)) >> 8;
vel_x += (matrix_24 * thrust_scaled) >> 32;  // forward X
vel_y += (matrix_28 * thrust_scaled) >> 32;  // forward Y
vel_z += (matrix_2c * thrust_scaled) >> 32;  // forward Z
```

**Vertical lift** (param_4[0xc] = hover/lift force):
```c
vel_y += (dt >> 5) * lift_force >> 8;
```

**Pitch damping:**
```c
pitch -= pitch * (dt >> 10) >> 9;   // exponential decay
```

**Roll damping:**
```c
roll -= roll * (dt >> 10) >> 8;     // faster decay than pitch
```

**Velocity drag** (all axes):
```c
drag_factor = dt << 11;
vel_x -= (vel_x * drag_factor) >> 32;
vel_y -= (vel_y * drag_factor) >> 32;
vel_z -= (vel_z * drag_factor) >> 32;
```

### Velocity Integration (CONFIRMED — `FUN_00412da0`)

game_logic.c line 7023:
```c
dt_scaled = dt >> 5;
pos_x += (short)((dt_scaled * vel_x) >> 15);
pos_z += (short)((dt_scaled * vel_z) >> 15);
pos_y += (short)((dt_scaled * vel_y) >> 15);
flags |= 0x20;  // position changed
```

### Velocity Drag in Entity Tick (CONFIRMED — `FUN_00412da0`)

game_logic.c line 6975. When flags `& 0x880000 == 0x880000`:
```c
decay_rate = dt >> 8;
// Each velocity component reduced by decay_rate per frame, clamped to zero
// Then clear bit 0x800000
```

### Gravity (CONFIRMED — `FUN_0040e100`)

game_logic.c line 4039:
```c
vel_y -= (dt * 0x300000) >> 31;   // constant = 3145728
```

This subtracts 11 raw velocity units at an 8,000-us update and 29 at a
20,000-us update.

### Ground Snap (CONFIRMED — `FUN_0040df70`)

game_logic.c line 3907:
- Zeros Y velocity: `vel_y = 0`
- Bilinear interpolation of heightmap at entity XZ position
- Each heightmap byte scaled by 0x20 (32)
- Adds bone/attachment offset if applicable
- Sets grounded flag `0x800000`

### Ground Bounce (CONFIRMED — `FUN_0040e100`)

game_logic.c line 4058. When entity is below ground by >100 units:
- **Spring force:** `vel_y -= (dt << 13) * (height_error * 4) >> 32`
- **If rising fast (vel_y > 0x600):** `vel_y += (dt << 13) * 300 >> 32` (upward push)
- **Friction on all axes:** `vel -= (dt << 10) * vel >> 32`

### Collision Bounce (CONFIRMED — `FUN_00411760`)

game_logic.c line 5939:
```c
dot = (surface_normal DOT velocity) >> 12;
if (dot < 0) {  // moving INTO surface
    // Remove normal component
    vel -= surface_normal * dot >> 12;
    // Add bounce: bounce = dot * restitution * 16 >> 15
    vel += surface_normal * bounce >> 12;
    // Position correction
    pos += surface_normal * penetration >> 12;
}
```

### Admitted Static Damage Programs (CONFIRMED)
Moved verbatim to [ENTITY_STATIC_DAMAGE_PROGRAMS.md](ENTITY_STATIC_DAMAGE_PROGRAMS.md) during the cohesion split; it is the durable owner of every admitted kind program.

### Wind Forces (CONFIRMED — `FUN_0044ec60`)

game_logic.c line 40847. Global wind vector at `DAT_004f71a8/aa/ac` (3 × int16):
```c
target_vel = wind * (height_above_ground * 0x20) >> 15;
delta_vel = target_vel - current_vel;
// Apply roll torque from cross product
roll += (dt * cross(delta_vel, forward)) >> 32;
// Apply pitch torque from dot product
pitch += (dt * dot(delta_vel, right)) >> 32;
// Apply velocity correction
force = (dt * wind_strength) / (entity_weight << 3);
vel += force * delta_vel >> 15;
```

When no wind: simple velocity damping toward zero.

### Pitch/Roll Update (CONFIRMED — `FUN_0041a690`)

game_logic.c line 11545. The pitch (+0xA4) update has multiple modes controlled by
`*(char*)(control_struct + 0x41)`:

**Mode 0:** No pitch correction. The roll tail below still executes.

**Mode 1 — Proportional decay:**
```c
pitch -= (dt_scaled * pitch) >> 9;  // exponential decay toward zero
```

**Modes 2-15 — Asymmetric target seeking:**
```c
target = ((mode - 1) * 0x2800) / 15;  // mode 2 → 682, mode 15 → 9557
error = target - pitch;
if (error < 0) { // overshooting
    pitch += (dt * error) / ((mode * -256 + 256) / 15 + 512);
} else {          // approaching
    pitch += (error * dt) / ((mode * 512 - 512) / 15 + 512);
    if (error > 0x4000) { // fast catch-up for large errors
        pitch += extra_correction;
    }
}
```

**Alternate `+0x3F/+0x40` flight-state drain** (game_logic.c line 11685; not
the captured normal player VTOL branch):
```c
drain = ((dt << 13) * (pitch + 0x3000)) / 2^32;
pitch -= drain;
// Drain accelerates as pitch exceeds -0x3000
```

**Former clamp interpretation (corrected 2026-07-16):** the ±0x1800 and
±0x3000 constants in this cluster are not global body-pitch bounds. Captured
type-46 VTOL play sustains pitch above +0x4800. Their exact branch-local roles
must not be generalized to VTOL. The matched 2026-07-30 submerged mode-launch
pair names the ±0x1800 role precisely: after VTOL switches to Hover and its
retained extreme basis drives one C/B/A force pass, Hover's effective
environment bit `0x10000` makes shared `FUN_0040E640` clamp pitch and roll to
that range. Do not clamp the player VTOL angle or clamp inside the TAB style
swap.

**External pitch adjustment** from control input at `control_struct + 0x1E` is
added before the selected 0..15 pitch recurrence:
```c
pitch += *(short*)(control_struct + 0x1e);
```

### Self-Righting System (CONFIRMED)

The 0-15 **Self Righting** menu option controls a byte at flight control data
`+0x41`, mirrored at player/vehicle state `+0x20C`. The controller constructors
initialize this byte to **0x0F (15)**, but normal session setup then copies the
persisted frontend setting directly over it. The executable's initialized
frontend setting is **1**; a retail capture may therefore show another value
(both July 16 VTOL acceleration traces show 10) when the player has changed
the option.

**Setting propagation:**
```
Menu toggle → FUN_00448450(player_state, value)
  → stores at player_state+0x20C
  → resolves entity resource chain: ResourceCache → +0x4C → +0x0C → +0x18
  → calls FUN_0041b9c0(flight_control, value) → sets +0x41
```

**Behavior by mode value** (in `FUN_0041a690`, the pitch/roll update handler):

| Mode | Behavior |
|------|----------|
| 0 | Self-righting OFF — no pitch correction in this handler |
| 1 | Gentle — pitch proportionally damps toward zero: `pitch -= (dt * pitch) >> 9` |
| 2-15 | Aggressive — pitch drives toward target angle `((mode-1) * 0x2800) / 15`; higher mode = steeper target + faster correction |

The 2026-08-09 retail Self-Righting-0 replay reproduced the port's sustained
Up+Space inversion and terrain crash. Together with the matched retail/demo C,
this closes that report as authentic mode-zero behavior rather than a missing
angle clamp or collision safeguard.

**Roll always auto-corrects** in this self-righting handler regardless of its
mode byte: `roll -= ((dt>>10)*roll)>>8`. The separate generic component callback
below is not dispatched for the captured normal type-46 VTOL path; applying
both would contradict its exact mode-10 pitch fixed point.

Normal type-46 Sub-D steering first computes
`step=((turn*52/28)*(global_dt>>2))>>15`, then subtracts it from both heading
and roll. The near-surface force phase retains that post-Sub-D/pre-A690 roll;
A690 applies the damping above afterward. The trimmed July 17 trace replays all
53 clean sustained steering transitions exactly.

**Generic component damping** (when `FUN_004236d0` is selected, line 17431):
- Pitch: `pitch -= pitch * (dt >> 10) >> 9` (1/512 per tick)
- Roll: `roll -= roll * (dt >> 10) >> 8` (1/256 per tick — **2x stronger** than pitch)
- Dead entities: roll increases (gravity tumble), pitch still damps

### Ground/Hover Steering (CONFIRMED — `FUN_00423890`)

game_logic.c line 17461. Uses sine table with heading:
```c
sin_val = g_state_table[wheel_angle & 0x3FFC >> 2];
steering_output = sin_val >> 14 / steering_divisor;
// Phase offsets at 0x3000, 0x5000, 0x7000 for multi-axle steering
// Angle accumulator update:
wheel_angle += (dt >> 8) * clamp(speed, 10, 20);
// Pitch output clamped to ±0x600 (±1536)
```

### Displacement Table (CONFIRMED — `DAT_004cd4b8`)

**Purpose:** Pre-computed pseudo-random 3D scatter vectors for **particle effects** (explosions,
debris, smoke). NOT used for player movement.

- **Address:** `0x004CD4B8` in V2000.EXE
- **Size:** 100 entries × 6 bytes = 600 bytes
- **Entry format:** `[int16 X, int16 Y, int16 Z]` (packed as int32 + int16)
- **Frame counter:** `DAT_004de840` incremented per particle emission, indexed via `counter % 100`
- **Value range:** approximately -1800 to +1950 per component

Used by particle emitters:
- `FUN_004407d0` — L1-normalized particle scatter (explosions)
- `FUN_00440dc0` — Direct particle scatter (debris, Y forced upward)
- `FUN_00440e80` — Fragment emitter
- `FUN_00441a50` — Additional emitter

### DAT_004f72cc — Particle Load-Shedding Scale (CONFIRMED)

- **Address:** `0x004F72CC`; unsigned Q16, capped at `0x10000`.
- **Purpose:** reduces selected particle allocation counts when recent host
  frames are slow. It is load shedding, not frame-rate-independent motion.
- **Initialization:** `FUN_0044FA30` fills the eight duration samples with
  125,000 microseconds, sets their sum/average to 1,000,000/125,000, and
  starts the scale at zero.
- **Writer:** once per normal active-world-chain frame `FUN_0044FFA0` increments
  its frame counter, replaces sample `counter & 7` when the *uncapped* duration is at
  most 125,000 microseconds, and computes `average = sum >> 3`. Averages below
  59,464 select `0x10000`; otherwise the scale is `125000 - average`.
  A duration above 125,000 instead restores all eight 125,000 samples and a
  zero scale. Thus exactly 125,000 and 125,001 take different paths.
- **Scope/lifetime:** the application service constructs this state once and
  level-effect teardown does not reset it. Intro2 and ordinary gameplay run
  the writer. Frontend menus, AVI, loading, pause/fullscreen-map, and both
  post-Intro Klaus handoff legs do not; the handoff advances the 50-Hz clock
  through a different chain while preserving the last particle scale.
- **Consumer policies are intentionally not uniform.**
  `FUN_004407D0` and `FUN_00440DC0` multiply a base count by the scale,
  arithmetic-shift by 16, and promote an exact zero to one.
  `FUN_00440E80` uses the same rule on a signed strength and then uses the sign
  to mirror its Y term. In contrast, the E4F0 selector-six and E8A0 response
  loops use `DAT_004F72CC >> 13` directly and can emit zero children.
- **Port status:** the Intro2/gameplay writer and the recovered meteor/wreck,
  common explosion, ordinary surface, whole-body water-entry, submerged
  downwash, E4F0 water, and E8A0 terrain/water consumers are implemented.
  The separate `DAT_004CB500 = 5000000 - DAT_004F72CC * 64` trail-cadence
  consumer and `FUN_00413500` adaptive presentation-tier output remain
  distinct open boundaries rather than particle-count policies.

### Sine Table (CONFIRMED — `g_state_table` / `DAT_004d14d0`)

- **Address:** `0x004D14D0` in V2000.EXE
- **Format:** Quarter-period lookup table, 4096 × int16
- **Scale:** 32767 = 1.0 (full int16 range)
- **Period:** 16384 angle units = 360°
- **Access pattern:** Byte offset masked with `0x3FFC`, quadrant bits for sign/complement
- **Precision:** Max error 1.0 LSB (at index 3933)
- **Monotonic:** Yes (strictly non-decreasing within quarter)

Key values: `sin(0°) = 0`, `sin(45°) = 23170`, `sin(90°) = 32767`

The exact table bytes are now shared by the port's fixed-point body-basis and
wave paths. The stored quarter wave is equivalently
`floor(32768 * sin(2π*i/16384))` for `i=0..4095`; using 32767 or rounding is
off by one for thousands of entries.

### Hover Wave Sample (CONFIRMED — `FUN_0041F470`)

The hover controller samples the signed 8.8 X/Z words after the optional
200-unit body-up offset, bilinearly interpolates signed terrain bytes in integer
arithmetic, and only then evaluates the displaced surface. Wave time is the
integer 50 Hz `g_default_param`, not a continuous fractional phase. The three
sine phases are:

```text
(tick * 0x40 + x * 0x0C) * 2
(x + z) * 0x20 + tick * 0x140
(tick * 0x40 + (z - x) * 6) * 8
```

Retail narrows `(wave3 >> 6)` and `((wave2 + wave1) >> 5)` independently to
signed words, adds them with word wrapping, scales by
`(sea - terrain + 0x200) >> 15`, adds the sea height, and clamps the result to
terrain. The port now preserves those narrowing and tick-quantization points
and receives the same integer `DAT_004FED60` snapshot used by water rendering,
model callbacks, and world effects—without reconstructing it from elapsed
floating-point seconds.

### Physics Constants (CONFIRMED from code)

| Constant | Value | Context |
|----------|-------|---------|
| 0x300000 | 3145728 | Gravity force (in `FUN_0040e100`) |
| 0x1800 | 6144 | Ground pitch/roll clamp |
| 0x3000 | 12288 | Flight pitch/roll clamp |
| 0x600 | 1536 | Ground steering pitch output clamp |
| 0x2800 | 10240 | Pitch target scaling base (modes 2-15) |
| 0x4000 | 16384 | Sine table period; also pitch fast-catchup threshold |
| 0x3FFC | 16380 | Sine table index mask (byte offset) |
| 0x20 | 32 | Heightmap byte scaling factor |
| 0x100 | 256 | Max terrain grid dimension |

---

## 3. Controls / Input System

DirectInput binding rows feed 37 control slots. Movement is continuously
polled, while discrete actions use press/release edges. Self Righting is a
frontend setting copied into flight control; setting 0 authentically permits
the uncontrolled crash behavior. Plain C/D schedule signed +80/-80 ms beam
commands with strict crossing. The authoritative keymap and input pipeline live
in [CONTROLS.md](CONTROLS.md); craft integration, cargo, beam timing, and the
Type-93 materialiser live in [PLAYER_CRAFT.md](PLAYER_CRAFT.md).

---

## 4. Weapons System

### Weapon Types (CONFIRMED — 28+ from level text)

**Energy weapons:**
- Plasma Gun (Red, Green, Blue variants)
- Chain Gun (Infinite Ammo)
- Possession Ray
- Water Cannon
- Flame Thrower / Large Flame Thrower
- Aquatic Plasma Vortex

**Projectile weapons:**
- Guided Missiles
- Rockets
- Grenades
- Depth Charges
- Flares
- Proximity Mines
- Napalm Bombs
- Smart Bombs

**Biological weapons:**
- Virus
- Antidote
- Seedpods
- Virus Bomb
- Antidote Bomb

**Special:**
- Megablast (power attack)
- Targetter (aiming assist)
- Shields (protective)

### Player Weapon Inventory (`FUN_00445A90`)

The player controller owns 32 fixed 24-byte weapon descriptors at `+0x2A8`.
For a selector below `0x32`, retail takes the first slot that is empty or
already contains that selector and independently finds its canonical master
descriptor. An empty slot receives the complete master record. The stored
resource count becomes `max(previous,incoming)` under the routine's signed
comparisons, and the slot is auto-selected only when the incoming amount is
strictly greater than the previous amount. A valid duplicate still returns
accepted and is consumed even when the amount does not rise. Manual next/
previous `FUN_004440D0` walks occupied slots with wraparound and skips finite
zero-ammunition descriptors; flag-bit-0 infinite weapons remain usable even
with a stored zero count. Exhaustion preserves the descriptor for reacquisition
and save/restore. The HUD snapshot retains its selector/count pair, while the
automatic switch removes it from the selected presentation after the carousel
transition.

Automatic exhaustion is a separate, now-recovered path. After a successful
shot leaves the selected finite count at zero, `FUN_00444FA0` calls
`FUN_00445290`. The latter walks the eight selector edges stored at executable
VA `0x004CDE48`:

`0x12→0x01, 0x0E→0x12, 0x0D→0x0E, 0x0C→0x0D, 0x0A→0x16,
0x13→0x0A, 0x1D→0x07, 0x1C→0x08`.

A selector absent from that table first falls back to `0x0C`. Each candidate is
looked up in the 32 owned slots; missing descriptors and finite descriptors
with zero ammunition are skipped, while a nonzero finite count or the infinite
flag accepts the slot. Thus exhausted selector 2 normally follows
`0x02→0x0C→0x0D→0x0E→0x12→0x01` back to the default machine gun. This
automatic selector graph is distinct from manual next/previous traversal;
both reject finite-zero candidates. A restored finite-zero selection rejects
firing without invoking the post-shot successor walk. Native relocation of a
new selection to the occupied tail requires positive signed trigger-duration
`motion+0xC` and absent pending replacement; frame elapsed is a separate input.
That relocation, the post-callback infinite-count writeback, and the
infected selector-`0x1A` regeneration branch remain separate port boundaries
in [the player firing contract](PLAYER_CRAFT.md#fire-both-modes).
`FUN_0044EA60` classifies the selected descriptor dword
copied to weapon-component `+0x10`: selectors `5/0xC/0xD/0xE/0xF/0x11/0x15/0x1F`
are straight-ray and every other selector is ballistic. Targetter uses that
family plus `DAT_004D02C0` speed. Firing emits recovered `FUN_0043F590`,
`FUN_0043F6E0`, and `FUN_0043F780` rows; class-5 static `FUN_0043F920` is
`FUN_00427DE0`. `FUN_00411250` plays type `+0x82` (type 17 sound 92) when
dying bit `0x4000` was clear, then type-46 `FUN_00415040` through empty-link
`FUN_004484A0` (4000 hull). Factory selector `0x12` now fires its class-2
machine-gun projectile every 40 ms with infinite ammunition; stored pickup
amount 500 is an acquisition value. Its larger alternating mounts, damage,
underwater behavior and per-callback audio follow
[the player firing contract](PLAYER_CRAFT.md#fire-both-modes).
Selector7 / `FUN_0043F7C0` now emits the source-proven Antidote class6 family.

The retail pause-menu `Cheats -> Weapons` list proves that the cure-named
entries are genuine player weapon selectors, not unattached strings. Both use
`FUN_00455F50`, which forwards the packed payload through `FUN_00413600` and
the normal player inventory callback:

| Weapon | Packed menu argument | Selector / amount | Canonical descriptor evidence |
|---|---:|---:|---|
| Antidote | `0x0003E707` | `7 / 999` | finite; cadence byte 4 (80 ms), sound 64, HUD sprite 922, text id `0x010B` |
| Antidote Bomb | `0x0003E71D` | `29 / 999` | finite; cadence byte 20 (400 ms), sound 88, HUD sprite 536, text id `0x011B` |

Their paired Virus selectors are 8 and 28 with corresponding red HUD art.
Selector7's class6 descriptor and retained executable now prove its surface
and static terrain-clear binding; the native cure-hit transaction and explicit
unsupported target/style boundaries are recorded in
[the player firing contract](PLAYER_CRAFT.md#fire-both-modes).
Matched retail firing remains open. Selector29's fired impact semantics,
natural pickup/factory availability, radial `FUN_00437630`/`FUN_00456710`
binding, and any relationship to `Cleansing Landscape` remain separate gaps.

The byte-exact new-game selector-1 record stores the authored `888` sentinel,
but flag byte `+0x0C` bit zero makes it infinite rather than 888 finite rounds.
The Level-1 selector-2 record starts with 200 finite rounds. Selector `0x3C` is
not a weapon slot: it owns the independent Targetter capability transaction.

### Building Status Runtime (CORRECTED — `FUN_00419010`)

The `0xB8`-byte state consumed by `FUN_00419010` is the Main Base/Working
Factory status component, not per-weapon state. Its five states drive the
production, delivery, waiting, cooldown, and idle phases documented in §17.
The same component's `+0x94` field owns progressive destruction, and its six
output pointers at `+0x98..+0xAC` publish building counts/progress into model
variables through `FUN_00419630`.

`FUN_00419D90` is reached by the progressive destruction stage renderer; it is
not the general player-projectile constructor. Generic damage is independently
filtered and delivered through `FUN_00414D30`, `FUN_00415040`, and
`FUN_00414E90`. Player ammunition remains in the controller's 32 fixed
24-byte descriptors described above; infinite-ammo policy is descriptor flag
bit zero, and selector 2's Level-1 pickup carries 200 finite rounds.

---

## 5. Cargo / Beam System

### Cargo Hold (CONFIRMED)

**Capacity has two independent values.** Type 46's Section-12 Sub-J payload is
an immutable descriptor with count 5, reserved byte zero, and five zero-policy,
zero-offset slots. The currently unlocked count is the controller byte at
`+0x199`. Fresh-player setup calls `FUN_00443560` with the 99-dword profile
block at `DAT_004D0B30`; the copied source byte for `+0x199` is executable
address `0x004D0C55`, whose initial value is exactly 1.

`FUN_00418620` sets/resizes the live attachment-list capacity to
`min(Sub-J maximum, unlocked count)`, and `FUN_00418410`/`FUN_00418440`
enforce it. `FUN_00445A90`'s type-`0x3A` "Larger Cargo Capacity" pickup raises
the unlocked byte to its authored target, revealing more of the five-slot
maximum. The former 37-slot interpretation confused an unrelated controller
bitfield with cargo and is disproven; `FUN_00418EB0` is not the hold-capacity
gate.

The live list is entity-owned ordered runtime state, not a mutable view of the
five authored slots. Fresh/profile-restored player setup writes its outer
runtime policy `+0x0C = 1`. On a new append, each player's zero slot-policy
clears child state bit `0x800`; the nonzero outer policy then sets child state
bit `0x20000000`. The separate relation helper publishes the child-to-player
owner/backlink after those list and state updates. Ordered runtime identities
are authoritative for attached mass, HUD occupancy, campaign serialization,
and LIFO cargo drop; an inverse backlink scan is not a substitute.

`FUN_004292B0` draws one masked 6x6 global sprite 533 for every unlocked slot.
That HUD width is the raw unlock byte, not the live list's clamped capacity.
For occupied overlays it walks the runtime rows in order and accepts only a
resolved entity with a nonzero state word and dying bit `0x4000` clear. An
accepted slot overlays the attached entity type's primary Section-12 model at
fixed raw depth 4000. Rigid 3D cargo spins through
`FUN_0042A2E0(cargo_spin,0,0)`, with
`cargo_spin += (min(dt,125000)*20000)>>20`. Variant-zero HUD points are base
`(7,165)`, cargo origin `(37,72)`, and horizontal spacing `(10,0)`.

Retail installs model callback `0x42A520` for those occupied slots. Callback
index 0 returns the low 16 bits of the 50-Hz clock and index 1 returns
`(tick / 6) & 3`. Flat `man2` cargo consumes that second channel as its
four-phase walk/mirror cycle. User-verified retail comparison supersedes the
former universal-spin inference: structurally planar actors remain upright and
camera-facing in the cargo HUD, while rigid models retain the common angle and
simply ignore the animation channels.

Strings: "Your cargo bay is full", "There is only limited space in your cargo hold"

### Beam Types (CONFIRMED)

- **Remote Beamer** — beam from distance
- **Tractor Beam** — pull/lift mechanic

### Authored Marker De-duplication (CONFIRMED — `FUN_0042efb0`)

The old interpretation of `FUN_0042efb0` as the cargo beam was incorrect.
This post-load pass removes overlapping generic type-`0x6F` terrain-marker
instances: another `0x6F` within horizontal distance² `0x40000`, or a
type-`0x43` within `0x100000`, wins when the wrapped vertical separation is
below 8000. `FUN_00410b70` then defers destruction of the duplicate marker.
The secret-transition capture confirms that type `0x6F` can be an authored
exit marker and is not intrinsically a pickup, cargo object, or destination.
Cargo collection/drop behavior is owned by the separate player/controller
beam path described above.

### Beam-in Items (CONFIRMED from level text)

- Natives (rescue, ferry to base/factory)
- Scientists (specialized personnel for reactors/factories)
- Weight (puzzle item — "Add weight to hold", affects flight physics → "Flying Brick" mode)
- Mini Radar (equipment)
- Remote Beam-Out (tool)

### Gameplay Loop (CONFIRMED)

1. Beam natives/scientists aboard
2. Ferry to base/factory
3. Trained natives man factories
4. Factories produce power-ups: "Power-up ready at factory"

---

## 6. Level / World System

### Level Structure (CONFIRMED)

- 53 level IDs (0-52)
- System levels: 0-12 (UI, palettes, models, sounds)
- Game worlds: 13-50 (38 playable levels)
- Special: 51-52

### World Types

| Level | Name | Key Mechanic |
|-------|------|-------------|
| 13 | Peasant | Basic rescue, hive destruction |
| 14 | Medaeval | Creature capture prevention, 3-death limit |
| 15 | Castle | Virus containment |
| 16 | Colorado | Water hazards, lobster enemies |
| 17 | Alpine | Ice melt → drowning hazard |
| 18 | Amazon | Fire-based combat |
| 19 | Sunny Ice | Research base, virus confusion |
| 20 | Dark Jungle | Secret paths, no mandatory save |
| 21 | Flooding | Time-pressure evacuation |
| 22 | Water | Aquatic environment |
| 23 | Reef | Sharks, underwater combat |
| 30 | Cistern | Hidden access, friendly fish |
| 31 | Canyon | Boulder puzzles, 8/10 survival |
| 35 | Marble | Rock-rolling puzzles |
| 36 | Atoll | Deep water threat, weight puzzle |
| 37 | Plateau | Nuclear reactor protection |
| 38 | Reactor | Submarine meltdown timer |
| 39 | Dragon | Two hives, lava, dragons |
| 40 | High Plateau | 3 reactors, flooding, 2-death limit |
| 42,46,47 | Alien 1-3 | Final worlds, Alien Queen boss |
| 43 | Volcano | Lava, zero-casualty scientists |

### Win Conditions (CONFIRMED per level text)

- Kill all creatures → hive becomes vulnerable → destroy hive
- Rescue N natives (varies per level)
- Casualty limits (e.g., "Lose another person and this world will be lost")
- Contain virus spread below threshold
- Prevent reactor meltdown (timed)
- "The Hive is now vulnerable" (gating message)

### Native Casualty Limit (`0042DD10` selector 0)

The survivor census, signed authored thresholds, one-shot warning, death-owner
evidence and shared world-loss callback chain are owned by
[Campaign Failure and Casualty Limits](CAMPAIGN_FAILURE.md).

### Environmental Hazards

- **Virus spread:** Percentage-based, "If Virus covers too much of the landscape, the world will be lost"
- **Flooding/Drowning:** Water level rises, natives drown
- **Ice melt:** Temperature causes ice to flood
- **Lava:** Lethal terrain
- **Wind/Currents:** Global wind vector at `DAT_004f71a8/aa/ac`, applied via `FUN_0044ec60`

---

## 7. Rank & Progression

### Rank Tiers (CONFIRMED — 14 levels)

| Threshold | Rank |
|-----------|------|
| 0 | Absolute Beginner |
| 3 | Beginner |
| 5 | Trainee |
| 10 | Good Trainee |
| 15 | Cadet |
| 20 | Just Qualified |
| 25 | Reliable |
| 30 | Dependable |
| 40 | Competent |
| 50 | Expert |
| 60 | Hero |
| 70 | Ace |
| 80 | Genius |
| 90 | Elite |

### Ranking System (CONFIRMED — `FUN_00456ef0`)

Score = count of active control slots (bit 0 set, via `FUN_0042ece0`) + base value from `player_state+0x126`. Looked up in string table at `DAT_004fe628 + offsets 0x478..0x4AD`.

### Per-World Statistics (CONFIRMED)

Retail keeps these counters for in-world completion statistics. Hive death
calls `FUN_004567B0` to stamp session `+0x2BC` and commit campaign completion,
without changing the active descriptor. `FUN_00452CB0` draws the Section-2
rows below only when that stamp is greater than 500, using age
`(DAT_004FED60 - +0x2BC) * 1000 / 50`. The HUD/radar and two-slot hive hints
remain visible. Authored prefixes carry stagger delay, duration, and Y;
field 6 supplies `FUN_00452790`'s type-on interval and shared cue.

| Global id | Prefix Y | Line |
|---:|---:|---|
| `0xBD` | 10 | `World saved in %d:%02d.` |
| `0xBE` | 17 | `Time trophy collected.` (only if that bit was claimed) |
| `0xBF` | 30 | `You rescued %d natives.` |
| `0xC0` | 37 | `%d natives were killed.` |
| `0xC1` | 44 | `You killed %d alien creatures.` |
| `0xC2` | 51 | `The landscape was %d%% virused.` |
| `0xC3` | 58 | `Your rank is %s.` |
| `0xC4` | 65 | `You found the hidden trophy.` (only if claimed) |

The `0xBF`/`0xC0`/`0xC1` values recense through `FUN_0042DD10`'s
`+0xD0==0` prologue against world-load `+0x170`/`+0x16C` snapshots.
`FUN_0042E210` counts capability masks `0x400`/`0x800`/`0x08` and adds
Sub-M `+0x68` (factory native count) into `+0x1A8` before the capability
loop, without the dying gate. The virus row uses `FUN_004366F0` and the
live Section-10 infection count. `0xBD` formats `+0x2BC / 3000` minutes
and `(+0x2BC / 50) % 60` seconds. `0xBE` uses formatter case `0xE`:
`FUN_0042EE10` must be set, and the line is empty while
`DAT_004FED60 / 40` is even. Field 7 selects the formatter before type-on.
Rank uses the recovered saved-world thresholds; the EXE has no writer of
`player_state+0x126`, so its new-game base value remains zero.

The progress map is a later owner, entered after the dead-hive exit. Its
`004D0AA0` descriptor replaces HUD/radar, and `FUN_00454390` draws the
map plus global string 25 at Section-1 Y=95. `DAT_004D1204` places sprites
3733..3762; `FUN_00454C70` adds sprite 3765 at each visible `00454FF0`
world node. `00454460` blinks the destination slot on odd `+0x270 / 100`;
flag 8/0x20 adds sprite 532 from overlay-3 S1 31/32, and clear flag 4
stretches sprite 583 over the marker rectangle. `00454520` connects nodes
with 3763/3764 using overlay-3 Section-14 record 3 and `0042EDC0` bits
9..15, written by `0042DD10` selector 0 on secret-pad contact.

Map Space comes from input table `004C2580` through `004D0620` to
`FUN_00455CC0` (sound 3, `+0x294=1`), then `FUN_004555A0` leaves the
map and proceeds to the selected world. The `004D1090`/`00456170`/
`+0x297` callback is part of descriptor `004D0AD8`, not this map.
The port retains the exact decoded arrival through the modal screen; it does
not make Space on the earlier HUD statistics bypass the wreck contact.
See [campaign exit flow](LOADING_TRANSITIONS.md#campaign-exit-routes-runtime-validated-2026-07-22).

---

## 8. Entity System

### Entity Types (CONFIRMED — 79 unique types, range 2-129)

- **Type 6:** Main Base (`college` in the first world; behavior descriptor
  `Main Base`)
- **Type 46 (0x2E):** Persistent player craft (`player4`)
- **Types 52, 62, 66-68:** NPCs (natives, scientists, captured)
- **Types 2-45:** Alien creatures (spiders, dragons, etc.)
- **Type 61:** Most frequent combat alien (113 instances, 34 levels)
- **Type 0x3D (61):** Standard projectile
- **Type 0x43 (67):** Beam/special projectile (custom vtable)
- **Type 0x5D (93):** Powerup projectile
- **Type 0x6F (111):** Generic authored terrain-marker/effect instance; not
  intrinsically a pickup, cargo object, or campaign destination
- **Types 86-87:** Specialized high-HP aliens
- **Type 95:** Elite creatures (3.61x difficulty scaling)

### Entity Lifecycle (CONFIRMED)

**Creation:** `FUN_00438080` (fgdk_engine.c line 187)
- Calls `FUN_0040d720` for core resource instantiation
- Special handling for types 0x33, 0x43, 0x2E (custom vtable patches)

**Destruction (deferred):** `FUN_00410b70` (fgdk_engine.c line 9)
- Sets flag `0x100000` in resource status (+8)
- Increments global destruction count
- Called 15 times across game_logic.c

**Death (immediate):** `FUN_00410c10` (fgdk_engine.c line 27)
- Sets DYING flag `0x4000`
- Triggers death handler via entity vtable
- Plays death sounds
- Checks mission completion conditions

### Difficulty Scaling (CONFIRMED)

Section 4 parameter tuples: `[HP, Speed, Flag, Category]` per entity type.

- Easy (Variant 0): Base values
- Hard (Variants 1-3): Average 1.84x scaling (range 1.25x-3.61x)
- HP range: 181-1063 (active entities)
- Speed range: -127 to +106 (signed)

### Collision System (CONFIRMED)

Section 12: 3D parametric collision volumes.
- Scale values: 1, 10, 50, 100, 200, 400, 500, 600, 1000
- 14 optional sub-sections per collision model
- Ground offset: 200 in 86% of entries

### Entity Tick (CONFIRMED — `FUN_00412da0`)

game_logic.c line 6809 (~230 lines). Per-frame master update:
1. ResourceCache lookup
2. Normal-world common scheduler prefix (`+0x70/+0x6C/+0x68/+0xB6`)
3. Behavior callback dispatch via vtable
4. Smoke/debris particle generation (`FUN_00441670`)
5. Sound position update (`FUN_0044c920`)
6. Velocity-to-position integration (see §2)
7. Velocity drag when drag flags active

The common scheduler prefix wraps `dt` into `+0x70` and uses the low 16 RNG
bits to scale a 250,000-unit retain/reset threshold. It then wraps `dt` into
`+0x6C`: at or below 125,000 a second low-16-bit random threshold may retain
the accumulator and return early; above it, the field retains the remainder
and the callback delta is capped at 125,000. State bit `0x02000000` bypasses
both random waits, while nonzero byte `+0xB6` forces a continuing delta to one.
The accepted delta wraps into recent-relation age `+0x68`. RNG is consumed only
on the reachable retail branches; the early wait also clears `+0xB2`.

---

## 9. Save System

### Native save scope and restoration boundary (CONFIRMED)

Native `Slot00..Slot13` records, complete player/controller restoration,
checkpoint writes, settings registry storage, migration, and remaining
unowned fields are documented in
[SAVE_AND_SETTINGS.md](SAVE_AND_SETTINGS.md). The fifteenth frontend row is
informational and is not a `Slot14` save. Binary record framing remains in
[FORMAT_DOCUMENTATION.md](FORMAT_DOCUMENTATION.md#save-file-format).

---

## 10. Audio System

### Sound Effects (CONFIRMED)

- 49 PCM blobs + 54 parametric aliases
- Format: 22050 Hz mono 16-bit signed PCM
- Duration range: 72ms - 2532ms
- Max simultaneous: 24 DirectSoundBuffers
- Frequency modulation: Randomized pitch variation

#### Positional sound transform (confirmed — `FUN_0044C970`)

`FUN_0044C830` creates a world-sound emitter and stores its raw i16 8.8 XYZ at
emitter offsets `+0x24..+0x28`; `FUN_0044C920` follows an owning entity by
updating those words each entity tick. `FUN_0044C970` transforms the signed
wrapped emitter/listener delta through the camera's Q1.31 basis and builds the
four-word volume/frequency/pan parameter block consumed by `FUN_004957C0`.

The two distance constants are initialized data in the retail executable:

| Address | Raw 8.8 | World units | Use |
|---------|---------|-------------|-----|
| `DAT_004D0118` | `0x2000` | 32 | Maximum audible distance |
| `DAT_004D011C` | `0x1000` | 16 | Camera-right distance that reaches full pan |

For each axis, retail computes `(int16)(emitter - listener)`, so the shortest
delta wraps on a 65536-raw / 256-world-unit period. After an axis-aligned
32-unit rejection, the mix has this shape:

```text
local = camera_basis * wrapped_delta
pan   = clamp(local.x / 0x1000, -1, +1) * 10000
dist  = integer_sqrt(local.x^2 + local.y^2 + local.z^2)

if dist <= 0x0100: gain = emitter_base_gain
if 0x0100 < dist < 0x2000:
    gain = emitter_base_gain * (0x2000 - dist) / 0x2000
otherwise: inaudible
```

Each Q31 transform product narrows before the three-axis sum. Within the pan
interval, `00457680(local.x + 0x1000, 0x2000)` is multiplied by 20000 in Q31
then reduced by 10000; its positive endpoint is 9999, while values above it
use 10000. These integer details affect both the audible gate and pan.
The full-volume one-unit branch creates a small discontinuity: immediately
past one unit the linear expression is just under `31/32`. Pan is passed to
DirectSound in its native `[-10000,+10000]` hundredths-of-a-decibel range;
negative favors the left channel and positive favors the right. Native
`sound::positional_sound_mix` retains the raw listener origin, full Q31 basis,
integer square root and volume/pan narrowing. The older float positional
wrappers remain presentation adapters for callers without native ownership.

`0044F450`/`0044C790` enqueue logical one-shots without alias RNG. The later
`0044C970` pass performs audibility, optional sound-11 warble, then ordered
alias resolution before physical playback. Inaudible records consume neither
warble nor alias words; `0044C940` discards their one-shots. Native resolution
runs even without SDL or with zero master volume. This does not model the
separate `DAT_004F716C` logical-mixer disable gate or native allocation failure.

#### Constructor-attached entity loops (confirmed)

`FUN_004104B0` reads the Section-12 sound id at `+0xB4` while constructing an
entity. Sound 11 takes the warbling `FUN_0044CD90` path; every other nonzero id
uses `FUN_0044C830` with gain and playback rate `0x10000`. The entity retains
the logical record at `+0x8C`. `FUN_00412DA0` calls `FUN_0044C920` after the
entity callback/effect phases but before final master-velocity integration,
then the later logical mixer `FUN_0044C970` creates, retunes, or culls the
physical voice. Generic release/destruction calls `FUN_0044CC90` and clears
only the per-allocation `+0x8C` custody. Retail/demo bodies for this constructor,
update, mixer, warble, and release chain match exactly.

The exhaustive normal-tier Section-12 owners are:

| Type | Primary model | Attached sound | Policy |
|---:|---:|---:|---|
| 15 | 276 | 11 | audible-only warble |
| 44 | 276 | 11 | audible-only warble |
| 61 | 82 | 44 | fixed 1.0x loop |
| 87 | 270 | 11 | audible-only warble |
| 108 | 1167 | 11 | audible-only warble |
| 111 | 16 | 100 | fixed 1.0x loop |

Type-61 spawn model overrides, including model 138, do not change its sound-44
ownership. The port now reconciles every live Type-61 allocation whose `+0x8C`
still contains 44 in retained entity-list order. Its logical row survives the
32-unit physical-voice cull, re-entry restarts the sample, and release,
disappearance, or world replacement stops/forgets it. This fixed loop consumes
no RNG. Sound 11's statically closed nine-dword warble is now retained as a
detached core too. Construction consumes no RNG; only an audible segment
rollover consumes exactly two process-global words, while Q31 interpolation
uses the original divisor-halving quantization, physical culling freezes the
logical state, and overshoot never catches up more than one segment.
`EntityPositionalAudio` admits moving owners only with their pre-integration
`FUN_0044C920` position snapshot and advances this retained warble during the
logical audio pass. Initial admission resolves aliases after warble; existing
loops use current wrapper parameters through `004957C0` without re-resolving
aliases. Optional hardware does not suppress this phase or restart warble.
Type 111's accepted campaign gates continue through their
dedicated sound-100 owner until general Type-111 entity materialization exists.

### CD Audio (CONFIRMED)

- 10 tracks (track02.ogg through track11.ogg)
- MCI device ID at `DAT_004fbf98`
- Initialized flag at `DAT_004fbf94`

### Deduplicated HUD Resource Events (CONFIRMED — `FUN_004568b0`)

Triggered only during gameplay (game_phase == 5). The seen-event mask survives
campaign world changes; frontend session selection resets it. Ordinary loading
clears displayed messages after construction/restoration without clearing that
mask; [load notification policy](CAMPAIGN_CARGO.md#load-time-notification-policy)
owns the source addresses and respawn exception. Known event IDs:

| ID | Event |
|----|-------|
| 0 | Neutral |
| 2 | Damage taken |
| 3 | Hive destroyed (`0xE4` fly-down) |
| 4 | Entity killed |
| 9 | Collision |
| 0xB | Weapon activated |
| 0x10 | Attract Attention initializer hint (`0xF0`; authored meaning unasserted) |
| 0x11 | Cargo hold capacity hint (`0xF1`) |
| 0x15 | Type-68 weight hint (`0xF5`) |
| 0x16 | Beam/vaporise spawned |

### Text Notification Messages (CONFIRMED — `FUN_00456900`)

| ID | Message |
|----|---------|
| 0xC9 | Mission event |
| 0xCD | Parameterized message |
| 0xCE | Entity destroyed |
| 0xD2 | `The Hive is now vulnerable` |
| 0xCF | Regeneration failed |
| 0xD1 | Weapon fired (charge complete) |
| 0xD6 | Damage event |
| 0xD9 | (filtered — never displayed) |
| 0xDC | Level event |
| 0xDD | Level event |
| 0xE0 | Mission event |

---

## 11. Camera and Rendering

Camera transforms, scene submission, terrain visibility, model projection, and
fixed-point shading are authoritative in
[RENDER_PIPELINE.md](RENDER_PIPELINE.md). Player chase-camera behavior and its
remaining acceptance work are maintained in
[WATER_WORLD_TODO.md](../../WATER_WORLD_TODO.md) and
[PORT_FIDELITY_GAPS.md](PORT_FIDELITY_GAPS.md); this mechanics index does not
duplicate their layouts or status history.

---

## 12. Shared Session State

The historical `g_rng_state` label covers a mixed session/controller root, not
a coherent player-state structure. Do not reproduce the former synthetic field
table. Proven consumers own their offsets: `+0x295` is the full-frame sprite
cursor, `+0x296` the phase selector, `+0x2C0/+0x2C4` notification storage,
`+0x2C8/+0x2CC` the shield display, and `+0x308` the entity-database reference.
Vehicle input, fuel, weapons, pose, and campaign state live in their documented
owners rather than this global.

---

## 13. Menu System (CONFIRMED)

The authoritative screen/item layouts, scene tree, transitions, sounds,
edge-triggered input, attract flow, and frontend save/load behavior live in
[MENU_SYSTEM.md](MENU_SYSTEM.md). The exact retail/demo idle-attract table and
low-16 RNG formula have been transferred there. Native save framing and payload
semantics remain in
[FORMAT_DOCUMENTATION.md](FORMAT_DOCUMENTATION.md#save-file-format).

---

## 14. Loading/Transition System (CONFIRMED)
Moved verbatim to [LOADING_TRANSITIONS.md](LOADING_TRANSITIONS.md) during the cohesion split.

## 15. HUD Rendering Pipeline (CONFIRMED)
Moved verbatim to [HUD_RENDERING.md](HUD_RENDERING.md) during the cohesion split.

## 16. Entity Vtable and View-Detail Classification (CONFIRMED)

### Vtable Dispatch Architecture

Every entity type has a record in `g_resource_table` indexed by `entity_type * 4`. The record contains a vtable pointer at `+0x7c`. All behavior is dispatched via indirect calls through this vtable.

**Vtable slots:**

| Offset | Role | Called From |
|--------|------|-------------|
| +0x04 | Init/update | `FUN_00418180` |
| +0x0C | Bare-terrain trampoline into behavior-style `+0x10`, then generic solid response | Common terrain contact |
| +0x10 | Whole-body surface trampoline into behavior-style `+0x14`, then generic response | Surface/water contact |
| +0x14 | Primary-hit impact callback | `FUN_00410eb0`, `FUN_00411180` |
| +0x18 | Infection-hit callback (common DA00, behavior-style `+0x20`) | `FUN_00411250` |
| +0x1C | Cure-hit callback (common DA60, behavior-style `+0x24`) | `FUN_00411320` |
| +0x20 | Detailed presentation callback | `FUN_00411400` — when inner/full-detail bit `0x04000000` is set |
| +0x24 | Detailed update callback | `FUN_00412DA0` — when broader-detail bit `0x02000000` is set |
| +0x28 | Coarse update callback | `FUN_00412DA0` — when broader-detail bit `0x02000000` is clear |
| +0x2C | Type-specific operation dispatch; semantics depend on the concrete consumer | `FUN_00413600` and operation callers |
| +0x30 | Damage receive | `FUN_00414e90` |
| +0x34 | Hit-by-bullet | `FUN_00412cf0` |
| +0x38 | Active entity-pair contact trampoline into behavior-style `+0x18` | `FUN_00411ad0` |
| +0x44 | On-acquire/pickup | `FUN_00416700` |
| +0x48 | On-destroy/deactivate | `FUN_00416750` |

Entity creation (`FUN_00438080`) copies 19 uint32s from the type's vtable into a per-level override buffer (`DAT_004dae90`), then patches `vtable[+0x2C]` to `LAB_004097a0`.

### Entity State Flags (`entity[+8]`)

| Bit | Meaning |
|-----|---------|
| `0x00000800` | Admits `FUN_00411400` view-detail classification |
| `0x00001000` | Pair-ineligible; also suppresses owner transitions that honor this gate |
| `0x00002000` | High active-model selector bit |
| `0x00004000` | Low active-model selector bit |
| `0x00008000` | Pair/checked-damage eligibility |
| `0x00010000` | Terrain/water collision enable |
| `0x00040000` | Master-motion enable |
| `0x02000000` | Broader-detail classification and detailed-update selector (`+0x24` versus `+0x28`) |
| `0x04000000` | Inner/full-detail classification, `+0x20` presentation callback, and primary-hit impulse enable |
| `0x08000000` | Fixed pair body; suppresses ordinary impulse while preserving damage |
| `0x80000000` | Remote-owned entity |

### View-Detail Classifier (`FUN_00411400`)

Retail `FUN_00411400` and demo `FUN_00411390` are accepted exact 786-byte
instruction matches. Retail remains authoritative, while the identical demo
body independently confirms that the unusual comparisons below are shipped
logic rather than a decompiler mistake.

1. Resolve the live entity and its type callback table. State bit `0x00000800`
   gates the classifier; a clear bit skips classification and continues to the
   attachment tail.
2. Clear the previous `0x06000000` detail result.
3. Obtain the current classification extents through `FUN_00433110`. X and Z
   subtract the context's signed words and narrow with 16-bit wrapping. Y
   subtracts the context's full signed dword. The rear plane uses a wrapping
   32-bit multiply before its arithmetic shift:

   ```text
   plane = wrapping_i32((entity_y_i16 - context_y_i32) * (U.z_q31 >> 15)) >> 16
   ```

4. Let `W = scan_columns / 2`, `H = scan_rows`, with distances in signed-8.8
   raw units. The tight test runs first and sets both bits `0x06000000` when
   `dx` is inside `[-W*256,+W*256]`, `dz <= H*256`, and
   `dz >= 0 || dz >= plane`. Otherwise the expanded test sets only
   `0x02000000` when `dx` is inside
   `[-(W+3)*256,+(W+3)*256]`, `dz <= (H+3)*256`, and
   `dz >= 0 || dz+0x300 >= plane`. There is deliberately no negative-Z box
   cutoff: both executables compare the positive Z extent with its own
   negation, a no-op for the normal nonnegative extents. The rear plane is the
   only lower boundary.
5. When inner/full-detail bit `0x04000000` is set, invoke type vtable slot `+0x20`;
   a null slot falls back to `FUN_004138F0`.
6. If presentation returned zero, process the entity's attachment list through
   the `FUN_00413BE0`/fallback attachment path. These are presentation and
   linkage operations, not targeting or sweep attacks.

The shared retail path `FUN_0042AF00` dispatches `FUN_00412DA0` before it later
reaches `FUN_00411400`. The classification written by presentation therefore
selects the next scheduler visit, not the callback already dispatched in that
pass. The port reproduces this phase boundary for authenticated Type-17
Common-Dying receipts. It publishes only the masked `0x06000000` state result;
the type `+0x20` presentation callback and attachment tail remain separate
presentation work.

### Default Detailed Submission (`FUN_004138F0`)

- Selects one of the four active model words at
  `entity[+0xA8/+0xAA/+0xAC/+0xAE]` from state bits `0x2000/0x4000`.
- Resolves the model descriptor and type submission data, builds the entity's
  transform and view-relative position, and copies the live basis into the
  submission record. The authenticated Common-Dying Type-17 scheduler now
  supplies that complete post-task basis to the port's ordinary world draw;
  the separate type `+0x20` callback and attachment tail are not implied.
- Uses the signed scrolling 32x32 terrain-light grid, with the underwater-height
  correction when active, to shift the 16-slot lighting lookup for submission;
  it restores the caller's table pointer afterward.
- Submits through `FUN_00465840`, then eases entity `+0x54` toward shield
  `+0x50` and may submit the [authored aura](PLAYER_SHIELD.md) through
  `FUN_004136C0`. It does not aim at a target or fire a projectile.

### Classification Extents

`FUN_00433110` supplies the current view-detail extents used by
`FUN_00411400`. The tighter bounds select full-detail presentation; an expanded
three-cell X/positive-Z margin and `0x300` rear-plane slack select the broader
detailed-update tier. These are classification extents, not entity-specific
aggro radii. `v2k-game::entity_view_detail` retains the exact three tiers and
state gate; the Hive component's outer detailed/coarse choice now derives from
the same classifier instead of maintaining a second approximation.

### Common Entity Scheduler

- `entity[+0x70]`: wrapping `dt` accumulator with the 250,000-unit randomized
  retain/reset subject-scan gate.
- `entity[+0x6C]`: wrapping callback accumulator with a 125,000-unit randomized
  early return and a capped callback delta/remainder path.
- State bit `0x02000000` skips both randomized waits; nonzero byte `+0xB6`
  forces an accepted callback delta to one.
- An accepted delta wraps into `entity[+0x68]`; `FUN_00411A20` suppresses a
  recently related pair only while the matching relation age is `< 750000`.
- Random samples are drawn only on the branches that need them. The `+0x6C`
  early-return branch also clears `entity[+0xB2]`.

### Intro and First-World Actor Audio Ownership (STATIC + OBSERVED 2026-08-10)

Section-11 filenames name resolved PCM samples, while actors usually author a
logical sound slot that can alias that PCM with its own rate, variance, and
volume. The relevant ownership map is:

| Actor/event | Authored logical sound | Resolved PCM | Result |
|---|---:|---|---|
| Intro type-15/model-276 `wasp` and type-87/model-270 `deathwas` | constructor attachment 11 | `sound_011.wav` | Persistent positional buzz; retail special-cases attachment 11 with pitch modulation. Intro Ptersect and `bluebee2` do not own this loop. |
| Intro type-10/model-351 `grendrag` successful flame emission | emitter 81 | `sound_021.wav` | Confirms the observed dragon flame cue; it belongs to projectile emission, not idle animation. |
| Type-47/model-302 `newant` successful shot | emitter 70 | `sound_013.wav` | The authored firing cue is not PCM 29. |
| Type-47 accepted hit | 92 | `sound_029.wav` | Explains why PCM 29 can coincide with combat; Intro `deathwas` also uses 92 for its emitter. |
| Type-47 death | 75 | `sound_018.wav` | Confirms the observed newant death cue. |
| Type-17/model-256 `spider` death | 94 | `sound_030.wav` | Confirms the penned Level-1 non-shooter; PCM 36 is not this actor's death cue. |
| Any type-61 power-up/pickup variant | constructor attachment 44 | `sound_044.wav` | One persistent loop independent of the inventory selector, including weapons and selector-`0x3F` hidden trophies. |

The port decodes and retains constructor attachment IDs, and its bounded death
paths know when to release them, but it does not yet create/update the general
per-entity positional voices. Consequently the wasp/deathwas sound-11 buzz and
type-61 sound-44 loop remain actor-lifecycle/audio integration work. Emitter
cues likewise become audible only with their native task/emitter owners. The
retail observation that Level-1 newant fire sounded like PCM 29 remains useful
matched-audio evidence, but static ownership identifies PCM 29 as its accepted-
hit cue and PCM 13 as its successful-shot cue; do not rewrite the descriptor
from the observation alone.

### Target Tracking

- Current target handle stored at `entity[+0x80]`
- Collision test (`FUN_00412530`): 3D sphere overlap — `dx² + dy² + dz² ≤ radius_sum²`
- These target/relation fields are independent of `FUN_00411400` and
  `FUN_004138F0`; neither view-detail function is a targeting or firing owner.

### Global type-61/pickup spawner (`FUN_004132D0`)

This is a global spawning policy, not a per-hive creature behavior. When
`DAT_004F73B8` is nonzero, its one-second timer chooses one of 12 authored
zones with `(Random_Next() & 0xFFFF) % 12`, rejects zones within squared range
`0x400000` of a player, and caps type 61 at 32 live allocations. Type 48 uses
a separate cap of 15; later allocations are immediately killed. Actor
waypoints, task behavior, and scheduler fields are documented from authenticated
consumers in [ACTOR_RUNTIME.md](ACTOR_RUNTIME.md), not inferred from this
spawner.

---

## 17. Factory System (CONFIRMED)

The detailed factory economy, staffing, delivery/ejection, output-selection,
Main Base systemic-abort, and reactor-adjacent contracts now live in
[FACTORY_SYSTEM.md](FACTORY_SYSTEM.md). This section remains as the stable
numbered route for older references; current priorities and capture status are
maintained outside the mechanics authority.

---

## 18. Virus Spread System (PARTIALLY RUNTIME-VALIDATED)
Moved verbatim to [VIRUS_SPREAD.md](VIRUS_SPREAD.md) during the cohesion split.

## 19. Building Status Component (CONFIRMED — Section 12 Sub-M)

An earlier interpretation incorrectly called the Main Base/Working Factory
component a per-weapon Sub-G block. The address arithmetic proves otherwise:

```text
type_record + 0xC8                 shifted base used by the decompiler
(type_record + 0xC8) + 0x3C       type_record + 0x104
type_record + 0x104               field 0x41, Section-12 Sub-M
type_record + 0xE8                field 0x3A, Section-12 Sub-G
```

`FUN_00419010`, `FUN_00419750`, and `FUN_00419B50` gate on the relocated
Sub-M pointer at `+0x104`. Main Base type 6 and Working Factory type 66 both
have Sub-M and have no Sub-G payload in any system-overlay display tier.

### Static Sub-M layout

Sub-M is an exact 18-byte descriptor:

| Offset | Type | Meaning |
|--------|------|---------|
| +0x00 | u16 | raw control word; direct consumer semantics remain unproven |
| +0x02 | u8[6] | one-based entity-variable binding selectors; zero leaves the output unbound |
| +0x08 | u8[10] | retained raw tail, not read by the audited static-data consumer |

The invariant system-level records are:

| Entity type | Raw word | Bindings | Tail | Sub-G |
|-------------|----------|----------|------|-------|
| 6 Main Base | 0 | `[0, 0, 1, 0, 0, 0]` | all zero | absent |
| 66 Working Factory | 8 | `[4, 3, 0, 2, 1, 0]` | all zero | absent |

`FUN_00409A80` allocates the zeroed `0xB8`-byte runtime block at component
table `+0x30` through `FUN_00418A60`. It resolves Sub-M bytes `+0x02..+0x07`
through `FUN_0040A950` into runtime output pointers `+0x98..+0xAC`.
`FUN_00419630` then publishes live counts and progress ratios through those
pointers.

`FUN_00418A90` does **not** copy Sub-G or Sub-M into this runtime block. Its
optional 22-dword source is the current behavior-instance template supplied at
the `FUN_004381F0` call boundary (`param_2 + 0x44`). Section-12 Sub-M supplies
the component-presence gate and variable bindings, while the behavior template
supplies initial runtime policy.

### Progressive destruction state

The behavior death callback `FUN_00419750` starts the progressive sequence by
restoring health to 10,000,000, clearing dying bit `0x4000`, and changing
runtime `+0x94` from zero to one. Matched full/demo `FUN_00419B50` first adds
frame time to that field, then advances one destruction stage per 100,000
microseconds. Stage 1 emits effect selector 25, stages 2 through 30 use
`stage / 2`, and stage 31 synchronously invokes selector `0x10000` while the
field still contains positive `old + dt`. Only after that return does stage 32
write `-1`, queue pickup destruction, and re-enter factory death; its action
snapshot still sees revived health, while callback acknowledgement and final
completion adopt zero. These thresholds and mutation boundaries are code
constants, not authored Sub-G fields. The synchronized retail authority remains
the CLOSED `runtime_re/captures/local/20260724-201127-friendly-factory-destruction/`
capture; the matched static proof requires no replacement Type-66 capture.

True Sub-G remains the distinct 104-byte block at type-record `+0xE8`; for
example type 46 uses it for hover configuration. Weapon inventory and fire
descriptors are recovered through their own runtime/resource paths and must
not be inferred from Sub-G. Likewise entity offsets `+0xA8..+0xAE` are the
four state-selected model slots, not weapon slots.

---

## 20. Gameplay World/Model Coordinate Contract (CONFIRMED)

The gameplay renderer uses one fixed-point coordinate domain for terrain,
entities, and model vertices:

- Terrain X/Z advances by `0x100` per grid cell. Entity X/Y/Z at
  `+0x96/+0x98/+0x9A` are signed 8.8 values, so one cell is `1.0` port unit.
- Terrain height bytes are signed and expanded with `height << 5`; divided by
  256 this is `height * 0.125` port units.
- Model records store signed `i16` X/Y/Z. In the type-flag-0 vertex handlers
  (`FUN_0046D610`, `FUN_0046D7E0`) each coordinate is multiplied by the
  entity/model 1.31 matrix using the high-word multiply and then added directly
  to the entity translation. There is no decimal `/100` conversion. An identity
  matrix therefore leaves the raw model coordinate in the same domain as the
  8.8 translation: gameplay world position is `raw / 256`.
- Inline model-instance attachment positions pass through the same matrix and
  must use the same `/256` conversion.

This also corrects two misleading investigation leads. `FUN_00465840` is a
wrapper around `FUN_00465870` used by projectile/effect queries, not the model
vertex transform. `FUN_00431A40 -> FUN_00431A60` is the second, water-specific
terrain pass; opaque terrain is `FUN_0042F960 -> FUN_0042F980`.

The bounded world draw is also explicit: `FUN_00433130` passes Section 13
`+0x88` into `FUN_004330D0`, which derives
`min(2*floor(depth*4/5), 52)` columns and `min(depth, 30)` rows
(`DAT_004CAB70`, `DAT_004CAB74`). The real levels author depths from 18 through
30. The port now builds that per-level camera-relative footprint and wraps only
the Section-10 sample indices, instead of drawing repeated complete 256x256
meshes. Exact fog fade values remain separate work.

`FUN_00431A60`, the second water-specific pass, loops over those same
`DAT_004CAB70/74` bounds. It does not create a larger near-water grid, a coarse
far ocean, or a damping seam. The port now applies the per-level scan footprint
to both opaque terrain and wave-displaced shoreline water.

The dynamic terrain-light buffer at `DAT_004FE820` is a signed 32×32 X-major
window rebuilt for each presented frame. The field-6 chain clears all 0x400
bytes; the field-4 chain then positions it with
`FUN_004383F0(camera_x_raw, camera_z_raw + 0x0A00)`, accumulates current
particle-frame light through `FUN_0043D410`/`FUN_004385E0`, and only afterward
runs terrain/model consumers. Signed 8.8 camera coordinates truncate toward
zero. Positive frame-middle values scale by height above the signed coarse
terrain cell (minimum 0x100 raw); negative values encode a radius multiplied by
0x100 and fade from age 201 through 255. `FUN_00441610` supplies the saturated
point writer and `FUN_00441420` the symmetric explosion pattern. The port now
wires ordinary particle lights into gameplay and Intro2; only the separately
proven point/explosion callers and the generic projection/lifecycle gate remain.

---
