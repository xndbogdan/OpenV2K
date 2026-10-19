# Platform runtime

This note owns V2000's FGDK platform layer, the code between the game and
Windows: module start-up and shutdown, cleanup scopes, the message pump and
main loop, multimedia timers, the banner movie player, error reports, CD music
and sound voices, the DirectDraw device and its screens, texture pages and
sprite blits, the joystick and mouse readers, loaded tables, and the
applications and modes the game's screens run in. The port replaces this layer
with SDL2 and its own renderers; the note records what retail does so
behaviour that depends on it (timer cadence, movie end, error paths) stays
explainable.

## Modules

FGDK modules are reference counted. A module descriptor is a counter followed
by a pointer to a table of (open, close) function pairs, terminated by a null
open: the module's own pair first, then one pair for each module it depends
on.

- `FUN_004575C0` opens a module. The first open runs every dependency's open
  in table order and then the module's own. When one fails, the dependencies
  already opened are closed in reverse order, the count is restored and the
  error is returned.
- `FUN_00457630` closes a module. The last close runs the module's own close,
  then the dependencies' closes in reverse order.
- `FUN_004B4F40` is an empty routine both call around a first open and a last
  close.

Every module exposes a pair of two-instruction routines that open and close
its descriptor. The game's top module is `004D0150`, opened by `FUN_0044E440`
and closed by `FUN_0044E450`; the platform modules' pairs are
`FUN_00494030`/`FUN_00494040` (`004D5E08`), `FUN_00494580`/`FUN_00494590`
(`004D5E10`), `FUN_00494840`/`FUN_00494850` (`004D5E18`),
`FUN_00494C70`/`FUN_00494C80` (`004D5E20`), `FUN_00494F30`/`FUN_00494F40`
(`004D5E28`), `FUN_00495460`/`FUN_00495470` (`004D5E98`),
`FUN_00495FB0`/`FUN_00495FC0` (`004D5EE0`), `FUN_004962C0`/`FUN_004962D0`
(`004D5EE8`), `FUN_00497920`/`FUN_00497930` (`004D5F80`),
`FUN_004AB390`/`FUN_004AB3A0` (`004D65C0`), `FUN_004AB450`/`FUN_004AB460`
(`004D65C8`), `FUN_004AB590`/`FUN_004AB5A0` (`004D65D0`),
`FUN_004AB770`/`FUN_004AB780` (`004D65D8`), `FUN_004AB900`/`FUN_004AB910`
(`004D65E0`), `FUN_004AB9A0`/`FUN_004AB9B0` (`004D65E8`),
`FUN_004ABB60`/`FUN_004ABB70` (`004D65F0`), `FUN_004ABF20`/`FUN_004ABF30`
(`004D65F8`), `FUN_004ABFD0`/`FUN_004ABFE0` (`004D6600`),
`FUN_004ABFF0`/`FUN_004AC000` (`004D6608`), `FUN_004AC410`/`FUN_004AC420`
(`004D6638`) and `FUN_004AC580`/`FUN_004AC590` (`004D6668`). `FUN_004959F0`
closes `004D5EB0` alone.

Modules `004D1490` (`FUN_00457290`/`FUN_004572A0`) and `004D14C8`
(`FUN_00457A70`/`FUN_00457A80`) have their own pairs.

