"""Bucket perf-sample.py stacks into a hotspot table.

Symbolizes the game's own frames with addr2line -i (inlined frames
expanded, attributed to every frame in the chain) and everything else by
module name. Prints samples, share and milliseconds per frame.

Usage:
    python3 scripts/perf-bucket.py --dir .tmp/perf/prof/soft30 --ms 2.7
--ms is the unsampled millisecond cost of one frame (e.g. 3000 frames /
median fps from perf-bench.ps1); shares turn into ms at that cost.
"""

import argparse
import json
import os
import subprocess
import sys
from collections import Counter


def load(dirpath):
    with open(os.path.join(dirpath, 'meta.json')) as f:
        meta = json.load(f)
    samples = []
    with open(os.path.join(dirpath, 'samples.jsonl')) as f:
        for line in f:
            line = line.strip()
            if line:
                samples.append(json.loads(line))
    return meta, samples


def module_ranges(meta):
    mods = sorted(meta['modules'], key=lambda m: m['base'])
    exe = os.path.abspath(meta['exe']).lower()
    ranges = []
    for i, m in enumerate(mods):
        base = m['base']
        path = m['path']
        # Size unknown; bound by the next module's base.
        top = mods[i + 1]['base'] if i + 1 < len(mods) else base + (1 << 32)
        ranges.append((base, top, path,
                       os.path.abspath(path).lower() == exe))
    return ranges


def find_module(ranges, rip):
    for base, top, path, is_exe in ranges:
        if base <= rip < top:
            return path, is_exe, rip - base
    return '<unknown>', False, rip


def main():
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument('--dir', required=True)
    ap.add_argument('--ms', type=float, default=None,
                    help='ms per frame at unsampled speed')
    ap.add_argument('--top', type=int, default=40)
    args = ap.parse_args()

    meta, samples = load(args.dir)
    ranges = module_ranges(meta)
    exe = meta['exe']

    exe_rvas = set()
    per_sample = []
    tids = Counter()
    for s in samples:
        tids[s['tid']] += 1
        frames = []
        for rip in s['stack']:
            path, is_exe, rva = find_module(ranges, rip)
            if is_exe:
                exe_rvas.add(rva)
                frames.append(('exe', rva))
            else:
                frames.append(('mod', os.path.basename(path)))
        per_sample.append(frames)
    print(f'samples: {len(samples)}, threads: {dict(tids)}')

    import bisect
    import shutil

    def find_nm():
        for cand in [shutil.which('llvm-nm'),
                     'C:/Users/bogdan/.rustup/toolchains/1.95-x86_64-pc-windows-msvc'
                     '/lib/rustlib/x86_64-pc-windows-msvc/bin/llvm-nm.exe']:
            if cand and os.path.exists(cand):
                return cand
        raise SystemExit('llvm-nm not found (needs the 1.95 MSVC toolchain)')

    # Symbolize by nearest function start: MinGW addr2line (2.39) cannot
    # parse rustc's DWARF, but llvm-nm demangles the text symbols fine.
    nm = find_nm()
    proc = subprocess.run([nm, '--defined-only', '--demangle', exe],
                          capture_output=True, text=True)
    syms = []
    for line in proc.stdout.splitlines():
        parts = line.split(None, 2)
        if len(parts) != 3:
            continue
        addr, kind, name = parts
        if kind in ('t', 'T', 'W', 'w'):
            try:
                syms.append((int(addr, 16), name))
            except ValueError:
                pass
    syms.sort()
    # nm addresses carry the link-time image base (0x140000000); RVAs from
    # the sampled process are rebased onto it for the nearest-below lookup.
    image_base = 0x140000000
    addrs = [a for a, _ in syms]

    def attribute(rva):
        target = image_base + rva
        i = bisect.bisect_right(addrs, target) - 1
        return syms[i][1] if i >= 0 else '??'

    counts = Counter()
    dropped = 0
    for frames in per_sample:
        for kind, value in frames:
            if kind == 'exe':
                fn = attribute(value)
                if fn == '??':
                    dropped += 1
                    continue
                counts[fn] += 1
            elif value == '<unknown>':
                dropped += 1
            else:
                counts[f'[{value}]'] += 1
    total = sum(counts.values())
    print(f'stack frames attributed: {total} (dropped {dropped} unattributable)')
    print(f"{'samples':>8} {'share':>6} {'ms':>7}  function")
    for fn, n in counts.most_common(args.top):
        share = 100.0 * n / total if total else 0
        ms = f'{share * args.ms / 100:.2f}' if args.ms else '-'
        print(f'{n:>8} {share:5.1f}% {ms:>7}  {fn}')
    return 0


if __name__ == '__main__':
    sys.exit(main())
