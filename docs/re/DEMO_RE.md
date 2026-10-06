# V2000 late-demo reverse-engineering workflow

## Why this build is useful

The German demo is a late, closely related engine build rather than a separate
prototype. Its executable was linked on 17 August 1998, one month before the
retail executable, and retains the complete first world, Intro2, menu, and core
engine. It is useful as a differential oracle: unchanged functions provide
address mappings into retail, while changed functions concentrate late fixes,
demo restrictions, and release-only behavior.

Retail remains the behavioral authority. A demo difference must not be copied
into the port merely because its function is easier to understand.

| build | SHA-256 | PE entry | `.text` virtual size |
|---|---|---:|---:|
| German demo `V2000.EXE` | `797F4046797FF30F1C4299C7F1200D31D8A7A59C567F75E575373E59B9B75F83` | `004AE910` | `000B6442` |
| Retail `V2000.EXE` | `E9BE7A833612FBA3A5A5AB92A974ECE1A689E4B7E72409D9EE8331380573B4BA` | `004B4850` | `000BC852` |

The executable pair has the same six-section PE layout and substantially the
same DirectDraw, DirectInput, DirectSound, DirectPlay, Quartz, Win32, and FGDK
surface. There is no safe global address delta.

## One-command setup

The setup script (`setup-demo-analysis.ps1`) is part of the private analysis
tooling and is not published here. It requires Ghidra 12 and a Java 21 JDK, and
reads the retail and demo corpora. It performs the complete non-destructive setup:

1. verifies the exact demo and retail executable hashes;
2. redirects Ghidra preferences into `.tmp/ghidra_user`;
3. imports and analyzes demo as the destination and retail as the source;
4. creates the `/RetailToDemo` Auto Version Tracking session;
5. exports every match without changing association state;
6. writes a compact summary beside the disposable Ghidra project.

Current output is local and reproducible:

- `.tmp/ghidra_demo/V2000Demo.gpr`
- `.tmp/ghidra_demo/retail-demo-matches.tsv`
- `.tmp/ghidra_demo/changed-function-candidates.tsv`
- `.tmp/ghidra_demo/retail-demo-summary.json`

Use `-Rebuild` only when the disposable project is stale. Use `-Decompile` to
also generate a generic full demo decompilation under
`.tmp/ghidra_demo/demo-bulk`. The generic profile deliberately disables all
hard-coded retail address classifications.

## Reviewing matches in Ghidra

1. Launch Ghidra 12 and open `.tmp/ghidra_demo/V2000Demo.gpr`.
2. Open the Version Tracking tool and select `/RetailToDemo`.
3. Confirm source is `/retail/V2000.EXE` and destination is `/V2000.EXE`.
4. Start with accepted exact instruction/byte matches. They are strong address
   anchors, even if their generated `FUN_` names are not semantic names.
5. Review reference, duplicate, and implied matches in context before using
   them. Duplicate helpers can have several structurally valid destinations.
6. Keep names and conclusions attached to the retail address in the owning
   subsystem note. Do not bulk-transfer speculative demo names into retail.

The initial correlation yields 5,919 match rows and 1,448 unique accepted
function pairs. Multiple correlators can support the same function pair, so raw
row count is not a function count. The generated changed-function report narrows
that set to 43 pairs supported by matching code/data references but not by an
exact byte, instruction, or mnemonic match. Those are the best first candidates
for late demo/retail logic changes; they are still leads, not proven semantics.

The optional generic bulk pass currently decompiles all 1,429 discovered demo
functions with zero failures. Its output is intentionally disposable and lives
under `.tmp/ghidra_demo/demo-bulk`.

Several first-pass pairs already land on named retail subsystems:

| retail | demo | subsystem | why inspect it |
|---:|---:|---|---|
| `0044F650` | `0044EE50` | core level loader | isolates the demo's smaller idle-attract start pool |
| `00412DA0` | `00412D10` | main entity tick | late engine behavior changed despite equal function length |
| `00411AD0` | `00411A60` | entity collision/hit dispatcher | apparent eight-byte change is compiler codegen only; policy is identical |
| `0042AAE0` | `0042A710` | DirectX startup wrapper | exposes late display/device initialization changes |

### Closed active-pair differential

The changed-function lead at retail `FUN_00411AD0` / demo
`FUN_00411A60` does not contain a late collision-policy change. The retail body
is 2,651 bytes and the demo body is 2,659 bytes; aligned control flow and calls
show that the extra eight demo bytes are distributed register/spill codegen
around the physical-response and damage suffix. The outer drivers, retail
`FUN_00411A80` and demo `FUN_00411A10`, are behaviorally identical too.

Both builds retain the same order: subject-authored, candidate-capability-gated
contact sounds, subject behavior,
immediate `0xA300` dispatch/null normalization, candidate behavior when no
subject object remains, subject component slots 0--2, candidate component slots
0--2, then physical response and shared damage. Component returns are ignored;
their mutations remain visible to the later callbacks and suffix. The exact
type-9 component pair is retail `FUN_00402DA0` / demo `FUN_00402DE0`, which
reaches the exact descriptor effect `FUN_00401A20`. The shipping implementation
must therefore preserve retail callback and mutation order; this demo size
difference authorizes no alternate policy.

### Closed campaign and authored-data boundaries

