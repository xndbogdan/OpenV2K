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
| `FUN_0046BA90`, `FUN_0046BAC0` | unpack a received dword into `004F73A8` for the caller; read a dword. Both belong to the default channel descriptor `004D4A00` (with `FUN_0046BA70` and `FUN_0046BAD0`), which `FUN_0046BED0` uses only when its caller passes none; both callers pass the game's `004CAFA8`, so they never run |
| `FUN_0046BAD0` | packs a test message: its length, the id, then `(id * 7) & 0x7F` bytes counting down |

## Names and announcements

Each slot has a 16-byte name (`004FE700`), 6-byte initials (`004FE7A0`) and a
known flag (`004FE6E0`). Announcements come from five line tables: kill,
death and self-kill lines, default names, and host lines.

| Routine | Does |
| --- | --- |
| `FUN_00439290` | resets the multiplayer state: names to "Player %d", and loads the five tables |
| `FUN_004393C0` | loads one table, `kill.txt`, `Die.txt`, `selfkill.txt`, `Names.txt` or `Host.txt`, from the save folder (`FUN_004394F0`: `FUN_004493D0`'s registry Save Path, else the data path, then `\` and the name), skipping leading blanks, empty lines and lines starting with `#`, and falling back to the language's built-in lines |
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
| `FUN_004377E0` | measures a run of up to 255 cells from (x, z), advancing X then Z with wrap, that share the infection bit and, on cells with an attribute byte or flag `0x08`, that flag and a comparison of the attribute byte (below); returns the length and a flags byte (1 infected, 2 flag `0x08`, 4 attribute byte clear, set only with 2) |
| `FUN_00437910` | applies a terrain update: the sea level when the sender is the host, then each run through `FUN_004379C0`, with `004DC9B4` marking the remote write |
| `FUN_004379C0` | applies a received run of cells from (x, z), advancing X then Z with wrap: sets each cell's infection bit to the run's (`FUN_00433720`) and, on a cell whose attribute byte is set, flag `0x08` to the run's (`FUN_004337E0`), clearing the attribute byte when the run's bit 4 says so (`FUN_00433860`) |
| `FUN_00437B60` | syncs one cell: its material through `FUN_00433900` and its flags `0x10` (`FUN_00433720`) and `0x08` (`FUN_004337E0`) when they differ |
| `FUN_00437CB0` | the host applies a client's cell request: set or clear flag `0x10` (`FUN_00433720`) or `0x08` (`FUN_004337E0`), or set the material (`FUN_00433900`) |
| `FUN_00428910` | starts the cell's static-object program here too, unless the cell has no object or its flag `0x08` is set (bit `0x800` of the cell word), through `FUN_00428720` with its kind's program (`004C9E40`); nothing is sent on |
| `FUN_00428720` | starts a static program at a cell unless one runs there (returning 0), which `FUN_00427950` (a hit) and `FUN_00427B20` (a crater) answer with kind 6, the cell's X in the low byte and Z above it |
| `FUN_00417240` | packs a body's sync record: handle, the word at `+7C` and a flag word that selects the optional fields that follow (position, velocity, Euler attitude, a basis compressed to two rows of 13 bits, the capability word and the `+88`/`+50` payloads), returning -1 when the buffer is too small; a successful pack clears the change words (`+84`, state bits 0-10) |
| `FUN_00417DB0` | packs a damage message: the target handle, the amount and the kind word, with the local slot (`004F7378`) in its high byte |
| `FUN_00417E10` | applies a damage message to a local body through `FUN_00414E90` |
| `FUN_00417EB0` | adds a message's velocity and attitude deltas to a local body |
| `FUN_00417FC0` | lists the handles of every body in state `0x82000000` |
| `FUN_00417F60` | gives each listed body a 750-ms wait (`+78`) and the detail bits `0x06000000` |
| `FUN_00418030` | kind 13 (parent, body, flags): a body this machine owns that is attached to nothing (state `0x80001000` clear) is attached to the parent through `FUN_00416700` with `&DAT_004DB0A8` as its slot, setting `0x20000000` for flag 1 and clearing render bit `0x800` without flag 2. Nothing else is written: master motion (`0x40000`) stays |
| `FUN_004180A0` | kind 14: releases a local body attached to the named parent through `FUN_00416750`. Nothing sends kind 14: its pack routine `FUN_004182F0` has no caller but the message table |
| `FUN_00418640` | the Sub-J pose step of `FUN_00412DA0` for a parent this machine owns. In network play (`DAT_004F741C`) a row whose body's relation (`+80`) is not the parent is looked at first: with no relation (`DAT_004DCA00`), a body of this machine's is attached again (`FUN_00416700`) and a peer's is asked for with kind 13 (the parent, the body, flags 1 for the Sub-J's `+0C` and 2 for a slot whose first byte is set), losing render bit `0x800` when that byte is zero; a body attached to anything else loses its row. A row whose body is gone, dying (`0x4000`) or has a zero state word loses its row too, the following rows taking its slot; the rest are posed on the parent |
| `FUN_00418290` | kind 15 (parent, body, X and Z words): `FUN_004180F0` on the body with the parent and the point |
| `FUN_004180F0` | for a body this machine owns that is attached to nothing or to the named parent: a Type93 (`FUN_00438080`) at the point's X and Z on the terrain's bilinear height; when it is built, the body is released (`FUN_00416750`) and attached to it (`FUN_00408F00`). `FUN_00412DA0` runs it too when a local body's parent is gone |
| `FUN_00443D30` | the player's cargo drop (a Sub-J row callback, `FUN_00418500`): for a peer's body no Type93 is built and nothing is released; kind 15 goes to its owner with the player, the body and the drop point's X and Z, then cue 8 there (`FUN_0044F480`) as for a local drop |
| `FUN_00438BE0` | keeps a stamp in `004DC780`; when it matches the slot's expected id (`004DC758`) and the session agrees (`FUN_00450C90`), marks the slot ready (`004DC7B8`), and a newer counter is recorded |

