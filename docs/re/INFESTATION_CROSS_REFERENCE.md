# Infestation cross-reference

Infestation (Frontier Developments, 2000) runs on the C++ successor of V2000's
engine. A comparison of the two retail executables, by instruction-level
function similarity, shared strings and identical data, shows which V2000
subsystems survived into Infestation and which were rewritten. This note
records those conclusions so that each game can serve as a second witness for
the other: when a V2000 routine is hard to read, its Infestation counterpart
may already be understood, and the reverse.

The comparison tooling and Infestation's own analysis notes are private, like
the rest of the primary evidence. A counterpart is a lead to confirm in
`V2000.EXE`, never evidence on its own: even near-identical functions can
differ in constants, field offsets and calling conventions.

## Lineage

| | V2000 | Infestation |
|---|---|---|
| Build | 1998-09-16, MSVC 5 linker | 2000-07-14, MSVC 6 linker |
| Engine source tags | `C:\Coding\FGDK\Code\Windows\DDCalls.c`, `Graph2D.c`, `movie.c` | `C:\Coding\Fgdk2\Code\Windows\DDCalls.cpp`, `Graph2D.cpp`, `movie.cpp` |
| Game source tag | `c:\Coding\virus2\code\common\objtable.c` | `C:\Coding\V3000\Code\Common\Objtable.cpp` |

Infestation's project was named "V3000", and the engine moved from C (FGDK)
to C++ (Fgdk2). Both executables import the same eleven DLLs and reach
Direct3D only through DirectDraw. About 300 strings of eight or more
characters occur in both. They include the DirectX error tables with identical
typos ("Somethic went wrong (generic)", "Other appplication has priority"), the
object-table fatal errors, the multiplayer message files (`Host.txt`,
`Names.txt`, `Die.txt`, `kill.txt`, `selfkill.txt`) and the settings value
names. The object table is the same source file, edited: its `__LINE__`
arguments moved from 362/371/376 to 365/374/379, and its C `cdecl` returns
became C++ `ret 4`.

## Shared code

About 14% of V2000's game-code bytes have an Infestation counterpart at
normalized-instruction similarity 0.7 or more (18.6% at 0.6). They concentrate
in `0x00470000..0x0047FFFF` (53% of that range) and `0x00490000..0x004AFFFF`
(39-53%). The hardware fill slots (`0x00481240..0x00492A10`) and nearly all
game logic have no counterpart.

| V2000 | Subsystem | Infestation | Similarity |
|---|---|---|---|
| `00472680..0047EB40` | [Software rasterizer](SOFTWARE_RASTER.md): clipping, scan conversion, edge routines, span fillers | `004A4C60..004B6750` | 0.72-0.95 |
| Graph2D `+0x101C..+0x10C4` | Fill slots installed by `FUN_00480D10` | Software renderer vtable `0x00528FF0`; Infestation slot = V2000 slot - `0xFFC` | 0.76-0.78 at `+0x1034/+0x1044/+0x104C/+0x108C/+0x1094`; other polygon handlers reworked |
| `00498010..004A7680` | Hardware texture-cache pixel conversion | `004F9260..00508900` | 0.64-0.95 |
| `004A9570..004AB540` | DirectDraw modes, RGB555/display conversion (`004AA2E0/004AA330`), window procedure | `0050B4A0..0050F830` | 0.71-0.93 |
| `004AC5D0..004ACE80` | DirectInput wrappers | `0050F950..00510330` | 0.77-1.00 |
| `0046BA70..0046CB50`, `00438EF0..00439E90` | DirectPlay sessions and kill/host messages | `00492F50..00494230`, `0045C8D0..0045DB20` | 0.70-0.98 |
| `00495F30..004960F0` | Microsecond clock (`00495FF0`) and timers | `004A47D0..004A4990` | 0.71-1.00 |
| `00496130`, `00496460` | CD audio through MCI | `004F57D0`, `004F5DF0` | 0.72-0.85 |
| `00494EA0`, `00495BC0`, `00495CC0` | AVI playback (Quartz) | `00494A50`, `004F4C10`, `004F4D20` | 0.82-0.87 |
| `00471030..004711B0` | Error scopes: push, cleanup, raise (`00471150`), pop | `004948A0..00494A00` | 0.85-1.00 |
| `00471400..004717C0`, `00428EA0..004291A0` | Input poll, event ring and binding sets | `004F0040..004F0200`, `00439AA0..00439DA0` | 0.76-0.88 |
| `0043A0C0..0043A870` | Object table: generational handles (`FUN_0043A580` resolves) | `00461CE0..00462460` | 0.76-0.93 |
| `00448950..00448A60` | [Save](SAVE_AND_SETTINGS.md) CRC32 and first-valid-copy reader | `00449E00..00449F50` | 0.74-1.00 |
| `0044C710..0044CCA0` | World sound emitters and logical sound collection | `0047FEF0..00480360` | 0.75-1.00 |
| `00412CC0`, `00457680`, `004576E0`, `00457730`, `00457F70` | Sine lookup, Q31 division, integer square root, arcsine | `0042A7D0`, `0048EF30`, `0048EF80`, `0048EFC0`, `0048F520` | 0.74-0.98 |
| `004572B0..00457370` | Allocate, free and fill wrappers | `00495940..004959E0` | 0.72-0.93 |
| `00445920` | Water wave height (three summed sines) | `00410F60` | 0.82 |
| `00466410..004669A0` | Model register operations | `004A0260..004A0770` | 0.51-0.78 |
| `0046B8C0` | Collision-program walker called by `FUN_0046AF20` | `004F3650`, called by the auxiliary-program interpreter | 0.87 |
| `00470F80` | Text width measurement | `004A4240` | 0.86 |
| `00428AD0`, `00428B00` | 50 Hz tick reset and advance | `00439530`, `00439560` | 0.71-1.00 |

