# Native Type78/86/95 people

The four-choice task kernel also serves [Type7](TYPE7_RUNTIME.md) through an
explicit `DiverWorker` profile. Its factory-capacity predicate, capability1404,
query-free steering, audio and damage are separate from the person catalog;
Main Base input remains restricted to people.

Ordinary Type78/86/95 own native A/B/D/I construction, four-branch root
selection, Class45 Attract Attention, Class10 Run Away, Class54 Go-To-Job,
Class6 Wander, hits, Class14 death/abort and living capture attach/release
through `v2k-game::native_type86`. Historical API names remain, but the receipt
stores an immutable `NativePersonProfile`; type, model and manager allocation
must all match it. No Type9/123 allocation, seed or receipt is borrowed.

## Evidence

- Canonical current release `v2k_game` / `v2k_formats` parsers loaded PRELOAD,
  system overlay3 tier1, then every tier1 ordinary world. No historical Python extractor
  was used as format authority.
- The initial River coverage contains nine Type86 spawns (indices
  0,1,2,17,18,19,20,21,22), all
  with param0, damage buffer0, model overrides `[0;4]`, no per-spawn animation
  and no configuration payload. World24/River is wind mode0 (zero Section-13
  vector), drag3, so its E100 suffix uses ordinary drag.
- The broader normal-tier corpus contains 47 Type86 births across
  worlds18/21/24/26/28/37. The shared constructor retains each actual authored
  record; the six-world Main Base regression adopts and ticks these cohorts
  before placing one native source at its own Base for Type90 conversion.
- Type95 has 57 births: worlds16/27/31/35/36/40 contain10/7/10/8/16/6.
  Type78 has21: worlds17/19/32/38 contain6/6/3/6. All use param0, rotation0,
  damage0, model overrides0, no animation/config payload. All are wind0/drag3
  except world32: wind1 and vector `[-300,0,-300]`. Its three people construct
  and run the shared E100 wind/drag suffix with the current frame's vector.
- Source: `bulk/ovl_handlers.c` `FUN_00410090` universally
  relocates Section12 records to the `4C8A28/4C8A30` tables and sets381F0/382B0.
  This is shared record initialization, not a Type86-specific handler. The
  `104B0 -> 09A80 -> D4A0 -> 381F0` family path, `bulk/game_logic.c`
  (`104B0`, `09A80`, `16550/16560/165B0` nearby predicates, `22C10`,
  `23030`, `235F0`, `18EB0`, `16700`, `16750`), the shared Class45
  BA40/AF50/C7D0 executor (`attract_attention.rs`, closed for Type9/123),
  the detached Run Away toolkit (`run_away.rs`: B6C0/B3F0/ADB0 setup,
  `evaluate_run_away_callback`, transition ordering) and the shared
  Go-To-Job plan (`go_to_job.rs`) supply the actual shared implementations.
- Retail PE table4C8A30 proves initD4A0, cleanupE9E0, deathDB80, primaryDAC0,
  infectedDA00, detailedDCA0, coarseE870, staticD920, attachDBF0, releaseDC50.
  `4C76A8` and `4C8740` both have release+0C=CE90 and death+2C=null;
  no additional capture is needed to resolve those style words.
