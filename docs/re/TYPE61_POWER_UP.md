# Type61 Power Ups

Ordinary campaign construction publishes Type61 through the shared
`104B0 -> AC60 -> 257F0` path (demo initializer `256C0`). Class23's singleton weighted choice still
consumes one word from the process RNG. Its initializer is infallible and
publishes no actor tasks. The source is the global Section-12 row and each
Section-13 spawn; no Level-1 spawn index or captured live state is required.

`native_type61` retains the current manager allocation and authored index.
It preserves the spawn's packed `+1C` payload, effective model overrides,
position and constructor damage buffer. Surface classification uses the
current load tick and authored wave policy. `257F0` retains class23's contact
style `4C96C0`; the generic damage modifier and type-hit callbacks are null.
Constructor sound attachment 44 remains owned by the collision/audio state.

The shared class49 explosion owner authenticates the native allocation,
metadata, current style and empty task graph before entering `10C10`.
It releases sound44, applies the ordinary dying prefix and selects alternate49
without another weighted selection. `BAF0` emits sixteen class94/95 scatter
particles, performs the static and live radial walks, then `BD20` clears tasks,
attempts a Type60 ring, and stages removal. The current active model supplies
the scatter extent, including authored model138 trophies. Live pose and health
are not frozen to a capture snapshot.

The older captured Level-1 and factory-birth adapters retain their own
provenance. Ordinary pickups use the shared terminal receipt so nested radial
callbacks cannot replay their burst. Reusing a handle from another manager
does not authenticate an allocation.

Pickup effects, hidden trophy bit2, time-award bit8, and their distinct reward
semantics remain in [Factory System](FACTORY_SYSTEM.md) and
[HUD rendering](HUD_RENDERING.md#time-trophy-and-countdown). This constructor
does not couple a physical pickup's removal to countdown expiry.

`native_type61::tests` checks authored payload/model/position publication in
Level1, Castle, Alpine, OVL18 and OVL37, plus foreign-allocation rejection.
The complete Castle casualty-abort regression composes the two authored
pickups with the native actor and static-world callbacks.

## Class63 carrier drops

Alternate class63 ("Auto Pilot", descriptor `0x004C8828`, style `0x004C7198`)
is the death program of every power-up carrier: Types 71, 80, 81, 117, 124 and
126..129. The style is zero apart from its `+40` initializer `40BC90`, which
retail runs once:

1. `43A580` resolves the actor and `40BAF0` emits the shared burst and the
   static/live radial walks.
2. A zeroed `0x4C`-byte request takes `DAT_004DCA00` at `+00`. That word lies
   past `.data`'s raw bytes and no instruction writes it, so it is zero and the
   constructor allocates a handle. Type 61 goes at `+08`, the carrier's
   `+96..+9A` position words at `+0C`, and its `+88` dword at `+20`. `438080`
   constructs it and `4575A0` disposes the result object, whether or not a
   body linked.
3. `410B70` stages the carrier's removal.

It never calls `A860`, so the carrier keeps its task slots until the sweep;
`410B70`'s `(state & 0xFFF9FFFF) | 0x100000` write stops them from running.
It does not test the remote bit or bind the newborn's `+60` relation.
Carrier `+88` is the Section-13 spawn's `+1C` dword, the same packed
selector/amount that authored Type61 rows carry. Solo play never rewrites it;
only network sync `417680` (flag `0x400`) does, so the port retains it at
construction as `auto_pilot_payload_packed` for alternate-63 rows only.

The port runs class63 through the shared BAF0 terminal in
[class49_death](../../crates/v2k-game/src/class49_death.rs) with a third
policy beside BAC0 and BD20. Before `10C10` commits anything, the source
closes every constructor input: the carrier payload, Type61 metadata, the
common body stamp lineage and the terrain cell under the carrier. The finish
then skips the task clear and calls
[append_auto_pilot_power_up](../../crates/v2k-game/src/entity/auto_pilot_power_up.rs).
That is the shared zero-record `104B0` body: a current-tick surface comparison
at the carrier position, the singleton class23 selector's single RNG word
after the whole radial transaction, the row's own model slots, zero rotation
and damage buffer, null modifier/type-hit callbacks, and a tail append. Its
native allocation records the carrier lease, type, position and payload, so
the drop is a native class23 allocation in every later respect: player pickup,
its own class49 ring death, and Main Base abort admission.

A finished class63 receipt does not require empty task slots. Like finished
class1/49 receipts, it stays radial-addressable until `14990`: `4566E0` has no
dying test, so the drop's own ring blast at the same position reaches the
dying carrier. `11AD0` has no dying test either, so until the sweep the
carrier's retained tasks still take their contact hooks; in the actor
descriptor contact the finished class63 receipt stands in for the owner the
terminal retired. In static contact, `A8B0` likewise calls each retained
task's `+20` hook. Two carrier families run this terminal:

- the Type124 fish ([FISH_RUNTIME](FISH_RUNTIME.md#particle-hits-and-quiet-death));
- the Type80/126 rows on the Type10 owner
  ([INTRO2_TYPE10](INTRO2_TYPE10.md#ordinary-type80126-power-up-carriers)).

The other six carrier types (71, 81, 117, 127, 128 and 129) still need their
living owners.

Controls in
[auto_pilot_tests](../../crates/v2k-game/src/shared_fish/auto_pilot_tests.rs)
check, in worlds 23, 30 and 34:

- the kept tasks;
- the single constructor word;
- the drop's payload, position, surface bits and receipt;
- the finished carrier state;
- a Playing kill followed by a re-hit before the sweep;
- an attached-particle kill in its own Playing world.

A [descriptor-contact control](../../crates/v2k-game/src/native_actor_descriptor_contact/fish_tests.rs)
pairs a finished Type124 corpse with a living fish: its retained Primary task
takes the same two-draw descriptor write as a living fish's.

The [abort control](../../crates/v2k-game/src/main_base_abort_production/native/type124_tests.rs)
runs the fish's class63 and then the appended drop's class49 ring under the
same abort cursor.
