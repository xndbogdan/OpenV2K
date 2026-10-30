# Small functions

Short retail routines that no larger note owns, one line each: accessors,
global setters and fixed-argument forwarders. Larger subsystems name their own
helpers.

## Census artifacts

These census entries are not functions; the progress report omits them.

- `0x0042CFEA`, `0x0043C1C6`, `0x0043E162`, `0x00441E7A`, `0x0044256E`,
  `0x0044EA82`, `0x004512AE`, `0x00471132` and `0x00496EA6` are the two-byte
  `mov edi, edi` that follows a function's `ret` and precedes its switch jump
  table.
- `0x004167BA` and `0x0043B3D5` are runs of `nop` between functions.
- These are parts of the function before them, which runs into them:
  `0x00410440` (`FUN_00410430`), `0x0042776E` and `0x004277C0`
  (`FUN_00427760`), `0x0043F230` (`FUN_0043F180`), `0x00456FE0`
  (`FUN_00456FB0`), `0x00493654` (`FUN_00493640`, which loads an overlay),
  `0x004956D7` (`FUN_004956B0`), `0x0046C5F1` (the exit of `FUN_0046C570`),
  and `0x0048D159` and `0x0048FADE`, the ends of the Direct3D handlers
  `FUN_0048CB90` and `FUN_0048F160`.

Six of them (`0x004167BA`, `0x0042776E`, `0x0043B3D5`, `0x0046C5F1`,
`0x0048D159` and `0x0048FADE`) are values of the span-reciprocal table at
`0x004D4B98` that the census took for code pointers.

## Field accessors

| Function | Does |
|---|---|
| `FUN_0040D150` | returns the dword at `+0x18` |
| `FUN_00418950` | returns the dword at `+0x04` |
| `FUN_0042E8D0` | returns the word at `+0xB0` |
| `FUN_00447C60` | returns the dword at `+0x190` |
| `FUN_00456760` / `FUN_00456770` | read and write the dword at `+0x2A8` |
| `FUN_00401260` | stores its second argument at `+0x28` of the record its first points to |
| `FUN_0041B9B0` | stores a byte at `+0x42` |
| `FUN_00424F10` | stores a dword at `+0x24` |
| `FUN_00437FF0` | clears the dword at `+0x0C` |
| `FUN_004713A0` | clears `+0x14` and `+0x18`, returns 0 |
| `FUN_00448760` | sets `+0x70` to 1 when it is non-zero |
| `FUN_00416290` | sets dwords 0 and 3 of a record to 1 |
| `FUN_0042A560` | zeroes three dwords |
| `FUN_0044F5F0` | returns the dword its argument points to |
| `FUN_00443B20` | writes its second argument through the first, returns 0 |
| `FUN_0042F4E0` | writes 0 through its argument, returns 0 |
| `FUN_00472550` | writes its second argument's `+0x08` through the first, returns 0 |
| `FUN_004A9E90` | copies a pair of dwords out through two pointers, returns 0 |
| `FUN_00428900` | writes its third argument through the first, returns 4 |
| `FUN_00468D00` | counts the nodes of a list |
| `FUN_00469630` / `FUN_00469640` | return the sum and the product of their second and third arguments |

## Globals

| Function | Does |
|---|---|
| `FUN_00414920` | returns `0x004DB090`, the live entity list |
| `FUN_004493C0` | returns the pointer at `0x004DEEC0` |
| `FUN_00456C10` | returns the session (`0x004F72C8`) plus `0x14` |
| `FUN_00456C90` | whether session `+0x264` is non-zero |
| `FUN_00456F70` | session `+0x27C`, or 0 without a session |
| `FUN_0044E720` / `FUN_0044E730` | store their argument at `0x004F718C` and `0x004F7190` |
| `FUN_0040F390` | clears `0x004DAF6C` and `0x004DAF40` |
| `FUN_0043C7D0` | clears `0x004DCED0`, `0x004DCEDC` and `0x004CB4E0` |
| `FUN_0043CB10` | whether `0x004DCEDC` is below 2 |
| `FUN_0043D270` | clears `0x004DCEAC` |
| `FUN_0046D520` | advances `0x004FB428` along the chain of offsets at `0x004F7424` |
| `FUN_0046D540` | clears `0x004FB428` |
| `FUN_00453FD0` | increments `0x004F72EC`, returns 0 |
| `FUN_0042CEB0` | sets `0x004DB218` to 1 when its argument is 0, else to 0 |
| `FUN_00449060` | returns 1 the first time (setting `0x004DE860`), 0 afterwards |
| `FUN_0042AA60` | copies into `0x004DB208` the first dword of the record that entry `0x20` of the `0x004FE63C` table points to, returns 0 |
| `FUN_004291D0` / `FUN_004291E0` / `FUN_004291F0` | return entry `n` of the tables `0x004FE62C`, `0x004FE63C` and `0x004FE640` point to |
| `FUN_00427030` | returns the 50-Hz world clock `0x004FED60` when its third argument is 0, else 0 |
| `FUN_0044E740` | returns the settings word at `+0x10` (the block `FUN_0043C7C0` returns) |
| `FUN_0042A850` | stores its argument as the HUD setting (`+0x38` of that block) |

