# The closing scene

Killing world 47's Queen ends the campaign ([TYPE110_QUEEN.md](TYPE110_QUEEN.md)).
Thirty seconds after her death `FUN_00456F20` sets session `+290`, requests
the full-frame flash, switches cheats 1 and 0 on, and moves to logical world
`0x25` (overlay 49) in phase 3. Overlay 49 in phase 3 is the closing scene:
captions, a tour of the villages, and the credits spelled out in the terrain
while an ice ant walks along them. About 290 s later it leaves for world 13
in ordinary play, with both cheats still on.

## Leaving a world in phase 3

The next `FUN_0044FFA0` visit consumes `+290` before anything else and
dispatches event 2, which selects descriptor `4D0A30`:

- `FUN_00453BA0`, its entry, runs `FUN_00453C20` (with no pending respawn):
  the class-1 camera proxy stands on the player's position at rest, heading
  `0x4000`, and Klaus takes command 3, the closing morph. Mode `world[4]`
  (`4D0710`) then runs `FUN_0044FE20`, which resets the 50-Hz clock, and
  `FUN_00450C00`: Klaus, the eye (`FUN_0040ED10` on the proxy) and the
  physical particles. Actors, static programs and contacts wait. The world
  stays drawn through the morph, and the flash sequence covers it and Klaus.
- `FUN_00453E00` waits for Klaus (`FUN_0042D210`), then sets phase 5 unless
  the next world `+34` is `0x25`, and returns to `4D0918`, whose
  `FUN_004515E0` loads the next world.

In the lab both legs take 54 passes at 20 ms: world 47's last tick at pass
3502, overlay 49 loaded at 3557; the closing scene's last tick at 18058,
world 13 loaded at 18112. The port runs the same leg (`ClosingSceneLeg`) and
reaches both loads after 54 frames.

## Loading overlay 49 in phase 3

`FUN_00451710` with phase 3:

- `FUN_00450390` is nonzero below phase 4, so no player is built; `456C20`
  still consumes the player's ordinal, and `FUN_00451C00` (cargo) does not
  run. The authored bodies take stamps `0x9401` on: eleven Type123 people
  (`0x9401..0x940B`), the Type122 ice ant (`0x940C`) and 34 Type52 beacons
  (`0x940D..0x942E`). A lab dump of the live list after the load reads
  exactly these.
- Every body with capability bit 8 (here the ant) loses `0x68000`.
- The class-1 proxy keeps the position `FUN_00453C20` gave it, plus `0x200`
  in Y and `0x700 - (far << 8)` in Z (`FUN_0042E920`, Section 13 `+88`,
  truncated to a word). In the lab it reads `(19004, -2260, 5825)`.
- The CD track follows `FUN_00456AF0`: below phase 3 track 9, in phase 3 the
  world style plus one, as in ordinary play.
- `FUN_004515E0` turns the HUD flag off (`FUN_0042A850(0)`) for a world above
  `0x24` or below phase 5. `FUN_004292B0` draws nothing without a player
  (`FUN_00443260` fails), so neither orbs nor radar appear.

## The cinematic

The world runs through the ordinary world visit. Below phase 5
`FUN_004503C0` drives the proxy toward session `+308`, the body the
Section-2 commands selected, with the snap and one-sixth easing Intro2 uses
([LOADING_TRANSITIONS.md](LOADING_TRANSITIONS.md#phase-tick-fun_004503c0));
its terrain bias is skipped only in world style 6. `FUN_004537F0` draws the
bottom-left emblem below phase 5. No craft keys are installed (`453330`);
the in-play keys still run: Space and Return (`455D30`), Escape and P
(`455D60`) call `FUN_00452770` below phase 5, which skips the rest; Shift+Escape
aborts to the frontend (`455C90`); the map (`456610`) needs phase 5.

`FUN_00452CB0` visits the world's records from its `4D056A` table entry. For
every world that entry is the world's first Section-2 string, except logical
`0x25`: 461, one past overlay 49's first string (460). So the first record,
a camera command on `0x9401` for 0..3000 ms, never runs; for three seconds
the proxy stays where it arrived and the view is olive, in the lab as in the
port. The others:

- captions at 2..4 s ("WELL DONE!"), 4..8 s ("You've completed V2000 by
  killing the Alien Queen."), 10..14, 14..18, 19..23 and 25..29 s;
- `FUN_00452270` body commands: operation 2 over the first second enables
  the people `0x9402..0x940B` (already enabled: they lack capability 8);
  from 3 s the camera takes them in turn, one every two seconds; at 23 s
  operation 2 enables the ant and from 24 s the camera follows it;
- command `0x40` at 290..292 s: `FUN_00452770` sets `+34` to 1 (phase 3
  only), sets `+290` and requests the flash.

`FUN_00452270`'s default case walks the live list for the body whose `+B4`
stamp is the command's; a miss leaves the walk on the last live body.
Operation 2 ORs `0x68000` into its state, 3 clears it, any other selects it
for the camera (`+2D8` the stamp, `+308` its handle), unless the stamp is
already selected. `FUN_00452790` returns after a body command, before it
draws or touches the type-on cadence.

## Arriving in world 13

The exit goes through the same leg; `FUN_00453E00` sets phase 5 for world 1,
and world 13 loads as after Intro2, with Klaus opening over it. `FUN_00451AF0`
restores the player from session `+38`, which `FUN_00451710` last saved when
the Queen's world loaded (phase 3 does not save it). The port carries that
arrival pose to world 13. Cheats 1 and 0 stay on (session `+1C4`).

## Port

`closing_scene.rs` owns the records, the command walk and the proxy's
arrival; `main.rs` runs the scene in the ordinary world loop without a player,
with the proxy as the camera, and the two legs as `ClosingSceneLeg`.
`{"closing_scene": [x, y, z]}` starts it directly for probes.

Matched against a lab run at 20 ms, the port's frames agree for the first
ten seconds: the olive opening, "WELL DONE!", the first village shots.
Later shots follow people and the ant whose own motion parts from retail.

## Open

- The ant's Follow Beacons parts from retail: the lab's ant walks straight
  from `(3840, -22272)` past `(4818, -22512)` at tick 1242 and `(7018,
  -22541)` at 1442, while the port's turns toward the first beacon near tick
  1200 and stalls. The port's credits therefore roll more slowly than
  retail's; the scene still ends at 290 s.
- The people's wander differs after a few seconds (their choices draw from
  the shared RNG, which the port does not run in lockstep), so the later
  village shots show other poses.
- Model 878's two type-14 external-frame records (consumer slots 14 and 15)
  have no owner in the retail view; their callback geometry is skipped and
  logged.
- Whether global sound 50, which the port plays when world 13 loads after
  Intro2, also accompanies overlay 49's load is unproven; the port does not
  play it there.
- Of session `+38`, the port carries only the pose to world 13; hull, craft
  and inventory are the port's own carried state.
