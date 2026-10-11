# V2000 Loading / Transition System

Verbatim move of former GAME_MECHANICS section 14 during the 2026-08-24 cohesion split.

## 14. Loading/Transition System (CONFIRMED)

### Game Phase State Machine

The byte at session offset `+0x296` is a phase selector, but the former
one-name-per-value table combined unrelated call-site meanings. Proven bounds
are narrower: phases 1 and 2 use the `0x10CC`-tick early timeout (50 Hz); phase 3 is the
entity-group enabling/setup transition; phase 4 uses the `0x0B22`-tick results
exit; and values greater than 4 pass gameplay-only notification guards. Phase 5
must not be globally relabelled as a cinematic state.

`FUN_00456F10` returns the byte, `FUN_00456F20` performs the phase-3 setup
transition, and `FUN_0044F8B0` supplies the load-time value.

Ordinary `451710` loading (`+0x28E == 0`) clears both visible notification
slots after cargo restoration while preserving the session's resource-event
mask. Respawn rebases only the direct slot timestamp. Exact phase guards,
reset ownership and post-load clock order are in
[CAMPAIGN_CARGO.md](CAMPAIGN_CARGO.md#load-time-notification-policy).

### Active world frame and presentation order

The EXE mode pump establishes ordering independently of the historical
"update/render chain" labels in the decompilation comments. `004942E0`
calls `00493FA0` before `00493F50`. For descriptor `004D0918`, the former
walks descriptor `+0x18` (`004D0840`, `FUN_004530D0`), which drives the
world scheduler; the latter walks `+0x10` (`004D07A8`), which contains
the presentation callbacks. The allocated descriptor has a `+0x0C` header,
so their corresponding runtime fields are `+0x24` and `+0x1C`.

The normal `FUN_0044FFA0` simulation visit orders `FUN_00413500` actor
movers, `FUN_004374B0` static programs, `FUN_0040ED10` eye/focus,
`FUN_00440120` physical particles, `FUN_00411A80` active contacts, and
`FUN_004503C0` cinematic follow. ED10 runs once with the supplied duration;
follow reads the surviving subject after contact and changes the next proxy
step without recomputing the already published eye.

`13500` ends its intrusive actor walk with the `FUN_00414990` deferred
sweep. A death queued during that walk is removed there; a later particle
or `11A80` contact death waits until the next `13500` sweep. `11A80` does
not run another sweep before presentation. Actor `DCA0/E870` callbacks
also do not invoke `11AD0`; holding their completed movement for an
unimplemented contact adapter incorrectly freezes an earlier phase.

Only afterward does `FUN_00453760 -> FUN_00411720 -> FUN_00411400` visit
the live actor list for presentation. 11400 publishes `0x06000000` detail
bits, submits the actor model, then drains its `+0x38` shot FIFO through
`FUN_00414870`. New projectiles can be presented at birth but first move
in the following 40120 visit. Aim append order is therefore distinct from
particle allocation and simulation order. With the final-card flag
`world+0x28A` set, 53760 calls `FUN_004113E0` for Klaus alone, leaving
ordinary actor detail bits and shot FIFOs untouched.

Section-2 actor commands run later in `FUN_00452CB0 -> FUN_00452790`.
The port reads the current level's actual strings up to `#`, preserving
table order, inclusive start/end milliseconds, and the layout-preset parser
skip. A zero end has no upper bound. `FUN_00452270` operation 2 ORs
`0x68000` on every active visit, operation 3 clears it, and other default
operations retain a camera allocation tag. `FUN_0044F8B0` initially clears
that selection. Commands therefore affect the following simulation pass;
record gaps preserve the prior target. Four enables omitted by the former
copied window table (spawns 24/26 at 15 seconds and 1/40 at 22 seconds)
are present in the original strings. The final backdrop change is strict
`4000 < tick`, distinct from a caption becoming active at tick 4000.

### Live worlds beneath the Klaus cover

The normal first-world handoff returns to descriptor `004D0918`, through
`FUN_004515E0 -> FUN_00451710`, while Klaus still owns command 2. The
125,000-us setup request primes his ordinary task; it does not create a
separate world-render-only phase. Subsequent `FUN_0044FFA0` callbacks update
the loaded world, particles, contacts, and camera beneath his opening model,
just as Intro2 begins updating beneath the New Game reveal.

The port retains the `MenuShell` only as the persistent Klaus task and
presentation state over `GameState::Playing`. Its callback precedes the
world actors, and its overlay is submitted after the world/HUD. Closing
before the load remains a separate `GameState::PostIntro` over the completed
black card. The first-world load resets the retail clock, starts sound 50
after setup, and restores live gameplay immediately; visible morph completion
only retires the cover. It does not start the world simulation.

Before load, descriptor `004D08C8 +0x08` points to `004D0710`, whose
callback array is `0044FE20, 00450C00, 0`. 50C00 advances Klaus, ED10,
and physical particles. The ordinary actor pass, camera-proxy integration,
static programs, active contacts, and cinematic follow stay suspended.
Particles therefore retain their real callbacks and random consumption
through the closing leg. The port separates `advance_proxy` from
`advance_eye` to preserve that distinction.

At the end of an active Intro2 visit, 503C0 sets `world+0x290` only when
the integer retail tick exceeds `0x10CC`. The next 44FFA0 visit advances
its clock, consumes the flag, and dispatches event 2 before ticking actors.
The port retains this exit latch instead of using a floating-point duration
comparison to leave during the same visit.

The exact commands, retained task fields, and descriptor transitions are in
[the Klaus handoff contract](MENU_SYSTEM.md#persistent-klaus-animation-intro-sequence-behavior).

### Final-card world audio

The user's observation that insect and other scene sounds stop when the screen
turns black has a source-backed positional-audio boundary. It does not imply
that every audio channel is muted.

At the first cinematic `503C0` visit with `tick > 4000` and `world+0x28A == 0`,
`4F3E0(0x20)` ORs the bit into `DAT_004F72D0` and passes the resulting mask to
`4CE70`. That function writes `DAT_004F716C`; on a zero-to-nonzero transition,
it walks every logical positional emitter and calls `4C810`. The latter stops
and releases the emitter's physical voice through `495700`, then clears its
voice pointer at `+0x1C`. The logical looping emitter remains allocated.

While the mask is nonzero, `4C970` skips positional mixing, audible sound-11
warble and alias resolution. It still calls `4C940` to discard disposable
one-shots. Thus this phase changes shared RNG consumption as well as audible
output. `4F400` clears requested mask bits; a remaining nonzero bit continues
to suppress mixing. The independent master-volume value is unchanged, and
global sound calls do not pass through this positional-emitter gate. General
mixing and voice custody are described in
[the audio contract](GAME_MECHANICS.md#positional-sound-transform-confirmed--fun_0044c970).

The accepted `20260722-022303-intro2-actor-ai.jsonl` corroborates the distinction.
At tick 4004, line 150651 stops the previously audible 10494-byte flight-voice
PCM family. Between ticks 4001 and 4301, the trace still records 25 physical
voice starts from the 2444-byte and 29012-byte global PCM families, with master
volume `65535` throughout. Multiple global IDs share those PCM resources, so
this passive evidence does not uniquely identify each call's sound ID.

The same recording clears the bit at `453EF9` (`44F400(0x20)` inside
`453E00`) when the first world starts; nothing clears it earlier, so the mask
lasts through the post-Intro closing morph. The port latches the card once,
releases the retained positional voices, and skips positional mixing until
that world entry, while still discarding disposable one-shots.

The source's world-emitter cutoff is established; matched port playback and
voice ownership at the black card remain an acceptance task. It should not be
implemented as a blanket stop of global sound or a master-volume change.

### Phase Tick (`FUN_004503c0`)

Per-frame phase management:
- **Phase 4:** After `0xB22` ticks, sets exit flag `+0x290 = 1`
- **Phase 1/2:** After `0x10CC` ticks, sets exit flag `+0x290 = 1`
- **Phase < 3, after tick 4000:** Disables world positional audio through
  `FUN_0044F3E0(0x20)`, requests presentation through `FUN_00456750`, and sets
  the final-card state `+0x28A = 1`.

For cinematic phases, the same routine drives a hidden camera entity (`+0x2A8`)
toward the current Section-2-selected target (`+0x308`):

- desired X velocity is `target.x + target.vx - camera.x`
- desired Z velocity additionally trails the target by `0x200` (2.0 port units)
- each desired velocity is clamped to `+-10000` fixed units and the camera's
  current velocity moves one sixth toward it per tick
- an axis error of `0x3201` or more snaps the camera entity to the target and
  clears all three velocities
- Y follows target displacement without adding target Y velocity, applies an
  additional terrain correction, and is clamped at least `0x100` above the
  sampled ground

Intro2's Section-2 default-operation records select the following early camera
targets (object IDs 38914..38975 map directly to spawn indices 0..61):

| Start | Target index | Object ID |
|-------|--------------|-----------|
| 0 ms | 32 | 38946 |
| 15000 ms | 8 | 38922 |
| 23000 ms | 25 | 38939 |
| 27500 ms | 0 | 38914 |
| 33000 ms | 46 | 38960 |

The 2026-07-17 full-session trace observes the class-1 camera at retail tick
751 still adjacent to target 32, with positive X velocity toward target 8,
confirming that the first real cut is the 15000-ms command rather than the old
7.5-second footage estimate.

`FUN_0040ED10` constructs the rendered chase view around that entity. Its
base camera distance is `0x800` (8.0 port units), so the Section-2 commands are
shot-target changes rather than instructions for a free orbit.

The same function adds the Display -> Active Camera value times `0x60`, then
modulates that addition with the tracked entity's orientation. For the level
craft this is `8 + setting*(0x60/256)*(1-cos(relative heading))`: the compass
bearing stays fixed, but the camera backs from 8.0 to 12.5 units at the default
setting 6 as the craft turns from facing away to facing toward the lens. Retail
capture confirmation and timestamps are in `RETAIL_CAPTURE_2026-07-12.md`.

The matched 2026-07-30 Minimum/Default/Maximum gameplay captures confirm exact
settings 0/6/10 and keep `FUN_0040ED10` parameters 3/4 at zero throughout the
usable foreground intervals. Active Camera therefore changes this horizontal
distance without adding a setting-dependent vertical offset. Joint-stable
minimum/median eye clearances are 2/765, 450/777, and 330/806 raw respectively;
the 2-raw Minimum sample is valid retail behavior over water, not evidence for
a missing universal height correction. The later VTOL routes diverge under
human control, so those clearance ranges are pose-normalized envelopes rather
than point-matched setting comparisons.

The independent look target is orientation-biased, not velocity-biased.
`FUN_0040ED10` copies the tracked entity's 3x3 basis from `+0x0C`, then adds
`0xFA` raw units along the third stored vector (`+0x24/+0x28/+0x2C`) to the
entity origin. Before springing, it enforces a signed forward separation of at
least `0x100` between focus Z and eye Z.

`FUN_0040F3A0` retains two 22-short state blocks. `FUN_0040F5E0` advances the
eye toward its target with per-axis velocity limits `[0x800,0x800,0x1000]`,
accelerations `[0x800,0x3000,0x2000]`, dead zones `[0x20,0x60,0x20]`, and
leash values `[0x200,0x100,0x200]` (the vertical eye axis is not hard-leashed).
`FUN_0040F800` advances focus with velocity limits `[0x1000;3]`, accelerations
`[0x4000,0x3000,0x3000]`, dead zones `[0x30;3]`, and `[0x100;3]` leashes.
Both use `FUN_0040F740` with `microseconds << 11` as signed-Q31 time. Because
positions and errors remain signed words, horizontal spring motion naturally
takes the short path across the world seam.

The active handler calls `FUN_0040ED10` once with that frame's elapsed
microseconds, then runs physical particles/contact and `FUN_004503C0` follow
control. The recorded 8,000-us entries do not establish a fixed camera
substep. `IntroCameraController` therefore integrates and applies the
one-sixth follow once per supplied world frame; it does not repeat follow
control to fill a nominal 8-ms clock. The captured 8-ms golden transitions
remain valid at their original duration.

Caller context matters: active `FUN_0044FFA0` passes the current terrain
pointer to `FUN_0040ED10`, activating its eye-elevation terrain ray before the
shared spring. The routine first bilinearly samples signed Section-10 height at
the craft, combines it with the Section-10 sea-height word, and adjusts a
vertical anchor with separate above-water, underwater, and high-altitude
branches. Starting 0x80 raw behind the craft, it then probes every 0x80 raw
until the chosen camera distance. `FUN_00436C30` makes each probe three cells
wide: it takes the maximum of the three signed four-corner cell-centre heights
and adds the selected Section-9 model's header `+0x0A` collision radius, except
for authored object kinds 11 and 28. A 0x25800000 Q31 clearance rise is added
before the steepest slope is normalized and scaled to the eye distance.

The port implements this entire raw-word terrain target using Section 10,
Section 9, and the global model pool. Intro2 uses this terrain context too:
both active `44FFA0` (return `45019F`) and closing `50C00` (return `450C7C`)
supply the current Section-10 pointer. The accepted camera call trace records
nonzero parameter 2 `221DDCC8` at Intro2 ticks 750, 1150 and 1375; the zero
parameters 3/4 are different arguments. Null-context frontend callers retain
the simpler craft-Y target. Do not disable the terrain ray for Intro2 based on
the null frontend calls. Both policies share the exact `FUN_0040F3A0` springs.

#### Intro2 type-34 meteor physics

The four Type34/model560 actors use class19 Boulder Trailing Fire. Their
constructor, activation events, displacement-driven Q31 rolling, shared
scheduler and RNG trail, terrain impact, radial damage, and deferred death
contract are owned by [INTRO2_METEORS.md](INTRO2_METEORS.md).

The opening renderer consumes their live physical pose. Sampled tumble onsets,
lifetimes, fitted rotation axes, and particle direction windows are observations,
not inputs to the live simulation. Retail-matched full-sequence presentation
remains the acceptance target.

#### Shared world-particle traversal

Class `0x10` then runs `FUN_0043ECD0` after ordinary position integration. It
applies full gravity with no damping and attempts one trail even when the
current traversal has a zero time delta. The post-integration centre is
compared with the authored flat sea plane: strict above selects class `0x1F`,
while equality or below selects class `0x2A`. Class `0x1F` animates the
recovered 23-frame 895..905 sequence, starts with raw velocity `[0,80,0]`, uses
strength-3/mass-5 drag, and lives 50 ticks; class `0x2A` retains its authored
descriptor bias and underwater callback. The allocator is a fixed 200-record
pool; saturation recycles eligible aged low-priority trails or drops an
emission, never grows the pool or evicts the higher-priority parent burst. The Rust port now routes
all four Intro2 impacts through this shared bounded particle/event path and the
recovered wrapped positional-audio mixer. It also preserves `FUN_0043D290`'s
virgin slot order, `FUN_00440A60`'s intrusive priority/victim rules,
`FUN_00440120`'s physical `0..199` update scan, priority/head-to-tail
presentation, and `FUN_0043D410`'s address-derived size phase. Inline trail
allocations therefore update in the same pass only when their recycled slot is
later than the parent. The same physical traversal now also invokes class 32's
owner-velocity refresh after integration and class 1's collision-mode dispatch
before advancing to the next slot. Class-1 impact/surface allocation occurs
before projectile deletion, so saturation can reject an effect even though the
projectile frees a slot immediately afterward. Any successful later-slot child
receives its own update in the birth pass; an earlier-slot child waits for the
next traversal. The Rust implementation intentionally treats class 1's
slow-speed self-free as terminal: retail nominally returns to the outer mode-3
switch with a reset class-0 record, but reproducing that double-free-adjacent
hazard without a focused runtime trace would be less defensible than the safe
observable result.

`FUN_00440A60` classifies water at allocation and stores that state in the
record; `FUN_00440120` compares it with the post-integration classification
rather than reconstructing an old state from an earlier wave tick. Descriptors
whose collision mode is 1, update callback is `FUN_0043F260`, and surface
callback is `FUN_0043E230` are exactly decimal classes 7..11, 33, 59, and
70..72 (not hexadecimal class `0x33` cargo). Their ordinary surface selectors
snap raw Y to the response height, negate half the raw vertical velocity with
signed truncation toward zero, and continue from water into the endpoint
terrain stage. Selector 6 always consumes its one-in-four RNG gate,
conditionally calls `FUN_0043E060` for sound 26/27 and rate, attempts one
stationary class-13 particle while the parent remains live, then deletes the
parent and short-circuits terrain. The terrain gate uses the four-corner
bilinear height; `FUN_0043DB60` subsequently supplies the coarse current-cell
height to the response callback for this descriptor family.

For the later E4F0 family, the ordinary branch passes strength `0x1000` to
`FUN_00440DC0`, producing `max(1, (4 * DAT_004F72CC) >> 16)` attempts.
Selector-6 water instead uses `DAT_004F72CC >> 13` directly, so a normal
`0x10000` frame attempts eight children while a scale below `0x2000` attempts
none. The port now preserves both policies and feeds them from the exact
eight-frame Intro2/gameplay governor.

Related state flags at `g_rng_state`:

| Offset | Flag |
|--------|------|
| +0x289 | Level active (set 1 at load, cleared at end) |
| +0x28A | Loading screen sprite shown |
| +0x28E | Level teardown done |
| +0x28F | Result screen shown |
| +0x290 | Exit phase / move to next state |
| +0x292 | Re-entering from load |
| +0x295 | Full-frame sprite cursor (0 inactive, 1..8 active) |
| +0x296 | **Game phase** |

### OVL Loading Sequence

**Path format** (`FUN_00493654`, string at 0x004D5D50): `"overlay\%dx%dxx.ovl"`
- First `%d` = variant (0-3), second `%d` = level ID (0-52)
- Examples: variant 0, level 2 → `0X2XX.OVL`; variant 1, level 14 → `1X14XX.OVL`

**Loading chain:**
1. `FUN_00493654` — format filename from variant + level ID
2. `FUN_00493680` — open file (allocates 8-byte handle, builds search path)
3. `FUN_00493BF0` — section reader: iterates 15 sections sequentially
4. `FUN_00493CE0` — per-section: reads 4 bytes, verifies against `"abcd"` magic
5. Handler vtable dispatch per section (load function at `[vtable + 0]`)
6. `FUN_00493DB0` — error check (success code `0x3600`)

**Buffered read** (`File_BufferedRead` at 0x468E70): Reads in chunks of `DAT_004d48e8` bytes, calling progress callback `FUN_00468DB0` between chunks — drives the loading bar update.

**PRELOAD.DAT** loaded by `FUN_00493860` — reads per-section per-level resource counts into `g_sprite_meta[]` arrays before any level OVL.

### Loading Screen Display (`FUN_0042b040`)

Renders during OVL load:
1. Draws 3 background sprites from `DAT_004fe62c + 0x1428/0x142C/0x1430`
2. Draws centered text:
   - String index 34 (0x22): **"Loading"** when `param_4 <= 2`
   - String index 35 (0x23): **"Retrying"** when `param_4 > 2`
3. Text centered via `FUN_00470f80` (measure width) → `screen_center_x - width/2`

### Briefing Sequence

**Timed tag format** in Section 2 strings:
```
< start_ms, end_ms, color, size, speed > Text body
```

Real examples:
```
<    *, 3000,2,30, *>Entering Peasant World
< 4000, 7000,2,30,*>Save the world by killing the creatures.
< 8000,11000,2,30,*>Kill all the creatures and then the hive.
```

`*` = wildcard (use default). Fields: start_ms, end_ms, color_index, font_size, speed.

**Tag parser** (`FUN_004521f0`): Comma-delimited number/wildcard parser within `< >` tags.

**Briefing renderer** (`FUN_00452790`): Applies timing conditions, renders text with computed font params:
- Case 2: color type → color 5, size 0x10, scale 0x50 for standard text
- Case 7: special render type — `0x40+` = face command, `0xF` = blinking cursor
- Blinking cursor: truncates text based on `g_default_param / 10` even/odd

**Face command dispatcher** (`FUN_00452270`):
- Case 1: Timer-based string substitution
- Case 2/8: Enemy type name lookup via vtable
- Case 7/9: Map/world name lookup (`FUN_00443260`)
- Case 0xC/0xE: Enemy count (empty string if 0)
- Case 0xF: Blinking every 40ms
- **Case 0x40:** Terminates briefing → sets `+0x290=1`

**"Press S to Save, Space to Continue"** — string index 26 in 0X2XX Section 2, displayed during briefing.

**Full-frame sprite sequence** (`+0x295`): this is a shared presentation
cursor, not a briefing-specific state or a results-transition gate.
`FUN_00456750` unconditionally writes one, so a new request restarts an active
sequence. Exact-matched retail `FUN_00453410` and demo `FUN_00452B90` consume
one entry per successfully built gameplay draw command:

`0x227, 0x228, 0x228, 0x228, 0x229, 0x22A, 0x22B, 0x22C`, then the null
sentinel resets the byte to zero. Command-arena exhaustion retains the current
entry; the sprite-pool pointer lookup itself is unchecked in the original.
Every selected-tier resource is a 2x2 fixed-row-28 material. Sprite `0x227`
has flags `0x04` (masked); `0x228..=0x22C` have flags `0x14` (additive).

The mode pump's `493FA0 -> 493F50` order places this presentation consumer
after simulation, so a simulation producer first appears in that same frame.
Its signed queue key zero sorts after positive-depth world geometry/effects
and Klaus, while captions and gameplay HUD belong to later passes. Intro2
preserves this boundary and composition. The legacy Playing adapter still
prepares before simulation, delaying its producers by one frame; that host
timing remains open. Both backends implement the masked/additive operations;
the software renderer's additive rows keep the top four bits of each RGB565
field, so an entry over black shows `texel & 0xF79E`.

Intro2's final card is this sequence's first producer: `503C0` requests it on
its first visit past tick 4000 (below). In the `V2000-nocd-faststart04.run`
recording, the card's frames from tick 4003 fill the whole surface with
`0xB596` four times (`0x227`, then `0x228` three times), then `0x8410`,
`0x738E`, `0x528A`, `0x2104` and black: a fade from grey. A TTD write query
on a mid-screen pixel of the tick-4003 frame stops at `0047838D`, reached from
the queue drain `00494A50` through a 640x480 textured record.

Kind 27's two distance-gated opcode-6 records are late producers of this same
cursor; they do not own a separate flash or presentation sequence.

### Level Completion Flow

**Win trigger** (`FUN_004567b0`, 0x4567B0):
- Requires `+0x28F == 0` (not abort) and session `+0x2BC < 1`
- Stamps `+0x2BC` with `DAT_004FED60` (50-Hz ticks since `FUN_00428AD0`)
- Calls `FUN_0042EE70` (campaign world-completion/time-trophy transaction) +
  `FUN_0042EB00` (current-world index; NULL out-arg is a no-op)
- This callback does not change the active descriptor or pause gameplay.
- In-game `FUN_00452CB0` draws `0xBD..0xC4` only while `+0x2BC > 500`,
  with age `(DAT_004FED60 - +0x2BC) * 1000 / 50`. The `0xBD` row formats
  minutes = `+0x2BC / 3000` and seconds = `(+0x2BC / 50) % 60` through
  `FUN_00452270` case 1. The HUD and hive-exit hint remain visible.

**Loss trigger** (`FUN_00456790`, 0x456790):
- If the session exists and `+0x2BC` is exactly zero, sets that dword to
  `0xFFFFFFFF` (-1, pending). It does not itself run the casualty predicate.

**Per-control-slot one-shot helper** (`FUN_00456820`, 0x456820):
- Parameter 0 tests/sets CLAIMED bit `0x02` through `FUN_0042ED20`; a new
  claim queues direct line `0xD0` and resource event `0x14`, then returns 1.
- A nonzero parameter tests/sets the separate bit `0x08` through
  `FUN_0042ED50` and may invoke operation `0x13F`; it still returns 0. The
  precise semantic name of that operation remains unresolved. This helper is
  not itself the win/loss resolver.

**Level teardown** (`FUN_004561a0`, 0x4561A0): Unloads OVL, reloads next level, resets entity controller, sets `+0x292 = 1` (re-entering flag).

**Level abort** (`FUN_00456960`, 0x456960): Collects disposable positional
audio through `0044C940`, sets session `+0x28F=1` and `+0x2BC=0`, and queues
sound `0x3E` at the controlled player when that actor resolves. Then
`0042F1A0` runs the actor abort sweep, terrain transform, controller state 5,
and authored replacement frame; `00456750` supplies the outer flash request.
The [native casualty limit](CAMPAIGN_FAILURE.md#native-casualty-limit-0042dd10-selector-0)
reaches this same body through `0042DD10` after its `0xD4` message. That cause
has no Main Base terminal actor. Main Base destruction's separate `+0x1F8`
request is consumed at the same deferred campaign-selector boundary.

**Return to menu** (`FUN_0042a940`, 0x42A940): Routes based on `DAT_004db200` — 0=single-player menu, 1-2=network lobby.

### Per-Level Time-Trophy Countdown (STATIC + PORT-LIVE 2026-08-10)

Section-13 header dword `+0x5C` is an authored time-trophy deadline in
seconds, not a virus threshold. Retail `FUN_0042E570` and demo
`FUN_0042DFC0` initialize signed controller timer `+0x1EC` and state `+0x1F0`.
Existing campaign-completion bit `0x1` wins first and selects state 4 when
time-trophy bit `0x8` is also set or state 3 otherwise. An active results byte
selects state 5 next. A zero deadline selects state 4, zero remaining time, and
claims bit `0x8`; every other fresh world starts state 1 at
`seconds * 1000 + 500`. Cistern stores 180, so its countdown begins at 3:00.

Retail `FUN_0042D9B0` and instruction-identical demo `FUN_0042D400` discard
each frame's sub-millisecond remainder (`elapsed_us / 1000`) and emit direct
global sound 51 only when the final whole-second quotient changed and matches
the following schedule. A long frame tests only its endpoint and does not
replay cadence points crossed in between:

| Remaining time | Warning interval | Playback rate |
|---|---:|---:|
| 60 seconds or more | 30 seconds | `0x10000` (1.0x) |
| 20..59 seconds | 10 seconds | `0x12000` (1.125x) |
| 10..19 seconds | 2 seconds | `0x14000` (1.25x) |
| 0..9 seconds | 1 second | `0x18000` (1.5x) |

The `+500` initialization and crossing test mean Cistern's first warning is at
2:30, followed by 2:00, 1:30, 1:00, 0:50/0:40/0:30/0:20,
0:18/0:16/0:14/0:12/0:10, then every second from 0:09 through the quotient-zero
warning. The `+500` bias leaves the timer active briefly after that warning;
when a later frame's elapsed milliseconds equal or exceed the remaining value,
`0x0042DB07..0x0042DB37` plays direct global sound 2 at rate `0x10000`, clears
the remaining time, and changes the controller to state 2.

The controller's per-level campaign bits separate this deadline from the
hidden pickup. Retail `FUN_0042EE70` / demo `FUN_0042E8B0` saves the world by
setting bit `0x1`; doing so while state 1 is still active changes to state 4
and claims the independent time-trophy bit `0x8`. Saving in every other state
changes it to state 3 and does not award that bit.
Selector `0x3F` instead claims hidden-trophy bit `0x2`; Cistern's sole authored
type-61/model-138 instance uses that selector and restores hull/buffer when
collected. The timed trophy and clock are a separate HUD presentation:
`FUN_004539D0 -> FUN_0042F100 -> FUN_004292B0` passes the controller state and
whole seconds, and `00429509` admits model 138 and the clock only in state 1.
Expiry therefore removes both from the HUD without destroying a world pickup.
The model uses depth 2000, twice the cargo spin, and the cargo animation
callback; Section-1 global points 40/41 (system-3 local 27/28) are absolute
model/text anchors. The clock uses global font 0 and `%1d:%02d`. This branch
was absent from the port's status-orb compositor and is now connected to the
same authoritative timer used by audio. See [HUD_RENDERING.md](HUD_RENDERING.md#time-trophy-and-countdown).

The port now decodes `+0x5C` through a typed `LevelDescriptor` accessor and
retains the timer in the existing authoritative world-controller storage—the
same owner whose atomic Main Base abort suffix writes state 5. World loads,
active-gameplay ticking, exact centered Q16 warning/expiry audio, and all three
campaign bits are live for the in-session controller. The
campaign-completion transition runs at hive death and retains state 3 or 4;
it does not force results state 5. Native checkpoints retain these campaign
bits; hive completion itself does not save to disk. A newly set bit `0x8` also
reaches `456820(1) -> 413600(player,0x13F) -> 40DB20 -> 445A90`. The packed
operation is selector `0x3F`, amount 1: count one trophy, grant one extra life
on every fifth count, and play sound 50 at normal rate or `0x20000` for that
extra life. Amount 1 does not restore shield/hull or claim hidden bit `0x2`.
The same counted-trophy transaction and feedback now handle this award and
physical pickups. Repeated initialization/completion of an already-claimed
bit does not count again. Retail intentionally grants the time award on a
zero-deadline world; its later `Time trophy collected.` results row is not
evidence of touching the independent hidden pickup.

NoCD05's actual played world is OVL15/control slot 3 (300-second deadline),
not the later failed OVL17 load. At `2E2352:1307` its state is 1 with 296051 ms;
at `688029:B4F` state remains 1 with 147944 ms. Thus that recording validates
the active countdown and callback identity, not expiry or a time award. The
expiry disappearance is established by the exact HUD state gate and tested
at the final `0:00` half-second and the following expiry.

The port additionally follows the user's physical-pickup clock-stop request
through a separate, labelled tick/HUD gate; it preserves the native hidden
and timed campaign bits. This policy's evidence bounds and follow-up owner
are in [Player shield](PLAYER_SHIELD.md#requested-pickup-clock-policy).

### Campaign Exit Routes (RUNTIME-VALIDATED 2026-07-22)

The secret route and ordinary victory route share the same authored Section-13
route-table mechanism, but use different activation/contact owners. Never
implement either one as a universal "type 111 means next level" rule.

**Authored secret gates.** `FUN_00433BD0` scans terrain attributes in X-major
order. Kinds `0x16..0x1A` map to independent subtypes 1..5 and call
`FUN_00416FF0` to construct a
generic type-`0x6F` (111) marker. The terrain descriptor refers to global model
39 (`levexit`); the retained live marker presents as model 16 (`exithide`). The
kind/subtype and generic runtime type do not contain the destination. Section
13 `+0x64/+0x68` supplies an array of 0x20-byte campaign records. For records
whose flags at `+0x0C` contain `0x10`, `FUN_0042DD10` matches signed subtype
`+0x1D` to the live marker; destination logical level is `+0x00`, signed-8.8
arrival XYZ is `+0x04/+0x06/+0x08`, and global overlay id is logical level +
12. `FUN_0042E270` copies that route-specific arrival and heading `0x4000`.
`FUN_00456D10` then raises player `+0x1F4` plus camera transition type 15.
`FUN_0042EF60` controls marker visibility/capability, not whether the marker
exists or whether only the two captured cells may route. The player static
contact path `427E20 -> 4464B0` stamps the actual terrain cell. `42DD10 ->
42E300 -> 446440` admits that stamp for fewer than two wrapped retail ticks;
proximity to Type111 is not the route predicate. Record `+0x1E` compares terrain
bit3 and `+0x1F` compares bit4, with signed selector2 meaning wildcard. Records
are evaluated in authored order, retaining their original indices for pair
links. Only the low flags byte is consumed; Cistern's `0x00AA0D10` is a normal
`0x10` route. All84 ordinary non-abort records in OVL13..48 use that policy;
three require selectors `[1,0]`, the others `[2,2]`. Level13's other two
records are abort records and remain outside the normal-route path.

**Late-demo authored-data boundary (STATIC DIFFERENTIAL 2026-08-02).** For
both retained display tiers of Levels 13, 14, 15, and Intro2, demo and retail
Section 10 are byte-identical and every Section-13 byte from `+0x40` through
EOF is byte-identical. The only Section-13 differences are stale build/source
bytes after the parsed NUL-terminated name inside the first 64 bytes. Thus all
descriptor parameters, spawn pointer arrays, entity records, optional
animation/config blocks, and campaign records are the retail data. World
resource overlay X6 is also byte-identical per tier, preserving marker kind and
model mapping. In particular, Level 15 still authors exits to absent demo
globals 19, 30, and 17 as well as the present Level 14.

The promotional result payload is elsewhere: demo system overlay X3 appends 12
Section-2 strings at local ids `182..193`, cumulative demo global ids
`300..311`. They contain the early-October availability line, feature list,
Grolier URL, and continue prompt; retail X3 ends before this block. Demo
`FUN_00453AF0` reads and draws all twelve entries.

That renderer occupies the corresponding content slot of demo completion
descriptor `004CAA70`; retail descriptor `004D0AA0` uses `FUN_00454390` for
its normal progression/result content. The upstream event dispatcher
`0044FCD0` / `0044F4D0`, predecessor descriptor `004D0A30` / `004CAA00`,
phase/flag selector `00453E00` / `00453560`, and mode switch `00456680` /
`00455FE0` have the same structure, with the switch pair instruction-exact.
Both initializers load system overlay 51, and exit pair `004556A0` /
`00454FF0` is instruction-exact. The selector enters this descriptor for
session phase `+0x296 > 4` when `+0x28E != 0` or the paired input helper
returns 1; paired update path `004558A0` / `004551F0` also enters it after a
pending transition exceeds 1000 ms.

The proven demo delta is therefore presentation inside the normal completion
mode, not campaign/terrain data or a special activation branch. The exact
Level-15 writer of those phase/flag inputs, any attempted Level-16 availability
check, and the post-Space destination remain unproven. Do not introduce a Rust
route restriction from the demo.

Accepted synchronized trace
`runtime_re/captures/local/20260722-105044-secret-level-transition/` proves:

- Level 13 cell `(149,239)`, kind `23`, live raw marker position
  `[-27264,-2080,-4224]` routes to Level 30 (Cistern).
- Level 30 cell `(21,39)`, kind `25`, live raw marker position
  `[5504,2912,10112]` routes back to Level 13.
- The Cistern arrival is raw `[7424,5120,9728]`, heading `0x4000`, velocity
  zero. The Level-1 return arrival is `[-27392,0,-5120]`, heading `0x4000`,
  velocity zero.
- Each retained gate owns a positional looping global sound 100. That resource
  aliases PCM global 38. Three transient duplicates in Cistern briefly start
  the same loop and stop when marker de-duplication removes them.
- Marker de-duplication does **not** remove or suppress Section-10 rendering.
  The static pass continues to submit model 39 (`levexit`) as the visible
  transporter shell while the retained type-111/model-16 allocation owns the
  live trigger/helper state. Do not filter model 39 merely because a live gate
  exists.
- There is no additional contact one-shot. Player contact requests the warp;
  it does not collect a type-61 object or mutate controller pickup state.
- The controller pointer remains identical across every unload/reload. Its
  weapon descriptors and exact raw fuel value `99119` remain unchanged while
  each destination allocates a new player entity handle. Campaign replacement
  therefore creates a fresh entity/component but restores serialized
  controller state. Static `FUN_00443440`/`FUN_00443560` confirms that mode,
  fuel, health, capabilities, cargo capacity, and weapon descriptors are
  controller-owned. `42E270 ->448070` stages override flags `0xF`; after
  teardown `443440` sets the authored arrival, zero velocity, yaw `0x4000`,
  zero pitch/roll and health `40000`. The shield remains in controller
  `+0x1BC` and `443560` restores it to new entity `+0x50`; the NoCD save
  replay retains `80833` across this boundary. Direct campaign loading and
  map-save loading now use this same player-first controller snapshot. Dying
  state, joints, fan state, runtime weapon allocation and target ownership
  belong to the new body.
- The accepted handoffs had an empty conventional cargo list, but static
  `FUN_00443260` closes the ownership question independently: it clears the
  raw unlocked controller slots at `+0x19C` (not merely the clamped live-list
  capacity), walks the player's direct attachment list in order, and stores
  each live entity's type index (`+0x58`) plus its stamp word (`+0xB4`). No
  mutable entity state is serialized in this block. `FUN_004292B0`'s HUD
  consumes only the low-half type. The omitted retail restore consumer
  `FUN_00451C00` runs after destination authored construction: it stops at the
  first zero type and skips types 7/8/9/78/79/86/90/91/95/116. Other entries
  search the live list by type and the `FUN_00456C20` stamp; a miss invokes
  native construction with a zeroed record, then restores the saved stamp.
  Both paths attach through `443B30`, including the child's real callback.
  [CAMPAIGN_CARGO.md](CAMPAIGN_CARGO.md) owns the exact load/counter order,
  signed high-half mismatch, exclusions, and retained error prefixes.
  The port preserves raw unlock byte `+0x199` and derives live capacity through
  `FUN_00418620`; its bounded restore repair enforces exclusions and first-zero
  termination. Eligible cargo still lacks exact native stamp/match/miss
  ownership, so generic destination reconstruction is not full restoration.
  The real 14→39→14 loader regression snapshots a beam-carried peasant and
  intentionally skips restoring it, preserving each destination's authored
  peasants; this is a policy test, not evidence of a retail gate on that route.
  Cargo under a type-93 drop proxy is absent from the direct player list and
  remains in the departing world.
- The first cold Level-30 load requests system overlay51. Absence of another
  resource load on cached returns does not establish absence of the map mode.

**Normal-route map entry.** The retail session pump at `0045032F` evaluates
the route callback. A nonzero result writes destination `session+0x34` and
pending `+0x290`; the next tick at `00450089` consumes that pending byte and
dispatches event2 through `494540`. `44FCD0` selects descriptor `4D0A30`.
`453E00` then selects map descriptor `4D0AA0` for phase `+0x296 > 4` after
the transition helper completes. There is no source-level, destination-level,
secret-route or first-visit gate. `4555A0` bypasses normal map input only for
respawn `+0x28E` or same-world load `+0x292`; the `+0x28E=1` writers at
`4502D7/450300` belong to player death, not normal routing. All normal campaign
routes therefore retain their authored destination/arrival and enter the map,
including both Cistern directions. Space releases that route once; S retains
the same checkpoint through Save Game / Continue. The camera/closing-shell
transition animation remains a separate presentation boundary.

**Direct world entry.** Section13 has no per-world default player spawn.
`42E270` copies the incoming route's XYZ, so one destination can have several
correct arrival poses. The port's direct `--level` convenience scans all
ordinary maps in the selected presentation tier and chooses the first incoming
record in source/record order. This is a deterministic debug policy, not a
claim that retail uses that tie-break. New Game keeps `4D0B30`'s initial pose;
real routes and native saves keep their own poses. The six arenas (OVL25,27,
28,29,41,44) have no incoming campaign records and explicitly use the controller
default `[19712,-500,14848]`, heading `0x4000`. No Main Base-relative position
or level1 arrival is invented for later campaign maps.

**Ordinary Level 1 victory.** Accepted synchronized trace
`runtime_re/captures/local/20260722-115649-level1-to-level2/` proves seven
objective hostiles: four type-17/model-256 actors at 5000 health and three
type-47/model-302 actors at 3000. The gate predicate requires a live non-dying
state, resource byte `+0x64` bit `0x08`, and resource flag `0x01000000`.
Captured primary hits subtract exactly 1800 health. When the last eligible
hostile reaches zero, the controller waits exactly 2,000,000 microseconds and
makes the type-67/model-341 hive vulnerable at 2000 health. Lethal damage moves
the hive to dying model 343. The normal exit is the authored static marker
guarded by that hive; no new Type111 teleporter is created on death and contact
adds no sound. Unlock writes controller state2 and death writes state0, so
class-5 spit stops. The later marker contact selects system overlay51 for the
progress map; Space continues to Level14
(terrain 1, waves enabled). Level 13's subtype-1 campaign record and terrain
cell `(187,129)` encode this same route to logical level 2/global 14; its exact
arrival is `[17408,2560,-32512]`, heading `0x4000`, velocity zero. The live
port applies the lethal entity transition and consumes the actual static
contact stamp through the same route evaluator used by the other worlds.
Hive death records completion and starts the in-world `0xBD..0xC4` rows;
it does not open the progress map, hide the HUD, or require Space. On the
admitted marker contact the port retains the decoded Level-14 arrival, selects
the destination node, and opens the modal map. Space consumes that route once
and loads the next world. The existing transition's camera/closing-shell
animation remains a separate presentation boundary. Map S enters the authored
`PAUSE_SIMPLE` Save Game / Continue / Options menu. Saving writes the full native
checkpoint to a local `SlotNN`, replacing an existing data-directory or
parent-directory slot under the [save storage policy](SAVE_AND_SETTINGS.md#native-save-scope-and-restoration-boundary-confirmed).
Continue releases the retained route.

The earlier interpretation of `1BEB0 ->56D10` as an ordinary wreck-route
selector is withdrawn. At `41C7DC..41C7E3`, `456CB0()` must be nonzero;
`456CB9` reads **session+28F, the shared Main Base/casualty abort flag**. That interior branch
uses wrapping XZ distance squared `< 0x10000`, signed `dy < -0x80` and the
Sub-N timer, but `56D10` stages the session's already retained arrival; it
does not select a campaign record from the wreck. `1BC20` stores a marker
origin, not its subtype. Cistern's hive guards subtype3, whereas Peasant's
guards subtype1; interpreting every dead hive as subtype1 sends later worlds
to the wrong destination. Normal routes now depend exclusively on the
source-proven static marker stamp, with no model343 or subtype1 shortcut.
The port retains this branch as a distinct `FailedWorldRetry`, before the
normal record scan when aborted controller `+1F4` is set. `42DE20..42DE43`
returns current logical world `+C4`; the map/Space/native checkpoint path then
reloads that same overlay with no route-pair or exit-bit write. Failed Hive
death also suppresses both `4567B0`'s time stamp and its `2EE70` saved-world
transaction. Controlled normal-tier Level1 loops cover Main Base and casualty
loss through actual projectile Hive death, wreck entry and fresh reconstruction;
matched retail warp presentation remains open.

Retry XYZ is session `+38/+3C`, retained across player movement. `44FEB0`
initializes it from the default profile; `44F650/448E50` copy native state at
session `+14`, mapping payload `+24` to these XYZ words. A normal marker first
stages its exact incoming tuple through `42E270 ->448070`; map initializer
`4556A0` calls `443260` with output `session+38` to commit that tuple. Other
`443260` callers may choose different snapshot buffers. `456D10` reuses only
the retained XYZ and forces zero velocity/attitude plus heading4000. The port
keeps incoming XYZ independently from later `451C00` player-surface clearance:
fresh Level1 retains defaultY=-500 while its post-clearance body isY=-256.
That adjusted body position must not become the next retry's retained input.

The accepted `20260722-115649-level1-to-level2` capture distinguishes these
owners: the completion stamp becomes 4845 at 97.1 seconds, the session selects
logical world 2 at 103.9 seconds, overlay 51 appears at 105.5 seconds, and
Level 14 is loaded by 112 seconds. The earlier claim that overlay 51 opened
on hive death conflated the HUD statistics with this later resource change.

**Hive-component HUD, completion statistics, and progress map.** The first
two share `FUN_00452CB0`; the map belongs to a different mode descriptor.

In-world two-slot HUD, submitted by type-67 component `FUN_0041BEB0` (so any
world whose hive uses that callback, not a Peasant-only special case):

| When | Call | String |
|---|---|---|
| Premature shot while `FUN_00415120` is true | `FUN_004568B0(10)` | `0xEB` `The hive can only be destroyed once the alien creatures are dead` |
| Two-second empty grace completes | `FUN_00456900(0xD2)` then `FUN_004568B0(0xE)` | `0xD2` `The Hive is now vulnerable` on the direct slot, `0xEF` `It is now possible to destroy the alien hive` on the resource slot — two rows |
| Hive lethal / dying continuation | `FUN_004568B0(3)` | `0xE4` `Fly down the hive to go to the next world` |

Retail draws the direct slot then the resource slot (`FUN_00452CB0`), which
is why the unlock pair appears as two stacked rows. That callback subsequently
draws the staggered `0xBD..0xC4` statistics when `+0x2BC > 500`; these rows
retain the HUD/radar and the fly-down hint. Their formatters and counters are
documented under [Per-World Statistics](GAME_MECHANICS.md#per-world-statistics-confirmed).

The later progress-map descriptor `004D0AA0` has primary chain `004D07E8`
with `FUN_00454390`, which draws the map and global string 25 at Y=95.
Its init `FUN_00454EE0` loads overlay 51 and input table `004C2580`. Space
at `004C2434` resolves through `004D0620` to `FUN_00455CC0`: sound 3, then
session `+0x294=1`. `FUN_004555A0` processes that request before restoring
`004D0918`. S uses `FUN_00455CF0` and also writes `+0x28B=1` for the save
branch. The previous `004D1090`/`FUN_00456170`/`+0x297` interpretation
belongs to the distinct `004D0AD8` descriptor, initialized by `004557D0`;
it is not the ordinary progress-map Space owner.

The live port runs the controller prefix per authenticated Hive in every
world. The four type17 and three type47 actors in the accepted first-world
capture demonstrate that scene's predicate; they are not an admission
whitelist. The shared component locks health at `1,000,000,000`, then restores
the current type's authored Section12 `+0x14` health on state2. It retains the
signed Sub-N `+0x50` word, also used by Main Base abort cleanup:

```text
if no_objective_hostile || timer <= 0:
    if timer < 2_000_000: timer = wrapping_i32(timer + elapsed_us)
else:
    timer = 0
```

For ordinary frame deltas the timer alternates between zero and one delta
while a hostile remains. After the final hostile enters its dying slot, it
accumulates to two seconds. Retail checks that threshold after the add without
rechecking the predicate, so large single deltas preserve that behavior too.
Each Hive keeps its own timer and state; model overrides and multiple Hives
do not change this policy. Infection precedes this prefix, and radial emission
sees the newly written state on the same callback. The exact health, text,
abort-timer and state2 rules are owned by
[HIVE_WRECK.md](HIVE_WRECK.md#shared-live-health-prefix).

This does **not** skip overlay 51. The live projectile transaction now admits
the exact surviving Hive path while type 67 retains class-46 variant 0
(`0x004C94C8`) and fixed state `0x08000000`: the captured first vulnerable
primary hit therefore commits `2000 -> 200` health. A lethal hit is refused while `FUN_00415120` still reports a live
objective hostile. Once that field is empty, it commits the planned entity
writes (health 0, dying slot 3/model 343, class-46 variant 1), queues
`FUN_004568B0(3)` / `0xE4`, and displays `FUN_00452CB0` result statistics
over the live HUD/radar while the wreck surrounds the ordinary static exit.
The player's actual static-marker contact commits the route before the
separate campaign map opens; wreck-interior `456D10` is abort-only.
The remaining continuation is
still an atomic component/effect transaction, not a model-slot shortcut:
`FUN_00425790` clears task slots 2 and 1, then
`FUN_00425F60` installs callback `FUN_004260F0`, emits a surface-selected
effect through `FUN_00440950`, and calls complete radial-damage helper
`FUN_004566E0` with template `0x004C9748`.

The static lethal plan is now pinned in `hive_death.rs`. The captured live
slot-2/model-341/class-46-variant-0 allocation becomes
slot-3/model-343/class-46-variant-1 with health zero. The new component replaces
slot 0 only after allocation succeeds and ticks at `0x004260F0`. The shared
surface burst uses dying slot 3/model 343's unsigned resource extent word at
`+0x08` after the generic damage path sets state bit `0x4000`,
ten scatter attempts whose two particle class bytes are both `0x10`, the usual
class-18 above-sea tail (or class 45/46 underwater tail), and one randomized
sound 62; unlike the player-wreck caller it has no second fixed-rate sound.
The radial template is inner radius `0x200`, outer radius `0x400`, impulse
`200`, damage channels `[1,4]`, amounts `[10000,8000]`, followed by source
type 67 and the live Hive handle. These values are exposed as a pure plan for
an already-admitted Hive allocation; allocation identity and publication stay
with the external live transaction. The generic `FUN_00440950` presentation
helper is now reusable without the player-only cue.

The projectile owner now commits the planned entity writes: health 0, dying
slot 3/model 343, and class-46 variant 1, and keeps the shared `FUN_0041BEB0`
program on the wreck because dying-slot-0 `FUN_004260F0` calls it. The host then
emits shared `FUN_00440950` scatter/surface (no
player-wreck second sound) and `FUN_004566E0` static-then-dynamic radial with
template `0x004C9748`. Dying-slot-0 `FUN_004260F0` ramps the Sub-K bound u16
(`elapsed_us >> 8`, cap `0xD000`); live `FUN_00425EA0` writes that word from a
sine of Sub-K `+0x08`. Type 67 Sub-K `[1, 0]` binds that word through
`FUN_0040a950(entity, 1)` into the model-callback word bank
(`AnimVars.dynamic[1]`). Live `FUN_00425760` and dying `FUN_00425790` both
`FUN_00401020`-publish slot 0 after clearing slots 2 then 1. The
[hive wreck authority](HIVE_WRECK.md) owns the marker-backed Sub-N origin,
signed timer, exact suction gates/coefficients, separate ring presentation and
proven player-before-hive update order. Ordinary contact uses the decoded
subtype-1 Level-14 arrival when the player enters the wreck. The later map
`004D0AA0` binds Space through `004D0620` to `FUN_00455CC0` (sound 3,
`+0x294 = 1`, consumed by `FUN_004555A0`) to continue to that destination.
Map S enters `004D0AD8` and `PAUSE_SIMPLE` / `004D1090`; its Continue
uses `FUN_00456170` / `+0x297` through `FUN_004558A0`. Ring appearance
and interactive pull/exit remain explicit comparison conditions; in-world
result statistics and the later map have separate owners. Live fall-in yank
`FUN_0041c830` / pair-restore
`FUN_0041ca90` stay fail-closed.

The ordinary route is not safe to enable from a counter alone.
`FUN_00410EB0` always calls type-vtable `+0x14` before
`FUN_00411030` and health mutation, but the resulting current-style `+0x28`
call is conditional: observed Wander/Follow/Guard and Capture variant-1 styles
use `FUN_0040C690`, while Run Away variant 1 and initial Attract Attention are
null; Capture variants 2--5 use distinct cleanup `FUN_0040D040`. A present
callback can reselect behavior and consume shared RNG before the three-draw
impact-reaction branch. Lethal `FUN_00410C10` enters `FUN_0040DB80`, which
invokes the post-impact style's `+0x2C` cleanup before the standard
alternate-behavior continuation. Capture cleanup `FUN_0040D040` can itself
re-enter selection while dying, initialize class 12 before revalidation, and
then reach a second standard initialization. The inert Rust planner rejects
that nested branch rather than representing it as the single-install order.

That Capture-People branch remains a focused runtime oracle, not permission to
generalize the ordinary route. One controlled lethal hit must join an exact
fresh-Level-1 type-17 actor in Capture People variant 2--5 across the
`FUN_0040DAC0` impact callback, `FUN_0040D040` death cleanup, every nested
`FUN_0040C690` selection and `FUN_0040C620` class-12 initialization, and the
outer `FUN_0040DB80` continuation. The capture must retain callback order,
RNG entry/exit state, context/style, all three task identities, component and
attachment/relation state, health/model-selector/state words, and the final
surviving class-12 owner. Acceptance requires proving whether the first
initialization survives, is replaced, or is followed by a second publication;
a passive end-state sample cannot answer that question.

Detailed primary-hit wrapper, impact-reaction, standard-death, and
Common-Dying contracts now live in
[ENTITY_DAMAGE_AND_DEATH.md](ENTITY_DAMAGE_AND_DEATH.md). That document retains
the exact callback/RNG/mutation order and static/runtime evidence; current work
and capture routing remain in `OBJECTIVE.md` and the runtime-RE ledger.

### Fade/Transition Effects

**Screen fade engine** (`FUN_0043c7f0`): Same function as menu transition tick. `_DAT_004cb4e0` counts down from `0x7000`, `DAT_004dceb0` accumulates fade value.

`DAT_004DB210` is the frontend transition-command state documented under
Menu-to-Game Transition, not a campaign completion/result state machine. In
particular, command 4 is used by the main-ring submenu transition and command
5 by both manual New Game and idle attract; the former next-level/results/
score/victory/defeat labels were unsupported. Gameplay warp uses the separate
entry below.

**Warp/enter level** (`FUN_00456d10`, 0x456D10): Sets camera to warp transition params (`local_8 = 0x4000` = full-speed warp, transition type 15).

### Exception Codes

| Code | Meaning |
|------|---------|
| 0xC00 | Clean cancel (menu returns) |
| 0xC01 | Game complete → `_DAT_004db224 = 6`, pop menu |
| 0xC02 | Game failed → `_DAT_004db224 = 7`, pop menu |
| 0xA403 | Game over → push `PTR_DAT_004c19e0` |
| 0xA404 | Save complete → replace with `PTR_DAT_004c1900` |

---

