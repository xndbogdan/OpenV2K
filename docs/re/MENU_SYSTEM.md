# V2000 Menu / Frontend System

Ground-truth investigation of the menu system (2026-06-09 → 2026-06-10;
**deep RE pass 2026-07-05**: sounds, render pass, scene/animation/timing,
input — all decoded to the byte level, adversarially verified).
Companion docs: `../port/crates/v2k-game/data/menu_tree.json` (screen tree),
`reference/menu_render_pass.json` (layout/fonts/context dumps),
`v2k-extract sounds` (regenerable canonical WAV pool), `MENU_FEEL_GAPS.md`
(ranked port-vs-original behavior diff).

## Status

| Area | Status |
|------|--------|
| Menu prop models (which/where stored) | ✓ DECODED |
| Settings registry map + runtime struct location | ✓ DECODED |
| Mode/game-state machine (descriptors) | ✓ DECODED |
| Menu screen engine (records, items, stack) | ✓ DECODED (record layout corrected 2026-07-05) |
| Full screen tree + string ids | ✓ DECODED (menu_tree.json, 34 screens) |
| Menu sounds (exact samples, params, volume math) | ✓ DECODED + WAVs exported |
| Menu music | ✓ DECODED — **frontend is silent**; CD music is in-game only |
| Text rendering (fonts, colors, typewriter) | ✓ DECODED — Section 4 = two sprite fonts |
| Prop ring placement math / transitions | ✓ DECODED |
| Camera / fog / background animation | ✓ DECODED (old "yaw target" reading corrected) |
| Key input incl. repeat semantics | ✓ DECODED — edge-triggered, **no auto-repeat** |
| Attract/demo sequence | ✓ DECODED (exit-action consumers found) |

## Time base (calibrates every number below)

- All engine clocks are **microseconds**: `FUN_00495FF0` = `timeGetTime()*1000`
  (int64). The frame pump `FUN_00494070` passes `delta_µs` to every
  update/render chain member. Min frame period (task+0x20) = **8000 µs**
  (125 fps cap); no fixed 30/60 Hz step — everything is delta-driven.
- Frontend clamps the frame delta to 0x1E848 = 125,000 µs (`DAT_004DB20C`,
  FUN_0042AA10).
- `DAT_004FED60` = **50 Hz tick counter**: 64-bit µs accumulator
  `DAT_004DB0E0` ÷ divisor `DAT_004BED80` = 20,000 µs (FUN_00428B00);
  reset to 0 at menu/AVI init (FUN_00428AD0).

**Port implementation (2026-07-17):** one `RetailTickClock` now carries the
sub-tick µs remainder and publishes a single wrapping integer snapshot to every
consumer in an active frame. Frontend, Intro2, post-intro, and gameplay advance
it once; the intro AVI, synchronous loading, and gameplay pause freeze it.
Successful level loads and confirmed frontend teardown reset it, and effect
tick cursors rebase at those boundaries. Klaus's phases and random sampling
follow its callbacks independently of this clock. The pause shell's authored
fly/typewriter clocks also remain delta-driven.

## Opening AVI playback

`BANNERHI.AVI` and `BANNERLO.AVI` contain the complete Grolier, Frontier and
V2000 idents: 314 frames at 15 Hz, respectively 640x480 and 320x240, with
22,050-Hz stereo unsigned 8-bit PCM. Both repository files are byte-identical
to the installed retail copies. Their Microsoft Video 1 / CRAM frames carry
RGB555 blocks and can retain pixels from earlier frames.

The port's [AVI decoder](../../crates/v2k-formats/src/avi.rs) owns both the
block stream and its dependency history. A zero flag word is a valid two- or
eight-color block while image blocks remain; it is only an end marker after
all blocks have been consumed. The former unconditional early return left
stale image regions in 155 high-resolution frames and one low-resolution
frame. Independently, the presentation loop's direct wall-clock frame requests
could omit delta predecessors after a stall. `decode_frame` now catches up
every missing packet, caches repeated requests and resets/replays backward
requests. Empty packets hold the previous picture.

Cold startup opens the PCM device paused and starts audio and the video clock
only after frame zero is presented. Sound/music initialization therefore cannot
consume the beginning of the movie. Presentation borrows the decoded image
instead of copying the entire RGBA buffer on every display refresh.

Both complete variants match FFmpeg's RGB555 output at every AVI timestamp,
including requests that display only every third frame. Reference extraction
must use `-fps_mode passthrough` and retain frame PTS: the high-resolution file
has twelve empty packets at indices 181..192, which FFmpeg's default constant-
frame-rate output can resample. Both concatenated PCM streams also match
FFmpeg byte-for-byte (923,144 bytes). Focused synthetic tests cover zero-word
blocks, skipped dependencies, repeated/empty frames and backward requests.
An optimized hidden OpenGL check makes 428 presentations across both tiers,
including skipped, repeated and backward requests. Scheduled-versus-sequential
image mismatches fall from 116 before the repair to zero; every rendered RGB
pixel matches the decoder output. The Grolier ghosted-text corruption is
visibly reproduced before the change and absent afterward.
These checks establish decoder correctness; they do not measure device-level
audio underruns.

## Layer 1 — Mode state machine (FGDK task descriptors)

The game's top-level state (intro AVI → menu → game → results) is driven by
**14-dword mode descriptors** in `.data`, staged into the active mode object by
`FUN_004942C0(mode_obj, descriptor)`. Switch helpers: `FUN_0042A9B0(session,
desc)` / `FUN_00456680(session, desc)`. **`FUN_0042A9B0(session, NULL)` =
quit to desktop** (`FUN_00495ED0`).

