//! The launcher on Linux: the Windows launcher's installation check, Play,
//! Options, setup tools and Quit in an egui window (OpenGL through glow, on
//! X11 or Wayland). The rows and their rules are the Windows launcher's.
//! Linux plays any valid installation, as its launcher-less start did, so
//! setup never copies the executable: a disc image installs game data only.
//! Checks and installs run on a worker thread, as on Windows.

use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;
use std::time::{Duration, Instant};

use eframe::egui;
use v2k_render::config::{
    ControllerLayout, DisplayModes, GameConfig, GraphicsDetail, RendererChoice, ScalingMode,
    WindowMode,
};
use v2k_render::music::SoundtrackAvailability;

use super::header_animation::HeaderAnimation;
use super::{
    music_description, popup_description, report_description, LaunchSelection, LauncherRequest,
};
use crate::setup::{DiscImage, InstallRequest, InstallResult, SetupProgress, ValidationReport};

const TITLE: &str = "V2K";
const RENDERERS: [&str; 3] = ["Automatic", "OpenGL", "Software"];
const SCALINGS: [&str; 3] = ["Native", "Preserve 4:3", "Stretched 4:3"];
const DETAILS: [&str; 2] = ["High (normal)", "Low (320 x 240)"];

#[cfg(test)]
#[path = "linux_tests.rs"]
mod tests;

pub fn show_error(message: &str) {
    let _ = sdl2::messagebox::show_simple_message_box(
        sdl2::messagebox::MessageBoxFlag::ERROR,
        TITLE,
        message,
        None,
    );
}

pub fn run(request: LauncherRequest) -> Result<Option<LaunchSelection>, String> {
    let header = HeaderAnimation::load()?;
    // SDL reads the modes before the window opens, so the two never share
    // the display connection's start.
    let modes = primary_display_modes();
    let selection = Rc::new(RefCell::new(None));
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title(TITLE)
            .with_app_id("openv2k")
            .with_inner_size([620.0, 520.0])
            .with_min_inner_size([540.0, 420.0]),
        renderer: eframe::Renderer::Glow,
        // Wayland's swap waits for a frame callback, which a hidden or
        // blanked window never gets: the check and close would stall.
        vsync: false,
        ..Default::default()
    };
    let root = initial_root(&request);
    let shared = selection.clone();
    let opened = eframe::run_native(
        TITLE,
        options,
        Box::new(move |cc| {
            cc.egui_ctx.set_visuals(egui::Visuals::dark());
            Ok(Box::new(Launcher::new(
                request, header, modes, shared, None,
            )))
        }),
    );
    if let Err(error) = opened {
        // No window (no display server): start as the launcher-less build
        // did, when the installation is ready.
        eprintln!("The launcher window cannot open: {error}");
        let report = crate::setup::validate_installation(&root);
        return if report.is_ready() {
            Ok(Some(LaunchSelection::Play {
                data_dir: report.root,
            }))
        } else {
            Err(report_description(&report).replace("\r\n", "\n"))
        };
    }
    Ok(selection.take())
}

/// The installation the window opens with: the discovered one, else the
/// setup hint, else the executable's folder.
fn initial_root(request: &LauncherRequest) -> PathBuf {
    request
        .initial_root
        .clone()
        .or_else(|| request.setup_hint.clone())
        .or_else(|| request.executable.parent().map(Path::to_path_buf))
        .unwrap_or_else(|| PathBuf::from("."))
}

/// What the primary display offers, as the game's SDL start reads it.
fn primary_display_modes() -> DisplayModes {
    let Ok(video) = sdl2::init().and_then(|sdl| sdl.video()) else {
        return DisplayModes::default();
    };
    let size = |mode: sdl2::video::DisplayMode| (mode.w as u32, mode.h as u32);
    let desktop = video.desktop_display_mode(0).ok().map(size);
    let count = video.num_display_modes(0).unwrap_or(0);
    DisplayModes::new(
        desktop,
        (0..count).filter_map(|index| video.display_mode(0, index).ok().map(size)),
    )
}

