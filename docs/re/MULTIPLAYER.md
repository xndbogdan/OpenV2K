# Multiplayer

V2000's network play is a deathmatch for up to eight players, run on
DirectPlay 3. The port plays it between machines over its own transport
([The port](#the-port)); everything above the transport (menus, messages,
timing, slots, scores and rules) follows the routines below. Retail creates
`IDirectPlay3A` (IID at `004BF370`) from `CLSID_DirectPlay` (`004BF380`), and
its sessions carry V2000's application GUID
`{276B21C0-3D0C-11D2-B1F7-444553540001}` (`004BF3D0`). The Infestation
counterparts are in [INFESTATION_CROSS_REFERENCE.md](INFESTATION_CROSS_REFERENCE.md).

## Sessions

Module `004D4A18` (`FUN_0046BD60` opens it, `FUN_0046BD70` closes it) owns the
DirectPlay object `004F73C8` and its session. `004F73B8` is set on the host.

| Routine | Does |
| --- | --- |
| `FUN_0046BC60` | opens the layer: `CoInitialize`, the stop and receive events (`004F73E8`, `004F73CC`), then a lobby launch's session (`FUN_0046C6B0`); each failure returns its own error |
| `FUN_0046C6B0` | a game launched by a DirectPlay lobby: the lobby's connection settings with session flags `0x44` and 8 players, `Connect`, the local player, and the host flag from the lobby's create flag |
| `FUN_0046BD00` | closes it: stops a running session, closes both events, `CoUninitialize`, and frees the network timer `004F7418` |
| `FUN_0046C0E0` | creates the `IDirectPlay3A` object (`CoCreateInstance`) |
| `FUN_0046C8C0` | resets the session state, creates the object and enumerates the service providers (`EnumConnections` with V2000's GUID), each through `FUN_0046C850`, which copies the record into a global buffer for the caller's callback |
| `FUN_0046CA60` | releases the object, creates a fresh one and initializes it with the chosen connection (`InitializeConnection`) |
| `FUN_0046C9A0` | enumerates the available sessions asynchronously (`EnumSessions`, flags `0x11`) |
| `FUN_0046C110` | hosts: `Open` with `DPOPEN_CREATE` and a session of V2000's GUID (flags `0x44`, migrating host and keep-alive, at most 8 players), then creates the local player |
| `FUN_0046C210` | joins a session by its instance GUID (`DPOPEN_JOIN`), creates the local player and clears the host flag |
| `FUN_0046CAC0` | joins under the Windows user name (`GetUserNameA`, else "unknown") and starts the session |
| `FUN_0046CB50` | hosts a session named after the computer (`GetComputerNameA`, else "Session") through `FUN_0046C110` and starts it |
| `FUN_0046BED0` | starts a session: clears the nine-slot player table `004F73F0` and puts the local DirectPlay id first, sizes the message buffer from `GetCaps` (`FUN_00468FD0`), and starts the receive thread `FUN_0046C600` |
| `FUN_0046C600` | the receive thread: waits on the receive or stop event and drains each batch through `FUN_0046C570` until stopped |
| `FUN_0046C570` | receives messages (`Receive`): those from DirectPlay itself go to `FUN_0046C300`, game messages to the inbound queue (`FUN_00469470`, `FUN_00469520`) |
| `FUN_0046C300` | the system messages: a player joining ("%s has joined") or leaving ("%s has left", freeing its slot), the session lost (`004F73C0`), and host migration ("You have become the host", which sets the host flag) |
| `FUN_0046C050` | sends the pending outgoing buffer to every player (`Send`) and empties it |
| `FUN_0046BE80` | resets before hosting or joining: stops a running session and clears the lost and migrated flags |
| `FUN_0046BFC0` | stops the session: closes it (`FUN_0046C660`), stops the receive thread, returns the local slot to 1 and clears the player table |
| `FUN_0046C660` | closes a session object: destroys the local player, `Close`, `Release`, and closes its event |
| `FUN_0046CA30` | frees a global buffer from the enumeration callbacks |

## Player slots

`004F73F0` holds the DirectPlay ids of the nine slots; `004F7378` is the local
player's slot and `004F737C` flags a change.

| Routine | Does |
| --- | --- |
| `FUN_0046BD80` | the network timer's callback: on the slot authority (`004F73D4`), finds the local slot or takes the first free one; every twelfth visit (or on request, `004F73DC`) sends an empty message; then flushes the outgoing queue (`FUN_004691D0`) and re-arms itself |
| `FUN_0046BC10`, `FUN_0046BB10` | pack the slot table (all nine ids on the slot authority, the local one otherwise); unpack a peer's table, where a client finds its own slot |
| `FUN_0046C0C0` | copies the slot table out |
| `FUN_0046BA90`, `FUN_0046BAC0` | unpack a received dword into `004F73A8` for the caller; read a dword |
| `FUN_0046BAD0` | packs a test message: its length, the id, then `(id * 7) & 0x7F` bytes counting down |

## Names and announcements

Each slot has a 16-byte name (`004FE700`), 6-byte initials (`004FE7A0`) and a
known flag (`004FE6E0`). Announcements come from five line tables: kill,
death and self-kill lines, default names, and host lines.

| Routine | Does |
| --- | --- |
| `FUN_00439290` | resets the multiplayer state: names to "Player %d", and loads the five tables |
| `FUN_004393C0` | loads one table, `kill.txt`, `Die.txt`, `selfkill.txt`, `Names.txt` or `Host.txt`, from the game folder (`FUN_004394F0`), falling back to the language's built-in lines |
| `FUN_004394F0` | reads a text file's lines into a string table |
| `FUN_00439010` | each frame, records the local name and initials (the profile's, or `Names.txt`'s first two lines when present); on the host, takes the frag limit from `Host.txt`'s first line, clamped to 50..5000; and on any slot whose DirectPlay id changed, clears its score and name and stages a departed player's entities for deferred destruction (`FUN_00410B70`) |
| `FUN_00439720` | sends the local name (message `0x14`) once it is known |
| `FUN_004397C0`, `FUN_00439820` | pack the local initials and name (32 bytes); unpack a peer's and mark it known |
| `FUN_00439770` | packs a length-prefixed string of at most 250 characters |
| `FUN_00439880` | unpacks a chat line of at most 79 characters into the message buffer |
| `FUN_00439BE0` | formats an announcement, replacing `@1` and `@2` with the two players' names ("a stranger" when unknown) |
| `FUN_00439C90` | runs on the victim's machine when its player dies: a random line (`FUN_00457930`), a self-kill line, a death line when the world killed the player (killer 0, unless a player's hit is held), otherwise a kill line, shown at once and sent (kind 21); the host scores the kill (`FUN_00439990`), a client advances its kill counter `004DC9A4` and sends it (kind 16). Nothing happens once there is a winner |

## Kills and scores

The score table holds six bytes per slot (`004FE7E0`); `004DC9C0` holds the
frag limit (low word) and the winner (high word).

| Routine | Does |
| --- | --- |
| `FUN_00439DB0` | reports another player's hit on the local player: keeps the attacker for 750 ms (`004DC9C8`, `004DC9C4`), so a death soon after still credits that player, then the host scores the hit (`FUN_00439A70`) and a client sends it (kind 17) |
| `FUN_00439A70` | scores a hit: hitting oneself costs a point; otherwise the attacker gains one, two when the victim's score is higher, and the victim loses as many while it has more; reaching the frag limit names the winner |
| `FUN_00439990` | scores a kill: the killer's kills and the victim's deaths rise by one; a self-kill costs 20 (a score under 21 falls to 0), otherwise the killer gains 20, or 30 when the victim's score is higher; reaching the frag limit names the winner |
| `FUN_00439A40`, `FUN_00439940` | unpack a kill report; unpack a peer's score word and score its change on the host |
| `FUN_00439B40`, `FUN_004398E0` | pack the frag limit, winner and score table; unpack them, calling `FUN_00456750` when the winner changes |
| `FUN_00439BC0`, `FUN_00439BA0`, `FUN_00439700` | pack the kill counter `004DC9A4`, a word and a dword |
| `FUN_00439DF0` | packs a record: a header word from its kind byte and three data bytes, then the optional words its flag bits 0 and 1 select |
| `FUN_00439E70` | calls `FUN_00439E90` with a zero third argument |
| `FUN_00438F60` | reports a game event of kind 0, 1 or 2 with two words: the host sends message 4 with their high bytes, a client message 5 with the kind folded into a third byte |

## Messages

The message table at `0x4CAE08` holds one 16-byte entry per kind: the apply
and pack routines, the kind and a size word. The pair after kind 25,
`FUN_00438BE0` and `FUN_00438BB0`, handles the 12-byte stamp. Impact reactions
send kind 10 for network-owned bodies, and the session tick sends kinds 3, 7,
12, 16, 18, 19 and 21.

| Kind | Size word | Apply | Pack | Carries |
| --- | --- | --- | --- | --- |
| 0 | 0 | `FUN_00438050` | `FUN_00438050` | nothing |
| 1 | 20 | `FUN_00439FB0` | `FUN_00439F80` | a particle |
| 2 | 12 | `FUN_0043A020` | `FUN_00439FF0` | a burst |
| 3 | 16 | `FUN_00437910` | `FUN_00437730` | the sea level and terrain runs |
| 4 | 4 | `FUN_00437B60` | `FUN_00437B00` | one cell, from the host |
| 5 | 4 | `FUN_00437CB0` | `FUN_00437C50` | a client's cell request |
| 6 | 4 | `FUN_00428910` | `FUN_00428900` | a static object's destruction |
| 7 | 64 | `FUN_00417680` | `FUN_00417240` | a body's sync record |
| 8 | 4 | `FUN_00417D90` | `FUN_00417D70` | a body's removal |
| 9 | 8 | `FUN_00417E10` | `FUN_00417DB0` | damage |
| 10 | 16 | `FUN_00417EB0` | `FUN_00417E70` | an impact's velocity and attitude deltas |
| 11 | 4 | `FUN_00417F30` | `FUN_00417D70` | a body's state bit 1 |
| 12 | 64 | `FUN_00417F60` | `FUN_00417FC0` | the live-body list |
| 13 | 12 | `FUN_00418030` | `FUN_004182C0` | an attach |
| 14 | 8 | `FUN_004180A0` | `FUN_004182F0` | a release |
| 15 | 12 | `FUN_00418290` | `FUN_004182C0` | a record for `FUN_004180F0` |
| 16 | 4 | `FUN_00439940` | `FUN_00439BC0` | a score word |
| 17 | 4 | `FUN_00439A40` | `FUN_00439BA0` | a kill report |
| 18 | 40 | `FUN_004398E0` | `FUN_00439B40` | the frag limit, winner and scores |
| 19 | 4 | `FUN_00439720` | `FUN_00439700` | a name request |
| 20 | 32 | `FUN_00439820` | `FUN_004397C0` | a name and initials |
| 21 | 64 | `FUN_00439880` | `FUN_00439770` | a chat or announcement line |
| 22 to 25 | 4, 8, 12, 16 | `FUN_00439E70`, `FUN_00439F20`, `FUN_00439F40`, `FUN_00439F60` | `FUN_00439DF0` | a sound cue at a position (`FUN_00439E90` to `FUN_0044F450`), with optional gain and rate words |

| Routine | Does |
| --- | --- |
| `FUN_00439FB0` | spawns a remote particle from the 20-byte record through `FUN_00440A60` |
| `FUN_0043A020` | presents a remote burst through `FUN_00440950` from the message's three words, with no source |
| `FUN_00437730` | packs a terrain update: the sea level, then run-length pairs from `FUN_004377E0` up to 0x60 bytes or 4,096 cells |
| `FUN_004377E0` | measures a run of up to 255 cells from (x, z), advancing X then Z with wrap, that share the infection bit and, on cells with an attribute byte or flag `0x08`, that flag and whether the attribute byte is clear; returns the length and a flags byte (1 infected, 2 flag `0x08`, 4 attribute byte clear) |
| `FUN_00437910` | applies a terrain update: the sea level when the sender is the host, then each run through `FUN_004379C0`, with `004DC9B4` marking the remote write |
| `FUN_004379C0` | applies a received run of cells from (x, z), advancing X then Z with wrap: sets each cell's infection bit to the run's (`FUN_00433720`) and, on a cell whose attribute byte is set, flag `0x08` to the run's (`FUN_004337E0`), clearing the attribute byte when the run's bit 4 says so (`FUN_00433860`) |
| `FUN_00437B60` | syncs one cell: its material through `FUN_00433900` and its flags `0x10` (`FUN_00433720`) and `0x08` (`FUN_004337E0`) when they differ |
| `FUN_00437CB0` | the host applies a client's cell request: set or clear flag `0x10` (`FUN_00433720`) or `0x08` (`FUN_004337E0`), or set the material (`FUN_00433900`) |
| `FUN_00428910` | destroys the cell's static object, unless it carries `0x800`, through `FUN_00428720` with its kind's template (`004C9E40`) |
| `FUN_00417240` | packs a body's sync record: handle, the word at `+7C` and a flag word that selects the optional fields that follow (position, velocity, Euler attitude, a basis compressed to two rows of 13 bits, the capability word and the `+88`/`+50` payloads), returning -1 when the buffer is too small; a successful pack clears the change words (`+84`, state bits 0-10) |
| `FUN_00417DB0` | packs a damage message: the target handle, the amount and the kind word, with the local slot (`004F7378`) in its high byte |
| `FUN_00417E10` | applies a damage message to a local body through `FUN_00414E90` |
| `FUN_00417EB0` | adds a message's velocity and attitude deltas to a local body |
| `FUN_00417FC0` | lists the handles of every body in state `0x82000000` |
| `FUN_00417F60` | gives each listed body a 750-ms wait (`+78`) and the detail bits `0x06000000` |
| `FUN_00418030` | attaches a local body to its parent through `FUN_00416700`, setting `0x20000000` for flag 1 and clearing render bit `0x800` without flag 2 |
| `FUN_004180A0` | releases a local body from the named parent through `FUN_00416750` |
| `FUN_00438BE0` | keeps a stamp in `004DC780`; when it matches the slot's expected id (`004DC758`) and the session agrees (`FUN_00450C90`), marks the slot ready (`004DC7B8`), and a newer counter is recorded |

## Session tick, handles and queues

| Routine | Does |
| --- | --- |
| `FUN_00450CC0` | the in-game network step (simulation chain `0x4D0700`, after the gameplay frame), for network play only: when the slot or the world changed, flushes (`FUN_004691D0`), records the world (`004DC9D8`), refreshes the stamp (`FUN_00438B60`) and `FUN_00469110(1)`; records the local name, running `FUN_00414990` on a change; runs the session tick (`FUN_00438CB0`); drains the inbound queue (`FUN_004695E0`); and sets the abort flag (`+28D`) when `004F73BC` or the session-lost flag `004F73C0` is set |
| `FUN_00438CB0` | the network session tick: decays the killer hold (`004DC9C4`); every `004CAE00` microseconds sends the live-body list (kind 12) and the periodic messages: the host's announcement of the winner (kind 21) every 150 ticks of the 50 Hz clock (3 s), a random line from `Host.txt`'s second line on, when the file has more than one line, its terrain record (kind 3), the host's or a client's status (kind 18 or 16), and a name request (kind 19) for each newly ready slot; then resends each of the slot's own bodies (kind 7) when its sync timer (`+74`) runs out, skipping bodies in state `0x10000000`: a resting body (state `0x08000000` without `0x80`) after 1 s plus a random 16-bit count of microseconds, one without capability bit 0 (`+64`) after 70 ms plus a random 12-bit count, any other (the player's craft) after 35 ms, counting resends at `+7C`; then flushes (`FUN_004691D0`) |
| `FUN_00438A20` | the network message lines: counts each of the four slots' lifetimes down (`004DC7F0`, 0x54 bytes apart), skips the oldest index past expired ones (`004DC9DC`), and draws the live lines word-wrapped (`FUN_00470AE0`) down the screen's top-left |
| `FUN_00410BB0` | spins until the local network slot `004F7378` is set, then frees each live entity's handle (`FUN_0043A600`) and gives it a new one under that slot (`FUN_0043A290`) |
| `FUN_0043A1D0` | builds one block of network handles: its entries (12 bytes each) chained into a free list, each handle carrying the block's number in its upper bits |
| `FUN_0043A600` | frees a network handle: forgets the cached lookup (`004DC9F0`) when it names this handle and returns the entry to its block's free list |
| `FUN_0043A800` | frees every network handle block (`004DC9F4`, 0x18 bytes each, count `004DC9F8`) and its entry array, and resets the cached lookup |
| `FUN_00469110` | starts a fresh outgoing message on channel n (records at `004F7340`, 0x1C bytes apart): sends a pending one first (`FUN_0046C050`), writes the header (two zero bytes, the channel and the local slot `004F7378`), lets the channel's header callback (`004F7384`) add its part, rounded up to 4 bytes (a negative size is error `0x3902`), and sets the write cursor, the room left and an empty count |
| `FUN_004694F0` | the address of a ring's next entry, `+10` + `+4` × `+C`, giving its index `+C` through the second argument (from the receive step `FUN_0046C570`) |

