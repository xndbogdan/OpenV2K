# Retail capture notes — 2026-07-12

Source: 256.167-second, 960x720/60 Hz gameplay capture supplied by the user.
SHA-256: `1644522F12B76126AA42FBF07DF3EFEFC2C7F6E6D330DBAA8EBABD6A11F94E00`.
Timestamps below are capture timestamps and may be a few frames after the
underlying 15 Hz AVI or 50 Hz game event.

## Startup and frontend

- 00:00-00:05: Grolier Interactive ident.
- 00:05-00:11: moon/Frontier Developments ident.
- 00:11-00:19: pulsing V2000 logo ident.
- About 00:19: frontend menu appears. These three idents are one continuous
  `BANNERHI.AVI` presentation in the shipped data, not three OVL scenes. The
  repository's high-detail AVI is 640x480, 15 Hz, 20.933 seconds with 22050 Hz
  stereo PCM; `BANNERLO.AVI` is the equivalent 320x240 presentation.
- 00:28-00:40: Display submenu. Klaus and the V billboard remain visible above
  the panel and continue their red pulse/animation.

## New Game and Intro2

- About 00:44.4: New Game is selected.
- 00:44.75-00:45.25: the hovercraft flies toward the lens from below while
  Klaus's mouth is open. The menu props/copyright disappear before the craft
  crosses the mouth. An iris/wipe reveals the in-engine Intro2 world.
- About 00:45.5-02:10.5: complete in-engine Intro2 story montage. The animated
  red V2000 emblem remains at bottom-left throughout world cuts, captions,
  meteors, attacks, explosions, virus spread, and `End transmission`.
- About 02:11.5: black Klaus handoff begins with the mouth open.
- About 02:12.5: iris/wipe reveals the first world/loading HUD.
- About 02:13: first playable 0X13XX/Peasant frame. A later live-process
  snapshot confirmed player `(80,60)`, matching 0X13XX exactly; the earlier
  Medaeval identification came from the port loading the wrong default world.

The observed story duration agrees with the `0x10CC` 50 Hz cutoff (86.0 s)
once the menu-to-world wipes are excluded. This validates the Intro2 level-50
path rather than the obsolete 4.3-second interpretation.

### Accepted full-frame DirectDraw trace (2026-07-27)

Local source: `<workspace>\v2000-intro-framecapture.trace`, 670,845,274
compressed bytes, SHA-256
`0E9D4FAB271B639F3281ADDE71E6B6E305DF0A66EDD307B1826822AFE9A8DC41`.
The trace is complete through call `376428`. It retains the retail
Lock-written 1024×768 RGB565 framebuffer rather than only screenshots, making
individual DirectDraw call numbers exact visual boundaries:

- call `321664` selects 1024×768×16 and call `321665` creates the system-memory
  primary flip chain/backbuffer;
- call `325164` is the last frontend per-frame black `COLORFILL`;
  `325165..325649` is the Klaus/iris-to-Intro2 handoff, with a fully visible
  world by `325649`;
- calls `327478..328904` contain the complete first village impact
  presentation. Multiple large square camera-facing fire/smoke frames and
  white flashes coexist with authored grey billboard debris and disappear
  without a camera pan. For classes 16/30/93, the PE resolves Section-3
  frames699--702 through `43DC90 -> 43D410`; the final pixels do not establish
  a missing 3D model-debris path;
- call `356152`, immediately after Flip `356151`, resumes black `COLORFILL`
  for the final outro;
- calls `359193..360083` cover the Klaus/loading handoff; Level-1 imagery
  begins at `360042`, is clear by `360083`, and the first stable
  `Entering Peasant World` frame is `360124`.

The impact crosses both `x=640` and `y=480` and clips only at the actual
1024×768 edges. Its visible square boundary and enormous off-screen extent are
authored billboard presentation, not a 640×480 sub-viewport bug.

The trace wrapper retained every tenth presented Lock and the review atlas
retained every tenth captured sample. Atlas labels are therefore ordinal
navigation aids (one image per 100 presented frames), **not seconds**. Use the
call numbers above for deterministic comparisons.

