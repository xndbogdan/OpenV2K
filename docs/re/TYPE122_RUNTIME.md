# Type122 native actor and capture dependencies

Type122 has a native A/B/C/D/E/H/J constructor and allocation receipt across all ten authored births. Its shared ground executor, emitter, contact and lifecycle adapters retain Type122 data; its capture relation shares source-proven callbacks with Type17. Ordinary Capture admits all 27 native Type8/9/90/116/123/86 children through their actual constructors; no ordinary candidate still lacks a native living/Carried owner. This authority records source evidence and current implementation boundaries; no new retail capture was needed for this recovery.

Type122 has the complete **A/B/C/D/E/H/J** topology. E is a real projectile emitter and J is a real one-slot capture attachment component. Type53 has A/B/C/D/H/J, omitting E. Shared constructor and task primitives must preserve those independently authenticated components and the actual Type122 model/body.

## Evidence and reproduction

- Canonical current release `v2k_game` / `v2k_formats` parsers loaded PRELOAD, system overlay3 tier1, then each tier1 world13..50. No historical Python extractor was used as format authority.
- `.tmp/type122-corpus-probe.exe` prints the authored birth census, exact invariant 0x128-byte header, Type53/Type122 metadata, and model extents/collision radii. Output retained in `.tmp/type122-corpus-probe-output.txt`.
- `.tmp/type122-model-probe.exe` separately compares the **complete `EntityTypeRuntimeMetadata`** over worlds13..50 and prints model274 links/geometry/cues in every authored world. Output retained in `.tmp/type122-model-probe-output.txt`. Result: `FULL_METADATA_VARIANTS=1`.
- Those ignored probe binaries were compiled from Rust supplied on stdin, against the canonical current release rlib. They do not mutate original overlays.
- Source: `bulk/ovl_handlers.c` (`410090`), `bulk/game_logic.c` (`104B0`, `09A80`, `D920`, `D9B0`, `12CF0`, `A8B0`), and read-only `objdump` of a local no-CD retail executable for callbacks omitted by the bulk decompiler.
- Related authorities: [Type53](INTRO2_TYPE53.md), [Type17](INTRO2_TYPE17.md), [Type57](INTRO2_TYPE57.md), [actor runtime](ACTOR_RUNTIME.md), [damage/death](ENTITY_DAMAGE_AND_DEATH.md), [static damage programs](ENTITY_STATIC_DAMAGE_PROGRAMS.md), and [cross-level runtime](CROSS_LEVEL_GAMEPLAY_RUNTIME.md).

The source handler at `410090` explicitly installs common type vtable `4C8A30` into runtime type+7C, `381F0` into type+B8, and `382B0` into type+BC. The on-disk header+7C is zero, not a callback identity. The handler also overwrites the on-disk leading `CC` allocation-size word with runtime `D4`; do not authenticate the raw leading word as a special Type122 constructor selector.

## Ordinary corpus and Intro2

There are **nine ordinary Type122 births** across worlds13..49, plus Intro2 spawn21 (world50). The complete type metadata is identical across all 38 loaded worlds, including worlds with no birth.

| World | Authored spawn | Position raw signed16 | Euler heading/pitch/roll words |
| --- | --- | --- | --- |
| 24 | 3 | `[19456,0,16640]` | `[20024,0,0]` |
| 24 | 14 | `[17920,0,21504]` | `[16384,0,0]` |
| 24 | 15 | `[15360,0,25600]` | `[23665,0,0]` |
| 24 | 32 | `[12544,0,29184]` | `[20024,0,0]` |
| 24 | 34 | `[15616,0,-9216]` | `[40049,0,0]` |
| 42 | 36 | `[7936,0,512]` | `[0,0,0]` |
| 46 | 31 | `[21760,0,3072]` | `[0,0,0]` |
| 46 | 32 | `[19968,0,4352]` | `[29127,0,0]` |
| 49 | 11 | `[3840,0,-22272]` | `[59528,0,0]` |
| 50 / Intro2 | 21 | `[-29952,0,-29184]` | `[52792,0,0]` |

All ten have authored constructor parameter1, initial damage buffer0, model overrides `[0;4]`, no per-spawn animation, and no configuration payload. Parameter1 and damage buffer0 are separate spawn fields: `104B0` therefore sets its `0x01000000` parameter bit for every one of these births. The Euler words above are raw u16 representations; preserve the exact bit pattern when using signed angle words. Spawn index is authored identity, not a whitelist or capture-derived allocation receipt. Intro2 spawn21 currently corresponds to host entity22; ordinary persistent-player allocation and generation still belong to the loader.

## Authored metadata

