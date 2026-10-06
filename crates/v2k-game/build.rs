//! Embed the authored OpenV2K icon as `IDI_ICON1` in the Windows game binary.

use std::path::PathBuf;
use std::process::Command;

fn main() {
    v2k_test_support::configure();
    let manifest_dir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    let icon = manifest_dir.join("../v2k-render/assets/openv2k.ico");
    println!("cargo:rerun-if-changed={}", icon.display());

    if std::env::var("CARGO_CFG_TARGET_OS").ok().as_deref() != Some("windows") {
        return;
    }
    if !icon.is_file() {
        println!("cargo:warning=missing OpenV2K icon at {}", icon.display());
        return;
    }

    let out_dir = PathBuf::from(std::env::var("OUT_DIR").unwrap());
    let rc_path = out_dir.join("openv2k_icon.rc");
    let obj_path = out_dir.join("openv2k_icon.o");
    let icon_rc = icon
        .canonicalize()
        .unwrap_or(icon)
        .to_string_lossy()
        .replace('\\', "/");
    std::fs::write(&rc_path, format!("IDI_ICON1 ICON \"{icon_rc}\"\n")).expect("write icon.rc");

    let status = Command::new("windres")
        .args([
            "-O",
            "coff",
            "-i",
            rc_path.to_str().expect("utf-8 rc path"),
            "-o",
            obj_path.to_str().expect("utf-8 obj path"),
        ])
        .status();
    match status {
        Ok(status) if status.success() => {
            println!("cargo:rustc-link-arg-bins={}", obj_path.display());
        }
        Ok(status) => {
            println!("cargo:warning=windres failed ({status}); the game EXE keeps the default icon")
        }
        Err(error) => println!(
            "cargo:warning=windres unavailable ({error}); the game EXE keeps the default icon"
        ),
    }
}
