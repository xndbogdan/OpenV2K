# V2000 VTOL Flight Model (FLY `FUN_00445310`)
Verbatim move of PLAYER_CRAFT.md's FLY integrator section during the
2026-08-24 cohesion split. Mode state machine, input channels, Hover, fire,
contact damage, and the model hierarchy remain in PLAYER_CRAFT.md.

### FLY `FUN_00445310` (VTOL/heli) — game_logic.c:34932

The 2026-07-16 mode trace shows that the common environment tail remains live
in style 1: the entity retains Section-12 bit 3 (`entity+0xC8 = 0x8`) after the
hover behavior's extra `0x10010` mask disappears. Common gravity, underwater
response, exact no-wind signed fixed-point drag, and signed-8.8 position
integration now apply to both styles in callback order. The removed VTOL float
gravity/friction/damping and float position update had duplicated that tail.
The specialized thrust/height-assist force phase is now ported. The normal
player branch's pitch/self-right recurrence and type-46 Sub-D yaw/bank path are
also exact. The alternate terrain-derived attitude branch now has an exact
detached arithmetic plan, but remains deliberately disconnected because no
accepted player trace identifies a live type-46 transition into it.

- **Barrel joint locked**: `*barrel -= dt>>6`, floored at 0
  (game_logic.c:34977-34982) — decays to level every frame, input never
  reaches it. This is the "guns locked in heli" the player perceives.
  (The **trigger** still works — fire is mode-independent.) The port now uses
  this native integer step directly; it also preserves the retail asymmetry
  where a negative hover aim snaps to zero on the first fly update.
- **pitch channel → body-pitch rate**:
  `rate = ((dt*0xD)/physics[0x14]) * channel >> 15` (:35111), applied to body
  pitch `+0xA4` by `FUN_0041A690` (:11620). Nose-down tilt → forward flight.
  The port applies this independent signed-i16 contribution exactly with type
  46's authored divisor 28, then runs the live Self Righting mode's integer
  correction every frame. The normal player callback writes zero to Sub-G
  `+0x1C` immediately before the call, disabling the earlier terrain-derived
  target phase; that target remains relevant only to alternate callers/state.
- **LEFT/RIGHT → shared yaw/bank step**: type-46 Sub-D computes
  `rate=trunc0(turn*52/28)` and
  `step=(rate*(global_dt_us>>2))>>15`, then subtracts that signed word from
  heading and roll. The near-surface correction consumes this post-Sub-D,
  pre-A690 roll; `FUN_0041A690` subsequently applies exactly one
  `roll -= ((dispatch_dt_us>>10)*roll)>>8` damping pass. All 53 clean sustained
  steering transitions in the July 17 trace replay exactly. Partial key-edge
  intervals remain subject to the input reader's sub-frame duty cycle.
- **Rotor node ∝ throttle/ascent**: `[ctrl+0x18]+0x20/+0x24` written from the
  ascent/throttle channel (:35104-35105); turbo pickup bit `+0x197&2` gives a
  persistent **1.5×** boost and sets motion-blur flag `+0x3d` (:35096-35102).
  The port now acquires selector `0x3E` into that campaign-owned bit and maps
  it to the existing `VtolBoost::Active` force/height policy. The callback's
  model-side blur consumer remains unimplemented rather than guessed.
- SPACE/RSHIFT author signed Q16 throttle and therefore +19/−19 manual lift.
  Both directions burn the absolute pre-near-correction total through
  `FUN_00445A40`; hover-height PD P=24/1000 D=5/1000 uses the strict altitude
  window [−299, 999]. Positive SPACE, but not reverse RSHIFT, also enables the
  bounded `-body_pitch/6` powered-pitch coupling when Self Righting is above 1;
  either S/X fine-pitch key suppresses that coupling without suppressing its
  own pitch input. Retail gates this on the dedicated SPACE binding, not the
  net signed throttle, so SPACE+RSHIFT can retain the coupling at zero net lift.

#### Exact data/runtime linkage (2026-07-16 audit)

`FUN_00446640` resolves the fly configuration through the entity's type index:

```text
entity +0x58 -> g_resource_table[type] -> type record +0xC8
```

The pointer at `+0xC8` is `FUN_00445310::param_3`; its pointers at `+0xD0`,
`+0xD4`, `+0xD8`, `+0xDC`, `+0xE0`, `+0xE4`, `+0xE8`, `+0xF4`, and `+0x10C`
are Section-12 Sub-A/B/C/D/E/F/G/J/O respectively. Runtime component state is
separate: `entity+0x4C -> root`, `root+0x0C -> component table`, and table
`+0x18` is the 0x44-byte Sub-G runtime block allocated by `FUN_0041B8C0` from
the static Sub-G record. Do not mutate the retained Section-12 bytes while
porting this state.

Type 46 has identical relevant data in all four display/resource overlays: Sub-D
divisor 28; Sub-F absent; Sub-J count 5; Sub-O bytes `05 06 07 08`; environment
mask 8; and model slots `[41,67,41,67]`. Its 104-byte Sub-G starts with
`10,100,0,0,600,700,500,0x4000,1500`, followed by seven null joint references
and five zeroed 12-byte animation channels. The current Rust parser already
retains every required byte, model collision-radius word, and attachment-mass
record; this needs typed offset-named decoding, not an extractor change.

#### Recovered update order

The normal player path performs these phases in order:

1. Sub-D steering uses global microsecond delta `DAT_004D04E4` to compute the
   shared signed-word step, then performs `heading -= step` and
   `roll_pre_A690 = roll - step` through `FUN_00420360`.
2. Clear Sub-O runtime `+0x10/+0x14`, then decay the barrel by
   `dispatch_dt_us >> 6` (312 raw angle units at 20 ms).
3. Apply the fuel/Sub-G gate. A callback that starts empty has already steered
   and decayed its barrel, but requests Hover and skips lift, manual pitch,
   Self Righting, and A690 roll damping.
4. Sample the exact terrain/wave ride surface and active-model collision
   height, then apply the strict altitude-error window `[-299,999]`.
5. Compute manual thrust `(motion.throttle * 19) >> 16`, add the non-negative
   P=24/1000, D=5/1000 assist, and burn fuel with `FUN_00445A40`'s original
   Q31 multiply/shift order.
6. Apply the near-surface Q31 trigonometric correction inside `(-300,800)`
   using `roll_pre_A690`,
   populate Sub-G runtime thrust/pitch fields, and derive pitch delta as
   `i16((((dt_us * 13) / 28) * motion.pitch) >> 15)` (`+652/-653` at the
   captured 20 ms `+2303/-2303` input).
7. `FUN_0041A690` adds the manual pitch delta, then applies the live Self
   Righting byte from Sub-G `+0x41`: 0 leaves pitch unchanged, 1 damps toward
   zero, and 2..15 asymmetrically seek `((mode-1)*0x2800)/15`. It also derives
   the authored 600..700 lift state, applies the 1500 attitude-velocity limit,
   and damps the paired attitude word. The normal player callback has cleared
   Sub-G `+0x1C`, so its preceding terrain-derived target step is inactive. It
   also damps `roll_pre_A690` exactly once with
   `roll -= ((dispatch_dt_us>>10)*roll)>>8`.
8. `FUN_0041B210` projects lift through the **previous-frame** Q31 body-up
   basis using total self+Sub-J mass. After the behavior callback, retail
   rebuilds the body basis, applies common gravity/underwater response and
   common wind/no-wind drag, then integrates signed 8.8 positions.