| Domain | Type122 |
| --- | --- |
| Model slots | `[274;4]`, model `iceant` |
| Mass / capability / health | `100 / 8 / 7000` |
| Default C8 policy | `0x439` |
| Common axis | strict range `0x1200` (4608), filter word `0xC05` |
| Damage thresholds | `[0,4000,3000,0,200,0,0]` |
| Damage Q8 multipliers | `[0,256,256,512,128,0,0]` |
| World effects | surface selectors `[1,0]`, lifetime2000ms, low-health words `[0;3]` |
| Terrain task lifetime | 0 |
| Accepted hit / death cues | `92 / 75` |
| Surface/static cues +86/+88/+8A | `0 / 0 / 0` |
| Constructor, infected-model, generic-hit, target-warning cues | absent |
| Search prelude/Aim and RunAway optional cues | absent; sound periods0 |
| Model variables / actor animation / status / SubN | zero / absent / absent / absent |
| Topology | **A/B/C/D/E/H/J**; F/G/I/K/L/M/N/O absent |

Component descriptors:

- A: acceleration3000, overspeed correction-3000, signed base speed460.
- B: projection threshold rate10000, correction rate1000.
- C: base clearance50, lift range75, strength`0x300000`, near boost100, damping200, surface mode0 (terrain), offset sample0, reserved words0.
- D: divisor64, yaw-to-roll0, pitch enable0, forward probe512, lateral probe256, classifier flags19 (`0x13`), reserved0.
- E: projectile method24, random interval400000us, spread100, aim threshold12000, speed override0, target axis tolerance2560, sound70, raw+12 word150, alternate emitter0, stochastic mode0, auxiliary command0, variable bindings `[0;4]`.
- H: six records, no completion cue. Every record uses resolver flags`0x20000000`, phase rate`0x30000000`, axis mode0. Vertex refs and dependencies are below.
- J: one authored slot, policy1, local offset `[0,0,120]`, reserved0.

| H record | Vertex refs | Dependencies |
| --- | --- | --- |
| 0 | `[86,88,94]` | `[1,2,3,4]` |
| 1 | `[87,89,95]` | `[0,2,3,5]` |
| 2 | `[148,90,96]` | `[0,1,4,5]` |
| 3 | `[149,91,97]` | `[0,1,4,5]` |
| 4 | `[2,92,98]` | `[0,2,3,5]` |
| 5 | `[3,93,99]` | `[1,2,3,4]` |

The choice list is identical in structure to Type53:

| Rule | Multiplier | Class |
| --- | --- | --- |
| Always1 | 1 | Follow Beacons33 |
| Under Attack2 | 20 | Search7 |
| People Nearby10 | 4 | Capture9 |
| Player Nearby6 | 5 | Search7 |

Rule reference1; alternate class12. Constructor +34 is freshly zero, so the under-attack choice does not become true merely because birth occurs at a late retail tick. Nearby predicates use Type122's own range4608 and actual preceding live allocations; they must not use Type53's3840 or a fixed captured selection.

## Model/body provenance

All authored worlds agree on model274 `iceant`, intrinsic child links `[275,275]`, model slot_count152, header+08 extent260, and **header+0A collision radius250**. These two radii are distinct; model+0A is decimal offset10, not hexadecimal+10. Type53/model302 has extent182 and collision radius140.

The exact collision program is retained in `type122-model-probe-output.txt`. The six H references include vertices148/149 and therefore require model274's full live shape, not a Type53 body receipt. The generic model-tree/H submission primitives can be reused after authenticating the actual model274 hierarchy. The complete canonical child275 rendering structure is covered below; it needs no separate entity-level animation or component owner.

## Constructor and runtime receipt

The shared source path is `104B0 -> 09A80 -> common vtable+00 D4A0 -> 381F0/425680` followed by the selected behavior producer and final body basis. `410090` establishes the common vtable; there is no evidence for a separate Type122 C++-style constructor or special whitelist.

`intro2_type53/construction.rs` is a useful phase-order reference, not an admission alias:

1. Admit the actual loader-issued allocation lease/generation and authored row. Retain authored index, model slots, full Euler words, current resource-domain ownership, actual initial damage buffer, empty task graph, and authentic component storage.
2. Use the shared native `09A80` component allocation order. Existing `entity.rs` already allocates D storage for descriptor-bearing actors, including unresolved Type122; consume that actual `NativeSubDConstruction`, its seed and frame-owner cache. Do not draw a replacement stagger word in a Type122 publisher.
3. Keep `104B0` base state `0x06078801 | (param != 0 ? 0x01000000 : 0)`, actual pre-D4A0 authored-Y sea/wave classification, terrain model-slot bit, last-hit+34=0, and exact C8=`0x439` policy translation. The +44 damage modifier is null for this type under `104B0`'s explicit type switch.
4. D4A0 copies C8 before selection. Bit20 is set, bit40 clear: with terrain, ground Y through445860 and add C clearance50; no model-extent add. Copy the resulting immutable+90 anchor before nearby selection. With absent terrain the source skips the placement branch; a terrain-requiring authored publisher may explicitly retain that unsupported boundary.
5. H/D storage precedes the `20450` SubA RNG word, and the selector follows A. Evaluate choices against the actual preceding successful intrusive allocation prefix, including a persistent player when present.
6. Publish the actual selected B6C0 or B740 acquiring graph, and apply each successfully allocated task's06070 suffix in source order. Initializer failure/fallback must preserve its actual committed prefix and RNG use.
7. Build the final body basis after the initializer. Own the model274/H external-frame body and mutable D runtime/cache; do not substitute a captured pose, seed, or nominal Type53 graph.

