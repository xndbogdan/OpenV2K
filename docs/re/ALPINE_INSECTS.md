# Alpine insect resistance and death programs

Authority: normal-tier OVL17, the canonical Section-12 and executable weapon
tables, retail x86 consumers, and `V2000-nocd-faststart01/02` replay. The
capture ledger
owns retained filenames and exact launch. Neither a matching damage profile
nor a rendered model establishes complete live actor support in the port.

## Authored cohort

| Entity type | Asset | Alpine spawn indices | Initial health | Alternate death class |
|---|---|---|---:|---|
| 30 | model1042 `icecurly` | 10–16 | 12,000 | 12, Flip Over And Die |
| 40 | model1040 `icelous` | 2, 23 | 8,000 | 18, Split And Explode |

Both have capability8, mass100 and zero authored pre-health buffers.
`004104B0` assigns neither an instance damage modifier at `entity+0x44`;
the constructor's exceptions are other types. The actual damage buffer is
`entity+0x50`, not one of the transform dwords at `+0x0C..+0x2C`.

Type30 owns components ABCDEHKL, initializer `0x0439`, and Sub-E method20.
Its authored choices are Always×1/class5, Player Nearby×9/class7 Search And
Attack Target, and Always×4/class26 Trash Furniture. Type40 owns ABCDEH,
initializer `0x0039`, Sub-E method1, and Under Attack×1/class7 plus
Always×1/class9 Capture People. Neither has Sub-J. These are distinct native
programs; borrowing Type26 construction or a common death callback is not an
evidenced implementation of either one.

### Canonical profile and reuse bounds

The tracked [canonical metadata example](../../crates/v2k-game/examples/alpine_actor_metadata.rs)
reproduces complete runtime metadata, selected-model extent/collision radius,
and authored occurrences across normal-tier worlds13..50:

```powershell
cargo run --release -p v2k-game --example alpine_actor_metadata
```

Run from the repository root; an optional first argument overrides its retail data
directory. The October3 corpus walk finds one complete metadata variant for
each of Type30/40/56 across all38 loaded worlds. Type30 has14 authored births
in17/19/32/38; Type40 has8 in17/19/32/45. Type56 has no authored birth in this
corpus: the recorded split children require the dynamic construction path.

| Field | Type30 | Type40 | Type56 |
|---|---|---|---|
| Topology | ABCDEHKL | ABCDEH | ABCDEH |
| C8 policy; common axis/filter | `0x439`;3584/`0xA5` | `0x39`;5120/3 | `0x39`;3072/1 |
| Sub-A acceleration/correction/base | 1500/−3000/200 | 1500/−3000/200 | 1500/−3000/600 |
| Sub-D divisor; probes; flags | 96;700/400;`0x12` | 128;512/256;`0x13` | 64;512/256;`0x13` |
| H record count | 10 | 14 | 14 |
| Selected model extent/collision radius | 280/440 | 288/234 | 144/117 |
| Sub-E method | 20 | 1 | 30 |

Type56's weighted root is Player Nearby6×1/Search7, Always1×1/Defecate Virus4,
and Player Nearby6×1/Run Away10. It does not have the simple class5 Aimless
root. Its real method30 emitter retains300000us interval, spread64,
threshold32000, axis tolerance2560, sound70 and raw+12 word96. H has phase
rate`0x30000000`, with resolver`0x40000000` except record8's`0x04000000`;
it cannot borrow Type122's six-row`0x20000000` body. Type30 also retains four
model variables, Sub-K`[3,4]`, and Sub-L`[2,1,64,31,160,15]`.

The shared `01430` mover supports ABCDEH after independently authenticating
the actual A/B/C/D/H metadata and allocation. Type40 and Type56 retain their
own authored/dynamic constructor receipts, task graphs and emitter methods
while using that common motion phase. Its separate ABCDEHKL
admission retains Type30's native four-word bank and real K`[3,4]`/L`[2,1]`
bindings, including the NULL-target Class12 phase. Type122's transport admission
cannot be copied to Type40's J-free Capture graph, and Type26's class4 owner
does not substitute for Type56's complete7/4/10 root.

