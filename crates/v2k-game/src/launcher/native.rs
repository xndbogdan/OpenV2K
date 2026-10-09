//! Native classic-control presentation. Long-running setup operations send
//! messages through a channel; only the UI thread touches window handles.
#![allow(unsafe_op_in_unsafe_fn)]

#[path = "win32.rs"]
mod win32;

#[path = "header_animation.rs"]
mod header_animation;
use header_animation::HeaderAnimation;

use std::cell::RefCell;
use std::ffi::{OsStr, OsString};
use std::mem::{size_of, zeroed};
use std::os::windows::ffi::{OsStrExt, OsStringExt};
use std::path::{Path, PathBuf};
use std::ptr::{null, null_mut};
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread::{self, JoinHandle};
use std::time::Instant;

use v2k_render::config::{
    DisplayModes, GameConfig, GraphicsDetail, RendererChoice, ScalingMode, WindowMode,
};
use v2k_render::music::SoundtrackAvailability;

use super::{LaunchSelection, LauncherRequest};
use crate::setup::{
    DiscImage, InstallRequest, InstallResult, IssueSeverity, SetupProgress, ValidationReport,
};
use win32::*;

const WM_CREATE: u32 = 0x0001;
const WM_DESTROY: u32 = 0x0002;
const WM_PAINT: u32 = 0x000f;
const WM_CLOSE: u32 = 0x0010;
const WM_COMMAND: u32 = 0x0111;
const WM_TIMER: u32 = 0x0113;
const WM_VSCROLL: u32 = 0x0115;
const SB_BOTTOM: usize = 7;
const WM_SETFONT: u32 = 0x0030;
const CB_RESETCONTENT: u32 = 0x014b;
const CBN_SELCHANGE: usize = 1;
const DPI_AWARENESS_CONTEXT_SYSTEM_AWARE: isize = -2;
const ENUM_CURRENT_SETTINGS: u32 = u32::MAX;
const MONITOR_DEFAULTTOPRIMARY: u32 = 1;
const DM_GETDEFID: u32 = 0x0400;
const WS_CHILD: u32 = 0x4000_0000;
const WS_VISIBLE: u32 = 0x1000_0000;
const WS_TABSTOP: u32 = 0x0001_0000;
const WINDOW_STYLE: u32 = 0x00c8_0000; // caption, system menu, fixed border
const EX_CONTROL_PARENT: u32 = 0x0001_0000;
const MAIN_CLASS: &str = "V2000PortLauncher";
const OPTIONS_CLASS: &str = "V2000PortLauncherOptions";
const ID_PLAY: usize = 101;
const ID_OPTIONS: usize = 102;
const ID_QUIT: usize = 103;
const ID_LOCATE: usize = 104;
const ID_INSTALL: usize = 105;
const ID_VERIFY: usize = 106;
const ID_EXTRACT: usize = 107;
const ID_SAVE: usize = 108;
const ID_CANCEL: usize = 109;
const ID_RESOLUTION: usize = 201;
const ID_RENDERER: usize = 202;
const ID_SCALING: usize = 203;
const ID_DETAIL: usize = 204;
const ID_DISPLAY: usize = 205;
const ID_MUSIC: usize = 207;
const ID_EFFECTS: usize = 208;
const ID_EFFECTS_VOLUME: usize = 209;
const ID_SKIP: usize = 210;
const WORKER_TIMER: usize = 1;
const HEADER_TIMER: usize = 2;

struct WindowHost(RefCell<WindowState>);

#[derive(Default)]
struct MainControls {
    installation_group: Handle,
    files_group: Handle,
    root: Handle,
    details: Handle,
    instruction: Handle,
    progress: Handle,
    play: Handle,
    options: Handle,
    quit: Handle,
    locate: Handle,
    install: Handle,
}

#[derive(Default)]
struct OptionControls {
    resolution: Handle,
    renderer: Handle,
    scaling: Handle,
    detail: Handle,
    display: Handle,
    music: Handle,
    music_availability: Handle,
    effects: Handle,
    effects_volume: Handle,
    skip: Handle,
    save: Handle,
    install: Handle,
    verify: Handle,
    extract: Handle,
    progress: Handle,
}

#[derive(Clone, Copy)]
enum SetupAction {
    ActivateInstallation,
    MusicOnly,
}

enum WorkerEvent {
    Progress(SetupProgress),
    Validated(ValidationReport, bool),
    Installed(InstallResult, SetupAction),
    Failure(String),
}

struct WindowState {
    request: LauncherRequest,
    window: Handle,
    options_window: Handle,
    host: *mut WindowHost,
    controls: MainControls,
    option_controls: OptionControls,
    font: Handle,
    header_animation: HeaderAnimation,
    header_last_tick: Instant,
    header_brush: Handle,
    scale: f64,
    root: PathBuf,
    report: Option<ValidationReport>,
    /// The pending choices while Options is open.
    option_config: Option<GameConfig>,
    /// What the primary display offers, read when Options opens.
    display_modes: DisplayModes,
    worker: Option<JoinHandle<()>>,
    sender: Sender<WorkerEvent>,
    receiver: Receiver<WorkerEvent>,
    busy: bool,
    worker_changes_files: bool,
    compact_layout: bool,
    close_requested: bool,
    selection: Option<LaunchSelection>,
    started: Instant,
}

fn wide(value: impl AsRef<OsStr>) -> Vec<u16> {
    value.as_ref().encode_wide().chain(Some(0)).collect()
}

unsafe fn text(window: Handle, value: &str) {
    SetWindowTextW(window, wide(value).as_ptr());
}

unsafe fn redraw_after_dialog(owner: Handle) {
    if owner != 0 {
        InvalidateRect(owner, null(), 1);
        let parent = GetWindow(owner, 4);
        if parent != 0 {
            InvalidateRect(parent, null(), 1);
        }
    }
}

unsafe fn notice(owner: Handle, title: &str, message: &str, flags: u32) -> i32 {
    let result = MessageBoxW(owner, wide(message).as_ptr(), wide(title).as_ptr(), flags);
    // Modal dialogs pump paint messages while the application-state borrow is
    // held. Queue a fresh paint once that borrow is released.
    redraw_after_dialog(owner);
    result
}

unsafe fn show_interactive_window(window: Handle) {
    // https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-showwindow
    // Windows honors STARTUPINFO's show mode on the first ShowWindow call,
    // even when SW_SHOW is requested. A host may request a hidden child process;
    // the launcher itself is an explicitly interactive window.
    ShowWindow(window as *mut std::ffi::c_void, 5);
    ShowWindow(window as *mut std::ffi::c_void, 5);
    UpdateWindow(window);
}

