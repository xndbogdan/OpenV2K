"""Suspend-sample a Windows process's threads with frame-pointer walks.

Launches the game itself, waits out loading, then suspends each busy
thread ~60 times a second and walks its RBP chain (the profiling build
forces frame pointers, so no debug engine is needed). No administrator
rights needed for our own child process. Output is JSONL plus a meta
file with the process's module list (for ASLR rebasing).

Usage:
    python3 scripts/perf-sample.py --out .tmp/perf/prof/soft30
        --warmup 8 --duration 60 --rate 100 --
        target/prof/ship/v2k-game.exe --data-dir retail --settings ...
Pass the game and its arguments after `--`. The game is left to exit on
its own (--frames should outlast warmup + duration); sampling also stops
when the process exits. Needs a profiling build: same profile as the
measured binary plus line tables and frame pointers, in its own target
dir so the measured binaries are untouched, e.g.
    CARGO_TARGET_DIR=target/prof CARGO_PROFILE_SHIP_DEBUG=line-tables-only
    RUSTFLAGS="-C link-arg=-Wl,--stack,8388608 -C target-cpu=x86-64-v2 -C force-frame-pointers=yes"
    cargo build --profile ship -p v2k-game
Only ctypes + stdlib; 64-bit Python required (matches the 64-bit game).
"""

import argparse
import ctypes
import json
import os
import subprocess
import sys
import time

kernel32 = ctypes.WinDLL('kernel32', use_last_error=True)

TH32CS_SNAPTHREAD = 0x00000004
INVALID_HANDLE_VALUE = ctypes.c_void_p(-1).value
THREAD_SUSPEND_RESUME = 0x0002
THREAD_GET_CONTEXT = 0x0008
THREAD_QUERY_INFORMATION = 0x0040
CONTEXT_FULL = 0x10000B


class CONTEXT(ctypes.Structure):
    _fields_ = [('P1Home', ctypes.c_uint64),
                ('P2Home', ctypes.c_uint64),
                ('P3Home', ctypes.c_uint64),
                ('P4Home', ctypes.c_uint64),
                ('P5Home', ctypes.c_uint64),
                ('P6Home', ctypes.c_uint64),
                ('ContextFlags', ctypes.c_uint32),
                ('MxCsr', ctypes.c_uint32),
                ('SegCs', ctypes.c_uint16),
                ('SegDs', ctypes.c_uint16),
                ('SegEs', ctypes.c_uint16),
                ('SegFs', ctypes.c_uint16),
                ('SegGs', ctypes.c_uint16),
                ('SegSs', ctypes.c_uint16),
                ('EFlags', ctypes.c_uint32),
                ('Dr0', ctypes.c_uint64),
                ('Dr1', ctypes.c_uint64),
                ('Dr2', ctypes.c_uint64),
                ('Dr3', ctypes.c_uint64),
                ('Dr6', ctypes.c_uint64),
                ('Dr7', ctypes.c_uint64),
                ('Rax', ctypes.c_uint64),
                ('Rcx', ctypes.c_uint64),
                ('Rdx', ctypes.c_uint64),
                ('Rbx', ctypes.c_uint64),
                ('Rsp', ctypes.c_uint64),
                ('Rbp', ctypes.c_uint64),
                ('Rsi', ctypes.c_uint64),
                ('Rdi', ctypes.c_uint64),
                ('R8', ctypes.c_uint64),
                ('R9', ctypes.c_uint64),
                ('R10', ctypes.c_uint64),
                ('R11', ctypes.c_uint64),
                ('R12', ctypes.c_uint64),
                ('R13', ctypes.c_uint64),
                ('R14', ctypes.c_uint64),
                ('R15', ctypes.c_uint64),
                ('Rip', ctypes.c_uint64),
                ('Rest', ctypes.c_ubyte * (1232 - 256))]


assert ctypes.sizeof(CONTEXT) == 1232


class THREADENTRY32(ctypes.Structure):
    _fields_ = [('dwSize', ctypes.c_uint32),
                ('cntUsage', ctypes.c_uint32),
                ('th32ThreadID', ctypes.c_uint32),
                ('th32OwnerProcessID', ctypes.c_uint32),
                ('tpBasePri', ctypes.c_int32),
                ('tpDeltaPri', ctypes.c_int32),
                ('dwFlags', ctypes.c_uint32)]


class FILETIME(ctypes.Structure):
    _fields_ = [('dwLowDateTime', ctypes.c_uint32),
                ('dwHighDateTime', ctypes.c_uint32)]


def _ft(f):
    return (f.dwHighDateTime << 32) | f.dwLowDateTime