Class2's `C470` terminal clears current Primary/Secondary/Tertiary slots and
marks deferred destruction after the enclosing native death prefix. Type56
now supplies its own allocation, exact Class2 context and completed-terminal
receipt; the linked deferred body remains damage-addressable until removal.
Fish and Type15/87 retain their separate allocation, style, audio and scheduler
custody.

## Resistance is a per-channel threshold

`00415040` and the radial sibling `00414E10` call `004255E0` with the
Section-12 profile starting at type-record `+0x18`. Each of the two packet
slots is independently filtered. An amount **equal to or below** its channel's
threshold contributes zero; otherwise it contributes
`((amount - threshold) * multiplier_q8) >> 8`, with the original signed
arithmetic and optional delivery scaling. The sum is then delivered through
`00414E90`, which owns buffers, health and lethal dispatch. This is not a
plasma whitelist. `00425590` separately sums channel1/2 amounts for physical
impact reaction; its result must not be substituted for filtered health damage.

| Channel | Type30 threshold / multiplier | Type40 threshold / multiplier |
|---:|---:|---:|
| 0 | 0 / 0 | 0 / 0 |
| 1 | 4,000 / 256 | 2,000 / 256 |
| 2 | 4,000 / 256 | 3,000 / 256 |
| 3 | 500 / 256 | 0 / 512 |
| 4 | 200 / 128 | 200 / 128 |
| 5 | 0 / 0 | 0 / 0 |
| 6 | 0 / 0 | 0 / 0 |

The following are unscaled **per-projectile direct-hit** results, before
buffer/lifecycle policy. Explosion falloff and secondary effects are separate.
Weapon names/selectors come from the retail menu/master table, and packet
addresses from the canonical particle descriptors.

| Weapon | Packet channels / amounts | Type30 damage | Type40 damage |
|---|---|---:|---:|
| Default gun / Chain Gun / Flares | `2 / 2000` | 0 | 0 |
| Red plasma | `2,3 / 3000,1000` | 500 | 2,000 |
| Green plasma | `2,3 / 4000,1500` | 1,000 | 4,000 |
| Blue plasma | `2,3 / 6000,2000` | 3,500 | 7,000 |
| Flame Thrower / Large Flame Thrower | `2,3 / 500,6000` | 5,500 | 12,000 |
| Smart Bomb / Napalm Bomb direct contact | `2 / 10000` | 6,000 | 7,000 |
| Aquatic Plasma Vortex | `2,4 / 4500,2000` | 1,400 | 2,400 |

Antidote, Virus, Seedpods and Water Cannon direct packets filter to zero for
these profiles; this does not assert that their special non-health effects
are identical. Rockets, guided missiles, grenades and mines create entities
with separate explosion programs rather than the direct packet rows above.

## Recorded hits and kills

Both recordings load the repaired Alpine overlay successfully, use the
faststart instruction bytes at `0044E316/0042D5C2`, and have recorder exit0.
Recording 01 lasts 73.172 s; recording 02 lasts 150.313 s. These durations describe
the recordings, not startup benchmarks.

In recording 01, Type30 handle `04EC0001` retains health 12,000 after all four
recorded machine-gun/flare primary hits: `F60B3:465`, `F7DE6:341`,
`FBE04:FDF`, and `12EDB2:1C3`. Their packet is channel2/2000, from `004CBF70`
or `004CBF88`. The checked filter returns zero. Later, the Type59 explosion
at tick `0x610` delivers `[2,3,14000,12000]`, filtering to 21,500 and killing
the insect at `131594:1F09`. The bounded reverse launch receipt now
distinguishes the shared Type59 body: this shot is selector3 **Grenades**,
descriptor callback4, rather than selector4 Depth Charges.

Recording 02 gives a complete red-plasma chain against Type30 handle
`04EA0001`. Packet `004CC0A8` is `[2,3,3000,1000]` with player attribution.
The first two hits at tick `0x893` reduce 12,000→11,500→11,000. Twenty-four
accepted live-target projectile hits reduce health to zero, with lethal
`00410C10` at `1D4F99:95F` (tick `0xBA8`). Two later accepted 500-point deliveries
hit the already-dying corpse and must not be counted as additional hits needed
to kill it. This measures projectile impacts, not trigger pulls; the paired
guns can deliver two projectiles in one tick.

Rockets are also directly confirmed, not inferred only from profile arithmetic:

| Recording 02 target | Rocket entity | Falloff-adjusted packet | Filtered damage | Death position |
|---|---|---|---:|---|
| Type30 `04EC0001` | Type42 `04DF0001` | `[2,3,11921,10218]` | 17,639 | `EFF33:112` |
| Type40 `04E30001` | Type42 `04DC0001` | `[2,3,12468,10687]` | 30,842 | `24ABFC:622` |

Selector5 Rockets constructs Type42. Both observed lethal paths are
`0040BC85 -> 004566E0 -> 00414AE0 -> 00414E10 -> 00414E90 -> 00410C10`.
The radial branch legitimately uses `00414E10` rather than the checked
`00415040` entry, so a trace watching only primary-hit callbacks misses these
kills. The oracle reads the actual radial packet and filtered delivery at
each boundary; stack-unwind hints alone are not the authority.

### Weapon launch and lifetime receipts

The tracked weapon custody and lifetime queries authenticate the supported
faststart executable, recorded code windows and unchanged indexed recording
trios. Their complete retained bundles and commands are in the capture ledger.
These receipts follow particular allocations; they do not establish every
weapon selection, acquisition result, underwater branch or allocation failure.

Recording01's grenade `04E00001` is published at `130788:452`, from the
player's `04FD0001` selector3 dispatch at `13077C:16B1`. Its primary task
executes `401120 -> 404690` for125000us: elapsed advances0→125ms and the
private stillness count0→1. Active-pair hook `40D8D0` at `131573:F44` receives
Type30 handle `04EC0001`; `40CEF0 -> 410C10` follows at `131573:F78/F7B`.
Thus this grenade terminates on active contact before its2000-ms/stillness
expiry. `40BAF0` then submits the radial packet and a Type60 ring request;
task clearing and the deferred flag are complete at `131595:210D`. The ring
receipt is a constructor request, not a proof of successful ring publication.

Recording02's first rocket `04E00001` is selector5/callback5, published at
`CF8FA:166A`. Its three task arguments are scalar0/1/31 for callbacks
`403860/401FB0/406A70`. The forward receipt observes15 executions of each,
15 mover visits, and1446000us of age. Terrain hook `40D7F0` at `D7152:25C2`
dispatches `40CEF0 -> 410C10` at `D7152:25DE/25E1`; radial explosion follows
before all three slots are cleared and deferred destruction is marked at
`D7154:1114`. No active/static/water hook or Type60 ring request executes.

Rocket `04DF0001`, which kills the Type30 in the lethal oracle, is published
at `E928B:FD9`. Fourteen executions of each rocket task and mover visits
reach1305000us of age. It follows the same terrain terminal at
`EFF31:1D07/1D23/1D26`; radial begins at `EFF32:619` and deferred destruction
is complete at `EFF33:178C`. Type30's checked death `EFF33:112` lies inside
that radial/defer span. No active/static/water hook or Type60 request executes.

The later rocket `04DC0001` is also selector5/callback5, published at
`245A73:1A53`, with nine executions of each task and955000us of age. It
terminates through terrain `24ABFB:E1`, `40CEF0` at `24ABFB:FD`, and
`410C10` at `24ABFB:100`. Its radial call begins at `24ABFB:85E`; deferred
destruction is reached at `24AC15:2301`. The lethal oracle's Type40 damage
and death at `24ABFC:622`, followed by the eight-child split, lie inside
this radial/defer span. This joins the recorded weapon lifetime to the
separately checked target damage chain; the rocket does not die on a direct
active contact with Type40. No active/static/water hook or Type60 request
executes for this rocket either. Acquisition callbacks executed, but these
queries do not retain their RNG gate or successful candidate-return boundary.

Every retained radial packet above attributes both source type46 and handle
`04FD0001` to the player through `416F90`'s live source relation. It does not
combine the projectile's type with the player's handle. The grenade packet
has inner512/outer2048; all three rocket packets have inner512/outer1024. All use
impulse2000, channels2/3 and unattenuated amounts14000/12000 before radial
falloff and target filtering.