pub fn show_error(message: &str) {
    unsafe {
        notice(0, "V2K", message, 0x10);
    }
}

pub fn run(request: LauncherRequest) -> Result<Option<LaunchSelection>, String> {
    unsafe { run_native(request) }
}

pub(super) fn attach_parent_console() {
    unsafe {
        // A Windows GUI-subsystem executable receives no private console.
        // Attach only to a caller's console; Explorer normally has none.
        // https://learn.microsoft.com/en-us/windows/console/attachconsole
        let redirected = [(-10_i32) as u32, (-11_i32) as u32, (-12_i32) as u32].map(|identifier| {
            let handle = GetStdHandle(identifier);
            let kind = if handle.is_null() || handle as isize == -1 {
                0
            } else {
                GetFileType(handle) & 0xffff
            };
            (identifier, matches!(kind, 1 | 3).then_some(handle))
        });
        AttachConsole(u32::MAX); // ATTACH_PARENT_PROCESS; never AllocConsole.
                                 // AttachConsole may replace standard handles. Keep Command::output,
                                 // PowerShell pipelines and disk redirections connected to their owner.
        for (identifier, handle) in redirected {
            if let Some(handle) = handle {
                SetStdHandle(identifier, handle);
            }
        }
    }
}
unsafe fn run_native(request: LauncherRequest) -> Result<Option<LaunchSelection>, String> {
    let header_animation = HeaderAnimation::load()?;
    // The process is per-monitor DPI aware for the game; the launcher keeps
    // its system-DPI layout, scaled by `GetDpiForSystem` below.
    let dpi_context = SetThreadDpiAwarenessContext(DPI_AWARENESS_CONTEXT_SYSTEM_AWARE);
    let com_result = CoInitializeEx(null_mut(), 2);
    let dpi = GetDpiForSystem().max(96);
    let root = request
        .initial_root
        .clone()
        .or_else(|| request.executable.parent().map(Path::to_path_buf))
        .ok_or_else(|| "Cannot determine the executable directory".to_string())?;
    let (sender, receiver) = mpsc::channel();
    // Fit both windows on the current display at high DPI and on older small
    // displays. Every control and font uses the same physical-pixel scale.
    let scale = (f64::from(dpi) / 96.0)
        .min(f64::from((GetSystemMetrics(0) - 32).max(1)) / 660.0)
        .min(f64::from((GetSystemMetrics(1) - 80).max(1)) / 642.0);
    let font = CreateFontW(
        -(13.0 * scale).round() as i32,
        0,
        0,
        0,
        400,
        0,
        0,
        0,
        1,
        0,
        0,
        0,
        0,
        wide("MS Sans Serif").as_ptr(),
    );
    let host = Box::new(WindowHost(RefCell::new(WindowState {
        request,
        window: 0,
        options_window: 0,
        host: null_mut(),
        controls: MainControls::default(),
        option_controls: OptionControls::default(),
        font,
        header_animation,
        header_last_tick: Instant::now(),
        header_brush: CreateSolidBrush(0),
        scale,
        root,
        report: None,
        option_config: None,
        display_modes: DisplayModes::default(),
        worker: None,
        sender,
        receiver,
        busy: false,
        worker_changes_files: false,
        compact_layout: false,
        close_requested: false,
        selection: None,
        started: Instant::now(),
    })));
    let host_pointer = (&*host as *const WindowHost).cast_mut();
    host.0.borrow_mut().host = host_pointer;
    let result = (|| {
        register_class(MAIN_CLASS, main_procedure)?;
        register_class(OPTIONS_CLASS, options_procedure)?;
        let window = create_top_window(MAIN_CLASS, "V2K", 660, 503, 0, host_pointer, scale)?;
        {
            let mut state = host.0.borrow_mut();
            state.window = window;
            state.create_main_controls();
            state.verify(false);
        }
        show_interactive_window(window);

        host.0.borrow_mut().header_last_tick = Instant::now();
        SetTimer(window, WORKER_TIMER, 100, null());
        SetTimer(window, HEADER_TIMER, 20, null());
        let mut message = Message::default();
        loop {
            let status = GetMessageW(&mut message, 0, 0, 0);
            if status == 0 {
                break;
            }
            if status < 0 {
                return Err(format!("Windows message loop failed ({})", GetLastError()));
            }
            let options = host.0.borrow().options_window;
            let dialog = if options != 0 { options } else { window };
            if IsDialogMessageW(dialog, &mut message) == 0 {
                TranslateMessage(&message);
                DispatchMessageW(&message);
            }
        }
        Ok(host.0.borrow_mut().selection.take())
    })();
    let mut state = host.0.borrow_mut();
    if IsWindow(state.options_window) != 0 {
        DestroyWindow(state.options_window);
    }
    if IsWindow(state.window) != 0 {
        DestroyWindow(state.window);
    }
    if let Some(worker) = state.worker.take() {
        let _ = worker.join();
    }
    DeleteObject(state.font);
    DeleteObject(state.header_brush);
    if com_result >= 0 {
        CoUninitialize();
    }
    if dpi_context != 0 {
        SetThreadDpiAwarenessContext(dpi_context);
    }
    result
}

unsafe fn register_class(name: &str, procedure: WindowProc) -> Result<(), String> {
    let name = wide(name);
    let instance = GetModuleHandleW(null());
    let icon = LoadIconW(instance, wide("IDI_ICON1").as_ptr());
    let class = WindowClass {
        size: size_of::<WindowClass>() as u32,
        style: 3,
        procedure: Some(procedure),
        class_extra: 0,
        window_extra: 0,
        instance,
        icon,
        cursor: LoadCursorW(0, 32512usize as *const u16),
        background: 16,
        menu_name: null(),
        class_name: name.as_ptr(),
        // WNDCLASSEX hIconSm = NULL lets Windows select the small image from
        // hIcon's multiresolution resource, rather than scaling its large one.
        small_icon: 0,
    };
    if RegisterClassExW(&class) == 0 && GetLastError() != 1410 {
        Err(format!(
            "Cannot register launcher window ({})",
            GetLastError()
        ))
    } else {
        Ok(())
    }
}