The game's own modules pair the same way: `FUN_00401000`/`FUN_00401010`
(`004C7030`), `FUN_004012C0`/`FUN_004012D0` (`004C7038`),
`FUN_00406180`/`FUN_00406190` (`004C7090`), `FUN_00409A60`/`FUN_00409A70`
(`004C70B8`), `FUN_0040AB90`/`FUN_0040ABA0` (`004C89E0`),
`FUN_0040D300`/`FUN_0040D310` (`004C8A20`), `FUN_0040ECF0`/`FUN_0040ED00`
(`004C8A88`), `FUN_0040F9D0`/`FUN_0040F9E0` (`004C8A90`),
`FUN_00410070`/`FUN_00410080` (`004C8A98`), `FUN_00410460`/`FUN_00410470`
(`004C9298`), `FUN_00418A40`/`FUN_00418A50` (`004C93A8`),
`FUN_0041BBB0`/`FUN_0041BBC0` (`004C93C0`), `FUN_0041E4B0`/`FUN_0041E4C0`
(`004C93E8`), `FUN_004218F0`/`FUN_00421900` (`004C9450`),
`FUN_004261B0`/`FUN_004261C0` (`004C9808`), `FUN_00427010`/`FUN_00427020`
(`004CA3B8`), `FUN_00428A00`/`FUN_00428A10` (`004CA438`),
`FUN_00428AB0`/`FUN_00428AC0` (`004CA440`), `FUN_00428BF0`/`FUN_00428C00`
(`004CA5C8`), `FUN_00429200`/`FUN_00429210` (`004CA5D8`),
`FUN_00429220`/`FUN_00429230` (`004CA6A8`), `FUN_0042A920`/`FUN_0042A930`
(`004CA830`), `FUN_0042D860`/`FUN_0042D870` (`004CAA20`),
`FUN_0042EC80`/`FUN_0042EC90` (`004CAB68`), `FUN_0042F510`/`FUN_0042F520`
(`004CACA0`), `FUN_00433BB0`/`FUN_00433BC0` (`004CAD68`),
`FUN_00437E40`/`FUN_00437E50` (`004CAD78`), `FUN_00437FA0`/`FUN_00437FB0`
(`004CADE8`), `FUN_00438060`/`FUN_00438070` (`004CADF0`),
`FUN_004383B0`/`FUN_004383C0` (`004CADF8`), `FUN_004389E0`/`FUN_004389F0`
(`004CAFC0`), `FUN_0043A0A0`/`FUN_0043A0B0` (`004CB048`),
`FUN_0043B1A0`/`FUN_0043B1B0` (`004CB4E8`), `FUN_0043D2E0`/`FUN_0043D2F0`
(`004CD710`), `FUN_00442EB0`/`FUN_00442EC0` (`004CDB50`),
`FUN_00448930`/`FUN_00448940` (`004CE008`), `FUN_0044A5A0`/`FUN_0044A5B0`
(`004CE098`), `FUN_0044C770`/`FUN_0044C780` (`004D0120`),
`FUN_0044E750`/`FUN_0044E760` (`004D04C0`), `FUN_0044EB20`/`FUN_0044EB30`
(`004D04D8`), `FUN_0044F5D0`/`FUN_0044F5E0` (`004D0CC0`),
`FUN_00457FA0`/`FUN_00457FB0` (`004D3CD0`), `FUN_00468CE0`/`FUN_00468CF0`
(`004D48E0`), `FUN_00468D30`/`FUN_00468D40` (`004D48F0`),
`FUN_0046BD60`/`FUN_0046BD70` (`004D4A18`), `FUN_00470990`/`FUN_004709A0`
(`004D4B48`), `FUN_00470A50`/`FUN_00470A60` (`004D4B50`),
`FUN_00471010`/`FUN_00471020` (`004D4B58`), `FUN_00471260`/`FUN_00471270`
(`004D4B60`), `FUN_00472640`/`FUN_00472650` (`004D4B90`),
`FUN_0047A700`/`FUN_0047A710` (`004D5C68`) and `FUN_00493A20`/`FUN_00493A30`
(`004D5D48`).

A module's own pair, first in its table, is `FUN_00438050`, which returns 0,
and `FUN_0042E8E0`, which does nothing, for most game modules. These have
their own:
`004C9298` (`FUN_00410400`/`FUN_00410430`), `004CA3B8`
(`FUN_00426F90`/`FUN_00426FF0`), `004CA440` (`FUN_00428AA0`, no-op close),
`004CA5C8` (`FUN_00428B40`/`FUN_00428BC0`), `004CA830`
(`FUN_0042A890`/`FUN_0042A8C0`), `004CACA0` (`FUN_0042F4F0`, no-op close),
`004CAD68` (`FUN_00433AD0`/`FUN_00433B40`), `004CADF8` (`FUN_004383A0`, no-op
close), `004CAFC0` (`FUN_00438880`/`FUN_00438920`), `004CB048`
(`FUN_0043A060`/`FUN_0043A090`), `004CD710` (`FUN_0043D290`, no-op close),
`004CDB50` (`FUN_00442E60`/`FUN_00442E90`), `004CE008`
(`FUN_00448920`/`FUN_00448910`), `004CE098` (`FUN_0044A500`/`FUN_0044A580`),
`004D0120` (`FUN_0044C710`/`FUN_0044C740`), `004D0150`
(`FUN_0044E1F0`/`FUN_0044E3D0`), `004D04D8` (`FUN_0044EAE0`, no-op close),
`004D0CC0` (`FUN_0044F540`/`FUN_0044F5B0`), `004D48F0` (`FUN_00468D20`, no-op
close), `004D4A18` (`FUN_0046BC60`/`FUN_0046BD00`), `004D4B58`
(`FUN_00470FD0`, no-op close), `004D4B60` (`FUN_00471200`, no-op close),
`004D4B90` (`FUN_004725F0`/`FUN_00472620`), `004D5D48`
(`FUN_004937C0`/`FUN_004939A0`), `004D5E08` (`FUN_00493FE0`/`FUN_00494010`),
`004D5E20` (`FUN_00494C10`/`FUN_00494C40`), `004D5E98`
(`FUN_00495360`/`FUN_004953A0`), `004D5EB0` (`FUN_004959C0`, no-op close),
`004D5EE0` (`FUN_00495F30`/`FUN_00495FA0`), `004D5EE8`
(`FUN_00496180`/`FUN_00496190`), `004D5F80` (`FUN_00497570`/`FUN_00497860`),
`004D65F0` (`FUN_004ABB30`/`FUN_004ABB40`), `004D6638`
(`FUN_004AC110`/`FUN_004AC3D0`), `004D6668` (`FUN_004AC4F0`/`FUN_004AC560`)
and `004D66E0` (`FUN_004292A0`, no-op close).