## Forwarders

| Function | Does |
|---|---|
| `FUN_0040AC30` | frees its argument (`FUN_00457320`, which skips null) |
| `FUN_004103E0` | frees the pointer its first argument holds when its fifth argument is positive |
| `FUN_0040AC40` | calls `FUN_0040AC60` with the dword its second argument points to |
| `FUN_0040C730` / `FUN_0040C770` / `FUN_0040C790` / `FUN_0040C7B0` | install behaviour style 0, 2, 3 or 5 (`FUN_0040C6B0`) |
| `FUN_004090B0`, `FUN_0042A910` | flag an entity for the deferred sweep (`FUN_00410B70`, which sets `+08` bit `0x100000` once and counts it), return 0 |
| `FUN_0040D750` / `FUN_00417D90` | the same, returning nothing or 4 |
| `FUN_004054A0` | clears the word `FUN_0040A950` returns for index 1 |
| `FUN_00439F20` / `FUN_00439F40` / `FUN_00439F60` | call `FUN_00439E90` with 1, 2 or 3, as `FUN_00439E70` does with 0 |
| `FUN_00433960` / `FUN_004339E0` / `FUN_00433A00` | add a callback and argument to the lists at `0x004DC5F8`, `0x004DC628` and `0x004DC608` (`FUN_00433980`) |
| `FUN_00433A20` / `FUN_00433A90` / `FUN_00433AB0` | remove that pair from the same lists (`FUN_00433A40`) |
| `FUN_0042AA40` | resets the primitive queue (`FUN_00428E60`) of the record at `+0xD0` of its third argument |
| `FUN_0042AA80` / `FUN_0042AAB0` | call `FUN_00428DA0` and `FUN_00428E00`, which treat error `0x707` from `FUN_00494720` and `FUN_00494780` as success |
| `FUN_0044FA20` | runs the frame pump `FUN_00494070` on the device its argument holds |
| `FUN_00450E40` | runs Skip to World (`FUN_004512E0`) on the session when its argument is negative |
| `FUN_00450E60` | clears session byte `+0x28B` when its argument is negative |
| `FUN_004513E0` | calls `FUN_0044F600` with the record at `+0xD4` |
| `FUN_00451400` | runs `FUN_0043C7F0(1)` and `FUN_0043A9A0`, returns 0 |
| `FUN_004562A0` | when its second argument is 2, runs `FUN_004170F0`, which publishes the death of every live objective actor |
| `FUN_004564A0` | runs `FUN_0043A9F0` when its second argument is non-zero |
| `FUN_0043D0D0` | runs `FUN_0043D070` and clears `0x004DCEA8` |
| `FUN_0043D200` | runs `FUN_0043D1B0`, then `FUN_0043D070` |
| `FUN_0043D280` | runs `FUN_00439290` and sets `0x004DCEAC` to 2 |
| `FUN_00443B30` | calls `FUN_00443B50` with the controller's player handle (`+0x68`) |
| `FUN_0046A890` | forwards to `FUN_0046A950` |
| `FUN_0046D580` | clears the projected byte of `n` vertex-cache slots (`FUN_0046D550`) |
| `FUN_004713B0` / `FUN_00471590` | drop one reference through `FUN_004713D0`, whose last release runs `FUN_0042E8E0` (nothing) or `FUN_004715B0` |
| `FUN_00472660` / `FUN_00472670` | call the library routines `FUN_004ACAE0` and `FUN_004ACA90`, return 0 |
| `FUN_00438A00` | calls the library routine `FUN_004B3FF0` with its argument when `0x004CAFC8` is set |
| `FUN_00493640` | formats `overlay\%dx%dxx.ovl` and opens it (`FUN_00493680`) |
| `FUN_00493780` | closes a file (`FUN_00493790`) |
| `FUN_00438000` | writes the dword at `+0x0C` as one framed save record (`FUN_00448990`) |