The fly ceiling is an emergent force balance, not a position clamp.
`FUN_0041B210`'s player branch (`entity+0x64 & 1`) samples the animated ride
surface at the craft center and at X/Z offsets `±0x100`, then deliberately
uses the **minimum** of those five heights. It measures the model underside
against that surface. Unless the signed session mode byte is 4, clearance
above `0x800` raw units
(`0xC00` while runtime turbo flag `Sub-G+0x3D` is set) divides the manual
`Sub-G+0x20` lift by
`max(1, trunc_toward_zero((clearance-threshold)/128))`. Upward velocity always
adds a second divisor, `max(1, trunc_toward_zero(vy/512))`. The separate
altitude-assist component at `Sub-G+0x24` has already faded to zero by 1000 raw
clearance, after which the attenuated manual lift and gravity establish the
observed maximum height. Fuel `controller+0x88`, turbo capability
`controller+0x197`,
runtime turbo state, and both surface policies must therefore accompany any
ceiling trace; fuel depletion alone can otherwise look like a height limit.

The clean `20260717-014227-vtol-ceiling-traverse-trimmed` trace closes the
operator-observation side of this rule. From the authoritative Space press the
five-probe underside clearance crossed `0x800` after 2.01 seconds and reached
its first apex after 3.48 seconds at 3025 raw. Its settled open-water median was
2372 raw (mean 2386.5), with no cargo, turbo, session-mode bypass, or fuel
exhaustion. Because integer division still returns one at `0x801..0x8FF`, the
first actual height reduction is `/2` at `0x900`, then `/3` at `0x980` and `/4`
at `0xA00`. When terrain replaced the animated wave under the probes, clearance
fell and lift responded within half a second before overshooting again; there
is no separate terrain ceiling and no position snap.

The earlier ceiling runs remain useful, but they are not interchangeable with
that authority. `20260717-004249-vtol-ceiling-traverse` is the original
long-duration qualitative calibration. `20260717-010226-vtol-ceiling-traverse`
contains valid early acceleration/height-loss intervals but ends in a reported
tree strike. `20260717-012746-vtol-ceiling-traverse-braked` uniquely isolates
the Space+Up to Space+Down active-braking transition, while still assuming that
Space alone keeps the naturally nose-down craft level. The corrected
`20260717-014227-vtol-ceiling-traverse-trimmed` protocol permits operator trim
and is therefore the sole numeric ceiling and mixed-surface authority. All four
captures are analyzed; none needs repeating.

Water contact intentionally uses two different surfaces. Fly/skimmer ride
clearance follows the animated wave, while common `FUN_0040E100` buoyancy uses
`max(integer-cell terrain, static sea)`. Its depth branch starts only when the
entity center is strictly below `-100`; the actual underwater response also
requires effective environment bit `0x4` to be clear and the nested
water-capability byte to be nonzero. Hover's wave controller is force-based
and does not snap wave penetration back to the surface, so a high skimmer drop
may enter the visible water briefly before recovery. Adding a post-integration
water clamp would erase retail behavior.

The guided
`20260717-033821-gameplay-contacts-fuel-impacts.jsonl` session closes the
high-speed VTOL water case as well. RShift begins at **139650.5 ms / tick
9082**. Downward velocity reaches **-3435** raw at 141200.4 ms while the craft
is still above the water response surface. The strict common-depth gate first
crosses below -100 at **141540.4 ms / tick 9176**, the fully-submerged flag is
set at 141780.3 ms, and the center bottoms at **-401 raw depth** at 142030.2 ms.
Velocity has already reversed upward at 142040.3 ms, while RShift remains held
until 142450.3 ms. The fully-submerged flag clears 20 ms after that release,
the common-depth gate clears at 142690.1 ms, and the fully-above-water flag
returns at 142930.5 ms. Health, the pre-health buffer, and the last-damage tick
remain `40000/0/0` throughout. This proves that recovery is the common
depth-proportional buoyancy response, not an input-release effect, a surface
snap, or collision damage.

The same session records three other bounded contact results:

- The tracked kind-4 fuel model (cell `[89,56]`, model 140) changes fuel
  `0 -> 100000` and clears its Section-10 cell in the same stable tick 4544.
  The first Tab edge at 106560.5 ms begins the VTOL force/fuel state and the
  second at 157310.1 ms ends it at fuel 92542; fuel then stays exactly 92542
  throughout the intervening skimmer drive until the next Tab edge at
  202020.2 ms. The sampled `controller+0x20D` request remains zero because the
  request is consumed between 100-Hz samples; it is not a persistent mode bit.
- Two nonlethal tree-model responses are visible at ticks 10865 and 10934
  against kind-0/model-444 Section-10 candidates. Both produce abrupt velocity
  changes, but health/buffer/last-hit remain `40000/0/0`. The final guided drop
  is likewise nonlethal: downward velocity reaches -3398 raw, then a static
  response at tick 14220 changes the adjacent sampled velocity from
  `[420,-3076,414]` to `[687,-824,1503]` without damage. That endpoint is not
  an isolated terrain strike: the retail scan contains kind-2, kind-3, and
  kind-9 static models, and ride clearance is still +63 raw. It must not be
  used as evidence for terrain-height collision damage.
- The initial Main Base approach is a separate active-entity collision. With
  type 6/model 286 nearest, tick 3240 changes player velocity from
  `[156,19,1713]` to `[199,651,1333]`; tick 3244 changes it again to
  `[398,702,165]`, and tick 3290 supplies a later vertical response. Health is
  unchanged. Retail routes this through `FUN_00411AD0`'s live-entity scan,
  `FUN_00412530`, the two-body mass/flag response, and `FUN_00412760`, not the
  Section-10 `FUN_00427100`/`FUN_00412CF0` path. Checkpoint `19c2402d`
  subsequently wired only the audited active Main Base, Working Factory, and
  weight pairs through the recovered two-body response. Other active actors
  remain outside that deliberately bounded policy; reusing the one-body static
  response for them would still be observably wrong.

The later guided
`20260719-123523-vtol-collision-survey.jsonl` supplies a cleaner acceptance
matrix with complete target basis/radii/mass. Every one of its 22,834 samples
is tick- and topology-stable. Up-driven seabed responses occur at ticks 2625
and 3083, while tick 2841 is a water-only comparator; the clean Up-driven dry
terrain response is tick 6184. These clean surfaces respond while the reported
craft-bottom clearance is still positive by 94..106 raw, and none changes
health/buffer/last-hit from `40000/0/0`. The neutral intended tree response at
tick 9060 is kind 0/model 444/cell `[151,162]`, changing velocity from
`[476,-556,744]` to `[-60,255,-473]` without mutating the cell. A separate
neutral kind-9 contact at tick 9460 changes model 518/state 65 to model
519/state 73 on the following tick. The cleanest Main Base impulse is tick
12010: player velocity changes by `[+108,+475,-347]` while its craft bottom is
1029 raw above terrain, no static candidate is present, and the fixed type-6
model-286 target lies inside the 248+1100 broad-radius gate. The target's exact
position, Q31 basis, rotation, mass 1000, and model resources remain in every
nearby record. Finally, player state bit `0x00800000` changes only at the first
seabed contact and stays set for the remaining 207.8 seconds; it cannot delimit
later contacts for type 46.