def setup():
    kernel32.CreateToolhelp32Snapshot.restype = ctypes.c_void_p
    kernel32.OpenThread.restype = ctypes.c_void_p
    kernel32.OpenThread.argtypes = [ctypes.c_uint32, ctypes.c_int, ctypes.c_uint32]
    kernel32.OpenProcess.restype = ctypes.c_void_p
    kernel32.OpenProcess.argtypes = [ctypes.c_uint32, ctypes.c_int, ctypes.c_uint32]
    kernel32.SuspendThread.argtypes = [ctypes.c_void_p]
    kernel32.ResumeThread.argtypes = [ctypes.c_void_p]
    kernel32.GetThreadContext.argtypes = [ctypes.c_void_p, ctypes.c_void_p]
    kernel32.ReadProcessMemory.argtypes = [
        ctypes.c_void_p, ctypes.c_void_p, ctypes.c_void_p,
        ctypes.c_size_t, ctypes.POINTER(ctypes.c_size_t)]


def list_threads(pid):
    snap = kernel32.CreateToolhelp32Snapshot(TH32CS_SNAPTHREAD, 0)
    if snap == INVALID_HANDLE_VALUE:
        return []
    entry = THREADENTRY32()
    entry.dwSize = ctypes.sizeof(THREADENTRY32)
    tids = []
    if kernel32.Thread32First(snap, ctypes.byref(entry)):
        while True:
            if entry.th32OwnerProcessID == pid:
                tids.append(entry.th32ThreadID)
            if not kernel32.Thread32Next(snap, ctypes.byref(entry)):
                break
    kernel32.CloseHandle(snap)
    return tids


def thread_cpu(handle):
    creation, exit_t, kernel, user = FILETIME(), FILETIME(), FILETIME(), FILETIME()
    if not kernel32.GetThreadTimes(handle, ctypes.byref(creation),
                                   ctypes.byref(exit_t), ctypes.byref(kernel),
                                   ctypes.byref(user)):
        return None
    return _ft(kernel) + _ft(user)


def _nt_query_teb(h_thread):
    """TEB address of a thread via NtQueryInformationThread."""
    ntdll = ctypes.WinDLL('ntdll', use_last_error=True)

    class THREAD_BASIC_INFORMATION(ctypes.Structure):
        _fields_ = [('ExitStatus', ctypes.c_long),
                    ('TebBaseAddress', ctypes.c_void_p),
                    ('UniqueProcessId', ctypes.c_void_p),
                    ('UniqueThreadId', ctypes.c_void_p),
                    ('AffinityMask', ctypes.c_uint64),
                    ('BasePriority', ctypes.c_long),
                    ('Priority', ctypes.c_long)]

    ntdll.NtQueryInformationThread.restype = ctypes.c_long
    ntdll.NtQueryInformationThread.argtypes = [
        ctypes.c_void_p, ctypes.c_int, ctypes.c_void_p,
        ctypes.c_uint32, ctypes.POINTER(ctypes.c_uint32)]
    info = THREAD_BASIC_INFORMATION()
    status = ntdll.NtQueryInformationThread(
        h_thread, 0, ctypes.byref(info), ctypes.sizeof(info), None)
    if status != 0:
        return None
    return info.TebBaseAddress


def _read_stack_word(h_process, address):
    word = ctypes.c_uint64(0)
    n = ctypes.c_size_t(0)
    if not kernel32.ReadProcessMemory(
            h_process, ctypes.c_void_p(address),
            ctypes.byref(word), 8, ctypes.byref(n)) or n.value != 8:
        return None
    return word.value


def walk_stack(h_process, h_thread, max_frames=64):
    """Frame-pointer chain walk (the profiling build forces frame pointers).

    Every step is validated: the next RBP must move up the thread's stack
    and the return address must land in a known module. A broken chain ends
    the stack instead of producing garbage. Returns [current RIP, ...].
    """
    ctx = CONTEXT()
    ctx.ContextFlags = CONTEXT_FULL
    if not kernel32.GetThreadContext(h_thread, ctypes.byref(ctx)):
        return []
    teb = _nt_query_teb(h_thread)
    if not teb:
        return []
    lo = _read_stack_word(h_process, teb + 16)
    hi = _read_stack_word(h_process, teb + 8)
    if not lo or not hi:
        return []
    pcs = [ctx.Rip]
    rbp = ctx.Rbp
    for _ in range(max_frames):
        if not rbp or rbp < lo or rbp >= hi or rbp & 7:
            break
        prev = _read_stack_word(h_process, rbp)
        ret = _read_stack_word(h_process, rbp + 8)
        if prev is None or ret is None:
            break
        pcs.append(ret)
        if prev == 0:
            break
        if prev <= rbp or prev >= hi:
            break
        rbp = prev
    return pcs


