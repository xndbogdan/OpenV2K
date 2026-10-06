# V2000 Portable Radar (Type 83)

This document owns the recovered class-60 Portable Radar constructor, task,
coverage, and lifecycle contracts. **The native runtime remains unimplemented.**
Source recovery does not admit Type83 construction, authored gameplay execution,
or campaign-cargo restoration. A constructor-only adapter would omit the task's
shared radar mutation and its destructor's cleanup.

[CAMPAIGN_CARGO.md](CAMPAIGN_CARGO.md) owns saved identities and the common
match/miss/attachment transaction. [ACTOR_RUNTIME.md](ACTOR_RUNTIME.md) owns
native allocation and task custody; [ENTITY_DAMAGE_AND_DEATH.md](ENTITY_DAMAGE_AND_DEATH.md)
owns the common hit and death dispatch.

## Evidence and authored inputs

The evidence is the local retail PE and canonical Section-12/13 parsing of
normal-tier gameplay overlays 13 through 49. The cargo census finds 14 authored
Type83 births in overlays **30, 39, 40, 42, 46, and 47**. Its header is invariant
across that corpus. Birth counts prove reachability, not execution of a native
task in the port. No new capture is required to establish the branches below.

| Input | Recovered value |
|---|---|
| Type / model slots | 83 / `[79, 79, 79, 79]`, model `tinyscan` |
| Mass / capabilities | 50 / `0x00001004` |
| Initial health | 30000 |
| Damage thresholds | `[0, 5000, 1000, 1000, 200, 0, 0]` |
| Damage multipliers, Q8 | `[0, 256, 256, 256, 0, 0, 0]` |
| Initializer flags | `0x00025021` |
| Common axis descriptor | `[512, 8]` |
| Components | SubK only; A/B/C/D/E/F/G/H/I/J/L/M/N/O absent |
| SubK payload | `[1, 0]`: one variable, initially zero |
| Weighted choice | Rule1, multiplier1, class60; rule ref1 |
| Alternate class | 49 |
| Death sound | 62 |
| Other sounds | No constructor attachment, accepted-hit, generic-hit, infected-model, or target-warning sound |
| Common effects | Surface selectors `[0, 0]`, surface lifetime0, low-health words `[0, 0, 0]`, terrain-contact task lifetime0 |

The [runtime metadata decoder](../../crates/v2k-game/src/entity_collision_state.rs)
and [format sections](../../crates/v2k-formats/src/sections.rs) retain the
canonical data interpretation. The class catalog in
[entity_behavior.rs](../../crates/v2k-game/src/entity_behavior.rs) records
class60, name pointer `004C8850`, and style descriptor `004C7300`.

The 18 dwords at `004C7300` are zero except style `+08 = 0040C690`,
`+0C = 0040D1C0`, and `+40 = 0040C560`. Thus attach, release, and initializer
are the only nonnull class-60 entries. In particular, primary-hit `+28`,
infected-model `+20`, death-cleanup `+2C`, terrain/surface callbacks, and
enable/disable callbacks are null; initializer argument `+44` is zero.

The omitted bulk-decompiler routines can be reproduced read-only from the
repository root with these PE disassembly ranges:

```powershell
objdump -D --start-address=0x40c560 --stop-address=0x40c5e0 retail/V2000.EXE
objdump -D --start-address=0x405540 --stop-address=0x4056d0 retail/V2000.EXE
objdump -D --start-address=0x40baf0 --stop-address=0x40bdb4 retail/V2000.EXE
objdump -s --start-address=0x4c7300 --stop-address=0x4c7348 retail/V2000.EXE
objdump -D --start-address=0x40c4d0 --stop-address=0x40c500 retail/V2000.EXE
objdump -s --start-address=0x4c74f8 --stop-address=0x4c7540 retail/V2000.EXE
objdump -s --start-address=0x4c8888 --stop-address=0x4c8890 retail/V2000.EXE
```

