# Shared Type58 actor

Section-13 spawn40 is the model273 actor at raw X/Z `BC00/0D00`, with
authored Y0 and zero heading/pitch/roll. Its native allocation, task graph and
component custody belong to
[`intro2_type58.rs`](../../crates/v2k-game/src/intro2_type58.rs).
[Actor runtime](ACTOR_RUNTIME.md) owns shared task semantics;
Intro2 acceptance owns the complete
scene and matched retail comparison.

## Native allocation and weighted selection

The normal-tier Section-12 record has model273 in all four slots, render
radius280, collision radius250, mass100, health7000, capability8 and default
C8 `0039`. Its topology is A/B/C/D/E/H, with six H records and no J. Common
axis words are `1000/0005` hexadecimal. A is `[1500,-3000,300]`, B is
`[10000,1000]`, C is `[50,75,3145728,100,200,0,0]`, and the no-pitch D
descriptor is `[96,0,0,512,200,13h,0]`. These are this actor's inputs, not a
Type16 or Type26 substitute.

104B0/09A80 calls H1D2A0, D203D0, A20450 and E24E30. Only A consumes a
shared word during component construction. D4A0 applies the authored terrain
snap before AC60 selection. E starts with zero cadence/counters, method20,
sound70 and null bindings; H uses terrain-only policy because C's surface
mode is0. The native receipt retains this allocation's D/E state across later
task selection and death. The port's explicit birth B2=0 policy is not a
captured animation contribution and is never reapplied during C690.

AC60/25680 evaluates only the already linked authored prefix at birth:

| Predicate | Weight | Class |
|---|---:|---|
| Always | 1 | 33 Follow Beacons |
| Furniture Nearby | 5 | 26 Trash Furniture |
| Player Nearby | 10 | 7 Search And Attack |

One shared low16 word selects the first cumulative weight strictly greater
than `(word * capped_total) >> 16`; the draw still occurs when only Always
qualifies. Rule8 `416650` passes the current signed common-axis word divided
by4 toward zero to4230C0. The untouched radius is therefore `0400`.
Rule6 `416550/4164D0/422C10` uses capability1 and the full live `1000` axis.
The query is a live actor-list predicate, not an assumed player position.

Successful Follow/Search birth uses four words: A constructor, weighted
selector, then two6030/6070 task suffixes. Furniture uses three: A, selector,
then its one6030/6070 suffix. Each suffix enables H, sets A direction1 and
writes `base + trunc(((low16 >> 8) * signed_base) / 0xA00)` before task
publication. Later C690 repeats selection and the selected task initializer
without reconstructing D/E, resetting B2 or drawing the A20450 birth word.

## Task and frame ownership

Class26 uses descriptor4C8948, style4C7738 and initializerB7C0. It clears
Tertiary, then Secondary, prepares1CA0/1D30 and publishes Primary only after
6030/6070 succeeds. A fallible task allocation retains the old Primary until
the normal C6B0 initializer-failure path. An unresolved port input is not an
original allocator failure.

1D30 writes target X/Z0, null tracked handle, filter-1, direction1 and reversal
timer0; target Y remains allocator residue. 1DA0 always scans while X=Z=0;
otherwise it draws once and scans iff unsigned low16 `< 0x0FFF`. It scans the
full current actor axis and resolves Y only after finding furniture. 4230C0
visits rings `r=1..trunc(radius/256)-1`, steps `s=0..2r-1`, and offsets
`[-r,-r-s]`, `[r,r-s]`, `[-r-s,r]`, `[r-s,-r]` in that order. It retains
coordinate low bytes, wraps the terrain grid, skips attribute0/damaged-bit8,
and returns the first matching static record. It omits the current cell and
outer ring. A failed scan still calls01430 with the retained target before
returning tag9C01; an unwritten Y remains an explicit boundary.

01120 adds `floor(dt_us/1000)` before1DA0. After exact wrapper unwind,
9C01 and strict age `>2000` select style+00 C690;9C00 uses+04, also C690.
01430's Boolean is discarded on the normal furniture path. Native Follow33
shares its acquiring and wander owners; Search7 shares target acquisition
and Chase/Aim. A new Primary waits until the next pass, while new Secondary
or Tertiary can execute later in the current pass.

Native12DA0 retains detailed/coarse gates, random waits, elapsed carry and
the125000-us cap. D yaw writes precede C/A/B using the incoming body matrix;
DCA0/E870 rebuild the matrix after the ordered task walk, then run the world
tail and master motion. H mode0 update belongs to that component traversal;
D360/DF20 cache writeback belongs to actual model presentation.

