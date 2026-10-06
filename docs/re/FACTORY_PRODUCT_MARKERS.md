# Draw-owned factory and base product markers

This note owns Sub-M's external-frame product position rule. Construction and
production custody remain in [AUTHORED_BASE_FACTORY_RUNTIME.md](AUTHORED_BASE_FACTORY_RUNTIME.md)
and [FACTORY_SYSTEM.md](FACTORY_SYSTEM.md). This rule uses the same retained
descriptor and product handle for every applicable authored factory or base;
model names and level coordinates are not its authority.

## Reproduction and root cause

The former adapter searched only model 227's `lifterarmfork` grandchild and
used that child's attach origin. It followed Type61 entities with a matching
recent relation at `+60`. A Level2 factory uses root model218 and its animated
child220, so the name search returned no point and its product remained at its
birth position inside the building. Even Level1 omitted the fork's authored
slot8 point `[0,78,0]` relative to the child origin. The missing shared runtime
consumer is `0A9F0 ->198E0 ->199B0`, not an absent birth offset or cargo link.

The Level1 capture `20260731-001224-second-scientist-factory-activation-v2`
records the factory product birth request at raw `[5700,FD00,3A00]` with a zero
descriptor offset. Its writer is the versioned
capture-second-scientist-factory-activation.ps1.
The completed capture ledger was audited before this investigation; no new
replay query was used to assign the marker semantics below.

## Source rule

The repository retail image and local `<install>/V2000.EXE` match SHA256
`E9BE7A833612FBA3A5A5AB92A974ECE1A689E4B7E72409D9EE8331380573B4BA`.
The decisive missing `198E0` body was recovered directly from that PE's machine
code, rather than guessed from the incomplete bulk-C call graph. It can be
rechecked with `objdump -D -Mintel --start-address=0x4198E0
--stop-address=0x4199B0 V2000.EXE`. `199B0` is also present in
bulk_clean/game_logic.c.

1. `19010` births the product at entity `+96/+98/+9A` plus signed descriptor
   `+0C/+0E/+10`. It records the product in runtime Sub-M `+88` and the recent
   producer relation at product `+60`. It does not call cargo attachment `08F00`.
2. `0A9F0` consumes any Sub-H and Sub-I selector ranges first. When Sub-M is
   present and the remaining selector is zero, it calls `198E0`. Type6 and
   Type66 have neither preceding selector family in the audited metadata.
3. `41991C..41993F` obtains the current descriptor/runtime, reads descriptor
   word `+00`, and passes that model slot plus the current draw context to
   `199B0`. The context is the actual nested node, with its current animation,
   linked slots and transform. This is not necessarily the root building.
4. `199B0` resolves the selected model slot in that context, converts its
   cached view coordinates through the inherited viewport frame when
   necessary, adds the viewport origin, and returns both dword point
   coordinates and wrapping short world coordinates.
5. `419944..419990` resolves tracked product `+88`, writes the returned short
   marker plus signed descriptor world offsets to product `+96/+98/+9A`, and
   sets product state bit `20`. The offsets are added after the hierarchy
   transform; they are not a rotated local displacement. A missing product
   does not prevent returning marker geometry. The product lookup is handle
   registration/generation, not an active-state admission: `43A580..43A5F4`
   never reads the allocation's state flags, and `419956` only tests nonnull.
   The suffix therefore writes all three pose words before state20 even when
   the registered product is ineligible for simulation or collision.

The callback also clears the current actor's linked presentation-row `+2A`
words (`419993..4199A2`). Those native transient rows have no corresponding
retained factory presentation queue in the current port; this unrepresented
side effect is an explicit boundary, not an invented product motion rule.

## Authored applicability

Normal-tier system3 models contain referenced selector-zero tf14 records and
the following descriptor-selected plain points. Model numbers below are global;
all these factories use descriptor slot8. No model-name condition is required.

| Model | Authored name | Slot8 point, raw model units |
|---|---|---|
| 195 | factor13 | `[0,140,0]` |
| 200 | factor11lift | `[0,100,0]` |
| 203 | factor10lift | `[0,35,0]` |
| 205 | factory9lift | `[0,60,0]` |
| 207 | factory8lift | `[0,60,0]` |
| 208 | factory7 | `[0,0,-750]` |
| 211 | factory6lift | `[0,75,0]` |
| 212 | factory5 | `[0,249,0]` |
| 217 | factory4door | `[0,102,-76]` |
| 220 | factory3lift | `[0,60,0]` |
| 222 | factory2lift | `[0,128,0]` |
| 229 | lifterarmfork | `[0,78,0]` |
| 234 | chernobylpowerup | `[0,45,0]` |