The downstream campaign handoff does not own the promotional-page delta. The
following retail → demo pairs are exact instruction matches: objective predicate
`00415120` → `00415090`, complete Level-1 hive/contact owner `0041BEB0` →
`0041BC80`, terrain-marker constructor `00433BD0` → `00433610`, arrival
resolver `0042E270` → `0042DCC0`, warp-ready predicate `00456CB0` →
`00456610`, teardown `004561A0` → `00455AF0`, and warp request
`00456D10` → `00456670`. The request writer `0042F140` → `0042EB80` is
byte-identical.

Authored data independently closes the same boundary. Across both retained
tiers of Levels 13, 14, 15, and Intro2, Section 10 is exact and every
Section-13 byte from `+0x40` through EOF is exact. Only stale bytes after the
NUL-terminated name differ inside `+0x00..+0x3F`; descriptor parameters,
entity/spawn/config/animation data, and every 0x20-byte campaign/goal record
are unchanged. Both tier copies of world resource X6 are byte-identical too.
Level 15 therefore retains retail routes to globals 19, 30, 17, and 14 even
though the demo omits overlay files 17, 19, and 30.

The complete PRELOAD 15×53 resource-count matrix differs in only five cells:
Section-2 levels 2/3/13 are `118/194/11` in the demo versus `121/182/6` in
retail, Section-5 level 2 is `2` versus `4`, and Section-11 level 3 is `105`
versus `103`. PRELOAD still carries retail counts for the omitted destination
levels, so its count matrix is not the terminal policy.

The presentation payload is now located. Demo system X3 appends a 611-byte,
12-string Section-2 block at local ids `182..193`, cumulative demo global ids
`300..311` (SHA-256
`0829A8167E485C25A290FF4B0E35CACE87B3AA1BED5F4519997113C679C3D51F`).
It contains the early-October availability line, feature list, Grolier URL,
and Space-to-continue prompt. Demo `FUN_00453AF0` is its exact executable
consumer: it reads and draws all twelve global strings at pool offsets
`+0x4B0..+0x4DC`.

The activation trace finds no demo-only upstream branch. The paired mode path
is:

| role | retail | demo | result |
|---|---:|---:|---|
| event dispatcher | `0044FCD0` | `0044F4D0` | event 2 selects the corresponding predecessor descriptor |
| predecessor descriptor | `004D0A30` | `004CAA00` | corresponding mode data |
| phase/flag selector | `00453E00` | `00453560` | same control structure |
| mode switch | `00456680` | `00455FE0` | exact instruction match |
| completion descriptor | `004D0AA0` | `004CAA70` | same mode shape; render content diverges |
| content renderer | `00454390` | `00453AF0` | retail progression/result line versus all twelve demo promo lines |
| descriptor init | `00454EE0` | `00454850` | same structure; loads system overlay 51 |
| descriptor exit | `004556A0` | `00454FF0` | exact instruction match |

The phase selector enters that completion descriptor when session byte
`+0x296 > 4` and either `+0x28E != 0` or retail `0042D210` / demo `0042CC60`
returns 1. A second paired path, `004558A0` / `004551F0`, selects the same
descriptor after a pending transition exceeds 1000 ms. The behavioral delta
therefore begins inside the selected descriptor's render chain, not in its
event dispatch, activation, initialization, or exit.

The core-loader difference is unrelated to manual New Game or that end card.
Sub-mode 4 is selected only by the 60-second idle-attract path. Retail chooses
one of 12 logical start slots; the demo restricts that attract pool to slots 2
and 3. Manual New Game stores sub-mode 2 and takes the fixed Intro2 slot. Do
not transfer the demo attract restriction to the retail opening or campaign.

This closes the promotional page as a substitution inside the normal
completion/progression mode, not a separate campaign-routing policy. The exact
Level-15 writer/event that establishes the phase/flag inputs, whether an
attempted Level-16 availability check precedes the page, and the post-Space
destination remain unproven. Do not fork the Rust campaign transition or infer
a restriction from absent OVL files alone.

These pair identities are correlation results, not permission to copy the demo
body. Compare the two bodies and validate any behavioral conclusion against the
retail build before changing the port.

## Investigation order

Use the demo where the comparison can answer a concrete retail question:

1. **Demo post-page boundary:** identify the Level-15 writer/event feeding the
   proven shared completion descriptor and the exact post-Space destination;
   do not repeat the already-closed promo renderer, authored-route, terrain,
   request, arrival, or teardown comparisons.
2. **Save/load and media policy:** compare the demo path with retail's
   LaserLock-backed Load shutdown boundary; never assume the demo omits all
   media checks merely because it has no later worlds.
3. **Intro2 and menu:** compare Klaus hierarchy, Intro2 actor programs, menu
   resources, and transition state where retail traces still leave ambiguity.
4. **First-world actor/economy logic:** use matched functions to improve names
   around villagers, factories, hives, and task/mover ownership.
5. **Changed resources:** focus asset comparison on overlays X13, X14, X15,
   X2, X3, and X50. The other inspected system overlays are byte-size
   identical, so a blind re-extraction of every overlay has low return.

For each finding, record both addresses, match provenance, the relevant
decompiled/static evidence, and whether retail runtime evidence agrees. That
keeps the demo a high-value map into the shipping program instead of a second,
competing specification.
