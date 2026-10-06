# Native Type123 people

Ordinary Type123 (model 889, world49/Intro1, eleven births) owns its native
A/B/D/I construction, Always-weighted Class45 Attract Attention lifecycle,
Wander, hits, Class14 death/abort and capture attach/release through
`v2k-game::native_type123`. No Type9 allocation, seed or receipt is borrowed.
All 27 ordinary Type86/90/116/123 children now own native constructors and
Carried callbacks; see [TYPE86_RUNTIME](TYPE86_RUNTIME.md) for the nine
world24 Type86 people that close complete ordinary capture.

## Evidence

- Canonical current release `v2k_game` / `v2k_formats` parsers loaded PRELOAD,
  system overlay3 tier1, then tier1 world49. No historical Python extractor
  was used as format authority.
- Authored corpus: eleven Type123 spawns (indices 0..10), all with param0,
  damage buffer0, model overrides `[0;4]`, no per-spawn animation and no
  configuration payload. World49/Intro1 is wind mode0 (zero Section-13 vector),
  drag3, so its shared E100 environment suffix uses ordinary drag.
- Source: `bulk/ovl_handlers.c` Type123 handler (same
  `104B0 -> 09A80 -> D4A0 -> 381F0` family path as the other A/B/D/I people),
  `bulk/game_logic.c` (`104B0`, `09A80`, `D920`, `D9B0`, `12CF0`, `A8B0`,
  `16700`, `16750`), and the shared Class45 BA40/AF50/C7D0 executor in
  `crates/v2k-game/src/attract_attention.rs` (closed for Type9).