enum WorkerEvent {
    Progress(SetupProgress),
    Validated(ValidationReport, bool),
    Installed(InstallResult),
    Failure(String),
}

/// A message with OK, or a question with Continue and Cancel.
struct Notice {
    title: String,
    text: String,
    confirm: Option<Confirm>,
}

/// What Continue does.
enum Confirm {
    Install {
        image: PathBuf,
        destination: PathBuf,
    },
    ExtractMusic {
        image: PathBuf,
    },
}

/// Options' rows while open, as entry indices like the Windows combos.
struct Pending {
    config: GameConfig,
    renderer: usize,
    scaling: usize,
    detail: usize,
    music: bool,
    effects: bool,
    effects_volume: u32,
    controller: usize,
    fix_beam: bool,
    skip: bool,
}

struct Launcher {
    request: LauncherRequest,
    root: PathBuf,
    root_text: String,
    report: Option<ValidationReport>,
    details: String,
    progress: String,
    worker: Option<Receiver<WorkerEvent>>,
    worker_changes_files: bool,
    notice: Option<Notice>,
    options: Option<Pending>,
    modes: DisplayModes,
    header: HeaderAnimation,
    banner: Option<egui::TextureHandle>,
    header_tick: Instant,
    started: Instant,
    close_requested: bool,
    selection: Rc<RefCell<Option<LaunchSelection>>>,
    /// A test's own settings folder; the game's settings otherwise live in
    /// the installation.
    settings_dir: Option<PathBuf>,
    /// `V2K_LAUNCHER_SNAPSHOT`: a PNG of the launcher's own window once the
    /// check is done (`V2K_LAUNCHER_SNAPSHOT_PAGE=options` shows Options).
    snapshot: Option<PathBuf>,
    snapshot_frames: u32,
}

impl Launcher {
    fn new(
        request: LauncherRequest,
        header: HeaderAnimation,
        modes: DisplayModes,
        selection: Rc<RefCell<Option<LaunchSelection>>>,
        settings_dir: Option<PathBuf>,
    ) -> Self {
        let root = initial_root(&request);
        let mut launcher = Self {
            root_text: root.display().to_string(),
            root,
            request,
            report: None,
            details: "Checking files...".to_string(),
            progress: String::new(),
            worker: None,
            worker_changes_files: false,
            notice: None,
            options: None,
            modes,
            header,
            banner: None,
            header_tick: Instant::now(),
            started: Instant::now(),
            close_requested: false,
            selection,
            settings_dir,
            snapshot: std::env::var_os("V2K_LAUNCHER_SNAPSHOT").map(PathBuf::from),
            snapshot_frames: 0,
        };
        launcher.verify(false);
        launcher
    }

    fn busy(&self) -> bool {
        self.worker.is_some()
    }

    fn can_play(&self) -> bool {
        !self.busy() && self.report.as_ref().is_some_and(ValidationReport::is_ready)
    }

    fn can_extract_music(&self) -> bool {
        self.can_play()
            && self.report.as_ref().is_some_and(|report| {
                report.music.availability() != SoundtrackAvailability::Complete
            })
    }

