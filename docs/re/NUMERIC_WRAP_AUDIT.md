# V2000 Numeric Width and Wraparound Audit

This owns numeric-width conclusions and their source-backed repairs. The
original 2026-08-09 audit changed no runtime behavior; later implemented
corrections are recorded in their sections. Priorities remain in
OBJECTIVE.md.

## Why this exists

The Castle/Intro2 yellow wash was caused by an upstream packed-operand decode
error. A value meant to be `-48` became `+47`; later 16-bit subtraction wrapped
an ordinary glow size into roughly 65,500, and the renderer correctly treated
that billboard field as unsigned. The useful general rule is therefore:

> A wrap is not itself evidence of a bug. First prove the original operation's
> width and signedness, then prove the consumer's units. Treat a huge rendered
> value as a producer/decoder alarm rather than adding a blanket renderer clamp.

The port intentionally reproduces a great deal of 8-, 16-, and 32-bit wrapping
arithmetic. This note separates currently safe/intentional behavior from
confirmed semantic seams that could create similar symptoms later.

## Result and corpus census

No second live, authored 65K-style visual outlier was found after the packed
immediate complement fix.

The audit combined source review, executable/decompiled comparison, and these
data checks:

- Parsed Section 8 from all 53 high-resolution `Overlay/1X*.OVL` files and
  walked commands by opcode arity. There are 209 `4D`/`5D`/`9D` shift-register
  commands; every shift count is an immediate from 1 through 13. There are no
  authored `AD` register-callback commands.
- Loaded the common pool plus each high-resolution system model pack 6 through
  11. Materializing every addressable global model with `dynamic[0]` sampled
  through the full word domain at step 257 found no billboard at or above 4096
  and no absolute vertex coordinate at or above 100,000 raw units.
- Independently setting each dynamic slot 0 through 63 to `0x7FFF`, `0x8000`,
  and `0xFFFF` found no billboard at or above 4096. This is negative evidence,
  not proof for every combination of callback values.
- The largest authored global-model collision radius is 4224 (`mangrove`). The
  largest LOD radius is 32512 (`nightsky2`), an expected far-background value.
- Across all 96 particle descriptors, the maximum draw scale is `0x0F00`
  (class 18), maximum referenced frame scale is `0x0100`, maximum collision
  radius is 600 (class 47), and no authored size-jitter divisor has its sign bit
  set. The only referenced negative frame middle word is class 3 frame 0 at
  `-8`.

## Confirmed semantic seams

These are documented investigation boundaries, not instructions to change code
without a matched retail/data check.

### Signed-word antipodal world delta — repaired

`4138F0` narrows entity-minus-viewport coordinates to signed SHORT before
selecting a world image. A difference of `0x8000` is therefore `-32768`,
including either direction between antipodal points. The float shared
`v2k_core::world::delta` advertised the same rule but used `d > 128` after
`rem_euclid(256)`, retaining `+128` in both directions. It now uses `>=` and
documents the interval `[-128,128)`.

The six production calls have three owners: HUD camera distance squares X/Z
(tie sign does not change that distance); `camera_relative` supplies actor,
static, particle and ring presentation images; `presentation_delta` carries
muzzle/projectile presentation offsets before horizontal position wrapping.
No caller requires an intentional positive antipodal tie. Y remains outside
this helper. Unquantized float inputs retain the interval policy rather than
being silently quantized to native 8.8.

The independent regression compares all65536 word displacements at five
origins, in signed and unsigned coordinate images:655360 exact comparisons
against `i16::wrapping_sub`. Before repair it fails at origin `-32768`,
target0 with `+128` instead of `-128`; after repair the core suite passes.
The separate [draw-custody owner](MODEL_DRAW_CUSTODY.md) records the actual
Intro2 case and source admission/callback consequence.

