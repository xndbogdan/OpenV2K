use std::ffi::{c_void, OsString};
use std::mem::size_of;
use std::os::windows::ffi::OsStringExt;
use std::thread::sleep;
use std::time::{Duration, Instant};

type Handle = *mut c_void;

const TH32CS_SNAPPROCESS: u32 = 0x0000_0002;
const PROCESS_VM_READ: u32 = 0x0010;
const PROCESS_QUERY_INFORMATION: u32 = 0x0400;
const INVALID_HANDLE_VALUE: Handle = -1isize as Handle;

pub const RETAIL_IMAGE_BASE: usize = 0x0040_0000;
pub const RETAIL_MACHINE_I386: u16 = 0x014C;
pub const RETAIL_PE_TIMESTAMP: u32 = 0x35FF_80A4;
pub const RETAIL_SIZE_OF_IMAGE: u32 = 0x0010_F000;
pub const RETAIL_SHA256: &str = "E9BE7A833612FBA3A5A5AB92A974ECE1A689E4B7E72409D9EE8331380573B4BA";

#[repr(C)]
struct ProcessEntry32W {
    size: u32,
    usage: u32,
    process_id: u32,
    default_heap_id: usize,
    module_id: u32,
    threads: u32,
    parent_process_id: u32,
    priority_class_base: i32,
    flags: u32,
    exe_file: [u16; 260],
}

impl Default for ProcessEntry32W {
    fn default() -> Self {
        Self {
            size: size_of::<Self>() as u32,
            usage: 0,
            process_id: 0,
            default_heap_id: 0,
            module_id: 0,
            threads: 0,
            parent_process_id: 0,
            priority_class_base: 0,
            flags: 0,
            exe_file: [0; 260],
        }
    }
}

#[link(name = "kernel32")]
extern "system" {
    fn CreateToolhelp32Snapshot(flags: u32, process_id: u32) -> Handle;
    fn Process32FirstW(snapshot: Handle, entry: *mut ProcessEntry32W) -> i32;
    fn Process32NextW(snapshot: Handle, entry: *mut ProcessEntry32W) -> i32;
    fn OpenProcess(access: u32, inherit_handle: i32, process_id: u32) -> Handle;
    fn ReadProcessMemory(
        process: Handle,
        address: *const c_void,
        buffer: *mut c_void,
        size: usize,
        bytes_read: *mut usize,
    ) -> i32;
    fn CloseHandle(handle: Handle) -> i32;
}

struct OwnedHandle(Handle);

