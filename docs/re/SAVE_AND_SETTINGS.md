# Saves and settings persistence

This document owns V2000 native save restoration, writable checkpoint policy,
and retail settings storage. Binary record framing remains in
[FORMAT_DOCUMENTATION.md](FORMAT_DOCUMENTATION.md#save-file-format); frontend
slot selection remains in [MENU_SYSTEM.md](MENU_SYSTEM.md#frontend-load-sequence-and-slot-count-runtime-confirmed-2026-07-17).

## Native saves

### Native save scope and restoration boundary (CONFIRMED)

Retail exposes 14 playable rows backed by `Slot00` through `Slot13`. Its
fifteenth row is the non-selectable **"Used for game settings"** entry, not a
`Slot14` save. See [MENU_SYSTEM.md](MENU_SYSTEM.md#frontend-load-sequence-and-slot-count-runtime-confirmed-2026-07-17)
for the frontend behavior and the authoritative
[Save File Format](FORMAT_DOCUMENTATION.md#save-file-format) for framing, CRC,
and recovery details.

Retail preserves one complete `0x248`-byte state payload; the earlier minimal
field sketch and conclusion that all other runtime state was reconstructed are
withdrawn. State `+0x20` is a one-based logical campaign/controller slot, not a
global overlay id. Saved-game mode 6 passes it through `FUN_0042E3B0`, and
`FUN_0042E570` loads global gameplay OVL `logical + 12`. The native corpus
therefore resolves Mediaeval `2 -> 14`, Castle `3 -> 15`, Alpine `5 -> 17`, and
Cistern `18 -> 30`. The raw mapping covers logical `1..38`, but 37/38 resolve
to the cinematic Intro1/Intro2 overlays 49/50 and cannot enter the ordinary
save-game `Playing` path. The native loader accepts only interactive
logical worlds `1..36` (global `13..48`) and marks every other value
`Unsupported` rather than addressing a system or cinematic OVL incorrectly.

`FUN_00443260` and `FUN_00443440` prove the live player subset serialized at
state `+0x24`: signed-8.8 position at `+0x24..+0x29`, signed-8.8 velocity at
`+0x2A..+0x2F`, heading/pitch/roll words at `+0x30/+0x32/+0x34`, and hull health
at `+0x3C`. `FUN_00443560` restores the controller block and writes these values
back to the reconstructed type-46 entity, rebuilding its basis through
`413F70`. `451710 ->451AF0 ->443560` creates/restores the player **before**
`42E570` constructs authored actors; `451C00` then restores cargo, followed by
`443260` refreshing the controller snapshot. Native load requests use this
player-first shared construction path, including the first construction stamp,
nonzero pitch/roll, health and the entity `+0x50` shield buffer. The generic
player-at-tail adapter remains specific to portable compatibility snapshots.

The [red shield aura](PLAYER_SHIELD.md) reads this restored buffer. Its
draw-time smoothing and spin are transient new-body state, so loading or
changing worlds restores remaining protection without replaying a pickup.

Within42E570, the33BD0 terrain [gate helpers](GATE_HELPERS.md) construct before
Section13 actors. All helper attempts consume body stamps and selector RNG,
including helpers later marked by42EFB0 duplicate/Hive cleanup. The restored
campaign word selects each helper's visited state through42EF60. Player SubA
uses the process RNG before these helpers; loading never resets a private
propulsion stream or draws again when binding PlayerCraft.

The native request retains all `0x248` original bytes and decodes the following
owner inputs without replaying pickups or substituting fresh defaults:

| State bytes | Restore owner |
|---|---|
| `+0x36`, `+0x38` | Controller movement style and signed fuel |
| `+0x40..+0x13F`, `+0x146` | 32 weapon selector/count pairs and selected slot; stop at the first zero selector |
| `+0x140`, `+0x144`, `+0x147` | Player entity type, extra lives, Targetter/Turbo capability bits |
| `+0x149`, `+0x14A`, `+0x14C..+0x16B` | Raw cargo limit, trophy count, eight packed conventional cargo words |
| `+0x16C`, `+0x170..+0x1AF` | Shield buffer and the separate auxiliary-owned type list |
| `+0x1B4..+0x247` | All 37 campaign progress dwords through `42ECC0`; preserve unrecognized bits |

Unsupported player types/styles, unknown weapon descriptors, invalid selected
slots, and overflowing cargo profiles fail explicitly. Cargo uses the shared
identity/constructor path and its existing unsupported-constructor errors;
loading never manufactures a generic replacement for an unsupported family.

Native completed-world reconstruction now applies `42E440` **after each birth**:
saved-world capability `0x80` fills the owned Sub-M staffing through
`418C20(INT_MAX)`; capability `0x10` invokes the Hive's ordinary death owner;
otherwise capability `0x08` stages `410B70` removal. Claimed selector-`0x3F`
trophies also stage removal, independently of world completion. Births still
consume constructor RNG and stamps, and deferred actors remain allocated until
the normal sweep. Hive death installs `4260F0`, emits `440950`, then performs
static-before-dynamic radial delivery before the next authored actor. The
actual retained Slot00 world-2 fixture exercises nine hostile removals, one
claimed trophy, Hive death/exit, factory capacity three, and kind-22 ignition
at terrain cell `(195,110)`. Its native cargo restoration is covered separately
in the same integration test. Constructor terrain changes use a working copy
that each later birth reads and that the app commits on successful load.
Other death families, attached Hive voices, and constructor-time crater
callbacks still require their explicit live owners and reject unsupported loads.

Session `+0x1C4` (state `+0x1B0`, applied by `44F830 ->450FC0`) and controller
`+0x195` remain retained inputs awaiting complete runtime ownership. This is
bounded native restoration, not a claim that every saved field is implemented.

The four-byte tail has separate semantics: `456C70 ->438020` restores it to
the resource-notification seen-event mask at `[session+0x2C4]+0x0C`. Mode 6
then clears that mask through `437FF0` at `44F7E1`, before authored/cargo
events repopulate it. Do not import the tail directly into the final HUD mask.

Portable JSON saves are also explicit compatibility previews. New writes carry
position, velocity, heading, port-owned body pitch/roll, and health; their
craft mode, fuel, damage buffer, inventory, cargo, capabilities, and campaign
state remain outside the format. Old level-only JSON imported from
`data-root/saves` fails closed as `Unsupported` and cannot shadow a valid
same-numbered legacy native save; malformed or unreadable legacy JSON is
`Corrupt`, and both classifications fall back to native evidence when it
exists. With no native fallback they remain non-loadable. The portable
compatibility API has no gameplay caller. Under the executable-bound runtime
context it writes `slot_N.json` directly beside the executable and can explicitly
replace a native checkpoint at that same writable location.

The campaign-map Save route persists its complete native checkpoint, including
shield, inventory, cargo and campaign fields, to `SlotNN` directly beside the
running port executable. `SavePathContext::beside_executable` binds this writable
location to the absolute executable path before `SaveManager::load_paths` scans
rows. Game-data selection, the working directory, and retail's registry-selected
Save Path never choose the runtime write destination. Writes require an empty
row or an explicit confirmed-overwrite disposition. Saving an imported row
creates a replacement beside the executable; the imported original remains
unchanged. This is the requested portable storage adapter, while native payload,
CRC framing, menu guards, and restoration remain retail-derived.

Confirmed overwrites prepare and flush a unique sibling temporary file before
replacing the destination. Preparation or replacement failure preserves the
previous native file and reports failure; empty-slot creation remains exclusive.

Runtime loading first probes native `SlotNN`, then `slot_N.json`, beside the
executable. A corrupt or unsupported row at that writable location remains
occupied and cannot fall through to an older import. If both are missing,
read-only imports preserve the older precedence: `data-root/saves/SlotNN`
first, even when corrupt, then supported legacy portable JSON, then native
slots in the data directory, its parent, and the registry-selected Save Path.
Invalid legacy JSON may fall back to native evidence as described above.
Loading never migrates, creates, or deletes files. A successful native write
retires only the same-slot portable JSON beside the executable; all imported
`saves/`, data-directory, parent-directory, and registry originals remain intact.

`SaveManager::load_all` retains the historical layout for existing compatibility
fixtures: portable files under `data-root/saves`, native writes into the existing
data/parent save set, and retirement of same-slot legacy shadows after an
explicit successful native write. Runtime uses the executable-bound `load_paths`
API; tests cover the two storage policies independently.


## Retail settings

`FUN_00449140` and `FUN_004491D0` write/read **15 REG_DWORD values** under
`HKEY_CURRENT_USER\Software\Frontier Developments Ltd\V2000\1.0`.
Retail does not store these options in an INI file or a fifteenth save slot.
The static name/offset table is `004C2160..004C21D7`; its setting words are
relative to `004CB3D8` (`FUN_0043C7C0`). Each write is exactly four bytes.
The names and offsets below are decoded from the retail executable, together
with the initialized words at `004CB3D8`.

| Registry value | Offset | Initial DWORD |
|---|---:|---:|
| Sound On | `+00` | 15 |
| Ambient On | `+04` | 15 |
| Joystick Mode | `+08` | 1 |
| Game type | `+0C` | 0 |
| Resolution | `+10` | 1 |
| Renderer | `+14` | 1 |
| Bilinear Filtering | `+18` | 1 |
| Full Screen | `+1C` | 1 |
| Self Righting | `+24` | 1 |
| Full Analogue | `+28` | 0 |
| Sensitivity | `+2C` | 10 |
| Camera | `+30` | 6 |
| Targetter | `+34` | 1 |
| HUD | `+38` | 1 |
| Language | `+3C` | 0 |

The gaps `+20` and `+40` are absent from this table; Vibration at `+40` is
not a persisted PC option. Game type is retained as an opaque DWORD, not
reinterpreted as the port's Difficulty selector. Resolution is the authored
system-overlay tier: 0/1/2/3 mean 320×240/640×480/800×600/1024×768. It is
not the index into the port's modern output-mode list. Ambient's exact
0..15 word persists even though its consumer is only a zero/nonzero CD
pause gate. Self Righting is a strength word, not a boolean.

The port currently projects native tier 0 to Low and tiers 1..3 to High while
retaining the exact DWORD. Its Low/High presentation selects overlay variants
0/1; importing a native 800×600 or 1024×768 mode does not add variant 2/3 layout
support. Modern output dimensions and scaling remain separate port preferences.

`FUN_00449270` separately queries the string values Player Name and Player
Initials. The port has no corresponding live menu/controller owner yet;
these identity preferences are not imported or persisted, and the saved
checkpoint display name must not be substituted for them. `FUN_004493D0`
separately queries **Save Path** as `REG_SZ`; when
absent it uses the executable's working data path. These strings are not
part of the 15-DWORD settings write and must not be overwritten by it.
The port imports Save Path explicitly as the final read-only save source,
after executable-directory slots, legacy `data-root/saves` checkpoints and
portable rows, and the established data-directory/parent-directory native
paths. The selected import path is retained for the occupied-row recheck before
writing. Runtime checkpoints create or replace `SlotNN` beside the executable;
the registry-selected fallback never chooses that destination.

## Port storage and migration

The original retail settings registry key is read-only. Windows writes the
same names and DWORD types under the distinct port-owned key
`HKEY_CURRENT_USER\Software\V2000 Port\1.0`. Non-Windows uses the named
DWORD map in `<data-dir>/settings.json`; Windows also maintains that map as
a portable fallback. This JSON file is a port adapter, not an invented
retail binary format.

Loading precedence for retail options is the port registry key, a valid
`settings.json`, a valid older `config.json`, the original retail key, then
port defaults. Missing or malformed files use the next available source;
absent DWORDs use the fallback record or the source-defined initial word.
The registry loader accepts only four-byte `REG_DWORD` records. Imported
raw DWORDs which exceed the runtime's representable range are retained;
editing one menu setting changes only that setting's word. Opaque Game type
and unknown fallback DWORDs survive the save. Registry writes leave unrelated
values untouched.

Modern backend choice, output width/height, Native/4:3/Stretched scaling,
graphics detail, and port Difficulty are kept separately in
`<data-dir>/port-config.json`; a retired `classic_framebuffer` key there is
ignored. Existing legacy display values
are retained on upgrade and this small file takes precedence afterward.
The old `config.json` is never deleted, renamed, or rewritten. This avoids
losing existing preferences while moving retail options out of that file.
Loading performs no writes. `GameConfig::try_save` reports persistence errors;
file replacement uses a temporary sibling followed by rename. A failed
Windows registry write is reported before replacing either file, since
silently saving a fallback behind an older registry record would be lost
on the next startup.

## Evidence and remaining boundaries

The accepted `nocd02-save-state.txt` replay already covers the retained
native loading path: at `2C89C0:7F` the global `004DEEC8` payload contains
logical world 2; at `2C89C0:60E` all `0x248` bytes have been copied into the
live session. `2C9056:88B..D09` covers controller/player restoration, including
position `(0x2A00, -0x200, -0x3500)`, heading `0x4000`, health `0x9C40`, shield
buffer `0x13BC1`, type 46, and first construction stamp `0x800`.
`2CB7CC:62D..CED` covers the subsequent cargo restore. These observations
support the native player-first loader; they do not establish that every
remaining controller/HUD field has an owner. The current retained boundaries
are described above.

The accepted replay, exact command lines, identity, and companion save UI
transcripts remain in the capture ledger
and [NoCD replay provenance](CLEANSING_VEHICLE.md#nocd02-replay-provenance).
No new recording or replay was required to establish registry persistence:
`449140/4491D0/4493D0`, the static table, and the existing save transcript
are sufficient. The old retail Load shutdown was the already-proven
LaserLock mismatch and is not reproduced by the port.
