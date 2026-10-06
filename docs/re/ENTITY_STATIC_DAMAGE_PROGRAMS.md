# Admitted Static Damage Programs

Verbatim move from GAME_MECHANICS.md during the 2026-08-24 cohesion split; this document owns every admitted static-kind damage program.

### Admitted Static Damage Programs (CONFIRMED)

Static contacts deliver the raw two-slot packet independently to the static
kind and the player hull. `FUN_00427950` filters kind 0 through the complete
profile at `0x004C9BE8`:

```text
thresholds  [0, 4000, 2000, 0, 6000, 0, 0]
Q8 slopes   [0,  256,   64, 512,  256, 0, 0]
chance gate 1000
```

Primary projectile classes 1 and 3 use distinct globals at `0x004CBF70` and
`0x004CBF88` with identical six-dword values
`[2,0,2000,0,source_type,owner_handle]`; both filter as channels `[2,0]` and
amounts `[2000,0]`. `FUN_0043FF10` writes the two trailing provenance words
before `FUN_0043F800`, which writes them again and calls `FUN_00427950` with
the static collision record's cell-centre X/Z. The latter resolves the current
live terrain attribute/type/kind at delivery time and filters only the first
four dwords. `source_type` is the zero-extended particle byte at `+0x1C`,
populated from the owner's entity-type byte at `+0x58` (46 for the player),
while `owner_handle` is the particle dword at `+0x14`.

The entity sweep follows the same provenance rule. `FUN_0043F980` writes the
particle's immutable birth type and owner into global packet `+0x10/+0x14`
before the class-1 `FUN_0043F590` callback enters `FUN_00410EB0`. The port's
common `ParticleEntityImpact` reuses `BallisticDamageRequest` to retain that
exact packet/type/owner shape. Its `delivery_record()` conversion requires both
provenance values and otherwise fails closed; it never re-resolves the
shooter's potentially changed type at impact time.

This primary packet cannot start the kind-0 program: its channel-2 amount 2000
is exactly the channel-2 threshold, and `FUN_004255E0` requires
`amount > threshold`. The result is severity zero with no RNG draw and no
program node. At the inline F800 boundary, the runtime port resolves the
current effective cell through mutations already committed by earlier physical
slots and submits that current target; it does not require equality with the
collision-time attribute, type, model, or kind. Exact bridge and scheduler
tests cover this negative kind-0 result.

If terrain-type bit `0x08` is already set, the filtered severity is divided by
two with signed truncation, except for static kind 10. Each kind compares the
result against its own descriptor chance gate. For kind 0, severity `>=1000`
is deterministic and severity `1..999` consumes one word from the shared
retail RNG, accepting when
`(rng & 0xFFFF) <= floor(severity*65535/1000)`. The RNG draw precedes the
burned-state and cell-deduplication branches; nonpositive severity consumes no
RNG.

Kind2's repair-object descriptor at `004C9E90` admits the same null-program
branch with chance gate zero. A direct read of the supported retail PE
(`E9BE7A83...B4BA`) confirms all eleven descriptor words:
`[004C9C58,0,0,0,10000,0,0,0,1,004E2037,0000FF00]` (decimal 10000).
Profile `004C9C58` has thresholds `[0,4000,4000,4000,4000,0,0]` and Q8
slopes `[0,256,256,256,256,0,0]`. `11760` submits collision channel1 and
the raw impact to `27950 -> 255E0`; the strict threshold test returns zero
through impact4000, before any RNG, burn or program admission. This no-op
must return to the following actor-damage delivery rather than park a walking
actor. A positive result immediately requests `4337E0(...,1)`, including
when a timed program already occupies the cell. Already-burned severity is
halved first, then returns without another burn. No kind2 packet draws RNG:
nonpositive results return before the zero gate, and positive results cannot
be less than it. Its null `+0C`/`+14` burn suffix has no crater or scatter.
The separate contact-action words `+20/+24` do not change this damage path.
The world15 native Type9 regression uses actual model142 at cell `(117,73)`
and retains its completed walking owner after nonzero, filtered-zero impact.

Kind4's fuel-object descriptor at `004C9EE8` has profile `004C9DE0`, chance
gate500 and a null timed-program pointer. Its thresholds are
`[0,4000,500,0,0,0,0]` and Q8 slopes `[0,256,512,1024,0,0,0]`.
`427AC4 -> 427AF8 -> 4337E0(...,1)` is an immediate ignition branch: after
the same filter, chance and burned-state gates, it sets terrain bit08 before
the caller continues, with no FIFO node. Its `427760` burn-listener particle
suffix is a separate owner from the damage filter. This branch precedes
active-cell deduplication; an existing program at the cell does not suppress it.
Already-burned kind4 still halves severity and can draw chance RNG, then returns
without another mutation. Primary F800 damage yields severity3000 and burns
deterministically; the ballistic2500 packet filters to zero.