## Entities and tasks

| Function | Does |
|---|---|
| `FUN_00402190` / `FUN_00402780` / `FUN_00403840` / `FUN_00404B40` | install a task through `FUN_00405F80` with callback `FUN_004021B0`, `FUN_004027A0`, `FUN_00403860` or `FUN_00404B60` |
| `FUN_00402800` | the same with no callback |
| `FUN_0040B8E0` / `FUN_0040B9B0` | clear task slots 2 and 1 (`FUN_0040A7A0`), then call `FUN_004066B0` or `FUN_00404B40` |
| `FUN_004086A0` | frees the record at `+0x28` of the one its argument points to, and clears that field |
| `FUN_0040AEE0` | calls `FUN_00416360` with its third argument's `+0x08`, then `FUN_0040AF50` |
| `FUN_0040E9E0` | calls the entity's `+0xC4` callback when one is set |
| `FUN_00410480` | the `+0x08` word of an entity's type record, or 0 when the handle does not resolve |
| `FUN_00416430` | 1 when the handle does not resolve or `+08` bit `0x2000` is clear |
| `FUN_00416530` | 1 when `+08` bit `0x4000000` is clear |
| `FUN_00417F30` | sets `+08` bit 1 unless bit `0x80000000` is set, returns 4 |
| `FUN_00421C40` | flags the entity at `+0xF4` for the deferred sweep and frees the record |
| `FUN_004382B0` | frees an entity's `+0xCC` record |
| `FUN_0044CF00` | calls `FUN_0044CF30` with no second argument, then `FUN_00414AA0` on `+0x48` |
| `FUN_0044CFB0` | calls method `+0x10` of the object at `+0x44` when both exist |
| `FUN_00461B00` | lights normal `n` (`FUN_0046D3F0`) when its 8-byte record's byte 5 is set |
| `FUN_0046FB50` | runs `FUN_0046FB80` with a zero last argument, then marks the slot projected (`0x80`) |
| `FUN_004167C0` | returns a body's health as a fraction of its type's initial health (`+14`), scaled to `0xFFFF`, for health under `0x10000` |
| `FUN_00418F30` | the word `+B4` of the record that the Primary task's `+30` points to, for the kind-7 pack |

## Arithmetic and storage

| Function | Does |
|---|---|
| `FUN_0040F9F0` | adds its argument divided by 1000 to `0x004DAFC4`, returns 0 |
| `FUN_00415890` | `(x*a + y*b + z*c) >> 12` over a triple of signed words |
| `FUN_0041F190` | stores three words, each its argument shifted right by 19 |
| `FUN_004255C0` | `(a << 8)` divided by the word at `+0x1C + 4n` of its third argument, or `a` when that is 0 |
| `FUN_00469680` | calls the model context's callback (`+0x58`, with `+0x60`) at `a + b`, as opcode `0xAD` does |
| `FUN_00472520` | stores three dwords at `+0x4C + 12n` |
| `FUN_004208C0` | allocates and zeroes `0x4C` bytes |
| `FUN_00437FC0` | allocates 16 bytes and clears the first and fourth dwords |
| `FUN_0042E380` | allocates `0x1FC` bytes and initializes them with `FUN_0042E3B0` |
| `FUN_0041D340` | frees the pointer its argument holds, then the argument |
| `FUN_004389B0` | frees `n` pointers of an array |
| `FUN_00433B80` / `FUN_00436930` | unlink and free every node of a list |
| `FUN_00438020` | reads the dword at `+0x0C` back from a framed save record (`FUN_00448A60`) |
| `FUN_00456C50` / `FUN_00456C70` | write and read the session's `+0x2C4` record that way (`FUN_00438000`, `FUN_00438020`) |
| `FUN_00449030` | on first use (`FUN_00449060`), clears the first byte of each `0x28`-byte record from `0x004DE868` to `0x004DEAC0` |
| `FUN_00449120` | ensures that, then returns the first byte of record `n`, signed |
| `FUN_004725C0` | stores a callback and its argument at `0x004FBD70` and `0x004FBDB8`, and calls it once with `0x004FBD78` |
| `FUN_00471280` / `FUN_00471510` | add a record through `FUN_004712B0` to the lists at `0x004FB858` (`0x30` bytes, `FUN_004713A0`) and `0x004FB830` (`0x24` bytes, `FUN_00471540`) |
| `FUN_00493B70` | closes a handle (`FUN_00493AF0`) and clears it |
| `FUN_004577D0` | the integer square root of a signed 64-bit value (zero for values not above zero), bit by bit from the highest power of four not above it; `FUN_00457730` is the 32-bit one |
| `FUN_00418960` | the dword at `+8` of a table's 12-byte entry `i` (`[table] + 8 + 12i`), unchecked |
| `FUN_00471370` | finds the record of a list whose `+0x0C` and `+0x10` equal its two arguments, or none |

