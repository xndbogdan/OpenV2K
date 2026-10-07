# V2000 software raster (16-bpp Graph2D fill table)

How retail `V2000.EXE` turns a queued primitive into RGB565 pixels when the
surface is a 16-bpp software surface, and how `v2k-render::software`
reproduces it. This is the CPU path; the alternate/Direct3D `Graph2D` table
(`FUN_00481240..FUN_00492A10`) is a different consumer and is not described
here. See [RENDER_PIPELINE.md](RENDER_PIPELINE.md) for the queue, producers
and the frame-level evidence.

Status: every reachable fill slot and every span row a slot can bind is
ported and matches retail byte for byte on the native receipts below, and
the primitive queue reproduces the retail queue controls. The opaque ground
producer is ported and byte-exact against the retail scan. `--renderer
software` (`crates/v2k-render/src/sw_backend.rs`) draws videos, menus, 2-D
overlays and the gameplay ground through them; water, sky, model and
particle producers are not ported yet.

## Pipeline

1. **Queue thunks** `0x0047A720..0x0047AB00` call one Graph2D fill slot with
   the queue record's payload (`record + 8`).
2. **Fill-slot handler.** `FUN_00480D10` stores the software handlers at
   Graph2D `+0x101C..+0x10C4` when its mode descriptor's word `+8` is zero
   and word `+4` (bits per pixel) is 16; a non-zero `+8` installs the
   alternate table. The queue reaches 32 of the 43 software slots;
   `+0x101C`, `+0x1068..+0x107C` and `+0x10B0..+0x10BC` have no thunk or
   call site.
   - Six **2D slots** write the surface themselves (`+0x1020..+0x1034`).
   - 26 **polygon slots** decode corners and attributes into the vertex pool,
     bind a material and a span row, store `0xE49C` into the dither word
     `+0x10C8`, and run the scan converter.
3. **Scan converter** `FUN_00472B20` walks the polygon's two vertex chains,
   clips them to the Graph2D rectangle, and calls the bound row's span filler
   once per scanline with the left and right edge records.
4. **Span row.** Each of two static 36-row tables (indexed texels
   `0x004D55A0`, raw texels `0x004D5900`) holds per row a span filler and
   the five routines of one attribute class (edge init, x-clip interpolation,
   clip clamp, multi-line step, edge-to-vertex copy).

### Graph2D block and device fields

| Offset | Meaning |
|---|---|
| device `+0x00/+0x04` | surface width/height (the clip setter clamps to them) |
| device `+0x18/+0x1C` | locked surface pointer, pitch in bytes |
| device `+0x20/+0x24/+0x28` | red/green/blue channel shift words (RGB565: 7, 2, 4) |
| `+0x0C` | clip rectangle `x0, y0, x1, y1` (i16; left/top inclusive) |
| `+0x14` | bound Section-3 material record |
| `+0x18` | per-primitive word; byte `+0x1B` is the fixed palette row |
| `+0x1C` | flat colour / packed word for untextured rows |
| `+0x20` | bound span row |
| `+0x24..+0xE34` | vertex pool: 100 × nine dwords (attributes 0..3, x 16.16, y, u, v, fade) |
| `+0xE34` | live pool length |
| `+0xE38..+0x1018` | four ten-entry `{from, to, flags}` chain lists |
| `+0x1018` | span-row table base: untextured handlers write the raw table, unfogged textured handlers the material's; fogged handlers leave it |
| `+0x10C8` | dither word |

The pool and lists persist between primitives; a list slot read past its live
length sees the previous primitive's entry, and the port keeps that.

## Fixed point

- Products use the Q31 multiply `imul; shl eax,1; rcl edx,1`.
- Slopes are Q31 products with a reciprocal; most fillers round a negative
  slope one unit toward zero, but the indexed fillers leave the u step
  unrounded.
- Edge and span reciprocals come from a 641-entry table (`0x4D4B98`). Spans
  wider than 640 pixels index past it into the span-row tables that follow,
  and use those routine addresses as reciprocals.
- Clip interpolation halves the numerator, clamps the ratio to `0x3FFFFFFF`,
  picks its sign from the branch taken, then doubles it.

## Scan conversion

- The top vertex starts chain A (walking backwards) and chain B (forwards).
  A chain that turns upward ends the walk; for a quad, `FUN_004734E0`
  instead draws two triangles sharing the chain's second and fourth vertices,
  re-entering the converter 0x1A0 bytes deeper on the stack.