E's method20 descriptor has interval400000us, spread256, aim threshold16000,
axis tolerance2560, sound70, speed override0 and raw+12=150. All four binding
selectors are0; raw+12 is not a binding. The
[shared native ballistic host](../../crates/v2k-game/src/intro2_native_ballistic_aim.rs)
retains the same2300/24650 action and RNG order as Type16 through explicit
actor profiles. Restricted mode ages the wrapper before returning without
emitter/target/basis reads. Native E alone owns cadence. Method20's queued
11400 commands survive task replacement and class12; presentation drains them
in manager order into class52 at table speed3000. New particles first move on
the next simulation update. Final-card-only presentation does not drain them.

## Hit, death and surface policy

Common vtable4C8A30 dispatches primary10EB0 throughDAC0 and infected11250
throughDA00. Follow rows4C7B28/7B70, Trash4C7738 and Search4C7A50/7A98
have C690 at both infected+20 and primary+28. Search completion4C7AE0,
class12 rows4C7ED0/7F18 and fallback4C74F8 have nulls at both slots. All
these styles have null death+2C. Other actor styles are not admitted by this
cohort's hit adapter merely because their callback happens to be C690.

Primary stamps+34 before its style callback; infected commits the model bit
and optional cue first, without the primary timestamp or suffix. C690 transfers
its current or blocked-prefix task owner before11030 impulse and15040 checked
damage. Pending actor work rejects either hit before a new prefix. A zero raw
or filtered amount does not skip the already ordered callback/reaction.
Channel1 threshold2000 makes a2500 fragment return500; channel6 multiplier0
filters the infected `[6,0,2000,0,0,0]` delivery to zero. Surviving primary
damage can play cue84; capability8's class5 suffix still runs after lethal
damage, using the current model radius. Type58 has no infected/generic-hit cue.

Standard death publishes native common12 synchronously with sound75, retaining
mass/model/components and D/E/FIFO custody. Dynamic14AE0 damage shares that
death owner without substituting the primary particle entry or reselection.
The living surface lifetime is **20 milliseconds**: header+74 stores20,
162B0 adds `dt_us/1000` and subtracts it directly. E370 only takes this lifetime
path while the actor is below the sea-level-minus-quarter-model-radius gate;
non-deep visits decay the timer. At exact expiry,16750 release/default
restoration and10C10 death precede the bubble gate. Overshoot's unsigned
remaining percentage can return before that gate. This is independent of
common12's strict9000-ms task timeout.

## Late static contact

The [late dispatcher](../../crates/v2k-game/src/intro2_contacts.rs) visits
admitted meteor terrain termination and Type58 living/Class12 static contact in one
intrusive actor order after physical particles. It re-resolves each actor at
the current cursor, so an earlier meteor's damage and synchronous death-owner
publication affect a later contact visit. Meteor finalization retires its
matching scheduler receipt; a blocked death suffix keeps that receipt. A blocked
Type58 static callback stops its subsequent pair phase in both Intro2 and
Playing, while later allocations still receive their own late-contact visit.

The [Type58 contact adapter](../../crates/v2k-game/src/intro2_type58/contact.rs)
requires completed native task custody and the ordinary static-scan
eligibility bits. The deepest model contact retains its original plane/cell
through 12CF0/D920. Canonical type+8A is null, capability8 excludes27E20's
player pickup modes. Furniture's1CA0 task leaves its A8B0-dispatched+20 hook null.
C8 has no generic crush bit400, so only C890 submits
`[1,0,40000,0,type,owner]` to27950. Burned-kind10 cell changes apply immediately.
C890 then calls C690 even when filtering, duplicate registration or an empty
changed cell prevents damage; this is not a primary/infected particle hit.

The new task owner is retained before D920 re-resolves the actor for11760.
That tail captures velocity after reselection, applies the original contact
plane, and computes impact from the resulting velocity delta. Nonzero impact
damages the current static cell before checked actor damage. A resulting
common12 owner replaces the living owner immediately, including when a later
damage suffix blocks. Other committed failures retain a pending current owner
instead of replaying contact or task-initializer prefixes.

Follow33 acquiring/following and Search7 acquiring/Chase use the same02CA0
Primary hook. The retail installers write it at402B6B,4033C3 and403BDB;
their acquisition Secondaries and any coexisting Tertiary Aim retain null
static hooks. Follow/Search style+1C is null, so these contacts do not run
Furniture's damage delivery or C690 reselection.

