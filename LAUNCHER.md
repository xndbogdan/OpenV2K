# V2000 launcher and disc setup

The normal Windows executable opens a classic desktop launcher before SDL or
the game renderer starts. Play uses the chosen installation, Options shares
the game's existing settings owner, and Quit leaves the game unopened.
It uses the Windows GUI subsystem, so desktop startup creates no console.
Shell startup can attach to the existing parent terminal, and redirected
command-line output remains available. The in-game developer console is
created only when explicitly requested with its toggle key.

## Artwork and public releases

The launcher windows and game executable use the authored OpenV2K V-only icon,
with transparent orange glow and embedded sizes from 16 to 256 pixels. Native
launcher windows choose separate small and large images for the current system
metrics; SDL game windows use the matching 256-pixel RGBA export.
The header displays the authored OpenV2K glow mark over a slowly moving dark
red atmospheric backdrop. Broad procedural folds share the lettering's vertical
filament and glow treatment, confined to a horizontally diffused halo derived
from the text. Restrained brightness modulation lets the backdrop breathe
gently. The complete scene is prerendered at 624 x 98 pixels into a 192-frame,
50 Hz indexed flipbook. Its 3.84-second loop contains four repetitions of the
recovered 960 ms menu billboard pulse. The backdrop is new procedural launcher
artwork, with no retail texture inputs. Its Blender sources and export recipe
are kept outside this repository; only the exported atlas is tracked here. The atlas decodes once to approximately 11.2 MiB of
palette indices and one shared palette. A separate 20 ms timer updates a single
cached BGRA frame and invalidates only the header when its frame changes;
delayed timers skip to the current loop position. Ordinary GDI painting keeps
the aspect ratio and reuses the cached frame. Both the icon and header are
embedded, so the launcher can display them before installation, without
Blender or an OpenGL context.
The game and launcher window titles use **V2K**, including Options and setup
dialogs. The main window has no repeated heading or footer branding.

## Installation discovery

Normal startup validates the executable's own directory. A remembered path,
read-only retail registry hint or shortcut's working directory cannot enable
Play when that directory is missing its game data. Those paths are only
suggestions for the setup folder chooser. Explicit `--data-dir` remains an
asset-root override for direct command-line diagnostic runs.

The setup screen offers an existing installation or a disc-image installation
into a chosen directory. Image installation copies the port executable,
`PRELOAD.DAT`, all 212 original overlays into `Overlay/`, both intro videos,
and available CD audio into `cdaudio/`. Choosing an existing installation
places the port executable there without replacing its game data. Setup then
reopens that installed executable; the original copy never plays remote data.
In a valid local installation, the main window hides installation controls;
Options keeps file verification available, disables disc installation, and
offers music extraction only while the soundtrack is missing or incomplete
and the local game files are ready.
Changing installation is a setup action rather than an Options control.
The validation log places affected paths on separate lines and scrolls to the
latest detail after each check or installation.

Original overlay bytes are copied
verbatim; the retail installer and original executable are not required.

## Disc images and music

The file chooser defaults to `V2000.bin` and also accepts CUE and ISO files.
A BIN automatically uses its companion CUE if present. A CUE resolves its
referenced image files relative to itself and supplies the physical track
boundaries. Bare BIN data can be installed without inventing audio boundaries.
Conventional ISO images supply data files; absent CD audio is a warning rather
than a reason to block Play.

CD audio is extracted as lossless 44,100 Hz, stereo, signed 16-bit PCM WAV.
The runtime streams both WAV and existing OGG files through the same bounded
worker queue. There is no FFmpeg or encoder dependency. Physical names
`track02` through `track11` retain their original identities: missing tracks
stay missing rather than assigning another track to the wrong world. A valid
OGG is preferred when both formats exist, with a valid WAV as fallback.

The launcher distinguishes a complete soundtrack, an incomplete soundtrack,
and unavailable music. Missing music does not turn off sound effects or
overwrite the user's music preference. Extract Music can add or repair the
soundtrack in a validated local installation without reinstalling game files. An absent `cdaudio/` directory
shows a short message directing the user to Options; unexpected directory
read errors retain their diagnostic details.

## File verification and replacement

Validation checks the seven embedded PRELOAD overlays, all 53 logical overlays
in each of the four presentation tiers, declared final-section boundaries,
and canonical dependency records. Parser success alone is insufficient:
the [historical truncated overlays](docs/re/RETAIL_DATA_INTEGRITY.md)
retained valid container markers while losing final dependency bytes.