Descriptor layout (pairs of `{value, ctx}`): [1] init handler, [4] → primary
chain (null-terminated fn-ptr list), [6] → preceding chain, [8] per-frame tick
(`FUN_00428D00` = display-surface upkeep, not animation), [9] mode object,
[10] exit handler, [12] aux handler. `004942E0` runs [6] before [4]; for
the in-game descriptor [6] drives simulation and [4] presents the world.
See the [active world frame order](LOADING_TRANSITIONS.md#active-world-frame-and-presentation-order).

| Addr | Mode | Init | Primary chain |
|------|------|------|--------------|
| 0x4CA7C0 | Intro AVI player | FUN_0042AAE0 | 0x4CA758 → FUN_0042AE00 |
| 0x4CA7F8 | **Frontend menu** | FUN_0042B230 | 0x4CA760 → 42AA40,42AA60,42B870,42AA80,42BA30,42AA90,42AAB0; render 0x4CA788 → 42B5D0 |
| 0x4C9840 | Attract/demo loader | FUN_00426320 | 0x4C9830 → FUN_004263F0 |
| 0x4D0918 | In-game | FUN_004515E0 | 0x4D07A8 |
| 0x4D0950/0988/09C0/09F8 | Results A/B/C | 0x453330/451390/451420 | via ptr table 0x4D0DA0 |

The late-demo differential closes the promotional page inside this same mode
system. Retail event dispatcher `0044FCD0`, predecessor descriptor `004D0A30`,
phase/flag selector `00453E00`, and switch helper `00456680` correspond to demo
`0044F4D0`, `004CAA00`, `00453560`, and `00455FE0`; their control path is the
same and the switch pair is instruction-exact. Both select completion/
progression descriptor retail `004D0AA0` / demo `004CAA70`. Its content slot is
the actual delta: retail `FUN_00454390` renders normal progression/result
content, while demo `FUN_00453AF0` reads and draws all twelve appended X3
strings at global ids `300..311`. The paired initializers load system overlay
51 and exit pair `004556A0` / `00454FF0` is instruction-exact. Thus the demo
reuses normal completion activation and substitutes page content; the exact
Level-15 input writer and post-Space destination remain unclassified.

`DAT_004DB200`: 0 = intro AVI, 1 = **scene-only intro** (3.0 s), 2 =
interactive menu. **There is NO palette/alpha fade during state 1**: the 3D
scene, background flash, fog animation and Klaus actor all run normally, but
FUN_0042BA30 skips the screen renderer (FUN_0043AA40) and FUN_0042B5D0 skips
menu input/animation (FUN_0043C7F0). At tick counter `DAT_004FED60 > 150`
(= 3.0 s at 50 Hz) state → 2 and the main ring appears via the normal
fly-in transition. The perceived "fade-in" = 3 s of live scene, then the UI
flies in over 0.63 s.

## Layer 2 — Global resource pools (cumulative ids)

Pool pointer array base `DAT_004FE620`, one slot per section: `pool[k]` at
`0x4FE620 + 4k`. Populated by LoadPreload `FUN_00493860` from the PRELOAD.DAT
15×53 count matrix (level order = `PTR_DAT_004D0020`); per-level cumulative
base at `level_record+0x10+type*8`, count at `+0x14+type*8`.

| Slot | Section | Contents / menu-relevant bases |
|------|---------|-------------------------------|
| 0x4FE620 | S0 | live display block ptrs: [0..5] ≈ x0,y0,**width**,**height**,center_x,center_y (code tests ÷640/480) |
| 0x4FE624 | S1 | **layout points** (packed s16 x,y) — see Text & layout |
| 0x4FE628 | S2 | strings: L2 #0–120, L3 #121–302, L4 #303–304, L5 #305–309 |
| 0x4FE62C | S3 | sprites: L0:0, L1:11, L2:42, L3:419, **L5:1285** (+37), L6:1322 … L51:3733 |
| 0x4FE630 | S4 | **fonts** (2 records, L2 only): [0] green normal, [1] yellow selected |
| 0x4FE638 | S6 | shade ramps |
| 0x4FE63C | S7 | colors (clear color = entry 32; menu terminal depth-fade color at ctx+0x7C = entry 55 = RGB555 pure red) |
| 0x4FE640 | S8 | models: L2 0–12, L3 13–314, **L5 315–323 = menu props** |
| 0x4FE648 | S10 | terrain |
| 0x4FE64C | S11 | **sounds: L2 ids 0–6, L3 ids 7–109** (all other levels: 0) |
| 0x4FE650 | S12(§16 types) | entity type records (`g_resource_table`) |

Level-5 menu prop model global ids: 315 flags, 316 screenop, 317 screeno2,
318 sfxopt, 319 sfxopt2, 320 slopt, 321 pcontrol, 322 psjoypad, 323 multipc.
(L3 props: 41 = `player4` vehicle, 75 = `optexit`, 73 = `optionsh` backdrop.)
`multipc` itself authors three opcode-`0x22` sprite ribbons using global sprite
608, radiating from its hub to the three computer branches. They are model
geometry: rotating with the Network prop is intentional. Their projected
endpoints and per-endpoint depth-fade bytes must remain attached to the body;
the frontend fade turns their white source toward Section-7 entry 55 red.
`FUN_00459000` passes that raw size short (`0x000C`) to `FUN_004594C0`; it is
not `FUN_00470840`-decoded. Packed decode would collapse 12 to 0 and force
the 1px perpendicular guard. The small authored size keeps the links as
narrow patterned ribbons rather than the sprite's 64×16 card.

## Layer 3 — Menu screen engine

### Screen record (0x38 bytes, .data/.rdata) — corrected 2026-07-05

```
+0x00 ptr  item_array
+0x04 s16  initial selection index
+0x08 u32  flags (see below)
+0x0C ptr  on-open callback (called when screen becomes top)
+0x10 ptr  on-switch-away callback(direction ±1)
+0x14      row groups: EXACTLY 3 × 10 bytes {u16 flags, u16 pos_id,
           u16 step_id, u16 window_rows, u16 item_count}
+0x34 ptr  per-tick callback(dt_µs), only when top-of-stack, no transition
```

Row-group fields: `pos_id`/`step_id` index the **Section-1 layout-point pool**
(`DAT_004FE624`, packed s16 x,y): pos = row start (px), step = per-item delta.
`window_rows` = max simultaneously visible rows (scrolling window) — the old
"col_base" reading was wrong. Row-group flags: bit0 = center text
horizontally, bit1 = 3D-prop ring row (x-coord becomes ring angle),
bit3 = typewriter reveal, bit4 = suppress re-reveal on scroll-into-view.
`menu_tree.json`'s 4th "group" is a garbage read past +0x32.

Screen flags: **4** = selection wrap (ring/list wraps last↔first). NOTE:
the static-RE guess that the wrap "jumps ±count×200 → spins the long way
around" was WRONG — user ground truth (2026-07-05, original hardware) is
that the ring loops CONTINUOUSLY: stepping off the last item rotates one
notch forward into the first (an infinite carousel), not a spin back to the
start. The port bumps the scroll interpolator by the circular-shortest
delta. **0x08** = fly transition in the IN-GAME (pause) context,
**0x10** = draw full-screen backdrop prop first (`FUN_0043B130`: global model
73 `optionsh`, angle 0, y=0x1004 → z=2900, projection center = layout pt5,
shade 0x1E00+z). `optionsh` contains two mirrored model-74 `anim` children;
their decorated side is local -Z. Retail's angle-zero menu basis presents that
side to the camera, equivalent to a π root turn in the port's identity-based
model convention. Without it the authored-plane cull rejects the emblem,
dark-green fill, inner frame, and hazard-tape faces and exposes only their
back-side pipes/light. **0x20** = fly transition only at the screen's base depth
(frontend depth 2 / in-game depth 1 — deeper pushes don't re-fly),
**0x40** = fly transition in the FRONTEND context, **0x80** = left/right act
as up/down (ring), **0x100** = up/down invoke the selected item's select_cb.
Context chosen by `FUN_00456780(FUN_0044F3D0())` session test. Bit0 (0x1) is
never tested in the nav/render cluster (legacy).

### Item record (0x1C bytes)

```
+0x00 u32  flags    (visibility mask vs FUN_0043CBA0(); bit3 = non-selectable)
+0x04 ptr  draw_cb(rt, ctx, reveal_s16, pos_ptr(s16 x,y in/out),
                   flags{bit0=selected→yellow font, bit1=centered,
                   bit3=measure-only/invisible}, id, arg)
+0x08 u32  id: string id | global model id (draw=FUN_0043B410) |
           packed {aux_hi16, string_id_lo16} (spinner items)
+0x0C u32  arg: live settings ptr (value rows) / label string id (props)
+0x10 ptr  select_cb(ctx, action, arg1, arg2)   action: 0=left 1=right 2=fire
+0x14 u32  arg1     (spinner: max value; cheat: weapon code)
+0x18 u32  arg2     (submenu: target screen record; spinner: settings ptr)
```

After each item the engine advances pos by the step point. Only the TOP
screen of the stack is rendered (no parents behind).

### Runtime state (BSS)

- `DAT_004DCEDC` = depth in the screen stack; slots at `0x4DCA10 + depth*8`
  = `{screen_ptr, s16 sel_index, s16 window_scroll}` (max 10 deep).
- `DAT_004DCEBC` (ring/horizontal) / `DAT_004DCEC0` (list/vertical) =
  selection scroll interpolators: cursor moves add ±200/step; each tick both
  decay `val -= (dt_µs · val·5/4) >> 18` → **time constant τ ≈ 0.21 s**
  (min step 1). Lists shift by `step.dy × val/200` px (whole list slides;
  the highlight itself snaps instantly via font swap). Ring:
  `x_angle = floor(65535/(count·200)) · (val + (idx−sel)·200)`.
- `DAT_004DCEB0` = ring prop spin angle: `+= (dt_µs·20000·2^12)>>32` ≈
  19,073 units/s → **one revolution per 3.436 s** (single axis; b4/b8 = 0).
- `DAT_004DCEC4/C8` = ms since screen open / since last scroll (typewriter).
- `_DAT_004CB4E0` = **fly transition clock**: armed to 0x7000 by
  `FUN_0043A8E0` when the old/new screen qualifies (flags 0x40/0x08 + 0x20
  rule); decays ≈ 45,776/s → **0.6264 s per leg**. While nonzero the stack
  does NOT switch (tick early-outs) and prop labels/typewriter are held.
  Fly-out: selected prop depth y = 0xDAC − (0x7000−clk)/16 (pulls toward
  camera), others y = 0xDAC + (0x7000−clk)/2 (recede). Then the stack
  switches, DCEC4 resets, clock re-arms for fly-in: props at y = 0xDAC +
  clk/2. Full ring↔submenu transition ≈ 1.25 s, then text types on.
  Prop-draw anim channels (LAB_0043B610): ch0 = DAT_004FED60 (50 Hz tick),
  ch1-2 = (0x7000−clk) during fly-out / clk during fly-in. This is visible in
  L5 `screenop`: its stream applies op-0x5C Y rotation from callback operand
  `0x81` before instancing `screeno2` (global model 317). The monitor therefore
  turns from 0→0x7000 while leaving the ring and unwinds 0x7000→0 while flying
  into Display, on top of the continuously advancing DAT_004DCEB0 root spin.
- Settings struct base = **0x4CB3D8** (.data): +0 Sound, +4 Ambient,
  +8 Joystick Mode, +0x10 Resolution, +0x1C Full Screen, +0x24 Self
  Righting, +0x28 Full Analogue, +0x2C Sensitivity, +0x30 Camera,
  +0x34 Targetter, +0x38 HUD, +0x3C Language. Accessor `FUN_0043C7C0()`.
  Initialized values read directly from `.data` are 15,15,1,0,1,1,1,1,
  0,1,0,10,6,1,1,0,15 for offsets +0x00..+0x40 respectively (including
  the unused gaps at +0x0C/+0x20). In particular Active Camera starts at 6,
  not 0, and Self Righting is a 0-15 strength rather than a boolean.

### Core engine functions

| Function | Role |
|----------|------|
| FUN_0043A960(screen) | push/open screen |
| FUN_0043B850 / FUN_0043A9A0 | back/pop (returns depth; arms fly-out) |
| FUN_0043C1E0 / FUN_0043C4A0 | cursor down / up (sound 0 only when it moves) |
| FUN_0043C6C0 / FUN_0043C720 | cursor left / right (or spinner dec/inc) |
| FUN_0043C450(ctx, action) | invoke selected item's select_cb |
| FUN_0043C780 | select dispatcher (throws 0xC00-0xC02 exit codes) |
| FUN_0043CBA0 | current item-visibility mask |
| FUN_0043AA40 | screen renderer (row groups, scroll, reveal, transitions) |
| FUN_0043C7F0 | screen-engine tick (all interpolators above) |
| FUN_0043B3E0 → FUN_0043B1C0 | draw text item |
| FUN_0043B410 | draw 3D prop item (ring math below) |
| FUN_0043B650/B690/B740 | setting rows (label + value / spinner bar) |
| FUN_0043B7C0(ctx,dir,max,&val) | spinner: sound 1 on change, 0 at limit |
| FUN_0043B820 | open submenu (sound 3) |
| FUN_0043BB20 | save/load slot rows (multi-line, writes back pos.y) |
| FUN_0042BD60 | main-ring select: prop anim cmd 4 + open submenu |
| FUN_0042BBF0 | ring vehicle select → start game (+ **CD disc check**, see Music) |
| FUN_0042D010 | Exit prop → quit |

## Text rendering & fonts (Section 4 = FONTS)

**OVL Section 4 of level 2 contains the two menu fonts** (loader
`FUN_004ABA70`, handler table 0x4CE0D8 + 4·0x10 — FORMAT_DOCUMENTATION's old
"difficulty-scaled entity stats" reading was wrong; see corrections there).
Font record: 0x40 header; +0x10 fixed-point scale divisor = 100 (running pen
coordinates use 1/100 px); +0x14/+0x18 line advance+gap (lo-res:
(1041+244)/100 = 12.85 px wrap step); +0x2C fallback char 0x7F; +0x38 →
256×8-byte glyph metrics
{s16 advance, s16 kern, s16 xoff, s16 yoff}, where advance/kern use the
fixed-point pen units but xoff/yoff are signed whole pixels; +0x3C → 256×u32 glyph **sprite
ids** into the global Section-3 pool. Font 0 = GREEN glyphs (normal rows);
font 1 = same metrics, sprite ids **+173** = YELLOW glyphs (selected row).
**Selection highlight = font swap only — no palette math, no pulsing.**
Metrics (lo-res): space 2.00 px, '0' 5.00+1.00 kern, 'A' 6.00+1.00,
'M' 8.00+1.00.

Pipeline: `FUN_0043B3E0` → `FUN_0043B1C0(rt, pos, flags, reveal, label,
value)` → `FUN_00470AE0` (word-wrap at spaces, wrap width = 60% of screen
width) / `FUN_00470A70` (single line) → `FUN_00470B60` per glyph →
`FUN_00470F30` blit at `(pen_x/100 + xoff, pen_y/100 + yoff − sprite_h + 1)`
— **y is the baseline**. Sprite resolver callback = 0x4291D0 =
`return DAT_004FE62C[id]` (passing NULL = measure-only pass, used to measure
scrolled-out items). Setting rows: value string = `string_pool[label_id + 1 +
*value_ptr]`, drawn right-aligned at `label_x + pt7.x (185)`. FUN_0043B740
draws "Off" at value 0; otherwise it passes `filled=value−1` to FUN_0043B690.
FUN_0043B690 treats the packed row-id high word as the **total glyph width**
(Sound/Ambient = 14, Sensitivity = 15): it builds `[0x1B, 0x1C..., 0x1D]` to
exactly that width, then overwrites the first `filled` glyphs with 0x1E. Thus
the fill overwrites the left cap as soon as `filled>0`, and can overwrite the
right cap when `filled == width`. Glyphs: 0x1B '[' cap, 0x1C empty, 0x1D ']'
cap, 0x1E filled block (L2 sprites 42–45 / yellow 215–218). **No arrow
sprites on spinner rows.** Active Camera is the exception to the `B740`
"Off" wrapper: its record calls `FUN_0043B690` directly with packed width 10,
so value 0 is a zero-filled 10-glyph bar and value N fills N glyphs. A vertical
clip rect `[row_y0 − step.dy, row_y0 + 5·step.dy]` is set per text row group.

Runtime comparison on 2026-07-11 exposed a port-only rasterization collapse:
the 5-pixel bar cells met edge-to-edge after scaling, erasing the visible dark
cell breaks in both filled and empty runs. The menu bar path now preserves a
one-virtual-pixel break inside each cell advance while retaining the original
14/15-glyph total width and cap/fill construction.

**Typewriter reveal** (row flag 8): each item's text truncated to
`(strlen+1)·reveal >> 15` chars, `reveal = clamp((DCEC4_ms − idx·100)·100,
0, 0x7FFF)` — 100 ms stagger per item, each types on over 327.67 ms, starting
only after the fly-in clock reaches 0. Items scrolled into view re-type via
DCEC8 unless row flag 0x10. Labels grow from the row origin. Value-column
text and bars right-align that same prefix at `label_x + pt7.x` so the
option appears from the right.

**Layout points** (L2 variant0, 320×240): pt0=(160,155) ring
anchor/projection center, pt1=(?,55) billboard y, pt3=(67,105) settings-list
start, pt4=(0,14) row step, pt5=(160,106) backdrop projection center,
pt6=(160,170) settings-screen prop center, pt7=(185,0) value column,
pt12=(160,220) prop-label/bottom-text anchor. System2 scalars 2/3 supply the
selected canvas; each high tier has its own Section1 points:

| Tier/canvas | List start pt3 | Row step pt4.y | Prop center pt6 | Value offset pt7.x | Bottom anchor pt12 |
| --- | --- | --- | --- | --- | --- |
| 1 / 640×480 | (134,212) | 27 | (320,360) | 370 | (320,420) |
| 2 / 800×600 | (167,265) | 27 | (400,450) | 462 | (400,540) |
| 3 / 1024×768 | (214,339) | 43 | (512,576) | 592 | (512,708) |

All three high fonts retain the same native pixels and metrics: A/N are
11×14 with advance 11/kern 1; p is 9×14 with advance 9/kern 1 and yoff 4; the
filled bar glyph is 10×20 with advance 10/kern 0. Their wrap line step is
25.69 pixels. These metrics are independent from the authored row steps
27/27/43. The
canonical parser-backed `menu_text_submission` control checks the three
selected canvases, metrics and spacing; the glyph decoder never applies a
presentation-dependent scale.

`UiSubmissionPolicy::NativeCanvas` maps complete High Native menu/story
compositions through the selected centred authored canvas. Above the original
outputs, it uses the port's continuous readability scale, bounded by the
complete-canvas fit: up to 1.8 at 1080p and 3.6 at 4K, or 1.40625/2.8125 for
the 1024×768 tier. Glyphs, sprite art, bars, row clips and frontend prop
anchors/pixel focal lengths share that mapping. Intrinsic decoding and font
metrics remain unchanged. `FitAuthoredCanvas` preserves Low and 4:3/Stretched;
their physical viewport enlargement remains separate from logical submission,
with Classic optionally composing through its authored framebuffer first.

Gameplay corner groups use `NativeHud { anchor }`, sharing sprite/clip scale
and model anchor/pixel focal length without fitting their unused complete
canvas. High Native `WorldOverlay` messages use actual viewport percentage
anchors and this HUD glyph scale, with wrapping bounded by available width,
baseline adjustment and drawable clipping. Matching original displays retain
the authored message submission. Overlay51/results prompts use
`AuthoredCanvas`, preserving their text/backdrop composition. The M modal,
entity icons and status panel retain their separate centred fit. World lenses,
full-frame video, background images and retained pause underlays keep their
dedicated policies. Matching 640×480, 800×600 and 1024×768 layouts retain their
original geometry; growth beyond them is a modern port-owned extension, not
retail acceptance. The scale and mode boundaries are owned by
[Render presentation](RENDER_PIPELINE.md#classic-framebuffer-colour-and-resolution).
High Resolution/Scaling/Classic changes stage the selected system layouts and
font, projection, HUD/radar/map-status snapshots before atomic publication,
without replacing intrinsic glyph pixels or advancing world/effect clocks.
Low/High artwork and renderer replacement remain their existing launch-time
boundaries. High layout selection retains the 1024×768 ceiling. The exact
resource transaction and invariant-byte checks are owned by
[System-overlay layout refresh](SYSTEM_OVERLAY_LAYOUTS.md).
The port's centered camera must convert pt5/pt6 into world-Y offsets rather
than treating them as ordinary model positions: angle-0 FUN_0043B410 still
contributes local Y=-0x140, placing the screen prop visibly below pt6.

## 3D prop ring (FUN_0043B410)

Position: `(X,Y,Z) = (800·sin θ, −320·cos θ, y_in − 1200·cos θ)`, θ = item
x-coord as 16-bit angle (0x10000 = turn) via quarter-sine u16 table 0x4D14D0;
`y_in` = 0xDAC (3500) for ring rows; instance scale = 800. Ring of 7:
angular step = floor(65535/1400)·200 = 9200 (≈50.5°); **selected prop at
θ=0 = front (Z≈2300), lowest and closest** — distinguished ONLY by position
+ its label (yellow font, centered at pt12, only when transition clock==0).
All props spin about one axis via DAT_004DCEB0 (3.436 s/rev). The ring row has
flags `0x03`. Before **every** flag-`0x02` item callback, `FUN_0043AA40` resets
the local depth-fade pair to
`near=min(world_ctx.near-0x900,0x200), far=0x200`; `FUN_0043B410` then shifts
that fresh pair by only the current prop's root Z before calling
`FUN_00470A10`. Ring roots therefore do not accumulate. The terminal color at
local context `+0x7C` is copied from the menu world context and is Section-7
entry 55, RGB555 `0x7C00` pure red. This path is separate from the Section-6
light table at context `+0x48`. Ring labels: L2 strings
26 'New Game', 27 'Display', 28 'Load Game', 29 'Network', 30 'Sounds',
31 'Controls', 66 'Exit'. Interpreter context template 0x4CB460 (0x80 bytes):
identity matrix, dwords[15..17] = (0x49,0x49,−0x49) light vector, +0x48 →
16 Section-6 color ids, +0x64 = 0x4D4A78 transform table, +0x68/6C/70 =
sprite/palette/model pool resolvers (0x4291D0/E0/F0), +0x74/78 initial local
near/far −0x200/0x200, +0x7C terminal depth-fade color.

## Menu 3D scene, camera, background

> **Scene composition CONFIRMED 2026-07-06 (read-only RE workflow):** both
> menu entities spawn **co-located at world origin (0,0,0)** with a pure 90°
> yaw (heading +0xA2=0x4000, pitch/roll 0) and **NO per-entity scale**
> (record scale fields +0x18/+0x1C = 0; the spawn record is fully zeroed).
> So the whole visible layout — big creature backdrop behind, low-center
> craft — is entirely in each **model's own local geometry**, not entity
> placement: a renderer draws BOTH models at origin, yaw 90°, no extra scale.
> Entity type (entity+0x58) = the record's class field (constructor-driven
> model attach via type_record[+0xC]). The camera is a **chase camera locked
> to the first type-1 `flag` anchor** (`FUN_0042B5D0` scans entity+0x58==1 →
> `FUN_0040ED10`→`FUN_0040F3A0` spring, game_logic.c:4472). The accepted
> `20260724-202233-menu-poses` sweep proves that the frontend trailing
> distance is the fixed **0x800=2048 raw**: camera eye `[0,0,-2048]` and
> focus `[0,0,249]` remain unchanged across every Active Camera value 0..10.
> The frontend call supplies zero dynamic camera arguments, so the gameplay
> Active Camera setting must not be added to this scene. `local_20=0xC00` is
> the pre-normalization direction basis, not the camera distance. Look-at =
> entity_pos + 250·forward. The tracked flag stays at origin, but Klaus is
> separately pinned to the camera: `FUN_0042C090` writes private view XYZ and
> `FUN_0040F9C0` identifies him for `FUN_0040ED10` to transform back to world
> coordinates. His private view Z is `0xA00` at rest, not the camera's `0x800`
> trailing distance; the submenu-away pose reaches `0x1400`.
> **NO terrain and NO water are rendered** (heightmap ptr NULL,
> terrain-scatter skipped, world-ctx terrain index 0, water 0) → black
> background behind the models. **Backdrop model RESOLVED 2026-07-06:** the
> type→model chain is `DAT_004FE650[type]` (**Section-12** per-type records,
> 0x128 B) `+0xC` = model index → `DAT_004FE640[idx]` (**Section-8** models).
> `DAT_004FE640`/`DAT_004FE650` are entries [8]/[12] of the section-pointer
> table `DAT_004FE620[N]`, concatenated across preloaded OVLs — so L2's 2
> Section-12 records = types 0,1 and L3's 128 = types 2-129 (why the entity
> catalog starts at type 2). **BACKDROP (type 0) = L2 Section-12 record 0
> (type_tag 0xE0, activation dist 99999) +0xC = model 1 = GLOBAL MODEL 1 =
> `klaus`** — the hierarchical bat-wing pterosaur (op-0x5C/op-0x0E assembly of
> the L2 parts: `ptersectklaus` body, `pteranasetheadklaus` head/beak,
> `insectwing1a-5aklaus` nested wing chains, `insectjawklaus`,
> `insecttail1-3klaus`). Type 1 +0xC = model 145 = L3 `flag` (a minimal
> camera anchor; the visible craft is the separate ring `player4` prop).
> The part names (**pteranodon** head, pterosaur body, wings, jaw, tail) match
> the game's bony membranous bat-wing creature, so the type-0→model-1→`klaus`
> binding is high-confidence.
>
> **HIERARCHY/POSE RESOLVED 2026-07-11:** op-0x5C is an oriented mount
> (`[orientation code, axis, packed angle]`), not a second three-vertex mount.
> The engine stores its basis vectors as columns, so the Rust row-major child
> transform must transpose that basis once. The menu entity callback
> `FUN_0040D320` reads its per-entity animation words, and `FUN_0042C660`
> seeds Klaus's folded rest pose (notably vars 3/4/5/11 = F000/F800/E800/F000).
> With those values the nested wings arc down both sides and the head/body
> match the original silhouette. At idle `FUN_0042C710` adds the live,
> staggered 2.02–5.24 s sine pose. Its primary packed phase is restarted by
> Intro Sequence commands 4/1 and bends the pose through fly-out/fly-in; a secondary
> random 0..0xFFFF phase adds the same twitch to channels 6/7 with a divisor
> in 0x20..0x3F. The complete Klaus hierarchy is enclosed in one authored
> sorted group. With callback 1 equal to zero, model 1 uses vertex slot 12's
> local Z `0xA00`, giving an idle outer key `0x1400` after the private view Z
> is added. The flame at `0xC00` therefore composites over the whole group.
> The former claim that geometric head/torso depth should mask the emblem
> was incorrect; internal body and matte groups cannot sort independently
> against it. See [authored painter groups](RENDER_PIPELINE.md#authored-model-painter-groups).
> In the final camera-relative view Klaus needs the effective 180° root
> turn: head toward the viewer, tail away (confirmed against the original
> runtime on 2026-07-11).
>
> **SPRITE-FACE TEXTURING RESOLVED 2026-07-11:** Klaus's sprite-backed faces
> are real textured primitives, not representative flat colours. Its global
> sprites 393–413 literally contain the ribs, teeth, bright wing rails and
> dark membrane panels. The original rasterizer synthesizes triangle/quad UVs
> from the sprite dimensions; Gouraud 0xA3/0xA4 commands additionally carry
> per-corner normal refs. The port now preserves those UVs/corner normals,
> mirrored mapping and transparent index zero and caches the decoded textures.
> **DEPTH-FADE COLOR/TIMING CORRECTION 2026-09-05:** menu init
> `FUN_0042B230` installs cumulative Section-7 entry 55 at world-context
> `+0x7C`; both retail presentation tiers author RGB555 `0x7C00`, decoded as
> `(248,0,0)`. The `0x500`/`0x800` billboard-frame pins change the live near
> plane, and the raster path blends the resulting per-vertex fade bytes toward
> that red terminal color. They do not alter the Section-6 light lookup, face
> RGB, or emissive color. The former port-wide material pulse was correctly
> removed, but the conclusion that no red model pulse remained was wrong: the
> authentic effect is this depth fade, independent of the additive billboard.
> For each flag-`0x02` prop row item, `FUN_0043AA40` rebuilds the pair as
> `[min(world_near-0x900,0x200), 0x200]` before `FUN_0043B410` adds that item's
> root Z. In the ring branch, frame 0 produces a `-0x400/+0x200` red band,
> frame 4 produces `-0x100/+0x200`, and idle `0xC00` collapses it to an equal
> `+0x200` step. Roots never accumulate. `optionsh` alone uses its equal
> `0x1E00/0x1E00` pair plus root; the following flags-`0x03` submenu decoration
> resets from the live world near plane instead of inheriting the panel shift.
> Submenu command 4 selects private state 1, whose `FUN_0042D210` result is 4,
> so the receded branch replaces the ring's low pins with upper clamps. Once
> world near reaches `0xB00`, the same row formula becomes equal `0x200/0x200`
> and the decoration's red band disappears without changing its material.
> `FUN_0042B5D0` evolves the planes from the state query before C090 consumes
> the pending command; Klaus and the current emblem use that evolved pair.
> After `FUN_0042D030` advances, `FUN_0042B870` queries the updated state and
> pins or clamps near; later AA40 rows consume that resulting near.
> Klaus and each opaque menu hierarchy carry their own recovered planes/color
> into `ModelDraw`; later submissions do not inherit a prior prop's state.

- Init `FUN_0042B230`: draw window 52×30 cells (`FUN_004330D0(0x1E,0x1E)`),
  spawns two entities via `FUN_00438080` (type 1 → `DAT_004DB1E0` = the
  `flag` camera anchor; type 0 → `DAT_004DB1DC` = persistent Klaus),
  both heading 0x4000 (90°), flag |= 0x10000000; installs world context 0x4CA838
  (0x2A dwords → BSS 0x4FEC40 via FUN_00433BD0). **The constants
  0x4CA888=−100 / 0x4CA88C=50 / 0x4CA890=−50 are the LIGHT/SHADOW DIRECTION
  vector (ctx +0x50/54/58), not camera position** (installer derives shadow
  slope ratios from them). Context: +0x5C..0x98 = 16 Section-6 shade indices
  (menu: grayscale 0–7), +0xA4 = water enable (menu: 0). Background preload
  worker spawned via FUN_0042D7F0, awaited by FUN_0042D830.
- CAMERA: `FUN_0042B5D0` finds the first type-1 entity and renders the world
  around it via `FUN_0040ED10(handle,0,0,0,dt)` — camera focus = the `flag`
  anchor's position plus its 250-raw forward look-at. The accepted full
  0..10 setting sweep holds the frontend eye/focus at `[0,0,-2048]` /
  `[0,0,249]`; Active Camera (settings+0x30) affects the gameplay chase camera,
  not this frontend call. Camera/viewport fields refresh every frame from the
  Section-0 display block (FUN_0042B870 → globals DAT_004FEED0).
- MENU PROP PROJECTION: `FUN_0043B130`/`FUN_0043B410` replace the render
  context's projection centre per prop row (layout pt0 for the ring, pt5 for
  `optionsh`, pt6 for the submenu prop). The model keeps its authored local Y
  while its depth changes; depth-dependent model translation gives the right
  root pixel but subtly bends articulated geometry and fly trajectories.
- DEPTH FADE (once misread as "camera yaw targets"): camera-struct +0x74/+0x78
  (0x4D055C/60, init 0x1000/0x1400) are near/far, fed to `FUN_00470A10`
  (sets the 0x1000000/(far−near) interpolator); +0x7C is the terminal color,
  initialized to Section-7 entry 55 pure red. `FUN_0042D210` returns an integer
  state query, not the boolean inferred by the decompiler. Both fade callers
  compare EAX **exactly to 1** (`0042B615`, `0042B9AE`). Before C090 consumes
  the pending command, `FUN_0042B5D0` evolves the pair from its incoming delta:
  query 1 uses `far=max(far-dt_us/1000,0x1000)` and
  `near=min(near+dt_us/200,0xC00)`; any other result uses
  `far=min(far+dt_us/200,0x1900)` and `near=min(near+dt_us/200,0x1500)`.
  Constants `004CA8E0/004CA8E4` hold `0x1000/0x1900`. These integer divisions
  precede B5D0's 125,000-µs world-callback clamp and retain no fractional remainder.
  Klaus and the current emblem are queued from the evolved pair. After
  `FUN_0042D030` advances the emblem, `FUN_0042B870` queries the post-C090
  state: result 1 pins near to `0x500` on returned frame 0 or `0x800` on frame 4;
  other results use `near=min(near,0xF00)` or `min(near,0x1200)` respectively.
  Other frames leave near unchanged. The later prop rows consume this suffix
  in the same frontend update. Command 4's state 1 returns 4, so a descending
  submenu decoration naturally loses the red band as near reaches `0xB00`.
  The receded branch is live; it must not be replaced by a truthiness test.
  Ring attacks remain red depth fade, not Section-6 or emissive-material changes.
- BACKGROUND BILLBOARD (`FUN_0042D030`, table 0x4CA938, 9 × {hi_sprite,
  lo_sprite, duration_µs}): sprites 1294–1299 (L5 flame/'V2000' emblem;
  low-resolution alternatives 420–425 in L3), durations ms 0/120/80/80/80/80/180/140/
  180 → **loop ≈ 940 ms**; sequence = flash decaying to base with two
  flickers. On wrap it plays **sound pool id 57 (0x39)** — one low ~230 Hz
  rumble/zap per cycle (658 ms; regenerate it with `v2k-extract sounds`).
  .data initial state (frame=8, accum=2,000,000) makes it fire once
  immediately at menu entry. The shipped software material flag `0x10`
  composites source + destination; the dormant alternate backend callback
  `LAB_0048BA20` independently maps that state to D3D `ONE/ONE`. The emblem is
  therefore **additive**, not an ordinary opaque/alpha quad. Its root-queue
  key `0xC00` is in front of Klaus's complete idle group at `0x1400`; the
  body's own surface Z must not cut holes in the red/yellow emblem. Ring
  props retain their separate queue keys. AA40 aliases the same world queue;
  its row flags do not themselves create an enclosing group.
  C090 consumes raw zoom `Z` (rest `0xFFFF`) and signed presentation height
  `Y`, Section-0 entries 1/2/3 (`focal_y`, framebuffer `W/H`), Section-1
  point 1's `layout_y`, and reference sprite **420**'s metadata `rw/rh`.
  With `q31(a,b) = (i64(a)*i64(b)) >> 31`, it computes
  `S = q31(Z << 15, 0x20500 - 32*Y)` and `P = rw*S`, then
  `x = W/2 - (((P >> 16) - (P >> 31)) >> 1)` and
  `y = layout_y + trunc(-Y / (H < 480 ? 24 : 11))`.
  Above 480 lines it additionally subtracts
  `((trunc(W*rw/640) - rw)*S) >> 17` from x and
  `trunc((trunc(rh*H/480) - rh)/2)` from y. D030 independently sizes the
  **selected frame sprite** using its metadata `fw/fh`:
  `h = ((((focal_y << 6) >> 8)*S) >> 16)` and `w = trunc(fw*h/fh)`.
  These are signed integer operations: division truncates toward zero,
  shifts are arithmetic, 32-bit operations wrap except for q31's explicitly
  widened product, and the submitted corners wrap to signed 16-bit values.
  The reference width determines the left edge; the raster width does not
  recenter it. Rest scale is **132349**, preserving the raw zoom endpoint.
  The port computes this authored integer rectangle before viewport mapping.
  At rest, tiers 0/1/2/3 produce `(x,y,w,h)` of `(109,55,102,129)`,
  `(218,110,206,258)`, `(273,121,258,323)`, and `(349,138,330,413)`.
  AA20's software scaler EF10 samples metadata bounds
  `(dimension << 16)-1`; all selected flame frames have matching decoded
  rectangles (102×128 in tier 0, 203×254 in tiers 1–3), so no source crop is
  required. Matched retail-frame position acceptance remains open,
  separately from the recovered arithmetic and group-depth clipping.
  Port validation matches 62 submitted rectangles to an independent integer
  oracle. All differences in 60 paired tier/phase GL frames stay inside the
  old/new emblem bounds; 89 of 91 standard scene controls are unchanged,
  with only the two frontend menu scenes differing. Cinematic, gameplay and
  Klaus-only controls remain identical in that billboard comparison. Selected
  tier font/layout submission follows the separate explicit UI policy above.
- BOTTOM SPRITES (`FUN_0042B040(rt,0,1,0)` every frame): global sprite ids
  1290 (Frontier logo, x=0), 1292 (copyright line, centered), and 1291
  (Grolier logo, right), each at y = screen_h − sprite_h. Variant-0 dimensions
  are 54×33 / 155×14 / 46×22; variants 1–3 are 110×68 / 309×28 / 115×56.
  `FUN_0042B040` submits those native dimensions in the selected framebuffer;
  any later monitor stretch scales the complete row with the framebuffer.
  The port's High Native footer independently uses the readability scale above
  the original outputs (1.8 at 1080p, 3.6 at 4K), keeping the actual left/centre/right
  bottom-edge anchors. Matching original presets, Low and 4:3/Stretched retain
  their existing submission; 4:3/Stretched physically enlarge the complete
  logical row with or without Classic. This Native growth is a port-owned
  extension, not retail acceptance. `FUN_0042B040`'s text mode
  (strings 34 'Loading'/35 'Retrying', blink (tick/20)&1) is the loading
  screen, not the interactive menu.
- FRAME COMPOSITION (presentation chain order): 42AA40 display-list reset → 42AA60
  latch clear color (S7 entry 32) → 42B870 3D world pass (hangar scene +
  billboard queue + fog pins) → 42AA80 begin surface → 42BA30 UI pass
  (screens via 0043AA40 if state 2: backdrop model → group0 → group1 text →
  group2 props; then bottom sprites) → 42AA90 depth-sort + execute display
  list → 42AAB0 present. Control chain 42B5D0 = camera drift + entity tick +
  menu tick + attract countdown.

## Persistent Klaus animation ("Intro Sequence" behavior)

The type-0/global-model-1 Klaus entity carries named behavior **"Intro
Sequence"** (behavior table 0x4C8AC0 entry 52 → descriptor 0x4CA8E8 →
FUN_0042BEE0 → block {ctor FUN_0042BFD0, update FUN_0042C090, dtor
FUN_0042C050}). State block (0x24 bytes, `DAT_004DB214`): [0] state,
[1] spin phase (+= dt/32 → 2.097 s full spin), [2]/[3] idle-twitch phase/rate
(random 32..63 → 2.1–4.1 s cycle, triggered p=1/32 per callback when phase zero
in states 0/1/3),
[4] whole-model transition progress (`+0x10`, 0..0xFFFF), [5] background zoom,
[6].lo/hi X/Y offset, [7].lo Z. Executable `0042BFD0..0042C043` clears the
whole allocation, then sets twitch divisor 30, zoom 0xFFFF and Z=0xA00.
Progress, phases and Y therefore start at zero; the entity transform receives
`Y offset + 0x40`. `0042BFB9` queues command 1 after behavior installation.
This behavior does **not** own the visible `player4` ring craft or the type-1
`flag` camera anchor.