The 154 descriptor routines above (each `push descriptor; call open or close`)
are glue with no behaviour of their own, so the progress report omits them.
The modules' own pairs, which do the work, stay counted.

## Cleanup scopes

The error scopes (`FUN_00471030` and its siblings) keep a downward-growing
stack at `004FEE90` of typed records: a scope frame, a scope marker, a
registered cleanup (function and argument) and a cancelled slot. A failure
unwinds to the innermost frame and runs the cleanups registered above it.

- `FUN_00471050` registers a cleanup in the current scope.
- `FUN_004710B0` cancels a registered cleanup, matched by function and
  argument, so a later unwind skips it.

## Message pump and main loop

- `FUN_00495E20` is the run loop. It parses the command line
  (`FUN_00495CC0`), creates the window (`FUN_00495BC0`), opens the platform
  module `004D5EB0` (`FUN_004959E0`) and the game's top module, pumps until
  the running flag `004D5EA0` clears, then closes the game's module and the
  platform module (`FUN_004959F0`). A returned error object is freed before
  it exits.
- `FUN_00495CC0` keeps the current directory, without a trailing backslash,
  and reads the FGDK options `systemmemory` and `emulation`; any other word
  is an error.
- `FUN_00495BC0` registers the `Frame` window class and creates, shows and
  focuses the window.
- `FUN_00495DA0` is one pump: when a message waits it gets, translates and
  dispatches it (a `WM_QUIT` ends the loop). With the loop still running it
  calls the game's frame entry `FUN_0044E460`, and after a completed frame
  `FUN_00495940(0, 1)` lets a pending activation change take effect.
- `FUN_00495940` tracks application activation: -1 deactivates, 1
  activates. A change applies only while the counter `004D5EA8` is zero, and
  then sets it to one, so the next change waits for a completed frame.
  Deactivation calls `FUN_0044E6A0`, which sets the suspended flag
  `004FE60C`, raises pause reason 4 (`FUN_0044F3E0`) and calls the game's
  deactivate hook; activation calls `FUN_0044E6E0`, which reverses them.
- `FUN_0044E460` runs the current game mode's frame and then the frame pump;
  a failure from either goes to `FUN_0044E4A0`.
- `FUN_00495EE0` is the device-change callback: an arrival (`0x8000`) or a
  removal (`0x8004`) of a volume flagged as media passes the drive's unit
  mask to `FUN_004AD130`, which clears or sets those drives in the removed-
  media mask `004FC660` and calls the registered media callback with it.
- `FUN_00495FA0` ends the multimedia timer resolution set at start-up.

## Multimedia timers

A timer is a 20-byte record: callback, its argument, the periodic flag, the
period in milliseconds and the live WinMM event id.

- `FUN_00496020` allocates one from a callback, argument, periodic flag and a
  period in microseconds (divided by 1000).
- `FUN_004960A0` starts it: an existing event is killed, then `timeSetEvent`
  arms it at the start-up resolution, periodic when the flag is set; a
  failed arm returns an error object.
- `FUN_00495FD0` is the WinMM callback: it clears the event id of a one-shot
  timer and calls the record's callback with its argument.
- `FUN_004960F0` stops a timer: it kills a live event and clears its id.
- `FUN_00496080` frees one, killing its event first.
- `FUN_00496110` reads a timer's period and `FUN_00496120` sets it.

