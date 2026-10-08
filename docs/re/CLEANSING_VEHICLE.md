# V2000 Cleansing Vehicle

Type49/model266 (`rover`) owns the mobile cleansing path. It is distinct from
the player inventory's Antidote7 and Antidote Bomb29 bindings, whose names do
not establish a connection to this actor. [VIRUS_SPREAD.md](VIRUS_SPREAD.md)
owns terrain infection and shared mode5/6 particles;
[CAMPAIGN_CARGO.md](CAMPAIGN_CARGO.md) owns saved cargo identity and restoration.

## Authored construction and choices

The canonical normal-tier corpus contains14 births in worlds15,21,26,31,35,39,40.
Its invariant Section12 record has mass100, health2000, capability1204, all four
model slots266, common-axis3072/0, and initializer flags8039. Components are
A/B/C/D only:

| Component | Descriptor |
|---|---|
| A | acceleration1500, correction-3000, base speed500 |
| B | projection100000, correction100000 |
| C | clearance100, range75, strength300000h, boost100, damping200, surface mode1, offset0 |
| D | divisor64, no roll coupling/pitch, probes256/128, classifier11h |

SubD11h rejects steep terrain and obstructing objects but deliberately skips
water and infected-material rejection. The existing shared classifier owns
those branches. Native construction retains the process allocation counter
and an explicitly empty native cache, rather than borrowing a captured seed.
SubA20450 consumes one shared random word before AC60 selection.

The ordered choices are Base Nearby(rule12)×32000→class68 Stationary and
Always(rule1)×1→class42 Cleansing Landscape. Base Nearby4165B0 uses the shared
first-eligible22C10 search with capability20h and the live common-axis limit.
The selector consumes one word even when only one choice has weight.
Stationary C490 clears Secondary then Tertiary and installs a9000-ms Primary
null-callback timer, whose expiry is strict. Its task construction still calls
06070 and consumes the SubA reset word.

## Cleansing and carrying tasks

Class42's canonical style4C85D8 uses initializer40AD50. It constructs/publishes
Tertiary402820(mode6, packed00060000, lifetime0), clears Secondary, then
constructs Primary402FB0/tick403040 with the shared0x24-byte target record.
**Both task constructions call06070**, so successful AD50 consumes two SubA
reset words in slot2 then slot0 order. Initial successful cleansing birth thus
uses A-constructor, selector, terrain-task and movement-task words in order.

403040 inspects infection at its current target, not at the actor position.
If infected, one low16 RNG word retains the target unless modulo64 is zero.
A clean target searches without that gate draw. Search starts at radius256;
each candidate consumes X then Z, using unsigned-low16 division by
`32768 / radius`, and copies anchorY. Coordinates wrap at16 bits. Failed
probes grow radius by signed `3*radius/2`; after eight failures the last target
survives. The center is entity+90, while integrated position is+96. The search
runs in both callback modes before01430. Mover zero returns singleton4BE148
tag9C01; nonzero returns null. The completed wrapper then owns C690/reselection.

The source403040 search was independently executed in Unicorn against the
original PE:800/800 target coordinates and RNG counts matched, seed493040,
with varying infection density, wrapped coordinates, and both callback modes.
Only handle lookup and the following mover were stubbed. This is a static PE
oracle, not a TTD execution receipt (`.tmp/cleansing-seek-oracle.py`).

The probes stay around the anchor, which only construction and a release
from cargo set: the point the rover was beamed out at. The eighth probe
reaches about seventeen cells, so a rover beamed out farther than that from
any infection never finds it; each tick its target becomes another random
clean point and it mills about the drop point. Within the common-axis limit
of the Main Base (twelve cells), Base Nearby parks it in Stationary instead.
This is retail behaviour and the port keeps it.

402850 suppresses its callback while attached (1000h). Detailed mode emits
class6 through440DC0, caller return4029DB; coarse mode directly clears through
33720, return4028EB. Class6's43E1A0 terminal response clears through33720 with
return43E21B. Particle response6 instead dampens velocity and keeps the particle.

Normal cleansing attachment CD50/CD70 clears8000h and selects variant1,
style4C8620, enable80h/disable2. ADB0 clears Tertiary then Secondary and installs
Primary None03230/03250. This constructor consumes one SubA reset word; its
tick has no effect because Type49 has no SubI. Stationary and already carrying
styles have null attachment callbacks and retain their existing tasks.

Release from Stationary/normal cleansing runs D1C0→16AC0 terrain basis→AC60.
Release from carrying runs CE90, sets8000h, skips absent SubI, then AC60.
Player→Type93 and Type93→world are separate releases and selections.
409030 copies the proxy position to both+90/+96 before the latter release.
Neither release replaces the rover's construction stamp or resets its health.

## Damage and class49 ownership