- Related authorities: [Type122 capture children](TYPE122_RUNTIME.md#exact-ordinary-child-prerequisite-matrix),
  [Type123 people](TYPE123_RUNTIME.md), [Type8 workers](INTRO2_TYPE8.md),
  [cross-level runtime](CROSS_LEVEL_GAMEPLAY_RUNTIME.md).

## Authored metadata

These three profiles share A/B/D/I topology, mass10, flags `0x2F`,
behavior rule1, alternate class14, Sub-A `(1500,-3000,250)`, Sub-B
`(10000,1000)`, effects selectors `[1,0]`, zero low-health effects and no
C/E/F/G/H/J/K/L/M/N/O. The canonical metadata objects are equal after
normalizing exactly model slots, axis, choice order, surface lifetime and
death cue:

| Field | Type78 | Type86 | Type95 |
|---|---|---|---|
| Model slots | `[978;4]` | `[889;4]` | `[661;4]` |
| Axis range/filter | `3840 / 0x84` | `1536 / 0x04` | `3072 / 0x84` |
| Ordered `(rule,multiplier,class)` | `(7,10,10),(6,3,45),(12,200,54),(1,1,6)` | `(6,3,45),(7,10,10),(12,200,54),(1,1,6)` | `(1,1,6),(7,10,10),(6,3,45),(12,200,54)` |
| Surface lifetime | 4000ms | 5000ms | 5000ms |
| Death cue | 74 | 74 | 35 |

Remaining common fields:

| Field | Value |
|---|---|
| Capability / health | `0x1804 / 1500` |
| D divisor | 32 (`ORDINARY_TYPE90_SUB_D`; rest identical to divisor-20) |
| Sub-I cue triple | `[85,0,72]` (bit-3 85, mask-0x201 0, attention-stop 72) |
| Damage thresholds / Q8 | `[0,2000,400,0,200,0,0]` / `[0,256,256,512,128,0,512]` (Type9-identical) |
| Sub-I binding / directional stride | `1 / 4` |
| Accepted-hit cue / constructor attachment | `95 / null` |
| RunAway optional sound | absent (`None`; Type9 carries `Some(85)`) |

The shared `ActorAbdiTopology` admits each own `Type78Person`, `Type86Person`
or `Type95Person` kind
against `ORDINARY_TYPE90_SUB_D`, and the ABDI frame threads that descriptor
through the target prelude and `apply_type9_sub_d`. `GoToJobConstructorSuffix`
admits the kind, so rule-12 births run the shared AF90/03650 suffix.

## Constructor and four-branch root

`publish_authored_type86` consumes the loader-issued allocation, process
Sub-D construction, authored row, terrain anchor and the 20450 word, then runs
the weighted selector (one word, each record's actual order and enabled
nearby weights), the initial
state policy, and the selected branch:

- Nearby predicates evaluate against the already-constructed authored prefix
  in intrusive order through the owner's own authored range/filter: rule 6 PlayerNearby
  (mask `0x01`, `16550`), rule 7 BaddieNearby (mask `0x08`, `16560`), rule 12
  BaseNearby (mask `0x20`, `165B0`; pure existence/proximity, no capacity and
  no owner-capability requirement — unlike the worker's rule 13).
- Class45 runs the shared BA40 transaction (parity, optional candidate,
  event-`0x10` receipt for the phase-5 load drain, cue with sound72, forced
  stop, 1000ms local wander).
- Class10 runs B6C0: Tertiary clear, acquisition Secondary (phase0) and 500ms
  wander Primary (phase1) through `apply_run_away_task_setup_with_retirement`
  plus `prepare_shared_acquiring_runtime_task`, each followed by its 06070
  Sub-A-only suffix word.
- Class54 runs the shared AF90/03650 plan against rule-12 candidates from the
  same prefix, with the actual Sub-A binding and the `(base*4)/3` target-speed
  overwrite.
- Class6 runs the shared Wander setup.

`retain_native_type86_runtime` keeps profile, allocation lease/generation, authored
index, model slots, anchor, Sub-D receipt and the birth graph. Exploding
graphs are rejected at this boundary.

## Live tasks

The scheduler owner ticks the Primary first, then Secondary, then Tertiary in
retail slot order, stopping after any graph change:

- RunAway acquiring ticks the 500ms wander Primary (Neutral) and the
  TargetAcquisition Secondary through the shared 22CD0 first-eligible walk
  (baddie mask, owner range). Acceptance publishes B3F0 fleeing synchronously:
  variant+1 context with the target, S/T clears, 2000ms fleeing Primary, one
  shared constructor word plus the fixed 5/3 Sub-A overwrite (416 from base
  250). Selection consumes no RNG, so the immutable select precedes the
  mutating handoff under the still-open visit with no interleaving code.
- RunAway fleeing runs `evaluate_run_away_callback` (reflected-static-point
  vs direct-target paths, shared mover) with the pre-visit target snapshot
  for validation; the optional-sound draw shares the mover RNG stream through
  sequential borrows and never fires (sound id 0).
- Attract acquiring/target, Wander, carried and Exploding follow the Type123
  owner exactly (ForcedStop local wander, Neutral target-route/Wander,
  cue-expiry and transition reselection, stale S/T retirement on Wander
  entry, 03250 carried ticks, shared class14 terminal).
- AF50 retires the executing acquisition Secondary as part of successful
  target publication. Unwinding that dead wrapper does not cancel the
  handoff: the scheduler adopts the new target graph, leaves its elapsed
  time at zero, and first advances it on the next actor visit. The real
  world22 arrival position exercises this path for Type7.
- Go-To-Job runs `evaluate_go_to_job_callback` with the live target snapshot,
  owned-range predicate and shared mover, mirroring the worker arm.

Hits dispatch C690 only where the EXE style words say so: Wander, Go-To-Job
and Attract-target reselect in both slots; Run Away acquiring/fleeing
reselect on infected hits only (primary +28 is null, +20 is C690); the
acquiring triple, class14, carrying and cue leaves never reselect. Checked
damage uses the shared 15040 path (class38 `[500,6000]` filters to 12100,
lethal at health 1500); surviving primary hits play cue95; standard death
plays the profile's cue74/35, then C3A0 installs the 1000ms class14 graph.

## Attach, release, abort, radial

- `16700 -> DBF0 -> style+08` attaches every living graph; CD50
  selects carrying variant1 for class6/54, CE70 variant2 for class10/45,
  mirroring the Type9 carrying table. `420760` selects the child's Sub-I cue
  using the **parent's** capability and emits at the parent's position before
  changing Sub-I. For cues `[85,0,72]`, mask0x201 parents are silent and bit8
  captors request85. The earlier child-capability/silent interpretation is
  withdrawn. Carried installs the real None Primary with S/T cleared.
- The four carried styles are `4C7A08/4C87D0/4C76A8/4C8740` for
  class6/54/10/45. All retain the allocation and Sub-D state. RunAway's
  `4C76A8` must be admitted as a real None graph, including its null hit slots.
- `16750 -> DC50 -> CE90` releases with fixed relation writes, `0x8000` set,
  Sub-I release, then full four-branch reselection at commit time. Class14
  keeps its graph through release.
- A person killed while already carried keeps its Class14 owner across the
  next12DA0 visit. If the existing parent still has the Sub-J row, the
  relation stays attached. After18640 removes the dying row,12DA0 applies
  only16750's fixed release writes; Class14's null DC50 hook preserves its
  task, Sub-I and RNG stream. Membership is checked before scheduler writes,
  and release commits only after a continuing scheduler prefix. A missing
  parent still blocks explicitly at the separate180F0 boundary. Production
  controls cover real78/86/95 and123 with retained and compacted parent rows.
- Reattaching Class14 enters its real `4C70C0+08=CD50`: `40CD70` runs420760,
  clears8000, and `C6B0` selects variant1 `4C7108`. Its initializerC470 calls
  A860, retiring physical task slots0→1→2 (`40A87F..40A8A3`), then410B70 marks
  deferred removal. No selector, new death cue, None task or RNG word is
  introduced. `native_actor_attachment` shares this complete suffix with
  Type123 and workers8/79/90/91/116. All return explicit `DeferredDestroy` so
  their callers retire scheduler/cursor custody; parent/Sub-J linkage survives
  until ordinary cleanup. The living `RetainedGraph` result keeps the actual
  relation transfer receipt and ADB0's distinct2→1→replacement0 order.
- Attachment plans authenticate both allocation leases, all task IDs/private
  states, context, Sub-I, and the parent's capability/position before optional
  sound or callback writes. Pending/remote actors do not enter a fresh attach.
- Main Base abort dispatches these profiles through the existing person route;
  Playing radial gates on the native receipt and publishes
  `DynamicRadialDeathPublication::NativeType86`; shared particle hits route
  through `SharedActorImpactOutcome::Type86`.

## Validation

Focused tests cover all78/95 authored births, 30-frame owners including world32,
retained F70/angular wind and unchanged task RNG, profile/cross-profile metadata rejection,
death-cue selection, unchanged RNG/health on foreign receipts, plus all nine
world24 Type86 births and their full lifecycle (adopt, 30 movement ticks,
non-Blocked/Pending/Dropped visits, class38 lethal 12100 with death
publication, class14 deferred removal), transplant isolation, and C910 attach
in world24 with model889/cue85/72 retention. A real nearby-baddie reselector
publishes RunAway before C910 attach and D040 cleanup release, proving
`4C76A8` owner transfer and preservation of its own Sub-D storage. RunAway
sound and mover closures borrow the process RNG only during their own draws;
holding the optional-sound borrow through the mover would panic even with
the authored sound disabled.

Attachment regressions use actual native people78/86/95/123 and all five
worker profiles: standard death followed by CD50 attachment proves immediate
terminal context, retired task wrappers, one deferred queue entry, unchanged
Sub-D/body/Sub-A and RNG, preserved relation until the sweep, and no repeated
death cue. Living/dead Type86 attachment to a real Type122 independently
checks cue85. Foreign receipts, remote/pending actors and changed parent
evidence reject before callback effects.

The conversion controls in `converted_worker_runtime` use actual authored
people and Base allocations: Type86 in six worlds, Type95 in six, Type78 in
three zero-wind worlds. `main_base_person_contact` reads their
current three-slot callback graph, retained matrix and private direction;
the Sub-I descriptor suffix consumes no RNG and preserves task, animation and
matrix state. Dynamic Type90/91/79 output retains its own birth and live owner.
Foreign manager receipts fail before notification, destruction or birth;
unsupported counterpart damage remains explicit. See
[the Base/Factory limits](AUTHORED_BASE_FACTORY_RUNTIME.md#remaining-limits-and-validation-boundary).

## Remaining boundaries and retail observations

- World32's authored steady wind uses the shared44EC60 sea-side/shelter policy.
  Actor callbacks consume the frame's current vector without advancing its
  phase; F70 is retained before angular wind, grounding and master motion.
- These people remain excluded from campaign auxiliary cargo reconstruction
  by the retail type policy; fresh native ownership does not widen save admission.
- RunAway same-pass acquiring→fleeing handoff timing for Type86 is
  program-predicted (Type9 oracle `20260730-142219` covers Type9 only);
  living C690 reselection from class10, CE70 attach from variant0 vs
  variant1, and CE90 release reselection on the Type86 list are likewise
  implemented but unobserved live. No new tracer build is requested for any
  of these; a passive placement/tick observation is the only retail-
  comparison form.