The loading workers re-arm one-shot timers from their own callbacks and stop
by spinning until the timer thread acknowledges, so their cadence follows the
timer thread, not the frame.

`FUN_00457170` starts one such worker, a one-shot 50 ms timer whose callback
`FUN_00456FB0` measures the real time since its last visit (clamped to 7.8 to
125 ms), adds it in milliseconds to the session's `+270`, and advances the
record at `004F71B0` (`FUN_0042AF00`, `FUN_0042B1E0`). `FUN_004571B0` stops
it, spinning until the callback acknowledges and then freeing the timer.

## Banner movie

The frontend's banner AVIs (see [MENU_SYSTEM.md](MENU_SYSTEM.md)) play through
a DirectShow multimedia stream drawn straight into the DirectDraw screen.
`004FBE5C` holds the movie being shown.

- `FUN_00494F50` opens a movie: it converts the file name to wide characters,
  creates the multimedia stream, initialises it for reading, adds the
  primary video and audio streams, opens the file and keeps the video
  stream's DirectDraw interface. The record's destructor is registered as a
  cleanup.
- `FUN_00495280` starts showing one: it creates a sample on the current
  screen surface, makes the movie current, runs it, presents its first frame
  and registers the stop as a cleanup.
- `FUN_004950A0` runs the stream unless it has ended (state 2) or is paused
  (state 3); `FUN_00495100` resumes a paused stream.
- `FUN_00495160` stops the stream and seeks back to its start.
- `FUN_00495220` advances the current movie each frame: it updates the
  sample, marks the movie ended when the stream reports its end (`0x40003`)
  and presents the screen.
- `FUN_00495320` stops showing the current movie and releases its sample;
  `FUN_004951D0` destroys a movie record and its three interfaces.

## Errors and message boxes

FGDK errors are objects: a table, a code, the failing result, the source file
and the line. Each subsystem builds its own from a failed `HRESULT` (high bit
set); a success returns no object.

| Routine | Does |
| --- | --- |
| `FUN_00494E00` | the movie module's (code `0x3D04`) |
| `FUN_00496ED0` | DirectDraw's (code `0xA04` for `DDERR_SURFACELOST`, otherwise `0xA00`) |
| `FUN_00494EA0` | formats a movie error's text: DirectShow's own error string, "Movie Error: ... from ... line ..." |
| `FUN_00496E10` | shows a message box on the main window: FGDK flags `0x100`, `0x80` and `0x200` choose the stop, exclamation or question icon; `1` with `2` gives OK/Cancel, and without `1`, `0x1C` gives Abort/Retry/Ignore and `0x60` Yes/No; the pressed button maps back to an FGDK result |

The census's two-byte `FUN_00496EA6` is no routine: it is the `mov edi, edi`
that aligns `FUN_00496E10`'s button jump table at `00496EA8`.

Error objects are a vtable (free, text) and a code. `004C5068` is the static
out-of-memory error (code `0x100`).

| Routine | Does |
| --- | --- |
| `FUN_00457490`, `FUN_00457520` | create an error object with a code (vtable `004C5078`), or with a code and a 256-character message (`004C5080`) |
| `FUN_00457430`, `FUN_004574C0` | their text methods: "Status 0x%08x", and "Status 0x%08x (%s)" with the message |
| `FUN_00457480` | the first vtable's free |
| `FUN_00457580`, `FUN_004573F0` | ask an error object for its text; format and report one (`FUN_0044E5A0`) |
| `FUN_004572E0`, `FUN_00457340` | allocate a block in an error scope, raising the out-of-memory error on failure and registering the block's free as a cleanup; free a block and cancel that cleanup |

## CD music

Music plays from the CD through MCI. A track record holds the track number;
`004FBF90` is the one playing. MCI failures become error code `0x2800` with
MCI's text ("Music: MCI error <...> encountered").

| Routine | Does |
| --- | --- |
| `FUN_00496130` | a music error's text: "Music: unknown MCI error number 0x%x encountered" with the error's number |
| `FUN_004962E0` | allocates an 8-byte track record from its description |
| `FUN_00496340` | plays a track (`MCI_PLAY` from that track to the next) after stopping any other, and makes it current |
| `FUN_00496AD0` | stops a track (`MCI_STOP`) and clears the current one |
| `FUN_00496310` | stops and frees a track record |
| `FUN_00496190` | the music shutdown: stops the current track, frees the track-length list `004FBFA8` and closes the device |
| `FUN_00496DD0` | returns the disc's track count `004FBFA0`, which `FUN_00496BF0` reads with each track's length and the total `004FBFA4` (the disc check built on them is in [MENU_SYSTEM.md](MENU_SYSTEM.md)) |