## Constructor and native task

| Routine | Source-ordered contract |
|---|---|
| `0040C560..0040C5D4` | Zero entity Euler words `+A2`, `+A4`, `+A6`; clear Secondary through `40A7A0(entity, 1, NULL)`; clear Tertiary through slot2; return `405540(entity, 0, 0)` |
| `00405540..004055B0` | Resolve entity `+4C` context; prepare the `405FF0` wrapper with tick `405610`, context, and supplied lifetime; override template destructor `+14` with `4055C0`; call `406030`; replace the requested task slot through `40A7A0` only on success |
| `00406030` | Allocate through `401020`; only a successful allocation runs component reset `406070` |
| `0040A7A0` | Destroy the old slot through `4010D0` before storing its replacement; initialize the new task's context field afterwards |
| `0040A860` | Destroy and clear Primary, Secondary, Tertiary in that order |

The latter shared routines are in game_logic.c.
For Type83, all optional components touched by `406070` are absent, so its
successful task preparation consumes no component-reset RNG. The singleton
`40AC60` behavior selection still consumes its normal weighted-selection draw.

If Primary allocation fails, the direct C560/405540 entry has already zeroed
Euler and cleared Secondary and Tertiary, but the old Primary remains installed.
Its destructor has not run at this boundary. New task preparation itself does
not reset the shared SubK word; initial component construction supplies zero,
and successful replacement destroys the old task.

The complete behavior dispatch has a later failure phase. `40C6B0` receives
C560's nonzero result, reports it, and calls `40ABB0` with descriptor
`004C8888`. That unnamed fallback selects style `004C74F8`, applies its
`+34 = 0x1280` enable policy, and invokes `40C4D0`. This initializer clears
**Secondary, Tertiary, then Primary**, without another allocation or weighted
selection. A surviving old deployed Primary is therefore destroyed here and
removes coverage. The context's target and auxiliary words survive descriptor
replacement. Distinguish the direct failed-install prefix from this terminal
outer fallback; neither leaving coverage installed after the complete fallback
nor removing it before the failed allocator returns matches retail.

Common `40D4A0` grounding sees flag `0x20`, no SubC, and no model-radius flag
`0x40`, selecting plain terrain height. C560 then overwrites authored Euler;
the `4104B0` birth suffix builds the resulting identity body basis. Release
has a distinct basis sequence described below.

### Deployment tick and destruction

`405610` accesses SubK variable1 through `40A950(entity, 1)` as an **unsigned
16-bit word**. It does not use the detailed/coarse callback mode argument.
Every path returns zero and does not request a behavior transition.

1. If entity state `+08` has attachment bit `0x1000`, write K1=0 and return.
   This branch itself does not remove coverage: ordinary successful attach
   already removes it by replacing and destroying the old Primary.
2. If K1 is `0xFFFF`, return without refreshing radar or advancing it.
3. Otherwise write `min(0xFFFF, old_K1 + (delta_us >> 4))`.
4. Only if that write reaches `0xFFFF`, call
   `44A8D0(current_position_x, current_position_z, +1)` at `004056AB`.

There is no fractional time accumulator. At fixed 20000-us visits, each adds
1250; the 53rd visit reaches deployment. The wrapper uses lifetime0 rather
than a separate timed behavior transition.

Those are callback visits, not unconditional display frames. The enclosing
`12DA0` scheduler owns deferral, accumulated time and its callback cap. The
class-60 effective flags remain `0x25021`: detailed `DCA0` and coarse `E870`
both reach `A800` because `0x2000` is clear. Both retain the incoming body
basis (`0x4000`), skip terrain attitude (`0x10` clear), apply E100 gravity
(`0x4` clear), skip wind/drag (`0x8` clear), and skip DF70 ground snap (`0x2`
clear). SubC is absent, so E100 has no underwater buoyancy branch. The zero
surface selectors leave E370's timer decay, except when state `0x20000000`
suppresses that owner. Reusing the turret's `0x25027` suffix would incorrectly
suppress gravity and introduce ground snap.