The native Type122 receipt retains allocation lease/generation, entity identity, authored index, model slots, immutable anchor, D runtime/frame owner, and authenticated private component ownership. Its distinct native E/Aim owner retains a persistent presentation FIFO. Live dispatch must authenticate the issuing manager, current behavior context, and all three task identities. Re-entry refreshes the lease; failures with committed prefixes must not replay time, target changes or RNG.

## Shared live graph and proven primitives

| Phase | Source path | Existing primitive / distinction |
| --- | --- | --- |
| Weighted birth and reselection | `381F0 -> 425680`, `C690`, `B6C0/B740` | Type53 choice logic is the same shape, but use Type122 range, A base and E/J body |
| Follow acquiring/following | `B740`, `AFD0`, shared Follow task | `follow_beacons::live_primary`; actual Primary/Secondary slot order and callback unwind remain required |
| Search acquisition | `C7D0/C6B0 -> ADE0` | Publish style before initializer; Tertiary Aim, clear Secondary, then Primary Chase |
| Chase movement | `403490 -> 01430 -> 423030` | `chase_target::live_primary` and `intro2_common_mover`; own controller and D cache |
| Capture acquisition/pursuit | `C7D0/C6B0 -> AEE0/AF50 -> 403650/403780` | `intro2_capture_pursuit` owns acquisition/pursuit; `native_actor_capture` owns attachment, carrying and cleanup |
| Aim | `02300 -> 24650` | `intro2_native_ballistic_aim` / `generic_projectile_emitter`; Type122 has a real emitter |
| World scheduling | `12DA0 -> DCA0/E870` | `native_ground_actor` binds detailed/coarse dispatch, sound gate, body basis, gravity/drag, surface timer and motion to the Type122 profile |
| Visible H body | `D360` during submission | Full visible native model274/H provenance, following existing whole-hierarchy/view admission ordering |

`intro2_common_mover.rs` already admits the exact structural `ABCDEHJ_TOPOLOGY`; it is not limited to the Type53 ABCDHJ branch. Shared `01430` reads the actual A/B/C/D/H metadata. Persistent D rollback/committed-prefix semantics must remain those of the owning actor.

Search/Capture constructor signed `base*4/3` gives **613** for Type122 (460*4/3 truncation), not Type53's533. Chase's H/A near-target controller must restore460 or write1 according to the actual wrapped thresholds; it must not retain Type53's400.

### Real E emitter, not the Type53 absent-emitter task

Projectile method24 selects executable table row `{flags0, speed2000, particle68, entity0}` at `4D02C0 + 24*0x10 = 4D0440`. Its canonical row is already in `projectile_emitter.rs`.

Type57 already uses method24/class68 through the generic native ballistic transaction, but has sound93 and a different descriptor. The Type122 profile uses interval400000, spread100, threshold12000, tolerance2560 and sound70. Reusing Type53's non-emitting Aim would be false acceptance.

WorldFx already owns class68's `442950` static-route sweep, descriptor-selected `CLASS68_STATIC_ROUTE_DAMAGE_PACKET`, `441180` entity delivery, and the `42E8E0` null solid-static callback that consumes the parent. Existing combat-sweep tests cover classes52/68/85. This does not require recovering a new particle class.

Retain the real2300/24650 RNG/cadence and queued1EFD0 transaction, with11400 presentation drain at the shared draw boundary. Pending commands must survive C690/Class12 while the original allocation remains valid and must not leak to a replacement allocation.

## Steady wind and drag

Ordinary world46 exercises `44EC60` through E100's effective drag bit8.
`common_mover/environment.rs` implements shared mode0 drag and mode1 steady
wind for Type53/122, Class12 and native people/workers. Mode2 consumes the
current vector retained by the frame-owned44EBD0 gust phase; individual actor
callbacks do not advance it.

`44EB40` initializes the Section13 mode at+90, strength+94, maximum sheltered
height+98 and vector dwords+9C/+A0/+A4 (each consumed as a signed16 word).
A zero vector disables wind regardless of the authored mode. The sea-side
gate uses `42ED00`/Section13+80 and the signed comparison `sea < actorY`;
it does not use the terrain's water-rendering flag.

For mode1 on the admitted sea side, sample terrain at the wrapped X/Z
positions minus half the wind vector and minus the full vector. The half
is a **signed16 arithmetic shift**: `44ECD4` is `sar ax,1` for Z, and
`44ECE0` is `sar cx,1` for X. Let
`h = min(actorY - max(half_sample, full_sample), authored_max_height)`.
Negative h returns before angular and linear drag. Otherwise each relative
velocity word is the wrapped signed16 result of
`(wind_axis * (h << 5) >> 15) - velocity_axis`.