The Primary hook reverses its direction, sets the reversal timer to2500, and
consumes two shared low16 words in X/Z order, each shifted right6 minus0x200.
Its typed private-state commit preserves targetY, tracked identity, task
clocks and the other task slots. Type58 has no Sub-I/F/G:019C0 sees its actual
Sub-A, and the hook preserves heading and the incoming body basis. Sub-D,
emitter state and queued shots also survive. The original selected contact
plane then reaches11760, including static-before-actor collision damage and
the common12 publication boundary. Unsupported graphs and pending/executing
owners fail before mutation; later failures park the owner of the retained
prefix, including any newly published common12 task. Search completion style
4C7AE0 remains outside the current Type58 live graph; this static extension
does not admit that transition.

Class12's404120 retains405FF0's null task+20, and4C7ED0 has null style+1C.
An authenticated CommonDying Primary therefore reaches11760 without any
task/private-state/Sub-A/RNG writes or living reselection. The same completed
manager/task boundary and static-before-actor damage order apply. Class12's
disable mask2015 produces effective28 from default39; it does not rewrite
the entity's default C8 or enable400 generic crushing.

## Late Class12 terrain and water

The [shared surface owner](../../crates/v2k-game/src/native_actor_surface_contact.rs)
runs11AD0's oriented12870 solid query before129B0 water classification. Its
private family policy authenticates native Type17/58 allocations and preserves
Type17's capture cleanup versus Type58's common death publisher. Model, mass,
position, wave policy and type-record cues remain actual per-actor inputs.
Type58's model273 has collision radius250 and mass100; solid/water/static cues
are0/91/0. Type17's radius300 and water cue27 are not substitutes.

Solid response retains the selected model and plane, commits separation and
material scatter, then velocity and checked collision damage. Water reads the
surviving pose with the retained entry radius, updates crossing bits and queues
cue91 before its null D860 style hook reaches141D0. Scatter and hard-entry
Type60 construction use the shared world RNG. Ring-construction failure still
runs sound17 and the source vertical-velocity suffix. Neither contact phase
ticks or reconstructs the CommonDying task. Late failures park that actual owner
and stop subsequent static/pair phases in both Playing and Intro2.

The [surface controls](../../crates/v2k-game/src/native_actor_surface_contact/type58_tests.rs)
and [static controls](../../crates/v2k-game/src/intro2_type58/contact_class12_tests.rs)
cover native worlds14/24/31/50, the strict radius250 water edge, solid-before-water
classification, sound/ring suffixes after rejected construction, seven invalid
ownership states, and parked prefixes across a subsequent contact phase. The
five existing Type17 surface controls also pass after the shared-phase move.
Release OpenGL controls in `.tmp/type58-class12-fixed` and
`.tmp/type58-class12-varied` render146 frames each, including native Type58
dry/water contacts and one real hard-entry ring in worlds14/50. The varied run
is clean; the fixed run retains exactly the same eleven Type122 reports as the
preceding Follow/Search smoke below, with no additional unresolved reports.

## Evidence and acceptance boundaries

### Shared ordinary-world construction

Ordinary `from_authored_world` and native Intro2 now publish the same104B0
constructor with a retained manager-generation receipt and their actual
`NativeSubDConstruction`. The normal-tier corpus contains seven ordinary
Type58 actors: four in14, two in24 and one in31. Weighted selection sees the
already linked prefix, including the persistent arriving player. Authored
heading, parameter bit, damage buffer and grounded+90 anchor remain instance
data. Native Sub-D uses the established empty, initially unpositioned cache
policy; it does not borrow spawn40's captured first-query receipt.

The separate replay entry still authenticates spawn40, X/Z`[-17408,3328]`
and seed19. Both entries share the H/A/E and task initializer phase, retaining
their own D state. Reselection and common12 preserve E cadence and its FIFO.
Living, aim, hit, surface-death and Main Base abort paths validate the issuing
manager and completed task custody; a foreign receipt or pending callback
cannot be adopted as a successful owner. Player-caused lethal hits retain
their feedback through the shared notification host.

The Medaeval regression constructs actual Type17 spawn28, publishes its
Class12 corpse, and contacts the actual Type58 allocation. This replaces the
previous `UnresolvedBehavior` stop observed at ticks667–964 in the varied
smoke (`.tmp/resumed-chain-final-varied/runtime-issues.txt`). Pair descriptor
dispatch includes Follow/Search02DA0, while TrashFurniture's component slot
is null:405FF0 writes template+18=0 at406013 and401CA0 only overwrites
offsets00/04/10/14. Its separate static callback remains C890.

Playing now uses the existing manager-order11400 projectile drain during
presentation, as Intro2 does. The old Playing path drained only Type13 then
Type47 before particle simulation; it omitted Type58 and advanced new shots
in their birth frame. The shared pass admits each retained family FIFO,
preserves replacement/death queues, and first moves new particles on the next
simulation update. Intro2's final-card skip remains its explicit scene policy.

