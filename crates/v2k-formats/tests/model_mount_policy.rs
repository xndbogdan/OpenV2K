//! Mount-restore opcode policy over every shipped Section-8 model stream.
//!
//! Retail `FUN_00467010` (opcode `0x10`) publishes the composed mount basis
//! as the current mount. The port skips the publish because every shipped
//! `0x10` immediately follows an `0x0F` mount reset, so the composed basis is
//! Q1.31 identity and the publish is a fixed point
//! (see `FORMAT_DOCUMENTATION.md` and the `0x10` arm in `models.rs`).
//! This test machine-checks that precondition over the full corpus: all
//! tiers' Overlay OVLs plus `PRELOAD.DAT`'s embedded OVLs. If any stream
//! ever uses `0x10` anywhere else, the skip is no longer exact and the
//! test fails until a real restore is implemented.

use std::collections::BTreeSet;
use std::path::PathBuf;

fn overlay_dir() -> PathBuf {
    v2k_test_support::retail_dir().join("Overlay")
}

fn fixed_len(op: u16) -> Option<usize> {
    Some(match op {
        0x03 | 0x43 | 0x83 | 0xC3 | 0x07 | 0x47 | 0x87 | 0xC7 => 6,
        0x04 | 0x44 | 0x84 | 0xC4 | 0x08 | 0x48 | 0x88 | 0xC8 => 7,
        0x23 | 0xA3 | 0x27 | 0xA7 => 9,
        0x24 | 0xA4 | 0x28 | 0xA8 => 11,
        0x02 => 4,
        0x22 | 0x68 | 0xE8 | 0x78 | 0xB8 | 0xF8 => 5,
        0x46 | 0x15 => 2,
        0xA6 | 0x1C => 3,
        0xC6 | 0x3C | 0x5C => 4,
        0x66 | 0x86 | 0xE6 | 0x0F | 0x10 => 1,
        0x0B | 0x0C => 3,
        0x2B | 0x2C => 4,
        0x13 | 0x14 => 4,
        op if op <= 0xFF && op & 0x0F == 0x0D => 4,
        _ => return None,
    })
}

/// Walk one stream with the canonical dispatch (both sides of every
/// conditional branch). Returns (0x10 command positions, command starts,
/// unknown-opcode stops, truncated stops).
fn walk(words: &[u16]) -> (Vec<usize>, BTreeSet<usize>, u64, u64) {
    let mut restores = Vec::new();
    let mut starts = BTreeSet::new();
    let (mut unknown, mut truncated) = (0u64, 0u64);
    let mut pending = vec![0usize];
    while let Some(pc) = pending.pop() {
        if pc >= words.len() || !starts.insert(pc) {
            continue;
        }
        let op = words[pc];
        if op == 0x00 {
            continue;
        }
        if op > 0xFF {
            continue;
        }
        if op == 0x10 {
            restores.push(pc);
        }
        if op == 0x0E {
            if pc + 3 >= words.len() {
                truncated += 1;
                continue;
            }
            let next = pc.saturating_add(1).saturating_add((words[pc + 3] / 2) as usize);
            if next <= words.len() {
                pending.push(next);
            } else {
                truncated += 1;
            }
            continue;
        }
        if matches!(op, 0x06 | 0x26 | 0x38) {
            let mut q = pc + 1;
            while q < words.len() && words[q] != 0xFFFF {
                q += 1;
            }
            if q < words.len() {
                pending.push(q + 1);
            } else {
                truncated += 1;
            }
            continue;
        }
        let Some(len) = fixed_len(op) else {
            unknown += 1;
            continue;
        };
        if matches!(op, 0x0B | 0x0C) && pc + 1 < words.len() {
            if let Some(t) = (pc + 1).checked_add_signed(words[pc + 1] as i16 as isize) {
                if t < words.len() {
                    pending.push(t);
                }
            }
        }
        if matches!(op, 0x2B | 0x2C) && pc + 1 < words.len() {
            let t = pc + 1 + words[pc + 1] as usize;
            if t < words.len() {
                pending.push(t);
            }
        }
        if pc + len <= words.len() {
            pending.push(pc + len);
        } else {
            truncated += 1;
        }
    }
    restores.sort_unstable();
    restores.dedup();
    (restores, starts, unknown, truncated)
}

#[v2k_test_support::retail_test]
fn every_restore_follows_a_reset() {
    let mut files: Vec<PathBuf> = std::fs::read_dir(overlay_dir())
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|e| e.eq_ignore_ascii_case("ovl")))
        .collect();
    files.sort();
    let preload_data = std::fs::read(v2k_test_support::retail_dir().join("PRELOAD.DAT")).unwrap();
    let preload = v2k_formats::preload::PreloadDat::parse(&preload_data).unwrap();

    let mut ovls: Vec<v2k_formats::ovl::OvlFile> = Vec::new();
    for path in &files {
        let data = std::fs::read(path).unwrap();
        let Ok(ovl) = v2k_formats::ovl::OvlFile::parse(&data) else {
            continue;
        };
        ovls.push(ovl);
    }

    let (mut streams, mut restores, mut unknown, mut truncated) = (0u64, 0u64, 0u64, 0u64);
    let mut violations = Vec::new();
    let mut scan = |ovl: &v2k_formats::ovl::OvlFile| {
        let Ok(models) = v2k_formats::sections::parse_models(ovl) else {
            return;
        };
        for (i, entry) in models.all_entries.iter().enumerate() {
            if entry.cmd_words.is_empty() {
                continue;
            }
            streams += 1;
            let (hits, starts, u, t) = walk(&entry.cmd_words);
            unknown += u;
            truncated += t;
            for pc in hits {
                restores += 1;
                // 0x0F and 0x10 are both length-1 commands, so a 0x0F word
                // immediately before is the preceding command itself.
                let ok = pc > 0
                    && entry.cmd_words[pc - 1] == 0x0F
                    && starts.contains(&(pc - 1));
                if !ok {
                    violations.push(format!(
                        "entry {i} ({}) 0x10 at word {pc} not preceded by 0x0F",
                        entry.name.as_deref().unwrap_or("?")
                    ));
                }
            }
        }
    };
    for ovl in &ovls {
        scan(ovl);
    }
    for embedded in preload.embedded_ovls {
        scan(&embedded.ovl);
    }
    assert_eq!(unknown, 0, "unknown opcodes would misalign the walk");
    assert_eq!(truncated, 0, "truncated streams would hide 0x10 sites");
    assert!(
        restores > 0,
        "no 0x10 found: the guard would pass vacuously (scanned {streams} streams)"
    );
    assert!(
        violations.is_empty(),
        "{} restore(s) outside 0F/10 (of {restores} in {streams} streams): {:?}",
        violations.len(),
        violations.iter().take(8).collect::<Vec<_>>()
    );
}
