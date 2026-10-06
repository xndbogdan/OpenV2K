# Authored Main Base and Working Factory runtime

This document owns native Type6/Type66 construction and task custody across
ordinary worlds. The economy, pair callbacks, and systemic abort contracts
remain in [FACTORY_SYSTEM.md](FACTORY_SYSTEM.md). The scene-only health policy
and hut effects remain in [INTRO2_TYPE66.md](INTRO2_TYPE66.md).
[The cross-level audit](CROSS_LEVEL_GAMEPLAY_RUNTIME.md) records the original
missing-context reproduction; it is not a source of constructor identities.

## Canonical scope and identity

The normal-tier Section 12/13 census covers all 37 ordinary overlays 13–49.
It contains 81 Type66 factories, including 20 zero-capacity autonomous
factories, and 28 Type6 Main Bases. Each world containing a Base has exactly
one. All corresponding complete Section 12 metadata rows equal the ordinary
Level 1 row; Section 13 model overrides, pose, and configuration vary.

Production enters this path for normal ordinary-world selection, post-Intro2
first-world loading, and campaign warps. Native/portable compatibility-save
previews retain their separate generic reconstruction contract; these previews
do not gain full actor-task restoration from the authored-world change.

| Family | Shared metadata | Instance authority |
|---|---|---|
| Type6 Main Base | health 99999, mass 1000, capability `0x20`, initializer `0x25027`, class 41 | actual spawn/config/model slots and manager allocation lease; all 28 configs are 22 zero dwords |
| Type66 Working Factory | health 99999, mass 1000, capability `0x84`, initializer `0x25027`, class 39 | actual spawn/config/model slots and manager allocation lease; all 22 config dwords retained |

An authored override takes precedence independently for each of the four
model slots. Equal active/wreck slots are legal: Level 19 spawn 8 and Level 43
spawn 1 use model 285 in all four slots. Level 14 spawn 39 instead uses 218/225,
with a three-worker capacity and its own stock/timers. Level 18 spawn 5 is an
autonomous 285/225 factory with 250000-microsecond production/delivery/cooldown.
None of these differences authorizes a Level 1 fixture or substituted model.

The retained runtime binds the entity id, authored spawn index, complete
configuration, selected models, and actual manager allocation generation.
Copying that runtime into another manager does not authenticate its body.
Active task adoption also checks the current context, Primary id, and exact
private task type. A class id or model match alone is insufficient custody.

## Birth and Primary publication

`104B0 ->09A80 ->18A90` initializes Sub-M before `D4A0 ->AC60` selection.
The constructor takes the actual spawn parameter, damage buffer, terrain,
rotation, and configuration. Its shared `0x20` policy grounds at the authored
XZ; there is no model-radius or Sub-C clearance addition for these profiles.
There is no Sub-D allocation. The port explicitly initializes the transient,
retail-unwritten `+B2` mass word to zero as native allocation policy; this is
not a claim about arbitrary retail heap residue.

Both singleton choices still consume one shared AC60 random word. Neither
initializer has a 06070 component-reset suffix:

| Family | Source constructor and Primary | Live style |
|---|---|---|
| Main Base | `25730` clears T then S; `25CD0 ->01020` publishes `25D60`, zero lifetime/latch | class 41, `4C9480` |
| Working Factory | `257C0` clears T then S; `25BD0 ->01020` publishes `25C60`, zero lifetime/latch | class 39, `4C9558` |

The native publishers are
[publish_main_base](../../crates/v2k-game/src/main_base_runtime/native.rs)
and [publish_working_factory](../../crates/v2k-game/src/intro2_type66/native.rs).
The historical Type66 module name does not impose a scene restriction.
Intro2's later `451710` health 1 pass stays separate from construction; ordinary
factories retain their 99999 current health, cached health, and repair profile.

## Live callbacks and production

The native owners run at their actual intrusive-list position in the shared
`13500 ->12DA0 ->DCA0/E870` actor pass. They retain coarse/detailed timing,
transient-mass consumption, Primary wrapper lifetime, and the fixed-building
outer phases. Claims exclude these exact allocations from the later generic
progression pass, including a visit that parks after a committed prefix.
12DA0 caps one callback's elapsed time at 125000 microseconds and retains the
remainder in `+6C`; disabling random waits does not remove that cap.