- Related authorities: [Type122 capture children](TYPE122_RUNTIME.md#exact-ordinary-child-prerequisite-matrix),
  [Type8 workers](INTRO2_TYPE8.md), [cross-level runtime](CROSS_LEVEL_GAMEPLAY_RUNTIME.md).

## Authored metadata (differs from Type9 only where listed)

All four ordinary people share A/B/D/I topology, mass10, flags `0x2F`,
behavior rule1, alternate class14, Sub-A `(1500,-3000,250)`, Sub-B
`(10000,1000)`, world effects `[1,0]`/5000ms and no C/E/F/G/H/J/K/L/M/N/O.
Type123's own record:

| Field | Type123 |
|---|---|
| Model slots/name | `[889;4]` (same slots as Type86, different program) |
| Capability / health | `0x1804 / 1500` |
| Axis range/filter | `1536 / 0x04` (`0x600`) |
| D divisor | 32 (`ORDINARY_TYPE90_SUB_D`; rest identical to divisor-20) |
| Sub-I cue triple | `[0,0,56]` (bit-3 0, mask-0x201 0, attention-stop 56) |
| Choices in order | `(1,3,45), (1,1,6)` (Always-only; no nearby witness needed) |
| Damage thresholds / Q8 | `[0,2000,400,0,200,0,0]` / `[0,256,256,512,128,0,512]` (Type9-identical) |
| Accepted-hit / death cue | `95 / 74` |
| Constructor/death sounds | null / `Some(74)` |

The shared `ActorAbdiTopology` admits 123 as its own `Type123Person` kind
against `ORDINARY_TYPE90_SUB_D`, and the ABDI frame threads that descriptor
through the target prelude and `apply_type9_sub_d` instead of the divisor-20
constant. `GoToJobConstructorSuffix` is untouched: Type123 never selects
class54 (its choice list has no rule 12/13 entry).

## Constructor and root selection

`publish_authored_type123` consumes the loader-issued allocation, process
Sub-D construction, authored row, terrain anchor and the 20450 word, then runs
the weighted Always-only selector (one word, cumulative `[3,4]`), the initial
state policy, and the selected BA40/AD10 branch:

- Class45 runs the shared `execute_attract_attention_initial_setup`
  transaction: parity draw, optional candidate Secondary, event-`0x10`
  resource-text receipt (retained in the shared canonical-owner vec for the
  phase-5 load drain), cue Tertiary with positional sound56, Sub-I
  forced-stop, and the 1000ms local-wander Primary. RNG order is parity,
  then one suffix word per successful phase — the same stream positions as
  Type9's BA40.
- Class6 runs the shared Wander setup through `plan_ordinary_type9_wander_setup`.
- `retain_native_type123_runtime` keeps allocation lease/generation, authored
  index, model slots, anchor, Sub-D receipt and the birth graph. Exploding
  graphs are rejected at this boundary (class14 is entered only through death).

`published_graph` authenticates five graph shapes by style address plus exact
slot occupancy: variant0 acquiring (both BA40 parities — odd with candidate
Secondary, even with cleared Secondary), variant1 target-route, Wander,
carried None (variant-1 `4C7A08`/`4C87D0` or variant-2 `4C8740`), and
Exploding (1000ms SharedRetarget). Anything else fails closed.

## Live tasks

The scheduler owner ticks the Primary first, then Secondary, then Tertiary in
retail slot order, stopping after any graph change so a newborn task is never
visited in its publication frame:

- Local-wander runs the ABDI mover under `AttractAttentionForcedStop`
  (BA40's forced-stop byte is set); target-route and Wander run under Neutral.
- Candidate acquisition runs the shared `22C10` first-eligible walk with the
  `0x201` filter override from BA40, then the C7D0/AF50 handoff: store target,
  publish variant+1 style, clear S/T with retirement, install the 5000ms
  target-route Primary with one constructor word. The cue destructor (2AC0)
  clears forced-stop through 20830, which is what makes the target-route
  mover neutral again.
  AF50 also retires the executing Secondary. Its wrapper's unsuccessful
  survival check is expected; the accepted publication still transfers
  scheduler ownership to the new graph, whose first tick is the next visit.
- Cue expiry and local-wander/target-route/Wander transitions reselect the
  Always-only root through `birth::reselect` (one selector word plus the
  selected branch's constructor words). Reselecting Wander retires stale
  S/T auxiliaries first so the cue destructor clears forced-stop.
- Carried ticks the None task plus Sub-I (03250, both modes, never D).
  Carrying keeps effective flags `(0x2F|0x80)&~2 = 0xAD` and skips DF70
  ground snap like the worker owners.

Hits dispatch C690 only for the single-task living graphs (Wander 79C0,
target-route 86F8); the acquiring triple, class14, carrying and cue leaf
never reselect — the same table as Type9's `calls_c690`, where 86B0 is also
a non-reselecting style. Checked damage uses the shared 15040 path with the
Type123 profile (class38 `[500,6000]` filters to 12100, lethal at health
1500); surviving primary hits play cue95; standard death plays cue74, then
C3A0 installs the 1000ms class14 graph with one Sub-A word and speed1.

## Attach, release, abort, radial

- `16700 -> DBF0 -> style+08` attaches living Wander/Attract graphs;
  CD50 selects carrying variant1 for class6, CE70 variant2 for class45. The
  Sub-I attach sound is selected from the parent's capability/position;
  Type123's `[0,0,56]` has zero for both attach cue branches. Carried
  installs the real None Primary with S/T cleared.
- `16750 -> DC50 -> CE90` releases: fixed relation writes, `0x8000` set,
  Sub-I release, then AC60-equivalent full Always-only reselection at commit
  time (after the 18500 pop and anchor update). Class14 keeps its graph
  through release.
- Class14 reattachment is CD50→CD70→C6B0→C470, not a null callback:
 420760 attaches Sub-I,8000 clears, terminal4C7108 publishes, A860 retires
  slots0→1→2, then410B70 stages deferred destruction. No RNG, additional
  death cue or carrying task is emitted. The shared
  [`native_actor_attachment` contract](TYPE86_RUNTIME.md#attach-release-abort-radial)
  returns `DeferredDestroy` to retire caller task custody while preserving
  parent/Sub-J linkage until cleanup.
- Standard death while already carried retains the Class14 task. Its next
  12DA0 visit keeps a valid parent row or, after18640 compaction, runs only
  16750's fixed null-hook release prefix before continuing the same task.
  The real world49 control checks both paths, unchanged Sub-I and no release
  RNG; missing-parent180F0 remains an explicit pre-mutation boundary. This
  follows the [shared person relation order](TYPE86_RUNTIME.md#attach-release-abort-radial).
- Main Base abort dispatches Type123 through its own `Type123Person` route
  into the standard-death publisher with scheduler registration, counts and
  same-family replacement. Playing radial gates on the native receipt and
  publishes `DynamicRadialDeathPublication::NativeType123`. Shared particle
  hits route through `SharedActorImpactOutcome::Type123`.

## Validation

Focused tests cover all eleven world49 births with per-field metadata
provenance, metadata rejection (model, health, axis, choice order, death
sound), full lifecycle (adopt, 30 movement ticks, non-Blocked/Pending/Dropped
visits, class38 lethal 12100 with death publication, cue74/death cue74,
class14 deferred removal), transplant isolation (receipt from another manager
adopted by 10, death and mutation custody refused), and C910 attach in
world49 with model889/cue56 retention. All27 children in the original bounded
capture cohort now own native constructors; unadopted task custody still
blocks before relation/RNG writes.
The shared attachment regression also proves a real dying Type123 retires
immediately on reattachment, remains silent, retains its body/components and
RNG, and rejects foreign, remote, pending or changed-parent evidence.

## Remaining oracles

- The shared `0x004C8740` PE record is now read: release+0C=CE90 and
  death+2C=null. [The person authority](TYPE86_RUNTIME.md#evidence) records
  this style-table evidence; it is not a separate Type123 observation.
- Both attach cue words are zero, so the static420760 parent-capability
  lookup proves Type123 remains silent for player and captor attachment.
- O4 (first live Type123 Class45 tick) is now port regression evidence
  (lifecycle test ticks acquiring graphs live) rather than an unobserved
  branch; a passive placement/tick observation remains the only
  retail-comparison form, not a tracer build.
- Type86 now owns its native four-branch lifecycle ([TYPE86_RUNTIME](TYPE86_RUNTIME.md));
  this owner must not borrow that one's descriptors, tasks or receipts.