## Terrain and cells

`FUN_00438880` subscribes `FUN_00438F60` to the three cell-change lists of
`FUN_004337A0`: list 0 (`004DC5F8`) the infection bit, notified by
`FUN_00433720` when it changes; list 1 (`004DC628`) flag `0x08`, by
`FUN_004337E0` when it changes; list 2 (`004DC608`) the attribute byte, by
`FUN_00433860` (cleared, notifying the old byte whether or not it was set) and
`FUN_00433900` (written, always notifying). Their local writers are the
infection spread and its particles (`FUN_00402850`, `FUN_00427DE0`,
`FUN_00427E00`, `FUN_00436750`, `FUN_00436960`, `FUN_00437630`,
`FUN_0043E1C0`), the static burns (`FUN_00427950`, `FUN_00427B20`,
`FUN_004281A0`, the Main Base abort's `FUN_00433E30`) and the attribute
clears (`FUN_0041CF90`, `FUN_00427E20`); `FUN_00433900` only the receivers.

| Routine | Does |
| --- | --- |
| `FUN_00438F60` | unless `004DC9B4` is set: the host sends kind 4 for the cell; a client sends kind 5 with an operation, 0 or 1 infect or clean, 2 or 3 burn or unburn, 4 the attribute byte |
| `FUN_00437B00` | packs kind 4: the cell, then its flags and attribute byte as they are now |
| `FUN_00437C50` | packs kind 5; for operation 4 its last byte is the cell's attribute byte now (the other operations carry the position word's top byte, which their apply never reads) |
| `FUN_00437CB0` | on the host only, applies a request when it changes the cell: operations 0-3 through `FUN_00433720` and `FUN_004337E0`, 4 through `FUN_00433900`. `004DC9B4` stays clear, so the host's own notifications send kind 4 to everyone |
| `FUN_00438CB0` | the host's periodic kind 3, while a level terrain is loaded, from the cursor `004DC9E0` (X, Z), which nothing resets, so it runs on from one session to the next |