Destructor `4055C0` checks K1 at `004055D0`. When it equals `0xFFFF`, it reads
the current position and calls `44A8D0(x, z, -1)` at `004055F4`. Only after
that call does it resolve K1 again and write zero at `00405607`. It records
no activation center, and no separate boolean may replace the shared component
word. Destruction of a partly deployed task only resets the word.

## Coverage and raster mutation

`44A8D0` and its immediate raster consumer `44A5C0` are retained in
game_logic.c. Coverage is the packed byte array at
`004EF130`; the 256x256 terrain raster is at `004DF118`.

`44A8D0` reads the common scalar pointers at `DAT_004FE620 + 2C` (footprint
radius), `+28` (maximum contribution), and `+20` (raster color argument).
For the authored positive-radius domain, the operation is:

1. Convert input raw X/Z to unsigned high-byte cell coordinates.
2. Visit offsets from `-radius` through `+radius` inclusive, **X outer, Z inner**.
3. Compute integer square-root distance, clamp it to radius, and subtract
   `trunc(maximum * distance / radius)` from maximum. Multiply the resulting
   byte contribution by the signed add/remove argument.
4. Wrap cell coordinates to the 256-cell world. The coverage index is
   `z * 256 + x`; even X uses the low nibble, odd X the high nibble.
5. Multiply the byte contribution by16 for odd X and add it to the **whole
   packed byte with byte wrapping**. Retail does not clamp or independently
   saturate each nibble: low-nibble carries and borrows affect its neighbor.
6. Immediately call `44A5C0(x, z, x << 8, z << 8, raster_argument)` before
   advancing to the next cell.

The square includes cells whose distance is at least the radius, contributing
zero. Their raster refresh still executes. Deduplicating cells, refreshing only
nonzero contributions, or rebuilding coverage from the final set of deployed
actors would change the original mutation and RNG sequence.

`44A5C0` reads the resulting coverage nibble. A zero nibble selects hidden
index32 without RNG. Every nonzero nibble consumes one shared `457930` draw
before the terrain color and grid-overlay suffix; grid lines may then replace
the pixel with index33. Thus adding and removing coverage both consume RNG,
including zero-contribution cells that remain covered by another source.
The material sample uses the raw-coordinate `+0x80` rounding rule, while
object/layer selection and coverage use the unrounded input cell.