## Sound voices

A voice is a DirectSound buffer with a counter that plays it while negative.
`004FBE60` lists them.

| Routine | Does |
| --- | --- |
| `FUN_00495360` | starts the voice list and keeps `ln 65536` for the volume scale |
| `FUN_004953B0` | retires voices: with 0, those whose buffer has stopped (it asks the buffer's status every frame); with 1, every voice. `FUN_004953A0` retires them all |
| `FUN_00495740` | sets a voice's counter: going below zero restores a lost buffer and plays it (looping when the voice's `+30` flag is set); returning to zero or above stops it |
| `FUN_004957A0` | counts a voice down by one, so a voice at zero starts playing |
| `FUN_004956D7` | the branch of `FUN_004956B0`'s volume scale (`volume * factor >> 16`) for a volume of `0x10000` or more: it splits the volume into its high word and only the low byte of its fraction, as [MENU_SYSTEM.md](MENU_SYSTEM.md) notes |
| `FUN_00495700` | deletes a voice: its counter to zero (stopping it), then releases the buffer, unlinks and frees it |

Retiring by the buffer's status ties voice lifetime to real playback time, not
the game clock.

## Handler lists

Module `004D5E20` keeps a list of 44-byte handler records at `004FBE50`
(`FUN_00494C10` starts it, `FUN_00494C40` frees every record). With the module
open, `FUN_00494C90` appends a record copied from its description (a missing
callback defaults to `FUN_00438050`), `FUN_00494D30` calls a visitor on every
record until one returns an error, `FUN_00494D80` calls a record's callback
with its argument, and `FUN_00494DB0` copies a record's six-word description
out. `FUN_00494D10` unlinks and frees one.

## The DirectDraw device

The device record `004FBFC0` holds DirectDraw, `IDirectDraw2`, `IDirect3D2`,
the driver's capabilities and the chosen Direct3D device. Errors carry the
source `C:\Coding\FGDK\Code\Windows\DDCaps...` and a line.

| Routine | Does |
| --- | --- |
| `FUN_00497570` | creates the device for the primary driver and starts its three lists (texture records `004FC230`, palettes `004FC240`, pixel-format listeners `004FEE80`) |
| `FUN_004975E0` | `DirectDrawCreate`, then `IDirectDraw2` and `IDirect3D2`, the driver's capabilities, and the Direct3D device enumeration (`FUN_00497770`) |
| `FUN_00497770` | the enumeration callback: keeps the first device whose capabilities fit (hardware preferred over an emulated one already kept) |
| `FUN_00497870` | releases the device: the normal cooperative level, then `IDirect3D2`, `IDirectDraw2` and DirectDraw |
| `FUN_004A95C0` | a driver enumeration callback: builds a device for each driver, rates it (`FUN_004A93D0`) and keeps the best |
| `FUN_004A93D0` | rates a device for a requested mode: one lacking a requested 3D device, or (in a window) whose display depth differs, rates zero |
| `FUN_004A9570` | the display-mode enumeration callback: flags the mode matching the requested width, height and depth |
| `FUN_004979B0` | sets the device flags from the command line's `systemmemory` and `emulation` (`004FC2F0`, `004FC2F4`); `FUN_004959C0` applies them |
| `FUN_00496F70` | handles a lost surface: restores it, and after a restore re-applies the window placement |
| `FUN_00497000` | turns a DirectDraw or Direct3D result into its text ("Generic Error" ...) |

## Screens

A screen owns one to three buffers (`+2C`, `0x1AC` bytes each), the current
back buffer index `+14` and the full-screen flag `+9`.