unsafe fn create_top_window(
    class: &str,
    title: &str,
    width: i32,
    height: i32,
    owner: Handle,
    host: *mut WindowHost,
    scale: f64,
) -> Result<Handle, String> {
    let mut bounds = Rect {
        left: 0,
        top: 0,
        right: (f64::from(width) * scale).round() as i32,
        bottom: (f64::from(height) * scale).round() as i32,
    };
    AdjustWindowRectEx(&mut bounds, WINDOW_STYLE, 0, EX_CONTROL_PARENT);
    let width = bounds.right - bounds.left;
    let height = bounds.bottom - bounds.top;
    let window = CreateWindowExW(
        EX_CONTROL_PARENT,
        wide(class).as_ptr(),
        wide(title).as_ptr(),
        WINDOW_STYLE,
        (GetSystemMetrics(0) - width) / 2,
        (GetSystemMetrics(1) - height) / 2,
        width,
        height,
        owner,
        0,
        GetModuleHandleW(null()),
        host.cast(),
    );
    if window == 0 {
        Err(format!(
            "Cannot create launcher window ({})",
            GetLastError()
        ))
    } else {
        Ok(window)
    }
}

unsafe fn host_for_message(window: Handle, message: u32, lparam: isize) -> *mut WindowHost {
    if message == WM_CREATE {
        let create = &*(lparam as *const CreateStruct);
        SetWindowLongPtrW(window, -21, create.parameter as isize);
    }
    GetWindowLongPtrW(window, -21) as *mut WindowHost
}

unsafe extern "system" fn main_procedure(
    window: Handle,
    message: u32,
    wparam: usize,
    lparam: isize,
) -> isize {
    if message == WM_DESTROY {
        KillTimer(window, WORKER_TIMER);
        KillTimer(window, HEADER_TIMER);
        PostQuitMessage(0);
        return 0;
    }
    if message == DM_GETDEFID {
        return ((0x534b_u32 << 16) | ID_PLAY as u32) as isize;
    }
    let host = host_for_message(window, message, lparam);
    if !host.is_null() {
        // Native dialogs pump nested messages. Skipping mutation during that
        // nested dispatch avoids aliasing mutable application state; the timer
        // catches up immediately after the dialog returns.
        if let Ok(mut state) = (*host).0.try_borrow_mut() {
            match message {
                WM_COMMAND if wparam >> 16 == 0 => {
                    state.command(wparam & 0xffff);
                    return 0;
                }
                WM_TIMER if wparam == WORKER_TIMER => {
                    state.poll_worker();
                    return 0;
                }
                WM_TIMER if wparam == HEADER_TIMER => {
                    state.animate_header();
                    return 0;
                }
                WM_CLOSE => {
                    state.quit();
                    return 0;
                }
                WM_PAINT => {
                    state.paint_banner(window);
                    return 0;
                }
                _ => {}
            }
        }
    }
    DefWindowProcW(window, message, wparam, lparam)
}

unsafe extern "system" fn options_procedure(
    window: Handle,
    message: u32,
    wparam: usize,
    lparam: isize,
) -> isize {
    if message == DM_GETDEFID {
        return ((0x534b_u32 << 16) | ID_SAVE as u32) as isize;
    }
    let host = host_for_message(window, message, lparam);
    if !host.is_null() {
        if let Ok(mut state) = (*host).0.try_borrow_mut() {
            match message {
                WM_COMMAND if wparam >> 16 == 0 => {
                    state.command(wparam & 0xffff);
                    return 0;
                }
                WM_COMMAND if wparam >> 16 == CBN_SELCHANGE => {
                    state.display_choice_changed(wparam & 0xffff);
                    return 0;
                }
                WM_CLOSE => {
                    state.close_options();
                    return 0;
                }
                _ => {}
            }
        }
    }
    DefWindowProcW(window, message, wparam, lparam)
}

impl WindowState {
    fn pixel(&self, logical: i32) -> i32 {
        (f64::from(logical) * self.scale).round() as i32
    }

    unsafe fn control(
        &self,
        parent: Handle,
        class: &str,
        label: &str,
        style: u32,
        id: usize,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
    ) -> Handle {
        let child = CreateWindowExW(
            if class == "EDIT" { 0x200 } else { 0 },
            wide(class).as_ptr(),
            wide(label).as_ptr(),
            WS_CHILD | WS_VISIBLE | style,
            self.pixel(x),
            self.pixel(y),
            self.pixel(width),
            self.pixel(height),
            parent,
            id as Handle,
            GetModuleHandleW(null()),
            null_mut(),
        );
        let empty = [0u16];
        SetWindowTheme(child, empty.as_ptr(), empty.as_ptr());
        SendMessageW(child, WM_SETFONT, self.font as usize, 1);
        child
    }

    unsafe fn label(
        &self,
        parent: Handle,
        label: &str,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
    ) -> Handle {
        self.control(parent, "STATIC", label, 0, 0, x, y, width, height)
    }

    unsafe fn button(
        &self,
        parent: Handle,
        label: &str,
        id: usize,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
    ) -> Handle {
        self.control(
            parent,
            "BUTTON",
            label,
            WS_TABSTOP | u32::from(id == ID_PLAY || id == ID_SAVE),
            id,
            x,
            y,
            width,
            height,
        )
    }

    unsafe fn group(
        &self,
        parent: Handle,
        label: &str,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
    ) -> Handle {
        self.control(parent, "BUTTON", label, 7, 0, x, y, width, height)
    }

    unsafe fn checkbox(
        &self,
        parent: Handle,
        label: &str,
        id: usize,
        x: i32,
        y: i32,
        width: i32,
        checked: bool,
    ) -> Handle {
        let control = self.control(parent, "BUTTON", label, WS_TABSTOP | 3, id, x, y, width, 23);
        SendMessageW(control, 0x00f1, usize::from(checked), 0);
        control
    }

    unsafe fn combo(
        &self,
        parent: Handle,
        values: &[String],
        selection: usize,
        id: usize,
        x: i32,
        y: i32,
        width: i32,
    ) -> Handle {
        let control = self.control(
            parent,
            "COMBOBOX",
            "",
            WS_TABSTOP | 0x0020_0203,
            id,
            x,
            y,
            width,
            280,
        );
        for value in values {
            SendMessageW(control, 0x0143, 0, wide(value).as_ptr() as isize);
        }
        SendMessageW(control, 0x014e, selection as usize, 0);
        control
    }

