# Multiplayer

V2000's network play runs on DirectPlay 3. The port has no network play: this
note records what retail does, so code that touches the network state (the
local player slot, remote kills, scores) stays explainable. Retail creates
`IDirectPlay3A` (IID at `004BF370`) from `CLSID_DirectPlay` (`004BF380`), and
its sessions carry V2000's application GUID
`{276B21C0-3D0C-11D2-B1F7-444553540001}` (`004BF3D0`). The Infestation
counterparts are in [INFESTATION_CROSS_REFERENCE.md](INFESTATION_CROSS_REFERENCE.md).

## Sessions

Module `004D4A18` (`FUN_0046BD60` opens it, `FUN_0046BD70` closes it) owns the
DirectPlay object `004F73C8` and its session. `004F73B8` is set on the host.

| Routine | Does |
| --- | --- |
| `FUN_0046BC60` | opens the layer: `CoInitialize`, the stop and receive events (`004F73E8`, `004F73CC`), then the DirectPlay object (`FUN_0046C6B0`); each failure returns its own error |
| `FUN_0046BD00` | closes it: stops a running session, closes both events, `CoUninitialize`, and frees the network timer `004F7418` |
| `FUN_0046C0E0` | creates the `IDirectPlay3A` object (`CoCreateInstance`) |
| `FUN_0046C8C0` | resets the session state, creates the object and enumerates the service providers (`EnumConnections` with V2000's GUID), each through `FUN_0046C850`, which copies the record into a global buffer for the caller's callback |
| `FUN_0046CA60` | releases the object, creates a fresh one and initializes it with the chosen connection (`InitializeConnection`) |
| `FUN_0046C9A0` | enumerates the available sessions asynchronously (`EnumSessions`, flags `0x11`) |
| `FUN_0046C110` | hosts: `Open` with `DPOPEN_CREATE` and a session of V2000's GUID (flags `0x44`, migrating host and keep-alive, at most 8 players), then creates the local player |
| `FUN_0046C210` | joins a session by its instance GUID (`DPOPEN_JOIN`), creates the local player and clears the host flag |
| `FUN_0046CAC0` | joins under the Windows user name (`GetUserNameA`, else "unknown") and starts the session |
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
| `FUN_00439C90` | announces a kill with a random line (`FUN_00457930`): a self-kill line, a death line when the world killed the player, otherwise a kill line |

## Kills and scores

The score table holds six bytes per slot (`004FE7E0`); `004DC9C0` holds the
frag limit (low word) and the winner (high word).

| Routine | Does |
| --- | --- |
| `FUN_00439DB0` | reports a kill: keeps the killer for 750 ms (`004DC9C8`, `004DC9C4`), so a death soon after still credits that player, then scores it on the host or sends it (message `0x11`) |
| `FUN_00439A70` | scores a kill: a self-kill costs a point; otherwise the killer gains one, two when the victim was ahead, and the victim loses as many when it has more; reaching the frag limit names the winner |
| `FUN_00439990` | the host's kill bookkeeping, with per-slot counters |
| `FUN_00439A40`, `FUN_00439940` | unpack a kill report; unpack a peer's score word and score its change on the host |
| `FUN_00439B40`, `FUN_004398E0` | pack the frag limit, winner and score table; unpack them, calling `FUN_00456750` when the winner changes |
| `FUN_00439BC0`, `FUN_00439BA0`, `FUN_00439700` | pack the kill counter `004DC9A4`, a word and a dword |
| `FUN_00439DF0` | packs a record: a header word from its kind byte and three data bytes, then the optional words its flag bits 0 and 1 select |
| `FUN_00439E70` | calls `FUN_00439E90` with a zero third argument |
| `FUN_00438F60` | reports a game event of kind 0, 1 or 2 with two words: the host sends message 4 with their high bytes, a client message 5 with the kind folded into a third byte |
