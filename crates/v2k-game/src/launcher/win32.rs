//! Minimal Win32 declarations for the launcher. Handle ownership remains in
//! `native`: windows/controls, fonts and brushes have one explicit lifetime.

#![allow(non_snake_case, dead_code)]

use std::ffi::c_void;

pub type Handle = isize;
pub type WindowProc = unsafe extern "system" fn(Handle, u32, usize, isize) -> isize;

#[repr(C)]
#[derive(Default, Clone, Copy)]
pub struct Point {
    pub x: i32,
    pub y: i32,
}

#[repr(C)]
#[derive(Default, Clone, Copy)]
pub struct Rect {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

#[repr(C)]
pub struct WindowClass {
    pub size: u32,
    pub style: u32,
    pub procedure: Option<WindowProc>,
    pub class_extra: i32,
    pub window_extra: i32,
    pub instance: Handle,
    pub icon: Handle,
    pub cursor: Handle,
    pub background: Handle,
    pub menu_name: *const u16,
    pub class_name: *const u16,
    pub small_icon: Handle,
}

#[repr(C)]
#[derive(Default)]
pub struct Message {
    pub window: Handle,
    pub message: u32,
    pub wparam: usize,
    pub lparam: isize,
    pub time: u32,
    pub point: Point,
    pub private: u32,
}

#[repr(C)]
pub struct CreateStruct {
    pub parameter: *mut c_void,
    pub instance: Handle,
    pub menu: Handle,
    pub parent: Handle,
    pub height: i32,
    pub width: i32,
    pub y: i32,
    pub x: i32,
    pub style: i32,
    pub name: *const u16,
    pub class: *const u16,
    pub extended_style: u32,
}

#[repr(C)]
pub struct PaintStruct {
    pub dc: Handle,
    pub erase: i32,
    pub paint: Rect,
    pub restore: i32,
    pub increment: i32,
    pub reserved: [u8; 32],
}

#[repr(C)]
#[derive(Default)]
pub struct BitmapInfoHeader {
    pub size: u32,
    pub width: i32,
    pub height: i32,
    pub planes: u16,
    pub bit_count: u16,
    pub compression: u32,
    pub image_size: u32,
    pub x_pixels_per_meter: i32,
    pub y_pixels_per_meter: i32,
    pub colors_used: u32,
    pub colors_important: u32,
}

#[repr(C)]
pub struct BitmapInfo {
    pub header: BitmapInfoHeader,
    pub colors: [u32; 1],
}

#[repr(C)]
pub struct OpenFileName {
    pub size: u32,
    pub owner: Handle,
    pub instance: Handle,
    pub filter: *const u16,
    pub custom_filter: *mut u16,
    pub max_custom_filter: u32,
    pub filter_index: u32,
    pub file: *mut u16,
    pub max_file: u32,
    pub file_title: *mut u16,
    pub max_file_title: u32,
    pub initial_directory: *const u16,
    pub title: *const u16,
    pub flags: u32,
    pub file_offset: u16,
    pub file_extension: u16,
    pub default_extension: *const u16,
    pub custom_data: isize,
    pub hook: *const c_void,
    pub template_name: *const u16,
    pub reserved: *mut c_void,
    pub reserved_count: u32,
    pub extended_flags: u32,
}

pub type BrowseCallback = unsafe extern "system" fn(Handle, u32, isize, isize) -> i32;

#[repr(C)]
pub struct BrowseInfo {
    pub owner: Handle,
    pub root: *const c_void,
    pub display_name: *mut u16,
    pub title: *const u16,
    pub flags: u32,
    pub callback: Option<BrowseCallback>,
    pub parameter: isize,
    pub image: i32,
}

#[link(name = "user32")]
unsafe extern "system" {
    pub fn RegisterClassExW(class: *const WindowClass) -> u16;
    pub fn CreateWindowExW(
        extended: u32,
        class: *const u16,
        title: *const u16,
        style: u32,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
        parent: Handle,
        menu: Handle,
        instance: Handle,
        parameter: *mut c_void,
    ) -> Handle;
    pub fn DefWindowProcW(window: Handle, message: u32, wparam: usize, lparam: isize) -> isize;
    pub fn DestroyWindow(window: Handle) -> i32;
    pub fn ShowWindow(window: *mut c_void, command: i32) -> i32;
    pub fn UpdateWindow(window: Handle) -> i32;
    pub fn GetMessageW(message: *mut Message, window: Handle, minimum: u32, maximum: u32) -> i32;
    pub fn TranslateMessage(message: *const Message) -> i32;
    pub fn DispatchMessageW(message: *const Message) -> isize;
    pub fn IsDialogMessageW(window: Handle, message: *mut Message) -> i32;
    pub fn PostQuitMessage(code: i32);
    pub fn SetWindowLongPtrW(window: Handle, index: i32, value: isize) -> isize;
    pub fn GetWindowLongPtrW(window: Handle, index: i32) -> isize;
    pub fn SetWindowTextW(window: Handle, text: *const u16) -> i32;
    pub fn EnableWindow(window: Handle, enabled: i32) -> i32;
    pub fn IsWindow(window: Handle) -> i32;
    pub fn MoveWindow(window: Handle, x: i32, y: i32, width: i32, height: i32, repaint: i32)
        -> i32;
    pub fn SetWindowPos(
        window: Handle,
        after: Handle,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
        flags: u32,
    ) -> i32;
    pub fn GetWindow(window: Handle, command: u32) -> Handle;
    pub fn InvalidateRect(window: Handle, rect: *const Rect, erase: i32) -> i32;
    pub fn SetFocus(window: Handle) -> Handle;
    pub fn SendMessageW(window: Handle, message: u32, wparam: usize, lparam: isize) -> isize;
    pub fn SetTimer(window: Handle, id: usize, milliseconds: u32, callback: *const c_void)
        -> usize;
    pub fn KillTimer(window: Handle, id: usize) -> i32;
    pub fn MessageBoxW(owner: Handle, text: *const u16, title: *const u16, flags: u32) -> i32;
    pub fn LoadCursorW(instance: Handle, name: *const u16) -> Handle;
    pub fn LoadIconW(instance: Handle, name: *const u16) -> Handle;
    pub fn AdjustWindowRectEx(rect: *mut Rect, style: u32, menu: i32, extended: u32) -> i32;
    pub fn GetSystemMetrics(index: i32) -> i32;
    pub fn GetDpiForSystem() -> u32;
    pub fn SetProcessDPIAware() -> i32;
    pub fn BeginPaint(window: Handle, paint: *mut PaintStruct) -> Handle;
    pub fn EndPaint(window: Handle, paint: *const PaintStruct) -> i32;
    pub fn FillRect(dc: Handle, rect: *const Rect, brush: Handle) -> i32;
    pub fn DrawEdge(dc: Handle, rect: *mut Rect, edge: u32, flags: u32) -> i32;
}

#[link(name = "kernel32")]
unsafe extern "system" {
    pub fn GetModuleHandleW(name: *const u16) -> Handle;
    pub fn GetLastError() -> u32;
    pub fn AttachConsole(process: u32) -> i32;
    pub fn GetStdHandle(identifier: u32) -> *mut c_void;
    pub fn SetStdHandle(identifier: u32, handle: *mut c_void) -> i32;
    pub fn GetFileType(handle: *mut c_void) -> u32;
}

#[link(name = "gdi32")]
unsafe extern "system" {
    pub fn CreateFontW(
        height: i32,
        width: i32,
        angle: i32,
        orientation: i32,
        weight: i32,
        italic: u32,
        underline: u32,
        strike: u32,
        charset: u32,
        output: u32,
        clipping: u32,
        quality: u32,
        family: u32,
        face: *const u16,
    ) -> Handle;
    pub fn StretchDIBits(
        dc: Handle,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
        source_x: i32,
        source_y: i32,
        source_width: i32,
        source_height: i32,
        pixels: *const c_void,
        info: *const BitmapInfo,
        usage: u32,
        operation: u32,
    ) -> i32;
    pub fn SetStretchBltMode(dc: Handle, mode: i32) -> i32;
    pub fn SetBrushOrgEx(dc: Handle, x: i32, y: i32, previous: *mut Point) -> i32;
    pub fn CreateSolidBrush(color: u32) -> Handle;
    pub fn DeleteObject(object: Handle) -> i32;
}

#[link(name = "comdlg32")]
unsafe extern "system" {
    pub fn GetOpenFileNameW(file: *mut OpenFileName) -> i32;
    pub fn CommDlgExtendedError() -> u32;
}

#[link(name = "shell32")]
unsafe extern "system" {
    pub fn SHBrowseForFolderW(info: *mut BrowseInfo) -> *mut c_void;
    pub fn SHGetPathFromIDListEx(id: *const c_void, path: *mut u16, length: u32, flags: u32)
        -> i32;
}

#[link(name = "ole32")]
unsafe extern "system" {
    pub fn CoInitializeEx(reserved: *mut c_void, mode: u32) -> i32;
    pub fn CoUninitialize();
    pub fn CoTaskMemFree(memory: *mut c_void);
}

#[link(name = "uxtheme")]
unsafe extern "system" {
    pub fn SetWindowTheme(window: Handle, app: *const u16, classes: *const u16) -> i32;
}
