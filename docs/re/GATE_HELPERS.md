# Native Type111 terrain helpers

This note owns the actor created for every authored terrain warp marker. Campaign
destinations and static contact remain separate policies. A missing route does not
prevent the helper's constructor, body stamp, class-0 task or positional sound.
Related authorities: [ACTOR_RUNTIME.md](ACTOR_RUNTIME.md),
[FACTORY_AUDIO.md](FACTORY_AUDIO.md) (shared positional mixer), and
[ENTITY_DAMAGE_AND_DEATH.md](ENTITY_DAMAGE_AND_DEATH.md) (deferred destruction).

## Evidence and construction order

The exact source below is the local retail `V2000.EXE` x86 image. Some routines
are absent from the bulk C export; their instructions were checked with
`objdump -d -Mintel --start-address=... --stop-address=... ../V2000.EXE`.
No new capture or observed playback is needed to establish these branches.

`42E570 -> 42EA30 -> 433BD0` creates helpers after the persistent player and
before the Section-13 authored entity loop. `42EFB0` performs the cleanup after
that loop. Consequently even helpers later marked for deletion consume their
own common-body `+B4` ordinal and initial behavior-selection RNG word. Filtering
markers before construction would shift subsequent actor identity and RNG.

`433BD0` scans all 256 by 256 terrain cells in X-major order, with Z as the inner
loop. Attribute zero has no descriptor. The terrain object's `+08` kind selects
`0x16..=0x1A`, yielding helper subtypes 1 through 5. All five reach `416FF0`.
The helper's signed 8.8 position is:

```
x = wrapping_i16(cell_x * 256 + 128)
y = wrapping_i16((signed_i8(cell_height) + 16) * 32)
z = wrapping_i16(cell_z * 256 + 128)
```

`433D8B..433DB1` passes the subtype's `42EF60` result and terrain-type bit 8
to `416FF0`. `42EF60` reads the campaign controller's current logical-world
index `+C4`, checks its 37-slot range and subtype 1..5, then tests
`(controller[world]+D8 >> 4) & (1 << (subtype-1))`. A marker contains no
destination or route-availability test.

`416FF0` zeroes a 0x4C constructor request, copies XYZ, sets type `0x6F`, and
calls `438080 -> 4104B0`. After successful construction:

| Input | Source write |
|---|---|
| terrain bit 8 absent | clear live state `0x800` |
| controller subtype bit present | set capability `0x8000` |
| controller subtype bit absent | set live state `0x1000`, entity `+80 = self handle` |

The last word is **not** a SubJ parent or cargo attachment. `413039..41303D`
explicitly skips Type111 in the relation-repair branch. Native storage retains
that self handle in the class-0 allocation receipt, separately from
`Entity.attached_to`; otherwise normal world iteration would hide the helper.

## Constructor and class-0 contract

Canonical Section-12 Type111 has model slots `[16;4]`, mass 1, capability 0,
health 1, zero damage multipliers, initializer `0x27285`, and a singleton class-0
Always choice of weight 1 (rule reference 1, alternate class reference 2).
Only SubK is present, with payload `[1,0]`; model-variable count is 1.
Constructor attachment `+B4` is sound 100. The remaining sound/effect selectors
are zero. Runtime models still come from the authenticated record, rather than
substituting a route-specific or first-world model.

`4104B0` creates the positional loop through `44C830` before component and D4A0
initialization, at the requested marker position, gain/rate `0x10000`.
Construction failure releases the attachment through `44CC90`.

`40D513` tests default bit `0x20` around D4A0's whole terrain-height, optional
model `+08` height and `+90` copy block. Type111 lacks this bit and therefore
retains the supplied marker Y. Default `0x40` adds a model height only inside
the bit-`0x20` branch; the shared class-0 constructor preserves this nesting.
`41061F..41063E` already copied the requested `+96` position into `+90`, so the
ungrounded helper's retained anchor is the marker pose, not a zero vector.