    unsafe fn create_main_controls(&mut self) {
        let window = self.window;
        let installation_group = self.group(window, " Installation ", 16, 132, 628, 58);
        let root = self.control(
            window,
            "EDIT",
            &self.root.display().to_string(),
            WS_TABSTOP | 0x0880,
            0,
            26,
            155,
            608,
            22,
        );
        let files_group = self.group(window, " Game files and music ", 16, 200, 628, 121);
        let details = self.control(
            window,
            "EDIT",
            "Checking files...",
            WS_TABSTOP | 0x0020_0844,
            0,
            26,
            223,
            608,
            86,
        );
        let instruction = self.label(
            window,
            "Choose an installation or install from disc.",
            16,
            331,
            628,
            20,
        );
        let locate = self.button(
            window,
            "&Use existing installation...",
            ID_LOCATE,
            16,
            357,
            306,
            31,
        );
        let install = self.button(
            window,
            "&Install from disc image...",
            ID_INSTALL,
            338,
            357,
            306,
            31,
        );
        let progress = self.label(window, "", 16, 400, 628, 32);
        let play = self.button(window, "&Play", ID_PLAY, 16, 443, 192, 42);
        let options = self.button(window, "&Options...", ID_OPTIONS, 234, 443, 192, 42);
        let quit = self.button(window, "&Quit", ID_QUIT, 452, 443, 192, 42);
        self.controls = MainControls {
            installation_group,
            files_group,
            root,
            details,
            instruction,
            progress,
            play,
            options,
            quit,
            locate,
            install,
        };
        EnableWindow(play, 0);
    }

    fn is_executable_root(&self, root: &Path) -> bool {
        super::is_executable_installation(root, &self.request.executable)
    }

    fn can_play(&self) -> bool {
        self.report
            .as_ref()
            .is_some_and(|report| report.is_ready() && self.is_executable_root(&report.root))
    }

    unsafe fn move_control(&self, control: Handle, x: i32, y: i32, width: i32, height: i32) {
        MoveWindow(
            control,
            self.pixel(x),
            self.pixel(y),
            self.pixel(width),
            self.pixel(height),
            1,
        );
    }

    unsafe fn reflow_main(&mut self, compact: bool) {
        if compact == self.compact_layout {
            return;
        }
        self.compact_layout = compact;
        for control in [
            self.controls.installation_group,
            self.controls.root,
            self.controls.instruction,
            self.controls.locate,
            self.controls.install,
        ] {
            ShowWindow(
                control as *mut std::ffi::c_void,
                if compact { 0 } else { 5 },
            );
        }
        if compact {
            self.move_control(self.controls.files_group, 16, 132, 628, 111);
            self.move_control(self.controls.details, 26, 155, 608, 76);
            self.move_control(self.controls.progress, 16, 258, 628, 30);
        } else {
            self.move_control(self.controls.files_group, 16, 200, 628, 121);
            self.move_control(self.controls.details, 26, 223, 608, 86);
            self.move_control(self.controls.progress, 16, 400, 628, 32);
        }
        let button_y = if compact { 300 } else { 443 };
        self.move_control(self.controls.play, 16, button_y, 192, 42);
        self.move_control(self.controls.options, 234, button_y, 192, 42);
        self.move_control(self.controls.quit, 452, button_y, 192, 42);
        let mut bounds = Rect {
            left: 0,
            top: 0,
            right: self.pixel(660),
            bottom: self.pixel(if compact { 358 } else { 503 }),
        };
        AdjustWindowRectEx(&mut bounds, WINDOW_STYLE, 0, EX_CONTROL_PARENT);
        let width = bounds.right - bounds.left;
        let height = bounds.bottom - bounds.top;
        SetWindowPos(
            self.window,
            0,
            (GetSystemMetrics(0) - width) / 2,
            (GetSystemMetrics(1) - height) / 2,
            width,
            height,
            0x0014,
        );
        InvalidateRect(self.window, null(), 1);
    }

    fn header_bounds(&self) -> Rect {
        Rect {
            left: self.pixel(16),
            top: self.pixel(16),
            right: self.pixel(644),
            bottom: self.pixel(118),
        }
    }

    unsafe fn animate_header(&mut self) {
        let now = Instant::now();
        let elapsed_micros = now.duration_since(self.header_last_tick).as_micros() as u64;
        self.header_last_tick = now;
        if self.header_animation.advance(elapsed_micros) {
            // The opaque baked frame repaints only the banner, without erasing
            // the client background or invalidating any setup controls.
            InvalidateRect(self.window, &self.header_bounds(), 0);
        }
    }

    unsafe fn paint_banner(&self, window: Handle) {
        let mut paint: PaintStruct = zeroed();
        let dc = BeginPaint(window, &mut paint);
        let mut banner = self.header_bounds();
        let header_image = self.header_animation.frame();
        FillRect(dc, &banner, self.header_brush);
        let image_height = self.pixel(98);
        let image_width = image_height * header_image.width / header_image.height;
        let image_x = (banner.left + banner.right - image_width) / 2;
        let bitmap = BitmapInfo {
            header: BitmapInfoHeader {
                size: size_of::<BitmapInfoHeader>() as u32,
                width: header_image.width,
                height: -header_image.height, // top-down PNG rows
                planes: 1,
                bit_count: 32,
                ..BitmapInfoHeader::default()
            },
            colors: [0],
        };
        SetStretchBltMode(dc, 4); // HALFTONE preserves the downscaled glow.
        SetBrushOrgEx(dc, 0, 0, null_mut());
        StretchDIBits(
            dc,
            image_x,
            self.pixel(18),
            image_width,
            image_height,
            0,
            0,
            header_image.width,
            header_image.height,
            header_image.bgra.as_ptr().cast(),
            &bitmap,
            0,           // DIB_RGB_COLORS
            0x00cc_0020, // SRCCOPY
        );
        DrawEdge(dc, &mut banner, 0x000a, 0x000f);
        EndPaint(window, &paint);
    }

    unsafe fn command(&mut self, id: usize) {
        match id {
            2 => {
                if self.options_window != 0 {
                    self.close_options();
                } else {
                    self.quit();
                }
            }
            ID_PLAY if !self.busy && self.can_play() => {
                self.request.preferences.data_dir = Some(self.root.clone());
                if let Err(error) = self
                    .request
                    .preferences
                    .save(&self.request.preferences_path)
                {
                    notice(self.window, "V2K settings", &format!("The game can run, but the selected installation could not be remembered:\n\n{error}"), 0x30);
                }
                self.selection = Some(LaunchSelection::Play {
                    data_dir: self.root.clone(),
                });
                DestroyWindow(self.window);
            }
            ID_OPTIONS if !self.busy => self.open_options(),
            ID_QUIT => self.quit(),
            ID_CANCEL => self.close_options(),
            ID_SAVE if !self.busy => self.save_options(),
            ID_LOCATE if !self.busy && !self.can_play() => {
                self.close_options();
                self.choose_existing();
            }
            ID_INSTALL if !self.busy && !self.can_play() => {
                self.close_options();
                self.install_image();
            }
            ID_VERIFY if !self.busy => self.verify(true),
            ID_EXTRACT if !self.busy && self.can_extract_music() => self.extract_music(),
            _ => {}
        }
    }