Commands (`FUN_0042CEA0(n)` → the single pending word `DAT_004DB210`):

| Command | Callback-owned state change |
|---|---|
| 1 | Enter state 0. If previously state 1, restart spin at 1 and play sound 0x57; otherwise set render-admission bit 0x800, Z=0xA00 and zoom=0xFFFF. Preserve transition progress, which state 0 then drains. |
| 2 | Enter state 2 and reset transition progress to zero; advance it in the same callback. |
| 3 | Set render-admission bit 0x800, enter state 0, set progress and zoom to 0xFFFF, and immediately write animation channel 1=0xFFFF; drain private progress later in that callback. |
| 4 | Enter state 1, restart spin at 1 and play sound 0x57. |
| 5 | Enter state 3; zoom decays by `dt_us/8`, saturating at zero (about 524 ms from full). It does not start the opening morph. |

Commands 2/3 are reachable: the former claim that only 1/4/5 existed omitted
retail callers absent from the extracted C. Direct executable calls are
`0045190C` and `00454FD1` for command 2, and `00453D33` for command 3.
Bit 0x800 enables rendering (`FUN_00411400`), so setting it shows Klaus;
clearing it at the end of state 2 removes him from the model submission.

The transition increment is the unsigned shift `step = dt_us >> 4`, with
no retained fractional remainder. State 2 adds it to the signed dword
progress; only a result **greater than** 0xFFFF sets progress=0, state=4,
and clears render-admission bit 0x800. States 0/3 subtract it when progress
is nonzero, clamping a negative signed result to zero. State 1 and state 4
leave progress unchanged. The nominal full interval is 1,048,576 µs, with
callback quantization; this is not a smoothstep or a fixed jaw angle.
`FUN_0042D210` is an integer query: private states 1, 3 and 4 return 4, 5
and 2 respectively; all other states return 1 when progress is zero, otherwise
0. Exact PE `0042D210..0042D23F` preserves these distinct results; the extracted
C's `bool` return loses them. Frontend fade callers compare exactly to 1.
A closing owner waiting for that result therefore follows progress, not a
one-second timer.

