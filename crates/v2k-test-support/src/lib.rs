//! Shared private-corpus policy for build scripts and tests, never game runtime.
//!
//! Missing/empty installations produce real libtest `ignored` results. Once
//! payload files exist, missing files and parse errors fail instead of skipping.

use std::path::{Path, PathBuf};

pub use v2k_test_macros::{demo_test, retail_test};

fn repository_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("test support lives under crates/")
        .to_path_buf()
}

fn configured_dir(variable: &str, fallback: &str) -> PathBuf {
    let root = repository_root();
    match std::env::var_os(variable) {
        Some(value) => {
            assert!(!value.is_empty(), "{variable} must not be empty");
            // Relative overrides are always repository-relative, including when
            // Cargo invokes build scripts and tests from different directories.
            root.join(value)
        }
        None => root.join(fallback),
    }
}

/// Directory containing PRELOAD.DAT and Overlay/ from a private retail install.
pub fn retail_dir() -> PathBuf {
    configured_dir("V2K_RETAIL_DIR", "retail")
}

/// Optional private late-demo installation; it is not a retail fallback.
pub fn demo_dir() -> PathBuf {
    configured_dir("V2K_DEMO_DIR", "demo")
}

fn corpus_present(path: &Path) -> bool {
    if !path.try_exists().expect("inspect private corpus directory") {
        return false;
    }
    if !path.is_dir() {
        return true; // A file in place of the directory is a broken install.
    }
    std::fs::read_dir(path)
        .expect("read private corpus directory")
        .any(|entry| {
            let name = entry.expect("read private corpus entry").file_name();
            name != "README.md" && name != ".gitignore"
        })
}

/// Call from the build script of each crate containing corpus-backed tests.
/// Cargo re-runs this when either override or the corpus directory changes.
pub fn configure() {
    for (variable, cfg, path) in [
        ("V2K_RETAIL_DIR", "v2k_retail", retail_dir()),
        ("V2K_DEMO_DIR", "v2k_demo", demo_dir()),
    ] {
        println!("cargo:rerun-if-env-changed={variable}");
        println!("cargo:rustc-check-cfg=cfg({cfg})");
        println!("cargo:rerun-if-changed={}", path.display());
        // Tracked setup notes keep default watch directories present: watching
        // a nonexistent path makes Cargo rebuild every test on every invocation.
        // Check for any payload, not completeness: partial installs must fail.
        if corpus_present(&path) {
            println!("cargo:rustc-cfg={cfg}");
        } else {
            println!(
                "cargo:warning={cfg} data missing at {}; dependent tests are ignored ({variable} overrides this path)",
                path.display()
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn repository_root_is_independent_of_the_callers_working_directory() {
        let root = repository_root();
        assert!(root.join("Cargo.toml").is_file());
        assert!(root.join("crates/v2k-test-support").is_dir());
    }
}
