# Cheats

V2000 keeps six cheats as bits 0..5 of the session word `+1C4`. Each one
shows the Cheats screen's row of the same index (`0x4D0FE8`, rows at
`4D0F20 + id * 0x1C`):

| Id | Row | Its code | In-play key it also unlocks |
|---:|---|---|---|
| 0 | Skip to %s World | 52367323 | LSHIFT+D: the session's body takes checked damage |
| 1 | Weapons Menu | 66389545 | F9: every Weapons row |
| 2 | Repairs Menu | 11647239 | F8 shields, F11 hull repair, F12 fuel |
| 3 | Cargo Menu | 65437321 | (none; but see the row grants below) |
| 4 | Complete this World | 62367832 | |
| 5 | Complete All Worlds | 96238275 | |

The master code 16245729 switches all six on. The seventh row, Disable All
Cheats, is always shown with the Cheats screen.

## Switching a cheat (`FUN_00450FC0`)

`FUN_00450FC0(id, on)` sets the bit, plays global sound `0x57` through
`FUN_0042A860` and shows the row (item flags lose 8, gain 2). Switching one
off, or switching one on in a network game (`004F741C`), clears the bit and
hides the row without a sound. The pause menu's Cheats row (`4D1154`) shows
while any of the six bits is set. The port keeps the word in its session
and applies it to the menu tree whenever a pause menu opens.

## Typing the codes (`FUN_00455BD0`)

The in-play binding set types digits 1..9 while LSHIFT is held with C, E,
R, A, T, H, S, O or Q (`FUN_00455AB0`..`FUN_00455BB0`); a console pad types
them with R1 and Left, Right, Triangle, Circle, Cross, Square, R2, L1 or L2.
A record acts when its keys become all held, in either order, so the master
code is LSHIFT held over C H E A T S E Q.

Session `+280` holds the digits typed so far, at most eight; `FUN_0044F8B0`
clears it on every world entry. Each digit is appended, then compared with
the twelve codes of the table at `004D1328` in order:

- a code that begins with the digits typed (fewer than eight) keeps them;
- a full eight-digit match switches that code's cheat on and the comparison
  goes on, so the master code's six entries switch on 1, 0, 2, 3, 4 and 5,
  each with its sound;
- with no partial match the buffer restarts from the digit just typed.

After the master code the buffer is therefore `9`, the first digit of
96238275. Each switch-on sound draws the shared random word, as every global
sound does; a retail lab run shows the six draws at the eighth digit.

## The Cheats rows

The Cheats screens lack screen flag `0x80`, so Left and Right call the
row's callback with 0 and 1 (`FUN_0043C6C0`, `FUN_0043C720`) and select
calls it with 2. What each callback does with them:

| Row | Callback | Left / Right | Select |
|---|---|---|---|
| Skip to %s World | `FUN_00450D70` | steps the world | skips to it |
| Weapons, Repairs, Cargo Menu | `FUN_0043B820` | nothing | opens the screen |
| Each weapon, repair and cargo row | `FUN_00455F50` | nothing | grants the row's operation |
| Larger Cargo Capacity | `FUN_00455FD0` | grants operation `0x53A` | the same |
| Add native to hold | `FUN_00455FF0` | grants the world's native | the same |
| Add scientist to hold | `FUN_00456020` | grants the world's scientist | the same |
| All weapons | `FUN_00456050` | nothing | `FUN_00455E20` |
| Complete this World | `FUN_004562A0` | nothing | `FUN_004170F0` |
| Complete All Worlds | `FUN_00456300` | nothing | every campaign word set |
| Disable All Cheats | `FUN_004562B0` | all off, one pop | the same |

**Grants (`FUN_00455F50`).** Only with cheat 3 (Cargo Menu) on does the row
play menu sound 3 and, when the player's body is alive (not dying, state
nonzero), call `FUN_00413600(player, code)`: the player type's
`FUN_00445A90` with the code's low byte as the operation and the signed
rest as the amount, with a pickup's texts and sounds at the player. So
with only the Weapons and Skip cheats the Queen's death switches on, the
weapon rows do nothing; All weapons still works. The Weapons rows carry
the 25 weapon descriptors with 999 rounds (Proximity Mines 10), Megablast
`0x539`, Autobot `0x4038` and Defender Ball `0x3F38`; Repairs carries Extra
Life `0x136`, Extra Fuel `0x186A033`, Shields `0x186A035`, Repairs
`0x186A037`, Targetter `0x3C`, Larger Cargo Capacity, Level Trophy `0x3F`
and Turbo Thrusters `0x3E`; Cargo carries weight `0x4438`, a native, a
scientist, a mini radar `0x5338` and a remote beam-out `0x4A38`.

The native and scientist come from the world's theme (Section 13 `+0x48`):
natives 9, 95, 86 and 78 for themes 1..4 (`FUN_0042EBE0`, none for 5 and 6,
which then do nothing), scientists 8, 91, 90, 79, 116 and 7 for themes 1..6
(`FUN_0042EB70`). Either becomes operation `0x38` with that type.