Class-0 style `4C7468` has root `40C690`, cleanup `40D1C0`, and otherwise null
hooks; `+34=0x2001`, `+38=0`. Effective flags remain `0x27285`.
`40C490` clears Secondary then Tertiary and invokes
`402800 -> 405F80` for a Primary null-callback timer of 9000 ms.
Its timer publication has no additional component reset. The wrapper's timeout is
strictly `elapsed_ms > 9000`; ordinary living reselection consumes the singleton
selector word. The self-relation state suppresses that reselection through the
existing `416410` relation guard.

`40DCA0` detailed visits tick the task slots. Effective `0x2000` without `0x1000`
would set live `0x40000`; Type111 has both, so does not take that write.
Its zero sound selectors consume no sound RNG. Effective bit `0x10` is absent,
so E640/SubK animation is not dispatched; `0x4000` suppresses F70, bit 4 skips
gravity, and bits 8/2/`0x800` omit wind, terrain snap and infected-ground damage.
The zero E370 selectors retain only the surface-timer decay law.
`40E870` coarse visits instead clear live `0x40000` and return immediately
because effective `0x2000` is present: no task or environment update occurs.

## SubK and sound lifetime

`409B06..409B43` allocates and zeroes the model-variable array. `424450` allocates
and zeroes SubK's 0x10-byte block. `409ECB..409F55` binds signed K[0]=1 through
`40A950` to the first variable and leaves K[1]=0 null. `40A950` returns a pointer,
not an initial value. Neither C490 nor the admitted detailed/coarse callbacks
write this variable, so the model's dynamic variable remains zero. A global
clock animation would invent a writer; `42EF60` is only a campaign-bit reader.

`413151..41316C` resets transient B2 and calls `44C920` to copy the actor's current
`+96` position into its `+8C` sound row after the callback and before master
motion. This also runs on coarse or callback-disabled visits. Sound ownership
therefore follows the authenticated native allocation and its retained loop;
it is not created by the route table. Shared positional masking stops physical
voices while preserving logical rows, as described in the audio authority.
Sound 100 resolves through the retail alias pool to PCM 38. A fixed loop rate
does not imply zero RNG: `4954EC` unconditionally draws once per alias hop,
including zero variance, on physical admission. Existing-voice `4957C0` updates
reuse admission without alias or warble draws; resume after buffer destruction
admits the alias again. Masking and final release themselves consume no word.

## Ordered deferred cleanup

`42EFB0` walks the actor list in order. A nonzero, non-dying Type111 searches
later nonzero/non-dying Type111 rows or any Type67 row; the first overlap marks
the current helper. A Type67 row instead marks every overlapping later Type111
without a later-helper alive filter. The list is not shortened during this pass.

`425370` forms wrapping signed-i16 XYZ differences, widens their absolute values
to i32, and returns `max(absXYZ) + ((sum(absXYZ)-max(absXYZ)) >> 1)`.
The approximate XYZ distance must be strictly less than 8000. The squared XZ
distance must be strictly less than `0x100000` for a Hive/helper overlap, or
`0x40000` for two helpers. Equality at either threshold does not delete.

`410B70` resolves the handle and, unless already pending, writes
`state = (state & !0x60000) | 0x100000`, incrementing `DAT_004DAFD0` once.
It neither frees the body nor refunds its constructor ordinal/RNG. The normal
`414990` deferred sweep later destroys tasks, components and the positional row.
Cleanup concerns only helper lifetime; it does not remove the authored marker,
change a campaign destination or decide which routes are unlocked.

## Native implementation boundary

The implementation is in `entity/gate_helpers.rs`, the shared
`entity/class0_actor.rs` and `entity/class0_actor/live.rs`, and
`entity_positional_audio.rs`. Helpers are constructed for native ordinary-world
loads, retained in the class-0 allocation/task custody map, then marked by the
ordered cleanup. Model-variable presentation uses the proven zero initial value.
Campaign routing remains in `campaign_transition.rs` and static contact handlers.
Focused constructor, metadata, cleanup, ownership and audio controls accompany
the implementation; final validation results belong to the current change's
test log rather than a prediction in this authority.
