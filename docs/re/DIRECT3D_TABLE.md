# Direct3D fill table (unused)

`FUN_00480D10` binds Graph2D's 43 fill slots (`+0x101C..+0x10C4`) from a mode
descriptor. Bits per pixel must be 16, and the descriptor's table word selects
the handlers: 0 for the [software raster](SOFTWARE_RASTER.md), 1 for a
Direct3D table. Any other combination fails with error `0x802`.

`FUN_004945E0` builds that descriptor from a Section-5 display-mode record:
bits per pixel from byte `+0x0C`, the table from byte `+0x0D`. It also passes
whether that byte is 1 to the surface setup, `FUN_004A8920`. Retail's four
records (320x240, 640x480, 800x600 and 1024x768, in `1X2XX.OVL`) all hold 16
and 0. Retail V2000 therefore never installs this table: it is FGDK code
compiled into the executable. The port does not implement it.

## Device

The table drives DirectX 5's `IDirect3DDevice2`:

- **Scene.** Before drawing, `FUN_004AB050` begins the scene (`BeginScene`,
  vtable `+0x28`) unless its flag records that it already has.
- **Vertices.** Polygon slots write each corner as a 32-byte transformed and
  lit vertex: screen x, y, depth, reciprocal w, ARGB colour, specular, u and
  v. The colour is packed from the surface's 565 channel shifts.
- **Textures.** Untextured slots clear the bound texture handle
  (`SetRenderState(1, 0)`, vtable `+0x5C`) when one is bound. Textured slots
  fetch the material's texture through `FUN_004A7D30`'s cache, which creates
  it on first use and ages each entry's priority.
- **Clipping.** `FUN_00481570` passes a polygon whose corners all lie inside
  the Graph2D rectangle. Otherwise it runs one Sutherland-Hodgman pass per
  edge (`FUN_00481690`, `FUN_00481E80`, `FUN_00482450`, `FUN_00482A50`).
- **Drawing.** The clipped fan is drawn with `DrawIndexedPrimitive` (vtable
  `+0x78`): a triangle list of transformed and lit vertices, the shared fan
  indices at `0x004D5C70` (0,1,2, 0,2,3, ...) and flags `0xC` (no clipping,
  no extents update). A `DDERR_WASSTILLDRAWING` result retries once.
  `FUN_00483D90` clips and draws an N-gon for the lit fog triangles, and
  `FUN_00489DA0` draws an untextured quad as one fan when it is convex and
  splits it into triangles when it is not.

## Slots

