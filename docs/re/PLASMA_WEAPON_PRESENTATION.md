# Plasma weapon presentation

The coloured plasma effect is an authored moving additive glow over a fixed
coloured gun body. Turret and player models share the same 50-Hz render clock
and phase arithmetic. No palette cycling or firing-state oscillator is needed.
Gameplay and emitter ownership remain in [PLAYER_CRAFT.md](PLAYER_CRAFT.md)
and [INTRO2_TYPE102.md](INTRO2_TYPE102.md); the shared red/green/blue projectile
packets and water pairs are documented in
[combat projectile execution](INTRO2_COMBAT_PROJECTILES.md#descriptor-owned-execution).

## Callback and clock authority

The Section12 loader in v2000_section_handlers.c
sets every type row's `+0x78` to `4C8A28`. The retail table contains
`40D320/40D350`, including Type 46 and plasma turret types
92/96/97/99/102/103. The constructor chain `38080 → D720 → 104B0`, component
initialization `09A80`, and the common `D4A0 → 381F0` initializer retain this
pair. Their writes initialize actor/component/task state, not type `+0x78`.
The source audit found no replacement of these six rows' callback pair.
`38080`'s type-specific table substitutions affect the separate `+0x7C`
field for hive 67 and player 46/51; no plasma turret enters those branches.
This is a type presentation contract, independent of Intro2 spawn identity
or whether the port has a native combat owner for the actor.

Retail disassembly of `40D320` establishes the selector split: selector zero
returns `AX = word[4FED60]` directly at `40D344`; nonzero selectors resolve
the entity's retained joint words through `40A950`. The clock does not depend
on a component bank or the age of the actor.

`428B00` accumulates elapsed microseconds in the 64-bit `4DB0E0/4DB0E4`,
divides by the verified 64-bit constant `4BED80 = 20000`, and stores the low
dword quotient in `4FED60`. `428AD0` resets both the accumulator and tick to
zero. The positive-time contract is therefore
`tick = floor(accumulated_us / 20000)`; callback zero exposes its low 16 bits.
The clock functions are retained in bulk/game_logic.c.

## Authored phase and geometry

Command `5D` (`466590`) decodes both operands through `470840`, shifts the
first by the second masked with 31, and stores a wrapping word. The plasma
program's operands `0080,800A` mean callback zero and shift count 10:

```text
phase = u16(u16(tick) << 10) = (tick & 63) * 1024
opposed_phase = u16(phase + 32768)
```

Command `0D` (`466410`) supplies the wrapping half-cycle addition. Thus one
sweep lasts 64 ticks, or 1.28 seconds; opposing barrels differ by 32 ticks,
or 0.64 seconds. These are sawtooth phases, without sine, RNG or fire gating.
See cmd_stream_handlers.c.

TF8 (`46F7D0`, with near/far variants `46F8F0/46FA20`) uses the selected
unsigned word register to interpolate authored endpoint slots. Per axis,
the exact integer expression is `a + 2 * (((b - a) * phase) >> 17)`, with a
signed wide product and arithmetic shift. The existing `retail_tf8_axis`
implementation preserves this quantization; see
vertex_transform_handlers.c.

In the normal-tier red double turret, model 163 writes register 1 with the
clock phase before drawing one model 164 barrel, then adds `32768` before the
other. Model 164 slot 20 is TF8 from slots 16 to 18, moving its glow from raw
local Z=66 toward 205. The red single gun 172 uses register 5 and Z=45 toward
140. Player models 46–48 initialize register 5 with the clock phase and
register 6 with the opposed phase; TF8 slots 50/32 move the respective glows
from Z=32 toward 108. The parent barrel selector in register 3 chooses which
glow branch is submitted. HUD models 117–119 use register 5 and Z=56/208.
The phase wraps before reaching the exact endpoint.

## Normal-tier asset mapping

These global IDs are authenticated through the canonical Rust model and
sprite parsers using system variant 1 (`1X3XX.OVL`).

| Colour | Player attachment / HUD | Double turret type / hierarchy | Single turret type / hierarchy | Core / glow sprites |
|---|---|---|---|---|
| Red | 46 / 118 | 102 / 162 → 163 → 164 | 92 / 171 → 172 | 599 / 603 |
| Green | 47 / 119 | 103 / 168 → 169 → 170 | 96 / 175 → 176 | 598 / 602 |
| Blue | 48 / 117 | 99 / 165 → 166 → 167 | 97 / 173 → 174 | 597 / 601 |

All four Section12 model slots of each turret type contain the listed root.
The canonical entity-model table contains exactly these six type rows for
the six plasma turret roots.

Player inventory selectors 14/13/12 select model callback values 6/7/8 for
red/green/blue respectively. The core sprites are 1×1 with one shade and
render flags `0x0C` (fixed shade plus half additive). The moving glows are
14×14, have 14 shades, and use flags `0x15` (keyed, fixed shade, additive).
These flags come from the sprite entry's `pal_size` low byte; its separate
`flags` field packs dimensions. Materials stay fixed while the glow's
authored TF8 position changes, producing the apparent colour pulsation.

## Production ownership and acceptance

`Entity::presentation_anim_vars(retail_tick)` publishes callback zero for
all six plasma turret types through `uses_audited_clock_model_callback`,
covering generic and native actors in intro and ordinary-world presentation.
Retained nonzero joint values remain owned by the native gun-turret runtime;
clock publication does not require that runtime to exist.
`draw_player_craft` likewise inserts `retail_tick as u16` into `dynamic[0]`
before submitting the complete linked model hierarchy. The intrinsic
`PlayerCraft::anim_vars()` remains a component-only bank for its other
consumers. Both presentation paths use the existing session clock; no
per-weapon timer or material-update API is introduced.

The defect was that these presentation paths omitted callback zero, leaving
the authored glow phases frozen despite working TF8 geometry. Source and
normal-tier assets establish the repair without a new retail capture.
Corpus-backed regressions acquire all three player weapons and call the actual
player draw, then cover all six turret types without a native task receipt.
They verify positive-sized, unclipped additive glows, quarter-cycle movement,
64-tick repetition and the callback's 16-bit wrap, with stationary core geometry
and materials. Native Type92/102 controls retain aim words across clock updates.

Normal-tier 640x480 OpenGL checks cover the three player colours, default gun,
six ordinarily constructed turret variants and two native Intro2 turrets.
Every plasma fixture has four distinct quarter-cycle images and returns
byte-for-byte to phase zero at tick64. Default guns remain byte-identical to
the baseline; ordinary and native red turrets match at every sampled phase.
The changes stay in the gun/glow regions. These checks establish the port's
authored animation and presentation path; the retail timing authority is the
callback and clock source above.

The complete fixed40ms/frontend100 New Game/Intro2/first-world OpenGL run
also completes with no runtime issues. Actor health/model transition summaries
match the preceding renderer checkpoint, including all three turret deaths and
the dragon-driven factory destruction. Menu, village, final card, closing Klaus
and first-world reveal control frames remain byte-identical.