## The frontend

| Routine | Does |
| --- | --- |
| `FUN_0043D0E0` | opens `0x4C11D0` (the ring's Network prop): clears both lists (`FUN_0043D1B0`, `FUN_0043D070`) and enumerates the connections (`FUN_0046C8C0`) |
| `FUN_0043D110` | a connection row (at most six): its name and connection, the item shown (flags `\|4 &~8`) |
| `FUN_0043CEB0` | a connection chosen: initializes it (`FUN_0046CA60`) and opens the session list `0x4C1198`; a failure plays menu sound 2 (`FUN_0042A860`) |
| `FUN_0043D0D0` | opens `0x4C1198`: hides the session rows and zeroes the search clock `004DCEA8` |
| `FUN_0043CF90` | `0x4C1198`'s tick: when more than 50 ticks of the 50 Hz clock passed since the last search, hides the rows and searches again (`FUN_0046C9A0`) |
| `FUN_0043CFD0` | a session row (at most eight): its name and instance; `FUN_0043CF30` draws string 20 then the name |
| `FUN_0043D200` | leaving a network screen: empties both lists |
| `FUN_0043CD50` | row 0, "Host session on this machine" (string 21): the CD check (`FUN_00496BF0`, `FUN_00496DE0`), a session named after the computer (`FUN_0046CB50`), `FUN_00410BB0`, `FUN_00439290`, then the game starts in mode 7 (`0xC02` to the ring); a failure plays sound 2 |
| `FUN_0043CD00` | a session row: joins it under the Windows user name (`FUN_0046CAC0`) and opens the wait `0x4C10F0` ("Waiting for host to start game", "Please Wait"); a failure plays sound 2 |
| `FUN_0043D270` | opens `0x4C10F0`: clears `004DCEAC` |
| `FUN_0043D210` | draws "Please Wait": each frame records the local name (`FUN_00439010`) and drains the inbound queue (`FUN_004695E0`); once the row is fully typed (reveal `0x7FFF`) and the local slot is known, `FUN_00410BB0` and `004DCEAC` = 1 |
| `FUN_0043C7F0` | the menu tick, for a client with a session open: at `004DCEAC` 1 opens `0x4C1160`; at 2 clears the host's world `004DC9B0`, records the name and drains, and starts mode 7 (`0xC02`) once a host's stamp named the world |
| `FUN_0043D280` | opens `0x4C1160`: resets the multiplayer state (`FUN_00439290`) and sets `004DCEAC` to 2 |
| `FUN_0042D240`..`FUN_0042D290` | frontend keys 1-6: store 1-6 in `DAT_004DB228` |
| `FUN_0042CF40` | starts a game; in mode 7 hands the host `DAT_004DB228` and a client the host's world; clears `DAT_004DB228` at every start |
| `FUN_0042CED0` | the ring's open callback: stops an open session (`FUN_0046BFC0`, `FUN_00410BB0`) unless the action starting is 7 |

With DirectPlay's TCP/IP provider the first search raises DirectPlay's own
address dialog; a blank address searches the local network.

## Starting a game

`FUN_0044F650` mode 7 picks the host's world from the arena table at
`0x4D0D88`: worlds 13, 15, 16, 17, 29 and 32 (global overlays 25, 27, 28,
29, 41 and 44), entry `DAT_004DB228 - 1` when keys 1-6 forced one, otherwise
one of the first five by `FUN_00457930() % 5`. A client loads the host's
world. `FUN_0042E570` builds it; a client skips the authored records (and
`FUN_0042EFB0`), so every body it shows there is a peer's.

`FUN_00451AF0`'s network entry places the player at a random X and Z
(`FUN_00457930`), 0x600 above the higher of the cell's height (× 0x20) and
the sea, with fuel `+4C` 100,000, cargo capacity `+15D` 2, `+159` 0, weapon
slot 0 selector 0x12 (the Chain Gun with infinite ammunition) and profile
`+15B` bit 0 (the Targetter). After a death, `FUN_00450980` re-enters the same
world (mode 3, `+28E` keeping the world) through `FUN_00451AF0` at a new
random point.

## Bodies between machines

Every body has a handle, `slot << 26 | index << 16 | generation`, from its
owner's block (`FUN_0043A6E0(9, 0x100)` makes a block of 256 per slot, grown by
doubling to 0x400). `FUN_00417680` applies a sync record (kind 7): a known
handle updates its body; an unknown one is born (`FUN_00438080`) when the
record carries the type, a player's craft (type 46) arriving as type 51;
otherwise the owner is asked for a whole record (kind 11).