| Slot | Software | Direct3D | Notes |
|---|---|---|---|
| `+0x101C` | `FUN_0047ABB0` | `FUN_004292A0` | plot one pixel; Direct3D returns 0 |
| `+0x1020` | `FUN_0047AC30` | `FUN_0047AC30` | word-image blit, shared |
| `+0x1024` | `FUN_0047B360` | `FUN_004292A0` | line; Direct3D returns 0 |
| `+0x1028` | `FUN_0047B6C0` | `FUN_00480CF0` | faded line; Direct3D returns 0 |
| `+0x102C` | `FUN_0047AB20` | `FUN_0047AB20` | clip rectangle, shared |
| `+0x1030` | `FUN_0047AD90` | `FUN_00480A90` | sprite, see below |
| `+0x1034` | `FUN_0047B9F0` | `FUN_00480A60` | fill; Direct3D uses a colour-fill blit (`FUN_004A9C20`) |
| `+0x1038` | `FUN_0047BAB0` | `FUN_00481240` | flat triangle |
| `+0x103C` | `FUN_0047BB70` | `FUN_00483030` | fog triangle |
| `+0x1040` | `FUN_0047BD80` | `FUN_004834F0` | tinted triangle |
| `+0x1044` | `FUN_0047BE40` | `FUN_004838D0` | lit fog triangle |
| `+0x1048` | `FUN_0047C150` | `FUN_00483E40` | Gouraud triangle |
| `+0x104C` | `FUN_0047C2F0` | `FUN_00484220` | per-vertex lit fog triangle |
| `+0x1050` | `FUN_0047C600` | `FUN_004846E0` | textured triangle |
| `+0x1054` | `FUN_0047C7C0` | `FUN_00484C20` | textured fog triangle |
| `+0x1058` | `FUN_0047CBE0` | `FUN_00485270` | textured triangle, one palette row |
| `+0x105C` | `FUN_0047CD30` | `FUN_004858B0` | textured triangle, one shade, fog |
| `+0x1060` | `FUN_0047CFA0` | `FUN_00485F20` | textured triangle, per-vertex shade |
| `+0x1064` | `FUN_0047D240` | `FUN_00486530` | per-vertex shade and fog |
| `+0x1068..+0x107C` | six unreachable handlers | `FUN_00486BA0`, `FUN_004872A0`, `FUN_00487A60`, `FUN_004881F0`, `FUN_00488A80`, `FUN_004891F0` | no thunk or call site |
| `+0x1080` | `FUN_0047E150` | `FUN_00489A60` | flat quad |
| `+0x1084` | `FUN_0047E220` | `FUN_0048A040` | fog quad |
| `+0x1088` | `FUN_0047E490` | `FUN_0048A5A0` | tinted quad |
| `+0x108C` | `FUN_0047E560` | `FUN_0048A9D0` | lit fog quad |
| `+0x1090` | `FUN_0047E930` | `FUN_0048B030` | Gouraud quad |
| `+0x1094` | `FUN_0047EB40` | `FUN_0048B460` | per-vertex lit fog quad |
| `+0x1098` | `FUN_0047EF10` | `FUN_0048BA20` | textured quad |
| `+0x109C` | `FUN_0047F060` | `FUN_0048C250` | textured fog quad |
| `+0x10A0` | `FUN_0047F320` | `FUN_0048CB90` | textured quad, one palette row |
| `+0x10A4` | `FUN_0047F480` | `FUN_0048D4A0` | textured quad, one shade, fog |
| `+0x10A8` | `FUN_0047F750` | `FUN_0048DE50` | textured quad, per-vertex shade |
| `+0x10AC` | `FUN_0047F9E0` | `FUN_0048E7A0` | per-vertex shade and fog |
| `+0x10B0` | `FUN_0047FCC0` | `FUN_0048F160` | unreachable |
| `+0x10B4` | `FUN_0047FE00` | `FUN_0048FBD0` | unreachable |
| `+0x10B8` | `FUN_004800B0` | `FUN_00490730` | unreachable |
| `+0x10BC` | `FUN_00480200` | `FUN_00491280` | unreachable |
| `+0x10C0` | `FUN_004804C0` | `FUN_00491EC0` | explicit UVs, per-vertex shade |
| `+0x10C4` | `FUN_00480790` | `FUN_00492A10` | explicit UVs, per-vertex shade, fog |

The Direct3D handlers for `+0x1068..+0x1080` are only ever stored, never
called directly, so Ghidra's census missed them; they are seeded into it by
hand.

The Direct3D sprite handler draws a half-additive sprite (material flag
`0x08`) of at most 256x256 through the textured-quad thunk `FUN_0047AA20`, so
that it blends. It clips any other sprite to the Graph2D rectangle and blits
it from its DirectDraw surface (`FUN_004A9EB0`, `FUN_004A9DC0`).

## Omitted from the progress report

Since retail never installs the table, the progress report omits the 59
functions only it reaches. They are a forward closure from every function
that two dead sites name: the table-1 branch of `FUN_00480D10`
(`00480D99..00480FAB`) and the block of `FUN_004A8920` that creates the
Direct3D device and viewport (`FUN_004AA390`, `004A8B6C..004A8C0C`), which
`FUN_004A8920` enters only when `FUN_004945E0` passes the mode's table byte
as 1. The closure follows calls, code references and the function tables they
name, and keeps out any function that live code or a live table also reaches.
Besides the handlers it holds the clipping passes (`FUN_00481570`, its four
edge passes and their shared `FUN_00481C80`), the N-gon and quad helpers
(`FUN_00483D90`, `FUN_00489DA0`), the scene start `FUN_004AB050`, the sprite
blit `FUN_004A9DC0`, and the textures and texture pages (`FUN_004A9EB0`,
`FUN_004AA0E0`, `FUN_004AA140`, `FUN_004A7D30`, `FUN_004A7F10`,
`FUN_004AA8E0`, `FUN_004AAF80`). The census splits two handlers at values
of the span-reciprocal table: `0x0048D159` and `0x0048FADE` are the ends of
`FUN_0048CB90` and `FUN_0048F160` ([census artifacts](SMALL_FUNCTIONS.md#census-artifacts)).
