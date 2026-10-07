# V2000 Menu: Port vs Original — ranked feel gaps

Produced by the 2026-07-05 menu deep-RE pass. Ground truth for every item is
in `MENU_SYSTEM.md` (with EXE/decompiled citations); this file is the
actionable diff for `port/crates/v2k-game`. Ranked by how much each gap
contributes to "doesn't feel like the original".

> **Implementation status (2026-07-05):** gaps 1–17 addressed in the port
> (commit pending). Sound: full global pool (110 slots), per-voice playback
> rate + per-trigger pitch variance, silent frontend, in-game track =
> world+1, whoosh 0x57 + rumble 0x39. Feel: fly transitions (no fades),
> scroll τ0.21 s + infinite-carousel wrap, 3 s scene intro, typewriter reveal.
> Ring: exact `(800·sinθ, −320·cosθ, 3500−1200·cosθ)` math, shared prop
> spin 3.436 s/rev, uniform scale, no title/hint. Text: Section-4 sprite
> fonts (green/yellow swap), list layout via layout points, spinner glyph
> bars with discrete cell breaks. Background: 940 ms flame billboard.
> Verified on-screen (OpenGL).
> Remaining/deferred are marked **[deferred]** below. ✅ = done.
>
> **In-game pause update (2026-07-17):** normal single-player Escape now opens
> the authored full pause screen `0x4D1190` with visibility mask
> `VIS_INGAME | VIS_INGAME_SP`. Pause owns only its menu shell: the current
> level/cache, entities, world effects, renderer resources, CD track, and sound
> pool stay resident while world input and animation time are frozen. Continue
> plays sound 3 and keeps the pause active through its authored 0x7000
> (~626.4 ms) fly-out, resuming only at the switch-away boundary; confirmed
> Quit alone enters the shared frontend teardown. A
> captured last-presented world frame remains beneath `optionsh` and the pause
> rows; Klaus, the flame billboard, and frontend branding are excluded.
> Display changes (Classic Framebuffer, scaling, resolution, fullscreen and
> window resize) rebuild that underlay immediately from isolated pause-entry
> world owners with zero elapsed time. Retained particle, shield and HUD/radar/text commands are
> reprojected without re-running their RNG, sound or cadence producers. Native
> scaling recomputes the world lens; Classic off restores full colour precision.
> The disposable redraw never commits actor/effect writes to the live world.
> Authored resource-tier selection remains restart-applied. Root
> Escape is inert at the pause root (the recovered depth-1 pop cannot remove
> its base screen). P remains results-screen-only in table `0x4C439C`; normal
> gameplay table `0x4C2B38` binds Escape alone. The NoCD02 investigation and
> retail executable resolve the simpler `PAUSE_SIMPLE` / `0x4D1090` root:
> static code establishes progress-map S through descriptor `0x4D0AD8`,
> opening Save Game / Continue / Options; the recording did not press S. The
> Continue callback `FUN_00456170` sets `+0x297` for `FUN_004558A0`. The port
> retains the destination route through saving and Continue. Ordinary map
> Space remains `0x4D0AA0` / `FUN_00455CC0` / `+0x294`; see the
> [map/save contract](LOADING_TRANSITIONS.md#campaign-exit-routes-runtime-validated-2026-07-22).
> Other unproven session predicates remain evidence-gated.

## Remaining menu priorities (ordered, 2026-07-05)

Interactive feel is complete (user-confirmed), including the right-column
typewriter edge (High 8b). What's left is visual richness, ordered
by leverage:

1. ✅ **Material binding → textured props** *(DONE 2026-07-11)* —
   the face `mat` operand colours each face: opcode bit 0x80 clear → Section-7
   palette index; set → global sprite id → the sprite is texture-mapped over
   the primitive with implicit triangle/quad UVs. The port preserves the
   triangle's authored upper/right domain and the quad rasterizers' ordinary
   full-rectangle corner order. Mirror-family opcodes repeat that same order
   with XOR-reflected slots. The frontend's explicit local-X model basis—not a
   global UV reversal—keeps Save (`V 2000`) and door-left/arrow-right Exit
   readable without detaching Klaus's asymmetric keyed wing pixels from their
   joints. Factory-emblem and peasant-hut regressions pin the parser policy;
   Klaus and the directional ring props separately pin frontend presentation.
   It also preserves
   transparent index zero and Gouraud `s0..s3` corner normals; textures are
   decoded/uploaded once. Klaus now shows
   its actual ribs, teeth, wing bones and dark membranes rather than averaged
   tan faces. `v2k-formats::models` face materials/UVs/corner normals +
   `v2k-game::model_color::ModelMaterialCache` + renderer material draw path.
   **2026-07-17 correction:** the former "real per-sprite flat-colour field"
   refinement was stale wording from the superseded average-colour pass. All
   15,064 Section-3 records in the complete 48-OVL sprite corpus contain zero
   at on-disk +0x14/+0x18; retail uses those dwords as runtime-created resource
   handles. Sprite-backed faces already use their authored texture, so no
   flat-colour refinement remains. The table at 0x4CC138 is the exact
   96-record particle descriptor array, not an unwired model-material table;
   its proven frame/scalar fields are now used by `world_fx`, while
   unidentified callback/data words remain raw. The formerly suspected
   0x66/0x86/0xA6/0xC6/0xE6
   "material state" family was disproven: those handlers manipulate deferred
   depth keys/lists, and 0x180 cannot be an 8-bit V2000 stream opcode.
2. **Full hangar 3D scene behind the ring** (gap 2) — *scene mechanics RE'd +
   backdrop model id RESOLVED 2026-07-06; RENDER not yet fully faithful.* CONFIRMED:
   both menu entities spawn co-located at world origin (90° yaw, no scale —
   layout is all in the models); chase camera on the type-1 `flag` anchor → fixed
   pose at origin; NO terrain/water (black bg); light dir −100,50,−50. The
   backdrop = **global model 1 `klaus`** (type-0 Section-12 record +0xC;
   pteranodon/pterosaur parts — see FORMAT_DOCUMENTATION §12 for the
   type→model chain). ✅ Hierarchy/pose fixed 2026-07-11: op-0x5C basis
   transpose, mirrored wing, and `FUN_0042C660` folded rest angles restore
   the skeletal silhouette. Sprite-face texturing
   fixed 2026-07-11: the extracted 393–413 artwork supplies the original dark
   membranes and bone details. ✅ Scale/layout correction 2026-07-11:
   The accepted `20260724-202233-menu-poses` sweep proves frontend Klaus uses
   its private `FUN_0042C090` Z directly: 0xA00 at rest and 0x1400 in the
   submenu-away pose. Active Camera values 0..10 do not change the frontend
   eye, focus, root, or private pose; that setting belongs to gameplay chase
   camera policy. Menu prop groups use the original off-centre projection
   rather than depth-dependent model translation, fixing the selected
   monitor's subtly bent fly path.
   Steady-idle `FUN_0042C710` wing sway, its click-driven primary pose and its
   random channel-6/7 twitch are live; billboard motion and bottom-branding
   edge alignment use the recovered layout. Billboard placement preserves
   C090/D030's raw integer sizing and centering. Klaus's mouth handoff also
   depends on callback 1's authored morph and the model's additional `0x1C`
   mount rotations; callback 9 alone supplies only the spin contribution.
   The exact jaw binding and opening range are documented under
   [authored mount rotations](RENDER_PIPELINE.md#authored-mount-rotations).
   The missing body sections during both handoffs came from skipped nested
   painter groups: geometric depth let the black cover polygons obscure
   Klaus. His linked draw now preserves authored group/instance order and
   uses the game scene's fade after New Game commit; closing resets the
   retail clock. Native widescreen extends only the outer matte supports.
   See [painter groups](RENDER_PIPELINE.md#authored-model-painter-groups) and
   [widescreen matte](RENDER_PIPELINE.md#klaus-widescreen-matte). Matched retail
   frame acceptance remains separate from these code-backed corrections.
   ✅ Depth-fade color/timing
   correction 2026-09-05: menu context `+0x7C` is Section-7 entry 55, RGB555
   `0x7C00` pure red. When the integer `FUN_0042D210` query is exactly 1,
   frame 0/4 pin the live near plane to `0x500`/`0x800`;
   Klaus uses the evolved pre-advance near/far pair, while every later
   flag-`0x02` prop callback independently rebuilds
   `[near,far]=[min(world_near-0x900,0x200),0x200]+root_z`. Ring roots do not
   accumulate. `optionsh` alone uses its equal `0x1E00/0x1E00` pair plus root;
   the flags-`0x03` submenu decoration resets from the post-suffix world near
   instead of inheriting the panel. Submenu command 4 makes the query return 4,
   selecting the receded near/far evolution and frame-0/4 upper clamps
   (`0xF00`/`0x1200`). The prop band becomes equal at world near `0xB00`, so
   the descending item settles without red pulsation while its rotation and
   the additive emblem animation continue. B5D0 evolves before C090 consumes
   the command; B870 chooses its suffix from the updated state. Submission-local
   state keeps this fade off the additive flame and later draws. The synthetic
   port-wide material/
   emissive pulse remains removed; the authentic red pulse is the restored
   depth fade and is independent of the billboard's own additive glow. The
   pixel-space `0x22` ribbons (notably `multipc`'s three network links) retain
   the same model-local fade as endpoint bytes instead of bypassing it.
   ✅ Blend/phase and outer-group depth: the shipped software material
   flag `0x10` composites source + destination; alternate callback
   `LAB_0048BA20` corroborates it with D3D ONE/ONE. Model 1 encloses all of
   Klaus in one sorted group, keyed at `0x1400` at idle. The emblem's `0xC00`
   key puts it in front of the whole creature; the former head/torso masking
   claim confused surface depth with group order. The frontend now preserves
   internal painter order and writes the single outer depth through
   `OpaqueSceneGroup`. This fixes port GL clipping by the body, inner wings
   and tail; frontend state 1 has no black matte faces. Ring depth retains
   its existing approximation. See [painter groups](RENDER_PIPELINE.md#authored-model-painter-groups).
   Klaus and the current emblem are queued
   before billboard advancement; the returned frame is then pinned before the
   prop-row traversal, aligning both red depth-fade attacks with the paired-beat
   sound sample. Billboard layout uses C090's reference-sprite-420 centering,
   its resolution-dependent Y divisor and framebuffer corrections, and
   D030's separate integer sizing of the selected frame before viewport
   mapping. Matched retail-frame position acceptance remains open; see
   [the billboard contract](MENU_SYSTEM.md#menu-3d-scene-camera-background).
   ✅ Section-6 lighting correction 2026-07-27: the indexed shade rows,
   untextured RGB values, contextual signed shift, and affine per-vertex
   shade/fog interpolation were already live. The remaining approximation was
   the light vector: every submission incorrectly inherited the local/world
   `(73,73,-73)` direction. Model submissions now carry their signed raw
   vector explicitly, so persistent frontend Klaus uses its recovered
   `(-100,50,-50)` context while ring props retain `(73,73,-73)`. Since
   2026-10-07 world models use the level's reduced Section-10 direction, and
   every vector is applied in VIEW space; see
   [the model light table](RENDER_PIPELINE.md#model-light-table).
   ✅ Camera-relative root orientation corrected 2026-07-11: the port applies
   the effective 180° view-space turn so Klaus presents his head toward the
   camera and his tail behind, matching the observed original menu.
3. ✅ **Klaus Intro Sequence animation** (gap 11) — exact C090 integer
   phase/Y/Z/zoom state and C710 fixed-point pose are live. Only the shared
   process-global RNG call order (exact twitch onset) remains unresolved.
4. ✅ **Extra keybindings** (gap 12) *(DONE 2026-07-06)* — Shift+Esc instant
   quit, Alt+Enter fullscreen toggle (new `Renderer::set_fullscreen`), keys
   1–3 direct quick-load of save slots. Verified on-screen.
5. **Attract-mode demo** (gap 15) — 60 s idle → demo; needs the gameplay/AI
   path. Low priority, forward-looking.

## Critical (immediately obvious)

1. ✅ **Music: the frontend menu must be SILENT.** The old port looped track02
   (`main.rs` GameState::Menu: `mp.play_track(0)` whenever not playing). The
   original never starts CD audio in the frontend — music exists only
   in-game (track = world+1, i.e. tracks 2–7, mode overrides 8/9), pauses on
   results, and is stopped+rewound on return to frontend. Menu audio = SFX
   only, over the hangar scene. **Transport parity completed 2026-07-17:** all
   Ambient values 0–15 round-trip unchanged, positive values play decoded PCM
   at unity, zero pauses without moving the cursor, and ordinary world loads
   select their authored track even while Ambient is Off. Intro2 remains
   silent; Escape pause leaves the live gameplay transport alone; returning
   to the frontend stops, rewinds, and releases the selection.

2. ✅ **The menu is a live 3D scene, not a screen.** Retail spawns persistent
   type-0/global-model-1 Klaus plus a co-located type-1/global-model-145
   `flag` camera anchor; the visible craft is the separate `player4` ring prop.
   It renders no terrain or water. The scene includes the animated flame
   billboard behind the
   ring (9 sprite frames 1294–1299, 940 ms loop) whose frame also PINS the
   depth-fade near plane (frames 0/4 → 0x500/0x800). This expands the authored
   fade toward context `+0x7C` pure red; it does not change model illumination.
   The additive billboard independently supplies its own red/yellow sprite
   glow. A low rumble sound (pool id 57,
   `menu_39_bgloop.wav`) plays once per cycle (and once immediately at menu
   entry).

3. ✅ **Ring geometry and motion.** Original: 7 props at
   `(800·sin θ, −320·cos θ, 3500 − 1200·cos θ)` — the selected prop at θ=0
   is CLOSER and LOWER (drops toward the viewer), not just "in front";
   angular step 9200/65536 (≈50.5°, i.e. spacing for count=7, not TAU/n of a
   full circle — the seven props occupy ≈354°, effectively a full ring);
   **all props spin continuously about one axis at 3.436 s/rev**
   (`DAT_004DCEB0`), the selected one is NOT scaled up. Port
   (`render_ring_3d`): flat ellipse (y = 0), only the selected prop spins,
   selected scaled ×1.3, per-prop size normalization (original uses fixed
   instance scale 800 for all).

4. ✅ **Ring scroll feel.** Original interpolator: ±200 per notch with
   exponential decay τ ≈ **0.21 s** (`val -= dt·val·1.25/2^18`). The ring is
   a seamless **infinite carousel** — stepping off the last item rotates one
   notch forward into the first (user ground truth 2026-07-05; the earlier
   "spins the long way around ±count·200" static-RE reading was WRONG). Port
   bumps the scroll by the circular-shortest delta (`ring_step_delta`).
   (Original port bug: τ = 1/12 s, too snappy, plus a literal long-way wrap.)

5. ✅ **Screen transitions: fly, don't fade.** Original has NO alpha/black
   fades. Ring↔submenu (frontend depth 2, flags 0x40+0x20): 0.6264 s
   fly-OUT (selected prop pulls toward camera at (0x7000−clk)/16, others
   recede at (0x7000−clk)/2), stack switches, 0.6264 s fly-IN (props from
   far), ≈1.25 s total; deeper pushes (e.g. Options→Display in-game) do NOT
   re-fly. Port: 0.3 s black fade overlay on every push/pop
   (`FadeState`/`FADE_DURATION`, `draw_fade_overlay`).
   The fly clock is also supplied to 3D prop callback channels 1/2. L5
   `screenop` consumes channel 1 as the child monitor's Y rotation, so the
   selected monitor turns during fly-out and reverses/unwinds during fly-in;
   its independent global root spin never pauses across the stack switch.

6. ✅ **Text: the two sprite fonts.** Original text = OVL Section-4 sprite
   fonts — font 0 GREEN (normal rows), font 1 YELLOW (same metrics, sprite
   ids +173) for the selected row; **highlight = font swap only**, no
   color modulation, no pulsing. Baseline-anchored glyph blits, 1/100-px
   advance/kerning with signed whole-pixel blit offsets, word wrap at 60%
   screen width. Port: own rasterizer
   (`rasterize_text_colored`) with invented colors (yellow title, white
   labels, cyan hints). The fonts are sitting in the menu OVL Section 4 —
   parse and use them (FORMAT_DOCUMENTATION §Section 4, dumps in
   `reference/menu_render_pass.json`).

## High (felt within seconds of interaction)

7. ✅ **Missing prop whoosh (global sound 0x57).** Selecting a ring item plays
   sound 3 AND triggers prop anim cmd 4 → the transform whoosh: L3 blob 16
   at 0.7×22050 Hz **±20% random pitch per trigger** (297–446 ms). The port
   loads only the 7 level-2 sounds, so ids 0x39/0x57 (L3 pool) don't exist;
   no whoosh, no bg rumble. Port must build the full global pool (L2 ids
   0–6 + L3 ids 7–109) and honor type-5 freq/vol multipliers + variance at
   *play* time (per-trigger randomization), not at load time.

8. ✅ **Typewriter text reveal.** Submenu rows (row-group flag 8) type on with
   a 100 ms per-item stagger, each over 327.67 ms, starting only after
   fly-in completes. The 2026-07-17 Display trace confirms that DCEC8 resets
   when Active Camera commits a new window top, while the Sounds trace confirms
   an ordinary in-window selection does not reset it. The entering bottom/top
   row now re-types from DCEC8 during the glide; row-group flag 0x10 preserves
   the ordinary DCEC4 reveal instead.

8b. ✅ **Right-column options type from the right.** Left-hand labels
    grow left-to-right (prefix typewriter at the label origin). Value-column
    text and bars reuse the same DCEC4/DCEC8 reveal and now right-align the
    visible prefix at `label_x + pt7.x`. The completed string keeps that
    right edge; a shorter prefix sits further right so the option appears
    from the right. Sounds, Display, and Controls use this path on every
    enter.

9. ✅ **List layout.** Original settings screens: rows LEFT-aligned at layout
   pt3=(67,105) with step (0,14) (320-space; ×2 hi-res), value text
   right-aligned at label_x+185, spinner bars drawn with glyphs 0x1B–0x1E
   (`[`, empty, `]`, filled) — no arrows; scrolling window (window_rows)
   with the whole list sliding (τ 0.21 s) while the yellow highlight snaps.
   Retail keeps a one-row navigation margin: reaching the bottom visible row
   scrolls down and reaching the top visible row scrolls up, bounds permitting.
   Port now uses the loaded variant's layout points, has no invented title or
   hint, and clips sliding rows to the panel. Bottom text anchor pt12 is
   (160,220) low / (320,420) high and shows only the selected ring prop's
   label. Ring props use pt0 as their movable projection center
   ((160,155) low / (320,310) high); centering them at half-screen leaves the
   selected model 35/70 authored pixels too high and creates a false gap to
   the correctly placed label. The Display list preserves the six-row retail
   set (Rendering, Resolution, Bilinear, Display, Active Camera, Targetter)
   and deliberately inserts the port-only aspect-scaling and Classic
   Framebuffer selectors as seventh and eighth QoL rows. Classic Framebuffer is
   unavailable, visibly dimmed, and forced off under Native scaling. Runtime
   availability is intentionally separate from the decoded select callback:
   non-action headings such as `Quit Game?` retain the normal font tone while
   remaining outside cursor navigation.

10. ✅ **3-second scene intro.** After the AVI, state 1 shows the 3D scene
    alone (billboard, fog, Klaus idle) for 150 ticks @ 50 Hz = 3.0 s, then
    the ring flies in (0.63 s). No fade of any kind. Port fades from black
    straight into the interactive menu.

11. ✅ **Persistent Klaus "Intro Sequence" animation.** The type-0 Klaus actor idles with
    quarter-sine sway on its anim vars (periods 2.6/2.9/2.0/5.2 s,
    leg-staggered), random twitches every 2.1–4.1 s, spins on select (2.1 s
    full spin, Z eases 0xA00→0x15F4 τ 393 ms after a 786 ms hold), settles
    back with τ ≈ 0.5 s, and on game start/attract (cmd 5) the background
    zooms away over 524 ms. ✅ The command-4 phase/Y/Z presentation state is
    now replicated: after the 786 ms hold it animates Klaus away, drives the
    complete primary pose curve, and scales/shifts the flame with the original
    formulas; cmd 1 restarts the pose phase and eases the scene back. The
    secondary random channel-6/7 twitch is also live. Its C090 movement/zoom
    now uses the executable's integer microsecond/Q31 formulas, and C710 uses
    the exact 4,096-entry sine table and signed narrowing. ✅ A 2026-07-17 targeted
    trace resolves command 5's two clocks: the background zoom completes in
    524 ms, but the outgoing ring remains live for the full 626.4-ms fly leg;
    selected model 41 flies toward the lens while the other props fly away,
    with labels and branding suppressed. The visible `player4` ring craft and
    type-1 `flag` camera anchor are separate from Klaus. REMAINING: only exact
    twitch timing, because retail shares one process-global RNG stream whose
    cross-system call order is not yet recovered.

## Medium (subtle but part of the texture)

12. ✅ **Esc semantics.** Runtime ground truth from
    the original (2026-07-11): plain Esc pops a submenu but does nothing at the
    root ring; quitting requires selecting the Exit prop. The port now matches
    that depth-1 no-op. Shift+Esc instant quit, Alt+Enter fullscreen toggle,
    and direct save-slot keys remain implemented.

13. ✅ **Sound trigger nuances** (mapping is otherwise right — ids 0–3 = L2
    entries 0–3, and id 3 IS the same 55 ms tick as id 0):
    - cursor sound 0 fires only when the selection actually moves;
    - spinner at limit = sound 0 (port does this) but sound 2 is NOT a
      generic left/right sound — it's save/load/network feedback (slot
      select, errors);
    - all menu sounds play at full volume, native rate, centre pan,
      fire-and-forget (max 24 voices).

14. ✅ **Volume curves.** Sound slider s (0–15): linear buffer gain s·17/255,
    converted to dB·100 = 10000·ln(linear/65536)/ln(65536) — ≈ −6.25 dB per
    halving; s=0 = −100 dB (silent but still playing). Applied live on every
    spinner change. **Ambient (0–15) is a pure on/off gate for CD music** —
    magnitude never used; in the frontend it does nothing at all.

15. **[deferred] Attract mode.** 60 s idle (reset by any key press OR release; 10 s
    after the first timeout) → pop one screen per timeout; at the ring:
    prop cmd 5 (bg zoom-out 524 ms) then game start with mode code 4 (or 1,
    p=1/8) — the demo. Port: none.

16. ✅ **Backdrop prop screens.** Screens with flag 0x10 draw model 73
    `optionsh` full-screen behind the rows (fixed angle 0, y=0x1004,
    projection center pt5); settings screens ALSO have their own prop row
    (e.g. `screenop` on Display) at pt6=(160,170), spinning at the global
    3.436 s/rev. The port now converts pt5/pt6 to the centered camera's
    equivalent world-Y offsets while retaining FUN_0043B410's local -0x140;
    this keeps the rows inside `optionsh` and places the chosen screen prop
    below the panel at its original screen position. The retail angle-zero
    basis faces `optionsh`'s authored local -Z decoration toward the camera;
    the port now applies its equivalent π root turn, restoring both emblems,
    the translucent dark-green fill, inner frame, and hazard-tape surround.

17. ✅ **Bottom branding row.** Original every frame: sprite 1290 Frontier
    logo at x=0, 1292 copyright line centered, 1291 logo right-aligned, all
    at y = screen_h − sprite_h. Port does this (draw_menu_branding) — keep,
    including vertical-list submenus and their fly transitions. Note the
    sprites' global ids differ per graphics detail (L5 hi-res 1290–1292 vs
    L3 lo-res alts). Classic 4:3/Stretched modes must retain the loaded UI
   variant's authored framebuffer (320×240 low; 640×480, 800×600 or 1024×768 high) so these
    native-pixel sprites scale with the whole frame rather than remaining
    desktop-sized pixels. ✅ The optional Classic Framebuffer path now renders
    those modes into an authored-resolution color/depth target and linearly
   quantizes completed logical pixels to RGB565 before scaling the frame once.
   Classic startup selects the matching retail display tier through 1024×768;
   changing the resident resource tier requires restart. RGBA8 draw composition
   remains a [documented approximation](RENDER_PIPELINE.md#classic-framebuffer-colour-and-resolution).
   Persistent model/world textures and
    internal overlays remain point sampled, matching the recovered software
    rasterizer. Native mode deliberately disables this path.

## Already matching (don't touch)

- **No key auto-repeat**: original is edge-triggered (one press = one step;
  matcher FUN_00472320 fires on match-count transitions only). The port
  already filters SDL repeats (`window.rs` `repeat: false`). ✓
- Sound ids 0–3 → L2 Section-11 entries 0–3 with alias resolution. ✓
- Data-driven screen tree / stack / visibility / spinner clamping
  (menu_engine.rs) — structurally verified against the originals. ✓
- 7-item wrap navigation on the ring. ✓

## Port asset misuse to fix

- `MenuResources::trophy_icons` (L5 sprites 8–14) are **not menu icons** —
  they are the flame-billboard animation frames (+ flags sprites). The 2D
  carousel fallback is built out of the background effect's frames. The
  billboard frames should drive the animated background (gap 2); the ring
  items are 3D models (315–323, 41, 75), never sprites.
- `FADE_DURATION = 0.3` claims to match "the original's ~0x7000 counter" —
  the 0x7000 clock is the prop FLY transition (0.626 s/leg), not a fade.

## Verification sources

- `MENU_SYSTEM.md` — full decoded spec (every number above is cited there).
- `v2k-extract sounds` — regenerate the canonical 52 WAV pool; the exact menu
  ids and audible roles are catalogued in `MENU_SYSTEM.md`.
- `reference/menu_render_pass.json` — screen records, layout points, both
  fonts (metrics + sprite ids), ctx template, billboard frame table.
- Tools (re-runnable): `v2k-extract sounds`; `menu_re_render_dump.py` for the
  EXE-owned subset; `menu_re_scene_dump.py`; `menu_re_keytable.py`. OVL fields
  in the render-pass capture are maintained by the Rust format pipeline.