    unsafe fn quit(&mut self) {
        if self.busy {
            self.close_requested = true;
            self.set_progress("Finishing the current operation before closing...");
        } else {
            DestroyWindow(self.window);
        }
    }

    fn can_extract_music(&self) -> bool {
        self.can_play()
            && self.report.as_ref().is_some_and(|report| {
                report.music.availability() != SoundtrackAvailability::Complete
            })
    }

    unsafe fn set_busy(&mut self, busy: bool) {
        self.busy = busy;
        let ready = self.can_play();
        EnableWindow(self.controls.play, i32::from(!busy && ready));
        if self.option_controls.save != 0 {
            EnableWindow(self.option_controls.save, i32::from(!busy && ready));
        }
        EnableWindow(self.controls.options, i32::from(!busy));
        for (control, available) in [
            (self.controls.locate, !ready),
            (self.controls.install, !ready),
            (self.option_controls.install, !ready),
            (self.option_controls.verify, true),
            (self.option_controls.extract, self.can_extract_music()),
        ] {
            if control != 0 {
                EnableWindow(control, i32::from(!busy && available));
            }
        }
        if self.option_controls.music_availability != 0 {
            let availability = self
                .report
                .as_ref()
                .map(music_description)
                .unwrap_or_else(|| "Music has not been checked.".to_string());
            text(self.option_controls.music_availability, &availability);
        }
    }

    unsafe fn set_progress(&self, message: &str) {
        text(self.controls.progress, message);
        if self.options_window != 0 {
            text(self.option_controls.progress, message);
        }
    }

