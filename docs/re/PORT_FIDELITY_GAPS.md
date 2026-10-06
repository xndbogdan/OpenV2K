# V2000 Port Fidelity Gaps

This is the acceptance list for differences observed against the original game.
It deliberately distinguishes user-observed behavior from conclusions obtained
only by static reverse engineering. An item is not complete until it has been
compared with the original in the same scenario.

Last observation pass: 2026-09-07 (first-world lighting/lens and peasant animation).

## Reported mismatches

### Intro2 opening sequence

- **Insects arrive before the asteroid impact.** The user reports retail
  insects arriving after the impact, while the port moved them immediately
  and allowed the asteroid to damage them. The executable's shared post-load
  activation clear was narrowed incorrectly to one insect in the port;
  particle collision also omitted the dormant-actor admission gate. Both
  source rules are now implemented and tested.
  [INTRO2_ACTIVATION.md](INTRO2_ACTIVATION.md) owns the source rule, authored
  releases and remaining matched-scene acceptance; do not hide models or add
  an impact-triggered release timer to compensate.
- **Turret/dragon combat.** [INTRO2_TYPE102.md](INTRO2_TYPE102.md) owns the
  native defensive turrets and the corrected Type13 Aim target admission.
  Their complete interaction and natural dragon-driven factory destruction
  remain subject to the Intro2 scene gate.