`FUN_0042C090` uses its supplied integer microsecond delta directly. The
frontend and the normal world scheduler cap their paths at 125,000 µs;
the callback itself does not clamp. State 1
holds while phase ≤0x6000, then computes
`step = (q31(dt_us << 12, 0x15F4 - Z) * 4) / 3` using C truncation, adds
`step` to Z (cap 0x1400), and adds `step/2` to Y. States 0/3 settle with
`q31(dt_us << 12, 700)` for negative Y,
`q31(dt_us << 12, 2*Y + 400)` for positive Y,
`q31(dt_us << 11, 2500)` below Z=0xA00, and
`q31(dt_us << 12, Z - 0x230)` above it, clamping crossings to rest. The former
τ≈393/524/262-ms descriptions are useful intuition only, not the algorithm.
State 3 drains zoom by integer `dt_us/8`, saturating at zero.
The billboard consumes raw zoom and Y through the integer layout above;
these presentation terms do not change the entity's private Z range
0xA00..0x1400.
At the start of each callback C090 first copies `DAT_004DB218` to animation
channel 2 and runs C610/C710 with the previous primary/twitch phases and
transition progress. Only
after that pose is recorded does it consume `DAT_004DB210`, advance phases,
start a random twitch, and update the entity transform. The port retains a
pre-update render snapshot and a single pending command word to preserve this
one-callback boundary. Command 3 is the explicit exception: its immediate
channel-1 write presents the full 0xFFFF morph although the private progress
has already decreased by callback return. `DAT_004DB218` is 1 in the interactive frontend and 0
after game-start commit until the frontend is initialized again.

