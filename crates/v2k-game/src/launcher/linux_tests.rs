//! Clicks the Linux launcher's controls headlessly (egui_kittest). The
//! desktop file and folder choosers (the xdg portal) are left out.

use super::*;
use crate::launcher_preferences::LauncherPreferences;
use egui::accesskit::Role;
use egui_kittest::kittest::Queryable;
use egui_kittest::Harness;
use std::fs;

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("v2k-launcher-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

/// The launcher on `root`, its settings and preferences in `dir`, after its
/// first file check. Tall enough that Options never scrolls.
fn launcher(dir: &Path, root: PathBuf) -> Harness<'static, Launcher> {
    let request = LauncherRequest {
        initial_root: Some(root),
        setup_hint: None,
        preferences: LauncherPreferences::default(),
        preferences_path: dir.join("launcher.json"),
        executable: dir.join("v2k-game"),
        runtime_dependencies: Vec::new(),
        smoke_timeout: None,
    };
    let modes = DisplayModes::new(Some((1920, 1080)), [(640, 480), (1024, 768), (1920, 1080)]);
    let launcher = Launcher::new(
        request,
        HeaderAnimation::load().unwrap(),
        modes,
        Rc::new(RefCell::new(None)),
        Some(dir.to_path_buf()),
    );
    let mut harness = Harness::builder()
        .with_size(egui::vec2(620.0, 2000.0))
        .build_state(|ctx, launcher: &mut Launcher| launcher.frame(ctx), launcher);
    settle(&mut harness);
    harness
}

/// Steps until the file check's worker is done.
fn settle(harness: &mut Harness<'_, Launcher>) {
    for _ in 0..6000 {
        harness.step();
        if !harness.state().busy() {
            harness.step();
            return;
        }
        thread::sleep(Duration::from_millis(10));
    }
    panic!("the file check never finished");
}

fn click(harness: &mut Harness<'_, Launcher>, label: &str) {
    harness.get_by_label(label).click();
    harness.step();
    harness.step();
}

fn open_combo(harness: &mut Harness<'_, Launcher>, name: &str) {
    harness.get_by_role_and_label(Role::ComboBox, name).click();
    harness.step();
    harness.step();
}

#[test]
fn an_empty_folder_offers_setup_instead_of_play() {
    let dir = scratch("empty");
    let empty = scratch("empty-root");
    let mut harness = launcher(&dir, empty);
    assert!(!harness.state().can_play());
    assert!(harness.query_by_label("Play").is_none());
    assert!(harness
        .query_by_label("Install from disc image...")
        .is_some());
    assert!(harness.state().details.contains("missing or inconsistent"));
    click(&mut harness, "Options...");
    assert!(harness.state().options.is_some());
    // Settings never make an apparent installation in an empty folder.
    click(&mut harness, "Save");
    assert!(harness.state().options.is_some());
    assert_eq!(fs::read_dir(&dir).unwrap().count(), 0);
    click(&mut harness, "Cancel");
    assert!(harness.state().options.is_none());
}

#[test]
fn quit_closes_without_a_selection() {
    let dir = scratch("quit");
    let mut harness = launcher(&dir, scratch("quit-root"));
    click(&mut harness, "Quit");
    assert!(harness.state().close_requested);
    assert!(harness.state().selection.borrow().is_none());
}

#[test]
fn display_rows_follow_the_windows_rules() {
    let dir = scratch("display");
    let mut harness = launcher(&dir, scratch("display-root"));
    click(&mut harness, "Options...");
    open_combo(&mut harness, "Display");
    click(&mut harness, "Borderless");
    let config = &harness.state().options.as_ref().unwrap().config;
    // Borderless shows the desktop it covers and keeps the window's size.
    assert_eq!(config.display, WindowMode::Borderless);
    assert_eq!(config.resolution(), (1920, 1080));
    assert_eq!(config.size(), (800, 600));
    open_combo(&mut harness, "Display");
    click(&mut harness, "Full Screen");
    open_combo(&mut harness, "Resolution");
    click(&mut harness, "1024 x 768");
    let config = &harness.state().options.as_ref().unwrap().config;
    assert_eq!(config.display, WindowMode::FullScreen);
    assert_eq!(config.size(), (1024, 768));
}

#[v2k_test_support::retail_test]
fn play_returns_the_installation_and_remembers_it() {
    let dir = scratch("play");
    let root = v2k_test_support::retail_dir();
    let mut harness = launcher(&dir, root.clone());
    assert!(harness.state().can_play(), "{}", harness.state().details);
    click(&mut harness, "Play");
    let state = harness.state();
    assert!(state.close_requested);
    assert!(matches!(
        state.selection.borrow().as_ref(),
        Some(LaunchSelection::Play { data_dir }) if *data_dir == root
    ));
    let saved = LauncherPreferences::load(&dir.join("launcher.json"));
    assert_eq!(saved.data_dir, Some(root));
}

#[v2k_test_support::retail_test]
fn options_save_writes_the_chosen_rows() {
    let dir = scratch("save");
    let mut harness = launcher(&dir, v2k_test_support::retail_dir());
    click(&mut harness, "Options...");
    open_combo(&mut harness, "Renderer");
    click(&mut harness, "Software");
    open_combo(&mut harness, "Art detail");
    click(&mut harness, "Low (320 x 240)");
    click(&mut harness, "Beam commands at any frame rate");
    click(
        &mut harness,
        "Skip the launcher when this installation is ready",
    );
    click(&mut harness, "Save");
    assert!(harness.state().options.is_none());
    assert_eq!(harness.state().progress, "Options saved.");
    let saved = GameConfig::load_portable(&dir);
    assert_eq!(saved.renderer, RendererChoice::Software);
    assert_eq!(saved.detail, GraphicsDetail::Low);
    assert!(!saved.fixes.beam_commands);
    assert!(LauncherPreferences::load(&dir.join("launcher.json")).skip_launcher);
}