Model193 `factor14` also contains a plain slot8 `[0,150,0]` and an unused
selector-zero tf14 record. No normal primitive references that tf14; only the
optional `0x38` vertex-number diagnostic names it. Source `4689D0` skips
resolution when `4FEEF4` is clear, so normal drawing does not publish that
point merely because its descriptor exists.

Level1 root227 reaches arm228 and fork229. Level2's authored root218 reaches
factory3lift220; factory2 root221 reaches child222 and provides an additional
applicable model with a different marker height. Native Main Base descriptor
word0 is handled by the same callback policy when a submitted node requests
Sub-M selector0. A Base or factory without that executed request retains its
birth position; the descriptor alone does not invent a marker occurrence.

## Port ownership and checks

[sub_m_external_frame.rs](../../crates/v2k-game/src/sub_m_external_frame.rs)
retains the descriptor, actual product handle, actor origin, and draw origin.
The shared [model_tree external-frame policy](../../crates/v2k-game/src/model_tree/external_frame.rs)
executes it after current-node far admission, executed view-command selection,
and face-normal admission, before near projection rejection. Each child resolves
the marker in its own current frame. The returned selector point also supplies
the renderer's tf14 geometry; the product write uses the same point plus world
offsets and state bit20. Ordinary gameplay installs that presentation and
commits its tracked-product suffix after the owning actor draw. Intro2 installs
the same policy for native physical actors; a cinematic pose proxy cannot own
these writes. The installation requires proven absence of preceding Sub-H/I
selectors and retains fallback for unresolved/composite policies. Simulation no
longer repositions the product before contacts through a name-based scan.

The intrinsic collector in
[factory_product_lift.rs](../../crates/v2k-game/src/factory_product_lift.rs)
is a corpus inspection/test adapter. Its all-view traversal is not a substitute
for live draw admission. The formats callback hook defaults to the existing
raw tf14 fallback for every other unowned external-frame policy.

Focused checks cover the Level1 fork and additional child points, Level2's
tracked product and signed world offsets, authored models across worlds13–49,
unchanged untracked pickups, current nested-frame geometry, normal admission,
toroidal origin conversion, and rejected-parent write suppression. Test results,
full affected-crate/repository checks and matched visual acceptance must be
recorded in the active objective; their absence is not established fidelity.

## Source production and rendered before/after

The Level2 world14 regression constructs the actual authored world with the
model-header extent resource, staffs factory spawn39/root218 through its
native owner, and runs production to a native product birth. It then installs
the draw-owned Sub-M presentation and follows the delivery animation with the
actual renderer; it does not manually place a pickup or apply a level offset.
The same admitted production/draw fixture was also run against archived HEAD
`31049dbf6add1a00574896d4d80fa3513527308d`, built separately as a release
library, to reproduce the old endpoint with the original code.

Both versions birth the product at signed raw `[15360,-1024,-32000]`.
Archived HEAD leaves it there through delivery. The final repaired fixture
installs the retained native chase viewport and places it at child220 slot8
`[15655,-966,-32301]` on its first admitted draw at native 20ms tick500.
It retains that point at tick600; by tick750, `WaitingForPickup` yields
`[15654,-848,-32301]`. These exact endpoint words also match the independent
retail PE instruction oracle described below. The before/after OpenGL images
were viewed. Focused cases separately exercise Level1's child229 point
`[0,78,0]`, child222's point `[0,128,0]`, all thirteen applicable authored
factory markers, signed descriptor offsets after the world transform, and
draw admission ordering.

An additional managed Level1 root227 run staffs its real factory and executes
the same native production owner through delivery. It births at
`[22272,-768,14848]` and draws the product at `[21468,-720,14840]` at tick300,
`[21429,-599,14839]` at tick400, and `[21445,-411,14839]` at delivered tick550.
The archived comparison executes its former `follow_factory_products_to_lift_platform`
stage before drawing, yielding `[21474,-796,14848]` at birth and
`[21450,-486,14848]` at delivery. This additional real-render case exercises
the authored arm/fork hierarchy and omitted marker78 with no placement offset.
Its logs are `work/factory-level1-original-camera.log` and
`work/factory-level1-integer-camera.log`; the viewed images show the resulting
delivery positions. This controlled owner/draw comparison does not claim to
replay every old Playing scheduler stage.

