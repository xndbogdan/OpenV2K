# Working Factory audio

This authority owns Sub-M's persistent sound rows, independently of an
entity's constructor-attached sound and the player's fan. Factory construction,
staffing and production remain in [FACTORY_SYSTEM.md](FACTORY_SYSTEM.md) and
[AUTHORED_BASE_FACTORY_RUNTIME.md](AUTHORED_BASE_FACTORY_RUNTIME.md).

## Source ownership and selector data

`104B0 ->09A80 ->18A90` creates the factory's Sub-M allocation before D4A0
grounds the body. `18A90` copies all 22 Section13 configuration dwords. Word 20
(`+50`) selects the primary sound and word 21 (`+54`) the secondary; zero omits
that row. The constructor resolves each nonzero global Section11 sound and
calls `44C830` with gain 0 and rate `10000`. Its handles live at **Sub-M+8C/+90**,
not entity+8C or entity+B4. The separate null-template branch defaults word 20
to decimal 31; it is not a license to replace an authored zero selector by 31.

`44C830` copies the current raw i16 position into the logical record. Because
this precedes D4A0, the emitter retains the authored constructor position,
even when the body is subsequently grounded. `18F60` passes a null position
to `44C8E0`; its gain/rate changes do not follow the body through `44C920`.

The primary and secondary rows are distinct even if their selectors resolve
to the same PCM. They are also independent of the player's lower-fan sound 31
and Type61's constructor sound 44. A native factory allocation and its Sub-M
component identity own each row; matching public entity IDs in different
managers do not establish ownership.

The canonical ordinary-world corpus (worlds 13–49, 81 factories) selects:

| Primary / secondary | Factory count |
|---|---:|
| 0 / 0 | 14 |
| 31 / 0 | 61 |
| 47 / 0 | 5 |
| 100 / 0 | 1 |

No ordinary factory requests a secondary row. The two-channel constructor and
retune laws still apply to a nonzero secondary configuration; controlled native
fixtures exercise both channels, including two rows using the same PCM.

## Retuning and lifetime

`19010` derives Sub-M+B4 from its completed production/repair branch:
bit 0 means staffing is at capacity (including zero-capacity autonomous
factories); bit 1 marks healthy delivery/cooldown; bit 2 marks active repair.
A damaged positive-capacity factory needs at least one scientist to repair,
but it need not be fully staffed. A zero-capacity factory repairs autonomously.
Reaching maximum health clears the repair condition in that same update.

`18F60` compares the previous and next state before calling `44C8E0`:

| Changed bits | Channel | Next state | Gain Q16 | Playback rate Q16 |
|---|---|---|---|---|
| XOR mask 5 | primary | bit 2 set | `10000` | `18000` (1.5x) |
| XOR mask 5 | primary | bit 2 clear | `(state & 1) << 16` | `10000` (1.0x) |
| XOR mask 2 | secondary | any | `(state & 2) << 15` | `10000` (1.0x) |
| unchanged | either | any | retained | retained |

The primary therefore continues from manufacturing into delivery and while
a staffed factory waits for collection. The phase 0 -> 1 edge does not restart
or stop it. The higher pitch during scientist repair is the **same sample at
1.5x**, not a separate repair sound or an approximation based on health.

`44C970` mixes the logical row using its stored position and gain. Zero gain
or positional culling releases the physical voice while keeping the logical
row; becoming audible again recreates playback. An audible rate change uses
`4957C0` on the existing voice. Direct samples require no alias/warble RNG.
`19750` silences both rows without freeing them; `18BE0` frees secondary then
primary through `44CC90` when Sub-M is released.

## Map and pause masking

Normal map initialization `454EE0` calls `44F3E0(8)` before loading overlay 51;
map exit `4556A0` clears bit 8 through `44F400`. The gameplay pause entry
`4513E0` likewise sets mask bit 8, and `4512E0` clears it. `44CE70` changes
`DAT_004F716C` and, on the zero-to-nonzero edge, calls `44C810` for every
logical positional row. This releases its retained physical buffer (`+1C`)
while retaining the logical row, sample controls and optional warble state.
While masked, `44C970` skips mixing, warble advancement and alias RNG; its
`44C940` cleanup still removes pending disposable requests.

The port suspends the physical handles held by the entity/factory, player-fan
and campaign-gate owners when entering those frozen phases. It clears the
entity owner's admitted alias result because a resumed buffer uses `495480`
again; factory gain/rate and fan logical-creation history are unchanged. The
first resumed world update recreates audible loops using their retained
controls. Repeated suspension is idempotent and consumes no random words.

This is deliberately not a global mixer stop. A disposable positional request
uses the `44C970` branch with row `+18 != 0`, which submits directly through
`4958C0` without storing the physical voice at `+1C`; `44C940` then frees its
logical request. Already-playing one-shots, including UI sounds, therefore
continue. Pending positional one-shots are garbage-collected on the frozen
transition without alias resolution. No playback-domain tagging is needed to
reproduce the retained-loop ownership law.

## Port ownership and evidence

`factory_production` retains the configured voice controls. Both receipt-bound
production machines now retain acknowledged 18F60/C8E0 changes in subsequent
prefixes and completion snapshots. Otherwise status publication would copy
the pre-retune controls back over the live component. `entity_positional_audio`
consumes these native Sub-M rows through the existing positional mixer in all
worlds and preserves physical voice handles across audible retunes.

The accepted `20260818-214036-factory-manufacturing-audio.jsonl` records a
second voice of global 31 / `sound_031.wav` (30306 PCM bytes) at exactly 1.0x
when the Level 1 factory reaches 2/2; the player's lower fan remains 0.5x. The
factory's entity+8C stays zero. The loop survives product creation, whose own
22162-byte sound 44 loop is distinct. The accepted same-session repair sample
`20260818-214313-factory-manufacturing-audio.jsonl` records the extra 31 voice
at exactly 1.5x after hits. These observations are consistent with the source
matrix above. Static `18A90/18F60/19010/44C830/44C8E0/44C970/18BE0` now identifies
the allocation and retune callers missing from the original passive analysis.
No new retail capture is required to establish those owners or rates.

Corpus controls cover all 81 ordinary factory configurations, including authored silence.
Native Level 1 controls exercise manufacturing, damage/repair,
recovery and delivery through the real factory task and retained snapshots.
Mixer controls cover position, independent channels, manager custody, culling,
silencing, release and idempotent map suspension/resumption with retained
warble and fresh alias admission. They do not claim a new matched audible retail/port
recording or complete global sound-allocation ordering across unrelated owners.