`25D60` sends an active death clock directly to 19B50. Otherwise 16490 determines
the under-attack state: a quiet frame clears the private latch; a newly
attacked Base queues text C8 and then sets it. The global Main Base abort flag
suppresses the notification without setting the latch. Working Factory's
25C60 keeps its corresponding notification before 19010 production.

The factory Primary drives the existing receipt-bound production machine
using the retained Section 13 configuration and real allocation/version.
[The production bridge](../../crates/v2k-game/src/intro2_type66/production.rs)
performs pickup presence checks, Type61 birth, output birth, Type93
materialiser birth, owner/attachment links, text, sound 8, and lifetime writes.
It commits each requested machine snapshot before its external action.
Real constructor refusal is a parked prerequisite error; it is not converted
into an invented retail allocation failure so production can advance.

Product birth and draw position have separate native owners. `19010` births
at the producer origin plus descriptor offsets; an executed Sub-M external
frame callback `198E0` resolves the current nested model marker and moves the
tracked `+88` product. This generalized Type6/66 rule, authored marker census,
draw admission and explicit precision boundaries are owned by
[FACTORY_PRODUCT_MARKERS.md](FACTORY_PRODUCT_MARKERS.md).

Shared scientist delivery authenticates actual native factory and worker
allocations, explicit factory/worker pair, collision model, and source task.
Native authored and dynamically born Type8/79/90/91/116 GoToJob arrivals use the same
proven pair callback. `425850` tests capability bit `0x400`, not a Type8 id;
`402DFE..402E13` resolves the contacting worker's own type record before
dispatching `401A20`. Intake therefore retains the actual worker metadata:
Type90's model890 and Sub-D divisor32, and Type116's model1136 and Sub-I
attention cue72, are not replaced by the Type8 record. All five profiles
have the shared Sub-I descriptor-contact branch, so the A300 suffix turns
heading by `0x2000`, retains direction and consumes no RNG. Native intake uses
the retained physical matrix and current angles, including terrain-adjusted
pitch/roll. It revalidates both before delivery actions and at suffix commit;
the legacy captured pose contract remains separate.

The scheduler requires both the factory and worker's completed allocation/task
custody. A copied receipt, an unadopted worker, or a pending callback cannot
staff the factory. Prepared delivery retains worker type, allocation, current
Primary/context and native receipt through commit; deferred destruction accepts
the same authenticated worker family. The legacy Type8 fixture remains a
separate path and does not authorize later worker types. See
[factory_pair_live.rs](../../crates/v2k-game/src/factory_pair_live.rs) and
[factory_activation_live.rs](../../crates/v2k-game/src/factory_activation_live.rs).

## Hits, progressive death, and terminal ownership

The live Main Base/Factory styles and class 0 wreck have null hit hooks at
`+20/+28`. Primary 10EB0 stamps `+34`; infected 11250 applies its model/cue
prefix and preserves the old stamp. The adapters retain 11030 reaction gates
and the shared 15040 filter/buffer/health writes. Both capability profiles lack
the capability 8 class 5 suffix. A surviving primary hit with signed nonzero
filtered damage queues sound 7 even when the buffer absorbs its health loss.

First 10C10 death uses 19750: health becomes 10000000, DYING clears, an idle
progression clock becomes 1, and one AC60 draw republishes the family's real
Primary while retaining Sub-M. Type66 has death sound 62; Type6 has no death
sound. The new owner is published before another hit can observe the body.
Health-only revival omits this required selection/task replacement.

19B50 commits elapsed time before walking crossed 100000-microsecond effect
stages. Each model point and its RNG/explosion commands retain source order.
At stage 32 it commits clock −1, defers a tracked pickup, and reenters 10C10.
The terminal path selects the actual wreck slot and executes 19750's template
and staffing suffix before class 0 publication. Its `C490 ->02800` Primary has
a null callback and strict elapsed `>9000ms` expiry, unlike the Sub-I 03230 task.
Detailed callback mode 0 requests 56750's session flash after terminal death.

The staged walker follows 19D90's linear cursor rules rather than 6AF20's
collision-query branches. Convex group/face records only advance its cursor;
only 8E spheres emit candidates. In particular 8A consumes alignment alone,
8D advances one byte, and 8C/8B/95 skip their aligned word. Model 195 (`factor13`,
used in Arena1) has a convex child that requires these rules; rejecting 8C
would park destruction before the wreck. Staged 30/31 mount-frame commands
remain explicit unsupported operations, outside the current ordinary
Base/factory model census.