Each angular dot product sums three separately shifted signed Q31 products.
The lateral basis at entity+0C/+10/+14 produces
`roll += q31(dt, -(lateral_dot << 14))`; the forward basis at+24/+28/+2C
produces `pitch += q31(dt, forward_dot << 12)`. Signed IMUL and the
SHL/RCL high-word extraction are visible at `44EEFF`, `44EF1D`, `44EF38`
and `44EF72`, `44EF8A`, `44EFA1`; stores target+A6 at`44EF5F` and+A4
at`44EFC3`. Angle writes wrap16 and do not rebuild the body basis again:
the later18640 attachment phase uses the preceding F70 basis.

The linear factor is the wrapped unsigned32 product `dt * strength`, divided
by `(callback_mass << 3)`. With admitted wind, add the signed product of this
factor and the relative word, shifted15. Without wind/on the other sea side,
subtract the signed product of factor and existing velocity, shifted15.
Negating velocity before multiplying changes the source's rounding and the
`-32768` case. Preserve the actual nonzero callback mass and both signedness
boundaries.

## Static, surface and pair ownership

**The important Type122 difference is generic crushing.** `game_logic.c` atD920 computes `(entity+C8 | style+34) & ~style+38`, then invokesD9B0 when bit`0x400` remains, before calling style+1C. Type122 default`0x439` retains400 in its normal Follow/Search/Capture acquiring/pursuit policies and in Class12 (`0x439 & ~0x2015 = 0x428`). Type53/58's`0x39` never takes this branch.

D9B0 constructs the exact two-channel packet:

```text
channels = [1, 0]
amounts  = [40000, 0]
source   = -5
owner    = 0
```

It calls27950 with signed/wrapped contact cell words at contact+2E/+30. Preserve the source packet, actor liveness recheck, and ordering. The separate Type58 furniture hookC890 also uses40000 but includes its own C690 policy; it is not a generic D9B0 replacement.

The outer static sequence in12CF0 is: optional raw+8A cue (zero forType122),27E20, A8B0 task walk, then type-vtable+34 D920. A8B0 visits the current P/S/T wrappers through01270 before the type hook. D920 then performs optional generic crush, style+1C, survivor lookup, and11760. `native_ground_actor/contact.rs` binds the exact D9B0 packet to `static_damage_live` and the static scheduler before the survivor/11760 continuation.

All Type122 contact selectors+86/+88/+8A are zero in this corpus. That resolves optional cue absence only; it does not omit solid/water callbacks or their movement/damage/state effects. Common vtable+0C/+10 are D7F0/D860, and+38 is D8D0 for pair behavior. A Type122 surface binding must preserve actual model274 collision radius250, mass100, raw C8 policy and per-style effective flags. Class12's+10/+14/+1C hooks are null; generic crush remains at the outer type layer.

Before native Type122 construction, Type17 pair handling reported `UnresolvedBehavior` against that unconstructed body. Replacing that with a null context would then either fail body custody or silently hide tasks and their callbacks. Pair admission requires native mover/body custody and an authenticated current task graph, plus existing reverse-order counterpart behavior; corpus presence does not establish that all possible pair families are handled.

World46's four Type35 records expose a separate representation boundary:
their authored model words are0, encoded as `None` by `Entity`. PRELOAD model0
is the actual `nothing` record with extent/radius0. Retail11AD0/12530 indexes
that record and takes the normal zero-radius rejection before callbacks.
`active_pair_body_from_entity` now resolves that raw0 through the current
model pool; it must still reject a missing pool0 or a missing nonzero model.
This requires neither a Type35 constructor nor a null behavior whitelist.

## Hit, death, abort, capture relation and release

The shared `4C8A30` table was read from the retail PE:

| Offset | Callback |
| --- | --- |
| +00 | D4A0 constructor |
| +04 | E9E0 |
| +08 | DB80 |
| +0C / +10 | D7F0 / D860 surface callbacks |
| +14 / +18 / +1C | DAC0 / DA00 / DA60 damage family |
| +20 | null |
| +24 / +28 / +2C | DCA0 / E870 / DB20 |
| +30 | null generic-hit callback |
| +34 / +38 | D920 static / D8D0 pair |
| +3C / +40 | D760 / D7A0 wrapper callbacks |
| +44 / +48 | DBF0 attach / DC50 release dispatch |

Use common primary10EB0/infected11250 presentation and DAC0/DA00 damage primitives with Type122's own thresholds7000-health profile, hit cue92 and death cue75. The constructor+44 modifier and common vtable+30 are null; this is source evidence, not permission to bypass style callbacks. The shared7/9/33 impact policies authenticate the actual Type122 graph; carrying keeps the primary/infected asymmetry recorded below.

Common class12 death admits Type122 through its own receipt and preserves C8=439. `intro2_common_dying/world.rs` already deliberately ignores400 for E640/DF70's admission check (`default & ~0x2415 ==0x28`, including Type26's400 case). This arithmetic intentionally ignores the independent generic-crush400 policy. Type122 static contact retains that policy separately: Class12 effective flags are428, not28.