Disc installation validates the source before publication, writes staged
files, compares source and destination SHA-256 checksums, and retains backups
when replacing managed files. `v2000-installation.json` records installed
file sizes and hashes for later verification. The executable also embeds the
verified retail reference for `PRELOAD.DAT` and all 212 overlays, so matching
retail installations need no sidecar JSON. A launcher-only manifest can record
just the executable while those 213 game files use the embedded fingerprints.
Gameplay settings and saves are not repair targets.

Required bytes that differ from the bundled reference must match a recorded
installer fingerprint. Otherwise Play is blocked as an **unrecognized release
or changed required file**, even if structural checks pass; a fingerprint
difference alone is not proof of corruption. Installing from a supported
custom disc image records its source hashes and allows subsequent verification
without requiring that release to match the bundled reference. A mismatch
against an existing installer-recorded required-file hash always blocks Play.
Replacing the port executable with a newer build is an optional checksum
warning; required retail game files still receive strict verification.

The reviewable [embedded reference](crates/v2k-game/src/setup/retail-checksums.json)
contains sizes and SHA-256 values only. It was generated from the supplied
`V2000.bin` installation manifest, with all 213 files independently matching
the canonical port corpus. Regeneration verifies the complete corpus against
an existing source/destination manifest before replacing this table:

```powershell
.\scripts\generate-retail-checksums.ps1 -ReferenceManifest .tmp\launcher\no-music-install\v2000-installation.json
```

Run that command from the repository root. Reference regeneration is explicit;
ordinary builds embed the checked-in table and need no private game data.

Missing or corrupt required assets block Play with file-specific diagnostics.
Missing videos or soundtrack tracks remain optional warnings. Setup runs in
a worker so the window stays responsive while importing or verifying files.

## Options and command-line operation

Launcher display and audio controls use `GameConfig::load` and `try_save`,
including the same portable settings files and port-owned registry namespace
used by the game menus. Retail registry settings remain import-only.
Display offers In a Window, Full Screen and Borderless. Resolution lists what
the game's Display menu lists for the primary display, where the game opens:
640x480, 800x600 and 1024x768 plus the display's reported modes, in physical
pixels ([colour depth and resolution](docs/re/RENDER_PIPELINE.md#colour-depth-and-resolution)).
Changing Display lists its resolutions again. Full Screen changes the display
mode when the game starts; Borderless covers the desktop, so Resolution shows
the desktop's size and is disabled.
Renderer offers Automatic, OpenGL and Software; without a command-line choice,
OpenGL that cannot start falls back to Software.
The game is per-monitor DPI aware. The launcher's own windows keep their
system-DPI layout.
The launcher remembers its data directory, last image and automatic-launch
preference separately under `%LOCALAPPDATA%/V2000 Port/launcher.json`.
`--launcher-state <path>` selects another preference file for a portable or
test session.

Runtime saves are written directly beside `v2k-game.exe`. Older saves under
the data directory, its `saves/` directory, its parent or the retail registry
path remain readable imports; new saves never write back to those locations.

`--launcher` forces the setup window. `--no-launcher` starts directly after
validation. A saved skip-launcher preference only skips a healthy installation;
broken data brings setup back. Explicit `--level` and VTOL tracing preserve
direct diagnostic startup. Existing `ovl` and `init` commands are unchanged.
On other platforms, command-line setup and direct game startup remain available.

From the repository root, build and use release binaries:

```powershell
cargo build --release -p v2k-game
cargo run --release -- --launcher
cargo run --release -- --no-launcher --data-dir . --skip-intro
cargo run --release -- verify .
cargo run --release -- install --image D:\Discs\V2000.bin --destination D:\Games\V2000-Port
cargo run --release -- extract-music --image D:\Discs\V2000.cue --data-dir D:\Games\V2000-Port
```

`install --no-music` imports game data only; `--assets-only` also omits the
port executable. Windows builds link SDL statically so a standalone executable
can reach the setup window even in an empty directory without SDL2.dll on PATH.
Building that Windows executable uses the SDL sources bundled with `sdl2-sys`
and requires CMake alongside the repository's existing MinGW toolchain.
The V2000 workspace Cargo configuration supplies CMake's external 3.5 policy baseline
for the bundled SDL sources when using CMake 4 or later.

## Ownership and verification

`v2k-game::setup` owns bounded image reading, asset validation and installation.
`installation_discovery` only supplies read-only path hints;
`launcher_preferences` owns the separate launcher state. The binary's launcher
module owns native Windows controls and asynchronous job presentation.
`v2k-render::music` owns soundtrack inventory and bounded audio decoding.

Focused tests cover malformed images, incomplete final sections, embedded
PRELOAD coverage, installation replacement/backups/checksums, executable-dir
discovery, numbered tracks and WAV streaming. Retail data remains local
evidence, and extraction tests write only disposable temporary installations.
The launcher is port setup policy, not a reconstructed original-game screen.