The final scratch logs are `work/factory-level2-before-camera.log` and
`work/factory-level2-integer-camera.log` in the task workspace. The image pairs
are `level2-product-birth-before/after.ppm`,
`level2-product-moving-before/after.ppm`, and
`level2-product-delivered-before/after.ppm`. They are local investigation
artifacts; the delivery report records their durable review copies.

This is a source-backed production and rendered comparison with the old port.
No newly matched retail Level2 rendering was recorded in this run. Native
source and captured input words establish the runtime rule and tested integer
arithmetic; they do not establish matched retail pixels. Objectives03 and08
link this owning note and retain that acceptance limit.

## Native precision audit

The nested draw frame's `+78` is the inherited viewport frame. The plain-slot
callback table entry at `4D4A78+78` selects `46D610`, which materializes a slot
in the current node's VIEW coordinates using `ctx+18..38` and
`ctx+08/+0C/+10`. `467410` consumes the parent attachment's resolved VIEW
point as the child origin; `4676C0` installs its VIEW basis. `464E60` retains
the viewport pointer, and `199B0` converts the child VIEW point back to world.
The inherited pointer therefore does not omit the child's attachment.

[NativeWorldViewport and NativeModelFrame](../../crates/v2k-game/src/native_model_frame.rs)
retain this integer pipeline separately from the renderer's float matrices.
`4138F0` forms wrapping short actor-minus-viewport deltas and copies the
retained entity body basis. `465870` projects that origin and basis with
separately shifted signed Q31 products. Child-frame provenance retains
`67410` register snapshots, linked-slot remaps, `67730` signed permutations,
and the native mount commands, including `66FC0` identity and `671F0`
table-word rotation. The `46D610` mirror negates its already shifted X term.
`46F7D0` interpolates already projected VIEW slots using
`a + 2*((delta*phase)>>17)`, rather than interpolating model coordinates
before projection. `199B0` inverse-projects with the inherited viewport's
columns, adds its origin, and narrows to wrapping shorts. The product suffix
then adds signed world offsets and writes state20. Native face admission
uses `46D3F0`'s raw record anchor, wrapping integer dot product and strict
negative sign; generated float anchors do not substitute for that input.

[ChaseCameraState::native_viewport](../../crates/v2k-game/src/chase_camera.rs)
publishes source `0F3A0 ->0F350` words from the owned raw eye/focus springs.
It wraps the eye-to-focus short difference, caps only positive Y at500,
uses the `57960` integer normalizer for forward and right, and forms up with
separately shifted Q31 cross-products. `0F350` supplies signed eye shorts,
the nine right/up/forward words and identity0. All 5,890 duplicate-stable
accepted Intro2 camera records and 1,164 accepted campaign camera records
match that source constructor. Eighteen retained camera cases form the
focused regression. Ordinary RetailChase and physical Intro2 draws install
this explicit viewport; a free camera does not manufacture native words.

The Level2 factory retains heading `6000`, origin
`[15360,-1024,-32000]`, and body columns `[1517813760,0,1518469120]`,
`[0,2147352576,0]`, and `[-1518469120,0,1517813760]`. Child220 has
orientation0 and marker8 `[0,60,0]`. Delivery selects attachment118 and
register5=65534 at tick750, following the parent command stream's upper
limit; that word is not replaced with a nominal 65535 phase. Root201
`factor10` additionally attaches child203 through tf12 slot56, aliasing the
plain point54 `[0,70,0,805]`. Its actual world callback is `4340B0`, rather
than the intrinsic table's pure XYZ alias. `NativeSlotSurface` preserves that
distinction: inverse VIEW projection, wrapping short X/Z terrain sample using
`45860`'s existing bilinear height rule, replacement Y relative to the
viewport, and forward VIEW projection. Missing world terrain leaves this
attachment unowned. No level offset or float surface transform supplies it.

