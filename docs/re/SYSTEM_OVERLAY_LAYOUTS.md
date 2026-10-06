# System-overlay layout refresh

This document owns the cache boundary for live high-tier layout changes.
[MENU_SYSTEM.md](MENU_SYSTEM.md) owns frontend composition and font consumers;
[RENDER_PIPELINE.md](RENDER_PIPELINE.md) owns submission and projection policy.
The refresh is a **port live-display policy**: it retains the current world and
its initialized simulation state while installing another authored high layout.
It does not establish retail resize behavior or visual acceptance by itself.

## Fixed source families

System2 and System3 variants1/2/3 have byte-identical Section2..14 headers and
payloads. Their Section0/1 tables retain different authored layouts for640x480,
800x600 and1024x768. System5 and51 high trios are entirely identical. These
invariance claims are checked against the retail corpus; decoded equivalence
cannot substitute for raw bytes, and the world Section13 name-padding exception
does not apply to these system files.

| System | Section0 words | Section1 packed points | Consumers |
| --- | ---: | ---: | --- |
| 2 | 6 | 13 | Authored lens, viewport/centre, menu layout |
| 3 | 18 | 35 | Radar configuration and rectangle, HUD/status layout |

Section0 is not uniformly presentation data. System3 words0..5 are
`[96,96,4,5,3,50]` in all three high sources. TerrainRadarConfig consumes
words2/3/4/5 as color bands, color stride, coverage maximum and footprint radius.
Words10 and13..17 also remain invariant. The layout refresh requires these
words to match the resident source. Only words6..9 and11..12 vary across the
actual high scalar tables. In particular, High1024's value4 is **word6/map X**;
word4/coverage maximum remains3. Section1 is installed verbatim, including
authored point differences that are not uniform scaling.

Variant0 owns320x240 art and different intrinsic sections. A low or unproven
resident source cannot enter this high-only transaction. Loading a different
intrinsic family remains an explicit asset-load boundary.

## Preparation and commit

[system_layout.rs](../../crates/v2k-game/src/system_layout.rs) exposes
HighSystemLayoutTier, SystemLayoutSource and PreparedHighSystemLayouts.
[GameSession](../../crates/v2k-game/src/session.rs) reads both candidates
before requesting a stage. The stage offers the same scalar and signed packed
point getters as the resident ResourceCache, so dependent presentation
snapshots can be validated without temporarily changing the live cache.

The normal disk auxiliary loader retains an immutable raw receipt for the
loaded high System2/3 intrinsic headers/payloads. Receipts retain no decoded
asset copies; the two normal resident sources require about4.7MB in total.
Changing layouts shares those receipts and never appends cache layers. Direct
parsed/ad-hoc auxiliary insertion has no raw receipt and cannot authenticate a
refresh merely through a filename or decoded resource shape.

Preparation requires canonical15-section OVL parsing, exact Section2..14
header/payload equality, the complete Section0/1 table sizes and their declared
byte lengths, invariant System3 scalars, and the selected System2 viewport.
The strict table check precedes the permissive fixup parser, which otherwise
drops a trailing incomplete word. A malformed or missing second candidate
leaves both resident layouts and their provenance untouched.

[The cache commit](../../crates/v2k-game/src/resource_cache/system_layout.rs)
checks both original receipt identities, layout origins and old table values
before its first write. A competing refresh or an independently reloaded
source rejects a stale stage. Once both checks pass, only the four fixup
tables and their layout origins are replaced. LevelState.source_path continues
to identify the intrinsic source; system_layout_origin identifies the selected
layout source separately. The caller publishes its already prepared dependent
snapshots only after successful commit.

No world layer, parsed sprite/model/font/palette/linkage pool, live terrain,
initialized TerrainRadar/config/raster, actor graph, simulation clock or shared
RNG is reconstructed. This lifetime boundary is deliberate: a future world
initialization reads the then-resident authored radar scalars, while an existing
world retains its initialized raster and configuration.

## Source controls and limits

[Focused corpus controls](../../crates/v2k-game/src/resource_cache/system_layout/tests.rs)
exercise all high tiers and repeated toggles with constant layer count and
intrinsic allocation identity. Missing second files, intrinsic header/payload
corruption, table-header damage, an actual truncated point word, viewport
mismatch and changed radar configuration reject before mutation. Competing
stages and a rebound source receipt also preserve the current complete state.
An actual Level1 construction retains terrain bytes, radar allocation/content/
revision, actor task/clock/context state and the shared FX/RNG stream through
the three-tier cycle.

These controls prove the resource transaction. Tier choice from output size,
Low/Classic enlargement, dependent paused-frame relayout and visual comparison
remain the presentation caller's responsibility.
