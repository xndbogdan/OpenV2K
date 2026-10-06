# Launcher artwork

`openv2k-header-animation.png` is the complete prerendered launcher header:
the V2K glow mark and a slowly moving dark red atmospheric backdrop. Broad
procedural noise folds share the lettering's vertical filament and thermal
glow treatment, confined to a horizontally diffused halo derived from the
actual text. The opaque 8-bit indexed PNG contains 192 frames in
16 columns and 12 rows. Each frame is 624 x 98 pixels; the atlas is
9984 x 1176. `header-animation.json` records that layout and the 20,000
microsecond frame duration. One 3.84-second loop contains four repetitions of
the recovered 960 ms logo pulse.

The animation was baked from six authored 1248 x 196 logo canvases. A shared
crop and scale keep the mark's proportions and placement fixed while its heat
changes. The Blender sources, export recipe and those canvases are kept outside
this repository. After re-exporting, rebuild `v2k-game`: the atlas and manifest are embedded at compile time and decoded
once before the window opens. The decoded atlas uses approximately 11.2 MiB
of indices with one shared 256-color RGB palette and a single cached opaque
BGRA frame. No retail installation, Blender or OpenGL context is needed at
runtime. The native launcher preserves the canvas aspect ratio and uses GDI
halftone scaling.

The exporter samples `FUN_0042D030`'s recovered nine-slot frame order and
durations at 50 Hz, retaining its 48-submission / 960 ms steady pulse. It
linearly interpolates 32 atmospheric field samples into the full animation,
with 16% heat-pulse modulation depth for gentle background brightness changes.
The runtime header has its own 20 ms timer and selects the current frame from
elapsed time modulo the full loop, skipping directly over delayed frames
without a catch-up loop.
Ordinary paints reuse the cached frame without advancing the clock. Only
changed header frames invalidate the banner, without erasing or invalidating
the setup controls.
This is a silent branding animation: it has no retail menu-entry rumble or
scene depth-fade effect.

The atmospheric field and spreading glow are newly authored procedural art,
with no retail texture inputs. Their motion, diffusion, brightness and vignette
are artistic parameters documented in the artwork recipe. This background
arrangement is not claimed as original retail menu behavior.

The same artwork recipe exports a V-only icon to the shared
`v2k-render/assets/openv2k.ico` and `openv2k.png`. The game and viewer embed the
ICO as `IDI_ICON1`; native launcher windows select small and large resource
images at their system sizes. SDL windows decode the matching RGBA PNG,
preserving its soft transparent glow.
