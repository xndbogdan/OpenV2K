# Retail overlay integrity

## NoCD05 Alpine load failure

`V2000-nocd05.run` records a data-read failure, followed by a separate error-dialog
crash. It does not establish a media-authentication failure. The recorded
image differs from retail only in its protection branch (see
[NoCD02 replay provenance](CLEANSING_VEHICLE.md#nocd02-replay-provenance)).

The recording loads registry save `<install>\Slot13`, enters Castle
(logical world 3, global OVL15), and later returns to the menu and loads
`Slot02` for Alpine (global OVL17). It does not contain a successful Alpine
entry or a Medieval-to-Alpine gameplay transition.

At `75177A:C91`, the CRT opens
`<install>\overlay\1x17xx.ovl` in `rb` mode successfully.
The Section-14 marker is at file offset `0x30A2F`; its length word says
`0x4DC` (1,244), but only `0x4D2` (1,234) bytes remain after the header.
At `751786:1AD1`, `004AB3B0` calls `00468E70` for the declared 1,244 bytes.
The stream reaches EOF and `00468E70` constructs status `0x1002` at
`751788:2C0`. Its ordinary reader is active (`004F7310 == 0`), with no
interleaved-media reader involved.

`00468E70` formats the CRT errno even for a short EOF read. The retained errno
2 produces the misleading **No such file or directory** text. `00493DB0`
wraps that as status `0x1103`, **Error loading overlay (1,17)**. The user's
screenshot agrees with the captured error at `753FD0:1B4A`.

The recorded 1,234-byte read buffer is byte-identical to the final section
in all inspected local copies before restoration. Its SHA-256 is
`2F609D23CEE65AA31AABEE879F6D210130950F11010CE2E5499765A277F35894`.
The truncated 200,457-byte OVL had SHA-256
`2E95142BF42E60E8EE5B2691606163D00C1BB89E08F06F40935F56D49168CE9D`
in the repository's retail directory, the port data directory, and
`<install>\Overlay`. All four presentation variants have this same
ten-byte Section-14 shortfall.

After cleanup, the fatal **Data file corrupted** message box re-enters the
window procedure on `WM_KILLFOCUS`. At `75406E:0`, `004AB380` finds a nonnull
object at `004FBFC8`, reads its null vtable, and faults on `call [ecx+28]`.
The process exits with `0xC000041D`. This secondary callback crash is not the
cause of the failed load, and suppressing the dialog or shutdown would not
repair the overlay.

## Disc-backed repair

The user subsequently supplied the mounted original disc at
`E:\V2000\OVERLAY`. Its Alpine file is 200,467 bytes, with SHA-256
`8657D87D074D8DEA487DE389809DAB2454E3E8F1998140F0615D4EA8AEFF9F65`.
The installed file is an exact prefix of that file; the missing ten bytes are
`00 00 89 0A 00 00 77 02 00 00`. They complete sprite ID `0x366` and supply
IDs `0xA89` and `0x277`, so zero-filling would corrupt the dependencies.

Comparing all 212 overlays against the disc found exactly 36 differences:
all four tiers of the nine worlds below. Every mismatch was an exact-prefix
truncation, and every disc file has its complete declared final section.
Those 36 files were restored verbatim in each of `v2000/Overlay`,
`Overlay` in the working directory, and `<install>\Overlay`, with preflight source and
destination hashes, backups, and post-copy hash verification. This changes
no executable bytes, save files, or internal overlay offsets. The corruption's
original source is not established by this recording.

The repair manifest and original short-read evidence remain with the local
capture; the repaired retail data is not a source-code change. The old copies
are retained under `.tmp/nocd05-overlay-originals/`.

## Provenance and truncation pattern

All 36 preserved damaged files are byte-identical to their blobs in the
repository's first commit, `6197fb9d2d1d07274fbd25d6c36e00e21dc55e4b`
(2026-02-14, the monorepo import). This is the root of the available history,
not a shallow-clone boundary. The data was therefore already short when
imported; subsequent port development and extraction cannot explain its
original truncation. That history does not identify the earlier installation,
archive or copy operation which supplied the files.

The four tiers have identical bytes and cutoff positions within each affected
world. Every retained byte matches the corresponding disc file. This supports
an inherited damaged source propagated to the local copies; it does not prove
which program first shortened that source. The cutoffs have no common
16-byte or 512-byte alignment.

Several specific explanations are contradicted by the bytes:

- DOS text-mode Ctrl-Z termination: Alpine retains 245 `0x1A` bytes, the
  first at offset 847 and the last at 200263, before its cutoff at 200457.
  Every affected world likewise retains earlier Ctrl-Z bytes.
- CRLF conversion: all retained bytes remain identical, including the
  embedded CRLF in worlds 24 and 25. Their losses occur only at EOF.
  A text-mode byte count used for a later raw copy also does not fit:
  Alpine loses ten bytes despite having no CRLF anywhere, and world 24
  loses six despite having only one. Worlds 33 and 45 have no CR or LF
  in their final section, yet lose five bytes and one byte respectively.
- Removing trailing NULs or whitespace: several short files still end in
  NUL bytes, and the lost tails of worlds 17, 22 and 24 contain nonzero
  sprite-ID bytes. The tail lengths are not a uniform off-by-one error.

The exact pre-import corruption mechanism remains unresolved. An incomplete
source or earlier copy/extraction defect is consistent with the evidence;
naming a particular tool or text conversion is not justified.

## Other originally short final sections

A read-only audit of normal-tier world overlays finds these declared/actual
Section-14 lengths before restoration. This table identifies the damaged local
corpus, not every release of the game.

| Global OVL | Declared bytes | Available bytes | Missing bytes | Exact lost tail (hex) |
|---|---:|---:|---:|---|
| 17 | 1244 | 1234 | 10 | `00 00 89 0A 00 00 77 02 00 00` |
| 19 | 1372 | 1370 | 2 | `00 00` |
| 22 | 620 | 615 | 5 | `00 66 03 00 00` |
| 24 | 1268 | 1262 | 6 | `00 00 47 03 00 00` |
| 25 | 2432 | 2431 | 1 | `00` |
| 27 | 1924 | 1923 | 1 | `00` |
| 33 | 40 | 35 | 5 | `00 00 00 00 00` |
| 45 | 40 | 39 | 1 | `00` |
| 49 | 628 | 627 | 1 | `00` |

Alpine's dependency record requests 302 four-byte sprite IDs starting at
offset 36, ending at the declared 1,244-byte boundary. The missing bytes are
therefore inside the requested dependency data, not proven dispensable padding.
Do not shorten the length word, skip the failed read, or append guessed bytes.
A repair requires an independently intact source of the corresponding overlay,
as provided by the mounted disc above.
The Rust container parser preserves available bytes, and the linkage reader
rejects the incomplete sprite-dependency list; neither is evidence that retail
can load this truncated file.

Replay scripts, transcripts, buffer and exact launch are retained in the
capture ledger.