def process_modules(pid):
    mods = []
    h = kernel32.OpenProcess(0x0400 | 0x0010, False, pid)  # QUERY_INFORMATION | VM_READ
    if not h:
        return mods
    try:
        arr = (ctypes.c_void_p * 1024)()
        needed = ctypes.c_uint32(0)
        psapi = ctypes.WinDLL('psapi', use_last_error=True)
        psapi.EnumProcessModules.argtypes = [
            ctypes.c_void_p, ctypes.c_void_p, ctypes.c_uint32,
            ctypes.POINTER(ctypes.c_uint32)]
        psapi.GetModuleFileNameExA.argtypes = [
            ctypes.c_void_p, ctypes.c_void_p, ctypes.c_char_p, ctypes.c_uint32]
        psapi.GetModuleFileNameExA.restype = ctypes.c_uint32
        if psapi.EnumProcessModules(h, arr, ctypes.sizeof(arr), ctypes.byref(needed)):
            count = needed.value // ctypes.sizeof(ctypes.c_void_p)
            for i in range(count):
                name = ctypes.create_string_buffer(1024)
                if psapi.GetModuleFileNameExA(h, arr[i], name, 1024):
                    mods.append({'path': name.value.decode('utf-8', 'replace'),
                                 'base': arr[i]})
    finally:
        kernel32.CloseHandle(h)
    mods.sort(key=lambda m: m['base'])
    return mods


def main():
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument('--out', required=True, help='output dir (.tmp/perf/...)')
    ap.add_argument('--warmup', type=float, default=8.0)
    ap.add_argument('--duration', type=float, default=60.0)
    ap.add_argument('--rate', type=float, default=100.0)
    ap.add_argument('cmd', nargs=argparse.REMAINDER,
                    help='game and its args after `--`')
    args = ap.parse_args()
    if not args.cmd or args.cmd[0] == '--':
        args.cmd = [c for c in args.cmd if c != '--']
    if not args.cmd:
        ap.error('no game command after `--`')
    # Store Python cannot resolve relative paths for CreateProcess.
    args.cmd[0] = os.path.abspath(args.cmd[0])
    setup()
    os.makedirs(args.out, exist_ok=True)
    winmm = ctypes.WinDLL('winmm')
    winmm.timeBeginPeriod(1)

    proc = subprocess.Popen(args.cmd)
    pid = proc.pid
    h_process = kernel32.OpenProcess(0x0400 | 0x0010 | 0x0008, False, pid)  # query/vm-read/vm-op
    if not h_process:
        print('OpenProcess failed', file=sys.stderr)
        proc.terminate()
        return 1
    try:
        deadline = time.perf_counter() + args.warmup
        while time.perf_counter() < deadline and proc.poll() is None:
            time.sleep(0.2)
        modules = process_modules(pid)
        with open(os.path.join(args.out, 'meta.json'), 'w') as f:
            json.dump({'exe': args.cmd[0], 'argv': args.cmd[1:], 'pid': pid,
                       'warmup': args.warmup, 'duration': args.duration,
                       'rate': args.rate, 'modules': modules}, f, indent=1)
        out = open(os.path.join(args.out, 'samples.jsonl'), 'w')
        interval = 1.0 / args.rate
        cpu = {}
        tids = []
        refreshed = 0.0
        rounds = 0
        skipped = 0
        end = time.perf_counter() + args.duration
        try:
            while time.perf_counter() < end and proc.poll() is None:
                tick = time.perf_counter()
                # Thread enumeration is ~60 ms; the game's threads are
                # long-lived after loading, so refresh at 0.2 Hz.
                if not tids or tick - refreshed > 5.0:
                    tids = list_threads(pid)
                    refreshed = tick
                busy = []
                dead = []
                for tid in tids:
                    h = kernel32.OpenThread(
                        THREAD_QUERY_INFORMATION, False, tid)
                    if not h:
                        dead.append(tid)
                        continue
                    try:
                        now = thread_cpu(h)
                    finally:
                        kernel32.CloseHandle(h)
                    last = cpu.get(tid, None)
                    cpu[tid] = now
                    if now is not None and (last is None or now != last):
                        busy.append(tid)
                    else:
                        skipped += 1
                if dead:
                    tids = [t for t in tids if t not in dead]
                for tid in busy:
                    h = kernel32.OpenThread(
                        THREAD_SUSPEND_RESUME | THREAD_GET_CONTEXT | THREAD_QUERY_INFORMATION,
                        False, tid)
                    if not h:
                        continue
                    try:
                        if kernel32.SuspendThread(h) == 0xFFFFFFFF:
                            continue
                        try:
                            stack = walk_stack(h_process, h)
                        finally:
                            kernel32.ResumeThread(h)
                        out.write(json.dumps(
                            {'t': time.time(), 'tid': tid, 'stack': stack}) + '\n')
                        rounds += 1
                    finally:
                        kernel32.CloseHandle(h)
                wait = interval - (time.perf_counter() - tick)
                if wait > 0:
                    time.sleep(wait)
        finally:
            out.close()
        print(f'samples: {rounds}, idle skips: {skipped}')
    finally:
        kernel32.CloseHandle(h_process)
        winmm.timeEndPeriod(1)
        if proc.poll() is None:
            proc.terminate()
    return 0


if __name__ == '__main__':
    sys.exit(main())