Capture is divided at its actual owners: `intro2_capture_pursuit` handles
variants0/1, and `native_actor_capture` owns C910, captured rows, variants2..5,
release/delivery and nested D040 cleanup. The shared callback bodies authenticate
Type17 or Type122 separately. Type53 remains pursuit-only. Ordinary children now own their native
constructors and callbacks (90/116/123/86); a matching J attachment
descriptor never substituted for those child owners.

## Current implementation and remaining boundaries

| Owner | Implemented responsibility |
| --- | --- |
| [Native Type122](../../crates/v2k-game/src/native_type122.rs) | Exact authored metadata, loader-issued generation and process Sub-D receipt, grounded anchor, weighted constructor graph, real method24 E/Aim and model274/H ownership |
| [Shared ground actor](../../crates/v2k-game/src/native_ground_actor.rs) | Separately authenticated53/122 Follow/Search/Capture task phases, current-slot dispatch and movement, body rebuild, steady-wind/drag and master motion; Type53 has no E and remains pursuit-only |
| [Shared captor relation](../../crates/v2k-game/src/native_actor_capture.rs) | Own captor/child allocation leases, actual Sub-J row/backlink, synchronous native8/9 attach/release/death, D040/CF90/delivery, Class9 carrying tasks and both directional pair callbacks/A900 walks |
| [Static contact](../../crates/v2k-game/src/native_ground_actor/contact.rs) | Authentic current task/static callback, D9B0 before11760, retained contact plane, Type122 living/carry439 and Class12-428 policy, and relation authentication for carrying |
| Shared native surface, impact, Class12, radial and abort adapters | Type122's actual model, radius/mass, cues, default/effective flags, child cleanup and scheduler ownership; late failures retain committed prefixes and prevent replay |
| Frontend activation and presentation | Native Type122 construction precedes its authored operation2 activation; live model274/H replaces the old synthetic pose fallback |

The relation owner permits the legitimate empty row left by callback-free18640
compaction. A stale child allocation/backlink or missing relation receipt is an
error before a carrying static callback changes motion, tasks, RNG or terrain.
Pending child custody during death blocks D040 after the reached health/death
prefix; the scheduler parks that exact parent owner rather than replaying it.