DirectDraw receives a completed software framebuffer: it performs no later
sprite, colour-key, lighting, fog, shadow, or HUD composition. The trace is
therefore authoritative for final-pixel ordering and visual lifetime, but not
for internal camera matrices, primitive sort keys, particle owners, or AI
callbacks.

### Reproducible offline retail pixel extraction

tools/retail_trace_pixels.py decodes the accepted
trace's apitrace-v6/Snappy container without replaying APIs or opening retail.
It validates compressed token structure even for skipped blobs and exports only
a successful full-surface `Lock` -> exact fake `memcpy` -> successful `Unlock`
-> successful explicit-backbuffer `Flip` sequence. Failed calls, subrectangle
locks, unsupported intervening writes, incomplete copies and changed pointers
cannot lend a completed frame. RGB565 masks, row pitch, payload/PNG hashes and
all four call numbers are recorded in `provenance.json`.

From the repository root, use fresh output paths:

```powershell
python tools/retail_trace_pixels.py v2000-intro-framecapture.trace --start 325165 --end 356151 --export-dir .tmp/retail-intro-pixels --every 10 --output .tmp/retail-intro-pixel-calls.jsonl
python -m unittest discover -s tools -p test_retail_trace_pixels.py -v
```

The accepted Intro2 range has 9,995 successful presentations, 999 retained full
payloads and 100 PNGs with that sampling. Exact inspected examples are Flip
`325496` (intact village), `327666`/`327976` (impact fire), `328596` (large white
destruction effects/debris), `328906` (the final payload copied at `328904`, then
presented after the documented impact range), and `332006`/`333556` (hive camera
handoff, bee and ground insects). These are recognizable final retail pixels,
not reconstruction screenshots. The nine focused helper tests cover compression,
cached call signatures, cross-chunk blobs, completed presentation, and rejected
malformed or invalidated receipts.

This visual corroboration is an independent, unsynchronized run: the trace has
no simulation tick, seed, actor identity or camera-state record. Do not convert
atlas ordinals into seconds or claim tick/RNG matches to the numeric seed-137
40-ms acceptance run. The trace enters Level 1 and contains no Level 2 imagery;
it therefore does not close matched retail Level 2 placement pixels.

### Targeted New Game handoff trace (2026-07-17)

The read-only full-session capture
`20260717-032945-menu-intro2-level1.jsonl` contains 8,314 records (SHA-256
`B36A390B0BEDB0C79D14B12D4E3621F6D7B08A31C714E50C9CED7E040B7A1F92`). It
resolves the first wipe more precisely than video alone:

- New Game is selected at sample 52 / 1,040.2 ms while the outgoing screen
  still owns model 41 `player4` and its 0x7000 fly clock;
- the frontend clears at sample 83 / 1,660.2 ms, about 620 ms later. This is
  the ordinary 626.4-ms fly leg, not command 5's separate 524-ms background
  zoom;
- Intro2's terrain and overlay set are resident by samples 84-85. Its first
  observed world clock is tick 6, while the persistent Klaus entity keeps its
  cover flag through tick 43 and clears it at tick 44;
- Klaus remains the same type-0/global-model-1 entity and model resource across
  the frontend, Intro2, and Level-1 handoffs. The port must therefore carry
  presentation state across the load rather than construct a new wipe;
- Intro2 reaches tick 4300 and tick 4301 before the Level-1 handoff resets the
  world clock. The authored timeline is one continuous 86-second run; revealing
  the world must not replay its first 44 hidden ticks.

The same capture closes three audio/state boundaries without turning the
cinematic into a second time script:

- `FUN_00452790`'s type-on cue is global Section-11 slot **0**, not `0x2D`.
  The executable passes `(int *)*DAT_004FE64C` to `FUN_004958C0`; `0x2D` is a
  text-layout value stored in `local_e0`. Intro2's authored layout records put
  `30` in the character-interval field, and the caller supplies integer
  `(tick * 1000) / 50` milliseconds. Runtime confirms both paths: every Intro2
  caption produces the 2,444-byte `sec11_2XX_000` PCM at native rate, half
  gain, and centred pan on the strict `> 2`-tick cadence (ticks
  `100,103,106,...` for the first caption) until its 30-ms prefix is complete.
  Inclusive adjacent windows both execute at ticks 1100, 1500, and 1750; the
  complete older record clears `DAT_004F72E8` before the new record cues in the
  same table-order pass. The final two raw records are local to the `#` page:
  its traced 80,000-ms origin composes them to 80,000..83,000 and
  83,500..86,000 ms.
  The same PCM and cadence resume for Level-1 text. Aliases 3, 4 and 6 resolve
  to that physical PCM with identical rate/gain, so the DirectSound list cannot
  distinguish which alias a caller used; the executable call resolves the
  logical slot.