**All weapons (`FUN_00455E20`).** With cheat 1 on and the player's body
present (dying or not), menu sound 3, then `FUN_00413600` with every
Weapons row whose callback is `FUN_00455F50`, in row order.

**Complete this World (`FUN_004170F0`).** `FUN_00410C10` on every body in
the live list with capability 8 and state bit `0x01000000` but not
`0x10000000`, reading each successor after the death. On Level 1 these are
its four Type17 and three Type47 objective hostiles; the Hive stays.

**Complete All Worlds.** `FUN_0042EE30` ORs `0xF`, then `FUN_0042EE50` ORs
`0xFFFFFFF0`, into each of the 37 campaign control words: every bit set.

**Disable All Cheats.** All six off, then `FUN_0043A9A0` pops one screen
with its fly transition and no sound.

## Skip to World

The selector's value is the process-global `DAT_004D04E0`, 1 at startup and
kept across pause menus and sessions. Left and Right step it between 1 and
the row's argument 36, passing over the arenas 13, 15, 16, 17, 29 and 32,
with menu sound 1, or sound 0 at a limit. The row draws string 138 with the
world's name (string 152 + world, else 127): global levels 13..24, 26,
30..40, 42, 43 and 45..48.

Select plays menu sound 3, sets session `+28C`, empties the menu stack at
once (`FUN_0043C7D0`, no fly-out) and resumes play (`FUN_004512E0`). The
next gameplay frame runs whole; at its end `FUN_0044FFA0` sees `+28C`,
writes the world to `+34` (over any route that frame chose) and raises
event 2, which opens the campaign map as a route does. Space there loads
the world.

No route record stages an arrival (`42E270`), so the player keeps its
position, velocity, heading, pitch, roll and health, and no exit or
pair-link bit is written. A retail lab run from Level 1 shows it: the
pause holds tick 791, tick 792 runs, the map holds the world, and Level 2's
first visit has tick 792's position `(19708, -604, 13935)`, attitude
`0x4000/251/-12`, velocity `(2, 10, -129)` and health 40000. The port
reproduces the shape: one tick after the menu closes, then the map, then
the destination with the source's last pose.

## In-play keys

Each in-play callback acts on a press and tests its cheat bit:

| Key | Callback | Does |
|---|---|---|
| LSHIFT+D | `FUN_00455DB0` | cheat 0: `FUN_00415040` on the session's body `+2A8` with channels 2 and 3, 3000 each, no ratio |
| F9 | `FUN_00455E20` | cheat 1: All weapons, as the row |
| F8 | `FUN_00456100` | cheat 2: operation `0x35` with 100000 (shields) |
| F11 | `FUN_00456070` | cheat 2: operation `0x37` with 40000 less the body's health |
| F12 | `FUN_00455EB0` | cheat 2: operation `0x33` with 200000 less the controller's fuel |

The last three need a living body. The port's own F11 (free-fly camera)
and F12 (debug panel) give way to these while cheat 2 is on in play.

## Saves and settings

A native save keeps the word whole at state `+1B0` (session `+1C4`). New
Game (`FUN_0044F650`) clears it; loading a save restores it, then
`FUN_0044F830` passes each of the six bits back through `FUN_00450FC0`, so
every cheat on plays its sound again. The retail settings (the 15
registry words) hold no cheat. A world change keeps the word, as does the
Queen's closing scene.

## Port status

Ported: the codes and digit keys, every row, the selector and its skip, the
in-play keys, and the word's save and load.

Operation `0x38` (Autobot, Defender Ball, the Cargo rows) runs the
campaign restore's own owner (`EntityManager::dispatch_controller_operation_0x38`):
a zero record of the type at the player's position, attitude and velocity
(`FUN_00438080`), owned by the player (`+60`), then the cargo
(`FUN_00443B50`, when `FUN_00418410` finds a slot) for a body with
capability `0x1000`, or the controller's `+220` list. The weight (Type68),
mini radar (Type83), remote beam-out (Type74), Defender Ball (Type63) and
Autobot (Type64) rows work (the Autobot's own limits are in
[special weapons](PLAYER_CRAFT.md#special-weapons)). A retail lab run
(Level 1, every row once at tick 791) births the five bodies at the player
`(19708, -604, 13935)`; the weight takes the one free slot and the radar,
beamer, native and scientist stay on the ground under the player at Y -883.
The port's test repeats the weight, radar and beamer. A type with no port
constructor is a logged stop that also ends All weapons at that row:

- the natives and scientists (Type9, 95, 86, 78; Type8, 91, 90, 79, 116, 7).

Operation `0x39` (Megablast) runs as a pickup does
([special weapons](PLAYER_CRAFT.md#special-weapons)).

LSHIFT+D targets the player; a possessed body in `+2A8` is not modelled.
Complete this World's victims die through the shared `FUN_00410C10`
owners the Queen's sweep uses; on Level 1 they leave the live list on the
first tick after the menu closes, where retail's leave on the third to
fifth.
