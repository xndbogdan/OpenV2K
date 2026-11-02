# Type2 tractor-beam walkers and their Type69 beams

This document owns world 47's two Type2 walkers and the Type69 beams their
guns fire. Type2 is the fifth row of the
[Type38 owner](../../crates/v2k-game/src/native_type38.rs) on the shared
ground host; the beams have their own owner,
[`native_type69`](../../crates/v2k-game/src/native_type69.rs).

## Type2

| Field | Value |
|---|---|
| Model / health / mass | 1168 / 4000 / 100 |
| Damage thresholds / multipliers | `[0, 8000, 6000, 5000, 200, 0, 0]` / `[0, 256, 256, 128, 128, 512, 0]` |
| Components | A, B, C, D, E, H (fourteen legs), K without L |
| A | 1500 / -3000 / 200 |
| B / C | 10000 / 10000; clearance 75, surface mode 0 |
| D | Type38's: divisor 100, probes 512/256, flags `0x13` |
| K | selectors 1 and 2 of a two-word bank |
| E | method 25, 400 ms, spread 256, threshold 30000, tolerance 3072, sound 8 |
| `+C0` / axis | `0x39` / 3840, `0x1` |
| Choices | Always ×1 Defecate Virus4, Always ×1 Move About Aimlessly5, UnderAttack ×1 Search And Attack7 |
| Alternate | class1 (BAF0, then BAC0) |

The authored entry is the Type38 owner's `104B0/09A80/D4A0`; K alone runs
through the common mover (`01430` dispatches no L, and the post-Sub-D fan-out
writes K's smoothed value only). A fresh `+34` leaves UnderAttack false, so a
birth publishes Defecate Virus or Aimless; a hit reselects with Search
weighted in. Search's Aim drains method 25 through `44E770`'s actor branch:
the table row `{0, 1, 0, 69}` adds the shooter's velocity to the shot's
direction at speed 1, and `438080` builds a Type69 at the shot's position
with the shooter's current Euler words, then writes the shooter to its
`+60`. Speed 1 at a target that does not move divides by zero in retail's
shot math; the port's shot math stops there too.

## Type69

| Field | Value |
|---|---|
| Model / health / mass / capability | 78 (radius 0) / 1 / 10 / 0 |
| Components | none |
| `+C0` / axis | `0x85` (no gravity, drag or E640; basis rebuilt) / 512, `0x8` |
| Choices / alternate | Always ×1 class58 Tractor Beam / class2 |
| Death sound | 62 |

Nothing touches a beam: its model has no radius, so it meets no contact or
particle, and its state never gains the damage-enabled bit, so blasts skip
it. It dies only through its tasks or the Main Base abort's `10C10`, which
plays sound 62 and runs class2's quiet `C470`.

Class58's search style `4C7228` (`B500`) copies the type's axis `+04`,
clears Tertiary, installs `01F80`'s guard search in Secondary with the style's
filter override `0xC` (capability 4 or 8) and `402B10`'s retarget in Primary
for 1000 ms. The retarget's `01430` has no component to move and returns
nonzero, so its wrapper times out after 1000 ms (the 51st 20-ms visit) and
style `+00` (`CEF0`) calls `10C10`. The search draws a word every visit and,
one time in four, writes the override to the axis and takes the first
eligible candidate inside the 512-unit box;
`C7D0` stores it at context `+08` and selects the tractor style `4C7270`,
whose `C5E0` clears Secondary and Tertiary and installs `4051B0`'s tractor
in Primary (no lifetime).

The tractor's `405240` acts only on a detailed visit. A missing or dying
target returns nonzero and style `+00` (`C690`) reselects class58: the
singleton word, then `B500` again. Otherwise the beam takes the target's
position. With `d` the target minus the shooter, `d²` (wrapping) above
`0x640000` (2560 units) calls `10C10`; above `0x100000` (1024 units) the
normalised `d` (`457960`) scaled by `(dt >> 9) * ((0x640000 - d²) / 20000) >> 8`
(each word the Q31 product's bits 31..46) is taken from the target's
velocity and added to the shooter's. After the tasks `DCA0`/`E870` rebuild
the basis (`13F70`) and decay `E370`'s timer; the master motion then moves
the beam by its own velocity.

## Retail crashes, held in the port

- A beam holding something reads its shooter's position through `+60`
  without a null check. Once the Type2 is gone, retail crashes (`004052F8`,
  reproduced in the lab). The port stops that beam's owner with
  `Type69Error::RetailCrashOwnerGone`, logged.
- Method 25's speed 1 aimed at a perfectly still target divides by zero in
  the shot math; the port's shot math reports the error and fires nothing.

## Validation

[`native_type38/type2_tests.rs`](../../crates/v2k-game/src/native_type38/type2_tests.rs)
authenticates both rows, checks world 47's two births and their K-only bank,
runs both walkers for ten seconds with the static walk, times an empty beam
out at its 51st visit, has a beam hold, pull and finally drop the player, and
stops a held beam whose shooter is gone.