Thresholds are `[0,10000,200,0,200,0,0]`; Q8 multipliers are
`[0,256,256,512,128,0,64]`. Before runtime eligibility/modifiers, canonical
player-primary damage filters to1800, Newant47/class87 to1050, and class5
infection to500. Current stationary/cleansing/carrying style20/28/2C hooks are
null; that does not bypass common primary/infected wrappers or checked damage.
Alternate class49 is40BD20: BAF0 explosion/radial bundle, clear tasks, conditional
Type60 ring construction, then deferred removal. The native hit/death bridge
owns this complete synchronous sequence through the shared
[class49 death](../../crates/v2k-game/src/class49_death.rs) and
[terminal](../../crates/v2k-game/src/class49_terminal.rs) modules. A retained
source profile preserves the distinct rover and E/L turret burst branches.

BAF0 tests runtime A/B/N/G before its capability and type-specific branches.
The rover's nonnull A/B selects ten class37 particles at40BBB3. Its later
type49 branch (thirty class6 particles) requires all four tested components to
be absent and is not reachable for this authenticated native construction.
After440950, BAF0 dispatches the Section12 radial packet through4566E0.
BD20 then clears tasks with A860, conditionally constructs the Type60 ring
unless80000000h is set, and requests deferred removal through10B70.

The NoCD02 TTD provides a separate, stronger weapon observation. At
`49FB4F:369`, tick644h, a class55 particle from player type46/owner04C90001
delivers channels `[2,3]`, amounts `[3000,1000]` to rover04940001. The filter
returns4800, reducing health2000 to-2800 before10C10 zeroes it and selects
class49. The BD20 entry is `49FB4F:656`; deferred removal returns at
`49FB5F:42C`. This recorded kill is player damage, not a Newant attribution.

The same recording proves a detailed cleansing particle's terminal clear:
at `460588:973`, particle4DE080 is class6, birth owner04940001/type49, and
position `[8FC0,FBA9,3F08]`. The33720 call at `460588:985` comes from43E21B
with mode0; cell `[143,63]` changes bytes `DD0054` to `DD0044` by
`46058A:17A3`. Thus only infection bit10h clears; cell height and other
material bits survive. Accepted trace identity and query commands belong in
the capture ledger; the local bounded query is
`.tmp/nocd02-cleansing-oracle-v2.txt`.

Native construction, current task state, cargo restoration and live scheduling
are implemented in [cleansing_vehicle](../../crates/v2k-game/src/cleansing_vehicle.rs).
The current outer frame admits the shared no-wind environment path. Nonzero wind
and unrelated ordinary static-world dispatch remain explicit boundaries, not
type- or level-specific no-ops. Runtime TTD evidence and current hit/death
acceptance belong in the capture ledger and owning damage notes. Focused
regressions cover native birth RNG, all14 authored births, saved missing-row
restoration, collect/drop/settle, both callback modes, null primary/infected
hooks, Newant damage1050, the recorded player damage4800, and a second physical
hit after completed class49 without duplicating the blast or ring.

## NoCD02 replay provenance

The user supplied `<install>\V2000-nocd02.run`. Original and NoCD images
are both936448 bytes at image base400000h, with identical sections and entry.
`nocd02-identity.json` records SHA256 values:

| Image | SHA256 |
|---|---|
| Retail `V2000.EXE` | `e9be7a833612fba3a5a5ab92a974ece1a689e4b7e72409d9ee8331380573b4ba` |

The NoCD image differs from retail only in its protection branch; gameplay
addresses used here are unchanged. The user-supplied executable stays local
and private; only the retail hash above is versioned.

Accepted query stems are `nocd02-timeline`, `nocd02-save-state`,
`nocd02-cleansing-events`, `nocd02-cleansing-oracle-v2`,
`nocd02-save-ui-code`, and `nocd02-checkpoints`. Each `.txt` and `.windbg.cmd`,
the shared `nocd02-query.ps1` launcher, and `nocd02-identity.json` are retained
byte-for-byte under `runtime_re/captures/local/`. The failed
`nocd02-cleansing-oracle` v1 query is excluded. Exact headless command for v2:

```powershell
& '<WinDbg>\cdb.exe' -sins -y '<workspace>\.tmp\ttd-worker-symbols' -logo '<workspace>\.tmp\nocd02-cleansing-oracle-v2.txt' -z '<install>\V2000-nocd02.run' -c '$$><<workspace>\.tmp\nocd02-cleansing-oracle-v2.windbg.cmd'
```

The other five invocations use the same executable, switches, symbol path and
trace, replacing only the log/script stem with the exact names above; the
retained launcher prints each complete command. These are read-only replay
queries, with pseudo-register scratch values rather than retail-memory writes.
All six logs reach their `NOCD02_*_COMPLETE` marker. The headless debugger's final
command-file syntax diagnostic occurs after that marker; the launcher then
closes only its owned debugger process. Missing local Windows DLL symbol
images do not replace the retained game-address reads. Runtime evidence does
not establish an Antidote7/Antidote Bomb29 firing binding or a Newant kill.