The bounded retail instruction executor
runs the verified PE instructions for `46D610`, `46F7D0`, `70700`, `199B0`
and the applicable world alias `4340B0 ->445860` in controlled memory.
Its 34 cases include cold/warm slots, mirrors,
accepted factory/camera inputs, and the actual final OpenGL fixture's
viewport. Its nine embedded guards reject unmapped memory, PE writes, control
outside the approved functions, unknown instructions/callbacks, instruction
overflow and unavailable/invalid terrain; eleven adjacent unit tests exercise
the interpreter and its input guards. It computes its expected coordinates
from PE instructions, rather
than from the port's math. The `4138F0/465870` prefix and orientation0 child
copy are source-reconstructed setup, explicitly identified in the oracle.
Accepted factory and camera snapshots are independent cross-track records;
they are controlled valid inputs, not a claimed simultaneous native callback.
No captured Type8 `199B0` return is present in the retained campaign window.

With the accepted-camera fixture, native birth/delivery shorts are
`[15655,-964,-32302]` and `[15655,-845,-32304]`. With the final render's
retained chase viewport, they are `[15655,-966,-32301]` and
`[15654,-848,-32301]`, exactly matching production writes. The earlier float
hierarchy result `[15657,-964,-32297]` was therefore not native-exact for the
tested Level2 case. The new regression constructs the genuine authored
world14 factory and resolves the real root218/child220 data at both endpoints.
The corpus regression independently walks worlds13–49 with the source Euler
basis writer and initial/delivered status controls, covering twelve reached
marker models including Chernobyl child234's point `[0,45,0]`, and 28 bases
without a selector-zero request. A separate root221/child222 authored-data
case covers the additional factory2 lift at both controls. That inspection
does not manufacture live allocation custody. Five actual `factor10` terrain
fixtures from worlds19/24/28/29/36 match the PE's full `4340B0 ->445860`
VIEW outputs, including a sloped sample; no height callback is stubbed.

The native register handlers `66410..669A0` do not invalidate vertex flags.
A static over-approximation of all branches in the applicable factory paths
audits eighteen child emissions and their recursive attachment dependencies.
No earlier warm slot is followed by a write to a register that its attachment
reads: tf8 attachments read register5, established before use and unchanged
through their emissions; plain and tf12 attachments have no register input.
The marker points are plain and the mounts retain their source rotation
values. Thus the current snapshots preserve the retained-cache coordinates
on those audited paths, without claiming complete native frame execution.

The source PCs below are word offsets in the owning authored `cmd_words`,
with both conditional emissions retained for inspection. They make the
attachment audit reproducible from the model pool and the public decoder.

| Parent → child | Command PCs | Attachment slots | Register input |
|---|---|---|---|
| 199 → 200 | 327 / 336 | 86 / 82 | 5 / none |
| 201 → 203 | 222 | 56 (world tf12) | none |
| 204 → 205 | 280 / 289 | 38 / 34 | 5 / none |
| 206 → 207 | 193 / 202 | 38 / 34 | 5 / none |
| 210 → 211 | 261 / 270 | 38 / 34 | 5 / none |
| 215 → 217 | 247 / 256 | 30 / 26 | 5 / none |
| 218 → 220 | 93 / 102 | 118 / 114 | 5 / none |
| 221 → 222 | 621 / 633 | 44 / 40 | 5 / none |
| 227 → 228 | 313 | 36 | none |
| 228 → 229 | 32 | 8 | none |
| 231 → 234 | 625 | 150 | none |

## Remaining boundaries

The integer attachment resolver owns plain/mirrored points, tf8 VIEW
interpolation, linked tf11 slots and world/intrinsic tf12 distinctions used by
the applicable factory hierarchy. Live Sub-M marker publication admits plain
and mirrored coordinates only. Generated descriptor markers need callback-time
register/cache custody before admission and report `UnsupportedMarkerSlot`;
every reached authored factory marker in this corpus is plain. An installed
native viewport with an unowned actor/child frame, unsupported marker generator,
or unowned face plane records an explicit `NativeMarkerBoundary` and does not
accept a float marker write. Free/inspection cameras retain the existing float
lane and remain outside the native-word claim. Source vertex cache warmth
outside the tested attachment and marker paths is not a generalized claim of
complete native projector execution.

A profile combining preceding Sub-H/I selector ranges with Sub-M still needs
composite dispatch; none occurs in the audited Type6/66 metadata. Native
linked presentation-row `+2A` clears remain unrepresented. Painter-program
presentation has no applicable ordinary factory installation. The complete
native-owner delivery, exact tested marker arithmetic and rendered comparison
are established, but matched retail/port Level2 pixels remain unverified.