Infestation's retail software renderer is therefore largely this port's
reference rasterizer; its Direct3D renderer was rewritten and shares no code
with V2000's hardware path.

## Byte-identical data

| Content | V2000 | Infestation | Bytes |
|---|---|---|---:|
| Quarter-wave sine (4096 words, `0x4000` = 90 degrees), then the arcsine table | `0x004D14D0`, `0x004D34D0` | `0x0053DF00`, `0x0053FF00` | 10,245 |
| [Span reciprocal](RENDER_PIPELINE.md#key-addresses), 641 entries | `0x004D4B98` | `0x005415F8` | 2,564 |
| 100-vector [displacement table](GAME_MECHANICS.md) | `0x004CD4B8` | `0x00539960` | 604 |

The data order also survived: sine, arcsine, model dispatch tables, then the
reciprocal table. V2000 reads the arcsine table in `FUN_00457F70`,
`FUN_00457FA0` and `FUN_00457FB0`; these notes do not document them yet.

## Formats

| Content | V2000 section | Infestation handler |
|---|---:|---:|
| Sprites and 2048-byte-stride atlas | 3 | 3 |
| Fonts | 4 | 4 |
| Display records | 5 | 5 |
| Shade and light table | 6 | 6 |
| RGB555 colour words | 7 | 7 |
| Models and collision programs | 8 | 8 |
| Strings | 2 | 0 |
| Sounds | 11 | 9 |
| 256-entry terrain cell-to-model table | 9 | 11 |
| Terrain grid with a 0x14-byte header | 10 | 12 |
| Actor type and constructor records | 12 | 13 |
| Level spawns and placements | 13 | 14 |

Infestation's overlays drop the `"abcd"` markers, skip handlers whose count is
zero, and replace the flat PRELOAD matrix with a scene tree whose segment lists
map local to global slots. In both games the display tier selects the overlay
variant (V2000's leading digit, Infestation's `d` digit in `NdM.ovl`).

- **Sprites.** The 28-byte record is identical: id, zero word, flags, palette
  size or shade count, atlas offset, palette offset, width, height, and two
  runtime slots released at unload. Flag bits `0x01` (key), `0x02` (raw
  RGB555), `0x08` (`source + destination/2`) and `0x10` (additive) mean the
  same in both. Infestation adds `0x20` darken and `0x8000` shared palette, and
  wraps the record in a per-render-mode variant entry.
- **Fonts.** The 0x40-byte header and the four-word glyph metrics match field
  for field, including the fallback byte 127. Infestation adds a mode-2 glyph
  grid in the words that are zero in V2000.
- **Display records.** V2000 stores `{width, height, 0, 0x02000010,
  0x00010101}`; Infestation stores `{width, height, 0, 0x02010010, 0x00010101}`
  with an added 512x384 tier.
- **Section 6.** Infestation's handler 6 holds V2000's 26 entries from slot 7
  onwards: the eight-step grey ramp (rounded to exact multiples of 18), then
  `(4,4,4)`, `(0,0,0)` and the sixteen-step ramp unchanged. The fourth byte,
  V2000's software shade row, is zero.
- **Models.** The container rules match: flag `0x20` forces the near table and
  `0x40` omits the name; vertex slots are doubled with odd slots mirrored in X;
  the normal pool holds `(n-2)/2` entries after an implicit pair; packed
  operands decode the same way. Infestation moves the header into 32 bytes and
  stores six-byte vertices and normals. Its `+0x0E` word is V2000's `+0x0A`
  collision radius, and both gate the collision program on it.
- **Model opcodes.** V2000's register operations `0x0D..0xFD` are Infestation's
  `0xF0..0xFF` in the same order. The frame-select jumps `0x2B/0x2C` became
  `0x16`; billboards `0x68/0xE8` and `0x78/0xB8/0xF8` became `0x19` and
  `0x1A/0x1B`; depth keys `0x06/0x26` and `0xA6` became `0xE8/0xE9` and
  `0xED`; faces moved to `0xD0..0xE6`, with mirror instancing as bit 10. The
  per-vertex `type_flag` generators became explicit opcodes `0x20..0x2D` with
  the same operations. Neither game has frame-marker records: mesh animation
  is register-driven in both.
- **Collision programs.** Infestation's handler-8 auxiliary program is this
  [collision program](FORMAT_DOCUMENTATION.md#collision-program--corrected-2026-07-13),
  with the same opcodes (`05`, `31`, `88`..`8F`, `90`, `95`) and alignment
  rules. Infestation also embeds vertex-generator opcodes in it.
- **Terrain.** The Section-10 header lines up with Infestation's handler-12
  header: sea level (`>>8`, with the same `-6144` value parking the water of
  dry worlds), light direction, and at `+0x10` the word pair V2000 uses for its
  underwater darkness ramp. Its commonest value, (-4096, 0), is the commonest
  in Infestation too, where the field is not yet decoded. Both grids are X-major
  with signed heights `<< 5` and wrapping 16-bit world coordinates. Both reduce
  four-corner material blends by the square's eight symmetries: V2000's five
  materials give 120 tiles, Infestation's four give 55.
- **Sounds.** The 0x14-byte records, 22050 Hz mono PCM, recursive alias chains
  and 24-voice cap match.
- **Saves.** Both use reflected CRC32 `EDB88320` with seed 0 and no final XOR,
  0x80-aligned redundant frames read first-valid-first, and 14 slots.

## Runtime conventions

- The microsecond clock is `timeGetTime()*1000`, with an 8 ms minimum step and
  a 125 ms clamp; simulation runs before presentation. Infestation's scheduler
  is new C++ code.
- Angles are 16-bit (`0x4000` = 90 degrees), folded with `0x3FFC` into the
  quarter-wave table, and products are Q31.
- V2000 handles keep the generation in the low word, the slot in bits 16..25
  and the owner table in bits 26..31; actor handles such as `0x048B0001` are
  owner 1, slot `0x8B`, generation 1. Infestation inserts a mark bit at 26 and
  moves the owner to bits 27..31.
- Settings live under `Software\Frontier Developments Ltd\<game>\1.0`, with
  the shared value names in the same order and the default player name
  "Novice".
- The random generators differ: both are 32-bit LCGs returning the high word,
  but V2000 uses 214013 and 2531011 while Infestation uses `0x13D56D` and
  `0x13D573`.

## Not shared

Infestation rewrote the Direct3D backend, the model and collision interpreter
bodies, the overlay loader, the scheduler and mode pump, the primitive queue,
terrain storage and drawing, particles, and all actor, task and behaviour
logic. For those, Infestation offers analogies and vocabulary, not V2000
behaviour.