The port now also runs type 46's live, oriented `player4` collision program
against the triangulated bare terrain/seabed before whole-body water
classification. This replaces the insufficient center-minus-half-radius test:
the accepted tick-6184 pose still has positive scalar clearance while one of
the eight authored, pitch-rotated spheres contacts the ground. The physical
response matches `FUN_004141D0(..., 0x7fffffff, 2, 1)` and keeps the water plane
penetrable. Static table recovery now identifies type-vtable `+0x0C` as the
common `FUN_0040D7F0` trampoline. It invokes the current behavior-style `+0x10`
callback when present, re-resolves the entity, and unconditionally runs that
`FUN_004141D0` tail if the entity survives. Type 46 begins in Player Control
Hover style `0x004CD940`; VTOL uses the adjacent `0x004CD988` record. Both
styles bind `+0x10 = FUN_00448280`, which computes inward speed for force
feedback and can submit a separate material-class-7 damage request; it does
not clamp pitch, alter lift, or suppress the common response. That style suffix
is now live too. It authenticates the current controller, class-24 descriptor,
and actual Hover/VTOL mode, then projects the pre-response velocity onto the Q12 contact
normal with wrapping 32-bit accumulation, and enters feedback only when the
inward speed is strictly greater than `0x200`. The port has no force-feedback
backend, but retains the exact staged strength
`min(0xFF, vibration * (inward - 0x200) / 15)` as an unsupported-presentation
result rather than suppressing later gameplay.

The same callback samples the nearest wrapped X-major terrain cell with
`(u16(position) + 0x80) >> 8`, maps `terrain_type & 7` through the active
level's ground-response selector table, and submits exact delivery
`[3, 0, 15000, 0, 0xFFFF_FFFD, 0]` only for selector 7. Type 46 filters that
to 30,000 raw damage. Material damage precedes the common physical/collision
tail, and that tail still runs if the material hit makes the hull dying.
Retail and demo callback bodies match apart from relocated addresses and the
known controller-layout shift; no gameplay capture remains for this suffix.
Non-player activation stays blocked until their outer solid-terrain order,
type-record sound, and style callback can be represented.

#### Active-entity pair contact contract and implementation boundary

`FUN_00411AD0`, `FUN_00412530`, and `FUN_00412760` now give a complete
arithmetic decision matrix for the physical pair response. They do **not** yet
give the port permission to scan `EntityManager::iter_collidable()` and apply
that response to every returned object: the retail scan consumes runtime state
and ordering which are not all recovered. The port now retains constructor
health/buffer/profile data and an explicit current model slot, while callback-
owned fields remain marked unresolved instead of being guessed.

| Phase | Exact retail decision |
|---|---|
| Subject eligibility | state `& 0x8000 != 0`, state `& 0x1000 == 0`, entity `+0x70 == 0`, and the model selected by state bits `0x2000/0x4000` has nonzero Section-8 collision radius `+0x0A` |
| Pair order | after the deepest Section-10 response, walk the subject's intrusive live-neighbour chain from `entity+0x00`; a frame may resolve more than one candidate, in that chain order |
| Candidate eligibility | candidate state `& 0x8000 != 0`, state `& 0x1000 == 0`, not both entities carrying bit `0x80000000`, and no recent-relation suppression |
| Recent-relation suppression | `FUN_00411A20` rejects while either entity's `+0x60` names the other and that same entity's `+0x68 < 750000`; the result becomes false exactly at 750000, and this is not a generic cargo-parent test |
| Broad phase | select the candidate's live model slot, reject radius zero, subtract all three positions with signed-word wrapping, then test the radius-sum AABB and squared sphere |
| Narrow phase | run the subject model program first with the candidate nested beneath it; `FUN_0046AE40` returns Q12 normal words at `+4/+6/+8` and signed penetration at `+0x0C` |
| Pre-response effects | subject Section-12 type record `+0x8E/+0x8C` may emit capability-gated sounds; retail then calls the subject behavior-style `+0x18`, interprets that return immediately, conditionally calls the candidate behavior, and finally runs the subject and candidate component-contact chains |
| Behavior returns | `0xA300` is dispatched immediately, normalized to null, and clears physical response, so a tagged subject result still permits the candidate behavior. A retained non-tagged subject result suppresses the candidate behavior; with no earlier A300 it survives both component chains and the response/damage suffix and is returned afterward. Component returns are ignored, but their side effects are not |
| Damage | sum the two mass-weighted velocity-change energies, clamp the shared channel-1 raw amount against both live health values through `FUN_00414D30`, then deliver that same remaining packet in both directions unless the opposite entity carries `0x80000000` |

The response itself has three branches. Let `n` be the Q12 contact normal,
`p` the penetration, `m_s/m_t` the unsigned runtime masses, and `v_s/v_t`
the signed-word velocities:

- If both entities are movable, each axis first computes the truncating signed
  centre velocity `c = (m_s*v_s + m_t*v_t) / (m_s+m_t)`, then writes
  `v_s' = c - (v_s >> 1)` and `v_t' = c - (v_t >> 1)`, where `>>` is the
  executable's arithmetic shift. Separation is mass weighted:
  the subject receives `m_t*p/(m_s+m_t)` along `+n`, while the target receives
  `m_s*p/(m_s+m_t)` along `-n`.
- If only the target has state bit `0x08000000`, the target remains unchanged.
  The subject removes its complete normal projection and then restores the
  executable's narrowed one-eighth projection; the subject receives all of
  `p` along `+n`.
- If the subject has `0x08000000`, the symmetric projection response is applied
  to the target and the target receives all of `p` along `-n`. This is also the
  executable's fall-through when both are fixed; do not invent a fourth
  no-op branch.

Every multiply/shift and final word write narrows in the same order as the
executable. One particularly important asymmetry is that active-pair damage
widens each old/new signed velocity word before subtracting it. It is **not**
the wrapping-word delta used by one-body `FUN_00411760`. For each body the
active-pair term is
`((((new_x-old_x)^2 + (new_y-old_y)^2 + (new_z-old_z)^2) >> 7) * mass) >> 10`,
with signed 32-bit wrapping between operations. `FUN_00414D30` then filters the
shared channel-1 amount against the subject; if filtered damage exceeds
`health+1`, it subtracts `(excess << 8) / channel_multiplier` from the shared
raw amount, and repeats for the target before clamping the result to zero.
The Rust damage layer now owns this as a pure ordered transition, including the
observable truncation difference when the two targets are swapped. It also
owns the following `FUN_00414E90` buffer/health arithmetic without claiming the
surrounding callbacks: positive `+0x50` is drained before the dying check, live
targets request generic hit sound `+0x98` and their hit callback even when the
remainder is zero, and wrapping health below one reports a required death
dispatch without clamping or generic removal. Entity runtime state now retains
that selector, death sound `+0x90`, accepted-hit sound `+0x80`, and the
per-entity `+0x34` presentation tick as distinct fields. Its pure
`FUN_00410D30` transition uses the exact unsigned `last_tick < tick - 10`
comparison, updates `+0x34` before its final dying/sound gate, and therefore
does not collapse the three audio paths into a contact-time cue.
Class-1 primary bullets use the separate `FUN_00410EB0` wrapper: it stamps
`+0x34` before checked delivery and has no ten-tick gate on accepted sound
`+0x80`. Main Base/Working Factory have a null effective behavior-impact slot
and fixed-state impulse suppression, so their surviving bullet path is now
wired without generalizing that policy to other entity types or lethal hits.
The F590 handler now invokes a synchronous receipt-journaled type-17 owner before
that Base/Factory fallback. Against an already-published exact context it
unit-proves the ordinary nonlethal and lethal routes through reselection,
reaction, checked damage, class-12 publication, selector-4 notification,
authored sound, and class 5. Fresh-New-Game Level-1 construction now selects
and publishes the fallible context/task graph before each authored type-17
actor joins the live list; their constructor state remains exact `0x07468805`.
This bounded seam is not a type-wide damage policy and does not
enable Capture People variants 2--5 or any lethal callback family beyond the
exact standard null-hook route.

The first-world records bound the desired initial subset tightly:

| Type | Live model / radius | Captured mass / health | Captured fixed bit | Channel-1 filter | Current style release `+0x0C` / pair `+0x18` / death `+0x2C` |
|---:|---:|---:|---:|---:|---:|
| 46 player | 41 / 248 | 100 / 40000 | clear | threshold 6000, Q8 multiplier 256 | `0 / 0x00447D70 / 0` |
| 6 Main Base | 286 / 1100 | 1000 / 99999 | set | threshold 2000, Q8 multiplier 256 | `0 / 0x004258A0 / 0x00419750` |
| 66 lifter | 227 / 1248 | 1000 / 99999 | set | threshold 2000, Q8 multiplier 256 | `0 / 0x00425850 / 0x00419750` |
| 68 weight | 81 / 110 | 200 / 50000 | set while loose | multiplier zero in every channel | `0x0040D1C0 / 0 / 0` |

Type 46's Section-12 type record authors zero in all four contact-sound words
`+0x88..+0x8F`, so the player-subject path adds no pair-contact sound before
those callbacks. The Rust Section-8 interpreter already supports the collision
programs needed by
models 41, 286, 227 (including child 179), and 81. Geometry is therefore no
longer the integration blocker.

**Do not implement the pair scan yet.** The collision-state foundation now
retains Section-12 initial health, damage profile, mass/capabilities/model slots,
Section-13's `param_2[9]` damage buffer, and the initializer payload. Type 46
mirrors its mutable hull/buffer values into that record. The raw `+0x08` state
is represented by value plus known mask: creation-tick-dependent surface bits
`0x00600000` remain unknown for general/direct construction, while the exact
fresh post-Intro Level-1 type-17 cohort is statically closed at `0x07468805`.
Eligibility, fixedness, scheduler, and the authoritative model selector remain
independently readable. Dying model
selection writes those selector bits and the compatibility draw view together.
Constructor `+0x60` is the null relation sentinel; `+0x68/+0x6C/+0x70` and byte
`+0xB6` remain exact zero through every reachable initializer; malloc-owned
`+0xB2` remains unresolved until the scheduler clears it. Mutable neighbour
order is also owned by `EntityManager`. Exact retail `FUN_00422C10` / demo
`FUN_00422AE0` prove that the bounded type-17 candidate walk reads only
`state != 0 && (state & 0x5000) == 0`. It now consumes the real
`RetailStateWord`, admits independently proven predicates despite unknown
surface bits, and fails closed only when one of those exact reads is ambiguous.
What still gates broad pair dispatch is the
shared RNG/environment evaluation for weighted behavior types, the side effects
of the now-identified behavior and component pair callbacks, and mutable
non-player damage/death—not a first-world type whitelist.

The remaining death boundary is concrete. Checked and falloff delivery may run
an entity `+0x44` modifier before generic arithmetic: type 46 redirects and
propagates through player links, types 73/109 can own lethal effects and death,
and type 110 gates through component/source state. Default type death reaches
the current behavior style's `+0x2C` slot before the standard continuation.
Main Base and Working Factory use `FUN_00419750`, which can restore health to
10,000,000 and clear dying; Capture People variants 2--5 use
`FUN_0040D040` for component cleanup. Therefore a generic “health <= 0 means
remove” rule would corrupt both ordinary gameplay and the first-world base.
The ordinary fresh-Level-1 type-17 null-hook owner/journal is implemented for
the constructor-published exact context. Its birth owner evaluates only the
already-linked prefix, consumes one selector word and two constructor words on
full success, and publishes the selected initializer or proven fallback before
linkage. The authenticated production Common-Dying scheduler now runs its
detailed/coarse post-task suffix, retains all nine Q31 matrix dwords, and feeds
that matrix to ordinary Type-17 draw and particle/model collision. This does
not authorize the general pair solver. Capture People's nested cleanup is
different:
its impact-selected variant-zero `FUN_0040B6C0` publication is now implemented,
including the persistent type-filter reset, distinct `0x0C00` acquisition
override, ordered constructor RNG/effects, and outer fallback. Its exercised
variant-2--5 callback/selection/class-12 order remains a focused exact-handle
capture request, while the separate early-unlink writer remains a second
runtime oracle.

The two natural broad histories already supply the birth acceptance oracle.
In spawn order 17--20, `20260712-203635` records Capture, Follow, Follow,
Capture, while `20260717-032945` records Follow, Capture, Follow, Capture; all
eight actors begin at state `0x07468805`. The differing patterns are expected
because the selector consumes the process-global RNG after all earlier session
draws. They must not be hardcoded per spawn, and no new birth capture is needed.

The existing captures also retain adjacent lifecycle fields without making
them constructor constants. In
`20260717-032945-menu-intro2-level1.jsonl`, the first observed Level-1 list at
tick 53 is already past every type initializer and the level-handoff state
edit: type 46 has state `0x06438805`, type 68 has `0x0E408805`, and types 6 and
66 have `0x0E428805`; all four have zero at `+0x60`, `+0x68`, and `+0x70`.
Type 46 construction itself retains `0x00040000`; a later handoff/attachment
path clears it, and the first ordinary gameplay update restores it four ticks
later (`0x06478805`). Later stationary snapshots show the expected scheduler
changes at `+0x68/+0x70`. These are lifecycle observations rather than
constructor constants. The July 16 entity snapshots also preserve the actual
intrusive list through each record's `next` pointer: the relevant records occur
at list indices 2 (player), 9 (weight), 10 (Main Base), and 27 (factory), with
unrelated live entities between them. Sorting candidates by type, distance, or
an unowned allocation container would not reproduce that order.

The long gameplay/contact captures expanded only the type-46 component chain.
Its sole live component state has zero at pair-contact callback/context
`+0x18/+0x1C`; types 6, 66, and 68 were intentionally serialized only as
compact nearby records, so their component callbacks cannot be inferred from
those files. Run the following while idle in the first playable Level-1 world
to close precisely that remaining read-only gap:

```powershell
cd <private-re>\runtime_re
.\scripts\capture-pair-collision-boundary.ps1
```

Leave the craft in skimmer mode near the starting base and do not move, pause,
collect the weight, or change modes during the roughly two-second capture. It
writes one complete intrusive-chain snapshot plus short type-46/6/66/68 tracks.
Each track records the exact live entity state/relation/gate fields and all
three component-state callback/context dwords at `+0x18/+0x1C`; the inspector
never invokes them.

The resulting `20260718-024518-pair-collision-boundary-*` set is stable and
closes that requested callback question exactly:

- the 39-record intrusive chain is stable across its duplicate tick/topology
  reads, with player/type 46 at index 2, weight/type 68 at 9, Main Base/type 6
  at 10, and lifter/type 66 at 27;
- all four tracked entities select model slot zero, are pair eligible, have a
  zero recent-relation handle, a relation timer already above 750,000 us, and a
  zero subject scheduler gate; types 6/66/68 carry the fixed-response bit while
  type 46 does not;
- their effective words are `0x06478825` (46), `0x0E408805` (68), and
  `0x0EC28805` (6/66); these are initializer/update results, not constructor
  constants; and
- each type has one live component state, whose pair-contact callback and
  context are both zero in every sample. The other two component slots are
  absent. This does not mean type-vtable `+0x38` is null: it is the common
  `FUN_0040D8D0` trampoline into current behavior-style `+0x18`. Player, Main
  Base, and lifter therefore dispatch `FUN_00447D70`, `FUN_004258A0`, and
  `FUN_00425850` respectively during a pair contact. The weight's current
  style has no pair callback; its class-0 `FUN_0040D1C0` lives at style
  `+0x0C` and runs only through the separate relation-release dispatcher
  `FUN_0040DC50`. Type-46's Section-12 contact-sound words remain zero.

