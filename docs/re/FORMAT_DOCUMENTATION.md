# V2000 (1998) File Format Documentation

## Overview

This document describes the file formats used by V2000 (1998) by Frontier Developments, based on reverse engineering of `V2000.EXE` (915 KB, 32-bit PE, MSVC compiled). V2000 is built on Frontier's C engine, FGDK, and is the direct predecessor of Infestation (2000), whose source tree was named `V3000`. Infestation runs on a C++ port of the same engine and shares the OVL container and many engine-level structures; see the [Infestation cross-reference](INFESTATION_CROSS_REFERENCE.md).

## OVL (Overlay) File Format

### File Naming Convention

Files are named `NxMxx.OVL` where:
- `N` = variant (0–3), stored in the first character
- `M` = level ID (0–52), stored between `X` and `XX`

```
overlay\%dx%dxx.ovl    →    0X14XX.OVL = variant 0, level 14 (Medaeval)
```

53 levels × 4 variants = 212 files total.

| Level Range | Category | Description |
|-------------|----------|-------------|
| 0–12 | System | Palettes, settings, UI, languages, animation, models |
| 13–50 | Game world | 38 playable levels (deathmatch arenas + campaign) |
| 51–52 | Misc | Additional data |

### OVL Loading Function (0x00493654)

Formats the path `overlay\%dx%dxx.ovl` and calls the file open chain. The main dispatch function `FUN_00493E40` (OVLDispatch) checks if a level is already loaded, calls the init vtable function, then invokes `SectionReader` (0x00493BF0).

### SectionReader (0x00493BF0)

Iterates all 15 sections sequentially:
1. Reads 4 bytes and verifies against `"abcd"` magic (via OVLHeaderVerify at 0x00493CE0)
2. Calls the handler's Load function from the vtable

Each handler's Load function reads a `uint32 block_size` from the file stream, allocates memory, reads `block_size` bytes, then processes the data. The `uint32` value field after `"abcd"` in the file equals this `block_size`.

### Section Handler Vtable (0x004CE0D8)

15 entries × 16 bytes (4 function pointers each: Load, Unload, Init, Cleanup).

> **2026-07-05 correction:** the original transcription of this table was
> misaligned for several rows. Rows marked ✓ were re-verified by dumping the
> table bytes from V2000.EXE (menu deep-RE pass; `tools/menu_re_render_dump.py`).
> Unmarked rows are the old readings — re-verify before relying on them.

| Section | Load Function | Record Size | Description |
|---------|---------------|-------------|-------------|
| 0 | 0x004AB920 | pointer array | Pointer fixup table (live display block) |
| 1 | 0x0048F130 | pointer array | Layout points (packed s16 x,y) — menu row positions |
| 2 | 0x00484260 | string table | Null-terminated ASCII strings |
| 3 | 0x004ABB80 ✓ | variable | Sprite atlas (size dword = unpacked atlas size) |
| 4 | 0x004ABA70 ✓ | 0x40 + tables | **Menu FONTS** (see Section 4 — old "parameter tables" reading was wrong) |
| 5 | 0x00471900 | 0x14 (20) | Display mode configuration (4 modes) |
| 6 | loader pending re-verification | 4 | Render shade lookup (26 packed entries) |
| 7 | 0x0046E400 | 4 | RGB555 color palettes (1–86 entries) |
| 8 | 0x00465C50 | variable | Model command streams (size dword = (model_count<<16)\|stream_bytes) |
| 9 | 0x00428A20 ✓ | 0xC00 (3,072) | Animation frame lookup tables (256 × 12B) |
| 10 | 0x00437E60 ✓ | 0x30014 | Terrain heightmaps (0x14 header + 256 × 256 × 3 bytes) |
| 11 | 0x004AB470 ✓ | 0x14 (20) | Sound effects (PCM blobs + parametric aliases) |
| 12 | 0x00410090 | 0x128 (296) | 3D collision volume models |
| 13 | 0x0042D880 ✓ | 0xD0 (208) base | Level descriptors + entity spawns |
| 14 | 0x004AB3B0 | 0x1C (28) | Entity/object linkage data |

### Global Constants

| Address | Value | Description |
|---------|-------|-------------|
| `0x004CE0A0` | 15 | Section count |
| `0x004CE0A4` | 53 | Level count |
| `0x004D0020` | — | Handler descriptor array (53 pointers to per-level structs) |
| `0x004D703D` | `g.dat` | Unknown global data filename |

### OVL Binary Layout

```
[Section 0]
  "abcd" [uint32 block_size] [block_size bytes data]
[Section 1]
  "abcd" [uint32 block_size] [block_size bytes data]
...
[Section 14]
  "abcd" [uint32 block_size] [block_size bytes data]
```

Data boundaries are determined by `"abcd"` marker positions. Each section's data spans from its marker + 8 bytes to the next marker (or EOF for Section 14).

---

## Per-Section Format Specifications

### Section 0: Data Pointer Fixup Table

**Handler:** `FUN_004ab920` — Reads `[uint32 block_size][data]`, then iterates `count` uint32 entries, adding the block base address to each (pointer relocation). No standalone asset value — runtime fixup arrays for data pointers.

**Entry format:** Flat array of uint32 relative offsets. Each is relocated at load time: `entry[i] += block_base`.

**Present in:** 2 levels (level 2: 6 entries/24 bytes, level 3: 18 entries/72 bytes), 4 variants each = 8 OVLs.

**Canonical decoder:** `v2k-formats::system`

---

### Section 1: Code Pointer Fixup Table

**Handler:** `FUN_0048f130` — Same pattern as Section 0. Reads block and fixes up internal pointers. Used for code/function pointer relocation.

**Entry format:** Flat array of uint16 values (packed pointer offsets). Entry count from PRELOAD.DAT; block size = count × 4 bytes.

**Present in:** 3 levels (level 2: 13 entries/52 bytes, level 3: 35 entries/140 bytes, level 51: 31 entries/124 bytes), 4 variants each = 12 OVLs.

**Canonical decoder:** `v2k-formats::system`

---

### Section 2: String Table

**Handler:** `FUN_00484260` — Reads block containing null-terminated ASCII strings. Used for level text, UI strings, and weapon/rank names.

**Present in:** System and game world OVLs. Over 1 million strings extracted across all 212 files.

---

### Section 3: Sprite Atlas

**Load handler:** `FUN_004ABB80` — reads the 28-byte sprite records, palette
data, and mixed indexed/raw atlas with 2048-byte stride. `FUN_004788D0` is a
masked software scanline filler that consumes a resolved sprite record; it is
not the Section-3 loader.

**Texel format and flags:** the low byte at `+0x04` selects rendering:
`0x01` keys zero, `0x02` selects raw RGB555 words instead of indexed bytes,
`0x04` selects fixed palette row 28 (`pal_offset + 0x380`) for indexed
unlit draws, `0x08` selects `source + destination/2`, and `0x10` selects
additive composition. A nonzero `pal_offset` at `+0x0C` points at a local
palette; for indexed sprites, `shade_count` at `+0x06` selects its layout.

The record's final dwords at `+0x14` and `+0x18` are runtime cache slots, not
authored colour metadata. Every one of the 15,064 records in all 48 Section-3
OVLs stores zero in both positions on disk. `FUN_004ABB80` relocates only the
atlas pointer at `+0x08` and the optional palette pointer at `+0x0C`, leaving
the zero tail intact. Section-3 unload (`FUN_004ABEB0`) later releases nonzero
runtime resources through `FUN_004979E0` (`+0x14`) and `FUN_00497A40`
(`+0x18`).

- **Indexed, `shade_count ≤ 16` → shade ramp.** RGB555, shade-major:
  `palette[shade_level·16 + pixel_value]`. **32 shade levels × 16 pixel slots**
  = 1024-byte block. Pixel values are 0–15; the sprite is drawn at a chosen
  shade level. Used by logos, UI, terrain sprites.
  Pixel value 0 is transparent only when render flag `0x01` is set. With that
  bit clear it is ordinary authored surface colour, independently of
  `shade_count`, including one-colour records.
- **Indexed, `shade_count > 16` → flat direct-indexed palette.** A run of
  `shade_count` RGB555 colours starting at `pal_offset`, indexed **straight
  by the pixel byte** (`palette[pixel_value]`, values 0..shade_count). NOT a
  shade ramp — there is no brightness selection. These are full-colour
  sprites: e.g. the menu **flame billboard** frames (global sprite ids
  1294–1299, 204×255, ~247 colours, pixel values 0–246) render as the red
  "2000" fire emblem. A shade-row offset would overrun the ~480-byte palette
  into the next sprite's block.
- **Raw, flag `0x02` → little-endian RGB555 words.** No palette or shade-row
  lookup occurs during intrinsic decoding. Expand R5/G5/B5 with `<<3`, so
  white is `(248,248,248)`. Bit 15 is discarded, not used as alpha.
  `004ABB80` registers `004ABDA0`, which passes converter `004AA2E0` to
  `004ABDD0`; raw entries convert each metadata-width row at 2048-byte stride,
  while indexed entries convert their palettes. The RGB565 display mapping is
  `((p & 0x7FE0) << 1) | (p & 0x1F)`. `004ABE80/004AA330` reverses the
  display conversion. Keyed raw filler `004762E0` skips a converted zero
  word: intrinsic alpha is zero iff flag `0x01` is set and
  `(p & 0x7FFF) == 0`. Otherwise black is opaque. Runtime raw lit/fog spans
  have separate packed RGB operations (`004763F0` adds interpolated light);
  shade-independent source decoding does not imply unlit presentation.

Software fillers `004783E0` and `00478510` use the interpolated high shade
byte as a 0–31 row and add a 0–15 texel index. The hardware path at
`0048DE50` independently consumes the same 32-level shade value.

**Entry boundary:** system level 51 contains 33 entries; its first 30 are
raw and have `pal_offset == 0`. Entries 30–32 point to offsets 924, 1948,
and 2972, with `924 == 33 * 28`. The metadata boundary is the earliest
plausible nonzero, 28-byte-aligned palette offset, not necessarily the first
entry's palette pointer. Parsing the entire 3996-byte block as records
fabricates 109 entries from RGB555 palette bytes.

**Atlas coordinates:** rectangle X bounds, widths and `tex_offset` are bytes;
the row stride is 2048 bytes. Indexed pixel width equals rectangle byte width;
raw pixel width is half that width. Metadata `+0x10/+0x12` stores pixel
width/height. All 129 raw records across the 48 Section-3 OVLs match this
relationship and have no palette. Their 301,822 source words have bit 15 clear.
Sprite 677 is opaque black 1×1 in every tier's X3 overlay; sprite 1319 is
4×8 in every X5 overlay. Copyright sprite 1292 is keyed 155×14 in
`0X5XX.OVL` only. X51 sprites 3733–3762 are 30 opaque level previews:
32×24 in tier 0 and 64×48 in tiers 1–3, including visible zero-valued black.

**Present in:** 48 OVLs (12 unique levels × 4 variants). Total: 15,064 sprites extracted.

---

### Section 4: Menu Fonts

> **2026-07-05 CORRECTION:** this section was previously documented as
> "difficulty-scaled entity stat tables (variant 0 = easy, 1.84× hard
> scale)". That reading was **wrong**. The real loader is `FUN_004ABA70`
> (handler table 0x4CE0D8 + 4×0x10, byte-verified; 0x437E60 is the Section
> 10 terrain loader). Section 4 exists only in level 2 — the system/menu
> OVL — and contains the **two menu sprite fonts**. The variant-0 vs 1–3
> differences are lo-res (320×240) vs hi-res (640×480) font scaling, not
> difficulty. Anything downstream that consumed Section 4 as entity stats
> inherited the misread; the retired Python entity catalog was removed rather
> than preserved as misleading reference data.

**Handler:** `FUN_004ABA70` — reads block, fixes up two pointer fields per
record (glyph metrics at +0x38, glyph sprite ids at +0x3C).

**Record format (0x40 = 64 bytes), 2 records = font 0 (GREEN, normal rows) and font 1 (YELLOW, selected row):**
```
+0x00  uint32  runtime_id     (48908 lo-res / 48917 hi-res)
+0x04  uint32  type           1 = sprite font
+0x08  uint32  glyph_count    256
+0x0C  uint32  cap_height?    80 lo-res / 170 hi-res (×1/100 px)
+0x10  uint32  scale_divisor  100 — running pen coordinates use
                              1/100-pixel fixed point
+0x14  uint32  line_advance   1041 lo / 2082 hi (÷100 px)
+0x18  uint32  line_gap       244 lo / 487 hi   (wrap step = (adv+gap)/100)
+0x1C  uint32  param_e
+0x20  uint32  param_f
+0x24  uint32  zero
+0x28  uint32  max_index      127
+0x2C  uint32  fallback_char  127 (0x7F glyph used for missing chars)
+0x30  uint32  zero
+0x34  uint32  zero
+0x38  uint32  metrics_ofs    → 256 × 8 bytes {s16 advance, s16 kern, s16 xoff, s16 yoff};
                              advance/kern are fixed-point pen units, xoff/yoff are signed pixels
+0x3C  uint32  sprites_ofs    → 256 × uint32 glyph SPRITE IDs (global Section-3 pool)
```

Font 1's sprite ids = font 0's + 173 for every glyph (same metrics). Sample
pen metrics (lo-res, ÷100): space adv 2.00 px; '0' 5.00 + kern 1.00; 'A' 6.00 +
1.00; 'M' 8.00 + 1.00. Offsets are not divided: high-resolution descenders
such as `p` and `y` carry `yoff=4` pixels. The old "default tuple
[943,−118,1,0]" is the unused-glyph metric. Glyphs 0x1B–0x1E = spinner-bar
characters ('[', empty, ']', filled block — L2 sprites 42–45 / yellow
215–218). Text draw pipeline, baseline rule and typewriter reveal: see
`MENU_SYSTEM.md` §Text rendering.

**Evidence:** `reference/menu_render_pass.json` retains the captured fonts,
metrics, and sprite ids. `v2k-formats::params` is the maintained Section-4
decoder; `menu_re_render_dump.py` now refreshes only the EXE-owned render tables
and preserves the captured OVL fields rather than embedding another parser.

**Present in:** 4 OVLs (level 2 × 4 variants). 2 font records each.

---

### Section 5: Display Mode Configuration

**Handler:** `FUN_00471900` — Reads `[uint32 block_size][data]`, stores pointer at global+0x24. Contains display resolution configurations available to the player.

**Record format (0x14 = 20 bytes):**
```
+0x00  uint32  width       screen width in pixels
+0x04  uint32  height      screen height in pixels
+0x08  uint32  reserved    always 0
+0x0C  uint32  param_1     display parameter (0x02000010 — bit depth + flags)
+0x10  uint32  param_2     display parameter (0x00010101 — mode flags)
```

**Display modes (4 records):**
| Index | Resolution | Aspect |
|-------|-----------|--------|
| 0 | 320×240 | 4:3 |
| 1 | 640×480 | 4:3 |
| 2 | 800×600 | 4:3 |
| 3 | 1024×768 | 4:3 |

All records share identical param_1 (0x02000010) and param_2 (0x00010101) values, indicating 16-bit color mode with consistent display flags.