    fn start_worker(
        &mut self,
        label: &str,
        changes_files: bool,
        job: impl FnOnce(Sender<WorkerEvent>) -> Result<WorkerEvent, String> + Send + 'static,
    ) {
        self.worker_changes_files = changes_files;
        self.progress = label.to_string();
        let (sender, receiver) = mpsc::channel();
        thread::spawn(move || {
            let result =
                std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| job(sender.clone())));
            let event = match result {
                Ok(Ok(event)) => event,
                Ok(Err(error)) => WorkerEvent::Failure(error),
                Err(_) => WorkerEvent::Failure("The setup operation stopped unexpectedly. Your source image was not changed. Verify the destination before trying again.".to_string()),
            };
            let _ = sender.send(event);
        });
        self.worker = Some(receiver);
    }

    fn verify(&mut self, show_result: bool) {
        let root = self.root.clone();
        self.start_worker(
            "Checking PRELOAD.DAT, every overlay and the soundtrack...",
            false,
            move |_| {
                Ok(WorkerEvent::Validated(
                    crate::setup::validate_installation(&root),
                    show_result,
                ))
            },
        );
    }

    fn apply_report(&mut self, report: ValidationReport) {
        self.root = report.root.clone();
        self.root_text = self.root.display().to_string();
        self.details = report_description(&report).replace("\r\n", "\n");
        self.report = Some(report);
    }

    fn poll_worker(&mut self) {
        let Some(receiver) = &self.worker else {
            return;
        };
        let mut events = Vec::new();
        let mut finished = false;
        while let Ok(event) = receiver.try_recv() {
            finished = !matches!(event, WorkerEvent::Progress(_));
            events.push(event);
        }
        if finished {
            self.worker = None;
        }
        for event in events {
            match event {
                WorkerEvent::Progress(progress) => {
                    let count = if progress.total > 0 {
                        format!(" ({}/{})", progress.completed, progress.total)
                    } else {
                        String::new()
                    };
                    self.progress = format!("{}{}: {}", progress.phase, count, progress.message);
                }
                WorkerEvent::Validated(report, show_result) => {
                    let description = popup_description(&report).replace("\r\n", "\n");
                    let ready = report.is_ready();
                    self.apply_report(report);
                    self.progress = if ready {
                        String::new()
                    } else {
                        "Choose an installation or install from disc.".to_string()
                    };
                    if show_result && !self.close_requested {
                        self.notice("V2K file verification", &description);
                    }
                }
                WorkerEvent::Installed(result) => {
                    let mut summary = format!(
                        "Wrote {} files to {}.\n\n{}",
                        result.files_written,
                        result.root.display(),
                        popup_description(&result.report).replace("\r\n", "\n")
                    );
                    if let Some(backup) = &result.backup_directory {
                        summary.push_str(&format!(
                            "\n\nPrevious files were preserved in {}.",
                            backup.display()
                        ));
                    }
                    if !result.warnings.is_empty() {
                        summary.push_str(&format!("\n\n{}", result.warnings.join("\n")));
                    }
                    self.apply_report(result.report);
                    self.progress = "Setup complete.".to_string();
                    if !self.close_requested {
                        self.notice("V2K setup complete", &summary);
                    }
                }
                WorkerEvent::Failure(error) => {
                    self.report = None;
                    self.progress =
                        "Setup could not complete. Your source image was not changed.".to_string();
                    if !self.close_requested {
                        self.notice("V2K setup", &error);
                    }
                    if self.worker_changes_files {
                        self.verify(false);
                    }
                }
            }
        }
    }

    fn notice(&mut self, title: &str, text: &str) {
        self.notice = Some(Notice {
            title: title.to_string(),
            text: text.to_string(),
            confirm: None,
        });
    }

    fn confirm(&mut self, title: &str, text: String, confirm: Confirm) {
        self.notice = Some(Notice {
            title: title.to_string(),
            text,
            confirm: Some(confirm),
        });
    }

    fn change_installation(&mut self, root: PathBuf) {
        self.root = root;
        self.root_text = self.root.display().to_string();
        self.details = "Checking files...".to_string();
        self.report = None;
        self.verify(false);
    }

    fn choose_installation(&mut self) {
        // The xdg desktop portal's folder chooser; without a portal it
        // returns nothing and the path field still takes a typed folder.
        if let Some(root) = rfd::FileDialog::new()
            .set_title("Select the V2K folder containing PRELOAD.DAT and Overlay.")
            .set_directory(&self.root)
            .pick_folder()
        {
            self.change_installation(root);
        }
    }

    fn select_image(&mut self) -> Option<PathBuf> {
        let mut dialog = rfd::FileDialog::new()
            .set_title("V2K - Select disc image (BIN, CUE or ISO)")
            .add_filter("Disc images", &["bin", "cue", "iso", "BIN", "CUE", "ISO"]);
        if let Some(folder) = self
            .request
            .preferences
            .last_image
            .as_deref()
            .and_then(Path::parent)
        {
            dialog = dialog.set_directory(folder);
        }
        let path = dialog.pick_file()?;
        self.request.preferences.last_image = Some(path.clone());
        if let Err(error) = self
            .request
            .preferences
            .save(&self.request.preferences_path)
        {
            self.progress = format!("Could not remember the disc image: {error}");
        }
        Some(path)
    }

    fn ask_install(&mut self) {
        let Some(image) = self.select_image() else {
            return;
        };
        let Some(destination) = rfd::FileDialog::new()
            .set_title("Choose where to install V2K. You can create a new folder here.")
            .set_directory(&self.root)
            .pick_folder()
        else {
            return;
        };
        let text = format!(
            "Install V2K to:\n{}\n\nSource image:\n{}\n\nPRELOAD.DAT, Overlay, intro videos and available music will be copied. Existing replaced files will be backed up. A data-only image installs with music disabled.\n\nContinue?",
            destination.display(),
            image.display()
        );
        self.confirm("Install V2K", text, Confirm::Install { image, destination });
    }

    fn install(&mut self, image: PathBuf, destination: PathBuf) {
        self.options = None;
        let request = InstallRequest {
            destination,
            executable: None,
            runtime_dependencies: Vec::new(),
            extract_music: true,
        };
        self.start_worker("Opening disc image...", true, move |sender| {
            let image = DiscImage::open(&image)?;
            let result = image.install(&request, |progress| {
                let _ = sender.send(WorkerEvent::Progress(progress));
            })?;
            Ok(WorkerEvent::Installed(result))
        });
    }

    fn ask_extract_music(&mut self) {
        let Some(image) = self.select_image() else {
            return;
        };
        let text = format!(
            "Extract the soundtrack from:\n{}\n\nDestination:\n{}\n\nExisting replaced tracks will be backed up. Track numbers will be preserved. Continue?",
            image.display(),
            self.root.join("cdaudio").display()
        );
        self.confirm("Extract V2K music", text, Confirm::ExtractMusic { image });
    }

    fn extract_music(&mut self, image: PathBuf) {
        let root = self.root.clone();
        self.start_worker("Opening soundtrack source...", true, move |sender| {
            let image = DiscImage::open(&image)?;
            let result = image.extract_music(&root, |progress| {
                let _ = sender.send(WorkerEvent::Progress(progress));
            })?;
            Ok(WorkerEvent::Installed(result))
        });
    }

    fn play(&mut self) {
        self.request.preferences.data_dir = Some(self.root.clone());
        if let Err(error) = self
            .request
            .preferences
            .save(&self.request.preferences_path)
        {
            eprintln!(
                "The game can start, but the launcher could not remember this installation: {error}"
            );
        }
        *self.selection.borrow_mut() = Some(LaunchSelection::Play {
            data_dir: self.root.clone(),
        });
        self.close_requested = true;
    }

    fn load_config(&self) -> GameConfig {
        match &self.settings_dir {
            Some(dir) => GameConfig::load_portable(dir),
            None => GameConfig::load(&self.root),
        }
    }

    fn open_options(&mut self) {
        let mut config = self.load_config();
        config.desktop = self.modes.desktop;
        self.options = Some(Pending {
            renderer: match config.renderer {
                RendererChoice::OpenGL => 1,
                RendererChoice::Software => 2,
                RendererChoice::Auto | RendererChoice::Wgpu => 0,
            },
            scaling: config.scaling.index() as usize,
            detail: usize::from(config.detail == GraphicsDetail::Low),
            music: config.ambient_enabled && config.music_volume > 0.0,
            effects: config.sound_enabled,
            effects_volume: (config.sfx_volume.clamp(0.0, 1.0) * 15.0).round() as u32,
            controller: config.controller_layout.index() as usize,
            fix_beam: config.fixes.beam_commands,
            skip: self.request.preferences.skip_launcher,
            config,
        });
    }

    fn save_options(&mut self) {
        // Setup tools stay available in an empty folder, but gameplay
        // settings must never make an apparent installation there.
        if !self.can_play() {
            return;
        }
        let Some(pending) = self.options.as_mut() else {
            return;
        };
        let mut config = pending.config.clone();
        // Display and Resolution already hold their pending choices.
        config.renderer = match pending.renderer {
            1 => RendererChoice::OpenGL,
            2 => RendererChoice::Software,
            _ => RendererChoice::Auto,
        };
        config.scaling = ScalingMode::from_index(pending.scaling as u32);
        config.detail = if pending.detail == 1 {
            GraphicsDetail::Low
        } else {
            GraphicsDetail::High
        };
        config.ambient_enabled = pending.music;
        config.music_volume = if !pending.music {
            0.0
        } else if config.music_volume > 0.0 {
            config.music_volume
        } else {
            1.0
        };
        config.sound_enabled = pending.effects;
        config.sfx_volume = pending.effects_volume.min(15) as f32 / 15.0;
        config.controller_layout = ControllerLayout::from_index(pending.controller as u32);
        config.fixes.beam_commands = pending.fix_beam;
        let skip = pending.skip;
        let dir = self
            .settings_dir
            .clone()
            .unwrap_or_else(|| self.root.clone());
        if let Err(error) = config.try_save(&dir) {
            return self.notice(
                "Save V2K options",
                &format!("The game's settings could not be saved:\n\n{error}"),
            );
        }
        self.request.preferences.skip_launcher = skip;
        if let Err(error) = self
            .request
            .preferences
            .save(&self.request.preferences_path)
        {
            return self.notice("Save launcher options", &format!("The game's settings were saved, but the launcher's preference could not be:\n\n{error}"));
        }
        self.options = None;
        self.progress = "Options saved.".to_string();
    }

    fn banner(&mut self, ctx: &egui::Context) -> egui::TextureHandle {
        let elapsed = self.header_tick.elapsed();
        self.header_tick = Instant::now();
        let changed = self.header.advance(elapsed.as_micros() as u64);
        let frame = self.header.frame();
        let image = || {
            let rgba: Vec<u8> = frame
                .bgra
                .chunks_exact(4)
                .flat_map(|p| [p[2], p[1], p[0], 255])
                .collect();
            egui::ColorImage::from_rgba_unmultiplied(
                [frame.width as usize, frame.height as usize],
                &rgba,
            )
        };
        let banner = self.banner.get_or_insert_with(|| {
            ctx.load_texture("banner", image(), egui::TextureOptions::LINEAR)
        });
        if changed {
            banner.set(image(), egui::TextureOptions::LINEAR);
        }
        banner.clone()
    }

    fn main_page(&mut self, ui: &mut egui::Ui) {
        group(ui, |ui| {
            ui.label("Installation");
            ui.horizontal(|ui| {
                let field = ui.add_enabled(
                    !self.busy(),
                    egui::TextEdit::singleline(&mut self.root_text)
                        .desired_width(ui.available_width() - 100.0),
                );
                if field.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                    self.change_installation(PathBuf::from(self.root_text.trim()));
                }
                if ui
                    .add_enabled(!self.busy(), egui::Button::new("Change..."))
                    .clicked()
                {
                    self.choose_installation();
                }
            });
        });
        group(ui, |ui| {
            ui.label("Game files");
            egui::ScrollArea::vertical()
                .max_height(150.0)
                .auto_shrink([false, true])
                .stick_to_bottom(true)
                .show(ui, |ui| ui.label(&self.details));
        });
        ui.label(&self.progress);
    }

    fn main_buttons(&mut self, ui: &mut egui::Ui) {
        let size = egui::vec2((ui.available_width() - 16.0) / 3.0, 40.0);
        let ready = self.can_play();
        let idle = !self.busy();
        ui.horizontal(|ui| {
            if ready || !idle {
                if ui
                    .add_enabled(ready, egui::Button::new("Play").min_size(size))
                    .clicked()
                {
                    self.play();
                }
            } else if ui
                .add(egui::Button::new("Install from disc image...").min_size(size))
                .clicked()
            {
                self.ask_install();
            }
            if ui
                .add_enabled(idle, egui::Button::new("Options...").min_size(size))
                .clicked()
            {
                self.open_options();
            }
            if ui.add(egui::Button::new("Quit").min_size(size)).clicked() {
                self.close_requested = true;
            }
        });
    }

    fn options_page(&mut self, ui: &mut egui::Ui) {
        let progress = self.progress.clone();
        let music_text = self
            .report
            .as_ref()
            .map(music_description)
            .unwrap_or_else(|| "Music has not been checked.".to_string());
        let idle = !self.busy();
        let ready = self.can_play();
        let can_extract = self.can_extract_music();
        let modes = &self.modes;
        let Some(p) = self.options.as_mut() else {
            return;
        };
        let mut action = None;
        egui::ScrollArea::vertical().show(ui, |ui| {
            group(ui, |ui| {
                ui.strong("Display");
                egui::Grid::new("display").num_columns(2).show(ui, |ui| {
                    let mut mode = p.config.display.index() as usize;
                    ui.label("Display:");
                    let labels = WindowMode::ALL.map(WindowMode::label);
                    if combo(ui, "Display", "display", &mut mode, &labels) {
                        p.config
                            .select_window_mode(WindowMode::from_index(mode as u32), modes);
                    }
                    ui.end_row();
                    ui.label("Resolution:");
                    let sizes = modes.resolutions(p.config.display);
                    let labels: Vec<String> =
                        sizes.iter().map(|(w, h)| format!("{w} x {h}")).collect();
                    let labels: Vec<&str> = labels.iter().map(String::as_str).collect();
                    let mut index = modes
                        .selection(p.config.display, p.config.size())
                        .unwrap_or(usize::MAX);
                    let enabled = p.config.display != WindowMode::Borderless && !sizes.is_empty();
                    if ui
                        .add_enabled_ui(enabled, |ui| {
                            combo(ui, "Resolution", "resolution", &mut index, &labels)
                        })
                        .inner
                    {
                        p.config.select_resolution(index, modes);
                    }
                    ui.end_row();
                    ui.label("Renderer:");
                    combo(ui, "Renderer", "renderer", &mut p.renderer, &RENDERERS);
                    ui.end_row();
                    ui.label("Image scaling:");
                    combo(ui, "Image scaling", "scaling", &mut p.scaling, &SCALINGS);
                    ui.end_row();
                    ui.label("Art detail:");
                    combo(ui, "Art detail", "detail", &mut p.detail, &DETAILS);
                    ui.end_row();
                });
                ui.weak("Display settings apply when Play starts the game.");
            });
            group(ui, |ui| {
                ui.strong("Sound");
                ui.checkbox(&mut p.music, "Music enabled");
                ui.weak(&music_text);
                ui.checkbox(&mut p.effects, "Sound effects");
                ui.horizontal(|ui| {
                    let label = ui.label("Effects volume:");
                    ui.add(egui::Slider::new(&mut p.effects_volume, 0..=15))
                        .labelled_by(label.id);
                });
            });
            group(ui, |ui| {
                ui.strong("Controls");
                egui::Grid::new("controls").num_columns(2).show(ui, |ui| {
                    ui.label("Controller layout:");
                    let labels = ControllerLayout::ALL.map(ControllerLayout::label);
                    combo(
                        ui,
                        "Controller layout",
                        "controller",
                        &mut p.controller,
                        &labels,
                    );
                    ui.end_row();
                });
                ui.weak("Gamepads only. Flight sticks always use the PC original.");
            });
            group(ui, |ui| {
                ui.strong("Fixes for retail bugs (off restores retail)");
                ui.checkbox(
                    &mut p.fix_beam,
                    "The C and D beam commands work at high frame rates",
                );
            });
            ui.checkbox(
                &mut p.skip,
                "Skip the launcher when this installation is ready",
            );
            ui.weak("Start with --launcher to open it again. Missing game files still open it.");
            group(ui, |ui| {
                ui.strong("Installation tools");
                ui.horizontal(|ui| {
                    if ui
                        .add_enabled(idle, egui::Button::new("Verify files"))
                        .clicked()
                    {
                        action = Some(ToolAction::Verify);
                    }
                    if ui
                        .add_enabled(idle && can_extract, egui::Button::new("Extract music..."))
                        .clicked()
                    {
                        action = Some(ToolAction::ExtractMusic);
                    }
                    if ui
                        .add_enabled(
                            idle && !ready,
                            egui::Button::new("Install from disc image..."),
                        )
                        .clicked()
                    {
                        action = Some(ToolAction::Install);
                    }
                });
            });
            ui.label(&progress);
        });
        match action {
            Some(ToolAction::Verify) => self.verify(true),
            Some(ToolAction::ExtractMusic) => self.ask_extract_music(),
            Some(ToolAction::Install) => self.ask_install(),
            None => {}
        }
    }

    fn options_buttons(&mut self, ui: &mut egui::Ui) {
        let save = !self.busy() && self.can_play();
        let mut close = None;
        // A row first: a bare right-to-left layout would fill the panel's height.
        ui.horizontal(|ui| {
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("Cancel").clicked() {
                    close = Some(false);
                }
                if ui.add_enabled(save, egui::Button::new("Save")).clicked() {
                    close = Some(true);
                }
            });
        });
        match close {
            Some(true) => self.save_options(),
            Some(false) => self.options = None,
            None => {}
        }
    }

    fn take_snapshot(&mut self, ctx: &egui::Context) {
        let Some(path) = self.snapshot.clone() else {
            return;
        };
        let shot = ctx.input(|input| {
            input.raw.events.iter().find_map(|event| match event {
                egui::Event::Screenshot { image, .. } => Some(image.clone()),
                _ => None,
            })
        });
        if let Some(image) = shot {
            let [width, height] = image.size;
            let rgba: Vec<u8> = image
                .pixels
                .iter()
                .flat_map(|p| p.to_srgba_unmultiplied())
                .collect();
            let saved = std::fs::File::create(&path)
                .map_err(|e| e.to_string())
                .and_then(|file| {
                    let mut encoder = png::Encoder::new(
                        std::io::BufWriter::new(file),
                        width as u32,
                        height as u32,
                    );
                    encoder.set_color(png::ColorType::Rgba);
                    encoder.set_depth(png::BitDepth::Eight);
                    let mut writer = encoder.write_header().map_err(|e| e.to_string())?;
                    writer.write_image_data(&rgba).map_err(|e| e.to_string())
                });
            eprintln!("launcher snapshot {}: {saved:?}", path.display());
            self.snapshot = None;
            return;
        }
        if self.report.is_none() || self.busy() {
            return;
        }
        if self.snapshot_frames == 0
            && self.options.is_none()
            && std::env::var("V2K_LAUNCHER_SNAPSHOT_PAGE").as_deref() == Ok("options")
        {
            self.open_options();
        }
        self.snapshot_frames += 1;
        if self.snapshot_frames == 30 {
            ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(Default::default()));
        }
    }
}