That first snapshot did **not** close the no-whitelist implementation boundary.
It also contained pair-eligible instances of types 9, 17, 47, 54, 61, 62, and
67 in addition to 6/46/66/68. Their component callback state had not yet been
expanded, and their initializer-produced eligibility/fixed/model-slot state had
no general Rust lifecycle owner. The now-explicit chain remains mutable:
collection, attachment, destruction, and spawn/remove callbacks can change
both state and neighbour order after this idle sample. Finally, pair damage is
delivered to both bodies, so a high-energy contact can enter an unported
non-player damage or death path even when the physical response itself is
known. Treating the captured flags/order as a level-13 constant would therefore be a
port-only collision policy. Retail still walks every live pair. The player-solid
adapter omits identities whose constructor or census pair fields are unresolved
instead of aborting the whole pass, so constructor-closed solids and the Level-1
census still resolve.

The focused follow-up `capture-pair-runtime-census.ps1` completed successfully
as `20260719-140359-pair-runtime-census/`. Its manifest covers all 21 exact
handles, with five valid samples per handle except one type-47 set with four,
and confirms that the complete intrusive order stayed unchanged from tick 266
through 509. All 21 are pair eligible, relation-clear, and have null entity
`+0x44` damage modifiers, null type-vtable `+0x30` hit callbacks, and null linked
damage states. Types 9/17/47/62 use component pair callback `FUN_00402DA0` in
slot zero; types 54/61/67 have no component pair callback. Type-8 scientists
are not in that idle census; their constructor-installed Go-To-Job
(`FUN_00403650`) and Wander Near (`FUN_00402E20`) tasks write the same
`FUN_00402DA0` pair slot as Attract Attention's shared target-route task,
which uses the identical `FUN_00403650` constructor. Current behavior
pair callbacks at style `+0x18` are non-null only for three type-17 instances
(`FUN_0040C910`), all three type-61 instances (`FUN_00425AF0`), and type 67
(`FUN_004259F0`). Types 9, 47, 54, and 62 have no behavior pair callback in
this neutral sample. The fourth type-17 instance uses class-33 Follow Beacons
variant 1 (`0x004C7B70`) and also has a null pair slot.

The port now retains the allocation-specific callback evidence for authored
spawn indices 17--20 only: type 17/model 256 owns descriptor contact
`FUN_00402DA0` in component slot 0, absent component slots 1/2, and null
instance damage-modifier/type-hit callbacks. The census does not authenticate a
stable pitch/roll basis for ordinary pre-Common-Dying pair dispatch, so that
pair orientation remains unresolved and this evidence alone does not admit
them to the physical pass. The death scheduler's later post-task matrix is a
narrow draw/particle-collision exception.

The successful directory uses the inspector's original v1 schema. Its keys
named style `+0x0C` as pair, style `+0x18` as tick, and entity `+0x88..+0x8E` as
contact sounds. Static call-site audit disproved all three labels: `+0x18` is
the behavior pair slot, `+0x0C` is relation release, and the sound words belong to
the Section-12 type record. Preserve the v1 raw windows as evidence, but do not
consume those semantic key names. Its 0x3C-byte type-vtable window also stops
before `+0x48`; the link/release dispatcher is established statically, not by
this directory. The inspector and census script now emit a corrected v2 schema,
extend that window through `+0x48`, and read the sounds from their real owner.

The retail/demo dispatcher differential is now closed. Retail
`FUN_00411AD0` and demo `FUN_00411A60` are behaviorally identical; the demo's
eight additional bytes are distributed register/spill codegen rather than a
branch or policy change. Their outer drivers, retail `FUN_00411A80` and demo
`FUN_00411A10`, are behaviorally identical as well. Both retain
subject-authored, candidate-capability-gated contact sounds -> subject behavior
-> immediate `0xA300` handling -> conditional candidate behavior -> subject
component slots 0--2 -> candidate component slots 0--2 -> physical
response/shared damage. Component returns remain ignored, but component
mutations feed every later stage. No demo-only callback order or collision
policy belongs in the port.

The callback programs are not optional notifications. Their relevant Level-1
semantics are now statically bounded:

- `FUN_0040C910` is the class-9 Capture People contact transition. It requires
  the contacted entity's capability `0xC00`, checks the captor's attachment
  capacity, either kills the excess target or attaches it with `FUN_00416700`,
  advances behavior, and normally returns tagged object `0x004BE328`.
- `FUN_00425AF0` is Power Up contact. It dispatches the recipient type-vtable
  `+0x2C` / current style `+0x30` inventory handler with the pickup id/amount,
  emits feedback and deferred-destroys the pickup when accepted, and normally
  returns `0x004BEAA8` or `0x004BEAB0`.
- All three return objects above contain tag `0xA300`. `FUN_00411AD0`
  dispatches such an object immediately after the behavior callback, normalizes
  it to null, and clears only the physical-response flag. A tagged subject
  return therefore does not suppress the candidate behavior, and neither tag
  skips the subsequent subject/candidate component chains.
- `FUN_004259F0` is the hive contact hook. It consumes `0xC00` targets, or
  records a sufficiently energetic `0x2000` impact in the hive's nested state;
  it returns zero, so it does not cancel the physical response by itself.
- Component callback retail `FUN_00402DA0` / demo `FUN_00402DE0` performs an
  authored forward-half-space test and calls exact descriptor effect
  `FUN_00401A20` to mutate/randomize descriptor-driven contact state and
  effects. Its return is ignored, but omitting it would lose those side effects
  and alter shared RNG order for 16 of the 21 captured actors.

The three authored Level-1 type-61 allocations copy their Section-13 spawn
dword `+0x1C` to live entity `+0x88`. Spawn indices 32, 33, and 34 carry
`0x0000003C` (Targetter), `0x0000003F` (trophy), and `0x0000C802` (weapon
selector 2 with amount 200), respectively. The selector is the low byte and
the amount is the signed arithmetic shift of the complete dword by eight.
This field meaning is type-61-specific; other initializers reuse `+0x88`.

Rust now classifies the audited core behavior-style pair callbacks directly
from their retained addresses; every other address remains explicit rather
than collapsing to no callback. The accompanying pure callback planner is
still not an `EntityManager` dispatcher. Its currently closed surface is
deliberately narrow:

- Lifter `FUN_00425850` has the exact ordered rejection gates, signed
  occupancy/capacity comparison, operation `0x33`, deduplicated HUD resource
  event 5, occupancy advance, optional remote feedback, staged deferred
  destroy, and tagged `0x004BEAA0` return.
- Hive `FUN_004259F0` consumes a capability-`0xC00` target before considering
  capability `0x2000`; operation `0x33` takes its remote marker from the Hive
  source lookup rather than the target. Otherwise its energetic branch uses
  the strict projection `> 500` and signed wrapping product `> 75000` tests
  before setting the nested latch. Both closed Hive branches return null.
- Capture People `FUN_0040C910` is closed only through target eligibility and
  the capacity-full death dispatch. The attachment and behavior-advance
  transaction remains unresolved.