impl Drop for OwnedHandle {
    fn drop(&mut self) {
        if !self.0.is_null() && self.0 != INVALID_HANDLE_VALUE {
            unsafe {
                CloseHandle(self.0);
            }
        }
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct BuildFingerprint {
    pub machine: u16,
    pub timestamp: u32,
    pub image_base: u32,
    pub size_of_image: u32,
    pub expected_sha256: &'static str,
    pub verified_retail_build: bool,
}

pub struct Process {
    handle: OwnedHandle,
    pub process_id: u32,
    pub exe_name: String,
    pub build: BuildFingerprint,
}

impl Process {
    pub fn attach(requested_pid: Option<u32>) -> Result<Self, String> {
        let (process_id, exe_name) = find_original_process(requested_pid)?;
        Self::attach_identified(process_id, exe_name)
    }

    fn attach_identified(process_id: u32, exe_name: String) -> Result<Self, String> {
        let handle =
            unsafe { OpenProcess(PROCESS_VM_READ | PROCESS_QUERY_INFORMATION, 0, process_id) };
        if handle.is_null() {
            return Err(format!(
                "could not open {exe_name} (PID {process_id}); run the probe at the same privilege level"
            ));
        }
        let mut process = Self {
            handle: OwnedHandle(handle),
            process_id,
            exe_name,
            build: BuildFingerprint {
                machine: 0,
                timestamp: 0,
                image_base: 0,
                size_of_image: 0,
                expected_sha256: RETAIL_SHA256,
                verified_retail_build: false,
            },
        };
        process.build = process.read_build_fingerprint()?;
        if !process.build.verified_retail_build {
            return Err(format!(
                "unsupported executable build: machine={:#06X}, timestamp={:#010X}, image_base={:#010X}, size={:#010X}; expected the retail V2000.EXE documented in runtime_re/profiles",
                process.build.machine,
                process.build.timestamp,
                process.build.image_base,
                process.build.size_of_image
            ));
        }
        Ok(process)
    }

    /// Wait for a supported retail process and require it to remain readable
    /// for a short settling interval before handing it to a timeline probe.
    ///
    /// The retail launcher can briefly expose a V2000-named process and then
    /// replace it. Retrying the complete attach (including the PE fingerprint)
    /// avoids binding a long capture to that transient PID.
    pub fn attach_wait(
        requested_pid: Option<u32>,
        timeout: Duration,
        stable_for: Duration,
    ) -> Result<Self, String> {
        let start = Instant::now();
        let retry_interval = Duration::from_millis(100);
        let mut last_error = String::from("original V2000.EXE is not running");

        loop {
            let candidates = match find_original_processes() {
                Ok(candidates) => candidates
                    .into_iter()
                    .filter(|(pid, _)| requested_pid.is_none_or(|requested| requested == *pid))
                    .collect::<Vec<_>>(),
                Err(error) => {
                    last_error = error;
                    Vec::new()
                }
            };
            if candidates.is_empty() {
                last_error = requested_pid.map_or_else(
                    || "original V2000.EXE is not running".into(),
                    |pid| format!("PID {pid} is not a running V2000.EXE/virus2.exe"),
                );
            }

            for (process_id, exe_name) in candidates {
                match Self::attach_identified(process_id, exe_name) {
                    Ok(process) => {
                        let stable_start = Instant::now();
                        let mut remained_readable = true;
                        while stable_start.elapsed() < stable_for {
                            if start.elapsed() >= timeout {
                                return Err(attach_wait_timeout(timeout, &last_error));
                            }
                            sleep(
                                retry_interval
                                    .min(stable_for.saturating_sub(stable_start.elapsed())),
                            );
                            match process.read_bytes(RETAIL_IMAGE_BASE, 2) {
                                Ok(bytes) if bytes == b"MZ" => {}
                                Ok(_) => {
                                    last_error = format!(
                                    "{} (PID {}) stopped exposing the retail image while settling",
                                    process.exe_name, process.process_id
                                );
                                    remained_readable = false;
                                    break;
                                }
                                Err(error) => {
                                    last_error = format!(
                                        "{} (PID {}) vanished while settling: {error}",
                                        process.exe_name, process.process_id
                                    );
                                    remained_readable = false;
                                    break;
                                }
                            }
                        }
                        if remained_readable {
                            return Ok(process);
                        }
                    }
                    Err(error) => last_error = error,
                }
            }

            if start.elapsed() >= timeout {
                return Err(attach_wait_timeout(timeout, &last_error));
            }
            sleep(retry_interval.min(timeout.saturating_sub(start.elapsed())));
        }
    }

    pub fn read_bytes(&self, address: usize, size: usize) -> Result<Vec<u8>, String> {
        let mut bytes = vec![0u8; size];
        let mut read = 0usize;
        let ok = unsafe {
            ReadProcessMemory(
                self.handle.0,
                address as *const c_void,
                bytes.as_mut_ptr().cast(),
                size,
                &mut read,
            )
        };
        if ok == 0 || read != size {
            return Err(format!(
                "ReadProcessMemory({address:08X}, {size:#x}) read {read:#x} bytes"
            ));
        }
        Ok(bytes)
    }

    pub fn read_u16(&self, address: usize) -> Result<u16, String> {
        let bytes = self.read_bytes(address, 2)?;
        Ok(u16::from_le_bytes([bytes[0], bytes[1]]))
    }

    pub fn read_i16(&self, address: usize) -> Result<i16, String> {
        Ok(self.read_u16(address)? as i16)
    }

    pub fn read_u32(&self, address: usize) -> Result<u32, String> {
        let bytes = self.read_bytes(address, 4)?;
        Ok(u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
    }

    pub fn read_i32(&self, address: usize) -> Result<i32, String> {
        Ok(self.read_u32(address)? as i32)
    }

    fn read_build_fingerprint(&self) -> Result<BuildFingerprint, String> {
        let dos = self.read_bytes(RETAIL_IMAGE_BASE, 0x40)?;
        if &dos[0..2] != b"MZ" {
            return Err("process image at 0x00400000 has no DOS MZ header".into());
        }
        let pe_offset = u32_at(&dos, 0x3C) as usize;
        if !(0x40..=0x1000).contains(&pe_offset) {
            return Err(format!("implausible PE header offset {pe_offset:#x}"));
        }
        let pe = self.read_bytes(RETAIL_IMAGE_BASE + pe_offset, 0x80)?;
        if &pe[0..4] != b"PE\0\0" {
            return Err("process image has no PE signature".into());
        }
        let machine = u16_at(&pe, 4);
        let timestamp = u32_at(&pe, 8);
        let optional_magic = u16_at(&pe, 24);
        if optional_magic != 0x010B {
            return Err(format!(
                "expected PE32 optional header, found {optional_magic:#06x}"
            ));
        }
        let image_base = u32_at(&pe, 24 + 28);
        let size_of_image = u32_at(&pe, 24 + 56);
        Ok(BuildFingerprint {
            machine,
            timestamp,
            image_base,
            size_of_image,
            expected_sha256: RETAIL_SHA256,
            verified_retail_build: machine == RETAIL_MACHINE_I386
                && timestamp == RETAIL_PE_TIMESTAMP
                && image_base as usize == RETAIL_IMAGE_BASE
                && size_of_image == RETAIL_SIZE_OF_IMAGE,
        })
    }
}

fn attach_wait_timeout(timeout: Duration, last_error: &str) -> String {
    format!(
        "timed out after {:.1}s waiting for a stable supported retail process; last attach error: {last_error}",
        timeout.as_secs_f64()
    )
}

fn find_original_process(requested_pid: Option<u32>) -> Result<(u32, String), String> {
    let mut candidates = find_original_processes()?;
    candidates.retain(|(pid, _)| requested_pid.is_none_or(|requested| requested == *pid));

    match candidates.len() {
        0 if requested_pid.is_some() => Err(format!(
            "PID {} is not a running V2000.EXE/virus2.exe",
            requested_pid.unwrap()
        )),
        0 => Err("original V2000.EXE is not running".into()),
        1 => Ok(candidates.remove(0)),
        _ => Err(format!(
            "multiple original processes are running ({}); select one with --pid",
            candidates
                .iter()
                .map(|(pid, name)| format!("{name}:{pid}"))
                .collect::<Vec<_>>()
                .join(", ")
        )),
    }
}

fn find_original_processes() -> Result<Vec<(u32, String)>, String> {
    let snapshot = unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) };
    if snapshot == INVALID_HANDLE_VALUE {
        return Err("CreateToolhelp32Snapshot failed".into());
    }
    let snapshot = OwnedHandle(snapshot);
    let mut entry = ProcessEntry32W::default();
    let mut candidates = Vec::new();
    let mut ok = unsafe { Process32FirstW(snapshot.0, &mut entry) } != 0;
    while ok {
        let end = entry
            .exe_file
            .iter()
            .position(|&unit| unit == 0)
            .unwrap_or(entry.exe_file.len());
        let name = OsString::from_wide(&entry.exe_file[..end])
            .to_string_lossy()
            .into_owned();
        let named_original =
            name.eq_ignore_ascii_case("v2000.exe") || name.eq_ignore_ascii_case("virus2.exe");
        if named_original {
            candidates.push((entry.process_id, name));
        }
        ok = unsafe { Process32NextW(snapshot.0, &mut entry) } != 0;
    }

    Ok(candidates)
}

pub fn u16_at(bytes: &[u8], offset: usize) -> u16 {
    u16::from_le_bytes([bytes[offset], bytes[offset + 1]])
}

pub fn i16_at(bytes: &[u8], offset: usize) -> i16 {
    u16_at(bytes, offset) as i16
}

pub fn u32_at(bytes: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes([
        bytes[offset],
        bytes[offset + 1],
        bytes[offset + 2],
        bytes[offset + 3],
    ])
}

pub fn plausible_heap_pointer(pointer: usize) -> bool {
    plausible_data_pointer(pointer) && pointer & 3 == 0
}

/// A bounded 32-bit user-space address without a structural alignment claim.
///
/// Retail PCM allocations are raw byte/word payloads and may begin at an
/// address that is only two-byte aligned. Their bounded `ReadProcessMemory`
/// result validates readability; the stricter four-byte helper remains for
/// linked structures and pointer tables.
pub fn plausible_data_pointer(pointer: usize) -> bool {
    (0x1_0000..=0x7FFF_FFFF).contains(&pointer)
}

#[cfg(test)]
mod pointer_tests {
    use super::{plausible_data_pointer, plausible_heap_pointer};

    #[test]
    fn raw_data_allows_word_aligned_pcm_without_weakening_structural_pointers() {
        let retail_pcm = 0x31A0_CC22;
        assert!(plausible_data_pointer(retail_pcm));
        assert!(!plausible_heap_pointer(retail_pcm));
        assert!(plausible_heap_pointer(0x31A0_CC24));
        assert!(!plausible_data_pointer(0));
        assert!(!plausible_data_pointer(0x8000_0000));
    }
}
