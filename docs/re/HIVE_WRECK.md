# Hive controller, wreck suction and exit

This document owns the Type-67 live/wreck component, authored creature births,
player contact policy and descending-ring presentation. Damage admission and
the lethal transaction remain in
[ENTITY_DAMAGE_AND_DEATH.md](ENTITY_DAMAGE_AND_DEATH.md); campaign routing,
HUD messages and overlay 51 remain in
[LOADING_TRANSITIONS.md](LOADING_TRANSITIONS.md#level-completion-flow).
The original callback is `FUN_0041BEB0` in
`bulk_clean/game_logic.c`. The port's retained owner
is [`entity_emitters.rs`](../../crates/v2k-game/src/entity_emitters.rs), with
live admission and velocity publication in
[`entity.rs`](../../crates/v2k-game/src/entity.rs).

## Controller and retained components

`FUN_0041BEB0` controller word 0 has three states:

| State | Meaning | Class-5 spit |
|---:|---|---|
| 1 | Locked while the objective remains incomplete | Detailed callbacks only, strict authored interval |
| 2 | Vulnerable after the objective's two-second grace | Disabled; accumulator reset |
| 0 | Dead | Disabled; accumulator reset |

Unlock restores authored health and writes state 2. Lethal continuation writes
state 0 and zero health. This stops new class-5 spit; it does not erase particles
already allocated. Infection evolution has its own preceding objective and
interval gates and must not be conflated with particle emission.

### Shared live health prefix

`FUN_00415120` scans the live list for capability8, state `0x01000000`, and
no dying bit `0x4000`. It does not select hostile types, models, a world, or
an initial census. `FUN_0041BEB0` uses that result for every authenticated
live Hive. The earlier overlay13/four-Spider/three-ground-actor admission was
a capture-bounded port adapter, not a retail content rule; it is removed.
Multiple Hives retain independent controller/timer words and use their own
authored model slots and current Section12 `+0x14` health.

The source order is live Sub-K, infection, health/controller, class5 radial
emission, then the authored creature-birth walk below. State2 suppresses radial
emission and clears its accumulator on the same callback. The signed Sub-N
`+0x50` timer is also the word that
`FUN_0041CF90` writes to `-3_000_000` during alternate world-abort cleanup:

```text
if no_objective_hostile || timer <= 0:
    if timer < 2_000_000: timer = wrapping_i32(timer + elapsed_us)
else:
    timer = 0
if timer >= 2_000_000 || controller_state != 1:
    if controller_state == 1 || health > authored_health:
        controller_state = 2
        notify D2, resource E, and 456790; health = authored_health
else if health < 1_000_000_000:
    health = 1_000_000_000
    if detailed: notify resource A
```

Assembly `41BF9B..41C040` confirms signed comparisons and wrapping addition.
There is no second hostile check after the add: a single delta of at least
two seconds can cross the threshold even while the predicate is true.
State2 never heals ordinary damage; it only clamps excess health. Lock hints
depend on detailed callback mode (`param5==0`), including a first lock stamp;
the old exemption for previous health2000 was not a retail condition. Text
submission retains the caller's gameplay phase. `456790` changes a zero
completion timestamp to `-1` independently of that text phase, without
opening results or the map.

The port's live component requires the actual class46 live slot0, loader-owned
Sub-N and authored health. Its native Type15 creature-birth/ejection lane now
runs at that component's `13500 -> 12DA0` visit. Other birth types and missing
custody fail closed per record; the source forced-death/contact boundaries
below remain distinct.
The existing dying owner, abort transaction and wreck timer remain separate.
The guarded pair-impact lane below retains Sub-N `+0x44`; it does not turn
that latch into a synthetic damage packet or immediate dying-model switch.
The implementation is in [`hive_controller.rs`](../../crates/v2k-game/src/hive_controller.rs),
called between infection and radial in the retained component pass. Native
tests cover Peasant, Cistern, overridden later-world models, and multiple
Hives with independently paused timers.

The live class46 style `4C94C8` binds pair-contact slot `+0x18` to `4259F0`.
Its earlier opposite-capability `0x0C00` consume branch returns immediately;
only other actors with capability `0x2000` reach `1CE10` at `425ACA..425AD2`.
That consumer computes inward motion from wrapping signed position/velocity
words toward the authored Hive attachment. Strict `inward > 500` and
`i32(wrapping_u32(u16(other+0xB0) * inward)) > 75_000` write Sub-N
`+0x44 = 1` at `41CF7F`. There is no session-abort gate. `1CE90` is an
interior address of `1CE10`, not its entry or writer site. The
[impact policy](../../crates/v2k-game/src/hive_impact.rs) preserves wrapping
word deltas, signed low-word integer length, asymmetric `0x7FFF` saturation,
the wrapping dot product and both strict thresholds. The native pair adapter
keeps `0x0C00` consumption explicitly unsupported and admits no unowned
physical suffix. In the ordinary authored overlays13–48, the `0x2000`
counterparts are Type3/27 rolling boulders in overlays27/31/35. Their class20
construction, tasks, contacts, hits, deaths and player pairs are native
([Rolling Boulder](ROLLING_BOULDER.md)), but their pairs with other actors are
not, so real pair attempts return `UnsupportedHiveImpactCounterpart` before
callback writes. Corpus
controls retain both boulder types, the consume boundary and a null native
gunner callback. This is a guarded source foundation, not accepted ram play.

The following live `1BEB0` visit bypasses locked health when `+0x44` is
nonzero. After infection, its component death branch conditionally calls
`4568B0(3) -> 1D050(objective_hostile) -> 4567B0`: forced death while objective
actors remain skips this completion prefix. It then writes controller0 and
health0, plays the authored Sub-N cue at the current entity position, emits
`40950` scatter and `4566E0` radial at the retained `+0x54` anchor using the
currently selected model extent, and arms marker-backed `+0x48=-6_000_000`.
Spit resets and no state1 creature row is visited. `425EA0` ignores the
callback's health-zero return; this visit does not publish a dying model or
task. The next state-zero visit requests `10C10` only for a resolved entity
with nonzero state flags and clear `0x4000`, before its marker clock advances.
That later `25F60` initializer has its own current-position/dying-model burst.
The [component policy and controls](../../crates/v2k-game/src/hive_death/component.rs)
retain these two visits and the completion matrix; the complete forced-effect
host remains blocked with the unowned boulder route. The billion-health lock
therefore does not establish immunity to every retail contact.

Live `FUN_00425760` and dying `FUN_00425790` clear task slots 2 then 1 before
publishing slot 0. Both retain the component table. Live `FUN_00425EA0` writes
the Sub-K sine before `FUN_0041BEB0`; dying `FUN_004260F0` calls `FUN_0041BEB0`
first and then advances the bound Sub-K word only for a detailed callback.
The dying update first requires the existing unsigned word to be below
`0xD000`, adds `elapsed_us >> 8` with word wrapping, then caps values above
`0xD000`. An already-at/above-cap word is preserved. Type-67 Sub-K `[1, 0]`
binds this through `FUN_0040A950(entity, 1)` to `AnimVars.dynamic[1]`.

## Authored creature births and failed-world selection

`FUN_0041BEB0`'s `41C32D..41C546` walk is distinct from the class5 particle
plume. After health and radial work, it requires the **current** Sub-N state
to remain 1. Detailed/coarse callback mode does not gate creature births.
`FUN_00456CB0` reads the signed session-abort byte `+0x28F`: normal callbacks
select records whose flag bit `0x2` is clear; failed-world callbacks select
records whose bit is set. Casualty loss and Main Base loss share this byte
and the same `56960 -> 2F1A0 -> 170A0` entry; see
[Campaign Failure](CAMPAIGN_FAILURE.md). Failure preserves the Hive state.
An already-unlocked state2 Hive stays vulnerable and produces no creatures;
`1CF90` does not return it to state1. Its alternate callback writes the
live-health timer `+0x50 = -3_000_000` and clears the marker attributes when
present; it does not reset the creature timers, produced counts or child lists.

The canonical [Section-13 parser](../../crates/v2k-formats/src/levels.rs)
retains this per-spawn program as `EntityAnimation`: six raw header dwords,
then `0x1C`-byte records. `1BC20` copies the header to Sub-N `+0x1C..+0x30`,
allocates `count * 0x14` runtime bytes at `+0x18`, and initializes each runtime
record with the authored initial timer, produced count zero and an empty
intrusive child FIFO. The header's final dword is an on-disk Section-13 pointer,
not a gameplay count. The birth-record consumer gives these raw dwords their
meaning:

| Record offset | Consumer interpretation |
|---:|---|
| `+0x00` | Entity type for `FUN_00438080` |
| `+0x04` | Initial signed timer, microseconds |
| `+0x08` | Signed delay base, microseconds |
| `+0x0C` | Signed jitter operand |
| `+0x10` | Signed total-production cap; values at or below zero are unlimited |
| `+0x14` | Signed retained-child cap; admission requires count strictly below it |
| `+0x18` | Bit `0x2` selects failed world; bit `0x1` enters the objective constructor flag |

The [static corpus control](../../crates/v2k-game/tests/hive_failure_records.rs)
pins all four original Level-1 presentation tiers: spawn 24/Type67 has header
`[80_000, 5_000, 5, 5, 1, pointer]` and exactly one record,
`[15, 0, 2_000_000, 1_000_000, 3, 3, 3]`. There is no normal-world creature
record in this Hive. The failed-world row permits **three Type15 flyers in
total**, with at most three retained live children. Death does not refund
the total-production count. Other worlds consume their own authored rows;
these Level-1 values are evidence, not a type-wide replacement table.

Each eligible record uses signed `timer < elapsed_us`. Equality subtracts
the delta and leaves zero for a later visit. An expired timer first checks
the total cap; if permitted, `468D00` counts its FIFO. Only when that count is
nonzero and at least the retained-child cap does it prune nodes using
`16460` (missing allocation or dying bit `0x4000`). It then rechecks the cap.
A blocked cap leaves the expired timer unchanged and consumes no RNG; a
nonpositive retained-child cap therefore blocks births rather than meaning
unlimited. Ineligible rows freeze their timer. There is at most one attempt
per record per callback; overshoot does not produce a catch-up burst.

An admitted attempt zeroes the `0x4C` creation request, copies the Hive's
current position and record type, and passes record bit `0x1` to `38080`.
On success it re-resolves the child, writes the Hive handle to child `+0x60`
when present, appends the child to the record FIFO through `14930`, increments
the produced count, and calls `1C830`. This source field is distinct from the
attachment relation `+0x80`. `104B0` maps the supplied bit to child state
`0x01000000`; Type15 capability8 therefore satisfies `15120` while living,
including its temporary ejection interval with bit `0x8000` clear.
Constructor dispatch occurs before the delay draw. A returned constructor
error is dispatched and still resets the delay; the successful
child/list/count/ejection suffix does not run.

The reset consumes one shared RNG word and is exactly:

```text
timer = wrapping_i32(base + (wrapping_i32(low16(random) * jitter) >> 16))
```

PE `41C517/41C51E/41C521` uses low-dword `IMUL`, signed `SAR 16`, then `ADD`.
The Level-1 operand `1_000_000` consequently gives a signed high-word offset
around the two-second base, bounded by `-32_768..32_767` microseconds; it is
not a mathematical two-to-three-second interval.

`1C830` uses two retained temporary ejection slots. An already-recorded child
returns; if both slots resolve, it returns without a new launch. A free or
missing slot records the child and `500_000` microseconds, clears child state
`0x8000`, and copies the retained Sub-N anchor. It advances Z by the selected
Hive model's unsigned extent plus 100, then consumes three RNG words in order:
side/X displacement modulo that nonzero extent; signed X launch speed from
the next low 8 bits with yaw 0 or `0x8000`; and positive Z speed from the last
low 8 bits. Y is bilinear signed terrain height plus 400, vertical velocity is
850 raw units, and pitch/roll are zero. These are signed position/velocity
words, not world floats. Ejection's draws precede the record-delay reset draw.

The callback's final `1CA90` runs after the creature and contact walks, in
both callback modes. It subtracts the same delta from active ejection timers;
at equality or overshoot it restores bit `0x8000` if the child still resolves
and clears the slot. A missing child also clears the slot. Thus a newly
launched child's half-second timer already ages on its birth visit.

The health-before-birth order limits the conclusion about failed-world
invulnerability. For a still-locked Hive's first failure, an ordinary 20-ms
next callback keeps state1 and attempts the row's initially-zero timer; its
objective child can then keep the health gate locked. As a local arithmetic
control, a single delta of at least five seconds can instead cross directly
from `-3_000_000` to `2_000_000`, select state2, and suppress births before
the row is visited. Three total births and their later deaths also do not
establish a permanent lock. Killing the Hive after failure cannot save that
world: `567B0` requires session `+0x28F == 0` before its completion timestamp
and `2EE70` campaign-bit write.

The [record owner](../../crates/v2k-game/src/hive_birth.rs) and
[manager host](../../crates/v2k-game/src/entity/hive_birth_host.rs) now retain
this source order for native Type15 children. The
[native append](../../crates/v2k-game/src/entity/hive_child.rs) admits the
canonical Type15 metadata before body/process-counter/RNG mutation, constructs
the actual parent-position zero-instance body, and publishes the complete
class7 Search task graph through the
[flyer birth phase](../../crates/v2k-game/src/intro2_flyers_live/birth.rs).
The receipt binds allocation generation/id, with no synthetic authored index
or captured Sub-D seed. Native first Sub-D query retains the existing named
`NativeFirstQueryReset` policy, and fresh callback-mass `+0xB2` is explicitly
initialized to zero because retail leaves that heap word unwritten. These
replaceable native-birth policies preserve later writers but do not establish
retail allocator residue or count as capture acceptance. `10090`'s common
type-vtable `4C8A30+30` is null for
Type15; `438080`'s special type67/46/51 patches do not change it. Primary,
infected and cured hits reuse the complete current-style flyer owner, including
source46's actual selector4 notification after lethal objective damage.

Successful birth writes `+0x60`, FIFO/count and ejection before the delay draw;
`1CA90` ages the new launch in the same visit. The scheduler reads the live
successor after the callback, so a newly appended tail receives its first task
visit during that pass. Constructor physical basis survives `1C830`'s raw
Euler/velocity writes because that callback does not rebuild it. Unsupported
type/resources reject before a birth attempt. A defensive failure after
constructor entry retains the committed process/RNG prefix and parks the row,
rather than fabricating a native allocation-error return or replaying it.

Corpus-backed controls execute the actual `1CF90` alternate-cleanup prefix
against live Section10, then isolate the reached row/task lane. They cover
ordinary versus failed selection, native same-pass birth, three total children
with varied preceding RNG and 20-ms/large callback deltas, state2 and five-second
unlock suppression, half-second pair-bit restoration, primary hit and quiet
death. The production OpenGL aftermath checks in
[campaign failure](CAMPAIGN_FAILURE.md#level1-failed-world-aftermath) additionally
run both actual Level1 failure triggers through the exact `56960` sweep,
later Hive unlock, projectile death and current-level retry. They do not
establish an accepted Level1 casualty recording. Other creature constructors,
unowned active-pair/static-route callbacks and native allocation failure remain
explicit feature boundaries. No new capture is needed to establish the static
source/data laws above.

## Marker origin and timer

`FUN_0041BC20` initializes Sub-N `+0x54/+0x56/+0x58` from the entity position
plus authored X/Z attachment offsets; Y remains the entity origin. It searches
the surrounding 3-by-3 cells for static kinds 22 through 26. The first match
sets Sub-N `+0x4C` and snaps only X/Z to that cell's centre. Wreck contact uses
this retained origin, not the entity centre or a guessed model-hole position.
An absent marker does not enable the wreck's suction/contact walk. Sub-N does
not store the marker subtype or destination; normal routes use the actual
static-model contact stamp described below.

Marker-backed death writes Sub-N `+0x48` (controller word `0x12`) to
`-6_000_000`. Each admitted dead callback adds its elapsed microseconds before
walking potential contacts, even if no eligible player exists. The signed
dword addition wraps; if the result is strictly greater than `600_000_000`,
one subtraction of `600_000_000` follows. Suction and the abort-only interior request require the result
to be strictly positive. Detailed/coarse mode does not gate this timer or
suction. Disabling the component callback freezes its timer and contact work.

## Player suction and interior contact

`FUN_0041BEB0` walks the live intrusive entity list. A suction candidate needs
entity state bit `0x8000`, capability byte `+0x64` bit 0, a dead/missing hive
allocation, and the positive marker timer. `FUN_00425370` computes signed,
wrapping position-word deltas `dx/dy/dz` from the Sub-N origin and requires
`max(abs(delta)) + ((other_abs_1 + other_abs_2) >> 1) < 8000`.

Let `r2 = dx*dx + dz*dz`. The pull envelope is strictly `r2 < 0x190000`
and `dy < 3000`; there is no symmetric lower-Y envelope. Arithmetic operates
on the entity's raw signed position and velocity words, not world floats.
Define the callback's word delta as:

```text
delta(dt, shift, value) = i16((i64(i32(dt << shift)) * i64(value)) >> 31)
```

The shift occurs in a signed 32-bit value before the signed wide multiply.
The arithmetic right shift preserves retail's negative rounding.

- Inside `r2 < 0x10000`, vertical velocity approaches `-1000` by
  `delta(dt, 11, 1500)`. Velocity below `-999` adds that step; other velocity
  subtracts it. The signed word result is clamped against crossing `-1000`.
- If `r2 > 0x40000`, replace X/Z displacement with signed integer
  `(displacement << 18) / r2`, truncating toward zero and storing each word.
  At or below that threshold retain the original displacement.
- For each horizontal axis, subtract `delta(dt, 12, displacement)` when
  `velocity * displacement < 1`; otherwise subtract
  `delta(dt, 14, displacement)`. Outward motion therefore receives four times
  the braking coefficient used for resting or already inward motion. Do not
  reverse these branches or enforce artificial sign symmetry after rounding.

At `dt=20_000`, `dx=100`, and zero velocity, the result is `vx=-3`; initial
outward `vx=100` becomes `85`, while inward `vx=-100` becomes `-103`.
At `dx=-100`, resting `vx` becomes `4`, reflecting signed Q31 rounding.

The direct interior request additionally requires `r2 < 0x10000` and
`dy < -0x80`, but it is **shared world abort only**. At `41C7DC` the callback calls
`456CB0`; the `TEST`/`JE` at `41C7E1/41C7E3` skips `456D10` when that query is
zero. `456CB9` reads signed byte `session+0x28F`, shared by Main Base and
casualty loss.
The three-axis `< 8000` outer gate and wrapping signed-word Y difference
remain exact for that branch. The earlier interpretation as a normal campaign
exit predicate is withdrawn; normal mode must not manufacture a route from
wreck proximity or assume subtype1.

`56D10` stages the session's retained arrival and raises controller `+0x1F4`.
With abort already active, selector0's `42DE20..42DE43` branch returns the
current world `+0xC4` when that flag is set, before scanning authored records.
The port's distinct `FailedWorldRetry` carries that current world and retained
session XYZ to the existing map/loading owner, with heading4000 and no physical
marker, pair-link or exit-bit mutation. The selector handles it before authored
records only when the abort flag and interior request are both set; an aborted
world without that request still visits normal marker records. Completion time
and the saved-world transaction are both suppressed on failed Hive death.
Retained arrival, checkpoint and same-world reconstruction belong to
[loading](LOADING_TRANSITIONS.md#campaign-exit-routes-runtime-validated-2026-07-22);
matched warp animation/presentation remains a separate acceptance boundary.

Normal routing uses `427E20 -> 4464B0` on the player's real static-model
contact, followed by `42DD10 -> 42E300 -> 446440`. This applies to kinds22..26
without a model343 check. Authored Hive dying slot3 varies by world (for
example model585 in OVL16 and1188 in Cistern). Cistern's Hive is beside a
subtype3 marker, whereas its subtype1 marker is elsewhere. Neither Hive death
nor a positive suction timer creates a normal route; the contacted marker's
subtype and terrain state select the authored campaign record.

The separate live-hive player fall-in/ejection branch remains an unimplemented
boundary. Child-birth `FUN_0041C830` ejection and `FUN_0041CA90` restoration are
owned by the birth host above; neither replaces the wreck's suction law.

## Player and hive update order

`FUN_00413500` walks the intrusive list and calls `FUN_00412DA0` per entity.
That function runs the admitted type/component callback and integrates the
same entity's position before the walk advances. The later `FUN_00411A80`
contact pass does not own that position integration.

The accepted Level-1-to-Level-2 capture listed in the
retail ledger
retains player Type-46 handle `0x04BE0001` at intrusive index 2. Hive Type-67
handle `0x04A30001` begins at index 28 and remains after the player through
death (index 23) and wreck contact (indices 22/21). In that capture's
`world-behavior.jsonl`, baseline line 11 and death line 144355 establish the
identities and ordering. Thus player integration precedes the hive suction
write for this proven scene. Preserve that relative order; do not infer a
global task-before-all-integration ordering from the separate contact pass.

## Wreck ring presentation

The ring has a separate presentation owner. Its recovered draw program and
renderer submission contract belong here; model-mouth Sub-K animation and
the suction force above do not substitute for that render path.
The pure submission program is
[`hive_wreck_presentation.rs`](../../crates/v2k-game/src/hive_wreck_presentation.rs),
from retail `LAB_0041CB10` (`0x0041CB10..0x0041CE0F`).

`FUN_00438080` special-cases type 67 after successful construction: it copies
the nineteen-dword shared type vtable to `DAT_004DC690`, changes presentation
slot `+0x20` to `LAB_0041CB10`, then installs the copied table on the type.
The ordinary loader's null presentation callback is therefore superseded.
The previous port drew the selected hive mesh but omitted this suffix, so
opening the four `dh1flap` petals could not create the descending rings.

The callback first draws the ordinary body through `FUN_004138F0`. After a
successful return, it requires controller state zero and Sub-N `+0x4C != 0`.
It submits global model **243**, with its own `FUN_00421570` callback: selector
zero returns the global tick, selector one returns the ring's private shrink
word, and other selectors return zero. Body Sub-K words and body terrain shade
adjustments do not carry into these draws. Geometry remains the original
model asset; no particle or type-111 entity is allocated.

Ten sample indices `n = 0..9` are considered in order. Each uses signed
`time_us = SubN[+0x48] + n * 600000`; nonpositive values skip only that sample.
The phase is integer `(time_us / 1000) % 6000`. Consequently the first ring
appears strictly **after 600 ms** of the six-second death delay; nine rings
are admitted at timer zero and all ten at the first positive timer value.
Do not gate presentation on suction being active.

At the Sub-N anchor X/Z, the callback bilinearly samples the four signed
Section-10 height bytes with the two arithmetic-shift interpolation stages,
then adds 200 raw units. This is the ring's base Y; Sub-N anchor Y and the sea
surface are not used. For phase `p` in milliseconds, the height above that
base and the private model output are:

| Phase | Height above base, raw | Model callback output 1 |
|---|---|---|
| `p < 3000` | `3000 - (p / 3)^2 / 1000` | `65535 - max(64, (3000 - p) * 65535 / 3000)` |
| `p >= 3000` | `2000 - (p - 3000) / 2` | `65471` |

All divisions truncate before subsequent operations. Ring rotation applies
`FUN_00457C30` to a Q31 identity: odd samples use `(p*p) >> 8`, even samples
use `(-p*p) >> 8`. The even branch negates **before** the arithmetic shift;
negating the already shifted odd result loses a bit for nonmultiples of 256.
Only the low angle word enters the original sine table. Native consecutive
vectors are local axes, so transpose the Q31 basis once when submitting the
renderer's world-from-model rows; local +X at a quarter turn points along +Z.
Position words wrap before camera-relative unwrapping, and each ring retains ordinary model fog
and near/far clipping. Sampling and drawing consume no RNG and change no
simulation clock.

## Acceptance boundaries

Focused component tests cover controller state, timer admission and strict
bounds, signed suction branches, vertical clamping, toroidal deltas and Sub-K
publication. Corpus-backed tests retain the real Level-13 Sub-N marker and
verify player/render admission and that global model 243 consumes the private
morph word to contract its authored vertices.

The hidden OpenGL check uses normal-tier Level 13, an explicitly cleared
objective flag and the checked lethal transaction to create model 343. It runs
the production component and gameplay draw through nine seconds, inspecting
above and oblique views: rings build up and descend into the mouth, with no new
class-5 emissions. The unchanged above-view fixture is byte-identical to the
pre-ring renderer through 600 ms and first differs at 620 ms; submission counts
are zero at 600 ms, one at 620 ms, nine at six seconds and ten at 6.02 seconds.
The positive suction boundary is also visited. The oblique view must remain
inside the authored entity scan; an outside view omits the body and its suffix.

This controlled check holds the player position fixed to isolate callback
velocity writes and omits the initial death burst. It establishes production
ring visibility and timing, not a matched retail replay or full player physics.
Matched ring appearance and interactive pull remain comparison conditions.
Native OVL13/16/30 tests exercise their actual dying model, retained Sub-N,
player/static-model collision and correct campaign subtype without an invented
interior-route fallback. No new retail capture is needed to establish these
source-backed laws.
