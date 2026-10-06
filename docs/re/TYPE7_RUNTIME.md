# Native Type7 diver worker

Type7 is the world-style6 worker selected by `FUN_0042EB70`. Its native
constructor and runtime owner retain the complete four-choice graph through
authored births, factory ejection and Main Base output. The shared task kernel
uses an explicit `DiverWorker` profile; Type7 never borrows a two-choice
worker receipt or joins the Main Base person-input catalog. Current priorities
remain in shared gameplay runtime.

This authority uses the canonical Rust Section12/13 parsers, PRELOAD and normal
tier1 overlays for every ordinary world13–49, plus the retail executable and
bulk gameplay source. No new capture is required by the
recovered contracts below. Related owners are documented in
[native workers](INTRO2_TYPE8.md), [native people](TYPE86_RUNTIME.md),
[actor runtime](ACTOR_RUNTIME.md) and [factories](FACTORY_SYSTEM.md).

## Canonical profile and corpus

| Field | Type7 value |
|---|---|
| Models | `[1249;4]`, `diver2`; model radius and collision radius180 |
| Body | Mass10, health1500, capability`0x1404` |
| Initializer | Default policy`0x2F`, rule1, alternate class14 |
| Components | A/B/D/I only; model-variable count1 |
| Sub-A | Acceleration1500, overspeed correction−3000, target-speed base250 |
| Sub-B | Projection threshold10000, correction rate1000 |
| Sub-D | Divisor32; yaw/roll coupling0, pitch steering0; probes512/256; classifier flags0, reserved0 |
| Sub-I | Capability8 cue0, mask201 cue0, attention-stop cue106, binding1, frames4 |
| Axis | Strict range3072, filter word`0xA1` |
| Root choices, in order | Always`(1,1,6)`, JobNearby`(13,200,54)`, PlayerNearby`(6,20,45)`, BaddieNearby`(7,10,10)` |
| Damage thresholds | `[0,4000,200,0,200,0,0]` |
| Damage multipliers, Q8 | `[0,256,256,512,128,0,256]` |
| Hit/constructor audio | Accepted-hit, infected-model, generic-hit and constructor attachment absent |
| Other audio | Death35; RunAway sound/period0; detailed healthy sound40, described below |
| Surface | Selectors`[0,0]`, lifetime0 |

There are exactly eight authored Type7 births:

| World | Spawn indices | Wind | Waves | Ground height / sea, raw |
|---|---|---|---|---|
|22 Water|25,26,27|Mode1, `[1000,0,0]`|Enabled|4064 /4096|
|34 DarkReef|12,13,14,15,16|Mode0, zero vector|Disabled|2336,2336,2336,2304,2272 /4096|

All eight have param0, zero Euler words/damage buffer/model overrides, and no
configuration or authored animation payload. All birth cells are wet and
uninfected. Model1249 is resident in all style6 worlds22/23/30/33/34. Factories
occur at22 spawn32 and34 spawn11. World33 has mode2 wind`[0,0,20000]`, but no
Type7 birth or factory; a controlled placement there is separate coverage.

## Constructor, tasks and lifecycle

`104B0` charges the common body stamp, `09A80` allocates D then A, and `20450`
consumes Sub-A's constructor RNG word. The wet comparison uses the caller's
tick and authored wave setting before `D4A0` grounds the actor. `425680`
selects from the actual ordered choices and already-constructed intrusive
prefix; the chosen constructor precedes linking and F70 basis publication.
The D allocation still owns its real process allocation seed. At `41F660`,
classifier0 jumps to `41FC90` without a terrain-cache query, while retaining
the stagger increment/row invalidation and `20260` steering update. No captured
worker cache origin or seed is needed for this query-free branch.

| Graph | Source contract |
|---|---|
|6 Wander|`AD10`, ordinary Wander Primary; one successful constructor suffix word |
|54 Go-To-Job|`AF90`, actual job target and Primary; one successful suffix word |
|45 Attract, acquiring|`BA40`: parity word, optional candidate Secondary, cue Tertiary and local-wander Primary; one suffix word per successful allocation, totaling3/4 branch words |
|45 Attract, target|Existing `AF50/03610` target-route graph; preserve actual axis/filter and target context |
|10 RunAway, acquiring|`B6C0`: acquisition Secondary and500ms wander Primary, two successful suffix words |
|10 RunAway, fleeing|`B3F0`, real target and authored absent optional sound |
|14 Exploding|`C3A0`,1000ms SharedRetarget; terminal `C470 -> A860 -> 410B70` retires physical slots0→1→2 and marks deferred destruction |

Class45's initial event16 is class behavior. BA40 reads the actual Sub-I
`+04` attention cue: Type7 plays106; 72 belongs to other profiles and is not
a separate fixed Class45 sound. Rule13 is `165C0/18EB0` JobNearby,
including free factory capacity and capability400. It is not the native
people's rule12 BaseNearby predicate. Type7 lacks capability800, so it is a
factory staffing worker, not a Main Base conversion input.