`FUN_004377E0` compares the attribute byte the wrong way round. A later cell
with neither an attribute byte nor flag `0x08` joins the run on its infection
bit alone; any other must match the run's flag `0x08`, then its "attribute
byte set" (4 when set) must equal the first cell's "attribute byte clear" (4
when clear). So a burned run that starts on a bare cell takes only burned
objects, and one that starts on a burned object takes only bare burned cells.
The receiver clears the attribute byte of every object cell in a run with bit
4.

`004DC9B4` is set while a record from the host (stamp byte 11 nonzero) is
applied by `FUN_00437910` or `FUN_00437B60`, then cleared: those writes
notify no peer, and `FUN_00427760`, the burn list's other subscriber, makes
no burst for them, though its crater still runs. Every other burst of
`FUN_00427760` and `FUN_00427C30` carries network byte 1 on a client
(`DAT_004F741C` set, `DAT_004F73B8` clear), else 0. With byte 1,
`FUN_00440950` sends no kind 2 and `FUN_00440A60` refuses every class whose
descriptor flags hold 2; any nonzero byte sets `+1D` bit 0, so the particles
do no damage.

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
| `FUN_0043A870` | a network handle block by number: the low byte of its argument indexes the blocks (`004DC9F4`, 0x18 bytes each) when below the count `004DC9F8`, else 0 |
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
| `FUN_0043CFD0` | a session row (at most eight): its name and instance; `FUN_0043CF30` draws string 20 then the name. A search that loses the session under the cursor leaves the cursor on its hidden row, lighting no row, and Select there still runs `FUN_0043CD00` (`FUN_0043C450` calls the cursor row's callback whatever its flags) |
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
otherwise the owner is asked for a whole record (kind 11). The birth is the
full constructor (`FUN_0040D720`): the peer's copy has its type's components
and a behavior class chosen here, with the shared random draws those take,
though it never runs them. The record handed to `FUN_00438080` is zero but
for the handle, the type and the position, whose words `FUN_00417680` reads
only when the record carries them (flag `0x20`; otherwise from stale stack).
For type 51, `FUN_00438080` first builds a remote player's controller
(`FUN_00443110`) from a zeroed profile: its body link (`+68`) is still null,
so `FUN_00443560` writes nothing to a body, and the list it joins is only
ever freed (`FUN_00442E90`); the body's `+88` takes the controller's
address until a record writes it. `FUN_004104B0` then gives the copy state
`0x6078801` with the surface bits and bit 31 (the handle's slot is not this
machine's), whose `0x04000000` no sync record clears: a copy is always
jolted by `FUN_00411030`. A peer's craft (type 51: C, D, E, G, J and O,
type 46's flight table and steering, class 7 at weight 10 with alternate
class 25, Sub-E method 0) draws four words: Sub-G's (`FUN_0041B8C0`), the
class selector, and the Sub-G word of each of B6C0's two task constructors
(`FUN_00406070`; type 51 has no Sub-A, whose word the attract craft adds).

The record ends with the body's relation. With state bit `0x1000` it names
the parent (`+80`): a record without its own position or velocity gives the
body the parent's, and when the parent is this machine's and its Sub-J
(`FUN_00418A00`) does not list the body, kind 15 goes back with the parent,
the body and the parent's X and Z, so the owner drops it there. Without the
bit, a body that a parent of this machine's still lists loses render bit
`0x800` (it rides hidden in that parent), and the relation is cleared. A
record that newly sets the dying bit `0x4000` runs `FUN_00417D30`, which
zeroes the word at `+8` of the Sub-H component its type has (`+EC`).

So a player carries a peer's body like its own: the beam (`FUN_00446C80`)
takes any body with capability `0x1000`, and `FUN_00443B50` appends it to the
player's Sub-J and runs `FUN_00416700` on the copy here, its style `+08`
callback included. The owner's next record clears the relation, and the
player's `FUN_00418640` then sends kind 13 each frame until the owner has
attached its body to the player's craft. Dropping it sends kind 15 from
`FUN_00443D30`, and the owner builds the Type93 that sets it down.

A peer's body carries state bit `0x80000000`. `FUN_00412DA0` runs none of its
callbacks: it moves the body on by its last velocity, `((elapsed >> 5) * v)
>> 15` per axis, and removes it when no record came for 1.25 s (state `0x800`
without `0x08000000`) or 3.5 s otherwise.

The record `FUN_00417680` hands `FUN_00438080` is the 0x4C request an
authored record's `FUN_0042E440` builds, but zero after the handle, the type
and the position, so a copy is the authored constructor's body for an empty
record. `FUN_004104B0` takes the handle (`+00`, against `DAT_004DCA00` for an
authored record) and sets bit 31 before Sub-D's `FUN_00409A80` and the
type's initializer run. The common initializer `FUN_004381F0` reads three
pointers from the request: the Sub-N animation (`+40`, through
`FUN_0041BC20`), the Sub-M template (`+44`, through `FUN_00418A90`) and the
initial behavior (`+48`, else the type's own at `+118`, through
`FUN_0040AC60`). All three are null in a copy. `FUN_00418A90` without a
template leaves the Sub-M `FUN_00418A60` zeroed but for `+50`, which becomes
0x1F, so a peer's factory or Main Base copy has no scientists, output or
timers and a primary `FUN_0044C830` voice 0x1F at gain 0, which only its
own callbacks (`FUN_00418F60`) would retune. None of the state word's other
readers (`FUN_0041BEB0`, the emitters `FUN_004013A0`, `FUN_00402850`,
`FUN_004061A0`, `FUN_0040E370`, `FUN_004141D0`, `FUN_00414870`,
`FUN_00422840`, `FUN_00445310`, `FUN_00446FC0`, `FUN_004475F0`, the
contacts `FUN_00411760` and `FUN_00411AD0`, and the network routines) runs
inside a constructor.

`FUN_00411AD0` runs a body's surface and static phases only without state
`0x88000000`: a peer's body has neither, and goes straight to its pairs,
where a pair of two peers' bodies is skipped. `FUN_00417130`'s harmful sea
burns a peer's body like any other, so its owner hears of it as kind 9.

| Routine | Does |
| --- | --- |
| `FUN_00414E90` | damage to a peer's body goes to its owner (kind 9: handle, amount, `(attacker type + 6) \| slot << 8`), whatever dealt it: a particle through `FUN_00415040`, a blast through `FUN_00414E10`, an attached particle through `FUN_004425D0`. On the local player, a lethal hit runs `FUN_00439C90` (the killer is the attacker's slot when a player's craft, type 46, dealt it) and a player's nonlethal hit `FUN_00439DB0` |
| `FUN_004104B0` | the body constructor: the instance damage modifier `+44` is `FUN_004484A0` for type 46, `FUN_00416820` for types 73 and 109, `FUN_00421910` for type 110 and none otherwise; a body whose handle names another slot gets state bit `0x80000000` |
| `FUN_00414AE0` | a blast moves a peer's body here too and sends its owner the impulse (kind 10), but that record's handle comes from a stack slot the loop writes only after the send (`[ESP+14]`, set at `00414C06` from the target's `+5C`): the previous target's handle, or for the first whatever an earlier call left there, and its three attitude words (`[ESP+2E..33]`) are written nowhere in the routine, so they too are earlier calls' leftovers. The owner adds the record (`FUN_00417EB0`) to its own body of that handle, if it has one. The damage then reaches the owner as kind 9 |
| `FUN_00417E10` | kind 9 for a body this machine owns: `FUN_00414E90` with the amount, the attacker's type (the byte less 6) and slot, past `FUN_00415040`'s admission, filter and modifier. So the owner runs the buffer `+50`, the hit cue, the type's hit callback, health and the death `FUN_00410C10`; a craft's kill (type 46) of a body with state `0x1000000` raises the owner's own kill notification (`FUN_004568B0(4)`) |
| `FUN_00440A60` | a local birth of a class whose descriptor flags hold 2 is sent (kind 1); a peer's arrives with byte 17 = 2, which sets `+1D` bit 0 (no damage) and is not sent on |
| `FUN_00442950` | classes 52, 68 and 85 hitting an entity deal their damage without testing `+1D` bit 0, so a peer's copy of such a creature shot also hits on every machine that shows it |
| `FUN_00440950` | a burst: its scatter (`FUN_004407D0`), then for a local one (network byte 0) kind 2 with the request's first 12 bytes (count, X, Y, Z, extent, the two scatter classes), then the surface particle and its own cue 62 (`FUN_0044F450`). `FUN_0043A020` replays a peer's with network byte 2 and no owner, so its particles are harmless and nothing is sent on |
| `FUN_0044F480` | a cue every machine hears: in network play it sends kind 22 (23 with a rate, 24 with a gain, 25 with both), the cue's low byte and the high bytes of X, Y and Z, then plays it here; `FUN_00439E90` plays a peer's through `FUN_0044F450`, the local-only cue. Its callers are the weapons' and creatures' fire (`FUN_00424650`, `FUN_00402300`, `FUN_00425160`), every hit and death cue (`FUN_00414E90` +98, `FUN_00410C10` +90), the low-health and surface cues (`FUN_0040DCA0`, `FUN_0040E370`), Sub-G's flight cue (`FUN_0041AA60`), the attract-attention and attach cues (`FUN_00420870`, `FUN_00420760`), search and run-away (`FUN_0040ADE0`, `FUN_00403F40`), the factory (`FUN_00419010`), cargo (`FUN_00443B50`, `FUN_00443D30`), infection (`FUN_00436750`), a weapon pickup (`FUN_00445A90`), the player's wreck (`FUN_004475F0`) and the Main Base abort (`FUN_00456960`); every caller passes full gain |
| `FUN_00413500` | in network play counts each local body's detail wait `+78` down, holding state `0x06000000` while it runs, and on the host runs `FUN_004132D0` each second while fewer than 32 Type61s live |
| `FUN_004132D0` | the host's power-up: at a random X and Z with no body within 2,048, 0x80 above the ground, a Type61 (`FUN_00438080`) whose payload comes from a random entry of the twelve at `0x4C92D0` |
| `FUN_00447280` | in network play a dying player leaves its power-ups: a Type61 for each weapon slot up to the first empty one whose count is nonzero and whose selector is neither 1 nor 0x12; then one for the cargo capacity (selector 0x3A) when `+199` is above 2, and one for the special weapon (0x39) when its count `+195` is nonzero, each of these two after building the previous request once more. It then clears the active slot (`+196`, `+5A8`) and the weapon component's (`+14`) |
| `FUN_00447750` | where a dropped power-up lands: X and Z within 0x200 of the player and Y up to 0x1FF above it, held between 0x80 over the ground and 0x800 over the higher of that and the sea |
| `FUN_00425AF0` | a player touching a Type61 runs the pickup (`FUN_0040DB20`, `FUN_00445A90`); once it is accepted, kind 8 goes out for the power-up, whichever machine owns it, and the power-up is destroyed (`FUN_00410B70`) |
| `FUN_00417D90` | kind 8: `FUN_00410B70` on the body the handle names, a peer's or one of this machine's, so the owner destroys a power-up a peer picked up. Destroying a peer's body sends nothing (`FUN_004149F0`) |

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
| `FUN_00455D60` | Escape or P (the in-play table at `0x4C25E0`): after the level opening, opens the pause menu `0x4D1190` and suspends (`FUN_004513E0`, `FUN_0044F600`) |
| `FUN_0044F600` | counts the suspension (`+260`); the first raises pause reason 8 (`FUN_0044F3E0`), which stops the world's voices (`FUN_0044CE70`), and outside network play switches to the suspended simulation chain `0x4D08F0`. Then event 5 (`FUN_00494540`) |
| `FUN_0044FCD0`, `FUN_0044FD60` | the in-play mode's events: event 5 picks the mode from the table at `0x4D0DA0` by the suspension and the window's activity; a suspended game gets `0x4D09F8` (`FUN_00451420`), whose keys are the menu table `0x4C45B0` (arrows, Return, Escape back, Shift+Escape abort) in place of the in-play and craft tables, whose tick is the menu's (`FUN_00451480`) and whose draw list (`0x4D0768`) has the menu (`FUN_004514A0`) but neither the world's sound pass (`FUN_00453AA0`) nor the HUD text and scores (`FUN_00452CB0`) |
| `FUN_004512E0` | the resume (after Continue's fly-out): the count falls; at zero, pause reason 8 clears, the simulation chain returns outside network play, and event 5 restores the in-play mode (`FUN_00453330`) |
| `FUN_0043CBA0` | the in-game menu mask: 3, or 1 in network play, so `0x4D1190` shows Continue, Options and Quit there |
| `FUN_00456610` | the map opens only after the level opening and outside network play |
| `FUN_00455C90` | Shift+Escape: menu cue 3 and the abort flag `+28D`, which `FUN_0044FE20` turns into the return to the frontend (`FUN_0044FEB0`, which stops the session) |

Becoming the host, or losing the session, sets the abort flag too
(`FUN_00450CC0`).

So in network play the world runs on under the pause menu: peers keep moving,
shooting and syncing, the local craft gets no keys, and the world's sounds
wait (the sound records `FUN_0044C830` queues are started and discarded by
`FUN_0044C970`, which the pause's draw list never calls, so they play at the
resume).

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

The port sends every cue a `FUN_0044F480` caller it has ported plays, and
the bursts of `FUN_00440950`. `FUN_00402850`'s cue is zero in the shipped
data; the callers it has not ported (`FUN_004024E0`, `FUN_00404D50`,
`FUN_00405760`, `FUN_0040B340`, `FUN_0040B800`, `FUN_0040CAF0` and
`FUN_00422840`) play nothing here either. The Megablast deploy's cue 79
(`FUN_00444400`) is sent with its rate.

Above it the port keeps retail's messages: the slot table on channel 0, the
stamped game messages on channel 1 with their 26 kinds and layouts, the
inbound ring of 64, the 100 ms session tick, handles, sync records and their
timers, scores, lines and the HUD. The menus are retail's screens; the one
connection row is the port's transport, and a port-owned prompt asks for the
host's address in place of DirectPlay's dialog. The CD check before hosting
is not run.

Terrain and cells follow [Terrain and cells](#terrain-and-cells). The level
cache journals each notification its writers make while a network game runs,
with the cell's bytes right after it, and the frame sends the journal as kinds
4 or 5 after its effects' records, and again after the records it applied, so
a host's answer to a request leaves in the same frame as in retail. The
static scheduler likewise records each program a hit or crater starts, sent
as kind 6. A host's
record is applied with the journal set aside (`004DC9B4`). Kind 5's last byte
is 0 for operations 0-3. Not sent: burns made while a world is built, which
the port writes to a private copy of the terrain (retail notifies them too).
A peer's cell change runs the same listeners as a local one: the radar's
`FUN_0044A850` and `FUN_00427C30`'s carriers for an infection (harmless on
a client), `FUN_0044AA50` for a burn bit or an attribute.

The special weapon's power-up (selector `0x39`) is taken as in single
player: the count `+195` rises (at most 4) and a Type65 is born at the
player, and a dying player drops one for its count. Its pods are bodies like
any other, so peers see them through kind 7 and hear the deploy's cue.

The line files are read from retail's Save Path in the registry when it is
set, else from the data folder; a `--settings` run reads no registry and so
uses the data folder.

A player beams up a peer's body when every behavior class its type can be
born with has a null style `+08` (classes 0, 29 and 59: Type68, Type74 and
the gun-turret types), so that `FUN_00416700` runs no callback on the copy
whichever class it holds; any other peer's body stops the beam ("a peer's
cargo attach callback"). Its row then sends kind 13 (`FUN_00418640`'s
network branch, run on the player's Sub-J in the network step), dropping it
sends kind 15 and the cue, and a peer's record that names this machine's
body as a parent which does not list it sends kind 15 (`FUN_00417680`).
The port runs the rest of the player's `FUN_00418640` (the pose, and the
rows of its own bodies) as it does in single player.

`FUN_00414AE0`'s kind 10 is not sent. Its handle and attitude words are
other calls' stack contents (above), which no port can reproduce, and the
owner would add them to whichever of its bodies the stale handle names.

A peer's copy is built by the port's constructor for its type's authored
record, given the empty record above (the Sub-M's null template included),
with bit 31 set: its components, its behavior chosen at birth and the shared
words those draw. A peer's craft (type 51) is built by its constructor as
above. No copy is ever scheduled: no task owner adopts one, and the actor
pass skips its visit. Its contacts skip the surface and static phases. With
two processes on loopback in each of the six arenas, every type the host
sent but those held below was built so: 8, 9, 14, 15, 27, 35, 39, 55, 61,
66, 68, 71, 82, 86, 87, 94, 95, 102, 103, 126, 127, 128 and 129.

A peer's craft is never scheduled either.
A particle's hit on it runs the wrapper its class takes on the copy:
`FUN_00410EB0` (stamping `+34`), the static route's `FUN_00411180` (`+80`
first, eight times the impulse), an infection's `FUN_00411250` or a cure's
`FUN_00411320` (the `0x2000` bit and its cue). The type's hit hook is class
7's `FUN_0040C690`, so `FUN_0040AC60` chooses the copy's behavior again: a
selector word and the two task constructors' Sub-G words. `FUN_00411030`
then draws three more, adds the deltas to the copy (which moves on by them)
and sends them to the owner (kind 10), whose `FUN_00417EB0` adds the same
words to its craft, and `FUN_00415040`'s damage follows as kind 9. Two
machines on loopback agree word for word: each of the host's 23 Chain Gun
hits on the guest's craft sent deltas the guest applied unchanged.

A body this machine owns that a peer's craft carries is attached to that
craft on the peer's kind 13 (`FUN_00418030`, flags as above); the actor's
`FUN_00412DA0` then gives it the peer's craft's position each visit and
sets `0x20`. The peer's kind 15 (`FUN_00418290`) runs `FUN_004180F0`: a
Type93 at the point, on the terrain's bilinear height, whose selector draws
a word; the body's release (`FUN_00416750`, which for Type68's and Type74's
class runs D1C0's terrain basis and living reselection, a second word);
and `FUN_00408F00`'s attach to the Type93, which sets it down as it does a
player's drop. When the peer's craft is gone (it lapsed or left), the
body's own `FUN_00412DA0` finds no parent and runs `FUN_004180F0` at its
own X and Z, from the same two words; its callbacks stay off for that
visit, as the gate read at its entry says while it is carried. A guest
beaming up the host's Type68 in arena 1 and dropping it saw it carried and
set down at the same point as the host.

`FUN_00447280` (class 25's initializer) on a body other than the one the
local controller holds (its `+68`) finds no controller (`FUN_00443AF0`
returns its `0x8802` error),
so it builds no power-ups and no wreck: it clears task slots 2 and 1,
zeroes model word 7 (`FUN_0040A950`) and installs a 3-second timed Primary
(`FUN_004476F0`), drawing nothing. That is what a hit on a dying peer's
craft does to its copy: class 7's `FUN_0040C690` reaches `FUN_0040AC60`,
which for a body with state `0x4000` (`FUN_00416460`) takes the type's
alternate through `FUN_00425660` and `FUN_00438340`; `FUN_0040C6B0`
installs the Dying bounce style (`4CDAA8`, whose `+34` and `+38` state
policies are null, so `FUN_0040EA10` writes no state) and runs
`FUN_00447280`. The jolt and the damage then go to the owner as for a live
craft. The port does all of this but keeps the copy's slot 0 empty, as it
does the local player's, whose timer its host owns: `FUN_00412DA0` never
visits a copy's tasks, so the timed object is never run.

A peer's kind 6 names a cell whose static program starts here too. A
retail sender never sends it for a kind whose program is null: both
`FUN_00427950` and `FUN_00427B20` burn such a cell at once instead. If the
receiver's cell holds such a kind (the two terrains disagree), retail's
`FUN_00428910` still starts the null program, which `FUN_004281A0`
dereferences on its next visit, a crash; the port stops there.

Held, each with a logged stop ("Network: ... is held"):

- a peer's body whose type the port does not build by its own constructor
  (the Hive, type 67, and the exploding ring, type 60, among the arenas'),
  whose copy stays hollow (its models and no components, no behavior chosen
  at birth, no birth draws): "a peer's TypeN copy (its constructor is not
  ported)";
- a peer's Main Base (type 6): the null template gives its Sub-M the voice
  0x1F, which the port's Main Base does not carry;
- a peer's worker whose Job Nearby search meets a hollow copy (types 79, 90,
  91 and 116 beside the arenas' Hives and Main Bases): the search needs the
  copy's Sub-M ("worker: Job Nearby evidence");
- a hit on a peer's copy other than a class-7 flyer's: it sends only its
  damage, without its type's hit hook or the jolt ("FUN_00411030's kind 10
  on a peer's TypeN copy"). A class-7 flyer's copy (types 14, 15, 39, 41, 87
  and 108) takes the hit as a peer's craft does;
- kinds 13 and 15 for an owned body of a type other than Type68 and Type74
  ("a peer's kind 13 for an owned TypeN"), and a kind 13 naming a parent
  unknown here (retail keeps the handle, and `FUN_00412DA0` then drops the
  body where it is). Besides Type68 (class 0), the arenas' bodies a player
  can beam (capability `0x1000`) are people: Type8 (classes 54 and 6),
  Type9 (10, 45, 54, 6), Type86 (45, 10, 54, 6), Type90 (6, 54) and Type95
  (6, 10, 45, 54). Each of those classes' first styles has an attach
  callback at `+08` (`FUN_0040CD50` for 6 and 54, `FUN_0040CE70` for 10 and
  45, styles `4C79C0`, `4C8788`, `4C7618` and `4C86B0`), the carried
  transition, which `FUN_00416700` runs on the beamer's copy and, on kind
  13, on the owner's body, whose kind 15 then runs the carried style's
  release. The port runs those transitions only on this machine's bodies,
  under their task owners and with this machine's player as the parent, so
  the beamer stops first ("a peer's cargo attach callback") and a port
  never sends kind 13 for a person;
- kind 14, which retail never sends;
- a peer's kind 6 for a static of a kind with a null program (a retail
  crash, above) or outside the kinds the port admits;
- a particle's damage to a peer's body of type 73, 109 or 110: a hit on one
  runs its type's hit hook (held, above) before
  `FUN_00415040`'s modifier, which on the copy reads its own `+68` age,
  `+50` and `+30` (`FUN_00416820`, which then births a particle and calls
  `FUN_00410C10`, a no-op on a peer's body) or the Queen's component
  (`FUN_00421910`);
- a peer's damage to an owned creature whose type's hit callback or death the
  port's shared damage phase has not ported (the same types its blasts stop
  at), reported per type ("a peer's damage to an owned TypeN");
- the components (Sub-H frames) of a peer's body, which `FUN_0041D0A0` and
  `FUN_00420920` draw on the owner's side, and the dying record's
  `FUN_00417D30`.