The scheduler returns an explicit `ImmediateBurn` result for this admitted
null-program branch. Intro2 and Playing's mutable particle hosts invoke the
shared burn listener at the callback boundary, after committing the bit and
before the parent physical slot is freed. Constructor, actor-contact and
radial adapters use that same listener suffix. Both live particle hosts retain
the presentation dirty flag, including burns from late actor contacts, so the
terrain redraw observes the changed model in the same frame. The detached
borrowed event-handler API still reports an `ImmediateBurn` terrain journal
write; it does not own a mutable world listener and is not the live Playing
path.
Shared Type17 contact delivers its collision packet to the actor only after
the static result returns; a filtered-zero fuel hit is not an unsupported-kind
failure. This closes the world20 native spider's reached fuel-contact branch.

Playing still queues its hive-death burst and radial until particle traversal
finishes. That helper observes the committed traversal journal, but this does
not establish retail's exact interleaving: a static write from a particle after
the lethal hive hit can precede the deferred radial in the port. This existing
queued hive boundary is separate from the synchronous static/F800 overlay and
the mutable Intro2 host; shared Type17 hit callbacks do not scan static terrain.

Accepted kind-0 hits enter the cell-keyed FIFO used by `FUN_004281A0`:

| Cumulative time | Recovered actions |
|---:|---|
| 0 us | Require the static target; effect 18; positional sound 68 |
| 500,000 us | Effect 18; positional sound 90 |
| 600,000 us | OR terrain type with `0x08`; dispatch radial template; terminate |

The selected model header's unsigned `+0x0A` radius is reacquired once per
active program per update. Program coordinates use signed-word wrap: base X is
`(cellX<<8)+0x80`, base Z is `cellZ<<8`, and base Y is the single signed
Section-10 height byte shifted left five. The two effect offsets are
`[0,+3R/4,-R/4]` and `[0,+R/2,-R/4]`; both sounds use the unshifted base at
gain/rate `0x10000`.

Enqueue (`FUN_00428720`) does not itself execute time-zero records. The main
update enqueues direct collision hits before its scheduler pass, so they run
their initial records in that frame and inherit its delta. Radial child hits
are enqueued from inside the pass; its saved next pointer excludes them until
the following update. A terminating parent remains in the active-cell list
while its opcode-12 radial scan runs, preventing its own blast from restarting
the same cell. The following opcode 0 removes that parent before the next FIFO
program's radial returns, so two parents terminating in one update retain their
original per-program removal order.

