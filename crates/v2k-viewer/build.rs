use std::path::PathBuf;
use std::process::Command;

fn main() {
    v2k_test_support::configure();
    embed_windows_icon();
}

fn embed_windows_icon() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let icon = manifest_dir.join("../v2k-render/assets/openv2k.ico");
    println!("cargo:rerun-if-changed={}", icon.display());
    if std::env::var("CARGO_CFG_TARGET_OS").ok().as_deref() != Some("windows") {
        return;
    }
    if !icon.is_file() {
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
    if std::fs::write(&rc_path, format!("IDI_ICON1 ICON \"{icon_rc}\"\n")).is_err() {
        return;
    }
    let Ok(status) = Command::new("windres")
        .args([
            "-O",
            "coff",
            "-i",
            rc_path.to_str().unwrap_or(""),
            "-o",
            obj_path.to_str().unwrap_or(""),
        ])
        .status()
    else {
        println!("cargo:warning=windres unavailable; workbench EXE keeps the default icon");
        return;
    };
    if status.success() {
        println!("cargo:rustc-link-arg-bins={}", obj_path.display());
    } else {
        println!("cargo:warning=windres failed; workbench EXE keeps the default icon");
    }
}