- Top clipping (`FUN_00472680`) drops segments above the clip top. Side
  clipping (`FUN_00472880`) splits segments at both clip sides and tags each
  piece. The rebuild's skip loop ends chain A at its split count but chain B
  only past it (`jg`), so it can read one stale list entry. When a rebuilt
  chain-B segment starts away from the previous one's end, its new start
  vertex is clamped toward chain B's own edge instead of chain A's
  (`00473281`).
- Each scanline goes to the filler with the smaller-x edge as left; equal x
  picks chain B.

## Span rows and fillers

Row selection, by handler and material flags (`0x01` keyed, `0x02` raw,
`0x04` row 28, `0x08` half-additive, `0x10` additive):

- flat 0 (half 12), tinted 1 (13), Gouraud 2 (14);
- textured 4/5 (one-row 5), +4 keyed, +12 half-additive, +24 additive;
- per-vertex shaded 6, with the same offsets;
- fogged rows 7, keyed 11, half 19/23, additive 31/35.

Rows 3, 15 and 24..27 are bound by no slot; their fillers are not ported.

Notable filler behaviour (each port names its retail routine):

- **Direction.** Indexed fillers write right to left from the right edge;
  raw and untextured fillers write left to right from the left edge.
- **Dither.** Shaded indexed fillers add generator noise (`state * 9`, seeded
  from `+0x10C8`) to the palette row and fog level, and each keeps the
  generator differently. `00478510` puts the left edge record's address high
  half above the stored word, so the retail stack location of the scan
  converter is an input to those pixels. `004799B0` stores back the last
  pixel's halved destination. The additive shaded rows (`0047A0B0`,
  `0047A540`) step `0x43FD` per drawn pixel and write an unmasked saturating
  sum. The half-additive shaded rows (`00479220`, `004799B0`) use `u >> 16`
  as their "noise".
- **Pixel pairs.** `00479220` samples one texel per aligned pixel pair; a
  two-pixel span starting at an address of 2 mod 4 draws nothing.
- **Fog ramp.** `FUN_0047CA20` rebuilds sixteen packed ramp entries and the
  saturation masks only when the fog colour changes. Process memory starts
  zeroed, so until a fogged primitive supplies a non-zero colour the masks
  are zero and additive draws write black.
- **Raw rows.** Raw half-additive *and* additive rows never read the
  destination: they write the (lit) texel at half intensity, masked with
  `!(0x80 << (green+1) | 0x80 >> (blue-1))`. Raw fogged rows walk the fade
  from the right edge toward the left, read the low word of ramp entry
  `(fade >> 20) - 1`, and saturate on each field's lowest bit instead of
  its carry.
- **Accumulation.** `00476990` (and the unbound `00477AD0`) add the previous
  pixel's sum instead of the colour, so a span brightens toward white.

## Primitive queue

Producers append records to one bump arena (`FUN_0044F540` requests 0x19000
bytes; `FUN_00494860` keeps the last twelve free) that `FUN_00428E60` resets
(`FUN_004948C0`) and `FUN_00428E70` drains (`FUN_004948F0`):

- the 24-byte root holds the current scope's mode (1 sorted, 0 FIFO), the
  root list head, the tail (address of the last `next` field), the current
  scope header, the limit and the cursor;
- `FUN_00459D10` appends `{next, callback, payload}`, `FUN_0045B220`
  `{key, next, callback, payload}`, and `FUN_0045B280` picks by the current
  mode; payloads are rounded up to dwords;
- `FUN_00494AB0` / `FUN_00494B60` append a group record whose payload is a
  child list header `{saved mode, head, saved tail, saved scope}` and whose
  callback sorts and drains (`FUN_00494930`) or only drains
  (`FUN_00494A50`) it; `FUN_00494A80` restores the enclosing scope;
- the bottom-up merge sort orders by descending signed key, equal keys by
  ascending record address, and relinks the list in place; draining stops
  at the first non-zero callback result and does not consume records;
- a record past the limit raises an engine error through `FUN_00471150`
  without linking; a group that does not fit returns an error and leaves
  the scope unchanged.

Every record callback is a thunk (`0x0047A720 + 0x20 * k`) that calls one
fill slot with the device and payload and returns its result.
`crates/v2k-render/src/software/queue.rs` keeps the arena as bytes with this
layout; its tests replay the retail queue controls.

## Ground producer

`FUN_0042F960 -> FUN_0042F980` queues the opaque ground
(`crates/v2k-render/src/software/terrain.rs`):

- `FUN_00431890` builds a context from the world viewport: the first eight
  Section-6 dwords (shade words), the queue, the fog colour (`+0x7C`), the
  projector at dispatch-table `+0xB4`, the eye words, the darkness band from
  the Section-10 header, the infection base and a signed row lead from
  camera basis word 12 (`0x200`, or `0x200 + ((w + 0x58000000) * 0x1C00 >>
  31)` when that sum is negative). `FUN_00433530` advances the infection
  motion offsets.