- **Meteor/explosion presentation is wrong.** The previously described yellow
  object is an incorrectly presented explosion effect, not an unidentified
  model, player projectile, or hovercraft artifact. There is no player craft in
  Intro2.
  - Runtime observation: original-game footage `V2000 2026.07.12 -
    13.03.06.01LE.mp4`, approximately 0:45--2:10. The first village impacts are
    visible around 0:51--0:55.
  - The retail impact is not one persistent yellow shape. A descending meteor
    streak is followed by several overlapping camera-facing white/yellow/orange
    explosion sprites, grey debris chunks, smoke, and ground-level fire. Each
    layer changes frame, size, position, and opacity while successive impacts
    overlap. The camera changes shot while the tail of the sequence is still
    active.
  - Dense 5-fps review of 0:50--1:00 exposes additional required behavior:
    - the meteor is a visible tumbling rock before its bright descending trail;
    - impact produces a near-full-screen white attack that visually washes the
      terrain, actors, and sky. Static call-path inspection does **not** connect
      the meteor event to `FUN_00441420`'s terrain-light writer, so this should
      first be reproduced by the recovered large additive sprite layers rather
      than by inventing an unverified dynamic terrain light;
    - the camera changes composition while the flash/explosion masks the
      transition, so the first shot cannot be validated from sparse camera
      target samples alone;
    - numerous grey chunks follow ballistic paths and animate/rotate while the
      bright surface explosions evolve independently;
    - multiple scorch/fire sites remain after the bright attack, and new smoke
      puffs continue to spawn for several seconds rather than all particles
      belonging to one impact-time burst;
    - the narrative caption types on while the effects play; it does not appear
      as one complete static string;
    - the alien walkers in the following wide shot have independent locomotion,
      limb/pose animation, and ground shadows while smoke emitters continue.
  - Dense 5-fps review of the later 1:00--1:10 interval adds a second golden
    segment:
    - the camera passes directly through/alongside large tree foliage, making
      correct camera-facing masked billboards, aspect, depth testing, and fog
      particularly visible;
    - the shoreline and animated water occupy much of the frame during the
      pan, providing a useful water-material comparison independent of normal
      gameplay controls;
    - the purple hive continuously emits overlapping pink/grey puffs rather
      than displaying one static plume;
    - several large flying insects cross very close to the camera while ground
      insects continue moving independently;
    - a second and then third narrative string visibly type on with a trailing
      underscore cursor;
    - the following village view has strong blue distance treatment on trees,
      buildings, and terrain.
  - The small animated red `V2000` flame/emblem at the bottom-left is separate
    from the impacts and is authentic: the footage keeps that cinematic HUD
    sprite active across the Intro2 shots. Do not remove it as an artifact.
  - Status: partially implemented, visual acceptance still open. The shared
    world-sprite path now uses the source Section-3 flags for masked,
    source-plus-destination-half, or additive compositing and the authored fixed
    shade row. The 2026-07-19 renderer also runs `FUN_0043D410`'s destructive
    pre-light centre gate once, sharing one survivor list between terrain light
    and sprite presentation. Null-frame, behind/far/extreme-screen deletion,
    descriptor-`0x02` retain-hidden behavior, and expanded sprite overlap now
    follow the retail policy with captured Intro2 far `0x1800` at both
    `0x004D0560` and `0x004FEEF0`; the earlier `0x1000` is the
    menu/pre-handoff value and must not gate visible Intro2. This is the
    evidence-backed correction for the yellow effect that formerly survived
    until a camera pan. A same-shot recapture is still required, and the port's
    floating centre projector is not yet the retail Q31 implementation. The
    Intro2 captions now use `FUN_00452790`'s authored integer 30-ms character
    interval and trailing underscore cursor. Their type-on cue also uses
    physical global Section-11 slot zero at half gain on the captured strict
    `> 2` retail-tick cadence. Inclusive adjacent records are evaluated in
    Section-2 table order, the final `#` page begins at 80 seconds, and one
    process-global cadence field is shared with gameplay notifications.
  - **Unrelated static-glow yellow wash closed (2026-08-09).** A new matched
    Castle report isolated a persistent camera-dependent wash that also occurs
    heavily at the start of Intro2. It is not a long-lived impact particle:
    the exact yellow Castle views include static `bigfuel` model 141 at cells
    `(149,98)` and `(162,99)`, while the clean view includes neither; Intro2
    contains eleven such objects. Each model has two additive sprite-617 glow
    billboards. The packed-immediate decoder omitted `FUN_00470840`'s final
    bit-`0x40` complement, so `0xF052` became +47 instead of -48 and the size
    subtraction wrapped to 65471..65507. The correctly unsigned GL consumer
    then built enormous depth-tested quads. Restoring the post-rotate
    complement keeps the animation in an ordinary positive range. Retail's
    exact DD sine-table path is 30..65 while the remaining floating port path
    is 30..66; this small, non-wash seam is recorded in
    [`NUMERIC_WRAP_AUDIT.md`](NUMERIC_WRAP_AUDIT.md).
    This correction does not close the separate meteor/impact presentation
    acceptance above.
  - **Meteor impact lifetime/camera dependency closed (2026-07-27).** The
    accepted pool trace births class 16 x10 and class 18 x1 at tick 144;
    class 18 is present through tick 198 and absent at 199, while class 16 is
    present through tick 204 and absent at 205. Those are the exact inclusive
    54/60-tick descriptor lifetimes. A constant-projector regression now
    passes the same boundaries through `prepare_presentation` and drains the
    later class-31 trails without changing camera state. A current-build
    column that outlives that point is therefore not evidence for changing
    impact lifetime. Instrument its prepared class/age/frame/size or prove the
    prepared list empty before changing production behavior. The still-missing
    descriptor-byte-`+0x09` ground-projected strip shares the stored
    flag-`0x20` ground-reference integration boundary below; its writer no
    longer needs a capture.
  - **Retail particle painter order restored (2026-07-27).** `FUN_0043D410`
    queues every billboard with the signed key `projected depth +
    descriptor[0x12]`; the global stable sort runs across masked,
    half-additive, and additive records, preserving intrusive-list order only
    for ties. `WorldSprite` now carries that raw key and the GL backend stably
    sorts the complete particle submission by it. Masked records alone retain
    depth writes; material family no longer overrides retail painter order.
  - **Stored particle surface writer recovered; integration remains open
    (2026-10-03).** The accepted passive Intro2 trace proves live use of flag
    `0x20`. Retail PE disassembly closes the writer that the bulk C omitted:
    `FUN_0043DB60` loads `particle+0x1D` at `0x0043DB67`, ORs `0x20` at
    `0x0043DB6B`, and stores it at `0x0043DB70` before sampling. Bit `0x10`
    set selects signed, toroidal four-corner bilinear interpolation, first X
    then Z with arithmetic `>> 8`; clear selects the current-cell coarse
    sample. Both scale signed terrain-height bytes by `0x20` and write the
    signed-8.8 cache `particle+0x18` (`0x0043DC48` / `0x0043DC84`). Allocation
    resets flags to `0x10` at `0x00440AF5`; the two draw sweeps clear only
    `0x10` at `0x0043DD79` / `0x0043DE69`, preserving `0x20`. This is a
    mutable first-sample cache, not a particle-class policy or a cache reset
    on every draw.
    `FUN_0043D410` calls the sampler only while `0x20` is clear; otherwise it
    reads `+0x18`, for both terrain-light radius and descriptor-byte-`+0x09`
    ground strips. Call/copy custody matters: `FUN_0043D300` copies all
    `0x20` bytes to the stack at `0x0043D320` and projects that same mutable
    copy ten times (`0x0043D3BC` / `0x0043D3EB`), shifting its position
    between calls without copying the cache back to the pool record. The
    direct `FUN_0043DC90` wrapper instead passes the original record.
    Implementation must retain the first-call sampling mode and cache on the
    actual record or copy across these consumers; matched terrain-wash/strip
    presentation remains unaccepted. The unknown-setter capture request is
    retired; the prepared hardware-watch script remains a diagnostic for a
    named lifecycle regression. Do not fold the independent, closed type-13
    black/cyan-rod callback defect into this boundary.
  - **Ptersect authored wing oscillator restored (2026-07-22), native movement
    still open.** Static Sub-G recovery now drives bindings
    `[1,2,3,4,5,8,9]` with the three authored sine records and exact phase/Q31
    arithmetic; real-data tests pin every raw record and prove that the linked
    wing hierarchy changes. The port deliberately leaves movement selectors 7
    and 10 at zero. Its scripted pose also lacks the entity velocity needed for
    `FUN_0041AC40`'s projected terrain sample, and its exact LCG recurrence is
    still a private surrogate stream rather than the recovered process-global
    allocation sequence. Do not tune those missing inputs from footage or call
    this native actor behavior complete.
  - Remaining audit: sub-frame animation age, repeated-event gating, billboard
    scale/middle fields, depth ordering, sustained fire/smoke behavior, actor
    behavior/animation, and the camera transitions. The recovered meteor
    wake, impact, and debris-trail emitters now use the shared bounded pool;
    the unrelated persistent emitters seen later in both golden intervals
    remain a concrete gap.
  - Corrected 2026-07-17 from the full-session trace: Section-13 actors 52--54
    (`tulazred` and the two `turetred` actors) are only selected as camera
    subjects by default-operation records. They retain their authored
    positions and parked `Ry(-pi/2)` basis after those cuts; the port therefore
    no longer gives them synthetic translation or headings. Their distinct
    later disappearance ticks are combat outcomes, not deterministic cinematic
    despawn cues, and remain pending the native damage/death path.
  - Particle-source audit updated 2026-07-16; keep the distinct entity and
    static-terrain callers separate:
    - `FUN_0040E370` is the underwater actor-bubble emitter (particle class
      42). Intro2 hive type 67 has both controlling bytes at Section-12
      `+0x72/+0x73` zero, so this path cannot own the hive/fire plume.
    - The layered explosion helper `FUN_00441200` and its
      `FUN_00441420` terrain-light writer are reached only from model opcode
      `0x8E` or animation events `0x1E/0x3A`. The complete Intro2 meteor,
      hive, hut, and factory model trees contain no opcode `0x8E`; do not graft
      this effect or dynamic terrain light onto meteor impact without new
      runtime evidence.
    - `FUN_00441670` emits class 20 above sea or class 42 underwater.
      `FUN_00412DA0` repeats it for an *entity* only when runtime byte `+0x84`
      bit 3 is set. In the retail full-session capture the hive remains `0x00`
      and the hut remains `0x04` through its destroyed-model transition, so
      neither entity owns that callback.
    - There is a separate proven caller in the static Section-10 object draw.
      `FUN_0042F650` visits each visible nonzero-attribute cell and, when the
      Section-9 object kind is not 9 and terrain-type bit 3 is set, consumes a
      1/8 RNG gate for `FUN_00441670`. Kind zero also consumes an independent
      1/16 gate for `FUN_00441700` (class 21 above sea / 43 underwater).
      Intro2 has seven qualifying cells. The destroyed angled house at wrapped
      cell `(188,134)` is only 7.906 horizontal cells from the hive and emits
      class-20 smoke at raw `[-17280,-472,-31104]`. The port now runs these
      exact per-cell gates, positions, classes, allocator velocity bias,
      emitter RNG order, and class-20/21/42/43 callbacks on every render
      traversal. The retail capture includes repeated births from one source
      under the same 50-Hz clock value, so this path deliberately has no tick
      latch.
    - The incoming meteor streak is the distinct entity-owned class-40
      (`0x28`) path. In the entity-`+8` high-bit branch, `FUN_00412DA0`
      selects it from runtime byte `+0x84`, and
      `FUN_004061A0` deposits one zero-velocity particle at the meteor's current
      position per normal component update. The descriptor supplies eight
      frames, rate `0x18`, scale `0x0800`, priority 2, radius `0x28`, and a
      15-tick lifetime. The live capture finds 577 durable type-34-owned
      births/recycles against 598 attempts implied by the four captured onset/
      impact intervals; fixed-pool rejection explains the difference and the
      second meteor's late first survivor. The helper and position integration
      consume the same component delta; the port now submits every nominal
      8-ms attempt and all crossed impacts in one timestamped order, and
      preserves the same owner on the class-16/class-18 impact records.
    - Type 67's `+0x124` target resolves to action descriptor `0x004C9730`;
      the live hive runs action state `0x004C9558`. This is entity action state,
      not a hidden particle descriptor. Intro2 Section 13 also contains no
      hidden smoke/fire entities.
  - Live pool attribution is now captured. Unowned source-handle zero class-20
    and class-21 records repeatedly begin at the exact eligible static-cell
    centres (2,114 and 406 generation starts respectively), independently
    confirming the path above. The same capture contains 550 source-zero
    class-42 starts, but Intro2's seven eligible static cells are all above sea;
    those bubbles therefore belong to a separate water-response caller and are
    not evidence for an underwater Intro2 static emitter. Static disassembly
    narrows `FUN_00441670` to four parent returns: entity common
    `0x00412FF9`, static terrain cell `0x0042F7E2`, generic entity
    `0x00446A32`, and special entity `0x0044725F`. The special-entity caller
    does not initialize the helper request's source dword before the call,
    while the passive pool trace retains neither that parent return nor the
    allocator result. The prepared
    `capture-intro2-source-zero-class42.ps1` WinDbg oracle records both sides
    of the exact underwater helper allocation; do not assign these 550 births
    to a port emitter until that transcript identifies the live parent. The
    fixed 200-record pool, priority/victim rules, and cross-material painter
    order are already implemented from independent static and runtime
    evidence, so this unresolved provenance is not permission to change those
    policies. The hive itself
    produces 144 observationally stable class-5 births from source handle
    `0x047F0001` at about 19.765..26.910 seconds. They arrive as one durable
    pool record about every 49.97 ms, not as observed four-particle groups. The
    tracer samples are duplicate full-pool reads with validated seven-list
    topology and remain explicitly *observationally stable*, not atomic, so
    allocate/free or victim-recycle activity wholly inside one retail update
    is still invisible.
  - **The type-67 class-5 owner and request cadence are now recovered.** The
    complete WinDbg entry reduction at
    `runtime_re/captures/reference/20260718-intro2-class5-allocation-trace.txt`
    contains 422 class-5 attempts. Exactly 145 belong to the hive's
    run-dependent source handle; all return through `0x0041C30C`, the call in
    `FUN_0041BEB0`'s live action branch. They occupy unique ticks 790..1150,
    with 72 two-tick and 72 three-tick gaps: one allocation at every 50-ms
    cadence crossing, never a four-particle group. All use global request block
    `0x004C93C8`, position `[-17536,-240,-32640]`, mode/tail zero, and signed
    velocity ranges X `-730..769`, Y `753..1260`, Z `-689..733`.
  - The decompiled owner consumes exactly three RNG calls per request: angle,
    radial magnitude modulo 1600, then vertical velocity
    `750 + (rng & 511)`; the retail sine/cosine table projects the radial value
    into X/Z before `FUN_00440A60`. This explains the previously captured
    `+770/-713` X extremes and confirms `FUN_00422840` as a false positive.
    The reusable integration boundary is therefore the type-67 entity action,
    not an Intro2 timeline effect. The earlier passive run's 144 durable births
    versus this separate run's 145 attempts is consistent with one ordinary
    fixed-pool rejection, but is not same-run proof; implementation must submit
    every cadence crossing to the shared allocator rather than cap the plume at
    144 survivors.
  - **Native hive component implemented 2026-07-18.** The port identifies the
    retained controller from initializer `FUN_00425760`, its exact Sub-N
    attachment `[384,400,384]`, and the spawn animation's 50,000-us interval.
    Intro2 operation 2 enables authored spawn 24 at 15 seconds. The shared
    action advances only through update mode zero; `FUN_00411400`'s outer
    camera-eye bounds are reproduced with wrapping world X/Z words, the
    `(columns/2+3, rows+3)` slack, and its `U.z` rear plane. Coarse/disabled
    calls reset the accumulator, interval equality does not emit, and catch-up
    crossings remain synchronous. Each crossing consumes all three global RNG
    words before the fixed-pool attempt, so rejection cannot alter later RNG.
    A real-data integration test pins Intro2 spawn 24's source position,
    attachment, interval, and emitted raw position
    `[-17536,-240,-32640]`.
  - Class 5's proven `FUN_0043E180` surface tail is also live. An above-to-
    water transition is dispatched before endpoint terrain contact. Selector 6
    divides signed raw X/Z velocity by four and Y by two with truncation toward
    zero, keeps the particle, and does not snap Y. Every other selector calls
    `FUN_00433720`'s equivalent wrapped-cell OR-`0x10` mutation (deduplicated
    only when already applied) and deletes the particle. This closes the
    passive trace's 50..102-tick hive deaths instead of letting every puff live
    to the descriptor's 200-tick limit. Class 5's distinct mode-3 entity-hit
    `FUN_0043F780` entity hits and solid-static `FUN_0043F920` /
    `FUN_00427DE0` infection are live; selector `0x12` / `FUN_0043F7C0` /
    `FUN_0043F950` stay fail-closed and are not fabricated by this Intro2
    checkpoint.
  - **Level-1 gameplay evidence accepted 2026-07-22.** Keep
    `runtime_re/captures/local/20260722-022643-hive-virus.jsonl` and its
    `.protocol.json` sidecar together; the run is complete and should not be
    repeated. Type-67 handle `0x043A0001` produces 463 stable class-5 births at
    the authored 80,000-us cadence from raw owner
    `[-17920,-256,-32768]` plus attachment `[384,400,384]`. A real-data test
    now pins that Level-1 animation header to `[80000,5000,5,5,1]` and checks
    the strict-equality boundary and first emitted position
    `[-17536,144,-32384]`.
  - The same 53.545-second trace advances the exact terrain infection count
    from 95 to 139. It contains 47 `0x10` sets and three clears; 37 sets follow
    a class-5 removal in the same wrapped cell within the preceding terrain
    sample interval. That is strong same-run validation of the implemented
    impact mutation, but not the whole landscape algorithm. Class-5 particles
    are transient carriers; the blood-like persistent infection is terrain-cell
    bit `0x10`, and later controller-owned cell spread must not be modeled as a
    long-lived particle or inferred one-for-one from particle births.
    Static rendering now proves that this presentation is direct terrain
    ownership, not a callback-created decal: `FUN_00430430` maps the four
    corner bits through `DAT_004CACC8/CC` onto fixed-row-28 sprites at terrain
    base `+120..124`. The port loads and draws that keyed/opaque overlay and
    hashes the live type-byte grid so stationary-camera mutations rebuild the
    bounded mesh. `FUN_0042FCC0`/`FUN_00430140` additionally apply
    `FUN_00433530`'s animated signed X/Z offsets to infected vertices. The GL
    terrain path now applies the recovered two-phase motion and triangle-wave
    envelope to the shared base/overlay vertices, repeats retail's 16x16
    selector layout, and refreshes it only at the invisible zero-amplitude
    boundary. Its selector realization is deterministic until the port owns
    retail's single process-global RNG call order. The overlay is depth-read-only
    with a small raster bias so separate GL programs do not z-fight while later
    world geometry still tests against the base terrain depth.
    Controller words 1..3 also drive `FUN_00436960`'s local-neighbor set/clear
    decision and `FUN_00436750`'s queued 350,000-us cardinal propagation.
    `infection_evolution.rs` now contains their exact live terrain executor:
    signed strict word-1 crossings, toroidal 3x3 thresholds, state-1 dense
    random walk, FIFO tails, one shared RNG word per queued step, anchor
    cancellation, and signed underwater termination are test-pinned. Each tail
    queues fixed-rate/full-gain positional global sound 64 immediately before
    its ordered bit-0x10 write; 64 is not a visual object type. Intro2 and
    gameplay run this before the same component's radial class-5 emission,
    replay writes into the current Level allocation, and invalidate cached
    terrain geometry on change. Unresolved predicate/terrain state fails closed.
    Only Level-26's authored runtime envelope remains an acceptance boundary.
    Do not label the observed clears as a cure: static recovery separately
    proves that `"Cleansing Landscape"` installs mode 6 and clears infection
    through `FUN_00433720(...,0)`, while `"Defecate Virus"` installs mode 5 and
    sets it. Retail's pause-menu Cheats -> Weapons path proves normal player
    inventory selectors 7 (`Antidote`, packed grant `0x0003E707`) and 29
    (`Antidote Bomb`, `0x0003E71D`), both with exact canonical descriptors.
    Static evidence does not connect either fired selector to the cleanser,
    however, and their radius/cadence/natural acquisition remain unknown.
    Run `runtime_re/scripts/capture-antidote-impact.ps1` and retain its JSONL,
    protocol, WinDbg transcript, and command file together; require an exact
    shot-correlated bit-`0x10` clear/call path before implementing a cure.
    Capture a long natural Level-26 spread separately to validate that world's
    authored envelope; the broader executor is already live-bound.
  - **Native Type26 owns weighted Furniture/Follow/Defecate tasks; matched
    scene acceptance and the complete contact scan remain open.** Authored
    spawns10/25 use their actual allocation/component receipts and shared RNG.
    The older accepted handle`047E0001` class4 topology remains an explicit
    fixture, not a forced production choice. B9E0 clears Secondary, installs
    slot2`02850` packed mode`00050000`, then the2000-ms`02BA0` Primary on
    successful construction. Zero is success, not a fallback. Class4 style
    `4C7E88` has zero+34/+38 masks; zero01430 result returns9C01 through+00
    C690 after unwind, before its strict`age > 2000` timeout. Both paths
    preserve16410 task-result suppression.
    Detailed02850 consumes one shared word, gates on
    `random16 < (elapsed_us >> 2)`, and submits class5 at scale800 through
    `440DC0`, retaining the forward-Q31 origin, owner sign, rejected-pool
    cursor advances and zero sound payload. Coarse mode consumes two words
    and infects one wrapping cell within[-256,+255] raw X/Z, without a
    particle. Detailed carriers later run40120/3E180 and set terrain bit10;
    the lasting mark is terrain presentation. The unsigned type+A2 lifetime
    remains authoritative.
    Furniture is the separate B7C0/1CA0 graph. Its successful6030/6070 suffix
    enables H and consumes one A target-speed word before Primary publication;
    the earlier no-RNG interpretation was incorrect. Its live-axis4230C0 scan
    writes target Y only on success, then calls01430 even when a later scan
    fails. [Type58](INTRO2_TYPE58.md) owns the shared scanner/constructor
    formulas and its bounded class26 C890/C690/11760 static-contact adapter,
    ordered with meteor contacts. Fence convex commands0x8C/8A/8D now execute
    their source half-space program, including live Type58 contact. Type58's
    [shared native construction](INTRO2_TYPE58.md#shared-ordinary-world-construction)
    now covers ordinary worlds14/24/31, with actual task/component custody
    through hits, Class12, abort and spider pairs. Follow/Search static hooks
    now retain the exact02CA0 private-state writes and RNG before11760.
    Class12 static contact retains null hooks and shares Type17's native
    terrain/water response through each actor's model/cues/death policy.
    Independent Type58 pair scanning and Type26's late contact
    remain open.
    These behavior tasks remain separate from Type67's radial hive emitter
    and controller-owned landscape spread. Player cure-named selectors7/29
    still lack an authenticated binding to mode6 cleansing.
  - **Type49 Cleansing Landscape native runtime and exact class-6 clear ownership are implemented.**
    The analyzed `20260731-185548` run binds type-49/model-266's parented
    presentation and transient type-93 handoff to canonical style
    `0x004C85D8`, slot-2 `FUN_00402850` mode `0x00060000`, slot-0
    `FUN_00403040`, 181 source-owned class-6 births, and 14
    duplicate-validated bit-`0x10` clears along the vehicle path. The
    `20260731-184134` run is the corresponding attached-state negative control.
    Particle descriptor class 6 owns callback `FUN_0043E1A0`, the clearing
    sibling of class 5's `FUN_0043E180`. The port now shares the exact detailed
    chance/origin/pacing and coarse-cell RNG planner across explicit mode 5/set
    and mode 6/clear policies, materializes the authored class, and preserves
    ordered same-cell set/clear writes through `WorldFx`.
    Native Type49 construction, class42 infection-seeking movement, carrying,
    campaign restoration and class49 destruction now own the live path.
    The accepted NoCD02 TTD joins the cargo/AD50 transition and proves
    class6→`43E21B`→`33720(...,0)`, preserving cell bytes `DD00` while
    material54h becomes44h. It separately attributes the fatal4800 damage
    to player46/class55. [Cleansing Vehicle](CLEANSING_VEHICLE.md) owns
    source formulas, acceptance tests and remaining nonzero-wind/static-world
    boundaries. Player Antidote selectors7/29 remain a separate, unproven
    fired-weapon binding; they are not needed to identify this machine.
  - Corrected 2026-07-17 from authored Section-2 default-operation records and
    the complete retail session trace: camera target 32 remains selected until
    15.0 seconds, followed by targets 8 at 15.0, 25 at 23.0, and 0 at 27.5
    seconds. The former footage estimate cut at 7.5 seconds and left the port
    one shot ahead through 27.5 seconds. Retail tick 751 independently catches
    the class-1 camera still beside target 32 while accelerating toward target
    8 immediately after the 15-second command.
  - The hidden class-1 proxy now follows `FUN_004503C0` at the exact boundary:
    raw X/Z error `0x3200` still follows and `0x3201` snaps. Its Y controller
    uses target displacement plus terrain correction without injecting target
    Y velocity.
    The 2026-07-18 WinDbg reduction at
    `runtime_re/captures/reference/20260718-intro2-camera-call-trace.txt` closes
    all five live `FUN_0040ED10` entry arguments and caller identities across
    11,354 calls; parameters 3 and 4 are zero throughout.
  - **Sprung eye/focus mechanics closed 2026-07-19.** The complementary passive
    run at
    `runtime_re/captures/local/20260719-012555-intro2-camera-spring.jsonl`
    duplicates both spring blocks, the hidden proxy, and the resolved chased
    subject at 200 Hz across all three early cuts. The port now starts the
    proxy at authored raw `[0,0x200,-0x1100]`, keeps the caller's literal zero
    camera offset distinct from the persisted Active Camera setting consumed
    internally by `FUN_0040ED10`, preserves
    the 8-ms integrate -> spring -> follow order, and advances the spring even
    when the subject handle cannot resolve. Bit-exact golden transitions pin
    both `FUN_0040F800` and the no-reset/reset/no-reset decisions at 15.0,
    23.0, and 27.5 seconds. The hidden first call inherits the prior menu
    spring in retail; the first subject snap resets it before Intro2 becomes
    visible. Do not retune this subsystem.
  - Moving-subject composition remains owned by the general Intro2 AI gap, not
    by a camera approximation. The camera now reads live entity position and
    velocity words. Retail target trajectories differ between runs because
    types 47/26/13 execute Guard Location, Defecate Virus, and Search And
    Attack Target behavior/style transitions through shared RNG. Never bake
    one captured trajectory or compensate with a camera offset. Defecate
    Virus's infection callback and companion-task topology are statically
    exact above, but its common mover is not yet live. Guard Location's
    class-32 topology and slot-1 acquisition callback are exact:
    successful `FUN_00401F80` construction (zero return) installs its slot-1
    candidate-acquisition task, after which the duration-5000 slot-0 anchor
    wander task is installed as a normal companion, not as a failure fallback.
    Its one-draw `(low16 & 3) == 0` gate commits a nonzero constructor filter
    override before first-eligible intrusive-list selection; zero behavior
    results map to the `0x004BE1A8`/`0x9C02` singleton and nonzero results
    propagate unchanged. The generic scheduler consumes `0x9C02` without an
    owner transition. Authenticated Type-47 wrappers now bind live RNG, list
    snapshots, and the behavior handoff for Level-1 spawns 11/12/13 and Intro2
    spawns 6/7/8; other owners remain adapter boundaries. Joined static and
    accepted Intro2 evidence closes normal C690 tag/timeout inputs and results:
    the D760/D7A0 trampolines supply live behavior context despite the
    scheduler's literal zero, context word zero selects TypeDefault `+0x118`,
    Chase invalid-target and Aim timeout both join to Guard publication, and
    one selector RNG word selects Guard x9 / Wander x1. The live owners apply
    atomic graph replacement, freshly read only later slots, and explicitly
    block Intro2 source-dying C690 before selector RNG or graph mutation; its
    class-12 Common-Dying owner is not authenticated for this cohort. Direct Aim
    `0x9C00` / `+0x04` is statically closed but not capture-observed. The shared
    stream is the zero-initialized
    process-global MSVC LCG, not a per-actor or per-Intro seed. Variation is
    history/timing-sensitive draw interleaving. Search And Attack's closest
    target selection is deterministic for the live candidate set and consumes
    no RNG; different human/building targets arise after prior motion, cadence,
    death, or intrusive-list state has diverged. Validate behavior, target
    eligibility, timing/motion envelopes, and the functional cinematic outcome
    rather than frame-identical trajectories.
    Variant 1 replaces that pair with the recovered slot-2 auxiliary and
    slot-0 target-proximity tasks. Accepted focused transcript
    `runtime_re/captures/local/20260727-235423-intro2-actor-task-mover.txt`
    closes the task/caller boundary: 132 style switches each issue three
    ordered slot-install calls, and 671 valid `FUN_00401430` returns preserve
    all seven arguments. It binds stable movement-state pointers
    `0x138200DC/0x13821810/0x13823EC0` to types 13/26/47 and shows scheduler
    mode one only for types 13/26. Ten candidate-callback entries establish
    synchronous dispatch through `0x0040D7A0`, but the disabled return hook
    leaves callback results unresolved. Static recovery separately closes
    `FUN_00401430` and all eleven direct callees, including component order,
    dispatcher priority, target lifetime, the A/B/C force tail, and the D
    terrain/orientation path. Rust now also owns the detached resumable mover
    frame transaction: target rejection precedes component evidence, Sub-D
    precedes its dependent F/K reads, post-callback snapshots select every
    later phase, Sub-A is re-gated after Sub-C, and phase-tagged commits plus
    durable evidence blocks prevent callback/effect replay. Rust now owns the
    mutation-safe three-slot
    lifecycle and phased scheduler, the ordinary type-9 Wander owner bridge
    (failure-ordered setup, post-allocation Sub-A reset, one-or-three-draw
    retarget, and post-unwind tagged-result/timeout selection), the detached
    Guard Location initializer and acquisition transaction (authored-context
    copy, slot-2 clear, fallible slot-1 search, exact callback/selector, then
    conditional duration-5000 slot-0 wander), typed A/B/D
    descriptors, exact A-before-B phases, type-9 D classifier/cache,
    separately exact callback drag and old-XZ snap,
    the atomic mode-zero D -> heading -> I -> A -> B and any-nonzero
    D -> heading -> A -> B type-9 frame routes, the exact zero-RNG Sub-I
    direction-mismatch turn/propagation path, the centralized
    `FUN_004018A0` F/H/I/G then K/L route planner, the ordinary
    `FUN_0040E370` underwater timer/bubble/sound planner, its class-42
    materializer, and the later master integration gates. Exact Section-12
    component topology is now retained and the type-9 executor consumes the
    central route planner, so F/H/G/K/L cannot silently pre-empt or supplement
    either route. Restricted mode bypasses all Sub-I validation and mutation
    while preserving its output selector; A/B deliberately use the pre-D
    entity basis even after the mismatch branch writes heading. The corrected suffix order is drag -> old-XZ
    snap -> `FUN_0040E370` surface ownership -> callback return -> master
    integration, not integration before snap. The exact drag/snap helpers are
    intentionally not presented as the complete callback because first-world
    type 9 has Section-12 `+0x72=1,+0x73=0,+0x74=5000`: later
    `FUN_0040E370` owns the separately ported underwater timer, exact shared-RNG
    class-42 bubble packet, and fixed sound-106 gate. Every recovered E370
    return path is zero, but its mutations still matter. The exact expiry
    boundary remains an explicit lifecycle continuation because retail calls
    `FUN_00416750`/`FUN_00410C10` synchronously and then reads their mutated
    live record. The master helper likewise
    requires `FUN_00412DA0`'s carry-adjusted/capped effective step rather than
    assuming the outer frame delta.
    Faithful execution remains blocked by attaching that bounded owner bridge
    and the detached mover transaction to live entities, supplying concrete
    component/effect/world adapters, and connecting the
    seven-argument mover plus behavior-transition/contact dispatch through the
    process-shared RNG owner, and closing the post-expiry lifecycle
    continuation—not by another broad Intro2 actor survey. In particular,
    The accepted fresh-process Sub-D captures
    `20260730-034232-sub-d-constructor-provenance.txt` and
    `20260730-035135-sub-d-constructor-provenance.txt` close the constructor
    owner and process-counter sequence. Each has 56 balanced successful
    transactions in the same semantic order: counter `0x00..0x37`, matching
    runtime `+0x3A`, final global `0x38`, and no reset at the Intro2 -> Level-1
    tick handoff. A Level-1-local zero seed is therefore still forbidden.
    Every constructor clears all eight cache rows, but it preserves
    `+0x38/+0x39`; matched owners receive different origin bytes across the
    two runs, proving allocator residue rather than authored coordinates.
    The accepted follow-up
    `20260730-064237-sub-d-first-consumer.txt` joins all six fresh-Level-1
    ordinary type-9 allocations to their first classifier call. Every call
    enters through `0x0041F7A8`, selects the full reset despite varied
    allocator residue, anchors to its query cell, returns class zero, and
    changes the zero rows to `[1,0,0,0,0,0,0,0]`; two owners also prove that
    first-use order can differ from construction order. A narrow
    fresh-New-Game ordinary-type-9 pending-first-query reset is therefore
    authorized. Live overlay-13 Section-13 rebuilds (New Game, a development
    Normal first-world load, and Cistern return to Peasant) now call that same
    fresh constructor. Generic Sub-D owners, Load Game, alternate allocation
    histories, and supplied live cache state remain fail-closed. This closes
    that first-consumer evidence request, not the concrete live
    component/effect/world adapters.
    Search And Attack Target class 7 is now exact through acquisition and
    mutation-safe task-owner installation: descriptor `0x004C88A8` owns
    variants `0x004C7A50/7A98/7AE0`;
    deterministic nearest-target search `FUN_00422CD0` feeds the target into
    corrected variant-1 initializer `FUN_0040ADE0`. The accepted actor trace
    shows Ptersect `0x04970001` transition at approximately 31,900 ms from the
    normal concurrent slot-1 search `0x00402080` and duration-500 slot-0 wander
    `0x00402BA0` to
    5,000-ms slot-0 chase `0x00403490` and slot-2 fire `0x00402300`, targeting
    `0x047F0001`. The owner bridge preserves each variant's exact partial
    commit on allocation failure without activating live callbacks. The
    detached slot-1 `FUN_00402050/00402080` task now also preserves the exact
    constructor filter override, selector-tag consumption, optional synchronous
    behavior handoff, zero-result `0x004BE1B0`/`0x9C02` transition, nonzero
    result propagation, and wrapper-replacement semantics in the central
    heterogeneous dispatcher. Its concrete live entity/component snapshot
    binds `FUN_00422CD0` to the manager live list and applies the recovered
    C7D0/C6B0 variant+1 and ADE0 publication from type `+0x9C/+0xA8`.
    Captured Intro2 production publishes spawn 0's class-7 B6C0 graph and ticks
    slot-0 `FUN_00402BA0` then slot-1 `FUN_00402080`. The B6C0 owner now
    commits Type-13's classifier-free target/Sub-D/K/L prefix and complete
    normal A690/AA60/AC40/B210 Sub-G callback as one transaction, including
    persistent component state, angle words, velocity, and optional sound. No new
    normal Sub-G capture is needed. The callback is exact for its supplied
    pre-call basis. Production now publishes the post-task DCA0/E870 `13F70`
    matrix after completed traversal, while same-pass Aim reads the retained
    pre-refresh columns. Blocked work retains the old matrix and pending C690
    retains the original effective-flag latch. Normal Intro2 production now
    owns 12DA0 timing, dormant B2 clearing, the 22-second activation, detailed
    G/K/L or coarse G dispatch, E100 gravity/drag, E370 timer decay, and final
    master motion. Coarse Aim ages without target/emitter reads. Its selector outputs
    reach the Entity animation-variable bank, then the current Intro2
    presentation controller may overwrite them. Tagged or strict post-500-ms
    transitions now enter the live class-7 variant-0 C690/AC60 root. Its
    `0x1000` suppression gate precedes metadata, context, and RNG access; an
    admitted root uses one selector word (`< 0x4000` class 5, otherwise class
    7), reuses the behavior context while preserving `+0x08/+0x0C` and
    `initial_behavior`, and consumes the selected initializer's exact one or
    two suffix words. Suppression and a new class-7 graph freshly visit
    Secondary in the same pass without revisiting Primary. Dying class 1 stays
    fail-closed before selector RNG or graph mutation. Class-5 publication is
    closed, and the specialized owner retains that 5,000-ms Primary for later
    `FUN_00401430` visits without a Secondary. A successful ADE0 handoff
    retains the pursuing graph and later Chase visits commit that same
    `FUN_00401430`. Chase tag/timeout C690 from variant-1 `+0x00` is live.
    Later Aim and ADE0 same-pass Aim run Type-13 `FUN_00424650` (method 10 /
    sound 75 / speed 2400). A later live-list drain materializes class 38, not
    class 87. Relation-owner, remote, dying, and nonzero-wind paths remain
    outside this bounded owner. Matched presentation acceptance and selector
    ownership remain pending, so Intro2 still draws this actor through the
    presentation proxy.
    Slot-2 Aim And Fire now has an exact detached
    `FUN_00402220/00402300` runtime in the central heterogeneous dispatcher:
    it preserves successful-allocation Sub-F
    activation before publication, target-invalid `0x004BE158` / tag `0x9C00`,
    the optional-sound RNG gate before emitter work, typed seven-argument
    `FUN_00424650` submission, nonzero propagation, generic-scheduler
    tag/timeout precedence, the exact completed-versus-absent/state-gated
    owner-callback fallthrough, and wrapper self-removal/replacement semantics.
    Its exact common `FUN_00424650` control transaction is now detached too:
    cadence/RNG gates, target and manual paths, cached-allocation refreshes,
    lead/gravity carry-forward, joint ordering, transient primary/auxiliary
    appends, counters, sound, and nonzero propagation are closed. Live Sub-E
    component binding plus entity/allocation, process-shared RNG,
    aim/forward-half-space evaluation, fixed-point launch, intrusive-list,
    joint, and audio adapters remain external. Do not substitute the bounded
    type-47 firing core for type 13: cohort-specific wrappers authenticate
    Level-1 spawns 11/12/13 with type-46 targets and Intro2 Type-47 spawns
    6/7/8 with captured type-9 targets. Intro2 Type-13 spawn 0 now owns a
    separate method-10 / sound-75 / speed-2400 adapter; it does not reuse
    method 30, class 87, or sound 70. Keep the Intro2 proxy for framing
    gaps, not for the now-live Type-47 Guard pursuit or Type-13 ADE0/later
    Aim and class-38 drain.
  - Implemented 2026-07-27: the accepted full-frame DirectDraw trace resumes
    black `COLORFILL` for Intro2's final `#` page. The port now switches from
    the world montage to a black backdrop at retail tick 4000, while retaining
    the exact authored captions (`You are our last chance`, then
    `End transmission`), type-on cadence, and animated bottom-left emblem
    through the tick-4301 handoff frame.

### Intro and first-world insect/audio/lifecycle pass

- **The fixed Type-61 loop and sound-11 follow are live.** Exhaustive
  normal-tier Section-12 inspection finds sound 11 on types 15, 44, 87, and
  108; sound 44 on type 61; and sound 100 on type 111. Thus the buzzing Intro
  `wasp`/type 15 and `deathwas`/type 87 are only two of four sound-11 owning
  types; Ptersect and `bluebee2` still do not own it. One ordered entity-audio
  sidecar now starts, retunes, physically culls/restarts, and releases the fixed
  sound-44 loop for every live Type-61 pickup, including model-138 overrides
  and factory births. It keys lifetime from retained `+0x8C` custody and uses
  no RNG. Types 15/44/87/108 mix sound 11 from the `FUN_0044C920`
  pre-integration snapshot and the audible-only warble. A missing snapshot
  fail-closes; the Intro render proxy is not that position. Accepted
  Type-111 campaign gates retain their existing dedicated sound-100 owner.
- **Observed attack/death samples now have static owners.** Intro green-dragon
  flame emission uses logical 81 -> PCM `sound_021`; type-47 `newant` death
  uses 75 -> `sound_018`; and the penned Level-1 type-17 `spider` death uses 94
  -> `sound_030`, ruling out the observed PCM-36 alternative for that actor.
  The one conflict is newant fire: its successful-shot descriptor stores 70 ->
  `sound_013`, while 92 -> `sound_029` is its accepted-hit cue. PCM 29 can
  therefore overlap a firefight, and Intro `deathwas` also authors it as a
  projectile cue, but the Level-1 newant descriptor must not be changed from
  matched listening alone.
- **Ground-insect legs: construction, LUT, and live pixel-ortho fill.**
  Spider/stag/newant author legs as opcode-`0x22` sprite ribbons (spider
  985/986) plus `0x02` palette hairlines on type-14 selectors. The stream,
  Sub-H selector map, and `FUN_0041D360` type-0 slot geometry are recovered.
  Shared `09A80/1D2A0` construction now retains each authored record count and
  the outer `+0x0C` terrain/water policy without a type whitelist. Intro2
  Type94 uses terrain plus water; Types16/77/58 use terrain. D360/DF20 consume
  that policy with the current draw tick. [Type16's own task and Sub-D
  owners](INTRO2_TYPE16.md) now drive its live pose. The other cohorts retain
  separate task/first-query admission boundaries; constructed legs do not
  authorize substituting another creature's mover.
  Retail near-pass `FUN_00459000` builds a tapered screen-space quad,
  `DAT_004c5268` queues when the combined outcode is non-zero, and
  `FUN_0047AA20` thunks to `FUN_0047EF10`. The port constructs and clips that
  path headlessly and fills live by painting those pixel corners in a
  HUD-style ortho with window Z matched to the 3D pass. World-triangle
  submission through the entity modelview stretched the ribbons; view-space
  3D `GL_LINES` produced full-width cables. Palette `0x02` hairlines use
  `FUN_00458c60` and fill as pixel-ortho lines.
  Live GL no longer prints `V2000_EDGE_FILL_DIAG` per-quad dumps. Do not
  invent world-space ribbons or a seventh newant type-14 pair. Closed
  live-only call-site bugs (2026-08-25): raw-local vertices fed to the
  world-space selector, and width projection against float world depth
  instead of 8.8 raw. Closed (2026-08-31): type-14 Sub-H export used
  absolute `i16 / 256`, so feet at world X ≥ 128 sat one period from
  type-0 hips and pixel-ortho `0x02` lines became cables. Closed
  (2026-09-01): that rebase still used a sign-extended i32 subtract.
  Level-1 newants spawn at `+0x9A = 0x7F00` (world Z 127); yawing a
  +4 world foot wraps the stored 8.8 word and the i32 delta becomes
  ~−254 world. Live `0x02` hairlines then wrap i16 screen endpoints
  into sky slivers while the type-13 footprint still plants, which is
  the moving-shadow / missing-body shot. Draw delta is
  `(short)(endpoint - origin)`. `0x02` near-table overflow drops
  instead of wrapping. Closed (2026-09-15): first-world body draw resolved
  Sub-H through a read-only `with_external_frame` copy and never committed
  `FUN_0041D360`, so ordinary Type17 spiders translated while type-14 `0x22`
  legs stayed at rest. Both world loops now use draw-owned
  `with_sub_h_presentation`. Opcode `0x22` size remains the raw stream short
  (factory6 45, not packed 2048). Remaining: matched-retail whole-world insect
  acceptance; Gouraud `factory2pillar` and wreck 225; newant type-13 ground
  blobs versus type-14 legs. See
  [RENDER_PIPELINE.md](RENDER_PIPELINE.md).
- **The Type-17 belly-up corpse pose is closed at its authenticated scheduler
  boundary.** Common-Dying launches the corpse, damps its horizontal velocity,
  drives pitch/roll toward an upside-down pose, publishes the post-DCA0/E870
  nine-dword Q31 basis, and feeds that retained matrix to ordinary world draw
  and particle/model collision. Fresh Level-1 Type-47 now attaches that same
  death family on the production hit/scheduler path: lethal 15040 is
  `FUN_00410C10`, already-dying 15040 is buffer-only `FUN_00414E90`, the
  10EB0 suffix plays `+0x80` only while not dying and still emits class 5,
  and a class-12 receipt retires live Guard/Wander/Pursuing before Common-
  Dying production. Live `0x07068805` keeps `FUN_00411400` bit `0x02000000`,
  so `FUN_00412DA0` skips randomized waits; class 12 still runs the E870
  owner rather than stalling at health 0 with class-5 smoke. Do not invent a
  Type-47 `FUN_0040DCA0` fork from Type-17. A killed newant therefore no
  longer continues ordinary Guard/Wander motion from a missing death owner. Level-1 Type-47 first
  `FUN_0041FCB0` is closed by `V200003.run` as `full_reset` for seeds
  `0x2B/0x2C/0x2D`; Chase and Guard wander apply that owner. Replay-level
  seeds `0x3C/0x3D/0x3E` stay fail-closed. Live Guard/Wander now also run
  the E870/DCA0 `FUN_0040E100` gravity/drag suffix before `FUN_00412DA0`.
  Guard styles `0x004C7BB8`/`0x004C7C00` have `+0x34/+0x38` zero, so
  effective flags are type `+0xC0` `0x2039`: bit `0x04` clear (gravity),
  bit `0x08` set (mode-zero drag), bit `0x02` clear (skip `FUN_0040DF70`).
  Skipping E100 let Sub-C lift accumulate into world Y ~50 while type-13
  WorldSurface still planted the moving stain on the spit and shoreline.
  Do not invent a stay-home Guard rule or apply Type-9's DF70 bit `0x02`.
  Live Guard/Wander also commits `FUN_0041D360` onto the six-record Sub-H
  runtime after the mover. Constructor flags stay 0; D360's `0x02`/`0x08`
  cache bits are the writer's copy-target and dependency gate, which is
  how a stride starts. Pass 13F70 Q31 into DF20 unshifted (`>> 0x13` is
  already inside D360). Do not invent a bounce. Live Intro2 Type-47
  Guard-anchor now commits the same D360 writeback after `FUN_00412DA0`.
  The older Level1 Type-17 Follow binding rebuilds `FUN_00413F70` immediately
  after Sub-D yaw; the [native Intro2 audit](INTRO2_TYPE17.md#matrix-and-world-frame-ordering)
  disproves that placement. Native Type17/53 retain the incoming matrix through
  C/A/B and rebuild only in DCA0/E870 after the task walk. Correcting the older
  Level1 adapter remains separate work. Intro2 Type26's six-record stag
  Sub-H commits D360/DF20 only when its actual model records are submitted
  during presentation, after the actor phase;12DA0 itself is not that writer.
  Restricted mode1 does not invoke1D0A0, while detailed component traversal
  does. Capture People/Run Away Type17 births stay out of the older Follow
  owner.
  Intro2
  Type-47 first query now applies that full-reset for seeds `0x06/0x07/0x08`
  and shared `FUN_0041F660` yaw for `classifier_flags` `0x13`, then the
  six-record Sub-H writer and authored C -> A -> B tail. Captured Intro2
  production publishes the class-32 Guard graph, applies the pending
  first-query Sub-D owner from Primary `FUN_00402EB0`, freshly visits Secondary
  acquisition, runs C7D0/ADE0 for an accepted Type-9 target, skips the newborn
  Primary Chase, and visits newborn Tertiary Aim in the same pass. Later passes
  visit Chase then Aim. Native Intro2 Type16/53/58/94 also retain their incoming
  matrix through the task mover and rebuild it in the world tail. The
  [Type53 pursuit/Aim](INTRO2_TYPE53.md) and
  [Type94 water actor](INTRO2_TYPE94.md) authorities own their task, hit and
  lifecycle admissions; unimplemented cohorts remain separate work.

### Player craft controls and presentation

- **Sustained VTOL Up+Space terrain death is authentic and
  setting-dependent.**
  - Status: **closed by static retail/demo audit and a matched retail
    self-righting repro; no tuning change.**
    Retail genuinely holds a shallower powered attitude: the accepted
    Self-Righting-10 trace settles near pitch words 19,269 for Up alone and
    10,469 for Up+Space. The port already implements and regression-tests that
    Space-only opposing-sixth term, including its one-frame force-basis delay;
    the submerged mode-launch change affected the post-Tab Hover attitude path,
    not ordinary VTOL pitch.
  - On 2026-08-09 the reported inversion and terrain crash were reproduced in
    retail after setting Self Righting to 0. The executable's pristine Self
    Righting value is 1. In both regimes the
    `controller+0x20C > 1` gate disables SPACE's opposing-sixth term; sustained
    Up+Space can cross pitch `0x4000`, make body-up Y negative, and drive lift
    toward terrain. Both outcomes are authentic setting regimes. The port's
    former per-data-root `config.json` explained the mismatch. Current
    [settings persistence](SAVE_AND_SETTINGS.md#port-storage-and-migration)
    preserves those imports; compare effective settings before comparing feel.
  - The opt-in `--vtol-trace` stream retains the effective setting and also
    records any terrain-contact normal/penetration, collision impact/damage,
    and post-contact hull state. It remains a regression diagnostic; do not
    change pitch, lift, damping, or contact damage for this closed report.

- **Skimmer/ground movement feels as though the craft is on ice.** Acceleration,
  coasting, turning response, and/or lateral damping differ perceptibly from the
  original.
  - Status: implementation now supported by a matched retail trace; fresh
    visual/feel acceptance in the port remains required.
  - The 2026-07-16 50 Hz keyed capture measured +26 raw velocity per forward
    tick and -27 per reverse tick. Neutral decay exactly matches the existing
    signed fixed-point common drag (for example -1923→-1918), while turning
    deliberately preserves most momentum. The slight forward/reverse
    asymmetry and much of the sliding are authentic x86 shift behavior. Do not
    add a second reverse-friction coefficient unless a future matched port
    capture disproves numerical parity.

- **Skimmer input semantics are recovered.** Up/Down moves only the side-gun
  elevation in style 0; SPACE/RSHIFT supplies planar propulsion. It does not
  tilt the body forward to move.
  - Status: corrected. The 2026-07-16 live trace also maps style 0 to skimmer
    and style 1 to heli/VTOL, so the Rust mode terminology is no longer an open
    ambiguity. Default arrow pitch now passes through retail's Relative-mode
    sensitivity quantizer: ±2303 at sensitivity 10 and 20 ms, rather than the
    previous unconditional ±3456.

- **Alternating twin side-gun presentation corrected in code.** The two
  side-mounted gun models were already present, but the port presented every
  bullet at the center of the hull. The high-frequency retail capture
  `runtime_re/captures/local/20260714-221605-twin-guns-elevation.jsonl`
  disproved the earlier "two simultaneous streams" interpretation: each
  trigger event creates exactly one class-1 bullet, one class-15 auxiliary
  command, and one sound, while selector variants A/B alternate once per actual
  shot. A sustained burst therefore forms two interleaved visible streams, not
  two bullets per
  160-ms event.
  - Status: implemented 2026-07-15; in-game visual acceptance still required.
  - The weapon phase now survives release/re-press and Enter/right-mouse
    handoff, matching the trace. Simulation/collision retains the proven entity
    center origin; only presentation retains the selected muzzle displacement.
  - Visible origins resolve the active authored pair through the live `player4`
    hierarchy: callback value 0 uses two `pl4gatgun` instances, selector-2
    callback value 4 uses two `pl4tubegun` instances, factory callback 15 uses
    two `pl4biggatgun` instances, and plasma callbacks 6/7/8 use the matching
    `pl4plasma*` pair. Each pair keeps its authored attachment transform and
    +Z extreme-centroid; no symmetric body-space offset is hard-coded. Unknown
    weapon branches remain fail-closed because their discharge policy is not
    proven by mesh shape.
  - Implemented 2026-07-31: the retained runtime joint words now follow the
    exact pointer map and decay recurrence. Variant A pulses PLAYER4 callback
    word 3, variant B pulses word 2, and both proven descriptor rate-5 paths
    decay by `5 * (elapsed_us >> 4)` with a zero floor. Empty-ammunition
    attempts do not pulse or advance phase. Human visual acceptance can still
    name the neutral A/B sides, but no runtime or hierarchy guess remains.

- **Both rendered side guns now articulate with gun elevation.** Up/Down in
  skimmer mode affects only the shared side-gun barrel elevation: it does not
  pitch the craft or propel/translate it. The port applies the recovered angle
  at both authored `pl4gatgun` mounts, and the transformed model-local +Z axes
  agree with the class-1 direction basis.
  - Status: implemented 2026-07-15. The input semantics are established and do
    not need another capture. Any future trial is only for visual acceptance of
    the exact pitch limits/rate and alternating shot paths at minimum, middle,
    and maximum elevation.

- **Landing-gear and fan mode joints corrected in code.** In retail, style-0
  Hover deploys PLAYER4's gear/panels and turns its fan assembly horizontal;
  style-1 VTOL retracts the panels and returns the fan to its vertical-thrust
  pose.
  - Status: implemented 2026-07-17; in-game visual acceptance still required.
  - Type 46 Sub-O maps callback words 5/6 into `FUN_00420A50`. Hover writes
    target bits `{1,1}`, VTOL `{0,0}`, and the shared callback advances each
    retained u16 with retail's signed Q31 recurrence. The port now preserves
    those words across TAB and feeds both through the complete linked hierarchy.
  - Real `0X3XX.OVL` materialization proves word 5 adds the deployed panels and
    moves the sidepods, while word 6 already provides the quarter-turn fan and
    shutter mount bases. The former duplicate game-side fan rotation was removed.
  - Callback word 4 is a separate weapon/loadout selector. Mode changes now keep
    it at the default gatling-gun value rather than swapping both side guns to
    `pl4cannon` on entering VTOL.

- ~~**The shared fan RPM/audio envelope and its two dynamic loops were
  missing.**~~ **CLOSED 2026-07-18.** `FUN_00420A50/FUN_00420920` and the
  successful `20260718-045345-fan-audio-envelope.jsonl` acceptance trace prove
  that Space and right Shift use the same nonzero-throttle path and both keys
  together use the idle path. One retained delta-scaled fixed-point state now
  drives the visual fan and global-47 playback rate (1.5x to 2.25x), while its
  gain envelope continuously retunes persistent positional globals 47 and 31.
  The upper layer preserves retail's full-envelope creation call followed by
  `/6` steady updates; the lower layer remains fixed at 0.5x with
  `envelope/2 + 0x2000` gain. Generational handles reject stale reused slots and
  mutable updates retain loop phase while audible. In accordance with
  `FUN_0044C970/FUN_0044C810`, positional culling retains each logical record
  but stops its physical buffer; re-entry restarts the sample with the steady
  update gain. There is no EQ and global 38 is unrelated.

- **Static-object collision damage is only partially presented.** Retail sends
  one raw two-slot collision packet independently to the contacted static kind
  and the player hull; it does not share the player's already-filtered damage
  and does not model trees with an invented HP pool.
  - Status: exact kind-0 static delivery is live as of 2026-07-17. Kinds 1, 3,
    8, 9, 10, 11, 28, and 29 joined the same bounded catalog scheduler on
    2026-07-31 after their complete opcode closures became executable through
    the already-exact effect/audio/common-explosion/radial backends. Kind 27
    then joined after matched retail/demo C bound opcode 6 to the current chase-
    camera focus spring and the already-live full-frame cursor.
    The separate dynamic-entity radial computation and its mutation-safe
    bounded runtime adapter are live as of 2026-07-22; visual acceptance
    remains open.
    The port preserves the raw impact and decodes every Section-12 threshold/Q8
    slope; player type 46 keeps its strict channel-1 threshold of 6000. Kind 0
    now has its complete seven-channel profile, 1000-unit chance gate with
    shared-RNG-before-dedup ordering, cell-keyed FIFO, require-static
    cancellation, exact initial/500/600-ms actions, effect 18 and sounds 68/90,
    live terrain-type `|= 0x08`, and the static-first radial scan/falloff path.
    Kinds 1/8 now execute effect 18 plus sound 90 at time zero and burn at
    500 ms. Kind 28 reuses kind 0's exact 0/500/600-ms program with its own
    profile and gate. Kinds 3/11 execute event 30 plus sound 90 at time zero,
    effect 18 plus burn at 500 ms, and radial template `0x004C9878`; kind 10
    executes the non-radial sibling. The event-30 bundle preserves model
    header `+0x08` as an independent source extent from collision radius
    `+0x0A`. Accepted hits on already-burned kind 10 clear bit `0x08` and
    wrapping-increment the live Section-10 attribute, matching its exceptional
    immediate branch. Kind 9 now executes sound 59, its exact opcode-13
    pacing-scaled two-attempt class-79 direction-table scatter, and the burn
    write at time zero. The scatter preserves retail's minimum extent, cursor
    ordering, L1 offset (including the Y-for-Z bug), velocity scale, and
    first-allocation-failure short circuit. Kind 29 now emits one ordinary
    class-79 event plus sound 90 at half height, then follows the shared
    500-ms effect-18/burn/radial tail. Kind 27 now runs four half-second-spaced
    effect-18/sound-90 pairs, conditionally restarts the full-frame sequence at
    2.0 and 2.1 seconds when wrapped focus distance is `<=0x1400`, plays sound
    62, then burns and dispatches its exact `0x800/0xDAC` radial. The optional
    health-pack/radar run is audiovisual acceptance, not runtime authorization.
  - Runtime evidence: the 2026-07-17 100-Hz collision trace records kind-0
    cells changing terrain state by exactly `+0x08` and selecting model slot 1
    about 610--620 ms after the initiating contact (for example model
    444/radius 1224 to model 445/radius 360). This independently corroborates
    the recovered 500-ms plus 100-ms program rather than an HP threshold.
  - **Burned Level-1 tree presentation mismatch reported 2026-08-12.** Paired
    port/retail screenshots of the same bump-contact outcome show the port's
    sparse branch/support faces stretched away from the ground and apparently
    inverted, while retail shows an upright broad rooted charred stump. This is
    not evidence for a different terrain-state slot or replacement asset:
    Level 1 descriptor 24 is `[444, 445, 446, 447]`, kind 0, so bit `0x08`
    selects authored global model 445 `dbigtre3` in both the recovered selector
    and the accepted runtime trace. The high-resolution model itself contains
    six vertices/four textured triangles, one type-13 record, and the two
    authored components visible in retail: rooted-stump sprite 1427 and dark
    branch overlay 1449.
    Status: the reported stretch was the ungrounded alias pair — before the
    world tf-12 recovery, record 1 materialized at crown height and both quads
    collapsed into crown-height slivers. With the swapped `FUN_004340B0`
    family implemented (2026-08-24), the stump quad roots on the sampled
    terrain while the overlay quad lies along the ground; a static audit test
    locks records, sprite ids 1427/1449, two-sided planes, and full-sprite UVs.
    Matched visual recapture of the burned silhouette remains required; do not
    remap the descriptor, choose an unrelated model named `stump`, rotate all
    static objects, or reopen the closed 500/600-ms burn scheduler.
  - Runtime ordering preserves the retail list lifetime: a terminating parent
    remains deduplicated while its own radial scan submits children, then is
    retired before the next same-frame parent's radial returns. New radial
    children begin on the following scheduler update rather than inheriting
    their parent's saved FIFO traversal delta.
  - Projectile uncertainty is closed for the admitted exact families without
    adding generic static-damage admission. Primary classes 1 and 3 use
    identical-value records `0x004CBF70` and `0x004CBF88`, filtering as
    `[2,0] / [2000,0]`; ballistic classes 16, 30, 58, 79, 93, 94, and 95 use
    `0x004CC000`, filtering as `[1,0] / [2500,0]`. Their live F800 boundary
    runs F610, re-reads suppression, resolves the current Section-10 cell
    through the same-pass terrain-mutation prefix, submits inline, and only
    then frees the parent. All ten admitted static kinds may be submitted. The
    primary packet is positive only for unburned kind 9 (severity 2000) and
    kind 27 (severity 3000), both deterministic; the ballistic packet is zero
    for all ten. None of these exact packets consumes RNG or reaches kind 10's
    immediate mutation. Retrospective impact arrays are never replayed, class
    87 retains its separate static no-op, and unsupported descriptor families
    remain unadmitted.
  - `FUN_004566E0`'s dynamic half is no longer an RE unknown. Its live
    creation-order walk, raw `+0x08` eligibility gates, strict centre distance,
    inner/full and Q15 falloff scaling, zero-damage impulse acceptance, mass and
    flag exclusions, low-word velocity impulse, and checked-full versus
    unchecked-falloff delivery split are documented in `GAME_MECHANICS.md`.
  - The exact raw state domains, generic `+0x30` health/`+0x50` buffer, and
    mutation-safe live-list order are now retained. Entity state also keeps
    accepted-hit tick `+0x34` and the three distinct Section-12 audio selectors:
    accepted-hit presentation `+0x80`, death `+0x90`, and generic live-hit
    `+0x98`. Pure Rust transitions reproduce `FUN_00414D30`'s ordered
    `health + 1` cap, `FUN_00414E90`'s buffer-before-dying arithmetic, and
    `FUN_00410D30`'s exact unsigned `last_tick < tick - 10` cadence with its
    tick update before the final dying/sound gate, without prematurely
    committing callback side effects. Class-1 bullets are now separately
    identified as `FUN_00410EB0`: they stamp `+0x34` before checked delivery
    and request `+0x80` on every accepted surviving hit without that cadence.
    Primary entity-impact events now retain the exact class-1 packet, immutable
    birth-time source type/owner, and live post-update velocity; complete
    provenance maps fail-closed into the six-dword checked-delivery record.
    Their complete detached transaction now preserves exact channel-1/2
    wrapping impact summation, `FUN_0040DAC0` / style `+0x28` ordering,
    independently resolved reaction/damage helpers, optional post-reaction
    network request, and the final class-5 capability branch. Phase-tagged
    resumes are bound to a caller-supplied transaction identity and private
    monotonic action receipt. Accepted damage and accepted-hit sound each lead
    to a fresh cached-pointer identity sample before later suffix work. This
    prevents machine-side reissue and stale/cross-transaction completion; the
    external adapter must allocate transaction identities uniquely among live
    machines and deduplicate execution by receipt. Recoverable protocol
    rejections return the submitted receipt, so wrong-phase, wrong-kind, stale,
    or cross-machine routing cannot strand the issuing machine after the
    external action has already been journaled. Only an accepted completion or
    accepted durable block consumes it. The identity samples explicitly fail
    closed where retail would rely on a stale-pointer lifetime invariant.
  - The audited surviving class-1 path is live for Main Base type 6 and Working
    Factory type 66: their primary packet filters to 1800, their effective
    behavior-style impact slot and impulse are inert, their `+0x44` modifier is
    null, and generic/accepted sounds retain their independent order.
    Would-lethal hits still fail closed.
    Broader runtime delivery remains blocked on per-instance `+0x44` modifier
    policy and non-player death continuation. Player type 46 can redirect a packet to
    a linked type-63 entity and propagates packets through controller links;
    types 73/109 and 110 have separate interceptors. Default death invokes the
    current style's `+0x2C`: base/factory progression can restore health and
    clear dying, while Capture People variants own component cleanup.
    Approximating any of these with the port's `active` boolean, collision
    iterator, direct player-hull subtraction, or generic removal would be
    knowingly unfaithful.
  - The bounded dynamic adapter admits retained null modifiers and the
    empty-cargo type-46 identity context, synchronizes PlayerHull, and runs the
    proven Base/Factory progressive-death continuation. It preflights the whole
    live-list pass and discards every dynamic mutation/audio request when an
    in-range modifier, type-hit callback, remote owner, or lethal lifecycle is
    unresolved. Static children have already run at that boundary and the
    parent still retires, matching the enclosing opcode order. Remaining work
    is broader callback/death ownership plus visual comparison of burn
    particles, sounds, model-slot changes, and chained trees with retail.
  - Exact representable mode-1 callbacks are now separate from the unresolved
    damage programs. Section-10 kinds 2/4/7 apply the authored repair/fuel/shield
    operation before contact damage, preserve their strict thresholds and caps,
    mutate the shared terrain cell only when accepted, and reproduce each
    operation's distinct notification/sound contract. Other static operations
    still wait for their inventory or entity-action state rather than borrowing
    these three pickup rules.

### Main menu

- **Klaus's body clipped the animated flame/emblem.** Code and port GL
  coverage are corrected: his whole authored outer group is behind the
  emblem, even where individual body surfaces are nearer. The frontend uses
  validated opaque-group depth; ring ordering retains its current approximation.
  C090/D030 geometry now preserves raw zoom, signed Y division, reference-420
  centering, selected-frame aspect and the framebuffer corrections above 480
  lines. This removes float rounding and the repeated 640×480 layout mapping
  in tiers 2/3. Matched retail-frame acceptance remains open; other tier-2/3
  font/layout adaptation is separate. See [painter groups](RENDER_PIPELINE.md#authored-model-painter-groups)
  and [billboard geometry](MENU_SYSTEM.md#menu-3d-scene-camera-background).

- **Asymmetric textured menu props rotated with mirrored artwork.** The
  high-resolution Save face read `200V`, and the Exit door/arrow was reversed
  on the visible side.
  - Status: corrected parser/presentation split and user-accepted 2026-07-24;
    retain a full-rotation check whenever the menu model basis changes.
  - The source sprites are already authored correctly: `optexit` 1230/1231
    contains door-left/arrow-right artwork and `slopt` 1305 contains readable
    `V 2000`. Triangles retain
    `(0,0)`, `(Umax,0)`, `(Umax,Vmax)` because `pesnthut` sprite 1365 proves
    that exact upper/right half. Complete model quads use the ordinary full
    rectangle proven by retail handlers `FUN_0047EF10`, `FUN_0047F320`, and
    `FUN_0047F750`. Mirror opcodes repeat A/B/C/D with XOR-reflected refs in
    the same order. Save and Exit become screen-readable through a menu-root
    local-X presentation basis; reversing every quad's U instead moved Klaus's
    asymmetric wing masks away from the bone joints. `optexit` and `slopt`
    retain their separately authored reverse faces. Billboard, terrain, water,
    and UI mappings are separate and unchanged.

### World rendering

- **Foundational renderer audit: lighting, view commands and near rejection repaired.**
  Gouraud solids now retain the distinct near/far corner setup and pack RGB565
  after affine interpolation. Live `0x0B/0x0C` selection precedes faces,
  ribbons, child instances and painter groups; intrinsic inspection retains
  both sides. Per-child raw view input adapts the current float scene pose,
  so it does not claim bit-exact retail Q31 transformation. Separate
  model billboards and particles now use centre-depth indexed palette fog,
  including the separate Intro2 12..24 draw context. The 47 GL pixel checks
  and 91 paired port-scene frames pass; frontend controls stay byte-identical
  and distant stray light pixels disappear. Matched retail visual acceptance
  remains open; the shared shader retains its documented packed-colour limits.
  Original faces now reject as a unit when any projected corner is below raw
  Z 64, including mirrored quads, screen-midpoint dependencies and water.
  Frontend and world submissions retain independent coordinate units even
  with fog disabled. Retail integer projection and matched-frame acceptance
  remain separate limits. Terrain-cache validity now uses the same exact
  footprint as terrain/water selection, correcting stale boundary strips after
  fractional motion or small turns and covering scan-dimension/fog-width changes.
  Exact retail fractional-row geometry remains a separate boundary. Raw RGB555
  sprite decoding now restores the level-selection thumbnails in every tier
  and the low-tier copyright banner, with correct pixel width and zero keying.
  Intrinsic decoding does not close raw packed-light/fog filler parity.
  See the [owning audit](RENDER_PIPELINE.md#open-foundational-mismatches)
  for source addresses, affected assets, negative results and acceptance cases. The complete normal-tier
  census found no reachable `0x13/0x14` distance commands, so that dormant gap
  is not the next target.

- **Cyan fog fragments and early building disappearance.** The reported
  tree/fence patches reproduce in elevated port views. Indexed model fog
  previously added a full sky-colour contribution before half-additive or
  additive blending; retail reduces palette shade first and uses distinct
  material formulas. See [material-specific model fog](RENDER_PIPELINE.md#material-specific-model-fog).
  Normal entity drawing also used a yaw-rotated terrain-style rectangle,
  prematurely rejecting some Main Base/factory views inside retail's
  world-axis bounds. It now reuses the existing exact classifier, with the
  developer free camera explicitly retaining its compatibility policy; see
  [entity model visibility](RENDER_PIPELINE.md#entity-model-visibility).
  The accepted capture's parent fog context and active software planes both
  remain 13..21 cells. Increasing the global range or fading shadow opacity
  would not correct these recovered causes. Matched retail visual acceptance
  remains open.

- **Fence generated endpoints corrected; matched retail acceptance open.**
  The reported tall, needle-like stakes came from resolving tf-6 endpoints
  before their tf-12 inputs sampled terrain. World materialization now supplies
  that callback recursively for ordinary and spike fences, including mirrored
  variants. Intrinsic collision geometry and shooting damage remain unchanged;
  the user confirmed that retail's starting gun also breaks the pen. Exact
  provenance and the recovered type-13 linked dependency contract are in
  [world alias dependencies](RENDER_PIPELINE.md#world-alias-dependencies).
  Port OpenGL comparisons confirm short, leaning pen stakes and corrected
  ordinary village rails/supports. Menu/Klaus controls and the unaffected
  gameplay scene remain byte-identical; matching retail poses is still the
  separate visual acceptance gate.

- **Peasant walking directions and raised-hands help become stale.** The user
  reported incorrect animation during ordinary gameplay and after beam
  release. The port reproduction froze a walking selector while Attract was
  active, then retained forced-stop after returning to Wander. The recovered
  fixes are cue destruction and next-visit view-detail publication, with exact
  scheduler animation custody. All authored direction/help/death branches
  have canonical asset tests and production GL frame inspection; matched
  post-fix retail gameplay acceptance remains open. See
  [peasant animation ownership](ACTOR_RUNTIME.md#peasant-walking-and-attention-animation).

  The subsequent repeated-help report exposed a separate acquisition bug:
  behavior target handles were interpreted as search radii. The candidate now
  uses the original common-axis component and preserves behavior context words,
  allowing a villager to reacquire the player after a previous follow task.
  The live regression covers that repeated transition without an invented
  sound cooldown; occasional one-second repeats also occur in retail.

- **Hover hull and industrial buildings are too bright.** The user's retail
  and port screenshots show first-world gameplay at the same Active Camera
  setting (about 6). Static inspection found that `C3/C4/C7/C8` uniformly lit
  faces had been decoded as unlit, forcing sprite row 28 instead of the
  face-normal Section-6 lookup. The parser and renderer now preserve this
  third lighting family, including zero normals and genuinely unlit details.
  See [face shading](RENDER_PIPELINE.md#5-unlit-uniformly-lit-and-gouraud-face-shading).
  Tree variants mix these families: `bigtree3` receives normal-derived light;
  `bigtree1`/`bigtree2` keep their authored fixed-row trunk materials.
  Port GL smoke covers those trees, craft/buildings, menus and Klaus handoffs;
  the same-pose building comparison darkens the affected faces while retaining
  identical sky pixels and explicit texture-row controls.
  Matched post-fix retail acceptance remains open; the screenshot pair has
  different craft headings and scaling, so it cannot establish exact texel or
  chase-pose equality.

- **Camera-facing 2D sprites are a core world-rendering primitive.** The
  original uses billboards extensively for foliage, fire, smoke, glows, trails,
  and explosions. Tree crowns visibly turn to face/follow the camera because
  they are billboards; the planar trunk quads keep their authored identity root
  orientation (`FUN_0042F650`/`FUN_00427090` pass position only), and their
  bases are grounded by the world tf-12 handler swap recovered 2026-08-24
  (see [RENDER_PIPELINE.md](RENDER_PIPELINE.md)).
  Do not yaw static-object geometry toward the camera.
  - Status: shared material/render path corrected; visual audit still required
    across all model/entity/effect render paths.
  - This is also present in the extracted data: for example, Section-8 model
    `bigtree1` contains four ordinary triangles plus three textured billboard
    primitives using sprite 1433. Preserve the mesh trunk/support geometry and
    render each authored sprite as a camera-facing quad with its own anchor,
    angle, scale, transparency, depth test, and blend mode.
  - Implemented 2026-07-14: Section-8 model billboards now resolve the source
    sprite's authored shade row and low-byte material flags. Foliage such as
    `bigtree1` uses masked rendering with world fog, flag-`0x08` sprites use
    retail's `source + destination/2` operation, and glow effects retain
    additive `ONE/ONE` blending without adding the fog colour twice. This is a
    shared renderer correction and therefore applies in gameplay as well as
    Intro2.
  - Implemented 2026-07-14: masked billboards now render before translucent
    attachments and write depth only for alpha-tested visible texels. This
    prevents later water and farther crowns from drawing over foliage. Shared
    world sprites instead preserve one stable back-to-front queue across all
    material families; only masked records write depth.
  - Implemented 2026-08-09: packed immediate operands now apply
    `FUN_00470840`'s bit-`0x40` complement after mask/rotation. This fixes
    `bigfuel`/`chernobylreactor` glow animations whose size expressions were
    previously wrapped into 65k-unit values; drawing and diagnostic hierarchy
    bounds now receive the same ordinary positive extent.
  - **World type-13 callback policy closed and implemented 2026-07-22; visual
    recapture required.** The default `FUN_0046EB60` / `FUN_0046EBD0` /
    `FUN_0046EC60` family really does preserve view X/Z and force absolute view
    Y to signed `-32767`; it remains a separate explicit context. Applying it
    to broad world draws caused the black/teal tree, dragon, and village strips.
    They were authored type-13 faces under the wrong callback, not a generated
    cast-shadow mesh.

    `FUN_00433FA0` installs the actual world family `FUN_004349C0` /
    `FUN_00435090` / `FUN_00435780`. It samples an initial surface, applies one
    X/Z correction from the source-to-surface vertical delta, then resamples
    once for endpoint Y. General/above-water samples use
    `max(bilinear terrain, animated wave)`; deep-underwater samples use seabed.
    Inclusive `sea +/- 0x96` is a rejection band, not a length cap.
    `FUN_00433BD0` derives camera-independent Q1.31 slopes from Section-10 and
    saturates each magnitude at one; Level 1 yields `(+0.25,+0.25)` from its
    reduced `(-18,73,-18)` direction. Camera movement changes final projection,
    never that world direction.

    Broad gameplay and Intro2 draws now use explicit `WorldSurface` projection.
    The parser retains mixed and all-type-13 faces in the canonical face stream
    with their materials/UVs/normals/shading/cull policy instead of splitting a
    synthetic shadow layer. World projection keeps each face's authored
    material/blend/depth behavior and uses `LEQUAL` for potentially coplanar
    projected geometry rather than imposing a type-based read-only underlay.
    Only the legacy `CameraFacing` mixed-face compatibility path retains that
    exception. Real `bigtree1` data still proves the authored support/crown
    composition (sprites 1448, 1402, and billboard 1433); matched visual
    acceptance remains. The existing type-13 WinDbg generator hooks only the
    default family and cannot settle this world path, so do not rerun it for the
    former rods or replace the proven policy with endpoint tuning, a model
    whitelist, or an arbitrary stretch limit.
  - **Ordinary textured-face blend flags implemented 2026-07-18; visual
    recapture remains required.** Decompiled callback `FUN_0047EF10` is the
    Section-8 textured-primitive filler selector used by ordinary body faces as
    well as the billboard queue. Its sprite-entry flag tests select additive
    for `flags & 0x10`, `source + destination/2` for `flags & 0x08`, and masked
    otherwise.
    The port previously retained that selection only for billboard/world-sprite
    records and rendered ordinary sprite-backed model faces as opaque. The live
    body material now carries the same selection into GL: flag `0x08` uses
    `ONE/ONE_MINUS_SRC_ALPHA` with source alpha `1/2` to reproduce that
    equation, additive uses `ONE/ONE`, and both leave depth read-only; masked
    faces remain depth-writing, including under `WorldSurface`. Potentially
    coplanar projected type-13 faces use `LEQUAL` without changing that authored
    material policy. Only the legacy `CameraFacing` mixed-face compatibility
    path suppresses writes by type. `bigtree1` proves the relevant split
    directly: support sprite 1448 has flags `0x0D`, while complete-tree sprite
    1402 and crown sprite 1433 have flags `0x05`. No face reordering was
    introduced.
  - **Section-3 coverage and fixed shade corrected 2026-07-19; visual
    recapture required.** Flag bit `0x01` selects palette-index-zero keying. A
    clear bit makes index zero authored opaque colour, so globally discarding
    it cut holes through the hovercraft and main base. Flag `0x04` advances the
    fixed textured filler by `0x380` bytes, selecting palette row 28 rather than
    the brightest row 31. Indexed model textures and RGBA sprite decode now
    share the zero-key rule, while flat model/HUD materials use row 28 and
    Gouraud faces retain their interpolated Section-6 row.
  - Implemented 2026-07-17: the separate type-14 callback used by static
    terrain-object draws. Retail callback `0x00427050` ignores the authored
    operands and returns `[cell-centre X, absolute world Y=0, cell-centre Z]`;
    that world anchor now propagates through the full static model hierarchy.
    This does **not** invent or alter tree trunks: `bigtree1` has no type-14
    vertices. Its authored type-13 body/crown composition now follows the
    recovered world-surface policy above and still needs visual acceptance.
  - Implemented 2026-07-14, visual recapture required: flat Section-8 entity
    actors are cylindrically camera-facing. `man2` and `lev1sci2` are not 3-D
    characters or Section-8 billboard commands: each is one textured XY quad
    (two triangles) plus type-13 footprint vertices. The renderer now detects
    that structural actor family at entity submission and replaces authored
    heading with an upright camera-facing basis. Left-handed world cameras
    also reflect local +X at that submission so implicit sprite U tracks
    screen-right; parsed UVs stay authored. Do not treat `Camera.left_handed`
    as a flipped map or compensate with a heading/UV rewrite; see
    [RENDER_PIPELINE.md](RENDER_PIPELINE.md#port-live-world-camera-adapter).
    Flat world props such as fences retain their authored orientation.
  - Do not replace these with generic 3D meshes or fixed crossed planes merely
    because that looks less obviously like a billboard; the camera-following
    appearance is original behavior.
  - **Insect leg edge quads: construction/LUT recovered; live pixel-ortho fill.**
    Headless gate and `DAT_004c5268` are in `v2k-render::edge_quads`. Live GL
    paints constructor screen corners in a HUD-style ortho
    (`LIVE_MODEL_EDGE_QUAD_FILL` / `LIVE_MODEL_EDGE_LINE_FILL`) with window Z
    matched to the 3D pass. World-triangle submission through the entity
    modelview stretched the ribbons; view-space 3D `GL_LINES` produced
    full-width cables. Palette-style `0x02` hairlines use `FUN_00458c60`.
    Type-14 export rebases 8.8 endpoints onto the draw origin with a wrapping
    short delta. Parked rest-pose dumps submitted 20 quads + 14 lines with
    `clip_dropped: 0`. Live walking at the signed-8.8 Z seam must not wrap
    `0x02` hairlines into sky slivers. Tracked in
    objectives/04.

- **Water texture fidelity.**
  - Status: visually verified by the user on 2026-07-14. The indexed path
    distinguishes the partial shoreline zero-index dry mask from the solid
    full-water tile, so its decorative zero texels no longer punch holes to the
    seabed. Preserve this behavior while changing fog or terrain coverage.
  - Underwater projected-vertex refraction is implemented as of 2026-07-17;
    visual recapture remains. The renderer uses the strict retail below-sea
    gate and shares one exact post-perspective integer wobble table across
    models, terrain, water, model billboards, and world sprites. Screen-space
    HUD/cinematic layers are deliberately excluded. Compare a stationary
    underwater shot over several 50 Hz ticks and verify that all world layers
    move together without changing depth/fog or leaking into the next menu.

- **Gameplay fog begins too far from the camera.**
  - Status: **visually verified by the user on 2026-07-14.** Three read-only
    runtime timelines show the captured 32x21 retail world installing exact
    13..21-cell planes during active draws. The port ends fog at each level's
    capped terrain scan depth and begins it eight cells earlier, while
    preserving the original per-vertex byte plus affine interpolation order
    and hidden widescreen coverage.

- **Gameplay camera composition is off.** The camera looks slightly upward at
  the craft and can clip into the ground.
  - The 2026-09-07 report also exposed a separate lens mismatch: gameplay used
    generic 60° instead of the selected system-2 focal lengths (50.22967° at
    the normal tier). The authored lens is now applied on construction and
    viewport changes. Native/Stretched retain different horizontal coverage;
    the existing chase equations below are unchanged. See
    [authored world projection](RENDER_PIPELINE.md#authored-world-projection).
  - Status: **retail behavior closed; matched port visual smoke remains.**
    The simple `5*dt` blend has been replaced by the signed-word
    `FUN_0040F3A0` eye/focus springs. `FUN_0040ED10` biases focus 0xFA raw units
    along the tracked body's stored forward vector (not entity velocity) and
    enforces its 0x100 minimum forward separation. Active gameplay now also
    uses `FUN_0044FFA0`'s full terrain-aware eye target: exact sea/terrain
    anchor branches, half-cell ray steps, three-cell-wide height probes, and
    Section-9 static-model collision-radius clearance (with object kinds 11/28
    excluded as in `FUN_00436C30`). Null-context menu/cinematic callers keep
    their separate craft-Y target.
  - The accepted 2026-07-30 Active Camera 0/6/10 traces close the remaining
    retail-policy question. Dynamic `FUN_0040ED10` parameters 3/4 remain zero
    in all three runs, so the setting changes horizontal chase distance through
    the recovered body-basis formula and adds no vertical lift. Joint-stable
    minimum/median eye clearances are 2/765, 450/777, and 330/806 raw at
    Minimum, Default, and Maximum respectively. Retail itself comes within
    2 raw units of water at Minimum; a blanket Y offset would be unfaithful.
    The later VTOL routes diverge, so use those values as pose-normalized
    envelopes rather than a point-for-point monotonic comparison. Recheck the
    port at the same setting/pose and reopen only a demonstrated projection,
    rear-coverage, or terrain-context mismatch.
  - Implemented 2026-07-17, visual recapture required: terrain, water, and
    static-object traversal no longer assume a constant two-cell front edge.
    `FUN_0042F270`/`FUN_00431890` use camera basis word 12 (the direct Q31
    forward-Y component built by `FUN_0040F3A0`) to move the scan behind the
    eye at steep pitch. The exact wrapping thresholds and discontinuity are
    shared by all three port paths, so craft-visible camera poses retain a
    rear buffer instead of exposing the clear colour. The GL yaw-rotated
    footprint and fog/FOV overscan remain compatibility envelopes around that
    exact scalar rule; retail's software loops themselves walk an axis-aligned
    map rectangle. If a matching recapture still exposes an edge, reopen those
    envelopes without tuning the recovered row lead.

### Campaign transitions

- **Secret Level-1 <-> Cistern route: runtime restored; visual acceptance
  remains open.** The accepted synchronized directory
  `runtime_re/captures/local/20260722-105044-secret-level-transition/` contains
  two complete round trips and proves that the transporter is an authored
  terrain marker, not a collectible.
  - Level 13 cell `(149,239)`, terrain kind 23, routes to Level 30. The captured
    Cistern arrival is raw `[7424,5120,9728]` with heading `0x4000`.
  - Level 30 cell `(21,39)`, terrain kind 25, routes back to Level 13. The
    captured return arrival is raw `[-27392,0,-5120]` with heading `0x4000`.
  - The live gates are generic type-111/model-16 effects materialized from a
    `levexit` terrain descriptor. Type 111 and kinds 22..26 do not identify a
    destination. Flag-`0x10` records in Section 13's `+0x64/+0x68` campaign
    array provide destination, signed-8.8 arrival, and marker subtype. The port
    decodes that generic format but activates only these two capture-backed
    secret cells.
  - Keep Section-10 model 39 (`levexit`) in the static draw pass. Retail renders
    that transporter shell independently of the live type-111/model-16 helper;
    the Cistern de-duplication pass removes three nearby helpers, not the static
    terrain objects.
  - Each gate continuously plays positional global sound 100, aliasing PCM 38.
    Contact adds no one-shot. The first cold load uses system overlay 51;
    cached returns do not display another menu.
  - **Cistern's 3:00 warning timer is the level controller's time-trophy
    deadline, not the hidden selector-`0x3F` pickup's intrinsic lifetime.**
    Section-13 `+0x5C` stores 180 seconds. Retail plays direct sound 51 every
    30 seconds at 60+ remaining, every 10 seconds from 59 through 20, every two
    seconds from 19 through 10, and every second from 9 through quotient zero,
    with rates 1.0x/1.125x/1.25x/1.5x respectively. Crossing into quotient
    zero still plays sound 51; the `+500` bias leaves a subsecond tail, and a
    later frame whose elapsed milliseconds reach the remainder plays direct
    sound 2 and moves the controller to expired state 2. Saving before expiry claims
    campaign time-trophy bit `0x8`; the hidden selector claims independent bit
    `0x2`. The port now owns the typed deadline, exact state initialization,
    discarded per-frame sub-milliseconds, endpoint-only cadence/rates, expiry,
    and in-session campaign bits `0x1/0x2/0x8` in the same controller storage
    whose Main Base abort commits state 5. Hive death now calls the completion
    transaction; native snapshots retain these bits. Operation `0x13F` now
    uses selector `0x3F` amount 1's shared counted-trophy/extra-life reward and
    audio. The recovered HUD state-1 gate controls the visible challenge:
    model 138 and its clock now draw while active and disappear together at
    expiry. This does not remove the independent hidden pickup. See the
    [HUD contract](HUD_RENDERING.md#time-trophy-and-countdown).
  - The same retail controller and exact fuel value `99119` survive each
    handoff while the player entity is reallocated. The port now preserves
    controller mode/fuel and controller-mirrored hull health, uses both authored
    arrival poses, and disarms a new world's gate until the player has cleared
    it. Entity-local buffer/dying state, pose, velocity, joints, fan state,
    runtime weapon, and target owner are rebuilt.
  - The cargo substrate itself is now exact: type 46's immutable Sub-J
    descriptor contains five zero-policy/zero-offset slots, and the new player
    entity owns a separate ordered runtime with outer policy `1` and capacity
    clamped to the raw controller unlock byte. HUD/save width remains that raw
    byte, not the clamped capacity. A new append clears child bit `0x800`,
    sets `0x20000000`, then publishes the relation. The remaining occupied-cargo
    capture question is cross-world allocation/identity behavior, not list
    layout, ordering, capacity, or append policy.
  - Visual/audio acceptance should verify gate orientation, loop placement and
    attenuation, exact contact handoff, both arrival poses, and safe re-arming
    after the player clears the destination gate. The component-relative
    vertical contact anchor and first cold-load overlay presentation remain
    unresolved; the current vertical guard is deliberately trace-bounded. The
    capture's conventional cargo list was empty, so occupied-cargo handoff also
    remains unproven.

- **Ordinary Level 1 -> Level 2 route: trace complete, blocked on live hostile
  death plus atomic Hive/results integration.** The accepted synchronized directory
  `runtime_re/captures/local/20260722-115649-level1-to-level2/` proves four
  type-17/model-256 objective hostiles at 5000 health and three
  type-47/model-302 hostiles at 3000. Captured primary hits remove exactly
  1800 health. The eligibility predicate requires live non-dying state,
  byte-`+0x64` bit `0x08`, and flag `0x01000000`.
  The port already owns that exact seven-hostile live predicate, the Hive's
  one-billion-health lock, its timer oscillation/two-second grace, and the
  2000-health vulnerability transition.
  - After the seventh eligible hostile reaches zero, retail waits exactly two
    seconds before exposing the type-67/model-341 hive at 2000 health. Lethal
    damage then moves it to dying model 343.
    `hive_death.rs` now retains the exact fail-closed continuation plan:
    class-46 variant 1, ordered component clears `[2,1]`, slot-0 callback
    `0x004260F0`, dying-slot-3/model-343-`+0x08`-extent shared
    scatter/surface burst with two class bytes `0x10`, and radial
    `(inner=0x200, outer=0x400, impulse=200, channels=[1,4],
    damage=[10000,8000], source=[type 67, handle])`. `WorldFx`
    exposes the shared burst separately from the player-only second sound.
    This is a pure plan for a caller-admitted Hive allocation, not permission
    to commit a partial lethal mutation.
  - The dead hive itself is the ordinary exit. No type-111 teleporter is
    born and contact has no sound. `FUN_0041BEB0` requires wrapping XZ
    distance squared `< 0x10000` and `dy < -0x80` below the wreck origin.
    In-world `452CB0` owns the completion statistics (`0xBD` world-saved
    time, optional `0xBE` time trophy, `0xBF` rescued, `0xC0` natives killed,
    `0xC1` aliens killed, plus later virus/rank/hidden-trophy rows). These remain
    over the live HUD. Only wreck entry opens the overlay-51 map; Space then
    continues to Level 14 (terrain 1, waves enabled), while S opens its save menu.
    The in-world hive HUD
    is separate: unlock is `0xD2` over `0xEF`, premature shot is `0xEB`, and
    hive death queues `0xE4` (`Fly down the hive to go to the next world`).
  - The port now applies the lethal entity transition (slot 3/model 343,
    class-46 variant 1), keeps shared `FUN_0041BEB0` on the wreck, queues `0xE4`, shows
    the authored `0xBD..0xC4` HUD statistics after their timer gate, emits the shared
    `FUN_00440950` burst and `FUN_004566E0` radial, and polls that wreck
    contact at the Sub-N `+0x54` origin after the `-6_000_000` suction delay
    against the decoded subtype-1 Level-14 arrival
    `[17408,2560,-32512]`, heading `0x4000`. Contact opens the progress map;
    Space or the save menu's Continue consumes the route exactly once. Map-S
    writes the full retained controller checkpoint through the canonical native
    slot codec into port-owned storage; it is not available at hive destruction.
    The [hive wreck authority](HIVE_WRECK.md) owns controller states 2/0,
    stopped class-5 spit, marker/timer admission, signed suction arithmetic
    and separate ring presentation. Source arithmetic and focused tests do not
    close matched ring appearance or interactive pull/exit acceptance. Live
    fall-in yank `FUN_0041c830` stays fail-closed.
    Overlay-51 sprites 3763/3764 follow `FUN_00454520` from overlay-3
    Section-14 record 3 when `FUN_0042edc0` bits 9..=15 are set.
    `FUN_0042DD10` selector 0 now writes those bits for flag-`0x10`
    records on live secret-pad contact; abort records and unowned gates
    stay fail-closed. The ordinary Peasant hive wreck is not a type-111
    stamp and does not unlock a column.
    Dying-slot-0 `FUN_004260F0` keeps shared `FUN_0041BEB0` on the wreck
    and ramps the Sub-K bound u16 on detailed ticks. Live `FUN_00425EA0`
    writes that word from a sine of Sub-K `+0x08`. Type 67 authors Sub-K
    `[1, 0]`, so `FUN_00424450` binds `FUN_0040a950(entity, 1)` at Sub-K
    `+0` and those ticks publish the u16 into `AnimVars.dynamic[1]`. Do not
    invent a hive1xa morph; the recovered consumer is this word-bank write.
    Live `FUN_00425760` and dying `FUN_00425790` both
    `FUN_00401020`-publish slot 0 after clearing slots 2 then 1.
    `DAT_004d1204` now places sprites 3733..=3762 at overlay-51 Section-1
    percents of overlay-3 S1 33. `FUN_00454c70` then draws sprite 3765
    (`DAT_004fe62c+0x3AD4`) at each visible `FUN_00454ff0` world-node origin.
    `FUN_00454460` blinks the current slot on odd `session+0x270 / 100`.
    Flag 8/0x20 extras blit sprite 532 (`DAT_004fe62c+0x850`) from the tile
    center plus overlay-3 S1 31/32. When bit 4 is clear, sprite 583
    (`DAT_004fe62c+0x91c`) is stretched over the 3765 rectangle.
    Hive death now records completion through `FUN_004567B0` without
    opening the map or forcing controller state 5. `FUN_00452CB0` draws
    the timed `0xBD..0xC4` statistics alongside HUD/radar and the fly-down
    hint when `+0x2BC > 500`. The ordinary dead-hive contact subsequently
    opens descriptor `004D0AA0`'s map/save prompt and retains the decoded
    destination until Space. Its real callback is `FUN_00455CC0` (sound 3,
    `+0x294=1`), not `004D0AD8`'s `FUN_00456170`/`+0x297` callback.
    The map freezes gameplay and pauses CD music; continuing loads the
    retained route. Native save writing and the complete closing/camera
    transition remain separate. Counter, type-on, map-sprite, and callback
    authority is consolidated in
    [Per-World Statistics](GAME_MECHANICS.md#per-world-statistics-confirmed)
    and [Campaign Exit Routes](LOADING_TRANSITIONS.md#campaign-exit-routes-runtime-validated-2026-07-22).
  - Do not expose the hive or transition from a port-only kill counter. The
    detached `FUN_00411030` impulse/jolt is now exact through its
    `0x04000000`/`0x08000000` state gates, unsigned-mass scaling, wrapping
    linear/angular commits, three shared-RNG draws in heading-roll-pitch order,
    and optional post-commit `FUN_00469200(1,10,&entity_handle)` request. Raw
    assembly derives every delta before committing velocity X/Y/Z and then
    heading/roll/pitch. It does not grant live-damage authority. The
    type-17/type-47 current-style impact policy now has an authenticated
    detached normal type-17 reselection planner, and entity storage retains the
    exact angular words while the primary-impact event retains direction and
    packet provenance. Run Away Acquiring now also has exact shared slot-1
    acquisition and strict-500-ms slot-0 retarget runtime families. A bounded
    live publisher now authenticates exact type-17 context, same-tick live hit
    stamp, model, and component evidence before RNG, then binds the selected context, slot-2 clear,
    slot-1 publication, Sub-H -> one-draw Sub-A suffix, and slot-0 publication
    to the same Entity and shared `WorldFx` stream. Its exact eight-record Sub-H
    allocation is Entity-attached. Follow Beacons' task mechanics are closed
    from matched retail/demo C: its distinct
    `FUN_00402120` task forces capability `0x100`, ranks signed `+0x88` scores
    with tie-only shared RNG (including zero-score ties), and synchronously
    hands a positive target to `FUN_0040C7D0`. Its `FUN_0040B740` setup retains
    exact slot-2 clear, phase-local Sub-H/one-draw/Sub-A suffixes, publication,
    and partial-failure ordering. The shared dispatcher also discards all stale
    callback outcomes after the handoff replaces slot 1 and does not revisit a
    newly installed slot 0 in the same pass. Follow variant one's detached
    `FUN_0040AFD0/00403B70/00403CE0` lifecycle is now closed too: ordered
    slot-1/slot-2 clears, prepare-before-replace primary publication, generic
    one-draw type-17 constructor effects followed by signed `base * 4 / 3`,
    mover-first target/route/proximity checks, four exact tagged singleton
    identities, surviving result-object tag/state-gate classification, strict
    `elapsed > 9000`, and synchronous self-replacement are retained in the
    shared dispatcher. The bounded live binding now retains type 17's
    actor-local descriptor copy and type 52's signed, type-specific spawn
    priority. Impact-selected class 33 publishes variant zero with both
    constructor suffixes on the authenticated Entity. Its repeatable,
    task-ID-bound callback lease snapshots candidate Entities in intrusive
    order, changes only actor policy `+0x04` to `0x100`, performs exact signed
    ranking/tie RNG, and synchronously replaces positive selection with the
    target-bearing variant-one context and Primary task. The generic
    constructor suffix precedes the fixed signed `base * 4 / 3` overwrite;
    failure retains the proven prefix and enters the outer fallback. Metadata
    and other actors keep `{0x0A00,3}`, and already-mutated actors remain valid
    for later impacts. The returned variant-one owner now drives mover-first `FUN_00403CE0` on
    type 17's A/B/C/D/H/J `FUN_00401430` bind and applies the `V200002.run`
    first `FUN_0041FCB0` full-reset for spawn-18 seed `0x32`. After Sub-D
    yaw it rebuilds `FUN_00413F70` so Sub-A follows the new heading. Fresh-Level-1
    production adopts the published spawn-18 graph and ticks that
    acquire/handoff path into the first following frame, then E100,
    `FUN_00412DA0`, and live D360 on the eight-record Sub-H. The live visit
    still fail-closes when the tracked target or unpublished birth Q31
    basis is missing. No focused capture is required. Retail/demo comparison also closes the
    outer failure result: C6B0/C6C0 installs the special fallback, zeroes state
    mask `0x00068000`, and clears slots 1 -> 2 -> 0, so the partial task graph
    is transient. Evidence failures remain zero-draw/no-mutation; a modeled
    initializer failure is the completed fallback outcome. Entity now owns
    exact named/unnamed descriptor identity, raw style index, active style, and
    preserved target/auxiliary context words; alternate class 12 remains
    audited but outside the weighted choice pool. Constructor selection no
    longer masquerades as a published current context before its fallible
    initializer resolves. Power Up is the sole statically infallible weighted
    initializer; fresh Level-1 evidence separately seeds only the post-Intro
    player and Main Base contexts. The exact fresh-Level-1 type-17 class-12
    publisher is now live at the real checked-damage death-callback boundary:
    it authenticates the issued `FUN_0040DB80` action against the nested
    child's private transaction/sequence/callback/target, fresh-resolves the
    allocation, and preflights health, selector, null-death context, metadata,
    and live A/B/C/D/H/J storage before mutation. Success publishes class 12,
    clears slots 1 -> 2, applies the one-draw Sub-H/Sub-A/+500-Y suffix,
    publishes primary last, creates the synchronous tail owner, and
    acknowledges the same receipt. Modeled primary preparation failure commits
    the exact fallback and 1 -> 2 -> 0 clears without an RNG draw; forged or
    stale evidence returns the action unchanged. Primary-impact particle-scan
    placement is complete. A synchronous receipt-journaled owner is now wired
    at F590 before Base/Factory fallback. For the exact published context,
    focused nonlethal/lethal tests compose reselection, reaction,
    checked damage, class-12 publication, selector-4 notification, sounds
    84/94, and the cached class-5 suffix. Natural fresh-Level-1 construction now
    runs the matched weighted selector and selected initializer/fallback before
    list linkage, so each actor sees only the already-published prefix. Full
    success consumes one selector plus two constructor words from shared
    `WorldFx`. Matched retail/demo construction plus Level-1 terrain close exact
    fresh state at `0x07468805`; direct starts remain masked. Exact retail
    `FUN_00422C10` / demo `FUN_00422AE0` close the
    candidate read to `state != 0 && (state & 0x5000) == 0`; the live snapshot
    now carries partial `RetailStateWord` evidence and stops only on ambiguity
    in those predicates. Impact-selected class-9 variant-zero
    `FUN_0040B6C0` publication is now live, including the pre-clear persistent
    actor-filter reset, Capture's style-`+0x44` override `0x0C00`, both
    constructor RNG/effect bundles, allocator-failure prefixes, and the outer
    fallback. Existing `20260712-203635` and `20260717-032945` natural cohorts
    show Capture/Follow/Follow/Capture and Follow/Capture/Follow/Capture for
    spawns 17--20, respectively, all at `0x07468805`. They validate variation
    from prior process-global RNG history, not per-spawn constants; no new birth
    capture is needed. Earlier unbound RNG consumers still prevent absolute
    reproduction of either captured cohort, and retail's post-selector context-
    allocation failure has no analogue while the Rust context remains an
    infallible inline value. Capture-People variants 2--5 nested
    materialization/cleanup and Follow variant-one scheduler/mover execution
    remain open; no new generic type-17 null-hook death capture is needed. The
    matched focused type-17 death
    captures `20260731-181510`, `181555`, and `181640` now validate the outer
    observed result without closing those owners: three 1,800-point hits take
    the clean victim `5000 -> 3200 -> 1400 -> 0`; lethal handoff adds exact
    state bits `0x00814000`, switches model slot `0 -> 1`, installs live
    class-12 style/task `0x004C7ED0`/`0x00404220`, and launches raw Y at +500.
    Class-7 debris begins only when the corpse later contacts the surface. The
    same corpse clears `0x02000000` and unlinks near 7.86 seconds before strict
    `>9000` completion; static E870 dispatch now explains that same-tick
    transition even though passive samples missed the transient mark. The
    detached class-12 common-dying
    shell is exact: ordered slot-1/slot-2 clears and fallible 9,000-ms
    primary install; descriptor-conditional zero/one/two-draw setup; +500 Y
    launch; terrain effect before the ignored-result common mover; wrapping
    X/Z-only damping; immediate mode-`0x9C01`; and strict post-unwind
    transition decision after the pre-callback elapsed update, with completion
    only at `> 9000`. The production scheduler now retains each authenticated
    fresh-Level-1 type-17/model-256 receipt and visits it in manager live-list
    order before physical particles. Every admitted visit requires a local,
    callback-enabled subject and known relation bit `0x1000`; that bit may now
    be set. The pre-callback reconciliation retains an authoritative local-
    parent row, releases an absent local row through Type 17's null hook, or
    follows a remote parent's position. A null or unresolved parent now
    preflights and commits exact retail/demo `FUN_004180F0` Type-93
    materialisation. It authenticates the tier-invariant Type-93 initializer,
    singleton class-30 choice, and zero-offset one-row Sub-J descriptor before
    RNG; a continuing callback then consumes its one selector word, while a
    coarse scheduler wait consumes none and leaves the relation unchanged. The
    successful transaction tail-appends an identity proxy at subject X/Z and
    bilinear terrain Y plus one raw unit, borrows the active model into slots
    0/2, releases and reattaches the subject, and emits no player-cargo sound.
    The newborn receives its first callback later in the same manager pass.
    Stable timeout clears the Type-93 row before its release callback, copies
    proxy position to the subject, restores master motion, applies the normal
    release/default-state map, clears the backlink, and removes the proxy. A
    set `0x02000000` then preflights the exact A/B/C/D/H/J
    topology, A/B/C/H descriptors, A/H runtime, terrain, and task lease;
    executes fixed-gain parameter-4-one terrain
    attitude against Type 17's signed selector zero; preserves the pre-effect
    basis for the null-target H -> C -> A -> B mover; and atomically commits
    pitch/roll, Sub-C position, disabled Sub-H state, and post-mover/damped
    velocity. The production owner plans and commits the
    `+0x70/+0x6C/+0x68` scheduler transition first. State bit `0x02000000`
    bypasses both randomized waits and consumes no scheduler RNG. Natural
    coarse visits conditionally draw from shared `WorldFx` in exact branch-local
    `+0x70`-before-`+0x6C` order. A wait retains task age and the receipt, clears
    `+0xB2`, and leaves `+0x68` and callback mass unchanged; a continuation
    dispatches with the carry-adjusted step capped at 125,000 microseconds.
    Unsupported suffixes are conservatively preflighted before those draws.
    Disabled Sub-H emits
    no cue and consumes no shared RNG. DCA0 then commits the full post-attitude
    nine-dword Q31 basis followed by its completion bit, E100, and the
    preflighted E370 timer/random-effects/lifecycle result. The
    solo ordered Sub-J tail then compacts stale rows or anchors valid children
    at the post-callback pose before `+0xB2` clears. Continuing frames run exact outer master
    motion,
    while an allowed
    strict-`>9000` transition clears `0x00040000`, skips integration, and is
    spliced by the same-manager-tick sweep before the particle snapshot. A
    lethal F590 publication later in the pass remains age zero until the next
    scheduler invocation; exact `+0x48` evidence now survives detailed frames.

    Before either callback, retail writes `+0xB0` as wrapping Type-17 authored
    mass 100 plus `+0xB2`, promoting zero to one. Accepted publication attaches
    the capture/static-backed common case `+0xB2 = 0`; nonzero offsets retain
    the general rule. A clear `0x02000000` admits the bounded terminal E870
    callback/suffix when that callback mass, the E370 owner/model bits, and
    timer `+0x48` are exact.
    Mode 1 commits task age and exact singleton/tag `0x004BE170/0x9C01`. A clear
    post-prelude `0x1000` publishes variant one, clears slots 0/1/2, and stages
    deferred destruction; a retained parent relation suppresses that transition
    while preserving the task and later suffix/master-motion work.
    E870 rebuilds and stores the full body basis, publishes state bit `0x4`,
    applies E100
    gravity plus Level-1 no-wind drag strength 3, commits E370's complete
    authenticated Type-17 path, skips master motion after its transition clears
    `0x00040000`, and reaches the same-tick sweep. Exact 2,000-ms deep expiry
    performs the matched relation release, writes `+0xC8 = 0x39`, clears
    relation `+0x80`, preserves model bit `0x4000`, and enters the zero-percent
    divisor-two bubble gate. A normal overshoot repeats the release but returns
    without RNG; the nested style release hook is null and the following
    standard-death call is a `0x4000`-gated no-op. In the deep random window, a
    bubble miss consumes one shared word and a hit consumes six, queues class
    42 from the post-`FUN_00413F70` forward axis times model extent 315 plus
    jitter, and materializes it before physical-particle traversal. Coarse
    scheduler draws precede E370; detailed visits have no scheduler draw.
    Authenticated model slot 1/state `0x4000` suppresses the sound gate without
    consuming RNG. Exact-match retail `FUN_00411400` / demo
    `FUN_00411390` now supplies one shared tight/full, expanded/broader, and
    coarse classifier. It retains wrapping X/Z words, full-dword camera Y,
    wrapping rear-plane multiplication, the original no-negative-Z-cutoff
    invariant, the `0x800` gate, and a masked `0x06000000` write. The later
    presentation pass publishes that result for retained Type-17 receipts, so
    it selects the next scheduler visit just as retail's scheduler-before-
    presentation call order requires. The full-detail callback and presentation
    attachment tail remains separate. Type 17's exact authored
    Sub-J descriptor (one slot, policy `1`, offset `[0,10,110]`) and entity-owned ordered runtime (capacity
    one, outer policy `0`) are now live together with their solo production
    consumer. Attached-child mass consumed by the callback's common mover uses
    the original row set; later stable compaction removes missing, exact-zero,
    and `0x4000` children, reuses vacated slot ordinals, and applies the
    conditional Q31 offset at the owner's post-callback,
    pre-master pose. Wrong/null backlinks are intentionally ignored because
    retail gates their reconciliation on multiplayer global `DAT_004F741C`.
    The direct update invokes neither row callback. Missing-parent Type-93
    recovery and its stable release are live; multiplayer relation/network
    repair and other-owner pop/destruction callbacks remain separate scopes.
    No focused capture is needed for the implemented contract. Authenticated
    Common-Dying Type-17 now supplies its full post-task matrix to presentation
    and particle/model collision; the collision projection refreshes its live
    membership, center, model, radius, and matrix after every damaging F590
    handler. Broader non-Type-17 basis ownership and class-32's owner-velocity
    snapshot remain separate.
    No scheduler-arithmetic, classifier, DCA0, bounded E870, or E370
    random/lifecycle capture is required. This exactness is branch-local and
    does not claim globally
    identical RNG history while other scheduler families remain detached.
    Static C now joins the
    focused victim's observed detail-bit
    clear to E870 mode 1 and
    `FUN_00410B70 -> FUN_00414990 -> FUN_004149F0`, retiring its prepared
    mark-caller capture. Two earlier ~3.1/4.1-second route actors lack equivalent
    selector history and remain an evidence-audit question. Capture People's
    nested cleanup is a separate focused-capture boundary. The F590 owner
    runs before Base/Factory and preserves retail's pre-impact
    callback/reselection plus impact reaction; actors outside its exact evidence
    contract remain deliberately fail-closed. The inert outer planner still
    rejects Capture's
    potentially nested dying initialization. Only after those runtime owners
    exist can the captured two-second gate and dead-hive exit be accepted
    visually.

- **Ordinary actor AI surveys and focused caller trace accepted; live ownership
  remains partial.** The
  retained `20260724-041815-friendly-ai.jsonl`,
  `20260724-041937-enemy-ai.jsonl`, and
  `20260724-042053-enemy-ai.jsonl` close the observed ordinary styles for
  types 9/17/47. Every stable locomotor slot-0 path still reaches
  `FUN_00401430`; focused transcript
  `20260727-235423-intro2-actor-task-mover.txt` now closes its caller arguments
  and task-transition order, while static recovery closes the internal
  decision matrix. No further broad AI survey is requested. Several movers
  are now live, but this does not establish complete behavior scheduling.
  The [Type-17 audit](SPIDER_BEHAVIOR.md) confirms all four spiders start
  inside the pen; static fence contact, Capture/Run Away adoption, Follow
  continuation, and the native capture/carry transaction remain incomplete.

### General

- **Additional small inconsistencies remain.** Keep a running timestamped list
  rather than fixing them from memory. A short clip or paired screenshots should
  be enough to open an item; exact cause is not required at report time.

## Evidence standard

Use three labels in future RE notes:

- **Static evidence:** what decompiled code/data appears to say.
- **Runtime evidence:** values or call sequences captured from the original.
- **Observed acceptance:** what the original visibly or audibly does in a
  reproducible scenario.

Static evidence may guide an implementation but may not close a visible,
control, timing, or audio fidelity issue. A mismatch reported from the original
automatically reopens any conflicting `DONE` or `ground truth` claim.

## Current implementation priority

The authoritative ordered queue is the
V2000 priority queue. Keep this
file as the detailed evidence/acceptance ledger rather than maintaining a
second order that becomes stale. The immediate action is the renderer
acceptance bundle; the remaining Intro2 camera, entity lifecycle/pair
collision, and native actor behavior follow in that order. The delayed
player-wreck burst was closed from the stable 2026-07-22 retail trace.

## Suggested comparison captures

Keep captures short and scenario-specific, ideally with the original and port
using the same level, heading, camera setting, and input duration:

1. Intro2 explosions: use the recorded 0:49--0:55 village impact as the first
   golden sequence. Compare meteor streak, overlapping bright sprites, debris,
   smoke, ground fire, animation lifetime, and the shot change.
2. Intro2 world presentation: use 1:00--1:10 as the second golden sequence.
   Compare the close tree billboard, shoreline/water, hive puff emitter,
   near-camera flying insects, typed captions, and blue distance treatment.
3. Skimmer feel: five seconds idle, two seconds full forward, release for five
   seconds, then a full left/right turn from rest.
4. Guns: front and side views while tapping and holding fire; repeat at minimum
   and maximum Up/Down gun elevation.
5. Mode/gear: pause on each stable vehicle form and record one complete switch
   in each direction.
6. Exit prop: one complete 360-degree rotation.
7. Water/fog/camera: paired stationary screenshots at the same first-world
   location and Display/Active Camera setting.

For control trials, a simultaneous input overlay or a spoken key cue is more
valuable than a long unannotated playthrough.
