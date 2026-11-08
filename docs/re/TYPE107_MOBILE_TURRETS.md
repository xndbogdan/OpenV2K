# Type107 and Type121 mobile turrets

This document owns the mobile turrets: world 31's two Type107s and the
Type121s of worlds 26, 31 and 40. They are the sixth and seventh rows of the
[Type38 owner](../../crates/v2k-game/src/native_type38.rs) on the shared
ground host. Their class29 turns the walker into a gun turret: the same task
and fire the [native gun turrets](../../crates/v2k-game/src/intro2_gun_turret.rs)
run, on the walker's own components
([`turret`](../../crates/v2k-game/src/native_type38/turret.rs)).

## Rows

| Field | Type107 | Type121 |
|---|---|---|
| Model / health / mass / capability | 24 / 6000 / 100 / `0x1004` | 28 / 30000 / 100 / `0x1004` |
| Damage thresholds | `[0, 6000, 200, 0, 200, 0, 0]` | `[0, 8000, 2000, 500, 200, 0, 0]` |
| Damage multipliers | `[0, 256, 256, 512, 128, 0, 64]` | the same |
| Components | A, B, C, D, E, L without K or H | the same |
| A / B | 1500 / -3000 / 500; 100000 / 100000 | the same |
| C | clearance 100, lift 75, surface mode 1 | the same |
| D | divisor 64, probes 256/128, flags `0x15` | the same |
| L | output selectors 1 and 2, rates 64/31 and 64/31, of a four-word bank | the same |
| E | method 18, 500 ms, spread 4000, threshold 4000, speed 2000, tolerance 2560, sound 88 | method 13, sound 77; otherwise the same |
| E `+17` / bindings | 1 / `[2, 3, 4, 0]` | the same |
| `+C0` / axis | `0x8039` / 5120, `0x1A` | the same |
| Choices | Always ×1 Search And Attack7; BaddieNearby ×10 class29; BaseNearby ×32000 class29 | the same |
| Alternate | class49 (`FUN_0040BAF0`, then `FUN_0040BD20`) | the same |

Neither row has a death, hit or warning sound; the detailed-sound policy is
silent.

## Choosing

Rules 7 and 12 are `FUN_00416560` and `FUN_004165B0`: `FUN_004164D0` within the
live axis range (5120 at birth) for a candidate with capability bit `0x8`
(BaddieNearby) or `0x20` (BaseNearby). A birth sees only the allocations before
it in the authored list, so world 31's two Type107s, authored before their
base, search, while each Type121 has a base in range and starts as a turret.
A later `FUN_0040C690` (a lost target, an infected hit) sees the whole list:
in the retail lab run of world 31 one Type107 turns turret within three ticks.

## Class29

The class29 style `4C8230` has `FUN_0040C690` at `+00`/`+04`/`+20`/`+24`, the
release `FUN_0040D1C0` at `+0C`, set mask `0x4027` at `+34` and the initializer
`FUN_0040D190` at `+40`. `FUN_0040D190` clears Secondary then Primary, so the
walker stops steering, and `FUN_004086D0` builds the turret task in Tertiary
from L's current outputs (`FUN_00408DF0`): bank words 1 and 2 are its starting
yaw and pitch. An infected body trades capability `0x1004` for `0x8`; the task
sets state `0x100`, then `0x28`.

Each visit of `FUN_00408BD0` zeroes the velocity, picks a target every half
second or so (`FUN_00408770`: the nearest body within 5120 by priority; only
a capability-`0x8` body scores high enough to track, or, once the turret is
infected, a capability-`0x1` or `0x4` one), leads it with the method's speed (2000) and, for method 18
only, gravity (`FUN_0044EA60` leads every method except 5, 12 to 15, 17, 21 and
31), writes the angles through L's output words, and fires through
`FUN_00425160`. A target lost for six seconds asks `FUN_0040C690` for a new
choice. The task has no contact hook.

`FUN_00425160` appends each shot through `FUN_004147A0`, then, because E `+17`
is set, a method-15 auxiliary record with a zero direction and time and an
unwritten speed. Before each shot `FUN_00424EE0` writes `0xFFFF` to the bank
word E's selector names: selector 0 (these rows never toggle it, their
alternate muzzle word being zero) names runtime `+04`, bound to bank word 4.
Search's own Aim fires through `FUN_00424650` and appends the same pair; its
target path writes no joint. Both share the allocation's one command queue.

### The muzzle slot is past its node

E's muzzle word is source slot 16, but model 24's only type-14 records are in
model 27, three levels down (24 -> 25 -> 26 -> 27, twice). They are selector
0 (`FUN_00424F20`) and selector 1 (the terminal suffix). Model 27 has 16 slots,
so slot 16 is outside that node, but `FUN_00424FA0` still reads it.

`FUN_00464E60` stores each node's data back to back in the 0x4000-byte
scratch stack at `DAT_004F7428` (`FUN_0046D4B0`): the 20-byte slot cache, then
the face-normal cache, then a saved stack word. So slot 16 is the start of the
node's face-normal cache:

- Its X and Z words are the light words that `FUN_0046D5A0` writes to normal
  entries 0 and 1.
- Only the low half of its Y word is reset.
- Its valid byte is the low byte of normal entry 2, which nothing resets.

With the valid bit set, those words are the point. With it clear, the vertex
handler runs on "record 8", which is the first word of model 27's normal pool.
The transform pass (`FUN_00413BE0`) evaluates no normals, so the valid byte
is left over from earlier draws in the same stack. The port does not model
that scratch stack, so an undrawn Type107's queued shots stay held with
`TransformPass(Pass(Emitter(UnownedSourceSlot(16))))`.

`FUN_0044E770` sends methods 13, 15 and 18 through `FUN_004410B0`: method 18's
class2 and method 13's class56 bolts (class82 below the sea), and method 15's
class32, which the sea plane drops. Class32 starts at the shooter's position
with its velocity.

With `0x4027` the class29 effective policy is `0xC03F`. `FUN_0040DCA0` and
`FUN_0040E870` then run `FUN_0040E640`, keep the body matrix (`0x4000` skips
`FUN_00413F70`), run `FUN_0040E100` (gravity off, drag on) and `FUN_0040DF70`,
which zeroes the vertical velocity, puts the body on the bilinear ground height
and sets the grounded bit. A class29 turret therefore sits on the ground, about
a clearance (100) below where it was born.

A primary hit on class29 calls nothing (`+28` is null); an infected or cured
hit's `+20` reselects. The late static walk finds null hooks in class29 and in
the finished class49 style `4C71E0`.

## Death

Both die through class49: `FUN_0040BAF0` scatters ten class37 particles, then
`FUN_0040BD20` builds a Type60 ring. The Main Base abort walk reaches them the
same way, so worlds 26, 31 and 40 now complete it
([campaign failure](CAMPAIGN_FAILURE.md)).

## Lab

Retail's idle-player skip runs of worlds 26, 31 and 40 (1427 ticks each) and
the port from the same arrival: every mobile turret pairs, none takes damage
or dies on either side, and the turrets stay within 100 units of retail's.
The born turrets drop the clearance to the ground on their first visit; the
far ones (worlds 26 and 40) get it at retail's tick 4 and the port's tick 1,
the coarse-visit phase every far ground actor shows. World 31's turning
Type107 stops and drops in the same tick on both sides and stays within one
unit. The searching Type107 parts by a unit from tick 5, as the other ground
hosts' searchers do without RNG lockstep; by tick 102 it brushes the Type121
in retail, which drifts away from it, and not in the port.
