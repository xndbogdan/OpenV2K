//! Read-only retail installation hints. A hint is never proof of valid data.
use std::path::PathBuf;

pub fn registry_candidates() -> Vec<PathBuf> {
    let mut candidates = Vec::new();
    #[cfg(windows)]
    candidates.extend(windows::installation_paths());
    // Some retail installs keep saves beside game data, but Save Path can be
    // completely independent. The caller validates every candidate normally.
    if let Some(path) = v2k_render::config::retail_save_directory() {
        if !candidates.contains(&path) {
            candidates.push(path);
        }
    }
    candidates
}

#[cfg(windows)]
mod windows {
    use super::*;
    use std::ffi::{c_void, OsString};
    use std::os::windows::ffi::OsStringExt;
    use std::ptr;
    type Hkey = *mut c_void;
    const HKCU: Hkey = -2_147_483_647_isize as Hkey;
    const HKLM: Hkey = -2_147_483_646_isize as Hkey;
    #[link(name = "advapi32")]
    extern "system" {
        fn RegOpenKeyExW(
            key: Hkey,
            subkey: *const u16,
            options: u32,
            access: u32,
            result: *mut Hkey,
        ) -> i32;
        fn RegQueryValueExW(
            key: Hkey,
            name: *const u16,
            reserved: *mut u32,
            kind: *mut u32,
            data: *mut u8,
            size: *mut u32,
        ) -> i32;
        fn RegCloseKey(key: Hkey) -> i32;
    }
    fn wide(text: &str) -> Vec<u16> {
        text.encode_utf16().chain(Some(0)).collect()
    }

    fn read_path(root: Hkey, key: &str, name: &str, view: u32) -> Option<PathBuf> {
        let key = wide(key);
        let mut handle = ptr::null_mut();
        // SAFETY: terminated string, writable handle, query-only access.
        if unsafe { RegOpenKeyExW(root, key.as_ptr(), 0, 1 | view, &mut handle) } != 0 {
            return None;
        }
        let name = wide(name);
        let mut words = vec![0_u16; 32768];
        let mut size = (words.len() * 2) as u32;
        let mut kind = 0;
        // SAFETY: buffers have their declared capacity; ERROR_MORE_DATA is
        // rejected. Only bounded REG_SZ strings are admitted.
        let status = unsafe {
            RegQueryValueExW(
                handle,
                name.as_ptr(),
                ptr::null_mut(),
                &mut kind,
                words.as_mut_ptr().cast(),
                &mut size,
            )
        };
        unsafe {
            RegCloseKey(handle);
        }
        if status != 0 || kind != 1 || size % 2 != 0 || size as usize > words.len() * 2 {
            return None;
        }
        words.truncate(size as usize / 2);
        let end = words
            .iter()
            .position(|word| *word == 0)
            .unwrap_or(words.len());
        (end != 0).then(|| OsString::from_wide(&words[..end]).into())
    }
    pub(super) fn installation_paths() -> Vec<PathBuf> {
        let mut paths = Vec::new();
        for root in [HKCU, HKLM] {
            for view in [0, 0x100, 0x200] {
                for name in [
                    "Install Path",
                    "InstallPath",
                    "Game Path",
                    "Working Path",
                    "Path",
                ] {
                    if let Some(path) = read_path(
                        root,
                        r"Software\Frontier Developments Ltd\V2000\1.0",
                        name,
                        view,
                    ) {
                        if !paths.contains(&path) {
                            paths.push(path);
                        }
                    }
                }
                for product in ["V2000", "Virus2000", "Virus 2000"] {
                    let key =
                        format!(r"Software\Microsoft\Windows\CurrentVersion\Uninstall\{product}");
                    if let Some(path) = read_path(root, &key, "InstallLocation", view) {
                        if !paths.contains(&path) {
                            paths.push(path);
                        }
                    }
                }
            }
        }
        paths
    }
}
