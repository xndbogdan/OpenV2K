# Shared model/terrain collision

This is the `FUN_00415160` sphere/heightfield callback used by the moving
model's authored collision program. The implementation is
[`models/terrain_collision.rs`](../../crates/v2k-formats/src/models/terrain_collision.rs),
called by the shared sphere/gate/child interpreter in
[`models.rs`](../../crates/v2k-formats/src/models.rs). Actor admission,
response, damage and task ownership remain with the
[actor runtime](ACTOR_RUNTIME.md),
[player](PLAYER_CRAFT.md) and
[damage/death](ENTITY_DAMAGE_AND_DEATH.md) authorities.

## Collision-only query children

The shared walker also supports query-side `0x05` children. Native
`46AF20 ->46B6D0` copies the active callback context, composes the attachment
and signed permutation orientation, installs the child and linked-slot remaps,
then recursively executes the child's collision program. Thus the same
hierarchy can keep `415160` as its terrain primitive callback. It is separate
from the visible render hierarchy and does not use header-radius geometry.

`collide_terrain_raw_oriented_with_pool` provides child model lookup to this
shared path. The existing pool-free entry retains its child-free player
contract; an unavailable child is an explicit error. The focused regression
checks a nested sphere's exact attachment height, and model-contact tests
cover orientation/imports, miss scratch reset before gates, maximum
penetration and first-hit ties. A prior `0x31` mount basis required by child
orientation6 remains unsupported. This change supplies traversal only;
actor-specific terrain admission and responses still belong to their runtime
owners.

## Coordinate and surface contract

The callback adds the materialized sphere center to the query context's
translation and works in integer raw world units. Terrain X/Z cells span
256 raw units; Y is signed Section-10 height byte times 32. Horizontal cell
addresses wrap to eight bits. The sphere's AABB determines the visited cells,
with **Z outside X**. Equal penetrations keep the first result.

The terrain collision surface is one averaged normal per cell. It is not the
renderer’s two triangles or the bilinear surface used by other ride/grounding
owners. Let `h00,h01,h10,h11` denote the heights at `(x,z)`, `(x,z+1)`,
`(x+1,z)` and `(x+1,z+1)`. The callback rejects the cell if the greatest
corner height is at or below `center_y - radius`. Otherwise:

1. `415770` independently cross-products and normalizes the two corner
   tangent pairs, equivalent to normals of
   `[-256*(h10-h00),65536,-256*(h01-h00)]` and
   `[256*(h01-h11),65536,256*(h10-h11)]`.
2. `415610` normalizes the sum of those two **already normalized Q12**
   vectors. Averaging their unnormalized cross products is different.
3. `415890` obtains signed plane distance from the sphere center relative
   to the cell's `(x,h00,z)` corner: `sum(normalQ12 * delta) >> 12`.
   Distance at or above the radius is rejected.
4. `4158C0` multiplies that normal by signed distance, shifts by 12 and
   retains signed-word displacement components. Subtracting them from the
   center projects onto the plane. Both projected X/Z cell bytes must match
   the tested cell (`((projected ^ cell_origin) & 0xFF00) == 0`).
5. An admitted plane hit has penetration `radius - signed_distance` and
   that Q12 normal. Negative distance is accepted: there is no separate
   guessed deep-terrain recovery clamp.

The Q12 helpers use integer square roots and truncating signed division.
`415770` and `415690` first reduce large vectors by arithmetic shifts of
0/5/10/15/20 according to highest set magnitude bit thresholds 14/19/24/29.
`415610` takes the already bounded sum without that scaling. Zero length
returns `[4096,0,0]`. The executable's wrapped products and signed-word
intermediates remain part of the contract.

## Perimeter fallback

A projection outside the cell takes `415900`, which finds the closest
point on the four perimeter edges, clockwise from `(x,z)` through
`(x+1,z)`, `(x+1,z+1)` and `(x,z+1)`. There is no diagonal edge.
Each edge uses its integer length and Q12 direction, clamps the dot-product
distance to its endpoints, and retains signed-word displacement. Strictly
closer **unsigned** squared distances replace the retained point, preserving
edge ties (`415B49`, `415CB6`, `415DDB` use `jae`). The subsequent radius
admission is deliberately **signed**: `4154CF` uses `jge` against wrapped
`radius²`. `457730` returns zero for a signed nonpositive input; otherwise
penetration is the radius minus its signed-word integer distance. This
asymmetry matters when squared distance wraps across bit31.

If the sphere center is above the closest point, `415690` normalizes the
full center-minus-point vector. Otherwise the normal has zero Y, preserving
retail's suppression of downward edge separation. A recovered asymmetric
branch at `41551B/41551F` tests `center_x == closest_x` **and**
`center_z == closest_x`; both true produce `[4096,0,0]` directly. The second
comparison really uses closest **X**. Replacing it with closest Z or merely
relying on a zero-vector fallback changes reachable results.

## Evidence and acceptance boundary

The original `V2000.EXE` instructions at `415160..415601`, `415610..4158F8`,
`415900` and `46ABA0` establish the contract. A local Unicorn harness executes
those original PE instructions with a copied Level-50 terrain buffer; it does
not attach to retail, record gameplay, or emulate the complete engine. This is
an isolated instruction oracle, **not a TTD observation of an actor's route**.
The initial Python transcription in `.tmp/bee-collision-plan.py` matched the
original callback on 2,000 random probes (seed741). A separate optimized
standalone wrapper then compiled the actual Rust `terrain_collision.rs`
module and compared hit presence, Q12 normal and penetration against the
original PE instructions on **31,162 cases**, with zero mismatches:

- 15,080 copied-retail-terrain probes, covering random centers, exact
  tangency, radius0, signed-word Y extremes, cell edges and coordinate wrap
  through both signed32768 and unwrapped65536 boundaries;
- 2,770 each on flat, checkerboard saddle, full signed-byte steep, and
  sloping synthetic terrains, including equal-depth candidates with different
  normals and the source Z-outer/X-inner retention order;
- 5,000 wider integer probes with X/Z in±100,000 and Y in±131,072, plus
  two isolated signed-squared-distance controls. Those probes exposed the
  signed admission and nonpositive-square-root branches above before repair.

Every emulated call was checked to reach its return sentinel rather than
exhaust the instruction limit. The driver and detailed local results are in
`.tmp/terrain-oracle-differential.py` and `.tmp/terrain-oracle-*.json`.
Durable Rust regressions retain selected cases instead of depending on that
ignored tooling. Oversized non-retail radii retain the existing bounded
256-by-256-cell query policy and are outside this differential claim.

For the Type87-related sphere probe at raw center `[627,-3559,1920]`, radius
198, the original callback yields Q12 normal `[247,3903,-1216]` and penetration
572. The former two-triangle implementation missed this contact. A separate
diagonal-center probe `[8192,-2007,8192]`, radius 458, exercises the asymmetric
edge branch: retail returns `[4096,0,0]`, penetration 310, while dropping that
branch returns `[0,0,4096]` at the same depth. This rules out accepting a
random-sample match without the recovered exceptional path.

Validate the affected format tests and the gameplay contact callers, followed
by the workspace suite. The shared callback repair does not by itself close
the [bee water sequence](INTRO2_BEE_WATER.md), natural resurfacing, its audio,
or matched whole-scene presentation. The
[hut crater](INTRO2_TYPE66.md#retail-crater-and-water-comparison) is a separate
terrain mutation owner and retains its independently verified retail profile.