Launch XYZ belongs to the pending-record/render transaction. The grenade's
pending origin flag changes1→0 before consumption and its emitted XYZ is
copied through `414870 -> 44E770 -> 438080`; all three retained rockets keep
flag1 and use `411400`'s current-source-center fallback. These receipts do
not retain the exact `424F20` writer occurrence or its current-node VIEW
cache. Static normal-tier PLAYER4 callback4 selects `pl4tubegun`, whose
reached type14 selectors0/1 resolve Sub-E plain source slots18/20 in that
child's mounted frame. Callback5 selects `pl4fmissile`, whose type14 records
have no command references. The source-owned mount and callback audit
supports native integration; it is not a captured same-child transform receipt.

## Distinct deaths and eight-child split

After the red-plasma death, Type30 has health 0, dying state and style `004C7ED0`
within its class12 program; model slots remain 1042. Type40 instead selects
class18: table entry `004C8B30`, descriptor `004C8860`, style `004C73D8`,
initializer `0040C080`. This routine is recovered from executable disassembly;
it is absent from the available bulk decompilation.

The class18 initializer presents `00440950` using the selected model's extent,
then `0040A860` releases all three task slots. The local-entity branch builds a
zeroed `0x4C` birth record. Its type switch maps Type40→Type56/count8,
Type31→Type105/count8, Type27→Type3/count2 and Type34→Type4/count2; the default
uses the same type/count2. A count of the whole intrusive entity list limits
births to the remaining capacity below 80. `468D00` starts at the first
actor's next pointer and walks until null, replacing the omitted first actor
with the terminal sentinel at `004DB094`. Its result still equals the total
linked actor allocations; do not add an extra sentinel to an actor-manager
count. This is a global count, not an eight-child or per-family cap.

Each requested child consumes nine RNG words before construction: three
position offsets, three velocity components and three Euler words. The raw
offsets use `(low16 >> 6) - 512`; velocity X/Z use `(low16 >> 5) - 1024`, Y
uses `low16 >> 6`; pitch/roll use `(low16 >> 4) - 2048`, and yaw adds that
term to the parent yaw. The child parameter retains the parent's `0x01000000`
state policy. The birth entry is `00438080`, followed by `0040D73A ->
004104B0`. A failed birth disposes its result and stops the remaining loop.
Finally `00410B70` marks the parent for deferred destruction: clear `0x60000`,
set `0x100000`.

Birth-record+00 is the current null/default handle value from `DAT_004DCA00`
(all accepted split births carry zero), copied through `4380D3` to104B0 parameter4.
Zero selects fresh-handle43A290; nonzero calls43A3C0 with that requested handle.
It is neither the global address nor a timestamp. Historical evidence JSON named
this word `static_point_reference_raw`; the Rust request now names its consumer
`requested_entity_handle_raw`.

The type switch, requested entity-handle value and global capacity are selected once,
but each child reads the parent's current position, yaw and objective bit
after the previous constructor returns. Constructor-owned random draws remain
interleaved between the nine-word launch prefixes. Remote parents still
release their tasks and mark deferred destruction, without local births.
`00440950` returning nonzero exits before either suffix.

In recording 02, the Type40 death executes all eight Type56 constructors
between `24ABFC:FDB` and `24AC13:1346`; the parent returns at `24AC14:E73`.
Type56 is model1041 `smallicelous`, health 6,000, with channel2 threshold 1,900,
so an ordinary 2,000-point bullet can inflict 100. It uses alternate death
class2, distinct from both adult families. Six newborns die during the same
ongoing rocket blast; the other two survive that immediate sweep and die
later. Immediate nested births are therefore observable by the continuing
retail radial traversal, not deferred until a later frame.

The tracked bounded split replay
now checks that complete initializer at tick `0x101E` in recording02. The
native count is26, matching the26 actor allocations after accounting for the
count walk's omitted first actor and included terminal sentinel, allowing
all8 requests. Each child is constructed and published
before the next launch prefix. The complete transaction consumes105 RNG
draws:1 in the shared burst,72 launch draws and32 constructor draws
(4 per child). All three parent task slots are zero after `A860`; the parent
remains linked through its deferred-destruction return. Every launch record
matches its own nine draws and current parent words. Recorded C080 bytes
match the fingerprinted faststart EXE, and the trace/index/recorder remain
unchanged. Exact reproduction and bundle names are in the
capture ledger.
This above-sea successful split does not establish failed allocation,
capacity exhaustion, underwater presentation or final pixels.

## Port coverage and remaining boundary