enum ToolAction {
    Verify,
    ExtractMusic,
    Install,
}

/// A titled box across the page's whole width.
fn group(ui: &mut egui::Ui, contents: impl FnOnce(&mut egui::Ui)) {
    ui.group(|ui| {
        ui.set_width(ui.available_width());
        contents(ui);
    });
}

/// One drop-down row; true when the choice changed.
fn combo(ui: &mut egui::Ui, name: &str, id: &str, selected: &mut usize, entries: &[&str]) -> bool {
    let before = *selected;
    let shown = egui::ComboBox::from_id_salt(id)
        .width(220.0)
        .selected_text(entries.get(*selected).copied().unwrap_or(""))
        .show_ui(ui, |ui| {
            for (index, entry) in entries.iter().enumerate() {
                ui.selectable_value(selected, index, *entry);
            }
        });
    // Its row's label as the accessible name, for screen readers and tests.
    let enabled = ui.is_enabled();
    shown
        .response
        .widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::ComboBox, enabled, name));
    *selected != before
}

impl eframe::App for Launcher {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.frame(ctx);
    }
}

impl Launcher {
    /// One frame of the window: eframe's update, and the click tests' step.
    fn frame(&mut self, ctx: &egui::Context) {
        self.poll_worker();
        if self
            .request
            .smoke_timeout
            .is_some_and(|timeout| self.started.elapsed() >= timeout)
        {
            self.close_requested = true;
        }
        if ctx.input(|i| i.viewport().close_requested()) && self.busy() {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            self.close_requested = true;
        }
        if self.close_requested {
            if self.busy() {
                self.progress = "Finishing the current operation before closing...".to_string();
            } else {
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
        }
        if self.notice.is_none() && ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            if self.options.is_some() {
                self.options = None;
            } else {
                self.close_requested = true;
            }
        }
        let banner = self.banner(ctx);
        egui::TopBottomPanel::top("banner")
            .frame(egui::Frame::NONE.fill(egui::Color32::BLACK))
            .show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    let width = ui.available_width().min(banner.size_vec2().x);
                    let size = egui::vec2(width, width * banner.aspect_ratio().recip());
                    ui.add(egui::Image::new(&banner).fit_to_exact_size(size));
                });
            });
        egui::TopBottomPanel::bottom("buttons").show(ctx, |ui| {
            ui.add_space(6.0);
            ui.add_enabled_ui(self.notice.is_none(), |ui| {
                if self.options.is_some() {
                    self.options_buttons(ui);
                } else {
                    self.main_buttons(ui);
                }
            });
            ui.add_space(6.0);
        });
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.add_enabled_ui(self.notice.is_none(), |ui| {
                if self.options.is_some() {
                    self.options_page(ui);
                } else {
                    self.main_page(ui);
                }
            });
        });
        if let Some(notice) = &self.notice {
            let mut answer = None;
            egui::Modal::new(egui::Id::new("notice")).show(ctx, |ui| {
                ui.set_max_width(480.0);
                ui.heading(&notice.title);
                ui.label(&notice.text);
                ui.horizontal(|ui| {
                    if notice.confirm.is_some() {
                        if ui.button("Continue").clicked() {
                            answer = Some(true);
                        }
                        if ui.button("Cancel").clicked() {
                            answer = Some(false);
                        }
                    } else if ui.button("OK").clicked()
                        || ui.input(|i| i.key_pressed(egui::Key::Enter))
                    {
                        answer = Some(true);
                    }
                });
            });
            if let Some(answer) = answer {
                let confirm = self.notice.take().and_then(|notice| notice.confirm);
                match confirm.filter(|_| answer) {
                    Some(Confirm::Install { image, destination }) => {
                        self.install(image, destination)
                    }
                    Some(Confirm::ExtractMusic { image }) => self.extract_music(image),
                    None => {}
                }
            }
        }
        self.take_snapshot(ctx);
        // The banner's flipbook: one baked frame a 20 ms tick.
        ctx.request_repaint_after(Duration::from_millis(20));
    }
}