- The generic Power Up `FUN_00425AF0` planner remains closed only through the
  unsupported-recipient tagged return and known-absent-handler null return. A
  separate player/type-61 executor now owns the bounded single-player Level-1
  handler suffix:
  it walks live-list order, evaluates oriented Section-8 collision with signed-
  8.8 toroidal deltas, stages inventory/Targetter/hull/campaign progress
  transactionally, and queues accepted ids for deferred removal. A valid
  selector-2 duplicate is still accepted and consumed even when its max-not-add
  amount does not rise; a duplicate Targetter is rejected, stays live, and
  plays sound 5 at full gain with fixed playback rate `0xAAAA`.
  Selector `0x3E` uses the same staged capability owner: first contact sets
  controller bit 1, queues direct `0xCD/0x113`, plays positional sound 5 at
  native rate, and defers pickup removal. A duplicate stays live and plays
  sound 5 at fixed rate `0xAAAA`. The retained bit survives ordinary campaign
  world replacement, clears with the frontend session reset, and drives the
  exact 1.5× manual-lift / `0xC00` attenuation-threshold policy.
  Selector `0x3F` owns the exact zero-amount
  heal, per-control-slot claim, wrapping trophy/extra-life counters, direct and
  resource messages, sound 50's ordinary fixed `1x` or every-fifth fixed `2x`
  playback rate, and deferred removal. Level-1 overlay 13
  maps to campaign slot 1, so its CLAIMED bit lives at controller `+0xDC`.
- Component `FUN_00402DA0` and descriptor effect `FUN_00401A20` are now closed
  as one detached pure transaction. Each displacement is wrapping signed-i16
  8.8; the source basis is signed Q31 and the projection accumulates with
  wrapping i32 arithmetic. Projection `< 0` is a lazy no-op before live lookup,
  private task, topology, or RNG access; zero is accepted. An accepted
  nonnegative projection resolves the live source, private task, and topology
  in that order, then preserves the exact Sub-I zero-RNG wrapping heading
  `+0x2000` and retained-direction propagation through A/F/G. The no-Sub-I
  A/D/F/G reversal, retarget, and 2/3/4-draw
  cadence remain closed too; focused detached fixtures cover both topology
  families across captured type 9/17/47/62.
  An exact fresh-Level-1 type-9 whose Primary owns an already-published Wander
  carries retained direction `+1` and now admits the Sub-I branch in the player
  active-pair pass. Its authenticated source/task/component and pair-body
  leases are revalidated before the descriptor effect and resolved physical
  bodies commit atomically. Fresh production now selects and publishes all six
  type-9 branches through the shared RNG stream; selected Wander descriptor
  contact preserves its manager sidecar and task lease. Other selected type-9
  families and type-47/62 live integration remain fail-closed until their
  private task and optional
  Sub-D/Sub-F/Sub-G owners can be authenticated. Exact type-17 spawns 17--20
  retain their callback topology, but descriptor-contact execution remains
  fail-closed until their private task and pair-body orientation are
  authenticated. The separate Common-Dying matrix used for draw and particle
  collision does not establish that general pair custody.

Every unavailable live field and each unresolved suffix fails closed before
emitting any actions. The general callback plans are not a blanket
`EntityManager` dispatcher; integration still requires an atomic callback and
lifecycle executor for each admitted identity. Four bounded paths are
explicit: the player/type-61 inventory transaction, the player active-solid
pass, its exact already-published fresh-Level-1 type-9 descriptor contact, and
the exact fresh-Level-1 explicit-arrival Factory bridge.
The active-solid path runs on campaign worlds (not Intro2) and walks retained
live-list order. It visits only identities whose component, instance-modifier,
type-hit, and orientation fields are closed. Constructor `FUN_004104B0` writes a
null `+0x44` by default and type-46/`73`/`109`/`110` interceptors otherwise;
types 6, 66, and 67 also install no component pair callback, so Main Base,
Working Factory, and Alien Hive keep that constructor pair identity on every
world. Unresolved style still evaluates through the constructor pair policy:
Hive `FUN_004259F0` is an exact no-op against player capability set 5, so an
unpublished class-46 context no longer fails the solid pass. Type-9 conversion
and Factory pair arrivals remain Level-1 walkers.
The original Level-1 census additionally closes spawn 5/type 68 weight, spawn
8/type 54/model 560 Grock at active slot 0, and spawn 24/type 67/model 341 Hive
at active slot 2: both Grock and Hive have null component,
instance-damage-modifier, and type-hit callbacks. Grock's current behavior pair
slot is null. Their captured yaw-only bases match the authored
`LiveHeadingWithAuthoredPitchRoll` policy. Weight and Grock remain
spawn/model/rotation guarded rather than type-wide; type 67 now shares the
world-independent constructor identity of types 6 and 66.

Pending type-61 pickups are excluded from a second same-frame acquisition,
remain in the live list for the rest of that pass, and are spliced at the next
`EntityManager::update` while preserving survivor order. None of these paths
enables another type-9 identity, any type-17/47/62 descriptor contact,
multiplayer transport, or generic lethal lifecycle dispatch. The separately
bounded fresh-Level-1 type-17 null-hook composition is wired at F590 for an
exact constructor-published context. Production fresh actors now pass the
former birth-context gate. Its candidate walk preserves partial state
evidence and fails closed only when the retail zero/`0x5000` predicates are
genuinely unresolved. It is not a blanket exception to this
fail-closed contact surface. Pickup feedback is
integrated for the three authored Level-1 selectors; Targetter's separate
`+0x22C` scan program and cross-save campaign-progress persistence remain open.

No additional broad passive trace is needed for these identities. The
dispatcher differential and bounded type-9 Sub-I transaction are closed from
static evidence; no focused capture is required for them. Continue static
recovery/porting of one remaining callback effect and weighted style/RNG owner
at a time, and request a targeted contact breakpoint only if a specific branch
remains ambiguous after that audit.

The model-program gate is also closed for this census. Real-data tests resolve
zebrafsh (38), powerup (82), trophy (138), spider (256), newant (302), hive1xa
(341), man2 (558), and grock (560) from the high-resolution common/first-world
pools, exercise authored hit and miss paths under two orientations, and run all
64 query/target combinations. Every program stays within the supported
`0x8E/0x95/0x88` sphere/gate subset.

The weighted behavior boundary is now semantic rather than mysterious. Type 9
authors BaddieNearby x10 -> Run Away, PlayerNearby x3 -> Attract Attention,
BaseNearby x200 -> Go to job, and Always x1 -> Wander Near. Type 17 authors
PeopleNearby x4 -> Capture People, PlayerNearby x3 -> Run Away, UnderAttack x8
-> Run Away, and Always x1 -> Follow Beacons. Type 47 authors Always x9 ->
Guard Location and Always x1 -> Wander Near. `FUN_00425680` evaluates every
entry in authored order, wraps signed products/sums, upper-clamps only the total
to 32767, consumes one `Random_Next`, and picks the first cumulative weight
strictly greater than `((random16 * total) >> 16)`. The process-global RNG starts
at zero, is never reset, and follows `state = state * 214013 + 2531011` modulo
2^32 with `state >> 16` returned. This is enough to implement the rules, but not
to predict a particular instance without preserving every earlier shared draw.

Task results own later style changes. `0x9C00` with state bit `0x1000` clear
dispatches current style `+0x04`; `0x9C01` with that bit set dispatches `+0x00`;
`0x9C02` does neither, and an expired task dispatches `+0x00` while the bit is
clear. Relevant initial `+0x00` callbacks re-run weighted selection. Initial
Wander and GoToJob `+0x04` also reselect; RunAway, AttractAttention,
CapturePeople, FollowBeacons, and GuardLocation advance variant 0 to 1. Capture
People variant 1 owns `FUN_0040C910`; a successful capture advances to variant
2 with pair hook `FUN_0040D0B0` when its follow-up capability-`0x10` search
finds a target, or variant 3 with no pair hook otherwise. Reachable type-9 and
type-47 styles have no behavior pair hook, although their component hook remains
stateful.