**Present in:** 1 level (level 2 × 4 variants = 4 OVLs). Block size: 80 bytes.

**Canonical decoder:** `v2k-formats::system`

---

### Section 6: Render Shade Lookup Table

> **2026-07-14 correction:** this is not the terminal fog-colour source.
> Gameplay fog has separate render-context near/far words and terminates in
> the authored sky colour. The old `FUN_004708E0` loader attribution was also
> stale (`0x004708E0` lies inside an operand decoder); its loader address
> remains to be re-verified.

Contains packed render colour/shade lookup entries. Terrain construction
(`FUN_0042FCC0` / `FUN_00430140`) produces a 0–7 vertex shade. `FUN_00430430`
indexes the render-context table with that byte after `FUN_00431890` copied
the first eight Section-6 dwords into the context.

**Entry format (4 bytes):**
```
+0x00  uint8  R    red component (0–128)
+0x01  uint8  G    green component (0–128)
+0x02  uint8  B    blue component (0–128)
+0x03  uint8  shade_row  direct 0–31 row in the Section-3 RGB555 ramp
```

**Structure (26 entries = 104 bytes):**
- **Primary table (entries 0–7):** exact terrain/water shade-row mapping
  **`[7, 10, 14, 17, 21, 24, 28, 31]`**. The terrain software and hardware
  paths use the fourth byte; their first three bytes are not additive terrain
  RGB. The rasterizer interpolates these mapped 0–31 values across the face.
- **Transition (entries 8–9):** Bridge between ramps — (4,4,4) then (0,0,0), both shade level 7.
- **Ramp 2 (entries 10–25, fine):** 16 entries from (8,8,8) → (128,128,128) with shade levels 8→31. Grayscale gradient in 8-unit steps. Finer resolution for near-distance shading.

**Present in:** 1 level (level 2 × 4 variants = 4 OVLs). Block size: 104 bytes.

**Canonical decoder:** `v2k-formats::system`

---

### Section 7: RGB555 Color Palettes

**Handler:** `FUN_0046e400` — Reads `[uint32 block_size][data]`, stores pointer at global+0x30, then registers a display-mode-change callback. Contains the base color palettes used by the shade ramp system (Section 3 atlas references these as shade-level-0 colors).

**Entry format (4 bytes):**
```
+0x00  uint16  rgb555    15-bit color: RRRRR_GGGGG_BBBBB (bits 14:10 = R, 9:5 = G, 4:0 = B)
+0x02  uint16  padding   always 0x0000
```

**RGB555 decode:** Retail preserves the five authored bits as the high bits of
each byte: `R8 = R5 << 3`, `G8 = G5 << 3`, `B8 = B5 << 3`. It does **not**
replicate the high bits into the low three bits, so the maximum component is
248 rather than 255. This is visible in `FUN_004a84a0`, whose expanded channel
values are masked with `0xF8`.

**Palette sizes by level:**
| Level | Entries | Block Size | Description |
|-------|---------|-----------|-------------|
| 2 | 86 | 344 B | Master palette (terrain materials + UI + special colors + grayscale ramp) |
| 4 | 1 | 4 B | Settings/UI context (single black entry) |
| 6–11 | 48 each | 192 B | **Terrain biome palettes**: 8 groups × 6 colors (see Section 10) |

**Terrain biome palettes (levels 6–11):** Each of the 6 system levels defines a terrain biome color set. Groups 0–3 provide 4 LOD brightness levels with 6 surface type colors each; Groups 4–5 provide special/UI colors; Groups 6–7 provide a 12-step grayscale ramp. Game world levels (13–50) reference one of these biomes for their terrain rendering. See Section 10 for the full terrain texture pipeline.

**Level 2 master palette breakdown (86 entries):**
- Entries 0–14: Primary/secondary color channels (pure R, G, B at 3 brightness levels + RG, GB, RB combinations)
- Entry 15: Gray reference (123,123,123)
- Entries 16–31: Terrain material colors (greens, browns, cyans, grays)
- Entries 32–50: Grayscale ramp (19 shades from black to near-white)
- Entries 51–63: Special colors (magenta, orange, pure primaries, cyan, white)
- Entries 64–85: Game-specific colors (UI elements, minimap markers, entity highlights)

**Present in:** 8 unique levels (2, 4, 6–11), 4 variants each = 32 OVLs. Total: 329 unique palette entries.

**Canonical decoder:** `v2k-formats::system`

---

### Section 8: Model Sub-blocks (3D Mesh Data)

**Handler:** `FUN_004ab5b0` — Reads sub-blocks containing 3D model entries with vertices, normals, face render commands, and embedded name strings. Uses the shared FGDK sub-block format with a 12-byte entry header.

**Present in:** 36 OVLs (9 unique levels × 4 variants). 5,344 total entries, all named.

**`header_value` encoding:** Two packed u16 values:
- Low 16 bits: first sub-block alloc size (bytes of entry data)
- High 16 bits: total entry count hint

**Sub-block layout:**
- First sub-block: NO `[alloc][count]` header in the data. Alloc/count come from `header_value`. Entry data starts at data offset 0.
- Subsequent sub-blocks: `[u16 alloc_size][u16 entry_count][alloc_size bytes of entry data]`

**Entry Header (12 bytes) — CORRECTED 2026-07-13 (from render entry
`FUN_00464e60` and collision entry `FUN_00412530`):**

| Offset | Size | Field | Description |
|--------|------|-------|-------------|
| 0x00 | 2 | u16 | `cmd_word_count` — command stream length in words |
| 0x02 | 1 | u8 | `extra_count` — extra data dword count |
| 0x03 | 1 | u8 | `flags` — bit 6: if 0, embedded name string; bit 5: force near LOD table |
| 0x04 | 2 | u16 | `slot_count` — runtime vertex SLOT count = 2 × vertex count |
| 0x06 | 2 | u16 | `face_val` — normal pool size = `(face_val - 2) >> 1` |
| 0x08 | 2 | u16 | `radius` — LOD switch distance |
| 0x0A | 2 | u16 | `collision_radius` — active-model broad-phase radius, unsigned 8.8 units |

(The old "8-byte compact header + 4-byte gap" reading was wrong — the gap IS
the radius fields. The stride formula's `+6` words = this 12-byte header.)

`FUN_00412530` proves that `+0x0A` is collision data, not padding. It selects
each entity's active Section-8 model through the four model slots at entity
`+0xA8/+0xAA/+0xAC/+0xAE`, reads both models' unsigned `+0x0A` words, and
rejects a model whose word is zero. It adds the two radii for axis bounds and
a squared-distance sphere test before dispatching the authored model-collision
program through `FUN_0046AE40` / `FUN_0046AF20`.

**Stride formula (from loader `FUN_004ab5b0`):**
```python
vertex_half = slot_count >> 1          # actual vertex count
face_half   = (face_val - 2) >> 1      # normal pool entries
geom_size = (cmd_words + 6 + (extra_count + (vertex_half + face_half) * 2) * 2) * 2
# If flags & 0x40 == 0: add null-terminated name string + 1 byte
# Align total to 4 bytes
```

**Entry Data Region Layout:**
```
+0x00:   12 bytes              Header
+0x0C:   extra_count * 4 B     Authored collision bytecode
+vert:   vertex_half * 8 B     Vertices (int16 type_flag, X, Y, Z)
+norm:   face_half * 8 B       Normal pool (int16 anchor_slot, nX, nY, nZ)
+cmd:    cmd_words * 2 B       Command stream
after:   name string           Null-terminated ASCII (if !(flags & 0x40)), 4-aligned
```

#### Collision Program — CORRECTED 2026-07-13

The `extra_count * 4` region is not generic padding and is not part of the
render command words. `FUN_0046AF20` interprets it as a separate collision
program after the header-radius broad phase. Collision therefore must not be
reconstructed from the visible `MaterializedModel` triangles.

| Opcode | Aligned payload | Recovered behavior |
|--------|-----------------|--------------------|
| `0x88` | none | terminate; a final hit needs at least one raw unit of penetration |
| `0x8E` | align4: `u32 radius, i16 slot` | integer sphere centered on a resolved model slot; distance is `floor(sqrt(dx²+dy²+dz²))`, and tangent is sufficient for a gate |
| `0x8F` | align4: `u32 hx,hy,hz, i16 slot` | strict expanded AABB; center is transformed, extents stay query-axis aligned |
| `0x90` | align4: `u32 radius, u32 height, i16 slot` | upright cylinder rising `height` from the slot (`FUN_00469C20`, sphere-table `+4`). Its vertical span is strict on both ends and uses the cylinder radius, not the query's: `slot.y < q.y + radius`, `q.y - radius < slot.y + height`. It separates only horizontally: the normal is the XZ Q12 unit from the `i16` integer root, the penetration `radius + r - length`, and a zero length gives `(1,0,0)` |
| `0x8C` | align2: signed group span | begin convex plane group; span is relative to aligned payload |
| `0x8A` | align2: three `i16` slots | sphere/plane half-space; rejection branches to group end |
| `0x89` | align2: four `i16` words | same plane using first three slots; fourth word unused |
| `0x8B` | align2: one unused word | retain accepted callback, otherwise branch to group end |
| `0x8D` | none | commit accepted group's minimum penetration; compete with other primitives by maximum |
| `0x95` | align2: `i16 relative_offset` | continue on the previous primitive hit, otherwise branch from the aligned payload |
| `0x05` | align2: child id/orientation/span/attach/remaps | recurse into a collision-only child model hierarchy |

The child command is also valid on the **query** side. `FUN_0046AF20`
case `0x05` calls `FUN_0046B6D0`, which copies the selected callback context
and recurses into `FUN_0046AF20` with the child model, attachment-relative
translation, orientation and linked-slot remaps. This preserves the
`FUN_00469B70` model-sphere callback (or `FUN_00415160` terrain callback),
instead of interpreting only the parent's sphere/gate commands. The port's
shared query walker now follows the same collision-only hierarchy. A child
miss replaces the last callback scratch result, so a following `0x95` cannot
reuse a hit from an older sibling; final results still select maximum
penetration and retain the first equal-depth hit. The regression that exposed
the missing query path is the Intro2 Type66 factory, global model `210`,
against the Type10 dragon, global model `351` (child at query PC46). The
additional actual authored pairs are ordinary world15 factory `212` against
Type26 stag `267`, and world18 landing/base `285` against stag `267`. Those
ordinary pairs now complete the detailed geometry query without the prior
unsupported-child error. The Intro2 pair also completes its query; response
tests must choose penetration large enough to survive the actor family's
native fixed-point narrowing, rather than requiring every one-raw-unit
geometry hit to produce nonzero response displacement.

Five focused shared-format regressions pass: composed attachment/orientation
and imported slots (including model-query normals), sibling miss/unresolved
attachment scratch reset before gates, maximum penetration/first equal hit,
explicit mount6/missing-child/recursion limits, and nested terrain callback
preservation. The source-backed traversal does not widen actor eligibility or
normalize actor-family responses. Full crate/repository checks and actor-pair
acceptance remain recorded by the active objective and owning actor notes.

The supported child orientations compose the already recovered signed
permutation basis and parent frame. Orientation code6 depends on a preceding
`0x31` mount basis and remains an explicit unsupported error, as on the target
side. Child traversal does not broaden support to unowned query primitives
or render commands. The terrain entry accepting a model pool shares this
traversal; the legacy pool-free entry remains suitable for child-free player
geometry and reports unavailable children explicitly.

The collision-child hierarchy is distinct from render opcode `0x0E`. A census
of core global models 0..314 found 106 collision programs: all 106 use `0x8E`,
99 use `0x95`, 14 use child `0x05`, nine use `0x8F`, and ten use convex groups.
The first-world live set from the retail capture is player `41`, weight `81`,
Main Base `286`, and lifter factory `227`. Those programs use the recovered
`0x88/0x8E/0x8F/0x95/0x05` subset; the lifter's `0x05` child is global model
`179` (`factpole`).

Level 13's spike pen uses models `512/514/518/520/522/532/540/542`, all with
supported sphere/gate programs (`8E/95/88`). Collision deliberately keeps
type-12 aliases intrinsic: `FUN_0046AF20` installs table `0x004D4A78`, whose
type-12 entry `0x004D4B20` is the XYZ-copy handler `FUN_0046E9F0`. The separate
render table receives terrain-grounding handlers from `FUN_00433FA0`.
Consequently the pen's recursively derived collision slot 14 retains raw
Y=192 even when terrain grounding lowers its visible fence endpoints.
Do not apply a rendering alias callback to collision slots.

Ordinary fences `492..499` reach the supported convex group `8C/8A/8D`
after two sphere gates. The target sphere table `0x004D4920` selects
`FUN_0046ABF0`; the moving-model sphere callback `FUN_00469B70` installs this
same table for its nested target query. `FUN_0046A8B0` is the `0x89` wrapper.
Plane edges narrow to signed words before the integer cross product. The
highest magnitude bit selects a shift of 0/5/10/15/20 at boundaries
14/19/24/29; integer floor square root and signed division produce a Q12
normal. A zero length uses `(4096,0,0)`. Penetration is
`radius - ((query - first_vertex) dot normal >> 12)` and must be positive.
These are oriented half-spaces, not finite triangle-edge distance tests.
The first least-penetrating plane wins a tie; an empty group inherits the
previous successful primitive's depth/normal while retaining a separate
acceptance bit for `0x95`. Supported convex misses therefore remain misses
instead of falling back to the bounding sphere. Tests retain model493's
exact program and generated slots through sphere and model-pair queries.
Both fence families use static kind 9 and admit starting-gun damage; see
[static damage programs](ENTITY_STATIC_DAMAGE_PROGRAMS.md).

#### Command Stream — DECODED FROM GAME CODE (2026-06-09)

**Ground truth:** V2000.EXE has three 256-entry function-pointer dispatch
tables indexed by the command word: 0x4D3CE0 (render near), 0x4D40E0
(render far LOD — selected by distance vs header radius, entry flags bit 5
forces near), 0x4D44E0 (pass 3, transform-only — computes vertex positions
without emitting primitives). 0x4D55A0 (previously listed as "pass 4") is
NOT a command-stream table: it is the scanline rasterizer's edge-interpolator
function table (per-scanline span steppers/fillers). Render entry
`FUN_00464e60`; all handlers decompiled in `decompiled/cmd_stream_handlers.c`
(tool: `tools/ghidra_decompile_cmd_tables.py`). The dispatch uses the FULL
word value — valid opcodes are pure 8-bit; there are NO "9-bit base +
variant bits 9-10" (the old empirical model was wrong; its 42 "bases"
mixed real opcodes, opcode-family variants, and misparses — 0x012/0x09C
map to the DEFAULT handler, an infinite loop, so they can never appear).
The dispatch hangs on unknown opcodes, so streams are opcode-exact:
extraction reports ZERO unknown opcodes across all 36 OVLs.

**Runtime vertex slots:** 20-byte transformed-vertex slots, lazily computed:
slot 2i = vertex i, slot 2i+1 = X-NEGATED mirror copy. The per-vertex
`type_flag` (1st int16 of the 8-byte vertex record) selects the transform
function from a per-type table (see "Per-vertex type_flag transform table"
below). Vertex refs in commands are direct slot indices ("doubled"). Normal
refs are also doubled mirror refs, but 0/1 are an implicit runtime pair and do
not address stored data; stored normal record zero begins at refs 2/3.