For Type6's zero templates, terminal publication also issues an allocation-
bound MainBaseTerminalAbortOrigin. The scheduler transfers this linear receipt
once to the host abort owner; it is not cloned into a presentation report or
recreated from a matching entity id. The existing systemic-abort owner remains
responsible for the remaining actors and campaign transition.

## Remaining limits and validation boundary

Styles 1–6 construct native Type8/91/90/79/116/7 from Main Base conversion and
finite-stock factory ejection. Each birth retains its actual model, health,
axis, ordered choices, Sub-D descriptor, Sub-I cues and allocation receipt.
The shared construction transaction charges the body stamp, successful Sub-D
allocation and Sub-A20450 word before the selector and successful task-suffix
words. It does not recast the output as Type8 or repeat construction at adoption.
`42EB70` selects the current Section-13 descriptor's `world_style` at +48
through `450C90`; it does not read
the player craft. The native production scheduler now supplies this actual
value, and the Main Base conversion frame receives it explicitly. The retained
legacy first-world adapters use style 1 only behind their exact world-13
allocation/configuration admission. Style6 [Type7](TYPE7_RUNTIME.md) shares
the common body phase, then publishes its independent four-choice graph and
query-free Sub-D receipt. Intake retains its actual A/D/I and completed task
custody. Class45 event16 uses the current gameplay tick for dynamic births.

Main Base input is a separate boundary: `4258BB..4258E4` tests capability
`0x800` and remote ownership, without a Type9 restriction. Native9/78/86/95/123
inputs now authenticate their own manager lease and completed task graph.
After event1, deferred destruction and the replacement attempt, the component
suffix reads each actual Primary/Secondary/Tertiary slot: known null callbacks
stay null; `02DA0` uses the retained matrix, then `01A20`'s Sub-I branch adds
heading0x2000 and propagates that task's private direction to Sub-A without RNG.
Task/private state, animation and matrix remain intact. The source's own model,
mass and damage filters feed the existing physical suffix; nonzero filtered
damage remains a named boundary. Captured Type9's direction1 oracle is separate.

The six style3 worlds 18/21/24/26/28/37 contain 47 authored Type86 people.
Conversion controls tick their real native graphs, place one at the real Base,
and exercise Type86-to-Type90 conversion without synthetic source publication.
Type95 adds57 inputs in worlds16/27/31/35/36/40; Type78 adds21 in17/19/32/38.
The zero-wind cohorts use their own profiles for conversion to91/79; world32
still reaches the person mover's explicit steady-wind boundary. No authored Base in styles 5/6 has a
capability0x800 input cohort; Type116 birth support is not evidence of a normal
Base conversion there. World42's factory has zero capacity and world47 has no
Base or factory. Output availability must not invent staffing or ejections.

Factory config words 20/21 are C830 sound-voice selectors, not animation nodes.
Many ordinary factories request primary sound 31; Intro2 and some autonomous
factories request none. Production now allocates those loops from the template,
retunes gain/rate through 18F60 bit math, silences them on terminal 19750, and
releases secondary-then-primary through 18BE0/`FUN_0044CC90` when the factory
entity is freed. The shared logical mixer now consumes these native component
rows and their retained constructor positions;18F60 does not call44C920.
Source18F60 switches the primary between gain `0x10000`/rate
`0x18000` for state bit 4 and gain `(state&1)<<16`/rate `0x10000` otherwise; the
secondary uses gain `(state&2)<<15`/rate `0x10000`. See
[FACTORY_AUDIO.md](FACTORY_AUDIO.md) for the exact lifetime and repair matrix.

Focused tests cover all 81 factory and 28 Base births, actual model/config
retention, one selector draw, stale leases, an autonomous later-world pickup,
first-death task replacement, terminal wreck/abort receipt, and blocked-prefix
ownership. The full V2000 repository check passes with the retail corpus present,
including the workspace tests, formatting, script syntax, documentation links,
and staged-artifact checks. Hidden OpenGL smoke checks cover the menu, complete
Intro2, first-world handoff, and intact-to-wreck transitions in Levels 14, 25,
and 39 without unresolved runtime steps. Intro2's turret/factory death timeline
is unchanged. These are port regression checks, not complete cross-level or
matched retail visual acceptance; the remaining boundaries above stay open.