- The bottom-left emblem keeps the menu animation's sound ownership throughout
  the first handoff and visible Intro2. Successful starts resolve global alias
  57 to physical slot 52 (`sec11_3XX_045`, 29,012 bytes) and retain the
  approximately 940-ms wrap cadence from the frontend through the last
  cinematic cycle. Two approximately 1.88-second gaps occur while the active
  sound pool is saturated; an active-buffer trace records successful starts,
  not failed allocation attempts, so those gaps are not evidence for pausing
  the emblem clock. No such rumble starts during the post-Intro Klaus bridge.
- Level-1 sound 50 starts at `88,940.360` ms, 40 ms after the level-13 overlay
  catalog is first stable at `88,900.370` ms. It resolves directly to
  `sec11_3XX_043` (62,700 bytes), native 22,050 Hz, centred pan and volume
  20,104. This independently confirms that the cue belongs to completed
  first-world loading, not the tick-4301 Intro2 exit.

**Port status (2026-07-31):** the frontend-started path now retains and updates
one `MenuShell` presentation state while Intro2 is visible, transfers it into
the first post-Intro leg, carries it through the Level-1 load, and restarts only
the bridge-owned fields for the second leg. Primary/twitch phase, Y easing,
callback clock, and private RNG state are no longer reconstructed midway
through the handoff. Direct development entry through `--level 50` remains
deliberately shell-less.

The second run also bounds the two visible type-66 model changes more honestly
than either trace alone. The peasant hut changes from `pesnthut` to `dpsnthut`
at tick 309 in the earlier run and is first observed changed at tick 310 in
this run, a one-sample boundary. Its transition is accompanied in both runs by
the same eleven physical sound starts: three alias-62-family starts, six
alias-59-family starts, and two alias-90-family starts (the aliases are
identified by their 0.9/0.4/0.5 frequency multipliers). This is strong evidence
for a reusable destruction presentation, but the separate entity/audio list
reads do not recover callback order or particle multiplicity.

The factory is explicitly **not** a fixed tick-3500 event: it changes from
`factory6` to `dfactiny` at tick 3500 in the earlier run but tick 3479 in this
one. A time-only `elapsed >= 3500 / 50` selector is therefore a visual proxy,
not recovered actor behavior. Both runs place one alias-62-family sound at the
model change, while their other nearby sounds differ. Recover the type-66
damage/action owner and its event callback before moving either destruction
sound set into the shared world-effects path; do not add sounds at the current
Intro2 elapsed-time selector.

## Full-session runtime timeline

A later read-only 50 Hz trace followed the same retail process from startup
through the first stable playable frame. The local JSONL contains 10,199 valid
records over 131.240 seconds with no malformed lines or broken sample sequence.
Its SHA-256 is
`7B11D835E3B21F95B06C05DB36C4D3777B19F0830577789633426D9CD03D2F27`.

The loaded-overlay catalog changed as follows:

- `0.000`: startup set `[0,1,2,4,5,12,52]`;
- `20.700`: frontend replaces level 4 with level 3;
- `39.100`: New Game unloads levels 5/12 and loads levels 6/50 (`Intro2`);
- `126.200`: level 50/6 unload and first-world level 13 loads;
- `126.300`: shared level 6 reloads alongside level 13.