## Messages

These copy a fixed record into a caller's buffer and return its size, or -1
when the buffer is too small.

| Function | Record |
|---|---|
| `FUN_00417D70` | the dword at `+0x5C` (4 bytes) |
| `FUN_004182C0`, `FUN_00439FF0` | 12 bytes |
| `FUN_00439F80` | 20 bytes |
| `FUN_00438BB0` | a 12-byte stamp: `0x004DC9D0`, the counter `0x004DC9D4` (incremented first) and `0x004DC9D8` |
| `FUN_00418290` | passes a 12-byte record's fields to `FUN_004180F0`, returns 12 |

## Session and menus

| Function | Does |
|---|---|
| `FUN_0044FDF0` | frees a session, clearing `0x004F72C8` when it is the current one and closing its device (`FUN_00494230`) |
| `FUN_00456DD0` | 0 without a session or with session `+0x2BC` zero; 2 when it is -1; else 1 |
| `FUN_00456E20` | calls `FUN_00456E50` with `FUN_00456EF0`'s result |
| `FUN_00456F90` | copies the player's position through the session's controller (`+0x27C`, `FUN_00448720`) |
| `FUN_00451480` | runs `FUN_004383D0` and `FUN_0043C7F0`, returns 0 |
| `FUN_00451550` | clears `0x004F718C` and `0x004F7190`, then runs `FUN_00456A00` on its second argument's `+0xD4` |
| `FUN_00452770` | sets session `+0x34` to 1 in state 3, sets byte `+0x290` and runs `FUN_00456750` |
| `FUN_0044FDC0` | for message 6, clears byte `+0x289` of the session at `+0xD4` and routes the menu (`FUN_0042A940`) |
| `FUN_0044F420` | calls `FUN_0044F3E0` or `FUN_0044F400` by its second argument |
| `FUN_0042ED00` | the `+0x80` word of the current world's record (`FUN_00450C90`) |
| `FUN_004261D0` | clears `0x004DB0C4`; when selected, runs `FUN_0042A860(3)` and routes the menu |
| `FUN_004262C0` / `FUN_004262F0` | clear `0x004DB0C4`; when selected, run `FUN_0042A860(0)` and step `0x004DB0C0` down or up by `0x10000` |
| `FUN_0042BB60` / `FUN_0042BB90` / `FUN_0042BBC0` | set `0x004DB204` and `0x004DB21C` to 60000; when selected and `0x004FE60C` is 0, run `FUN_0043C1E0`, `FUN_0043C6C0` or `FUN_0043C720` |
| `FUN_004262A0` | returns 0 |
| `FUN_0042AA90` | drains the primitive queue (`FUN_00428E70`) of the record at `+0xD0` of its third argument |
| `FUN_0043DCB0` | runs `FUN_0043DDC0` when `FUN_0043DCD0` returns 0 |
| `FUN_00469740` | clears `+0x20` of its second argument, then calls `FUN_00469770` |
| `FUN_00469C00` | calls `FUN_00469C20` with a zero fourth argument |
| `FUN_0046AEF0` | calls `FUN_0046AF20` |
| `FUN_0042E8F0` | the level record's music track word (`+48`, from `004FE654` by the session's `+C4`), for `FUN_00456AF0`; error `4C0DC0` without a record |

## Larger helpers