Slot layout (20 bytes): +0x00/+0x04/+0x08 = view-space X/Y/Z (i32),
+0x0C/+0x0E = screen X/Y (i16), +0x10 = status byte (0 = untouched,
0x80 = view coords valid, low 7 bits = clip flags from projection:
0x40 = behind camera, x: 2 in / 4 right / 1 left, y: 0x10 in / 0x20 / 8),
+0x11 = fog/shade byte (far pass only). Face visibility = table lookup
`DAT_004C5268[OR of all corner clip flags]`.

**Half-model mirror instancing:** mirrored faces use SEPARATE OPCODES
(unlike Infestation's bit 10): the 0x_7/0x_8/0x27/0x28 families draw the
face twice — second time with every vertex AND normal ref XOR 1.

**Normal-pool face planes — CORRECTED 2026-07-17:** `FUN_0046D5A0`
initializes normal refs 0/1 as an implicit always-visible, Section-6-shade-zero
pair. For any ref `r >= 2`, the stored 8-byte record is
`normal_pool[(r >> 1) - 1]`, not `normal_pool[r >> 1]`. Its words are:

```
int16 anchor_slot, normal_x, normal_y, normal_z
```

Word 0 is an authored vertex-slot anchor, not a flag. `FUN_0046D3F0` resolves
that point and tests the face's authored plane before projection. The low mirror
bit is applied to both halves of the plane: odd `r` negates `normal_x` and
resolves `anchor_slot XOR 1`. After the model transform, translation, and scale,
the exact visibility condition is:

```
dot(world_normal, world_anchor - camera_position) < 0
```

Zero is rejected: a camera exactly on the plane does not draw the face. This is
why geometric triangle winding cannot substitute for the authored normal.

**Opcode table** (args = words after the cmd word):

| Opcode | Args | Meaning |
|--------|------|---------|
| 0x00 | — | end of stream |
| 0x02 | 3 | line/edge `[mat][v0][v1]` |
| 0x03/0x43/0x83/0xC3 | 5 | triangle `[mat][norm][v0][v1][v2]` (see mat note below) |
| 0x07/0x47/0x87/0xC7 | 5 | triangle + mirror instance |
| 0x04/0x44/0x84/0xC4 | 6 | quad `[mat][norm][v0..v3]` |
| 0x08/0x48/0x88/0xC8 | 6 | quad + mirror instance |
| 0x23/0xA3 | 8 | Gouraud triangle `[mat][norm][v0][v1][v2][s0][s1][s2]` |
| 0x27/0xA7 | 8 | Gouraud triangle + mirror |
| 0x24/0xA4 | 10 | Gouraud quad `[mat][norm][v0..v3][s0..s3]` |
| 0x28/0xA8 | 10 | Gouraud quad + mirror |
| 0x22 | 4 | line variant `[a][b][v0][v1]` |
| 0x0B | 2 | jump if normal backfacing: `[jump][norm]`, jump in words from args |
| 0x0C | 2 | jump variant `[jump][vertex]` |
| 0x13 | 3 | jump if `imm32 <= view_dist` `[jump][lo][hi]` (LOD select) |
| 0x14 | 3 | jump if `view_dist < imm32` |
| 0x2B | 3 | jump if `eval(a) != eval(b)` `[jump][a][b]` (animation frame select) |
| 0x2C | 3 | jump if `eval(a) == eval(b)` |
| 0x0D/0x1D/.../0xFD | 3 | variable ops on 64-entry register file at ctx+0x94 |
| 0x06 / 0x26 / 0x38 | scan | list to 0xFFFF: open unsorted group at max/min vertex Z; 0x38 is `4689D0` vertex-number diagnostics and skips slot resolution when `4FEEF4 == 0` |
| 0x46, 0x15 | 1 | vertex-Z group: 0x46 unsorted, 0x15 sorted |
| 0xA6 | 2 | unsorted group at fixed signed i32 key |
| 0xC6 | 3 | unsorted group at vertex Z + signed i32 offset; changes queue order, not geometry |
| 0x66/0x86/0xE6/0x10 | 0 | unsorted group at origin Z / fixed -1 / close group / other state |
| 0x0F | 0 | reset the current mount basis to Q1.31 identity (`FUN_00466fc0`) |
| 0x1C | 2 | compose axis rotation into the current mount `[axis][packed_angle]` (`FUN_004671D0` → `FUN_004671F0`); preserves the mount established by `0x5C` |
| 0x3C | 3 | mount basis from three resolved vertex slots (`FUN_00466cd0`) |
| 0x5C | 3 | mount orientation `[orient_code][axis][packed_angle]`: permute/negate parent basis, then optional axis rotation (`FUN_00466f80`) |
| 0x68/0xE8 | 4 | flat billboard `[slot][color_id][size][angle]` (FUN_00467840) |
| 0x78/0xB8/0xF8 | 4 | textured billboard `[slot][sprite_id][size][angle]` (FUN_00467c60) |
| 0x0E | var | inline instance of another model: `[?][model_id][skip_BYTES][attach_v]`; next cmd at args + skip_bytes |

**Face `mat` operand — DECODED 2026-07-06 (colouring):** the first face-command
operand (`mat`) is the per-face colour source, and the **opcode's 0x80 bit**
selects how it resolves (this is what the "variants" partly encode, alongside
cull mode):

- **bit 0x80 clear** (0x03/0x43, 0x07/0x47): `mat` is a **Section-7 palette
  colour index** (small, ≤85 — the level-2 master palette). Flat colour +
  face-normal directional shade.
- **bit 0x80 set** (0x83/0xC3, 0x87/0xC7 and the 0xA3/0xA4 families): `mat`
  is a **global sprite id** (large — a Section-3 pool id). The original
  passes the resolved sprite entry to a textured primitive rasterizer (for
  example 0x83 → vtable +0x1050 → triangle handler `FUN_0047C600`).
  `FUN_0047CBE0` is also a triangle handler, not the fourth-corner extension
  previously documented here. The model stream stores no explicit UV words:
  triangle handlers synthesize the upper/right domain
  `(0,0)`, `(Umax,0)`, `(Umax,Vmax)`, while quad handlers
  `FUN_0047EF10`, `FUN_0047F320`, and `FUN_0047F750` synthesize
  `(0,0)`, `(Umax,0)`, `(Umax,Vmax)`, `(0,Vmax)`. The exact triangle
  half is observable on `pesnthut` sprite 1365, whose ordinary palette-zero
  area becomes visible if the opposite half is sampled.

  Mirror-family handlers queue the authored refs once, then queue the same
  order with every ref XOR 1. Both passes restart the same implicit UV
  sequence; neither refs nor UVs are reversed. Screen-space handedness is a
  presentation transform, not a parser rule. The frontend applies its local-X
  model basis at the hierarchy root so `slopt` sprite 1305 reads `V 2000` and
  `optexit` 1230/1231 shows door-left/arrow-right while Klaus's asymmetric
  keyed wing art remains attached to its authored joints. Palette index zero
  is transparent only when Section-3 render flag bit `0x01` selects the keyed
  filler; otherwise it is authored opaque colour. Thus sprite artwork supplies
  real surface detail, not a
  representative flat colour — Klaus sprites 393–413 contain its bones, teeth
  and dark wing membranes.

Empirically (`0X3XX.OVL`): palette faces have mat 0–59, sprite faces 389–1225,
clean separation. Menu props are mostly sprite-textured (e.g. `flags` uses mats
1285–1289 = the L5 national-flag sprites). Gouraud `s0..s3` index the normal
pool (per-vertex lighting), so they must be retained alongside the implicit
UVs when quads are triangulated. Model light
vector = ctx dwords[15..17] = (0x49,0x49,−0x49). Port: `v2k-formats::models`
packs the sprite bit into `face_materials` bit 15 (`face_material()` decodes),
preserves continuous quad UVs and corner normals, and `v2k-game::model_color`
decodes/uploads the referenced sprite textures once.

**Billboard ops (0x68/0xE8, 0x78/0xB8/0xF8) — DECODED 2026-06-10:** these
are NOT mesh-vertex generators. They emit a screen-space rotated quad
(sprite/particle) anchored at the projected position of vertex `slot`:
`size` and `angle` are packed operands (FUN_00470840); the half-extent is
perspective-scaled by the slot's view depth (FUN_004594C0:
`px = size*xscale/z`, clamped to 0x1FFF), the quad corners are rotated by
`angle` (sine table DAT_004D14D0, 0x4000 = 90°), and the result is pushed
to the deferred render queue. 0x68/0xE8 fill with a flat color looked up
via ctx[0x1B](color_id) (device callbacks +0x1080 near / +0x1084 far+fog);
0x78/0xB8/0xF8 fetch a sprite via ctx[0x1A](sprite_id) and keep its aspect
ratio (width/height at sprite+0x10/+0x12, normalized by max dim × √2/2;
callbacks +0x1098/+0x109C). The opcode variants are aliases — all three
stream tables map each family to a single handler. Used for engine glows,
lights, and particle attachments (936 instances across all OVLs).

**Complemented immediate correction (2026-08-09):** the bit-7-clear packed
operand path applies one final `NOT` when operand bit `0x40` is set, after the
mask/rotate (retail `0x00470943..0x00470948`). The port formerly omitted that
step. In `bigfuel`, packed constant `0xF052` therefore decodes to `0xFFD0`
(-48), not `0x002F` (+47); subtracting it keeps the animated sprite-617 glow in
an ordinary positive range instead of wrapping near 65500, which the correctly
unsigned billboard consumer expanded into a roughly 512-world-unit additive
quad. The shared DD/ED path now uses retail's literal quarter-wave table,
repeated Q31 word and signed product extraction; `bigfuel` therefore retains
the exact30..65 range. The source arithmetic and controlled original-PE
regressions are recorded in [`NUMERIC_WRAP_AUDIT.md`](NUMERIC_WRAP_AUDIT.md). The Infestation canonical
decoders independently retain this same post-rotate complement rule.

#### Per-vertex type_flag transform table — DECODED 2026-06-10

Ground truth: `decompiled/vertex_transform_handlers.c` (tool:
`tools/ghidra_decompile_vertex_transforms.py`). The render context's
ctx[0x13] points to the table structure at **0x4D4A78** (.data):

| Offset | Contents |
|--------|----------|
| +0x00 | far-pass (table B) transform array — 15 ptrs, type_flag 0..14 |
| +0x3C | near-pass (table A) transform array — 15 ptrs |
| +0x78 | pass-3 transform array — 15 ptrs (transform only, no projection) |
| +0xB4 | world-point transform+project, with fog (FUN_0046D1E0) |
| +0xB8 | world-point transform+project, no fog (FUN_0046D010) |
| +0xBC | world-point camera transform only (FUN_0046CC20) |
| +0xC0 | project view→screen + fog byte, far pass (FUN_0046CEB0) |
| +0xC4 | project view→screen, near pass (FUN_0046CD90) |
| +0xC8 | perspective size scale w/ z-guard (FUN_0046CCF0) |

Dispatch site (identical in every opcode handler that touches a slot):
```c
rec = ctx[0x22] + (slot >> 1) * 8;                    // 8-byte vertex record
slot_ptr = (*(ctx[0x13] + PASS_OFF + rec->type_flag * 4))(ctx, slot_ptr, slot, rec);
```
Projection: `sx = x*xscale/z + cx`, `sy = cy - y*yscale/z` → the record
coordinate order is **(type_flag, X, Y, Z)**, X = right, Y = up,
Z = forward/depth (the old "(flag, Y, Z, X)" reading was wrong). The
transform applies the camera 3×3 (ctx[6..14], 1.31 fixed point) +
translation (ctx[2..4]); both slots of a pair are computed together —
the odd slot negates (or zeroes, tf 4) the X *contribution*, i.e. mirror =
model-space (-x, y, z). When a generator record is reached through an odd
slot, all its slot refs are XOR'd with 1, so mirroring propagates through
generator chains.

type_flag semantics (record fields: a = +2, b = +4, c = +6; "slot N"
refs are slot indices XOR'd with the requesting slot's mirror bit):

| tf | EXE count | Meaning |
|----|-----------|---------|
| 0 | 73,404 | plain vertex (a,b,c); mirror slot = (-a, b, c) |
| 1 | 0 | screen-space midpoint of slots b and c (avg screen X/Y, min depth; always uses far-pass transforms for its refs) |
| 2 | 4 | point reflection of slot c through the model origin (= model -v) |
| 3 | 72 | slot c + per-frame random jitter; magnitude = rand i16 >> a (LCG FUN_00457930) — flicker effects |
| 4 | 0 | plain vertex; mirror slot = (0, b, c) (pinned to symmetry plane) |
| 5 | 2,132 | 3-D midpoint of slots b and c |
| 6 | 620 | parallelogram completion: slot a + slot b − slot c |
| 7 | 172 | vector sum: slot b + slot c − origin (model: B + C) |
| 8 | 1,956 | fixed-point lerp(slot b, slot c), effective t = eval(a)/0x10000 — exact axis expression `A + 2*(((B-A)*eval)>>17)` (`FUN_0046F7D0`/`0046F8F0`/`0046FA20`) |
| 9 | 264 | cubic Bézier, P0 = slot b, P1 = slot c, P2 = slot c+2, P3 = slot b+2, t = eval(a)/0x10000; exact power-basis evaluation uses three signed fixed-point stages whose linear/quadratic/cubic contributions are quantized to multiples of 2/4/8 (`FUN_0046FB80`) |
| 10 | 0 | re-evaluate the generator record of vertex (a, byte) at phase eval(a)+packed(b,c), wrapping ×0x4000 — track follower; referenced records of type 0x13/0x14 are linear tracks (lerp refs +4/+6 by their own var +2) |
| 11 | 1,684 | vertex imported from the linked model at ctx+0xC0 (slot index remapped via table at ctx+0x90; saves/restores the global camera block); no link → model origin. Used with op 0x0E inline instances |
| 12 | 2,808 | alias of slot a (same position — separate shade/material per face set). Default handlers `FUN_0046E9F0`/`FUN_0046EA60`/`FUN_0046EAE0` copy X/Y/Z exactly; world setup additionally swaps this entry for a terrain-grounding handler — see the static-extraction caveat below |
| 13 | 7,036 | slot a with absolute view-space Y forced to signed −32767 in 8.8 units (−32767/256 world) while preserving view X/Z — "sky pin" (screen-vertical support/beam) |
| 14 | 1,908 | vertex in an external attachment frame: callback ctx+0x5C transforms (a,b,c) from another object's space, then re-bases into this model's frame (world-anchored points: contrails, tethers) |

`eval(v)` (FUN_00470700): `v&0xC0` = 0x00 → `v&0x3F`; 0x40 →
`(v&0x3F)<<10`; 0x80 → dynamic callback ctx+0x58 (entity-driven, e.g.
animation time); 0xC0 → register file u16[64] at ctx+0x94 (written by the
0x_D opcode family). Records with type 0x13/0x14 never appear in shipped
V2000 data, nor do tf 1/4/10 (counts above are across all 36 OVLs).