The accepted ordinary-gameplay surveys
`20260724-041815-friendly-ai.jsonl`,
`20260724-041937-enemy-ai.jsonl`, and
`20260724-042053-enemy-ai.jsonl` validate those reachable families in retail:
type 9 visits Wander Near/Run Away/Attract Attention, type 17 visits Capture
People/Run Away/Follow Beacons, and type 47 visits Guard Location/Wander Near.
They close Follow Beacons variant 1 at `0x004C7B70` and Attract Attention
variant 1 at `0x004C86F8`, both with null release/pair/death slots. The
shooting type-47 run contains sparse source-correlated particle classes
87/88/73; the non-shooting type-17 run has no sustained matching family. That
runtime distinction now agrees with the statically recovered common-emitter
path. Type 47's exact Sub-E chooses method 30 `{0,3000,87,0}`, stochastic
interval 750,000 us, spread 100, strict wrapped `+-0x600` X/Z range,
`+-16000` aim followed by a forward-half-space gate, and sound 70. Class 87
uses frames 840--842 and damage channels `[2,6]` with raw amounts
`[1000,1000]`; the type-46 hull profile produces the observed filtered
result. The isolated Rust planner preserves the exact shared-RNG ordering and
class-73/88 surface conversions while remaining backend-neutral. A bounded
fresh-Level-1 coordinator now authenticates authored spawns 11/12/13 with an
already-published Guard-Pursuing/Aim lease, consumes shared `WorldFx` RNG,
commits cadence and transient append before acknowledgement, queues sound 70,
and later drains each class-87 request exactly once. Fresh construction
publishes the authored Guard/Wander birth graph for all three spawns. The
specialized scheduler visits slot-0 Chase then slot-2 Aim; production then
drains the entity-owned shot FIFO in live-list order into class 87 before
particle traversal. Chase and Guard wander apply the `V200003.run` full-reset
first-query owner for Level-1 Type-47 seeds `0x2B/0x2C/0x2D`; replay-level
seeds remain fail-closed. This VTOL/emitter slice does not itself supply that
first-query owner or general damage dispatch.

Types 9/17/47 also share model callback pair
`{FUN_0040D320,FUN_0040D350}` with the accepted Intro2 actors. Selector zero
returns the 50-Hz clock. Nonzero selector N resolves word N-1 from the live
component-array through entity `+0x4C/+0x08`. Real-model tests show `man2`
consumes selector 1 while `spider` consumes none of selectors 1--63. The
peasant's real writer is now recovered: its exact eight-byte Sub-I descriptor
binds selector 1 with a four-frame stride, and the zeroed 20-byte controller
advances one phase on an 80-ms reload in one of eight heading groups. That
neutral path is live without assigning a clock oscillator. Linked/forced-stop
groups 32--35 and special group 38--41 remain pure, non-integrated policy until
the owning behavior scheduler supplies their links, velocity mutation, and
sound timing. Static Attract Attention evidence selects global sound 72
(aliasing PCM 15), runs a 1000-ms task, then waits 5000 ms before the next
decision. The retained friendly trace shows the same one-second/five-second
cadence and replaces a task wrapper without changing style `0x004C86F8`;
future scheduler state must not be keyed only by style address. Sound 72 also
varies pitch/volume/pan between events, so its PCM alias is not a fixed playback
packet. The `spider` mesh's 16 type-14 external-frame vertex records identify
the remaining leg-animation boundary. `FUN_0040D350 -> FUN_0040A9F0` maps
those records onto eight paired Sub-H component records and
`FUN_0041D360` resolves their individual output points. The exact detached
`FUN_0041D0A0`/`FUN_0041D120` phase writer, rotating update order,
dependency/completion rules, and paired selector lookup are now implemented
and tested against the authored spider data. The exact detached
`FUN_0041D360` fixed-point geometry transaction is now recovered too: it
preserves conditional C/A/B callback order, length caches, the four-probe plus
final-midpoint terrain constraint, animated sine arch, and all six secondary
axis modes. The current Rust renderer still exposes only one shared external
point, so live Sub-H state construction/ownership, authenticated vertex/model
and effective-surface adapters, and per-record renderer integration remain
unresolved.

The three core behavior callbacks are separately bounded. Player
`FUN_00447D70` can attach eligible cargo/contact actors, returning the static
`0xA300` cancellation at `0x004C20D0`, or validate controller ownership and
emit distance-scaled feedback; a mismatch returns the non-cancelling `0x8802`
object at `0x004C20C0`. Lifter `FUN_00425850` accepts capability `0x400` only
under its capacity gate, emits operation `0x33`, queues deduplicated HUD
resource event 5, advances occupancy, deferred-destroys the actor, and returns
`0xA300` object `0x004BEAA0`. Main Base
`FUN_004258A0` accepts non-remote capability `0x800`, emits resource event 1,
queues deferred destruction, maps the live selector through
`1->8, 2->0x5B, 3->0x5A, 4->0x4F, 5->0x74, 6->7`, and attempts to spawn the
replacement from the source position. Type 8's default `+0xC0` bit `0x20`
then makes `FUN_0040D4A0` terrain-snap its final Y; there is no generic Main-
Base `Y - 2` rule. Spawn success installs the base relation and operation
`0x33`; spawn failure preserves the already-emitted event/destruction and skips
those dependent steps. Every successful type-8 birth consumes one behavior-
selector plus one constructor-suffix RNG word. Class 6 is not taskless:
`FUN_0040AD10 -> FUN_00402E20 -> FUN_00406030/FUN_00406070` installs Wander
Near and consumes that suffix word. The callback returns null on every branch.
The focused post-selector capture
`runtime_re/captures/local/20260724-040316-base-conversion-selector.txt`
closes the Level-1 case: a real type-9 peasant selected replacement type 8
for class 1 without changing the mapping call's sampled shared RNG endpoint;
spawn succeeded
as handle `0x04970001` at raw position `[0x50BA, 0xFD02, 0x39EC]` and linked
to Main Base `0x04B50001`. `0x447D70`/`0x443B50` and `0x471460` must still
distinguish the player beam/contact modes and feedback resource before those
branches are claimed complete.

The pure active-pair coordinator exposes the exact directional stages instead
of one aggregate callback result: subject behavior, immediate tagged dispatch,
conditional candidate behavior, subject components, then candidate components.
Each stage's body changes are visible to the next. The bounded fresh-Level-1
Main Base adapter now consumes this ordering through its exact mutable-list
birth transaction. Accepted
`runtime_re/captures/local/20260727-235038-base-conversion-pair-tail.txt`
proves retail saves `next` for each candidate immediately before its callbacks,
not once at pass entry. The observed type-8 replacement was appended behind an
old tail that had not yet been visited; after reaching that tail, traversal
saved its mutated `next` and enumerated the replacement in the same pass. This
disproves the former blanket replacement-exclusion rule. An append is skipped
only when traversal already saved its predecessor's old `next` (for example,
when the converting source itself was the saved tail). The pending-destroy
source still ran its type-9 component and physical suffix.

The bounded Main Base path owns the mutable saved-next executor, ordered
non-rollback journal, live selector/RNG evaluation, type-8 task installation,
tail append, recent relation, component continuation, and physical suffix. The
general whole-chain preservation proof remains conservative for every identity
outside that authenticated transaction.

The factory's deterministic receiving-side state is bounded first by a pure
Rust observational replay: exact Section-13 configuration, health-derived
initialization, staffing clamp/countdown, phases 0--4, finite stock, type-61
pickup presence, strict `0x3567E1` output conversion, vehicle-class output,
type-93 materialiser attachment/positional sound 8, and cooldown. A second exact
fresh-Level-1 bridge
now admits an explicit Main-Base-born Go-To-Job scientist, executes operation
`0x33`, resource events 5/2, staffing and deferred removal against live state,
publishes `1/2 -> 2/2`, drives 300 20-ms full-health phase-0 frames to a real
tail-appended/owner-linked type-61 product carrying `0x0001F412`, completes all
250 phase-1 frames, and advances the finite phase-2 path through 176 wait
frames, both one-per-update type-8/type-93 ejection transactions, positional
sound 8, and phase 4.