Class38's standalone `4C98F8` program also enters this registry through
`441B70 -> 28720`: exactly `[0,11,1]` followed by `[0,0,0]`. The pointer at
`4C9910` is separate data after the terminator. Its opcode11 invokes `37100`
without requiring a static descriptor or model: subtract one from the cell's
high three lighting bits, clamp at zero and preserve the low five bits.
Only an actual write reaches `4A890`, retaining the original radar coverage
and consuming its covered-cell RNG in that static pass. Programs queued by
particles run on the next update, with the same cell deduplication as damage
and crater programs. The live level terrain and presentation dirty request
retain any committed write if the later radar refresh cannot proceed; the
consumed program is not replayed. See
[Intro2 combat](INTRO2_COMBAT_PROJECTILES.md#queued-fireball-ground-program)
for source coordinates and particle callback ordering.

The radial template has inner/outer radii 256/512, impulse 2000, channels
`[1,3]`, amounts `[1000,1000]`, and preserved trailing words `[-1,0]`. Static
scan order is X-outer/Z-inner over offsets -512..512 in 256-unit steps, with
toroidal cells, signed four-corner terrain height, and the target model radius
used for the candidate Y clamp. Distance sorts absolute wrapped-word deltas
`a>=b>=c` and computes `a+((b+c)>>1)`; 256 is full strength and 512 is a miss.
Retail delivers this static scan before its separate dynamic-entity phase and
does not apply radial impulse to static objects.

Kinds 1 and 8 use the shorter, fully closed program `0x004C9978`, while kind
28 deliberately reuses kind 0's complete program `0x004C9918`:

| Kind | Damage profile | Chance gate | Program |
|---:|---:|---:|---:|
| 1 | `0x004C9C20` | 1000 | `0x004C9978` |
| 8 | `0x004C9D38` | 2000 | `0x004C9978` |
| 28 | `0x004C9DA8` | 100000 | `0x004C9918` |

Program `0x004C9978` submits effect 18 at three-quarter model height and plays
positional sound 90 at time zero, then waits 500,000 us, ORs terrain type with
`0x08`, and terminates. It has no require-static or radial record. Kind 28
uses kind 0's exact require/effect/sound/burn/radial sequence and timing, but
retains its own profile and chance gate; sharing a program pointer does not
make the descriptors interchangeable.

Kinds 3, 10, and 11 now have the same complete admission closure. Their exact
catalog bindings are:

| Kind | Damage profile | Chance gate | Program |
|---:|---:|---:|---:|
| 3 | `0x004C9CC8` | 4000 | `0x004C9A60` |
| 10 | `0x004C9D70` | 4000 | `0x004C9AB8` |
| 11 | `0x004C9DA8` | 100000 | `0x004C9A60` |

Both programs require the current static at time zero, submit common explosion
event 30 at half model height, and play positional sound 90 at the unshifted
cell base. After 500,000 us they submit ordinary effect 18 at the same
half-height position and OR terrain type with `0x08`. Program `0x004C9A60`
(kinds 3 and 11) then dispatches radial template `0x004C9878` before
termination; kind 10's `0x004C9AB8` terminates without a radial. That radial
has inner/outer radii 512/1024, impulse 2000, channels `[1,3]`, amounts
`[2000,1000]`, and trailing words `[-1,0]`.

The event-30 position uses the selected current model's unsigned header
`+0x0A` collision radius for its half-height and quarter-depth offsets.
Independently, `FUN_004281A0` passes model header `+0x08` as the source extent
to layered writer `FUN_00441200`; substituting the collision radius here
changes the authored explosion bundle. The port reacquires both values from
the post-mutation model slot once per FIFO node update.

Kind 10 also owns the exceptional already-burned branch in `FUN_00427950`.
After filtering and any required chance draw, an accepted hit on a cell whose
bit `0x08` is already set does not enqueue `0x004C9AB8`: retail calls
`FUN_004337E0(...,0)` to clear only bit `0x08`, then `FUN_004338C0` to
wrapping-increment the live Section-10 attribute byte. The port performs those
two live terrain writes as one explicit immediate transition. Other admitted
burned kinds remain no-ops after their possible chance draw.

Kind 29's descriptor binds profile `0x004C9D00`, chance gate 2000, and program
`0x004C9B00`. Its time-zero opcode 3 submits one ordinary class-79 positional
event at half model height through `FUN_004410B0`, with zero input velocity and
without event 18's water substitution, then plays sound 90 at the cell base.
After 500,000 us it submits ordinary effect 18 at the same half-height
position, burns the cell, dispatches radial template `0x004C9878`, and
terminates. This is deliberately distinct from kind 9's two-attempt
direction-table scatter despite both paths selecting descriptor class 79.

### Synchronous static effects and null-program burn listeners

The disappearing village huts are Section9 kind29, models364/365 (attribute29),
distinct from the dynamic Type66 hut and kind27 secondary-crater owner.
`281A0 -> 4410B0 -> 440A60` allocates each ordinary18/79 effect synchronously
before the next opcode. The previous production adapter deferred those births
through `WorldFx`'s queue while common explosions and burn listeners allocated
immediately. That reverses allocation order and changes physical-slot age and
rendering in the same world pass. Both Intro2 and Playing now use the shared
immediate emitters; the500,000-us kind29 effect/burn/radial deadline is unchanged.

Timed effect placement and burn-listener scatter retain different model words.
PE instruction `4282CC` loads unsigned header `+0A` for `281A0`'s half-height/
quarter-depth position; `42792E` independently loads header `+08` for `427760`'s
scatter extent. The existing scheduler already preserves that distinction.
The real intact hut364 (`pesnthut`) has `+08=230`, `+0A=179`; burned hut365
(`dpsnthut`) has `+08=256`, `+0A=0`. Using `+08` for intact hut placement
would change Y/Z; zero burned collision radius does not erase its independently
authored `+08` extent. Kind29 has no `+14` listener scatter of its own.

The200-record allocator still follows descriptor priorities: fresh equal-
priority particles cannot be replaced, but higher-priority particles can evict
them. A focused before/after test leaves one free slot and demonstrates that
deferring an earlier18 gives it to a later18; another assertion retains native
79-over18 preemption. This is source-order evidence, not a claim that every
effect must survive pool pressure. Natural hut visual acceptance must inspect
the actual18/79 allocations and submission, not merely an effect request.

`27950`'s accepted null-program branch also calls `4337E0`, which changes bit08
before dispatching427760. Direct projectile/static/radial adapters previously
set that bit without the listener; crater null-program burns had the same
omission. They now share the source listener, including the burned model's
header+08 extent, cell-centre origin, zero provenance, descriptor+14 scatter
and sound62. Kind4 fuel runs80 class93 attempts; kind27 runs160 and its own
crater; kind29 has no+14 scatter and retains its authored18/79 FIFO program.
Repeated burns remain no-ops. Crater listeners execute between the current
height write and material replacement, retaining the pre-crater radial origin
and in-flight cell deduplication. Corpus tests cover fuel in worlds13/14/50,
kind29 program ownership and both existing numerical crater oracles.

`4410B0`'s class18 surface substitution compares its signed sea-plane word
unconditionally, independently of the water-render gate. The production static
action adapter therefore supplies the authored plane whenever terrain exists.
At/equal below that word it allocates45 then46 even when the visible water pass
is disabled; a focused signed-plane boundary regression preserves that rule.

Both class-79 paths pass the zero `DAT_004DCA00` owner sentinel from
`FUN_004281A0`. `FUN_004407D0` and the ordinary `FUN_004410B0` branch retain
that argument. At allocation, `FUN_00440A60` stores owner dword zero at particle
`+0x14`; its failed `FUN_0043A580(0)` lookup writes source byte zero at `+0x1C`.
These are known zero provenance words, so subsequent entity damage retains
the complete `[1,0,2500,0,0,0]` packet. The admitted `FUN_00441200` common
explosion callers supply the same zero owner, retained by both its class-30
scatter and four later debris allocations. The port keeps this evidence on
these emitters; an unrelated emitter with missing birth provenance remains
unresolved.

Kind 27 closes the tenth and final program-bearing static kind. Its retail
descriptor `0x004CA2DC` selects profile `0x004C9DE0`, chance gate 500, program
`0x004C99A8`, radial template `0x004C98B8`, and auxiliary block
`0x004C9E28`:

```text
thresholds  [0, 4000, 500,    0, 0, 0, 0]
Q8 slopes   [0,  256, 512, 1024, 0, 0, 0]
auxiliary   [0x800, 20, 0x400, 10]  (first pair: burn-callback crater radius/depth)
```

Its 15-record timeline is:

| Cumulative time | Recovered actions |
|---:|---|
| 0 us | Require the static target; effect 18 at three-quarter model height; positional sound 90 at the cell base |
| 500,000 us | Effect 18 at three-quarter height; sound 90 |
| 1,000,000 us | Effect 18 at three-quarter height; sound 90 |
| 1,500,000 us | Effect 18 at three-quarter height; sound 90 |
| 2,000,000 us | Opcode 6 with inclusive distance `0x1400`; sound 62 |
| 2,100,000 us | Opcode 6 with inclusive distance `0x1400`; burn; radial; terminate |

#### Burn callback and secondary crater

Opcode 10 enters `4337E0`, which commits terrain-type bit `0x08` and dispatches
the `4DC628` listener list only when that bit changes. Static initialization
`426F90` registers `427760` through `4339E0`. The listener resolves the current
Section-9 kind and, for a set burn bit (or exceptional kind 10), consumes the
kind descriptor's `+0x0C` pointer as a radius/depth pair. Only kind 27 has a
nonnull pointer in the retail table: `4C9E28` supplies radius 2048 and depth 20.
The remaining two retained dwords are not inputs to this call.

`427894 -> 436E00` runs synchronously with material-enable 1, before the next
program opcode. Its X/Z are the burned cell centre, `(word & 0xFF00) + 0x80`,
and Y is that cell's current signed height byte times 32. This differs from
the FIFO's ordinary effect/radial origin: `281A0` already cached that origin
before the same-deadline action sequence, so opcode 12 retains its pre-crater
position. Keep the parent registered through the callback and subsequent radial;
new crater-enqueued programs enter the next scheduler pass.

After the crater, `427760` also submits descriptor `+0x10 / 100` scatter
attempts of particle type `+0x14` through `440950`, with known null owner and
the cached pre-crater centre. Kind 27 supplies 160 attempts of class 93, using
the selected burned model's `+0x08` extent. `4DC9B4 != 0` suppresses this suffix
only during serialized terrain replay; normal simulation leaves it zero.
The second-crater return snapshot confirms zero at this Intro2 call.
Kind 4 independently specifies 80 class-93 attempts. The shared live immediate
burn path now executes this suffix before returning to its projectile/contact/
radial caller; repeated burns dispatch nothing. Its descriptor has no crater,
so these adapters preserve the existing kind27 crater owner and timing.

The later Intro2 replay proves why the listener matters. After the Type66 hut
crater, tick 436 enters `436E00` from `427899` at raw
`[-29056,-224,31872]`, centred in cell `[142,124]`. In the captured 19×19 patch,
164 heights change; the centre goes from `-7` to `-32`, below unchanged sea
raw Y `-847`. Submerged vertices increase from 23 to 44. This is the missing
foreground basin, separate from the earlier hut's radius-1280/depth-16 crater.
See the replay ledger
and [scene comparison](INTRO2_TYPE66.md#retail-crater-and-water-comparison).

The radial has inner/outer radii `0x800/0xDAC`, impulse 2000, channels
`[2,3]`, amounts `[6000,6000]`, and trailing words `[-1,0]`. Because the outer
radius is not cell-aligned, `FUN_00427F20` scans signed offsets
`-3500 + n*256`, `n=0..27`, ending at `+3412`: 28x28 candidates in X-outer,
Z-inner order. It does not round this to a symmetric cell box. The scheduler
also tests the next record's deadline before reacquiring the current static; a
removed require-static target retains its FIFO/dedup node until that deadline.

Side-by-side retail/demo scheduler bodies (`FUN_004281A0` / `FUN_00428070`)
have the same switch, timing, and mutation order after accounting for build-
specific globals and sound indices. The opcode-6 wrapper `FUN_0040F9A0`,
wrapped-distance helpers (`FUN_00425370` / `FUN_00425240`), and full-frame
setters (`FUN_00456750` / `FUN_004560B0`) additionally have accepted
instruction/byte-exact pairs. Together they close the remaining ownership
statically. The demo descriptor/profile/program/radial/auxiliary counterparts
are `0x004C42DC`, `0x004C3DE0`, `0x004C39A8`, `0x004C38B8`, and `0x004C3E28`.
Their structure and control values match after rebasing; sound indices are
build-specific, so retail's 90/62 rather than the demo's 92/64 remain
authoritative for this port.

Opcode 6 measures the wrapped-word distance from the static cell base to the
current camera **focus spring** at retail `DAT_004DAF18`, not to the render eye.
Distance `<= 0x1400` invokes the full-frame setter, which unconditionally
restarts cursor `+0x295` at one. The static FIFO runs before the later chase-
camera update and therefore samples the focus already stored for that frame.
The mode pump runs static simulation before the `53410` presentation consumer,
so an accepted request is first visible in that same frame. Intro2 follows this
order; the legacy Playing adapter's earlier preparation still delays it one
frame, as recorded in [RENDER_PIPELINE.md](RENDER_PIPELINE.md). All ten authored program-bearing kinds
are consequently live without a behavior capture; the prepared health-pack/
radar run remains optional visual/audio acceptance only.

#### Dynamic-Entity Radial Phase (CONFIRMED — `FUN_004566E0` / `FUN_00414AE0`)

`FUN_004566E0` runs the complete static scan above first, then immediately
walks the live dynamic-entity list. Entities are appended at the list tail, so
the walk is in creation order. It reads the current node's next link only after
that node's impulse and damage delivery return; this is not a distance-sorted
or snapshotted query. Static-phase mutations are therefore visible to the
dynamic phase, and damage callbacks may affect the remainder of the live walk.

Intro2 static-program blasts and meteor deaths share the live
[`intro2_radial`](../../crates/v2k-game/src/intro2_radial.rs) traversal. Each
accepted dynamic target commits impulse and checked/unchecked damage in order;
completed native death publications transfer to the scheduler before another
static action or actor visit, including publications carried by a later block.
The admitted death callbacks retain allocations for the deferred sweep and do
not append entities, which permits the adapter's retained list of identities.
The separate gameplay adapter still requires its player-hull policy.

Opcode12's source FIFO node remains registered throughout both scans. Even if
the dynamic suffix reports an unresolved target, the host consumes the batch
once and executes its terminating opcode0; it must not replay the blast next
frame. Static templates preserve source `-1` and owner0. Radial delivery has no
primary/infected style callback, `+34` stamp, `11030` angular RNG, accepted-hit
cue, or capability8 emission. Those belong to the separate particle wrappers.

A dynamic entity is considered only when its raw state flags at `+0x08` pass:

```text
(flags & 0x00008000) != 0
&& ((flags & 0x00000800) != 0 || (flags & 0x00001000) == 0)
```

The same wrapped-word distance approximation described above is measured from
the blast origin to the entity centre at `+0x96`; it must be strictly less than
the signed outer radius. Retail does not add the model collision radius, sort
by distance, deduplicate entities, or exclude the blast source, owner, attached
entities, or a particular entity type from damage.

`FUN_00425430` copies the template's six packet dwords byte-for-byte: two
channels, two signed amounts, and two trailing provenance words. Distance at or
inside the signed inner radius is full strength; distance at or beyond the
signed outer radius misses. Between them it computes:

```text
q15 = ((outer - distance) << 15) / (outer - inner)
scaled_value = (value * q15) >> 15
```

Only the two amounts and the sign-extended i16 impulse are scaled. Channels and
both trailing dwords remain unchanged. The helper accepts a damage hit when the
two scaled amounts have a nonzero wrapping-i32 sum. If impulse output was
requested, a zero-sum packet is also accepted when the scaled impulse is
greater than `0x20`, permitting an impulse-only hit.

Impulse output is requested only when state bit `0x08000000` is clear and the
unsigned mass word at `+0xB0` is greater than one. Its vector uses the same
wrapped signed-i16 `entity - origin` deltas:

```text
component = (delta * scaled_impulse) / max(distance, 1)
```

The i32 multiply wraps and signed division truncates toward zero. On an
accepted hit, the low i16 of each component is added with word wrapping to
velocity `+0x9C/+0x9E/+0xA0` before damage delivery. Entity type `0x23` still
receives damage but suppresses this velocity mutation. State bit `0x08000000`
also suppresses only impulse, not damage. Network-owned entities (state bit
`0x80000000`) additionally send message type 10; one dword in retail's packet
comes from an uninitialized stack slot, so an offline port must not invent a
meaning for it.

Finally, full-strength hits use checked `FUN_00415040`, which revalidates state
bit `0x8000` and retains its zero-filter feedback path. Falloff hits use
`FUN_00414E10`, which does not perform that second active-state check. Both
paths filter the scaled packet through the target's Section-12 profile, invoke
the optional per-instance damage modifier at entity `+0x44`, then pass the
result and packet source type `+0x10` to generic damage `FUN_00414E90`. Generic
damage consumes the `+0x50` pre-health buffer, honors dying bit `0x4000`,
invokes the target hit callback, subtracts from health at `+0x30`, and enters
the target's death path when required. Packet owner `+0x14` is not forwarded;
it is observable only to an optional `+0x44` modifier before generic damage.

`FUN_004255E0` also owns an optional post-filter ratio. A zero denominator
skips this suffix completely. Otherwise retail takes only the numerator's low
16 bits, performs a wrapping signed-i32 multiply with the filtered sum, and
signed-divides that product by the positive unsigned-16 denominator, truncating
toward zero. Several checked-damage call sites pass nonzero ratio arguments;
they are not globally interchangeable with the common `(0, 0)` skip case.

The shared active-pair cap in `FUN_00414D30` consults only `+0x30`, not the
buffer. In subject-then-candidate order it filters the current raw amount; when
the result exceeds `health + 1`, it converts that excess back through the
selected channel's Q8 multiplier (`(excess << 8) / multiplier`, or the excess
unchanged when the multiplier is zero), subtracts it, and clamps the final
signed raw amount to zero. Target order is observable because each signed
division truncates before the next filter.

`FUN_00414E90` mutates positive buffer before checking dying. A live target
then requests the generic type sound at `+0x98` and invokes a non-null
type-vtable hit callback `+0x30` even when the post-buffer amount is zero,
wrapping-subtracts health, and delegates values below one to `FUN_00410C10`.
That death transition independently requests type sound `+0x90` before its
death callback. The Rust port retains these two arithmetic phases and both
sound selectors without pretending that either is the accepted-hit cue below;
the legacy callback-free adapter rejects non-player death, while the Intro2
live adapter publishes its authenticated native Type9/26/47/53 death owners.
The default
type death callback invokes the current behavior style's `+0x2C`: Main
Base/Working Factory use
`FUN_00419750` (which may restore health and clear dying), while audited
Capture People variants 2--5 use `FUN_0040D040`. Per-instance `+0x44` packet
modifiers for types 46/73/109/110 also precede this generic path and remain
explicit runtime policies rather than assumed nulls.

The forwarded source type remains observable after filtering: value 46 gates
player-kill feedback selector 4 and participates in the local survivor/lethal
network suffixes. It is not passed into the type hit callback or death callback.
Type 17 has no per-instance `+0x44` modifier in either retail or demo, so packet
owner `+0x14` is inert on that local path after dispatch, but retaining it is
still required for the exact six-dword delivery and other actor families. This
retail/demo match closes provenance statically; no focused capture is needed.

The complete checked chain is now represented by a detached receipt-bound Rust
transaction. It retains the independently resolved admission, filter, generic,
and death allocations instead of collapsing them into one handle lookup. The
optional `+0x44` modifier may mutate all six delivery dwords and may return any
signed 32-bit value. In particular, a modifier result of zero or a negative
high-bit value still enters `FUN_00414E90`; only after generic hit effects,
health/death callbacks, and network suffixes have unwound does
`FUN_00415040` return those exact bits to its caller. Zero therefore suppresses
the outer accepted-hit suffix, not the inner generic lifecycle.

Every sound, callback, write, network submission, resource release, and
post-callback cached-pointer read is a separate action. The adapter must prove
the same allocation/type identity before continuing through a pointer cached by
retail. This fails closed on deletion or handle rebinding rather than
reproducing retail's stale-pointer dereference. Lethal damage independently
resolves the death target/type, preserves the death sound -> attachment release
-> death callback order, and returns to the original generic target only when
`FUN_00410C10` returns nonzero. Main Base or Factory callbacks may clear dying
and thereby make that helper return zero. The process-global player-type byte
is sampled twice, at the same two retail boundaries, so survivor and death
network packets do not accidentally share a later value.

The checked transaction does not allocate its own identity when nested under a
primary hit. A host keeps the outstanding primary-hit receipt, allocates a
separate checked-damage transaction identity, drives that child to a terminal
completion, and resumes the parent exactly once with the pure completion
mapping. This leaves live transaction-ID allocation and external action
journaling at the adapter boundary.

The detached `v2k-game::primary_hit_checked_damage` coordinator now owns that
host-side nesting rule without broadening the live claim. It starts only from
an authenticated issued `CheckedDamageDelivery` parent action, preserves the
delivery packet and ratio arguments bit-for-bit, and accepts the admission
target and filter binding as two independent call-time observations. While the
child is active the parent receipt is inaccessible; child completion resumes it
once, while a durable child evidence block records its detailed reason and
commits the parent boundary as unavailable. Parent and child identities remain
caller allocated and must differ, and all child actions still require the
existing checked-damage adapters.

The cadenced accepted-hit wrapper `FUN_00410D30` reads its positional cue from
the target's Section-12 word at `+0x80`. Playback occurs only after
`FUN_00415040` accepts nonzero damage and the raw unsigned comparison
`last_tick < tick - 10` succeeds. The callback writes entity `+0x34 = tick`
before testing the final dying state and selector, so a zero selector or a
death callback that leaves bit `0x4000` set still consumes that presentation
window. Conversely, a type death callback may clear dying before this final
test. The Rust transition preserves the 32-bit subtraction seam exactly.

Class-1 primary bullets do **not** use that cadence wrapper. Their descriptor
callback `FUN_0043F590` dispatches `FUN_00410EB0`, which derives the impact,
unconditionally writes entity `+0x34 = tick`, invokes type-vtable impact slot
`+0x14` (`FUN_0040DAC0` in the common vtable), applies the conditional
`FUN_00411030` impulse, and only then calls checked damage `FUN_00415040`.
`FUN_0040DAC0` packages the signed impact, provenance argument, and direction
pointer for the current behavior-style `+0x28` callback. It is not
`FUN_0040D860`: that separate type-vtable `+0x10` surface-contact trampoline
has a surviving-target `FUN_004141D0` tail which must never be added to primary
projectile hits. Every accepted nonzero result whose final state is not dying
requests the type's `+0x80` cue; there is no ten-tick comparison.
Main Base type 6 and Working Factory type 66 both author global sound 7 there.
Their effective current behavior-style impact slot `+0x28` is null and fixed
state suppresses the impulse, so the bounded Rust primary-hit path can
reproduce their surviving transaction without inventing either effect. A
geometry contact alone must not play this cue. Its positional submission uses
`FUN_0044F450`; generic hit and death cues
use `FUN_0044F480`. A future lifecycle commit must preserve those backend
families as well as the three selectors rather than flattening them into one
generic sound event.

Full-game `FUN_0043F590` and the demo callback at `0x0043EE40` have the same
synchronous shape. They copy the integrated endpoint from `particle + 8`, call F610, then
re-read `particle + 0x1D` bit zero. When clear they call the primary-hit wrapper
with the descriptor packet, the exact live `particle + 8` address, and the
post-update `particle + 0x0E` velocity pointer; the outer physical-pool scan
frees that slot only after the callback returns. F610 compares Y with the
authored flat sea-plane word even when the level's water-render gate is off.

`WorldFx::update_with_impact_handlers` now preserves that point for the
admitted class-1/class-3, seven ballistic, and class-87 descriptor families.
It materializes the common class-34/class-42 F610 child, invokes one ordered
handler while the parent slot is still live, then performs the outer free.
The event retains class provenance, integrated endpoint, exact post-update
velocity words, and the opaque retail argument token
`0x004DCF40 + slot * 0x20 + 8`. The current handler immediately runs the
audited Main-Base/Working-Factory damage/audio bridge; unsupported targets
still fail closed. A returned `entity_impacts` vector is retrospective only
and is never replayed. The other descriptors that happen to contain F590 are
not activated without their packet, static, and owner contracts.

The collision-model candidates begin as an owned frame-start snapshot, while
retail `FUN_00440120` runs every later physical slot against a fresh live-
entity scan. The live handler now closes that difference for all admitted F590
damage routes: after the synchronous Type-17 or Base/Factory transaction it
rebuilds the complete `iter_collidable` projection in manager order and returns
it to `WorldFx` before the outer parent free and next slot. Replacing the whole
list, rather than patching only the direct target, preserves attachment/proxy
eligibility, removal, model/radius/orientation changes, indirect side effects,
and scan order. Damage-suppressed callbacks make no entity mutation and return
`Unchanged`. A focused two-primary regression moves the target in the first
handler and proves the later physical slot does not report a stale hit.

This closes the stale frame-snapshot seam for membership/order, center, model,
radius, and the selected live collision basis. Authenticated Common-Dying
Type-17 consumes its retained nine-dword Q31 matrix; other admitted non-player
entities preserve the existing yaw-derived policy until their own matrix writer
is proven. Class 32's owner-velocity lookup remains a separate frame-start
snapshot; no current evidence authorizes folding that callback cache into the
collision refresh.

Seven mode-3 particle descriptors (decimal classes 16, 30, 58, 79, 93, 94,
and 95) share the same later collision contract: entity callback
`FUN_0043F590`, packet record `0x004CC000`, and static callback
`FUN_0043F800`. Their update order is integration, optional inline
`FUN_0043ECD0` trail, entity-model sweep, refined static-object sweep, optional
`FUN_0043F610` visual, and only on a complete miss the later
`FUN_0043E4F0` surface callback. Entity presentation uses the integrated
endpoint. The static path uses the refined probe returned by the authored
static test. Either hit selects class 34 strictly above the flat sea plane and
class 42 at or below it, then frees the parent.

The exact packet channels/amounts are `[1,0] / [2500,0]`. Its following
delivery words are not damage channels: retail fills them with the particle's
immutable source entity type captured at birth and its owner handle. Runtime
byte `+0x1D` bit 0 suppresses only this damage dispatch; it does not suppress
the visual or parent deletion and is inherited by ECD0, E4F0, and F610
children. The other observed `+0x1D` bits are unrelated, so the port represents
this as a predicate rather than storing or synthesizing the raw byte.
Full-game F800 also re-reads that bit after F610. Primary and ballistic static
events therefore retain `damage: None` while still materializing the visual
and deleting the parent. Otherwise the port follows the exact inline order:
F610 visual, suppression re-read, current cell resolution through the terrain
mutation prefix committed by earlier physical slots, static-scheduler
submission, then outer parent free. Resolution deliberately uses the current
cell rather than stale collision-snapshot equality, so an earlier same-pass
mutation affects a later F800 delivery and a later mutation cannot
retroactively affect an earlier one.

All ten admitted program-bearing kinds (0, 1, 3, 8, 9, 10, 11, 27, 28, and
29) may receive either exact packet family. For the primary `[2,0] / [2000,0]`
packet, the only positive unburned results are kind 9 at severity 2000 and kind
27 at severity 3000; both are deterministic. The seven ballistic classes use
record `0x004CC000` and packet `[1,0] / [2500,0]`, which filters to zero for
all ten kinds. Consequently none of these exact admitted packets draws RNG or
reaches kind 10's immediate burned-cell mutation. The returned primary and
ballistic static-impact arrays are retrospective telemetry and are never
replayed. Class 87 retains `FUN_0042E8E0`, its separate static no-op, rather
than entering F800. Unsupported descriptor families remain fail-closed.

Primary-fire projectile and muzzle roots retain the shooter's birth-time type;
host-created roots without equally proven provenance report it as unknown
rather than looking up a mutable current type. Unaudited generic entity
behaviors remain fail-closed.

The retained Level-1 pickup/fire trace also closes selector 2. Its byte-exact
descriptor is `02000000C800000004020F0504003C000000070204010000`:
callback selector 4, 200 finite rounds, cadence byte 15 (300 ms), sound 60, and
HUD sprite 519. Each funded event emits projectile/particle class 3 (sprite
704, raw speed 2000, lifetime 255) and the same distinct class-15 companion.
Selectors 1 and 2 share the persistent A/B side-gun alternator across weapon
changes. The port commits one round only when an event is emitted; zero rounds
suppress projectile, audio, and alternator advance. Class 3 shares the proven
entity-impact callback and damage packet, but retains its separate surface
callback rather than being relabeled as class 1.

Type 46 installs a nontrivial modifier at `0x004484A0`: an active linked type-63
entity receives the packet and zeros damage to the player; otherwise the packet
is propagated through the controller's linked-entity list with temporary
`0x8000` activation before the original filtered amount is returned. This link
contract is part of radial delivery, not optional presentation.

The bounded port integration now shares this arithmetic between the static and
dynamic domains, runs static children first, then preflights the complete
creation-order dynamic pass before committing anything from it. Retained null
modifiers and type-hit callbacks, the empty-cargo type-46 identity case, player
hull death, and Base/Factory progressive death are admitted. An in-range remote
owner, unresolved callback/modifier, nonempty type-46 link context, or unsupported
lethal lifecycle discards the entire dynamic plan including impulse and audio;
the enclosing static parent nevertheless retires after the dynamic return. This
atomic port boundary is deliberately stricter than retail callback-by-callback
mutation for unsupported cases, and prevents a fail-closed tail entity from
leaving earlier entities partially changed.

The 2026-07-17 100-Hz runtime trace independently observes terrain-state
`+0x08` and model-slot-1 selection 610--620 ms after kind-0 contacts, matching
the recovered 500-ms plus 100-ms schedule. This is a state program, not an
invented per-tree HP pool.

### Rendered secondary-explosion regression

The natural Intro2137/40ms before/after uses separately built original
commit31049dbf and current libraries with all existing Type57 adoption intact.
The tick430 OpenGL frames visibly restore additional secondary explosion
submissions while retaining the crater sequence. Across the inspected late
interval, class18 presentation visits increase38→86;333/16,667us increases
0→112. These are presentation visits across the scene, not unique particle
births or counts attributable solely to kind29. The isolated kind29 FIFO,
200-slot admission and crater-listener tests establish those source owners.
The grey chunks are authored billboard debris, not placeholder assets. Retail
classes 16 (meteor scatter), 30 (common explosion) and 93 (kind 4 burn) all use
Section-3 frames699--702 through `43DC90 -> 43D410`; the recovered native
Flip 328596 visibly contains the same shapes. Their sixteen-pixel sprites,
scale 1280, frame scale 256 and physical-slot jitter use the existing shared
render path. Matched RNG, births, camera, draw cadence and source ages are
still needed to accept exact projected sizes, counts and lifetimes. The
unsynchronized tick 430 images do not establish a debris-rendering defect.