Class10 selection and candidate acquisition deliberately use different filters.
BaddieNearby enables its root weight with capability8, while the Secondary
acquisition retains Type7's authored axis filter`0xA1`. A capability8-only
hostile can therefore select Class10 without itself becoming the fleeing
target. An eligible capability1 player can become that target once nearby.

Live class6/54 and class45 target styles dispatch `C690` in both primary`+28`
and infected`+20` hit slots. Class10's primary slot is null, while its infected
slot invokes `C690`. Attract acquiring, carrying and class14 hit slots are null.
Type7's channel6 Q8 multiplier256 makes actual infection damage observable;
ordinary worker immunity must not be substituted. Standard death retains cue35,
the actor's own allocation/A/D/I and any committed damage or constructor prefix.

Living attachment selects variant1 through `CD50` for classes6/54, or variant2
through `CE70` for10/45; `CD70/ADB0` publishes carrying None. `20760` uses the
parent's capabilities/position and the child's cue descriptor. Both Type7
attachment cue words are zero. `CE90` release must reselect its full four-choice
root. Class14 attachment instead selects terminal variant1 and runs `C470`;
it must not be treated as a null callback. Parent Sub-J removal remains owned
by the surrounding relation lifecycle.

Detailed/coarse movement retains D→I→A→B, with coarse mode skipping I. The
outer F70/E100/DF70/master-motion phases remain. Surface selectors0 make E370
skip162B0, so Type7 has no ordinary-worker drowning/release/bubble branch.

## Detailed sound40

The metadata field named `low_health_effect_words=[40,0,0]` preserves header
words`+94/+96/+A0`; its name does not describe Type7's consumer. Actual values
are sound40 at`+94`, alternate0 at`+96`, period3,000,000µs at`+A4`, and zero
low-health sound/period at`+A0/+B0`.

Retail `40DD39..40DD46` compares health with signed initial-health/2.
`40DDB9..40DE6C` requires health≥750 and visibility bit800, then draws one
low16 RNG word and accepts inclusively when
`random <= (dt << 6) / (3,000,000 >> 10)`; the divisor is2929. `44F480` queues
sound40 at gain/rate1. Below750 health, or when hidden, Type7 queues no sound
and consumes no sound-gate word. Coarse E870 omits this DCA0 sound phase.
The shared [sound planner](../../crates/v2k-game/src/actor_detailed_sound.rs)
already implements this consumer; no effect40 emitter is required.

## Implementation and validation boundary

`native_type86` is the historical name of the shared class45/10/54/6/14 kernel.
`NativeFourChoiceProfile::DiverWorker` owns Type7's exact metadata and immutable
allocation identity. `NativePersonProfile` remains the distinct capability800
input catalog. Root planning evaluates rule13 with actual factory capacity,
while people keep rule12. The shared ABDI mover requests evidence using the
actor's actual D descriptor and preserves the flags0 stagger/cache cadence.

Dynamic construction separates the common104B0 body from task publication.
Read-only world/predicate preflight precedes the body attempt; Sub-D and Sub-A
allocation precede the single425680 selector and all successful branch words.
Factory provenance records the complete constructor sequence. The resulting
four-choice receipt exists before owner linking and Type93 attachment; release
re-enters the same root. Class45's resource16 receipt drains at the current
gameplay tick, while the authored load drain retains its original load time.
Release checks world predicates at the prospective release position before
removing the parent's Sub-J row. Unresolved nearby factory capacity therefore
rejects without changing the relation or RNG; successful release still runs
the selector against the current list after the fixed release writes.

Factory intake authenticates a distinct diver receipt, the real A/D/I and F70
basis, plus Class54's private target. Deferred removal consumes the completed
scheduler owner. General cargo, descriptor contact, capture, primary/infected
hits, radial death and abort route to that same allocation-bound owner.

The focused controls cover canonical metadata rejection, all eight wet births,
detailed/coarse query-free cadence and world22 wind, exact Job Nearby vacancy,
both Class45 parity graphs and cue106, Class10 acquisition/fleeing, sound40
visibility/health gates, four living carry/release graphs, primary/infected
damage and Class14, foreign/parked custody, oriented staffing and abort.
Dynamic world34 controls execute factory output, Type93 carrying/release and
later staffing; a player-near output additionally proves Class45's complete
RNG sequence and live notification time. Main Base output has a controlled
constructor test: style6 has no authored capability800 input cohort, so this
is not evidence of a naturally occurring conversion in those maps.

The full V2000 repository gate and release build pass. A55-frame hidden
production-render smoke covers Klaus, all eight authored divers at birth and
after30 callbacks, and an actual world34 factory birth before/after Type93
release and movement, using front, side and isolated views. The player uses
each map's real arrival; unrelated actors remain at birth. This verifies port
lifecycle/presentation continuity, not a matched retail scene comparison.

Other actor families and remaining shared gameplay boundaries stay in the
objective. No retail capture
was needed to establish these source-owned Type7 contracts.