A peer's body carries state bit `0x80000000`. `FUN_00412DA0` runs none of its
callbacks: it moves the body on by its last velocity, `((elapsed >> 5) * v)
>> 15` per axis, and removes it when no record came for 1.25 s (state `0x800`
without `0x08000000`) or 3.5 s otherwise.

| Routine | Does |
| --- | --- |
| `FUN_00414E90` | damage to a peer's body goes to its owner (kind 9: handle, amount, `(attacker type + 6) \| slot << 8`), which applies it through `FUN_00417E10` with the sender's slot. On the local player, a lethal hit runs `FUN_00439C90` (the killer is the attacker's slot when a player's craft, type 46, dealt it) and a player's nonlethal hit `FUN_00439DB0` |
| `FUN_00440A60` | a local birth of a class whose descriptor flags hold 2 is sent (kind 1); a peer's arrives with byte 17 = 2, which sets `+1D` bit 0 (no damage) and is not sent on |
| `FUN_00442950` | classes 52, 68 and 85 hitting an entity deal their damage without testing `+1D` bit 0, so a peer's copy of such a creature shot also hits on every machine that shows it |
| `FUN_00413500` | in network play counts each local body's detail wait `+78` down, holding state `0x06000000` while it runs, and on the host runs `FUN_004132D0` each second while fewer than 32 Type61s live |
| `FUN_004132D0` | the host's power-up: at a random X and Z with no body within 2,048, 0x80 above the ground, a Type61 (`FUN_00438080`) whose payload comes from a random entry of the twelve at `0x4C92D0` |
| `FUN_00447280` | a dying network player drops a Type61 for each weapon slot it held, except selectors 1 and 0x12 (`FUN_00447750`) |