For entity draws, `FUN_004138F0` reads the dynamic callback/data pair through
the active entity type record at `+0x78`; `FUN_00465870` copies that pair into
render context `+0x58/+0x5C`. The callback address can therefore be identified
from a read-only type-record trace. It must not be confused with the unrelated
global render-direction triple `DAT_004FEC90/94/98` installed at model state
`+0x3C/+0x40/+0x44` by `FUN_00433FA0`.

Packed operands (jump comparands, factors) decode via `FUN_00470840` —
same rotate-immediate / variable / callback scheme as Infestation
(bit 7+6: register file; bit 7 only: dynamic callback; else rotate-left of
`val & 0xFF03` by `(val>>2) & 0xF`, followed by one's-complement when low-byte
bit `0x40` is set).

**Extraction:** `v2k-formats::models` is the canonical command-stream
interpreter and `v2k-extract models` serializes its diagnostic interchange
output (variables = 0 → animation frame 0; view_dist = 0 → near LOD branch;
mirror instancing materialized as model-space (-x, y, z) for odd slots;
generated vertices resolved in model space via the type_flag table — the
*affine* generators (tf 2/5/6/7/8/9 and the intrinsic tf-12 alias — midpoint,
parallelogram, sum, lerp, Bézier, alias) commute with the camera transform
when their inputs are intrinsic. World tf-12 inputs require the presentation
callback before dependent generators resolve; see
[world alias dependencies](RENDER_PIPELINE.md#world-alias-dependencies)).
Billboard ops are captured as anchored point primitives with id/size/angle
in OBJ manifests. Export with `cargo run -p v2k-extract -- models`.

> **Static-extraction caveat — the view-dependent generators DO NOT resolve
> to a fixed model-space point** (they are evaluated *after* the camera
> transform, so they are camera/runtime dependent and cannot be baked into a
> static mesh). Static exports necessarily approximate them; the live renderer
> evaluates the recovered runtime policy:
> - **tf 13 "context projection".** The generated record selects a source
>   slot, but the active draw context determines its endpoint. The default
>   `FUN_0046EB60` / `FUN_0046EBD0` / `FUN_0046EC60` family keeps slot *a*'s
>   view-space X/Z and forces absolute view-space **Y = −32767** in engine 8.8
>   units (`−32767/256` world). World setup instead installs
>   `FUN_004349C0` / `FUN_00435090` / `FUN_00435780`, which project the source
>   onto terrain, animated waves, or seabed along the fixed Section-10
>   direction. Referenced by **15,296 drawn face-corners across
>   2,100 of 5,344 models (~40%)** — pervasive because the game is full of
>   vegetation with stalks/stems (worst: `lilly2`, `seedpod1`, `clam`,
>   `virusedlilly*`). The extractor returns slot *a*'s **full** position, so
>   every stalk **collapses to zero length** (e.g. `lilly2` extracts as a
>   flat disc at Y=3.9 with no stem). Even hero props are touched: `player4`
>   has 24 tf-13 corners. The live port resolves either explicit callback
>   policy per frame. World projection performs one source-surface X/Z
>   correction and one endpoint resample; the inclusive `sea +/- 0x96` band is
>   a rejection interval, not a finite shadow-length cap.
> - **tf 12 "world alias grounding" (recovered 2026-08-24).** `FUN_00433FA0`
>   swaps the tf-12 entries of all three pass tables as well —
>   `FUN_004340B0`/`FUN_004343A0`/`FUN_004346B0` at table offsets that decode
>   exactly to type index 12 next to the type-13 family. The swapped handler
>   copies slot *a*'s position, rotates it into object-relative world offsets,
>   replaces Y with the plain `FUN_00445860` bilinear terrain sample at that
>   world X/Z relative to the object origin, and rotates back. No slope
>   correction, wave surface, or sea-band clip applies. This is what roots the
>   planar tree-trunk/shadow quads (`bigtree*`: record 1 aliases the crown
>   anchor, so the trunk base lands on the sampled ground) while default/menu
>   contexts keep the pure positional alias.
> - **tf 11 "linked-model import" → origin (0,0,0).** No linked context
>   exists statically (it is set by op 0x0E inline instancing via ctx+0xC0),
>   so these collapse to the model origin, producing **spikes to center**.
>   6,516 face-corners across 432 models; ~1,720 of 5,344 OBJs contain an
>   exact-origin vertex. Resolvable at render time when the parent supplies
>   the instance (the port exposes `ModelEntry.instances`).
> - **tf 14 "external attachment frame" → raw (a,b,c).** Coordinates are in
>   another object's frame (contrails/tethers); placed at face value here.
>   316 face-corners across 276 models. Minor.
> - **tf 1 (screen-space midpoint) / tf 3 (per-frame RNG jitter)** are also
>   view/time dependent; tf 1 is unused in shipped data and tf 3 (72 recs) is
>   frozen at its base position.
>
> Net: vehicle/structure models (mostly tf 0 + affine generators) extract
> faithfully, while static organic/plant exports remain flattened because their
> stalk/beam geometry is view-dependent. The faithful live path lives in the
> **renderer**, not the extractor: it carries the per-vertex
> "pin"/"linked"/"frame" flag and evaluates tf 13 through the callback family
> selected by the live draw context; tf 11/14 retain their separate runtime-
> context requirements.

The renderer exposes callback selection as explicit `CameraFacing`,
`WorldSurface`, `Raw`, and `Disabled` policies. `CameraFacing` applies the
default absolute-view-Y family. `WorldSurface` applies the callbacks installed
by `FUN_00433FA0`: `FUN_00433BD0` derives camera-independent Q1.31 slopes from
the Section-10 direction, saturating each axis at magnitude one, then each
corner receives one X/Z correction and one terrain/wave/seabed resample. Broad
gameplay/Intro2 draws now select this proven world policy and fail closed when
live terrain is unavailable. Applying `CameraFacing` there was the source of
the 2026-07-19 dragon/tree rods; no model-name rule or endpoint cap is needed.

Canonical materialization retains mixed and all-type-13 faces in the ordinary
face stream with their original opcode-derived material, UVs, normals, shading,
and cull plane. The legacy `shadow_triangles` collection is empty. `WorldSurface`
retains each face's authored blend/depth behavior and re-establishes `LEQUAL`
for potentially coplanar projected faces. It adds no categorical type-13
depth-write suppression. Only mixed faces under the legacy `CameraFacing`
compatibility policy remain read-only colour underlays.

Ordinary textured model faces retain the source sprite's Section-3 material
flags. `FUN_0047EF10` selects additive (`flags & 0x10`), `source +
destination/2` (`flags & 0x08`), or masked fillers for this path; this is not
restricted to 0x78-family billboard records. The live body pass therefore
composites those families with the same blend policy and leaves the two blended
families non-depth-writing. Flag bit `0x01` independently selects palette-index-
zero keying; with it clear, index zero is authored opaque colour. For unlit
faces (`03/04/07/08`, textured `83/84/87/88`), flag `0x04` selects the fixed
palette row at `+0x380` bytes (row 28); otherwise the palette base is row 0.
Uniformly lit faces (`43/44/47/48`, textured `C3/C4/C7/C8`) instead resolve one
Section-6 value from the header's face-normal reference. Gouraud families
(`23/24/27/28`, textured `A3/A4/A7/A8`) resolve the trailing corner-normal
references. Both lit families ignore the fixed-row flag. Reserved normal
references 0/1 and stored zero normals select the contextual light table's
slot 0; they do not represent an upward normal. See
[model light table](RENDER_PIPELINE.md#model-light-table) for the consumers.
In retail `bigtree1`, mixed-pin sprite
1448 is `0x0D`, while complete-tree face 1402 and crown billboard 1433 are
`0x05`. Authored face order remains authoritative.

The static-terrain type-14 callback is also bounded exactly. `FUN_00427070`
initializes the persistent static draw context at `0x004CA3C0`, whose `+0x40`
slot is `0x00427050`. `FUN_00427090` stores the current cell-centre position
pointer at context `+0x48`; `FUN_00464F90` copies those fields to model-render
context `+0x5C/+0x64`. The callback at `0x00427050` ignores type-14 operands
`(a,b,c)`, copies the current object's world X/Z, and returns absolute world
Y=0. The live renderer therefore supplies that one world point to every
type-14 vertex in the complete static hierarchy, then inverses it through each
node's GL model matrix. Other render contexts retain `Raw` until their distinct
callbacks are recovered. This path is unrelated to the medieval tree-trunk
gap: `bigtree1` contains no type-14 record; its body uses mixed type-13 faces
plus three crown billboards.

> **Winding is NOT consistent — use the authored face plane.** Every face op
> runs `FUN_0046D3F0`'s anchor-plus-normal plane test before projection; screen
> winding is never consulted. Consequently the artist data is **not** wound to
> any global convention — measured ~50/50 agreement between geometric winding
> and the supplied pool normal (e.g. `player4`: 29 agree / 27 disagree). A port
> that enables winding-based backface culling (`GL_CULL_FACE`) wrongly drops
> about half of a model's camera-facing faces. The Rust port therefore renders
> two-sided at the API level and applies the recovered strict authored-plane
> test explicitly in world/camera coordinates.
Results: 36 OVLs, 5,344 entries, **124,100 canonical triangles** (including the
2,992 all-type-13 faces formerly split into the legacy shadow collection),
3,152 edges, 912 billboards, and 1,608 child instances (old empirical parser:
84,896 tris — it missed mirror instances, misparsed streams, and read
generator records as raw coordinates), 0 unknown opcodes/type_flags. The old Python OBJ snapshots were
removed after the Rust parser became canonical, as were the old "vertex
processing commands", "Layout A/B", and "inline Gouraud normal" sections
formerly documented here (the "inline normals" were misread Gouraud face
records; per-vertex shade refs are the trailing words of 0x23/0x24-family
records, and the normal pool is the dedicated 8-byte-entry region).
All 124,100 faces carry a parallel face-plane policy in Rust materialization;
the legacy shadow collection is empty and the complete corpus resolves every
authored source (zero `Unresolved` policies).


### Section 9: Static Terrain-Object Descriptor Tables

**Handler:** `FUN_00428a20` — Reads block, assigns entries at stride 0xC00
(3,072 bytes). Each entry is a complete 256-record terrain-object table.

**Block size:** Always 3,072 bytes (256 × 12-byte records).

**Record format (12 bytes):**
```
+0x00  uint16[4]  model_ids     4 global Section-8 model ids
+0x08  uint32     kind_index    index into the executable's 44-byte object-kind table
```

The old `frame_quad/count/padding` interpretation was wrong. The apparent
zero `padding` was simply the high word of small `kind_index` values.

**Terrain lookup:** A nonzero Section-10 cell `attribute` selects one of the
256 descriptors. Both `FUN_0042F650` (rendering) and `FUN_00427410`
(collision) select its live model with `(terrain_type >> 3) & 3`. Attribute
zero means no static object. The four slots encode authored object state or
variant models; common layouts include `[A,A,A,A]`, `[A,B,A,B]`, and four
distinct ids.

The object root sits at the cell centre. Rendering uses raw X/Z `+0x80` while
the collision helper uses `+0x7f`; Y is the signed average of the cell's four
corner height bytes, with each byte representing 32 engine-world units. The
root orientation is the identity matrix. Collision then evaluates the selected
Section-8 model's authored collision program.

**Entries 251–255:** Shared across all 6 levels. All four slots use global
model 39; their object-kind indices are 26→22.

**Not the terrain texture table:** The opaque terrain pass independently takes
the low three bits from four adjacent cells, encodes the four 0–4 material
codes in base five, and maps that 625-way combination to one of 120
D4-canonical sprites rooted at Section 13 `+0x4C`. Section 9 instead describes
the model placed *on* a cell.

**Present in:** 6 system OVLs (levels 6–11 × 4 variants = 24 files). Cross-level byte similarity ranges 61–86%, with 10 records identical across all 6 levels and 47–99 unique per level.

**Canonical decoder:** `v2k-formats::anim_frames`

---

### Section 10: Terrain Heightmaps

**Handler:** `FUN_004277c0` — Reads block, assigns entries at stride 0x30000 (196,608 bytes = 256 × 256 × 3).

**Record size:** 196,628 bytes = 20-byte header + 196,608 bytes grid data (256 × 256 × 3 bytes per cell).

**Storage order:** cells are X-major, then Z: `index = x * 256 + z`.
`FUN_00445860` computes this address directly before bilinear interpolation.
Treating the bytes as conventional `z * 256 + x` row-major storage transposes
the complete world and separates authored entities from their terrain.

**Header (20 bytes = 5 × int32):**
```
+0x00  int32   sea_level         SEA LEVEL, as worldY × 256 (24.8 fixed; read everywhere
                                 as >>8). NOT a "world_y_offset" — this is the flat water
                                 plane's height. Mutable at runtime (Flood mechanic writes
                                 it directly). See "Water rendering" below.
+0x04  int32   constant_1        Always -73 (0xFFFFFFB7) ┐ constant vector, copied (X,Z /4)
+0x08  int32   constant_2        Always 73 (0x49)         │ into render ctx by FUN_0042ea30;
+0x0C  int32   constant_3        Always -73 (0xFFFFFFB7) ┘ likely light/sun direction
+0x10  int16   dark_start_y      Underwater darkness ramp: start world-Y (FUN_004336e0)
+0x12  int16   dark_range        Underwater darkness ramp range (0 = disabled)
```
(The old "+0x10 int32 packed_flags" was two int16 darkness-ramp fields, not opaque flags.)

**Per-cell format (3 bytes):**
```
byte 0:  height         Elevation value, SIGNED i8 → worldY = (i8)height × 32
                        (range -4096..+4064). Read as signed char in FUN_00445860.
byte 1:  attribute      Static terrain-object descriptor index in Section 9. Zero means
                        no object. This is NOT the water source (water = sea_level plane).
byte 2:  type           Packed corner material + shade/control byte; see below.
```

**Water rendering (decoded 2026-07-02, adversarially verified):** V2000 water is the
Zarch-heritage **flat sea plane** at `sea_level >> 8`, drawn as a second render pass after
terrain (`FUN_0042f270` → water pass `FUN_00431a40`) **only if `(sea_level >> 8) > -4096`**.
"Dry" levels park the plane at -6144 (below the -4096 terrain floor) to disable it. The
surface is wave-displaced (`FUN_00445920`: 3 summed sines from the quarter-wave table at
`0x4D14D0`, seamless across the world wrap), textured with a marching-squares shoreline
(corner-submerged code → 16-entry table at `0x4CACC8`, 5 frames), and blended ~50% in the
16bpp rasterizer. Underwater (`camY < sea_level`) swaps in a screen-space shimmer
(`FUN_00433fa0`) plus the darkness ramp above. Physics uses `ground = max(terrain, wave)`
so craft ride the surface. The Flood mechanic is the AI behavior "Change Sea Level"
(`FUN_004013A0`), which adds a rate-capped delta straight into this header word each tick.
Per-level sea levels and the remaining environment work are tracked in
[`WATER_WORLD_TODO.md`](../../WATER_WORLD_TODO.md).

**Terrain type byte encoding (byte 2), corrected from the draw code:**
```
Bits 2-0:  Corner material code (0–4 in every shipped terrain grid)
Bits 4-3:  Section-9 static-object model-state slot
Bit 4/0x10: Terrain infection flag. The same authored bit participates in the
            static-object state selection above, enables coordinate jitter
            before projection, and selects the infection overlay pass
Bits 7-5:  Base shade (0–7), adjusted by the local 32×32 lighting table and
           underwater darkness, then clamped to 0–7
Bit 3:     Used by the terrain-scatter/object path; exact semantic still open
```

**Terrain texture pipeline:**
```
1. Four corner codes `(type & 7)` form a base-5 index in 0..624.
2. `FUN_00433180` tests the eight rotations/reflections, maps that combination
   to one of 120 canonical transition sprites, and supplies its UV permutation.
3. Section-13 `+0x4C` is the GLOBAL sprite-pool base for those 120 sprites.
   Five fixed-row infection shapes at base+120..124 feed the terrain bit-`0x10`
   overlay; five shoreline shapes at base+125..129 feed the second water pass.
   Both overlays use the same 16-case marching-square rotation table at
   `DAT_004CACC8/CC`.
4. Per-corner `(type >> 5) + local_light - underwater_darkness` selects the
   0..7 shade input used while rasterizing the selected sprite.
5. Biome sprite/palette resources come from system levels 6–11:
   Level 6=Medaeval, 7=Colorado, 8=Alpine, 9=SunnyIce, 10=Volcano, 11=Alien
```

Section 9 still supplies animation metadata for biome resources, but it is not
the direct opaque-quad lookup previously documented here. The opaque pass is
`FUN_0042FCC0` → `FUN_00430430`; its sprite id comes from the 625→120 table at
`DAT_004DB270`, not `Section9[type]`.

**Section 7 biome palettes (48 entries):** the 48-entry grouping is real, but
the old "height chooses one of four LOD groups / classes 6–7 are grayscale"
interpretation was a visualization-tool hypothesis, not executable behavior.
Do not use it to color terrain. The detailed opaque pass selects a transition
sprite and carries an independently calculated 0–7 shade at every corner;
the exact role of all 48 Section-7 entries is being revalidated against the
sprite shade/material binding path.

**Cross-OVL data loading:** Terrain rendering requires data from multiple OVLs loaded at
different times. Game level OVLs (13–50) provide Section 10 terrain cells but have no
Section 7 palettes, Section 9 animation tables, or Section 3 sprite data. These come
from PRELOAD.DAT system OVLs (levels 0–5 for shared sprites) and biome OVLs (levels
6–11 for terrain-specific palettes, animation frames, and biome sprites).

**World-resource assignments (Section-13 `+0x48`):**

| `+0x48` style / system OVL | Levels |
|----------------------------|--------|
| 1 / `0X6XX` | 13 (Peasant), 14 (Medaeval), 15 (Castle), 25 (Arena1), 39 (Dragon1), 44 (Arena6), 48, **50 (Intro2)** |
| 2 / `0X7XX` | 16 (Colorado), 27 (Arena2), 31, 35 (Marbles), 36 (Desert Island), 40 (Plateau2) |
| 3 / `0X8XX` | 18, 20, 21 (Flood), 24, 26 (VSpread), 28 (Arena3), 37 (Plateau), 49 (Intro1) |
| 4 / `0X9XX` | 17 (Alpine), 19 (SunnyIce), 29 (Arena4), 32 (WindyIce), 38 (Reactor), 45 (WChannel) |
| 5 / `0X10XX` | 41 (Arena5), 42 (Alien1), 43 (Volcano), 46 (Alien2), 47 (Alien3) |
| 6 / `0X11XX` | 22 (Water), 23, 30, 33 (Pchannel), 34 (DarkReef) |

`FUN_0042E570` loads exactly one resource overlay at `5 + descriptor[+0x48]`;
this is data-driven rather than a hardcoded level-number map. Intro2 is the
important correction to the earlier auto-detection: retail explicitly loads
`0X6XX.OVL` beside `0X50XX.OVL`; routing it through pack 8 produces palms,
pyramids, and hazard-strip terrain objects instead of the authored medieval
village.

**Present in:** 38 game world OVLs (levels 13–50, variant 0). Total: 38 heightmaps.

**Canonical decoder/viewer:** `v2k-formats::terrain`; `v2k-viewer --mode terrain`
(LOD palette rendering with biome comparison grids)

---

### Section 11: Sound Effects

**Handler:** `FUN_004AB470` (handler table 0x4CE0D8 + 11×0x10; the old
"0x00425FE0" attribution was wrong — that address is mid-function game code).
Reads block containing a table of 0x14-byte entries followed by PCM blobs,
patches each entry in place, and appends entry pointers to the **global
sound pool** `DAT_004FE64C` (only levels 2 and 3 contribute: L2 = global ids
0–6, L3 = ids 7–109; `global_id = level_base + local_index`).

**Entry format (0x14 = 20 bytes), corrected 2026-07-05:**
```
+0x00  uint32  type        0=unused, 1=PCM data blob, 5=parametric alias
+0x04  uint32  ofs/target  type=1: relative offset to blob data;
                           type=5: GLOBAL pool id of the target entry
                           (loader rebases: ptr = block + (id − level_base)·0x14;
                           alias→alias chains are legal)
+0x08  uint32  size/var    type=1: blob size in bytes; type=5: freq_variance (16.16)
+0x0C  uint32  freq_mul    type=5: FREQUENCY multiplier (16.16; playback Hz = 22050 × product of chain)
+0x10  uint32  vol_mul     type=5: VOLUME multiplier (16.16; applied if ≠ 0x10000)
```
(The old doc had +0x0C/+0x10 as "freq/pitch" — +0x10 is the volume
multiplier, applied in FUN_00495480.)

**Audio format:** 22050 Hz mono 16-bit signed PCM (confirmed from WAVEFORMATEX at V2000.EXE:0x004D6740 and DirectSound buffer creation code). Same format as Infestation Handler 9.

**Type=5 entries** are parametric sound descriptors that reference another
entry by GLOBAL pool id and apply frequency/volume modulation at playback.
Randomized frequency variation at trigger time (FUN_00495480):
`freq += ±((rand16 × (freq × freq_variance)) >> 16)` with pseudo-random sign
— e.g. the menu prop whoosh (global id 0x57) plays at 0.7 × 22050 Hz ± 20%
on every trigger. Menu sound ground truth (exact id→entry mapping, volume
formula, exported WAVs): see `MENU_SYSTEM.md` §Menu sounds. Canonical bulk
Section-11 export is provided by `v2k-extract sounds`; menu call-site analysis
is maintained in `MENU_SYSTEM.md` rather than as a second generated sound dump.

**Present in:** System OVLs (levels 2–3 × 4 display/resource variants). The
variants are alternate copies, not extra runtime pool entries. The canonical
pool uses PRELOAD's system-level-2 table (3 PCM + 4 aliases) followed by base
`Overlay/0X3XX.OVL` (49 PCM + 54 aliases): 110 global slots and exactly 52 WAV
files (39.7s total duration, 72ms–2.5s per sound).

**Sound durations:** Range from 72ms (short clicks/impacts) to 2,532ms (ambient/engine loops). Most sounds are 200–1000ms (weapon fire, explosions, mechanical effects).

**Canonical decoder/exporter:** `v2k-formats::anim_sound::SoundPool` resolves
GLOBAL alias chains for both playback and tooling; `v2k-extract sounds` writes
one `sounds/manifest.json` with global/local/source identity and modulation
metadata plus the 52 PCM WAVs.

#### Sound Playback Pipeline

```
Game event (collision, weapon fire, entity spawn, etc.)
  → FUN_004958c0 (SoundEffect_Play)
    → FUN_00495480 (SoundBuffer_Create)
      if type=5: resolve alias recursively, apply freq modulation
      if type=1: create DirectSoundBuffer from PCM blob
    → FUN_004957c0 (SetParams): frequency, pan, volume
    → Insert into active sound linked list (DAT_004fbe60)

Max simultaneous sounds: 24 (0x18)
Master volume: DAT_004fbe6c (0x00000–0x10000)
```

**DirectSoundBuffer control functions:**
| Function | Address | IDirectSoundBuffer Method |
|----------|---------|--------------------------|
| FUN_004ad0a0 | 0x004AD0A0 | SetFrequency (vtable +0x44), clamped 100–100000 Hz |
| FUN_004ad0d0 | 0x004AD0D0 | SetPan (vtable +0x40) |
| FUN_004ad0f0 | 0x004AD0F0 | SetVolume (vtable +0x3C) |

#### CD Audio (Music)

10 Red Book CD-DA tracks (tracks 2–11) controlled via MCI string commands. Functions:
| Function | Address | Purpose |
|----------|---------|---------|
| FUN_00496460 | 0x00496460 | CD audio init (MCI open, set time format) |
| FUN_00496730 | 0x00496730 | Check play status ("status cdaudio mode") |
| FUN_00496890 | 0x00496890 | Query position ("status cdaudio position") |
| FUN_004961d0 | 0x004961D0 | CD audio shutdown (MCI close) |

**Globals:** DAT_004fbf94 = initialized flag, DAT_004fbf98 = MCI device ID.

**Status:** CD audio is disc media and intentionally lives outside the source tree.

#### Animation System

Mesh animation is program-driven: model streams select and move geometry with the register-file operations and the `0x2B`/`0x2C` frame-select jumps described above. Neither V2000 nor Infestation stores frame-marker records. Entity-level animation is driven by:

1. **Movement patterns:** 100-entry displacement lookup table at DAT_004cd4b8 (6 bytes per entry: int16 x, int16 y, int16 z). Frame counter DAT_004de840 cycles via `counter % 100`. Displacement is scaled by distance and normalized.

2. **Section 9 frame tables:** 256 entries × 0xC00 (3,072) bytes each. Loaded by handler FUN_00428a20. Used for terrain and entity sprite frame selection.

3. **Entity animation blocks** (Section 13, optional): Only ~2% of entities have animation data. Block format:
```
+0x00  uint32  timing_param_0     (e.g., 70000 — microseconds?)
+0x04  uint32  timing_param_1     (e.g., 10000)
+0x08  uint32  timing_param_2     (e.g., 5)
+0x0C  uint32  timing_param_3     (e.g., 5)
+0x10  uint32  frame_count        number of animation frames
+0x14  uint32  frame_data_ptr     relocated pointer to frame array
Frame data: frame_count × 0x1C (28) bytes per frame
```

4. **Particle sprite cycling:** The executable contains exactly 96 particle-class descriptors at `0x004CC138`, each `0x34` bytes. The array ends at `0x004CD4B8`, exactly where the unrelated 100-vector displacement table begins; the older 128-entry interpretation overran that boundary by 32 records. Proven descriptor fields are frame-list pointer (`+0x00`), frame count (`+0x06`), flags (`+0x07`), frame multiplier (`+0x08`), draw scale (`+0x0A`), size-jitter divisor (`+0x0C`), lifetime/strict-expiry byte (`+0x0D`), and radius (`+0x10`). Each referenced frame is a 6-byte `(sprite_id, signed middle, scale)` tuple. Frame selection is `((frame_multiplier × counter) >> 6) % frame_count`. The remaining callback/data words are retained raw rather than named speculatively.

---

### Section 12: 3D Collision Volume Models — AND entity type records

**Handler:** `FUN_00410090` — Reads the block via BulkRead and processes
entries at stride 0x128 plus variable sub-sections and a terminated behavior-
choice list. It relocates 14 sub-section pointers, resolves behavior evaluator
callbacks and named behavior descriptors, then sets runtime type 0xD4 and the
collision load/unload callbacks.

> **Section 12 = the entity TYPE-RECORD pool (DECODED 2026-07-06).** These
> 0x128-byte records ARE the per-entity-type records the spawner indexes by
> `entity+0x58` (the entity's type). The runtime pool `DAT_004FE650` is
> Section-12 data; `DAT_004FE640` is Section-8 (models) — both are just
> entries [12] and [8] of the section-pointer table `DAT_004FE620[N]`, filled
> by the concatenating loader `FUN_00493860` across the preloaded system OVLs
> (L2 first, then L3, L5). **The type→model chain:** an entity's model index
> `entity+0xA8` = the type record's **+0xC** u16 (all 4 LOD slots), and the
> renderer draws `DAT_004FE640[ that index ]` (Section-8 model). Because L2
> loads first with 2 records → **types 0,1**, then L3's 128 → **types 2-129**.
> The two **menu-only types 0/1 live in L2's Section 12**. Menu use: type 0 (record 0,
> type_tag 0xE0, activation-dist 99999) +0xC = model **1 = `klaus`** (the
> bat-wing pterosaur backdrop); type 1 (record 1) +0xC = model 145 = L3
> `flag`. See MENU_SYSTEM.md §Menu 3D scene. (This chain also gives gameplay
> entity models: type N → Section-12 record N +0xC → Section-8 model.)
> Type-specific initialization can replace these base slots. A retail live
> entity snapshot corrected an earlier misidentification: type 6 is the
> `college`/Main Base entity (model 286), while the persistent player craft is
> type 46 with global model 41 `player4`.
>
> **ACTIVE BIOME MODEL POOL (resolved 2026-07-11):** gameplay model ids do not
> stop at the core/menu pool's final id 323. Loaded system/biome OVLs 6..11
> append their Section-8 models to `DAT_004FE640`. Intro2 proves the boundary:
> its type-34 meteor overrides model id 560, which resolves to `grock` in the
> loaded level-6 model block after L2 + L3 + L5. Omitting active biome layers
> made nearly every Intro2 actor invisible in the port.

**Present in:** System levels 2 and 3 only (8 OVLs across 4 display/resource variants). Level 2: 2 entries, Level 3: 128 entries. 104 OBJ files extracted with proper face topology.

**Base record (0x128 = 296 bytes):**
```
+0x000  uint32     type_tag      (overwritten to 0xD4 at load; original values: 0xCC most common)
+0x004  uint32     scale         scale multiplier (1, 10, 50, 100, 200, 400, 500, 600, 1000)
+0x008  uint32     id_field      entity class / flags (8, 64, 68, 5124, 6148, etc.)
+0x00C  uint16     model_slot_0  global Section-8 model id (state bits clear)
+0x00E  uint16     model_slot_1  model id when entity flag 0x4000 is set
+0x010  uint16     model_slot_2  model id when entity flag 0x2000 is set
+0x012  uint16     model_slot_3  model id when flags 0x2000|0x4000 are set
+0x014  uint32     dims[0]       collision activation distance (1–99999 game units)
+0x018  uint32     dims[1]       always 0 (unused)
+0x01C  uint32     dims[2]       shape extent X (width) — primary dimension for vertex generators
+0x020  uint32     dims[3]       shape extent Y (height)
+0x024  uint32     dims[4]       shape extent Z (depth)
+0x028  uint32     dims[5]       ground offset / minimum height (200 in 86% of entries)
...
+0x038  uint32[5]  render_params render state flags ([256,256,0–512,128–750,0–512])
...
+0x070  uint32     field_1c      collision parameter index (0 or 441–66014)
+0x078  uint32     vtable_1      (set at load to PTR_LAB_004c8a28)
+0x07C  uint32     vtable_2      (set at load to PTR_LAB_004c8a30)
+0x080  uint16     accepted_hit_presentation_sound_id  FUN_00410D30 positional cue; 0 = none
+0x082  uint16     infected_model_presentation_sound_id FUN_00411250 positional cue; 0 = none
+0x090  uint16     death_sound_id                      FUN_00410C10 cue; 0 = none
+0x098  uint16     generic_hit_sound_id                FUN_00414E90 live-hit cue; 0 = none
...
+0x0B8  uint32     col_load      (set at load to FUN_004381f0 — collision load handler)
+0x0BC  uint32     col_unload    (set at load to LAB_004382b0 — collision unload handler)
+0x0C0  uint32     initializer_state_flags  authored policy translated into live entity +0x08 and copied raw to +0xC8
...
+0x0C8  uint32     flags         bitmask flags (0x000, 0xC00, 0xE00, 0xF00, 0x1900)
+0x0D0  uint32[14] sub-section pointers  (indices [0x34]–[0x43], relocated to block base)
+0x118  uint32     behavior_choices (relative pointer to terminated behavior-choice list at [0x46])
+0x11C  uint32     behavior_rule_ref (evaluator-rule index at [0x47], resolved via 0x004C8CDC)
...
+0x124  uint32     alternate_behavior_class (named behavior-class index at [0x49], resolved via PTR_PTR_004c8aa4)
```

**Sub-section pointer table:** 14 optional sub-sections at uint32 array indices `[0x34..0x43]`. Each non-zero pointer is relocated to the block base at load. Sizes accumulate with alignment:

| Index | Label | Size | Align | Description |
|-------|-------|------|-------|-------------|
| 0x34 | Sub-A | 6B | no | Position offset (3 × int16: X, Y, Z) |
| 0x35 | Sub-B | 8B | yes | Collision extents (2 × int32) |
| 0x36 | Sub-C | 16B | yes | Lift/surface controller parameters; layout is behavior-specific. Type 46: `{u16 base_clearance, u16 lift_range, u32 strength, u16 near_range, u16 damping_range, u8 wave_flag, u8 body_offset_flag, u16 reserved}` |
| 0x37 | Sub-D | 12B | yes | Physics parameters / flags |
| 0x38 | Sub-E | 28B | yes | Complex parameters (damage, behavior) |
| 0x39 | Sub-F | 14B | no | Unknown (present in wire-frame entities) |
| 0x3A | Sub-G | 104B | yes | Game config parameters (full entity stat block) |
| 0x3B | Sub-H | var | yes | External-frame/face descriptor: two uint16 header words + count × 0x14 records |
| 0x3C | Sub-I | 8B | no | Additional collision data |
| 0x3D | Sub-J | var | no | Attachment descriptor: 2-byte header + count × 8-byte slots; count <= 8 |
| 0x3E | Sub-K | 2B | no | Short value (collision flag pair) |
| 0x3F | Sub-L | 6B | no | Unknown (paired with Sub-E often) |
| 0x41 | Sub-M | 18B | no | Status-component descriptor; present on global types 6, 66, 82, and 125 in system level 3 |
| 0x42 | Sub-N | 10B | no | Rare |
| 0x43 | Sub-O | 4B | no | Rare (present with Sub-G + Sub-J) |

**Sub-J authored attachment descriptor:**

```text
+0x00  uint8     slot_count
+0x01  uint8     reserved
+0x02  slot[slot_count]

slot (8 bytes):
+0x00  uint16    policy_word_raw
+0x02  int16[3]  local_offset_raw
```

The constructor accepts at most eight slots, and a valid payload has the exact
length `2 + slot_count * 8`; trailing bytes are not part of this descriptor.
The descriptor is immutable authored data. Retail and demo agree on the two
important users: player type 46 has five all-zero slots, while Type 17 has one
slot with policy word `1` and local offset `[0, 10, 110]`.

`FUN_00418330` materializes a separate mutable runtime, and `FUN_00418620`
changes its live capacity. The 16-byte header is
`{records_ptr, length, capacity, outer_policy_raw_at_0x0c}` and each 12-byte
row is `{two_arg_detach_callback, one_arg_destroy_callback, child_handle}`.
The port retains the stable common contract as entity-owned ordered child
identities with independent length, capacity, authored ceiling, and outer
policy; it does not mutate or reinterpret the authored slots as live rows.
Player profile setup sets outer policy `+0x0C` to `1` and sets live capacity to
`min(raw +0x199 unlock byte, authored count 5)`, while Type 17 retains outer
policy `0` and capacity one. Per-row callback dispatch remains a separate owner-specific
runtime contract rather than part of the decoded file format.

**Sub-M status-component descriptor (18 bytes):**

```text
+0x00  uint16    model vertex slot for Sub-M external-frame selector zero
+0x02  uint8[6]  one-based entity-variable binding selectors; 0 = unbound
+0x08  uint8[4]  retained tail prefix (semantics not assigned here)
+0x0C  int16[3]  product world offsets X/Y/Z, added after marker transform
```

The type-record pointer for Sub-M is at `+0x104`. `FUN_00409A80` allocates a
zeroed `0xB8`-byte runtime status block when this pointer is non-null and
resolves selectors `+0x02..+0x07` to six u16 output pointers at runtime offsets
`+0x98..+0xAC`. `FUN_00419630` publishes the component's live counters and
progress ratios through those pointers. Main Base type 6 authors raw word 0
and bindings `[0, 0, 1, 0, 0, 0]`; Working Factory type 66 authors raw word 8
and bindings `[4, 3, 0, 2, 1, 0]`. Both records have no Sub-G payload.

`19010` adds the signed offsets when the product is born. The external-frame
callback `0A9F0 ->198E0 ->199B0` later resolves descriptor slot `+00` in the
current model draw context and updates tracked product Sub-M `+88` with the
same world offsets. See [FACTORY_PRODUCT_MARKERS.md](FACTORY_PRODUCT_MARKERS.md)
for the recovered machine-code writer and applicable authored models.

**Sub-H external-frame/face descriptor:**

```text
+0x00  uint16   count
+0x02  uint16   phase-completion sound id (0 = none)
```

`FUN_0041D2A0` constructs one 0x3C-byte live external-frame record per
authored entry (up to 16). `FUN_0041D0A0` advances every live record exactly
once per caller update and rotates only the first record processed.
`FUN_0041D120` computes its unsigned phase step from the low 32 bits of signed
`(elapsed_us * phase_rate_raw) >> 31`; the four bytes at `+0x10` are record
dependencies that gate phase start. `FUN_0041D360` resolves authored model
type-14 selector `s` as primary endpoint `s` when `s < count`, otherwise as
secondary endpoint `s - count` when `s < 2*count`.

The live 0x3C-byte record caches primary, secondary, terrain-offset, animated
target, phase origin, three lengths, and four squared-length terms. The
detached Rust transaction now preserves `FUN_0041D360`'s conditional C/A/B
callback order, exact integer-square-root length cache, four height probes plus
the final midpoint, repeated-word sine arch, and all six secondary-axis modes.
`41DF20` forms each midpoint from `low + (short)(high-low)/2`, with
signed truncation toward zero and a final WORD sum. Retaining the full
dword difference selects the long interval across the signed-coordinate
seam and can create a distant animated foot target. The shared Rust
constraint solver now narrows the delta at the same stage. Original PE
execution of all three reproduced moving-newant inputs confirms the
results and all twelve terrain probes; see the tracked
runtime oracle and
guards.
Native draw-time A/B/C anchors also retain the current VIEW cache followed
by inverse viewport conversion, rather than collapsing it into a body
rotation; [Type47's paired draw](TYPE47_RUNTIME.md#one-original-model302-draw-fixture)
records the authenticated six-leg input and result.

The shared component constructor retains the 0x10-byte header: rotating cursor
`+0x04 = 0`, enabled `+0x08 = 1`, and surface policy `+0x0C = 0`.
After `1D2A0`, `09A80` calls `1E4A0(1)` exactly when Sub-C exists and its
signed byte `+0x0C` is nonzero. This constructor has no entity-type, model,
birth-cohort, or RNG gate. Each record starts with flags and phase zero;
its other cached fields become readable only through D360's cache flags.

Policy zero selects bilinear terrain. Nonzero selects the signed maximum of
terrain and animated water in both D360's planted endpoint and DF20's four
constraint probes. Disabling waves retains the static sea plane; it does not
disable this policy or use the renderer's water-visibility gate. The port
retains this policy in `SubHRuntimeState` and uses the draw's retail tick for
the shared `SubHSurfaceSampler`. Intro2's requested-selector presentation
commits only the records actually submitted. Native task and Sub-D ownership
remain separate from this shared construction.

The first-world insect audit identifies a concrete render consumer for this
data. Spider model 256 authors 16 referenced type-14 selectors paired as
`(0,8)..(7,15)` and 40 edge commands; stag model 267 authors 12 referenced
selectors paired as `(0,6)..(5,11)` and 20 edges; newant model302 authors 13
type-14 records. Selectors0..11 form the same six H pairs; selector12 in
slot152 is consumed by Sub-E emitter0, whose authored source slot is150.
The stream reaches it in its first, transparent sprite668 triangle. The
sprite's 2x2 RGBA data is entirely zero, but reached corner resolution still
performs `424F20`'s queued-emitter stamp. The model submits 34 edges. See
[Type47 draw ownership](TYPE47_RUNTIME.md#authored-draw-and-firing-origin)
for the actor/FIFO epochs and explicit current-node custody boundary.
The long legs are opcode-`0x22`
sprite ribbons (spider 985/986) plus palette `0x02` hairlines: selector
vertices occur in those commands, not triangle faces. Near-pass
`FUN_00459000` extrudes a `FUN_0047AA20` quad using the sprite and the raw
`FUN_004594C0` size short (not `FUN_00470840` packed decode). `DAT_004c5268` queues when the combined outcode is
non-zero, and `FUN_0047AA20` thunks to `FUN_0047EF10` (device `+0x1098`)
for the textured fill with implicit UVs and Section-3 blend flags. The
port constructs and clips that path and fills live by painting those
screen corners in a HUD-style pixel ortho. Palette-style
`0x02` hairlines use A-pass `FUN_00458c60` (two endpoints, same LUT)
and fill as pixel-ortho lines. Do not invent world-space ribbons.

The complete actor type14 routing is `40A9F0`: subtract twice the H count,
then the count of nonzero Sub-E WORDs `+12/+14`, then present Sub-M's one
selector and live Sub-G's two selectors. The E ordinal selects the original
WORD position; a zero first slot is not compacted behind a nonzero second
slot. After these reservations, residual zero uses a valid recent-relation
handle or the actor itself, while later residuals use the actor. `40A980`
always consumes three RNG draws, with a zero displacement multiplier for
the relation point and multiplier one for self jitter. Negative selectors
and missing current selector owners must not fall into this terminal path.

After an actor type14 callback, `40D350` wraps all three endpoint WORDs
relative to the viewport origin WORDs, sign-extends those deltas and adds
the full origin dwords. `6ECF0` subtracts that origin and applies the native
Q31 viewport rows into the source VIEW cache. Preserve this WORD/dword
boundary at toroidal seams. World type13 then consumes the cached VIEW point,
inversely transforms with separately narrowed Q31 products, samples the
signed terrain/wave surface and forward-transforms the result. Its sea-band
rejection retains XYZ and publishes clip0x40 alongside cache-valid0x80.
The actual caller's wave-enable word reaches `445920`; inactive waves retain
the static sea plane instead of evaluating the moving surface.

Type13 shadow aliases resolve their source vertex callback before terrain
projection. In particular, newant slots136..146 alias the six primary H
points112..122. The source sequence is `4349C0 -> source slot callback ->
6ECF0 -> D350 -> A9F0 -> 1D360`; projecting the raw selector placeholder
coordinates first produces an unrelated shadow endpoint. Resolved world
endpoints retain that space through rendering rather than undergoing a
second body transform.

The same bytes retain their older face-oriented diagnostic view; field names
must therefore remain consumer-specific rather than being globally relabeled:

```
+0x00  uint32   resolver_flags / face flags
+0x04  int32    phase_rate_raw / secondary face flags
+0x08  uint16   model_ref_a / material reference
+0x0A  uint16   model_ref_b / normal reference
+0x0C  uint16   model_ref_c / secondary reference
+0x0E  uint16   axis_mode_raw / face diagnostic word
+0x10  uint8[4] dependency records / face vertex indices
```

**Behavior-choice list** (pointed to by uint32 index `[0x46]` / runtime
offset `+0x118`): terminated by `weight_rule_id == 0`. Each entry is 12 bytes:
```
+0x00  uint32   weight_rule_id      evaluator rule
+0x04  uint32   weight_multiplier   multiplier applied to the evaluator result
+0x08  uint32   behavior_class_id   named behavior class
```
At load, `weight_rule_id` is replaced with the evaluator callback from the
8-byte-stride table at `0x004C8CDC`, and `behavior_class_id` is replaced with
the named behavior descriptor from the table rooted at `0x004C8AA0`
(`PTR_PTR_004c8aa4` is its descriptor-pointer column). Proven authored rule
ids across the complete system-overlay corpus are 1 = Always, 2 = Under
Attack, 5 = Mutated, 6 = Player Nearby, 7 = Baddie Nearby, 8 = Furniture
Nearby, 9 = Buildings Nearby, 10 = People Nearby, 11 = Beacon Nearby,
12 = Base Nearby, and 13 = Job Nearby. `FUN_00425680` evaluates every
candidate, multiplies the signed result by `weight_multiplier`, caps only the
upper total at 0x7FFF, consumes one random word unconditionally, and selects
the first strict cumulative weight above the resulting threshold. These
records are behavior initialization data, not render materials.

The header fields at `+0x11C` and `+0x124` use the same evaluator-rule and
named-behavior tables. `+0x124` is the alternate behavior class installed by
the direct path when live entity state bit `0x4000` is set; ordinary fresh
entities select from the `+0x118` list. The loader resolves `+0x124` when the
authored `+0x11C` rule reference is nonzero. The later purpose of that rule
reference is not yet named more narrowly.

**Collision topology types (9 distinct):**

| Topology | Verts | Faces | Description | Dims used |
|----------|-------|-------|-------------|-----------|
| 6v_octahedron | 6 | 6 quad | Axis-aligned octahedron | dims[2]=X, [3]=Y, [4]=Z |
| 8v_box | 8 | 8 quad | Double octahedron / elongated diamond | dims[2]=X, [3]=Y, [4]=Z |
| 8v_double_tet | 8 | 8 tri | Two independent tetrahedra | dims[2], [3], [4] |
| 8v_ring | 8 | 8 tri | Antiprism (rotated top/bottom squares) | dims[2]=radius, [4]=height |
| 10v_pent_tube | 10 | 10 tri | Pentagonal prism | dims[2]=radius, [3]=height |
| 14v_hept_tube | 14 | 14 tri | Heptagonal prism | dims[2]=radius, [3]=height |
| 14v_composite | 14 | 6q+8t | Octahedron + 2 bridged tetrahedra | dims[2]=X, [3]=Y, [4]=Z |
| 14v_triple_tet | 14 | 14 tri | Three tetrahedra | dims[2], [3] |
| 4v_degenerate | 4 | 4 tri | Flat billboard quad (degenerate faces) | dims[2]=X, [4]=Z |

**Type tag distribution (across all 520 entries):**

| Type tag | Count | Notes |
|----------|-------|-------|
| 0x00CC | 340 | Standard collision volume (65%) |
| 0x0000 | 72 | No collision geometry |
| 0x00D0 | 48 | Extended collision |
| 0x00C4 | 20 | Composite shape |
| 0x0100 | 8 | Special entity |
| 0x00C8 | 8 | Variant |
| 0x00C0 | 8 | Minimal collision |
| 0x00E0 | 4 | Large entity |
| 0x00E4 | 4 | Large entity variant |
| 0x0178 | 4 | Complex entity |
| 0x0300 | 4 | Special case |

**Runtime initial-behavior selection:**
```
Entity spawn (Section 13)
  → entity_type → cumulative Section-12 type record
    → +0x0C..+0x13 Section-8 model slots
    → FUN_0040ac60 (initial behavior dispatch)
      → live state 0x4000: FUN_00425660 (install +0x124 alternate class)
      → ordinary path: FUN_00425680 (weighted +0x118 behavior selection)
        → FUN_00438340 (install the selected behavior descriptor)
```

`FUN_004381f0` remains the Section-12 collision load handler and
`FUN_00418a90` extracts type dimensions and computes the collision radius.
The older documentation incorrectly placed `FUN_0040ac60`, `FUN_00425660`,
and `FUN_00425680` in a sprite-vs-volume collision pipeline; those three
functions select and install entity behavior and do not reconstruct collision
vertices.

**Tool:** `v2k-formats::collision` (canonical decoder and diagnostics)

---

### Section 13: Level Descriptors + Entity Spawns

**Handler:** `FUN_0042d880` — Reads block via `FUN_00410380` (bulk read), then iterates entries with 0xD0-byte base + variable trailing data. Contains one level descriptor per game world, plus entity spawn sub-entries and 0x20-byte campaign/goal records.

**Base descriptor (0xD0 = 208 bytes):**
```
+0x00  64 bytes  Level name (null-padded ASCII, e.g., "Medaeval", "Colorado")
+0x40  36 bytes  Game parameters (terrain dimensions, settings)
+0x48  uint32    world_style (1–6) — `FUN_0042E570` loads system resource
                  overlay `5 + world_style`; this one pack supplies terrain
                  palettes/sprites, Section-9 static objects, and biome models
+0x4C  uint32    terrain_sprite_base — global Section-3 id of the 120
                  canonical opaque terrain transition tiles
+0x54  uint16    sky_color_index — system-level-2 master Section-7 palette
                  index used by the fullscreen `FUN_0047B9F0` fill
+0x56  uint16    sky_model — optional global Section-8 model id (only levels
                  13/14 use 305/306, `sky1`/`sky2`)
+0x58  uint16    main_base_abort_sky_color_index — alternate master-palette
                  background copied by `FUN_0042F1A0`
+0x5A  uint16    main_base_abort_sky_model — alternate optional sky model;
                  Level 13 uses zero to suppress normal model 305
+0x5C  uint32    time-trophy deadline in seconds; zero selects the no-authored-
                  countdown path. `FUN_0042E570` initializes controller timer
                  state from this value. The canonical Rust descriptor exposes
                  `time_trophy_deadline_seconds()`; corpus checks retain first
                  world = 0, Medaeval = 270, Cistern = 180, VSpread = 600
+0x64  uint32    campaign_record_count (number of 0x20-byte records)
+0x68  uint32    campaign_records_rel (relative pointer, patched at load)
+0x6C  92 bytes  More game parameters
  +0x88  uint32  terrain_draw_depth and world fog far distance in cells;
                  `FUN_00433130` bounds only the terrain scan footprint
  +0x8C  uint32  world fog width in cells — copied unchanged by normal setup
                  and Main Base abort; `FUN_0042E920` consumes it with +0x88
+0xC8  uint32    sub_count (entity/spawn sub-entries)
+0xCC  uint32    sub_arr_rel (relative pointer to sub-entry pointer array)
```

Gameplay Level 1 is descriptor/overlay 13. Across display tiers 0--3 its
normal pair is palette/model `27/305`, while the Main Base abort pair is
`32/0`; zero suppresses `sky1`. The former speculative description of `+0x8C`
as a virus radius/interval is retired: matched normal/abort request builders
prove the exact sixth request dword. The later original-PE audit establishes
its additional scene-fog meaning: `FUN_0042E920` reads the current Section-13
row's `+0x88/+0x8C`, shifts both dwords left eight bits, and the successful
`FUN_00451710` suffix at `45183F/451841/45184E` stores `far_raw-width_raw`
and `far_raw` at the parent scene's `+0x74/+0x78`. Those operations wrap at
32 bits; they apply neither the terrain's 30-row scan cap nor an eight-cell
constant. Canonical `LevelDescriptor::fog_width_cells()` and
`fog_planes_raw()` retain this owner without adding a guessed field to
synthetic descriptor literals.

All 152 retail descriptors (worlds 13–50, display tiers 0–3) agree across
tiers. Far distances range 18–30 cells; widths are 1, 2, 4, 5, 6, 7, 8, 10,
11, 12 and 29. Medaeval stores far 22/width 6 (near 16); worlds 15 and 31
store 22/10 (near 12) and 29/5 (near 24); Arena4 stores 30/29 (near 1).
The required corpus regression is
[`tests/level_fog.rs`](../../crates/v2k-formats/tests/level_fog.rs).
`execute-native-world-fog.py`
executes only the verified original descriptor reader, successful scene-store
suffix and projector setter on private buffers. Its
`guards` reject false corpus
headers, missing rows, unsupported instructions and non-u32 inputs. The
controlled far-40 case proves uncapped arithmetic independently of the
retail corpus, whose largest authored far is 30. This does not run a full
world loader or the native rasterizer.

**After the base descriptor:** `sub_count × uint32` pointer array, followed by sub-entry data and optional animation/config blocks. Campaign records are reached through the authored `+0x68` relative pointer; they must not be inferred from apparent entity payload sizes because alignment gaps occur.

**Sub-entry (0x44 = 68 bytes):**
```
+0x00  uint32    reserved       always 0
+0x04  uint32    entity_type    entity class ID (range 2–129)
+0x08  4 bytes   pos_data_1     signed 8.8 fixed X,Y (entity +0x96/+0x98)
+0x0C  4 bytes   pos_data_2     signed 8.8 fixed Z in low half (entity +0x9A)
+0x10  uint32    param          game parameter (0 or 1 typically)
+0x14  12 bytes  behavior args  per-spawn behavior parameters
+0x20  3 x uint16 rotation      yaw/pitch/roll copied to entity +0xA2/+0xA4/+0xA6;
                                 +0xA2 is heading/yaw
+0x26  6 bytes   behavior args  additional per-spawn behavior parameters
+0x2C  16 bytes  model slots    four optional per-spawn model-slot overrides
                                 (zero = type default)
+0x3C  uint32    anim_ptr       relative offset to animation block (0 = none)
+0x40  uint32    config_ptr     relative offset to 0x58-byte config block (0 = none)
```

**Optional animation block (when anim_ptr != 0):**
```
+0x00  16 bytes  header
+0x10  uint32    frame_count
+0x14  uint32    frame_data_ptr (relocated)
  Followed by frame_count × 0x1C bytes of frame data
```

**Optional config block (when config_ptr != 0):** 0x58 (88) bytes of gameplay parameters.

**Campaign/goal records (0x20 = 32 bytes each):** Counted by `+0x64` and addressed by `+0x68`. Records with flags dword `+0x0C & 0x10` are authored terrain-marker transitions consumed by `FUN_0042DD10` and `FUN_0042E270`:

```
+0x00  uint32    destination logical level (global gameplay overlay = value + 12)
+0x04  int16     destination arrival X (signed 8.8)
+0x06  int16     destination arrival Y (signed 8.8)
+0x08  int16     destination arrival Z (signed 8.8)
+0x0A  2 bytes   unknown
+0x0C  uint32    record flags; bit 0x10 selects marker-transition semantics
+0x10  13 bytes  other campaign/goal state, not yet classified
+0x1D  int8      marker subtype 1..5 (terrain kind = 0x15 + subtype)
+0x1E  int8      marker argument 0 (meaning unresolved)
+0x1F  int8      marker argument 1 (meaning unresolved)
```

Level 13 proves two such records: subtype 2 routes to logical level 18/global
30 at `[7424,5120,9728]`, while subtype 1 routes to logical level 2/global 14
at `[17408,2560,-32512]`. Level 30 subtype 4 routes back to logical level
1/global 13 at `[-27392,0,-5120]`. Records without flag `0x10` remain raw and
must not inherit transition semantics; Level 13 has four total records but only
two flag-`0x10` transitions.

**Entity statistics:** 1,445 entity placements across 38 game worlds (variant-0). 79 unique entity type IDs (range 2–129). Entity types include: spawnable objects (weapons, pickups), NPCs, level triggers, and decorative elements.

**Named levels (29 of 38):** Medaeval, Colorado, Alpine, SunnyIce, Flood, Water, Arena1–6, VSpread, WindyIce, Pchannel, DarkReef, Marbles, Desert Island, Plateau, Plateau2, Reactor, Dragon1, Alien1–3, Volcano, WChannel, Intro1, Intro2.

**Present in:** 152 OVLs (38 levels × 4 variants).

**Canonical decoder:** `v2k-formats::levels`

---

### Section 14: Sprite Dependency Lists + Spatial Grids

**Handler:** `FUN_004ab3b0` — Reads `[uint32 block_size][data]`, iterates `count` entries each 28 bytes. Relocates three pointer fields per record (offsets +0x10, +0x14, +0x18) to absolute addresses within the allocated block.

**Record format (0x1C = 28 bytes = 7 × uint32):**
```
+0x00  uint32  record_type   1 = sprite reference list, 2 = spatial grid data
+0x04  uint32  layer_count   number of layers (1 for most; 8 for multi-layer grids)
+0x08  uint32  entry_count   number of entries in Region 3
+0x0C  uint32  entry_size    bytes per Region 3 entry (4 for sprite lists, 1 or 16 for grids)
+0x10  uint32  region1_ptr   → Region 1 metadata (relocated at load)
+0x14  uint32  region2_ptr   → Region 2 per-layer params (relocated at load)
+0x18  uint32  region3_ptr   → Region 3 main data array (relocated at load)
```

**Region sizes:** R3 = `entry_count × entry_size` (exact match confirmed). R1 = 8 bytes (2 × uint32). R2 = `layer_count × 4` bytes.

#### Type 1: Sprite Dependency Lists (Game World Levels 13–50)

Each game world OVL has exactly 1 record (type=1) containing a list of **global sprite indices** that the level requires. Region 3 is an array of `entry_count` uint32 values, each indexing into the cumulative sprite pool built from system levels.

```
Region 1: uint32 = entry_count (redundant copy)
Region 2: uint32 = 4 (entry size, redundant copy)
Region 3: entry_count × uint32 = global sprite indices
```

**Cumulative sprite pool** (3,766 sprites total, from PRELOAD.DAT Section 3):
| System Level | Sprites | Cumulative Range |
|-------------|---------|-----------------|
| 0 | 11 | 0–10 |
| 1 | 31 | 11–41 |
| 2 (core) | 377 | 42–418 |
| 3 (common) | 866 | 419–1284 |
| 5 | 37 | 1285–1321 |
| 6 (biome A) | 490 | 1322–1811 |
| 7 (biome B) | 341 | 1812–2152 |
| 8 (biome C) | 484 | 2153–2636 |
| 9 (biome D) | 361 | 2637–2997 |
| 10 (biome E) | 340 | 2998–3337 |
| 11 (biome F) | 395 | 3338–3732 |
| 51 | 33 | 3733–3765 |

**Sprite source pattern:** Every game world level references sprites from:
1. Level 2 (core UI/shared) — ~20 sprites per level
2. Level 3 (common game objects) — majority of each level's sprites
3. Exactly **ONE** biome system level (6–11) — matches terrain palette biome assignment

Example sprite counts per source level:
| Game Level | Name | Total | Lv2 | Lv3 | Biome | Biome Lv |
|-----------|------|-------|-----|-----|-------|----------|
| 14 | Medaeval | 419 | 21 | 221 | 177 | 6 |
| 16 | Colorado | 330 | 21 | 188 | 121 | 7 |
| 25 | Arena1 | 599 | 23 | 323 | 252 | 6 |
| 22 | Water | 144 | 21 | 65 | 58 | 11 |

#### Type 2: Spatial Grid Data (System Level 3 only)

Level 3 has 4 records (type=2) containing grid-based spatial lookup tables:

```
Region 1: 2 × uint32 = grid dimensions (width, height)
Region 2: layer_count × uint32 = per-layer parameters
Region 3: entry_count × entry_size bytes = grid data
```

| Record | Grid | Entry Size | Entries | Description |
|--------|------|-----------|---------|-------------|
| 0 | 64×64 | 1 byte | 4096 | Coarse LOD/visibility grid (values 0–15, 68% zero) |
| 1 | 32×32 | 16 bytes | 1024 | Fog/atmosphere density (4 × float32, mostly 0.175 or 0.0) |
| 2 | 32×32 | 1 byte | 1024 | Terrain visibility mask (values 0–15, 15=visible) |
| 3 | 38×7 | 1 byte | 266 | Per-level property table (38 game levels × 7 properties) |

**Present in:** 39 OVLs (level 3 with count=4, levels 13–50 with count=1 each). Total: 42 records, 61,140 bytes trailing data.

**Tool:** `v2k-formats::linkage` (canonical decoder)

---

## PRELOAD.DAT Format

**File size:** 828,876 bytes (829 KB).

**Structure:** Two concatenated parts:
1. Resource count matrix (3,180 bytes)
2. Embedded OVL data (825,696 bytes)

### Resource Count Matrix

```
uint32[15][53]    Section-major order, little-endian
                  grid[section][level] = resource count
```

3,180 bytes = 15 sections × 53 levels × 4 bytes. Tells the engine how many resources to pre-allocate per section before loading each level's OVL file.

**Statistics:** 204 non-zero entries out of 795 total. 6,528 total resources across all sections and levels.

**Loader:** `FUN_00493860` (LoadPreload) reads the matrix, then iterates the 15 sections for each level that has nonzero counts.

### Embedded OVL Files

825,696 bytes containing 7 complete OVL files (each with 15 `"abcd"` sections). These are system-level resources loaded at startup before any game level.

**Identification:** 6 of 7 match variant-0 OVL files on disk:
- Embedded 0 → `0X0XX.OVL` (level 0)
- Embedded 1 → `0X1XX.OVL` (level 1)
- Embedded 2 → `0X2XX.OVL` (level 2)
- Embedded 3 → `0X4XX.OVL` (level 4)
- Embedded 4 → `0X5XX.OVL` (level 5)
- Embedded 5 → `0X12XX.OVL` (level 12)
- Embedded 6 → (no disk match)

**Canonical decoder:** `v2k-formats::preload`

---

## Key Addresses

### V2000.EXE

| Address | Function / Data | Description |
|---------|-----------------|-------------|
| `0x00493654` | LoadOverlay | Formats `overlay\%dx%dxx.ovl`, opens file |
| `0x00493680` | FileOpenChain | Allocates file handle, opens with path formatting |
| `0x00493860` | LoadPreload | Opens `preload.dat`, reads per-section resource counts |
| `0x00493BF0` | SectionReader | Iterates 15 sections: abcd verify + handler vtable dispatch |
| `0x00493CE0` | OVLHeaderVerify | Reads 4 bytes, compares against `"abcd"` magic |
| `0x00493E40` | OVLDispatch | High-level overlay load: checks if loaded, calls vtable |
| `0x00410380` | BulkRead | Reads `[uint32 size][data]` block, allocates + reads |
| `0x004572B0` | Mem_Alloc | malloc wrapper (89 callers) |
| `0x00457320` | Mem_Free | free wrapper (85 callers) |
| `0x00457370` | Mem_Fill | memset-like fill (45 callers) |
| `0x004575A0` | VTable_Dispatch | Indirect vtable call (189 callers) |
| `0x00468E70` | File_BufferedRead | Buffered file read with progress callback |
| `0x004CE0D8` | Handler vtable | 15 entries × 4 ptrs (load, unload, init, cleanup) |
| `0x004CE0A0` | Section count | = 15 |
| `0x004CE0A4` | Level count | = 53 |
| `0x004D0020` | Handler desc array | 53 pointers to per-level descriptor structs |
| `0x004D6740` | WAVEFORMATEX | Audio format: 22050 Hz, mono, 16-bit signed PCM |
| `0x004D703D` | `g.dat` string | Unknown global data filename |
| `0x004C8CDC` | Behavior evaluator table base | 8-byte indexed records; Section-12 loader reads the evaluator descriptor column at `base + id*8` |
| `0x004C8AA4` | Named behavior descriptor column | Indexed by Section-12 behavior-class ids |
| `0x004CC138` | Particle descriptor table | 96 entries × 52 bytes; ends at `0x004CD4B8` (frame data, proven class scalars, raw callback/data words) |
| `0x004DCEE8` | Palette list heads | 12 palette slot linked list heads (runtime) |
| `0x004DCEF0` | Palette pool | Palette state entry pool (runtime, 12-byte entries) |
| `0x004FE62C` | Sprite meta array | Sprite metadata pointer array (runtime, from Section 8) |
| `0x004FE640` | Palette ptr array | Palette pointer array (runtime, from Section 7) |
| `0x0043DC90` | MaterialRender | Primary sprite/billboard material renderer (78 materials) |
| `0x00440A60` | MaterialBind | Material binding: palette lookup + shade level + linked list insert |
| `0x00440E80` | EntityRender | Entity render setup: transforms + material dispatch |
| `0x00464E60` | ModelRender | Section 8 command-stream walker (near/far table select) |
| `0x004D3CE0` | (data) | cmd dispatch table A — render near (256 ptrs) |
| `0x004D40E0` | (data) | cmd dispatch table B — render far/LOD (256 ptrs) |
| `0x004D44E0` | (data) | cmd dispatch table C — transform-only pass (256 ptrs) |
| `0x004D55A0` | (data) | scanline rasterizer edge-interpolator table (NOT a cmd pass) |
| `0x004D4A78` | (data) | per-vertex type_flag transform table (ctx[0x13]): +0 far, +0x3C near, +0x78 pass 3, +0xB4.. projection helpers |
| `0x004D14D0` | (data) | sine table (0x4000 = 90°), used by billboard rotation |
| `0x00470840` | PackedOperandDecode | rotate-imm / register / callback operand scheme |
| `0x00470700` | EvalAnimVar | animation variable eval (imm / imm<<10 / callback / register) |
| `0x0046FB80` | BezierEval | cubic Bézier vertex evaluator (type_flag 9/10) |
| `0x004D14D0` | SinTable | Sine lookup table (0x2000 entries, fixed-point) |
| `0x00495480` | SoundBuffer_Create | Central sound playback: resolves type=5 aliases, creates DirectSoundBuffer (max 24) |
| `0x004957C0` | SoundBuffer_SetParams | Sets frequency (`value * 22050 >> 16`), pan, volume on sound buffer |
| `0x00495830` | SoundBuffer_SetVolume | Applies master volume scaling to individual buffer |
| `0x00495910` | SoundSystem_SetMasterVolume | Iterates all active buffers, updates master volume |
| `0x004958C0` | SoundPlay_Dispatch | Top-level sound trigger: index lookup → SoundBuffer_Create |
| `0x00496460` | CDaudio_Init | MCI open, set time format to TMSF |
| `0x004AD0A0` | DS_SetFrequency | IDirectSoundBuffer::SetFrequency (vtable +0x44) |
| `0x004AD0D0` | DS_SetPan | IDirectSoundBuffer::SetPan (vtable +0x40) |
| `0x004AD0F0` | DS_SetVolume | IDirectSoundBuffer::SetVolume (vtable +0x3C) |
| `0x004FBE60` | SoundBufferListHead | Active sound buffer linked list head (max 24 entries) |
| `0x004FBE6C` | MasterVolume | Master volume level (applied to all active buffers) |
| `0x004DE840` | AnimFrameCounter | Global animation frame counter (modulo 100 for movement) |
| `0x004CD4B8` | AnimDisplacementTable | 100-entry movement displacement lookup (entity animation) |
| `0x00410090` | Section12_Handler | Section 12 loader: 0x128-byte entries with 14 sub-sections |
| `0x004381F0` | CollisionLoad | Collision volume load handler (vtable at record +0xB8) |
| `0x00418A90` | CollisionConfig | Extracts dimension fields, computes collision radius |
| `0x0040AC60` | InitialBehaviorDispatch | Routes live-state 0x4000 entities to the alternate behavior class and ordinary entities to weighted selection |
| `0x00425680` | WeightedBehaviorSelector | Evaluates the Section-12 +0x118 choices and installs the selected named behavior |
| `0x00425660` | AlternateBehaviorSelector | Installs the Section-12 +0x124 behavior class directly as variant zero |

### Ghidra Analysis

- **Total functions:** 1,563
- **Import DLLs:** DDRAW.dll, DSOUND.dll, DINPUT.dll (DirectX 5)
- **Bulk decompiled:** All 1,563 functions in 6 categories (CRT, FGDK, OVL handlers, DirectX, game logic, thunks)
- **Post-processed:** 21,974 type replacements, 14 named functions, 14 named globals

---

## Cross-References with Infestation

Infestation's sixteen resource handlers keep the numbers of Sections 3-8
(sprites, fonts, display records, shade table, colours, models) and renumber
the rest: strings 2 to 0, sounds 11 to 9, terrain 10 to 12, cell-to-model
tables 9 to 11, type records 12 to 13 and level placements 13 to 14. Several
keep identical record layouts. Section-by-section correspondence, shared code
and identical tables are maintained in the
[Infestation cross-reference](INFESTATION_CROSS_REFERENCE.md).

---

## Save File Format

This section is the authoritative native-save format description. Menu behavior
is summarized in [MENU_SYSTEM.md](MENU_SYSTEM.md#frontend-load-sequence-and-slot-count-runtime-confirmed-2026-07-17),
and gameplay restoration status is summarized in
[GAME_MECHANICS.md](GAME_MECHANICS.md#9-save-system).

### Slot namespace and reserved row

The retail menu renders 15 rows, but only rows 0 through 13 are playable save
slots. They map directly to the 2,048-byte files `Slot00` through `Slot13` via
the `"%s\\Slot%02d"` path formatter. Row 14 is not a `Slot14` game save:
`FUN_0043BB20` renders resource string 80, **"Used for game settings"**, and
the extracted `0x4C1D28` menu item has no selection callback.

### Eight-record `0x800`-byte layout

Every native file consists of eight CRC-framed records. A frame contains its
payload, a little-endian CRC dword, then padding to the next *strict* `0x80`
boundary. `FUN_00448990` sources the padding bytes from the beginning of the
generated CRC table; the reader seeks over them and does not validate them.

| Group | Frame starts | Copies | Payload | CRC and padding | Frame span |
|-------|--------------|-------:|---------|-----------------|-----------:|
| Version | `0x000`, `0x080`, `0x100`, `0x180` | 4 | 4-byte little-endian value `5` | CRC at `+0x04`; padding `+0x08..+0x7F` | `0x80` |
| State | `0x200`, `0x480` | 2 | complete `0x248`-byte state snapshot | CRC at `+0x248`; padding `+0x24C..+0x27F` | `0x280` |
| Tail | `0x700`, `0x780` | 2 | opaque 4-byte runtime value | CRC at `+0x04`; padding `+0x08..+0x7F` | `0x80` |

There is **no encryption** in this path. The earlier GetTickCount/XOR
attribution came from unrelated protection helpers. Apparent CRC-table
fragments inside the file are frame padding, not encrypted headers or part of
the state payload.

### CRC and recovery policy

`FUN_00448950` builds the reflected table with polynomial `0xEDB88320`.
`FUN_00448A20` starts from seed zero and applies no final XOR/complement. The
stored CRC therefore covers only the frame payload and is exactly the raw
result of that update.

`FUN_00448A60` visits copies in physical order and returns the first one whose
CRC is valid. Once a valid copy has been recovered, later copies in that group
are skipped; they need not be valid or byte-identical. Failure of every copy in
a requested group is an error, and the selected version value must be `5`.
The full loader requests version, state, and tail groups; the menu probe reads
only version and state to derive status/name metadata. Redundancy is recovery,
not an equality invariant. Consequently menu status 4 means that the header and
state probe succeeded, not that the full load is guaranteed: if neither tail
copy passes CRC, `FUN_00448BD0` still fails. The port's compatibility importer
requires a recoverable record in all three groups before dispatching gameplay.
A tail-invalid native file keeps the ordinary probe-valid row label, remains
occupied, and is non-loadable; selection plays feedback sound 2 and stays on
the list. Retail additionally opens its `0x4C1890` error screen, which the port
does not yet represent.

### State payload, level domain, and current restoration boundary

The state record is one indivisible `0x248`-byte payload. Retail copies all
`0x92` dwords between the selected frame and `DAT_004DEEC8`; the former
448-byte-plus-"trailing data" split was incorrect. Mapped fields are:

| State offset | Type | Proven meaning | Current port handling |
|--------------|------|----------------|-----------------------|
| `+0x00..+0x1F` | char[32] | NUL-terminated editable save-row label; the probe forces byte 31 to zero | Displayed, but never treated as the authoritative level name |
| `+0x20` | u32 | One-based logical campaign/controller slot | Raw mapping accepts `1..38` as global OVL `logical + 12`; native loading accepts only interactive worlds `1..36` |
| `+0x24..+0x29` | i16[3] | Player XYZ in signed 8.8 world coordinates | Restored exactly |
| `+0x2A..+0x2F` | i16[3] | Player XYZ velocity in signed 8.8 units | Restored exactly |
| `+0x30` | u16 | Player heading binary-angle word | Restored exactly |
| `+0x32`, `+0x34` | u16, u16 | Player pitch and roll binary-angle words | Restored exactly, including nonzero words and the `413F70` entity basis |
| `+0x3C` | i32 | Player hull health mirrored through controller `+0x8C` | Restored exactly; non-positive native health is currently unsupported |

The `+0x20` value is **not** a global OVL id. In saved-game mode 6,
`FUN_0044F650` copies the recovered state to session `+0x14`, reads session
`+0x34` (state `+0x20`), and gives that logical slot to `FUN_0042E3B0`.
`FUN_0042E570` subsequently requests gameplay OVL `logical + 0x0C`. Examples
from the accepted native corpus are Mediaeval `2 -> 14`, Castle `3 -> 15`,
Alpine `5 -> 17`, and Cistern `18 -> 30`. Treating the saved value directly as
a global id addresses system/resource OVLs for the first three and Amazon OVL
18 for Cistern. Logical 37 and 38 still have valid raw mappings to Intro1 and
Intro2 (global 49 and 50), but those overlays have cinematic contracts and must
not enter the save loader's ordinary `Playing` path. They are therefore
`Unsupported`; the last saveable logical/global pair is `36 -> 48`.

The player subset has a separate static provenance. Before serialization,
`FUN_00443260` calls `FUN_00443440`, which mirrors the live type-46 entity's
position, velocity, heading/pitch/roll, and health into controller `+0x74`.
It then copies 99 dwords from controller `+0x74` into session `+0x38`, which is
state `+0x24`. `FUN_00443560` performs the reverse copy during restoration and
publishes those fields to the reconstructed player entity. Native loading
creates/restores that player before authored actor publication, then restores
cargo through its shared constructor/identity path. The complete payload now
travels with the load request; controller fuel/mode, inventory, capabilities,
shield, cargo and campaign inputs are decoded separately. The field mapping,
four-byte notification-mask tail, and remaining runtime boundaries are owned
by [the native restoration section](SAVE_AND_SETTINGS.md#native-save-scope-and-restoration-boundary-confirmed).
Unresolved saved fields remain retained; this does not claim complete native
world restoration.

### Portable JSON compatibility and precedence

Portable JSON is a separate port format and remains a compatibility preview,
not a complete persistence claim. A current portable save is loadable only
when it identifies an interactive gameplay OVL in `13..48`, includes position,
velocity, heading, body pitch/roll, and positive health, and declares the
portable source. New writes round-trip that player subset; craft mode, fuel,
damage buffer, inventory, cargo, capabilities, and campaign state remain
unserialized.
Legacy level-only JSON lacks that player snapshot and now fails closed as
`Unsupported`; malformed JSON is `Corrupt`.

`SaveManager::write_checkpoint` validates the complete native restoration
payload, then uses the canonical eight-record encoder with version `5` and the
caller-owned seen-event-mask tail. It writes only `saves/SlotNN`, updates the
menu row immediately, and preserves imported retail files and older JSON.
Empty-slot writes use exclusive creation and reject files that appeared after
the menu scan; occupied rows require an explicit overwrite disposition.

A port-owned `saves/SlotNN` has first precedence, including corrupt or unsupported
states: an older save must not silently replace a failed checkpoint. When it is
absent, supported portable JSON wins; otherwise the loader probes native retail
`SlotNN` in the data directory, then its parent. Thus legacy level-only JSON
cannot shadow a supported native import. Missing fallback leaves the JSON's
corrupt/unsupported row visible and non-loadable. The portable compatibility API
still works: confirmed replacement removes only a superseded port-owned native
checkpoint after writing JSON, and never changes either retail import path.

**Key decompiled functions:**

- `FUN_00448950`: generate the CRC table
- `FUN_00448990`: write one framed record
- `FUN_00448A20`: update the payload CRC
- `FUN_00448A60` / `FUN_00448B20`: first-valid copy recovery and frame seek
- `FUN_00448BD0`: load four version, two state, and two tail records
- `FUN_00448CF0`: write the same eight-record sequence
- `FUN_00448E70`: probe a slot and cache its 32-byte name/status

**Canonical decoder:** [`v2k-formats::saves`](../../crates/v2k-formats/src/saves.rs)

---

## Disc Image / CD Audio

Mixed-mode disc image containing 10 Red Book CD-DA audio tracks (~63 minutes total). Track 1 is the data track; tracks 2–11 are music.

**Status:** CD audio is external disc media, not an extracted source artifact.
