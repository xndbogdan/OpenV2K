//! Narrow Windows DWORD storage. No original-installation write entry point.

use std::ffi::c_void;
use std::io;
use std::ptr;

use super::{NativeSettings, FIELDS, PORT_KEY, RETAIL_KEY};

type Hkey = *mut c_void;
const CURRENT_USER: Hkey = -2_147_483_647_isize as Hkey;
const QUERY_VALUE: u32 = 0x0001;
const SET_VALUE: u32 = 0x0002;
const REG_DWORD: u32 = 4;

#[link(name = "advapi32")]
extern "system" {
    fn RegOpenKeyExW(
        key: Hkey,
        subkey: *const u16,
        options: u32,
        access: u32,
        result: *mut Hkey,
    ) -> i32;
    fn RegCreateKeyExW(
        key: Hkey,
        subkey: *const u16,
        reserved: u32,
        class: *mut u16,
        options: u32,
        access: u32,
        security: *const c_void,
        result: *mut Hkey,
        disposition: *mut u32,
    ) -> i32;
    fn RegQueryValueExW(
        key: Hkey,
        name: *const u16,
        reserved: *mut u32,
        kind: *mut u32,
        data: *mut u8,
        size: *mut u32,
    ) -> i32;
    fn RegSetValueExW(
        key: Hkey,
        name: *const u16,
        reserved: u32,
        kind: u32,
        data: *const u8,
        size: u32,
    ) -> i32;
    fn RegCloseKey(key: Hkey) -> i32;
}

struct Key(Hkey);
impl Drop for Key {
    fn drop(&mut self) {
        // SAFETY: Key is created only after a successful open/create and owns
        // exactly one handle. Predefined root handles are never wrapped.
        unsafe {
            RegCloseKey(self.0);
        }
    }
}

fn wide(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(Some(0)).collect()
}

pub(super) fn read(path: &str) -> Option<NativeSettings> {
    let mut handle = ptr::null_mut();
    let path = wide(path);
    // SAFETY: path is NUL-terminated, handle points to writable storage, and
    // QUERY_VALUE grants no write access to either the retail or port key.
    if unsafe { RegOpenKeyExW(CURRENT_USER, path.as_ptr(), 0, QUERY_VALUE, &mut handle) } != 0 {
        return None;
    }
    let key = Key(handle);
    let mut settings = NativeSettings(Default::default());
    let mut found = false;
    for (index, (name, _, _)) in FIELDS.into_iter().enumerate() {
        let name = wide(name);
        let mut value = [0_u8; 4];
        let mut kind = 0;
        let mut size = 4;
        // SAFETY: output pointers reference initialized storage of their
        // declared capacity; the API reports type and actual byte count.
        let status = unsafe {
            RegQueryValueExW(
                key.0,
                name.as_ptr(),
                ptr::null_mut(),
                &mut kind,
                value.as_mut_ptr(),
                &mut size,
            )
        };
        if status == 0 && kind == REG_DWORD && size == 4 {
            settings.set(index, u32::from_le_bytes(value));
            found = true;
        }
    }
    found.then_some(settings)
}

/// 4493D0 accepts REG_SZ only. This imported path is never a write target.
pub(super) fn read_save_directory() -> Option<std::path::PathBuf> {
    let path = wide(RETAIL_KEY);
    let mut handle = ptr::null_mut();
    // SAFETY: terminated path and writable handle; access is read-only.
    if unsafe { RegOpenKeyExW(CURRENT_USER, path.as_ptr(), 0, QUERY_VALUE, &mut handle) } != 0 {
        return None;
    }
    let key = Key(handle);
    let name = wide("Save Path");
    let mut kind = 0;
    let mut size = 0;
    // SAFETY: null data asks for the required byte count only.
    if unsafe {
        RegQueryValueExW(
            key.0,
            name.as_ptr(),
            ptr::null_mut(),
            &mut kind,
            ptr::null_mut(),
            &mut size,
        )
    } != 0
        || kind != 1
        || !(2..=65536).contains(&size)
        || size % 2 != 0
    {
        return None;
    }
    let mut words = vec![0_u16; size as usize / 2];
    // SAFETY: buffer contains the requested number of bytes. A growing value
    // returns ERROR_MORE_DATA instead of overflowing this buffer.
    if unsafe {
        RegQueryValueExW(
            key.0,
            name.as_ptr(),
            ptr::null_mut(),
            &mut kind,
            words.as_mut_ptr().cast(),
            &mut size,
        )
    } != 0
        || kind != 1
        || size % 2 != 0
        || size as usize > words.len() * 2
    {
        return None;
    }
    words.truncate(size as usize / 2);
    decode_save_directory(&words)
}

fn decode_save_directory(words: &[u16]) -> Option<std::path::PathBuf> {
    use std::os::windows::ffi::OsStringExt;
    // REG_SZ is not guaranteed to contain its terminator. Stop at the first
    // NUL, as the ANSI consumer, without reading beyond the returned length.
    let end = words
        .iter()
        .position(|word| *word == 0)
        .unwrap_or(words.len());
    (end != 0).then(|| std::ffi::OsString::from_wide(&words[..end]).into())
}

pub(super) fn write(path: &str, settings: &NativeSettings) -> io::Result<()> {
    // Keep the import-only guarantee local to the operating-system boundary.
    if path != PORT_KEY {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "retail settings are read-only",
        ));
    }
    let path = wide(path);
    let mut handle = ptr::null_mut();
    // SAFETY: NUL-terminated path, no class/security buffers, writable output.
    let status = unsafe {
        RegCreateKeyExW(
            CURRENT_USER,
            path.as_ptr(),
            0,
            ptr::null_mut(),
            0,
            SET_VALUE,
            ptr::null(),
            &mut handle,
            ptr::null_mut(),
        )
    };
    if status != 0 {
        return Err(io::Error::from_raw_os_error(status));
    }
    let key = Key(handle);
    for (index, (name, _, _)) in FIELDS.into_iter().enumerate() {
        let name = wide(name);
        let value = settings.get(index).to_le_bytes();
        // SAFETY: the DWORD input is exactly four bytes; name is terminated.
        let status =
            unsafe { RegSetValueExW(key.0, name.as_ptr(), 0, REG_DWORD, value.as_ptr(), 4) };
        if status != 0 {
            return Err(io::Error::from_raw_os_error(status));
        }
    }
    // Other values in the owned key are untouched, just as unknown values in
    // an imported retail key are never enumerated, deleted, or rewritten.
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn save_path_string_is_bounded_and_not_environment_expanded() {
        assert!(decode_save_directory(&[0]).is_none());
        let mut value: Vec<u16> = r"D:\Original Saves\Ω".encode_utf16().collect();
        assert_eq!(
            decode_save_directory(&value).unwrap(),
            std::path::PathBuf::from(r"D:\Original Saves\Ω")
        );
        value.extend([0, 0x58]);
        assert_eq!(
            decode_save_directory(&value).unwrap(),
            std::path::PathBuf::from(r"D:\Original Saves\Ω")
        );
        assert_eq!(
            decode_save_directory(&wide(r"%USERPROFILE%\Saves")).unwrap(),
            std::path::PathBuf::from(r"%USERPROFILE%\Saves")
        );
    }
}