States 0/1/3 each call `Random_Next` once per C090 invocation, including
`dt_us == 0` and callbacks whose twitch phase is still nonzero. If that word
passes `(sample & 0x1F) == 0` and the already-advanced phase is zero, C090
sets the phase to 1 and consumes a second word for divisor
`32 + (sample >> 11)`. States 2/4 consume no words. Command 2 is consumed
before this switch, so the synchronous opening prime also consumes none.
`Random_Next`/`FUN_00457930` updates the single process word `DAT_004F7308`
with `state = state*214013 + 2531011` and returns `state >> 16`; Klaus has no
private RNG state. The port's `MenuShell::update` and
`Intro2PresentationStage::advance` consume the caller's shared stream at each
callback through `KlausEffects`, with no 50-Hz catch-up or sampling rebase on a
world-clock reset. Commands 4 and selected-state command 1 resolve their whoosh
synchronously before twitch sampling. Audio submission may follow later; its
alias random words are already consumed even without an audio device.

The accepted `20260717-032945-menu-intro2-level1.jsonl` trace also proves that
the same type-0 entity, component root, task wrapper, state pointer, and C090
callback survive the frontend -> Intro2 -> Level-1 sequence. The port therefore
retains that state across world loads. Transition commands must preserve
primary/twitch phase and Y easing except where the command table explicitly
writes a field; they do not reset the process RNG. Direct `--level 50` entry
is the intentional shell-less path.

The executable distinguishes the following transition owners:

| Owner | Exact ordering and boundary |
|---|---|
| In-game descriptor `004D0918`, init `FUN_004515E0` → `FUN_00451710` | In the normal `world+0x28E == 0` branch, scan the persistent entities; for type 0, queue command 2, store its handle at `world+0x2AC`, then call `FUN_00412D80(Klaus, 125000)` at `00451920`. This is world-load setup, including Intro2; it follows the frontend's command-5 fly-off. |
| `FUN_00453C20` | Restore the type-1 anchor and type-0 Klaus basis, retain Klaus at `world+0x2AC`, then queue command 3 at `00453D33`. No synthetic elapsed callback occurs in this helper. `004D0A30` init `FUN_00453BA0` calls it when `+0x28E == 0`; `FUN_00453E00` and `FUN_004555A0` also call it on their first transition latch and subsequently poll `FUN_0042D210`. |
| Completion/progression descriptor `004D0AA0`, init `FUN_00454EE0` | When `+0x28E == 0` and `+0x292 == 0`, `FUN_00454FA0` locates type 0, queues command 2 at `00454FD1`, then requests `FUN_00412D80(Klaus, 1)` before returning to setup. |
| Separate descriptor `004D0AD8`, init `FUN_004557D0`, callback `FUN_004558A0` | Initialization queues command 4 and model state 1. The callback advances the global clock and menu transition first; with `+0x28B == 0`, `+0x297 == 0`, and counter `+0x2C8 == 0`, it queues command 1, selects model state 0, sets counter 1, then ticks Klaus. Later callbacks add `dt_us/1000` and switch to `004D0AA0` at counter >=1001. `+0x297 != 0` selects `004D0918` instead. This counter is separate from morph progress. |