Static initial coverage is a separate `44AB20` pass over undamaged/uninfected
kind8 terrain objects, followed by raster construction. `42E570 -> 44AFB0`
places initialization after authored constructors and their terrain mutations.
`44AFB0` stores the current dimensions, palette base and underwater policy,
then calls `44AB20(0)` to build coverage and raster. This precedes campaign
cargo restoration: `451710` calls `42E570` at `004517AB`, then `451C00` at
`004517C9`. Cargo constructors therefore consume their RNG after the initial
raster draws. [CAMPAIGN_CARGO.md](CAMPAIGN_CARGO.md#world-initialization-order)
owns the surrounding player-first construction sequence.

Fresh ordinary worlds, campaign destinations and native saved-game loads all
reconstruct through this order. Begin Intro uses the same post-constructor
radar phase with its separate skipped-player reservation. Returning from pause
or the M map retains the existing coverage/raster allocation and consumes no
initialization RNG. World replacement discards that allocation; task activation
and destruction within a world must mutate it in place.

### Presentation and process RNG

Both HUD `494A0` and fullscreen nearest/bilinear map `4C1F0`/`4BCF0` read the
same raster at `004DF118`. Scaling and projection consume no random words.
Marker flicker is a separate, source-owned consumer of the same process RNG
`457930`, whose state is `004F7308`; it is not a private presentation stream.

- HUD `44AFF0` first rejects zero/dying state unless capability `0x10` is
  present. An uncovered non-Type46 actor then consumes one draw and survives
  only when `(word & 3) == 0`. The capability `0x8EBD` visibility test follows
  that draw, so actors rejected by this later test still advance the stream.
- Fullscreen `44C430` first requires a nonzero map-icon selector and nonzero,
  non-dying, non-attached state. It then applies the same uncovered/non-Type46
  draw. `44C340` traverses markers only during a nonzero blink phase; map setup
  and the blank blink phase consume no marker RNG.

Retail has one exceptional reconstruction path: `494A0` compares the current
HUD width/height scalars with the values retained by `44AFB0`. A mismatch calls
`44AFB0` again and rebuilds static coverage/raster through the process RNG.
This display-tier-change behavior is distinct from ordinary redraw or resume.
The port's shared owner implements the loaded tier; live retail resolution
changes and their exceptional rebuild remain outside this checkpoint.

## Carry, release, hit, death, and abort

| Entry | Required sequence and boundary |
|---|---|
| Attach `416700` | Set state `0x1000`, store parent `+80`, then type callback `40DBF0 -> style +08 -> 40C690 -> 40AC60 -> 40C560` |
| Release `416750` | Clear `0x20001000`, set `0x800`, reapply type flags through `40D3C0`, clear parent, then `40DC50 -> style +0C -> 40D1C0` |
| Release callback `40D1C0` | First `416AC0(entity)`, then singleton `40AC60 -> 40C560` |
| Nonlethal hit | Common checked/generic damage owners retain timestamp, accepted damage and reaction ordering. Class60 has null primary/infected callbacks, no hit sounds, no capability8 hit-particle suffix, and zero low-health effect inputs |
| Standard death `410C10` | Remote returns before mutation; otherwise first death zeros health, sets dying `0x4000`, requests sound62, stops any attached sound, and enters common type death dispatch `40DB80 -> 40AC60`, selecting alternate class49 |
| Class49 `40BD20` | Run `40BAF0` scatter and radial first; then `40A860` task destruction; then construct Type60 at the current position unless remote; finally `410B70` deferred destruction |
| Main Base abort `4170A0` | Capability `0x1004 & 0x11 == 0` selects generic death. Type111 and state `0x10001000` exclusions apply before `410C10`, so carried/protected Type83 is skipped |

Relation and terrain-basis routines are in game_logic.c;
`410C10` and deferred destruction are in fgdk_engine.c.
Cargo membership, notifications, and success presentation surround the relation
callback as specified by [CAMPAIGN_CARGO.md](CAMPAIGN_CARGO.md).

`416AC0` samples terrain gradients and writes the full Q31 body basis, setting
state `0x28`; it does not write entity position. C560 then zeros Euler but does
not rebuild that basis. Preserve this release-versus-birth asymmetry and the
actual current position supplied by the cargo/drop owner.

The Type83 BAF0 scatter branch is **10 class16 particles**. At `0040BB3E`
it has no A/B/N/G; capability `0x40` is clear at `0040BB73`; type83 is neither
49 nor112..115, so `0040BB9C` chooses class16. It therefore differs from both
the rover's class37 branch and the turret's 94/95 branch. The shared `440950`
scatter is followed by the type-header radial template copied at `0040BC65`:
inner radius`0x200`, outer radius`0x400`, impulse2000, channels`[1, 3]`,
amounts`[4000, 4000]`, and initial trailing words`[0, 0]`; BAF0 then fills
the source/owner fields before `4566E0`.

Coverage removal belongs to Primary destruction, **not** class60's null
death-cleanup callback. BD20 calls BAF0 at `0040BD35`, A860 at `0040BD3E`,
and the Type60 constructor at `0040BD92`. Radar coverage must therefore remain
present through scatter and radial delivery, and be removed between the
completed radial pass and Type60 construction. Earlier removal changes both
radar observations and shared RNG ordering.

World teardown is a second destructor entry, without class49's death effects.
`42E980` traverses the retail actor list and invokes `42EA00`, which requests
`10B70` deferred removal for every type except 0 and 1. It then runs `14990`'s
ordered removal sweep. Each `149F0` first calls `A620`, which calls `A860`
before freeing actor components. A deployed Type83 consequently removes its
coverage and consumes the corresponding raster RNG while the old world is
still resident. Discarding the raster and actor allocations together does not
replace this observable RNG sequence. Teardown must precede resource unloading
and the next world's constructor/raster RNG. Ordinary pause and map close do
not enter this path.

## Port integration boundary

The current [class49 owner](../../crates/v2k-game/src/class49_death.rs)
authenticates GunTurret and CleansingVehicle profiles only. Its terminal
phase clears task slots without a Type83 resource/RNG-aware destructor.
Adding a Type83 profile or accepting a matching public type alone would not
establish valid allocation, component, or coverage custody.

The shared raster foundation is implemented. [ResourceCache](../../crates/v2k-game/src/resource_cache.rs)
owns `level_terrain_radar`, retains it through abort rollback, and supports
ordered cell refresh. [main.rs](../../crates/v2k-game/src/main.rs) initializes
ordinary gameplay and Intro2 at the post-constructor phase above, before cargo
restoration. Repeated initialization of an already resident world preserves
the allocation and consumes no RNG.

[GameplayRadar](../../crates/v2k-game/src/gameplay_radar.rs) reads this owner
for both HUD and fullscreen presentation. It constructs no second coverage map
or terrain raster and uses caller-supplied process RNG for marker flicker.
Fullscreen images follow current raster changes without rerunning static
coverage construction or its random draws. The shared terrain-light and crater
refresh paths now reach HUD/map output; this does not authorize Type83 deployment.

The shared owner also implements ordered `44A8D0` coverage mutation, including
whole-byte carries/borrows, wrapped coordinates, zero-contribution cell refresh
and immediate raster RNG. [portable_radar.rs](../../crates/v2k-game/src/portable_radar.rs)
implements detached `405610` and `4055C0` callbacks against that owner. These
borrow the caller's SubK word and current position; they do not install a task,
authenticate an entity allocation, run the scheduler, or dispatch cargo/death
callbacks. Partial deployment and partial destruction require no radar resource.

The detached callback preserves its source prefix on a resource error:
deployment writes K1=`FFFF` before attempting coverage, whereas destruction
zeros K1 only after successful removal. A future native owner must park a failed
deployment prefix and retain a task whose destructor failed; the saturated word
alone cannot prove that coverage was successfully installed. These adapter
errors are port boundaries, not additional retail return branches.

A coherent native Type83 checkpoint still needs all of the following:

- Authenticate the invariant header, actual allocation, SubK storage and
  wrapper/slot custody; implement construction prefixes and allocation failure.
- Execute deployment through detailed/coarse task scheduling with the shared
  world RNG and persistent radar state, routing activation to the implemented
  coverage operation without reconstructing static coverage.
- Run the destructor on successful replacement, cargo attachment/release,
  initializer-failure fallback, class49 post-radial task clearing, and world
  teardown before resource unloading. Suspended callbacks and abort rollback
  need the same task/resource transaction guarantees.
- Extend checked/radial hits, class49 scatter/terminal ownership, Main Base
  abort, authored births, campaign match/miss, and cargo drop as one lifecycle.

Validation must distinguish partial deployment, first saturation, repeated
deployed ticks, overlapping footprints, byte carry/borrow, world wrapping,
zero-contribution refresh RNG, successful replacement versus allocation failure,
direct allocation failure versus outer fallback, carry/release, post-radial
destruction ordering, and teardown RNG before the next world. Then inspect
ordinary-world HUD/map behavior with deployed Type83 coverage. Shared raster integration and
static routine recovery do not close those native lifecycle conditions.
