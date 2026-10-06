# Shared Sub-F swimming component

The B/D/F swimming topology uses the ordinary common mover's Sub-D steering,
Sub-F callback, and Sub-B lateral correction. Sub-F is not an independent
scheduler or position integrator. Its owner must preserve the detailed/coarse
gate, task order, retained body basis, and later common environment/master
motion phases. Fish behavior evidence and remaining acceptance are in
[ACTOR_RUNTIME](ACTOR_RUNTIME.md#reef-fish-movement-runtime-validated-2026-08-02).

## Descriptor and constructor

Section12 Sub-F (`type+C8+1C`, subsection field39) is exactly fourteen bytes:

| Offset | Consumer meaning |
| --- | --- |
| 00/02 | Signed clearance base and random span |
| 04 | Signed target-speed base |
| 06 | Animation mode byte:0/1/2 |
| 07..0C | Six signed `40A950` model-variable selectors; zero leaves a null pointer |
| 0D | Retained byte, no recovered consumer |

Successful `424220` zeroes a forty-hex-byte runtime allocation, then sets:

1. `+34 = base + ((low16(random1) * signed_span) >>16)`.
2. If `speed_base - (low16(random2)>>8) <50`, `+38=50`; otherwise
   **draw again** and use `speed_base - (low16(random3)>>8)` without another
   clamp. A base100 fixture can therefore produce speed-155.
3. `+24=3000` hex, followed by six signed selector resolutions in order.

Runtime `+18..+1C` is the target position; `+20` the animation phase; `+24`
the output-smoothing bias; `+28` smoothed steering; `+2C/+30` forward/vertical
acceleration; `+34/+38` clearance/speed; `+3C/+3D` reversal/follow mode.
Constructor writes precede behavior selection in the process RNG stream.

`424380` writes the reversal byte unchanged. `424390` writes the follow byte
and sets `+24=1000` hex when nonzero, otherwise `E000` hex. The Sub-F arm of
shared task initializer `406070` invokes both with zero and consumes no RNG.
Successful target-route constructor `403650` subsequently invokes `424390(1)`.

## Callback order and arithmetic

`4236D0` takes the living path only when entity state is nonzero and bit4000
is clear. Otherwise it advances roll and damps pitch; it does not animate,
sample terrain, control acceleration or apply velocity drag.

The living path is animation first, controller `423DF0` second, then forward
and vertical acceleration, pitch/roll damping, and Q31 velocity drag. Animation
therefore reads the **previous** acceleration and pre-controller pitch.

Modes0/1/2 dispatch `423890`/`423BA0`/`423C80`. All use the literal retail
quarter-sine table. Duplicate the positive table word into both halves before
negating the entire dword; duplicating an already-negative word differs.
Output pointers may alias: mode0's three steering writes precede the sine
writes, and the last bound output reads the word resulting from earlier writes.
The last output uses its previous **unsigned** word and signed division by4.

`423DF0` samples bilinear terrain at current XZ and one forward-basis offset
(`forward.x/z >>21`), using the higher terrain for model clearance. The active
model header+8 extent is unsigned. It clamps current vertical velocity to
[-300,250], computes speed correction, and clamps forward acceleration to
[-50,50]. Reversal uses negative `(target_speed>>7)` and vertical acceleration2.

Follow mode uses target Y only when both promoted signed-word X/Z differences
have absolute value strictly below500 hex. These differences do not wrap.
Otherwise clearance controls vertical acceleration. Pitch is adjusted through
word arithmetic and clamped to ±3000 hex.

The sea cap belongs to this callback, not construction: if Y exceeds sea-100,
underwater terrain makes Y=sea-100; otherwise Y=ground, roll=4000 hex, pitch=0,
and the first bound output gains2000 hex. Subsequent damping still runs. This
does not authorize clamping authored or parked coarse fish at birth.

## Native owner boundary

[FISH_RUNTIME](FISH_RUNTIME.md) owns Types22/24/124 construction, authored
task graphs, descriptor contact, and detailed/coarse scheduling. Its
[mover](../../crates/v2k-game/src/shared_fish/mover.rs) uses the common
01430/018A0 frame machine and retains the incoming body basis throughout B/D/F.
Sub-D smoothing uses the callback delta while yaw integration uses the
independent `DAT_004D04E4` clock. Sub-F uses the callback delta. The outer
callback publishes the next body basis and owns environment/master motion.

## Port and verification

- [Typed descriptor](../../crates/v2k-formats/src/collision.rs): exact length,
  signed words/selectors, retained final byte.
- [Component](../../crates/v2k-game/src/common_mover/sub_f.rs) and
  [animation writes](../../crates/v2k-game/src/common_mover/sub_f/animation.rs):
  explicit raw body/frame/runtime inputs, caller-resolved bank bindings,
  unknown-mode and invalid-bank rejection before writes.
- [Focused tests](../../crates/v2k-game/src/common_mover/sub_f/tests.rs):
  constructor RNG branches, selector aliases, task resets, fail-closed inputs,
  and independent executable callback vectors.

On2026-09-19, an ignored local Unicorn oracle ran original PE4236D0 for600
deterministic cases (seed424220), with all three animation modes, aliased and
null outputs, signed terrain, dt0..125000, sea/beach, reversal/follow, and dying
state. Every resulting body/runtime/output word matched the Rust component.
Seven independent cases are permanent golden fixtures. A separate PE424220
oracle verifies the two/three-draw constructor branches and signed binding order.
Local replay helpers are `.tmp/sub-f-oracle.py`, `.tmp/sub-f-probe.rs`, and
`.tmp/sub-f-constructor-oracle.py`. These are **executable emulation**, not TTD
trajectory evidence or proof that natural fish scheduling is complete.

The later [steady environmental-force leaf](../../crates/v2k-game/src/shared_fish/environment.rs)
was separately compared with original PE `44EC60` over 800 deterministic
cases (seed4460): mode-zero fallback, both sea-side selections, signed/wrapped
terrain probes, shelter, height caps, Q31 angle changes, and velocity overflow.
All outputs matched; three cases remain permanent golden tests. The ignored
helpers are `.tmp/fish-environment-oracle.py` and
`.tmp/fish-environment-probe.rs`. This emulates the force callback with lookup
and level-medium leaves resolved; it does not emulate the global mode2 clock.
