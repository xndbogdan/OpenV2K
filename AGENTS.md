# Repository Guidelines

These guidelines apply to human contributors and coding agents alike.

## Project Structure

OpenV2K is a Rust reimplementation of V2000. It is built from reverse
engineering of the original executable and data. The Cargo workspace lives at
the repository root, with these crates under `crates/`:

- `v2k-core`: error types.
- `v2k-formats`: OVL and section parsers.
- `v2k-extract`: bulk asset exporter.
- `v2k-game`: frontend, launcher and gameplay; the default member.
- `v2k-render`: SDL/OpenGL backends.
- `v2k-viewer`: asset workbench.
- `v2k-inspect`: read-only probes for the retail executable.
- `v2k-test-support` and `v2k-test-macros`: optional private-corpus testing.

Integration tests live in each crate's `tests/` directory. Reverse-engineering
writeups live in `docs/re/`. Only exported OpenV2K branding (the launcher atlas
and icons) is tracked; its Blender sources are kept outside this repository.

Original game data is never committed. Optional retail test data goes in the
ignored `retail/` directory (`PRELOAD.DAT` and `Overlay/`), or in the directory
named by `V2K_RETAIL_DIR`. Relative overrides resolve from the repository root.
Demo comparison data goes in `demo/` or `V2K_DEMO_DIR`.

## Build, Test, and Development Commands

Run everything from the repository root. Use `--release` for `cargo build` and
`cargo run` unless you are debugging a build-configuration-specific problem.

```sh
cargo build --release        # optimized build
cargo check                  # fast type check
cargo test --workspace       # complete workspace
cargo test -p v2k-formats    # parser-focused tests
cargo test -p v2k-game       # game/menu/runtime tests
cargo test -p v2k-game --test menu_pools # focused integration suite
cargo run --release          # launcher, then intro/menu from chosen installation
cargo run --release -- --no-launcher --data-dir retail --skip-intro
cargo run --release -- --data-dir retail --level 13 # first-world overlay 13
cargo run --release -p v2k-viewer -- retail/Overlay/1X14XX.OVL --mode models -o models_out
cargo run --release -p v2k-extract -- --data-dir retail all # regenerate exports
```

Before handing off work, run `.\scripts\check.ps1` (rustfmt, the optional-corpus
policy self-test, and workspace tests). Add `-RequireRetailData` for a fidelity
handoff that must exercise the retail corpus. Add `-Clippy` to opt into lints;
some pre-existing test-expression lints are not clean yet.

Retail-backed tests use `#[v2k_test_support::retail_test]` and are reported as
**ignored** when no retail corpus is configured. Once any payload is present,
incomplete or corrupt data must fail instead of being skipped.

On `x86_64-pc-windows-gnu`, `.cargo/config.toml` selects
`scripts/windows-gnu-gcc.cmd`. That script keeps MinGW GCC's CRT and SDL2 lookup
and uses the active toolchain's bundled LLD, so MinGW GCC must stay on `PATH`.
The test profile uses `line-tables-only` debug info. With full DWARF, the game
unit harness exceeds the Windows PE 4 GiB image limit.

Enable the repository hooks once per clone with
`git config core.hooksPath .githooks`. The pre-commit hook regenerates the
progress numbers in `README.md` and `docs/progress/`; never edit those by hand.
When a retail function is implemented, approximated, documented or deliberately
not needed, record it in `docs/progress/status.csv`. Cite retail functions by
their start address (`FUN_0042D030`), so the progress count stays meaningful.

## Coding Style & Naming Conventions

Follow standard `rustfmt` formatting:

- four-space indentation;
- `snake_case` functions and modules;
- `CamelCase` types;
- `SCREAMING_SNAKE_CASE` constants.

Prefer small modules that match existing crate boundaries. Keep RE names,
addresses and original-function references (for example `FUN_0042D030`) when
they clarify provenance.

## Testing Guidelines

Add or update focused tests when you change parsers, menu behavior, renderer
data paths, saves, or world/terrain logic. Run at least `cargo test -p <crate>`
for the crate you touched, and `cargo test --workspace` before broader changes
are handed off.

## Commit & Pull Request Guidelines

Use short imperative subjects with an optional scope, for example
`menu: resolve backdrop model`. Keep commits focused. A pull request should:

- summarize the change;
- list the validation commands;
- call out game-data assumptions;
- include screenshots or comparison notes for visible rendering or menu changes.

## Fidelity Rules

Faithfulness to the original game comes first. When decompiled code, extracted
assets or original-game observations can verify behavior, timing, visuals,
audio or data interpretation, prefer that evidence over convenience
approximations.

Some primary evidence is private and not part of this repository: decompiler
output, analysis scripts and time-travel debugging traces. `docs/re/` holds the
conclusions drawn from it. If a change needs evidence that the public notes
don't contain, say so instead of guessing.

Caution is welcome; a silent stall is not. If a fidelity gap blocks the work and
can't be closed within the task, do one of two things:

- Adopt a **labeled approximation**. Name it in code and in the owning
  `docs/re/` document, record the evidence bounds, and keep it replaceable.
  Never bake captured values in as behavior. An approximation never counts as
  retail acceptance.
- Or stop, report the blocker, and propose the approximation you would use.

Fail closed per feature: skip and log the unowned actor, effect or draw. Don't
fail a whole phase.

V2000 system-overlay variants are presentation tiers, not interchangeable
copies:

- `0X3XX.OVL` holds the authored 320x240 low-resolution art.
- `1X3XX.OVL` is the normal 640x480 source.
- Variants 2 and 3 keep the high-resolution pixels with 800x600 and 1024x768
  layout data.

Interactive runtime and visual tools must follow the selected tier and default
to variant 1.

## Maintainability and RE Safety

- Treat original OVL bytes as fixed-layout evidence. Never insert bytes or
  otherwise shift offsets inside an original overlay.
- Before adding a renderer, parser or gameplay API, search all call sites and
  existing variants. Prefer one data-bearing request type plus an explicit enum
  for genuine modes over a ladder of near-identical methods.
- Never encode a behavioral mode through a coincidental numeric value or other
  sentinel unless the original data format requires it. Name coordinate units
  and conversions. Verify signedness and units of each legacy field at its
  retail consumer.
- Respect the renderer's dual vertex-space contract. Inside a model draw,
  vertices are **raw-local**, and only the GL matrix stack maps them to world
  space. Manual helpers (projection, edge-quad selection, screen-space tools)
  work in true **world** coordinates. Convert explicitly at every boundary. See
  [RENDER_PIPELINE.md](docs/re/RENDER_PIPELINE.md).
- Keep intrinsic asset/model decoding independent of presentation context.
  Never repair one flipped object with a blanket texture/UV reversal or a
  context-dependent parser rule.
- Treat long functions as a cohesion problem. Extract complete phases with
  clear inputs and outputs; don't just move code around.
- For behavior-preserving refactors, write down the old decision matrix before
  editing. Rendering refactors also need a visual smoke test of the affected
  menu, cinematic and gameplay scenes.
- Keep RE provenance attached to the code that implements it. When evidence
  disproves an old interpretation, update the relevant `docs/re/` notes in the
  same change.
- Don't hand-edit or commit generated extracted output, local captures or game
  data.