The persistent Klaus entity changes its relevant presentation flag at
`125.160`, level 13 loads at `126.200`, and Klaus clears the flag at `127.180`.
Retail therefore keeps the second Klaus bridge visible for about two seconds,
with the first world loading halfway through it. Global Section-11 sound 50
(`sec11_3XX_043`, 62,700 bytes of PCM) starts at `126.420` at native 22,050 Hz,
centred pan and volume 20,104. It is not played immediately when Intro2 ends.
The same retained presentation state spans the load. The executable resolves
these sampled intervals as two different progress-driven legs: command 3
starts the full channel-1 morph and closes it by `dt_us >> 4` until
`FUN_0042D210` reports zero, then world-load setup queues command 2 and requests
a 125,000-us entity update to begin opening. Opening ends when progress exceeds
0xFFFF and clears render-admission bit 0x800. They are not two authored
one-second timers, and no fresh Klaus constructor is involved. Exact command
owners and callback ordering are in
[MENU_SYSTEM.md](MENU_SYSTEM.md#persistent-klaus-animation-intro-sequence-behavior).

That earlier trace resolves two Intro2 model-state changes that a static spawn
record alone cannot show (the cross-run audit above bounds their timing):

- peasant hut type 66 at `(-112,-1,-128)` uses slots
  `[364 pesnthut, 365 dpsnthut, 364, 365]` and changes to slot 1/model 365 at
  retail tick 309 in that run. Its raw position simultaneously changes from
  `[-28672,-256,-32768]` to `[-28672,-896,-32768]`, and its live Q1.31 body
  basis changes from
  `[2147352576,0,0, 0,2147352576,0, 0,0,2147352576]` to
  `[2083327002,520570420,0, -489386294,1957545177,-734603409,
  0,754008278,2010688743]` while the authored rotation words remain
  `[16384,0,0]`;
- the Intro2 factory type 66 at `(82,-1.25,-23)` uses
  `[210 factory6, 225 dfactiny, 210, 225]` and changes to slot 1/model 225 at
  retail tick 3500 in that run;
- Intro2 entity 61, type 115, is born and remains on active slot 2. Its model
  slots are `[331 sunflwr, 144 shadow, 332 virusedsunflower, 144 shadow]`, so
  the visible initial mesh is model 332 rather than the ordinary sunflower in
  slot 0. Auditing every other nonzero initial slot found only duplicate model
  IDs; this is the sole additional distinct initial-model selection. Thirteen
  full-session checkpoints across the load boundary and Intro2, through retail
  tick 2401 (Intro2 time 48.10 seconds), keep this entity exactly at raw
  position `[-16384,128,2816]` (`[-64,0.5,11]`), with zero velocity and zero
  rotation. Operation 2 changes its activation flags but does not turn the
  infected plant into a translated or rotated attacker.

The four type-34 meteor actors all spawn as model 560 `grock` with raw velocity
`[1500,-400,-3000]` (`[5.859375,-1.5625,-11.71875]` world units/second before
subsequent fixed-tick integration). Any port path that moves them only on the
vertical axis is therefore demonstrably incomplete.

### Targeted Intro2 meteor trace

The subsequent 20-second `capture-intro-meteor.ps1` run produced 1,000 stable
50-Hz samples with no topology warnings (local capture
`20260712-213639-intro-meteor.jsonl`, SHA-256
`C510FE73C64EFD61BAB2CC1F80C8A188D32B187EC3262BEBD8721275C7CEAB95`). It
followed entity index 31 from construction through deletion:

- transient authored position `[-30976,6400,-28672]`, followed in the same
  retail tick by setup position `[-32256,6400,-26112]` and velocity
  `[1500,-400,-3000]`;
- activation exactly at story tick 150 (3.0 seconds), with first motion on the
  following update;
- at 48 ms of component time, position `[-32188,6378,-26249]` and velocity
  `[1487,-467,-2978]`;
- last sample at 2,985 ms/tick 298: position
  `[-28865,98,32386]` (unwrapped Z `-33150`) and velocity
  `[1092,-3649,-1952]`;
- first observed absent on tick 299, with no persistent ground-clamped or
  destroyed-model state observed. The separate full-session trace caught a
  one-tick impact state at 299 and deletion at 300, so this is a sampling/
  scheduler bracket rather than contradictory behavior.

Static correlation identifies the exact early callback chain, including
gravity, Section-13 drag strength 3, mass 100, and signed-i16 position wrapping.
The trace also exposes a later body-basis tumble phase.

The corrected tracer rerun (`20260712-222040-intro-meteor.jsonl`, SHA-256
`20187425D2C76FFEE005CCE066BCA46038EE8F5659A73FB2062DFE12C5A54EAD`) produced
another 1,000 stable samples with no warnings and resolves the remaining force
ambiguity:

- live entity `+0xC8`, its Section-12 `+0xC0` source, and the behavior-adjusted
  effective mask are all `0x4008`; type-record `+0xC8` is the unrelated zero;
- height limit/mode, configured wind vector, and active wind vector stay zero
  throughout; drag strength stays 3;
- its normal-flight position differs from the first run by at most about 0.11
  world units immediately before collision;
- it catches the one-frame impact state at tick 298: raw position
  `[-28860,95,32396]`, rebound velocity `[290,1436,1740]`, slot 1 and behavior
  `0x004C7150`, followed by deletion at tick 299.

The full-session trace caught the same impact/deletion pair at ticks 299/300.
That one-tick variance follows callback scheduling. No further meteor-force
capture is needed. The port now reproduces the parked `Ry(-pi/2)` basis and an
orthonormal tumble fitted to the captured onset matrices (about 18 rad/s around
axis `[-0.301,+0.071,+0.951]`). Static impact dispatch identifies the remaining
one-frame presentation as a ten-particle class-`0x10` scatter using global
sprites 699..702, followed by one class-`0x12` plume using sprites 873..886,
plus positional global sound 62 (`sec11_3XX_055`). The plume branch is fixed by
the captured raw impact Y of 95 being above the level's `sea_level >> 8` of
-1129. Classes `0x2D`/`0x2E` and sprites 790..792 are the helper's alternate
surface branch, not the captured meteor path; no replacement model is involved.

## Live first-world entity snapshot

A read-only `ReadProcessMemory` snapshot of retail `V2000.EXE` after the first
world appeared resolved the earlier visual ambiguity. The executable is fixed
base x86 (image base `0x00400000`, no ASLR); the snapshot walked the stable
entity list at `DWORD[0x004DB090]` and reported 39 live nodes. Four are
persistent/frontend objects and 35 exactly match `0X13XX.OVL` Section 13.

Key live entities:

- persistent player: type 46, model 41 `player4`, about
  `(77,-2.109375,58)`, facing `0x4000`;
- Main Base: type 6, model 286 `college`, `(80,-3,60)`, behavior descriptor
  `0x004C9480`;
- cargo weight: type 68, model 81 `weight`, `(75,-3.25,60)`;
- Working Factory: type 66, model 227 `lifter`, `(87,-3,58)`, behavior
  descriptor `0x004C9558`;
- type 104 `tuackack` is absent.

The player's `-2.109375` Y versus nearby `-3` terrain gives one instantaneous
gap of `228/256`. Subsequent static tracing of `FUN_0041F1C0` proves this is not
a fixed clearance: the wave-aware lift controller preserves vertical inertia,
so clearance varies and can remain large after a crest falls.

This proves that retail transitions from Intro2 to `0X13XX`, not Medaeval's
`0X14XX`. It also corrects the old assumption that type 6 was the player:
type 6 is the Main Base, while the player is a persistent type-46 object not
stored in the world's Section-13 array. The Working Factory behavior replaces
type 66's static model slot with `lifter` before the first visible frame.

## Gameplay camera

- About 02:23-02:33: the craft is rotated through several headings while the
  camera retains a fixed compass bearing.
- Camera distance is not fixed: it backs away as the craft points toward the
  lens and closes in as it points away. This matches `FUN_0040ED10`:

  `distance = 8 + active_camera * (0x60 / 256) * (1 - cos(relative_heading))`

  With the retail/default Active Camera value 6, the horizontal distance spans
  8.0 (flying away), 10.25 (side-on), and 12.5 units (flying toward the lens).
  Setting Active Camera to zero collapses the range to the fixed 8-unit base.

## Confirmed implementation targets

1. Preserve the complete 20.933-second `BANNERHI.AVI` chain before the menu,
   with `BANNERLO.AVI` as the low-detail fallback.
2. Keep the hovercraft fly-at-camera model pass in the first Klaus mouth wipe.
3. Keep the pulsing V billboard live during every Intro2 shot and black end
   card, then render Klaus for the second mouth/iris handoff.
4. Apply the Active Camera heading-dependent chase distance without rotating
   the camera's compass bearing.
