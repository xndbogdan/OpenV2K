# Intro2 Type16 actors

Section-13 spawns5/42 are the two model257 actors. Their native allocation
and actor tasks belong to
[`intro2_type16.rs`](../../crates/v2k-game/src/intro2_type16.rs).
[ACTOR_RUNTIME.md](ACTOR_RUNTIME.md) owns shared task semantics;
Intro2 acceptance owns the whole
scene gate. The native constructor, living tasks, particle hits and Class12
death are implemented; accepted first-query evidence below authenticates
these two allocations independently.

## Native construction and styles

104B0/09A80 allocates A/B/C/D/E/H/J, with eight H records, Sub-A base200,
mass100, health12000, model257 and default behavior flags0439. D4A0 performs
the terrain snap. The component constructor calls run H, D, A, then E;
the remaining components use their generic allocation initialization.
AC60/25680 evaluates the already-linked actor prefix using
the actual shared random stream, in this authored order:

| Predicate | Weight | Class |
|---|---|---|
| Under Attack | 50 | 7 Search Attack |
| People Nearby | 10 | 9 Capture People |
| Player Nearby | 10 | 7 Search Attack |
| Always | 1 | 5 Move About Aimlessly |
| Always | 1 | 4 Defecate Virus |

The nearby test uses strict wrapped XYZ range0xE00. Fresh hit tick0 disables
Under Attack. The second common axis word is3109; it is not a distance.
The birth receipt retains the entity/spawn identity and its own D/E state
across later C690 selection and death, rather than forcing a sampled style.

Class4 B9E0 clears Secondary, calls02820 to install the 02850 terrain
tertiary, then032A0 to install the 2000-ms 02BA0 retarget/mover Primary.
Type16's terrain task lifetime+A2
is0, so that tertiary has no timeout. Primary's zero mover result produces
tag9C01 before the strict `elapsed > 2000` timeout. The generic wrapper
unwinds before dispatching either the tag or timeout through style+00 C690.
Style+04 is also C690 but is the generic9C00 result route, not this tag.
Ordinary16410 suppression applies to task-result reentry.

Class5 ACD0 uses a 5000-ms retarget/mover Primary. Search7 and Capture9 use
the acquisition Secondary plus 500-ms retarget graph. Acquisition can replace
the executing Secondary; its replacement Primary starts on the next actor
pass. Capture's variant1 pursuit uses the shared C7D0/AEE0/AF50 path and
signed `base*4/3 = 266`, retaining Type16's own mover and Sub-D cache.

The exact class4 row4C7E88 and class5 variant0 row4C7930 both have zero
style+34/+38 masks and preserve0439. Class5 variant1 row4C7978 instead adds80
and clears2, yielding04B9; it has the separate CE90 external callback.
No native continuation should treat that externally entered variant as the
ordinary class5 Primary merely because the class number matches.

## Frame and damage ownership

Native12DA0 owns detailed/coarse admission, random waits, elapsed carry and
the 125000-us cap. Primary, Secondary and Tertiary retain their ordered visit
and replacement boundaries. A/B/C/D/H use the shared source order and incoming
nine-word body matrix; the task's Sub-D yaw change does not rebuild it early.
Actual mode0 admits H, while mode1 skips H. DCA0/E870 rebuild the body basis
after tasks, then execute gravity/drag, surface lifetime and master motion.

Sub-E method20 uses the authored300000-us cadence and sound81. Its raw+12
word is158; all four binding selectors are0, so that word is not an animation
binding. Search acquisition, chase/aim and presentation-phase shot delivery
must retain the real E owner and shared random order. Class4 terrain writes
belong to its actual tertiary phase, not a timer attached to rendering.