These owners do not close the shared-runtime objective. Native
[workers8/79/90/91/116](INTRO2_TYPE8.md), [people78/86/95](TYPE86_RUNTIME.md)
and [Type123 people](TYPE123_RUNTIME.md) retain their own profiles and
constructor/task custody through capture, including terminal Class14
reattachment. The [cross-level authority](CROSS_LEVEL_GAMEPLAY_RUNTIME.md)
owns the full authored census; presence in a world does not prove a particular
capture, contact or selection event. [Type7](TYPE7_RUNTIME.md) still needs its
own four-choice worker graph and complete lifecycle.
The extended world46 control exposed the missing fresh Type97 owner at
entity19/authored spawn18. The [ordinary turret owner](INTRO2_TYPE102.md#ordinary-native-type97)
now publishes that body's real Class29/E/L allocation and authenticates its
proven null pair/Tertiary callbacks. Playing lethal pair hits now complete
through Class49 with the real world/player context; cinematic pairs without
that context retain the explicit boundary. Campaign cargo reconstruction does
not substitute for fresh authored task custody.

Outgoing projectile combat has a separate target-admission boundary.
`intro2_effects::deliver_entity_impact` dispatches particle52/68/85 to the
411180 owner before the generic DAC0/DA00 chain. That owner currently admits
Type13 targets and explicitly blocks other target families. The shared ground
hit adapter likewise rejects52/68/85 before any primary-hit timestamp, task,
health or RNG mutation in ordinary gameplay; it cannot relabel411180 as DAC0.
Type122's
method24/class68 emission and static route therefore do not certify complete
combat delivery to arbitrary entities. This is independent of D9B0 generic
crushing and of capture transport.

The original boundary was visible before this constructor checkpoint: the
Type58 contact smoke recorded11 `UnresolvedBehavior` pair failures against
Intro2 entity22/type122 at ticks3804..3864, before any prefix committed. That
was the missing Type122 owner; the contact log alone does not establish why
preceding work exercised the path. Source, constructor tests and current
runtime ownership replace the old scene-specific fallback as the authority.

## Capture counterpart census and child model closure

The read-only capability probe (`.tmp/type122-capture-probe.exe`, output `.tmp/type122-capture-probe-output.txt`) loads canonical tier1 worlds24/42/46/49/50 and enumerates authored capability masks, not runtime contact events. **Ordinary Type122 worlds do not supply native Type8/9 people.** Their eligible C910 children are:

| World | Authored capability0C00 people | Capability10 destinations | Capability100 beacons |
| --- | --- | --- | --- |
| 24 | Type86 spawns0,1,2,17,18,19,20,21,22; Type90 spawn23 | Hive67 spawns30,31 | Type52 spawns4..12,40 |
| 42 | Type116 spawns29,30,31 | Hive67 spawns4,5 | none |
| 46 | Type116 spawns24,25,27 | Hive67 spawns0,1 | none |
| 49 | Type123 spawns0..10 | none | Type52 spawns12..45 |
| Intro2/50 | Type8 spawns12,13,15; Type9 spawns2,3,9,11,16,17,18,19,49,50,58,59,60 | Hive67 spawn24 | Type52 spawns14,22,23,27,28,29,32,37,39,47,48,57 |

`specialized_actor_task_production/capture.rs::actual_child` now authenticates native8/9/90/116/123/86 through their actual constructors/current-task and attach/release/death owners. Its `CaptureTaskCustody` remains captor-independent. Presence is not proof of actual capture timing or that every listed candidate reaches the strict route predicate.

The prior uncertainty about child model275's intrinsic decoding is now bounded: canonical275 `icestagclaw` has flags0C, slots24, extent84, collision radius0, no collision byte program, no child instances, no view commands, no ribbons/billboards, and record types0/13. Its ordinary palette/texture face commands are enclosed by66/E6 painter grouping, supported directly by the canonical materializer. Parent274 uses two op0E instances at slots150/151, authored mirrored mount transforms, and command-produced registers1=8192 and2=32768; these are not new entity-level variables or a SubH child owner. Both274 and275 data are invariant across the five probed worlds. Native submission must retain their current dynamic materialization and parentH/body transform; no separate child-specific renderer is indicated.

The Class9 table was independently reread from the retail PE at4C7FF0..4C81A0. C910 appears only in variant1's pair slot; D0B0 appears only in variant2's pair slot. Carry3/4/5 pair slots are null. All carry2..5 have+20 null, +24/+28/+2C D040, and+34/+38 zero. Root/completion pairs are2 CF90/null;3 C7B0/C7D0;4 CF90/C790;5 CF90/CF90. Consequently Type122 preserves its default400 generic-crush policy throughout carrying too.

The sealed `NativeCaptorProfile` enum (Type17, Type122) selects each profile's own manager allocation/metadata authentication and living C690 reselector; it never substitutes a Type17 identity. A common relation stores captor and child allocation leases plus row-presence state. Shared C910/CAD0/443D10/D040/CF90 operations consume actual type metadata and SubJ, while the shared carry task kernel dispatches the actor's real mover. The ground frame explicitly distinguishes Type53's pursuit-only policy from Type122 transport with child scheduler/notification custody. All four ordinary child families (86/90/116/123) now own native constructors and callbacks; see [TYPE86_RUNTIME](TYPE86_RUNTIME.md) for the last one.


## Exact ordinary-child prerequisite matrix

The canonical metadata and initial entity probe retained in
`.tmp/type122-children-probe-output.txt` contains all **27** ordinary people:
9 Type86, 1 Type90, 6 Type116 and 11 Type123. At the audited baseline every
one has unresolved initial behavior and current context, and no Primary task.
An allocated basis or Sub-I controller does not establish its native task graph.

All four have A/B/D/I topology, mass10, one model variable, default policy
`0x2F`, behavior rule1, alternate class14, and no C/E/F/G/H/J/K/L/M/N/O.
All use A `(1500,-3000,250)` and B `(10000,1000)`. D has yaw/roll0,
pitch0, probes128/64, classifier flags23 (`0x17`) and reserved0; its divisor
is a meaningful difference below. I has model-variable binding1 and four
frames per direction. The cue triple below is
`[capability_bit_3, capability_mask_0x201, attention_stop]`.

| Type | Model slots/name | Capability / health | Axis range/filter | D divisor, decimal | I cue triple | Ordered `(rule,multiplier,class)` choices |
| --- | --- | --- | --- | --- | --- | --- |
| 86 | `[889;4]` | `0x1804 / 1500` | `1536 / 0x04` | 32 (`0x20`) | `[85,0,72]` | `(6,3,45), (7,10,10), (12,200,54), (1,1,6)` |
| 90 | `[890;4]`, `lev3sci2` | `0x1404 / 2000` | `2560 / 0` | 32 (`0x20`) | `[85,0,0]` | `(1,1,6), (13,200,54)` |
| 116 | `[1136;4]`, `vulc2` | `0x1404 / 1500` | `3840 / 0x84` | 20 (`0x14`) | `[0,0,72]` | `(13,100,54), (1,1,6)` |
| 123 | `[889;4]` | `0x1804 / 1500` | `1536 / 0x04` | 32 (`0x20`) | `[0,0,56]` | `(1,3,45), (1,1,6)` |

Choice order is source data, not an interchangeable set. In particular Type90
lists Wander6 before GoToJob54, while Type8/116 list54 before6. Rule IDs also
differ between Type86's54 candidate and the worker's54 candidate.

| Type cohort | Damage thresholds | Q8 multipliers | Accepted-hit / death cue |
| --- | --- | --- | --- |
| 86 / 123 | `[0,2000,400,0,200,0,0]` | `[0,256,256,512,128,0,512]` | `95 / 74` |
| 90 | `[0,2000,200,0,200,0,0]` | `[0,256,256,512,128,0,0]` | `74 / 35` |
| 116 | `[0,2000,200,0,200,0,0]` | `[0,256,256,512,128,0,0]` | absent / absent |

Their common world effects are selectors `[1,0]`, lifetime5000ms and no
low-health emissions; terrain task lifetime0. Constructor attachment,
infected-model, generic-hit, target-warning, Search prelude/Aim and RunAway
optional cues are absent, and optional cue periods are0. Preserve the
nonzero Sub-I capability/attention cues separately from these optional cues.

Type116 was the smallest coherent prerequisite cohort and is now implemented.
The canonical comparison in `.tmp/type122-child-compare-probe-output.txt`
finds the Type8/116 typed metadata differ in model slots559 versus1136 and
Sub-I attention-stop cue0 versus72; their D descriptor is **identical,
divisor20 decimal**. The existing `ORDINARY_TYPE9_SUB_D` name must not lead
to confusing decimal20 with hex20. The shared worker owner below retains each
family's actual allocation, model, task and receipt instead of borrowing the
other family's identity.

The native Type116 owner shares source-proven worker primitives while
retaining its own immutable construction identity and actual descriptors:

| Phase | Existing primitive and required admission |
| --- | --- |
| Authored birth | `104B0/09A80/D4A0/381F0`; loader-issued generation and Sub-D constructor receipt, actual model1136, terrain anchor, A draw, ordered54/6 selector and constructor suffixes |
| Living54 / 6 | Native GoToJob/Wander task publication, controller and immutable/mutable anchor roles; actual Type116 metadata in every task and mover callback |
| Movement and I | `01430` A/B/D/I ordering and actual D cache; I attention cue72 must remain distinct from Type8's0 |
| Hit / alternate14 | Own damage data, null accepted-hit/death cues, C690 and C3A0 graph replacement, actual task retirement and expiry |
| Attach / Carried | DBF0/CD50/CD70 retains current class/context and installs the correct carried style; real I parent binding, state changes and previous-task destruction |
| Release | DC50/CE90 resets I, reenables8000, and AC60 reselects against current world candidates using the child's own choice list |
| Abort / replacement | Actual owned tasks, I/D state and any relation receipt end with the same allocation; no callback or RNG replay on retry |

Type86's selected Class45 graph, Class10 with its BaddieNearby rule-7
evaluator, and its differing Base-Nearby rule-12 candidate now run through
the native Type86 owner ([TYPE86_RUNTIME](TYPE86_RUNTIME.md)) with Type86's
own divisor-32 descriptor, axis, sounds and choice order. No family borrows
another family's admission.

## Capture relation and callback decision matrix

J has two distinct policy fields: both Type17 and Type122 author a **slot
`policy_word_raw=1`**, but the current one-row runtime has
`policy_raw_at_0x0c=0`. Type17's local slot offset is `[0,10,110]`; Type122's
is `[0,0,120]`. Runtime policy0, slot policy1 and the offset must each retain
their own consumer meaning.

| Boundary | Retail behavior and ownership |
| --- | --- |
| C910, Capture9 variant1 pair | Reject invalid/dying/non-capability0C00 child. Full418410 capacity takes10C10 on the contact. Otherwise18440 appends the CAD0/443D10 callback row,16700 attaches the child, conditional gameplay hint8 runs, axis filter becomes10,22C10 searches a destination, and C6B0 enters2 when found or3 when absent. A300 performs the contact effect after that path. |
| 16700 -> common DBF0 | Set child's relation bit1000 and parent+80, then invoke the **current** style+08 attach callback. DBF0 does not choose a generic carried class independent of the current context. |
| CD50 / CE70 -> CD70 | CD50 selects absolute variant1; CE70 selects2. Capability0C00 requires actual Sub-I storage.420760 binds the parent and descriptor-selected cue, clears8000, then C6B0 replaces the actual current graph while preserving its context class/choice/auxiliary values. |
| CAD0 row release | `16750(child,captor)`; synchronous child release callback belongs to the actual child's graph, even while scheduler storage temporarily owns it. |
| 443D10 row destroy | `16750(child,child)` followed by10B70 only when release succeeds. This is distinct from the player's materialiser callback. |
| 16750 -> DC50 / CE90 | Fixed relation release writes precede the current style release callback. CE90 reenables8000, resets I and invokes AC60 using the retained type-default choice list. A failed release must preserve the reached prefix. |
| D040 cleanup |235D0 resets both authored axis words,18500 pops/releases one row; pop error returns before C690. Empty row returns BE5B0 without inventing a child or reselection. |
| CF90 release/kill root | Reset axis, draw one low16 RNG word and test `&3==0`, pop/release one row, dispose error, optionally10C10 the child/sentinel, then C690. The random draw occurs even for an empty row. |
| D0B0 delivery | Reset both axis words. A capability10 contact takes18590 pop/destroy callback; failure returns before C690. A non-destination contact ends after reset. |
| DB80 direct standard death | Mark dying/health0, play actual death cue, invoke current style death callback. Occupied-row D040 may install C620 once; DB80 rereads the changed current context and installs C620 again. Retain both constructor/RNG effects. |
| Primary DAC0 while carrying | Carry style+28 is D040. Cleanup precedes checked damage, with living C690 selection before any later lethal Class12 publication. This differs from infected damage and the occupied-row direct-death path. |
| Infected DA00 while carrying | Carry style+20 is null. A surviving hit retains the row. If checked damage is lethal, DB80 receives the occupied row and runs the nested and reread outer C620 constructors. The authored F780 packet is immune for Type122, as explained below. |
| 18640 carried pose | Runs after the captor world callback and before captor master integration. Callback-free stable compaction drops absent/dying children; receipt retains `row_present=false` until root cleanup. Surviving child velocity becomes0; child position uses captor pre-integration origin plus actual Q31 J offset only when child800 is set. |
| Abort / allocation replacement | Relation uses actual captor and child allocation leases, not IDs alone. Do not deliver release/death callbacks to a reused ID or retain rows past actor ownership teardown. |

The Class9 style table at `4C7FF0`, stride`0x48`, has the following carrying
policies. All variants2..5 have +20 null, +24/+28/+2C D040 and masks+34/+38
zero, so Type122 keeps generic crush400 while carrying.

| Variant / style | Initializer and task order | Root / completion | Pair |
| --- | --- | --- | --- |
| 2 / `4C8080` | AF10 clears Secondary/Tertiary;03650 route with duration0 meaning unlimited | CF90 / null | D0B0 |
| 3 / `4C80C8` | B780 clears Tertiary;02190/021B0 Secondary acquisition, then02B10 Primary500ms | C7B0 ->5 / C7D0 ->4 | null |
| 4 / `4C8110` | AFD0 clears Secondary/Tertiary;03B70 following9000ms | CF90 / C790 ->3 | null |
| 5 / `4C8158` | ACD0 clears Tertiary/Secondary;02B10 retarget5000ms | CF90 / CF90 | null |

Variant0 is B6C0 acquiring, with no pair hook. Variant1 is AEE0 pursuit and
owns C910. Publishing these styles requires the real actor's A/H constructor
effects, D/controller mover and scheduler callback ordering; matching style
addresses alone do not own the selected graph.

The hit-slot mapping is independently visible in the executable: primary
10EB0 reads type-vtable+14 at `410F0B`, whose common entry is DAC0;
DAC0 reads style+28 at `40DADB` and calls it at `40DB07`. Infected11250
reads type-vtable+18 at `4112DA`, whose common entry is DA00; DA00 reads
style+20 at `40DA1B` and calls it at `40DA47`. Do not label style+20 as
the primary hit merely because it precedes the other style hooks.

The actual `43F780` infected packet uses channel6, whose authored Type122 Q8
multiplier is zero. Its damage therefore remains zero even at health1. The
infected-hit regression proves immunity and preservation of the occupied row;
it does not exercise infected lethal damage. The conditional DA00-to-DB80
source law above remains distinct from the separately tested direct standard
death, which releases a real child and runs both C620 constructors.

There is no safe unauthenticated child shortcut through a `None` Primary task.
Class6,45,54 and other carried styles can retain distinct behavior contexts
while their carried initializer publishes an empty task. CD70 first destroys
the actual prior graph, and CE90 later reselects from its actual choice source.
Treating an unresolved child as an empty carried task would hide both callbacks
and the previous graph rather than implement them. Every ordinary child
family now owns its native constructor and callbacks; the-matrix rows below
remain the corpus census, not open work.

## Validation

Type122 controls cover all ten authored births, exact
component/model provenance, real E/Aim and its presentation FIFO, live
scheduling, static crushing, surface/impact/death/abort entry points, and late
radial child-custody failure in Playing, Intro2 and both cursor-storage cases.
The411180 particle guard rejects52/68/85 before primary-hit side effects.
Active-pair controls also prove the actual world46 model0 zero-radius skip
without suppressing missing resources or unresolved nonzero-body counterparts.

The shared-capture tests exercise actual8/9/90/116/123/86 attach/release and
delivery, the real model274 C910 pair, variants2..5, occupied direct death,
stale-relation rejection versus legitimate 18640 empty-row acceptance, and
task-custody gating for all 27 unadopted ordinary children (which blocks
before any row or RNG mutation). These are source and corpus
controls, not evidence that every ordinary world has reached and completed
transport or arbitrary projectile target delivery.

The release OpenGL controls use the normal640x480 tier with fixed40ms/frontend100
and varied-frame/frontend250 entries. They cover New Game/Klaus, uninterrupted
Intro2, final card, first-world handoff, ordinary worlds including24/46, and
native Type122 dry/water Class12 controls. The native Type97 pair regression
separately reaches the original entity19/spawn18 Class29 and Tertiary null
callbacks, requires the actual model274/model173 physical response, and rejects
foreign or unfinished ownership before writes.

Visual review covers ant movement/model274 claws, dry death, water rings,
menu/Klaus and the scene milestones. The current combined run status belongs to
Intro2 acceptance. These controls are
port regression evidence, not a new matched-retail arrival/combat comparison.