The [native entity-weapon owner](../../crates/v2k-game/src/native_entity_weapons/mod.rs)
now retains real Type42/59 constructor allocations and published task wrappers.
Selectors3/4 share Type59/class35 while keeping their distinct Grenade and
Depth-Charge launch speed/audio; selector5 retains Type42/class22 and class1
terminal policy. Class35 reuses the source rolling state, clears slots2 then1,
and preserves strict stillness `>10` and elapsed `>2000ms` dispatch after the
wrapper callback unwinds. Coincident stillness wins over timeout in `401120`.

The [weapon contact pass](../../crates/v2k-game/src/native_entity_weapons/contact/pass.rs)
latches `11AD0` subject admission and selected model before surface, static,
then active contact. Terrain uses the current physical Q31 matrix and actual
model collision program: Type59's model128 contact radius66 is distinct from
the rolling extent51. Rocket solid `style+10` enters `40CEF0` before the
shared `141D0` motion/damage tail; Type59's null solid/water style callbacks
still receive that common response. Hard water constructs the real Type60
model132/130 ring, adopts its Primary scheduler receipt, emits sound17, then
arithmetically halves Y velocity. Ordinary water retains velocity.

Static contact preserves `412CF0` cue/effect/task/type/style ordering and
the retained-plane `11760` response, then delivers static damage before
checked actor damage. The [shared active host](../../crates/v2k-game/src/native_actor_capture/pair.rs)
keeps both behavior callbacks, freshly read component slots and physical/
directional damage suffixes after synchronous class1/49 terminals. It also
owns player-first weapon pairs with the actual Playing hull and the strict
`+60` relation grace `<750000us`; the later detached player planner excludes
constructor-authenticated weapon allocations, including parked or finished
continuations. That routing rule does not authorize changed task graphs.

The [contact regressions](../../crates/v2k-game/src/native_entity_weapons/contact/tests.rs)
exercise real normal-tier Alpine constructors, model programs, native static
objects, opposite weapon callbacks, hard-water task transfer and Playing hull
delivery. These source-backed checks do not establish complete retail visual/
audio acceptance or every possible opposing actor pair. The
existing no-haptics presentation boundary remains explicit; guided missiles
and mines require their own native owners.

### Native Type30 owner

The [Type30 owner](../../crates/v2k-game/src/native_type30.rs) publishes actual
authored allocations, source-ordered H/D/K/L/A/E construction and the weighted
5/7/26 root. Class5 retains its 5000ms Primary; both style`4C7930+00/+04` are
`40C690`, so mover completion and timeout reselect the root rather than advancing
to variant1`4C7978`. Search7 retains real acquisition, Chase/Aim and Method20
FIFO custody. Furniture26 retains its separate `C890` static callback. Hits
preserve native components through reselection and Class12; detailed Class12
runs the same K/L bank with a NULL target and retains committed prefixes on a
later block. [Living/FIFO controls](../../crates/v2k-game/src/native_type30/live_tests.rs)
and [hit/death controls](../../crates/v2k-game/src/native_type30/tests.rs) use
canonical metadata and constructor-issued leases.

Actual model1042 `icecurly` is normal-tier `1X9XX.OVL` Section8 local69. Its
40-byte collision program contains an `8E` radius440 sphere at slot56, a `95`
gate, and radius140 detail spheres at slots36/58/60 before `88`; it has no
collision-child `05`. Those four records are plain type0 points, respectively
`[0,0,0]`, `[0,30,0]`, `[0,40,280]` and `[0,40,-280]`. K/L variables affect the
rendered hierarchy but do not change this reached collision shape. This bounds
Type30's particle-model query; it does not establish default animation inputs
for other models or their child collision programs.

Two replaceable allocation policies remain approximations. Transient `+B2`
starts at zero once, then accepts actual callback/animation writes. Source
`401D30` leaves Furniture target Y unwritten, although `401DA0` still calls
`01430` after a failed scan. Type30 therefore selects explicit
[`InitialActorHeightApproximation`](../../crates/v2k-game/src/trash_furniture.rs)
at each Furniture task publication: it seeds that task's Y from the current
actor once, and a successful scan overwrites it with retail bilinear Y. Existing
Type26/58 retain `SourceUnresolved`. The follow-up owner is native allocation
residue/first-consumer evidence; neither policy nor these source-backed controls
establishes matched Alpine motion, final pixels or audio acceptance.