## Scores on screen and the winner

| Routine | Does |
| --- | --- |
| `FUN_00452CB0` | in network play, a row per known slot 1-8, "%1.5s:%d::%d-%d" (initials, score, kills, deaths), at x = width / 4 × column + width / 100 and y = height × 4 / 100 × (row + 1), four to a row; the highest score (at least 1) in the yellow font. With a frag limit the rows blink with `(ticks / 20) & 1`: the dark frames show only scores under the limit less min(limit / 2, 50), or only the winner once there is one |
| `FUN_00438A20` | four message lines of 7.5 s, from system level 3's point 30 down by level 2's value 18, in the yellow font, wrapped at the screen's right edge |
| `FUN_00421590` | a peer's craft (type 51) wears its owner's initials at its origin plus (-100, +128) raw, in the yellow font, while its world Z exceeds the view's by more than 256 (a world-axis test, not depth) |
| `FUN_00456750` | a winner: session `+295`, the full-frame sprite flash. Scores stop changing and the game goes on until the players leave |

## Pause, map and leaving

| Routine | Does |
| --- | --- |
| `FUN_00455D60` | Escape or P: after the level opening, opens the pause menu `0x4D1190` and suspends (`FUN_0044F600`) |
| `FUN_0044F600` | pause reason 8 (`FUN_0044F3E0`); outside network play it also switches to the suspended simulation chain. In network play the world keeps running beneath the menu, which `FUN_0043CBA0` shows with mask 1 |
| `FUN_00456610` | the map opens only after the level opening and outside network play |
| `FUN_00455C90` | Shift+Escape: menu cue 3 and the abort flag `+28D`, which `FUN_0044FE20` turns into the return to the frontend (`FUN_0044FEB0`, which stops the session) |