| Function | Does |
|---|---|
| `FUN_0040AF10` / `FUN_0040C5E0` | clear task slots 1 and 2, then install through `FUN_00403650` or `FUN_004051B0` |
| `FUN_0040B1B0` | clears task slots 2 and 1, then calls `FUN_004076E0` |
| `FUN_0040B780` / `FUN_00426170` | clear task slot 2, install slot 1 through `FUN_00402190` or `FUN_00402050` and, when that succeeds, run `FUN_00402B10(entity, 0, 500)` |
| `FUN_004158C0` | scales a triple of signed words by a Q12 factor |
| `FUN_00417E70` | copies a 16-byte record into a caller's buffer (returns 16, or -1 when the buffer is too small) |
| `FUN_004183D0` | pops every entry of a callback stack (`FUN_00418590`), then frees its storage and the stack |
| `FUN_004198A0` | queues gameplay notification `0xD6` (`FUN_00456900`) and, when the entity's `+0x64` bit `0x4000` is set, publishes its death (`FUN_00410C10`) |
| `FUN_0042EC30` | maps the current world record's `+0x48` kind: 4 to 1, 5 to -1, anything else to 4 |
| `FUN_0042EEE0` | sets or clears bit `0x4` of the current world's control word (index `+0xC4`) |
| `FUN_0042F230` | for a world of kind 5, writes the terrain's water level and returns 1; else 0 |
| `FUN_004337A0` | calls every callback of a list with its argument and three more |
| `FUN_00437470` | unlinks and frees every node of the list at `0x004DC618` |
| `FUN_00438B60` | refreshes the 12-byte stamp: the dword at `0x004F73B4`, the counter incremented, and `0x004F7378`'s byte twice (the second only when `0x004F73B8` is set) |
| `FUN_0043B8C0` | the Sensitivity row: runs `FUN_0043B7C0`, then sets the analog gain from the new value (`FUN_00428EA0`) and raises any error that returns (`FUN_00471150`) |
| `FUN_0043CB50` | runs `FUN_0043B7C0`, then `FUN_00495910` with the nibble at `0x004CB3D8` repeated four times |
| `FUN_0043CF90` | at most every 50 world ticks, runs `FUN_0043D070` and walks with `FUN_0043CFD0` (`FUN_0046C9A0`) |
| `FUN_00447C20` | discards the controller's player (flag and sweep, `FUN_00414990`), stores the new type at `+0x190` and builds the player again (`FUN_004438A0`) |
| `FUN_00449080` | copies a save slot's name, or the default string at `0x004CE01C`, into a buffer, after probing the slot (`FUN_00448E70`; status 4 is occupied) |
| `FUN_004490D0` | an occupied slot's level name (`FUN_00456E50` of its `+0x24` word), else the empty string at `0x004DAFC8` |
| `FUN_0044A850` / `FUN_0044A890` | call `FUN_0044A5C0` with an x/z pair, its cell coordinates and the current terrain record |
| `FUN_0044AFB0` | stores four values at `0x004DF110`, `0x004DF114`, `0x004EF128` and `0x004EF12C`, runs `FUN_0044AB20(0)` and returns 0 |
| `FUN_0044C7D0` | unlinks a node, frees its `+0x20` buffer, runs `FUN_0044C810` on it and frees it |
| `FUN_0044CE70` | stores its argument at `0x004F716C`; the first non-zero value runs `FUN_0044C810` over the list at `0x004F7160` |
| `FUN_0044D910` | the word among `+0xA8..+0xAE` that belongs to the entity's active model slot |
| `FUN_0044E3F0` | closes a device record (`FUN_00494370`) and its `+0xD8` member, then frees it |
| `FUN_00456300` | when its second argument is 2, fills every world-control word of the session's controller (`FUN_0042EE30`, `FUN_0042EE50`) |
| `FUN_004694B0` | pops a ring buffer: returns `base + index * stride` and advances the read index, wrapping at the capacity; 0 when empty |
| `FUN_00469550` / `FUN_004695A0` | allocate a ring buffer of `count * size` bytes (an error for a null record, a count below 2 or a negative size) and free one |
| `FUN_004695E0` | drains the ring at `0x004F7328`, passing each entry of kind 0 or 1 to `FUN_004693A0` with that kind's queue (`0x004F7380 + 20 * kind`) |
| `FUN_0046FEA0` / `FUN_0046FEF0` | run `FUN_0046FB80`, then store the outcode of the vertex-type routine at `+0xC4` or `+0xC0` with bit `0x80` set |
| `FUN_00471710` | sets `+0x20` to 1 and, when `+0x14` and `+0x18` differ, calls `FUN_00471760` with an empty record |
| `FUN_004719F0` / `FUN_00471B80` | drop one reference from every entry of a table (`FUN_004713D0`, with `FUN_0042E8E0` or `FUN_004715B0`) |
| `FUN_00471BD0` | allocates an empty `0x18`-byte list header |
| `FUN_00471C80` / `FUN_00471CD0` | append a node holding their argument to the list at `+0x08` or `+0x14` |
| `FUN_004724E0` | unlinks a node, releases its `+0x08` handle (`FUN_00494D80`) and frees it |
| `FUN_0048AFF0` | sets the Direct3D texture handle (`SetRenderState(1, handle)`) when it differs from the one cached at `+0x1A4` |
| `FUN_0041BE30` | frees a list record: unlinks and frees every node of each of its `+2C` lists (`+18`, 20 bytes apart), then frees list `+8`, the array and the record |
| `FUN_004208F0` | releases a record's two sound handles (`+1C`, `+28`) through `FUN_0044CC90`, then frees it |
| `FUN_004288C0` | frees every node of the list at `004DB0D0` |
| `FUN_00468D50` | opens a file in mode 0, 1 or 2 (`"rb"`, `"wb"`, `"wb+"`); error `0x1001` with the name when it fails, `0x1000` for any other mode |
| `FUN_00468F80` | writes a buffer to a file (`fwrite`) and raises error `0x1003` with the C runtime's message for `errno` when fewer bytes were written |
| `FUN_0047C740` | fills one vertex of a software raster command (for the textured fill slots such as `FUN_0047C600`): four optional shade bytes as 16.16 values centred in their unit (`× 0x10000 + 0x8000`), the screen point (x << 16, y) and, when asked, two texture coordinates |