Independent Type58 active-pair scanning is implemented through the shared
native-captor lane: a native Type58 subject's own 11AD0 pass visits later
counterparts in intrusive order, running the current behavior and A900
component callbacks before physical writes. The five admitted living styles
preserve C8=39's cleared0x10000 scan bit, so their ordinary11AD0
bare-terrain/water branch stays gated off; the earlier E370 lifetime/bubble
suffix is a separate phase. Class12 restores that scan bit through2015 and
uses the shared late surface/static owners above. Counterparts without a
native owner stay fail-closed at the existing UnresolvedBehavior/
UnsupportedBehavior boundaries and are never whitelisted; that is the
designed cutoff, not a walker gap.

The authored, aim, shared-impact, descriptor-pair and abort regressions cover
all seven ordinary births, changed process histories, all three initial
classes and exact constructor draw counts, foreign manager receipts, parked
task custody, C690/Class12 FIFO survival and lethal player feedback. The
release OpenGL checks in `.tmp/type58-shared-fixed` and
`.tmp/type58-shared-varied` each produced134 frames and no runtime issues;
the latter uses varied Intro2 frame durations and250 frontend ticks. They
include New Game, Intro2/final card, first-world handoff and the Medaeval
corpse-contact interval. These are port regressions, not new retail evidence.
The checkpoint passes `repo-check.ps1 -Mode Full -Scope All` (both complete
workspace suites, formatting, retail-data doctor and repository hygiene) and
`cargo build --release -p v2k-game`.

The Follow/Search extension adds native world14 controls for all four admitted
task graphs, exact two-word02CA0 state, coexisting Aim, original contact-plane
response, invalid ownership without writes and retained late-failure custody.
The release checks `.tmp/type58-contact-fixed` and
`.tmp/type58-contact-varied` each render134 frames. Varied-frame validation
has no runtime issues. Fixed-frame validation reaches the existing Type122
spawn21 presentation fallback: spider5 contacts it at ticks3804--3864, yielding
eleven explicit `UnresolvedBehavior` reports before counterpart/physical writes.
The old/new full logs share20356 identical lines; their first difference is
the newly enabled Type58 Follow/Search static response at tick1564. This
exposes the unconstructed Type122 body described in
Intro2 acceptance, not authority to
substitute a null callback. The fixed smoke therefore remains blocked at that
known native-construction boundary; it is not a clean whole-scene pass.

### Captured Intro2 allocation and contact bounds

The accepted read-only `V200001.run` join authenticates spawn40/seed19 from
its own203D0 allocation through the first1F660/41FCC4 origin read. At tick045A,
parent00401602 passes125000us and query X/Z `BDFF/0DC8`; that allocation takes
full-reset, produces origin `BD/0D`, class0, row0=1 and counter1A. Only this
own-birth receipt admits the pending reset; later cache shifts/fills and cadence
run normally. Constructor clock0CC3 is the global004FED60 value before428AD0
resets it, not evidence of reversed TTD order or another birth. The
capture ledger
retains the exact join identities, positions and filenames. Other actors'
receipts and generic unknown origins remain unadmitted. First furniture target
Y separately requires a successful own scan before any mover reads it.

The independent11AD0 active-pair walk is covered for resolved counterparts:
the static adapter admits the explicit Furniture/Follow/Search and Class12
task graphs, and the pair lane regresses the independent Type58 subject
against a Type58 counterpart plus the existing spider-counterpart coverage
including the Medaeval corpse pair.
Type26's late contact is not implied by shared movement components.
Nonzero gain-scaled type+8A contact audio remains outside this null-cue profile.
Fence convex collision commands `0x8C/0x8A/0x8D` exposed by scene validation
now run the source integer half-space program in the model decoder. The
format regressions cover model493 through sphere and moving-model queries,
plane normalization, group branching and acceptance/depth custody. Live
Type58 contact reaches model499 and completes C890/C690/11760 without an
unsupported-command fallback.

Focused fixtures cover native selection, task/RNG custody, emitter and hit
entries, common12,20-ms surface boundaries and the C890/11760 callback order,
including filtered/changed cells and pending/death custody. Hidden OpenGL
walkthroughs exercise these paths through New Game, Intro2, the final card,
closing Klaus and first-world handoff. Current combined validation is recorded
with the [native Type94 checkpoint](INTRO2_TYPE94.md); the previous Type53
Search7 and Type94 particle boundaries now have native owners.
Matched retail presentation and the complete world contact scan remain the
owning objective's acceptance gates.