| Routine | Does |
| --- | --- |
| `FUN_004945A0` | creates a display device record (`0x1C` bytes) from a five-word mode description, with no screen yet |
| `FUN_004945E0` | opens a screen for a mode description, with its 3D renderer (`FUN_00480D10`); a failure releases it |
| `FUN_004946B0`, `FUN_004946F0` | close a device's screen; close and free the device record |
| `FUN_00494800` | sets the screen window's title (its parent's when it has one) |
| `FUN_004A9980` | presents: unlock and end the scene, then flip the chain (full screen, cycling the back buffer) or blit the back buffer into the window's client area; counts frames in `004FC2DC` |
| `FUN_004947A0`, `FUN_004947E0` | present, then lock the next back buffer (a lost surface is not an error); present only |
| `FUN_004A9C20`, `FUN_004A9C90` | lock the current buffer, retrying while it is still drawing; unlock it |
| `FUN_004A9DC0` | blits a surface onto the current buffer (`BltFast`, waiting, colour-keyed when asked), restoring a lost surface once |
| `FUN_004A9BD0` | restores every buffer of a screen |
| `FUN_004AB050`, `FUN_004AB110` | begin a 3D scene on the current buffer (creating the 3D device first when needed); end it |
| `FUN_004A9790` | releases a buffer: unlocks it, frees the texture pages, releases its attached surface and itself |
| `FUN_004A9670` | closes a screen: a last present, its buffers, and, when the window was minimised, a fresh DirectDraw device |
| `FUN_00497940`, `FUN_00497990` | record an enumerated buffer surface; clear a buffer record's lock and attachment state |

## Texture pages and sprite blits

Sprites drawn by Direct3D live in texture pages: a page's surface, its 256
palette entries with use counts, a remap from sprite indices to page entries
(`+828`) and its free area (`+92C`).

| Routine | Does |
| --- | --- |
| `FUN_004AA0E0`, `FUN_004AA140` | allocate a texture record on list `004FC230`; create a texture surface (in system memory under `systemmemory`) |
| `FUN_004A9EB0` | creates a texture from pixels: the record, its surface, then a locked copy |
| `FUN_004A7D30` | places a sprite in a page: a slot record on the palette list, space from `FUN_004A7F10`, the pixels through `FUN_004A87A0`; it takes the slot's area off the page and decays its use weight |
| `FUN_004A7F10`, `FUN_004AB2E0` | find a page with room for a slot, the page with the most free area first, through `FUN_004A8060` |
| `FUN_004A8060` | packs a slot's rectangle (at most 254 by 254 plus its border) and its palette entries into a page |
| `FUN_004AA8E0` | creates a texture page: a 256-by-256 surface in the format the enumeration callback `FUN_004AAF80` picks, with its palette |
| `FUN_004AA390` | creates a buffer's Direct3D device and viewport |
| `FUN_00497A80` | frees a slot: unlinks it, gives its area back to the page and drops its palette entries' use counts |
| `FUN_004A9860` | frees every page, each slot first, then the page's surfaces and palette |
| `FUN_004AB170` | frees every slot and resets every palette's page entries (`0xFFFF`) |
| `FUN_004A8700` | compacts a page's palette: unused entries return to the free list and the rest are renumbered |
| `FUN_004A87A0` | uploads a slot's sprites into its locked page: a palettized page through `FUN_004A7C20`, any other format through the converter table `004D5F88` gives for the sprite's flags, the page's depth (8, 16 or 32 bits) and the screen's capability bit 8; then it unlocks the page and loads it into the device, retrying while the surface is busy |
| `FUN_004A7C20` | the 8-bit blit: page indices through the slot's remap, with the sprite's edge pixels repeated as a one-pixel border |
| `FUN_004AA1D0`, `FUN_004AA270` | register a pixel-format listener, called at once with the current channel shifts (`004FC2E4..004FC2EC`) and again on a mode change; unregister one |
| `FUN_004AAF80` | the texture-format enumeration callback |
| `FUN_004A8D50` | sets a display mode; a second full-screen request is an error |

The 48 converters of table `004D5F88` each read 8-bit sprite pixels through a
16-bit colour table and pack them into the destination's channel masks, with
shifts from the masks' highest bits (`FUN_004576E0`), one per combination of
sprite flags, destination depth and capability, in table order: `FUN_00497B30`, `FUN_00498010`, `FUN_00498580`, `FUN_00498A70`, `FUN_00499020`, `FUN_00499510`, `FUN_00499AC0`, `FUN_00499FD0`, `FUN_0049A5A0`, `FUN_0049AA90`, `FUN_0049B020`, `FUN_0049B500`, `FUN_0049BAC0`, `FUN_0049BF50`, `FUN_0049C4A0`, `FUN_0049C950`, `FUN_0049CEB0`, `FUN_0049D3A0`, `FUN_0049D920`, `FUN_0049DE20`, `FUN_0049E390`, `FUN_0049E850`, `FUN_0049EDB0`, `FUN_0049F290`, `FUN_0049F860`, `FUN_0049FDA0`, `FUN_004A0370`, `FUN_004A08B0`, `FUN_004A0EA0`, `FUN_004A13F0`, `FUN_004A19F0`, `FUN_004A1F40`, `FUN_004A2560`, `FUN_004A2A80`, `FUN_004A3060`, `FUN_004A3580`, `FUN_004A3B80`, `FUN_004A4090`, `FUN_004A4650`, `FUN_004A4B70`, `FUN_004A5120`, `FUN_004A5630`, `FUN_004A5BC0`, `FUN_004A60F0`, `FUN_004A66E0`, `FUN_004A6BD0`, `FUN_004A7170`, `FUN_004A7680`.

