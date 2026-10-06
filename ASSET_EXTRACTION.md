# V2000 asset extraction

`v2k-extract` is the canonical bulk exporter. It uses the same `v2k-formats`
decoders as the game and viewer, so format corrections automatically apply to
new exports. It replaces the retired Python format/extraction scripts and owns
the generated `extracted/` tree.

Run from the repository root with a private installation in `retail/` (see the [test and data setup](README.md#building)):

```powershell
# PRELOAD.DAT plus every Overlay/*.OVL
cargo run --release -p v2k-extract -- --data-dir retail all

# One asset family
cargo run --release -p v2k-extract -- --data-dir retail sprites
cargo run --release -p v2k-extract -- --data-dir retail models
cargo run --release -p v2k-extract -- --data-dir retail sounds

# A selected OVL and destination
cargo run --release -p v2k-extract -- --data-dir retail all `
  --ovl Overlay/1X3XX.OVL `
  --skip-preload `
  --output extracted/common-assets
```

The leading digit in the system OVL name is a presentation tier.
`0X3XX.OVL` is the authored 320x240 low-resolution source;
`1X3XX.OVL` is the normal 640x480 high-resolution source, and variants 2/3
carry the larger layouts. Visual inspection and extraction should therefore
default to variant 1 unless a low-resolution comparison is intentional.

Sprite PNG export uses the brightest authored palette row by default. `--shade`
accepts the complete 0–31 ramp. Bulk sprite decoding is strict: if any metadata
entry cannot be decoded, the command fails and identifies every affected entry
instead of silently omitting assets.

The exporter writes per-source manifests next to PNG and OBJ files, plus a
batch `manifest.json` at the output root. Manifests preserve source names and
runtime metadata that interchange formats cannot represent.

`all` deliberately exports the OVLs embedded in `PRELOAD.DAT` as their own
resident sources as well as the disk Overlay set. Consequently a complete
retail run reports 15,520 sprites and 5,366 models, while the Overlay-only
corpus contains 15,064 sprites and 5,344 models. The workbench does not present
those resident copies twice: it browses deduplicated global ids, while loading
the selected disk tier for visual-bearing system levels—including levels 2 and
5, whose PRELOAD copies contain low-tier atlases—so model textures and sprite
previews remain coherent.

Model OBJ export re-materializes each command stream with
`AnimVars::default()`, including embedded register operations, and writes
gameplay world coordinates (`raw / 256`) with runtime face UVs/material ids.
Child instances remain a hierarchy; their global targets, attachment
transforms, linked slots, and register snapshots are recorded in the manifest.
View-dependent and external-frame limitations are stated explicitly rather
than baked into misleading static geometry.

`sounds` exports the retail GLOBAL Section-11 pool once under `sounds/`: system
level 2 from `PRELOAD.DAT[2]` supplies global ids 0-6, followed by system level
3 at ids 7-109. The exporter deliberately reads `Overlay/0X3XX.OVL` for this
nonvisual pool because its Section-11 bytes are identical in all four display
tiers; this is not a recommendation to use low-resolution visual art. Its
manifest records global id, source
level/path/local id, exact alias chains, per-hop frequency variance/frequency
multiplier/volume multiplier, and the resolved cumulative multipliers. The 110
slots resolve to 52 PCM samples, so exactly 52 WAV files are written. Display
variants carry duplicate copies of these sounds and are not exported as extra
audio.

Because these two sources define the runtime pool, `sounds` (and the sound
portion of `all`) always reads them. `--ovl` and `--skip-preload` restrict only
the sprite/model source scan.

Every run is built in a sibling staging directory and replaces the destination
as one managed tree only after all strict parsing and serialization succeeds.
This means a later `sprites` run cannot retain stale models or sounds from an
earlier `all` run. The exporter refuses to replace a non-empty directory it did
not create; `--keep-going` is the explicit opt-in for retaining sources with
section warnings.

Generated output defaults to `extracted/` and is not committed.
The directory is reproducible and may be replaced atomically by the extractor;
curated semantic evidence belongs in the RE notes, not in the generated asset tree.

`v2k-viewer` is the companion visual debugger. Its model and sprite tabs use
the runtime global pools, and its single-OVL sprite/model exporters call the
same `v2k-extract` library. Viewer `--mode audio` is a diagnostic dump of one
OVL's Section 11; it is not a substitute for the canonical 110-slot global
sound-pool export above. Viewer maps are raw Section-10 diagnostic images and
are likewise kept separate from the bulk runtime-asset export.