`FUN_00428B00` only advances the global clock; it cannot queue command 3.
For normal Intro2 completion, `FUN_004503C0` sets `world+0x290` when session
phase 1/2 exceeds tick 0x10CC. The next `FUN_0044FFA0` clears that flag and
sends event 2 (`00450089..004500A9`); `FUN_0044FCD0` selects `004D0A30`.
Its command-3 close is polled by `FUN_00453E00`. For phase 2 and campaign
slot 0x26 (Intro2), readiness sets session phase 5 and selects `004D0918`;
the special slot-0x25 branch preserves its phase. The ensuing world-load
initializer requests command 2. Thus the normal bridge closes before the
first-world load and opens after it, whereas the New Game frontend first
finishes command 5 and only starts the opening at Intro2 load setup.
The `12D80` prime requests above pass through the ordinary entity scheduler:
the actual callback delta also depends on retained cadence/detail state and
the `+0xB6` first-pass byte. If C090 receives 125000 µs with command 2, private
progress becomes 7812 while channel 1 retains the pre-command pose. Do not
replace that setup with an arbitrary visible-frame advance, or infer that
both sides of every load use `4558A0` because their durations look alike.
The in-game model pass `FUN_00453760` submits the ordinary entity list while
`world+0x28A == 0`, so the montage is already advancing behind Klaus. With
the final black-card flag set it submits Klaus alone through `FUN_004113E0`.
The command-2 render-bit clear removes the remaining Klaus submission;
the observed Intro2 tick-44 reveal is a capture landmark, not a fixed gate.
At New Game commit `42CF40` switches session `+0xD0` to game context
`4D04E8` and requests descriptor `4D0918`. The frontend `4CA760` chain
(`42B870` scene, `42D030` rotating billboard, `42BA30` menu/props) ceases;
there is no separate halo-freeze command. Model state remains zero through
both mouth handoffs. Their black cover polygons must use the
[authored painter groups](RENDER_PIPELINE.md#authored-model-painter-groups),
otherwise geometric depth hides parts of Klaus in both directions.

`4515E0` sets the game fade planes to `0x1E00/0x2800`; `453C20` preserves
them while enforcing minima `0xF00/0x1400`. `453570` installs those planes
and selects terminal colour through `2E8D0` (runtime level `+0xB0`, initialized
by `2E570` from Section 13 `+0x54`, the sky palette index). Handoff rendering
therefore stops using the evolving frontend planes and red terminal colour.
`453BA0` additionally sets world `+0x274=1` and selects world callback
descriptor `4D08C8`; its `44FE20` callback consumes that latch and calls
`428AD0` to reset the 50 Hz clock before the first closing actor callback.
The port resets/rebases that clock at the same transition rather than
continuing the completed Intro2 timeline.

The port runs the first-world `Playing` pass throughout command 2's opening,
with Klaus retained as its task and final presentation cover. Before that load,
`4D08C8 -> 4D0710 -> 50C00` advances Klaus, eye/focus and physical particles;
the ordinary actors, camera proxy, static programs and contacts stay suspended.
The [world-frame contract](LOADING_TRANSITIONS.md#live-worlds-beneath-the-klaus-cover)
owns this distinction and the next-visit Intro2 exit latch.

The visible idle motion is an **anim-var pose**, not rigid rotation: per tick
FUN_0042C660 resets vars {1:0, 3:0xF000, 4:0xF800, 5:0xE800, 6:0, 7:0x800,
8:0, 9:0, 10:0xF800, 11:0xF000}, then FUN_0042C710 adds quarter-sine sway
terms at periods 2.62 s (leg-staggered phases −0x4800/−0x7800/−0xA800),
2.91 s, 2.02 s, 5.24 s, amplitudes ±0x400..0x1000, scaled by a cubic
envelope of spin phase (max when idle, →0 during a spin), plus spin-transient
terms at phase·3/2. Random twitch drives vars 7/6 only. C710 writes the low
word of private `+0x10` directly to channel 1 after C660 resets it; retain it
as unsigned 0..0xFFFF at the authored model expressions, despite the C
pointer's `short` storage type. This channel drives Klaus's root, head and jaw
morph expressions. Channel 9 is a separate spin term
`q31(sin_q31(min(primary_phase, 0x7FFF)), 0x1000)`, consumed by the head's
`0x5C` jaw mount; a synthetic addition to channel 9 does not reproduce the
channel-1 morph. The head also composes the authored `0x1C` rotation using its
register expression derived from channel 1. Channel 6's idle contribution is
`q31(sin_q31(tick*250 + 0x4000), (0xFFFF-progress)/50)`, with signed division,
so a full transition suppresses that sway. Var 2 =
`DAT_004DB218` (1 in menu, 0 once game start commits).
The port now uses the executable's 4,096-entry quarter-sine table exactly
(including its low-two-bit mask), repeated-word Q31 values, signed narrowing,
and phase-zero one-LSB corrections. Matching a particular captured twitch
onset additionally requires matching the other consumers of the shared RNG.

## Menu sounds — exact ground truth

`FUN_0042A860(id)` = `FUN_004958C0(DAT_004FE64C[id], &0x4CA6C8)`; param block
0x4CA6C8 = {vol 0x10000, freq 0x10000, pan 0, 1} — **full volume, native
22050 Hz, centre pan, one-shot**. No settings gating in the path; muting is
purely master volume. Type-5 aliases resolve recursively with freq/vol
multipliers and randomized frequency variance. Each alias hop first computes
`f = (f * multiplier) >> 16`, then `a = (f * variance) >> 16`, retaining the
low 32 bits of each result. It consumes **one random word even when variance
is zero**, computes signed `d = (i32)(rand16 * a wrapping32) >> 16`, then
adds `d` if its low bit is one or subtracts `d` if even. This sign comes from
the perturbation, not the random word. The next hop consumes that modulated
frequency. `004956B0` separately scales volume, including the executable's
low-byte mask when incoming volume is at least `0x10000`; unity skips that
helper. Final `004957C0` Hz conversion wraps `f * 22050` in 32 bits before its
unsigned shift by 16, then `004AD0A0` clamps Hz to 100..100000.

| Id | Trigger | Resolved sample | Character |
|----|---------|--------------------------------------|-----------|
| 0 | cursor move (only when it moves), spinner at limit | L2 blob 0, 55.4 ms | instant-attack high tick (~2.0→1.8 kHz) |
| 1 | spinner value change | L2 blob 1, 167.8 ms | swelling blip, peak at ~120 ms |
| 2 | save/load & network feedback (slot select, errors) — **not** generic left/right | L2 blob 2, 182.9 ms | descending whoop 1.67→1.04 kHz |
| 3 | select / open submenu / back / confirm | L2 alias 3 → **blob 0, identical to id 0** (variance 0) | same 55 ms tick |
| 57 (0x39) | background flame-cycle wrap, every 940 ms | L3 blob (via alias 52), 658 ms | low ~230 Hz rumble/zap |
| 0x57 (87) | prop whoosh (cmd 4 select-spin; cmd 1 while spinning) | L3 alias 80 → blob 16 (global 87 → 23), multiplier `0xB333`, variance `0x3333`; **12348..18521 Hz** → about 297..446 ms | broadband rising swoosh |

Unused L2 extras: id 4 = alias dup of blob 0, id 5 = alias of blob 1,
id 6 = alias→alias→blob 0 with ±25% pitch variance.

Complete call-site catalog (39 sites, byte-scan verified): sound 0 also fires
from the attract-mode key slots; sound 3 from every pause-menu select
(FUN_00456380), Continue, Replay, Quit-confirm, cheat applies, game start
(FUN_0042BBF0 — which also fires prop cmd 5, so **ring select = sound 3 +
whoosh 0x57 together**); sound 0x57 also plays per enabled vehicle-upgrade
bit at game start (FUN_00450FC0 ×6). In-game pause sites gated by
`DAT_004FE60C==0` (not multiplayer).

**Volume math** (FUN_00495830): master `DAT_004FBE6C` (init 0xFFFF); Sound
setting s (0–15) applied as `FUN_00495910(s·0x1111)` (live, on every spinner
change). Per buffer: `linear = clamp(vol_16.16 · (master>>8)/255, 0,
0x10000)`; `SetVolume(10000·ln(linear/65536)/ln(65536))` hundredths-of-dB —
each halving of linear = −6.25 dB; s=15 → 0 dB, s=8 → −5.66 dB, s=1 →
−24.4 dB, s=0 → −100 dB (plays silently, not stopped). **Ambient setting is
a pure on/off gate for CD music** (only `!= 0` is ever tested); its 1–15
magnitude is never used.

Playback engine: FUN_004958C0 → FUN_00495480 (alias resolve, per-blob lazy
DirectSoundBuffer + DuplicateSoundBuffer, max 24 active), FUN_004957C0
SetFrequency/SetPan/volume, Play non-looping; finished buffers GC'd on next
dispatch.

The port resolves native requests through `sound::resolve_sound_playback`
using the canonical global `SoundPool` and caller-owned random stream, then
submits `ResolvedSoundPlayback` without further RNG. Resolution precedes
missing-device or voice-allocation failures, as the native alias recursion
precedes PCM/buffer creation. Older world sound wrappers still use their
separate pitch policy pending native request ownership at those call sites.

Positional requests have a distinct admission phase: `0044F450` → `0044C790`
appends a logical one-shot without resolving aliases. `0044C970` later tests
wrapped signed-word deltas against `0x2000`, applies all three Q31 view rows,
then computes integer distance, volume and pan. Distance through `0x100` keeps
full volume; larger audible distances use `(0x2000-distance)/0x2000`.
Inaudible requests consume no alias or warble words and one-shots are discarded
by `0044C940`. The Intro2 presentation table places this pass (`00453AA0`)
after the emblem and before `00452CB0`'s commands/captions. Alias resolution at
queue time would therefore consume words in the wrong phase.

`sound::resolve_positional_sound` owns that integer admission for one-shots;
`EntityPositionalAudio` retains constructor loops and their audible-only
sound-11 warble in live-list order. Each warble rollover consumes two shared
words before initial alias resolution, including a first audible zero-dt
callback. Existing loops retune through `004957C0` without revisiting aliases;
culling stops their voice but preserves warble state. Logical admission also
runs without SDL, so device availability cannot change the process RNG.

## Music — the frontend menu is SILENT

**No CD music plays in the frontend menu**, at cold boot or after quitting a
game. The only creator of the music descriptor (session+0x298) is
Music_SelectTrack 0x456AF0, called once from game-session level load
(FUN_00451710 ← in-game mode init FUN_004515E0). Return-to-frontend
(FUN_0044FEB0) stops the CD, rewinds the descriptor and frees it.

In-game track selection: world = Section13[level]+0x48 → **track = world+1**
(worlds 1–6 → tracks 2–7). Overrides by session mode byte +0x296: mode==4 &&
world!=6 && rand-even → track 8; mode ≤ 2 → track 9. Loop: MCI play
from position to track end; a 1.5 s poll (FUN_00456A00, countdown at
session+0x29C) restarts the track when `status cdaudio mode` ≠ "playing".
Ambient==0 pauses (position saved), results screens pause/resume.
Overlay-51 occupancy (`004D0AA0`) uses that same cursor so Ambient cannot
restart the track while the card is up; `FUN_004558A0`'s restore of
`004D0918` returns the zero/nonzero gate.

**CD disc check (copy protection, NOT an easter egg):** FUN_0042BBF0 (game
start), network-host and save-slot paths verify the disc fingerprint via
FUN_00496BF0/FUN_00496DE0: ≥9 CD entries, total length ≥ 4500 s, and track
lengths [471,438,463,357,357,303,211,182] s (= ripped tracks 02–09 ±1 s).
The old "hidden multi-key cheat check" reading of FUN_00496DE0 was wrong —
it reads CD track durations, no scancodes involved.

## Key input — edge-triggered, NO auto-repeat

Main key table at **0x4C05A0** (not 0x4C05F0 — the old doc missed the first
4 rows), 16 × 20-byte rows, zero-terminated at 0x4C06E0. Row =
`{slot_ptr, required_scancodes u32 (up to 4 packed DIK bytes, ALL must be
down), 0, excluded_scancodes u32 (up to 4 packed, NONE may be down), 0}`:

| Keys | Slot | Handler |
|------|------|---------|
| Esc (excl. both Shifts) | 0x4CA6E8 | FUN_0042BAD0 — sound 3 + pop one screen + prop cmd 1 if depth<3. Original runtime ground truth (2026-07-11): root-ring Esc is inert; quit requires the Exit prop. The earlier static interpretation that its action-8 write commits at the root was wrong. |
| LShift+Esc / RShift+Esc | 0x4CA6F0 | FUN_0042BAA0 — instant quit to desktop |
| Return (excl. Alt), Space | 0x4CA6E0 | FUN_0042BD90 — select |
| Up 0xC8 / Down 0xD0 / Left 0xCB / Right 0xCD | 0x4CA6F8/700/708/710 | FUN_0042BB30/BB60/BB90/BBC0 |
| Alt+Enter (0x1C38, both required) | 0x4CA718 | FUN_0042D5D0 fullscreen |
| 1–6 (0x02–0x07) | 0x4CA720–748 | FUN_0042D240..D290 (DAT_004DB228=1..6) |

Registration: menu init → `FUN_00428F70(ctx, &0x4C0818, mode)`; descriptor
0x4C0818: +0x00 key table, +0x18/+0x1C two **joystick tables**
(0x4C06F8/0x4C0788: codes 0x0D→exit, 0x0F→select, 5/7/8/6→up/down/left/right
— button/POV ids for two device mappings). Matcher `FUN_00472320`: takes a
256-bit currently-down bitmap; per row computes `match = all required ∧ none
excluded`; rows sharing a slot accumulate a per-slot match counter; the
handler fires only on counter **transitions** (0↔1) with state 1=press /
0=release. **There is no repeat timer anywhere: one press = one action.**
All nav/select handlers act on the press edge only (`param_2 != 0`) and
reset the attract timers on both edges. Results-screen table 0x4C439C
(7 rows: Return, arrows, P, Alt+Enter → slots 0x4D06A0+).

## Attract / demo

`DAT_004DB21C` decrements by dt_µs/1000 (ms); first timeout 60,000 ms, then
reload value `DAT_004DB204` becomes 10,000 ms. Any key press OR release
restores both to 60,000. On expiry (FUN_0042B5D0, only when
`DAT_004FE60C==0 && DAT_004F741C==0`): pop one screen (FUN_0043A9A0); if now
at the ring (depth<2): exit action `DAT_004DB224 = 4`, overwritten to 1 with
p=1/8, and prop cmd 5 (fly-off, bg zooms out 524 ms). Exit actions are
consumed by the MAIN-RING screen record 0x4C0CF8's callbacks (+0x0C =
FUN_0042CED0 handles 6/7 = 0xC01/0xC02 throws; +0x10 = FUN_0042CFA0 handles
1/2/4/5 → FUN_0042CF40 → FUN_0044F650/FUN_0044F8B0 game start with mode code
= action; 8 → quit). Direct-slot override DAT_004DB228 (keys 1–6) applies
when DAT_004F73B8 != 0.

### Late-demo idle-attract selector (STATIC DIFFERENTIAL 2026-08-02)

Version Tracking pairs retail `FUN_0044F650` with demo `FUN_0044EE50`:
the loader's control flow and its table-record contract match, but the
`sub_mode == 4` selector has a deliberately smaller table in the demo. This
is the 7-in-8 idle-attract auto-play path above, not manual New Game and not
evidence for the later demo-ending route.

Both builds consume exactly one `Random_Next()` value, derive an unsigned index
from its low 16 bits, copy the selected record's `+0x0A` dword and `+0x0E` word
to the loader receiver at `+0x38` and `+0x3C`, and pass its byte `+0x00` to the
common state initializer (`FUN_0042E3B0` retail / `FUN_0042DE00` demo).
`FUN_0042E3B0` stores that byte as the current logical control/campaign slot;
the corresponding gameplay-overlay id is slot + 12. The copied `+0x0A` dword,
`+0x0E` word, and other record fields remain unnamed.

| Build | Loader / table | Index calculation | Selected logical slots (`+0x00`) |
|---|---|---|---|
| Retail | `0x0044F650` / `0x004D0CC8`, 12 × 16-byte records | `((rng & 0xFFFF) * 12) >> 16` | `7, 27, 2, 3, 5, 6, 11, 12, 31, 11, 24, 26` |
| German demo | `0x0044EE50` / `0x004CAC98`, 2 × 16-byte records | `((rng & 0xFFFF) << 1) >> 16` | `2, 3` |

The shared caller (`FUN_0042CF40` retail / `FUN_0042C990` demo) passes the
front-end sub-mode directly to this loader. Both demo records also carry an
unclassified word `0x005A` at `+0x08` where the corresponding retail records
carry `0x0058`; this comparison does not name that tuning field. Do not copy
the demo's two-entry attract restriction into manual New Game or treat it as
proof of the inter-world demo-end branch. The independent completion-mode
trace below proves that the promotional page is a renderer substitution, not
an attract-table policy.

## Screen tree highlights (full tree in ../port/crates/v2k-game/data/menu_tree.json)

- **0x4C0CF8 = MAIN MENU RING** (flags 0xC4, 7 prop items): `player4`(start),
  `slopt`(save/load → 0x4C1740), `screenop`(Display → 0x4C14D0),
  `sfxopt`(Sounds → 0x4C12F0), `psjoypad`(Controls → 0x4C13D0),
  `optexit`(quit), `multipc`(Network → 0x4C11D0). Row group [3,0,4,7,7]
  (centered+ring, pos pt0, step pt4, window 7, count 7).
- 0x4C14D0 Display: Rendering/Resolution/Bilinear/Display/Active
  Camera/Targetter (groups [8,3,4,5,6]+[3,6,4,1,1] — typewriter list +
  centered prop). 0x4C12F0 Sounds: Sound, Ambient. 0x4C13D0 Controls:
  Absolute Mode, Sensitivity, Joystick, Self Righting (+hidden Vibration,
  flags=0), psjoypad prop.
- 0x4D1090 / 0x4D1190 = pause menus (Continue, Options, Mission Briefing,
  Player Status → 0x4C4FB0, Replay, Cheats → 0x4D0FE8, Quit → 0x4C4DD0).
- 0x4D0FE8 Cheats: Skip to %s World, Weapons → 0x4C4B48 (26 weapons,
  codes 0x3E7xx), Repairs, Cargo, Complete this/All Worlds, Disable All.
- Settings bindings: +0x14 Rendering (FUN_0043CC70), +0x18 Bilinear,
  +0x1C Display window/fullscreen, +0x10 Resolution (max = display-mode
  count FUN_00493EB0). Packed ids: lo16 = label string id, value strings
  follow (Sound 46 → Off/On 47/48; Self Righting 42 → Off/Level/Angled
  43-45; Joystick 52 → Absolute/Relative 53/54).

### Port Network availability

The port intentionally replaces `0x4C11D0`'s runtime session placeholders with
two frontend rows: disabled literal text `Not available yet` and selectable
`Back`. Back starts selected; direction inputs skip the notice, and selecting
Back or pressing Escape uses the normal screen pop and Klaus return command.
The native panel/typewriter policy and Network ring route remain in use.
This is a release policy while multiplayer is unimplemented, not a recovered
retail lobby. The extracted screen JSON remains unchanged for future networking
work.

### Complete frontend audit (runtime-confirmed 2026-07-17)

The independent 60-Hz captures `20260717-032247-complete-menu-audit` and
`20260717-032451-complete-menu-audit` bound the interactive settings paths as
well as the Load process-loss boundary:

- Across eight complete root↔submenu transitions (Display twice, Sounds once,
  and Load twice, including their returns where applicable), the first sampled
  nonzero fly clock reached the stack commit after 616.246–633.260 ms. The
  commit reset the clock to `0x7000`; the first sampled zero followed after
  633.051–633.669 ms. Those 60-Hz bounds include up to one sample of endpoint
  latency and agree with the statically recovered **626.4 ms per leg**.
- Bilinear changed `1→0` at 24,783.355 ms and `0→1` at 41,783.815 ms in
  the first capture, with Display still topmost and no fly/stack transition.
  It is a live setting mutation, not a screen transition. The static rasterizer
  audit proves the value has no render-path consumer; retail remains point
  sampled regardless of this menu value.
- The second capture traversed Sound through every integer `15↓0↑15`.
  The sampled DirectSound master was exactly `sound * 0x1111` for values 1–15
  (`0xFFFF` at 15) and zero at Off.
- Ambient independently traversed every integer `15↓0↑15` while the
  DirectSound master remained `0xFFFF`. Its magnitude does not scale effects;
  the static call-site evidence remains the basis for its zero/nonzero CD-music
  gate, and the frontend has no CD playback to change.
- Both Load attempts ended at the expected retail process-loss boundary. The
  first unreadable sample followed the final readable slot-screen sample by
  one 60-Hz interval (16.5–17.1 ms), and the inspector stopped about 0.5 s
  later by design. The later focused WinDbg capture
  `20260802-031147-load-shutdown-boundary.txt` proves that this is an orderly
  LaserLock authentication failure, not a fault or corrupt save: all mixed-mode
  MCI topology checks passed, the authentication pair failed, and retail
  cleared its run flag before successfully validating every `Slot00` record.
  This process loss is evidence about retail, **not behavior for the port to
  emulate**.

### Frontend Load sequence and slot count (runtime-confirmed 2026-07-17)

Selecting `slopt` from the root does not jump directly to the slot list. The
normal root-to-submenu fly first targets **0x4C1740**: after the 0.6264-second
fly-out commits, its `slopt`/`optionsh` access screen remains visible for the
complete 0.6264-second fly-in. Its item draw callback **FUN_0043B9A0** waits
for `_DAT_004CB4E0 == 0`, probes the card state, then replaces the top screen
with **0x4C1D28** without another fly.

0x4C1D28's row group contains **15 rendered rows, but 14 playable save
slots**. Rows 0..13 have the selection callback and map through the probe table
to `Slot00`..`Slot13`. Row 14 takes `FUN_0043BB20`'s special branch, renders
resource string 80 (**"Used for game settings"**), and has no selection
callback in the extracted menu tree. It is an informational settings row, not
a `Slot14` file. This corrects both the old three-slot assumption and the later
interpretation of all 15 rows as game saves.

For playable rows, `FUN_00448E70` classifies missing/unavailable (states 1/2),
corrupt (3), and header/state probe-valid (4) files. `FUN_0043BB20` renders
states 0..2 with a blank label, state 3 with the **"Corrupt"** resource string,
and state 4 with the resolved level label plus the saved 32-byte name. Every one
of the fourteen playable item records retains its selection callback and normal
enabled flag; missing and corrupt rows are selectable feedback targets, not
disabled menu items. Only the settings row lacks a callback.

`FUN_0043BFC0` re-probes on selection. States 1/2 play menu feedback sound 2
and remain on the list; state 3 plays sound 2 and enters retail feedback screen
`0x4C1890`; only state 4 calls `FUN_00448BD0` and continues the load. The port
preserves the selectable blank/corrupt rows and blocks every non-loadable row
with sound 2. It also exposes structurally readable but unsupported legacy
saves as **"Unsupported: <name>"** and blocks them the same way. Missing rows
stay on the list after sound 2. Corrupt, unsupported, and probe-valid but
tail-invalid rows now show the decoded **Access Failed** screen at `0x4C1890`;
frontend Select acknowledges it silently while Back plays sound 3, and both
restore the existing Load rows. A successful frontend Load dispatch is likewise
silent. The probe does not read the tail group; the full loader does. The port therefore requires
at least one CRC-valid header, state, and tail copy before a native row can
dispatch, and a tail-invalid file keeps the ordinary state-4 label while
remaining occupied and non-loadable.

Cancelling the frontend Load list is an ordinary stack return, not a synthetic
overlay dismissal. The port retains the access screen as the list's depth-2
owner; Back pops to depth 1, runs the normal 0.6264-second fly-out/fly-in, and
issues Intro Sequence command 1 so the ring and its front-end prop/background
return to their baseline state. Pause Save owns its list directly, so Back only
dismisses that overlay without popping the pause root. `FUN_0043BF30` routes a
probe-valid occupied Save row through decoded screen `0x4C1598` (**Overwrite
game?**, Yes/No); corrupt rows and empty rows advance directly to `0x4C17B0`
(**Saving game**). No or Back exits the Save flow. Yes authorizes the selected
write, which resolves to `0x4C1820` (**Game Saved**) or `0x4C1890` (**Access Failed**).
The campaign-map Save route writes its complete native checkpoint through
`SaveManager::write_checkpoint` to `SlotNN` directly beside the running port
executable. This portable placement adapter is independent of the data root,
working directory, and retail Save Path. Data-root, parent, `data-root/saves`,
and registry-selected originals remain read-only imports even when a row is
explicitly saved again. The [save storage policy](SAVE_AND_SETTINGS.md#native-save-scope-and-restoration-boundary-confirmed)
owns placement and retirement of same-directory portable shadows after success.
The portable compatibility API remains separate.
Both paths reject an occupied row without an explicit overwrite disposition.
Pause Select supplies sound 3 for confirmation and result actions; either
result acknowledgement returns to the pause root. The port's disk write
is synchronous, so the progress state is represented exactly but is not yet
guaranteed to remain visible for one rendered frame.

The exact eight-record framing, CRC fallback, logical-to-global `+12` mapping,
portable-JSON fallback, and payload contract live only in the authoritative
[native save format](FORMAT_DOCUMENTATION.md#save-file-format). Legacy
`data-root/saves/SlotNN` checkpoints remain loadable when no executable-directory
slot exists. Within those legacy imports, native checkpoints precede portable
JSON and direct/parent/registry native saves, including a corrupt checkpoint
that must not silently fall back to an older save. Executable-directory slots
have first precedence, and imported files are never migrated or deleted. Native player/controller, inventory, cargo
and campaign restoration and its remaining boundaries are documented in
[SAVE_AND_SETTINGS.md](SAVE_AND_SETTINGS.md#native-save-scope-and-restoration-boundary-confirmed).

The two independent 60-Hz audits `20260717-032247-complete-menu-audit` and
`20260717-032451-complete-menu-audit` both captured the access-screen boundary,
the zero-clock replacement, and the expected retail process loss after a Load
selection.

### Copy protection

The process loss after a retail Load selection comes from the game's copy
protection. The Rust port has no media or copy-protection checks.

The NoCD05 Alpine failure is a [short overlay read followed by an
error-dialog callback crash](RETAIL_DATA_INTEGRITY.md#nocd05-alpine-load-failure),
not a protection failure. Its missing-file text is stale CRT errno from EOF;
the requested file opens successfully.

## Decompilation artifacts & tools

| File | Contents | Tool |
|------|----------|------|
| `menu_system.c` | 189 fns: settings/action cluster | `tools/ghidra_decompile_menu.py` |
| `menu_mode_handlers.c` | 204 fns: mode-descriptor handlers | `tools/ghidra_decompile_mode_handlers.py` |
| `menu_nav.c` | 88 fns: navigation/item cluster | `tools/ghidra_decompile_menu_nav.py` |
| `menu_item_callbacks.c` | 119 fns: draw/select callbacks | `tools/ghidra_decompile_menu_callbacks.py` |
| `../port/crates/v2k-game/data/menu_tree.json` | 34 screens, labeled items | `tools/extract_menu_tree.py` |
| `reference/menu_mode_descriptors.json` | mode descriptors | `tools/menu_mode_descriptors.py` |
| `reference/menu_render_pass.json` | screen records, layout points, fonts, ctx template, bg table, camera clamps | `tools/menu_re_render_dump.py` (2026-07-05) |
| `v2k-extract sounds` output | canonical 110-slot pool and 52 regenerable WAVs; menu call-site analysis is recorded above | shared Rust runtime/export pipeline |
| (scene .data dumps) | tick divisor, fog clamps, bg table, world contexts, behavior entries | `tools/menu_re_scene_dump.py` (2026-07-05) |
| (key tables) | 0x4C05A0 main, 0x4C439C results, joystick tables | `tools/menu_re_keytable.py` (2026-07-05) |

## Remaining open questions (none port-blocking)

1. Game-start mode code semantics 4 vs 1 vs 2 (attract-timeout start vs 1/8
   variant vs ring-select) — which runs an AI demo. Runtime test needed.
2. Process-global RNG ownership for remaining world sound wrappers. Klaus's
   callback cadence and immediate whoosh alias ordering are implemented; other
   sounds must also resolve at their native call boundary.
3. How >640-wide modes scale the layout points (explicit compensation exists
   only for the billboard quad; needs a runtime check).
4. Exact name-entry editing behavior in `FUN_004494A0`; ordinary saved-name
   rendering, the `"%s\Slot%02d"` path, playable-row mapping, Load cancellation,
   and the occupied-slot confirmation/result screens are resolved above.