## What the port replaces

The port's SDL window, renderers, audio device and launcher stand where these
did, so the progress report omits them ([status.csv](../progress/status.csv))
after a check that none decides anything a player sees or hears beyond its
own platform:

- **The screen device**, replaced by the port's SDL platform: the DirectDraw
  device (`FUN_00497570`, `FUN_00497860`, `FUN_004975E0`, `FUN_00497770`,
  `FUN_00497870`, `FUN_004A95C0`, `FUN_004A93D0`, `FUN_004A9570`), its lost
  surfaces and result texts (`FUN_00496F70`, `FUN_00496ED0`, `FUN_00497000`),
  the screens and their buffers (`FUN_004945A0`, `FUN_004945E0`,
  `FUN_004946B0`, `FUN_004946F0`, `FUN_004A8D50`, `FUN_004A9980`,
  `FUN_004947A0`, `FUN_004947E0`, `FUN_004A9C20`, `FUN_004A9C90`,
  `FUN_004A9BD0`, `FUN_004AB110`, `FUN_004A9790`, `FUN_004A9670`,
  `FUN_00497940`, `FUN_00497990`), and the texture pages, which only
  Direct3D sprites fill (`FUN_004AB2E0`, `FUN_004A8060`, `FUN_00497A80`,
  `FUN_004979E0`, `FUN_00497A40`, `FUN_004A9860`, `FUN_004AB170`,
  `FUN_004A8700`, `FUN_004A87A0`, `FUN_004A7C20` and the 48 converters of
  table `004D5F88`). What the screen shows is ported from the software
  raster's fill slots ([SOFTWARE_RASTER.md](SOFTWARE_RASTER.md)); the code
  only the [Direct3D table](DIRECT3D_TABLE.md) reaches is omitted as dead.