Natural movement, proximity detection, the nonzero-multiplayer phase-1
presentation branch, and progressive-death frames remain explicitly outside
that production machine. Live pickup presence, cooldown, and repeat
manufacture now use the spawned-handle lookup. The scheduler owner now
ticks damaged-repair/`FUN_00419010` and visits `FUN_00425C60` for live `0xD3`
before that owner. The newborn
factory outputs' constructor-selected tasks are installed, including class-6
Wander Near, but their later execution and locomotion remain separate from the
Main-Base-born Go-To-Job actor's accepted movement trace.

Rust now has a pure, deliberately non-integrated active-pair policy. It samples
the subject model/domain once, walks candidates in supplied intrusive order,
requests narrow phase and callbacks at the exact current visit after prior
responses, checks that callbacks preserved the remaining chain, reproduces the
fixed-point response/impact and ordered shared cap, then re-filters damage per
target before the distinct entity `+0x44` modifier and type-vtable `+0x30` hit
boundaries. Missing state/model/callbacks, remote delivery, topology mutation,
and death dispatch all return explicit unresolved results. This is an
integration foundation, not permission to connect a callback-free collision
loop to gameplay.

The normal-world common prefix of `FUN_00412DA0` is now exact once those inputs
are known: wrapping `+0x70/+0x6C` accumulation, low-16-bit 250,000/125,000
random thresholds in retail draw order, the 125,000 cap/remainder, callback-wait
`+0xB2` reset, `+0xB6` one-unit override, and wrapping `+0x68` advance. State
bit `0x02000000` skips both randomized waits. The symmetric
`FUN_00411A20` relation predicate is also exact at its strict `< 750000`
boundary. If a prerequisite remains unresolved, the port mutates nothing and
draws no random number; this preserves the transition without fabricating its
initializer-owned starting state.

The remaining cutover is now precise: initial constructor/style state is
data-driven, and the current behavior style is a separate runtime value rather
than an alias for initializer identity. The neutral callback census is closed.
What remains is to port the identified pair-callback side effects, recover the
evaluator/shared-RNG transitions that select and later mutate weighted styles,
and retain the corresponding non-player damage/death mutations. Once those are
explicit runtime state rather than a type list or frozen snapshot, the already
recovered broad phase, Section-8 narrow phase, response, and damage arithmetic
can be connected without guessing.

This order explains forward flight: pitched body-up lift supplies horizontal
acceleration. The July 16 guided captures
`runtime_re/captures/local/20260716-215549-vtol-acceleration.jsonl` and
`runtime_re/captures/local/20260716-220810-vtol-acceleration.jsonl`
independently show no translation from Up/Down alone and no separate VTOL
friction or speed cap. Their stable
powered-pitch phases agree within capture noise (about +1834/+1818 raw/s² for
Up+Space and −1719/−1714 raw/s² for Down+Space); the non-identical phase edges
also confirm that the normal dispatch delta follows the recorded global delta.
The captured body pitch also passes `+0x4800`, disproving the port's former
global ±`0x1800` clamp; the Rust angle now wraps in the native signed-word
domain. The July 17 mode-10 capture pins the normal recurrence's truncation
fixed points at 6028/6029 neutral, 1854 under Space, 10469 under Space+Up, and
-7572 under Space+Down. This exactly explains both the slight powered forward
bias and the return to forward motion after braking. The port's former direct
`sin(body_pitch)*80`, float gravity/friction, vertical speed cap, direct fuel
approximation, direct yaw rate, and float bank target are removed. Only the
live owner/admission of the alternate terrain-derived attitude state remains
open.

Static recovery now closes the arithmetic behind that alternate state without
claiming it for ordinary flight. `FUN_0041A690`'s `Sub-G+0x42 == 0` branch
derives its pitch target as:

```text
candidate =
    signed_low_word(runtime+0x1C)
    + 2 * (center_y - active_model_radius - coarse_center_surface)
    - 2000
target = min(candidate, static_Sub_G+0x1C)
if runtime+0x3C != 0: target = -target
pitch -= (pitch - target) >> 4
pitch += signed_word(runtime+0x1E)
```

The coarse center surface is the signed current-cell terrain height and, when
static Sub-G byte `+0x2B` is zero, at least the static sea plane. The subsequent
`FUN_0041AC40` projects a second X/Z sample 0x300 raw units through the
previous-frame Q31 forward basis, uses the greater current/projected surface,
and computes clearance against the same model radius. It writes runtime rate
`+0x34` as the clamped midpoint of static dwords `+0x10/+0x14` plus
`trunc((runtime+0x38-clearance)/40)`. It clamps X/Z velocity symmetrically to
`±static+0x20`, clamps only positive Y to the literal `0x5DC` (1500), and
updates runtime byte `+0x3D` with the exact 1100/1200-clearance hysteresis while
Y velocity is greater than the literal `-0x5DD` (-1501). The vertical limits
are independent of static Sub-G `+0x20`.

`vtol.rs::plan_vtol_terrain_derived_attitude_raw` retains those integer,
wrapping, signed-shift, asymmetric-Y, and hysteresis rules as a pure plan.
Surface samples are explicit inputs because this branch's current/projected
terrain-or-water samplers are distinct from the five ordinary VTOL ride probes.
Malformed decoded bounds return no plan. There is intentionally no player call
site: this closes static arithmetic, not the missing runtime writer/owner gate.

The accepted acceleration pair contains 14,163 and 14,143 entity samples,
respectively. Every one of those 28,306 samples has runtime Sub-G
`+0x3E=0`, `+0x3F=0`, `+0x40=0`, `+0x41=10`, `+0x42=1`, and `+0x43=0`;
the session mode is always 5 and runtime `+0x38` is always 120. Thus UP,
DOWN, SPACE, RSHIFT, UP/DOWN+SPACE, and UP/DOWN+RSHIFT are all input
alternatives inside the same normal player branch, not evidence for alternate
Sub-G attitude branches. Those guided branches, including signed reverse lift
and absolute fuel burn, are live and regression-tested. The two captures are
closed for the contract they actually observe.

#### Remaining cutover gates

The fixed-point body-basis gate is closed by the exact `FUN_00413F70` port and
5,000 zero-mismatch captured samples. Two guided acceleration captures now also
close the ordinary-gameplay force/delta gate: manual and assist lift, fuel,
near-surface correction, height/upward-velocity attenuation, and previous-basis
projection are live in the Rust path for the captured normal Sub-G state
(`+0x3F=0`, `+0x40=0`, `+0x42=1`, `+0x43=0`). The normal player's
mode-driven pitch self-righting and type-46 Sub-D yaw/bank recurrence are also
closed for the captured live setting (`Sub-G+0x41=10`).

Separate evidence gates remain for authored/runtime states that never occur in
either acceleration trace: the live writer/owner which can select the now
statically recovered `+0x42=0` plan, `+0x43!=0` alternate projection
coefficients, the `+0x3F/+0x40` attitude branches, the initializer-owned
meaning/writers of constant runtime `+0x38`, partial-frame key duty, and session
mode 4. Static code proves these branches exist but not which entity or
transition owns them, so they must not be enabled on the normal player without
a focused writer/caller capture. The other independent cutover is the complete
`FUN_00412530`/`FUN_00412760` active-entity pair-contact solver. The stack-only
dispatch argument itself remains
unreadable and must not be claimed globally equal outside the correlated normal
player path.