The shared `DamageProfile` arithmetic already matches the recovered profiles.
The [corpus regression](../../crates/v2k-game/tests/alpine_weapon_resistance.rs)
connects actual OVL17 cohorts, canonical player weapon/particle tables, strict
threshold boundaries and expected filtered results. It does not fabricate
native construction, task, hit or death ownership; the separate native owners
must supply those receipts.

The investigation also found a concrete projectile-path gap: green plasma's
class56 could be emitted but its class56/82 pair was excluded from model
sweeps and the shared combat callbacks. The
[projectile contract](INTRO2_COMBAT_PROJECTILES.md) owns that correction,
including the same authored underwater birth/contact/emergence policy as
red and blue plasma. Projectile and actor ownership remain separate controls.

The [shared class18 kernel](../../crates/v2k-game/src/split_and_explode.rs)
now owns the source-backed initializer ordering, family switch, signed global
cap, exact zeroed birth record, per-child parent refresh, RNG/constructor
interleaving, native error disposal and deferred parent terminal. Its tests
also retain committed prefixes without replay when a host boundary blocks.
This kernel requires an authenticated live host; it does not admit an actor
family or replace its constructor/task ownership. The current radial walkers
already reread the live successor after a callback, so synchronous appended
children need no traversal workaround.

### Native Type40 and Type56 owners

The [Type40 owner](../../crates/v2k-game/src/native_type40.rs) retains actual
authored constructor leases, its fourteen-row H body, the 7/9 root and Method1
FIFO. Class9 acquisition publishes the real `4C8038` pursuit style. Its `C910`
contact sees the genuine absent Sub-J as full, calls the contact person's
`10C10`, and returns NULL; it cannot attach, issue a transport task or borrow
Type122's captured-child custody. Search7 retains native acquisition, Chase
and Aim. The shared hit and contact hosts preserve callback order and use the
current task/body receipts after reselection.

The [production Class18 host](../../crates/v2k-game/src/native_type40/death.rs)
publishes real Type56 children synchronously through the current manager and
scheduler. Every child consumes its nine launch draws followed by the actual
four constructor draws before the next child or outer radial target. The cap
uses the full native actor allocation count, including deferred and cargo
bodies, with no extra sentinel. Deferred parent marking is last. The retained
terminal receipt authenticates later corpse hits, and a blocked prefix cannot
repeat its burst, RNG or child constructors.

The [Type56 owner](../../crates/v2k-game/src/native_type56.rs) owns the dynamic
`104B0`/`D720` path, process Sub-D state, fourteen-row H body and full 7/4/10
root. Class4 retains the source zero-lifetime Tertiary and separate 2000ms
Primary; Method30 queues and drains its own requests. Class2 quiet death
clears all three task slots and defers removal exactly once. Primary, infected
and cured hits on its completed linked body use the actual null terminal
callbacks and preserve buffer access without reselecting a living graph.

[Type40 controls](../../crates/v2k-game/src/native_type40/tests.rs) and
[Type56 controls](../../crates/v2k-game/src/native_type56/tests.rs) cover real
constructor leases, root branches, FIFO retention, capacity boundaries,
underwater Class18 effects and committed failure prefixes. The enclosing
[weapon-to-split control](../../crates/v2k-game/src/native_entity_weapons/alpine_hits_tests.rs)
uses the production rocket terminal and outer radial walk; the separate
[NULL-J contact control](../../crates/v2k-game/src/native_actor_capture/pair_type40_tests.rs)
checks the native person and remaining pair suffix.

These implementations and source-backed controls do not establish complete
retail scene, motion, visual or audio acceptance. Type40/56 transient `+B2`
uses the same labeled initial-zero allocation approximation as Type30, pending
native allocation-residue/first-consumer evidence. Nonzero requested entity
handle overrides remain unowned; the reached solo split supplies the native
null/default value zero. A missing child resource or failed scheduler transfer
parks its committed host prefix; it is not a fabricated retail allocation-error
object. The shared kernel separately owns native error disposal, while actual
allocator failure delivery remains a host boundary. Retain the accepted
recordings for matched production checks instead of repeating an ordinary
Alpine fight.