This repairs the shared horizontal tie, not every actor/viewport conversion.
The separate [external-point display policy](MODEL_DRAW_CUSTODY.md#native-external-point-display-images)
now preserves each native endpoint's independent viewport WORD image and the
full DWORD difference onto the GL actor origin. Free/intrinsic presentation
retains the actor-relative compatibility image. Native Q31 rows, complete
vertical/body transport and body/shadow raster parity remain separate;
neither repair establishes those complete contracts.

### 1. Model register DD/ED table arithmetic — closed

[`models.rs`](../../crates/v2k-formats/src/models.rs) `apply_register_op`
now uses the literal-quarter-table product in
[`fixed_math.rs`](../../crates/v2k-formats/src/fixed_math.rs).
Retail `466870`/`466900` discard the angle's low two bits, repeat the positive
Q15 table word into both halves of a Q31 dword, then negate that whole dword
in the negative half-cycle. Signed IMUL followed by SHL EAX/RCL EDX extracts
product bits31..46; the result remains an unsigned register word. ED adds a
wrapping quarter turn before the same lookup. This restores the original
rounding and quadrant asymmetry without a floating sine or output clamp.

The bounded original-PE arithmetic oracle executes both command handlers,
their `470840` operand decoder and the original table. Its controlled inputs
cover every authored newant antenna clock phase and unsigned amplitude,
low-bit and quadrant boundaries. Newant's packed shift operand11 decodes12:
its antenna clock repeats every16 ticks. The positive phase peak is65535
after the authored `+0x8000`, where the old floating path wrapped to zero.
At bigfuel tick2292, original `466870` DD with the authored packed operands
`[1,0x400E,0x00C1]` and register1=`0xE800` (`2292 << 9`, narrowed
to a word) returns `0xFFF6` (-10). The following `1D` subtracts decoded
`-48`, producing38 for both sprite617 billboards, rather than the old37;
the exact range is30..65. The `bigfuel_dd_tick_2292` controlled PE case
and owning integration regression preserve these specific inputs and result. The arithmetic evidence is independent machine
execution, not a newly captured retail frame or proof of a visible newant
line artifact.

### 2. Shift register ops differ only for presently unauthored counts 16..31

Retail `4D`, `5D`, and `9D` shift a 16- or 32-bit machine operand using an x86
count masked to five bits, then store the low word. Counts 16..31 therefore
produce zero for logical shifts and zero or `0xFFFF` for signed right shift.
The Rust operations in
[`models.rs`](../../crates/v2k-formats/src/models.rs) operate as `u16`/`i16`;
`wrapping_shl` masks modulo 16, while ordinary oversized shifts are
build-check-dependent. Thus the semantics disagree for 16..31.

The current retail corpus cannot reach the mismatch: all 209 authored shift
commands use immediate counts 1..13, with no callback/register shift counts.

### 3. Register opcode AD narrows its callback index too early

Retail `FUN_00466750` zero-extends two decoded words, adds them in 32 bits, and
passes the full sum to the callback. The port uses `a.wrapping_add(b)` before
indexing `AnimVars::dynamic`, so `0xFFFF + 1` becomes index 0 instead of 65536.
The executable evidence is in
`cmd_stream_handlers.c` around lines 8771-8790; the
port path is in [`models.rs`](../../crates/v2k-formats/src/models.rs).

No `AD` command occurs in the 53 high-resolution overlays, so this is confirmed
but dormant for shipped data.

### 4. GL model billboards omit two retail projection safeguards

Retail projects the billboard anchor before queueing it:

- `FUN_0046CD90` returns clip bit `0x40` when anchor view depth is below raw
  `0x40`, and every billboard handler rejects that center; see
  `vertex_transform_handlers.c` around lines
  90-103 and `cmd_stream_handlers.c` around lines
  9413-9432.
- `FUN_004594C0` repeatedly halves both projected half-extents until their OR is
  at most `0x1FFF`; see
  `vertex_transform_handlers.c` around lines
  2476-2506.

[`gl_backend.rs`](../../crates/v2k-render/src/gl_backend.rs) currently turns
the raw unsigned size directly into a world-space camera-facing quad. It has no
equivalent anchor-depth rejection or projected `0x1FFF` cap. GPU near-plane
clipping is not the same policy: a quad centered behind or extremely near the
camera can intersect the near plane and become a transient screen-filling
wedge. This is a plausible future camera-dependent artifact even when the size
itself did not wrap. It was not the cause of the fixed Castle yellow wash.

### 5. Particle frame scale typing differs, but shipped data is bounded

Retail `FUN_0043D410` consumes the frame tuple's third word as signed and uses
32-bit products. [`particle_descriptors.rs`](../../crates/v2k-game/src/particle_descriptors.rs)
stores `scale_raw` as `u16`, while
[`main.rs`](../../crates/v2k-game/src/main.rs) widens the sprite-size product
to `i64` before conversion to world dimensions.

All 795 descriptor-referenced tuples are safe: their scales are
`0x002D..0x0100`, all 254 referenced sprites resolve, the largest sprite
dimension is 120, and the largest authored product is safely below signed
32-bit overflow. An aligned `0xFFF8` elsewhere in the contiguous raw frame area
is not referenced by any descriptor and must not be cited as a negative scale.

## Producer-range tripwires

These paths currently match their known inputs but amplify bad producer values:

- Model billboard size is zero-extended directly into both GL quad extent and
  linked-model diagnostic bounds in
  [`gl_backend.rs`](../../crates/v2k-render/src/gl_backend.rs) and
  [`model_tree.rs`](../../crates/v2k-game/src/model_tree.rs). Only zero is
  rejected. A future value near `0xFFFF` should reopen its operand producer,
  not prompt a signed cast at these consumers.
- Particle draw scale, signed jitter divisor, frame scale, projection extent,
  and wrapping screen cull form one chain across
  [`world_fx.rs`](../../crates/v2k-game/src/world_fx.rs) and
  [`main.rs`](../../crates/v2k-game/src/main.rs). The authored corpus is
  positive and bounded; new particle-class work should re-check that invariant.
- A negative particle frame middle word becomes a terrain-light radius. A
  corrupt `i16::MIN` would expand to 8,388,608 raw units and make
  [`terrain_light.rs`](../../crates/v2k-render/src/terrain_light.rs) attempt a
  roughly 65,536 by 65,536 radial walk. The only referenced negative authored
  value is `-8`.
- Model collision/effect radii are authored `u16` values but several faithful
  paths narrow them to signed words, including
  [`static_contact.rs`](../../crates/v2k-game/src/static_contact.rs),
  [`type17_primary_hit_live.rs`](../../crates/v2k-game/src/type17_primary_hit_live.rs),
  and [`static_damage.rs`](../../crates/v2k-game/src/static_damage.rs). A
  producer regression above `0x7FFF` can reverse placement or invalidate a
  scan. Current model and particle radii remain well below that boundary.
- Terrain-light point accumulation saturates only above `+127`; negative
  underflow wraps through signed `i8` and can become bright. That asymmetry in
  [`terrain_light.rs`](../../crates/v2k-render/src/terrain_light.rs) is
  explicitly retained from retail. Existing writers are positive.

## Known intentional wraps: do not normalize globally

- Model register add/subtract/multiply, `7D`'s deliberate `b - 1` underflow,
  `8D` quotient narrowing, signed `9D`, and negative DD/ED results stored as raw
  `u16` words are part of the original register machine.
- Signed 8.8 world X/Z and wrapped cell addressing implement the 256 by 256
  torus. Angle words likewise wrap by design.
- Water inputs, particle age bytes, radar packed coverage/color arithmetic, and
  tested static-damage placement use deliberate legacy-width arithmetic.
- Parser `offset + width` hardening for adversarial API inputs is a separate
  robustness topic. Corpus-derived offsets are bounded; it is not a gameplay or
  rendering-wrap lead.

Any future correction should attach retail provenance and a focused boundary
test at the producer/consumer seam. Do not replace the register machine or
world-coordinate arithmetic with saturation as a general cleanup.