- Rows are lines of constant world X. `FUN_0042FCC0` fills one row of
  `DAT_004CAB74` points along +Z from `eyeZ + lead`: the first and last
  points come from `FUN_00430140` at the fractional ends (bilinear height),
  interior points from whole cells. Each point gets its height, light-window
  and darkness shade index (0..7), infection motion, the projector's screen
  point, outcode, fade byte and a 16-bit depth word; points behind the near
  plane are reprojected at depth 0x40 by `FUN_0042FFF0`.
- The scan walks `DAT_004CAB70` rows toward -X from the eye's row, then
  restarts at the eye's row and walks toward +X. After each strip it
  retires leading points whose next point is past the near plane or off the
  screen side it walks toward (outcode `0x41`, then `0x44`).
- Each strip (`FUN_00430430`) takes the lower-X row first. Its first and
  last cells are mapped quads (`+0x10C0`, fogged `+0x10C4`) with the leading
  or trailing edge UVs interpolated by the eye's fractional Z; other cells
  are shaded quads (`+0x10A8`/`+0x10AC`) whose corners are permuted onto the
  canonical transition sprite (`FUN_00433180`). Cells with infected corners
  add an overlay allocated right after the base with the same key; a fully
  infected interior cell queues only the full infection shape. Cells are
  skipped when their outcodes cannot reach the screen, every corner is fully
  faded, or `FUN_004709B0` finds both triangles wound counter-clockwise on
  screen. Keys are the latest row's depth word plus 0x200.
- Every first-row point pair also queues `FUN_00431970`'s flat cap to the
  screen bottom (key 0, system-2 palette entry 11).

The world viewport words come from the chase or intro camera's native
viewport and the lens from system level 2; the fade ramp is
`[0x1000000 / (far - near), near, far]` over the world fog planes. Terrain
sprites are registered as native Section-3 records sharing their atlas's
display-format palette block.

## 2D slots

| Slot | Handler | Behaviour |
|---|---|---|
| `+0x1020` | `FUN_0047AC30` | dword image, low words written, optional zero key; draws nothing unless wholly inside the clip |
| `+0x1024` | `FUN_0047B360` | flat line as one clipped run per scanline |
| `+0x1028` | `FUN_0047B6C0` | two-colour faded line; compares 16.16 positions with the pixel clip and uses them as the address, so only points within 1/64 px of the origin draw |
| `+0x102C` | `FUN_0047AB20` | set the clip rectangle, clamped to the surface |
| `+0x1030` | `FUN_0047AD90` | unscaled sprite; flag 0x08 adds `(dst >> 1) & 0xFBEF` (RGB565) |
| `+0x1034` | `FUN_0047B9F0` | fills `height` rows' worth of words from `x0 / 4` dwords into row `y0`, ignoring the right extent, in 16-byte blocks counted back from the end |

## Evidence: native receipts

`crates/v2k-render/tests/software_raster_receipts.rs` replays 896 receipts.
Each was produced by private tooling that executes the unchanged retail
handler, scan converter and fillers on synthetic inputs (surface, atlas,
palette and word images generated from seeds by a hash the test
reimplements). The fixture holds parameters, packets, the scan converter's
stack frame, the final dither word and clip, and a 64-bit FNV-1a digest of
the output surface with its changed-pixel count. It contains no retail
data.

`crates/v2k-render/tests/software_ground_receipts.rs` replays 12 whole
ground scans (dry and wet projectors, fractional and wrapping eyes, steep
and negative-lead pitches, near-plane points, dense infection, heavy fog,
a 52-by-30 scan, a darkness band and a translated view) and compares a
digest of the queue arena they fill, so keys, links, record layouts and
allocation order are all covered.

Coverage: all 32 slots; every span row a handler can bind in both tables
(the test asserts the set), with material flags `0x00..0x13` in fifteen
combinations and both uniform and per-vertex shading; top, side and full
clipping; inset clip rectangles; quad splits; padded pitch; the
span-reciprocal overrun; and the zero fog-mask state.

## Open

- The retail stack address at the scan converter, which `00478510`'s pixels
  depend on, has not been measured in the shipped game; receipts use a fixed
  synthetic stack.
- The water, sky, model (face, group, billboard, edge) and particle
  producers are not ported; the backend draws no models yet. The software
  backend adapts RGBA-only port images to raw-texel materials and fades the
  surface with an 8-bit alpha blend; neither adapter is retail evidence.
- No full-frame comparison against the DirectDraw trace has been made with
  this raster.