Primary and infected particle entries remain distinct10EB0/11250 paths.
Primary stamps34 before DAC0/style28; infected writes2000 before DA00/style20.
Checked415040 damage uses the authored seven-channel filter. Primary cue84
requires checked nonzero damage and a non-dying result. The capability8 class5
suffix is independent and can run after death. Type16 has no infected or
generic-hit cue. Standard death uses sound86 and
native Class12, preserving the A/B/C/D/E/H/J allocation and own D/E state.
The living surface lifetime is20000ms. Source-backed callback admissions and
the tested integration, rather than another actor's policy, determine each
supported path.

## Own first-query evidence

The 2026-09-08 read-only `V200001.run` queries join both allocations:

| Spawn / seed | Handle / allocation | First frame tick | First query X/Z |
|---|---|---|---|
| 5 / 05 | 04F70001 / 02AE6A10 | 011E | 9500 / FFFF8BA8 |
| 42 / 1B | 04D20001 / 02AEE440 | 02FF | BD57 / 7A00 |

Every constructor, register, return, reverse-seek frame and classifier read
retains descriptor367B19F4 and thread7114. The actual parent is00401602,
elapsed125000us and query return0041F7A8. Each takes full-reset, returns0 and
fills row0=1; counter+3A is its own seed+1. Only these two spawn/seed pairs
receive a pending first-query reset. Once consumed, normal cache-window
fills, shifts and cadence resume. Generic unresolved origins remain blocked.
The accepted ledger
owns exact retained filenames and validation details; no new recording is
needed for these allocations.

## Validation and remaining acceptance

The late Section10 static pass now admits these two allocations through the
shared [insect static owner](INSECT_STATIC_CONTACT.md). Its retained02CA0
Primary hook, default439 generic crush and11760 response replace the omitted
late pass; the eight-H-record constructor and component allocation remain
Type16's own. Living D440 policy clears terrain/water bit10000, while actual
Class12 restores that permission and uses the shared surface owner. The bounded
live Type66 pair extension also admits the actual model364 hut and model210
factory in either intrusive seat. It retains the02DA0/01A20 Sub-A and Sub-D
branch,1500ms private timer, family RNG and source fixed-body response; the
static02CA0 callback has its separate2500ms timer and no immediate yaw write.

Incoming native Type17 pair contacts now admit Type16's completed allocation
and task/body custody. `4032A0` installs the existing `402DA0` contact callback
at `4032F3` for DefecateVirusWander; `02820 -> 05F80` leaves the emitter's
contact slot null. The shared descriptor contact updates retained private
state, Sub-A and Sub-D without advancing task age or rebuilding the body basis.
Regressions cover the real class-4 constructor, stale/pending/parked owners and
the full spider/Type16 physical pair. The restored static-burn crater/RNG
sequence exposed this previously unadmitted contact during Intro2; no new AI
or contact arithmetic was needed.

Corpus tests cover the authored weighted constructors, both first-query
receipts, incoming-matrix mover order, detailed/coarse callbacks, strict
lifetimes, terrain writes, Search/Capture handoff, shared emitter cadence,
shot FIFO, sound policy, primary/infected hits and native death. Five-second
varied-frame runs exercise both births starting in classes4/5; incomplete
prefix controls verify that elapsed time, task writes and RNG cannot replay.

Hidden OpenGL walkthroughs exercise both births through the final card,
closing Klaus and Level1 reveal, including native Class12 publication and
later removal. They validate the connected Type16 paths independently of
the remaining whole-scene boundaries. Current combined scene validation is
recorded with the [native Type94 checkpoint](INTRO2_TYPE94.md).

Capture attachment, carry/release variants2..5 and their relation cleanup
remain separate from variant1 pursuit. The complete11AD0 active-pair scan,
externally entered class5 variant1 and matched retail scene acceptance are
not established by constructor or first-query evidence. Native runtime and
whole-scene tests must report any unresolved callback prefix without dropping
its owner or substituting a script pose. [Type58](INTRO2_TYPE58.md) now owns
its separate native hit, firing and lifecycle contract, and
[Type94](INTRO2_TYPE94.md) owns its water actor, emitter and native hit paths.
Other unsupported actor cohorts remain separate from Type16 acceptance.