    unsafe fn start_worker(
        &mut self,
        label: &str,
        changes_files: bool,
        job: impl FnOnce(Sender<WorkerEvent>) -> Result<WorkerEvent, String> + Send + 'static,
    ) {
        self.worker_changes_files = changes_files;
        self.set_busy(true);
        self.set_progress(label);
        let sender = self.sender.clone();
        self.worker = Some(thread::spawn(move || {
            let result =
                std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| job(sender.clone())));
            let event = match result {
                Ok(Ok(event)) => event,
                Ok(Err(error)) => WorkerEvent::Failure(error),
                Err(_) => WorkerEvent::Failure("The setup operation stopped unexpectedly. Your source image was not changed. Verify the destination before trying again.".to_string()),
            };
            let _ = sender.send(event);
        }));
    }

    unsafe fn verify(&mut self, show_result: bool) {
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

    unsafe fn apply_report(&mut self, report: ValidationReport) {
        self.root = report.root.clone();
        text(self.controls.root, &self.root.display().to_string());
        let ready = report.is_ready();
        let local = ready && self.is_executable_root(&report.root);
        text(
            self.controls.instruction,
            if local {
                ""
            } else {
                "Choose an installation or install from disc."
            },
        );
        text(self.controls.details, &report_description(&report));
        if local {
            self.request.preferences.data_dir = Some(self.root.clone());
            if let Err(error) = self
                .request
                .preferences
                .save(&self.request.preferences_path)
            {
                if !self.close_requested {
                    notice(self.dialog_owner(), "V2K launcher preferences", &format!(
                        "This installation is valid, but launcher preferences could not be saved. The game can still run.\n\n{}\n\n{error}",
                        self.request.preferences_path.display()), 0x30);
                }
            }
        }
        self.report = Some(report);
        self.reflow_main(local);
        // Scroll after the final control size establishes its wrapped lines.
        SendMessageW(self.controls.details, WM_VSCROLL, SB_BOTTOM, 0);
    }

    unsafe fn poll_worker(&mut self) {
        while let Ok(event) = self.receiver.try_recv() {
            match event {
                WorkerEvent::Progress(progress) => {
                    if !self.close_requested {
                        let count = if progress.total > 0 {
                            format!(" ({}/{})", progress.completed, progress.total)
                        } else {
                            String::new()
                        };
                        self.set_progress(&format!(
                            "{}{}: {}",
                            progress.phase, count, progress.message
                        ));
                    }
                }
                WorkerEvent::Validated(report, show_result) => {
                    let description = popup_description(&report);
                    let ready = report.is_ready();
                    self.apply_report(report);
                    self.finish_worker();
                    self.set_progress("");
                    if show_result && !self.close_requested {
                        notice(
                            self.dialog_owner(),
                            "V2K file verification",
                            &description,
                            if ready { 0x40 } else { 0x30 },
                        );
                    }
                    if self.can_play() {
                        SetFocus(self.controls.play);
                    }
                }
                WorkerEvent::Installed(result, action) => {
                    let mut summary = format!(
                        "Wrote {} files to {}.\n\n{}",
                        result.files_written,
                        result.root.display(),
                        popup_description(&result.report)
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
                    let restart = if matches!(action, SetupAction::ActivateInstallation)
                        && result.report.is_ready()
                        && !self.is_executable_root(&result.root)
                    {
                        self.request
                            .executable
                            .file_name()
                            .map(|filename| result.root.join(filename))
                    } else {
                        None
                    };
                    if restart.is_some() {
                        summary.push_str("\n\nThe launcher installed in this folder will now open. Saves will stay beside that executable.");
                    }
                    self.apply_report(result.report);
                    self.finish_worker();
                    self.set_progress("Setup complete.");
                    if !self.close_requested {
                        notice(self.dialog_owner(), "V2K setup complete", &summary, 0x40);
                        if let Some(executable) = restart {
                            self.close_options();
                            self.selection = Some(LaunchSelection::Restart { executable });
                            DestroyWindow(self.window);
                            return;
                        }
                    }
                }
                WorkerEvent::Failure(error) => {
                    let changes_files = self.worker_changes_files;
                    self.report = None;
                    self.finish_worker();
                    self.set_progress(
                        "Setup could not complete. Your source image was not changed.",
                    );
                    if !self.close_requested {
                        notice(self.dialog_owner(), "V2K setup", &error, 0x10);
                    }
                    if changes_files {
                        self.verify(false);
                    }
                }
            }
        }
        if self
            .request
            .smoke_timeout
            .is_some_and(|timeout| self.started.elapsed() >= timeout)
        {
            self.close_requested = true;
        }
        if self.close_requested && !self.busy {
            self.close_options();
            DestroyWindow(self.window);
        }
    }

    unsafe fn finish_worker(&mut self) {
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
        self.set_busy(false);
    }

    fn dialog_owner(&self) -> Handle {
        if self.options_window != 0 {
            self.options_window
        } else {
            self.window
        }
    }

    unsafe fn choose_existing(&mut self) {
        let initial = self.request.setup_hint.as_deref().unwrap_or(&self.root);
        let Some(destination) = choose_folder(self.window,
            "Select the V2K folder containing PRELOAD.DAT and Overlay. The launcher will be installed there.", Some(initial)) else { return; };
        if !self.is_executable_root(&destination) && notice(self.window, "Use V2K installation", &format!(
            "Copy this launcher into:\n{}\n\nGame files and saves will be kept in that folder. Existing game assets will be preserved. The installed launcher will open after validation. Continue?",
            destination.display()), 0x24) != 6 { return; }
        let executable = self.request.executable.clone();
        let dependencies = self.request.runtime_dependencies.clone();
        self.start_worker(
            "Checking the selected installation and copying the launcher...",
            true,
            move |sender| {
                let result = crate::setup::install_launcher(
                    &destination,
                    &executable,
                    &dependencies,
                    |progress| {
                        let _ = sender.send(WorkerEvent::Progress(progress));
                    },
                )?;
                Ok(WorkerEvent::Installed(
                    result,
                    SetupAction::ActivateInstallation,
                ))
            },
        );
    }

    unsafe fn select_image(&mut self) -> Option<PathBuf> {
        match choose_image(
            self.dialog_owner(),
            self.request.preferences.last_image.as_deref(),
        ) {
            Ok(Some(path)) => {
                self.request.preferences.last_image = Some(path.clone());
                if let Err(error) = self
                    .request
                    .preferences
                    .save(&self.request.preferences_path)
                {
                    self.set_progress(&format!("Could not remember the disc image: {error}"));
                }
                Some(path)
            }
            Ok(None) => None,
            Err(error) => {
                notice(self.dialog_owner(), "Select disc image", &error, 0x10);
                None
            }
        }
    }

    unsafe fn install_image(&mut self) {
        let Some(image_path) = self.select_image() else {
            return;
        };
        let Some(destination) = choose_folder(
            self.window,
            "Choose where to install V2K. You can create a new folder here.",
            Some(&self.root),
        ) else {
            return;
        };
        if notice(self.window, "Install V2K", &format!("Install V2K to:\n{}\n\nSource image:\n{}\n\nThe launcher, required runtime files, PRELOAD.DAT, Overlay, intro videos and available music will be copied. Existing replaced files will be backed up. A data-only image installs with music disabled.\n\nContinue?", destination.display(), image_path.display()), 0x24) != 6 { return; }
        let request = InstallRequest {
            destination,
            executable: Some(self.request.executable.clone()),
            runtime_dependencies: self.request.runtime_dependencies.clone(),
            extract_music: true,
        };
        self.start_worker("Opening disc image...", true, move |sender| {
            let image = DiscImage::open(&image_path)?;
            let result = image.install(&request, |progress| {
                let _ = sender.send(WorkerEvent::Progress(progress));
            })?;
            Ok(WorkerEvent::Installed(
                result,
                SetupAction::ActivateInstallation,
            ))
        });
    }

    unsafe fn extract_music(&mut self) {
        let Some(image_path) = self.select_image() else {
            return;
        };
        let root = self.root.clone();
        if notice(
            self.dialog_owner(),
            "Extract V2K music",
            &format!(
                "Extract the soundtrack from:\n{}\n\nDestination:\n{}\n\nExisting replaced tracks will be backed up. Track numbers will be preserved. Continue?",
                image_path.display(),
                root.join("cdaudio").display()
            ),
            0x24,
        ) != 6
        {
            return;
        }
        self.start_worker("Opening soundtrack source...", true, move |sender| {
            let image = DiscImage::open(&image_path)?;
            let result = image.extract_music(&root, |progress| {
                let _ = sender.send(WorkerEvent::Progress(progress));
            })?;
            Ok(WorkerEvent::Installed(result, SetupAction::MusicOnly))
        });
    }

    unsafe fn open_options(&mut self) {
        if self.options_window != 0 {
            SetFocus(self.options_window);
            return;
        }
        let window = match create_top_window(
            OPTIONS_CLASS,
            "V2K Options",
            560,
            642,
            self.window,
            self.host,
            self.scale,
        ) {
            Ok(window) => window,
            Err(error) => {
                notice(self.window, "V2K Options", &error, 0x10);
                return;
            }
        };
        self.options_window = window;
        let mut config = GameConfig::load(&self.root);
        self.display_modes = primary_display_modes();
        config.desktop = self.display_modes.desktop;
        self.group(window, " Display ", 12, 12, 536, 250);
        self.label(window, "&Display:", 24, 38, 106, 20);
        let display = self.combo(
            window,
            &WindowMode::ALL.map(|mode| mode.label().to_string()),
            config.display.index() as usize,
            ID_DISPLAY,
            136,
            34,
            194,
        );
        self.label(window, "&Resolution:", 24, 74, 106, 20);
        let resolution = self.combo(window, &[], 0, ID_RESOLUTION, 136, 70, 194);
        fill_resolutions(resolution, &self.display_modes, &config);
        self.label(window, "Renderer:", 24, 110, 106, 20);
        let renderer_index = match config.renderer {
            RendererChoice::Auto => 0,
            RendererChoice::OpenGL => 1,
            RendererChoice::Software => 2,
            RendererChoice::Wgpu => 0,
        };
        let renderer = self.combo(
            window,
            &strings(&["Automatic", "OpenGL", "Software"]),
            renderer_index,
            ID_RENDERER,
            136,
            106,
            194,
        );
        self.label(window, "Image scaling:", 24, 146, 106, 20);
        let scaling = self.combo(
            window,
            &strings(&["Native", "Preserve 4:3", "Stretched 4:3"]),
            config.scaling.index() as usize,
            ID_SCALING,
            136,
            142,
            194,
        );
        self.label(window, "Art detail:", 24, 182, 106, 20);
        let detail = self.combo(
            window,
            &strings(&["High (normal)", "Low (320 x 240)"]),
            usize::from(config.detail == GraphicsDetail::Low),
            ID_DETAIL,
            136,
            178,
            194,
        );
        self.label(
            window,
            "Display settings apply when Play starts the game.",
            24,
            220,
            504,
            24,
        );
        self.group(window, " Sound ", 12, 274, 536, 94);
        let music = self.checkbox(
            window,
            "&Music enabled",
            ID_MUSIC,
            24,
            296,
            188,
            config.ambient_enabled && config.music_volume > 0.0,
        );
        let effects = self.checkbox(
            window,
            "Sound &effects",
            ID_EFFECTS,
            24,
            330,
            188,
            config.sound_enabled,
        );
        self.label(window, "Effects volume:", 234, 335, 112, 20);
        let effects_volume = self.combo(
            window,
            &(0..=15).map(|n| n.to_string()).collect::<Vec<_>>(),
            (config.sfx_volume.clamp(0.0, 1.0) * 15.0).round() as usize,
            ID_EFFECTS_VOLUME,
            356,
            330,
            168,
        );
        let music_availability = self.label(window, "", 234, 298, 294, 28);
        let skip = self.checkbox(
            window,
            "Skip the launcher when this installation is ready",
            ID_SKIP,
            16,
            382,
            528,
            self.request.preferences.skip_launcher,
        );
        self.label(
            window,
            "Use --launcher to open it again. Missing game files still open setup.",
            34,
            408,
            510,
            20,
        );
        self.group(window, " Installation tools ", 12, 440, 536, 132);
        let verify = self.button(window, "&Verify files", ID_VERIFY, 24, 464, 248, 30);
        let extract = self.button(window, "E&xtract music...", ID_EXTRACT, 284, 464, 252, 30);
        let install = self.button(
            window,
            "&Install from disc image...",
            ID_INSTALL,
            24,
            506,
            512,
            30,
        );
        let progress = self.label(window, "", 24, 545, 512, 22);
        let save = self.button(window, "&Save", ID_SAVE, 308, 592, 112, 32);
        self.button(window, "Cancel", ID_CANCEL, 436, 592, 112, 32);
        self.option_controls = OptionControls {
            resolution,
            renderer,
            scaling,
            detail,
            display,
            music,
            music_availability,
            effects,
            effects_volume,
            skip,
            save,
            install,
            verify,
            extract,
            progress,
        };
        self.option_config = Some(config);
        self.set_busy(self.busy);
        if !self.can_play() {
            self.set_progress(
                "Install this launcher beside valid game files before saving settings.",
            );
        }
        EnableWindow(self.window, 0);
        show_interactive_window(window);

        SetFocus(self.option_controls.display);
    }

    /// Display or Resolution changed: update the pending choice. A new
    /// Display offers a new Resolution row.
    unsafe fn display_choice_changed(&mut self, id: usize) {
        let controls = &self.option_controls;
        let Some(config) = self.option_config.as_mut() else {
            return;
        };
        match id {
            ID_DISPLAY => {
                let mode = WindowMode::from_index(combo_selection(controls.display) as u32);
                config.select_window_mode(mode, &self.display_modes);
                fill_resolutions(controls.resolution, &self.display_modes, config);
            }
            ID_RESOLUTION => {
                config.select_resolution(combo_selection(controls.resolution), &self.display_modes);
            }
            _ => {}
        }
    }

    unsafe fn close_options(&mut self) {
        if self.options_window != 0 {
            DestroyWindow(self.options_window);
            self.options_window = 0;
            self.option_controls = OptionControls::default();
            self.option_config = None;
            EnableWindow(self.window, 1);
            SetFocus(self.controls.options);
        }
    }

    unsafe fn save_options(&mut self) {
        // Setup tools remain available in an empty directory, but gameplay
        // settings must never create an apparent installation in that folder.
        if !self.can_play() {
            return;
        }
        let Some(mut config) = self.option_config.clone() else {
            return;
        };
        let controls = &self.option_controls;
        // Display and Resolution already hold their pending choices.
        config.renderer = match combo_selection(controls.renderer) {
            1 => RendererChoice::OpenGL,
            2 => RendererChoice::Software,
            _ => RendererChoice::Auto,
        };
        config.scaling = ScalingMode::from_index(combo_selection(controls.scaling) as u32);
        config.detail = if combo_selection(controls.detail) == 1 {
            GraphicsDetail::Low
        } else {
            GraphicsDetail::High
        };
        config.ambient_enabled = checked(controls.music);
        config.music_volume = if config.ambient_enabled {
            if config.music_volume > 0.0 {
                config.music_volume
            } else {
                1.0
            }
        } else {
            0.0
        };
        config.sound_enabled = checked(controls.effects);
        config.sfx_volume = combo_selection(controls.effects_volume).min(15) as f32 / 15.0;
        let skip = checked(controls.skip);
        if let Err(error) = config.try_save(&self.root) {
            notice(
                self.options_window,
                "Save V2K options",
                &format!("Could not save game settings:\n\n{error}"),
                0x10,
            );
            return;
        }
        self.request.preferences.skip_launcher = skip;
        if let Err(error) = self
            .request
            .preferences
            .save(&self.request.preferences_path)
        {
            notice(self.options_window, "Save launcher options", &format!("Game settings were saved, but the launcher preference could not be saved:\n\n{error}"), 0x10);
            return;
        }
        self.close_options();
        self.set_progress("Options saved.");
    }
}

/// The Resolution row for `config`'s display, with its entry selected.
/// Borderless shows the desktop it covers and can't be changed.
unsafe fn fill_resolutions(control: Handle, modes: &DisplayModes, config: &GameConfig) {
    SendMessageW(control, CB_RESETCONTENT, 0, 0);
    for (width, height) in modes.resolutions(config.display) {
        SendMessageW(
            control,
            0x0143,
            0,
            wide(format!("{width} x {height}")).as_ptr() as isize,
        );
    }
    if let Some(index) = modes.selection(config.display, config.size()) {
        SendMessageW(control, 0x014e, index, 0);
    }
    EnableWindow(control, i32::from(config.display != WindowMode::Borderless));
}

/// What the primary display offers; the game opens there. The same list as
/// SDL's display 0: its 15- to 32-bit modes as sizes, and the desktop mode.
/// `DEVMODE` sizes are physical pixels whatever the thread's DPI awareness.
unsafe fn primary_display_modes() -> DisplayModes {
    let mut monitor: MonitorInfo = zeroed();
    monitor.size = size_of::<MonitorInfo>() as u32;
    let primary = MonitorFromPoint(Point::default(), MONITOR_DEFAULTTOPRIMARY);
    if GetMonitorInfoW(primary, &mut monitor) == 0 {
        return DisplayModes::default();
    }
    let device = monitor.device.as_ptr();
    let mode = |index: u32| {
        let mut settings: DevMode = zeroed();
        settings.size = size_of::<DevMode>() as u16;
        (EnumDisplaySettingsW(device, index, &mut settings) != 0).then_some(settings)
    };
    let desktop =
        mode(ENUM_CURRENT_SETTINGS).map(|settings| (settings.pels_width, settings.pels_height));
    let reported = (0..)
        .map_while(mode)
        .filter(|settings| matches!(settings.bits_per_pel, 15 | 16 | 24 | 32))
        .map(|settings| (settings.pels_width, settings.pels_height));
    DisplayModes::new(desktop, reported)
}

fn strings(values: &[&str]) -> Vec<String> {
    values.iter().map(|value| (*value).to_string()).collect()
}

fn music_description(report: &ValidationReport) -> String {
    let count = report.music.available_count();
    if report.music.directory_missing {
        "Music directory missing.".to_string()
    } else if count == 0 {
        "Music disabled: no soundtrack tracks are available.".to_string()
    } else if report.music.missing_tracks().is_empty() {
        "Music available: all 10 CD audio tracks.".to_string()
    } else {
        format!("Music incomplete: {count} of 10 tracks available.")
    }
}

fn report_description(report: &ValidationReport) -> String {
    let mut lines = vec![format!(
        "Game files: {}. Checked {} overlays.",
        if report.is_ready() {
            "ready"
        } else {
            "missing or inconsistent"
        },
        report.checked_overlays
    )];
    // Missing-directory guidance is already a path-bearing validation warning.
    if !report.music.directory_missing {
        lines.push(music_description(report));
    }
    if !report.music.missing_tracks().is_empty() && report.music.available_count() > 0 {
        lines.push(format!(
            "Missing music tracks: {}.",
            report
                .music
                .missing_tracks()
                .iter()
                .map(|track| format!("{track:02}"))
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }
    if report.embedded_checksums_verified {
        lines.push("Bundled retail checksums verified.".to_string());
    } else if report.manifest_verified {
        lines.push("Installation checksum manifest verified.".to_string());
    }
    for issue in &report.issues {
        // One absent-directory warning also covers optional manifest tracks.
        if report.music.directory_missing
            && issue.severity == IssueSeverity::Warning
            && issue.path != report.music.directory
            && issue.path.starts_with(&report.music.directory)
        {
            continue;
        }
        lines.push(format!("{}\r\n{}", issue.path.display(), issue.message));
    }
    for error in &report.music.scan_errors {
        lines.push(error.clone());
    }
    for track in &report.music.tracks {
        for error in &track.errors {
            lines.push(format!("Music track {:02}: {error}", track.cd_track));
        }
    }
    lines.join("\r\n")
}

/// A completely empty directory has hundreds of diagnostics. Keep the native
/// popup within the screen; the main read-only, scrollable control retains all.
fn popup_description(report: &ValidationReport) -> String {
    let description = report_description(report);
    let lines: Vec<_> = description.lines().collect();
    if lines.len() <= 12 {
        description
    } else {
        format!(
            "{}\r\n\r\n{} additional details are available in the main window's scrollable list.",
            lines[..12].join("\r\n"),
            lines.len() - 12
        )
    }
}

unsafe fn combo_selection(control: Handle) -> usize {
    SendMessageW(control, 0x0147, 0, 0).max(0) as usize
}
unsafe fn checked(control: Handle) -> bool {
    SendMessageW(control, 0x00f0, 0, 0) == 1
}

unsafe fn choose_image(owner: Handle, previous: Option<&Path>) -> Result<Option<PathBuf>, String> {
    let mut buffer = vec![0u16; 32768];
    let default_name = wide(previous.unwrap_or_else(|| Path::new("V2000.bin")));
    let length = default_name.len().min(buffer.len());
    buffer[..length].copy_from_slice(&default_name[..length]);
    let filter: Vec<u16> = "V2000 disc images (*.bin;*.cue;*.iso)\0*.bin;*.cue;*.iso\0BIN disc image (*.bin)\0*.bin\0CUE track sheet (*.cue)\0*.cue\0ISO data image (*.iso)\0*.iso\0\0".encode_utf16().collect();
    let title = wide("V2K - Select disc image (BIN, CUE or ISO)");
    let extension = wide("bin");
    let mut request: OpenFileName = zeroed();
    request.size = size_of::<OpenFileName>() as u32;
    request.owner = owner;
    request.filter = filter.as_ptr();
    request.filter_index = 1;
    request.file = buffer.as_mut_ptr();
    request.max_file = buffer.len() as u32;
    request.title = title.as_ptr();
    request.default_extension = extension.as_ptr();
    request.flags = 0x0008_180c; // explorer, file/path must exist, hide readonly, no cwd changes
    let selected = GetOpenFileNameW(&mut request);
    redraw_after_dialog(owner);
    if selected != 0 {
        let length = buffer
            .iter()
            .position(|&value| value == 0)
            .unwrap_or(buffer.len());
        Ok(Some(PathBuf::from(OsString::from_wide(&buffer[..length]))))
    } else {
        let error = CommDlgExtendedError();
        if error == 0 {
            Ok(None)
        } else {
            Err(format!(
                "The file chooser failed (Windows error {error:#x})."
            ))
        }
    }
}

unsafe extern "system" fn browse_callback(
    window: Handle,
    message: u32,
    _lparam: isize,
    initial: isize,
) -> i32 {
    if message == 1 && initial != 0 {
        SendMessageW(window, 0x0467, 1, initial);
    }
    0
}

unsafe fn choose_folder(owner: Handle, prompt: &str, initial: Option<&Path>) -> Option<PathBuf> {
    let title = wide(prompt);
    let initial = initial.map(wide);
    let mut display_name = vec![0u16; 32768];
    let mut info = BrowseInfo {
        owner,
        root: null(),
        display_name: display_name.as_mut_ptr(),
        title: title.as_ptr(),
        flags: 0x0051,
        callback: Some(browse_callback),
        parameter: initial.as_ref().map_or(0, |value| value.as_ptr() as isize),
        image: 0,
    };
    let id = SHBrowseForFolderW(&mut info);
    redraw_after_dialog(owner);
    if id.is_null() {
        return None;
    }
    let mut path = vec![0u16; 32768];
    let success = SHGetPathFromIDListEx(id, path.as_mut_ptr(), path.len() as u32, 0);
    CoTaskMemFree(id);
    if success == 0 {
        return None;
    }
    let length = path
        .iter()
        .position(|&value| value == 0)
        .unwrap_or(path.len());
    Some(PathBuf::from(OsString::from_wide(&path[..length])))
}
