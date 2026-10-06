# Player shield and trophy pickup

This document owns the player entity's persistent shield buffer and its
`shipaura` presentation. Building Sub-M regeneration and the global
`FUN_00456BD0` notification value are separate state.

## Pickup, damage and persistence

Retail `FUN_00445A90` case `0x3F`, amount zero, writes entity hull `+0x30 =
40000` and shield `+0x50 = 100000` before checking the hidden-trophy campaign
bit. Even a duplicate hidden pickup restores both fields. A new claim enters
`FUN_00456820(0) -> FUN_0042ED20`, setting control-slot bit `0x02` and counting
one trophy. The independently awarded time trophy uses amount one (`0x13F`),
counts a trophy and grants every fifth trophy's extra life, but does not write
either hull field.

`FUN_00414E90` consumes positive `+0x50` before applying remaining damage to
hull health. There is no passive time drain of this buffer. `PlayerHull` owns
this arithmetic; its campaign replacement refills hull and retains the
remaining shield. `443260/443440` snapshots shield into controller `+0x1BC`;
`443560` restores it into the next player entity. Native save state `+0x16C`
retains that same controller word. Existing native save and campaign-arrival
tests exercise nonzero shield restoration before authored actors are born.

## Authored shield presentation

`FUN_004138F0`, after the ordinary active-model submission, reads the live
shield and entity presentation word `+0x54`. With both zero it does nothing.
Otherwise its signed difference is `old_display - shield`:

- If the difference is greater than four, write
  `display = max(old_display - trunc(difference / 4), 0)`.
- Otherwise copy shield directly into display. A pickup therefore appears
  immediately; damage eases the visual value down without altering the shield.
- Below display `40000`, consume one shared RNG word and skip the aura when
  `(rng & 0xFFFF) % max(display >> 13, 1) == 0`. This is depletion flicker,
  not additional damage.

`FUN_004136C0` submits global Section-8 model245 (`shipaura`), referenced by
`[DAT_004FE640 + 0x3D4]`. It copies the entity basis, then applies `457C30` and
`457DD0` at the angle `(entity+0x68 >> 8) & 0xFFFF`. In the port's column basis,
those are local `Ry(-angle)` followed by `Rz(angle)`. Its callback descriptor at
`004C9360` points to `00413670`: selector zero returns process tick masked to
16 bits; every other selector returns `clamp(trunc(difference / 1000), 0, 7)`.
The model's own command stream selects additive textures620..627 and supplies
the geometry and animated size. No generated sphere, tint or texture is used.

`player_shield::PlayerShieldPresentation` retains only transient display and
the current player adapter's admitted callback clock. It resets with each
new body; the actual shield stays in `PlayerHull` and native checkpoints.
Gameplay submits the authored aura after the live body or wreck with its
terrain-light shift. Missing aura assets suppress and log that draw only.
The spinner uses the existing float model-basis adapter; matched rendered
retail acceptance remains open.

Static callback inspection is reproducible with:

```powershell
objdump -s --start-address=0x4c9360 --stop-address=0x4c9370 retail/V2000.EXE
objdump -d --start-address=0x413670 --stop-address=0x4136c0 retail/V2000.EXE
```

## Requested pickup clock policy

The user's October 1 request explicitly stops and hides the clock on physical
trophy collection. This is a **user-directed approximation**, distinct from
the static retail time-award contract: `445A90/56820(0)/42ED20` does not itself
change `+0x1F0`. The retained NoCD05 trophy transcript shows an active clock
before casualty failure and does not record a successful pickup transition.

The port keeps this policy in `TimeTrophyRuntime::hidden_pickup_stops_countdown`.
A successful amount-zero trophy contact sets it; restored hidden bit `0x02`
reconstructs it when reentering that world. Ticking and HUD visibility share
one gate. It does not manufacture bit `0x08`, count another trophy or replay
pickup feedback. Existing timed completion still owns its independent award.
The follow-up owner is the time-trophy controller: a matched pickup observation
must establish any wider native transition before this policy can count as
retail acceptance.