- **The shell**, replaced by the port's SDL platform and launcher: MCI CD
  audio (`FUN_00496130`, `FUN_00496180`, `FUN_00496190`, `FUN_004961D0`,
  `FUN_004962E0`, `FUN_00496310`, `FUN_00496340`, `FUN_00496460`,
  `FUN_00496730`, `FUN_00496890`, `FUN_00496AD0`, `FUN_00496BF0`,
  `FUN_00496DD0`; the port plays the same tracks, and the game's own track
  choice, restart poll and disc check stay counted,
  [MENU_SYSTEM.md](MENU_SYSTEM.md#music--the-frontend-menu-is-silent)), the
  message box (`FUN_00496E10`), and the working directory and the
  `systemmemory`/`emulation` options (`FUN_00495CC0`, `FUN_004979B0`,
  `FUN_004959C0`; the launcher's settings instead).

The window, its title and pump, activation, the display-mode routines that
set the full-screen flag the pointer reads (`FUN_004A8C30`, `FUN_004A8D90`),
the pixel-format listeners and colour tables, the banner movie, sound voices,
timers, the joystick and the mouse stay counted.

## Joysticks and the mouse

| Routine | Does |
| --- | --- |
| `FUN_004AC010` | sets a joystick unit's consumer and argument (stopping its poll timer first) and, with a consumer, reads it at once through `joyGetPosEx`: buttons, then X, Y, Z, R, U and V, handed to the consumer with the buttons shifted to bit 1 |
| `FUN_004AC3D0` | releases every joystick unit: its timer and its handler record |
| `FUN_004AC4F0` | starts the mouse: its input thread with the packet routine `FUN_004AC430`, the device read, and a handler record (`004D6640`) |
| `FUN_004AC560` | stops the mouse: its handler record, then the thread |
| `FUN_004AC430` | a mouse packet: four buttons to bits 1..4 of `004FC608`, the X, Y and Z movement added to `004FC628..004FC630`, then the consumer |
| `FUN_004AC4C0` | sets the mouse consumer and argument and calls it at once with the current state |

## Rotations

`FUN_00457DD0` and `FUN_00457A90` rotate a Q31 basis by an angle word. Both
take sine and cosine from the quarter-sine table at `004D14D0` (index
`angle & 0x3FFC`, mirrored for the second and fourth quarters and negated
for the second half) as repeated-word Q31 values.

## Loaded tables

Data loaded from the overlays carries counted arrays; each kind has a loader
reading the count and the words from the stream (`FUN_00468E70`) and a matching
free.

| Routine | Does |
| --- | --- |
| `FUN_004AB790`, `FUN_004AB8C0` | load colour tables and register them as a pixel-format listener (`FUN_004AB830`/`FUN_004AB890`), so they follow the screen's channel layout; unregister and free them |
| `FUN_004AB830`, `FUN_004AB890` | the listener's two directions: pack a table from 5:5:5 into the screen's channel shifts (`FUN_004AA2E0`), or unpack it back (`FUN_004AA330`) |
| `FUN_004AB860` | applies such a converter to a counted table |
| `FUN_004AA2E0`, `FUN_004AA330` | pack and unpack 16-bit colours between 5:5:5 and the given channel shifts |
| `FUN_004AB9C0`, `FUN_004ABA40` | load screen descriptions (20 bytes each, given the main window); free them |
| `FUN_004ABF40` | loads a table of strings |
| `FUN_004AB740` | frees a chain of loaded blocks |
| `FUN_004ABB30`, `FUN_004ABB40` | clear and free the shared load buffer `004FC2FC` |

## Applications and modes

The game's screens are modes, built from the fourteen-word descriptors that
[MENU_SYSTEM.md](MENU_SYSTEM.md#layer-1--mode-state-machine-fgdk-task-descriptors)
lays out. Module `004D5E08` keeps the application records on `004FBE40`; a
mode joins the mode list (`+2C`) of the application its descriptor's first
word names. A mode slot (`0x54` bytes, MENU_SYSTEM's active mode object) holds
the live mode (`+3C`), a staged descriptor and the display device (`+40`).
`FUN_004942C0` stages a descriptor. The slot's frame `FUN_004942E0` does
nothing while the live mode's application has its `+40` word clear; otherwise
it swaps a staged descriptor in (the new mode keeps the old one's `+44` word)
and runs the mode's chains
([LOADING_TRANSITIONS.md](LOADING_TRANSITIONS.md#active-world-frame-and-presentation-order));
the presentation chain receives the slot's display device. `004FBE38` is the
current mode.

| Routine | Does |
| --- | --- |
| `FUN_00494180` | creates an application (`0x48` bytes) from a seven-word description, with an empty mode list and its own microsecond clock (`FUN_00495FF0`), then calls the description's create callback |
| `FUN_00493FE0`, `FUN_00494010` | start the application list; free every application |
| `FUN_00494230` | destroys an application: unlinks it, calls the description's destroy callback (word 2) and frees it |
| `FUN_0044F960` | creates the world session: a zeroed `0x30C`-byte record (`004F72C8`) and its application, from the description at `004D0B10` (create `FUN_0044FA30`, destroy `FUN_0044FC50`) |
| `FUN_0044FC50` | the world application's destroy callback: frees the session's blocks and its four `0x28`-byte records |
| `FUN_00494050` | sets an application's `+28` word; the module must be open |
| `FUN_00494260`, `FUN_00494370` | create a mode slot from a five-word description (its display device first) and call its create callback; destroy one: its destroy callback, its live mode, then the slot |
| `FUN_00494520` | gives a slot its display device: the display change `FUN_0044E0E0` and the frontend's movie swap devices this way |
| `FUN_004943B0` | creates a mode (`0x48` bytes) from a descriptor: its create callback (a failure frees it), then it joins its application's list and becomes current |
| `FUN_00494500` | makes a mode current and evaluates its binding set (descriptor word 3, `FUN_00471DE0`) |
| `FUN_00494530` | replaces a mode's binding set |
| `FUN_00494460` | destroys a mode: its exit handler (word 10), then unlinks and frees it |
| `FUN_00494540` | dispatches an event code and argument to an application's modes in order, through each one's handler (word 12), until one handles it |
| `FUN_00494490`, `FUN_004944D0` | create a `0x28`-byte record from a ten-word description, calling its first word; destroy one, calling its fifth. The world application's create callback makes four, from the descriptions at `004D0878`, `004D08A0`, `004D08F0` and `004D08C8` |
| `FUN_00494780`, `FUN_004948B0` | empty callbacks returning success |