## Vectors, terrain and system

| Function | Does |
|---|---|
| `FUN_00415610` | normalizes `(x, y, z)` to Q12 words through the integer square root `FUN_00457730` and returns the length; a zero vector gives `(0x1000, 0, 0)` and 0 |
| `FUN_00416230` | the same, given the squared length |
| `FUN_00416E90` | widens a triple of words to Q31 (each shifted left by 19), saturating beyond ±`0xFFF` |
| `FUN_00416F20` | the Q12 cross product of a triple of words with `(x, y, z)` |
| `FUN_00433900` | writes a terrain cell's attribute byte and passes the cell and its terrain type's bit `0x08` to the callbacks at `0x004DC608` |
| `FUN_00437B00` | writes a 4-byte cell record (x, z, terrain type, attribute), returns 4 |
| `FUN_00437C50` | refreshes such a record's attribute when its kind byte is 4, then copies it into a caller's buffer (4, or -1 when the buffer is too small) |
| `FUN_0046D4B0` | carves a vertex block (20 bytes a slot plus 8 a normal) from the `0x4000`-byte arena at `0x004F7428`, recording the previous offset after it for `FUN_0046D520`; an error when the arena is full |
| `FUN_0048AF90` | fills a Direct3D transformed-and-lit vertex: the screen point, depth 0.5, reciprocal w 2.0, the colour, specular, u and v |
| `FUN_00471C10` | frees every node of both of an input set's lists, first stopping it (`FUN_00471D20`) if it is the current one (`0x004FBD68`) |
| `FUN_0042D780` | pumps one Windows message: when one is waiting, gets it (an error on `WM_QUIT`), translates and dispatches it |
| `FUN_004387E0` | appends a formatted line to the file `debug_log`, emptied on the first call, and to the debugger output |
| `FUN_00414930` | appends a 12-byte node holding its first argument to the list at its second argument's `+0x08` |
| `FUN_00409150` | stores a 12-byte record holding the entity's position (and a zero dword) as a task's private record (`FUN_00401260`) |
| `FUN_0040A980` | a random point around an entity: each axis adds `((random >> 11) & 31) * step` to its position word, one RNG word per axis |
| `FUN_0044D8B0` | whether `(x, y, z)` lies within the sum of two radii: a box reject on each axis, then the squared distance |
| `FUN_0046A8D0` | sets dwords 0 and 3 to 1 and normalizes its third argument's `+0x10..+0x18` to a Q12 triple at `+4` (`(0x1000, 0, 0)` for zero) |