Becoming the host, or losing the session, sets the abort flag too
(`FUN_00450CC0`).

## The port

The port's transport stands in for DirectPlay: a peer-to-peer UDP session on
port 47629 (searches broadcast there, or go to a typed address), at most
eight players, the host answering searches and joins and resending the
player list each second, a peer silent for 10 s leaving, and the lowest id
taking over from a departed host. It hands the layer above what
`FUN_0046C300` reads (players created and destroyed, the session lost,
becoming the host). Its wire format is its own, so retail cannot join a port
session. It is Infestation's port transport with V2000's application GUID,
limit and names.

Above it the port keeps retail's messages: the slot table on channel 0, the
stamped game messages on channel 1 with their 26 kinds and layouts, the
inbound ring of 64, the 100 ms session tick, handles, sync records and their
timers, scores, lines and the HUD. The menus are retail's screens; the one
connection row is the port's transport, and a port-owned prompt asks for the
host's address in place of DirectPlay's dialog. The CD check before hosting
is not run. A search that loses the selected session moves the cursor to the
Host row; retail leaves it on the hidden row, where Select fails with sound
2.

Held, each with a logged stop ("Network: ... is held"):

- the pause menu over the running world (Escape and P do nothing in network
  play; Shift+Escape leaves);
- the power-ups: the host's spawner (`FUN_004132D0`) and a dying player's
  drops (`FUN_00447280`);
- sending kinds 2 (`FUN_00440950`'s bursts), 3 to 6 (terrain, cells and
  static objects), 10 (impacts on a peer's body), 13 to 15 (attachments) and
  22 to 25 (`FUN_0044F480`'s sound cues, from 32 callers);
- receiving kinds 2 to 6 and 13 to 15, kind 9 for an owned body other than
  the player, and kind 10's attitude deltas;
- the components (Sub-H frames) of a peer's body, which `FUN_0041D0A0` and
  `FUN_00420920` draw on the owner's side.
